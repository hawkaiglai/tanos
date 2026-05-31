//! System call interface for userspace
//! 
//! Provides safe wrappers around TanOS system calls.

use kernel_types::*;
use syscall_abi::{SyscallNumber, SyscallArgs};
use crate::error::{Result, Error};
use alloc::vec::Vec;

// Import syscall number constants from syscall_abi
use syscall_abi::numbers::*;

// Compatibility aliases for old names
const SYSCALL_CREATE_PROCESS: SyscallNumber = SYSCALL_FORK;
const SYSCALL_WAITPID: SyscallNumber = SYSCALL_WAIT;
const SYSCALL_ALLOCATE_MEMORY: SyscallNumber = SYSCALL_ALLOC_MEMORY;
const SYSCALL_DEALLOCATE_MEMORY: SyscallNumber = SYSCALL_FREE_MEMORY;
const SYSCALL_GET_STATS: SyscallNumber = SYSCALL_GET_MEMORY_STATS;

/// System call error type
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
}

impl From<u64> for SyscallError {
    fn from(value: u64) -> Self {
        match value & 0x7FFF_FFFF_FFFF_FFFF {
            1 => SyscallError::InvalidSyscall,
            2 => SyscallError::InvalidArgument,
            3 => SyscallError::PermissionDenied,
            4 => SyscallError::ResourceUnavailable,
            5 => SyscallError::ProcessNotFound,
            6 => SyscallError::EndpointNotFound,
            7 => SyscallError::MessageTooLarge,
            8 => SyscallError::OutOfMemory,
            _ => SyscallError::InvalidSyscall,
        }
    }
}

/// System call result type
pub type SyscallResult<T> = core::result::Result<T, SyscallError>;

const ERROR_MASK: u64 = 0x8000_0000_0000_0000;

/// Raw system call functions (architecture-specific)
#[cfg(target_arch = "x86_64")]
mod raw {
    /// Syscall with 0 arguments
    #[inline(always)]
    pub unsafe fn syscall0(number: u64) -> u64 {
        let result: u64;
        core::arch::asm!(
            "int 0x80",
            in("rax") number,
            lateout("rax") result,
            options(nostack, preserves_flags)
        );
        result
    }
    
    /// Syscall with 1 argument
    #[inline(always)]
    pub unsafe fn syscall1(number: u64, arg0: u64) -> u64 {
        let result: u64;
        core::arch::asm!(
            "int 0x80",
            in("rax") number,
            in("rdi") arg0,
            lateout("rax") result,
            options(nostack, preserves_flags)
        );
        result
    }
    
    /// Syscall with 2 arguments
    #[inline(always)]
    pub unsafe fn syscall2(number: u64, arg0: u64, arg1: u64) -> u64 {
        let result: u64;
        core::arch::asm!(
            "int 0x80",
            in("rax") number,
            in("rdi") arg0,
            in("rsi") arg1,
            lateout("rax") result,
            options(nostack, preserves_flags)
        );
        result
    }
    
    /// Syscall with 3 arguments
    #[inline(always)]
    pub unsafe fn syscall3(number: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
        let result: u64;
        core::arch::asm!(
            "int 0x80",
            in("rax") number,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            lateout("rax") result,
            options(nostack, preserves_flags)
        );
        result
    }
    
    /// Syscall with 4 arguments
    #[inline(always)]
    pub unsafe fn syscall4(number: u64, arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
        let result: u64;
        core::arch::asm!(
            "int 0x80",
            in("rax") number,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            in("r10") arg3,
            lateout("rax") result,
            options(nostack, preserves_flags)
        );
        result
    }
    
    /// Syscall with 5 arguments
    #[inline(always)]
    pub unsafe fn syscall5(number: u64, arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
        let result: u64;
        core::arch::asm!(
            "int 0x80",
            in("rax") number,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            in("r10") arg3,
            in("r8") arg4,
            lateout("rax") result,
            options(nostack, preserves_flags)
        );
        result
    }
    
    /// Syscall with 6 arguments
    #[inline(always)]
    pub unsafe fn syscall6(number: u64, arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
        let result: u64;
        core::arch::asm!(
            "int 0x80",
            in("rax") number,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            in("r10") arg3,
            in("r8") arg4,
            in("r9") arg5,
            lateout("rax") result,
            options(nostack, preserves_flags)
        );
        result
    }
}

pub use raw::*;

/// Helper function to check syscall result
fn check_syscall_result(result: u64) -> SyscallResult<u64> {
    if result & ERROR_MASK != 0 {
        Err(SyscallError::from(result))
    } else {
        Ok(result)
    }
}

// === Process Management Syscalls ===

/// Exit the current process
pub fn exit(code: i32) -> ! {
    unsafe {
        syscall1(SYSCALL_EXIT, code as u64);
    }
    unreachable!()
}

/// Get current process ID
pub fn getpid() -> ProcessId {
    let result = unsafe { syscall0(SYSCALL_GETPID) };
    ProcessId::new_const(result as u16)
}

