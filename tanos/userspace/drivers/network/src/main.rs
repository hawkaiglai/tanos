//! Network Driver for TanOS
//!
//! Stub network driver - initializes and enters a halt loop.

#![no_std]
#![no_main]

#[macro_use]
extern crate libmicro;
extern crate alloc;

use libmicro::syscall;

const DEVICE_CLASS_NETWORK: u64 = 4;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    debug_println!("Network driver starting...");

    let process_id = syscall::getpid();
    debug_println!("Network driver PID: {}", process_id.as_u16());

    // Create IPC endpoint for this driver
    if let Ok(endpoint) = syscall::create_endpoint() {
        let _ = syscall::register_driver(DEVICE_CLASS_NETWORK, endpoint, 0);
        let _ = syscall::set_driver_state(1); // Ready
        debug_println!("Network driver registered");
    }

    debug_println!("Network driver entering halt loop...");
    loop {
        let _ = syscall::yield_cpu();
    }
}
