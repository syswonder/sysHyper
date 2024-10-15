use core::ptr;

use crate::config::{HvPciConfig, CONFIG_MAX_PCI_DEV};
use crate::pci::{get_ecam_base, init_ecam_base};
use crate::percpu::this_zone;
use crate::zone::this_zone_id;
use crate::{
    error::HvResult, 
    memory::MMIOAccess,
    zone::Zone,
    memory::{MemoryRegion,GuestPhysAddr,MemFlags,mmio_perform_access},
};
use alloc::vec::Vec;

use super::bridge::BridgeConfig;
use super::endpoint::EndpointConfig;
use super::pcibar::BarRegion;
use super::{cfg_base, ECAM_BASE, NUM_BAR_REGS_TYPE0, NUM_BAR_REGS_TYPE1, PHANTOM_DEV_HEADER};

#[cfg(all(feature = "platform_qemu", target_arch = "aarch64"))]
use crate::arch::iommu::iommu_add_device;

#[derive(Debug)]
pub struct PciRoot {
    endpoints: Vec<EndpointConfig>,
    bridges: Vec<BridgeConfig>,
    alloc_devs: Vec<usize>, // include host bridge
    bar_regions: Vec<BarRegion>,
}
impl PciRoot{
    pub fn new() -> Self {
        let r = Self {
            endpoints: Vec::new(),
            bridges: Vec::new(),
            alloc_devs: Vec::new(),
            bar_regions: Vec::new(),
        };
        r
    }

    pub fn is_assigned_device(&self, bdf: usize) -> bool {
        if self.alloc_devs.contains(&bdf){
            true
        }else{
            false
        }
    }

    pub fn bars_register(&mut self){
        self.ep_bars_init();
        self.bridge_bars_init();
        self.get_bars_regions();
    }

    fn get_bars_regions(&mut self){
        for ep in self.endpoints.iter(){
            let regions = ep.get_regions();
            for mut region in regions {
                if region.size < 0x1000{
                    region.size = 0x1000;
                }
                self.bar_regions.push(region);
            }
        }
        for bridge in self.bridges.iter(){
            let regions = bridge.get_regions();
            for mut region in regions {
                if region.size < 0x1000{
                    region.size = 0x1000;
                }
                self.bar_regions.push(region);
            }
        }
        info!("PCI BAR regions init done");
    }

    fn ep_bars_init(&mut self){
        for ep in self.endpoints.iter_mut(){
            let cfg_base = cfg_base(ep.bdf);
            let offsets:[usize;NUM_BAR_REGS_TYPE0] = [0x10, 0x14, 0x18, 0x1c, 0x20, 0x24];
            for bar_id in 0..NUM_BAR_REGS_TYPE0{
                unsafe {
                    let reg_ptr = (cfg_base + offsets[bar_id]) as *mut u32;
                    let origin_val = *reg_ptr;
                    *reg_ptr = 0xffffffffu32;
                    let new_val = *reg_ptr;
                    ep.bars_init(bar_id, origin_val, new_val);
                    *reg_ptr = origin_val;
                }
            }
        }
    }

    fn bridge_bars_init(&mut self){
        for bridge in self.bridges.iter_mut(){
            let cfg_base = cfg_base(bridge.bdf);
            let offsets:[usize;NUM_BAR_REGS_TYPE1] = [0x10, 0x14];
            for bar_id in 0..NUM_BAR_REGS_TYPE1{
                unsafe {
                    let reg_ptr = (cfg_base + offsets[bar_id]) as *mut u32;
                    let origin_val = *reg_ptr;
                    *reg_ptr = 0xffffffffu32;
                    let new_val = *reg_ptr;
                    bridge.bars_init(bar_id, origin_val, new_val);
                    *reg_ptr = origin_val;
                }
            }
        }
    }

}

