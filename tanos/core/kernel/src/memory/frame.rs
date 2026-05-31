extern crate alloc;

use super::*;
use crate::boot::{MemoryRegion, MemoryRegionType};
use crate::*;

use alloc::vec::Vec;

pub struct FrameAllocator {
    bitmap: Bitmap,
    next_free: usize,
    free_frames: usize,
    total_frames: usize,
}

struct Bitmap {
    data: Vec<u64>,
    size: usize,
}

impl Bitmap {
    fn new(size: usize) -> Self {
        let word_count = (size + 63) / 64;
        Self {
            data: alloc::vec![0; word_count],
            size,
        }
    }
    
    fn set(&mut self, index: usize, value: bool) {
        if index >= self.size {
            return;
        }
        
        let word_index = index / 64;
        let bit_index = index % 64;
        
        if value {
            self.data[word_index] |= 1 << bit_index;
        } else {
            self.data[word_index] &= !(1 << bit_index);
        }
    }
    
    fn get(&self, index: usize) -> bool {
        if index >= self.size {
            return true; // Out of bounds = allocated
        }
        
        let word_index = index / 64;
        let bit_index = index % 64;
        
        (self.data[word_index] & (1 << bit_index)) != 0
    }
    
    fn find_free_range(&self, start: usize, count: usize) -> Option<usize> {
        for i in start..=(self.size.saturating_sub(count)) {
            let mut found = true;
            for j in 0..count {
                if self.get(i + j) {
                    found = false;
                    break;
                }
            }
            if found {
                return Some(i);
            }
        }
        None
    }
}

impl FrameAllocator {
    pub fn new(memory_map: &[MemoryRegion]) -> core::result::Result<Self, MemoryError> {
        // Calculate total memory size
        let mut max_addr = 0u64;
        for region in memory_map {
            if region.end.as_u64() > max_addr {
                max_addr = region.end.as_u64();
            }
        }
        
        let total_frames = (max_addr / crate::PAGE_SIZE as u64) as usize;
        let mut bitmap = Bitmap::new(total_frames);
        
        // Mark all frames as allocated initially
        for i in 0..total_frames {
            bitmap.set(i, true);
        }
        
        let mut free_frames = 0;
        
        // Mark available regions as free
        for region in memory_map {
            if region.region_type == MemoryRegionType::Available {
                let start_frame = (region.start.as_u64() / crate::PAGE_SIZE as u64) as usize;
                let end_frame = (region.end.as_u64() / crate::PAGE_SIZE as u64) as usize;
                
                for frame_index in start_frame..end_frame {
                    if frame_index < total_frames {
                        bitmap.set(frame_index, false);
                        free_frames += 1;
                    }
                }
            }
        }
        
        Ok(Self {
            bitmap,
            next_free: 0,
            free_frames,
            total_frames,
        })
    }
    
    pub fn allocate(&mut self) -> Option<Frame> {
        if self.free_frames == 0 {
            return None;
        }
        
        // Search from next_free hint
        for i in self.next_free..self.total_frames {
            if !self.bitmap.get(i) {
                self.bitmap.set(i, true);
                self.free_frames -= 1;
                self.next_free = i + 1;
                return Some(Frame::from_index(i));
            }
        }
        
        // Wrap around search
        for i in 0..self.next_free {
            if !self.bitmap.get(i) {
                self.bitmap.set(i, true);
                self.free_frames -= 1;
                self.next_free = i + 1;
                return Some(Frame::from_index(i));
            }
        }
        
        None
    }
    
    pub fn deallocate(&mut self, frame: Frame) {
        let index = frame.as_index();
        if index < self.total_frames && self.bitmap.get(index) {
            self.bitmap.set(index, false);
            self.free_frames += 1;
            if index < self.next_free {
                self.next_free = index;
            }
        }
    }
    
    pub fn allocate_contiguous_frames(&mut self, count: usize) -> core::result::Result<Vec<Frame>, MemoryError> {
        if count == 0 {
            return Ok(Vec::new());
        }
        
        if self.free_frames < count {
            return Err(MemoryError::OutOfMemory);
        }
        
        // Find contiguous free range
        if let Some(start) = self.bitmap.find_free_range(0, count) {
            let mut frames = Vec::with_capacity(count);
            
            // Allocate the range
            for i in 0..count {
                let index = start + i;
                self.bitmap.set(index, true);
                frames.push(Frame::from_index(index));
            }
            
            self.free_frames -= count;
            self.next_free = start + count;
            
            Ok(frames)
        } else {
            Err(MemoryError::OutOfMemory)
        }
    }
    
    pub fn reserve_range(&mut self, start: PhysAddr, end: PhysAddr) -> core::result::Result<(), MemoryError> {
        let start_frame = (start.as_u64() / crate::PAGE_SIZE as u64) as usize;
        let end_frame = (end.as_u64() / crate::PAGE_SIZE as u64) as usize;
        
        for frame_index in start_frame..=end_frame {
            if frame_index < self.total_frames && !self.bitmap.get(frame_index) {
                self.bitmap.set(frame_index, true);
                self.free_frames = self.free_frames.saturating_sub(1);
            }
        }
        
        Ok(())
    }
    
    pub fn free_frames(&self) -> usize {
        self.free_frames
    }
    
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    index: usize,
}

impl Frame {
    pub fn from_index(index: usize) -> Self {
        Self { index }
    }
    
    pub fn from_address(addr: PhysAddr) -> Self {
        Self {
            index: (addr.as_u64() / crate::PAGE_SIZE as u64) as usize,
        }
    }
    
    pub fn as_index(self) -> usize {
        self.index
    }
    
    pub fn start_address(self) -> PhysAddr {
        PhysAddr::new_unchecked((self.index * crate::PAGE_SIZE) as u64)
    }
    
    pub fn end_address(self) -> PhysAddr {
        PhysAddr::new_unchecked(((self.index + 1) * crate::PAGE_SIZE) as u64)
    }
}
