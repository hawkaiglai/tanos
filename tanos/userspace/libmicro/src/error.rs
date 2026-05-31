//! Error handling for libmicro
//! 
//! Unified error system for userspace applications

use crate::syscall::SyscallError;

/// Main error type for libmicro operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    // System call errors
    Syscall(SyscallError),
    
    // Memory management errors
    OutOfMemory,
    InvalidAddress,
    AlignmentError,
    
    // IPC errors
    EndpointNotFound,
    MessageTooLarge,
    InvalidMessage,
    IpcTimeout,
    
    // Process errors
    ProcessNotFound,
    ProcessCreationFailed,
    PermissionDenied,
    
    // I/O errors
    IoError,
    FileNotFound,
    InvalidPath,
    
    // General errors
    InvalidArgument,
    NotImplemented,
    ResourceUnavailable,
}

impl From<SyscallError> for Error {
    fn from(err: SyscallError) -> Self {
        Error::Syscall(err)
    }
}

/// Result type for libmicro operations
pub type Result<T> = core::result::Result<T, Error>;

impl Error {
    /// Get error code for this error
    pub fn code(self) -> i32 {
        match self {
            Error::Syscall(SyscallError::InvalidSyscall) => 1,
            Error::Syscall(SyscallError::InvalidArgument) => 2,
            Error::Syscall(SyscallError::PermissionDenied) => 3,
            Error::Syscall(SyscallError::ResourceUnavailable) => 4,
            Error::Syscall(SyscallError::ProcessNotFound) => 5,
            Error::Syscall(SyscallError::EndpointNotFound) => 6,
            Error::Syscall(SyscallError::MessageTooLarge) => 7,
            Error::Syscall(SyscallError::OutOfMemory) => 8,
            
            Error::OutOfMemory => 10,
            Error::InvalidAddress => 11,
            Error::AlignmentError => 12,
            
            Error::EndpointNotFound => 20,
            Error::MessageTooLarge => 21,
            Error::InvalidMessage => 22,
            Error::IpcTimeout => 23,
            
            Error::ProcessNotFound => 30,
            Error::ProcessCreationFailed => 31,
            Error::PermissionDenied => 32,
            
            Error::IoError => 40,
            Error::FileNotFound => 41,
            Error::InvalidPath => 42,
            
            Error::InvalidArgument => 50,
            Error::NotImplemented => 51,
            Error::ResourceUnavailable => 52,
        }
    }
    
    /// Get human-readable description
    pub fn description(self) -> &'static str {
        match self {
            Error::Syscall(SyscallError::InvalidSyscall) => "Invalid system call",
            Error::Syscall(SyscallError::InvalidArgument) => "Invalid system call argument",
            Error::Syscall(SyscallError::PermissionDenied) => "Permission denied",
            Error::Syscall(SyscallError::ResourceUnavailable) => "Resource unavailable",
            Error::Syscall(SyscallError::ProcessNotFound) => "Process not found",
            Error::Syscall(SyscallError::EndpointNotFound) => "Endpoint not found",
            Error::Syscall(SyscallError::MessageTooLarge) => "Message too large",
            Error::Syscall(SyscallError::OutOfMemory) => "Out of memory",
            
            Error::OutOfMemory => "Out of memory",
            Error::InvalidAddress => "Invalid memory address",
            Error::AlignmentError => "Memory alignment error",
            
            Error::EndpointNotFound => "IPC endpoint not found",
            Error::MessageTooLarge => "IPC message too large",
            Error::InvalidMessage => "Invalid IPC message format",
            Error::IpcTimeout => "IPC operation timed out",
            
            Error::ProcessNotFound => "Process not found",
            Error::ProcessCreationFailed => "Process creation failed",
            Error::PermissionDenied => "Permission denied",
            
            Error::IoError => "I/O error",
            Error::FileNotFound => "File not found",
            Error::InvalidPath => "Invalid file path",
            
            Error::InvalidArgument => "Invalid argument",
            Error::NotImplemented => "Feature not implemented",
            Error::ResourceUnavailable => "Resource unavailable",
        }
    }
}