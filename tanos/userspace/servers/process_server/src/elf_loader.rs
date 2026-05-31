use kernel_types::VirtAddr;
use core::mem::size_of;
use crate::lib_extensions::{self, Error, Result, memory};

pub struct ElfLoader {
    memory_usage: usize,
}

impl ElfLoader {
    pub fn new() -> Self {
        Self { memory_usage: 0 }
    }

    pub fn memory_usage(&self) -> usize {
        self.memory_usage
    }

    pub fn load(&mut self, elf_data: &[u8], address_space: u64) -> Result<VirtAddr> {
        // Parse ELF header
        let header = self.parse_elf_header(elf_data)?;

        // Verify ELF is valid for our architecture
        self.verify_elf(&header)?;

        // Load program segments
        let entry_point = self.load_segments(elf_data, &header, address_space)?;

        Ok(entry_point)
    }

    fn parse_elf_header(&self, data: &[u8]) -> Result<ElfHeader> {
        if data.len() < size_of::<ElfHeader>() {
            return Err(Error::InvalidElf);
        }

        let header = unsafe {
            *(data.as_ptr() as *const ElfHeader)
        };

        // Check magic number
        if &header.e_ident[0..4] != b"\x7fELF" {
            return Err(Error::InvalidElf);
        }

        Ok(header)
    }

    fn verify_elf(&self, header: &ElfHeader) -> Result<()> {
        // Check class (64-bit)
        if header.e_ident[4] != 2 {
            return Err(Error::UnsupportedElf);
        }

        // Check endianness (little endian)
        if header.e_ident[5] != 1 {
            return Err(Error::UnsupportedElf);
        }

        // Check machine type (x86_64)
        if header.e_machine != 0x3E {
            return Err(Error::UnsupportedElf);
        }

        // Check type (executable)
        if header.e_type != 2 {
            return Err(Error::UnsupportedElf);
        }

        Ok(())
    }

    fn load_segments(&mut self, data: &[u8], header: &ElfHeader, address_space: u64) -> Result<VirtAddr> {
        let ph_offset = header.e_phoff as usize;
        let ph_size = header.e_phentsize as usize;
        let ph_count = header.e_phnum as usize;

        for i in 0..ph_count {
            let ph_data = &data[ph_offset + i * ph_size..ph_offset + (i + 1) * ph_size];
            let ph = unsafe { *(ph_data.as_ptr() as *const ProgramHeader) };

            if ph.p_type == 1 { // PT_LOAD
                self.load_program_segment(data, &ph, address_space)?;
            }
        }

        Ok(VirtAddr::new_unchecked(header.e_entry))
    }

    fn load_program_segment(&mut self, data: &[u8], ph: &ProgramHeader, _address_space: u64) -> Result<()> {
        let vaddr = VirtAddr::new_unchecked(ph.p_vaddr);
        let file_size = ph.p_filesz as usize;
        let mem_size = ph.p_memsz as usize;
        let offset = ph.p_offset as usize;

        // Calculate number of pages needed
        let start_page = vaddr.page_align_down();
        let end_page = VirtAddr::new_unchecked(ph.p_vaddr + ph.p_memsz).page_align_up();
        let page_count = (end_page.as_u64() - start_page.as_u64()) / PAGE_SIZE;

        // Allocate and map pages
        let mut flags: u64 = 0x01; // USER_ACCESSIBLE
        if ph.p_flags & 1 != 0 { flags |= 0x04; } // EXECUTABLE
        if ph.p_flags & 2 != 0 { flags |= 0x02; } // WRITABLE

        for i in 0..page_count {
            let page_vaddr = VirtAddr::new_unchecked(start_page.as_u64() + i * PAGE_SIZE);
            let frame = memory::allocate_frame()?;

            memory::map_page(page_vaddr, 4096, flags)?;
        }

        // Copy segment data
        if file_size > 0 {
            let segment_data = &data[offset..offset + file_size];
            memory::copy_to_user(vaddr, segment_data)?;
        }

        // Zero out BSS section
        if mem_size > file_size {
            let bss_start = VirtAddr::new_unchecked(ph.p_vaddr + file_size as u64);
            let bss_size = mem_size - file_size;
            memory::zero_user_memory(bss_start, bss_size)?;
        }

        self.memory_usage += mem_size;
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ElfHeader {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ProgramHeader {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

const PAGE_SIZE: u64 = 4096;
