//! System call number definitions for TanOS
//! 
//! This module defines all system call numbers in a structured way,
//! grouping them by functionality.

/// System call number type
pub type SyscallNumber = u64;

// === IPC Subsystem (0x00-0x0F) ===

/// Send message to endpoint (async)
pub const SYSCALL_IPC_SEND: SyscallNumber = 0x00;

/// Receive message from endpoint (blocking)
pub const SYSCALL_IPC_RECEIVE: SyscallNumber = 0x01;

/// Call endpoint (send + receive in one operation)
pub const SYSCALL_IPC_CALL: SyscallNumber = 0x02;

/// Reply to received call
pub const SYSCALL_IPC_REPLY: SyscallNumber = 0x03;

/// Send notification (lightweight signal)
pub const SYSCALL_IPC_NOTIFY: SyscallNumber = 0x04;

/// Create new IPC endpoint
pub const SYSCALL_CREATE_ENDPOINT: SyscallNumber = 0x05;

/// Delete IPC endpoint
pub const SYSCALL_DELETE_ENDPOINT: SyscallNumber = 0x06;

/// Configure endpoint properties
pub const SYSCALL_CONFIGURE_ENDPOINT: SyscallNumber = 0x07;

/// Wait for multiple endpoints
pub const SYSCALL_IPC_WAIT_ANY: SyscallNumber = 0x08;

/// Set endpoint timeout
pub const SYSCALL_IPC_SET_TIMEOUT: SyscallNumber = 0x09;

/// Get endpoint statistics
pub const SYSCALL_IPC_GET_STATS: SyscallNumber = 0x0A;

// Reserved for future IPC operations: 0x0B-0x0F

// === Process Management (0x10-0x1F) ===

/// Exit current process
pub const SYSCALL_EXIT: SyscallNumber = 0x10;

/// Yield CPU to scheduler
pub const SYSCALL_YIELD: SyscallNumber = 0x11;

/// Create new process (fork)
pub const SYSCALL_FORK: SyscallNumber = 0x12;

/// Execute new program
pub const SYSCALL_EXEC: SyscallNumber = 0x13;

/// Wait for child process
pub const SYSCALL_WAIT: SyscallNumber = 0x14;

/// Kill process by PID
pub const SYSCALL_KILL: SyscallNumber = 0x15;

/// Get process ID
pub const SYSCALL_GETPID: SyscallNumber = 0x16;

/// Get parent process ID
pub const SYSCALL_GETPPID: SyscallNumber = 0x17;

/// Set process priority
pub const SYSCALL_SET_PRIORITY: SyscallNumber = 0x18;

/// Get process priority
pub const SYSCALL_GET_PRIORITY: SyscallNumber = 0x19;

/// Set process name
pub const SYSCALL_SET_PROCESS_NAME: SyscallNumber = 0x1A;

/// Get process information
pub const SYSCALL_GET_PROCESS_INFO: SyscallNumber = 0x1B;

/// Suspend process
pub const SYSCALL_SUSPEND: SyscallNumber = 0x1C;

/// Resume process
pub const SYSCALL_RESUME: SyscallNumber = 0x1D;

/// Create thread
pub const SYSCALL_CREATE_THREAD: SyscallNumber = 0x1E;

/// Get thread ID
pub const SYSCALL_GET_THREAD_ID: SyscallNumber = 0x1F;

// === Memory Management (0x20-0x2F) ===

/// Map memory region
pub const SYSCALL_MAP_MEMORY: SyscallNumber = 0x20;

/// Unmap memory region
pub const SYSCALL_UNMAP_MEMORY: SyscallNumber = 0x21;

/// Change memory protection
pub const SYSCALL_PROTECT_MEMORY: SyscallNumber = 0x22;

/// Create shared memory object
pub const SYSCALL_CREATE_SHARED_MEM: SyscallNumber = 0x23;

/// Attach to shared memory
pub const SYSCALL_ATTACH_SHARED_MEM: SyscallNumber = 0x24;

/// Detach from shared memory
pub const SYSCALL_DETACH_SHARED_MEM: SyscallNumber = 0x25;

