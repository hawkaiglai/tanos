//! IRQ management and routing

use crate::ProcessId;
use crate::capability::CapabilityId;
use crate::ipc::{EndpointId, Message, MessageType};
use alloc::collections::BTreeMap;
use spin::Mutex;

/// IRQ Manager
pub struct IrqManager {
    handlers: Mutex<BTreeMap<u8, IrqHandler>>,
    statistics: Mutex<IrqStatistics>,
}

/// IRQ Handler
#[derive(Debug, Clone)]
struct IrqHandler {
    process_id: ProcessId,
    endpoint_id: EndpointId,
    capability_id: CapabilityId,
}

/// IRQ Statistics
#[derive(Debug, Default, Clone, Copy)]
struct IrqStatistics {
    total_irqs: u64,
    unhandled_irqs: u64,
    per_irq_counts: [u64; 16],
}

impl IrqManager {
    pub fn new() -> Self {
        Self {
            handlers: Mutex::new(BTreeMap::new()),
            statistics: Mutex::new(IrqStatistics::default()),
        }
    }
    
    /// Register IRQ handler
    pub fn register_handler(
        &self,
        irq: u8,
        process_id: ProcessId,
        endpoint_id: EndpointId,
        capability_id: CapabilityId,
    ) -> core::result::Result<(), IrqError> {
        if irq >= 16 {
            return Err(IrqError::InvalidIrq);
        }
        
        let mut handlers = self.handlers.lock();
        
        // Check if IRQ is already handled
        if handlers.contains_key(&irq) {
            return Err(IrqError::AlreadyRegistered);
        }
        
        // Verify capability
        let cap_mgr = crate::capability::manager();
        if !cap_mgr.verify_irq_capability(process_id, capability_id, irq) {
            return Err(IrqError::InvalidCapability);
        }
        
        handlers.insert(irq, IrqHandler {
            process_id,
            endpoint_id,
            capability_id,
        });
        
        // Enable the IRQ
        super::enable_irq(irq);
        
        crate::info!("IRQ {} registered to process {} endpoint {}", 
                  irq, process_id, endpoint_id);
        
        Ok(())
    }
    
    /// Unregister IRQ handler
    pub fn unregister_handler(&self, irq: u8, process_id: ProcessId) -> core::result::Result<(), IrqError> {
        if irq >= 16 {
            return Err(IrqError::InvalidIrq);
        }
        
        let mut handlers = self.handlers.lock();
        
        if let Some(handler) = handlers.get(&irq) {
            // Verify ownership
            if handler.process_id != process_id {
                return Err(IrqError::AccessDenied);
            }
            
            handlers.remove(&irq);
            
            // Disable the IRQ
            super::disable_irq(irq);
            
            crate::info!("IRQ {} unregistered from process {}", irq, process_id);
            
            Ok(())
        } else {
            Err(IrqError::NotRegistered)
        }
    }
    
    /// Handle IRQ
    pub fn handle_irq(&self, irq: u8) {
        let mut stats = self.statistics.lock();
        stats.total_irqs += 1;
        if irq < 16 {
            stats.per_irq_counts[irq as usize] += 1;
        }
        drop(stats);
        
        // Special handling for timer IRQ
        if irq == 0 {
            self.handle_timer_irq();
            return;
        }
        
        let handlers = self.handlers.lock();
        
        if let Some(handler) = handlers.get(&irq) {
            // Send IRQ notification to registered handler
            let mut message = Message::new(MessageType::Notify);
            message.set_label(0xFFFFFFFF); // IRQ notification marker
            message.set_data(0, irq as u64);
            message.set_sender(crate::KERNEL_PID);
            
            // Try to deliver immediately
            let ipc_mgr = crate::ipc::manager();
            if let Err(e) = ipc_mgr.slowpath_send(handler.endpoint_id, &message, crate::KERNEL_PID) {
                crate::warn!("Failed to deliver IRQ {} notification: {:?}", irq, e);
                let mut stats = self.statistics.lock();
                stats.unhandled_irqs += 1;
            }
        } else {
            // No handler registered
            crate::debug!("Unhandled IRQ: {}", irq);
            let mut stats = self.statistics.lock();
            stats.unhandled_irqs += 1;
            
            // Disable unhandled IRQ to prevent spam
            super::disable_irq(irq);
        }
    }
    
    /// Handle timer IRQ (special case)
    fn handle_timer_irq(&self) {
        // Update system time
        super::manager().timer().tick();
        
        // Trigger scheduler
        crate::process::scheduler::tick();
    }
    
    /// Get IRQ statistics
    pub fn statistics(&self) -> IrqStatistics {
        *self.statistics.lock()
    }
    
    /// Check if IRQ is registered
    pub fn is_registered(&self, irq: u8) -> bool {
        self.handlers.lock().contains_key(&irq)
    }
    
    /// Get handler for IRQ
    pub fn get_handler(&self, irq: u8) -> Option<(ProcessId, EndpointId)> {
        self.handlers.lock().get(&irq).map(|h| (h.process_id, h.endpoint_id))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IrqError {
    InvalidIrq,
    AlreadyRegistered,
    NotRegistered,
    AccessDenied,
    InvalidCapability,
}
