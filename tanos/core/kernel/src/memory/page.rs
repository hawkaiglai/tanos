use super::*;
use crate::*;
use x86_64::structures::paging::PageTableFlags;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    number: usize,
}

impl Page {
    pub fn from_address(addr: VirtAddr) -> Self {
        Self {
            number: (addr.as_u64() / crate::PAGE_SIZE as u64) as usize,
        }
    }
    
    pub fn from_number(number: usize) -> Self {
        Self { number }
    }
    
    pub fn start_address(self) -> VirtAddr {
        VirtAddr::new_unchecked((self.number * crate::PAGE_SIZE) as u64)
    }
    
    pub fn end_address(self) -> VirtAddr {
        VirtAddr::new_unchecked(((self.number + 1) * crate::PAGE_SIZE) as u64)
    }
    
    pub fn as_number(self) -> usize {
        self.number
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFlags: u64 {
        const PRESENT = 1 << 0;
        const WRITABLE = 1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const NO_CACHE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const HUGE_PAGE = 1 << 7;
        const GLOBAL = 1 << 8;
        const NO_EXECUTE = 1 << 63;
    }
}

impl From<PageFlags> for PageTableFlags {
    fn from(flags: PageFlags) -> Self {
        let mut ptf = PageTableFlags::empty();
        
        if flags.contains(PageFlags::PRESENT) {
            ptf |= PageTableFlags::PRESENT;
        }
        if flags.contains(PageFlags::WRITABLE) {
            ptf |= PageTableFlags::WRITABLE;
        }
        if flags.contains(PageFlags::USER_ACCESSIBLE) {
            ptf |= PageTableFlags::USER_ACCESSIBLE;
        }
        if flags.contains(PageFlags::WRITE_THROUGH) {
            ptf |= PageTableFlags::WRITE_THROUGH;
        }
        if flags.contains(PageFlags::NO_CACHE) {
            ptf |= PageTableFlags::NO_CACHE;
        }
        if flags.contains(PageFlags::ACCESSED) {
            ptf |= PageTableFlags::ACCESSED;
        }
        if flags.contains(PageFlags::DIRTY) {
            ptf |= PageTableFlags::DIRTY;
        }
        if flags.contains(PageFlags::HUGE_PAGE) {
            ptf |= PageTableFlags::HUGE_PAGE;
        }
        if flags.contains(PageFlags::GLOBAL) {
            ptf |= PageTableFlags::GLOBAL;
        }
        if flags.contains(PageFlags::NO_EXECUTE) {
            ptf |= PageTableFlags::NO_EXECUTE;
        }
        
        ptf
    }
}

// Raw x86_64 page-table entry bits. PageFlags is defined with the same bit
// layout, so `PageFlags::bits()` can be OR'd straight into a leaf PTE.
const PTE_PRESENT: u64 = 1 << 0;
const PTE_WRITABLE: u64 = 1 << 1;
const PTE_USER: u64 = 1 << 2;
const PTE_HUGE: u64 = 1 << 7;
const PTE_ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

/// Low physical memory (bytes) identity-mapped as supervisor into EVERY
/// address space, using 2MB huge pages (one full PD = 512 entries = 1 GB,
/// mirroring the early boot page tables). This keeps the kernel — code, data,
/// bss, heap, boot stack, IDT — and ALL physical RAM (machines here have
/// ≤512MB) reachable under any CR3, so the kernel can still walk/allocate page
/// tables for arbitrary frames while a user address space is active.
///
/// User space must therefore begin at or above this boundary (≥ 1 GB virtual)
/// so it does not collide with the kernel's huge-page identity region.
pub const KERNEL_IDENTITY_SIZE: u64 = 0x4000_0000; // 1 GB

/// Interpret an identity-mapped physical frame address as a mutable array of
/// 512 page-table entries. Sound only because all RAM we touch lives in the
/// low region identity-mapped by the early boot page tables (< 1 GB).
#[inline]
unsafe fn table_at(phys: u64) -> &'static mut [u64; 512] {
    &mut *(phys as *mut [u64; 512])
}

