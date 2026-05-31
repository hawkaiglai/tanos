#![no_std]
#![no_main]

#[macro_use]
extern crate libmicro;
extern crate alloc;

use alloc::{collections::BTreeMap, vec::Vec, string::String};
use core::mem::size_of;
use kernel_types::ProcessId;
use kernel_types::EndpointId;
use kernel_types::CapabilitySet;
use kernel_types::SharedMemoryId;
use spin::{Mutex, RwLock};

mod protocol;
mod elf_loader;
mod process_table;
mod lib_extensions;

use protocol::*;
use elf_loader::ElfLoader;
use process_table::ProcessTable;
use lib_extensions::*;

struct ProcessServer {
    endpoint: EndpointId,
    process_table: RwLock<ProcessTable>,
    next_pid: Mutex<u16>,
    waiting_processes: Mutex<BTreeMap<ProcessId, Vec<ProcessId>>>,
}

impl ProcessServer {
    fn new() -> Result<Self> {
        let endpoint = ipc::create_endpoint()?;

        // Register with system registry
        let registry = EndpointId::well_known(REGISTRY_SERVICE);
        let mut msg = Message::new(MessageType::Call);
        msg.set_label(RegistryOp::Register as u32);
        msg.set_data(0, SERVICE_PROCESS_MANAGER as u64);
        msg.set_data(1, endpoint.as_u64());

        ipc::call(registry, &msg)?;

        Ok(Self {
            endpoint,
            process_table: RwLock::new(ProcessTable::new()),
            next_pid: Mutex::new(100), // Start user PIDs at 100
            waiting_processes: Mutex::new(BTreeMap::new()),
        })
    }

    fn allocate_pid(&self) -> ProcessId {
        let mut next = self.next_pid.lock();
        let pid = ProcessId::new_const(*next);
        *next += 1;
        pid
    }

    fn handle_spawn(&self, msg: &Message) -> Message {
        let request = unsafe {
            &*(msg.get_data(0) as *const SpawnRequest)
        };

        let pid = self.allocate_pid();
        let parent_pid = ProcessId::new_const(msg.sender().as_u32() as u16);

        // Load ELF data from shared memory
        let elf_data_id = request.elf_data_id;
        let elf_data = match memory::map_shared_memory(elf_data_id) {
            Ok(data) => data,
            Err(e) => return Message::error(e),
        };

        // Create new address space
        let address_space = match memory::create_address_space() {
            Ok(as_id) => as_id,
            Err(e) => return Message::error(e),
        };

        // Load ELF into address space
        let mut loader = ElfLoader::new();
        let elf_slice = unsafe { core::slice::from_raw_parts(elf_data, request.elf_size) };
        let entry_point = match loader.load(elf_slice, address_space) {
            Ok(entry) => entry,
            Err(_) => {
                memory::destroy_address_space(address_space);
                return Message::error(Error::InvalidElf);
            }
        };

        // Parse argv and env
        let argv = self.parse_string_array(request.argv_shm_id, request.argc);
        let _env = self.parse_string_array(request.env_shm_id, 0);

        // Create process structure
        let process_info = ProcessInfo {
            pid,
            ppid: parent_pid,
            state: ProcessState::Ready,
            priority: Priority::Normal,
            cpu_time: 0,
            memory_usage: loader.memory_usage(),
            name: {
                let mut buf = [0u8; 32];
                if let Some(name) = argv.get(0) {
                    let len = name.len().min(31);
                    buf[..len].copy_from_slice(&name.as_bytes()[..len]);
                }
                buf
            },
        };

        // Add to process table
        {
            let mut table = self.process_table.write();
            table.insert(pid, process_info);
        }

        // Create the actual process in kernel
        let caps = CapabilitySet::new();
        match syscall::create_process(pid, entry_point, address_space, &caps) {
            Ok(()) => {
                let mut reply = Message::new(MessageType::Reply);
                reply.set_data(0, pid.as_u64());
                reply
            }
            Err(e) => {
                // Cleanup on failure
                let mut table = self.process_table.write();
                table.remove(pid);
                memory::destroy_address_space(address_space);
                Message::error(e)
            }
        }
    }

    fn handle_kill(&self, msg: &Message) -> Message {
        let target_pid = ProcessId::new_const(msg.get_data(0) as u16);
        let sender_pid = ProcessId::new_const(msg.sender().as_u32() as u16);

        // Check permissions
        if !self.can_kill(sender_pid, target_pid) {
            return Message::error(Error::PermissionDenied);
        }

        // Send kill signal to kernel
        match syscall::kill_process(target_pid) {
            Ok(()) => {
                // Update process table
                {
                    let mut table = self.process_table.write();
                    if let Some(process) = table.get_mut(target_pid) {
                        process.state = ProcessState::Zombie;
                    }
                }

                // Notify waiting processes
                self.notify_waiters(target_pid);

                Message::success()
            }
            Err(e) => Message::error(e),
        }
    }

