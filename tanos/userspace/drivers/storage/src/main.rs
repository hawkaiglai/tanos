//! Storage Driver for TanOS
//!
//! Stub storage driver - initializes and enters a halt loop.

#![no_std]
#![no_main]

#[macro_use]
extern crate libmicro;
extern crate alloc;

use libmicro::syscall;

const DEVICE_CLASS_STORAGE: u64 = 3;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    debug_println!("Storage driver starting...");

    let process_id = syscall::getpid();
    debug_println!("Storage driver PID: {}", process_id.as_u16());

    // Create IPC endpoint for this driver
    if let Ok(endpoint) = syscall::create_endpoint() {
        let _ = syscall::register_driver(DEVICE_CLASS_STORAGE, endpoint, 0);
        let _ = syscall::set_driver_state(1); // Ready
        debug_println!("Storage driver registered");
    }

    debug_println!("Storage driver entering halt loop...");
    loop {
        let _ = syscall::yield_cpu();
    }
}