/// Allocate anonymous memory
pub const SYSCALL_ALLOC_MEMORY: SyscallNumber = 0x26;

/// Free allocated memory
pub const SYSCALL_FREE_MEMORY: SyscallNumber = 0x27;

/// Get memory statistics
pub const SYSCALL_GET_MEMORY_STATS: SyscallNumber = 0x28;

/// Lock memory pages
pub const SYSCALL_LOCK_MEMORY: SyscallNumber = 0x29;

/// Unlock memory pages
pub const SYSCALL_UNLOCK_MEMORY: SyscallNumber = 0x2A;

/// Advise memory usage pattern
pub const SYSCALL_MEMORY_ADVISE: SyscallNumber = 0x2B;

/// Synchronize memory
pub const SYSCALL_MEMORY_SYNC: SyscallNumber = 0x2C;

/// Copy memory between address spaces
pub const SYSCALL_COPY_MEMORY: SyscallNumber = 0x2D;

/// Map physical memory (privileged)
pub const SYSCALL_MAP_PHYSICAL: SyscallNumber = 0x2E;

/// Get physical address (privileged)
pub const SYSCALL_GET_PHYSICAL_ADDR: SyscallNumber = 0x2F;

// === Capability Management (0x30-0x3F) ===

/// Grant capability to another process
pub const SYSCALL_GRANT_CAPABILITY: SyscallNumber = 0x30;

/// Revoke capability from process
pub const SYSCALL_REVOKE_CAPABILITY: SyscallNumber = 0x31;

/// Derive new capability with reduced rights
pub const SYSCALL_DERIVE_CAPABILITY: SyscallNumber = 0x32;

/// Delete capability
pub const SYSCALL_DELETE_CAPABILITY: SyscallNumber = 0x33;

/// List capabilities
pub const SYSCALL_LIST_CAPABILITIES: SyscallNumber = 0x34;

/// Check capability rights
pub const SYSCALL_CHECK_CAPABILITY: SyscallNumber = 0x35;

/// Create capability from resource
pub const SYSCALL_CREATE_CAPABILITY: SyscallNumber = 0x36;

/// Set capability label
pub const SYSCALL_SET_CAPABILITY_LABEL: SyscallNumber = 0x37;

/// Get capability information
pub const SYSCALL_GET_CAPABILITY_INFO: SyscallNumber = 0x38;

/// Mint new capability
pub const SYSCALL_MINT_CAPABILITY: SyscallNumber = 0x39;

/// Mutate capability rights
pub const SYSCALL_MUTATE_CAPABILITY: SyscallNumber = 0x3A;

/// Copy capability
pub const SYSCALL_COPY_CAPABILITY: SyscallNumber = 0x3B;

/// Move capability
pub const SYSCALL_MOVE_CAPABILITY: SyscallNumber = 0x3C;

/// Invoke capability
pub const SYSCALL_INVOKE_CAPABILITY: SyscallNumber = 0x3D;

// Reserved for future capability operations: 0x3E-0x3F

// === Interrupt Management (0x40-0x4F) ===

/// Request IRQ capability
pub const SYSCALL_REQUEST_IRQ: SyscallNumber = 0x40;

/// Release IRQ capability
pub const SYSCALL_RELEASE_IRQ: SyscallNumber = 0x41;

/// Wait for interrupt
pub const SYSCALL_WAIT_IRQ: SyscallNumber = 0x42;

/// Acknowledge interrupt
pub const SYSCALL_ACK_IRQ: SyscallNumber = 0x43;

/// Mask interrupt
pub const SYSCALL_MASK_IRQ: SyscallNumber = 0x44;

/// Unmask interrupt
pub const SYSCALL_UNMASK_IRQ: SyscallNumber = 0x45;

/// Set interrupt handler
pub const SYSCALL_SET_IRQ_HANDLER: SyscallNumber = 0x46;

/// Get interrupt statistics
pub const SYSCALL_GET_IRQ_STATS: SyscallNumber = 0x47;

// Reserved for future interrupt operations: 0x48-0x4F

// === I/O Port Management (0x50-0x5F) ===

