//! Interrupt handler stubs
//! The actual handlers are the assembly stubs in entry.S which call
//! interrupt_handler_wrapper() in mod.rs. This module just holds the
//! extern declarations for the assembly symbols.

extern "C" {
    pub fn divide_error_asm();
    pub fn debug_asm();
    pub fn nmi_asm();
    pub fn breakpoint_asm();
    pub fn overflow_asm();
    pub fn bound_range_asm();
    pub fn invalid_opcode_asm();
    pub fn device_not_available_asm();
    pub fn double_fault_asm();
    pub fn invalid_tss_asm();
    pub fn segment_not_present_asm();
    pub fn stack_segment_fault_asm();
    pub fn general_protection_fault_asm();
    pub fn page_fault_asm();
    pub fn x87_floating_point_asm();
    pub fn alignment_check_asm();
    pub fn machine_check_asm();
    pub fn simd_floating_point_asm();
    pub fn irq0_asm();
    pub fn irq1_asm();
    pub fn irq2_asm();
    pub fn irq3_asm();
    pub fn irq4_asm();
    pub fn irq5_asm();
    pub fn irq6_asm();
    pub fn irq7_asm();
    pub fn irq8_asm();
    pub fn irq9_asm();
    pub fn irq10_asm();
    pub fn irq11_asm();
    pub fn irq12_asm();
    pub fn irq13_asm();
    pub fn irq14_asm();
    pub fn irq15_asm();
    pub fn syscall_int_asm();
}
