//! VGA Display Driver for TanOS
//!
//! Text-mode VGA driver. Accepts display commands via IPC from the shell
//! and other applications.

#![no_std]
#![no_main]

#[macro_use]
extern crate libmicro;
extern crate alloc;

use kernel_types::EndpointId;
use libmicro::syscall;
use libmicro::protocols::{endpoints, DisplayOp};
use core::fmt::Write;

// VGA text mode constants
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;
const VGA_BUFFER_ADDR: usize = 0xB8000;

const DEVICE_CLASS_DISPLAY: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

impl Color {
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Black,
            1 => Self::Blue,
            2 => Self::Green,
            3 => Self::Cyan,
            4 => Self::Red,
            5 => Self::Magenta,
            6 => Self::Brown,
            7 => Self::LightGray,
            8 => Self::DarkGray,
            9 => Self::LightBlue,
            10 => Self::LightGreen,
            11 => Self::LightCyan,
            12 => Self::LightRed,
            13 => Self::Pink,
            14 => Self::Yellow,
            _ => Self::White,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

struct VgaBuffer {
    chars: *mut [[ScreenChar; VGA_WIDTH]; VGA_HEIGHT],
}

pub struct VgaWriter {
    column_position: usize,
    row_position: usize,
    color_code: ColorCode,
    buffer: VgaBuffer,
}

impl VgaWriter {
    pub fn new() -> Self {
        VgaWriter {
            column_position: 0,
            row_position: 0,
            color_code: ColorCode::new(Color::Yellow, Color::Black),
            buffer: VgaBuffer {
                chars: VGA_BUFFER_ADDR as *mut [[ScreenChar; VGA_WIDTH]; VGA_HEIGHT],
            },
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            0x08 => {
                // Backspace
                if self.column_position > 0 {
                    self.column_position -= 1;
                    let row = self.row_position;
                    let col = self.column_position;
                    unsafe {
                        (*self.buffer.chars)[row][col] = ScreenChar {
                            ascii_character: b' ',
                            color_code: self.color_code,
                        };
                    }
                }
            }
            byte => {
                if self.column_position >= VGA_WIDTH {
                    self.new_line();
                }

                let row = self.row_position;
                let col = self.column_position;
                let color_code = self.color_code;
                unsafe {
                    (*self.buffer.chars)[row][col] = ScreenChar {
                        ascii_character: byte,
                        color_code,
                    };
                }
                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' | 0x08 => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        if self.row_position >= VGA_HEIGHT - 1 {
            // Scroll up
            for row in 1..VGA_HEIGHT {
                for col in 0..VGA_WIDTH {
                    unsafe {
                        let character = (*self.buffer.chars)[row][col];
                        (*self.buffer.chars)[row - 1][col] = character;
                    }
                }
            }
            self.clear_row(VGA_HEIGHT - 1);
        } else {
            self.row_position += 1;
        }
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..VGA_WIDTH {
            unsafe {
                (*self.buffer.chars)[row][col] = blank;
            }
        }
    }

    pub fn clear_screen(&mut self) {
        for row in 0..VGA_HEIGHT {
            self.clear_row(row);
        }
        self.column_position = 0;
        self.row_position = 0;
    }

    pub fn set_color(&mut self, foreground: Color, background: Color) {
        self.color_code = ColorCode::new(foreground, background);
    }
}

impl Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

static mut VGA_WRITER: Option<VgaWriter> = None;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    if let Err(_) = init_vga_driver() {
        syscall::exit(-1);
    }
    driver_main_loop();
}

fn init_vga_driver() -> core::result::Result<(), DriverError> {
    debug_println!("VGA driver starting...");

    // Initialize VGA writer
    unsafe {
        VGA_WRITER = Some(VgaWriter::new());
    }

    let process_id = syscall::getpid();
    debug_println!("VGA driver PID: {}", process_id.as_u16());

    // Create IPC endpoint for this driver
    let _endpoint = syscall::create_endpoint()
        .map_err(|_| DriverError::RegistrationFailed)?;

    // Register with kernel driver framework
    syscall::register_driver(
        DEVICE_CLASS_DISPLAY,
        _endpoint,
        0,
    ).map_err(|_| DriverError::RegistrationFailed)?;

    // Initialize display
    init_vga_display()?;

    syscall::set_driver_state(1)
        .map_err(|_| DriverError::RegistrationFailed)?;

    debug_println!("VGA driver registered and ready");
    Ok(())
}

fn init_vga_display() -> core::result::Result<(), DriverError> {
    debug_println!("Initializing VGA display...");

    unsafe {
        if let Some(ref mut writer) = VGA_WRITER {
            writer.clear_screen();
            writer.set_color(Color::White, Color::Black);
            writeln!(writer, "TanOS VGA Driver Initialized").unwrap();
            writeln!(writer, "Resolution: {}x{} text mode", VGA_WIDTH, VGA_HEIGHT).unwrap();
        }
    }

    debug_println!("VGA display initialized successfully");
    Ok(())
}

fn driver_main_loop() -> ! {
    debug_println!("VGA driver entering main loop...");

    let display_ep = EndpointId::new_unchecked(endpoints::DISPLAY_SERVICE);
    let mut recv_buf = [0u8; 64];

    loop {
        match syscall::receive_message(display_ep, &mut recv_buf) {
            Ok(_len) => {
                // Decode label from first 4 bytes of a simple encoding:
                // For our IPC, data[0] carries the payload.
                // The label is sent in the message header; here we
                // use a simplified approach where the sender packs
                // label in the message struct and we decode from the buffer.
                //
                // In the real kernel IPC path, labels come via MessageHeader.
                // For this demo, keyboard/shell send data[0] = payload byte
                // and we receive it directly.
                let payload_byte = recv_buf[0];

                // Try to interpret as a display operation via label.
                // Since our simplified IPC currently sends raw data,
                // treat any non-zero byte as WriteChar for the demo path.
                if payload_byte != 0 {
                    handle_write_char(payload_byte);
                }
            }
            Err(_) => {
                let _ = syscall::yield_cpu();
            }
        }
    }
}

fn handle_write_char(ch: u8) {
    unsafe {
        if let Some(ref mut writer) = VGA_WRITER {
            writer.write_byte(ch);
        }
    }
}

fn handle_clear_screen() {
    unsafe {
        if let Some(ref mut writer) = VGA_WRITER {
            writer.clear_screen();
        }
    }
}

fn handle_set_color(fg: u8, bg: u8) {
    unsafe {
        if let Some(ref mut writer) = VGA_WRITER {
            writer.set_color(Color::from_u8(fg), Color::from_u8(bg));
        }
    }
}

#[derive(Debug)]
enum DriverError {
    RegistrationFailed,
}
