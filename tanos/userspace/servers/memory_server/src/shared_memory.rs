//! Shared Memory Management for Memory Server
//!
//! Handles creation, mapping, and lifecycle of shared memory regions between processes.

use alloc::{collections::BTreeMap, vec::Vec};
use kernel_types::{ProcessId, VirtAddr, PhysAddr};

use crate::protocol::*;
use crate::lib_extensions::*;

pub struct SharedMemoryManager {
    shared_regions: BTreeMap<SharedMemoryId, SharedMemoryRegion>,
    next_id: u32,
    statistics: SharedMemoryStatistics,
}

#[derive(Debug)]
struct SharedMemoryRegion {
    id: SharedMemoryId,
    size: usize,
    flags: SharedMemoryFlags,
    owner: ProcessId,
    mapped_processes: Vec<ProcessMapping>,
    physical_pages: Vec<PhysAddr>,
    virtual_base: VirtAddr,
}

#[derive(Debug)]
struct ProcessMapping {
    process_id: ProcessId,
    virtual_address: VirtAddr,
    access_flags: MappingFlags,
}

#[derive(Debug, Default)]
struct SharedMemoryStatistics {
    total_shared_regions: usize,
    total_shared_memory: usize,
    active_mappings: usize,
    peak_shared_memory: usize,
}

pub struct SharedMemoryInfo {
    pub id: SharedMemoryId,
    pub size: usize,
    pub flags: SharedMemoryFlags,
    pub owner: ProcessId,
    pub mapping_count: usize,
}

impl SharedMemoryManager {
    pub fn new() -> Self {
        Self {
            shared_regions: BTreeMap::new(),
            next_id: 1,
            statistics: SharedMemoryStatistics::default(),
        }
    }

    pub fn create(&mut self, size: usize, flags: SharedMemoryFlags, owner: ProcessId) -> Result<SharedMemoryInfo> {
        if size == 0 || size > 1024 * 1024 * 1024 {
            return Err(Error::InvalidParameters);
        }

        let page_size = 4096;
        let aligned_size = (size + page_size - 1) & !(page_size - 1);
        let page_count = aligned_size / page_size;

        let physical_pages = self.allocate_shared_pages(page_count)?;
        let virtual_base = self.allocate_kernel_virtual_region();

        self.map_shared_pages(&virtual_base, &physical_pages)?;

        let id = SharedMemoryId(self.next_id);
        self.next_id += 1;

        let region = SharedMemoryRegion {
            id,
            size: aligned_size,
            flags,
            owner,
            mapped_processes: Vec::new(),
            physical_pages,
            virtual_base,
        };

        self.shared_regions.insert(id, region);

        self.statistics.total_shared_regions += 1;
        self.statistics.total_shared_memory += aligned_size;
        self.statistics.peak_shared_memory =
            self.statistics.peak_shared_memory.max(self.statistics.total_shared_memory);

        Ok(SharedMemoryInfo {
            id,
            size: aligned_size,
            flags,
            owner,
            mapping_count: 0,
        })
    }

    pub fn destroy(&mut self, id: SharedMemoryId, requester: ProcessId) -> Result<()> {
        let region = self.shared_regions.get(&id)
            .ok_or(Error::SharedMemoryNotFound)?;

        if region.owner != requester {
            return Err(Error::PermissionDenied);
        }

        let mapped_processes: Vec<ProcessId> = region.mapped_processes
            .iter()
            .map(|m| m.process_id)
            .collect();

        for process_id in mapped_processes {
            let _ = self.unmap_from_process(id, process_id);
        }

        if let Some(region) = self.shared_regions.remove(&id) {
            let _ = self.unmap_shared_pages(region.virtual_base, region.size);
            let _ = self.free_shared_pages(&region.physical_pages);

            self.statistics.total_shared_regions -= 1;
            self.statistics.total_shared_memory -= region.size;
        }

        Ok(())
    }

    pub fn map_to_process(&mut self, id: SharedMemoryId, process_id: ProcessId, access_flags: MappingFlags) -> Result<VirtAddr> {
        // Extract what we need before borrowing self mutably again
        let (size, physical_pages, flags) = {
            let region = self.shared_regions.get(&id)
                .ok_or(Error::SharedMemoryNotFound)?;

            if region.mapped_processes.iter().any(|m| m.process_id == process_id) {
                return Err(Error::AlreadyMapped);
            }

            if access_flags.contains(MappingFlags::WRITE) && !region.flags.contains(SharedMemoryFlags::WRITE) {
                return Err(Error::PermissionDenied);
            }

            (region.size, region.physical_pages.clone(), region.flags)
        };
        let _ = flags; // used for validation above

        let virtual_address = self.find_process_virtual_address(process_id, size)?;

        self.map_to_process_space(virtual_address, &physical_pages, access_flags)?;

        let region = self.shared_regions.get_mut(&id).unwrap();
        region.mapped_processes.push(ProcessMapping {
            process_id,
            virtual_address,
            access_flags,
        });

        self.statistics.active_mappings += 1;

        Ok(virtual_address)
    }

