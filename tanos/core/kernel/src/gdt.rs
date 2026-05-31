//! Global Descriptor Table + Task State Segment.
//!
//! The early GDT in entry.S only has null/kernel-code/kernel-data, which is
//! enough to run the kernel in ring 0 but cannot enter ring 3. This module
//! builds the full GDT — kernel code/data, user code/data, and a TSS — and
//! loads it. The TSS's RSP0 supplies the kernel stack the CPU switches to when
//! a ring-3 thread enters the kernel (via the int 0x80 syscall gate or an
//! interrupt), which is mandatory for ring-3 execution.
//!
//! Selector layout (matches process::context segment constants):
//!   0x08 kernel code, 0x10 kernel data, 0x18 user code, 0x20 user data,
//!   0x28 TSS.

use spin::Once;
use x86_64::VirtAddr;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::instructions::tables::load_tss;
use x86_64::instructions::segmentation::{Segment, CS, DS, ES, SS};

/// Dedicated stack the CPU switches to (via TSS.RSP0) on a ring-3 → ring-0
/// transition. Kept separate from the boot stack so a user-mode entry lands on
/// clean kernel stack space.
const RSP0_STACK_SIZE: usize = 32 * 1024; // 32KB
static mut RSP0_STACK: [u8; RSP0_STACK_SIZE] = [0; RSP0_STACK_SIZE];

static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<GlobalDescriptorTable> = Once::new();
static SELECTORS: Once<Selectors> = Once::new();

#[derive(Debug, Clone, Copy)]
pub struct Selectors {
    pub kernel_code: SegmentSelector,
    pub kernel_data: SegmentSelector,
    pub user_code: SegmentSelector,
    pub user_data: SegmentSelector,
    pub tss: SegmentSelector,
}

/// Build and load the GDT + TSS, and reload the segment registers. Must be
/// called once, before any ring-3 entry, on the CPU that will use it.
pub fn init() {
    let tss = TSS.call_once(|| {
        let mut tss = TaskStateSegment::new();
        // RSP0: top of the dedicated kernel stack (stacks grow down).
        let stack_top = unsafe {
            let base = core::ptr::addr_of!(RSP0_STACK) as u64;
            VirtAddr::new(base + RSP0_STACK_SIZE as u64)
        };
        tss.privilege_stack_table[0] = stack_top;
        tss
    });

    let mut gdt = GlobalDescriptorTable::new();
    let kernel_code = gdt.append(Descriptor::kernel_code_segment());
    let kernel_data = gdt.append(Descriptor::kernel_data_segment());
    let user_code = gdt.append(Descriptor::user_code_segment());
    let user_data = gdt.append(Descriptor::user_data_segment());
    let tss_sel = gdt.append(Descriptor::tss_segment(tss));

    let gdt: &'static GlobalDescriptorTable = GDT.call_once(|| gdt);
    gdt.load();

    unsafe {
        CS::set_reg(kernel_code);
        SS::set_reg(kernel_data);
        DS::set_reg(kernel_data);
        ES::set_reg(kernel_data);
        load_tss(tss_sel);
    }

    SELECTORS.call_once(|| Selectors {
        kernel_code,
        kernel_data,
        user_code,
        user_data,
        tss: tss_sel,
    });

    crate::info!(
        "GDT/TSS loaded: kernel_code={:#x} kernel_data={:#x} user_code={:#x} user_data={:#x} tss={:#x}",
        kernel_code.0, kernel_data.0, user_code.0, user_data.0, tss_sel.0
    );
}

/// The loaded segment selectors (available after `init`).
pub fn selectors() -> &'static Selectors {
    SELECTORS.get().expect("GDT not initialized")
}
