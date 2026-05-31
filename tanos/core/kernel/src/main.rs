#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(asm_const)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![allow(incomplete_features)]

pub extern crate alloc;

// Re-export types from kernel_types
pub use kernel_types::{CapabilityId,
    ProcessId, VirtAddr, PhysAddr, EndpointId, Rights,
    DriverId
};

// Re-export common alloc items
pub use alloc::{format, vec, string::String, vec::Vec, boxed::Box};
// Re-export types from submodules
pub use memory::{Frame, Page, PageFlags};
pub use process::{Process, ProcessState, Priority};

pub mod boot;
pub mod memory;
pub mod process;
pub mod ipc;
pub mod gdt;
pub mod interrupt;
pub mod capability;
pub mod debug;
pub mod syscall;
pub mod driver;


use linked_list_allocator::LockedHeap;

// Global kernel heap allocator
// Starts with a static 256KB bootstrap region for early boot.
// After memory::init(), the heap can be extended with physical frames.
const BOOTSTRAP_HEAP_SIZE: usize = 256 * 1024; // 256KB
static mut BOOTSTRAP_HEAP: [u8; BOOTSTRAP_HEAP_SIZE] = [0; BOOTSTRAP_HEAP_SIZE];

#[global_allocator]
static KERNEL_ALLOCATOR: LockedHeap = LockedHeap::empty();



// Kernel constants
pub const PAGE_SIZE: usize = 4096;
pub const KERNEL_HEAP_SIZE: usize = 1024 * 1024; // 1MB
pub const MAX_PROCESSES: usize = 1024;
pub const MAX_ENDPOINTS: usize = 4096;
pub const MAX_CAPABILITIES: usize = 8192;

// Memory layout constants
pub const KERNEL_BASE: usize = 0x100000;
// User space lives above the kernel's 1GB identity-mapped region (see
// memory::page::KERNEL_IDENTITY_SIZE). USER_STACK_TOP must be a canonical
// lower-half address — the previous 0x800000000000 had bit 47 set with the
// high bits clear, which is non-canonical and #GPs when loaded into RSP.
pub const USER_BASE: usize = 0x4000_0000;        // 1 GB — userspace link base
pub const USER_STACK_TOP: usize = 0xC000_0000;   // 3 GB — top of initial user stack
pub const KERNEL_HEAP_BASE: usize = 0xFFFF800000000000;

// Well-known process IDs
pub const KERNEL_PID: ProcessId = ProcessId::new_const(0);
pub const INIT_PID: ProcessId = ProcessId::new_const(1);

// Well-known endpoint IDs
pub const DEVICE_MANAGER_ENDPOINT: EndpointId = EndpointId::well_known(1);
pub const PROCESS_SERVER_ENDPOINT: EndpointId = EndpointId::well_known(2);
pub const VFS_SERVER_ENDPOINT: EndpointId = EndpointId::well_known(3);
pub use driver::DriverState;

// Global allocator for kernel heap

/// Kernel main entry point — called by assembly boot stub after 32→64 bit mode switch.
/// Arguments are passed from entry.S: RDI = multiboot magic, RSI = multiboot info pointer.
#[no_mangle]
pub extern "C" fn kernel_main(magic: u64, info_ptr: u64) -> ! {
    // Initialize bootstrap heap (must be first — alloc depends on this)
    unsafe {
        let heap_start = BOOTSTRAP_HEAP.as_mut_ptr();
        KERNEL_ALLOCATOR.lock().init(heap_start, BOOTSTRAP_HEAP_SIZE);
    }

    // Initialize serial for early debug output
    debug::serial::init();
    crate::info!("TanOS kernel starting...");
    crate::info!("Boot magic={:#x}, info_ptr={:#x}", magic, info_ptr);

    // Parse boot information from bootloader
    let boot_info = if magic as u32 == boot::multiboot::MULTIBOOT2_INFO_MAGIC && info_ptr != 0 {
        crate::info!("Parsing multiboot2 boot information...");
        unsafe { boot::parse_boot_info(magic, info_ptr) }
    } else {
        crate::warn!("No valid multiboot2 info — using hardcoded memory map");
        static MEMORY_MAP: [boot::MemoryRegion; 2] = [
            boot::MemoryRegion {
                start: PhysAddr::new_unchecked(0),
                end: PhysAddr::new_unchecked(0x100000),
                region_type: boot::MemoryRegionType::Reserved,
            },
            boot::MemoryRegion {
                start: PhysAddr::new_unchecked(0x200000),
                end: PhysAddr::new_unchecked(0x20000000), // 512MB
                region_type: boot::MemoryRegionType::Available,
            },
        ];
        boot::BootInfo {
            memory_map: &MEMORY_MAP,
            kernel_start: PhysAddr::new_unchecked(0x100000),
            kernel_end: PhysAddr::new_unchecked(0x200000),
            initrd_start: None,
            initrd_end: None,
            framebuffer: None,
            boot_params: boot::BootParams {
                cmdline: None,
                loader_name: None,
            },
        }
    };

    // Log boot info
    crate::info!("Memory map: {} regions", boot_info.memory_map.len());
    for (i, region) in boot_info.memory_map.iter().enumerate() {
        crate::info!("  region[{}]: {:#x}-{:#x} {:?}",
            i, region.start.as_u64(), region.end.as_u64(), region.region_type);
    }
    if let Some(ref cmdline) = boot_info.boot_params.cmdline {
        crate::info!("Command line: {}", cmdline);
    }
    if let Some(ref loader) = boot_info.boot_params.loader_name {
        crate::info!("Boot loader: {}", loader);
    }

    // Initialize subsystems in order
    memory::init(&boot_info);
    crate::info!("Memory subsystem initialized");

    // Load the full GDT + TSS (user segments + ring-0 stack) before enabling
    // interrupts, so the syscall/interrupt entry paths can switch to ring 0.
    gdt::init();

    interrupt::init();

    process::init();
    ipc::init();
    capability::init();

    crate::info!("All subsystems initialized.");

    // Load the init process (PID 1) from the initrd module: parse its ELF and
    // map its segments + stack into a fresh user address space. (Execution in
    // ring 3 comes once the GDT user segments / TSS / int 0x80 path are wired.)
    match process::load_init(&boot_info) {
        Ok(pid) => {
            crate::info!("init process loaded as PID {}", pid.as_u16());
            // Load a second process from the same initrd to exercise the
            // scheduler / context switch (and, next, cross-process IPC).
            match process::load_init(&boot_info) {
                Ok(pid2) => crate::info!("second process loaded as PID {}", pid2.as_u16()),
                Err(e) => crate::error!("failed to load second process: {:?}", e),
            }
            // Start scheduling in ring 3. Does not return.
            process::start();
        }
        Err(e) => crate::error!("failed to load init: {:?}", e),
    }

    // Only reached if init failed to load.
    crate::info!("Entering halt loop (init not started)");
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}

// Panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    crate::debug::serial::print_panic(info);
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}

// Panic implementation for abort

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}
