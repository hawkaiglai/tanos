//! Error types for TanOS system

use core::fmt::{self, Display, Formatter};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// System error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Error {
    /// Operation succeeded
    Success = 0,
    
    // Generic errors (1-99)
    /// Invalid parameter provided
    InvalidParameter = 1,
    /// Operation not permitted
    PermissionDenied = 2,
    /// Resource not found
    NotFound = 3,
    /// Resource already exists
    AlreadyExists = 4,
    /// Operation would block
    WouldBlock = 5,
    /// Operation interrupted
    Interrupted = 6,
    /// Invalid operation for current state
    InvalidOperation = 7,
    /// Operation not supported
    NotSupported = 8,
    /// Resource temporarily unavailable
    TryAgain = 9,
    /// Operation timed out
    TimedOut = 10,
    /// End of file/stream
    EndOfFile = 11,
    /// Broken pipe
    BrokenPipe = 12,
    /// Connection refused
    ConnectionRefused = 13,
    /// Connection reset
    ConnectionReset = 14,
    /// Connection aborted
    ConnectionAborted = 15,
    /// Network unreachable
    NetworkUnreachable = 16,
    /// Host unreachable
    HostUnreachable = 17,
    /// Address in use
    AddressInUse = 18,
    /// Address not available
    AddressNotAvailable = 19,
    /// Operation cancelled
    Cancelled = 20,
    
    // Memory errors (100-199)
    /// Out of memory
    OutOfMemory = 100,
    /// Invalid memory address
    InvalidAddress = 101,
    /// Memory access violation
    AccessViolation = 102,
    /// Memory region not mapped
    NotMapped = 103,
    /// Memory region already mapped
    AlreadyMapped = 104,
    /// Invalid memory alignment
    InvalidAlignment = 105,
    /// Memory protection violation
    ProtectionViolation = 106,
    /// Page fault
    PageFault = 107,
    /// Segmentation fault
    SegmentationFault = 108,
    /// Memory fragmentation
    Fragmented = 109,
    /// Memory leak detected
    MemoryLeak = 110,
    
    // Process errors (200-299)
    /// Invalid process ID
    InvalidProcessId = 200,
    /// Process not found
    ProcessNotFound = 201,
    /// Process already exists
    ProcessExists = 202,
    /// Process not running
    ProcessNotRunning = 203,
    /// Process already terminated
    ProcessTerminated = 204,
    /// Process limit exceeded
    TooManyProcesses = 205,
    /// Invalid thread ID
    InvalidThreadId = 206,
    /// Thread not found
    ThreadNotFound = 207,
    /// Deadlock detected
    Deadlock = 208,
    /// Priority inversion
    PriorityInversion = 209,
    /// Scheduler error
    SchedulerError = 210,
    
    // IPC errors (300-399)
    /// Invalid endpoint ID
    InvalidEndpoint = 300,
    /// Endpoint not found
    EndpointNotFound = 301,
    /// Endpoint already exists
    EndpointExists = 302,
    /// No receiver waiting
    NoReceiver = 303,
    /// No sender waiting
    NoSender = 304,
    /// Message too large
    MessageTooLarge = 305,
    /// Message queue full
    QueueFull = 306,
    /// Message queue empty
    QueueEmpty = 307,
    /// IPC timeout
    IpcTimeout = 308,
    /// Invalid message type
    InvalidMessageType = 309,
    /// Call without reply
    NoReply = 310,
    /// Reply without call
    UnexpectedReply = 311,
    /// IPC system overload
    IpcOverload = 312,
    
    // Capability errors (400-499)
    /// Invalid capability ID
    InvalidCapability = 400,
    /// Capability not found
    CapabilityNotFound = 401,
    /// Insufficient rights
    InsufficientRights = 402,
    /// Cannot grant capability
    CannotGrant = 403,
    /// Cannot revoke capability
    CannotRevoke = 404,
    /// Capability limit exceeded
    TooManyCapabilities = 405,
    /// Invalid capability type
    InvalidCapabilityType = 406,
    /// Capability already granted
    CapabilityExists = 407,
    /// Capability derivation failed
    DerivationFailed = 408,
    /// Capability verification failed
    VerificationFailed = 409,
    
    // File system errors (500-599)
    /// File not found
    FileNotFound = 500,
    /// Directory not found
    DirectoryNotFound = 501,
    /// File already exists
    FileExists = 502,
    /// Directory not empty
    DirectoryNotEmpty = 503,
    /// Not a directory
    NotADirectory = 504,
    /// Is a directory
    IsADirectory = 505,
    /// File too large
    FileTooLarge = 506,
    /// Disk full
    DiskFull = 507,
    /// Read-only filesystem
    ReadOnlyFilesystem = 508,
    /// Invalid filename
    InvalidFilename = 509,
    /// Too many open files
    TooManyOpenFiles = 510,
    /// Cross-device link
    CrossDeviceLink = 511,
    /// Invalid file descriptor
    InvalidFileDescriptor = 512,
    /// File system corrupted
    FilesystemCorrupted = 513,
    /// Mount point busy
    MountPointBusy = 514,
    /// Not a mount point
    NotAMountPoint = 515,
    
    // Device errors (600-699)
    /// Device not found
    DeviceNotFound = 600,
    /// Device busy
    DeviceBusy = 601,
    /// Device error
    DeviceError = 602,
    /// Device not ready
    DeviceNotReady = 603,
    /// Device offline
    DeviceOffline = 604,
    /// Device removed
    DeviceRemoved = 605,
    /// Invalid device operation
    InvalidDeviceOperation = 606,
    /// Device timeout
    DeviceTimeout = 607,
    /// Device overrun
    DeviceOverrun = 608,
    /// Device underrun
    DeviceUnderrun = 609,
    /// Device configuration error
    DeviceConfigError = 610,
    /// Driver not found
    DriverNotFound = 611,
    /// Driver error
    DriverError = 612,
    /// Hardware error
    HardwareError = 613,
    /// Firmware error
    FirmwareError = 614,
    
    // Network errors (700-799)
    /// Network down
    NetworkDown = 700,
    /// Host down
    HostDown = 701,
    /// Protocol error
    ProtocolError = 702,
    /// Invalid protocol
    InvalidProtocol = 703,
    /// Protocol not supported
    ProtocolNotSupported = 704,
    /// Socket error
    SocketError = 705,
    /// Invalid socket type
    InvalidSocketType = 706,
    /// Socket not connected
    NotConnected = 707,
    /// Socket already connected
    AlreadyConnected = 708,
    /// Connection in progress
    ConnectionInProgress = 709,
    /// Message size too large
    MessageSize = 710,
    /// No route to host
    NoRoute = 711,
    
    // System errors (800-899)
    /// System call failed
    SystemCallFailed = 800,
    /// Kernel panic
    KernelPanic = 801,
    /// System overload
    SystemOverload = 802,
    /// Resource limit exceeded
    ResourceLimit = 803,
    /// System configuration error
    ConfigurationError = 804,
    /// Version mismatch
    VersionMismatch = 805,
    /// System not initialized
    NotInitialized = 806,
    /// System already initialized
    AlreadyInitialized = 807,
    /// Shutdown in progress
    ShuttingDown = 808,
    /// System suspended
    Suspended = 809,
    /// Power management error
    PowerError = 810,
    /// Clock error
    ClockError = 811,
    /// Timer error
    TimerError = 812,
    
    // Security errors (900-999)
    /// Authentication failed
    AuthenticationFailed = 900,
    /// Authorization failed
    AuthorizationFailed = 901,
    /// Security violation
    SecurityViolation = 902,
    /// Encryption error
    EncryptionError = 903,
    /// Decryption error
    DecryptionError = 904,
    /// Certificate error
    CertificateError = 905,
    /// Signature verification failed
    SignatureError = 906,
    /// Key not found
    KeyNotFound = 907,
    /// Key expired
    KeyExpired = 908,
    /// Access denied
    AccessDenied = 909,
    /// Audit log full
    AuditLogFull = 910,
    /// Security policy violation
    PolicyViolation = 911,
    
    // Unknown error
    Unknown = 0xFFFFFFFF,
}

