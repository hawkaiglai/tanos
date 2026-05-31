//! Heap Allocation Management for Memory Server
//!
//! Manages physical memory allocation and provides heap services to processes.

use alloc::{collections::BTreeMap, vec::Vec};
use kernel_types::{ProcessId, VirtAddr, PhysAddr};

use crate::protocol::*;
use crate::address_space::Allocation;
use crate::lib_extensions::*;

pub struct HeapManager {
    allocations: BTreeMap<AllocationId, HeapAllocation>,
    next_id: u32,
    statistics: HeapStatistics,
}

#[derive(Debug, Clone)]
struct HeapAllocation {
    id: AllocationId,
    size: usize,
    alignment: usize,
    flags: MemoryFlags,
    physical_pages: Vec<PhysAddr>,
    owner: ProcessId,
}

#[derive(Debug, Default)]
struct HeapStatistics {
    total_allocated: usize,
    allocation_count: usize,
    peak_allocation: usize,
}

impl HeapManager {
    pub fn new() -> Self {
        Self {
            allocations: BTreeMap::new(),
            next_id: 1,
            statistics: HeapStatistics::default(),
        }
    }

    pub fn allocate(&mut self, size: usize, alignment: usize, flags: MemoryFlags) -> Result<Allocation> {
        if size == 0 || !alignment.is_power_of_two() || size > 1024 * 1024 * 1024 {
            return Err(Error::InvalidParameters);
        }

        let page_size = 4096;
        let aligned_size = (size + page_size - 1) & !(page_size - 1);
        let page_count = aligned_size / page_size;

        let physical_pages = self.allocate_physical_pages(page_count)?;

        let allocation_id = AllocationId(self.next_id);
        self.next_id += 1;

        let allocation = Allocation {
            id: allocation_id,
            size: aligned_size,
            alignment,
            flags,
            physical_pages: physical_pages.clone(),
        };

        let heap_allocation = HeapAllocation {
            id: allocation_id,
            size: aligned_size,
            alignment,
            flags,
            physical_pages,
            owner: current_process_id(),
        };

        self.allocations.insert(allocation_id, heap_allocation);

        self.statistics.total_allocated += aligned_size;
        self.statistics.allocation_count += 1;
        self.statistics.peak_allocation = self.statistics.peak_allocation.max(self.statistics.total_allocated);

        Ok(allocation)
    }

    pub fn deallocate(&mut self, allocation_id: AllocationId) {
        if let Some(allocation) = self.allocations.remove(&allocation_id) {
            let _ = self.free_physical_pages(&allocation.physical_pages);
            self.statistics.total_allocated -= allocation.size;
            if self.statistics.allocation_count > 0 {
                self.statistics.allocation_count -= 1;
            }
        }
    }

    pub fn _cleanup_process(&mut self, process_id: ProcessId) {
        let allocations_to_free: Vec<AllocationId> = self.allocations
            .iter()
            .filter(|(_, alloc)| alloc.owner == process_id)
            .map(|(&id, _)| id)
            .collect();

        for allocation_id in allocations_to_free {
            self.deallocate(allocation_id);
        }
    }

    fn allocate_physical_pages(&self, page_count: usize) -> Result<Vec<PhysAddr>> {
        let mut pages = Vec::with_capacity(page_count);

        for _ in 0..page_count {
            match syscall::allocate_memory(4096, MemoryFlags::READABLE.bits() as u64 | MemoryFlags::WRITABLE.bits() as u64) {
                Ok(vaddr) => {
                    let paddr = PhysAddr::new_unchecked(vaddr.as_u64());
                    pages.push(paddr);
                }
                Err(_) => {
                    for page in pages {
                        let _ = syscall::deallocate_memory(VirtAddr::new_unchecked(page.as_u64()), 4096);
                    }
                    return Err(Error::OutOfMemory);
                }
            }
        }

        Ok(pages)
    }

    fn free_physical_pages(&self, pages: &[PhysAddr]) -> Result<()> {
        for &page in pages {
            let vaddr = VirtAddr::new_unchecked(page.as_u64());
            syscall::deallocate_memory(vaddr, 4096)?;
        }
        Ok(())
    }
}
