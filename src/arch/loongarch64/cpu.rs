use super::ipi::*;
use crate::device::common::MMIODerefWrapper;
use core::arch::asm;
use loongArch64::register::cpuid;
use loongArch64::register::pgdl;
use tock_registers::interfaces::Writeable;

use crate::{
    consts::{PER_CPU_ARRAY_PTR, PER_CPU_SIZE},
    memory::VirtAddr,
};

#[repr(C)]
#[derive(Debug)]
pub struct ArchCpu {
    pub r: [usize; 32], // r0~r31
    pub sepc: usize,
    pub stack_top: usize,
    pub cpuid: usize,
    pub power_on: bool,
}

impl ArchCpu {
    pub fn new(cpuid: usize) -> Self {
        ArchCpu {
            r: [0; 32],
            sepc: 0,
            stack_top: 0,
            cpuid,
            power_on: false,
        }
    }
    pub fn get_cpuid(&self) -> usize {
        self.cpuid
    }
    pub fn stack_top(&self) -> VirtAddr {
        PER_CPU_ARRAY_PTR as VirtAddr + (self.get_cpuid() + 1) as usize * PER_CPU_SIZE - 8
    }
    pub fn init(&mut self, entry: usize, cpu_id: usize, dtb: usize) {
        println!(
            "loongarch64: ArchCpu::init: entry={:#x}, cpu_id={}",
            entry, cpu_id
        );
    }
    pub fn run(&self) -> ! {
        println!("loongarch64: ArchCpu::run: cpuid={}", self.get_cpuid());
        panic!("should not reach here");
    }
    pub fn idle(&self) -> ! {
        println!("loongarch64: ArchCpu::idle: cpuid={}", self.get_cpuid());
        panic!("should not reach here");
    }
}

pub fn this_cpu_id() -> usize {
    cpuid::read().core_id()
}

pub fn cpu_start(cpuid: usize, start_addr: usize, opaque: usize) {
    println!(
        "loongarch64: cpu_start: cpuid={}, start_addr={:#x}, opaque={:#x}",
        cpuid, start_addr, opaque
    );
    let ipi: &MMIODerefWrapper<IpiRegisters> = match cpuid {
        1 => &CORE1_IPI,
        2 => &CORE2_IPI,
        3 => &CORE3_IPI,
        _ => {
            error!("loongarch64: cpu_start: invalid cpuid={}", cpuid);
            return;
        }
    };
    ipi.ipi_enable.write(IpiEnable::IPIENABLE.val(0xffffffff));
    let entry_addr = start_addr;
    mail_send(entry_addr, cpuid, 0);
    ipi_write_action(cpuid, SMP_BOOT_CPU);
}
