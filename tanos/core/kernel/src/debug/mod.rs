//! Kernel debugging facilities
//! Provides serial output, logging, and debug utilities.

pub mod serial;
pub mod log;
pub mod backtrace;

// Re-export log macros and types for convenience

use spin::Once;

/// Global debug manager
static DEBUG_MANAGER: Once<DebugManager> = Once::new();

/// Debug manager
pub struct DebugManager {
    serial: serial::SerialPort,
    log_level: log::LogLevel,
}

impl DebugManager {
    fn new() -> Self {
        let mut serial = serial::SerialPort::new(serial::COM1);
        serial.init();
        
        Self {
            serial,
            log_level: log::LogLevel::Info,
        }
    }
    
    /// Write to debug output
    pub fn write(&self, data: &[u8]) {
        serial::print(core::str::from_utf8(data).unwrap_or(""));
    }

    /// Write formatted string
    pub fn write_fmt(&self, args: core::fmt::Arguments) {
        serial::print_fmt(args);
    }
    
    /// Set log level
    pub fn set_log_level(&mut self, level: log::LogLevel) {
        self.log_level = level;
    }
    
    /// Get log level
    pub fn log_level(&self) -> log::LogLevel {
        self.log_level
    }
}

/// Initialize debug subsystem
pub fn init() {
    DEBUG_MANAGER.call_once(|| DebugManager::new());
    crate::info!("Debug subsystem initialized");
}

/// Get debug manager
pub fn manager() -> &'static DebugManager {
    DEBUG_MANAGER.get().expect("Debug subsystem not initialized")
}

/// Write to debug output.
/// Routes directly to the global serial port, which is initialized early in
/// boot via `serial::init()`. Does NOT depend on `DEBUG_MANAGER` (which may
/// never be initialized) so that log output works from the very first call.
pub fn write(data: &[u8]) {
    serial::print(core::str::from_utf8(data).unwrap_or(""));
}

/// Write formatted string to debug output.
/// Routes directly to the global serial port (see `write`). Performs no
/// heap allocation, so it is safe during early boot and in the panic handler.
pub fn write_fmt(args: core::fmt::Arguments) {
    serial::print_fmt(args);
}

/// Debug print macro
#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        $crate::debug::write_fmt(format_args!($($arg)*));
    };
}

/// Debug println macro
#[macro_export]
macro_rules! debug_println {
    () => ($crate::debug_print!("\n"));
    ($($arg:tt)*) => ($crate::debug_print!("{}\n", format_args!($($arg)*)));
}
