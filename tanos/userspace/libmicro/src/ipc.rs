//! IPC (Inter-Process Communication) support for userspace

use crate::{syscall, Result, Error};
use kernel_types::EndpointId;
use alloc::vec::Vec;

/// Maximum message size
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;

/// Initialize IPC subsystem
pub fn init() -> Result<()> {
    Ok(())
}

/// Cleanup IPC subsystem
pub fn cleanup() {
    // Cleanup is automatic when process exits
}

/// IPC endpoint wrapper
pub struct Endpoint {
    id: EndpointId,
}

impl Endpoint {
    /// Create a new endpoint
    pub fn create() -> Result<Self> {
        let id = syscall::create_endpoint()?;
        Ok(Self { id })
    }
    
    /// Get endpoint ID
    pub fn id(&self) -> EndpointId {
        self.id
    }
    
    /// Send a message to this endpoint
    pub fn send(&self, data: &[u8]) -> Result<()> {
        if data.len() > MAX_MESSAGE_SIZE {
            return Err(Error::MessageTooLarge);
        }
        syscall::send_message(self.id, data).map_err(|e| Error::Syscall(e))
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        let _ = syscall::close_endpoint(self.id);
    }
}

// === Module-level convenience functions ===

/// Create a new IPC endpoint
pub fn create_endpoint() -> Result<EndpointId> {
    syscall::create_endpoint().map_err(|e| Error::Syscall(e))
}

/// Send a message to an endpoint
pub fn send(endpoint: EndpointId, data: &[u8]) -> Result<()> {
    if data.len() > MAX_MESSAGE_SIZE {
        return Err(Error::MessageTooLarge);
    }
    syscall::send_message(endpoint, data).map_err(|e| Error::Syscall(e))
}

/// Receive a message from an endpoint
pub fn receive(endpoint: EndpointId, buffer: &mut [u8]) -> Result<usize> {
    syscall::receive_message(endpoint, buffer).map_err(|e| Error::Syscall(e))
}

/// Call an endpoint (send + receive)
pub fn call(endpoint: EndpointId, request: &[u8], response: &mut [u8]) -> Result<usize> {
    syscall::call_endpoint(endpoint, request, response).map_err(|e| Error::Syscall(e))
}

/// Reply to a received message
pub fn reply(data: &[u8]) -> Result<()> {
    syscall::reply_message(data).map_err(|e| Error::Syscall(e))
}