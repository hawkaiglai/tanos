//! Capability-based security system
//! Provides fine-grained access control using capabilities.

use spin::Mutex;
use crate::ProcessId;
use crate::EndpointId;
use crate::{VirtAddr};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::Once;
use core::sync::atomic::{AtomicU32, Ordering};

/// Global capability manager
static CAPABILITY_MANAGER: Once<CapabilityManager> = Once::new();

/// Capability identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct CapabilityId(u32);

impl CapabilityId {
    pub const INVALID: Self = Self(0);
    
    pub fn new(id: u32) -> Self {
        Self(id)
    }
    
    pub fn as_u32(self) -> u32 {
        self.0
    }
    
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

/// Capability rights
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct Rights: u32 {
        const NONE    = 0;
        const READ    = 1 << 0;
        const WRITE   = 1 << 1;
        const EXECUTE = 1 << 2;
        const GRANT   = 1 << 3;
        const DELETE  = 1 << 4;
        const ADMIN   = 1 << 5;
    }
}

/// Resource type
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Endpoint = 1,
    Memory = 2,
    Irq = 3,
    IoPort = 4,
    Process = 5,
    Thread = 6,
    Frame = 7,
}

/// Capability
#[derive(Debug, Clone)]
pub struct Capability {
    pub id: CapabilityId,
    pub resource_type: ResourceType,
    pub resource_id: u64,
    pub rights: Rights,
    pub owner: ProcessId,
    pub derived_from: Option<CapabilityId>,
    pub creation_time: u64,
}

impl Capability {
    fn new(
        id: CapabilityId,
        resource_type: ResourceType,
        resource_id: u64,
        rights: Rights,
        owner: ProcessId,
        derived_from: Option<CapabilityId>,
    ) -> Self {
        Self {
            id,
            resource_type,
            resource_id,
            rights,
            owner,
            derived_from,
            creation_time: crate::interrupt::rdtsc(),
        }
    }
}

/// Process capability set
#[derive(Debug, Default)]
pub struct CapabilitySet {
    capabilities: BTreeMap<CapabilityId, Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            capabilities: BTreeMap::new(),
        }
    }
    
    /// Create root capability set (for init process)
    pub fn root() -> Self {
        let set = Self::new();
        
        // Add capabilities for essential resources
        // These will be assigned proper IDs by the capability manager
        
        set
    }
    
    /// Inherit capabilities from parent (filtered)
    pub fn inherit(&self) -> Self {
        let mut new_set = Self::new();
        
        // Only inherit capabilities with GRANT right
        for (id, cap) in &self.capabilities {
            if cap.rights.contains(Rights::GRANT) {
                // Create derived capability with reduced rights
                let mut new_cap = cap.clone();
                new_cap.rights.remove(Rights::GRANT); // Remove grant right from child
                new_set.capabilities.insert(*id, new_cap);
            }
        }
        
        new_set
    }
    
    /// Add capability
    pub fn add(&mut self, capability: Capability) {
        self.capabilities.insert(capability.id, capability);
    }
    
    /// Remove capability
    pub fn remove(&mut self, id: CapabilityId) -> Option<Capability> {
        self.capabilities.remove(&id)
    }
    
    /// Get capability
    pub fn get(&self, id: CapabilityId) -> Option<&Capability> {
        self.capabilities.get(&id)
    }
    
    /// Check if has capability for resource
    pub fn has_access(&self, resource_type: ResourceType, resource_id: u64, required_rights: Rights) -> bool {
        self.capabilities.values().any(|cap| {
            cap.resource_type == resource_type &&
            cap.resource_id == resource_id &&
            cap.rights.contains(required_rights)
        })
    }
    
    /// List all capabilities
    pub fn list(&self) -> Vec<&Capability> {
        self.capabilities.values().collect()
    }
}

/// Capability manager
pub struct CapabilityManager {
    capabilities: Mutex<BTreeMap<CapabilityId, Capability>>,
    process_capabilities: Mutex<BTreeMap<ProcessId, CapabilitySet>>,
    next_capability_id: AtomicU32,
}

impl CapabilityManager {
    fn new() -> Self {
        Self {
            capabilities: Mutex::new(BTreeMap::new()),
            process_capabilities: Mutex::new(BTreeMap::new()),
            next_capability_id: AtomicU32::new(1),
        }
    }
    
