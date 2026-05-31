//! x86_64 system call implementation
//! 
//! This module implements the x86_64 system call calling convention for TanOS.
//! It uses the SYSCALL instruction for fast kernel entry.

use core::arch::asm;
use crate::{SyscallNumber, SyscallInterface, SyscallArgs};

/// x86_64 system call implementation
pub struct X86_64Syscall;

impl SyscallInterface for X86_64Syscall {
    /// Execute syscall with 0 arguments
    /// 
    /// # Safety
    /// - Caller must ensure syscall number is valid
    /// - Kernel must be properly initialized
    #[inline(always)]
    unsafe fn syscall0(number: SyscallNumber) -> u64 {
        let ret: u64;
        unsafe {
            asm!(
                "syscall",
                in("rax") number,
                lateout("rax") ret,
                out("rcx") _, // SYSCALL clobbers RCX with return address
                out("r11") _, // SYSCALL clobbers R11 with RFLAGS
                options(nostack, preserves_flags)
            );
        }
        ret
    }

    /// Execute syscall with 1 argument
    /// 
    /// # Arguments
    /// - RDI: arg0
    /// 
    /// # Safety
    /// - Caller must ensure syscall number is valid
    /// - Arguments must be valid for the specific syscall
    #[inline(always)]
    unsafe fn syscall1(number: SyscallNumber, arg0: u64) -> u64 {
        let ret: u64;
        unsafe {
            asm!(
                "syscall",
                in("rax") number,
                in("rdi") arg0,
                lateout("rax") ret,
                out("rcx") _,
                out("r11") _,
                options(nostack, preserves_flags)
            );
        }
        ret
    }

    /// Execute syscall with 2 arguments
    /// 
    /// # Arguments
    /// - RDI: arg0
    /// - RSI: arg1
    /// 
    /// # Safety
    /// - Caller must ensure syscall number is valid
    /// - Arguments must be valid for the specific syscall
    #[inline(always)]
    unsafe fn syscall2(number: SyscallNumber, arg0: u64, arg1: u64) -> u64 {
        let ret: u64;
        unsafe {
            asm!(
                "syscall",
                in("rax") number,
                in("rdi") arg0,
                in("rsi") arg1,
                lateout("rax") ret,
                out("rcx") _,
                out("r11") _,
                options(nostack, preserves_flags)
            );
        }
        ret
    }

    /// Execute syscall with 3 arguments
    /// 
    /// # Arguments
    /// - RDI: arg0
    /// - RSI: arg1
    /// - RDX: arg2
    /// 
    /// # Safety
    /// - Caller must ensure syscall number is valid
    /// - Arguments must be valid for the specific syscall
    #[inline(always)]
    unsafe fn syscall3(number: SyscallNumber, arg0: u64, arg1: u64, arg2: u64) -> u64 {
        let ret: u64;
        unsafe {
            asm!(
                "syscall",
                in("rax") number,
                in("rdi") arg0,
                in("rsi") arg1,
                in("rdx") arg2,
                lateout("rax") ret,
                out("rcx") _,
                out("r11") _,
                options(nostack, preserves_flags)
            );
        }
        ret
    }

    /// Execute syscall with 4 arguments
    /// 
    /// # Arguments
    /// - RDI: arg0
    /// - RSI: arg1
    /// - RDX: arg2
    /// - R10: arg3 (note: not RCX, as SYSCALL clobbers RCX)
    /// 
    /// # Safety
    /// - Caller must ensure syscall number is valid
    /// - Arguments must be valid for the specific syscall
    #[inline(always)]
    unsafe fn syscall4(number: SyscallNumber, arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
        let ret: u64;
        unsafe {
            asm!(
                "syscall",
                in("rax") number,
                in("rdi") arg0,
                in("rsi") arg1,
                in("rdx") arg2,
                in("r10") arg3,
                lateout("rax") ret,
                out("rcx") _,
                out("r11") _,
                options(nostack, preserves_flags)
            );
        }
        ret
    }

    /// Execute syscall with 5 arguments
    /// 
    /// # Arguments
    /// - RDI: arg0
    /// - RSI: arg1
    /// - RDX: arg2
    /// - R10: arg3
    /// - R8: arg4
    /// 
    /// # Safety
    /// - Caller must ensure syscall number is valid
    /// - Arguments must be valid for the specific syscall
    #[inline(always)]
    unsafe fn syscall5(
        number: SyscallNumber,
        arg0: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
    ) -> u64 {
        let ret: u64;
        unsafe {
            asm!(
                "syscall",
                in("rax") number,
                in("rdi") arg0,
                in("rsi") arg1,
                in("rdx") arg2,
                in("r10") arg3,
                in("r8") arg4,
                lateout("rax") ret,
                out("rcx") _,
                out("r11") _,
                options(nostack, preserves_flags)
            );
        }
        ret
    }

