extern crate alloc;

use super::*;
use crate::memory::{self, page::AddressSpace};
use crate::*;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Sleeping,
    SendWait,
    ReceiveWait,
    ReplyWait,
    Zombie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Idle = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Priority {
    pub fn as_usize(self) -> usize {
        self as usize
    }
}

#[derive(Debug)]
pub struct ProcessStats {
    pub cpu_time: u64,
    pub context_switches: u64,
    pub voluntary_switches: u64,
    pub page_faults: u64,
    pub syscalls: u64,
}

impl ProcessStats {
    pub fn new() -> Self {
        Self {
            cpu_time: 0,
            context_switches: 0,
            voluntary_switches: 0,
            page_faults: 0,
            syscalls: 0,
        }
    }
}

pub struct Process {
    pub id: ProcessId,
    pub state: ProcessState,
    pub context: super::context::Context,
    /// Saved user register state (the interrupt frame). Updated on every
    /// kernel entry where the process is switched away from; restored to
    /// resume the process via `interrupt::resume_user`.
    pub saved: crate::interrupt::InterruptFrame,
    pub address_space: AddressSpace,
    pub capabilities: Vec<CapabilityId>,
    pub priority: Priority,
    pub quantum: u32,
    pub stats: ProcessStats,
    pub parent: Option<ProcessId>,
    pub children: Vec<ProcessId>,
    /// Reincarnation generation: 0 for the original, incremented each time the
    /// process is restarted after a crash. Passed to the new process in RDI.
    pub generation: u32,
}

impl Process {
    pub fn create(id: ProcessId, elf_data: &[u8], parent: Option<ProcessId>) -> core::result::Result<Self, super::ProcessError> {
        // Create address space
        let mut address_space = if id == crate::KERNEL_PID {
            AddressSpace::kernel_space()
        } else {
            AddressSpace::user_space()
        }.map_err(|_| super::ProcessError::OutOfMemory)?;
        
        // Load ELF
        let entry_point = load_elf(elf_data, &mut address_space)?;

        // Setup initial context, bound to this process's page table.
        let mut context = super::context::Context::new(entry_point, id == crate::KERNEL_PID);
        context.cr3 = address_space.cr3().as_u64();

        // Initial saved frame for first dispatch into ring 3.
        let saved = crate::interrupt::InterruptFrame::new_user(
            entry_point.as_u64(),
            context.rsp,
            context.cs as u64,
            context.ss as u64,
        );
        
        // Initialize capabilities
        let capabilities = if parent.is_some() {
            // Inherit some capabilities from parent
            Vec::new() // Simplified for now
        } else {
            // Root capabilities for init
            Vec::new()
        };
        
        Ok(Self {
            id,
            state: ProcessState::Ready,
            context,
            saved,
            address_space,
            capabilities,
            priority: Priority::Normal,
            quantum: scheduler::INITIAL_QUANTUM,
            stats: ProcessStats::new(),
            parent,
            children: Vec::new(),
            generation: 0,
        })
    }
    
    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }
    
    pub fn set_message(&mut self, _message: &crate::ipc::Message) {
        // Set message in process context for IPC
        // Implementation would store message in process memory
    }

    pub fn set_received_message(&mut self, _message: crate::ipc::Message) {
        // Store received IPC message for process to read
    }

    pub fn set_reply_endpoint(&mut self, _endpoint: EndpointId) {
        // Store reply endpoint for IPC call/reply pattern
    }
}

// ── ELF64 loader ────────────────────────────────────────────────────────────
// Parses a static ELF64 executable, maps each PT_LOAD segment into the target
// address space at its p_vaddr, copies the file contents, and zeroes the BSS
// tail. Userspace binaries are linked non-relocatably at a fixed base (1GB) by
// userspace/userspace.ld, so no relocation processing is required.

const PT_LOAD: u32 = 1;
const ELF_PH_ENTSIZE: usize = 56; // ELF64 program header size

#[inline]
fn rd_u16(d: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([d[off], d[off + 1]])
}
#[inline]
fn rd_u32(d: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
}
#[inline]
fn rd_u64(d: &[u8], off: usize) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&d[off..off + 8]);
    u64::from_le_bytes(b)
}

