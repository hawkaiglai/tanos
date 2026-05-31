//! Keyboard Driver for TanOS
//!
//! PS/2 keyboard driver that sends keypresses to the shell via IPC.

#![no_std]
#![no_main]

#[macro_use]
extern crate libmicro;
extern crate alloc;

use alloc::vec::Vec;
use kernel_types::EndpointId;
use libmicro::syscall;
use libmicro::protocols::{endpoints, KeyboardOp};
use libmicro::{ServerMessage, MessageType};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    if let Err(_) = init_keyboard_driver() {
        syscall::exit(-1);
    }
    driver_main_loop();
}

fn init_keyboard_driver() -> core::result::Result<(), DriverError> {
    debug_println!("Keyboard driver starting...");

    let process_id = syscall::getpid();
    debug_println!("Keyboard driver PID: {}", process_id.as_u16());

    // Create IPC endpoint for this driver
    let endpoint = syscall::create_endpoint()
        .map_err(|_| DriverError::RegistrationFailed)?;

    // Register with device manager via syscall
    syscall::register_driver(
        DEVICE_CLASS_INPUT,
        endpoint,
        0,
    ).map_err(|_| DriverError::RegistrationFailed)?;

    debug_println!("Keyboard driver registered");

    // Initialize hardware
    init_ps2_keyboard()?;

    debug_println!("Keyboard driver ready");
    Ok(())
}

const DEVICE_CLASS_INPUT: u64 = 1;

fn init_ps2_keyboard() -> core::result::Result<(), DriverError> {
    debug_println!("Initializing PS/2 keyboard hardware...");

    // Disable devices
    outb(0x64, 0xAD);
    outb(0x64, 0xA7);

    // Flush output buffer
    while inb(0x64) & 0x01 != 0 {
        inb(0x60);
    }

    // Set controller configuration
    outb(0x64, 0x20);
    wait_for_output();
    let mut config = inb(0x60);
    config &= !0x43;
    outb(0x64, 0x60);
    wait_for_input();
    outb(0x60, config);

    // Test controller
    outb(0x64, 0xAA);
    wait_for_output();
    if inb(0x60) != 0x55 {
        debug_println!("PS/2 controller self-test failed");
        return Err(DriverError::HardwareInitFailed);
    }

    // Test first port
    outb(0x64, 0xAB);
    wait_for_output();
    if inb(0x60) != 0x00 {
        debug_println!("PS/2 port test failed");
        return Err(DriverError::HardwareInitFailed);
    }

    // Enable first port
    outb(0x64, 0xAE);

    // Reset keyboard
    outb(0x60, 0xFF);
    wait_for_output();
    if inb(0x60) != 0xFA {
        debug_println!("Keyboard reset failed");
        return Err(DriverError::HardwareInitFailed);
    }

    // Wait for BAT completion
    wait_for_output();
    if inb(0x60) != 0xAA {
        debug_println!("Keyboard BAT failed");
        return Err(DriverError::HardwareInitFailed);
    }

    // Enable scanning
    outb(0x60, 0xF4);
    wait_for_output();
    if inb(0x60) != 0xFA {
        debug_println!("Failed to enable scanning");
        return Err(DriverError::HardwareInitFailed);
    }

    // Enable interrupts
    config |= 0x01;
    outb(0x64, 0x60);
    wait_for_input();
    outb(0x60, config);

    debug_println!("PS/2 keyboard initialized successfully");
    Ok(())
}

fn driver_main_loop() -> ! {
    debug_println!("Keyboard driver entering main loop...");
    let shell_ep = EndpointId::new_unchecked(endpoints::SHELL_SERVICE);
    let mut scan_code_buffer = Vec::new();

    loop {
        if inb(0x64) & 0x01 != 0 {
            let scan_code = inb(0x60);
            handle_scan_code(scan_code, &mut scan_code_buffer, shell_ep);
        }
        let _ = syscall::yield_cpu();
    }
}

fn handle_scan_code(scan_code: u8, buffer: &mut Vec<u8>, shell_ep: EndpointId) {
    buffer.push(scan_code);
    if let Some(ascii) = scan_code_to_ascii(scan_code) {
        // Send keypress to shell via IPC
        let mut msg = ServerMessage::new(MessageType::Send);
        msg.set_label(KeyboardOp::KeyPress as u32);
        msg.set_data(0, ascii as u64);
        let _ = syscall::send_message(
            shell_ep,
            &msg.data[0].to_le_bytes(),
        );
        buffer.clear();
    }
}

fn scan_code_to_ascii(scan_code: u8) -> Option<u8> {
    match scan_code {
        0x1C => Some(b'\n'),
        0x0E => Some(0x08), // backspace
        0x39 => Some(b' '),
        0x1E => Some(b'a'), 0x30 => Some(b'b'), 0x2E => Some(b'c'),
        0x20 => Some(b'd'), 0x12 => Some(b'e'), 0x21 => Some(b'f'),
        0x22 => Some(b'g'), 0x23 => Some(b'h'), 0x17 => Some(b'i'),
        0x24 => Some(b'j'), 0x25 => Some(b'k'), 0x26 => Some(b'l'),
        0x32 => Some(b'm'), 0x31 => Some(b'n'), 0x18 => Some(b'o'),
        0x19 => Some(b'p'), 0x10 => Some(b'q'), 0x13 => Some(b'r'),
        0x1F => Some(b's'), 0x14 => Some(b't'), 0x16 => Some(b'u'),
        0x2F => Some(b'v'), 0x11 => Some(b'w'), 0x2D => Some(b'x'),
        0x15 => Some(b'y'), 0x2C => Some(b'z'),
        0x0B => Some(b'0'), 0x02 => Some(b'1'), 0x03 => Some(b'2'),
        0x04 => Some(b'3'), 0x05 => Some(b'4'), 0x06 => Some(b'5'),
        0x07 => Some(b'6'), 0x08 => Some(b'7'), 0x09 => Some(b'8'),
        0x0A => Some(b'9'),
        0x33 => Some(b','), 0x34 => Some(b'.'), 0x35 => Some(b'/'),
        0x27 => Some(b';'), 0x28 => Some(b'\''),
        0x0C => Some(b'-'), 0x0D => Some(b'='),
        _ => None,
    }
}

fn wait_for_input() {
    while inb(0x64) & 0x02 != 0 {}
}

fn wait_for_output() {
    while inb(0x64) & 0x01 == 0 {}
}

fn outb(port: u16, value: u8) {
    unsafe { core::arch::asm!("out dx, al", in("dx") port, in("al") value); }
}

fn inb(port: u16) -> u8 {
    let result: u8;
    unsafe { core::arch::asm!("in al, dx", out("al") result, in("dx") port); }
    result
}

#[derive(Debug)]
enum DriverError {
    RegistrationFailed,
    HardwareInitFailed,
}