    pub fn unmap_from_process(&mut self, id: SharedMemoryId, process_id: ProcessId) -> Result<()> {
        // Extract mapping info before calling self methods
        let (virtual_address, size) = {
            let region = self.shared_regions.get_mut(&id)
                .ok_or(Error::SharedMemoryNotFound)?;

            let mapping_pos = region.mapped_processes
                .iter()
                .position(|m| m.process_id == process_id)
                .ok_or(Error::NotMapped)?;

            let mapping = region.mapped_processes.remove(mapping_pos);
            (mapping.virtual_address, region.size)
        };

        self.unmap_from_process_space(virtual_address, size)?;

        self.statistics.active_mappings -= 1;

        Ok(())
    }

    pub fn get_ptr(&self, id: SharedMemoryId) -> Option<*mut u8> {
        self.shared_regions.get(&id)
            .map(|region| region.virtual_base.as_mut_ptr())
    }

    pub fn _cleanup_process(&mut self, process_id: ProcessId) {
        let regions_to_cleanup: Vec<SharedMemoryId> = self.shared_regions
            .iter()
            .filter(|(_, region)| {
                region.owner == process_id ||
                region.mapped_processes.iter().any(|m| m.process_id == process_id)
            })
            .map(|(&id, _)| id)
            .collect();

        for id in regions_to_cleanup {
            if let Some(region) = self.shared_regions.get(&id) {
                if region.owner == process_id {
                    let _ = self.destroy(id, process_id);
                } else {
                    let _ = self.unmap_from_process(id, process_id);
                }
            }
        }
    }

    fn allocate_shared_pages(&self, page_count: usize) -> Result<Vec<PhysAddr>> {
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

    fn free_shared_pages(&self, pages: &[PhysAddr]) -> Result<()> {
        for &page in pages {
            let vaddr = VirtAddr::new_unchecked(page.as_u64());
            syscall::deallocate_memory(vaddr, 4096)?;
        }
        Ok(())
    }

    fn allocate_kernel_virtual_region(&self) -> VirtAddr {
        let base_addr = 0xFFFF800000000000u64 + (self.next_id as u64 * 0x100000);
        VirtAddr::new_unchecked(base_addr)
    }

    fn map_shared_pages(&self, virtual_base: &VirtAddr, physical_pages: &[PhysAddr]) -> Result<()> {
        let page_flags = PageFlags::PRESENT | PageFlags::WRITABLE | PageFlags::READABLE;
        for (i, &paddr) in physical_pages.iter().enumerate() {
            let vaddr = VirtAddr::new_unchecked(virtual_base.as_u64() + (i as u64 * 4096));
            syscall::map_memory(vaddr, paddr, page_flags)?;
        }
        Ok(())
    }

    fn unmap_shared_pages(&self, virtual_base: VirtAddr, size: usize) -> Result<()> {
        let page_count = (size + 4095) / 4096;
        for i in 0..page_count {
            let vaddr = VirtAddr::new_unchecked(virtual_base.as_u64() + (i as u64 * 4096));
            syscall::unmap_memory(vaddr)?;
        }
        Ok(())
    }

    fn find_process_virtual_address(&self, process_id: ProcessId, _size: usize) -> Result<VirtAddr> {
        let base = 0x200000000u64 + (process_id.as_u16() as u64 * 0x100000000);
        let offset = self.shared_regions.len() as u64 * 0x100000;
        Ok(VirtAddr::new_unchecked(base + offset))
    }

    fn map_to_process_space(&self, vaddr: VirtAddr, physical_pages: &[PhysAddr], flags: MappingFlags) -> Result<()> {
        let page_flags = self.mapping_flags_to_page_flags(flags);

        for (i, &paddr) in physical_pages.iter().enumerate() {
            let page_vaddr = VirtAddr::new_unchecked(vaddr.as_u64() + (i as u64 * 4096));
            syscall::map_memory(page_vaddr, paddr, page_flags)?;
        }
        Ok(())
    }

    fn unmap_from_process_space(&self, vaddr: VirtAddr, size: usize) -> Result<()> {
        let page_count = (size + 4095) / 4096;
        for i in 0..page_count {
            let page_vaddr = VirtAddr::new_unchecked(vaddr.as_u64() + (i as u64 * 4096));
            syscall::unmap_memory(page_vaddr)?;
        }
        Ok(())
    }

    fn mapping_flags_to_page_flags(&self, flags: MappingFlags) -> PageFlags {
        let mut page_flags = PageFlags::PRESENT | PageFlags::USER_ACCESSIBLE;

        if flags.contains(MappingFlags::WRITE) {
            page_flags |= PageFlags::WRITABLE;
        }
        if flags.contains(MappingFlags::EXECUTE) {
            page_flags |= PageFlags::EXECUTABLE;
        }

        page_flags
    }
}
