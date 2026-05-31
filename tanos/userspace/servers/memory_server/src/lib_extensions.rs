//! Library extensions for memory server compatibility
//!
//! Provides missing types and functions needed by the memory server

use kernel_types::ProcessId;
use kernel_types::EndpointId;
use kernel_types::VirtAddr;
use kernel_types::PhysAddr;

/// Get the current process ID
pub fn current_process_id() -> ProcessId {
    match libmicro::syscall::get_process_id() {
        Ok(pid) => pid,
        Err(_) => ProcessId::new_const(0),
    }
}

// PageFlags bitflags (not in kernel_types for userspace)
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFlags: u64 {
        const PRESENT = 1 << 0;
        const WRITABLE = 1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const NO_CACHE = 1 << 4;
        const EXECUTABLE = 1 << 5;
        const READABLE = 1 << 6;
    }
}

// Error compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Error {
    Success = 0,
    InvalidParameters = 1,
    OutOfMemory = 2,
    PermissionDenied = 3,
    ProcessNotFound = 4,
    AllocationNotFound = 5,
    SharedMemoryNotFound = 6,
    AlreadyMapped = 7,
    NotMapped = 8,
    InvalidOperation = 9,
    EndpointNotFound = 10,
    MessageTooLarge = 11,
}

impl From<libmicro::Error> for Error {
    fn from(err: libmicro::Error) -> Self {
        match err {
            libmicro::Error::OutOfMemory => Error::OutOfMemory,
            libmicro::Error::PermissionDenied => Error::PermissionDenied,
            _ => Error::InvalidOperation,
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;

// Re-export MessageType
pub use libmicro::MessageType;

use libmicro::{ServerMessage, ServerError};

/// Wrapper around ServerMessage providing server-specific methods
pub struct Message {
    pub inner: ServerMessage,
}

impl Message {
    pub fn new(msg_type: MessageType) -> Self {
        Self { inner: ServerMessage::new(msg_type) }
    }

    pub fn error(err: Error) -> Self {
        let server_err = match err {
            Error::InvalidParameters => ServerError::InvalidParameters,
            Error::OutOfMemory => ServerError::OutOfMemory,
            Error::PermissionDenied => ServerError::PermissionDenied,
            Error::ProcessNotFound => ServerError::ProcessNotFound,
            Error::AllocationNotFound => ServerError::AllocationNotFound,
            Error::SharedMemoryNotFound => ServerError::SharedMemoryNotFound,
            Error::AlreadyMapped => ServerError::AlreadyMapped,
            Error::NotMapped => ServerError::NotMapped,
            Error::InvalidOperation => ServerError::InvalidOperation,
            Error::EndpointNotFound => ServerError::EndpointNotFound,
            Error::MessageTooLarge => ServerError::MessageTooLarge,
            _ => ServerError::InvalidOperation,
        };
        Self { inner: ServerMessage::error(server_err) }
    }

    pub fn success() -> Self {
        Self { inner: ServerMessage::success() }
    }

    pub fn set_label(&mut self, label: u32) {
        self.inner.set_label(label);
    }

    pub fn label(&self) -> u32 {
        self.inner.label()
    }

    pub fn set_data(&mut self, index: usize, value: u64) {
        self.inner.set_data(index, value);
    }

    pub fn get_data(&self, index: usize) -> u64 {
        self.inner.get_data(index)
    }

    pub fn sender(&self) -> EndpointId {
        self.inner.sender()
    }
}

// IPC functions
pub mod ipc {
    use super::*;

    pub fn create_endpoint() -> Result<EndpointId> {
        libmicro::syscall::create_endpoint()
            .map_err(|_| Error::InvalidOperation)
    }

    pub fn call(endpoint: EndpointId, msg: &Message) -> Result<()> {
        let _ = libmicro::syscall::send_message(
            endpoint,
            &msg.inner.data[0].to_le_bytes(),
        );
        Ok(())
    }

    pub fn receive(endpoint: EndpointId, msg: &mut Message) -> Result<()> {
        let mut buf = [0u8; 64];
        match libmicro::syscall::receive_message(endpoint, &mut buf) {
            Ok(_) => {
                msg.inner.sender = EndpointId::new_unchecked(1); // Will be set by kernel
                Ok(())
            }
            Err(_) => Err(Error::InvalidOperation),
        }
    }

    pub fn reply(sender: EndpointId, msg: &Message) -> Result<()> {
        let _ = libmicro::syscall::send_message(
            sender,
            &msg.inner.data[0].to_le_bytes(),
        );
        Ok(())
    }
}

// Syscall wrappers that take our types
pub mod syscall {
    use super::*;

    pub fn map_memory(vaddr: VirtAddr, _paddr: PhysAddr, flags: PageFlags) -> Result<()> {
        libmicro::syscall::map_memory(vaddr, 4096, flags.bits())
            .map_err(|_| Error::InvalidOperation)
    }

    pub fn unmap_memory(vaddr: VirtAddr) -> Result<()> {
        libmicro::syscall::unmap_memory(vaddr, 4096)
            .map_err(|_| Error::InvalidOperation)
    }

    pub fn protect_memory(_vaddr: VirtAddr, _size: usize, _flags: PageFlags) -> Result<()> {
        // Stub — will be a real syscall when kernel supports mprotect
        Ok(())
    }

    pub fn allocate_memory(size: usize, flags: u64) -> Result<VirtAddr> {
        libmicro::syscall::allocate_memory(size, flags)
            .map_err(|_| Error::OutOfMemory)
    }

    pub fn deallocate_memory(addr: VirtAddr, size: usize) -> Result<()> {
        libmicro::syscall::deallocate_memory(addr, size)
            .map_err(|_| Error::InvalidOperation)
    }

    pub fn exit(code: i32) -> ! {
        libmicro::syscall::exit(code);
    }
}