/// A user/kernel virtual address space backed by a real 4-level page table.
pub struct AddressSpace {
    /// Physical address of the PML4 (the value loaded into CR3).
    cr3: PhysAddr,
}

impl AddressSpace {
    /// Create a fresh address space: a zeroed PML4 with the kernel's low
    /// identity region mapped so the kernel survives a switch into it.
    pub fn new() -> core::result::Result<Self, MemoryError> {
        let pml4 = Self::alloc_table()?;
        let mut space = Self { cr3: PhysAddr::new_unchecked(pml4) };
        space.map_kernel_identity()?;
        Ok(space)
    }

    pub fn kernel_space() -> core::result::Result<Self, MemoryError> {
        Self::new()
    }

    pub fn user_space() -> core::result::Result<Self, MemoryError> {
        Self::new()
    }

    /// Allocate a page-table frame and zero it, returning its physical address.
    fn alloc_table() -> core::result::Result<u64, MemoryError> {
        let frame = super::with_memory_manager(|mm| mm.allocate_frame())
            .ok_or(MemoryError::OutOfMemory)?;
        let phys = frame.start_address().as_u64();
        unsafe { core::ptr::write_bytes(phys as *mut u8, 0, crate::PAGE_SIZE); }
        Ok(phys)
    }

    /// Map `[0, KERNEL_IDENTITY_SIZE)` identity, supervisor, with 2MB pages.
    fn map_kernel_identity(&mut self) -> core::result::Result<(), MemoryError> {
        let mut phys = 0u64;
        while phys < KERNEL_IDENTITY_SIZE {
            self.map_huge_2mb(phys, phys, PTE_PRESENT | PTE_WRITABLE)?;
            phys += 0x20_0000;
        }
        Ok(())
    }

    /// Ensure the next-level table referenced by `table[index]` exists and
    /// return its physical address. Intermediate entries are permissive
    /// (PRESENT|WRITABLE|USER); the leaf entry decides the real access rights.
    unsafe fn next_table(table: &mut [u64; 512], index: usize)
        -> core::result::Result<u64, MemoryError>
    {
        if table[index] & PTE_PRESENT == 0 {
            let next = Self::alloc_table()?;
            table[index] = next | PTE_PRESENT | PTE_WRITABLE | PTE_USER;
            Ok(next)
        } else {
            Ok(table[index] & PTE_ADDR_MASK)
        }
    }

    /// Map a 2MB huge page at PD level.
    fn map_huge_2mb(&mut self, vaddr: u64, paddr: u64, flags: u64)
        -> core::result::Result<(), MemoryError>
    {
        let pml4_i = ((vaddr >> 39) & 0x1FF) as usize;
        let pdpt_i = ((vaddr >> 30) & 0x1FF) as usize;
        let pd_i   = ((vaddr >> 21) & 0x1FF) as usize;
        unsafe {
            let pml4 = table_at(self.cr3.as_u64());
            let pdpt = table_at(Self::next_table(pml4, pml4_i)?);
            let pd   = table_at(Self::next_table(pdpt, pdpt_i)?);
            pd[pd_i] = (paddr & !0x1F_FFFF) | flags | PTE_HUGE;
        }
        Ok(())
    }

