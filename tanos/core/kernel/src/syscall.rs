//! System call interface
//! Provides the kernel-side implementation of system calls for userspace interaction

use crate::format;


use crate::{process, ipc, memory};
use crate::{Page, PageFlags, ProcessState};
use crate::*;
use syscall_abi::numbers::*;


/// System call error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    InvalidSyscall,
    InvalidArgument,
    PermissionDenied,
    ResourceUnavailable,
    ProcessNotFound,
    EndpointNotFound,
    MessageTooLarge,
    OutOfMemory,
    InvalidPointer,
    AccessDenied,
    InvalidProcess,
    WouldBlock,
    InternalError,
    InvalidOperation,
}

impl From<SyscallError> for u64 {
    fn from(error: SyscallError) -> Self {
        match error {
            SyscallError::InvalidSyscall => 1,
            SyscallError::InvalidArgument => 2,
            SyscallError::PermissionDenied => 3,
            SyscallError::ResourceUnavailable => 4,
            SyscallError::ProcessNotFound => 5,
            SyscallError::EndpointNotFound => 6,
            SyscallError::MessageTooLarge => 7,
            SyscallError::OutOfMemory => 8,
            SyscallError::InvalidPointer => 9,
            SyscallError::AccessDenied => 10,
            SyscallError::InvalidProcess => 11,
            SyscallError::WouldBlock => 12,
            SyscallError::InternalError => 13,
            SyscallError::InvalidOperation => 14,
        }
    }
}

/// System call result type
pub type SyscallResult = Result<u64, SyscallError>;

/// System call statistics
#[derive(Debug)]
pub struct SyscallStats {
    pub total_calls: u64,
    pub call_counts: [u64; 64], // Support up to 64 different syscalls
    pub error_counts: [u64; 8],
}

impl Default for SyscallStats {
    fn default() -> Self {
        Self {
            total_calls: 0,
            call_counts: [0u64; 64],
            error_counts: [0u64; 8],
        }
    }
}

static mut SYSCALL_STATS: SyscallStats = SyscallStats {
    total_calls: 0,
    call_counts: [0; 64],
    error_counts: [0; 8],
};

/// Initialize system call interface
pub fn init() {
    // Install syscall handler in interrupt table
    // On x86_64, syscalls typically use interrupt 0x80 or SYSCALL instruction
    
    #[cfg(target_arch = "x86_64")]
    {
        // TODO: Interrupt handler registration is being reworked.
        // Previously: crate::interrupt::manager().install_handler(0x80, syscall_handler as *const ());
    }
    
    crate::debug::serial::println("System call interface initialized");
}

/// Syscall entry wrapper called from entry.S syscall_entry.
/// User registers at call time: rax=syscall_num, rdi=arg0, rsi=arg1,
/// rdx=arg2, r10=arg3, r8=arg4, r9=arg5.
/// This naked function rearranges into SysV C calling convention and
/// tail-calls syscall_handler.
#[naked]
#[no_mangle]
pub unsafe extern "C" fn syscall_handler_wrapper() -> u64 {
    core::arch::asm!(
        // Rearrange: rdi=num, rsi=a0, rdx=a1, rcx=a2, r8=a3, r9=a4
        // arg5 (original r9) is already pushed on stack by entry.S — but
        // syscall_handler takes 7 args and the 7th would need to be on stack.
        // For now we pass 6 via registers and ignore arg5 (unused currently).
        "push r9",          // save arg5 as 7th C arg on stack
        "mov r9, r8",       // r9 = arg4
        "mov r8, r10",      // r8 = arg3
        "mov rcx, rdx",     // rcx = arg2
        "mov rdx, rsi",     // rdx = arg1
        "mov rsi, rdi",     // rsi = arg0
        "mov rdi, rax",     // rdi = syscall_num
        "call {handler}",
        "add rsp, 8",       // pop arg5
        "ret",
        handler = sym syscall_handler,
        options(noreturn)
    );
}

