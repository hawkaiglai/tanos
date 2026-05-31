//! Kernel logging system

use core::fmt;
use spin::Mutex;

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN ",
            LogLevel::Info => "INFO ",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
    
    pub fn color_code(&self) -> &'static str {
        match self {
            LogLevel::Error => "\x1b[31m", // Red
            LogLevel::Warn => "\x1b[33m",  // Yellow
            LogLevel::Info => "\x1b[32m",  // Green
            LogLevel::Debug => "\x1b[36m", // Cyan
            LogLevel::Trace => "\x1b[37m", // White
        }
    }
}

/// Logger implementation
struct Logger {
    level: LogLevel,
    use_colors: bool,
}

static LOGGER: Mutex<Logger> = Mutex::new(Logger {
    level: LogLevel::Info,
    use_colors: true,
});

impl Logger {
    fn log(&self, level: LogLevel, args: fmt::Arguments, file: &str, line: u32) {
        if level <= self.level {
            let timestamp = crate::interrupt::INTERRUPT_MANAGER
                .get()
                .map(|m| m.timer().uptime_ms())
                .unwrap_or(0);
            
            if self.use_colors {
                crate::debug::write_fmt(format_args!(
                    "{}[{:8}.{:03}] {} {}:{} - {}\x1b[0m\n",
                    level.color_code(),
                    timestamp / 1000,
                    timestamp % 1000,
                    level.as_str(),
                    file,
                    line,
                    args
                ));
            } else {
                crate::debug::write_fmt(format_args!(
                    "[{:8}.{:03}] {} {}:{} - {}\n",
                    timestamp / 1000,
                    timestamp % 1000,
                    level.as_str(),
                    file,
                    line,
                    args
                ));
            }
        }
    }
    
    fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }
    
    fn set_colors(&mut self, use_colors: bool) {
        self.use_colors = use_colors;
    }
}

/// Log a message
pub fn log(level: LogLevel, args: fmt::Arguments, file: &str, line: u32) {
    let logger = LOGGER.lock();
    logger.log(level, args, file, line);
}

/// Set log level
pub fn set_level(level: LogLevel) {
    let mut logger = LOGGER.lock();
    logger.set_level(level);
}

/// Enable/disable colored output
pub fn set_colors(use_colors: bool) {
    let mut logger = LOGGER.lock();
    logger.set_colors(use_colors);
}

/// Get current log level
pub fn level() -> LogLevel {
    let logger = LOGGER.lock();
    logger.level
}

/// Logging macros
#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        $crate::debug::log::log(
            $level,
            format_args!($($arg)*),
            file!(),
            line!()
        );
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::log!($crate::debug::log::LogLevel::Error, $($arg)*);
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::log!($crate::debug::log::LogLevel::Warn, $($arg)*);
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::log!($crate::debug::log::LogLevel::Info, $($arg)*);
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::log!($crate::debug::log::LogLevel::Debug, $($arg)*);
    };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        $crate::log!($crate::debug::log::LogLevel::Trace, $($arg)*);
    };
}
