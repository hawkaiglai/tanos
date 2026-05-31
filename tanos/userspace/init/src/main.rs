//! Init process for TanOS.
//!
//! Shipped demo: FAULT ISOLATION + REINCARNATION.
//!   PID 1 = a "survivor" that keeps running across a sibling's crash.
//!   PID 2 = a "faulty driver" that crashes (ud2) on its first life; the kernel
//!           catches the fault, restarts it (incrementing the generation passed
//!           in RDI), and the reincarnated copy runs normally.
//!
//! An IPC client/server benchmark variant of this file (server echo+1 via the
//! ReplyRecv fastpath; client times N call/reply round-trips with rdtsc, using
//! SYS_IPC_CALL=0x02 / SYS_IPC_REPLY_RECV=0x0C / SYS_REPORT=0xF2) was used to
//! measure IPC latency (~3,800 cycles/round-trip under KVM). See docs/OSDEV_POST.md.

#![no_std]
#![no_main]

extern crate libmicro;

use core::arch::asm;
use libmicro::syscall;

/// `generation` is supplied by the kernel in RDI: 0 for the original launch,
/// incremented each time the kernel reincarnates this process after a crash.
#[no_mangle]
pub extern "C" fn _start(generation: u64) -> ! {
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