    /// Create a new capability
    pub fn create_capability(
        &self,
        resource_type: ResourceType,
        resource_id: u64,
        rights: Rights,
        owner: ProcessId,
    ) -> core::result::Result<CapabilityId, CapabilityError> {
        let id = CapabilityId(self.next_capability_id.fetch_add(1, Ordering::SeqCst));
        
        let capability = Capability::new(
            id,
            resource_type,
            resource_id,
            rights,
            owner,
            None,
        );
        
        // Store capability
        self.capabilities.lock().insert(id, capability.clone());
        
        // Add to process capability set
        self.process_capabilities.lock()
            .entry(owner)
            .or_insert_with(CapabilitySet::new)
            .add(capability);
        
        crate::debug!("Created capability {} for process {} ({:?} on {})", 
                   id.0, owner, resource_type, resource_id);
        
        Ok(id)
    }
    
    /// Derive a capability (create child capability with reduced rights)
    pub fn derive_capability(
        &self,
        parent_id: CapabilityId,
        new_rights: Rights,
        new_owner: ProcessId,
        requester: ProcessId,
    ) -> core::result::Result<CapabilityId, CapabilityError> {
        let parent_info = {
            let capabilities = self.capabilities.lock();
            let parent = capabilities.get(&parent_id)
                .ok_or(CapabilityError::NotFound)?;

            // Check if requester owns the parent capability
            if parent.owner != requester {
                return Err(CapabilityError::AccessDenied);
            }

            // Check if parent has GRANT right
            if !parent.rights.contains(Rights::GRANT) {
                return Err(CapabilityError::InsufficientRights);
            }

            // Check if new rights are a subset of parent rights
            if !parent.rights.contains(new_rights) {
                return Err(CapabilityError::InsufficientRights);
            }

            (parent.resource_type, parent.resource_id)
        };
        
        let id = CapabilityId(self.next_capability_id.fetch_add(1, Ordering::SeqCst));
        
        let capability = Capability::new(
            id,
            parent_info.0,
            parent_info.1,
            new_rights,
            new_owner,
            Some(parent_id),
        );
        
        // Store capability
        self.capabilities.lock().insert(id, capability.clone());
        
        // Add to new owner's capability set
        self.process_capabilities.lock()
            .entry(new_owner)
            .or_insert_with(CapabilitySet::new)
            .add(capability);
        
        crate::debug!("Derived capability {} from {} for process {}", 
                   id.0, parent_id.0, new_owner);
        
        Ok(id)
    }
    
    /// Revoke a capability
    pub fn revoke_capability(
        &self,
        capability_id: CapabilityId,
        requester: ProcessId,
    ) -> core::result::Result<(), CapabilityError> {
        let mut capabilities = self.capabilities.lock();
        let capability = capabilities.get(&capability_id)
            .ok_or(CapabilityError::NotFound)?;
        
        // Check if requester can revoke this capability
        let can_revoke = capability.owner == requester ||
                        capability.derived_from.map_or(false, |parent_id| {
                            capabilities.get(&parent_id)
                                .map_or(false, |parent| parent.owner == requester)
                        });
        
        if !can_revoke {
            return Err(CapabilityError::AccessDenied);
        }
        
        let owner = capability.owner;
        let _resource_type = capability.resource_type;
        let _resource_id = capability.resource_id;
        
        // Remove from global store
        capabilities.remove(&capability_id);
        drop(capabilities);
        
        // Remove from process capability set
        if let Some(cap_set) = self.process_capabilities.lock().get_mut(&owner) {
            cap_set.remove(capability_id);
        }
        
        // Revoke all derived capabilities
        self.revoke_derived_capabilities(capability_id);
        
        crate::debug!("Revoked capability {} from process {}", capability_id.0, owner);
        
        Ok(())
    }
    
    /// Revoke all capabilities derived from a parent
    fn revoke_derived_capabilities(&self, parent_id: CapabilityId) {
        let capabilities = self.capabilities.lock();
        let derived_ids: Vec<CapabilityId> = capabilities.values()
            .filter(|cap| cap.derived_from == Some(parent_id))
            .map(|cap| cap.id)
            .collect();
        drop(capabilities);
        
        for derived_id in derived_ids {
            let _ = self.revoke_capability(derived_id, crate::KERNEL_PID);
        }
    }
    
    /// Check if process has access to endpoint
    pub fn has_endpoint_access(
        &self,
        process: ProcessId,
        endpoint: EndpointId,
        required_rights: Rights,
    ) -> bool {
        if let Some(cap_set) = self.process_capabilities.lock().get(&process) {
            cap_set.has_access(ResourceType::Endpoint, endpoint.as_u64(), required_rights)
        } else {
            false
        }
    }
    