/// Main system call handler
#[no_mangle]
pub extern "C" fn syscall_handler(
    syscall_num: u64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    _arg4: u64,
    _arg5: u64,
) -> u64 {
    // Record syscall statistics
    unsafe {
        SYSCALL_STATS.total_calls += 1;
        if (syscall_num as usize) < SYSCALL_STATS.call_counts.len() {
            SYSCALL_STATS.call_counts[syscall_num as usize] += 1;
        }
    }
    
    // Get current process
    let current_pid = process::get_current_process()
        .unwrap_or(crate::KERNEL_PID);
    
    // Dispatch to appropriate syscall implementation
    let result = match syscall_num {
        SYSCALL_DEBUG_PRINT => syscall_debug(arg0),
        SYSCALL_EXIT => syscall_exit(current_pid, arg0 as i32),
        SYSCALL_CREATE_ENDPOINT => syscall_create_endpoint(current_pid),
        SYSCALL_IPC_SEND => syscall_send(current_pid, EndpointId::new_unchecked(arg0 as u32), arg1, arg2),
        SYSCALL_IPC_RECEIVE => syscall_receive(current_pid, EndpointId::new_unchecked(arg0 as u32)),
        SYSCALL_IPC_REPLY => syscall_reply(current_pid, arg0, arg1),
        SYSCALL_ALLOC_MEMORY => syscall_allocate_memory(current_pid, arg0, arg1),
        SYSCALL_FREE_MEMORY => syscall_deallocate_memory(current_pid, arg0, arg1),
        SYSCALL_MAP_MEMORY => syscall_map_memory(current_pid, arg0, arg1, arg2),
        SYSCALL_EXEC => syscall_create_process(current_pid, arg0, arg1),
        SYSCALL_YIELD => syscall_yield_cpu(current_pid),
        SYSCALL_GET_PROCESS_INFO => {
            match ProcessId::new(arg0 as u16) {
                Some(target_pid) => syscall_get_process_info(current_pid, target_pid),
                None => Err(SyscallError::InvalidArgument),
            }
        }
        SYSCALL_REGISTER_DRIVER => syscall_register_driver(current_pid, arg0, arg1, arg2, arg3),
        // TODO: SYSCALL_SET_DRIVER_STATE needs DriverId type integration
        // SYSCALL_DEVICE_POWER => syscall_set_driver_state(current_pid, DriverId::new(arg0 as u32), arg1),
        _ => Err(SyscallError::InvalidSyscall),
    };
    
    // Record error statistics and return result
    match result {
        Ok(value) => value,
        Err(error) => {
            unsafe {
                let error_index = (error as u8 - 1) as usize;
                if error_index < SYSCALL_STATS.error_counts.len() {
                    SYSCALL_STATS.error_counts[error_index] += 1;
                }
            }
            u64::from(error) | 0x8000_0000_0000_0000 // Set error bit
        }
    }
}

// Individual system call implementations

/// Debug system call - print message to debug output
fn syscall_debug(message_ptr: u64) -> SyscallResult {
    // For safety, limit message length and validate pointer
    const MAX_DEBUG_LEN: usize = 256;
    
    let current_pid = process::get_current_process()
        .ok_or(SyscallError::ProcessNotFound)?;
    
    // TODO: Validate that message_ptr is in process address space
    if message_ptr == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    unsafe {
        let message_slice = core::slice::from_raw_parts(
            message_ptr as *const u8, 
            MAX_DEBUG_LEN
        );
        
        // Find null terminator
        let mut len = 0;
        for &byte in message_slice {
            if byte == 0 {
                break;
            }
            len += 1;
        }
        
        if len > 0 {
            let message = core::str::from_utf8_unchecked(&message_slice[..len]);
            crate::debug::serial::println(&format!("Process {}: {}", current_pid.as_u16(), message));
        }
    }
    
    Ok(0)
}

/// Exit process system call
fn syscall_exit(pid: ProcessId, exit_code: i32) -> SyscallResult {
    process::with_process_manager(|pm| {
        pm.kill_process(pid)
    }).map_err(|_| SyscallError::ProcessNotFound)?;
    
    // This syscall doesn't return - process is terminated
    Ok(exit_code as u64)
}

/// Create IPC endpoint system call
fn syscall_create_endpoint(pid: ProcessId) -> SyscallResult {
    // Check capability for endpoint creation
    // TODO: Implement capability checking

    let endpoint_id = ipc::manager()
        .create_endpoint(pid)
        .map_err(|_| SyscallError::ResourceUnavailable)?;

    Ok(endpoint_id.as_u64())
}

/// Send IPC message system call
fn syscall_send(sender_pid: ProcessId, endpoint_id: EndpointId, data_ptr: u64, data_len: u64) -> SyscallResult {
    if data_len > ipc::MAX_MESSAGE_SIZE as u64 {
        return Err(SyscallError::MessageTooLarge);
    }

    // TODO: Validate data_ptr is in sender's address space
    if data_ptr == 0 && data_len > 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Build a Message from raw data pointer
    let message = if data_len > 0 {
        unsafe {
            let ptr = data_ptr as *const ipc::Message;
            *ptr
        }
    } else {
        // Default empty message
        unsafe { core::mem::zeroed() }
    };

    ipc::manager()
        .slowpath_send(endpoint_id, &message, sender_pid)
        .map_err(|_| SyscallError::EndpointNotFound)?;

    Ok(0)
}

