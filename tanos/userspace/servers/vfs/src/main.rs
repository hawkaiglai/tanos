//! VFS (Virtual File System) Server for TanOS
//!
//! Manages file system operations, mount points, and file descriptors.

#![no_std]
#![no_main]

#[macro_use]
extern crate libmicro;
extern crate alloc;

use alloc::{collections::BTreeMap, string::String, sync::Arc};
use kernel_types::{ProcessId, EndpointId};
use spin::{Mutex, RwLock};
use core::mem::size_of;

mod protocol;
mod filesystem;
mod file_table;
mod path;
mod initrd;
mod lib_extensions;

use protocol::*;
use filesystem::FileSystem;
use file_table::{FileTable, FileHandle};
use path::Path;
use initrd::InitrdFS;
use lib_extensions::*;

struct VfsServer {
    endpoint: EndpointId,
    file_table: RwLock<FileTable>,
    mount_table: RwLock<BTreeMap<String, Arc<dyn FileSystem>>>,
    root_fs: Arc<dyn FileSystem>,
    next_fd: Mutex<u32>,
}

impl VfsServer {
    fn new() -> Result<Self> {
        let endpoint = ipc::create_endpoint()?;

        // Register with system registry
        let registry = EndpointId::well_known(REGISTRY_SERVICE);
        let mut msg = Message::new(libmicro::MessageType::Call);
        msg.set_label(RegistryOp::Register as u32);
        msg.set_data(0, SERVICE_VFS as u64);
        msg.set_data(1, endpoint.as_u64());

        ipc::call(registry, &msg)?;

        // Initialize root filesystem (initrd)
        let root_fs: Arc<dyn FileSystem> = Arc::new(InitrdFS::new()?);
        let mut mount_table_inner = BTreeMap::new();
        mount_table_inner.insert(String::from("/"), root_fs.clone());

        Ok(Self {
            endpoint,
            file_table: RwLock::new(FileTable::new()),
            mount_table: RwLock::new(mount_table_inner),
            root_fs,
            next_fd: Mutex::new(3), // 0,1,2 reserved for stdin/stdout/stderr
        })
    }

    fn allocate_fd(&self) -> u32 {
        let mut next = self.next_fd.lock();
        let fd = *next;
        *next += 1;
        fd
    }