/// Request I/O port access
pub const SYSCALL_REQUEST_IO_PORT: SyscallNumber = 0x50;

/// Release I/O port access
pub const SYSCALL_RELEASE_IO_PORT: SyscallNumber = 0x51;

/// Read from I/O port (8-bit)
pub const SYSCALL_IO_READ8: SyscallNumber = 0x52;

/// Write to I/O port (8-bit)
pub const SYSCALL_IO_WRITE8: SyscallNumber = 0x53;

/// Read from I/O port (16-bit)
pub const SYSCALL_IO_READ16: SyscallNumber = 0x54;

/// Write to I/O port (16-bit)
pub const SYSCALL_IO_WRITE16: SyscallNumber = 0x55;

/// Read from I/O port (32-bit)
pub const SYSCALL_IO_READ32: SyscallNumber = 0x56;

/// Write to I/O port (32-bit)
pub const SYSCALL_IO_WRITE32: SyscallNumber = 0x57;

// Reserved for future I/O operations: 0x58-0x5F

// === Time Management (0x60-0x6F) ===

/// Get current time
pub const SYSCALL_GET_TIME: SyscallNumber = 0x60;

/// Set current time (privileged)
pub const SYSCALL_SET_TIME: SyscallNumber = 0x61;

/// Sleep for duration
pub const SYSCALL_SLEEP: SyscallNumber = 0x62;

/// Get high-resolution timer
pub const SYSCALL_GET_TIMER: SyscallNumber = 0x63;

/// Create timer
pub const SYSCALL_CREATE_TIMER: SyscallNumber = 0x64;

/// Delete timer
pub const SYSCALL_DELETE_TIMER: SyscallNumber = 0x65;

/// Set timer
pub const SYSCALL_SET_TIMER: SyscallNumber = 0x66;

/// Cancel timer
pub const SYSCALL_CANCEL_TIMER: SyscallNumber = 0x67;

/// Get uptime
pub const SYSCALL_GET_UPTIME: SyscallNumber = 0x68;

/// Get CPU time
pub const SYSCALL_GET_CPU_TIME: SyscallNumber = 0x69;

// Reserved for future time operations: 0x6A-0x6F

// === System Information (0x70-0x7F) ===

/// Get system information
pub const SYSCALL_GET_SYSTEM_INFO: SyscallNumber = 0x70;

/// Get kernel version
pub const SYSCALL_GET_KERNEL_VERSION: SyscallNumber = 0x71;

/// Get CPU information
pub const SYSCALL_GET_CPU_INFO: SyscallNumber = 0x72;

/// Get memory information
pub const SYSCALL_GET_MEMORY_INFO: SyscallNumber = 0x73;

/// Get platform information
pub const SYSCALL_GET_PLATFORM_INFO: SyscallNumber = 0x74;

/// Get performance counters
pub const SYSCALL_GET_PERF_COUNTERS: SyscallNumber = 0x75;

/// Set system configuration
pub const SYSCALL_SET_SYSTEM_CONFIG: SyscallNumber = 0x76;

/// Get system configuration
pub const SYSCALL_GET_SYSTEM_CONFIG: SyscallNumber = 0x77;

/// Enumerate system resources
pub const SYSCALL_ENUM_RESOURCES: SyscallNumber = 0x78;

/// Get resource information
pub const SYSCALL_GET_RESOURCE_INFO: SyscallNumber = 0x79;

// Reserved for future system operations: 0x7A-0x7F

// === Security & Audit (0x80-0x8F) ===

/// Set security context
pub const SYSCALL_SET_SECURITY_CONTEXT: SyscallNumber = 0x80;

/// Get security context
pub const SYSCALL_GET_SECURITY_CONTEXT: SyscallNumber = 0x81;

/// Audit event
pub const SYSCALL_AUDIT_EVENT: SyscallNumber = 0x82;

/// Get audit log
pub const SYSCALL_GET_AUDIT_LOG: SyscallNumber = 0x83;

/// Set audit policy
pub const SYSCALL_SET_AUDIT_POLICY: SyscallNumber = 0x84;

