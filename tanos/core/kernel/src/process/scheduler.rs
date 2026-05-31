extern crate alloc;


use super::*;
use crate::*;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

pub const NUM_PRIORITIES: usize = 5;
pub const INITIAL_QUANTUM: u32 = 10; // 10ms
pub const MIN_QUANTUM: u32 = 1;
pub const MAX_QUANTUM: u32 = 100;

pub struct Scheduler {
    ready_queues: [RunQueue; NUM_PRIORITIES],
    current: Option<ProcessId>,
    idle_process: Option<ProcessId>,
    tick_count: u64,
    total_runtime: u64,
}

struct RunQueue {
    processes: VecDeque<ProcessId>,
    quantum_boost: u32,
}

impl RunQueue {
    fn new() -> Self {
        Self {
            processes: VecDeque::new(),
            quantum_boost: 0,
        }
    }
    
    fn enqueue(&mut self, pid: ProcessId) {
        self.processes.push_back(pid);
    }
    
    fn dequeue(&mut self) -> Option<ProcessId> {
        self.processes.pop_front()
    }
    
    fn is_empty(&self) -> bool {
        self.processes.is_empty()
    }
    
    fn len(&self) -> usize {
        self.processes.len()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            ready_queues: [
                RunQueue::new(), RunQueue::new(), RunQueue::new(),
                RunQueue::new(), RunQueue::new()
            ],
            current: None,
            idle_process: None,
            tick_count: 0,
            total_runtime: 0,
        }
    }
    
    pub fn add_process(&mut self, pid: ProcessId) {
        // Add to normal priority queue by default
        self.ready_queues[Priority::Normal.as_usize()].enqueue(pid);
    }
    
    pub fn remove_process(&mut self, pid: ProcessId) {
        // Remove from all queues
        for queue in &mut self.ready_queues {
            queue.processes.retain(|&p| p != pid);
        }
        
        if self.current == Some(pid) {
            self.current = None;
        }
    }
    
    /// O(1) scheduler - finds highest priority ready process
    pub fn schedule(&mut self) -> Option<ProcessId> {
        // Save current process if it's still runnable
        if let Some(current_pid) = self.current {
            if let Some(process) = super::with_process_manager(|pm| {
                pm.get_process(current_pid).map(|p| (p.state, p.priority))
            }) {
                let (state, priority) = process;
                if state == super::process::ProcessState::Running {
                    // Put back in ready queue
                    self.ready_queues[priority.as_usize()].enqueue(current_pid);
                }
            }
        }
        
        // Find highest priority ready process
        for (_priority_level, queue) in self.ready_queues.iter_mut().enumerate().rev() {
            if let Some(next_pid) = queue.dequeue() {
                self.current = Some(next_pid);
                
                // Update process state
                super::with_process_manager(|pm| {
                    if let Some(process) = pm.get_process_mut(next_pid) {
                        process.set_state(super::process::ProcessState::Running);
                        process.stats.context_switches += 1;
                    }
                });
                
                return Some(next_pid);
            }
        }
        
        // No ready processes, return idle if available
        self.current = self.idle_process;
        self.idle_process
    }
    
    pub fn tick(&mut self) {
        self.tick_count += 1;
        self.total_runtime += 1;
        
        if let Some(current_pid) = self.current {
            // Update current process quantum and stats
            super::with_process_manager(|pm| {
                if let Some(process) = pm.get_process_mut(current_pid) {
                    process.stats.cpu_time += 1;
                    
                    if process.quantum > 0 {
                        process.quantum -= 1;
                    }
                    
                    // Check if quantum expired
                    if process.quantum == 0 {
                        // Recalculate quantum based on priority
                        process.quantum = calculate_quantum(process.priority);
                        
                        // Mark for rescheduling
                        process.set_state(super::process::ProcessState::Ready);
                        
                        // Trigger reschedule
                        self.current = None;
                    }
                }
            });
        }
        
        // Periodic priority boost to prevent starvation
        if self.tick_count % 1000 == 0 {
            self.priority_boost();
        }
    }
    
    pub fn block_current(&mut self, reason: super::process::ProcessState) {
        if let Some(current_pid) = self.current {
            super::with_process_manager(|pm| {
                if let Some(process) = pm.get_process_mut(current_pid) {
                    process.set_state(reason);
                }
            });
            self.current = None;
        }
    }
    
    pub fn unblock_process(&mut self, pid: ProcessId) {
        super::with_process_manager(|pm| {
            if let Some(process) = pm.get_process_mut(pid) {
                if matches!(process.state, 
                    super::process::ProcessState::Blocked |
                    super::process::ProcessState::SendWait |
                    super::process::ProcessState::ReceiveWait |
                    super::process::ProcessState::ReplyWait
                ) {
                    process.set_state(super::process::ProcessState::Ready);
                    let priority = process.priority;
                    self.ready_queues[priority.as_usize()].enqueue(pid);
                }
            }
        });
    }
    
    pub fn boost_priority(&mut self, pid: ProcessId) {
        super::with_process_manager(|pm| {
            if let Some(process) = pm.get_process_mut(pid) {
                // Temporarily boost to high priority
                if process.priority != Priority::Critical {
                    let old_priority = process.priority;
                    process.priority = Priority::High;
                    
                    // If process is ready, move to high priority queue
                    if process.state == super::process::ProcessState::Ready {
                        // Remove from old queue and add to high priority
                        self.ready_queues[old_priority.as_usize()].processes.retain(|&p| p != pid);
                        self.ready_queues[Priority::High.as_usize()].enqueue(pid);
                    }
                }
            }
        });
    }
    
    fn priority_boost(&mut self) {
        // Move all processes from lower priority queues to higher ones
        // This prevents starvation
        for priority in 0..NUM_PRIORITIES-1 {
            while let Some(pid) = self.ready_queues[priority].dequeue() {
                self.ready_queues[priority + 1].enqueue(pid);
                
                // Update process priority
                super::with_process_manager(|pm| {
                    if let Some(process) = pm.get_process_mut(pid) {
                        if priority + 1 < NUM_PRIORITIES {
                            process.priority = match priority + 1 {
                                0 => Priority::Idle,
                                1 => Priority::Low,
                                2 => Priority::Normal,
                                3 => Priority::High,
                                4 => Priority::Critical,
                                _ => Priority::Normal,
                            };
                        }
                    }
                });
            }
        }
    }
    
    pub fn current(&self) -> Option<ProcessId> {
        self.current
    }
    
    /// Set idle process
    pub fn set_idle_process(&mut self, pid: ProcessId) {
        self.idle_process = Some(pid);
    }
    
    pub fn statistics(&self) -> SchedulerStats {
        let queue_lengths: Vec<usize> = self.ready_queues.iter()
            .map(|q| q.len())
            .collect();
        
        SchedulerStats {
            tick_count: self.tick_count,
            total_runtime: self.total_runtime,
            current_process: self.current,
            queue_lengths,
        }
    }
}

