//! IPC System Call Handlers

use core::mem::size_of;
use super::*;
use crate::process;
use crate::ProcessId;
use crate::syscall::{SyscallResult, SyscallError};
use crate::capability::{Rights};
/// IPC Send system call
pub fn sys_ipc_send(
    endpoint_id: u32,
    message_ptr: *const Message,
    caller: ProcessId,
) -> SyscallResult {
    // Validate message pointer
    if message_ptr.is_null() {
        return Err(SyscallError::InvalidPointer);
    }
    
    // Check capability
    let cap_mgr = crate::capability::manager();
    if !cap_mgr.has_endpoint_access(caller, EndpointId::new_unchecked(endpoint_id), Rights::WRITE) {
        return Err(SyscallError::AccessDenied);
    }
    
    // Copy message from userspace
    let message = unsafe {
        if !crate::memory::is_user_readable(message_ptr as usize, size_of::<Message>()) {
            return Err(SyscallError::InvalidPointer);
        }
        *message_ptr
    };
    
    let ipc_mgr = manager();
    let mut caller_process = process::get_mut(caller)
        .ok_or(SyscallError::InvalidProcess)?;

    // Try fast path first
    match ipc_mgr.fastpath_send(EndpointId::new_unchecked(endpoint_id), &message, &mut caller_process) {
        Ok(true) => {
            // Fast path succeeded
            Ok(0)
        }
        Ok(false) => {
            // Fast path failed, use slow path
            drop(caller_process);
            ipc_mgr.slowpath_send(EndpointId::new_unchecked(endpoint_id), &message, caller)
                .map_err(|e| match e {
                    IpcError::InvalidEndpoint => SyscallError::InvalidArgument,
                    IpcError::AccessDenied => SyscallError::AccessDenied,
                    IpcError::WouldBlock => SyscallError::WouldBlock,
                    _ => SyscallError::InternalError,
                })?;
            
            // Process will be blocked by slow path
            Ok(0)
        }
        Err(e) => Err(match e {
            IpcError::InvalidEndpoint => SyscallError::InvalidArgument,
            IpcError::AccessDenied => SyscallError::AccessDenied,
            _ => SyscallError::InternalError,
        }),
    }
}

/// IPC Receive system call
pub fn sys_ipc_receive(
    endpoint_id: u32,
    message_ptr: *mut Message,
    caller: ProcessId,
) -> SyscallResult {
    // Validate message pointer
    if message_ptr.is_null() {
        return Err(SyscallError::InvalidPointer);
    }
    
    if !crate::memory::is_user_writable(message_ptr as usize, size_of::<Message>()) {
        return Err(SyscallError::InvalidPointer);
    }
    
    // Check capability
    let cap_mgr = crate::capability::manager();
    if !cap_mgr.has_endpoint_access(caller, EndpointId::new_unchecked(endpoint_id), Rights::READ) {
        return Err(SyscallError::AccessDenied);
    }
    
    let ipc_mgr = manager();
    
    match ipc_mgr.receive(EndpointId::new_unchecked(endpoint_id), caller) {
        Ok(Some(message)) => {
            // Message available immediately
            unsafe {
                *message_ptr = message;
            }
            Ok(0)
        }
        Ok(None) => {
            // No message available, process will be blocked
            Ok(0)
        }
        Err(e) => Err(match e {
            IpcError::InvalidEndpoint => SyscallError::InvalidArgument,
            IpcError::AccessDenied => SyscallError::AccessDenied,
            _ => SyscallError::InternalError,
        }),
    }
}

/// IPC Call system call (send + receive)
pub fn sys_ipc_call(
    endpoint_id: u32,
    message_ptr: *mut Message,
    caller: ProcessId,
) -> SyscallResult {
    // Validate message pointer
    if message_ptr.is_null() {
        return Err(SyscallError::InvalidPointer);
    }
    
    if !crate::memory::is_user_readable(message_ptr as usize, size_of::<Message>()) ||
       !crate::memory::is_user_writable(message_ptr as usize, size_of::<Message>()) {
        return Err(SyscallError::InvalidPointer);
    }
    
    // Check capability
    let cap_mgr = crate::capability::manager();
    if !cap_mgr.has_endpoint_access(caller, EndpointId::new_unchecked(endpoint_id), Rights::READ | Rights::WRITE) {
        return Err(SyscallError::AccessDenied);
    }
    
    // Copy message from userspace
    let message = unsafe { *message_ptr };
    
    let ipc_mgr = manager();
    let mut caller_process = process::get_mut(caller)
        .ok_or(SyscallError::InvalidProcess)?;

    // Try fast path first
    match ipc_mgr.fastpath_call(EndpointId::new_unchecked(endpoint_id), &message, &mut caller_process) {
        Ok(true) => {
            // Fast path succeeded, process will be blocked for reply
            Ok(0)
        }
        Ok(false) => {
            // Fast path failed, use slow path
            drop(caller_process);
            ipc_mgr.slowpath_send(EndpointId::new_unchecked(endpoint_id), &message, caller)
                .map_err(|e| match e {
                    IpcError::InvalidEndpoint => SyscallError::InvalidArgument,
                    IpcError::AccessDenied => SyscallError::AccessDenied,
                    IpcError::WouldBlock => SyscallError::WouldBlock,
                    _ => SyscallError::InternalError,
                })?;
            
            Ok(0)
        }
        Err(e) => Err(match e {
            IpcError::InvalidEndpoint => SyscallError::InvalidArgument,
            IpcError::AccessDenied => SyscallError::AccessDenied,
            _ => SyscallError::InternalError,
        }),
    }
}

/// IPC Reply system call
pub fn sys_ipc_reply(
    endpoint_id: u32,
    message_ptr: *const Message,
    caller: ProcessId,
) -> SyscallResult {
    // Validate message pointer
    if message_ptr.is_null() {
        return Err(SyscallError::InvalidPointer);
    }
    
    // Copy message from userspace
    let message = unsafe {
        if !crate::memory::is_user_readable(message_ptr as usize, size_of::<Message>()) {
            return Err(SyscallError::InvalidPointer);
        }
        *message_ptr
    };
    
    let ipc_mgr = manager();
    
    ipc_mgr.reply(EndpointId::new_unchecked(endpoint_id), &message, caller)
        .map_err(|e| match e {
            IpcError::InvalidEndpoint => SyscallError::InvalidArgument,
            IpcError::InvalidState => SyscallError::InvalidOperation,
            IpcError::AccessDenied => SyscallError::AccessDenied,
            _ => SyscallError::InternalError,
        })?;
    
    Ok(0)
}

/// Create endpoint system call
pub fn sys_create_endpoint(caller: ProcessId) -> SyscallResult {
    let ipc_mgr = manager();
    
    match ipc_mgr.create_endpoint(caller) {
        Ok(endpoint_id) => Ok(endpoint_id.as_u64()),
        Err(_) => Err(SyscallError::OutOfMemory),
    }
}

/// Delete endpoint system call
pub fn sys_delete_endpoint(endpoint_id: u32, caller: ProcessId) -> SyscallResult {
    let ipc_mgr = manager();
    
    ipc_mgr.delete_endpoint(EndpointId::new_unchecked(endpoint_id), caller)
        .map_err(|e| match e {
            IpcError::InvalidEndpoint => SyscallError::InvalidArgument,
            IpcError::AccessDenied => SyscallError::AccessDenied,
            _ => SyscallError::InternalError,
        })?;
    
    Ok(0)
}
