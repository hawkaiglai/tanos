//! Device Driver Framework for TanOS Kernel
//! Provides the core infrastructure for managing device drivers in the microkernel.

use crate::format;

use alloc::{collections::BTreeMap, vec::Vec, string::String};
use crate::*;
use spin::{Mutex, Once};

/// Driver registry for managing all device drivers
static DRIVER_REGISTRY: Once<Mutex<DriverRegistry>> = Once::new();

/// Driver capabilities and permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverCapabilities {
    pub can_access_memory: bool,
    pub can_handle_interrupts: bool,
    pub can_perform_dma: bool,
    pub max_memory_regions: usize,
    pub allowed_ports: PortRange,
}

/// Port access range for drivers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

/// Driver metadata and state
#[derive(Debug, Clone)]
pub struct DriverInfo {
    pub id: DriverId,
    pub name: String,
    pub version: String,
    pub process_id: ProcessId,
    pub capabilities: DriverCapabilities,
    pub state: DriverState,
    pub device_classes: Vec<DeviceClass>,
}

/// Driver lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverState {
    Unloaded,
    Loading,
    Ready,
    Active,
    Suspended,
    Error,
    Unloading,
}

/// Device classes that drivers can handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceClass {
    Storage,
    Network,
    Display,
    Input,
    Audio,
    Timer,
    Interrupt,
    Memory,
    Power,
    Sensor,
}

/// Driver registry managing all loaded drivers
pub struct DriverRegistry {
    drivers: BTreeMap<DriverId, DriverInfo>,
    process_to_driver: BTreeMap<ProcessId, DriverId>,
    class_to_drivers: BTreeMap<DeviceClass, Vec<DriverId>>,
    next_driver_id: u32,
}

/// Driver communication interface
pub struct DriverInterface {
    endpoint: EndpointId,
    driver_id: DriverId,
}

impl DriverRegistry {
    fn new() -> Self {
        Self {
            drivers: BTreeMap::new(),
            process_to_driver: BTreeMap::new(),
            class_to_drivers: BTreeMap::new(),
            next_driver_id: 1,
        }
    }
    
    /// Register a new driver with the system
    pub fn register_driver(
        &mut self,
        name: String,
        version: String,
        process_id: ProcessId,
        capabilities: DriverCapabilities,
        device_classes: Vec<DeviceClass>,
    ) -> core::result::Result<DriverId, DriverError> {
        // Check if process already has a driver registered
        if self.process_to_driver.contains_key(&process_id) {
            return Err(DriverError::ProcessAlreadyRegistered);
        }
        
        let driver_id = DriverId::new(self.next_driver_id);
        self.next_driver_id += 1;
        
        let driver_info = DriverInfo {
            id: driver_id,
            name,
            version,
            process_id,
            capabilities,
            state: DriverState::Loading,
            device_classes: device_classes.clone(),
        };
        
        // Register driver
        self.drivers.insert(driver_id, driver_info);
        self.process_to_driver.insert(process_id, driver_id);
        
        // Register for each device class
        for class in device_classes {
            self.class_to_drivers.entry(class)
                .or_insert_with(Vec::new)
                .push(driver_id);
        }
        
        crate::debug::serial::println(&format!("Registered driver: {} (ID: {})", 
                                               self.drivers[&driver_id].name, 
                                               driver_id.as_u32()));
        
        Ok(driver_id)
    }
    
    /// Unregister a driver from the system
    pub fn unregister_driver(&mut self, driver_id: DriverId) -> core::result::Result<(), DriverError> {
        let driver_info = self.drivers.get_mut(&driver_id)
            .ok_or(DriverError::DriverNotFound)?;
        
        // Mark as unloading
        driver_info.state = DriverState::Unloading;
        
        let process_id = driver_info.process_id;
        let device_classes = driver_info.device_classes.clone();
        
        // Remove from class mappings
        for class in device_classes {
            if let Some(drivers) = self.class_to_drivers.get_mut(&class) {
                drivers.retain(|&id| id != driver_id);
                if drivers.is_empty() {
                    self.class_to_drivers.remove(&class);
                }
            }
        }
        
        // Remove from registry
        self.process_to_driver.remove(&process_id);
        self.drivers.remove(&driver_id);
        
        crate::debug::serial::println(&format!("Unregistered driver: {}", driver_id.as_u32()));
        
        Ok(())
    }
    