    /// Execute syscall with 6 arguments (maximum)
    /// 
    /// # Arguments
    /// - RDI: arg0
    /// - RSI: arg1
    /// - RDX: arg2
    /// - R10: arg3
    /// - R8: arg4
    /// - R9: arg5
    /// 
    /// # Safety
    /// - Caller must ensure syscall number is valid
    /// - Arguments must be valid for the specific syscall
    #[inline(always)]
    unsafe fn syscall6(
        number: SyscallNumber,
        arg0: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64,
    ) -> u64 {
        let ret: u64;
        unsafe {
            asm!(
                "syscall",
                in("rax") number,
                in("rdi") arg0,
                in("rsi") arg1,
                in("rdx") arg2,
                in("r10") arg3,
                in("r8") arg4,
                in("r9") arg5,
                lateout("rax") ret,
                out("rcx") _,
                out("r11") _,
                options(nostack, preserves_flags)
            );
        }
        ret
    }
}

// Re-export the syscall functions for convenience
pub use X86_64Syscall as Syscall;

/// Convenience wrapper functions

/// Execute syscall with 0 arguments
#[inline(always)]
pub unsafe fn syscall0(number: SyscallNumber) -> u64 {
    unsafe { X86_64Syscall::syscall0(number) }
}

/// Execute syscall with 1 argument
#[inline(always)]
pub unsafe fn syscall1(number: SyscallNumber, arg0: u64) -> u64 {
    unsafe { X86_64Syscall::syscall1(number, arg0) }
}

/// Execute syscall with 2 arguments
#[inline(always)]
pub unsafe fn syscall2(number: SyscallNumber, arg0: u64, arg1: u64) -> u64 {
    unsafe { X86_64Syscall::syscall2(number, arg0, arg1) }
}

/// Execute syscall with 3 arguments
#[inline(always)]
pub unsafe fn syscall3(number: SyscallNumber, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    unsafe { X86_64Syscall::syscall3(number, arg0, arg1, arg2) }
}

/// Execute syscall with 4 arguments
#[inline(always)]
pub unsafe fn syscall4(number: SyscallNumber, arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    unsafe { X86_64Syscall::syscall4(number, arg0, arg1, arg2, arg3) }
}

/// Execute syscall with 5 arguments
#[inline(always)]
pub unsafe fn syscall5(
    number: SyscallNumber,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
) -> u64 {
    unsafe { X86_64Syscall::syscall5(number, arg0, arg1, arg2, arg3, arg4) }
}

/// Execute syscall with 6 arguments
#[inline(always)]
pub unsafe fn syscall6(
    number: SyscallNumber,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
) -> u64 {
    unsafe { X86_64Syscall::syscall6(number, arg0, arg1, arg2, arg3, arg4, arg5) }
}

/// Execute syscall with structured arguments
#[inline(always)]
pub unsafe fn syscall_args(number: SyscallNumber, args: &SyscallArgs) -> u64 {
    unsafe { X86_64Syscall::syscall_args(number, args) }
}

/// Fast system call entry point optimized for IPC
/// 
/// This is a specialized version for high-frequency IPC operations
/// that skips some overhead for maximum performance.
/// 
/// # Arguments
/// - RAX: SYSCALL_IPC_SEND, SYSCALL_IPC_CALL, etc.
/// - RDI: endpoint_id
/// - RSI: message_ptr
/// - RDX: flags
/// 
/// # Returns
/// - RAX: result (0 = success, negative = error)
/// 
/// # Safety
/// - Message pointer must be valid and properly aligned
/// - Endpoint ID must be valid
/// - Flags must be valid for the operation
#[inline(always)]
pub unsafe fn fast_ipc_call(
    syscall_num: SyscallNumber,
    endpoint_id: u64,
    message_ptr: u64,
    flags: u64,
) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            in("rax") syscall_num,
            in("rdi") endpoint_id,
            in("rsi") message_ptr,
            in("rdx") flags,
            lateout("rax") ret,
            out("rcx") _,
            out("r11") _,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Get current timestamp counter for benchmarking
