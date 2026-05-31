//! Capability-based security system types

use core::fmt::{self, Display, Formatter};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{EndpointId, PhysAddr, ProcessId, SharedMemoryId};

/// Unique identifier for a capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CapabilityId(u32);

impl CapabilityId {
    /// Invalid capability ID
    pub const INVALID: Self = Self(0);
    
    /// Create a new capability ID
    pub const fn new(id: u32) -> Option<Self> {
        if id == 0 {
            None
        } else {
            Some(Self(id))
        }
    }
    
    /// Create a capability ID without validation
    pub const fn new_unchecked(id: u32) -> Self {
        Self(id)
    }
    
    /// Get the raw capability ID value
    pub const fn as_u32(self) -> u32 {
        self.0
    }
    
    /// Get the capability ID as u64
    pub const fn as_u64(self) -> u64 {
        self.0 as u64
    }
    
    /// Check if this is a valid capability ID
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

impl Default for CapabilityId {
    fn default() -> Self {
        Self::INVALID
    }
}

impl Display for CapabilityId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(f, "Cap({})", self.0)
        } else {
            write!(f, "Cap(INVALID)")
        }
    }
}

/// Types of resources that can be referenced by capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ResourceType {
    /// IPC endpoint
    Endpoint = 1,
    /// Memory region
    Memory = 2,
    /// Hardware interrupt
    Irq = 3,
    /// I/O port range
    IoPort = 4,
    /// Process control
    Process = 5,
    /// Physical memory frame
    Frame = 6,
    /// Shared memory object
    SharedMemory = 7,
    /// Device memory mapping
    DeviceMemory = 8,
    /// Timer resource
    Timer = 9,
    /// Debug interface
    Debug = 10,
}

impl ResourceType {
    /// Create from raw u32 value
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::Endpoint),
            2 => Some(Self::Memory),
            3 => Some(Self::Irq),
            4 => Some(Self::IoPort),
            5 => Some(Self::Process),
            6 => Some(Self::Frame),
            7 => Some(Self::SharedMemory),
            8 => Some(Self::DeviceMemory),
            9 => Some(Self::Timer),
            10 => Some(Self::Debug),
            _ => None,
        }
    }
    
    /// Get the raw u32 value
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

impl Display for ResourceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            ResourceType::Endpoint => "Endpoint",
            ResourceType::Memory => "Memory",
            ResourceType::Irq => "Irq",
            ResourceType::IoPort => "IoPort",
            ResourceType::Process => "Process",
            ResourceType::Frame => "Frame",
            ResourceType::SharedMemory => "SharedMemory",
            ResourceType::DeviceMemory => "DeviceMemory",
            ResourceType::Timer => "Timer",
            ResourceType::Debug => "Debug",
        };
        write!(f, "{}", s)
    }
}

/// Rights/permissions associated with a capability
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct Rights: u32 {
        /// No rights
        const NONE = 0;
        
        /// Read access
        const READ = 1 << 0;
        
        /// Write access
        const WRITE = 1 << 1;
        
        /// Execute access
        const EXECUTE = 1 << 2;
        
        /// Grant capability to others
        const GRANT = 1 << 3;
        
        /// Delete/destroy the resource
        const DELETE = 1 << 4;
        
        /// Map memory
        const MAP = 1 << 5;
        
        /// Unmap memory
        const UNMAP = 1 << 6;
        
        /// Send messages (for endpoints)
        const SEND = 1 << 7;
        
        /// Receive messages (for endpoints)
        const RECEIVE = 1 << 8;
        
        /// Call operation (send + receive)
        const CALL = Self::SEND.bits() | Self::RECEIVE.bits();
        
        /// Reply to messages
        const REPLY = 1 << 9;
        
        /// Control process (kill, suspend, etc.)
        const CONTROL = 1 << 10;
        
        /// Debug process
        const DEBUG = 1 << 11;
        
        /// Allocate from resource
        const ALLOCATE = 1 << 12;
        
        /// Configure resource
        const CONFIGURE = 1 << 13;
        
        /// Share resource with others
        const SHARE = 1 << 14;
        
        /// All rights (administrative access)
        const ALL = u32::MAX;
    }
}

impl Rights {
    /// Create rights for read-only access
    pub const fn read_only() -> Self {
        Self::READ
    }
    
    /// Create rights for read-write access
    pub const fn read_write() -> Self {
        Rights::from_bits_truncate(Self::READ.bits() | Self::WRITE.bits())
    }
    