    /// Check if process has access to memory
    pub fn has_memory_access(
        &self,
        process: ProcessId,
        address: VirtAddr,
        size: usize,
        required_rights: Rights,
    ) -> bool {
        if let Some(cap_set) = self.process_capabilities.lock().get(&process) {
            // Check for capabilities covering the memory range
            let start_addr = address.as_u64();
            let _end_addr = start_addr + size as u64;
            
            // Find overlapping memory capabilities
            cap_set.capabilities.values().any(|cap| {
                if cap.resource_type != ResourceType::Memory {
                    return false;
                }
                
                // For simplicity, we store base address in resource_id
                // In a real implementation, we'd have a more sophisticated memory capability format
                let cap_addr = cap.resource_id;
                
                cap_addr <= start_addr && cap.rights.contains(required_rights)
            })
        } else {
            false
        }
    }
    
    /// Verify IRQ capability
    pub fn verify_irq_capability(
        &self,
        process: ProcessId,
        capability_id: CapabilityId,
        irq: u8,
    ) -> bool {
        if let Some(cap_set) = self.process_capabilities.lock().get(&process) {
            if let Some(capability) = cap_set.get(capability_id) {
                capability.resource_type == ResourceType::Irq &&
                capability.resource_id == irq as u64 &&
                capability.rights.contains(Rights::ADMIN)
            } else {
                false
            }
        } else {
            false
        }
    }
    
    /// Create initial capabilities for a process
    pub fn create_initial_capabilities(&self, process: ProcessId) -> CapabilitySet {
        let cap_set = CapabilitySet::new();
        
        // Create basic capabilities for the process
        if let Ok(_process_cap_id) = self.create_capability(
            ResourceType::Process,
            process.as_u64(),
            Rights::READ | Rights::WRITE,
            process,
        ) {
            // The capability is already added to the process set by create_capability
        }
        
        cap_set
    }
    
    /// Transfer capability to another process
    pub fn transfer_capability(
        &self,
        capability_id: CapabilityId,
        from_process: ProcessId,
        to_process: ProcessId,
    ) -> core::result::Result<(), CapabilityError> {
        let mut process_caps = self.process_capabilities.lock();
        
        // Remove from source process
        let from_caps = process_caps.get_mut(&from_process)
            .ok_or(CapabilityError::NotFound)?;
        
        let mut capability = from_caps.remove(capability_id)
            .ok_or(CapabilityError::NotFound)?;
        
        // Update owner
        capability.owner = to_process;
        
        // Add to destination process
        process_caps.entry(to_process)
            .or_insert_with(CapabilitySet::new)
            .add(capability.clone());
        
        // Update global store
        self.capabilities.lock().insert(capability_id, capability);
        
        crate::debug!("Transferred capability {} from process {} to process {}", 
                   capability_id.0, from_process, to_process);
        
        Ok(())
    }
    
    /// Get process capabilities
    pub fn get_process_capabilities(&self, process: ProcessId) -> Option<Vec<Capability>> {
        self.process_capabilities.lock()
            .get(&process)
            .map(|cap_set| cap_set.list().into_iter().cloned().collect())
    }
    
    /// Remove all capabilities for a process (on process death)
    pub fn remove_process_capabilities(&self, process: ProcessId) {
        let cap_set = match self.process_capabilities.lock().remove(&process) {
            Some(set) => set,
            None => return,
        };

        // Collect the ids first, then remove them from the global store under a
        // short-lived lock, and only AFTER releasing it revoke any derived
        // capabilities. revoke_derived_capabilities re-locks `capabilities`, and
        // spin::Mutex is NOT reentrant, so holding that lock across the call
        // would deadlock.
        let ids: Vec<CapabilityId> = cap_set.list().into_iter().map(|c| c.id).collect();
        {
            let mut global_caps = self.capabilities.lock();
            for id in &ids {
                global_caps.remove(id);
            }
        }
        for id in ids {
            self.revoke_derived_capabilities(id);
        }

        crate::debug!("Removed all capabilities for process {}", process);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CapabilityError {
    NotFound,
    AccessDenied,
    InsufficientRights,
    InvalidResource,
    AlreadyExists,
}

/// Initialize capability subsystem
pub fn init() {
    CAPABILITY_MANAGER.call_once(|| CapabilityManager::new());
    crate::info!("Capability subsystem initialized (IPC endpoint access enforced)");
}

/// Get capability manager
pub fn manager() -> &'static CapabilityManager {
    CAPABILITY_MANAGER.get().expect("Capability subsystem not initialized")
}