/// Authenticate
pub const SYSCALL_AUTHENTICATE: SyscallNumber = 0x85;

/// Create security token
pub const SYSCALL_CREATE_TOKEN: SyscallNumber = 0x86;

/// Validate security token
pub const SYSCALL_VALIDATE_TOKEN: SyscallNumber = 0x87;

// Reserved for future security operations: 0x88-0x8F

// === Device Management (0x90-0x9F) ===

/// Enumerate devices
pub const SYSCALL_ENUM_DEVICES: SyscallNumber = 0x90;

/// Get device information
pub const SYSCALL_GET_DEVICE_INFO: SyscallNumber = 0x91;

/// Open device
pub const SYSCALL_OPEN_DEVICE: SyscallNumber = 0x92;

/// Close device
pub const SYSCALL_CLOSE_DEVICE: SyscallNumber = 0x93;

/// Device I/O control
pub const SYSCALL_DEVICE_IOCTL: SyscallNumber = 0x94;

/// Register device driver
pub const SYSCALL_REGISTER_DRIVER: SyscallNumber = 0x95;

/// Unregister device driver
pub const SYSCALL_UNREGISTER_DRIVER: SyscallNumber = 0x96;

/// Device power management
pub const SYSCALL_DEVICE_POWER: SyscallNumber = 0x97;

// Reserved for future device operations: 0x98-0x9F

// === Event Management (0xA0-0xAF) ===

/// Create event
pub const SYSCALL_CREATE_EVENT: SyscallNumber = 0xA0;

/// Delete event
pub const SYSCALL_DELETE_EVENT: SyscallNumber = 0xA1;

/// Signal event
pub const SYSCALL_SIGNAL_EVENT: SyscallNumber = 0xA2;

/// Wait for event
pub const SYSCALL_WAIT_EVENT: SyscallNumber = 0xA3;

/// Wait for multiple events
pub const SYSCALL_WAIT_EVENTS: SyscallNumber = 0xA4;

/// Reset event
pub const SYSCALL_RESET_EVENT: SyscallNumber = 0xA5;

/// Pulse event
pub const SYSCALL_PULSE_EVENT: SyscallNumber = 0xA6;

/// Get event state
pub const SYSCALL_GET_EVENT_STATE: SyscallNumber = 0xA7;

// Reserved for future event operations: 0xA8-0xAF

// === Synchronization (0xB0-0xBF) ===

/// Create mutex
pub const SYSCALL_CREATE_MUTEX: SyscallNumber = 0xB0;

/// Delete mutex
pub const SYSCALL_DELETE_MUTEX: SyscallNumber = 0xB1;

/// Lock mutex
pub const SYSCALL_LOCK_MUTEX: SyscallNumber = 0xB2;

/// Unlock mutex
pub const SYSCALL_UNLOCK_MUTEX: SyscallNumber = 0xB3;

/// Try lock mutex
pub const SYSCALL_TRYLOCK_MUTEX: SyscallNumber = 0xB4;

/// Create semaphore
pub const SYSCALL_CREATE_SEMAPHORE: SyscallNumber = 0xB5;

/// Delete semaphore
pub const SYSCALL_DELETE_SEMAPHORE: SyscallNumber = 0xB6;

/// Wait on semaphore
pub const SYSCALL_WAIT_SEMAPHORE: SyscallNumber = 0xB7;

/// Signal semaphore
pub const SYSCALL_SIGNAL_SEMAPHORE: SyscallNumber = 0xB8;

/// Try wait semaphore
pub const SYSCALL_TRYWAIT_SEMAPHORE: SyscallNumber = 0xB9;

/// Create condition variable
pub const SYSCALL_CREATE_CONDVAR: SyscallNumber = 0xBA;

/// Delete condition variable
pub const SYSCALL_DELETE_CONDVAR: SyscallNumber = 0xBB;

/// Wait on condition variable
pub const SYSCALL_WAIT_CONDVAR: SyscallNumber = 0xBC;

/// Signal condition variable
pub const SYSCALL_SIGNAL_CONDVAR: SyscallNumber = 0xBD;

