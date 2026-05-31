//! Process and thread management types

use core::fmt::{self, Display, Formatter};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Unique identifier for a process
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProcessId(u16);

impl ProcessId {
    /// Invalid process ID
    pub const INVALID: Self = Self(0xFFFF);
    
    /// Create a new process ID
    pub const fn new(id: u16) -> Option<Self> {
        if id == 0xFFFF {
            None
        } else {
            Some(Self(id))
        }
    }
    
    /// Create a new process ID without validation (const version)
    pub const fn new_const(id: u16) -> Self {
        Self(id)
    }
    
    /// Create a process ID from raw value (unsafe)
    pub const unsafe fn from_raw(id: u16) -> Self {
        Self(id)
    }
    
    /// Get the raw process ID value
    pub const fn as_u16(self) -> u16 {
        self.0
    }
    
    /// Get the raw process ID as u32
    pub const fn as_u32(self) -> u32 {
        self.0 as u32
    }
    
    /// Get the raw process ID as u64
    pub const fn as_u64(self) -> u64 {
        self.0 as u64
    }
    
    /// Check if this is a valid process ID
    pub const fn is_valid(self) -> bool {
        self.0 != 0xFFFF
    }
    
    /// Check if this is the kernel process
    pub const fn is_kernel(self) -> bool {
        self.0 == 0
    }
    
    /// Check if this is a userspace process
    pub const fn is_userspace(self) -> bool {
        self.0 > 0 && self.0 != 0xFFFF
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::INVALID
    }
}

impl Display for ProcessId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(f, "PID({})", self.0)
        } else {
            write!(f, "PID(INVALID)")
        }
    }
}

impl From<u16> for ProcessId {
    fn from(id: u16) -> Self {
        Self::new(id).unwrap_or(Self::INVALID)
    }
}

/// Unique identifier for a thread within a process
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ThreadId(u16);

impl ThreadId {
    /// Invalid thread ID
    pub const INVALID: Self = Self(0xFFFF);
    
    /// Main thread ID (always 0 for process main thread)
    pub const MAIN: Self = Self(0);
    
    /// Create a new thread ID
    pub const fn new(id: u16) -> Option<Self> {
        if id == 0xFFFF {
            None
        } else {
            Some(Self(id))
        }
    }
    
    /// Create a new thread ID without validation (const version)
    pub const fn new_const(id: u16) -> Self {
        Self(id)
    }
    
    /// Create a thread ID from raw value (unsafe)
    pub const unsafe fn from_raw(id: u16) -> Self {
        Self(id)
    }
    
    /// Get the raw thread ID value
    pub const fn as_u16(self) -> u16 {
        self.0
    }
    
    /// Get the raw thread ID as u32
    pub const fn as_u32(self) -> u32 {
        self.0 as u32
    }
    
    /// Get the raw thread ID as u64
    pub const fn as_u64(self) -> u64 {
        self.0 as u64
    }
    
    /// Check if this is a valid thread ID
    pub const fn is_valid(self) -> bool {
        self.0 != 0xFFFF
    }
    
    /// Check if this is the main thread
    pub const fn is_main(self) -> bool {
        self.0 == 0
    }
}

impl Default for ThreadId {
    fn default() -> Self {
        Self::INVALID
    }
}

impl Display for ThreadId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(f, "TID({})", self.0)
        } else {
            write!(f, "TID(INVALID)")
        }
    }
}

impl From<u16> for ThreadId {
    fn from(id: u16) -> Self {
        Self::new(id).unwrap_or(Self::INVALID)
    }
}

/// Global thread identifier combining process and thread IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GlobalThreadId {
    /// Process ID
    pub process: ProcessId,
    /// Thread ID within the process
    pub thread: ThreadId,
}

impl GlobalThreadId {
    /// Create a new global thread ID
    pub const fn new(process: ProcessId, thread: ThreadId) -> Self {
        Self { process, thread }
    }
    
    /// Create from raw values
    pub const fn from_raw(process: u16, thread: u16) -> Self {
        Self {
            process: ProcessId::new_const(process),
            thread: ThreadId::new_const(thread),
        }
    }
    
    /// Check if this is a valid global thread ID
    pub const fn is_valid(self) -> bool {
        self.process.is_valid() && self.thread.is_valid()
    }
    
    /// Check if this is a kernel thread
    pub const fn is_kernel(self) -> bool {
        self.process.is_kernel()
    }
    
    /// Check if this is the main thread of a process
    pub const fn is_main_thread(self) -> bool {
        self.thread.is_main()
    }
}

impl Display for GlobalThreadId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.process, self.thread)
    }
}

