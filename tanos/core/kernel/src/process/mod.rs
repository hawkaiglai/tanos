pub mod process;
pub use process::{Process, ProcessState, Priority};
pub mod scheduler;
pub mod context;

use crate::boot::BootInfo;
use crate::*;
use spin::{Mutex, Once};
use alloc::vec::Vec;
use alloc::boxed::Box;

static PROCESS_MANAGER: Once<Mutex<ProcessManager>> = Once::new();

/// Cached initrd ELF image (identity-mapped physical address, length) so a
/// process can be (re)spawned from it — used for reincarnation after a crash.
static INITRD: Once<(u64, usize)> = Once::new();

/// Maximum reincarnation generation. Generation 0 is the original; a process
/// is restarted at most up to this generation, after which the kernel gives up
/// (so a persistently-crashing process can't loop forever).
const MAX_GENERATION: u32 = 1;

pub struct ProcessManager {
    processes: Vec<Option<Box<process::Process>>>,
    next_pid: u16,
    current_process: Option<ProcessId>,
    scheduler: scheduler::Scheduler,
}

impl ProcessManager {
    fn new() -> Self {
        let mut processes = Vec::with_capacity(crate::MAX_PROCESSES);
        processes.resize_with(crate::MAX_PROCESSES, || None);
        
        Self {
            processes,
            next_pid: 1, // PID 0 reserved for kernel
            current_process: None,
            scheduler: scheduler::Scheduler::new(),
        }
    }
    
    pub fn create_process(&mut self, elf_data: &[u8], parent: Option<ProcessId>) -> core::result::Result<ProcessId, ProcessError> {
        let pid = self.allocate_pid()?;
        let process = process::Process::create(pid, elf_data, parent)?;
        
        self.processes[pid.as_u16() as usize] = Some(Box::new(process));
        self.scheduler.add_process(pid);
        
        Ok(pid)
    }
    
    pub fn get_process(&self, pid: ProcessId) -> Option<&process::Process> {
        self.processes.get(pid.as_u16() as usize)?
            .as_ref()
            .map(|p| p.as_ref())
    }
    
    pub fn get_process_mut(&mut self, pid: ProcessId) -> Option<&mut process::Process> {
        self.processes.get_mut(pid.as_u16() as usize)?
            .as_mut()
            .map(|p| p.as_mut())
    }
    
    /// Remove a process from the table (and the scheduler), returning its boxed
    /// Process so the caller controls exactly when it is dropped — dropping it
    /// reclaims its entire address space (see AddressSpace::drop). Used by the
    /// death paths, which drop the corpse only after switching CR3 to another
    /// process so the freed frames are never the active page tables.
    fn take_process(&mut self, pid: ProcessId) -> Option<Box<process::Process>> {
        let taken = self.processes.get_mut(pid.as_u16() as usize)?.take();
        if taken.is_some() {
            self.scheduler.remove_process(pid);
        }
        taken
    }

    pub fn kill_process(&mut self, pid: ProcessId) -> core::result::Result<(), ProcessError> {
        if let Some(_process) = self.processes.get_mut(pid.as_u16() as usize).unwrap().take() {
            self.scheduler.remove_process(pid);
            // Process will be dropped here, cleaning up resources
            Ok(())
        } else {
            Err(ProcessError::ProcessNotFound)
        }
    }
    
    pub fn schedule(&mut self) -> Option<ProcessId> {
        self.scheduler.schedule()
    }

    /// Round-robin: the next runnable process after the current one (wrapping),
    /// excluding the current process. Returns None if no other process is
    /// runnable.
    fn pick_next(&self) -> Option<ProcessId> {
        let n = self.processes.len();
        if n == 0 {
            return None;
        }
        let cur = self.current_process.map(|p| p.as_u16() as usize).unwrap_or(0);
        for off in 1..=n {
            let idx = (cur + off) % n;
            if let Some(ref p) = self.processes[idx] {
                if matches!(p.state, process::ProcessState::Ready | process::ProcessState::Running) {
                    return Some(p.id);
                }
            }
        }
        None
    }
    
    pub fn current_process(&self) -> Option<ProcessId> {
        self.current_process
    }
    
    pub fn set_current_process(&mut self, pid: Option<ProcessId>) {
        self.current_process = pid;
    }
    
