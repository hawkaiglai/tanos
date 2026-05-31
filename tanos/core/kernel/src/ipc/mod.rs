//! Inter-Process Communication subsystem
//! Provides fast, secure message passing between processes using endpoints.
//! Optimized for sub-1000 cycle IPC latency.

pub mod endpoint;
pub mod message;
pub mod syscalls;

use crate::{Process, ProcessId, ProcessState};


use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;
use alloc::collections::BTreeMap;


pub use endpoint::{Endpoint, EndpointState};
pub use crate::EndpointId;
pub use message::{Message, MessageType, MessageHeader, MessageData, MessageFlags};

/// Global IPC subsystem state
static IPC_MANAGER: spin::Once<IpcManager> = spin::Once::new();

/// IPC Manager - coordinates all message passing
pub struct IpcManager {
    endpoints: Mutex<BTreeMap<EndpointId, Mutex<Endpoint>>>,
    next_endpoint_id: AtomicU32,
    fastpath_stats: Mutex<FastpathStats>,
}

#[derive(Debug, Default)]
struct FastpathStats {
    calls: u64,
    hits: u64,
    misses: u64,
    avg_cycles: u64,
}

impl IpcManager {
    fn new() -> Self {
        Self {
            endpoints: Mutex::new(BTreeMap::new()),
            next_endpoint_id: AtomicU32::new(256), // Start after well-known endpoints
            fastpath_stats: Mutex::new(FastpathStats::default()),
        }
    }

    /// Create a new endpoint
    pub fn create_endpoint(&self, owner: ProcessId) -> core::result::Result<EndpointId, IpcError> {
        let id = EndpointId::new_unchecked(self.next_endpoint_id.fetch_add(1, Ordering::SeqCst));
        let endpoint = Endpoint::new(id, owner);
        
        self.endpoints.lock().insert(id, Mutex::new(endpoint));
        Ok(id)
    }

    /// Delete an endpoint
    pub fn delete_endpoint(&self, id: EndpointId, caller: ProcessId) -> core::result::Result<(), IpcError> {
        let endpoints = self.endpoints.lock();
        
        if let Some(endpoint_mutex) = endpoints.get(&id) {
            let endpoint = endpoint_mutex.lock();
            
            // Check ownership
            if endpoint.owner != caller {
                return Err(IpcError::AccessDenied);
            }
            
            // Endpoint will be dropped, notifying any waiting processes
            drop(endpoint);
            drop(endpoints);
            
            self.endpoints.lock().remove(&id);
            Ok(())
        } else {
            Err(IpcError::InvalidEndpoint)
        }
    }

    /// Fast-path IPC send
    pub fn fastpath_send(
        &self,
        endpoint_id: EndpointId,
        message: &Message,
        _sender: &mut Process,
    ) -> core::result::Result<bool, IpcError> {
        let start_cycles = crate::interrupt::rdtsc();
        
        let endpoints = self.endpoints.lock();
        let endpoint_mutex = endpoints.get(&endpoint_id)
            .ok_or(IpcError::InvalidEndpoint)?;
        
        let mut endpoint = endpoint_mutex.try_lock()
            .ok_or(IpcError::EndpointBusy)?;
        
        let success = match endpoint.state {
            EndpointState::Receiving(receiver_id) => {
                // Fast path hit - receiver is waiting
                let receiver = crate::process::get_mut(receiver_id)
                    .ok_or(IpcError::InvalidProcess)?;
                
                // Direct message transfer
                receiver.set_received_message(*message);
                receiver.set_state(ProcessState::Ready);
                
                // Update endpoint state
                endpoint.state = EndpointState::Idle;
                
                // Schedule receiver immediately
                crate::process::scheduler::wake_process(receiver_id);
                
                true
            }
            _ => false, // Slow path needed
        };
        
        let end_cycles = crate::interrupt::rdtsc();
        self.update_fastpath_stats(success, end_cycles - start_cycles);
        
        Ok(success)
    }