    /// Create rights for full access
    pub const fn full_access() -> Self {
        Rights::from_bits_truncate(Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits())
    }
    
    /// Create rights for IPC send only
    pub const fn send_only() -> Self {
        Self::SEND
    }
    
    /// Create rights for IPC receive only
    pub const fn receive_only() -> Self {
        Self::RECEIVE
    }
    
    /// Create rights for IPC call (send + receive)
    pub const fn call_access() -> Self {
        Self::CALL
    }
    
    /// Check if rights allow reading
    pub const fn can_read(self) -> bool {
        self.contains(Self::READ)
    }
    
    /// Check if rights allow writing
    pub const fn can_write(self) -> bool {
        self.contains(Self::WRITE)
    }
    
    /// Check if rights allow execution
    pub const fn can_execute(self) -> bool {
        self.contains(Self::EXECUTE)
    }
    
    /// Check if rights allow granting to others
    pub const fn can_grant(self) -> bool {
        self.contains(Self::GRANT)
    }
    
    /// Check if rights allow deletion
    pub const fn can_delete(self) -> bool {
        self.contains(Self::DELETE)
    }
    
    /// Check if rights allow IPC operations
    pub const fn can_ipc(self) -> bool {
        self.intersects(Rights::from_bits_truncate(Self::SEND.bits() | Self::RECEIVE.bits()))
    }
    
    /// Derive new rights by removing some permissions
    pub const fn derive(self, remove_rights: Rights) -> Self {
        Rights::from_bits_truncate(self.bits() & !remove_rights.bits())
    }
    
    /// Intersect with other rights (keep only common rights)
    pub const fn intersect(self, other: Rights) -> Self {
        Rights::from_bits_truncate(self.bits() & other.bits())
    }
}

impl Display for Rights {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "None");
        }
        
        let mut first = true;
        
        macro_rules! write_right {
            ($flag:expr, $name:expr) => {
                if self.contains($flag) {
                    if !first { write!(f, "|")?; }
                    write!(f, $name)?;
                    first = false;
                }
            };
        }
        
        write_right!(Rights::READ, "R");
        write_right!(Rights::WRITE, "W");
        write_right!(Rights::EXECUTE, "X");
        write_right!(Rights::GRANT, "G");
        write_right!(Rights::DELETE, "D");
        write_right!(Rights::MAP, "M");
        write_right!(Rights::SEND, "S");
        write_right!(Rights::RECEIVE, "Rc");
        write_right!(Rights::CONTROL, "C");
        
        Ok(())
    }
}

/// A capability grants rights to access a specific resource
#[derive(Debug, Clone, Copy)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Capability {
    /// Unique capability identifier
    pub id: CapabilityId,
    /// Type of resource this capability grants access to
    pub resource_type: ResourceType,
    /// Rights/permissions granted by this capability
    pub rights: Rights,
    /// ID of the specific resource instance
    pub resource_id: u64,
    /// Additional data specific to resource type
    pub data: CapabilityData,
}

/// Additional data stored with a capability
#[derive(Clone, Copy)]
#[repr(C)]
pub union CapabilityData {
    /// Raw data (8 bytes)
    pub raw: u64,
    /// For memory capabilities
    pub memory: MemoryCapData,
    /// For endpoint capabilities  
    pub endpoint: EndpointCapData,
    /// For IRQ capabilities
    pub irq: IrqCapData,
    /// For I/O port capabilities
    pub ioport: IoPortCapData,
    /// For process capabilities
    pub process: ProcessCapData,
}

/// Memory capability data
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryCapData {
    /// Size of the memory region
    pub size: u32,
    /// Memory protection flags
    pub protection: u32,
}

/// Endpoint capability data
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EndpointCapData {
    /// Endpoint ID
    pub endpoint_id: EndpointId,
    /// Reserved for future use
    pub reserved: u32,
}

/// IRQ capability data
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IrqCapData {
    /// IRQ number
    pub irq_number: u8,
    /// IRQ flags/configuration
    pub flags: u8,
    /// Reserved
    pub reserved: [u8; 6],
}

/// I/O port capability data
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IoPortCapData {
    /// Starting port number
    pub start_port: u16,
    /// Number of consecutive ports
    pub port_count: u16,
    /// Reserved
    pub reserved: u32,
}

/// Process capability data
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ProcessCapData {
    /// Target process ID
    pub process_id: ProcessId,
    /// Reserved
    pub reserved: [u8; 6],
}

