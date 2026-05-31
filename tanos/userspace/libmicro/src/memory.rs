//! Memory management for userspace applications

use crate::{syscall, Result, Error};
use kernel_types::VirtAddr;
use core::alloc::Layout;
use core::ptr::NonNull;

/// Memory allocation flags
pub mod flags {
    pub const READ: u64 = 1 << 0;
    pub const WRITE: u64 = 1 << 1;
    pub const EXEC: u64 = 1 << 2;
    pub const SHARED: u64 = 1 << 3;
    pub const ZERO: u64 = 1 << 4;
}

/// Initialize memory management
pub fn init() -> Result<()> {
    Ok(())
}

/// Cleanup memory management
pub fn cleanup() {
    // Cleanup is handled automatically when the process exits
}

/// Allocate memory with specific size and alignment
pub fn allocate(size: usize, align: usize) -> Result<NonNull<u8>> {
    if size == 0 {
        return Err(Error::InvalidArgument);
    }
    
    let allocation_size = if size >= 4096 {
        (size + 4095) & !4095 // Round up to page boundary
    } else {
        size
    };
    
    let addr = syscall::allocate_memory(allocation_size, flags::READ | flags::WRITE)?;
    
    if addr.as_u64() % align as u64 != 0 {
        let _ = syscall::deallocate_memory(addr, allocation_size);
        return Err(Error::AlignmentError);
    }
    
    Ok(NonNull::new(addr.as_u64() as *mut u8).unwrap())
}

/// Deallocate memory
pub fn deallocate(ptr: *mut u8, size: usize, _align: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }

    let addr = VirtAddr::new_unchecked(ptr as u64);
    let _ = syscall::deallocate_memory(addr, size);
}

// === Server-facing memory API ===

/// Allocate a physical frame (for servers with appropriate capabilities)
pub fn allocate_frame() -> Result<VirtAddr> {
    let addr = syscall::allocate_memory(4096, flags::READ | flags::WRITE)?;
    Ok(addr)
}

/// Map a page at a specific virtual address
pub fn map_page(addr: VirtAddr, size: usize, page_flags: u64) -> Result<()> {
    syscall::map_memory(addr, size, page_flags).map_err(|e| Error::Syscall(e))
}

/// Copy data to a user-space address
pub fn copy_to_user(dest: VirtAddr, src: &[u8]) -> Result<()> {
    unsafe {
        core::ptr::copy_nonoverlapping(
            src.as_ptr(),
            dest.as_u64() as *mut u8,
            src.len(),
        );
    }
    Ok(())
}

/// Zero out a user-space memory region
pub fn zero_user_memory(addr: VirtAddr, size: usize) -> Result<()> {
    unsafe {
        core::ptr::write_bytes(addr.as_u64() as *mut u8, 0, size);
    }
    Ok(())
}

/// Map shared memory region
pub fn map_shared_memory(id: u64) -> Result<VirtAddr> {
    syscall::map_shared_memory(id).map_err(|e| Error::Syscall(e))
}

/// Create a new address space (returns an opaque handle)
pub fn create_address_space() -> Result<u64> {
    // For now, address spaces are managed via IPC to the memory server
    // This is a stub that will be wired up when the memory server is running
    syscall::allocate_memory(0, 0)
        .map(|addr| addr.as_u64())
        .map_err(|e| Error::Syscall(e))
}

/// Destroy an address space
pub fn destroy_address_space(_handle: u64) -> Result<()> {
    // Stub — will be an IPC call to memory server
    Ok(())
}

/// Protect memory region
pub fn protect_memory(addr: VirtAddr, size: usize, new_flags: u64) -> Result<()> {
    syscall::map_memory(addr, size, new_flags).map_err(|e| Error::Syscall(e))
}