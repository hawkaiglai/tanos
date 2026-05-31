pub mod multiboot;
pub mod uefi;

use crate::*;


pub const KERNEL_STACK_SIZE: usize = 64 * 1024; // 64KB
pub const KERNEL_STACK_BASE: usize = 0x200000;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: PhysAddr,
    pub end: PhysAddr,
    pub region_type: MemoryRegionType,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    Available = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    AcpiNvs = 4,
    BadRam = 5,
}

#[repr(C)]
#[derive(Debug)]
pub struct BootInfo {
    pub memory_map: &'static [MemoryRegion],
    pub kernel_start: PhysAddr,
    pub kernel_end: PhysAddr,
    pub initrd_start: Option<PhysAddr>,
    pub initrd_end: Option<PhysAddr>,
    pub framebuffer: Option<FramebufferInfo>,
    pub boot_params: BootParams,
}

#[repr(C)]
#[derive(Debug)]
pub struct FramebufferInfo {
    pub address: PhysAddr,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct BootParams {
    pub cmdline: Option<&'static str>,
    pub loader_name: Option<&'static str>,
}

/// Parse boot information based on the bootloader magic value.
///
/// SAFETY: `info_ptr` must point to a valid boot info structure, and the
/// magic must correspond to the actual format at that pointer.
pub unsafe fn parse_boot_info(magic: u64, info_ptr: u64) -> BootInfo {
    if magic as u32 == multiboot::MULTIBOOT2_INFO_MAGIC {
        multiboot::parse_multiboot2_info(info_ptr as *const u8)
    } else {
        // Unknown bootloader — fall back to UEFI parser or hardcoded
        uefi::parse_uefi_info(info_ptr as *const u8)
    }
}

extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
    static __kernel_size: u8;
}

pub fn kernel_physical_range() -> (PhysAddr, PhysAddr) {
    unsafe {
        let start = PhysAddr::new_unchecked(&__kernel_start as *const _ as u64);
        let end = PhysAddr::new_unchecked(&__kernel_end as *const _ as u64);
        (start, end)
    }
}
