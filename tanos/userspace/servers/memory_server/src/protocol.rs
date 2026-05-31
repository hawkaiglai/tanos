//! Memory Server Protocol Definitions

use kernel_types::VirtAddr;
use bitflags::bitflags;

pub const SERVICE_MEMORY_MANAGER: u32 = 3;
pub const REGISTRY_SERVICE: u32 = 1;

#[repr(u32)]
pub enum MemoryOp {
    AllocateMemory = 0x2000,
    FreeMemory = 0x2001,
    CreateSharedMemory = 0x2002,
    MapSharedMemory = 0x2003,
    UnmapSharedMemory = 0x2004,
    ProtectMemory = 0x2005,
    GetMemoryInfo = 0x2006,
}

impl MemoryOp {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0x2000 => Some(Self::AllocateMemory),
            0x2001 => Some(Self::FreeMemory),
            0x2002 => Some(Self::CreateSharedMemory),
            0x2003 => Some(Self::MapSharedMemory),
            0x2004 => Some(Self::UnmapSharedMemory),
            0x2005 => Some(Self::ProtectMemory),
            0x2006 => Some(Self::GetMemoryInfo),
            _ => None,
        }
    }
}

#[repr(u32)]
pub enum RegistryOp {
    Register = 0x100,
    _Lookup = 0x101,
    _Unregister = 0x102,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemoryFlags: u32 {
        const READABLE = 0b00000001;
        const WRITABLE = 0b00000010;
        const EXECUTABLE = 0b00000100;
        const SHARED = 0b00001000;
        const DEVICE = 0b00010000;
        const CACHED = 0b00100000;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SharedMemoryFlags: u32 {
        const READ = 0b00000001;
        const WRITE = 0b00000010;
        const EXECUTE = 0b00000100;
        const ANONYMOUS = 0b00001000;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MappingFlags: u32 {
        const READ = 0b00000001;
        const WRITE = 0b00000010;
        const EXECUTE = 0b00000100;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AllocationInfo {
    pub id: AllocationId,
    pub vaddr: VirtAddr,
    pub size: usize,
    pub flags: MemoryFlags,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryInfo {
    pub total_allocated: usize,
    pub allocation_count: usize,
    pub virtual_size: usize,
    pub physical_used: usize,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AllocationId(pub u32);

impl AllocationId {
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SharedMemoryId(pub u32);

impl SharedMemoryId {
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }

    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}