    fn handle_wait(&self, msg: &Message) -> Message {
        let target_pid = ProcessId::new_const(msg.get_data(0) as u16);
        let waiter_pid = ProcessId::new_const(msg.sender().as_u32() as u16);

        // Check if target exists and is child
        {
            let table = self.process_table.read();
            if let Some(process) = table.get(target_pid) {
                if process.ppid != waiter_pid {
                    return Message::error(Error::PermissionDenied);
                }

                if process.state == ProcessState::Zombie {
                    // Process already dead, return immediately
                    let mut reply = Message::new(MessageType::Reply);
                    reply.set_data(0, target_pid.as_u64());
                    reply.set_data(1, 0); // Exit code TODO: track exit codes
                    return reply;
                }
            } else {
                return Message::error(Error::ProcessNotFound);
            }
        }

        // Add to waiters list
        {
            let mut waiters = self.waiting_processes.lock();
            waiters.entry(target_pid).or_default().push(waiter_pid);
        }

        // This will be replied to when process dies
        Message::async_pending()
    }

    fn handle_get_info(&self, msg: &Message) -> Message {
        let target_pid = ProcessId::new_const(msg.get_data(0) as u16);

        let table = self.process_table.read();
        if let Some(process) = table.get(target_pid) {
            let mut reply = Message::new(MessageType::Reply);

            // Copy process info to shared memory
            let shm_id = match memory::create_shared_memory(size_of::<ProcessInfo>()) {
                Ok(id) => id,
                Err(e) => return Message::error(e),
            };
            let shm_ptr = match memory::map_shared_memory(shm_id) {
                Ok(ptr) => ptr,
                Err(e) => return Message::error(e),
            };

            unsafe {
                *(shm_ptr as *mut ProcessInfo) = *process;
            }

            reply.set_data(0, shm_id);
            reply
        } else {
            Message::error(Error::ProcessNotFound)
        }
    }

    fn handle_list_processes(&self, _msg: &Message) -> Message {
        let table = self.process_table.read();
        let process_count = table.len();

        // Create shared memory for process list
        let shm_size = process_count * size_of::<ProcessInfo>();
        let shm_id = match memory::create_shared_memory(shm_size) {
            Ok(id) => id,
            Err(e) => return Message::error(e),
        };
        let shm_ptr = match memory::map_shared_memory(shm_id) {
            Ok(ptr) => ptr,
            Err(e) => return Message::error(e),
        };

        // Copy all processes
        let mut offset = 0;
        for (_, process) in table.iter() {
            unsafe {
                let dst = (shm_ptr as *mut u8).add(offset) as *mut ProcessInfo;
                *dst = *process;
            }
            offset += size_of::<ProcessInfo>();
        }

        let mut reply = Message::new(MessageType::Reply);
        reply.set_data(0, shm_id);
        reply.set_data(1, process_count as u64);
        reply
    }

    fn can_kill(&self, killer: ProcessId, target: ProcessId) -> bool {
        // Root can kill anyone
        if killer.as_u16() < 10 {
            return true;
        }

        let table = self.process_table.read();

        // Can kill own children
        if let Some(target_process) = table.get(target) {
            if target_process.ppid == killer {
                return true;
            }
        }

        // Can kill self
        killer == target
    }

    fn notify_waiters(&self, dead_pid: ProcessId) {
        let mut waiters = self.waiting_processes.lock();
        if let Some(waiting_list) = waiters.remove(&dead_pid) {
            for waiter in waiting_list {
                let mut reply = Message::new(MessageType::Reply);
                reply.set_data(0, dead_pid.as_u64());
                reply.set_data(1, 0); // Exit code

                // Send reply to waiting process
                let _ = ipc::reply_to_process(waiter, &reply);
            }
        }
    }

    fn parse_string_array(&self, shm_id: u64, count: usize) -> Vec<String> {
        if shm_id == 0 {
            return Vec::new();
        }

        let mut result = Vec::new();
        let shm_ptr = match memory::map_shared_memory(shm_id) {
            Ok(ptr) => ptr,
            Err(_) => return Vec::new(),
        };

        unsafe {
            let ptr_array = shm_ptr as *const *const u8;
            for i in 0..count {
                let str_ptr = *ptr_array.add(i);
                if !str_ptr.is_null() {
                    let mut len = 0;
                    while *str_ptr.add(len) != 0 {
                        len += 1;
                    }

                    let slice = core::slice::from_raw_parts(str_ptr, len);
                    if let Ok(string) = core::str::from_utf8(slice) {
                        result.push(String::from(string));
                    }
                }
            }
        }

        result
    }

    fn run(&self) -> ! {
        debug_println!("Process server started");

        loop {
            let mut msg = Message::new(MessageType::Receive);

            match ipc::receive(self.endpoint, &mut msg) {
                Ok(()) => {
                    let reply = match ProcessOp::from_u32(msg.label()) {
                        Some(ProcessOp::Spawn) => self.handle_spawn(&msg),
                        Some(ProcessOp::Kill) => self.handle_kill(&msg),
                        Some(ProcessOp::Wait) => self.handle_wait(&msg),
                        Some(ProcessOp::GetInfo) => self.handle_get_info(&msg),
                        Some(ProcessOp::ListProcesses) => self.handle_list_processes(&msg),
                        _ => Message::error(Error::InvalidOperation),
                    };

                    if !reply.is_async_pending() {
                        let _ = ipc::reply(msg.sender(), &reply);
                    }
                }
                Err(e) => {
                    debug_println!("Process server receive error: {:?}", e);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let _ = libmicro::init();

    let server = match ProcessServer::new() {
        Ok(s) => s,
        Err(_) => {
            debug_println!("Failed to create process server");
            libmicro::syscall::exit(-1);
        }
    };
    server.run()
}
