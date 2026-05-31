extern crate alloc;
use crate::PageFlags;
use super::*;
use crate::*;

use alloc::collections::BTreeMap;
use alloc::boxed::Box;

/// Virtual Memory Manager handles page tables and virtual address spaces
pub struct VirtualMemoryManager {
    /// Current kernel page table (CR3 on x86_64).
    /// Boxed so the 4KB page table lives on the heap rather than inline:
    /// an inline `PageTable` (align 4096) bloats every containing struct to a
    /// multiple of 4KB, and moving such a struct onto/through the small kernel
    /// stack overflows it and corrupts adjacent statics.
    kernel_page_table: Box<PageTable>,
    /// Page table frame allocations
    page_table_frames: BTreeMap<PhysAddr, usize>, // frame -> ref_count
}

/// Page Table Entry for x86_64
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    const PRESENT: u64 = 1 << 0;
    const WRITABLE: u64 = 1 << 1;
    const USER_ACCESSIBLE: u64 = 1 << 2;
    const WRITE_THROUGH: u64 = 1 << 3;
    const NO_CACHE: u64 = 1 << 4;
    const ACCESSED: u64 = 1 << 5;
    const DIRTY: u64 = 1 << 6;
    const HUGE_PAGE: u64 = 1 << 7;
    const NO_EXECUTE: u64 = 1 << 63;
    
    const ADDRESS_MASK: u64 = 0x000F_FFFF_FFFF_F000;
    
    pub fn new() -> Self {
        Self(0)
    }
    
    pub fn is_present(self) -> bool {
        self.0 & Self::PRESENT != 0
    }
    
    pub fn set_frame(&mut self, frame: Frame, flags: PageFlags) {
        self.0 = frame.start_address().as_u64() & Self::ADDRESS_MASK;
        self.0 |= flags.bits();
    }
    
    pub fn get_frame(self) -> Option<Frame> {
        if self.is_present() {
            Some(Frame::from_address(PhysAddr::new_unchecked(self.0 & Self::ADDRESS_MASK)))
        } else {
            None
        }
    }
    
    pub fn set_flags(&mut self, flags: PageFlags) {
        self.0 = (self.0 & Self::ADDRESS_MASK) | flags.bits();
    }
}

/// Page Table with 512 entries (x86_64)
#[derive(Debug)]
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    pub fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); 512],
        }
    }
    
    pub fn zero(&mut self) {
        for entry in &mut self.entries {
            entry.0 = 0;
        }
    }
    
    pub fn get_entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }
    
    pub fn get_entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }
}

impl VirtualMemoryManager {
    pub fn new(frame_allocator: &mut super::frame::FrameAllocator) -> core::result::Result<Self, MemoryError> {
        // Allocate frame for kernel page table
        let kernel_pt_frame = frame_allocator.allocate()
            .ok_or(MemoryError::OutOfMemory)?;
        
        let mut kernel_page_table = Box::new(PageTable::new());
        kernel_page_table.zero();
        
        // Set up direct mapping for kernel space (higher half)
        // Map kernel at 0xFFFF800000000000
        let mut page_table_frames = BTreeMap::new();
        page_table_frames.insert(kernel_pt_frame.start_address(), 1);
        
        Ok(Self {
            kernel_page_table,
            page_table_frames,
        })
    }
    
    pub fn map_page(
        &mut self,
        address_space: &mut super::page::AddressSpace,
        page: super::page::Page,
        frame: super::frame::Frame,
        flags: super::page::PageFlags,
    ) -> core::result::Result<(), MemoryError> {
        address_space.map_page(page, frame, flags)
    }
    
    pub fn unmap_page(
        &mut self,
        address_space: &mut super::page::AddressSpace,
        page: super::page::Page,
    ) -> core::result::Result<super::frame::Frame, MemoryError> {
        address_space.unmap_page(page)
    }
    
