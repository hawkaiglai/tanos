use super::*;

// Simplified UEFI boot info parsing
// In a real implementation, this would parse UEFI memory map
pub unsafe fn parse_uefi_info(_uefi_ptr: *const u8) -> BootInfo {
    // For now, create a minimal boot info for UEFI
    // This would need proper UEFI protocol handling in a real implementation
    
    static DEFAULT_MEMORY_MAP: [MemoryRegion; 1] = [
        MemoryRegion {
            start: PhysAddr::new_unchecked(0x100000),
            end: PhysAddr::new_unchecked(0x40000000), // 1GB
            region_type: MemoryRegionType::Available,
        }
    ];
    let memory_map: &'static [MemoryRegion] = &DEFAULT_MEMORY_MAP;
    
    let (kernel_start, kernel_end) = crate::boot::kernel_physical_range();
    
    BootInfo {
        memory_map,
        kernel_start,
        kernel_end,
        initrd_start: None,
        initrd_end: None,
        framebuffer: None,
        boot_params: BootParams {
            cmdline: None,
            loader_name: Some("UEFI"),
        },
    }
}
