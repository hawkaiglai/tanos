//! Memory Server for TanOS
//!
//! Manages memory allocation, shared memory, and address spaces for all processes.

#![no_std]
#![no_main]

#[macro_use]
extern crate libmicro;
extern crate alloc;

use alloc::{collections::BTreeMap, vec::Vec};
use kernel_types::{ProcessId, EndpointId, VirtAddr};
use spin::{Mutex, RwLock};

mod protocol;
mod address_space;
mod shared_memory;
mod heap_allocator;
mod lib_extensions;

use protocol::*;
use address_space::AddressSpaceManager;
use shared_memory::SharedMemoryManager;
use heap_allocator::HeapManager;
use lib_extensions::*;

struct MemoryServer {
    endpoint: EndpointId,
    address_spaces: RwLock<AddressSpaceManager>,
    shared_memory: RwLock<SharedMemoryManager>,
    heap_manager: RwLock<HeapManager>,
    allocations: Mutex<BTreeMap<ProcessId, Vec<AllocationInfo>>>,
}

impl MemoryServer {
    fn new() -> Result<Self> {
        let endpoint = ipc::create_endpoint()?;

        // Register with system registry
        let registry = EndpointId::well_known(REGISTRY_SERVICE);
        let mut msg = Message::new(libmicro::MessageType::Call);
        msg.set_label(RegistryOp::Register as u32);
        msg.set_data(0, SERVICE_MEMORY_MANAGER as u64);
        msg.set_data(1, endpoint.as_u64());

        ipc::call(registry, &msg)?;

        Ok(Self {
            endpoint,
            address_spaces: RwLock::new(AddressSpaceManager::new()),
            shared_memory: RwLock::new(SharedMemoryManager::new()),
            heap_manager: RwLock::new(HeapManager::new()),
            allocations: Mutex::new(BTreeMap::new()),
        })
    }

    fn handle_allocate_memory(&self, msg: &Message) -> Message {
        let size = msg.get_data(0) as usize;
        let alignment = msg.get_data(1) as usize;
        let flags = MemoryFlags::from_bits_truncate(msg.get_data(2) as u32);
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        if size == 0 || !alignment.is_power_of_two() {
            return Message::error(Error::InvalidParameters);
        }

        // Allocate memory region
        let allocation = {
            let mut heap = self.heap_manager.write();
            match heap.allocate(size, alignment, flags) {
                Ok(alloc) => alloc,
                Err(e) => return Message::error(e),
            }
        };

        // Map into process address space
        let vaddr = {
            let mut addr_spaces = self.address_spaces.write();
            match addr_spaces.map_allocation(process_id, &allocation) {
                Ok(addr) => addr,
                Err(e) => {
                    let mut heap = self.heap_manager.write();
                    heap.deallocate(allocation.id);
                    return Message::error(e);
                }
            }
        };

        // Track allocation for cleanup
        {
            let mut allocations = self.allocations.lock();
            allocations.entry(process_id).or_default().push(AllocationInfo {
                id: allocation.id,
                vaddr,
                size,
                flags,
            });
        }

        let mut reply = Message::new(libmicro::MessageType::Reply);
        reply.set_data(0, vaddr.as_u64());
        reply.set_data(1, allocation.id.as_u64());
        reply
    }

    fn handle_free_memory(&self, msg: &Message) -> Message {
        let alloc_id = AllocationId(msg.get_data(0) as u32);
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        // Find allocation
        let allocation_info = {
            let mut allocations = self.allocations.lock();
            let process_allocs = allocations.get_mut(&process_id);

            if let Some(allocs) = process_allocs {
                if let Some(pos) = allocs.iter().position(|a| a.id == alloc_id) {
                    allocs.remove(pos)
                } else {
                    return Message::error(Error::AllocationNotFound);
                }
            } else {
                return Message::error(Error::AllocationNotFound);
            }
        };

        // Unmap from address space
        {
            let mut addr_spaces = self.address_spaces.write();
            if let Err(e) = addr_spaces.unmap_allocation(process_id, allocation_info.id) {
                return Message::error(e);
            }
        }

        // Free the allocation
        {
            let mut heap = self.heap_manager.write();
            heap.deallocate(alloc_id);
        }

        Message::success()
    }

    fn handle_create_shared_memory(&self, msg: &Message) -> Message {
        let size = msg.get_data(0) as usize;
        let flags = SharedMemoryFlags::from_bits_truncate(msg.get_data(1) as u32);
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        if size == 0 {
            return Message::error(Error::InvalidParameters);
        }

        let shared_mem = {
            let mut shm_mgr = self.shared_memory.write();
            match shm_mgr.create(size, flags, process_id) {
                Ok(shm) => shm,
                Err(e) => return Message::error(e),
            }
        };

        let mut reply = Message::new(libmicro::MessageType::Reply);
        reply.set_data(0, shared_mem.id.as_u64());
        reply
    }