impl Error {
    /// Create an error from a raw error code
    pub const fn from_u32(code: u32) -> Self {
        match code {
            0 => Self::Success,
            1 => Self::InvalidParameter,
            2 => Self::PermissionDenied,
            3 => Self::NotFound,
            4 => Self::AlreadyExists,
            5 => Self::WouldBlock,
            6 => Self::Interrupted,
            7 => Self::InvalidOperation,
            8 => Self::NotSupported,
            9 => Self::TryAgain,
            10 => Self::TimedOut,
            11 => Self::EndOfFile,
            12 => Self::BrokenPipe,
            13 => Self::ConnectionRefused,
            14 => Self::ConnectionReset,
            15 => Self::ConnectionAborted,
            16 => Self::NetworkUnreachable,
            17 => Self::HostUnreachable,
            18 => Self::AddressInUse,
            19 => Self::AddressNotAvailable,
            20 => Self::Cancelled,
            
            100 => Self::OutOfMemory,
            101 => Self::InvalidAddress,
            102 => Self::AccessViolation,
            103 => Self::NotMapped,
            104 => Self::AlreadyMapped,
            105 => Self::InvalidAlignment,
            106 => Self::ProtectionViolation,
            107 => Self::PageFault,
            108 => Self::SegmentationFault,
            109 => Self::Fragmented,
            110 => Self::MemoryLeak,
            
            200 => Self::InvalidProcessId,
            201 => Self::ProcessNotFound,
            202 => Self::ProcessExists,
            203 => Self::ProcessNotRunning,
            204 => Self::ProcessTerminated,
            205 => Self::TooManyProcesses,
            206 => Self::InvalidThreadId,
            207 => Self::ThreadNotFound,
            208 => Self::Deadlock,
            209 => Self::PriorityInversion,
            210 => Self::SchedulerError,
            
            300 => Self::InvalidEndpoint,
            301 => Self::EndpointNotFound,
            302 => Self::EndpointExists,
            303 => Self::NoReceiver,
            304 => Self::NoSender,
            305 => Self::MessageTooLarge,
            306 => Self::QueueFull,
            307 => Self::QueueEmpty,
            308 => Self::IpcTimeout,
            309 => Self::InvalidMessageType,
            310 => Self::NoReply,
            311 => Self::UnexpectedReply,
            312 => Self::IpcOverload,
            
            400 => Self::InvalidCapability,
            401 => Self::CapabilityNotFound,
            402 => Self::InsufficientRights,
            403 => Self::CannotGrant,
            404 => Self::CannotRevoke,
            405 => Self::TooManyCapabilities,
            406 => Self::InvalidCapabilityType,
            407 => Self::CapabilityExists,
            408 => Self::DerivationFailed,
            409 => Self::VerificationFailed,
            
            500 => Self::FileNotFound,
            501 => Self::DirectoryNotFound,
            502 => Self::FileExists,
            503 => Self::DirectoryNotEmpty,
            504 => Self::NotADirectory,
            505 => Self::IsADirectory,
            506 => Self::FileTooLarge,
            507 => Self::DiskFull,
            508 => Self::ReadOnlyFilesystem,
            509 => Self::InvalidFilename,
            510 => Self::TooManyOpenFiles,
            511 => Self::CrossDeviceLink,
            512 => Self::InvalidFileDescriptor,
            513 => Self::FilesystemCorrupted,
            514 => Self::MountPointBusy,
            515 => Self::NotAMountPoint,
            
            600 => Self::DeviceNotFound,
            601 => Self::DeviceBusy,
            602 => Self::DeviceError,
            603 => Self::DeviceNotReady,
            604 => Self::DeviceOffline,
            605 => Self::DeviceRemoved,
            606 => Self::InvalidDeviceOperation,
            607 => Self::DeviceTimeout,
            608 => Self::DeviceOverrun,
            609 => Self::DeviceUnderrun,
            610 => Self::DeviceConfigError,
            611 => Self::DriverNotFound,
            612 => Self::DriverError,
            613 => Self::HardwareError,
            614 => Self::FirmwareError,
            
            700 => Self::NetworkDown,
            701 => Self::HostDown,
            702 => Self::ProtocolError,
            703 => Self::InvalidProtocol,
            704 => Self::ProtocolNotSupported,
            705 => Self::SocketError,
            706 => Self::InvalidSocketType,
            707 => Self::NotConnected,
            708 => Self::AlreadyConnected,
            709 => Self::ConnectionInProgress,
            710 => Self::MessageSize,
            711 => Self::NoRoute,
            
            800 => Self::SystemCallFailed,
            801 => Self::KernelPanic,
            802 => Self::SystemOverload,
            803 => Self::ResourceLimit,
            804 => Self::ConfigurationError,
            805 => Self::VersionMismatch,
            806 => Self::NotInitialized,
            807 => Self::AlreadyInitialized,
            808 => Self::ShuttingDown,
            809 => Self::Suspended,
            810 => Self::PowerError,
            811 => Self::ClockError,
            812 => Self::TimerError,
            
            900 => Self::AuthenticationFailed,
            901 => Self::AuthorizationFailed,
            902 => Self::SecurityViolation,
            903 => Self::EncryptionError,
            904 => Self::DecryptionError,
            905 => Self::CertificateError,
            906 => Self::SignatureError,
            907 => Self::KeyNotFound,
            908 => Self::KeyExpired,
            909 => Self::AccessDenied,
            910 => Self::AuditLogFull,
            911 => Self::PolicyViolation,
            
            _ => Self::Unknown,
        }
    }
    
