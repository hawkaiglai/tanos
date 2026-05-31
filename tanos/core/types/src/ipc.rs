//! Inter-process communication types and message formats

use core::fmt::{self, Display, Formatter};
use core::mem;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{CapabilityId, ProcessId};

/// IPC endpoint identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EndpointId(u32);

impl EndpointId {
    /// Invalid endpoint ID
    pub const INVALID: Self = Self(0);
    
    /// Create a well-known endpoint ID (0-255 are reserved)
    pub const fn well_known(id: u32) -> Self {
        debug_assert!(id < 256);
        Self(id)
    }
    
    /// Create a new endpoint ID
    pub const fn new(id: u32) -> Option<Self> {
        if id == 0 {
            None
        } else {
            Some(Self(id))
        }
    }
    
    /// Create an endpoint ID without validation
    pub const fn new_unchecked(id: u32) -> Self {
        Self(id)
    }
    
    /// Get the raw endpoint ID value
    pub const fn as_u32(self) -> u32 {
        self.0
    }
    
    /// Get the endpoint ID as u64
    pub const fn as_u64(self) -> u64 {
        self.0 as u64
    }
    
    /// Check if this is a valid endpoint ID
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
    
    /// Check if this is a well-known endpoint
    pub const fn is_well_known(self) -> bool {
        self.0 < 256
    }
}

impl Default for EndpointId {
    fn default() -> Self {
        Self::INVALID
    }
}

impl Display for EndpointId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(f, "Endpoint({})", self.0)
        } else {
            write!(f, "Endpoint(INVALID)")
        }
    }
}

/// Message type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MessageType {
    /// Asynchronous send (no reply expected)
    Send = 0,
    /// Synchronous call (expects reply)
    Call = 1,
    /// Reply to a call
    Reply = 2,
    /// Notification (lightweight, no data)
    Notify = 3,
    /// Interrupt notification
    Irq = 4,
}

impl MessageType {
    /// Create from raw u32 value
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Send),
            1 => Some(Self::Call),
            2 => Some(Self::Reply),
            3 => Some(Self::Notify),
            4 => Some(Self::Irq),
            _ => None,
        }
    }
    
    /// Get the raw u32 value
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
    
    /// Check if this message type expects a reply
    pub const fn expects_reply(self) -> bool {
        matches!(self, MessageType::Call)
    }
    
    /// Check if this is a reply message
    pub const fn is_reply(self) -> bool {
        matches!(self, MessageType::Reply)
    }
    
    /// Check if this is a notification
    pub const fn is_notification(self) -> bool {
        matches!(self, MessageType::Notify | MessageType::Irq)
    }
}

impl Display for MessageType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            MessageType::Send => "Send",
            MessageType::Call => "Call",
            MessageType::Reply => "Reply",
            MessageType::Notify => "Notify",
            MessageType::Irq => "Irq",
        };
        write!(f, "{}", s)
    }
}

/// Message flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct MessageFlags: u32 {
        /// No flags set
        const NONE = 0;
        
        /// Message contains capabilities
        const HAS_CAPABILITIES = 1 << 0;
        
        /// Message is urgent (higher priority)
        const URGENT = 1 << 1;
        
        /// Message should not block
        const NON_BLOCKING = 1 << 2;
        
        /// Message payload is in shared memory
        const SHARED_MEMORY = 1 << 3;
        
        /// Message is a broadcast
        const BROADCAST = 1 << 4;
        
        /// Message requires acknowledgment
        const ACK_REQUIRED = 1 << 5;
        
        /// Message is from kernel
        const KERNEL_MESSAGE = 1 << 6;
        
        /// Message contains error information
        const ERROR = 1 << 7;
    }
}

/// Message header (16 bytes)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MessageHeader {
    /// Message type
    pub msg_type: MessageType,
    /// Message flags
    pub flags: MessageFlags,
    /// Sender process ID
    pub sender: ProcessId,
    /// Receiver process ID (for routing)
    pub receiver: ProcessId,
    /// User-defined label for message classification
    pub label: u32,
}

