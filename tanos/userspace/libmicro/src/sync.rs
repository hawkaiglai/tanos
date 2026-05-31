//! Synchronization primitives for userspace

use spin::Mutex as SpinMutex;

/// Re-export spin mutex for userspace
pub type Mutex<T> = SpinMutex<T>;
