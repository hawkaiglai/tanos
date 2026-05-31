//! Library extensions for network stack compatibility

use kernel_types::EndpointId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Error {
    Success = 0,
    InvalidParameters = 1,
    OutOfMemory = 2,
    PermissionDenied = 3,
    InvalidOperation = 4,
    SocketNotFound = 5,
    NotConnected = 6,
    AlreadyBound = 7,
    AlreadyConnected = 8,
    NotBound = 9,
    ConnectionRefused = 10,
    ParseError = 11,
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

pub use libmicro::MessageType;
use libmicro::{ServerMessage, ServerError};

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
            _ => ServerError::InvalidOperation,
        };
        Self { inner: ServerMessage::error(server_err) }
    }

    pub fn success() -> Self {
        Self { inner: ServerMessage::success() }
    }

    pub fn set_label(&mut self, label: u32) { self.inner.set_label(label); }
    pub fn label(&self) -> u32 { self.inner.label() }
    pub fn set_data(&mut self, index: usize, value: u64) { self.inner.set_data(index, value); }
    pub fn get_data(&self, index: usize) -> u64 { self.inner.get_data(index) }
    pub fn sender(&self) -> EndpointId { self.inner.sender() }
}

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
                msg.inner.sender = EndpointId::new_unchecked(1);
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

pub mod memory {
    use super::*;

    pub fn map_shared_memory(id: u64) -> Result<*mut u8> {
        match libmicro::syscall::map_shared_memory(id) {
            Ok(addr) => Ok(addr.as_u64() as *mut u8),
            Err(_) => Err(Error::InvalidOperation),
        }
    }
}

pub mod syscall {
    pub fn exit(code: i32) -> ! {
        libmicro::syscall::exit(code);
    }
}