/// Create a new process
pub fn create_process(elf_data: &[u8]) -> SyscallResult<ProcessId> {
    let result = unsafe {
        syscall3(
            SYSCALL_CREATE_PROCESS,
            elf_data.as_ptr() as u64,
            elf_data.len() as u64,
            0, // flags
        )
    };
    check_syscall_result(result).map(|pid| ProcessId::new_const(pid as u16))
}

/// Yield CPU to other processes
pub fn yield_cpu() -> SyscallResult<()> {
    let result = unsafe { syscall0(SYSCALL_YIELD) };
    check_syscall_result(result).map(|_| ())
}

/// Wait for process to terminate
pub fn waitpid(pid: ProcessId) -> SyscallResult<i32> {
    let result = unsafe { syscall1(SYSCALL_WAITPID, pid.as_u16() as u64) };
    check_syscall_result(result).map(|exit_code| exit_code as i32)
}

// === Memory Management Syscalls ===

/// Allocate virtual memory
pub fn allocate_memory(size: usize, flags: u64) -> SyscallResult<VirtAddr> {
    let result = unsafe {
        syscall2(SYSCALL_ALLOCATE_MEMORY, size as u64, flags)
    };
    check_syscall_result(result).map(|addr| VirtAddr::new_unchecked(addr))
}

/// Deallocate virtual memory
pub fn deallocate_memory(addr: VirtAddr, size: usize) -> SyscallResult<()> {
    let result = unsafe {
        syscall2(SYSCALL_DEALLOCATE_MEMORY, addr.as_u64(), size as u64)
    };
    check_syscall_result(result).map(|_| ())
}

/// Map memory with specific flags
pub fn map_memory(addr: VirtAddr, size: usize, flags: u64) -> SyscallResult<()> {
    let result = unsafe {
        syscall3(SYSCALL_MAP_MEMORY, addr.as_u64(), size as u64, flags)
    };
    check_syscall_result(result).map(|_| ())
}

/// Unmap memory
pub fn unmap_memory(addr: VirtAddr, size: usize) -> SyscallResult<()> {
    let result = unsafe {
        syscall2(SYSCALL_UNMAP_MEMORY, addr.as_u64(), size as u64)
    };
    check_syscall_result(result).map(|_| ())
}

// === IPC Syscalls ===

/// Create a new IPC endpoint
pub fn create_endpoint() -> SyscallResult<EndpointId> {
    let result = unsafe { syscall0(SYSCALL_CREATE_ENDPOINT) };
    check_syscall_result(result).map(|id| EndpointId::new_unchecked(id as u32))
}

/// Send message to endpoint
pub fn send_message(endpoint: EndpointId, data: &[u8]) -> SyscallResult<()> {
    let result = unsafe {
        syscall3(
            SYSCALL_IPC_SEND,
            endpoint.as_u32() as u64,
            data.as_ptr() as u64,
            data.len() as u64,
        )
    };
    check_syscall_result(result).map(|_| ())
}

/// Receive message from endpoint
pub fn receive_message(endpoint: EndpointId, buffer: &mut [u8]) -> SyscallResult<usize> {
    let result = unsafe {
        syscall3(
            SYSCALL_IPC_RECEIVE,
            endpoint.as_u32() as u64,
            buffer.as_mut_ptr() as u64,
            buffer.len() as u64,
        )
    };
    check_syscall_result(result).map(|len| len as usize)
}

/// Call endpoint (send + receive)
pub fn call_endpoint(
    endpoint: EndpointId, 
    request: &[u8], 
    response: &mut [u8]
) -> SyscallResult<usize> {
    let result = unsafe {
        syscall6(
            SYSCALL_IPC_CALL,
            endpoint.as_u32() as u64,
            request.as_ptr() as u64,
            request.len() as u64,
            response.as_mut_ptr() as u64,
            response.len() as u64,
            0, // timeout
        )
    };
    check_syscall_result(result).map(|len| len as usize)
}

/// Reply to received message
pub fn reply_message(data: &[u8]) -> SyscallResult<()> {
    let result = unsafe {
        syscall2(
            SYSCALL_IPC_REPLY,
            data.as_ptr() as u64,
            data.len() as u64,
        )
    };
    check_syscall_result(result).map(|_| ())
}

/// Close endpoint
pub fn close_endpoint(endpoint: EndpointId) -> SyscallResult<()> {
    let result = unsafe { syscall1(SYSCALL_DELETE_ENDPOINT, endpoint.as_u32() as u64) };
    check_syscall_result(result).map(|_| ())
}

// === Time and Sleep ===

/// Sleep for specified milliseconds
pub fn sleep(milliseconds: u64) -> SyscallResult<()> {
    let result = unsafe { syscall1(SYSCALL_SLEEP, milliseconds) };
    check_syscall_result(result).map(|_| ())
}

