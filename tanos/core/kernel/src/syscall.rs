//! System call error and result types.
//!
//! NOTE: the live syscall dispatch path is `dispatch_syscall` in
//! `interrupt::mod` (reached via the `int 0x80` IDT gate). An earlier, separate
//! syscall dispatcher used to live here (`syscall_handler` plus a `syscall_*`
//! helper per call, statistics, driver registration, etc.) but it was never
//! wired into the live path and has been removed. This module now only provides
//! the shared error/result types still used by `ipc::syscalls`.

/// System call error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    InvalidSyscall,
    InvalidArgument,
    PermissionDenied,
    ResourceUnavailable,
    ProcessNotFound,
    EndpointNotFound,
    MessageTooLarge,
    OutOfMemory,
    InvalidPointer,
    AccessDenied,
    InvalidProcess,
    WouldBlock,
    InternalError,
    InvalidOperation,
}

impl From<SyscallError> for u64 {
    fn from(error: SyscallError) -> Self {
        match error {
            SyscallError::InvalidSyscall => 1,
            SyscallError::InvalidArgument => 2,
            SyscallError::PermissionDenied => 3,
            SyscallError::ResourceUnavailable => 4,
            SyscallError::ProcessNotFound => 5,
            SyscallError::EndpointNotFound => 6,
            SyscallError::MessageTooLarge => 7,
            SyscallError::OutOfMemory => 8,
            SyscallError::InvalidPointer => 9,
            SyscallError::AccessDenied => 10,
            SyscallError::InvalidProcess => 11,
            SyscallError::WouldBlock => 12,
            SyscallError::InternalError => 13,
            SyscallError::InvalidOperation => 14,
        }
    }
}

/// System call result type
pub type SyscallResult = Result<u64, SyscallError>;