impl MessageHeader {
    /// Create a new message header
    pub const fn new(msg_type: MessageType) -> Self {
        Self {
            msg_type,
            flags: MessageFlags::NONE,
            sender: ProcessId::new_const(0),
            receiver: ProcessId::new_const(0),
            label: 0,
        }
    }
    
    /// Set message flags
    pub const fn with_flags(mut self, flags: MessageFlags) -> Self {
        self.flags = flags;
        self
    }
    
    /// Set sender
    pub const fn with_sender(mut self, sender: ProcessId) -> Self {
        self.sender = sender;
        self
    }
    
    /// Set receiver
    pub const fn with_receiver(mut self, receiver: ProcessId) -> Self {
        self.receiver = receiver;
        self
    }
    
    /// Set label
    pub const fn with_label(mut self, label: u32) -> Self {
        self.label = label;
        self
    }
    
    /// Check if message has capabilities
    pub const fn has_capabilities(self) -> bool {
        self.flags.contains(MessageFlags::HAS_CAPABILITIES)
    }
    
    /// Check if message is urgent
    pub const fn is_urgent(self) -> bool {
        self.flags.contains(MessageFlags::URGENT)
    }
    
    /// Check if message is non-blocking
    pub const fn is_non_blocking(self) -> bool {
        self.flags.contains(MessageFlags::NON_BLOCKING)
    }
    
    /// Check if message contains error
    pub const fn is_error(self) -> bool {
        self.flags.contains(MessageFlags::ERROR)
    }
}

/// Message data payload (48 bytes)
#[derive(Clone, Copy)]
#[repr(C)]
pub union MessageData {
    /// Raw word data (6 x 8 bytes)
    pub words: [u64; 6],
    /// Raw byte data
    pub bytes: [u8; 48],
    /// Capability IDs for capability transfer
    pub caps: [CapabilityId; 12],
    /// Structured data access
    pub structured: StructuredData,
}

/// Structured access to message data
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StructuredData {
    /// First 8 bytes as u64
    pub word0: u64,
    /// Second 8 bytes as u64
    pub word1: u64,
    /// Third 8 bytes as u64
    pub word2: u64,
    /// Fourth 8 bytes as u64
    pub word3: u64,
    /// Fifth 8 bytes as u64
    pub word4: u64,
    /// Sixth 8 bytes as u64
    pub word5: u64,
}

impl MessageData {
    /// Create empty message data
    pub const fn empty() -> Self {
        Self {
            words: [0; 6],
        }
    }
    
    /// Create from words
    pub const fn from_words(words: [u64; 6]) -> Self {
        Self { words }
    }
    
    /// Create from bytes
    pub const fn from_bytes(bytes: [u8; 48]) -> Self {
        Self { bytes }
    }
    
    /// Get word at index
    pub unsafe fn get_word(&self, index: usize) -> u64 {
        if index < 6 {
            self.words[index]
        } else {
            0
        }
    }
    
    /// Set word at index
    pub unsafe fn set_word(&mut self, index: usize, value: u64) {
        if index < 6 {
            self.words[index] = value;
        }
    }
    
    /// Get byte at index
    pub unsafe fn get_byte(&self, index: usize) -> u8 {
        if index < 48 {
            self.bytes[index]
        } else {
            0
        }
    }
    
    /// Set byte at index
    pub unsafe fn set_byte(&mut self, index: usize, value: u8) {
        if index < 48 {
            self.bytes[index] = value;
        }
    }
}

impl fmt::Debug for MessageData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Safety: Reading as words is always safe
        unsafe {
            f.debug_struct("MessageData")
                .field("words", &self.words)
                .finish()
        }
    }
}

impl Default for MessageData {
    fn default() -> Self {
        Self::empty()
    }
}

/// Complete IPC message (64 bytes, cache-line aligned)
#[derive(Debug, Clone, Copy)]
#[repr(C, align(64))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Message {
    /// Message header (16 bytes)
    pub header: MessageHeader,
    /// Message data (48 bytes)
    pub data: MessageData,
}

