
//! IPC Messages
//! Optimized message format for fast IPC.

use crate::ProcessId;
use crate::capability::CapabilityId;
use core::mem;

/// IPC Message (64 bytes, cache-line aligned)
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct Message {
    pub header: MessageHeader,
    pub data: MessageData,
}

/// Message header (16 bytes)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MessageHeader {
    pub msg_type: MessageType,
    pub flags: MessageFlags,
    pub sender: ProcessId,
    pub receiver: ProcessId,
    pub label: u32,
}

/// Message data (48 bytes)
#[repr(C)]
#[derive(Clone, Copy)]
pub union MessageData {
    pub words: [u64; 6],
    pub bytes: [u8; 48],
    pub caps: [CapabilityId; 12],
}

/// Message type
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageType {
    Send = 0,
    Call = 1,
    Reply = 2,
    Notify = 3,
}

bitflags::bitflags! {
    /// Message flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct MessageFlags: u32 {
        const NONE = 0;
        const NO_BLOCK = 1 << 0;
        const URGENT = 1 << 1;
        const CAPABILITY_TRANSFER = 1 << 2;
        const ERROR = 1 << 3;
    }
}

impl Message {
    /// Create a new message
    pub fn new(msg_type: MessageType) -> Self {
        Self {
            header: MessageHeader {
                msg_type,
                flags: MessageFlags::NONE,
                sender: ProcessId::INVALID,
                receiver: ProcessId::INVALID,
                label: 0,
            },
            data: MessageData { words: [0; 6] },
        }
    }
    
    /// Create a success reply
    pub fn success() -> Self {
        let mut msg = Self::new(MessageType::Reply);
        msg.set_success();
        msg
    }
    
    /// Create an error reply
    pub fn error(error_code: u32) -> Self {
        let mut msg = Self::new(MessageType::Reply);
        msg.set_error(error_code);
        msg
    }
    
    /// Set message label
    pub fn set_label(&mut self, label: u32) {
        self.header.label = label;
    }
    
    /// Get message label
    pub fn label(&self) -> u32 {
        self.header.label
    }
    
    /// Set message flags
    pub fn set_flags(&mut self, flags: MessageFlags) {
        self.header.flags = flags;
    }
    
    /// Get message flags
    pub fn flags(&self) -> MessageFlags {
        self.header.flags
    }
    
    /// Set sender
    pub fn set_sender(&mut self, sender: ProcessId) {
        self.header.sender = sender;
    }
    
    /// Get sender
    pub fn sender(&self) -> ProcessId {
        self.header.sender
    }
    
    /// Set data word
    pub fn set_data(&mut self, index: usize, value: u64) {
        debug_assert!(index < 6);
        unsafe {
            self.data.words[index] = value;
        }
    }
    
    /// Get data word
    pub fn get_data(&self, index: usize) -> u64 {
        debug_assert!(index < 6);
        unsafe { self.data.words[index] }
    }
    
    /// Set all data words
    pub fn set_data_words(&mut self, words: [u64; 6]) {
        unsafe {
            self.data.words = words;
        }
    }
    
    /// Get all data words
    pub fn get_data_words(&self) -> [u64; 6] {
        unsafe { self.data.words }
    }
    
    /// Set data bytes
    pub fn set_data_bytes(&mut self, bytes: &[u8]) {
        let len = bytes.len().min(48);
        unsafe {
            self.data.bytes[..len].copy_from_slice(&bytes[..len]);
            // Zero remaining bytes
            for i in len..48 {
                self.data.bytes[i] = 0;
            }
        }
    }
    
    /// Get data bytes
    pub fn get_data_bytes(&self) -> &[u8] {
        unsafe { &self.data.bytes }
    }
    
    /// Set capability
    pub fn set_capability(&mut self, index: usize, cap: CapabilityId) {
        debug_assert!(index < 12);
        unsafe {
            self.data.caps[index] = cap;
        }
        self.header.flags |= MessageFlags::CAPABILITY_TRANSFER;
    }
    
    /// Get capability
    pub fn get_capability(&self, index: usize) -> CapabilityId {
        debug_assert!(index < 12);
        unsafe { self.data.caps[index] }
    }
    
    /// Mark message as success
    pub fn set_success(&mut self) {
        self.header.flags.remove(MessageFlags::ERROR);
        self.set_data(0, 0); // Error code 0 = success
    }
    
    /// Mark message as error
    pub fn set_error(&mut self, error_code: u32) {
        self.header.flags |= MessageFlags::ERROR;
        self.set_data(0, error_code as u64);
    }
    
    /// Check if message is an error
    pub fn is_error(&self) -> bool {
        self.header.flags.contains(MessageFlags::ERROR)
    }
    
    /// Get error code (if error message)
    pub fn error_code(&self) -> Option<u32> {
        if self.is_error() {
            Some(self.get_data(0) as u32)
        } else {
            None
        }
    }
    
    /// Check if message is IRQ notification
    pub fn is_irq_notification(&self) -> bool {
        self.header.msg_type == MessageType::Notify && self.header.label == 0xFFFFFFFF
    }
    
    /// Fast zero-copy send
    #[inline(always)]
    pub unsafe fn send_fast(&self, endpoint: u32) -> core::result::Result<(), u32> {
        let result: u64;
        core::arch::asm!(
            "syscall",
            in("rax") 0x00, // SYSCALL_IPC_SEND
            in("rdi") endpoint as u64,
            in("rsi") self as *const _ as u64,
            lateout("rax") result,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
        
        if result == 0 {
            Ok(())
        } else {
            Err(result as u32)
        }
    }
}

// Compile-time assertions
const _: () = assert!(mem::size_of::<Message>() == 64);
const _: () = assert!(mem::align_of::<Message>() == 64);
const _: () = assert!(mem::size_of::<MessageHeader>() == 16);
const _: () = assert!(mem::size_of::<MessageData>() == 48);

impl core::fmt::Debug for Message {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("Message")
            .field("type", &self.header.msg_type)
            .field("flags", &self.header.flags)
            .field("sender", &self.header.sender)
            .field("receiver", &self.header.receiver)
            .field("label", &self.header.label)
            .field("data", &unsafe { self.data.words })
            .finish()
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new(MessageType::Send)
    }
}