/// Ensure the 4KB page containing `vaddr` is mapped as a user page in
/// `address_space`, allocating and zeroing a fresh frame if needed. Returns
/// the backing frame (whether newly mapped or already present, since adjacent
/// ELF segments may share a page).
fn ensure_user_page(address_space: &mut AddressSpace, vaddr: u64)
    -> core::result::Result<memory::frame::Frame, super::ProcessError>
{
    let page = memory::page::Page::from_address(VirtAddr::new_unchecked(vaddr & !0xFFF));
    if let Some(frame) = address_space.translate(page) {
        return Ok(frame);
    }
    let frame = memory::with_memory_manager(|mm| mm.allocate_frame())
        .ok_or(super::ProcessError::OutOfMemory)?;
    // SAFETY: the frame was just handed out by the allocator, so it is unaliased
    // and owned by this address space. It lives in low RAM (< 1GB), which is
    // identity-mapped in every address space, so its physical address is a valid
    // writable pointer here. We zero exactly one PAGE_SIZE-sized frame.
    unsafe {
        core::ptr::write_bytes(frame.start_address().as_u64() as *mut u8, 0, crate::PAGE_SIZE);
    }
    let flags = memory::page::PageFlags::PRESENT
        | memory::page::PageFlags::WRITABLE
        | memory::page::PageFlags::USER_ACCESSIBLE;
    address_space.map_page(page, frame, flags)
        .map_err(|_| super::ProcessError::OutOfMemory)?;
    Ok(frame)
}

/// Write `byte` to the physical frame backing user `vaddr`. The frame is
/// reachable through the kernel's identity map (all RAM < 1GB is identity
/// mapped), so we write via its physical address rather than the user vaddr
/// (which is not the active address space).
fn write_user_byte(address_space: &AddressSpace, vaddr: u64, byte: u8)
    -> core::result::Result<(), super::ProcessError>
{
    let page = memory::page::Page::from_address(VirtAddr::new_unchecked(vaddr & !0xFFF));
    let frame = address_space.translate(page).ok_or(super::ProcessError::OutOfMemory)?;
    let dst = (frame.start_address().as_u64() + (vaddr & 0xFFF)) as *mut u8;
    // SAFETY: `frame` backs `page` in this address space (translate succeeded),
    // so it is owned by the process being constructed. The frame is in
    // identity-mapped low RAM, and `vaddr & 0xFFF` keeps the write within the
    // frame's PAGE_SIZE bounds, so `dst` is a valid in-bounds writable pointer.
    unsafe { *dst = byte; }
    Ok(())
}

fn load_elf(elf_data: &[u8], address_space: &mut AddressSpace)
    -> core::result::Result<VirtAddr, super::ProcessError>
{
    use super::ProcessError::InvalidElf;

    // ELF64 header: magic, class==2 (64-bit), little-endian.
    if elf_data.len() < 64 || &elf_data[0..4] != b"\x7fELF" || elf_data[4] != 2 {
        return Err(InvalidElf);
    }

    let e_entry = rd_u64(elf_data, 0x18);
    let e_phoff = rd_u64(elf_data, 0x20) as usize;
    let e_phentsize = rd_u16(elf_data, 0x36) as usize;
    let e_phnum = rd_u16(elf_data, 0x38) as usize;

    if e_phentsize < ELF_PH_ENTSIZE {
        return Err(InvalidElf);
    }

    for i in 0..e_phnum {
        let ph = e_phoff + i * e_phentsize;
        if ph + ELF_PH_ENTSIZE > elf_data.len() {
            return Err(InvalidElf);
        }
        if rd_u32(elf_data, ph) != PT_LOAD {
            continue;
        }
        let p_offset = rd_u64(elf_data, ph + 0x08) as usize;
        let p_vaddr = rd_u64(elf_data, ph + 0x10);
        let p_filesz = rd_u64(elf_data, ph + 0x20) as usize;
        let p_memsz = rd_u64(elf_data, ph + 0x28) as usize;

        if p_offset + p_filesz > elf_data.len() || p_memsz < p_filesz {
            return Err(InvalidElf);
        }

        // Map every page the segment occupies (memsz covers the BSS tail).
        let seg_end = p_vaddr + p_memsz as u64;
        let mut v = p_vaddr & !0xFFF;
        while v < seg_end {
            ensure_user_page(address_space, v)?;
            v += crate::PAGE_SIZE as u64;
        }

        // Copy file contents; the remaining [filesz, memsz) is already zeroed.
        for off in 0..p_filesz {
            write_user_byte(address_space, p_vaddr + off as u64, elf_data[p_offset + off])?;
        }

        crate::info!(
            "  ELF: PT_LOAD vaddr={:#x} filesz={:#x} memsz={:#x} mapped",
            p_vaddr, p_filesz, p_memsz
        );
    }

    // Map an initial user stack just below USER_STACK_TOP.
    const USER_STACK_PAGES: u64 = 4; // 16KB
    let top = crate::USER_STACK_TOP as u64;
    for i in 1..=USER_STACK_PAGES {
        ensure_user_page(address_space, top - i * crate::PAGE_SIZE as u64)?;
    }

    crate::info!("  ELF: entry={:#x}, user stack mapped at {:#x}", e_entry, top);
    Ok(VirtAddr::new_unchecked(e_entry))
}