// Ensure message is exactly 64 bytes
const _: () = assert!(mem::size_of::<Message>() == 64);
const _: () = assert!(mem::align_of::<Message>() == 64);

impl Message {
    /// Create a new message
    pub const fn new(msg_type: MessageType) -> Self {
        Self {
            header: MessageHeader::new(msg_type),
            data: MessageData::empty(),
        }
    }
    
    /// Create a send message
    pub const fn send() -> Self {
        Self::new(MessageType::Send)
    }
    
    /// Create a call message
    pub const fn call() -> Self {
        Self::new(MessageType::Call)
    }
    
    /// Create a reply message
    pub const fn reply() -> Self {
        Self::new(MessageType::Reply)
    }
    
    /// Create a notification message
    pub const fn notify() -> Self {
        Self::new(MessageType::Notify)
    }
    
    /// Create an IRQ notification
    pub const fn irq() -> Self {
        Self::new(MessageType::Irq)
    }
    
    /// Create a success reply
    pub const fn success() -> Self {
        Self::reply()
    }
    
    /// Create an error reply
    pub fn error(error_code: u32) -> Self {
        let mut msg = Self::reply();
        msg.header.flags |= MessageFlags::ERROR;
        unsafe {
            msg.data.set_word(0, error_code as u64);
        }
        msg
    }
    
    /// Set message label
    pub fn set_label(&mut self, label: u32) {
        self.header.label = label;
    }
    
    /// Get message label
    pub const fn label(&self) -> u32 {
        self.header.label
    }
    
    /// Set sender
    pub fn set_sender(&mut self, sender: ProcessId) {
        self.header.sender = sender;
    }
    
    /// Get sender
    pub const fn sender(&self) -> ProcessId {
        self.header.sender
    }
    
    /// Set receiver
    pub fn set_receiver(&mut self, receiver: ProcessId) {
        self.header.receiver = receiver;
    }
    
    /// Get receiver
    pub const fn receiver(&self) -> ProcessId {
        self.header.receiver
    }
    
    /// Set message flags
    pub fn set_flags(&mut self, flags: MessageFlags) {
        self.header.flags = flags;
    }
    
    /// Add message flags
    pub fn add_flags(&mut self, flags: MessageFlags) {
        self.header.flags |= flags;
    }
    
    /// Get message flags
    pub const fn flags(&self) -> MessageFlags {
        self.header.flags
    }
    
    /// Set data word
    pub fn set_data(&mut self, index: usize, value: u64) {
        unsafe {
            self.data.set_word(index, value);
        }
    }
    
    /// Get data word
    pub fn get_data(&self, index: usize) -> u64 {
        unsafe {
            self.data.get_word(index)
        }
    }
    
    /// Set data bytes
    pub fn set_bytes(&mut self, bytes: &[u8]) {
        let len = bytes.len().min(48);
        unsafe {
            for i in 0..len {
                self.data.set_byte(i, bytes[i]);
            }
            // Zero remaining bytes
            for i in len..48 {
                self.data.set_byte(i, 0);
            }
        }
    }
    
    /// Get data bytes
    pub fn get_bytes(&self) -> &[u8] {
        unsafe {
            &self.data.bytes
        }
    }
    
    /// Check if message is an IRQ notification
    pub const fn is_irq_notification(&self) -> bool {
        matches!(self.header.msg_type, MessageType::Irq)
    }
    
    /// Check if message expects a reply
    pub const fn expects_reply(&self) -> bool {
        self.header.msg_type.expects_reply()
    }
    
    /// Check if message is a reply
    pub const fn is_reply(&self) -> bool {
        self.header.msg_type.is_reply()
    }
    
    /// Check if message is an error
    pub const fn is_error(&self) -> bool {
        self.header.is_error()
    }
    
    /// Check if message contains capabilities
    pub const fn has_capabilities(&self) -> bool {
        self.header.has_capabilities()
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new(MessageType::Send)
    }
}

