//! Interrupt handling subsystem
//! Manages interrupt descriptor table, interrupt routing, and IRQ delivery.
//!
//! Since nightly-2024-01-15 doesn't properly support `extern "x86-interrupt"`,
//! we build the IDT manually with raw 64-bit gate descriptors pointing to
//! assembly stubs in entry.S. The stubs save all registers and call
//! `interrupt_handler_wrapper()` in Rust.

pub mod idt;
pub mod irq;
pub mod timer;
pub mod handlers;

use x86_64::instructions::interrupts;
use spin::Once;
use core::sync::atomic::{AtomicU64, Ordering};

/// Global interrupt subsystem state
pub(crate) static INTERRUPT_MANAGER: Once<InterruptManager> = Once::new();

// ─── Raw IDT ────────────────────────────────────────────────────────────────

/// A single 64-bit IDT gate descriptor (16 bytes).
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,        // bits 0-2 = IST index, rest zero
    type_attr: u8,  // P(1) DPL(2) 0 Type(4)
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Create an interrupt gate (DPL=0, present, 64-bit interrupt gate type=0xE)
    fn new(handler: u64) -> Self {
        Self::with_dpl(handler, 0)
    }

    /// Create an interrupt gate with an explicit DPL. DPL=3 is required for the
    /// `int 0x80` syscall gate so ring-3 code is permitted to invoke it.
    fn with_dpl(handler: u64, dpl: u8) -> Self {
        Self {
            offset_low: handler as u16,
            selector: 0x08,  // Kernel code segment selector (GDT entry 1)
            ist: 0,
            // Present=1, DPL, Type=1110 (64-bit interrupt gate). 0x8E | (dpl<<5).
            type_attr: 0x8E | ((dpl & 0x3) << 5),
            offset_mid: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            reserved: 0,
        }
    }
}

/// IDT pointer structure for `lidt`
#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

/// Raw IDT — 256 entries (4KB), statically allocated
static mut RAW_IDT: [IdtEntry; 256] = [IdtEntry::missing(); 256];

/// Initialize the raw IDT with assembly handler addresses and load it.
/// SAFETY: Must be called exactly once, before interrupts are enabled.
unsafe fn init_idt() {
    // Exception handlers (vectors 0-19)
    RAW_IDT[0]  = IdtEntry::new(handlers::divide_error_asm as u64);
    RAW_IDT[1]  = IdtEntry::new(handlers::debug_asm as u64);
    RAW_IDT[2]  = IdtEntry::new(handlers::nmi_asm as u64);
    RAW_IDT[3]  = IdtEntry::new(handlers::breakpoint_asm as u64);
    RAW_IDT[4]  = IdtEntry::new(handlers::overflow_asm as u64);
    RAW_IDT[5]  = IdtEntry::new(handlers::bound_range_asm as u64);
    RAW_IDT[6]  = IdtEntry::new(handlers::invalid_opcode_asm as u64);
    RAW_IDT[7]  = IdtEntry::new(handlers::device_not_available_asm as u64);
    RAW_IDT[8]  = IdtEntry::new(handlers::double_fault_asm as u64);
    // Vector 9 (coprocessor segment overrun) is reserved
    RAW_IDT[10] = IdtEntry::new(handlers::invalid_tss_asm as u64);
    RAW_IDT[11] = IdtEntry::new(handlers::segment_not_present_asm as u64);
    RAW_IDT[12] = IdtEntry::new(handlers::stack_segment_fault_asm as u64);
    RAW_IDT[13] = IdtEntry::new(handlers::general_protection_fault_asm as u64);
    RAW_IDT[14] = IdtEntry::new(handlers::page_fault_asm as u64);
    // Vector 15 is reserved
    RAW_IDT[16] = IdtEntry::new(handlers::x87_floating_point_asm as u64);
    RAW_IDT[17] = IdtEntry::new(handlers::alignment_check_asm as u64);
    RAW_IDT[18] = IdtEntry::new(handlers::machine_check_asm as u64);
    RAW_IDT[19] = IdtEntry::new(handlers::simd_floating_point_asm as u64);

    // IRQ handlers (vectors 32-47)
    RAW_IDT[32] = IdtEntry::new(handlers::irq0_asm as u64);
    RAW_IDT[33] = IdtEntry::new(handlers::irq1_asm as u64);
    RAW_IDT[34] = IdtEntry::new(handlers::irq2_asm as u64);
    RAW_IDT[35] = IdtEntry::new(handlers::irq3_asm as u64);
    RAW_IDT[36] = IdtEntry::new(handlers::irq4_asm as u64);
    RAW_IDT[37] = IdtEntry::new(handlers::irq5_asm as u64);
    RAW_IDT[38] = IdtEntry::new(handlers::irq6_asm as u64);
    RAW_IDT[39] = IdtEntry::new(handlers::irq7_asm as u64);
    RAW_IDT[40] = IdtEntry::new(handlers::irq8_asm as u64);
    RAW_IDT[41] = IdtEntry::new(handlers::irq9_asm as u64);
    RAW_IDT[42] = IdtEntry::new(handlers::irq10_asm as u64);
    RAW_IDT[43] = IdtEntry::new(handlers::irq11_asm as u64);
    RAW_IDT[44] = IdtEntry::new(handlers::irq12_asm as u64);
    RAW_IDT[45] = IdtEntry::new(handlers::irq13_asm as u64);
    RAW_IDT[46] = IdtEntry::new(handlers::irq14_asm as u64);
    RAW_IDT[47] = IdtEntry::new(handlers::irq15_asm as u64);

    // Syscall gate (int 0x80). DPL=3 so ring-3 code may invoke it.
    RAW_IDT[0x80] = IdtEntry::with_dpl(handlers::syscall_int_asm as u64, 3);

    // Load IDT via lidt
    let idt_ptr = IdtPointer {
        limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
        base: RAW_IDT.as_ptr() as u64,
    };

    core::arch::asm!(
        "lidt [{}]",
        in(reg) &idt_ptr,
        options(nostack, preserves_flags)
    );
}

