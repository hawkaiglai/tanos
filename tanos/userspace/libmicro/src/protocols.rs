//! Shared IPC protocol definitions for TanOS userspace services.
//!
//! Well-known endpoint IDs, operation labels, and helper types
//! used by drivers, servers, and applications to communicate.

/// Well-known service endpoint IDs (0–255 reserved).
pub mod endpoints {
    pub const DEVICE_MANAGER: u32 = 1;
    pub const PROCESS_SERVER: u32 = 2;
    pub const MEMORY_SERVER: u32 = 3;
    pub const VFS_SERVER: u32 = 4;
    pub const NETWORK_SERVER: u32 = 5;
    pub const KEYBOARD_SERVICE: u32 = 10;
    pub const DISPLAY_SERVICE: u32 = 11;
    pub const SHELL_SERVICE: u32 = 12;
}

/// Keyboard driver → shell IPC operations.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardOp {
    /// A key was pressed. data[0] = ASCII byte.
    KeyPress = 0x5000,
}

/// Shell/application → VGA driver IPC operations.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayOp {
    /// Write a single character. data[0] = ASCII byte.
    WriteChar = 0x6000,
    /// Write a string. data[0] = ptr, data[1] = len (shared memory).
    WriteString = 0x6001,
    /// Clear the entire screen.
    ClearScreen = 0x6002,
    /// Set text color. data[0] = foreground, data[1] = background.
    SetColor = 0x6003,
}

impl DisplayOp {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0x6000 => Some(Self::WriteChar),
            0x6001 => Some(Self::WriteString),
            0x6002 => Some(Self::ClearScreen),
            0x6003 => Some(Self::SetColor),
            _ => None,
        }
    }
}
