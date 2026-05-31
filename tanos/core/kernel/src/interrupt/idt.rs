//! Interrupt Descriptor Table (IDT) management
//! This module wraps x86_64 IDT functionality

pub use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

// IDT initialization will be done through x86_64 crate
// This module exists to provide a clean interface
