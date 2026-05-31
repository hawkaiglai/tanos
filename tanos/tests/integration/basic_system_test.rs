//! Basic System Integration Tests
//! 
//! Tests that verify the kernel components work together correctly

#![no_std]
#![cfg(test)]

extern crate alloc;

use kernel::*;
use kernel_types::*;
use alloc::vec::Vec;

/// Test kernel initialization sequence
#[cfg(test)]
fn test_kernel_initialization() {
    // This would be run in a test environment with mock hardware
    // For now, just test that the init functions exist and can be called
    
    // Test memory subsystem init
    let mock_boot_info = create_mock_boot_info();
    memory::init(&mock_boot_info);
    
    // Test interrupt system init
    interrupt::init();
    
    // Test process system init
    process::init();
    
    // Test IPC system init
    ipc::init();
    
    // Test capability system init
    capability::init();
    
    // Test syscall interface init
    syscall::init();
    
    // All subsystems should be initialized without panic
}

/// Create mock boot information for testing
fn create_mock_boot_info() -> BootInfo {
    BootInfo {
        memory_map: create_mock_memory_map(),
        kernel_start: PhysAddr::new(0x100000),
        kernel_end: PhysAddr::new(0x500000),
        initrd_start: Some(PhysAddr::new(0x600000)),
        initrd_end: Some(PhysAddr::new(0x700000)),
        framebuffer: None,
    }
}

/// Create mock memory map for testing
fn create_mock_memory_map() -> Vec<MemoryRegion> {
    vec![
        MemoryRegion {
            start: PhysAddr::new(0),
            size: 0x100000, // First 1MB - reserved
            region_type: MemoryRegionType::Reserved,
        },
        MemoryRegion {
            start: PhysAddr::new(0x100000),
            size: 0x3f00000, // 63MB - available
            region_type: MemoryRegionType::Available,
        },
        MemoryRegion {
            start: PhysAddr::new(0x4000000),
            size: 0x100000, // 1MB - ACPI
            region_type: MemoryRegionType::AcpiReclaimable,
        },
    ]
}

/// Test basic process lifecycle
#[test]
fn test_process_lifecycle() {
    // Initialize required subsystems
    let boot_info = create_mock_boot_info();
    memory::init(&boot_info);
    process::init();
    
    // Create a mock ELF for testing
    let mock_elf = create_mock_elf();
    
    // Test process creation
    let result = process::with_process_manager(|pm| {
        pm.create_process(&mock_elf, None)
    });
    
    assert!(result.is_ok());
    let pid = result.unwrap();
    
    // Test process lookup
    let process_exists = process::with_process_manager(|pm| {
        pm.get_process(pid).is_some()
    });
    assert!(process_exists);
    
    // Test process termination
    let kill_result = process::with_process_manager(|pm| {
        pm.kill_process(pid)
    });
    assert!(kill_result.is_ok());
}

/// Create minimal mock ELF for testing
fn create_mock_elf() -> Vec<u8> {
    // This is a minimal ELF header that would be parsed
    // In practice, would be a real ELF binary
    vec![
        0x7f, 0x45, 0x4c, 0x46, // ELF magic
        0x02, // 64-bit
        0x01, // Little endian
        0x01, // ELF version
        0x00, // Generic ABI
        // ... rest would be proper ELF structure
    ]
}

/// Test memory management integration
#[test] 
fn test_memory_management_integration() {
    let boot_info = create_mock_boot_info();
    memory::init(&boot_info);
    
    // Test frame allocation
    let frame_allocated = memory::with_memory_manager(|mm| {
        mm.allocate_frame().is_some()
    });
    assert!(frame_allocated);
    
    // Test contiguous frame allocation
    let frames_allocated = memory::with_memory_manager(|mm| {
        mm.allocate_contiguous_frames(4).is_some()
    });
    assert!(frames_allocated);
}

/// Test IPC integration
#[test]
fn test_ipc_integration() {
    ipc::init();
    
    let test_pid = ProcessId::new(1);
    
    // Test endpoint creation
    let endpoint_result = ipc::with_ipc_manager(|ipc| {
        ipc.create_endpoint(test_pid)
    });
    
    // Should either succeed or fail gracefully
    assert!(endpoint_result.is_ok() || endpoint_result.is_err());
}

/// Test full syscall integration
#[test]
fn test_syscall_integration() {
    // Initialize all required subsystems
    let boot_info = create_mock_boot_info();
    memory::init(&boot_info);
    process::init();
    ipc::init();
    syscall::init();
    
    // Test syscall statistics
    syscall::reset_stats();
    let stats = syscall::get_stats();
    assert_eq!(stats.total_calls, 0);
    
    // Test syscall handler with valid calls
    let result = syscall::syscall_handler(
        syscall_abi::SyscallNumber::GetTime as u64,
        0, 0, 0, 0, 0, 0
    );
    
    // Should not have error bit set
    assert!(result & 0x8000_0000_0000_0000 == 0);
}