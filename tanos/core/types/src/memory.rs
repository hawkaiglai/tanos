//! Memory management types and utilities

use core::fmt::{self, Display, Formatter};
use core::ops::{Add, AddAssign, Sub, SubAssign};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Physical memory address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PhysAddr(u64);

impl PhysAddr {
    /// Null physical address
    pub const NULL: Self = Self(0);
    
    /// Maximum valid physical address (52 bits on x86_64)
    pub const MAX: Self = Self((1u64 << 52) - 1);
    
    /// Create a new physical address
    pub const fn new(addr: u64) -> Option<Self> {
        if addr <= Self::MAX.0 {
            Some(Self(addr))
        } else {
            None
        }
    }
    
    /// Create a physical address without bounds checking
    pub const fn new_unchecked(addr: u64) -> Self {
        Self(addr)
    }
    
    /// Create a physical address from a pointer
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::new_unchecked(ptr as u64)
    }
    
    /// Get the raw address value
    pub const fn as_u64(self) -> u64 {
        self.0
    }
    
    /// Get the address as a pointer
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }
    
    /// Get the address as a mutable pointer
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }
    
    /// Check if the address is null
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
    
    /// Check if the address is aligned to the given alignment
    pub const fn is_aligned(self, align: u64) -> bool {
        self.0 % align == 0
    }
    
    /// Align the address down to the given alignment
    pub const fn align_down(self, align: u64) -> Self {
        Self(self.0 & !(align - 1))
    }
    
    /// Align the address up to the given alignment
    pub const fn align_up(self, align: u64) -> Self {
        Self((self.0 + align - 1) & !(align - 1))
    }
    
    /// Check if the address is page-aligned
    pub const fn is_page_aligned(self) -> bool {
        self.is_aligned(crate::PAGE_SIZE as u64)
    }
    
    /// Align to page boundary (down)
    pub const fn page_align_down(self) -> Self {
        self.align_down(crate::PAGE_SIZE as u64)
    }
    
    /// Align to page boundary (up)
    pub const fn page_align_up(self) -> Self {
        self.align_up(crate::PAGE_SIZE as u64)
    }
    
    /// Get the page number for this address
    pub const fn page_number(self) -> u64 {
        self.0 / crate::PAGE_SIZE as u64
    }
    
    /// Get the offset within the page
    pub const fn page_offset(self) -> u64 {
        self.0 % crate::PAGE_SIZE as u64
    }
}

impl Display for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "PhysAddr(0x{:016x})", self.0)
    }
}

impl Add<u64> for PhysAddr {
    type Output = Self;
    
    fn add(self, offset: u64) -> Self::Output {
        Self::new_unchecked(self.0.saturating_add(offset))
    }
}

impl AddAssign<u64> for PhysAddr {
    fn add_assign(&mut self, offset: u64) {
        *self = *self + offset;
    }
}

impl Sub<u64> for PhysAddr {
    type Output = Self;
    
    fn sub(self, offset: u64) -> Self::Output {
        Self::new_unchecked(self.0.saturating_sub(offset))
    }
}

impl SubAssign<u64> for PhysAddr {
    fn sub_assign(&mut self, offset: u64) {
        *self = *self - offset;
    }
}

impl Sub<PhysAddr> for PhysAddr {
    type Output = u64;
    
    fn sub(self, other: PhysAddr) -> Self::Output {
        self.0.saturating_sub(other.0)
    }
}

/// Virtual memory address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VirtAddr(u64);

impl VirtAddr {
    /// Null virtual address
    pub const NULL: Self = Self(0);
    
    /// User space start
    pub const USER_START: Self = Self(0x0000_0000_0000_1000);
    
    /// User space end
    pub const USER_END: Self = Self(0x0000_7FFF_FFFF_F000);
    
    /// Kernel space start
    pub const KERNEL_START: Self = Self(0xFFFF_8000_0000_0000);
    
    /// Kernel space end
    pub const KERNEL_END: Self = Self(0xFFFF_FFFF_FFFF_F000);
    
    /// Create a new virtual address
    pub const fn new(addr: u64) -> Option<Self> {
        // Check for canonical address on x86_64
        if addr < crate::CANONICAL_BOUNDARY || addr >= !crate::CANONICAL_BOUNDARY {
            Some(Self(addr))
        } else {
            None
        }
    }
    
    /// Create a virtual address without canonical address checking
    pub const fn new_unchecked(addr: u64) -> Self {
        Self(addr)
    }
    