    /// Set driver state
    pub fn set_driver_state(&mut self, driver_id: DriverId, state: DriverState) -> core::result::Result<(), DriverError> {
        let driver_info = self.drivers.get_mut(&driver_id)
            .ok_or(DriverError::DriverNotFound)?;
        
        let old_state = driver_info.state;
        driver_info.state = state;
        
        crate::debug::serial::println(&format!("Driver {} state: {:?} -> {:?}", 
                                               driver_id.as_u32(), old_state, state));
        
        Ok(())
    }
    
    /// Get driver info
    pub fn get_driver(&self, driver_id: DriverId) -> Option<&DriverInfo> {
        self.drivers.get(&driver_id)
    }
    
    /// Find driver by process ID
    pub fn find_driver_by_process(&self, process_id: ProcessId) -> Option<DriverId> {
        self.process_to_driver.get(&process_id).copied()
    }
    
    /// Get all drivers for a device class
    pub fn get_drivers_for_class(&self, class: DeviceClass) -> Vec<DriverId> {
        self.class_to_drivers.get(&class)
            .map(|drivers| drivers.clone())
            .unwrap_or_default()
    }
    
    /// Get all active drivers
    pub fn get_active_drivers(&self) -> Vec<DriverId> {
        self.drivers.values()
            .filter(|driver| driver.state == DriverState::Active)
            .map(|driver| driver.id)
            .collect()
    }
    
    /// Check if driver has capability
    pub fn check_capability(&self, driver_id: DriverId, operation: DriverOperation) -> bool {
        if let Some(driver) = self.drivers.get(&driver_id) {
            match operation {
                DriverOperation::AccessMemory => driver.capabilities.can_access_memory,
                DriverOperation::HandleInterrupts => driver.capabilities.can_handle_interrupts,
                DriverOperation::PerformDMA => driver.capabilities.can_perform_dma,
                DriverOperation::AccessPorts(port) => {
                    let range = driver.capabilities.allowed_ports;
                    port >= range.start && port <= range.end
                }
            }
        } else {
            false
        }
    }
}

/// Driver operations requiring capability checks
#[derive(Debug, Clone, Copy)]
pub enum DriverOperation {
    AccessMemory,
    HandleInterrupts,
    PerformDMA,
    AccessPorts(u16),
}

/// Driver-specific errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverError {
    DriverNotFound,
    ProcessAlreadyRegistered,
    InsufficientCapabilities,
    InvalidState,
    ResourceBusy,
}

/// Public API for driver management
pub fn register_driver(
    name: String,
    version: String,
    process_id: ProcessId,
    capabilities: DriverCapabilities,
    device_classes: Vec<DeviceClass>,
) -> core::result::Result<DriverId, DriverError> {
    DRIVER_REGISTRY.get().expect("Driver registry not initialized").lock().register_driver(name, version, process_id, capabilities, device_classes)
}

pub fn unregister_driver(driver_id: DriverId) -> core::result::Result<(), DriverError> {
    DRIVER_REGISTRY.get().expect("Driver registry not initialized").lock().unregister_driver(driver_id)
}

pub fn set_driver_state(driver_id: DriverId, state: DriverState) -> core::result::Result<(), DriverError> {
    DRIVER_REGISTRY.get().expect("Driver registry not initialized").lock().set_driver_state(driver_id, state)
}

pub fn get_driver(driver_id: DriverId) -> Option<DriverInfo> {
    DRIVER_REGISTRY.get().expect("Driver registry not initialized").lock().get_driver(driver_id).cloned()
}

pub fn find_driver_by_process(process_id: ProcessId) -> Option<DriverId> {
    DRIVER_REGISTRY.get().expect("Driver registry not initialized").lock().find_driver_by_process(process_id)
}

pub fn get_drivers_for_class(class: DeviceClass) -> Vec<DriverId> {
    DRIVER_REGISTRY.get().expect("Driver registry not initialized").lock().get_drivers_for_class(class)
}

pub fn check_driver_capability(driver_id: DriverId, operation: DriverOperation) -> bool {
    DRIVER_REGISTRY.get().expect("Driver registry not initialized").lock().check_capability(driver_id, operation)
}

/// Initialize driver subsystem
pub fn init() {
    DRIVER_REGISTRY.call_once(|| Mutex::new(DriverRegistry::new()));
    crate::debug::serial::println("Driver subsystem initialized");
}

/// Cleanup drivers for a terminated process
pub fn cleanup_process_drivers(process_id: ProcessId) {
    if let Some(driver_id) = find_driver_by_process(process_id) {
        let _ = unregister_driver(driver_id);
        crate::debug::serial::println(&format!("Cleaned up driver for process {}", process_id.as_u16()));
    }
}