    /// Get the raw error code
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
    
    /// Check if this represents success
    pub const fn is_success(self) -> bool {
        matches!(self, Error::Success)
    }
    
    /// Check if this represents an error
    pub const fn is_error(self) -> bool {
        !self.is_success()
    }
    
    /// Check if this is a memory-related error
    pub const fn is_memory_error(self) -> bool {
        matches!(self as u32, 100..=199)
    }
    
    /// Check if this is a process-related error
    pub const fn is_process_error(self) -> bool {
        matches!(self as u32, 200..=299)
    }
    
    /// Check if this is an IPC-related error
    pub const fn is_ipc_error(self) -> bool {
        matches!(self as u32, 300..=399)
    }
    
    /// Check if this is a capability-related error
    pub const fn is_capability_error(self) -> bool {
        matches!(self as u32, 400..=499)
    }
    
    /// Check if this is a filesystem-related error
    pub const fn is_filesystem_error(self) -> bool {
        matches!(self as u32, 500..=599)
    }
    
    /// Check if this is a device-related error
    pub const fn is_device_error(self) -> bool {
        matches!(self as u32, 600..=699)
    }
    
    /// Check if this is a network-related error
    pub const fn is_network_error(self) -> bool {
        matches!(self as u32, 700..=799)
    }
    