/// Process state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ProcessState {
    /// Process is ready to run
    Ready = 0,
    /// Process is currently running
    Running = 1,
    /// Process is blocked waiting for IPC
    Blocked = 2,
    /// Process is waiting for a reply
    ReplyWait = 3,
    /// Process is waiting to send
    SendWait = 4,
    /// Process is waiting to receive
    ReceiveWait = 5,
    /// Process has exited
    Exited = 6,
    /// Process has been killed
    Killed = 7,
    /// Process is being created
    Creating = 8,
}

impl ProcessState {
    /// Check if the process can be scheduled
    pub const fn is_schedulable(self) -> bool {
        matches!(self, ProcessState::Ready)
    }
    
    /// Check if the process is running
    pub const fn is_running(self) -> bool {
        matches!(self, ProcessState::Running)
    }
    
    /// Check if the process is blocked
    pub const fn is_blocked(self) -> bool {
        matches!(
            self,
            ProcessState::Blocked
                | ProcessState::ReplyWait
                | ProcessState::SendWait
                | ProcessState::ReceiveWait
        )
    }
    
    /// Check if the process is terminated
    pub const fn is_terminated(self) -> bool {
        matches!(self, ProcessState::Exited | ProcessState::Killed)
    }
}

impl Display for ProcessState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProcessState::Ready => "Ready",
            ProcessState::Running => "Running",
            ProcessState::Blocked => "Blocked",
            ProcessState::ReplyWait => "ReplyWait",
            ProcessState::SendWait => "SendWait",
            ProcessState::ReceiveWait => "ReceiveWait",
            ProcessState::Exited => "Exited",
            ProcessState::Killed => "Killed",
            ProcessState::Creating => "Creating",
        };
        write!(f, "{}", s)
    }
}

/// Process priority type (0 = highest priority, 255 = lowest)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Priority(u8);

impl Priority {
    /// Highest priority (kernel)
    pub const HIGHEST: Self = Self(0);
    
    /// High priority (system services)
    pub const HIGH: Self = Self(50);
    
    /// Normal priority (user applications)
    pub const NORMAL: Self = Self(100);
    
    /// Low priority (background tasks)
    pub const LOW: Self = Self(150);
    
    /// Lowest priority (idle)
    pub const LOWEST: Self = Self(255);
    
    /// Create a new priority
    pub const fn new(priority: u8) -> Self {
        Self(priority)
    }
    
    /// Get the raw priority value
    pub const fn as_u8(self) -> u8 {
        self.0
    }
    
    /// Check if this is a system priority (< 100)
    pub const fn is_system(self) -> bool {
        self.0 < 100
    }
    
    /// Check if this is a user priority (>= 100)
    pub const fn is_user(self) -> bool {
        self.0 >= 100
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl Display for Priority {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Priority({})", self.0)
    }
}

impl From<u8> for Priority {
    fn from(priority: u8) -> Self {
        Self::new(priority)
    }
}

/// CPU affinity mask
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CpuAffinityMask(u64);

impl CpuAffinityMask {
    /// No CPU affinity (can run on any CPU)
    pub const ANY: Self = Self(0);
    
    /// CPU 0 only
    pub const CPU0: Self = Self(1);
    
    /// Create a new CPU affinity mask
    pub const fn new(mask: u64) -> Self {
        Self(mask)
    }
    
    /// Create affinity for specific CPU
    pub const fn cpu(cpu_id: u8) -> Self {
        if cpu_id >= 64 {
            Self::ANY
        } else {
            Self(1u64 << cpu_id)
        }
    }
    
    /// Get the raw mask value
    pub const fn as_u64(self) -> u64 {
        self.0
    }
    
    /// Check if process can run on any CPU
    pub const fn is_any(self) -> bool {
        self.0 == 0
    }
    
    /// Check if process can run on specific CPU
    pub const fn can_run_on(self, cpu_id: u8) -> bool {
        if self.is_any() || cpu_id >= 64 {
            true
        } else {
            (self.0 & (1u64 << cpu_id)) != 0
        }
    }
    
    /// Add CPU to affinity mask
    pub const fn with_cpu(self, cpu_id: u8) -> Self {
        if cpu_id >= 64 {
            self
        } else {
            Self(self.0 | (1u64 << cpu_id))
        }
    }
    
    /// Remove CPU from affinity mask
    pub const fn without_cpu(self, cpu_id: u8) -> Self {
        if cpu_id >= 64 {
            self
        } else {
            Self(self.0 & !(1u64 << cpu_id))
        }
    }
}

impl Default for CpuAffinityMask {
    fn default() -> Self {
        Self::ANY
    }
}