    fn resolve_path<'a>(&'a self, path_str: &str) -> Result<(Arc<dyn FileSystem>, String)> {
        let _path = Path::new(path_str)?;
        let mounts = self.mount_table.read();

        let mut best_match_len = 0;
        let mut best_fs = &self.root_fs;

        for (mount_point, fs) in mounts.iter() {
            if path_str.starts_with(mount_point.as_str()) && mount_point.len() > best_match_len {
                best_match_len = mount_point.len();
                best_fs = fs;
            }
        }

        let relative_path = if best_match_len <= 1 {
            String::from(path_str)
        } else {
            String::from(&path_str[best_match_len..])
        };

        Ok((best_fs.clone(), relative_path))
    }

    fn handle_open(&self, msg: &Message) -> Message {
        let path_data = msg.get_data(0) as *const u8;
        let flags = OpenFlags::from_bits_truncate(msg.get_data(1) as u32);
        let mode = msg.get_data(2) as u32;
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        let path_str = unsafe {
            let mut len = 0;
            while *path_data.add(len) != 0 && len < 256 {
                len += 1;
            }
            let slice = core::slice::from_raw_parts(path_data, len);
            match core::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => return Message::error(Error::InvalidPath),
            }
        };

        let (fs, relative_path) = match self.resolve_path(path_str) {
            Ok((fs, path)) => (fs, path),
            Err(e) => return Message::error(e),
        };

        let vnode = match fs.open(&relative_path, flags, mode) {
            Ok(vnode) => vnode,
            Err(e) => return Message::error(e),
        };

        let fd = self.allocate_fd();

        {
            let mut file_table = self.file_table.write();
            let file_handle = FileHandle {
                fd,
                flags,
                position: 0,
                vnode,
                process_id,
            };
            file_table.insert(process_id, fd, file_handle);
        }

        let mut reply = Message::new(libmicro::MessageType::Reply);
        reply.set_data(0, fd as u64);
        reply
    }

    fn handle_close(&self, msg: &Message) -> Message {
        let fd = msg.get_data(0) as u32;
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        let mut file_table = self.file_table.write();
        match file_table.remove(process_id, fd) {
            Some(_) => Message::success(),
            None => Message::error(Error::BadFileDescriptor),
        }
    }

    fn handle_read(&self, msg: &Message) -> Message {
        let fd = msg.get_data(0) as u32;
        let buffer_shm = msg.get_data(1);
        let count = msg.get_data(2) as usize;
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        let (vnode, current_pos) = {
            let mut file_table = self.file_table.write();
            match file_table.get_mut(process_id, fd) {
                Some(handle) => {
                    if !handle.flags.contains(OpenFlags::READ) {
                        return Message::error(Error::PermissionDenied);
                    }
                    (handle.vnode.clone_vnode(), handle.position)
                }
                None => return Message::error(Error::BadFileDescriptor),
            }
        };

        let buffer_ptr = match memory::map_shared_memory(buffer_shm) {
            Ok(ptr) => ptr,
            Err(e) => return Message::error(e),
        };

        let buffer = unsafe { core::slice::from_raw_parts_mut(buffer_ptr, count) };

        let bytes_read = match vnode.read(current_pos, buffer) {
            Ok(n) => n,
            Err(e) => return Message::error(e),
        };

        {
            let mut file_table = self.file_table.write();
            if let Some(handle) = file_table.get_mut(process_id, fd) {
                handle.position += bytes_read as u64;
            }
        }

        let mut reply = Message::new(libmicro::MessageType::Reply);
        reply.set_data(0, bytes_read as u64);
        reply
    }

    fn handle_write(&self, msg: &Message) -> Message {
        let fd = msg.get_data(0) as u32;
        let buffer_shm = msg.get_data(1);
        let count = msg.get_data(2) as usize;
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        let (vnode, current_pos) = {
            let mut file_table = self.file_table.write();
            match file_table.get_mut(process_id, fd) {
                Some(handle) => {
                    if !handle.flags.contains(OpenFlags::WRITE) {
                        return Message::error(Error::PermissionDenied);
                    }
                    (handle.vnode.clone_vnode(), handle.position)
                }
                None => return Message::error(Error::BadFileDescriptor),
            }
        };

        let buffer_ptr = match memory::map_shared_memory(buffer_shm) {
            Ok(ptr) => ptr,
            Err(e) => return Message::error(e),
        };

        let buffer = unsafe { core::slice::from_raw_parts(buffer_ptr as *const u8, count) };

        let bytes_written = match vnode.write(current_pos, buffer) {
            Ok(n) => n,
            Err(e) => return Message::error(e),
        };

        {
            let mut file_table = self.file_table.write();
            if let Some(handle) = file_table.get_mut(process_id, fd) {
                handle.position += bytes_written as u64;
            }
        }

        let mut reply = Message::new(libmicro::MessageType::Reply);
        reply.set_data(0, bytes_written as u64);
        reply
    }

    fn handle_seek(&self, msg: &Message) -> Message {
        let fd = msg.get_data(0) as u32;
        let offset = msg.get_data(1) as i64;
        let whence = SeekWhence::from_u32(msg.get_data(2) as u32).unwrap_or(SeekWhence::Set);
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        let mut file_table = self.file_table.write();
        match file_table.get_mut(process_id, fd) {
            Some(handle) => {
                let stat = match handle.vnode.stat() {
                    Ok(s) => s,
                    Err(e) => return Message::error(e),
                };

                let new_pos = match whence {
                    SeekWhence::Set => offset as u64,
                    SeekWhence::Current => (handle.position as i64 + offset) as u64,
                    SeekWhence::End => (stat.size as i64 + offset) as u64,
                };

                handle.position = new_pos;

                let mut reply = Message::new(libmicro::MessageType::Reply);
                reply.set_data(0, new_pos);
                reply
            }
            None => Message::error(Error::BadFileDescriptor),
        }
    }

    fn handle_stat(&self, msg: &Message) -> Message {
        let path_data = msg.get_data(0) as *const u8;

        let path_str = unsafe {
            let mut len = 0;
            while *path_data.add(len) != 0 && len < 256 {
                len += 1;
            }
            let slice = core::slice::from_raw_parts(path_data, len);
            match core::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => return Message::error(Error::InvalidPath),
            }
        };

        let (fs, relative_path) = match self.resolve_path(path_str) {
            Ok((fs, path)) => (fs, path),
            Err(e) => return Message::error(e),
        };

        let stat = match fs.stat(&relative_path) {
            Ok(stat) => stat,
            Err(e) => return Message::error(e),
        };

        // Return stat data inline in the message
        let mut reply = Message::new(libmicro::MessageType::Reply);
        reply.set_data(0, stat.size);
        reply.set_data(1, stat.mode as u64);
        reply.set_data(2, stat.inode);
        reply.set_data(3, stat.links as u64);
        reply
    }

    fn handle_mkdir(&self, msg: &Message) -> Message {
        let path_data = msg.get_data(0) as *const u8;
        let mode = msg.get_data(1) as u32;

        let path_str = unsafe {
            let mut len = 0;
            while *path_data.add(len) != 0 && len < 256 {
                len += 1;
            }
            let slice = core::slice::from_raw_parts(path_data, len);
            match core::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => return Message::error(Error::InvalidPath),
            }
        };

        let (fs, relative_path) = match self.resolve_path(path_str) {
            Ok((fs, path)) => (fs, path),
            Err(e) => return Message::error(e),
        };

        match fs.mkdir(&relative_path, mode) {
            Ok(()) => Message::success(),
            Err(e) => Message::error(e),
        }
    }

    fn handle_mount(&self, msg: &Message) -> Message {
        let _device_data = msg.get_data(0);
        let mountpoint_data = msg.get_data(1) as *const u8;
        let fstype_data = msg.get_data(2) as *const u8;

        let mountpoint = unsafe { self.read_cstring(mountpoint_data) };
        let fstype = unsafe { self.read_cstring(fstype_data) };

        let mountpoint = match mountpoint {
            Ok(s) => s,
            Err(e) => return Message::error(e),
        };
        let fstype = match fstype {
            Ok(s) => s,
            Err(e) => return Message::error(e),
        };

        let fs: Arc<dyn FileSystem> = match fstype.as_str() {
            "initrd" => match InitrdFS::new() {
                Ok(fs) => Arc::new(fs),
                Err(e) => return Message::error(e),
            },
            _ => return Message::error(Error::UnsupportedFilesystem),
        };

        {
            let mut mounts = self.mount_table.write();
            mounts.insert(mountpoint, fs);
        }

        Message::success()
    }

    unsafe fn read_cstring(&self, ptr: *const u8) -> Result<String> {
        let mut len = 0;
        while *ptr.add(len) != 0 && len < 256 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(ptr, len);
        let str_ref = core::str::from_utf8(slice).map_err(|_| Error::InvalidPath)?;
        Ok(String::from(str_ref))
    }

    fn run(&self) -> ! {
        debug_println!("VFS server started");

        loop {
            let mut msg = Message::new(libmicro::MessageType::Receive);

            match ipc::receive(self.endpoint, &mut msg) {
                Ok(()) => {
                    let reply = match VfsOp::from_u32(msg.label()) {
                        Some(VfsOp::Open) => self.handle_open(&msg),
                        Some(VfsOp::Close) => self.handle_close(&msg),
                        Some(VfsOp::Read) => self.handle_read(&msg),
                        Some(VfsOp::Write) => self.handle_write(&msg),
                        Some(VfsOp::Seek) => self.handle_seek(&msg),
                        Some(VfsOp::Stat) => self.handle_stat(&msg),
                        Some(VfsOp::Mkdir) => self.handle_mkdir(&msg),
                        Some(VfsOp::Mount) => self.handle_mount(&msg),
                        _ => Message::error(Error::InvalidOperation),
                    };

                    let _ = ipc::reply(msg.sender(), &reply);
                }
                Err(e) => {
                    debug_println!("VFS server receive error: {:?}", e);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let _ = libmicro::init();

    let server = match VfsServer::new() {
        Ok(s) => s,
        Err(e) => {
            debug_println!("Failed to create VFS server: {:?}", e);
            libmicro::syscall::exit(-1);
        }
    };
    server.run()
}