pub struct SchedulerStats {
    pub tick_count: u64,
    pub total_runtime: u64,
    pub current_process: Option<ProcessId>,
    pub queue_lengths: Vec<usize>,
}

pub fn calculate_quantum(priority: Priority) -> u32 {
    match priority {
        Priority::Idle => 5,
        Priority::Low => 10,
        Priority::Normal => 20,
        Priority::High => 30,
        Priority::Critical => 50,
    }
}

/// Start the scheduler - this never returns

fn run_process(pid: ProcessId) {
    super::with_process_manager(|pm| {
        if let Some(process) = pm.get_process(pid) {
            // Activate the process address space
            process.address_space.activate();
            
            // The actual context switch would happen here
            // This is simplified - real implementation would use assembly
        }
    });
}

/// Called on timer tick
pub fn tick() {
    // Update time slices, handle round-robin scheduling
    // TODO: Implement proper time slice accounting
}

/// Wake a blocked process
pub fn wake_process(_pid: ProcessId) {
    // Move process from blocked to ready queue
    // TODO: Implement process state transitions
}

/// Boost priority of interactive process
pub fn boost_priority(_pid: ProcessId) {
    // Increase priority for responsive UI
    // TODO: Implement dynamic priority adjustment
}

/// Start the scheduler - this never returns
pub fn start(init_pid: ProcessId) -> ! {
    crate::debug::serial::println("Starting scheduler...");
    
    super::with_process_manager(|pm| {
        // Set the init process as current
        pm.set_current_process(Some(init_pid));
        
        // Mark init process as running
        if let Some(process) = pm.get_process_mut(init_pid) {
            process.state = super::process::ProcessState::Running;
        }
    });
    
    // Main scheduler loop
    loop {
        if let Some(next_pid) = schedule() {
            // TODO: Implement actual process switching
            crate::debug!("Would switch to process {:?}", next_pid);
        }
        
        // Yield CPU
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
