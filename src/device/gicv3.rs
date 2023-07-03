// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2020-2022 Andre Richter <andre.o.richter@gmail.com>

//! GICv2 Driver - ARM Generic Interrupt Controller v2.
//!
//! The following is a collection of excerpts with useful information from
//!   - `Programmer's Guide for ARMv8-A`
//!   - `ARM Generic Interrupt Controller Architecture Specification`
//!
//! # Programmer's Guide - 10.6.1 Configuration
//!
//! The GIC is accessed as a memory-mapped peripheral.
//!
//! All cores can access the common Distributor, but the CPU interface is banked, that is, each core
//! uses the same address to access its own private CPU interface.
//!
//! It is not possible for a core to access the CPU interface of another core.
//!
//! # Architecture Specification - 10.6.2 Initialization
//!
//! Both the Distributor and the CPU interfaces are disabled at reset. The GIC must be initialized
//! after reset before it can deliver interrupts to the core.
//!
//! In the Distributor, software must configure the priority, target, security and enable individual
//! interrupts. The Distributor must subsequently be enabled through its control register
//! (GICD_CTLR). For each CPU interface, software must program the priority mask and preemption
//! settings.
//!
//! Each CPU interface block itself must be enabled through its control register (GICD_CTLR). This
//! prepares the GIC to deliver interrupts to the core.
//!
//! Before interrupts are expected in the core, software prepares the core to take interrupts by
//! setting a valid interrupt vector in the vector table, and clearing interrupt mask bits in
//! PSTATE, and setting the routing controls.
//!
//! The entire interrupt mechanism in the system can be disabled by disabling the Distributor.
//! Interrupt delivery to an individual core can be disabled by disabling its CPU interface.
//! Individual interrupts can also be disabled (or enabled) in the distributor.
//!
//! For an interrupt to reach the core, the individual interrupt, Distributor and CPU interface must
//! all be enabled. The interrupt also needs to be of sufficient priority, that is, higher than the
//! core's priority mask.
//!
//! # Architecture Specification - 1.4.2 Interrupt types
//!
//! - Peripheral interrupt
//!     - Private Peripheral Interrupt (PPI)
//!         - This is a peripheral interrupt that is specific to a single processor.
//!     - Shared Peripheral Interrupt (SPI)
//!         - This is a peripheral interrupt that the Distributor can route to any of a specified
//!           combination of processors.
//!
//! - Software-generated interrupt (SGI)
//!     - This is an interrupt generated by software writing to a GICD_SGIR register in the GIC. The
//!       system uses SGIs for interprocessor communication.
//!     - An SGI has edge-triggered properties. The software triggering of the interrupt is
//!       equivalent to the edge transition of the interrupt request signal.
//!     - When an SGI occurs in a multiprocessor implementation, the CPUID field in the Interrupt
//!       Acknowledge Register, GICC_IAR, or the Aliased Interrupt Acknowledge Register, GICC_AIAR,
//!       identifies the processor that requested the interrupt.
//!
//! # Architecture Specification - 2.2.1 Interrupt IDs
//!
//! Interrupts from sources are identified using ID numbers. Each CPU interface can see up to 1020
//! interrupts. The banking of SPIs and PPIs increases the total number of interrupts supported by
//! the Distributor.
//!
//! The GIC assigns interrupt ID numbers ID0-ID1019 as follows:
//!   - Interrupt numbers 32..1019 are used for SPIs.
//!   - Interrupt numbers 0..31 are used for interrupts that are private to a CPU interface. These
//!     interrupts are banked in the Distributor.
//!       - A banked interrupt is one where the Distributor can have multiple interrupts with the
//!         same ID. A banked interrupt is identified uniquely by its ID number and its associated
//!         CPU interface number. Of the banked interrupt IDs:
//!           - 00..15 SGIs
//!           - 16..31 PPIs

mod gicd;
mod gicr;
use crate::arch::sysreg::{read_sysreg, write_sysreg};
use crate::hypercall::SGI_HV_ID;
/// Representation of the GIC.
pub struct GICv3 {
    /// The Distributor.
    gicd: gicd::GICD,

    /// The CPU Interface.
    gicr: gicr::GICR,
}
impl GICv3 {
    /// - The user must ensure to provide a correct MMIO start address.
    pub const unsafe fn new(gicd_mmio_start_addr: usize, gicr_mmio_start_addr: usize) -> Self {
        Self {
            gicd: gicd::GICD::new(gicd_mmio_start_addr),
            gicr: gicr::GICR::new(gicr_mmio_start_addr),
        }
    }
    pub fn read_aff(&self) -> u64 {
        self.gicr.read_aff()
    }
}
fn sdei_check() -> i64 {
    unsafe {
        core::arch::asm!(
            "
    ldr x0, =0xc4000020
    smc #0
    ret
    ",
            options(noreturn),
        );
    }
}
pub fn gicv3_cpu_init() {
    // unsafe {write_sysreg!(icc_sgi1r_el1, val);}
    // let intid = unsafe { read_sysreg!(icc_iar1_el1) } as u32;
    //arm_read_sysreg(ICC_CTLR_EL1, cell_icc_ctlr);
    let sdei_ver = sdei_check();
    info!("sdei vecsion: {}", sdei_ver);
    info!("gicv3 init!");
    unsafe {
        let ctlr = read_sysreg!(icc_ctlr_el1);
        write_sysreg!(icc_ctlr_el1, 0x2);
        let pmr = read_sysreg!(icc_pmr_el1);
        write_sysreg!(icc_pmr_el1, 0xf0);
        let igrpen = read_sysreg!(icc_igrpen1_el1);
        write_sysreg!(icc_igrpen1_el1, 0x1);
        let vtr = read_sysreg!(ich_vtr_el2);
        let mut vmcr = ((pmr & 0xff) << 24) | (1 << 1) | (1 << 9);
        write_sysreg!(ich_vmcr_el2, vmcr);
        write_sysreg!(ich_hcr_el2, 0x1);
    }
}