    /// Check if this is a system-related error
    pub const fn is_system_error(self) -> bool {
        matches!(self as u32, 800..=899)
    }
    
    /// Check if this is a security-related error
    pub const fn is_security_error(self) -> bool {
        matches!(self as u32, 900..=999)
    }
    
    /// Check if the operation should be retried
    pub const fn should_retry(self) -> bool {
        matches!(
            self,
            Error::WouldBlock
                | Error::TryAgain
                | Error::Interrupted
                | Error::TimedOut
                | Error::DeviceBusy
                | Error::SystemOverload
        )
    }
    
    /// Check if this is a fatal error
    pub const fn is_fatal(self) -> bool {
        matches!(
            self,
            Error::KernelPanic
                | Error::SegmentationFault
                | Error::HardwareError
                | Error::OutOfMemory
                | Error::SystemCallFailed
        )
    }
    
    /// Get error category as string
    pub const fn category(&self) -> &'static str {
        match self as &Error {
            e if e.is_memory_error() => "Memory",
            e if e.is_process_error() => "Process",
            e if e.is_ipc_error() => "IPC",
            e if e.is_capability_error() => "Capability",
            e if e.is_filesystem_error() => "Filesystem",
            e if e.is_device_error() => "Device",
            e if e.is_network_error() => "Network",
            e if e.is_system_error() => "System",
            e if e.is_security_error() => "Security",
            _ => "Generic",
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Error::Success => "Success",
            Error::InvalidParameter => "Invalid parameter",
            Error::PermissionDenied => "Permission denied",
            Error::NotFound => "Not found",
            Error::AlreadyExists => "Already exists",
            Error::WouldBlock => "Operation would block",
            Error::Interrupted => "Operation interrupted",
            Error::InvalidOperation => "Invalid operation",
            Error::NotSupported => "Operation not supported",
            Error::TryAgain => "Try again",
            Error::TimedOut => "Operation timed out",
            Error::EndOfFile => "End of file",
            Error::BrokenPipe => "Broken pipe",
            Error::ConnectionRefused => "Connection refused",
            Error::ConnectionReset => "Connection reset",
            Error::ConnectionAborted => "Connection aborted",
            Error::NetworkUnreachable => "Network unreachable",
            Error::HostUnreachable => "Host unreachable",
            Error::AddressInUse => "Address already in use",
            Error::AddressNotAvailable => "Address not available",
            Error::Cancelled => "Operation cancelled",
            
            Error::OutOfMemory => "Out of memory",
            Error::InvalidAddress => "Invalid memory address",
            Error::AccessViolation => "Memory access violation",
            Error::NotMapped => "Memory not mapped",
            Error::AlreadyMapped => "Memory already mapped",
            Error::InvalidAlignment => "Invalid memory alignment",
            Error::ProtectionViolation => "Memory protection violation",
            Error::PageFault => "Page fault",
            Error::SegmentationFault => "Segmentation fault",
            Error::Fragmented => "Memory fragmentation",
            Error::MemoryLeak => "Memory leak detected",
            
            Error::InvalidProcessId => "Invalid process ID",
            Error::ProcessNotFound => "Process not found",
            Error::ProcessExists => "Process already exists",
            Error::ProcessNotRunning => "Process not running",
            Error::ProcessTerminated => "Process terminated",
            Error::TooManyProcesses => "Too many processes",
            Error::InvalidThreadId => "Invalid thread ID",
            Error::ThreadNotFound => "Thread not found",
            Error::Deadlock => "Deadlock detected",
            Error::PriorityInversion => "Priority inversion",
            Error::SchedulerError => "Scheduler error",
            
            Error::InvalidEndpoint => "Invalid endpoint",
            Error::EndpointNotFound => "Endpoint not found",
            Error::EndpointExists => "Endpoint already exists",
            Error::NoReceiver => "No receiver waiting",
            Error::NoSender => "No sender waiting",
            Error::MessageTooLarge => "Message too large",
            Error::QueueFull => "Message queue full",
            Error::QueueEmpty => "Message queue empty",
            Error::IpcTimeout => "IPC timeout",
            Error::InvalidMessageType => "Invalid message type",
            Error::NoReply => "No reply received",
            Error::UnexpectedReply => "Unexpected reply",
            Error::IpcOverload => "IPC system overload",
            
            Error::InvalidCapability => "Invalid capability",
            Error::CapabilityNotFound => "Capability not found",
            Error::InsufficientRights => "Insufficient rights",
            Error::CannotGrant => "Cannot grant capability",
            Error::CannotRevoke => "Cannot revoke capability",
            Error::TooManyCapabilities => "Too many capabilities",
            Error::InvalidCapabilityType => "Invalid capability type",
            Error::CapabilityExists => "Capability already exists",
            Error::DerivationFailed => "Capability derivation failed",
            Error::VerificationFailed => "Capability verification failed",
            
            Error::FileNotFound => "File not found",
            Error::DirectoryNotFound => "Directory not found",
            Error::FileExists => "File already exists",
            Error::DirectoryNotEmpty => "Directory not empty",
            Error::NotADirectory => "Not a directory",
            Error::IsADirectory => "Is a directory",
            Error::FileTooLarge => "File too large",
            Error::DiskFull => "Disk full",
            Error::ReadOnlyFilesystem => "Read-only filesystem",
            Error::InvalidFilename => "Invalid filename",
            Error::TooManyOpenFiles => "Too many open files",
            Error::CrossDeviceLink => "Cross-device link",
            Error::InvalidFileDescriptor => "Invalid file descriptor",
            Error::FilesystemCorrupted => "Filesystem corrupted",
            Error::MountPointBusy => "Mount point busy",
            Error::NotAMountPoint => "Not a mount point",
            
            Error::DeviceNotFound => "Device not found",
            Error::DeviceBusy => "Device busy",
            Error::DeviceError => "Device error",
            Error::DeviceNotReady => "Device not ready",
            Error::DeviceOffline => "Device offline",
            Error::DeviceRemoved => "Device removed",
            Error::InvalidDeviceOperation => "Invalid device operation",
            Error::DeviceTimeout => "Device timeout",
            Error::DeviceOverrun => "Device overrun",
            Error::DeviceUnderrun => "Device underrun",
            Error::DeviceConfigError => "Device configuration error",
            Error::DriverNotFound => "Driver not found",
            Error::DriverError => "Driver error",
            Error::HardwareError => "Hardware error",
            Error::FirmwareError => "Firmware error",
            
            Error::NetworkDown => "Network down",
            Error::HostDown => "Host down",
            Error::ProtocolError => "Protocol error",
            Error::InvalidProtocol => "Invalid protocol",
            Error::ProtocolNotSupported => "Protocol not supported",
            Error::SocketError => "Socket error",
            Error::InvalidSocketType => "Invalid socket type",
            Error::NotConnected => "Not connected",
            Error::AlreadyConnected => "Already connected",
            Error::ConnectionInProgress => "Connection in progress",
            Error::MessageSize => "Message size error",
            Error::NoRoute => "No route to host",
            
            Error::SystemCallFailed => "System call failed",
            Error::KernelPanic => "Kernel panic",
            Error::SystemOverload => "System overload",
            Error::ResourceLimit => "Resource limit exceeded",
            Error::ConfigurationError => "Configuration error",
            Error::VersionMismatch => "Version mismatch",
            Error::NotInitialized => "Not initialized",
            Error::AlreadyInitialized => "Already initialized",
            Error::ShuttingDown => "System shutting down",
            Error::Suspended => "System suspended",
            Error::PowerError => "Power management error",
            Error::ClockError => "Clock error",
            Error::TimerError => "Timer error",
            
            Error::AuthenticationFailed => "Authentication failed",
            Error::AuthorizationFailed => "Authorization failed",
            Error::SecurityViolation => "Security violation",
            Error::EncryptionError => "Encryption error",
            Error::DecryptionError => "Decryption error",
            Error::CertificateError => "Certificate error",
            Error::SignatureError => "Signature verification failed",
            Error::KeyNotFound => "Key not found",
            Error::KeyExpired => "Key expired",
            Error::AccessDenied => "Access denied",
            Error::AuditLogFull => "Audit log full",
            Error::PolicyViolation => "Security policy violation",
            
            Error::Unknown => "Unknown error",
        };
        