/// 
/// # Returns
/// - CPU timestamp counter value
/// 
/// # Safety
/// - Safe to call, but results are only meaningful for timing
#[inline(always)]
pub unsafe fn rdtsc() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nostack, preserves_flags, nomem)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Serializing timestamp counter for precise benchmarking
/// 
/// # Returns
/// - CPU timestamp counter value (serialized)
/// 
/// # Safety
/// - Safe to call, but may be slower than rdtsc()
#[inline(always)]
pub unsafe fn rdtscp() -> u64 {
    let low: u32;
    let high: u32;
    let _aux: u32;
    unsafe {
        asm!(
            "rdtscp",
            out("eax") low,
            out("edx") high,
            out("ecx") _aux,
            options(nostack, preserves_flags, nomem)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// CPU pause instruction for spinloop optimization
/// 
/// # Safety
/// - Safe to call, used in spin loops to reduce power consumption
#[inline(always)]
pub unsafe fn cpu_pause() {
    unsafe {
        asm!("pause", options(nostack, preserves_flags, nomem));
    }
}

/// Memory fence for ordering guarantees
/// 
/// # Safety
/// - Safe to call, ensures memory ordering
#[inline(always)]
pub unsafe fn memory_fence() {
    unsafe {
        asm!("mfence", options(nostack, preserves_flags));
    }
}

/// Get current CPU ID (for SMP systems)
/// 
/// # Returns
/// - Current CPU/core ID
/// 
/// # Safety
/// - Safe to call, may require kernel support for userspace
#[inline(always)]
pub unsafe fn get_cpu_id() -> u32 {
    // This would typically use CPUID or a kernel syscall
    // For now, return 0 (single core assumption)
    0
}

/// Architecture-specific constants
pub mod constants {
    /// x86_64 page size (4KB)
    pub const PAGE_SIZE: usize = 4096;
    
    /// x86_64 page mask
    pub const PAGE_MASK: usize = PAGE_SIZE - 1;
    
    /// x86_64 cache line size (typical)
    pub const CACHE_LINE_SIZE: usize = 64;
    
    /// Maximum number of syscall arguments in x86_64 ABI
    pub const MAX_SYSCALL_ARGS: usize = 6;
    
    /// User space start address
    pub const USER_SPACE_START: u64 = 0x0000_0000_0000_0000;
    
    /// User space end address (canonical address limit)
    pub const USER_SPACE_END: u64 = 0x0000_7FFF_FFFF_FFFF;
    
    /// Kernel space start address
    pub const KERNEL_SPACE_START: u64 = 0xFFFF_8000_0000_0000;
    
    /// Kernel space end address
    pub const KERNEL_SPACE_END: u64 = 0xFFFF_FFFF_FFFF_FFFF;
    
    /// Default user stack size
    pub const DEFAULT_USER_STACK_SIZE: usize = 8 * 1024 * 1024; // 8MB
    
    /// Minimum user stack size
    pub const MIN_USER_STACK_SIZE: usize = 4 * 1024; // 4KB
    
    /// Maximum user stack size
    pub const MAX_USER_STACK_SIZE: usize = 128 * 1024 * 1024; // 128MB
}

/// Architecture-specific register context for debugging
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct X86_64Context {
    // General purpose registers
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    
    // Control registers
    pub rip: u64,
    pub rflags: u64,
    
    // Segment selectors
    pub cs: u16,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,
    pub ss: u16,
    
    // Padding for alignment
    _padding: [u16; 2],
}

impl Default for X86_64Context {
    fn default() -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0, rflags: 0x202, // IF = 1, Reserved = 1
            cs: 0, ds: 0, es: 0, fs: 0, gs: 0, ss: 0,
            _padding: [0; 2],
        }
    }
}

/// Test module for syscall functionality
#[cfg(test)]
mod tests {
    use super::*;
    use crate::numbers::*;
    
    #[test]
    fn test_syscall_interface() {
        // These tests would need kernel support to run properly
        // For now, we just test that the functions compile and link
        
        unsafe {
            // Test that we can call syscall functions without panicking
            // Note: These would fail in user mode without proper kernel
            let _result = syscall0(SYSCALL_DEBUG_TEST);
        }
    }
    
    #[test]
    fn test_context_size() {
        // Ensure context structure has expected size and alignment
        assert_eq!(core::mem::size_of::<X86_64Context>(), 144);
        assert_eq!(core::mem::align_of::<X86_64Context>(), 8);
    }
    
    #[test]
    fn test_constants() {
        // Verify architectural constants
        assert_eq!(constants::PAGE_SIZE, 4096);
        assert_eq!(constants::CACHE_LINE_SIZE, 64);
        assert_eq!(constants::MAX_SYSCALL_ARGS, 6);
        
        // Verify address space layout
        assert!(constants::USER_SPACE_END < constants::KERNEL_SPACE_START);
    }
}