/// IPC operation codes for system services
pub mod opcodes {
    /// Process server operations
    pub mod process {
        pub const SPAWN: u32 = 0x1000;
        pub const KILL: u32 = 0x1001;
        pub const WAIT: u32 = 0x1002;
        pub const GET_INFO: u32 = 0x1003;
        pub const LIST_PROCESSES: u32 = 0x1004;
        pub const ALLOCATE_MEMORY: u32 = 0x2000;
        pub const FREE_MEMORY: u32 = 0x2001;
        pub const SHARE_MEMORY: u32 = 0x2002;
        pub const GRANT_CAPABILITY: u32 = 0x3000;
        pub const REVOKE_CAPABILITY: u32 = 0x3001;
    }
    
    /// VFS server operations
    pub mod vfs {
        pub const OPEN: u32 = 0x4000;
        pub const CLOSE: u32 = 0x4001;
        pub const READ: u32 = 0x4002;
        pub const WRITE: u32 = 0x4003;
        pub const SEEK: u32 = 0x4004;
        pub const STAT: u32 = 0x4005;
        pub const MKDIR: u32 = 0x4100;
        pub const RMDIR: u32 = 0x4101;
        pub const READDIR: u32 = 0x4102;
        pub const MOUNT: u32 = 0x4200;
        pub const UNMOUNT: u32 = 0x4201;
    }
    
    /// Device manager operations
    pub mod device {
        pub const REGISTER_DRIVER: u32 = 0x5000;
        pub const UNREGISTER_DRIVER: u32 = 0x5001;
        pub const REQUEST_IRQ: u32 = 0x5002;
        pub const RELEASE_IRQ: u32 = 0x5003;
        pub const MAP_DEVICE_MEMORY: u32 = 0x5004;
    }
    
    /// Keyboard driver operations
    pub mod keyboard {
        pub const GET_SCANCODE: u32 = 0x6000;
        pub const SET_LEDS: u32 = 0x6001;
        pub const SET_REPEAT_RATE: u32 = 0x6002;
    }
    
    /// VGA driver operations
    pub mod vga {
        pub const SET_MODE: u32 = 0x7000;
        pub const WRITE_CHAR: u32 = 0x7001;
        pub const WRITE_STRING: u32 = 0x7002;
        pub const SET_CURSOR: u32 = 0x7003;
        pub const CLEAR_SCREEN: u32 = 0x7004;
    }
}

/// Message queue for IPC endpoint
#[derive(Debug)]
pub struct MessageQueue {
    messages: [Option<Message>; 16],
    head: usize,
    tail: usize,
    count: usize,
}

impl MessageQueue {
    /// Create a new empty message queue
    pub const fn new() -> Self {
        Self {
            messages: [None; 16],
            head: 0,
            tail: 0,
            count: 0,
        }
    }
    
    /// Check if the queue is empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    /// Check if the queue is full
    pub const fn is_full(&self) -> bool {
        self.count == 16
    }
    
    /// Get the number of messages in the queue
    pub const fn len(&self) -> usize {
        self.count
    }
    
    /// Enqueue a message
    pub fn enqueue(&mut self, message: Message) -> bool {
        if self.is_full() {
            false
        } else {
            self.messages[self.tail] = Some(message);
            self.tail = (self.tail + 1) % 16;
            self.count += 1;
            true
        }
    }
    
    /// Dequeue a message
    pub fn dequeue(&mut self) -> Option<Message> {
        if self.is_empty() {
            None
        } else {
            let message = self.messages[self.head].take();
            self.head = (self.head + 1) % 16;
            self.count -= 1;
            message
        }
    }
    
    /// Peek at the front message without removing it
    pub fn peek(&self) -> Option<&Message> {
        if self.is_empty() {
            None
        } else {
            self.messages[self.head].as_ref()
        }
    }
    
    /// Clear all messages from the queue
    pub fn clear(&mut self) {
        self.messages = [None; 16];
        self.head = 0;
        self.tail = 0;
        self.count = 0;
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new()
    }
}