    fn allocate_pid(&mut self) -> core::result::Result<ProcessId, ProcessError> {
        let start_pid = self.next_pid;
        
        loop {
            let pid = ProcessId::new_const(self.next_pid);
            self.next_pid = self.next_pid.wrapping_add(1);
            
            if self.next_pid == 0 {
                self.next_pid = 1; // Skip kernel PID
            }
            
            if self.processes[pid.as_u16() as usize].is_none() {
                return Ok(pid);
            }
            
            if self.next_pid == start_pid {
                return Err(ProcessError::OutOfProcessSlots);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProcessError {
    OutOfProcessSlots,
    ProcessNotFound,
    InvalidElf,
    OutOfMemory,
    PermissionDenied,
}

pub fn init() {
    let process_manager = ProcessManager::new();
    PROCESS_MANAGER.call_once(|| Mutex::new(process_manager));
}

pub fn with_process_manager<F, R>(f: F) -> R
where
    F: FnOnce(&mut ProcessManager) -> R,
{
    let process_manager = PROCESS_MANAGER.get()
        .expect("Process manager not initialized");
    
    f(&mut process_manager.lock())
}

pub fn load_init(boot_info: &BootInfo) -> core::result::Result<ProcessId, ProcessError> {
    // Load init process from initrd
    if let (Some(start), Some(end)) = (boot_info.initrd_start, boot_info.initrd_end) {
        let size = (end.as_u64() - start.as_u64()) as usize;
        // Cache the initrd image so processes can be (re)spawned from it later.
        INITRD.call_once(|| (start.as_u64(), size));
        let initrd_data = unsafe {
            core::slice::from_raw_parts(start.as_u64() as *const u8, size)
        };

        // For now, assume entire initrd is the init ELF
        // In practice, this would be a filesystem
        with_process_manager(|pm| {
            pm.create_process(initrd_data, None)
        })
    } else {
        // No initrd available - return error
        // In a real implementation, we'd have a built-in init
        Err(ProcessError::InvalidElf)
    }
}

/// Start scheduling: resume the first runnable process (PID 1) in ring 3.
/// Does not return — control transfers to userspace, which re-enters the
/// kernel via syscalls/interrupts.
pub fn start() -> ! {
    let (saved, cr3) = with_process_manager(|pm| {
        let pid = crate::INIT_PID;
        pm.set_current_process(Some(pid));
        let p = pm.get_process_mut(pid).expect("init process not loaded");
        p.state = process::ProcessState::Running;
        (p.saved, p.address_space.cr3().as_u64())
    });
    crate::info!("Starting scheduler: resuming PID 1 (cr3={:#x})", cr3);
    unsafe { crate::interrupt::resume_user(&saved as *const _, cr3) }
}

/// Cooperative yield: save the current process's register frame, pick the next
/// runnable process round-robin, and resume it. If no other process is
/// runnable, returns so the caller resumes the current process.
///
/// The process-manager lock is released before resuming, since the resumed
/// process will re-enter the kernel and need the lock.
pub fn schedule_yield(frame: &crate::interrupt::InterruptFrame) {
    let next = with_process_manager(|pm| {
        let cur = pm.current_process;
        if let Some(cur) = cur {
            if let Some(p) = pm.get_process_mut(cur) {
                p.saved = *frame;
                p.saved.rax = 0; // yield() returns 0 to the caller
                p.state = process::ProcessState::Ready;
            }
        }
        let next_pid = pm.pick_next()?;
        pm.set_current_process(Some(next_pid));
        let p = pm.get_process_mut(next_pid)?;
        p.state = process::ProcessState::Running;
        let cur_n = cur.map(|c| c.as_u16()).unwrap_or(0);
        crate::info!("yield: PID {} -> PID {} (resume rip={:#x} rsp={:#x} cr3={:#x})",
            cur_n, next_pid.as_u16(), p.saved.rip, p.saved.rsp, p.address_space.cr3().as_u64());
        Some((p.saved, p.address_space.cr3().as_u64()))
    });

    if let Some((saved, cr3)) = next {
        unsafe { crate::interrupt::resume_user(&saved as *const _, cr3) }
    }
    // No other runnable process — fall through; caller resumes current.
}

// ─── Minimal synchronous IPC (single rendezvous, for the call/reply demo) ────
//
// Models one synchronous channel: a server blocks in receive, a client blocks
// in call awaiting a reply. Messages are a single u64 word passed in a register
// (seL4-style register fastpath). Switching reuses the save-to-PCB + resume
// mechanism. This is the irreducible microkernel primitive being measured.

struct Rendezvous {
    /// A process blocked in `receive`, waiting for a message.
    receiver: Option<ProcessId>,
    /// A process blocked in `call`, waiting for a reply.
    caller: Option<ProcessId>,
    /// A message sent by a caller before any receiver was ready.
    pending_msg: u64,
    has_pending: bool,
}

static RENDEZVOUS: Mutex<Rendezvous> = Mutex::new(Rendezvous {
    receiver: None,
    caller: None,
    pending_msg: 0,
    has_pending: false,
});

/// `call`: send `msg` and block until a reply. Always switches away (the caller
/// blocks in ReplyWait); resumes the server (fastpath) or the next runnable
/// process. Does not return — the caller is resumed later by `reply`.
pub fn ipc_call(frame: &crate::interrupt::InterruptFrame, _ep: u64, msg: u64) -> ! {
    let (saved, cr3) = with_process_manager(|pm| {
        let cur = pm.current_process.expect("ipc_call: no current process");
        if let Some(p) = pm.get_process_mut(cur) {
            p.saved = *frame;
            p.state = process::ProcessState::ReplyWait;
        }
        let mut rdv = RENDEZVOUS.lock();
        rdv.caller = Some(cur);

        if let Some(server) = rdv.receiver.take() {
            // Fastpath: a receiver is already waiting — hand it the message.
            if let Some(s) = pm.get_process_mut(server) {
                s.saved.rax = msg;
                s.state = process::ProcessState::Running;
            }
            pm.set_current_process(Some(server));
            let s = pm.get_process(server).unwrap();
            (s.saved, s.address_space.cr3().as_u64())
        } else {
            // No receiver yet — stash the message, run someone else.
            rdv.pending_msg = msg;
            rdv.has_pending = true;
            let next = pm.pick_next().expect("ipc_call: no runnable process");
            pm.set_current_process(Some(next));
            let n = pm.get_process_mut(next).unwrap();
            n.state = process::ProcessState::Running;
            (n.saved, n.address_space.cr3().as_u64())
        }
    });
    unsafe { crate::interrupt::resume_user(&saved as *const _, cr3) }
}

/// `receive`: get the next message. Returns it in RAX if one is pending
/// (no switch); otherwise blocks in ReceiveWait and switches to the next
/// runnable process (resumed later by a caller's `call`).
pub fn ipc_receive(frame: &crate::interrupt::InterruptFrame, _ep: u64) -> u64 {
    let outcome: core::result::Result<u64, (crate::interrupt::InterruptFrame, u64)> =
        with_process_manager(|pm| {
            let cur = pm.current_process.expect("ipc_receive: no current process");
            let mut rdv = RENDEZVOUS.lock();
            if rdv.has_pending {
                rdv.has_pending = false;
                return Ok(rdv.pending_msg); // deliver immediately, keep running
            }
            if let Some(p) = pm.get_process_mut(cur) {
                p.saved = *frame;
                p.state = process::ProcessState::ReceiveWait;
            }
            rdv.receiver = Some(cur);
            let next = pm.pick_next().expect("ipc_receive: no runnable process");
            pm.set_current_process(Some(next));
            let n = pm.get_process_mut(next).unwrap();
            n.state = process::ProcessState::Running;
            Err((n.saved, n.address_space.cr3().as_u64()))
        });
    match outcome {
        Ok(msg) => msg,
        Err((saved, cr3)) => unsafe { crate::interrupt::resume_user(&saved as *const _, cr3) },
    }
}

/// `reply`: deliver `rval` to the blocked caller (making it runnable) and
/// return 0 to the server, which continues (and typically loops back to
/// `receive`, at which point control returns to the caller).
pub fn ipc_reply(_frame: &crate::interrupt::InterruptFrame, rval: u64) -> u64 {
    with_process_manager(|pm| {
        let mut rdv = RENDEZVOUS.lock();
        if let Some(caller) = rdv.caller.take() {
            if let Some(c) = pm.get_process_mut(caller) {
                c.saved.rax = rval;
                c.state = process::ProcessState::Ready;
            }
        }
    });
    0
}

/// `reply_and_receive` (seL4-style ReplyRecv): atomically deliver `rval` to the
/// blocked caller and block to receive the next message — one syscall instead
/// of separate reply + receive, halving the server's syscall count per
/// round-trip. Returns the next message in RAX, or switches away and is resumed
/// later by a caller.
pub fn ipc_reply_recv(frame: &crate::interrupt::InterruptFrame, rval: u64, _ep: u64) -> u64 {
    let outcome: core::result::Result<u64, (crate::interrupt::InterruptFrame, u64)> =
        with_process_manager(|pm| {
            let cur = pm.current_process.expect("ipc_reply_recv: no current process");
            let mut rdv = RENDEZVOUS.lock();

            // 1. Reply to the caller currently awaiting it.
            if let Some(caller) = rdv.caller.take() {
                if let Some(c) = pm.get_process_mut(caller) {
                    c.saved.rax = rval;
                    c.state = process::ProcessState::Ready;
                }
            }

            // 2. Receive the next message (deliver if pending, else block).
            if rdv.has_pending {
                rdv.has_pending = false;
                return Ok(rdv.pending_msg);
            }
            if let Some(p) = pm.get_process_mut(cur) {
                p.saved = *frame;
                p.state = process::ProcessState::ReceiveWait;
            }
            rdv.receiver = Some(cur);
            let next = pm.pick_next().expect("ipc_reply_recv: no runnable process");
            pm.set_current_process(Some(next));
            let n = pm.get_process_mut(next).unwrap();
            n.state = process::ProcessState::Running;
            Err((n.saved, n.address_space.cr3().as_u64()))
        });
    match outcome {
        Ok(msg) => msg,
        Err((saved, cr3)) => unsafe { crate::interrupt::resume_user(&saved as *const _, cr3) },
    }
}

/// Spawn a fresh process from the cached initrd ELF, tagged with the given
/// reincarnation `generation` (passed to the new process's `_start` in RDI).
pub fn spawn_from_initrd(generation: u32) -> core::result::Result<ProcessId, ProcessError> {
    let (addr, size) = *INITRD.get().ok_or(ProcessError::InvalidElf)?;
    let elf = unsafe { core::slice::from_raw_parts(addr as *const u8, size) };
    with_process_manager(|pm| {
        let pid = pm.create_process(elf, None)?;
        if let Some(p) = pm.get_process_mut(pid) {
            p.generation = generation;
            p.saved.rdi = generation as u64; // _start(generation): SysV arg0 = RDI
        }
        Ok(pid)
    })
}

/// Handle a fatal ring-3 CPU fault: the faulting process is terminated, and —
/// the MINIX-3 "reincarnation" idea — restarted from its image up to
/// MAX_GENERATION times. The kernel and all other processes keep running.
/// Does not return.
///
/// (Here reincarnation is driven in-kernel for the demo; the "pure" microkernel
/// design would run a userspace reincarnation server that the kernel notifies.)
pub fn fault_kill_current(vector: u8) -> ! {
    // Remove the faulting process from the table immediately (so it is never
    // scheduled again) but keep its Box alive in `dead` so we control when its
    // frames are reclaimed — only after switching CR3 away from it.
    let (dead, info) = with_process_manager(|pm| {
        match pm.current_process {
            Some(cur) => {
                let gen = pm.get_process(cur).map(|p| p.generation).unwrap_or(0);
                (pm.take_process(cur), Some((cur, gen)))
            }
            None => (None, None),
        }
    });

    let (cur, gen) = match info {
        Some(x) => x,
        None => { drop(dead); loop { unsafe { core::arch::asm!("hlt") }; } }
    };

    crate::error!(
        "fault isolation: PID {} (gen {}) crashed (CPU exception {}) — kernel survives",
        cur.as_u16(), gen, vector
    );

    // Decide what to resume. First choice — the MINIX-3 idea — reincarnate the
    // crashed process from its image (up to MAX_GENERATION). Otherwise fall back
    // to any other runnable process.
    let target: Option<(crate::interrupt::InterruptFrame, u64)> = if gen < MAX_GENERATION {
        match spawn_from_initrd(gen + 1) {
            Ok(newpid) => {
                crate::info!(
                    "reincarnation: restarting crashed PID {} as PID {} (generation {})",
                    cur.as_u16(), newpid.as_u16(), gen + 1
                );
                Some(with_process_manager(|pm| {
                    pm.set_current_process(Some(newpid));
                    let p = pm.get_process_mut(newpid).unwrap();
                    p.state = process::ProcessState::Running;
                    (p.saved, p.address_space.cr3().as_u64())
                }))
            }
            Err(e) => { crate::error!("reincarnation: respawn failed: {:?}", e); None }
        }
    } else {
        crate::warn!(
            "reincarnation: PID {} hit restart limit (gen {}) — not restarting",
            cur.as_u16(), gen
        );
        None
    };
    let target = target.or_else(|| with_process_manager(|pm| {
        let next_pid = pm.pick_next()?;
        pm.set_current_process(Some(next_pid));
        let p = pm.get_process_mut(next_pid)?;
        p.state = process::ProcessState::Running;
        Some((p.saved, p.address_space.cr3().as_u64()))
    }));

    reap_and_resume(dead, target, cur)
}

/// Switch to `target`'s address space, reclaim the dead process's frames (now
/// that we are no longer executing in its address space), report how many were
/// reclaimed, and resume `target`. If there is no target, reclaim and halt.
/// Does not return.
fn reap_and_resume(
    dead: Option<Box<process::Process>>,
    target: Option<(crate::interrupt::InterruptFrame, u64)>,
    dead_pid: ProcessId,
) -> ! {
    match target {
        Some((saved, cr3)) => {
            let before = crate::memory::with_memory_manager(|mm| mm.free_frames());
            // Make `cr3` active BEFORE dropping the corpse, so the page-table
            // frames we free are never the live address space.
            // SAFETY: `cr3` is the PML4 of `target`, a live process whose address
            // space identity-maps the kernel, so execution (code, the current
            // kernel stack, the heap) continues correctly after the switch. We
            // are running on a kernel stack in identity-mapped low RAM, which is
            // mapped identically in `target`, so no stack/instruction pointer
            // becomes invalid across the load.
            unsafe { core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nostack, preserves_flags)); }
            drop(dead);
            let after = crate::memory::with_memory_manager(|mm| mm.free_frames());
            crate::info!(
                "reclaimed {} frames from dead PID {} (free frames {} -> {})",
                after.saturating_sub(before), dead_pid.as_u16(), before, after
            );
            unsafe { crate::interrupt::resume_user(&saved as *const _, cr3) }
        }
        None => {
            drop(dead);
            crate::info!("All processes have exited. Halting.");
            loop { unsafe { core::arch::asm!("hlt") }; }
        }
    }
}

/// Terminate the current process and switch to the next runnable one. If none
/// remain, halts. Does not return.
pub fn exit_current(code: i64) -> ! {
    // Remove the exiting process from the table, holding its Box in `dead` so
    // its frames are reclaimed only after we switch CR3 to the next process.
    let (dead, dead_pid, next) = with_process_manager(|pm| {
        let cur = pm.current_process;
        let dead = cur.and_then(|c| pm.take_process(c));
        if let Some(c) = cur {
            crate::info!("exit: PID {} (code={})", c.as_u16(), code);
        }
        let next = pm.pick_next().map(|next_pid| {
            pm.set_current_process(Some(next_pid));
            let p = pm.get_process_mut(next_pid).unwrap();
            p.state = process::ProcessState::Running;
            (p.saved, p.address_space.cr3().as_u64())
        });
        (dead, cur.unwrap_or(crate::KERNEL_PID), next)
    });

    reap_and_resume(dead, next, dead_pid)
}

pub fn get_current_process() -> Option<ProcessId> {
    with_process_manager(|pm| pm.current_process())
}

pub fn current_process_id() -> Option<ProcessId> {
    get_current_process()
}

pub fn kill_process(pid: ProcessId) -> core::result::Result<(), ProcessError> {
    with_process_manager(|pm| pm.kill_process(pid))
}

pub fn get_mut(pid: ProcessId) -> Option<&'static mut process::Process> {
    // This is unsafe but needed for the syscall interface
    // In a real implementation, this would use proper locking
    unsafe {
        let pm = PROCESS_MANAGER.get()?;
        let mut guard = pm.lock();
        let proc = guard.get_process_mut(pid)?;
        Some(core::mem::transmute(proc))
    }
}

pub fn schedule() -> Option<ProcessId> {
    with_process_manager(|pm| pm.schedule())
}
