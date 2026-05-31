//! libmicro - TanOS Userspace Standard Library
//! 
//! Provides safe, high-level interfaces for userspace programs to interact
//! with the TanOS microkernel and system services.

#![no_std]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]
#![feature(allocator_api)]

extern crate alloc;

pub mod syscall;
pub mod ipc;
pub mod memory;
pub mod io;
pub mod process;
pub mod sync;
pub mod error;
pub mod protocols;

// Re-export commonly used types
pub use kernel_types::*;
pub use error::{Result, Error};
pub use syscall::{SyscallError, SyscallResult};
pub use ipc::Endpoint;
pub use process::{Process, ProcessInfo};

// Additional IPC types for server development
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Call,
    Reply,
    Send,
    Receive,
}

// Enhanced message structure for server use
#[derive(Debug)]
pub struct ServerMessage {
    pub msg_type: MessageType,
    pub sender: EndpointId,
    pub label: u32,
    pub data: [u64; 8],
}

impl ServerMessage {
    pub fn new(msg_type: MessageType) -> Self {
        Self {
            msg_type,
            sender: EndpointId::new_unchecked(0),
            label: 0,
            data: [0; 8],
        }
    }
    
    pub fn set_label(&mut self, label: u32) {
        self.label = label;
    }
    
    pub fn label(&self) -> u32 {
        self.label
    }
    
    pub fn set_data(&mut self, index: usize, value: u64) {
        if index < self.data.len() {
            self.data[index] = value;
        }
    }
    
    pub fn get_data(&self, index: usize) -> u64 {
        self.data.get(index).copied().unwrap_or(0)
    }
    
    pub fn sender(&self) -> EndpointId {
        self.sender
    }
    
    pub fn success() -> Self {
        let mut msg = Self::new(MessageType::Reply);
        msg.set_data(0, 0); // Success code
        msg
    }
    
    pub fn error(error: ServerError) -> Self {
        let mut msg = Self::new(MessageType::Reply);
        msg.set_data(0, error as u64 | 0x8000_0000_0000_0000); // Error bit
        msg
    }
}

// Server error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ServerError {
    Success = 0,
    InvalidParameters = 1,
    OutOfMemory = 2,
    PermissionDenied = 3,
    ProcessNotFound = 4,
    AllocationNotFound = 5,
    SharedMemoryNotFound = 6,
    AlreadyMapped = 7,
    NotMapped = 8,
    InvalidOperation = 9,
    EndpointNotFound = 10,
    MessageTooLarge = 11,
}

use alloc::alloc::{GlobalAlloc, Layout};
use core::alloc::AllocError;

/// Global allocator for userspace applications
struct UserspaceAllocator;

unsafe impl GlobalAlloc for UserspaceAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        memory::allocate(layout.size(), layout.align())
            .map(|ptr| ptr.as_ptr())
            .unwrap_or(core::ptr::null_mut())
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !ptr.is_null() {
            memory::deallocate(ptr, layout.size(), layout.align());
        }
    }
}

#[global_allocator]
static ALLOCATOR: UserspaceAllocator = UserspaceAllocator;

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("Memory allocation of {} bytes failed", layout.size());
}

/// Initialize libmicro
/// 
/// Must be called before using any other libmicro functions.
/// Typically called from the application's main function.
pub fn init() -> Result<()> {
    // Initialize memory management
    memory::init()?;
    
    // Initialize IPC subsystem
    ipc::init()?;
    
    // Set up signal handling
    process::setup_signal_handling()?;
    
    Ok(())
}

/// Clean shutdown of libmicro
pub fn shutdown() {
    // Clean up IPC resources
    ipc::cleanup();
    
    // Clean up memory management
    memory::cleanup();
}

/// Print debug message to kernel debug output
pub fn debug_print(message: &str) {
    let _ = syscall::debug(message);
}

/// Macro for convenient debug printing
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        {
            use alloc::format;
            $crate::debug_print(&format!($($arg)*));
        }
    };
}

/// Panic handler for userspace
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(location) = info.location() {
        debug_print(&alloc::format!(
            "PANIC at {}:{}: {}", 
            location.file(), 
            location.line(),
            info.message().unwrap_or(&format_args!("no message"))
        ));
    } else {
        debug_print("PANIC: unknown location");
    }
    
    // Exit with error code
    syscall::exit(-1);
}