impl Zone {
    pub fn pci_init(&mut self, pci_config: &HvPciConfig, num_pci_devs: usize, alloc_pci_devs: &[u64; CONFIG_MAX_PCI_DEV]){
        info!("PCIe init!");

        init_ecam_base(pci_config.ecam_base as _);

        info!("ecam base : {:#x}", get_ecam_base());
        info!("cfg base : {:#x}", cfg_base(0));
        
        for idx in 0..num_pci_devs {
            info!("PCIe device assigned to zone {}: {:#x}", self.id, alloc_pci_devs[idx]);
            self.pciroot.alloc_devs.push(alloc_pci_devs[idx] as _);
            #[cfg(all(feature = "platform_qemu", target_arch = "aarch64"))]
            if alloc_pci_devs[idx] != 0{
                iommu_add_device(self.id, alloc_pci_devs[idx] as _, 0);
            }
        }

        if self.id == 0 {
            self.root_pci_init(pci_config);
        }else{
            self.virtual_pci_mmio_init(pci_config);
            self.virtual_pci_device_init();
        }
    }
    pub fn root_pci_init(&mut self, pci_config: &HvPciConfig){
        // Virtual ECAM
        self.mmio_region_register(pci_config.ecam_base as _, pci_config.ecam_size as _, mmio_pci_handler, pci_config.ecam_base as _);

        self.gpm.insert(MemoryRegion::new_with_offset_mapper(
            pci_config.io_base as GuestPhysAddr, 
            pci_config.io_base as _, 
            pci_config.io_size as _, 
            MemFlags::READ | MemFlags::WRITE,
        )).ok();

        self.gpm.insert(MemoryRegion::new_with_offset_mapper(
            pci_config.mem32_base as GuestPhysAddr, 
            pci_config.mem32_base as _, 
            pci_config.mem32_size as _, 
            MemFlags::READ | MemFlags::WRITE,
        )).ok();

        self.gpm.insert(MemoryRegion::new_with_offset_mapper(
            pci_config.mem64_base as GuestPhysAddr, 
            pci_config.mem64_base as _, 
            pci_config.mem64_size as _, 
            MemFlags::READ | MemFlags::WRITE,
        )).ok();

    }

    //probe pci mmio
    pub fn virtual_pci_mmio_init(&mut self, pci_config: &HvPciConfig){
        self.mmio_region_register(pci_config.ecam_base as _, pci_config.ecam_size as _, mmio_pci_handler, pci_config.ecam_base as _);
        self.mmio_region_register(pci_config.io_base as _, pci_config.io_size as _, mmio_pci_handler, pci_config.io_base as _);
        self.mmio_region_register(pci_config.mem32_base as _, pci_config.mem32_size as _, mmio_pci_handler, pci_config.mem32_base as _);
        self.mmio_region_register(pci_config.mem64_base as _, pci_config.mem64_size as _, mmio_pci_handler, pci_config.mem64_base as _);
    }

    pub fn virtual_pci_device_init(&mut self){
        for bdf in self.pciroot.alloc_devs.clone() {
            if bdf != 0 {
                let base = cfg_base(bdf) + 0xe;
                let header_val = unsafe { ptr::read_volatile(base as *mut u8) };
                match header_val & 0b1111111 {
                    0b0 => self.pciroot.endpoints.push(EndpointConfig::new(bdf)),
                    0b1 => self.pciroot.bridges.push(BridgeConfig::new(bdf)),
                    _ => error!("unsupported device type!"),
                };
            }else{
                // host bridge
            }
        }
        
        trace!("pciroot = {:?}", self.pciroot);
        self.pciroot.bars_register();
        self.pci_bars_register();
    }

    fn pci_bars_register(&mut self){
        for region in self.pciroot.bar_regions.iter(){
            self.gpm.insert(MemoryRegion::new_with_offset_mapper(
                region.start as GuestPhysAddr,
                region.start,
                region.size,
                MemFlags::READ | MemFlags::WRITE,
            )).ok();
        }
    }
}

pub fn mmio_pci_handler(mmio: &mut MMIOAccess, base: usize) -> HvResult{
    let reg_addr = mmio.address & 0xfff;
    let bdf = mmio.address >> 12;
    let function = bdf & 0x7;
    let device = (bdf >> 3) & 0b11111;
    let bus = bdf >> 8;
    
    let zone = this_zone();
    let binding = zone.write();
    let is_assigned = binding.pciroot.is_assigned_device(bdf);

    match is_assigned {
        true => {
            mmio_perform_access(base, mmio);
        },
        false => {
            if reg_addr == 0 {
                let header_addr = base + mmio.address;
                let header_val = unsafe { ptr::read_volatile(header_addr as *mut u32) };
                if header_val == 0xffffffffu32 {
                    // empty device
                    mmio.value = 0xffffffffu32 as _;
                }else {
                    // phantom device
                    mmio.value = PHANTOM_DEV_HEADER as _;
                }
            } else {
                // for BAR, INTX etc.
                mmio_perform_access(base, mmio);
            }
        }
    }
    if mmio.is_write == true {
        trace!("ecam write {} bytes, {:x}:{:x}:{:x} off:{:#x} -> {:#x}", mmio.size, bus, device, function, reg_addr, mmio.value);
    }else {
        trace!("ecam read  {} bytes, {:x}:{:x}:{:x} off:{:#x} -> 0x{:#x}",mmio.size, bus, device, function, reg_addr, mmio.value);
    }

    Ok(())
}