    /// Create a virtual address from a pointer
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::new_unchecked(ptr as u64)
    }
    
    /// Get the raw address value
    pub const fn as_u64(self) -> u64 {
        self.0
    }
    
    /// Get the address as a pointer
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }
    
    /// Get the address as a mutable pointer
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }
    
    /// Check if the address is null
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
    
    /// Check if this is a canonical address
    pub const fn is_canonical(self) -> bool {
        self.0 < crate::CANONICAL_BOUNDARY || self.0 >= !crate::CANONICAL_BOUNDARY
    }
    
    /// Check if this is a user space address
    pub const fn is_user(self) -> bool {
        self.0 >= Self::USER_START.0 && self.0 < Self::USER_END.0
    }
    
    /// Check if this is a kernel space address
    pub const fn is_kernel(self) -> bool {
        self.0 >= Self::KERNEL_START.0 && self.0 < Self::KERNEL_END.0
    }
    
    /// Check if the address is aligned to the given alignment
    pub const fn is_aligned(self, align: u64) -> bool {
        self.0 % align == 0
    }
    
    /// Align the address down to the given alignment
    pub const fn align_down(self, align: u64) -> Self {
        Self(self.0 & !(align - 1))
    }
    
    /// Align the address up to the given alignment
    pub const fn align_up(self, align: u64) -> Self {
        Self((self.0 + align - 1) & !(align - 1))
    }
    
    /// Check if the address is page-aligned
    pub const fn is_page_aligned(self) -> bool {
        self.is_aligned(crate::PAGE_SIZE as u64)
    }
    
    /// Align to page boundary (down)
    pub const fn page_align_down(self) -> Self {
        self.align_down(crate::PAGE_SIZE as u64)
    }
    
    /// Align to page boundary (up)
    pub const fn page_align_up(self) -> Self {
        self.align_up(crate::PAGE_SIZE as u64)
    }
    
    /// Get the page number for this address
    pub const fn page_number(self) -> u64 {
        self.0 / crate::PAGE_SIZE as u64
    }
    
    /// Get the offset within the page
    pub const fn page_offset(self) -> u64 {
        self.0 % crate::PAGE_SIZE as u64
    }
    
    /// Get the page table index for the given level
    pub const fn page_table_index(self, level: usize) -> usize {
        ((self.0 >> (12 + 9 * level)) & 0x1FF) as usize
    }
}

impl Display for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "VirtAddr(0x{:016x})", self.0)
    }
}

impl Add<u64> for VirtAddr {
    type Output = Self;
    
    fn add(self, offset: u64) -> Self::Output {
        Self::new_unchecked(self.0.wrapping_add(offset))
    }
}

impl AddAssign<u64> for VirtAddr {
    fn add_assign(&mut self, offset: u64) {
        *self = *self + offset;
    }
}

impl Sub<u64> for VirtAddr {
    type Output = Self;
    
    fn sub(self, offset: u64) -> Self::Output {
        Self::new_unchecked(self.0.wrapping_sub(offset))
    }
}

impl SubAssign<u64> for VirtAddr {
    fn sub_assign(&mut self, offset: u64) {
        *self = *self - offset;
    }
}

impl Sub<VirtAddr> for VirtAddr {
    type Output = u64;
    
    fn sub(self, other: VirtAddr) -> Self::Output {
        self.0.wrapping_sub(other.0)
    }
}

/// Memory page representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Page {
    /// Virtual address of the page (page-aligned)
    address: VirtAddr,
}

impl Page {
    /// Create a new page from a virtual address
    pub const fn new(address: VirtAddr) -> Self {
        Self {
            address: address.page_align_down(),
        }
    }
    
    /// Create a page containing the given address
    pub const fn containing(address: VirtAddr) -> Self {
        Self::new(address)
    }
    
    /// Get the virtual address of this page
    pub const fn address(self) -> VirtAddr {
        self.address
    }
    
    /// Get the page number
    pub const fn number(self) -> u64 {
        self.address.page_number()
    }
    
    /// Get the next page
    pub const fn next(self) -> Self {
        Self::new(VirtAddr::new_unchecked(
            self.address.as_u64() + crate::PAGE_SIZE as u64,
        ))
    }
    
    /// Get a range of pages
    pub const fn range(start: VirtAddr, end: VirtAddr) -> PageRange {
        PageRange {
            start: Self::containing(start),
            end: Self::containing(end.align_up(crate::PAGE_SIZE as u64)),
        }
    }
}

/// Physical memory frame representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Frame {
    /// Physical address of the frame (page-aligned)
    address: PhysAddr,
}