impl Capability {
    /// Create a new capability
    pub const fn new(
        id: CapabilityId,
        resource_type: ResourceType,
        rights: Rights,
        resource_id: u64,
    ) -> Self {
        Self {
            id,
            resource_type,
            rights,
            resource_id,
            data: CapabilityData { raw: 0 },
        }
    }
    
    /// Create an endpoint capability
    pub fn endpoint(
        id: CapabilityId,
        endpoint_id: EndpointId,
        rights: Rights,
    ) -> Self {
        let mut cap = Self::new(id, ResourceType::Endpoint, rights, endpoint_id.as_u64());
        cap.data.endpoint = EndpointCapData {
            endpoint_id,
            reserved: 0,
        };
        cap
    }
    
    /// Create a memory capability
    pub fn memory(
        id: CapabilityId,
        phys_addr: PhysAddr,
        size: u32,
        rights: Rights,
        protection: u32,
    ) -> Self {
        let mut cap = Self::new(id, ResourceType::Memory, rights, phys_addr.as_u64());
        cap.data.memory = MemoryCapData { size, protection };
        cap
    }
    
    /// Create an IRQ capability
    pub fn irq(
        id: CapabilityId,
        irq_number: u8,
        rights: Rights,
    ) -> Self {
        let mut cap = Self::new(id, ResourceType::Irq, rights, irq_number as u64);
        cap.data.irq = IrqCapData {
            irq_number,
            flags: 0,
            reserved: [0; 6],
        };
        cap
    }
    
    /// Create an I/O port capability
    pub fn ioport(
        id: CapabilityId,
        start_port: u16,
        port_count: u16,
        rights: Rights,
    ) -> Self {
        let mut cap = Self::new(id, ResourceType::IoPort, rights, start_port as u64);
        cap.data.ioport = IoPortCapData {
            start_port,
            port_count,
            reserved: 0,
        };
        cap
    }
    
    /// Create a process capability
    pub fn process(
        id: CapabilityId,
        process_id: ProcessId,
        rights: Rights,
    ) -> Self {
        let mut cap = Self::new(id, ResourceType::Process, rights, process_id.as_u64());
        cap.data.process = ProcessCapData {
            process_id,
            reserved: [0; 6],
        };
        cap
    }
    
    /// Create a shared memory capability
    pub fn shared_memory(
        id: CapabilityId,
        shared_mem_id: SharedMemoryId,
        rights: Rights,
    ) -> Self {
        Self::new(id, ResourceType::SharedMemory, rights, shared_mem_id.as_u32() as u64)
    }
    
    /// Check if this capability is valid
    pub const fn is_valid(self) -> bool {
        self.id.is_valid()
    }
    
    /// Check if this capability allows a specific right
    pub const fn has_right(self, right: Rights) -> bool {
        self.rights.contains(right)
    }
    
    /// Check if this capability can be granted to others
    pub const fn can_grant(self) -> bool {
        self.has_right(Rights::GRANT)
    }
    
    /// Derive a new capability with reduced rights
    pub const fn derive(self, new_rights: Rights) -> Self {
        Self {
            id: self.id,
            resource_type: self.resource_type,
            rights: self.rights.intersect(new_rights),
            resource_id: self.resource_id,
            data: self.data,
        }
    }
    
    /// Get the endpoint ID if this is an endpoint capability
    pub fn endpoint_id(self) -> Option<EndpointId> {
        match self.resource_type {
            ResourceType::Endpoint => unsafe { Some(self.data.endpoint.endpoint_id) },
            _ => None,
        }
    }
    
    /// Get the memory size if this is a memory capability
    pub fn memory_size(self) -> Option<u32> {
        match self.resource_type {
            ResourceType::Memory => unsafe { Some(self.data.memory.size) },
            _ => None,
        }
    }
    
    /// Get the IRQ number if this is an IRQ capability
    pub fn irq_number(self) -> Option<u8> {
        match self.resource_type {
            ResourceType::Irq => unsafe { Some(self.data.irq.irq_number) },
            _ => None,
        }
    }
    
    /// Get the process ID if this is a process capability
    pub fn process_id(self) -> Option<ProcessId> {
        match self.resource_type {
            ResourceType::Process => unsafe { Some(self.data.process.process_id) },
            _ => None,
        }
    }
}

impl fmt::Debug for CapabilityData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Safety: Reading as raw u64 is always safe
        unsafe {
            f.debug_struct("CapabilityData")
                .field("raw", &self.raw)
                .finish()
        }
    }
}