/// Broadcast condition variable
pub const SYSCALL_BROADCAST_CONDVAR: SyscallNumber = 0xBE;

// Reserved for future sync operations: 0xBF

// === Reserved Ranges ===
// 0xC0-0xEF: Reserved for future kernel services
// 0xF0-0xFE: Debug and development syscalls
// 0xFF: Invalid/testing

// === Debug Operations (0xF0-0xFE) ===

/// Print debug message
pub const SYSCALL_DEBUG_PRINT: SyscallNumber = 0xF0;

/// Debug breakpoint
pub const SYSCALL_DEBUG_BREAK: SyscallNumber = 0xF1;

/// Get debug information
pub const SYSCALL_DEBUG_INFO: SyscallNumber = 0xF2;

/// Set debug level
pub const SYSCALL_DEBUG_SET_LEVEL: SyscallNumber = 0xF3;

/// Enable/disable tracing
pub const SYSCALL_DEBUG_TRACE: SyscallNumber = 0xF4;

/// Get trace buffer
pub const SYSCALL_DEBUG_GET_TRACE: SyscallNumber = 0xF5;

/// Performance profiling
pub const SYSCALL_DEBUG_PROFILE: SyscallNumber = 0xF6;

/// Memory dump
pub const SYSCALL_DEBUG_MEMDUMP: SyscallNumber = 0xF7;

/// Register dump
pub const SYSCALL_DEBUG_REGDUMP: SyscallNumber = 0xF8;

/// Stack trace
pub const SYSCALL_DEBUG_BACKTRACE: SyscallNumber = 0xF9;

/// Kernel statistics
pub const SYSCALL_DEBUG_KERNEL_STATS: SyscallNumber = 0xFA;

/// Test syscall (development only)
pub const SYSCALL_DEBUG_TEST: SyscallNumber = 0xFB;

/// Benchmark syscall
pub const SYSCALL_DEBUG_BENCHMARK: SyscallNumber = 0xFC;

/// Reset system (emergency)
pub const SYSCALL_DEBUG_RESET: SyscallNumber = 0xFD;

/// Halt system (emergency)
pub const SYSCALL_DEBUG_HALT: SyscallNumber = 0xFE;

/// Invalid syscall number
pub const SYSCALL_INVALID: SyscallNumber = 0xFF;

/// Maximum valid syscall number
pub const SYSCALL_MAX: SyscallNumber = 0xFE;

/// Total number of syscalls
pub const SYSCALL_COUNT: usize = (SYSCALL_MAX + 1) as usize;

/// Check if syscall number is valid
pub const fn is_valid_syscall(number: SyscallNumber) -> bool {
    number <= SYSCALL_MAX
}

/// Check if syscall is privileged (requires specific capabilities)
pub const fn is_privileged_syscall(number: SyscallNumber) -> bool {
    matches!(number,
        // Memory management
        SYSCALL_MAP_PHYSICAL | SYSCALL_GET_PHYSICAL_ADDR |
        // Time management
        SYSCALL_SET_TIME |
        // System configuration
        SYSCALL_SET_SYSTEM_CONFIG |
        // Device management
        SYSCALL_REGISTER_DRIVER | SYSCALL_UNREGISTER_DRIVER |
        // Debug operations (most are privileged)
        0xF0..=0xFE
    )
}

/// Get syscall category name
pub const fn syscall_category(number: SyscallNumber) -> &'static str {
    match number {
        0x00..=0x0F => "IPC",
        0x10..=0x1F => "Process",
        0x20..=0x2F => "Memory",
        0x30..=0x3F => "Capability",
        0x40..=0x4F => "Interrupt",
        0x50..=0x5F => "I/O Port",
        0x60..=0x6F => "Time",
        0x70..=0x7F => "System Info",
        0x80..=0x8F => "Security",
        0x90..=0x9F => "Device",
        0xA0..=0xAF => "Event",
        0xB0..=0xBF => "Synchronization",
        0xC0..=0xEF => "Reserved",
        0xF0..=0xFE => "Debug",
        0xFF => "Invalid",
        _ => "Unknown",
    }
}
