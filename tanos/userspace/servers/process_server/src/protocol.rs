use kernel_types::ProcessId;

pub const SERVICE_PROCESS_MANAGER: u32 = 2;
pub const REGISTRY_SERVICE: u32 = 1;

#[repr(u32)]
pub enum ProcessOp {
    Spawn = 0x1000,
    Kill = 0x1001,
    Wait = 0x1002,
    GetInfo = 0x1003,
    ListProcesses = 0x1004,
    AllocateMemory = 0x2000,
    FreeMemory = 0x2001,
    ShareMemory = 0x2002,
    GrantCapability = 0x3000,
    RevokeCapability = 0x3001,
}

impl ProcessOp {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0x1000 => Some(Self::Spawn),
            0x1001 => Some(Self::Kill),
            0x1002 => Some(Self::Wait),
            0x1003 => Some(Self::GetInfo),
            0x1004 => Some(Self::ListProcesses),
            0x2000 => Some(Self::AllocateMemory),
            0x2001 => Some(Self::FreeMemory),
            0x2002 => Some(Self::ShareMemory),
            0x3000 => Some(Self::GrantCapability),
            0x3001 => Some(Self::RevokeCapability),
            _ => None,
        }
    }
}

#[repr(u32)]
pub enum RegistryOp {
    Register = 0x100,
    Lookup = 0x101,
    Unregister = 0x102,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcessInfo {
    pub pid: ProcessId,
    pub ppid: ProcessId,
    pub state: ProcessState,
    pub priority: Priority,
    pub cpu_time: u64,
    pub memory_usage: usize,
    pub name: [u8; 32],
}

#[repr(C)]
pub struct SpawnRequest {
    pub elf_data_id: u64,
    pub elf_size: usize,
    pub argv_shm_id: u64,
    pub argc: usize,
    pub env_shm_id: u64,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready = 0,
    Running = 1,
    Blocked = 2,
    Zombie = 3,
    Suspended = 4,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Priority {
    Idle = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}