    fn handle_map_shared_memory(&self, msg: &Message) -> Message {
        let shm_id = SharedMemoryId(msg.get_data(0) as u32);
        let flags = MappingFlags::from_bits_truncate(msg.get_data(1) as u32);
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        let vaddr = {
            let mut shm_mgr = self.shared_memory.write();
            match shm_mgr.map_to_process(shm_id, process_id, flags) {
                Ok(addr) => addr,
                Err(e) => return Message::error(e),
            }
        };

        let mut reply = Message::new(libmicro::MessageType::Reply);
        reply.set_data(0, vaddr.as_u64());
        reply
    }

    fn handle_unmap_shared_memory(&self, msg: &Message) -> Message {
        let _vaddr = VirtAddr::new_unchecked(msg.get_data(0));
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);
        let shm_id = SharedMemoryId(msg.get_data(1) as u32);

        let mut shm_mgr = self.shared_memory.write();
        match shm_mgr.unmap_from_process(shm_id, process_id) {
            Ok(()) => Message::success(),
            Err(e) => Message::error(e),
        }
    }

    fn handle_protect_memory(&self, msg: &Message) -> Message {
        let vaddr = VirtAddr::new_unchecked(msg.get_data(0));
        let size = msg.get_data(1) as usize;
        let new_flags = PageFlags::from_bits_truncate(msg.get_data(2));
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        let mut addr_spaces = self.address_spaces.write();
        match addr_spaces.protect_memory(process_id, vaddr, size, new_flags) {
            Ok(()) => Message::success(),
            Err(e) => Message::error(e),
        }
    }

    fn handle_get_memory_info(&self, msg: &Message) -> Message {
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        let info = {
            let allocations = self.allocations.lock();
            let addr_spaces = self.address_spaces.read();

            let process_allocs = allocations.get(&process_id).map(|v| v.len()).unwrap_or(0);
            let address_space_info = addr_spaces.get_info(process_id);

            MemoryInfo {
                total_allocated: address_space_info.total_mapped,
                allocation_count: process_allocs,
                virtual_size: address_space_info.virtual_size,
                physical_used: address_space_info.physical_used,
            }
        };

        let mut reply = Message::new(libmicro::MessageType::Reply);
        reply.set_data(0, info.total_allocated as u64);
        reply.set_data(1, info.allocation_count as u64);
        reply.set_data(2, info.virtual_size as u64);
        reply.set_data(3, info.physical_used as u64);
        reply
    }

    fn _cleanup_process(&self, process_id: ProcessId) {
        // Free all allocations for the process
        let allocations = {
            let mut allocs = self.allocations.lock();
            allocs.remove(&process_id).unwrap_or_default()
        };

        for alloc in allocations {
            let mut heap = self.heap_manager.write();
            heap.deallocate(alloc.id);
        }

        // Cleanup address space
        let mut addr_spaces = self.address_spaces.write();
        addr_spaces.cleanup_process(process_id);
    }

    fn run(&self) -> ! {
        debug_println!("Memory server started");

        loop {
            let mut msg = Message::new(libmicro::MessageType::Receive);

            match ipc::receive(self.endpoint, &mut msg) {
                Ok(()) => {
                    let reply = match MemoryOp::from_u32(msg.label()) {
                        Some(MemoryOp::AllocateMemory) => self.handle_allocate_memory(&msg),
                        Some(MemoryOp::FreeMemory) => self.handle_free_memory(&msg),
                        Some(MemoryOp::CreateSharedMemory) => self.handle_create_shared_memory(&msg),
                        Some(MemoryOp::MapSharedMemory) => self.handle_map_shared_memory(&msg),
                        Some(MemoryOp::UnmapSharedMemory) => self.handle_unmap_shared_memory(&msg),
                        Some(MemoryOp::ProtectMemory) => self.handle_protect_memory(&msg),
                        Some(MemoryOp::GetMemoryInfo) => self.handle_get_memory_info(&msg),
                        None => Message::error(Error::InvalidOperation),
                    };

                    let _ = ipc::reply(msg.sender(), &reply);
                }
                Err(e) => {
                    debug_println!("Memory server receive error: {:?}", e);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let _ = libmicro::init();

    let server = match MemoryServer::new() {
        Ok(s) => s,
        Err(e) => {
            debug_println!("Failed to create memory server: {:?}", e);
            libmicro::syscall::exit(-1);
        }
    };
    server.run()
}
