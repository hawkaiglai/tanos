//! System timer management

use x86_64::instructions::port::Port;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// Programmable Interval Timer (PIT) frequency
const PIT_FREQUENCY: u64 = 1193182;

/// Target timer frequency (1000 Hz = 1ms ticks)
const TIMER_FREQUENCY: u64 = 1000;

/// Timer interval in PIT ticks
const TIMER_DIVISOR: u16 = (PIT_FREQUENCY / TIMER_FREQUENCY) as u16;

/// System timer
pub struct Timer {
    ticks: AtomicU64,
    start_time: AtomicU64,
    statistics: Mutex<TimerStatistics>,
}

#[derive(Debug, Default, Clone, Copy)]
struct TimerStatistics {
    total_ticks: u64,
    missed_ticks: u64,
    max_tick_time: u64,
    avg_tick_time: u64,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            ticks: AtomicU64::new(0),
            start_time: AtomicU64::new(0),
            statistics: Mutex::new(TimerStatistics::default()),
        }
    }
    
    /// Initialize timer
    pub fn init(&self) {
        self.start_time.store(super::rdtsc(), Ordering::SeqCst);
        
        // Configure PIT Channel 0 for periodic interrupts
        let mut command_port = Port::new(0x43);
        let mut data_port = Port::new(0x40);
        
        unsafe {
            // Set command byte:
            // - Channel 0
            // - Access mode: lo/hi byte
            // - Operating mode: rate generator
            // - Binary mode
            command_port.write(0x34u8);
            
            // Set divisor (low byte first, then high byte)
            data_port.write((TIMER_DIVISOR & 0xFF) as u8);
            data_port.write((TIMER_DIVISOR >> 8) as u8);
        }
        
        crate::info!("Timer initialized: {} Hz ({} PIT ticks)", 
                  TIMER_FREQUENCY, TIMER_DIVISOR);
    }
    
    /// Handle timer tick
    pub fn tick(&self) {
        let start_cycles = super::rdtsc();
        
        let _tick_count = self.ticks.fetch_add(1, Ordering::SeqCst) + 1;
        
        // Update statistics
        let mut stats = self.statistics.lock();
        stats.total_ticks += 1;
        
        let end_cycles = super::rdtsc();
        let tick_time = end_cycles - start_cycles;
        
        if tick_time > stats.max_tick_time {
            stats.max_tick_time = tick_time;
        }
        
        // Update average (running average)
        stats.avg_tick_time = (stats.avg_tick_time * (stats.total_ticks - 1) + tick_time) / stats.total_ticks;
        
        // Check for missed ticks (if tick time is too long)
        if tick_time > 1000 { // More than 1000 cycles is concerning
            stats.missed_ticks += 1;
        }
    }
    
    /// Get current tick count
    pub fn ticks(&self) -> u64 {
        self.ticks.load(Ordering::SeqCst)
    }
    
    /// Get uptime in milliseconds
    pub fn uptime_ms(&self) -> u64 {
        self.ticks() // 1 tick = 1 ms
    }
    
    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.uptime_ms() / 1000
    }
    
    /// Get high-resolution timestamp
    pub fn timestamp(&self) -> u64 {
        super::rdtsc()
    }
    
    /// Get system start time (in TSC cycles)
    pub fn start_time(&self) -> u64 {
        self.start_time.load(Ordering::SeqCst)
    }
    
    /// Get timer statistics
    pub fn statistics(&self) -> TimerStatistics {
        *self.statistics.lock()
    }
    
    /// Sleep for specified milliseconds (busy wait)
    pub fn sleep_ms(&self, ms: u64) {
        let start_ticks = self.ticks();
        while self.ticks() - start_ticks < ms {
            core::hint::spin_loop();
        }
    }
    
    /// Sleep for specified microseconds (busy wait)
    pub fn sleep_us(&self, us: u64) {
        let start_cycles = super::rdtsc();
        
        // Estimate CPU frequency from timer ticks
        // This is approximate but good enough for short sleeps
        let cpu_freq_mhz = 2000; // Assume 2GHz CPU
        let target_cycles = us * cpu_freq_mhz / 1000;
        
        while super::rdtsc() - start_cycles < target_cycles {
            core::hint::spin_loop();
        }
    }
}