pub fn gicv3_handle_irq_el1() {
    if let Some(irq_id) = pending_irq() {
        if (irq_id < 16) {
            debug!("sgi get {}", irq_id);
        }
        if irq_id == SGI_HV_ID as usize {
            info!("hv sgi got {}", irq_id);
            loop {}
        }

        deactivate_irq(irq_id);
        inject_irq(irq_id);
    }
}
fn pending_irq() -> Option<usize> {
    let iar = unsafe { read_sysreg!(icc_iar1_el1) } as usize;
    if iar >= 0x3fe {
        // spurious
        None
    } else {
        Some(iar as _)
    }
}
fn deactivate_irq(irq_id: usize) {
    unsafe {
        write_sysreg!(icc_eoir1_el1, irq_id as u64);
        if irq_id < 16 {
            write_sysreg!(icc_dir_el1, irq_id as u64);
        }
        //write_sysreg!(icc_dir_el1, irq_id as u64);
    }
}
fn read_lr(id: usize) -> u64 {
    unsafe {
        match id {
            //TODO get lr size from gic reg
            0 => read_sysreg!(ich_lr0_el2),
            1 => read_sysreg!(ich_lr1_el2),
            2 => read_sysreg!(ich_lr2_el2),
            3 => read_sysreg!(ich_lr3_el2),
            4 => read_sysreg!(ich_lr4_el2),
            5 => read_sysreg!(ich_lr5_el2),
            6 => read_sysreg!(ich_lr6_el2),
            7 => read_sysreg!(ich_lr7_el2),
            8 => read_sysreg!(ich_lr8_el2),
            9 => read_sysreg!(ich_lr9_el2),
            10 => read_sysreg!(ich_lr10_el2),
            11 => read_sysreg!(ich_lr11_el2),
            12 => read_sysreg!(ich_lr12_el2),
            13 => read_sysreg!(ich_lr13_el2),
            14 => read_sysreg!(ich_lr14_el2),
            15 => read_sysreg!(ich_lr15_el2),
            _ => {
                error!("lr over");
                loop {}
            }
        }
    }
}
fn write_lr(id: usize, val: u64) {
    unsafe {
        match id {
            0 => write_sysreg!(ich_lr0_el2, val),
            1 => write_sysreg!(ich_lr1_el2, val),
            2 => write_sysreg!(ich_lr2_el2, val),
            3 => write_sysreg!(ich_lr3_el2, val),
            4 => write_sysreg!(ich_lr4_el2, val),
            5 => write_sysreg!(ich_lr5_el2, val),
            6 => write_sysreg!(ich_lr6_el2, val),
            7 => write_sysreg!(ich_lr7_el2, val),
            8 => write_sysreg!(ich_lr8_el2, val),
            9 => write_sysreg!(ich_lr9_el2, val),
            10 => write_sysreg!(ich_lr10_el2, val),
            11 => write_sysreg!(ich_lr11_el2, val),
            12 => write_sysreg!(ich_lr12_el2, val),
            13 => write_sysreg!(ich_lr13_el2, val),
            14 => write_sysreg!(ich_lr14_el2, val),
            15 => write_sysreg!(ich_lr15_el2, val),
            _ => {
                error!("lr over");
                loop {}
            }
        }
    }
}

fn inject_irq(irq_id: usize) {
    // mask
    const LR_VIRTIRQ_MASK: usize = 0x3ff;
    const LR_PHYSIRQ_MASK: usize = 0x3ff << 10;

    const LR_PENDING_BIT: u64 = 1 << 28;
    const LR_HW_BIT: u64 = 1 << 31;
    let elsr: u64 = unsafe { read_sysreg!(ich_elrsr_el2) };
    let vtr = unsafe { read_sysreg!(ich_vtr_el2) } as usize;
    let lr_num: usize = (vtr & 0xf) + 1;
    let mut lr_idx = -1 as isize;
    for i in 0..lr_num {
        if (1 << i) & elsr > 0 {
            if lr_idx == -1 {
                lr_idx = i as isize;
            }
            continue;
        }
        // overlap
        let lr_val = read_lr(i) as usize;
        if (i & LR_VIRTIRQ_MASK) == irq_id {
            warn!("irq mask!{} {}", i, irq_id);
            return;
        }
    }
    //debug!("To Inject IRQ {}, find lr {}", irq_id, lr_idx);

    if lr_idx == -1 {
        warn!("full lr");
        return;
    } else {
        // lr = irq_id;
        // /* Only group 1 interrupts */
        // lr |= ICH_LR_GROUP_BIT;
        // lr |= ICH_LR_PENDING;
        // if (!is_sgi(irq_id)) {
        //     lr |= ICH_LR_HW_BIT;
        //     lr |= (u64)irq_id << ICH_LR_PHYS_ID_SHIFT;
        // }
        let mut val = 0;

        val = irq_id as u64; //v intid
        val |= 1 << 60; //group 1
        val |= 1 << 62; //state pending
        val |= 1 << 61; //map hardware
        val |= ((irq_id as u64) << 32); //p intid
                                        //debug!("To write lr {} val {}", lr_idx, val);
        write_lr(lr_idx as usize, val);
    }
}