        write!(f, "{}", msg)
    }
}

impl From<Error> for u32 {
    fn from(error: Error) -> u32 {
        error.as_u32()
    }
}

impl From<u32> for Error {
    fn from(code: u32) -> Error {
        Error::from_u32(code)
    }
}

/// Result type that can be converted to/from raw system call results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemResult<T> {
    /// Operation succeeded with value
    Ok(T),
    /// Operation failed with error code
    Err(Error),
}

impl<T> SystemResult<T> {
    /// Convert to standard Result
    pub fn into_result(self) -> Result<T, Error> {
        match self {
            SystemResult::Ok(value) => Ok(value),
            SystemResult::Err(error) => Err(error),
        }
    }
    
    /// Create from standard Result
    pub fn from_result(result: Result<T, Error>) -> Self {
        match result {
            Ok(value) => SystemResult::Ok(value),
            Err(error) => SystemResult::Err(error),
        }
    }
    
    /// Check if the result is success
    pub const fn is_ok(&self) -> bool {
        matches!(self, SystemResult::Ok(_))
    }
    
    /// Check if the result is an error
    pub const fn is_err(&self) -> bool {
        matches!(self, SystemResult::Err(_))
    }
    
    /// Get the value, panicking if error
    pub fn unwrap(self) -> T {
        match self {
            SystemResult::Ok(value) => value,
            SystemResult::Err(error) => panic!("SystemResult::unwrap() on error: {}", error),
        }
    }
    