    /// Create a new address space for a process
    pub fn create_address_space(&mut self, frame_allocator: &mut super::frame::FrameAllocator) -> core::result::Result<super::page::AddressSpace, MemoryError> {
        // Allocate frame for page table root
        let root_frame = frame_allocator.allocate()
            .ok_or(MemoryError::OutOfMemory)?;
        
        // Initialize page table
        let page_table_ptr = root_frame.start_address().as_u64() as *mut PageTable;
        // SAFETY: root_frame was just allocated (unaliased) and lives in
        // identity-mapped low RAM, so its physical address is a valid, uniquely
        // owned, suitably-aligned pointer to a PageTable-sized region we may zero.
        unsafe {
            let page_table = &mut *page_table_ptr;
            page_table.zero();
        }
        
        self.page_table_frames.insert(root_frame.start_address(), 1);
        
        super::page::AddressSpace::new()
    }
    
    /// Switch to a different address space
    pub fn switch_address_space(&mut self, address_space: &super::page::AddressSpace) {
        let root_frame = Frame::from_address(address_space.cr3());
        
        // Switch CR3 on x86_64
        // SAFETY: `root_frame` is the PML4 of a fully-constructed address space
        // that identity-maps the kernel (code, data, stack, heap) in its low
        // region, so the kernel keeps executing correctly across the CR3 load.
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!(
                "mov cr3, {}",
                in(reg) root_frame.start_address().as_u64(),
                options(nomem, nostack, preserves_flags)
            );
        }
    }
    
    /// Get current page table physical address
    pub fn current_page_table_addr(&self) -> PhysAddr {
        // SAFETY: reading CR3 has no memory or flag side effects; it just copies
        // the current page-table base into a register.
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let addr: u64;
            core::arch::asm!("mov {}, cr3", out(reg) addr, options(nomem, nostack, preserves_flags));
            PhysAddr::new_unchecked(addr)
        }
        
        #[cfg(not(target_arch = "x86_64"))]
        PhysAddr::new_unchecked(0) // Placeholder for other architectures
    }
    
    /// Handle page fault
    pub fn handle_page_fault(
        &mut self,
        _fault_addr: VirtAddr,
        error_code: PageFaultErrorCode,
        _current_process: ProcessId,
    ) -> core::result::Result<(), MemoryError> {
        // Check if this is a valid fault that can be handled
        if error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
            // Protection violation - check capabilities
            return Err(MemoryError::InvalidPermissions);
        }
        
        if error_code.contains(PageFaultErrorCode::PAGE_NOT_PRESENT) {
            // Page not present - might be swapped or demand-allocated
            // For now, just fail
            return Err(MemoryError::AddressNotMapped);
        }
        
        Ok(())
    }
    
    /// Flush TLB entries
    pub fn flush_tlb(&self, page: Option<super::page::Page>) {
        // SAFETY: invlpg and reloading CR3 only invalidate cached TLB
        // translations; they have no effect on Rust-visible memory state and are
        // always safe to issue in ring 0.
        #[cfg(target_arch = "x86_64")]
        unsafe {
            if let Some(page) = page {
                // Flush specific page
                core::arch::asm!(
                    "invlpg [{}]",
                    in(reg) page.start_address().as_u64(),
                    options(nomem, nostack, preserves_flags)
                );
            } else {
                // Flush entire TLB
                let cr3: u64;
                core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
                core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nomem, nostack, preserves_flags));
            }
        }
    }
}

bitflags::bitflags! {
    pub struct PageFaultErrorCode: u32 {
        const PAGE_NOT_PRESENT = 1 << 0;
        const WRITE_ACCESS = 1 << 1;
        const USER_MODE = 1 << 2;
        const RESERVED_WRITE = 1 << 3;
        const INSTRUCTION_FETCH = 1 << 4;
        const PROTECTION_VIOLATION = 1 << 5;
    }
}

impl PageFaultErrorCode {
    pub fn from_u32(value: u32) -> Self {
        Self::from_bits_truncate(value)
    }
}
