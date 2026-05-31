//! Address Space Management for Memory Server
//!
//! Manages virtual address spaces for processes and handles memory mapping operations.

use alloc::{collections::BTreeMap, vec::Vec};
use kernel_types::{ProcessId, VirtAddr, PhysAddr};

use crate::protocol::*;
use crate::lib_extensions::*;

pub struct AddressSpaceManager {
    process_spaces: BTreeMap<ProcessId, ProcessAddressSpace>,
    _next_allocation_id: u32,
}

struct ProcessAddressSpace {
    allocations: Vec<MappedAllocation>,
    total_mapped: usize,
    virtual_size: usize,
    next_vaddr: VirtAddr,
}

#[derive(Debug, Clone)]
struct MappedAllocation {
    id: AllocationId,
    vaddr: VirtAddr,
    size: usize,
    flags: MemoryFlags,
    physical_pages: Vec<PhysAddr>,
}

#[derive(Debug)]
pub struct AddressSpaceInfo {
    pub total_mapped: usize,
    pub virtual_size: usize,
    pub physical_used: usize,
}

#[derive(Debug)]
pub struct Allocation {
    pub id: AllocationId,
    pub size: usize,
    pub alignment: usize,
    pub flags: MemoryFlags,
    pub physical_pages: Vec<PhysAddr>,
}

impl AddressSpaceManager {
    pub fn new() -> Self {
        Self {
            process_spaces: BTreeMap::new(),
            _next_allocation_id: 1,
        }
    }

    pub fn map_allocation(&mut self, process_id: ProcessId, allocation: &Allocation) -> Result<VirtAddr> {
        let space = self.process_spaces.entry(process_id).or_insert_with(|| {
            ProcessAddressSpace::new(process_id)
        });

        // Find suitable virtual address
        let vaddr = Self::find_free_virtual_address(space, allocation.size, allocation.alignment)?;

        // Map pages using syscalls
        Self::map_pages(vaddr, &allocation.physical_pages, allocation.flags)?;

        // Track the allocation
        space.allocations.push(MappedAllocation {
            id: allocation.id,
            vaddr,
            size: allocation.size,
            flags: allocation.flags,
            physical_pages: allocation.physical_pages.clone(),
        });

        space.total_mapped += allocation.size;
        space.virtual_size = space.virtual_size.max(vaddr.as_u64() as usize + allocation.size);

        Ok(vaddr)
    }

    pub fn unmap_allocation(&mut self, process_id: ProcessId, allocation_id: AllocationId) -> Result<()> {
        let space = self.process_spaces.get_mut(&process_id)
            .ok_or(Error::ProcessNotFound)?;

        // Find and remove allocation
        let alloc_pos = space.allocations.iter()
            .position(|a| a.id == allocation_id)
            .ok_or(Error::AllocationNotFound)?;

        let allocation = space.allocations.remove(alloc_pos);

        // Unmap pages using syscalls
        Self::unmap_pages(allocation.vaddr, allocation.size)?;

        space.total_mapped -= allocation.size;

        Ok(())
    }

    pub fn protect_memory(&mut self, process_id: ProcessId, vaddr: VirtAddr, size: usize, flags: PageFlags) -> Result<()> {
        let space = self.process_spaces.get_mut(&process_id)
            .ok_or(Error::ProcessNotFound)?;

        // Find allocation containing this address
        let allocation = space.allocations.iter_mut()
            .find(|a| a.vaddr.as_u64() <= vaddr.as_u64() && vaddr.as_u64() < a.vaddr.as_u64() + a.size as u64)
            .ok_or(Error::AllocationNotFound)?;

        // Verify the protection doesn't extend beyond allocation
        if vaddr.as_u64() + size as u64 > allocation.vaddr.as_u64() + allocation.size as u64 {
            return Err(Error::InvalidParameters);
        }

        // Use syscall to change page protection
        syscall::protect_memory(vaddr, size, flags)?;

        // Update flags in our tracking
        allocation.flags = MemoryFlags::from_page_flags(flags);

        Ok(())
    }