    /// Map a single 4KB `page` to `frame` with `flags` (which must include
    /// PRESENT). Allocates intermediate tables as needed.
    pub fn map_page(&mut self, page: Page, frame: Frame, flags: PageFlags)
        -> core::result::Result<(), MemoryError>
    {
        let vaddr = page.start_address().as_u64();
        let paddr = frame.start_address().as_u64();
        let pml4_i = ((vaddr >> 39) & 0x1FF) as usize;
        let pdpt_i = ((vaddr >> 30) & 0x1FF) as usize;
        let pd_i   = ((vaddr >> 21) & 0x1FF) as usize;
        let pt_i   = ((vaddr >> 12) & 0x1FF) as usize;
        unsafe {
            let pml4 = table_at(self.cr3.as_u64());
            let pdpt = table_at(Self::next_table(pml4, pml4_i)?);
            let pd   = table_at(Self::next_table(pdpt, pdpt_i)?);
            if pd[pd_i] & PTE_HUGE != 0 {
                // Region is covered by a kernel huge page — refuse to shadow it.
                return Err(MemoryError::AlreadyMapped);
            }
            let pt = table_at(Self::next_table(pd, pd_i)?);
            if pt[pt_i] & PTE_PRESENT != 0 {
                return Err(MemoryError::AlreadyMapped);
            }
            pt[pt_i] = (paddr & PTE_ADDR_MASK) | flags.bits();
        }
        Ok(())
    }

    pub fn unmap_page(&mut self, page: Page) -> core::result::Result<Frame, MemoryError> {
        let vaddr = page.start_address().as_u64();
        let pml4_i = ((vaddr >> 39) & 0x1FF) as usize;
        let pdpt_i = ((vaddr >> 30) & 0x1FF) as usize;
        let pd_i   = ((vaddr >> 21) & 0x1FF) as usize;
        let pt_i   = ((vaddr >> 12) & 0x1FF) as usize;
        unsafe {
            let pml4 = table_at(self.cr3.as_u64());
            if pml4[pml4_i] & PTE_PRESENT == 0 { return Err(MemoryError::AddressNotMapped); }
            let pdpt = table_at(pml4[pml4_i] & PTE_ADDR_MASK);
            if pdpt[pdpt_i] & PTE_PRESENT == 0 { return Err(MemoryError::AddressNotMapped); }
            let pd = table_at(pdpt[pdpt_i] & PTE_ADDR_MASK);
            if pd[pd_i] & PTE_PRESENT == 0 { return Err(MemoryError::AddressNotMapped); }
            let pt = table_at(pd[pd_i] & PTE_ADDR_MASK);
            if pt[pt_i] & PTE_PRESENT == 0 { return Err(MemoryError::AddressNotMapped); }
            let frame = Frame::from_address(PhysAddr::new_unchecked(pt[pt_i] & PTE_ADDR_MASK));
            pt[pt_i] = 0;
            x86_64::instructions::tlb::flush(x86_64::VirtAddr::new(vaddr));
            Ok(frame)
        }
    }

    pub fn translate(&self, page: Page) -> Option<Frame> {
        let vaddr = page.start_address().as_u64();
        let pml4_i = ((vaddr >> 39) & 0x1FF) as usize;
        let pdpt_i = ((vaddr >> 30) & 0x1FF) as usize;
        let pd_i   = ((vaddr >> 21) & 0x1FF) as usize;
        let pt_i   = ((vaddr >> 12) & 0x1FF) as usize;
        unsafe {
            let pml4 = table_at(self.cr3.as_u64());
            if pml4[pml4_i] & PTE_PRESENT == 0 { return None; }
            let pdpt = table_at(pml4[pml4_i] & PTE_ADDR_MASK);
            if pdpt[pdpt_i] & PTE_PRESENT == 0 { return None; }
            let pd = table_at(pdpt[pdpt_i] & PTE_ADDR_MASK);
            if pd[pd_i] & PTE_PRESENT == 0 { return None; }
            if pd[pd_i] & PTE_HUGE != 0 {
                let base = pd[pd_i] & PTE_ADDR_MASK;
                return Some(Frame::from_address(PhysAddr::new_unchecked(base + (vaddr & 0x1F_FFFF))));
            }
            let pt = table_at(pd[pd_i] & PTE_ADDR_MASK);
            if pt[pt_i] & PTE_PRESENT == 0 { return None; }
            Some(Frame::from_address(PhysAddr::new_unchecked(pt[pt_i] & PTE_ADDR_MASK)))
        }
    }

