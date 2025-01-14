use crate::{arch::zone::HvArchZoneConfig, config::*};

pub const ROOT_ZONE_DTB_ADDR: u64 = 0x08300000;
pub const ROOT_ZONE_KERNEL_ADDR: u64 = 0x00280000;
pub const ROOT_ZONE_ENTRY: u64 = 0x00280000;
pub const ROOT_ZONE_CPUS: u64 = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3);

pub const ROOT_ZONE_NAME: &str = "root-linux";

pub const ROOT_ZONE_MEMORY_REGIONS: [HvConfigMemoryRegion; 1] = [
    // HvConfigMemoryRegion {
    //     mem_type: MEM_TYPE_RAM,
    //     physical_start: 0x200000,
    //     virtual_start: 0x200000,
    //     size: 0x80000000,
    // }, // ram
    // HvConfigMemoryRegion {
    //     mem_type: MEM_TYPE_IO,
    //     physical_start: 0xfe660000,
    //     virtual_start: 0xfe660000,
    //     size: 0x1000,
    // }, //uart
    // HvConfigMemoryRegion {
    //     mem_type: MEM_TYPE_IO,
    //     physical_start: 0xfd000000,
    //     virtual_start: 0xfd000000,
    //     size: 0x18e0000,
    // },
    // HvConfigMemoryRegion {
    //     mem_type: MEM_TYPE_IO,
    //     physical_start: 0x1f0000000,
    //     virtual_start: 0x1f0000000,
    //     size: 0x10000000,
    // },
    HvConfigMemoryRegion {
        mem_type: MEM_TYPE_RAM,
        physical_start: 0x0,
        virtual_start: 0x0,
        size: 0x7ffffff000,
    },
];

pub const ROOT_ZONE_IRQS: [u32; 1] = [
    0x76];

pub const ROOT_ARCH_ZONE_CONFIG: HvArchZoneConfig = HvArchZoneConfig {
    gicd_base: 0xfd400000,
    gicd_size: 0x10000,
    gicr_base: 0xfd460000,
    gicr_size: 0xc0000,
};
