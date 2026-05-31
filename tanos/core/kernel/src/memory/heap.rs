extern crate alloc;

use super::*;
use alloc::alloc::{Layout};


pub struct KernelHeap {
    // allocator: LockedHeap, // TODO
    start: VirtAddr,
    size: usize,
}

impl KernelHeap {
    pub fn new(frames: Vec<Frame>) -> core::result::Result<Self, MemoryError> {
        if frames.is_empty() {
            return Err(MemoryError::OutOfMemory);
        }

        let size = frames.len() * crate::PAGE_SIZE;
        let start = VirtAddr::new_unchecked(crate::KERNEL_HEAP_BASE as u64);

        // TODO: Initialize a proper heap allocator (e.g. linked_list_allocator)
        // For now, just track the region

        Ok(Self {
            start,
            size,
        })
    }
    
    pub fn allocate(&self, _layout: Layout) -> *mut u8 {
        // TODO: Implement with a proper allocator
        core::ptr::null_mut()
    }

    pub fn deallocate(&self, _ptr: *mut u8, _layout: Layout) {
        // TODO: Implement with a proper allocator
    }
    
    pub fn usage(&self) -> HeapUsage {
        // The linked_list_allocator doesn't provide usage stats
        // This would need a custom allocator implementation
        HeapUsage {
            total: self.size,
            used: 0, // Not available with current allocator
            free: 0, // Not available with current allocator
        }
    }
}

pub struct HeapUsage {
    pub total: usize,
    pub used: usize,
    pub free: usize,
}

// Global heap allocation functions
pub fn allocate(layout: Layout) -> *mut u8 {
    super::with_memory_manager(|mm| {
        mm.heap_allocator.allocate(layout)
    })
}

pub fn deallocate(ptr: *mut u8, layout: Layout) {
    super::with_memory_manager(|mm| {
        mm.heap_allocator.deallocate(ptr, layout)
    });
}