    pub fn cr3(&self) -> PhysAddr {
        self.cr3
    }

    /// Load this address space's PML4 into CR3.
    pub fn activate(&self) {
        unsafe {
            core::arch::asm!(
                "mov cr3, {}",
                in(reg) self.cr3.as_u64(),
                options(nostack, preserves_flags)
            );
        }
    }
}

impl AddressSpace {
    /// Free a single frame given its physical base address.
    fn free_frame(mm: &mut super::MemoryManager, phys: u64) {
        mm.deallocate_frame(Frame::from_address(PhysAddr::new_unchecked(phys)));
    }

    /// Free a PT (4KB-leaf level): every present entry is a mapped user page
    /// (an allocator frame), then the PT frame itself.
    ///
    /// SAFETY: `phys` must be a page-table frame reachable via the identity map.
    unsafe fn free_pt(mm: &mut super::MemoryManager, phys: u64) {
        let pt = table_at(phys);
        for &e in pt.iter() {
            if e & PTE_PRESENT != 0 {
                Self::free_frame(mm, e & PTE_ADDR_MASK);
            }
        }
        Self::free_frame(mm, phys);
    }

    /// Free a PD: 2MB huge-page entries are skipped (they alias existing RAM —
    /// the kernel identity region — not allocator frames); non-huge entries are
    /// PTs to recurse into. Then the PD frame itself.
    ///
    /// SAFETY: `phys` must be a page-table frame reachable via the identity map.
    unsafe fn free_pd(mm: &mut super::MemoryManager, phys: u64) {
        let pd = table_at(phys);
        for &e in pd.iter() {
            if e & PTE_PRESENT == 0 || e & PTE_HUGE != 0 {
                continue;
            }
            Self::free_pt(mm, e & PTE_ADDR_MASK);
        }
        Self::free_frame(mm, phys);
    }

    /// Free a PDPT: every present entry is a PD (we never map 1GB huge pages),
    /// then the PDPT frame itself.
    ///
    /// SAFETY: `phys` must be a page-table frame reachable via the identity map.
    unsafe fn free_pdpt(mm: &mut super::MemoryManager, phys: u64) {
        let pdpt = table_at(phys);
        for &e in pdpt.iter() {
            if e & PTE_PRESENT != 0 {
                Self::free_pd(mm, e & PTE_ADDR_MASK);
            }
        }
        Self::free_frame(mm, phys);
    }
}

impl Drop for AddressSpace {
    fn drop(&mut self) {
        // Recursively reclaim every frame this address space owns: all mapped
        // user pages and ALL four levels of page-table frames (PML4/PDPT/PD/PT),
        // including the private tables backing the kernel identity region.
        //
        // The identity region is mapped with 2MB huge pages that alias existing
        // physical RAM (the kernel image, heap, all of low memory) rather than
        // allocator frames, so their *targets* are skipped (see free_pd) — only
        // the table frames mapping them are freed. The frame allocator's
        // deallocate() also checks its bitmap first, so any frame that is
        // somehow reached twice (e.g. an aliased mapping) is freed at most once.
        //
        // SAFETY: this runs while `self` is NOT the active CR3 (the death paths
        // in process::mod switch CR3 immediately after dropping the process), so
        // reclaiming its frames cannot pull tables out from under a live walk.
        // All table frames are in the identity-mapped low region, so table_at is
        // valid regardless of which address space is currently active.
        super::with_memory_manager(|mm| unsafe {
            let pml4 = table_at(self.cr3.as_u64());
            for &e in pml4.iter() {
                if e & PTE_PRESENT != 0 {
                    Self::free_pdpt(mm, e & PTE_ADDR_MASK);
                }
            }
            Self::free_frame(mm, self.cr3.as_u64());
        });
    }
}
