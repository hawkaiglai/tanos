#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(missing_docs)]
#![warn(clippy::all)]

//! Core type definitions for TanOS microkernel
//! 
//! This crate provides the fundamental types used throughout the TanOS system,
//! including process identifiers, memory addresses, IPC primitives, capabilities,
//! and error types.

pub mod capability;
pub mod error;
pub mod ipc;
pub mod memory;
pub mod process;

pub use capability::*;
pub use error::*;
pub use ipc::*;
pub use memory::*;
pub use process::*;

// Driver types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DriverId(u32);

impl DriverId {
    pub const fn new(id: u32) -> Self {
        DriverId(id)
    }
    
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Version information for the types crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum number of processes in the system
pub const MAX_PROCESSES: u16 = 4096;

/// Maximum number of threads per process
pub const MAX_THREADS_PER_PROCESS: u16 = 256;

/// Maximum number of capabilities per process
pub const MAX_CAPABILITIES: u32 = 1024;

/// Size of a memory page (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Alignment requirement for pages
pub const PAGE_ALIGN: usize = PAGE_SIZE;

/// Maximum physical memory address (52 bits on x86_64)
pub const MAX_PHYS_ADDR: u64 = (1u64 << 52) - 1;

/// Canonical address space boundary for x86_64
pub const CANONICAL_BOUNDARY: u64 = 0x0001_0000_0000_0000;

/// Well-known process IDs
pub mod well_known {
    use super::ProcessId;
    
    /// Kernel process ID (special case)
    pub const KERNEL: ProcessId = ProcessId::new_const(0);
    
    /// Init process ID (first userspace process)
    pub const INIT: ProcessId = ProcessId::new_const(1);
    
    /// Process server
    pub const PROCESS_SERVER: ProcessId = ProcessId::new_const(2);
    
    /// Memory server
    pub const MEMORY_SERVER: ProcessId = ProcessId::new_const(3);
    
    /// VFS server
    pub const VFS_SERVER: ProcessId = ProcessId::new_const(4);
    
    /// Network stack
    pub const NETWORK_SERVER: ProcessId = ProcessId::new_const(5);
}

/// Well-known endpoint IDs
pub mod well_known_endpoints {
    use super::EndpointId;
    
    /// Device manager endpoint
    pub const DEVICE_MANAGER: EndpointId = EndpointId::well_known(0);
    
    /// Process server endpoint
    pub const PROCESS_SERVER: EndpointId = EndpointId::well_known(1);
    
    /// Memory server endpoint
    pub const MEMORY_SERVER: EndpointId = EndpointId::well_known(2);
    
    /// VFS server endpoint
    pub const VFS_SERVER: EndpointId = EndpointId::well_known(3);
    
    /// Network server endpoint
    pub const NETWORK_SERVER: EndpointId = EndpointId::well_known(4);
}

/// Common result type using TanOS error
pub type Result<T> = core::result::Result<T, Error>;

/// System limits and constants
pub mod limits {
    /// Maximum message size in bytes
    pub const MAX_MESSAGE_SIZE: usize = 4096;
    
    /// Maximum number of endpoints per process
    pub const MAX_ENDPOINTS_PER_PROCESS: u32 = 256;
    
    /// Maximum number of memory regions per process
    pub const MAX_MEMORY_REGIONS: u32 = 128;
    
    /// Maximum process name length
    pub const MAX_PROCESS_NAME_LEN: usize = 32;
    
    /// Default process priority
    pub const DEFAULT_PRIORITY: u8 = 100;
    
    /// Minimum process priority
    pub const MIN_PRIORITY: u8 = 1;
    
    /// Maximum process priority
    pub const MAX_PRIORITY: u8 = 255;
}
