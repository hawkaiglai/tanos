//! Multiboot2 boot information parser
//!
//! GRUB passes a multiboot2 info structure (magic 0x36D76289) as a sequence
//! of tagged entries. This module walks the tag list and extracts the memory
//! map, command line, modules (initrd), and framebuffer info.

use super::*;
use core::slice;

/// Multiboot2 info magic value (passed in EAX by bootloader)
pub const MULTIBOOT2_INFO_MAGIC: u32 = 0x36D76289;

// Tag types
const TAG_END: u32 = 0;
const TAG_CMDLINE: u32 = 1;
const TAG_LOADER_NAME: u32 = 2;
const TAG_MODULE: u32 = 3;
const TAG_MEMORY_MAP: u32 = 6;
const TAG_FRAMEBUFFER: u32 = 8;

/// Multiboot2 info header (first 8 bytes)
#[repr(C)]
struct Multiboot2Info {
    total_size: u32,
    reserved: u32,
}

/// Generic tag header — all tags start with this
#[repr(C)]
struct TagHeader {
    tag_type: u32,
    size: u32,
}

/// Memory map tag
#[repr(C)]
struct MemoryMapTag {
    header: TagHeader,
    entry_size: u32,
    entry_version: u32,
    // Followed by variable number of MemoryMapEntry
}

/// Memory map entry
#[repr(C)]
#[derive(Clone, Copy)]
struct MemoryMapEntry {
    base_addr: u64,
    length: u64,
    entry_type: u32,
    reserved: u32,
}

/// Module tag (for initrd)
#[repr(C)]
struct ModuleTag {
    header: TagHeader,
    mod_start: u32,
    mod_end: u32,
    // Followed by null-terminated string
}

/// Command line tag
#[repr(C)]
struct CmdlineTag {
    header: TagHeader,
    // Followed by null-terminated string
}

/// Framebuffer tag
#[repr(C)]
struct FramebufferTag {
    header: TagHeader,
    framebuffer_addr: u64,
    framebuffer_pitch: u32,
    framebuffer_width: u32,
    framebuffer_height: u32,
    framebuffer_bpp: u8,
    framebuffer_type: u8,
    reserved: u16,
}

/// Parse multiboot2 boot information from the pointer passed by GRUB.
///
/// SAFETY: `multiboot_ptr` must point to a valid multiboot2 info structure
/// at a physical address that is identity-mapped.
pub unsafe fn parse_multiboot2_info(multiboot_ptr: *const u8) -> BootInfo {
    let info = &*(multiboot_ptr as *const Multiboot2Info);
    let total_size = info.total_size as usize;

    let mut memory_map: &'static [MemoryRegion] = &[];
    let mut cmdline: Option<&'static str> = None;
    let mut loader_name: Option<&'static str> = None;
    let mut initrd_start: Option<PhysAddr> = None;
    let mut initrd_end: Option<PhysAddr> = None;
    let mut framebuffer: Option<FramebufferInfo> = None;

    // Walk tags — they start 8 bytes after the info header
    let mut offset: usize = 8;

    while offset < total_size {
        // Tags are 8-byte aligned
        offset = (offset + 7) & !7;
        if offset >= total_size {
            break;
        }

        let tag = &*(multiboot_ptr.add(offset) as *const TagHeader);

        if tag.tag_type == TAG_END {
            break;
        }

        match tag.tag_type {
            TAG_MEMORY_MAP => {
                let mmap_tag = &*(multiboot_ptr.add(offset) as *const MemoryMapTag);
                let entry_size = mmap_tag.entry_size as usize;
                if entry_size > 0 {
                    let entries_start = offset + 16; // after header(8) + entry_size(4) + entry_version(4)
                    let entries_end = offset + tag.size as usize;
                    memory_map = parse_mmap_entries(
                        multiboot_ptr.add(entries_start),
                        entries_end - entries_start,
                        entry_size,
                    );
                }
            }
            TAG_CMDLINE => {
                let str_ptr = multiboot_ptr.add(offset + 8); // after header
                cmdline = Some(parse_c_string(str_ptr));
            }
            TAG_LOADER_NAME => {
                let str_ptr = multiboot_ptr.add(offset + 8);
                loader_name = Some(parse_c_string(str_ptr));
            }
            TAG_MODULE => {
                let module = &*(multiboot_ptr.add(offset) as *const ModuleTag);
                // Use first module as initrd
                if initrd_start.is_none() {
                    initrd_start = Some(PhysAddr::new_unchecked(module.mod_start as u64));
                    initrd_end = Some(PhysAddr::new_unchecked(module.mod_end as u64));
                }
            }
            TAG_FRAMEBUFFER => {
                let fb = &*(multiboot_ptr.add(offset) as *const FramebufferTag);
                framebuffer = Some(FramebufferInfo {
                    address: PhysAddr::new_unchecked(fb.framebuffer_addr),
                    width: fb.framebuffer_width,
                    height: fb.framebuffer_height,
                    pitch: fb.framebuffer_pitch,
                    bpp: fb.framebuffer_bpp,
                });
            }
            _ => {
                // Unknown tag — skip
            }
        }

        // Advance to next tag
        offset += tag.size as usize;
    }

    let (kernel_start, kernel_end) = crate::boot::kernel_physical_range();

    BootInfo {
        memory_map,
        kernel_start,
        kernel_end,
        initrd_start,
        initrd_end,
        framebuffer,
        boot_params: BootParams {
            cmdline,
            loader_name,
        },
    }
}

/// Parse memory map entries from the multiboot2 memory map tag.
unsafe fn parse_mmap_entries(
    entries_ptr: *const u8,
    total_bytes: usize,
    entry_size: usize,
) -> &'static [MemoryRegion] {
    let mut regions = alloc::vec::Vec::new();
    let mut offset = 0;

    while offset + core::mem::size_of::<MemoryMapEntry>() <= total_bytes {
        let entry = &*(entries_ptr.add(offset) as *const MemoryMapEntry);

        let region_type = match entry.entry_type {
            1 => MemoryRegionType::Available,
            2 => MemoryRegionType::Reserved,
            3 => MemoryRegionType::AcpiReclaimable,
            4 => MemoryRegionType::AcpiNvs,
            5 => MemoryRegionType::BadRam,
            _ => MemoryRegionType::Reserved,
        };

        regions.push(MemoryRegion {
            start: PhysAddr::new_unchecked(entry.base_addr),
            end: PhysAddr::new_unchecked(entry.base_addr + entry.length),
            region_type,
        });

        offset += entry_size;
    }

    regions.leak()
}

/// Parse a null-terminated C string into a static str.
unsafe fn parse_c_string(ptr: *const u8) -> &'static str {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
        // Safety limit
        if len > 4096 {
            break;
        }
    }

    let bytes = slice::from_raw_parts(ptr, len);
    core::str::from_utf8_unchecked(bytes)
}
