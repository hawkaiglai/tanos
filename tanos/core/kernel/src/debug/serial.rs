//! Serial port driver for debugging

use x86_64::instructions::port::{Port};
use core::fmt;

/// COM port addresses
pub const COM1: u16 = 0x3F8;
pub const COM2: u16 = 0x2F8;
pub const COM3: u16 = 0x3E8;
pub const COM4: u16 = 0x2E8;

/// Serial port
pub struct SerialPort {
    base: u16,
    data: Port<u8>,
    int_enable: Port<u8>,
    fifo_ctrl: Port<u8>,
    line_ctrl: Port<u8>,
    modem_ctrl: Port<u8>,
    line_status: Port<u8>,
    modem_status: Port<u8>,
}

impl SerialPort {
    /// Create new serial port
    pub fn new(base: u16) -> Self {
        Self {
            base,
            data: Port::new(base),
            int_enable: Port::new(base + 1),
            fifo_ctrl: Port::new(base + 2),
            line_ctrl: Port::new(base + 3),
            modem_ctrl: Port::new(base + 4),
            line_status: Port::new(base + 5),
            modem_status: Port::new(base + 6),
        }
    }
    
    /// Initialize serial port
    pub fn init(&mut self) {
        unsafe {
            // Disable interrupts
            self.int_enable.write(0x00);
            
            // Enable DLAB (set baud rate divisor)
            self.line_ctrl.write(0x80);
            
            // Set divisor to 3 (38400 bps)
            self.data.write(0x03);
            self.int_enable.write(0x00);
            
            // 8 bits, no parity, one stop bit
            self.line_ctrl.write(0x03);
            
            // Enable FIFO, clear them, with 14-byte threshold
            self.fifo_ctrl.write(0xC7);
            
            // IRQs enabled, RTS/DSR set
            self.modem_ctrl.write(0x0B);
            
            // Test serial chip (send byte 0xAE and check if same is received)
            self.modem_ctrl.write(0x1E); // Set in loopback mode
            self.data.write(0xAE);
            
            if self.data.read() != 0xAE {
                panic!("Serial port {} is faulty", self.base);
            }
            
            // Set normal operation mode (not loopback)
            self.modem_ctrl.write(0x0F);
        }
    }
    
    /// Check if transmit buffer is empty
    fn is_transmit_empty(&mut self) -> bool {
        unsafe {
            self.line_status.read() & 0x20 != 0
        }
    }
    
    /// Write a byte
    pub fn write_byte(&mut self, byte: u8) {
        // Wait for transmit buffer to be empty
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }
        
        unsafe {
            self.data.write(byte);
        }
    }
    
    /// Write bytes
    pub fn write(&mut self, data: &[u8]) {
        for &byte in data {
            self.write_byte(byte);
        }
    }
    
    /// Check if data is available
    fn is_data_available(&mut self) -> bool {
        unsafe {
            self.line_status.read() & 0x01 != 0
        }
    }
    
    /// Read a byte (blocking)
    pub fn read_byte(&mut self) -> u8 {
        while !self.is_data_available() {
            core::hint::spin_loop();
        }
        
        unsafe {
            self.data.read()
        }
    }
    
    /// Try to read a byte (non-blocking)
    pub fn try_read_byte(&mut self) -> Option<u8> {
        if self.is_data_available() {
            unsafe {
                Some(self.data.read())
            }
        } else {
            None
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write(s.as_bytes());
        Ok(())
    }
}


/// Global serial port for debugging
static mut GLOBAL_SERIAL: Option<SerialPort> = None;

/// Initialize global serial port
pub fn init() {
    unsafe {
        let mut port = SerialPort::new(COM1);
        port.init();
        GLOBAL_SERIAL = Some(port);
    }
}

/// Print to serial port
pub fn print(s: &str) {
    unsafe {
        if let Some(port) = &mut GLOBAL_SERIAL {
            port.write(s.as_bytes());
        }
    }
}

/// Print line to serial port
pub fn println(s: &str) {
    print(s);
    print("\n");
}

/// Print formatted arguments
pub fn print_fmt(args: fmt::Arguments) {
    use fmt::Write;
    unsafe {
        if let Some(port) = &mut GLOBAL_SERIAL {
            let _ = port.write_fmt(args);
        }
    }
}

/// Print panic information
pub fn print_panic(info: &core::panic::PanicInfo) {
    println("\n=== KERNEL PANIC ===");
    if let Some(location) = info.location() {
        print("Location: ");
        print(location.file());
        print(":");
        // Can't easily print numbers without alloc, so skip line number
        println("");
    }
    if let Some(msg) = info.message() {
        print("Message: ");
        print_fmt(*msg);
        println("");
    }
    println("====================");
}