/// Receive IPC message system call
fn syscall_receive(receiver_pid: ProcessId, endpoint_id: EndpointId) -> SyscallResult {
    let result = ipc::manager()
        .receive(endpoint_id, receiver_pid)
        .map_err(|_| SyscallError::EndpointNotFound)?;

    match result {
        Some(_message) => {
            // Message received successfully
            // In practice, would copy message data to receiver's address space
            Ok(0)
        }
        None => {
            // No message available, process has been blocked
            Ok(0)
        }
    }
}

/// Reply to IPC message system call  
fn syscall_reply(_sender_pid: ProcessId, _data_ptr: u64, _data_len: u64) -> SyscallResult {
    // TODO: Implement IPC reply mechanism
    Ok(0)
}

/// Allocate memory system call
fn syscall_allocate_memory(pid: ProcessId, size: u64, flags: u64) -> SyscallResult {
    if size == 0 || size > 1024 * 1024 * 1024 {  // Limit to 1GB
        return Err(SyscallError::InvalidArgument);
    }
    
    // Round size up to page boundary
    let page_aligned_size = (size + crate::PAGE_SIZE as u64 - 1) & !(crate::PAGE_SIZE as u64 - 1);
    let page_count = (page_aligned_size as usize) / crate::PAGE_SIZE;
    
    // Convert flags to PageFlags
    let mut page_flags = PageFlags::USER_ACCESSIBLE;
    page_flags |= PageFlags::PRESENT; // All mapped pages are readable
    if flags & 0x2 != 0 { page_flags |= PageFlags::WRITABLE; }
    // Note: EXECUTABLE = !NO_EXECUTE, handled by default (no NX bit set)
    
    // Allocate memory in process address space
    process::with_process_manager(|pm| {
        if let Some(process) = pm.get_process_mut(pid) {
            memory::with_memory_manager(|mm| {
                // For now, allocate at a fixed virtual address range
                // In a real implementation, this would find free regions
                let base_addr = 0x10000000u64 + (pid.as_u16() as u64 * 0x1000000); // 16MB per process
                
                // Allocate physical frames and map them
                for i in 0..page_count {
                    if let Some(frame) = mm.allocate_frame() {
                        let virt_addr = VirtAddr::new_unchecked(base_addr + (i as u64 * crate::PAGE_SIZE as u64));
                        let page = Page::from_address(virt_addr);
                        
                        if let Err(_) = mm.map_page(&mut process.address_space, page, frame, page_flags) {
                            // Cleanup on failure
                            mm.deallocate_frame(frame);
                            return Err(SyscallError::OutOfMemory);
                        }
                    } else {
                        return Err(SyscallError::OutOfMemory);
                    }
                }
                
                Ok(base_addr)
            })
        } else {
            Err(SyscallError::ProcessNotFound)
        }
    })
}

/// Deallocate memory system call
fn syscall_deallocate_memory(pid: ProcessId, addr: u64, size: u64) -> SyscallResult {
    if addr == 0 || size == 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Ensure size is page-aligned
    if size % crate::PAGE_SIZE as u64 != 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Get process and deallocate memory from its address space
    process::with_process_manager(|pm| {
        if let Some(process) = pm.get_process_mut(pid) {
            // Deallocate memory from process address space
            let virt_addr = VirtAddr::new_unchecked(addr);
            let page_count = (size as usize) / crate::PAGE_SIZE;

            // Unmap pages from process virtual memory
            memory::with_memory_manager(|mm| {
                for i in 0..page_count {
                    let page_addr = VirtAddr::new_unchecked(virt_addr.as_u64() + (i as u64 * crate::PAGE_SIZE as u64));
                    let page = Page::from_address(page_addr);

                    if let Ok(frame) = mm.unmap_page(&mut process.address_space, page) {
                        mm.deallocate_frame(frame);
                    }
                }
            });
            Ok(0)
        } else {
            Err(SyscallError::ProcessNotFound)
        }
    })
}

/// Map memory system call
fn syscall_map_memory(_pid: ProcessId, _virt_addr: u64, _phys_addr: u64, _flags: u64) -> SyscallResult {
    // TODO: Implement memory mapping
    Ok(0)
}

