//! Stack backtrace functionality

use crate::VirtAddr;
use core::fmt;

/// Stack frame
#[derive(Debug, Clone, Copy)]
pub struct StackFrame {
    pub frame_pointer: VirtAddr,
    pub return_address: VirtAddr,
}

/// Backtrace iterator
pub struct Backtrace {
    current_frame: Option<StackFrame>,
    max_depth: usize,
    current_depth: usize,
}

impl Backtrace {
    /// Create new backtrace from current stack
    pub fn new() -> Self {
        let rbp: u64;
        unsafe {
            core::arch::asm!("mov {}, rbp", out(reg) rbp, options(nomem, nostack, preserves_flags));
        }
        
        Self::from_frame_pointer(VirtAddr::new_unchecked(rbp))
    }
    
    /// Create backtrace from specific frame pointer
    pub fn from_frame_pointer(frame_pointer: VirtAddr) -> Self {
        let current_frame = if frame_pointer.as_u64() != 0 {
            Some(StackFrame {
                frame_pointer,
                return_address: VirtAddr::new_unchecked(0), // Will be filled by first iteration
            })
        } else {
            None
        };
        
        Self {
            current_frame,
            max_depth: 64, // Reasonable limit
            current_depth: 0,
        }
    }
    
    /// Set maximum backtrace depth
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }
}

impl Iterator for Backtrace {
    type Item = StackFrame;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_depth >= self.max_depth {
            return None;
        }
        
        let frame = self.current_frame?;
        self.current_depth += 1;
        
        // Validate frame pointer
        if !is_valid_frame_pointer(frame.frame_pointer) {
            return None;
        }
        
        unsafe {
            // Read the saved frame pointer and return address
            let frame_ptr = frame.frame_pointer.as_u64() as *const u64;
            
            // Frame layout on x86_64:
            // [rbp]     -> previous frame pointer
            // [rbp + 8] -> return address
            
            let next_frame_pointer = VirtAddr::new_unchecked(*frame_ptr);
            let return_address = VirtAddr::new_unchecked(*frame_ptr.add(1));
            
            // Update current frame for next iteration
            self.current_frame = if next_frame_pointer.as_u64() != 0 &&
                                   next_frame_pointer.as_u64() > frame.frame_pointer.as_u64() {
                Some(StackFrame {
                    frame_pointer: next_frame_pointer,
                    return_address: VirtAddr::new_unchecked(0),
                })
            } else {
                None
            };
            
            Some(StackFrame {
                frame_pointer: frame.frame_pointer,
                return_address,
            })
        }
    }
}

/// Check if frame pointer looks valid
fn is_valid_frame_pointer(addr: VirtAddr) -> bool {
    let addr_val = addr.as_u64();
    
    // Basic sanity checks
    if addr_val == 0 {
        return false;
    }
    
    // Must be aligned to 8 bytes
    if addr_val % 8 != 0 {
        return false;
    }
    
    // Must be in valid kernel address space
    // This is a simplified check - in a real implementation,
    // we'd check against actual memory mappings
    if addr_val < 0xFFFF_8000_0000_0000 {
        return false;
    }
    
    // Don't traverse too far up the stack
    if addr_val > 0xFFFF_FFFF_FFFF_0000 {
        return false;
    }
    
    true
}

/// Print backtrace
pub fn print_backtrace() {
    crate::debug::write_fmt(format_args!("Backtrace:\n"));
    
    for (i, frame) in Backtrace::new().enumerate() {
        crate::debug::write_fmt(format_args!(
            "  #{:2}: frame={:016X} return={:016X}\n",
            i,
            frame.frame_pointer.as_u64(),
            frame.return_address.as_u64()
        ));
    }
}

/// Print backtrace from specific frame pointer
pub fn print_backtrace_from(frame_pointer: VirtAddr) {
    crate::debug::write_fmt(format_args!("Backtrace from {:016X}:\n", frame_pointer.as_u64()));
    
    for (i, frame) in Backtrace::from_frame_pointer(frame_pointer).enumerate() {
        crate::debug::write_fmt(format_args!(
            "  #{:2}: frame={:016X} return={:016X}\n",
            i,
            frame.frame_pointer.as_u64(),
            frame.return_address.as_u64()
        ));
    }
}

/// Get symbol name for address (placeholder)
pub fn symbol_name(_addr: VirtAddr) -> Option<&'static str> {
    // In a real implementation, this would look up symbols in a symbol table
    // For now, we just return None
    None
}

/// Format backtrace as string
impl fmt::Display for Backtrace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Backtrace:")?;
        
        for (i, frame) in self.clone().enumerate() {
            if let Some(symbol) = symbol_name(frame.return_address) {
                writeln!(f, "  #{:2}: {:016X} <{}>", 
                        i, frame.return_address.as_u64(), symbol)?;
            } else {
                writeln!(f, "  #{:2}: {:016X}", 
                        i, frame.return_address.as_u64())?;
            }
        }
        
        Ok(())
    }
}

impl Clone for Backtrace {
    fn clone(&self) -> Self {
        Self {
            current_frame: self.current_frame,
            max_depth: self.max_depth,
            current_depth: 0, // Reset depth for cloned iterator
        }
    }
}
