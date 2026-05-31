#![no_std]
#![feature(asm_const)]
#![deny(unsafe_op_in_unsafe_fn)]

//! System call ABI for TanOS microkernel
//! 
//! This crate defines the low-level system call interface between userspace
//! and the TanOS microkernel. It provides architecture-specific calling
//! conventions and syscall number definitions.

pub mod numbers;

// Architecture-specific implementations
#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

// Re-export syscall numbers
pub use numbers::*;

/// System call error codes
#[repr(i64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    Success = 0,
    InvalidArgument = -1,
    PermissionDenied = -2,
    NotFound = -3,
    AlreadyExists = -4,
    OutOfMemory = -5,
    WouldBlock = -6,
    Interrupted = -7,
    TimedOut = -8,
    InvalidOperation = -9,
    BufferTooSmall = -10,
    EndOfFile = -11,
    BrokenPipe = -12,
    AddressInUse = -13,
    NotConnected = -14,
    ConnectionRefused = -15,
    NetworkUnreachable = -16,
    ResourceBusy = -17,
    TooManyFiles = -18,
    FileTooLarge = -19,
    NoSpaceLeft = -20,
    ReadOnlyFilesystem = -21,
    InvalidSeek = -22,
    NotDirectory = -23,
    IsDirectory = -24,
    DirectoryNotEmpty = -25,
    CrossDeviceLink = -26,
    Deadlock = -27,
    NameTooLong = -28,
    NoLocks = -29,
    FunctionNotImplemented = -30,
    Unknown = -999,
}

impl SyscallError {
    /// Convert raw syscall return value to Result
    pub fn from_raw(value: u64) -> Result<u64, Self> {
        let signed = value as i64;
        if signed >= 0 {
            Ok(value)
        } else {
            Err(match signed {
                -1 => Self::InvalidArgument,
                -2 => Self::PermissionDenied,
                -3 => Self::NotFound,
                -4 => Self::AlreadyExists,
                -5 => Self::OutOfMemory,
                -6 => Self::WouldBlock,
                -7 => Self::Interrupted,
                -8 => Self::TimedOut,
                -9 => Self::InvalidOperation,
                -10 => Self::BufferTooSmall,
                -11 => Self::EndOfFile,
                -12 => Self::BrokenPipe,
                -13 => Self::AddressInUse,
                -14 => Self::NotConnected,
                -15 => Self::ConnectionRefused,
                -16 => Self::NetworkUnreachable,
                -17 => Self::ResourceBusy,
                -18 => Self::TooManyFiles,
                -19 => Self::FileTooLarge,
                -20 => Self::NoSpaceLeft,
                -21 => Self::ReadOnlyFilesystem,
                -22 => Self::InvalidSeek,
                -23 => Self::NotDirectory,
                -24 => Self::IsDirectory,
                -25 => Self::DirectoryNotEmpty,
                -26 => Self::CrossDeviceLink,
                -27 => Self::Deadlock,
                -28 => Self::NameTooLong,
                -29 => Self::NoLocks,
                -30 => Self::FunctionNotImplemented,
                _ => Self::Unknown,
            })
        }
    }
}

/// System call result type
pub type SyscallResult<T = u64> = Result<T, SyscallError>;

/// Convert result to raw syscall return value
pub fn result_to_raw<T>(result: SyscallResult<T>) -> u64 
where
    T: Into<u64>,
{
    match result {
        Ok(val) => val.into(),
        Err(err) => (err as i64) as u64,
    }
}

/// Syscall parameter types
#[derive(Debug, Clone, Copy)]
pub struct SyscallArgs {
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: u64,
    pub arg4: u64,
    pub arg5: u64,
}

impl SyscallArgs {
    pub const fn new() -> Self {
        Self {
            arg0: 0,
            arg1: 0,
            arg2: 0,
            arg3: 0,
            arg4: 0,
            arg5: 0,
        }
    }
    
    pub const fn with_args(
        arg0: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64,
    ) -> Self {
        Self { arg0, arg1, arg2, arg3, arg4, arg5 }
    }
}

impl Default for SyscallArgs {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level syscall trait for type safety
pub trait Syscall {
    const NUMBER: SyscallNumber;
    type Args;
    type Return;
    
    fn call(args: Self::Args) -> SyscallResult<Self::Return>;
}

/// Architecture-independent syscall interface
pub trait SyscallInterface {
    /// Execute syscall with 0 arguments
    unsafe fn syscall0(number: SyscallNumber) -> u64;
    
    /// Execute syscall with 1 argument
    unsafe fn syscall1(number: SyscallNumber, arg0: u64) -> u64;
    
    /// Execute syscall with 2 arguments
    unsafe fn syscall2(number: SyscallNumber, arg0: u64, arg1: u64) -> u64;
    
    /// Execute syscall with 3 arguments
    unsafe fn syscall3(number: SyscallNumber, arg0: u64, arg1: u64, arg2: u64) -> u64;
    
    /// Execute syscall with 4 arguments
    unsafe fn syscall4(number: SyscallNumber, arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> u64;
    
    /// Execute syscall with 5 arguments
    unsafe fn syscall5(
        number: SyscallNumber,
        arg0: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
    ) -> u64;
    
    /// Execute syscall with 6 arguments
    unsafe fn syscall6(
        number: SyscallNumber,
        arg0: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64,
    ) -> u64;
    
    /// Execute syscall with structured arguments
    unsafe fn syscall_args(number: SyscallNumber, args: &SyscallArgs) -> u64 {
        unsafe {
            Self::syscall6(
                number,
                args.arg0,
                args.arg1,
                args.arg2,
                args.arg3,
                args.arg4,
                args.arg5,
            )
        }
    }
}

/// Memory protection flags
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionFlags {
    None = 0,
    Read = 1,
    Write = 2,
    Execute = 4,
    ReadWrite = 3,
    ReadExecute = 5,
    WriteExecute = 6,
    ReadWriteExecute = 7,
}

/// IPC message flags
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcFlags {
    None = 0,
    Block = 1,
    DontWait = 2,
    Grant = 4,
    Label = 8,
}

/// Process creation flags
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessFlags {
    None = 0,
    InheritCapabilities = 1,
    CreateNewAddressSpace = 2,
    StartSuspended = 4,
}

/// File operation flags
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFlags {
    ReadOnly = 0,
    WriteOnly = 1,
    ReadWrite = 2,
    Create = 64,
    Exclusive = 128,
    Truncate = 256,
    Append = 512,
}