    pub fn get_info(&self, process_id: ProcessId) -> AddressSpaceInfo {
        if let Some(space) = self.process_spaces.get(&process_id) {
            AddressSpaceInfo {
                total_mapped: space.total_mapped,
                virtual_size: space.virtual_size,
                physical_used: space.allocations.iter()
                    .map(|a| a.physical_pages.len() * 4096)
                    .sum(),
            }
        } else {
            AddressSpaceInfo {
                total_mapped: 0,
                virtual_size: 0,
                physical_used: 0,
            }
        }
    }

    pub fn cleanup_process(&mut self, process_id: ProcessId) {
        if let Some(space) = self.process_spaces.remove(&process_id) {
            for allocation in space.allocations {
                let _ = Self::unmap_pages(allocation.vaddr, allocation.size);
            }
        }
    }

    fn find_free_virtual_address(space: &ProcessAddressSpace, size: usize, alignment: usize) -> Result<VirtAddr> {
        // Align the candidate address
        let align_mask = alignment as u64 - 1;
        let aligned_addr = (space.next_vaddr.as_u64() + align_mask) & !align_mask;
        let mut candidate = VirtAddr::new_unchecked(aligned_addr);

        // Check for overlaps with existing allocations
        loop {
            let end_addr = candidate.as_u64() + size as u64;

            let overlaps = space.allocations.iter().any(|alloc| {
                let alloc_end = alloc.vaddr.as_u64() + alloc.size as u64;
                !(end_addr <= alloc.vaddr.as_u64() || candidate.as_u64() >= alloc_end)
            });

            if !overlaps {
                return Ok(candidate);
            }

            candidate = VirtAddr::new_unchecked(candidate.as_u64() + size as u64);

            if candidate.as_u64() > 0x0000800000000000 {
                return Err(Error::OutOfMemory);
            }
        }
    }

    fn map_pages(vaddr: VirtAddr, physical_pages: &[PhysAddr], flags: MemoryFlags) -> Result<()> {
        let page_flags = flags.to_page_flags();

        for (i, &paddr) in physical_pages.iter().enumerate() {
            let page_vaddr = VirtAddr::new_unchecked(vaddr.as_u64() + (i as u64 * 4096));
            syscall::map_memory(page_vaddr, paddr, page_flags)?;
        }

        Ok(())
    }

    fn unmap_pages(vaddr: VirtAddr, size: usize) -> Result<()> {
        let page_count = (size + 4095) / 4096;

        for i in 0..page_count {
            let page_vaddr = VirtAddr::new_unchecked(vaddr.as_u64() + (i as u64 * 4096));
            syscall::unmap_memory(page_vaddr)?;
        }

        Ok(())
    }
}

impl ProcessAddressSpace {
    fn new(process_id: ProcessId) -> Self {
        let base_vaddr = 0x10000000u64 + (process_id.as_u16() as u64 * 0x10000000);

        Self {
            allocations: Vec::new(),
            total_mapped: 0,
            virtual_size: 0,
            next_vaddr: VirtAddr::new_unchecked(base_vaddr),
        }
    }
}

impl MemoryFlags {
    fn to_page_flags(self) -> PageFlags {
        let mut flags = PageFlags::PRESENT | PageFlags::USER_ACCESSIBLE;

        if self.contains(MemoryFlags::WRITABLE) {
            flags |= PageFlags::WRITABLE;
        }
        if self.contains(MemoryFlags::EXECUTABLE) {
            flags |= PageFlags::EXECUTABLE;
        }
        if !self.contains(MemoryFlags::CACHED) {
            flags |= PageFlags::WRITE_THROUGH;
        }

        flags
    }

    pub fn from_page_flags(flags: PageFlags) -> Self {
        let mut mem_flags = MemoryFlags::READABLE;

        if flags.contains(PageFlags::WRITABLE) {
            mem_flags |= MemoryFlags::WRITABLE;
        }
        if flags.contains(PageFlags::EXECUTABLE) {
            mem_flags |= MemoryFlags::EXECUTABLE;
        }
        if !flags.contains(PageFlags::WRITE_THROUGH) {
            mem_flags |= MemoryFlags::CACHED;
        }

        mem_flags
    }
}