// ─── Interrupt Manager ──────────────────────────────────────────────────────

/// Interrupt statistics
#[derive(Debug, Default)]
pub struct InterruptStats {
    pub total_interrupts: AtomicU64,
    pub spurious_interrupts: AtomicU64,
    pub irq_counts: [AtomicU64; 16],
    pub exception_counts: [AtomicU64; 32],
}

/// Interrupt manager
pub struct InterruptManager {
    irq_manager: irq::IrqManager,
    timer: timer::Timer,
    stats: InterruptStats,
}

impl InterruptManager {
    fn new() -> Self {
        Self {
            irq_manager: irq::IrqManager::new(),
            timer: timer::Timer::new(),
            stats: InterruptStats::default(),
        }
    }

    /// Get IRQ manager
    pub fn irq_manager(&self) -> &irq::IrqManager {
        &self.irq_manager
    }

    /// Get timer
    pub fn timer(&self) -> &timer::Timer {
        &self.timer
    }

    /// Record interrupt statistics
    pub fn record_interrupt(&self, vector: u8) {
        self.stats.total_interrupts.fetch_add(1, Ordering::Relaxed);

        match vector {
            0..=31 => {
                self.stats.exception_counts[vector as usize].fetch_add(1, Ordering::Relaxed);
            }
            32..=47 => {
                let irq = vector - 32;
                self.stats.irq_counts[irq as usize].fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// Record spurious interrupt
    pub fn record_spurious_interrupt(&self) {
        self.stats.spurious_interrupts.fetch_add(1, Ordering::Relaxed);
    }

    /// Get interrupt statistics
    pub fn stats(&self) -> &InterruptStats {
        &self.stats
    }
}

// ─── Initialization ─────────────────────────────────────────────────────────

/// Initialize interrupt subsystem
pub fn init() {
    // Initialize PIC first (remaps IRQs to vectors 32-47)
    pic::init();

    // Setup and load IDT with all handlers
    unsafe { init_idt(); }

    // Create interrupt manager
    let manager = InterruptManager::new();

    // Initialize timer (programs PIT for 1000Hz)
    manager.timer().init();

    INTERRUPT_MANAGER.call_once(|| manager);

    // Unmask timer IRQ (IRQ 0) so we get ticks
    pic::enable_irq(0);

    crate::info!("Interrupt subsystem initialized (IDT loaded, timer enabled)");

    // NOW it's safe to enable interrupts — IDT is loaded with all handlers
    interrupts::enable();
}

/// Get interrupt manager
pub fn manager() -> &'static InterruptManager {
    INTERRUPT_MANAGER.get().expect("Interrupt subsystem not initialized")
}

// ─── PIC ────────────────────────────────────────────────────────────────────

/// PIC initialization
mod pic {
    use x86_64::instructions::port::Port;

    const PIC1_COMMAND: u16 = 0x20;
    const PIC1_DATA: u16 = 0x21;
    const PIC2_COMMAND: u16 = 0xA0;
    const PIC2_DATA: u16 = 0xA1;

    const ICW1_INIT: u8 = 0x11;
    const ICW4_8086: u8 = 0x01;

    pub fn init() {
        let mut pic1_command: Port<u8> = Port::new(PIC1_COMMAND);
        let mut pic1_data: Port<u8> = Port::new(PIC1_DATA);
        let mut pic2_command: Port<u8> = Port::new(PIC2_COMMAND);
        let mut pic2_data: Port<u8> = Port::new(PIC2_DATA);

        unsafe {
            // Save masks
            let _mask1: u8 = pic1_data.read();
            let _mask2: u8 = pic2_data.read();

            // Initialize PICs
            pic1_command.write(ICW1_INIT);
            pic2_command.write(ICW1_INIT);

            // Set vector offsets
            pic1_data.write(32u8);  // Master PIC: vectors 32-39
            pic2_data.write(40u8);  // Slave PIC: vectors 40-47

            // Configure cascade
            pic1_data.write(4u8);   // IRQ2 connected to slave
            pic2_data.write(2u8);   // Slave cascade identity

            // Set mode
            pic1_data.write(ICW4_8086);
            pic2_data.write(ICW4_8086);

            // Mask ALL IRQs initially (we unmask individually as needed)
            pic1_data.write(0xFFu8);
            pic2_data.write(0xFFu8);
        }
    }

    pub fn enable_irq(irq: u8) {
        let mut port: Port<u8> = if irq < 8 {
            Port::new(PIC1_DATA)
        } else {
            Port::new(PIC2_DATA)
        };

        let irq = irq % 8;

        unsafe {
            let mask: u8 = port.read();
            port.write(mask & !(1u8 << irq));
        }
    }

    pub fn disable_irq(irq: u8) {
        let mut port: Port<u8> = if irq < 8 {
            Port::new(PIC1_DATA)
        } else {
            Port::new(PIC2_DATA)
        };

        let irq = irq % 8;

        unsafe {
            let mask: u8 = port.read();
            port.write(mask | (1u8 << irq));
        }
    }

    pub fn end_of_interrupt(irq: u8) {
        let mut pic1_command: Port<u8> = Port::new(PIC1_COMMAND);
        let mut pic2_command: Port<u8> = Port::new(PIC2_COMMAND);

        unsafe {
            if irq >= 8 {
                pic2_command.write(0x20u8);
            }
            pic1_command.write(0x20u8);
        }
    }
}

/// Enable specific IRQ
pub fn enable_irq(irq: u8) {
    pic::enable_irq(irq);
}

/// Disable specific IRQ
pub fn disable_irq(irq: u8) {
    pic::disable_irq(irq);
}

/// Send end-of-interrupt signal
pub fn end_of_interrupt(irq: u8) {
    pic::end_of_interrupt(irq);
}

// ─── Assembly handler wrapper ───────────────────────────────────────────────

/// Interrupt handler wrapper called from assembly (entry.S common_interrupt_handler).
/// The assembly pushes: r15-r8, rbp, rdi, rsi, rdx, rcx, rbx, rax, vector, error_code
/// Then passes rsp in rdi.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct InterruptFrame {
    pub r15: u64, pub r14: u64, pub r13: u64, pub r12: u64,
    pub r11: u64, pub r10: u64, pub r9: u64, pub r8: u64,
    pub rbp: u64, pub rdi: u64, pub rsi: u64, pub rdx: u64,
    pub rcx: u64, pub rbx: u64, pub rax: u64,
    pub vector: u64,
    pub error_code: u64,
    pub rip: u64, pub cs: u64, pub rflags: u64, pub rsp: u64, pub ss: u64,
}

impl InterruptFrame {
    /// Build the initial frame for a freshly-loaded user process: entry point,
    /// user stack, ring-3 segment selectors, interrupts enabled, GP regs zero.
    pub fn new_user(entry: u64, user_stack: u64, user_cs: u64, user_ss: u64) -> Self {
        let mut f = Self::default();
        f.rip = entry;
        f.rsp = user_stack;
        f.cs = user_cs;
        f.ss = user_ss;
        f.rflags = 0x202; // IF set
        f
    }
}

/// Resume a user process from a saved frame: switch to its address space and
/// restore its full register state, then `iretq` into ring 3. Does not return.
///
/// SAFETY: `cr3` must map the kernel (so this code keeps running), the `frame`
/// memory (it lives on the kernel heap, identity-mapped in every AS), and the
/// frame's user rip/rsp pages.
pub unsafe fn resume_user(frame: *const InterruptFrame, cr3: u64) -> ! {
    core::arch::asm!(
        "mov cr3, {cr3}",       // switch to the target address space
        "mov rsp, {frame}",     // point RSP at the saved frame
        // Restore GP registers in the same order common_interrupt_handler pops.
        "pop r15", "pop r14", "pop r13", "pop r12",
        "pop r11", "pop r10", "pop r9", "pop r8",
        "pop rbp", "pop rdi", "pop rsi", "pop rdx",
        "pop rcx", "pop rbx", "pop rax",
        "add rsp, 16",          // skip vector + error_code
        "iretq",                // -> rip/cs/rflags/rsp/ss from the frame
        cr3 = in(reg) cr3,
        frame = in(reg) frame,
        options(noreturn),
    );
}

#[no_mangle]
pub extern "C" fn interrupt_handler_wrapper(frame: *mut InterruptFrame) {
    let frame = unsafe { &mut *frame };
    let vector = frame.vector as u8;

    if let Some(mgr) = INTERRUPT_MANAGER.get() {
        mgr.record_interrupt(vector);
    }

    match vector {
        0x80 => {
            // Syscall via `int 0x80`. Arguments are in the saved registers; the
            // result is written back into the saved RAX so it is restored on
            // iretq into the caller.
            frame.rax = dispatch_syscall(frame);
        }
        0..=31 => {
            let cr2: u64;
            unsafe { core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags)); }
            let from_user = (frame.cs & 0x3) == 0x3;
            crate::error!(
                "CPU Exception: vector={} error_code={:#x} rip={:#x} cs={:#x} rsp={:#x} cr2={:#x} (from {})",
                vector, frame.error_code, frame.rip, frame.cs, frame.rsp, cr2,
                if from_user { "ring 3" } else { "ring 0" }
            );
            if from_user {
                // Fault isolation: a userspace crash kills only that process;
                // the kernel and other processes keep running. Does not return.
                crate::process::fault_kill_current(vector);
            }
            // A fault in ring 0 is a kernel bug — fatal.
            loop { unsafe { core::arch::asm!("hlt") }; }
        }
        32 => {
            // Timer IRQ (IRQ 0)
            if let Some(mgr) = INTERRUPT_MANAGER.get() {
                mgr.timer().tick();
            }
            end_of_interrupt(0);
        }
        33 => {
            // Keyboard IRQ (IRQ 1) — read scancode to acknowledge
            unsafe {
                let mut port: x86_64::instructions::port::Port<u8> =
                    x86_64::instructions::port::Port::new(0x60);
                let _scancode = port.read();
            }
            end_of_interrupt(1);
        }
        32..=47 => {
            // Other hardware IRQs
            let irq = vector - 32;
            end_of_interrupt(irq);
        }
        _ => {}
    }
}