    /// Fast-path IPC call (send + receive)
    pub fn fastpath_call(
        &self,
        endpoint_id: EndpointId,
        message: &Message,
        caller: &mut Process,
    ) -> core::result::Result<bool, IpcError> {
        let start_cycles = crate::interrupt::rdtsc();
        
        let endpoints = self.endpoints.lock();
        let endpoint_mutex = endpoints.get(&endpoint_id)
            .ok_or(IpcError::InvalidEndpoint)?;
        
        let mut endpoint = endpoint_mutex.try_lock()
            .ok_or(IpcError::EndpointBusy)?;
        
        let success = match endpoint.state {
            EndpointState::Receiving(receiver_id) => {
                // Fast path hit - receiver is waiting
                let receiver = crate::process::get_mut(receiver_id)
                    .ok_or(IpcError::InvalidProcess)?;
                
                // Direct message transfer
                receiver.set_received_message(*message);
                receiver.set_reply_endpoint(endpoint_id);
                receiver.set_state(ProcessState::Ready);
                
                // Block caller for reply
                caller.set_state(ProcessState::ReplyWait);
                endpoint.state = EndpointState::Call(caller.id);
                
                // Boost receiver priority for faster response
                crate::process::scheduler::boost_priority(receiver_id);
                
                true
            }
            _ => false, // Slow path needed
        };
        
        let end_cycles = crate::interrupt::rdtsc();
        self.update_fastpath_stats(success, end_cycles - start_cycles);
        
        Ok(success)
    }

    /// Slow-path IPC send
    pub fn slowpath_send(
        &self,
        endpoint_id: EndpointId,
        message: &Message,
        sender: ProcessId,
    ) -> core::result::Result<(), IpcError> {
        let endpoints = self.endpoints.lock();
        let endpoint_mutex = endpoints.get(&endpoint_id)
            .ok_or(IpcError::InvalidEndpoint)?;
        
        let mut endpoint = endpoint_mutex.lock();
        endpoint.enqueue_message(sender, *message);
        
        // Block sender
        let sender_process = crate::process::get_mut(sender)
            .ok_or(IpcError::InvalidProcess)?;
        sender_process.set_state(ProcessState::SendWait);
        
        Ok(())
    }

    /// IPC receive
    pub fn receive(
        &self,
        endpoint_id: EndpointId,
        receiver: ProcessId,
    ) -> core::result::Result<Option<Message>, IpcError> {
        let endpoints = self.endpoints.lock();
        let endpoint_mutex = endpoints.get(&endpoint_id)
            .ok_or(IpcError::InvalidEndpoint)?;
        
        let mut endpoint = endpoint_mutex.lock();
        
        // Check if there's a queued message
        if let Some((sender_id, message)) = endpoint.dequeue_message() {
            // Wake up sender
            crate::process::scheduler::wake_process(sender_id);
            Ok(Some(message))
        } else {
            // No message available, block receiver
            endpoint.state = EndpointState::Receiving(receiver);
            let receiver_process = crate::process::get_mut(receiver)
                .ok_or(IpcError::InvalidProcess)?;
            receiver_process.set_state(ProcessState::ReceiveWait);
            Ok(None)
        }
    }

    /// IPC reply
    pub fn reply(
        &self,
        endpoint_id: EndpointId,
        message: &Message,
        _replier: ProcessId,
    ) -> core::result::Result<(), IpcError> {
        let endpoints = self.endpoints.lock();
        let endpoint_mutex = endpoints.get(&endpoint_id)
            .ok_or(IpcError::InvalidEndpoint)?;
        
        let mut endpoint = endpoint_mutex.lock();
        
        match endpoint.state {
            EndpointState::Call(caller_id) => {
                // Deliver reply to caller
                let caller = crate::process::get_mut(caller_id)
                    .ok_or(IpcError::InvalidProcess)?;
                
                caller.set_received_message(*message);
                caller.set_state(ProcessState::Ready);
                
                // Reset endpoint state
                endpoint.state = EndpointState::Idle;
                
                // Wake up caller
                crate::process::scheduler::wake_process(caller_id);
                
                Ok(())
            }
            _ => Err(IpcError::InvalidState),
        }
    }

    fn update_fastpath_stats(&self, hit: bool, cycles: u64) {
        let mut stats = self.fastpath_stats.lock();
        stats.calls += 1;
        if hit {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }
        
        // Update running average
        stats.avg_cycles = (stats.avg_cycles * (stats.calls - 1) + cycles) / stats.calls;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IpcError {
    InvalidEndpoint,
    InvalidProcess,
    AccessDenied,
    EndpointBusy,
    InvalidState,
    MessageTooLarge,
    WouldBlock,
}

/// Initialize IPC subsystem
pub fn init() {
    IPC_MANAGER.call_once(|| IpcManager::new());
    crate::info!("IPC subsystem initialized");
}

/// Get IPC manager instance
pub fn manager() -> &'static IpcManager {
    IPC_MANAGER.get().expect("IPC not initialized")
}

/// Helper to access IPC manager safely
pub fn with_ipc_manager<F, R>(f: F) -> R
where
    F: FnOnce(&IpcManager) -> R,
{
    let ipc_manager = IPC_MANAGER.get()
        .expect("IPC manager not initialized");
    f(ipc_manager)
}
pub const MAX_MESSAGE_SIZE: usize = 4096;