/// Get current system time
pub fn get_time() -> SyscallResult<u64> {
    let result = unsafe { syscall0(SYSCALL_GET_TIME) };
    check_syscall_result(result)
}

// === Debug Functions ===

/// Print debug message to kernel debug output
pub fn debug(message: &str) -> SyscallResult<()> {
    // Pass both pointer and length: a Rust &str is not NUL-terminated, so the
    // kernel needs the explicit length to read exactly this string.
    let result = unsafe {
        syscall2(SYSCALL_DEBUG_PRINT, message.as_ptr() as u64, message.len() as u64)
    };
    check_syscall_result(result).map(|_| ())
}

/// Trigger debug breakpoint
pub fn debug_break() -> SyscallResult<()> {
    let result = unsafe { syscall0(SYSCALL_DEBUG_BREAK) };
    check_syscall_result(result).map(|_| ())
}

/// Get system statistics
pub fn get_stats() -> SyscallResult<u64> {
    let result = unsafe { syscall0(SYSCALL_GET_STATS) };
    check_syscall_result(result)
}

// === Driver Management Syscalls ===

/// Get current process ID (alias for getpid)
pub fn get_process_id() -> SyscallResult<ProcessId> {
    Ok(getpid())
}

/// Register a device driver with the kernel
pub fn register_driver(
    device_class: u64,
    endpoint: EndpointId,
    capabilities: u64,
) -> SyscallResult<()> {
    let result = unsafe {
        syscall3(
            SYSCALL_REGISTER_DRIVER,
            device_class,
            endpoint.as_u32() as u64,
            capabilities,
        )
    };
    check_syscall_result(result).map(|_| ())
}

/// Set driver state
pub fn set_driver_state(state: u64) -> SyscallResult<()> {
    let result = unsafe {
        syscall1(SYSCALL_DEVICE_IOCTL, state)
    };
    check_syscall_result(result).map(|_| ())
}

// === Shared Memory Syscalls ===

/// Shared memory handle
pub type SharedMemoryId = u64;

/// Create a shared memory region
pub fn create_shared_memory(size: usize) -> SyscallResult<SharedMemoryId> {
    let result = unsafe {
        syscall1(SYSCALL_CREATE_SHARED_MEM, size as u64)
    };
    check_syscall_result(result)
}

/// Map a shared memory region into the current address space
pub fn map_shared_memory(id: SharedMemoryId) -> SyscallResult<VirtAddr> {
    let result = unsafe {
        syscall1(SYSCALL_ATTACH_SHARED_MEM, id)
    };
    check_syscall_result(result).map(|addr| VirtAddr::new_unchecked(addr))
}

/// Unmap a shared memory region
pub fn unmap_shared_memory(id: SharedMemoryId) -> SyscallResult<()> {
    let result = unsafe {
        syscall1(SYSCALL_DETACH_SHARED_MEM, id)
    };
    check_syscall_result(result).map(|_| ())
}

/// Destroy a shared memory region
pub fn destroy_shared_memory(id: SharedMemoryId) -> SyscallResult<()> {
    // Re-use detach to destroy (kernel cleans up when all refs dropped)
    let result = unsafe {
        syscall1(SYSCALL_DETACH_SHARED_MEM, id)
    };
    check_syscall_result(result).map(|_| ())
}

// === I/O Port Syscalls ===

/// Request I/O port range access
pub fn request_io_port(base: u16, count: u16) -> SyscallResult<()> {
    let result = unsafe {
        syscall2(SYSCALL_REQUEST_IO_PORT, base as u64, count as u64)
    };
    check_syscall_result(result).map(|_| ())
}

/// Read byte from I/O port
pub fn io_read8(port: u16) -> SyscallResult<u8> {
    let result = unsafe {
        syscall1(SYSCALL_IO_READ8, port as u64)
    };
    check_syscall_result(result).map(|v| v as u8)
}

/// Write byte to I/O port
pub fn io_write8(port: u16, value: u8) -> SyscallResult<()> {
    let result = unsafe {
        syscall2(SYSCALL_IO_WRITE8, port as u64, value as u64)
    };
    check_syscall_result(result).map(|_| ())
}

// === IRQ Syscalls ===

/// Request IRQ capability
pub fn request_irq(irq: u8) -> SyscallResult<()> {
    let result = unsafe {
        syscall1(SYSCALL_REQUEST_IRQ, irq as u64)
    };
    check_syscall_result(result).map(|_| ())
}

/// Wait for an interrupt
pub fn wait_irq(irq: u8) -> SyscallResult<()> {
    let result = unsafe {
        syscall1(SYSCALL_WAIT_IRQ, irq as u64)
    };
    check_syscall_result(result).map(|_| ())
}

/// Acknowledge interrupt
pub fn ack_irq(irq: u8) -> SyscallResult<()> {
    let result = unsafe {
        syscall1(SYSCALL_ACK_IRQ, irq as u64)
    };
    check_syscall_result(result).map(|_| ())
}