// ─── Syscall dispatch (int 0x80) ─────────────────────────────────────────────

// Syscall numbers (subset; see core/syscall_abi/src/numbers.rs).
const SYS_IPC_RECEIVE: u64 = 0x01;
const SYS_IPC_CALL: u64 = 0x02;
const SYS_IPC_REPLY: u64 = 0x03;
const SYS_IPC_REPLY_RECV: u64 = 0x0C; // seL4-style ReplyRecv (reply + receive)
const SYS_EXIT: u64 = 0x10;
const SYS_YIELD: u64 = 0x11;
const SYS_GETPID: u64 = 0x16;
const SYS_DEBUG_PRINT: u64 = 0xF0;
/// Non-standard: report an IPC benchmark result (rdi=total cycles, rsi=count).
const SYS_REPORT: u64 = 0xF2;

/// Error return marker: high bit set (matches libmicro's ERROR_MASK).
const SYS_ERR: u64 = 0x8000_0000_0000_0000;

/// Dispatch a syscall from an `int 0x80`. Reads the number from RAX and
/// arguments from RDI/RSI/… in the saved frame; returns the value for RAX.
///
/// Runs in ring 0 with the *caller's* address space active, so user pointers
/// (e.g. the DEBUG_PRINT string) are read directly via their user vaddr.
fn dispatch_syscall(frame: &InterruptFrame) -> u64 {
    match frame.rax {
        SYS_DEBUG_PRINT => {
            // rdi = ptr, rsi = len (bytes).
            let ptr = frame.rdi as *const u8;
            let len = frame.rsi as usize;
            if !ptr.is_null() && len <= 4096 {
                let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
                if let Ok(s) = core::str::from_utf8(bytes) {
                    crate::debug::serial::print(s);
                }
            }
            0
        }
        SYS_IPC_CALL => {
            // Capability enforcement: sending on an endpoint requires a WRITE
            // (send) capability for that endpoint. Denied callers get an error
            // and the IPC is never performed.
            let pid = crate::process::get_current_process().unwrap_or(crate::KERNEL_PID);
            let ep = frame.rdi;
            if crate::capability::manager().has_endpoint_access(
                pid, crate::EndpointId::new_unchecked(ep as u32), crate::capability::Rights::WRITE)
            {
                crate::process::ipc_call(frame, ep, frame.rsi)
            } else {
                crate::warn!(
                    "capability: PID {} DENIED ipc_call on endpoint {} (no send capability)",
                    pid.as_u16(), ep
                );
                SYS_ERR | 3 // PermissionDenied
            }
        }
        SYS_IPC_RECEIVE => {
            // Receiving on an endpoint requires a READ (receive) capability.
            let pid = crate::process::get_current_process().unwrap_or(crate::KERNEL_PID);
            let ep = frame.rdi;
            if crate::capability::manager().has_endpoint_access(
                pid, crate::EndpointId::new_unchecked(ep as u32), crate::capability::Rights::READ)
            {
                crate::process::ipc_receive(frame, ep)
            } else {
                crate::warn!(
                    "capability: PID {} DENIED ipc_receive on endpoint {} (no receive capability)",
                    pid.as_u16(), ep
                );
                SYS_ERR | 3 // PermissionDenied
            }
        }
        SYS_IPC_REPLY => crate::process::ipc_reply(frame, frame.rdi),
        SYS_IPC_REPLY_RECV => crate::process::ipc_reply_recv(frame, frame.rdi, frame.rsi),
        SYS_REPORT => {
            let total = frame.rdi;
            let count = frame.rsi.max(1);
            crate::info!(
                "IPC benchmark: {} round-trips, {} cycles total, {} cycles/round-trip",
                count, total, total / count
            );
            0
        }
        SYS_GETPID => {
            crate::process::get_current_process()
                .map(|pid| pid.as_u16() as u64)
                .unwrap_or(0)
        }
        SYS_YIELD => {
            // Cooperative reschedule: switch to the next runnable process.
            // Returns here only if there is no other process to run.
            crate::process::schedule_yield(frame);
            0
        }
        SYS_EXIT => {
            // Terminate the current process and switch to the next runnable
            // one (or halt if none). Does not return.
            crate::process::exit_current(frame.rdi as i64);
        }
        other => {
            crate::warn!("syscall: unimplemented number {:#x}", other);
            SYS_ERR | 1 // InvalidSyscall
        }
    }
}

/// Read timestamp counter
#[inline(always)]
pub fn rdtsc() -> u64 {
    unsafe {
        let low: u32;
        let high: u32;
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
        ((high as u64) << 32) | (low as u64)
    }
}