impl Frame {
    /// Create a new frame from a physical address
    pub const fn new(address: PhysAddr) -> Self {
        Self {
            address: address.page_align_down(),
        }
    }
    
    /// Create a frame containing the given address
    pub const fn containing(address: PhysAddr) -> Self {
        Self::new(address)
    }
    
    /// Create a frame from a frame number
    pub const fn from_number(number: u64) -> Self {
        Self::new(PhysAddr::new_unchecked(number * crate::PAGE_SIZE as u64))
    }
    
    /// Get the physical address of this frame
    pub const fn address(self) -> PhysAddr {
        self.address
    }
    
    /// Get the frame number
    pub const fn number(self) -> u64 {
        self.address.page_number()
    }
    
    /// Get the next frame
    pub const fn next(self) -> Self {
        Self::new(PhysAddr::new_unchecked(
            self.address.as_u64() + crate::PAGE_SIZE as u64,
        ))
    }
}

/// Range of virtual pages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PageRange {
    /// Start page (inclusive)
    pub start: Page,
    /// End page (exclusive)
    pub end: Page,
}

impl PageRange {
    /// Create a new page range
    pub const fn new(start: Page, end: Page) -> Self {
        Self { start, end }
    }
    
    /// Check if the range is empty
    pub const fn is_empty(self) -> bool {
        self.start.address.as_u64() >= self.end.address.as_u64()
    }
    
    /// Get the number of pages in this range
    pub const fn count(self) -> u64 {
        if self.is_empty() {
            0
        } else {
            (self.end.address.as_u64() - self.start.address.as_u64()) / crate::PAGE_SIZE as u64
        }
    }
    
    /// Check if the range contains the given page
    pub const fn contains(self, page: Page) -> bool {
        page.address.as_u64() >= self.start.address.as_u64()
            && page.address.as_u64() < self.end.address.as_u64()
    }
    
    /// Check if this range overlaps with another
    pub const fn overlaps(self, other: PageRange) -> bool {
        self.start.address.as_u64() < other.end.address.as_u64()
            && other.start.address.as_u64() < self.end.address.as_u64()
    }
}

/// Memory region descriptor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MemoryRegion {
    /// Start physical address
    pub start: PhysAddr,
    /// Size in bytes
    pub size: u64,
    /// Memory region type
    pub region_type: MemoryRegionType,
}

impl MemoryRegion {
    /// Create a new memory region
    pub const fn new(start: PhysAddr, size: u64, region_type: MemoryRegionType) -> Self {
        Self {
            start,
            size,
            region_type,
        }
    }
    
    /// Get the end address (exclusive)
    pub const fn end(self) -> PhysAddr {
        PhysAddr::new_unchecked(self.start.as_u64() + self.size)
    }
    
    /// Check if this region contains the given address
    pub const fn contains(self, addr: PhysAddr) -> bool {
        addr.as_u64() >= self.start.as_u64() && addr.as_u64() < self.end().as_u64()
    }
    
    /// Check if this region overlaps with another
    pub const fn overlaps(self, other: MemoryRegion) -> bool {
        self.start.as_u64() < other.end().as_u64()
            && other.start.as_u64() < self.end().as_u64()
    }
    
    /// Check if this region is usable by the kernel
    pub const fn is_usable(self) -> bool {
        matches!(self.region_type, MemoryRegionType::Usable)
    }
}

/// Memory region type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MemoryRegionType {
    /// Usable RAM
    Usable = 1,
    /// Reserved by firmware
    Reserved = 2,
    /// ACPI reclaimable
    AcpiReclaimable = 3,
    /// ACPI NVS
    AcpiNvs = 4,
    /// Bad memory
    BadMemory = 5,
    /// Kernel code/data
    Kernel = 6,
    /// Boot information
    BootInfo = 7,
}

/// Shared memory identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SharedMemoryId(u32);

impl SharedMemoryId {
    /// Invalid shared memory ID
    pub const INVALID: Self = Self(0);
    
    /// Create a new shared memory ID
    pub const fn new(id: u32) -> Option<Self> {
        if id == 0 {
            None
        } else {
            Some(Self(id))
        }
    }
    
    /// Create a shared memory ID without validation
    pub const fn new_unchecked(id: u32) -> Self {
        Self(id)
    }
    
    /// Get the raw ID value
    pub const fn as_u32(self) -> u32 {
        self.0
    }
    
    /// Check if this is a valid shared memory ID
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

impl Display for SharedMemoryId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(f, "SharedMem({})", self.0)
        } else {
            write!(f, "SharedMem(INVALID)")
        }
    }
}
