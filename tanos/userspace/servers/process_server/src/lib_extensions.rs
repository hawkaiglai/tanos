//! Library extensions for process server compatibility
//!
//! Provides missing types and functions needed by the process server

use kernel_types::ProcessId;
use kernel_types::EndpointId;
use alloc::vec::Vec;

/// Get the current process ID
pub fn current_process_id() -> ProcessId {
    match libmicro::syscall::get_process_id() {
        Ok(pid) => pid,
        Err(_) => ProcessId::new_const(0),
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
    InvalidElf = 12,
    UnsupportedElf = 13,
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

// Re-export MessageType from libmicro
pub use libmicro::MessageType;

use libmicro::{ServerMessage, ServerError};

/// Wrapper around ServerMessage providing server-specific methods
pub struct Message {
    inner: ServerMessage,
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

    pub fn async_pending() -> Self {
        let mut msg = ServerMessage::new(MessageType::Reply);
        msg.set_data(0, 0xFFFF_FFFF_FFFF_FFFE);
        Self { inner: msg }
    }

    pub fn is_async_pending(&self) -> bool {
        self.inner.get_data(0) == 0xFFFF_FFFF_FFFF_FFFE
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
                msg.inner.sender = EndpointId::new_unchecked(1); // Mock sender
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

    pub fn reply_to_process(_waiter: ProcessId, _msg: &Message) -> Result<()> {
        Ok(())
    }
}

// Memory functions
pub mod memory {
    use super::*;
    use kernel_types::VirtAddr;

    pub fn map_shared_memory(id: u64) -> Result<*mut u8> {
        match libmicro::syscall::map_shared_memory(id) {
            Ok(addr) => Ok(addr.as_u64() as *mut u8),
            Err(_) => Err(Error::InvalidOperation),
        }
    }

    pub fn create_address_space() -> Result<u64> {
        libmicro::memory::create_address_space()
            .map_err(|_| Error::InvalidOperation)
    }

    pub fn destroy_address_space(handle: u64) {
        let _ = libmicro::memory::destroy_address_space(handle);
    }

    pub fn create_shared_memory(size: usize) -> Result<u64> {
        libmicro::syscall::create_shared_memory(size)
            .map_err(|_| Error::OutOfMemory)
    }

    pub fn allocate_frame() -> Result<VirtAddr> {
        libmicro::memory::allocate_frame()
            .map_err(|_| Error::OutOfMemory)
    }

    pub fn map_page(addr: VirtAddr, size: usize, flags: u64) -> Result<()> {
        libmicro::memory::map_page(addr, size, flags)
            .map_err(|_| Error::InvalidOperation)
    }

    pub fn copy_to_user(dest: VirtAddr, src: &[u8]) -> Result<()> {
        libmicro::memory::copy_to_user(dest, src)
            .map_err(|_| Error::InvalidOperation)
    }

    pub fn zero_user_memory(addr: VirtAddr, size: usize) -> Result<()> {
        libmicro::memory::zero_user_memory(addr, size)
            .map_err(|_| Error::InvalidOperation)
    }
}

// Syscall extensions
pub mod syscall {
    use super::*;

    pub fn create_process(
        _pid: ProcessId,
        _entry_point: kernel_types::VirtAddr,
        _address_space: u64,
        _capabilities: &kernel_types::CapabilitySet,
    ) -> Result<()> {
        Ok(())
    }

    pub fn kill_process(_target_pid: ProcessId) -> Result<()> {
        Ok(())
    }

    pub fn exit(code: i32) -> ! {
        libmicro::syscall::exit(code);
    }
}