    /// Get the error, panicking if success
    pub fn unwrap_err(self) -> Error {
        match self {
            SystemResult::Ok(_) => panic!("SystemResult::unwrap_err() on success"),
            SystemResult::Err(error) => error,
        }
    }
}

impl<T> From<Result<T, Error>> for SystemResult<T> {
    fn from(result: Result<T, Error>) -> Self {
        Self::from_result(result)
    }
}

impl<T> From<SystemResult<T>> for Result<T, Error> {
    fn from(result: SystemResult<T>) -> Self {
        result.into_result()
    }
}

/// Error context for debugging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// The error that occurred
    pub error: Error,
    /// Source file where error occurred
    pub file: &'static str,
    /// Line number where error occurred
    pub line: u32,
    /// Function name where error occurred
    pub function: &'static str,
    /// Additional context message
    pub message: Option<&'static str>,
}

impl ErrorContext {
    /// Create a new error context
    pub const fn new(
        error: Error,
        file: &'static str,
        line: u32,
        function: &'static str,
    ) -> Self {
        Self {
            error,
            file,
            line,
            function,
            message: None,
        }
    }
    
    /// Add a context message
    pub const fn with_message(mut self, message: &'static str) -> Self {
        self.message = Some(message);
        self
    }
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}:{}",
            self.error, self.file, self.line, self.function
        )?;
        
        if let Some(message) = self.message {
            write!(f, " ({})", message)?;
        }
        
        Ok(())
    }
}

/// Macro to create an error context with file/line information
#[macro_export]
macro_rules! error_context {
    ($error:expr) => {
        $crate::ErrorContext::new($error, file!(), line!(), module_path!())
    };
    ($error:expr, $message:expr) => {
        $crate::ErrorContext::new($error, file!(), line!(), module_path!())
            .with_message($message)
    };
}

/// Macro to return early with error context
#[macro_export]
macro_rules! bail {
    ($error:expr) => {
        return Err($crate::error_context!($error).error)
    };
    ($error:expr, $message:expr) => {
        return Err($crate::error_context!($error, $message).error)
    };
}

/// Macro to ensure a condition or return error
#[macro_export]
macro_rules! ensure {
    ($condition:expr, $error:expr) => {
        if !($condition) {
            $crate::bail!($error);
        }
    };
    ($condition:expr, $error:expr, $message:expr) => {
        if !($condition) {
            $crate::bail!($error, $message);
        }
    };
}