impl Default for CapabilityData {
    fn default() -> Self {
        Self { raw: 0 }
    }
}

impl Display for Capability {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{} {} ({})",
            self.id,
            self.resource_type,
            self.rights,
            self.resource_id
        )
    }
}

/// Set of capabilities owned by a process
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    capabilities: [Option<Capability>; crate::MAX_CAPABILITIES as usize],
    count: usize,
}

impl CapabilitySet {
    /// Create a new empty capability set
    pub const fn new() -> Self {
        Self {
            capabilities: [None; crate::MAX_CAPABILITIES as usize],
            count: 0,
        }
    }
    
    /// Create a root capability set (for init process)
    pub fn root() -> Self {
        let mut set = Self::new();
        
        // Add basic system capabilities
        let _ = set.add(Capability::endpoint(
            CapabilityId::new_unchecked(1),
            crate::well_known_endpoints::PROCESS_SERVER,
            Rights::CALL,
        ));
        
        let _ = set.add(Capability::endpoint(
            CapabilityId::new_unchecked(2),
            crate::well_known_endpoints::MEMORY_SERVER,
            Rights::CALL,
        ));
        
        let _ = set.add(Capability::endpoint(
            CapabilityId::new_unchecked(3),
            crate::well_known_endpoints::VFS_SERVER,
            Rights::CALL,
        ));
        
        set
    }
    
    /// Add a capability to the set
    pub fn add(&mut self, capability: Capability) -> Result<(), ()> {
        if self.count >= crate::MAX_CAPABILITIES as usize {
            return Err(());
        }
        
        // Find empty slot
        for slot in &mut self.capabilities {
            if slot.is_none() {
                *slot = Some(capability);
                self.count += 1;
                return Ok(());
            }
        }
        
        Err(())
    }
    
    /// Remove a capability by ID
    pub fn remove(&mut self, id: CapabilityId) -> Option<Capability> {
        for slot in &mut self.capabilities {
            if let Some(cap) = slot {
                if cap.id == id {
                    let result = *cap;
                    *slot = None;
                    self.count -= 1;
                    return Some(result);
                }
            }
        }
        None
    }
    
    /// Find a capability by ID
    pub fn find(&self, id: CapabilityId) -> Option<&Capability> {
        for slot in &self.capabilities {
            if let Some(cap) = slot {
                if cap.id == id {
                    return Some(cap);
                }
            }
        }
        None
    }
    
    /// Find a capability by resource type and ID
    pub fn find_by_resource(&self, resource_type: ResourceType, resource_id: u64) -> Option<&Capability> {
        for slot in &self.capabilities {
            if let Some(cap) = slot {
                if cap.resource_type == resource_type && cap.resource_id == resource_id {
                    return Some(cap);
                }
            }
        }
        None
    }
    
    /// Get the number of capabilities in the set
    pub const fn len(&self) -> usize {
        self.count
    }
    
    /// Check if the capability set is empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    /// Check if the capability set is full
    pub const fn is_full(&self) -> bool {
        self.count >= crate::MAX_CAPABILITIES as usize
    }
    
    /// Inherit capabilities from parent (create derived capabilities)
    pub fn inherit(&self) -> Self {
        let mut inherited = Self::new();
        
        for slot in &self.capabilities {
            if let Some(cap) = slot {
                // Only inherit capabilities with GRANT right
                if cap.has_right(Rights::GRANT) {
                    // Create derived capability with reduced rights
                    let derived = cap.derive(cap.rights.derive(Rights::GRANT));
                    let _ = inherited.add(derived);
                }
            }
        }
        
        inherited
    }
    
    /// Clear all capabilities
    pub fn clear(&mut self) {
        self.capabilities = [None; crate::MAX_CAPABILITIES as usize];
        self.count = 0;
    }
    
    /// Iterate over all capabilities
    pub fn iter(&self) -> CapabilityIterator {
        CapabilityIterator {
            capabilities: &self.capabilities,
            index: 0,
        }
    }
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over capabilities in a capability set
pub struct CapabilityIterator<'a> {
    capabilities: &'a [Option<Capability>; crate::MAX_CAPABILITIES as usize],
    index: usize,
}

impl<'a> Iterator for CapabilityIterator<'a> {
    type Item = &'a Capability;
    
    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.capabilities.len() {
            if let Some(ref cap) = self.capabilities[self.index] {
                self.index += 1;
                return Some(cap);
            }
            self.index += 1;
        }
        None
    }
}
