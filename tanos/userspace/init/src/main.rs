//! Init process for TanOS.
//!
//! This single binary is launched by the kernel as PID 1 and PID 2 (same ELF,
//! roles chosen by PID). It contains two demos, selected at build time:
//!
//!  * DEFAULT — FAULT ISOLATION + REINCARNATION.
//!      PID 1 = a "survivor" that keeps running across a sibling's crash.
//!      PID 2 = a "faulty driver" that crashes (ud2) on its first life; the
//!              kernel catches the fault, restarts it (incrementing the
//!              generation passed in RDI), and the reincarnated copy runs fine.
//!
//!  * `--features console_demo` — USERSPACE VIRTUAL-CONSOLE DRIVER.
//!      PID 1 = a ring-3 console-driver *server*: it receives bytes over
//!              synchronous IPC and emits each one to the console.
//!      PID 2 = a ring-3 *client* that sends a line of text to the driver,
//!              one byte per IPC round-trip, then signals end-of-stream.
//!      This is the microkernel pattern: a service that a monolithic kernel
//!      would implement in-kernel (the tty/console) instead runs as an
//!      isolated user process reached only through message passing.
//!
//!      NOTE: a *hardware* driver would do port I/O directly from ring 3 (which
//!      needs IOPL / a TSS I/O-permission bitmap — not yet delegated here), so
//!      this "virtual" console emits through the kernel's debug-output
//!      primitive. The point being demonstrated is the isolation + IPC
//!      structure of a userspace driver, not raw device access.

#![no_std]
#![no_main]

extern crate libmicro;

#[cfg(not(feature = "console_demo"))]
use core::arch::asm;
use libmicro::syscall;

// ─── Live kernel IPC ABI (int 0x80 dispatch in kernel interrupt::mod) ─────────
// Single-word (register) messages. These are the numbers the running kernel
// actually implements, distinct from libmicro's older pointer-based wrappers.
const SYS_IPC_RECEIVE: u64 = 0x01; // rdi=endpoint            -> rax=message
const SYS_IPC_CALL: u64 = 0x02; // rdi=endpoint, rsi=message  -> rax=reply
const SYS_IPC_REPLY: u64 = 0x03; // rdi=reply value           -> rax=0
const SYS_IPC_REPLY_RECV: u64 = 0x0C; // rdi=reply, rsi=endpoint -> rax=next message
const SYS_DEBUG_PRINT: u64 = 0xF0; // rdi=ptr, rsi=len

/// A message value that is not a valid byte (0..=255): "end of stream".
const EOS: u64 = u64::MAX;

/// The (single) well-known IPC endpoint used by the demo.
const CONSOLE_EP: u64 = 0;

/// Emit raw bytes to the console (the kernel's debug output). A single ASCII
/// byte is valid UTF-8, which is what the kernel's DEBUG_PRINT expects.
#[inline]
fn emit(bytes: &[u8]) {
    unsafe {
        let _ = syscall::syscall2(SYS_DEBUG_PRINT, bytes.as_ptr() as u64, bytes.len() as u64);
    }
}

#[no_mangle]
pub extern "C" fn _start(generation: u64) -> ! {
    #[cfg(feature = "console_demo")]
    {
        let _ = generation;
        console_demo();
    }
    #[cfg(not(feature = "console_demo"))]
    {
        reincarnation_demo(generation);
    }
}

// ─── Default demo: fault isolation + reincarnation ───────────────────────────

/// `generation` is supplied by the kernel in RDI: 0 for the original launch,
/// incremented each time the kernel reincarnates this process after a crash.
#[cfg(not(feature = "console_demo"))]
fn reincarnation_demo(generation: u64) -> ! {
    let pid = syscall::getpid().as_u16();

    if pid == 1 {
        // Survivor: keeps running across a sibling's crash + restart.
        libmicro::debug_print("survivor (PID 1): running\n");
        let mut i = 0;
        while i < 10 {
            let _ = syscall::yield_cpu();
            i += 1;
        }
        libmicro::debug_print("survivor (PID 1): STILL ALIVE the whole time -- fault isolation works!\n");
        syscall::exit(0);
    } else if generation == 0 {
        // Faulty "driver", first life: crash with an illegal instruction.
        libmicro::debug_print("driver: starting (generation 0)\n");
        let _ = syscall::yield_cpu(); // let the survivor run first
        libmicro::debug_print("driver: hit a bug -- crashing (ud2)!\n");
        unsafe { asm!("ud2", options(noreturn)) }
    } else {
        // Reincarnated after the crash: run normally this time.
        libmicro::debug_print("driver: REINCARNATED and running normally -- recovery works!\n");
        let mut i = 0;
        while i < 3 {
            let _ = syscall::yield_cpu();
            i += 1;
        }
        libmicro::debug_print("driver: did useful work after recovery, exiting cleanly\n");
        syscall::exit(0);
    }
}

// ─── Optional demo: userspace virtual-console driver ─────────────────────────

#[cfg(feature = "console_demo")]
fn console_demo() -> ! {
    let pid = syscall::getpid().as_u16();
    if pid == 1 {
        console_driver();
    } else {
        console_client();
    }
}

/// PID 1: the console-driver server. Receives one byte per IPC round-trip and
/// emits it, until it receives the end-of-stream marker.
#[cfg(feature = "console_demo")]
fn console_driver() -> ! {
    libmicro::debug_print("console-driver (PID 1): ready, serving bytes over IPC\n");
    libmicro::debug_print("console-driver (PID 1): --- begin client output ---\n");

    // Block for the first byte.
    let mut msg = unsafe { syscall::syscall1(SYS_IPC_RECEIVE, CONSOLE_EP) };
    loop {
        if msg == EOS {
            // Release the client that sent the marker, then shut down.
            unsafe { syscall::syscall1(SYS_IPC_REPLY, 0); }
            libmicro::debug_print("\nconsole-driver (PID 1): --- end of stream, exiting ---\n");
            syscall::exit(0);
        }
        // "Render" the byte to the console device.
        let byte = [msg as u8];
        emit(&byte);
        // Reply to this byte's sender and block for the next byte in one call.
        msg = unsafe { syscall::syscall2(SYS_IPC_REPLY_RECV, 0, CONSOLE_EP) };
    }
}

/// PID 2: a client that prints a line *through* the userspace driver — every
/// character crosses an address-space boundary via synchronous IPC.
#[cfg(feature = "console_demo")]
fn console_client() -> ! {
    libmicro::debug_print("console-client (PID 2): sending a line through the userspace driver\n");

    let line = b"Hello from a ring-3 client, rendered by a ring-3 console driver via IPC!\n";
    for &b in line.iter() {
        // Each call blocks until the driver has emitted the byte and replied.
        unsafe { syscall::syscall2(SYS_IPC_CALL, CONSOLE_EP, b as u64); }
    }
    // Tell the driver we are done.
    unsafe { syscall::syscall2(SYS_IPC_CALL, CONSOLE_EP, EOS); }

    libmicro::debug_print("console-client (PID 2): line sent, exiting\n");
    syscall::exit(0);
}
