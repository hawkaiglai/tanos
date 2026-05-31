//! Memory Server Test Application
//!
//! Tests the memory server functionality by making various memory allocation requests.

#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate libmicro;

use libmicro::*;
use alloc::format;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize libmicro
    if let Err(_) = libmicro::init() {
        syscall::exit(-1);
    }

    debug_println!("Memory Server Test Starting...");
    debug_println!("Process ID: {}", syscall::get_process_id().map(|p| p.as_u16()).unwrap_or(0));

    // Test basic functionality
    test_memory_server();

    debug_println!("Memory server test completed successfully!");
    syscall::exit(0);
}

fn test_memory_server() {
    debug_println!("=== Memory Server Functionality Test ===");

    // Test 1: Basic memory allocation
    debug_println!("Test 1: Basic memory allocation");
    test_basic_allocation();

    // Test 2: Large memory allocation
    debug_println!("Test 2: Large memory allocation");
    test_large_allocation();

    // Test 3: Shared memory
    debug_println!("Test 3: Shared memory operations");
    test_shared_memory();

    // Test 4: Memory deallocation
    debug_println!("Test 4: Memory deallocation");
    test_deallocation();

    // Test 5: Error conditions
    debug_println!("Test 5: Error condition handling");
    test_error_conditions();
}

fn test_basic_allocation() {
    // Try to allocate 4KB of memory
    match syscall::allocate_memory(4096, memory::flags::READ | memory::flags::WRITE) {
        Ok(addr) => {
            debug_println!("Basic allocation successful: 0x{:x}", addr.as_u64());

            // Try to write to the allocated memory
            unsafe {
                let ptr: *mut u8 = addr.as_mut_ptr();
                *ptr = 0x42;
                let value = *ptr;

                if value == 0x42 {
                    debug_println!("Memory write/read test passed");
                } else {
                    debug_println!("Memory write/read test failed");
                }
            }

            // Clean up
            let _ = syscall::deallocate_memory(addr, 4096);
        }
        Err(e) => {
            debug_println!("Basic allocation failed: {:?}", e);
        }
    }
}

fn test_large_allocation() {
    // Try to allocate 1MB of memory
    match syscall::allocate_memory(1024 * 1024, memory::flags::READ | memory::flags::WRITE) {
        Ok(addr) => {
            debug_println!("Large allocation successful: 0x{:x}", addr.as_u64());

            // Test writing at different offsets
            unsafe {
                let ptr: *mut u8 = addr.as_mut_ptr();
                *ptr = 0x11;                    // Start
                *(ptr.add(512 * 1024)) = 0x22; // Middle
                *(ptr.add(1024 * 1024 - 1)) = 0x33; // End

                let start = *ptr;
                let middle = *(ptr.add(512 * 1024));
                let end = *(ptr.add(1024 * 1024 - 1));

                if start == 0x11 && middle == 0x22 && end == 0x33 {
                    debug_println!("Large memory access test passed");
                } else {
                    debug_println!("Large memory access test failed");
                }
            }

            // Clean up
            let _ = syscall::deallocate_memory(addr, 1024 * 1024);
        }
        Err(e) => {
            debug_println!("Large allocation failed: {:?}", e);
        }
    }
}

fn test_shared_memory() {
    // Create a shared memory region (1 arg: size only)
    match syscall::create_shared_memory(8192) {
        Ok(shm_id) => {
            debug_println!("Shared memory creation successful: ID {}", shm_id);

            // Map the shared memory (1 arg: id only)
            match syscall::map_shared_memory(shm_id) {
                Ok(addr) => {
                    debug_println!("Shared memory mapping successful: 0x{:x}", addr.as_u64());

                    // Write some data
                    unsafe {
                        let ptr: *mut u8 = addr.as_mut_ptr();
                        for i in 0..1024usize {
                            *(ptr.add(i)) = (i % 256) as u8;
                        }

                        // Verify data
                        let mut correct = true;
                        for i in 0..1024usize {
                            if *(ptr.add(i)) != (i % 256) as u8 {
                                correct = false;
                                break;
                            }
                        }

                        if correct {
                            debug_println!("Shared memory data integrity test passed");
                        } else {
                            debug_println!("Shared memory data integrity test failed");
                        }
                    }

                    // Unmap shared memory (1 arg: id only)
                    let _ = syscall::unmap_shared_memory(shm_id);
                }
                Err(e) => {
                    debug_println!("Shared memory mapping failed: {:?}", e);
                }
            }

            // Clean up shared memory
            let _ = syscall::destroy_shared_memory(shm_id);
        }
        Err(e) => {
            debug_println!("Shared memory creation failed: {:?}", e);
        }
    }
}

fn test_deallocation() {
    // Allocate memory
    let addr = match syscall::allocate_memory(4096, memory::flags::READ | memory::flags::WRITE) {
        Ok(addr) => {
            debug_println!("Allocation for deallocation test: 0x{:x}", addr.as_u64());
            addr
        }
        Err(e) => {
            debug_println!("Allocation for deallocation test failed: {:?}", e);
            return;
        }
    };

    // Deallocate it
    match syscall::deallocate_memory(addr, 4096) {
        Ok(()) => {
            debug_println!("Deallocation successful");
        }
        Err(e) => {
            debug_println!("Deallocation failed: {:?}", e);
        }
    }
}

fn test_error_conditions() {
    debug_println!("Testing invalid allocation size (0)");
    match syscall::allocate_memory(0, memory::flags::READ) {
        Ok(_) => debug_println!("Should have failed with size 0"),
        Err(_) => debug_println!("Correctly rejected size 0"),
    }

    debug_println!("Testing invalid deallocation (null pointer)");
    match syscall::deallocate_memory(VirtAddr::new_unchecked(0), 4096) {
        Ok(_) => debug_println!("Should have failed with null pointer"),
        Err(_) => debug_println!("Correctly rejected null pointer"),
    }

    debug_println!("Testing excessive allocation size");
    match syscall::allocate_memory(2 * 1024 * 1024 * 1024, memory::flags::READ) {
        Ok(_) => debug_println!("Should have failed with excessive size"),
        Err(_) => debug_println!("Correctly rejected excessive size"),
    }
}
