use crate::*;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Context {
    // General purpose registers
    pub rax: u64, pub rbx: u64, pub rcx: u64, pub rdx: u64,
    pub rsi: u64, pub rdi: u64, pub rbp: u64, pub rsp: u64,
    pub r8: u64,  pub r9: u64,  pub r10: u64, pub r11: u64,
    pub r12: u64, pub r13: u64, pub r14: u64, pub r15: u64,

    // Special registers
    pub rip: u64,
    pub rflags: u64,
    pub cr3: u64,  // Page table base

    // Segment selectors
    pub cs: u16, pub ds: u16, pub es: u16,
    pub fs: u16, pub gs: u16, pub ss: u16,
    
    // Padding to align to cache line
    _padding: [u64; 6],
}

impl Context {
    pub fn new(entry_point: VirtAddr, is_kernel: bool) -> Self {
        let (cs, ss, rflags, rsp) = if is_kernel {
            (
                KERNEL_CODE_SEGMENT,
                KERNEL_DATA_SEGMENT,
                0x202, // IF enabled
                crate::boot::KERNEL_STACK_BASE as u64 + crate::boot::KERNEL_STACK_SIZE as u64,
            )
        } else {
            (
                USER_CODE_SEGMENT,
                USER_DATA_SEGMENT,
                0x202, // IF enabled
                crate::USER_STACK_TOP as u64,
            )
        };
        
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            
            rip: entry_point.as_u64(),
            rflags,
            cr3: 0, // Will be set when address space is created
            
            cs, ss,
            ds: ss, es: ss, fs: ss, gs: ss,
            
            _padding: [0; 6],
        }
    }
    
    pub fn save_registers(&mut self) {
        // This would be called from assembly context switch code
        // to save the current register state
    }
    
    pub fn restore_registers(&self) {
        // This would be called from assembly context switch code
        // to restore register state
    }
}

/// Enter ring 3 at `entry` with stack pointer `user_stack`, using the given
/// user code/data selectors and page table (`cr3`). Switches the address space,
/// loads the user data segments, builds an iret frame, and `iretq`s into user
/// mode. Does not return.
///
/// SAFETY: `cr3` must be a valid PML4 that maps both the kernel (so the
/// trampoline keeps executing) and the user `entry`/`user_stack` pages.
pub unsafe fn enter_usermode(
    entry: u64,
    user_stack: u64,
    user_cs: u64,
    user_ss: u64,
    cr3: u64,
) -> ! {
    core::arch::asm!(
        "mov cr3, {cr3}",       // switch to the process's address space
        "mov ds, {seg:x}",      // user data segments (iretq does not load these)
        "mov es, {seg:x}",
        "mov fs, {seg:x}",
        "mov gs, {seg:x}",
        "push {ss}",            // iretq frame (high→low): SS
        "push {usp}",           //                        RSP
        "push {rflags}",        //                        RFLAGS (IF=1)
        "push {cs}",            //                        CS
        "push {rip}",           //                        RIP
        "iretq",
        cr3 = in(reg) cr3,
        seg = in(reg) user_ss,
        ss = in(reg) user_ss,
        usp = in(reg) user_stack,
        rflags = in(reg) 0x202u64,
        cs = in(reg) user_cs,
        rip = in(reg) entry,
        options(noreturn),
    );
}

// Segment selectors for x86_64
pub const KERNEL_CODE_SEGMENT: u16 = 0x08;
pub const KERNEL_DATA_SEGMENT: u16 = 0x10;
pub const USER_CODE_SEGMENT: u16 = 0x18 | 3; // RPL = 3
pub const USER_DATA_SEGMENT: u16 = 0x20 | 3; // RPL = 3

impl Default for Context {
    fn default() -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0, rflags: 0, cr3: 0,
            cs: 0, ds: 0, es: 0, fs: 0, gs: 0, ss: 0,
            _padding: [0; 6],
        }
    }
}