/// Create process system call
fn syscall_create_process(parent_pid: ProcessId, elf_ptr: u64, elf_size: u64) -> SyscallResult {
    if elf_ptr == 0 || elf_size == 0 || elf_size > 10 * 1024 * 1024 {  // Limit to 10MB
        return Err(SyscallError::InvalidArgument);
    }
    
    // TODO: Validate elf_ptr is in parent's address space
    
    let elf_data = unsafe {
        core::slice::from_raw_parts(elf_ptr as *const u8, elf_size as usize)
    };
    
    let child_pid = process::with_process_manager(|pm| {
        pm.create_process(elf_data, Some(parent_pid))
    }).map_err(|_| SyscallError::OutOfMemory)?;
    
    Ok(child_pid.as_u16() as u64)
}

/// Yield CPU system call
fn syscall_yield_cpu(pid: ProcessId) -> SyscallResult {
    // Mark current process as yielding and trigger scheduler
    process::with_process_manager(|pm| {
        if let Some(process) = pm.get_process_mut(pid) {
            process.state = ProcessState::Ready; // Mark as ready to run
            process.stats.voluntary_switches += 1;
        }
        
        // Trigger context switch to next process
        if let Some(next_pid) = pm.schedule() {
            pm.set_current_process(Some(next_pid));
            // Context switch would happen here in real implementation
            // For now, just return success
        }
    });
    
    Ok(0)
}

/// Get process information system call
fn syscall_get_process_info(_requester_pid: ProcessId, target_pid: ProcessId) -> SyscallResult {
    // TODO: Check if requester has permission to access target process info

    let info = process::with_process_manager(|pm| {
        pm.get_process(target_pid).map(|p| {
            // Return basic process information
            // In practice, would copy to userspace buffer
            p.stats.cpu_time
        })
    });

    match info {
        Some(cpu_time) => Ok(cpu_time),
        None => Err(SyscallError::ProcessNotFound),
    }
}

/// Get system call statistics
pub fn get_stats() -> &'static SyscallStats {
    unsafe { &SYSCALL_STATS }
}

/// Register driver system call
fn syscall_register_driver(pid: ProcessId, _name_ptr: u64, _capabilities_ptr: u64, _classes_ptr: u64, _count: u64) -> SyscallResult {
    // In a real implementation, would validate pointers and read data from user memory
    // For now, provide a simplified implementation
    
    use crate::driver::*;
    
    let capabilities = DriverCapabilities {
        can_access_memory: true,
        can_handle_interrupts: true,
        can_perform_dma: false,
        max_memory_regions: 4,
        allowed_ports: PortRange { start: 0x60, end: 0x6F },
    };
    
    let device_classes = alloc::vec![DeviceClass::Input]; // Simplified
    
    match crate::driver::register_driver(
        alloc::format!("Driver-{}", pid.as_u16()),
        alloc::string::String::from("1.0.0"),
        pid,
        capabilities,
        device_classes,
    ) {
        Ok(driver_id) => Ok(driver_id.as_u32() as u64),
        Err(_) => Err(SyscallError::PermissionDenied),
    }
}

/// Set driver state system call
fn syscall_set_driver_state(pid: ProcessId, driver_id: DriverId, state: u64) -> SyscallResult {
    use crate::driver::*;
    
    // Verify the process owns this driver
    if let Some(process_driver_id) = find_driver_by_process(pid) {
        if process_driver_id != driver_id {
            return Err(SyscallError::PermissionDenied);
        }
    } else {
        return Err(SyscallError::PermissionDenied);
    }
    
    let driver_state = match state {
        0 => DriverState::Unloaded,
        1 => DriverState::Loading,
        2 => DriverState::Ready,
        3 => DriverState::Active,
        4 => DriverState::Suspended,
        5 => DriverState::Error,
        6 => DriverState::Unloading,
        _ => return Err(SyscallError::InvalidArgument),
    };
    
    match set_driver_state(driver_id, driver_state) {
        Ok(()) => Ok(0),
        Err(_) => Err(SyscallError::InvalidArgument),
    }
}

/// Reset system call statistics
pub fn reset_stats() {
    unsafe {
        SYSCALL_STATS = SyscallStats::default();
    }
}
// Helper functions for driver management
fn find_driver_by_process(_pid: ProcessId) -> Option<DriverId> {
    // TODO: Implement driver registry lookup
    None
}

fn set_driver_state(_driver_id: DriverId, _state: crate::driver::DriverState) -> core::result::Result<(), ()> {
    // TODO: Implement driver state management
    Ok(())
}
