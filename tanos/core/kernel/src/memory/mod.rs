pub mod frame;
pub use frame::Frame;
pub use page::{Page, PageFlags};
pub mod page;

pub mod heap;
pub mod vmm;
// Re-export commonly used types

use crate::boot::BootInfo;
use crate::*;
use spin::{Mutex, Once};


use alloc::alloc::Layout;

static MEMORY_MANAGER: Once<Mutex<MemoryManager>> = Once::new();

pub struct MemoryManager {
    frame_allocator: frame::FrameAllocator,
    heap_allocator: heap::KernelHeap,
    vmm: vmm::VirtualMemoryManager,
}

impl MemoryManager {
    fn new(boot_info: &BootInfo) -> core::result::Result<Self, MemoryError> {
        // Initialize frame allocator
        let mut frame_allocator = frame::FrameAllocator::new(boot_info.memory_map)?;

        // Reserve kernel memory
        frame_allocator.reserve_range(boot_info.kernel_start, boot_info.kernel_end)?;

        // Reserve initrd if present
        if let (Some(start), Some(end)) = (boot_info.initrd_start, boot_info.initrd_end) {
            frame_allocator.reserve_range(start, end)?;
        }

        // Initialize kernel heap
        let heap_frames = frame_allocator.allocate_contiguous_frames(
            crate::KERNEL_HEAP_SIZE / crate::PAGE_SIZE
        )?;

        let heap_allocator = heap::KernelHeap::new(heap_frames)?;

        // Initialize virtual memory manager
        let vmm = vmm::VirtualMemoryManager::new(&mut frame_allocator)?;
        
        Ok(Self {
            frame_allocator,
            heap_allocator,
            vmm,
        })
    }
    
    pub fn allocate_frame(&mut self) -> Option<Frame> {
        self.frame_allocator.allocate()
    }
    
    pub fn deallocate_frame(&mut self, frame: Frame) {
        self.frame_allocator.deallocate(frame);
    }
    
    pub fn allocate_contiguous_frames(&mut self, count: usize) -> Option<Vec<Frame>> {
        self.frame_allocator.allocate_contiguous_frames(count).ok()
    }
    
    pub fn map_page(&mut self, address_space: &mut page::AddressSpace, page: Page, frame: Frame, flags: PageFlags) -> core::result::Result<(), MemoryError> {
        self.vmm.map_page(address_space, page, frame, flags)
    }
    
    pub fn unmap_page(&mut self, address_space: &mut page::AddressSpace, page: Page) -> core::result::Result<Frame, MemoryError> {
        self.vmm.unmap_page(address_space, page)
    }
}

pub fn init(boot_info: &BootInfo) {
    let memory_manager = MemoryManager::new(boot_info)
        .expect("Failed to initialize memory manager");

    MEMORY_MANAGER.call_once(|| Mutex::new(memory_manager));
}

pub fn with_memory_manager<F, R>(f: F) -> R 
where
    F: FnOnce(&mut MemoryManager) -> R,
{
    let memory_manager = MEMORY_MANAGER.get()
        .expect("Memory manager not initialized");
    
    f(&mut memory_manager.lock())
}

// Heap allocation interface
pub fn allocate(layout: Layout) -> *mut u8 {
    with_memory_manager(|mm| {
        mm.heap_allocator.allocate(layout)
    })
}

pub fn deallocate(ptr: *mut u8, layout: Layout) {
    with_memory_manager(|mm| {
        mm.heap_allocator.deallocate(ptr, layout)
    });
}

#[derive(Debug, Clone, Copy)]
pub enum MemoryError {
    OutOfMemory,
    InvalidAlignment,
    AddressNotMapped,
    AlreadyMapped,
    InvalidPermissions,
    FrameInUse,
}

/// Check if address range is readable by user processes
pub fn is_user_readable(addr: usize, size: usize) -> bool {
    // Basic check - address is in user space range
    addr >= USER_BASE && (addr + size) <= USER_STACK_TOP
}

/// Check if address range is writable by user processes
pub fn is_user_writable(addr: usize, size: usize) -> bool {
    // For now, same as readable (proper impl would check page permissions)
    is_user_readable(addr, size)
}

/// Handle page fault
pub fn handle_page_fault(addr: VirtAddr, error_code: u64) {
    crate::error!("Page fault at {:?}, error code: {:#x}", addr, error_code);
    // TODO: Implement proper page fault handling (demand paging, COW, etc.)
    panic!("Page fault handling not fully implemented");
}
