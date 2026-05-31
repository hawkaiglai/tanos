# TanOS Kernel Entry Point
# x86_64 assembly bootstrap code

# Multiboot2 header — placed in .multiboot_header section so the linker
# script can position it at the very start of the LOAD segment.
# (Multiboot1 cannot load 64-bit ELFs; use GRUB with multiboot2.)
.section .multiboot_header, "a"
.align 8
multiboot2_header:
    .long 0xE85250D6                # Multiboot2 magic
    .long 0                         # Architecture (i386)
    .long multiboot2_header_end - multiboot2_header  # Header length
    .long -(0xE85250D6 + 0 + (multiboot2_header_end - multiboot2_header))  # Checksum

    # End tag (no optional tags — keep it minimal for now)
    .align 8
    .short 0                        # Type: end
    .short 0                        # Flags
    .long 8                         # Size
multiboot2_header_end:

.section .bss
.align 16
# Initial stack (16KB)
stack_bottom:
    .skip 16384
stack_top:

# Early page tables for long mode
.align 4096
early_pml4:
    .skip 4096
early_pdpt:
    .skip 4096
early_pd:
    .skip 4096

.section .data
.align 8
early_gdt:
    .quad 0                         # Null descriptor
    .quad 0x00A09A0000000000        # Code descriptor (64-bit)
    .quad 0x00A0920000000000        # Data descriptor (64-bit)
early_gdt_end:

early_gdt_desc:
    .word early_gdt_end - early_gdt - 1
    .quad early_gdt

# Saved multiboot state. EAX (magic) and EBX (info pointer) are stashed here
# at _start because the 32->64 transition (setup_page_tables) clobbers the
# registers; long_mode_start reloads them before calling kernel_main.
.align 8
mb_magic:
    .long 0
mb_info:
    .long 0

.section .text
.code32

# Entry point called by bootloader
.global _start
.type _start, @function
_start:
    # Disable interrupts
    cli

    # Save multiboot magic (EAX) and info pointer (EBX) to memory immediately.
    # setup_page_tables/enable_long_mode below clobber EAX/ECX/EDI, so the
    # registers cannot be relied on; long_mode_start reloads from these slots.
    movl %eax, mb_magic
    movl %ebx, mb_info

    # Verify multiboot2 magic
    cmp $0x36D76289, %eax
    jne multiboot_error
    
    # Setup stack
    mov $stack_top, %esp
    
    # Check for long mode support
    call check_long_mode
    test %eax, %eax
    jz long_mode_error
    
    # Setup early page tables
    call setup_page_tables
    
    # Enable long mode
    call enable_long_mode
    
    # Load 64-bit GDT
    lgdt early_gdt_desc
    
    # Far jump to 64-bit code
    ljmp $8, $long_mode_start

# Check if CPU supports long mode
check_long_mode:
    # Check for CPUID support
    pushf
    pop %eax
    mov %eax, %ecx
    xor $0x200000, %eax             # Flip ID bit
    push %eax
    popf
    pushf
    pop %eax
    xor %ecx, %eax
    jz no_long_mode                 # ID bit didn't flip
    
    # Check for extended functions
    mov $0x80000000, %eax
    cpuid
    cmp $0x80000001, %eax
    jb no_long_mode
    
    # Check for long mode
    mov $0x80000001, %eax
    cpuid
    test $0x20000000, %edx          # LM bit
    jz no_long_mode
    
    mov $1, %eax
    ret

no_long_mode:
    xor %eax, %eax
    ret

# Setup early page tables for identity mapping
setup_page_tables:
    # Clear page tables
    mov $early_pml4, %edi
    mov $0xC00, %ecx                # 3 pages * 4096 bytes / 4 = 3072 dwords
    xor %eax, %eax
    rep stosl
    
    # Setup PML4
    mov $early_pml4, %edi
    mov $early_pdpt, %eax
    or $3, %eax                     # Present | Writable
    mov %eax, (%edi)
    
    # Setup PDPT
    mov $early_pdpt, %edi
    mov $early_pd, %eax
    or $3, %eax                     # Present | Writable
    mov %eax, (%edi)
    
    # Setup PD (identity map first 2MB with 2MB pages)
    mov $early_pd, %edi
    mov $0x83, %eax                 # Present | Writable | Page Size
    mov $512, %ecx                  # 512 entries
1:
    mov %eax, (%edi)
    add $0x200000, %eax             # 2MB
    add $8, %edi
    loop 1b
    
    ret

# Enable long mode and paging
enable_long_mode:
    # Load page table
    mov $early_pml4, %eax
    mov %eax, %cr3
    
    # Enable PAE
    mov %cr4, %eax
    or $0x20, %eax                  # PAE bit
    mov %eax, %cr4
    
    # Enable long mode in EFER
    mov $0xC0000080, %ecx           # EFER MSR
    rdmsr
    or $0x100, %eax                 # LME bit
    wrmsr
    
    # Enable paging
    mov %cr0, %eax
    or $0x80000000, %eax            # PG bit
    mov %eax, %cr0
    
    ret

multiboot_error:
    mov $multiboot_error_msg, %esi
    call print_error
    jmp halt

long_mode_error:
    mov $long_mode_error_msg, %esi
    call print_error
    jmp halt

# Print error message and halt
print_error:
    mov $0xB8000, %edi              # VGA text buffer
    mov $0x4F, %ah                  # White on red
1:
    lodsb
    test %al, %al
    jz halt
    stosw
    jmp 1b

halt:
    cli
    hlt
    jmp halt

.section .rodata
multiboot_error_msg:
    .asciz "FATAL: Not booted by Multiboot2 bootloader"
long_mode_error_msg:
    .asciz "FATAL: Long mode not supported by CPU"

.code64
.section .text

# 64-bit entry point
long_mode_start:
    # Setup segment registers
    mov $16, %ax                    # Data segment
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss
    
    # Setup stack
    mov $stack_top, %rsp
    
    # Clear direction flag
    cld

    # Early serial output — write "OK" to COM1 (0x3F8) to prove we reached 64-bit mode
    mov $0x3F8, %dx
    mov $'O', %al
    out %al, %dx
    mov $'K', %al
    out %al, %dx
    mov $'\n', %al
    out %al, %dx

    # Prepare arguments for Rust kernel. The registers holding magic/info were
    # clobbered during the 32->64 transition, so reload them from the saved
    # slots. 32-bit loads into EDI/ESI zero-extend into RDI/RSI.
    #   RDI = multiboot magic, RSI = multiboot info pointer
    mov $mb_magic, %rax
    movl (%rax), %edi
    mov $mb_info, %rax
    movl (%rax), %esi

    # Call Rust kernel main
    call kernel_main
    
    # Kernel returned (should never happen)
    cli
    hlt
    jmp halt

# Interrupt handlers (will be replaced by IDT)
.global divide_error_asm
divide_error_asm:
    push $0                         # Error code
    push $0                         # Vector number
    jmp common_interrupt_handler

.global debug_asm
debug_asm:
    push $0
    push $1
    jmp common_interrupt_handler

.global nmi_asm
nmi_asm:
    push $0
    push $2
    jmp common_interrupt_handler

.global breakpoint_asm
breakpoint_asm:
    push $0
    push $3
    jmp common_interrupt_handler

.global overflow_asm
overflow_asm:
    push $0
    push $4
    jmp common_interrupt_handler

.global bound_range_asm
bound_range_asm:
    push $0
    push $5
    jmp common_interrupt_handler

.global invalid_opcode_asm
invalid_opcode_asm:
    push $0
    push $6
    jmp common_interrupt_handler

.global device_not_available_asm
device_not_available_asm:
    push $0
    push $7
    jmp common_interrupt_handler

.global double_fault_asm
double_fault_asm:
    # Error code already pushed
    push $8
    jmp common_interrupt_handler

.global invalid_tss_asm
invalid_tss_asm:
    # Error code already pushed
    push $10
    jmp common_interrupt_handler

.global segment_not_present_asm
segment_not_present_asm:
    # Error code already pushed
    push $11
    jmp common_interrupt_handler

.global stack_segment_fault_asm
stack_segment_fault_asm:
    # Error code already pushed
    push $12
    jmp common_interrupt_handler

.global general_protection_fault_asm
general_protection_fault_asm:
    # Error code already pushed
    push $13
    jmp common_interrupt_handler

.global page_fault_asm
page_fault_asm:
    # Error code already pushed
    push $14
    jmp common_interrupt_handler

.global x87_floating_point_asm
x87_floating_point_asm:
    push $0
    push $16
    jmp common_interrupt_handler

.global alignment_check_asm
alignment_check_asm:
    # Error code already pushed
    push $17
    jmp common_interrupt_handler

.global machine_check_asm
machine_check_asm:
    push $0
    push $18
    jmp common_interrupt_handler

.global simd_floating_point_asm
simd_floating_point_asm:
    push $0
    push $19
    jmp common_interrupt_handler

# IRQ handlers (vectors 32-47)
.global irq0_asm
irq0_asm:
    push $0
    push $32
    jmp common_interrupt_handler

.global irq1_asm
irq1_asm:
    push $0
    push $33
    jmp common_interrupt_handler

.global irq2_asm
irq2_asm:
    push $0
    push $34
    jmp common_interrupt_handler

.global irq3_asm
irq3_asm:
    push $0
    push $35
    jmp common_interrupt_handler

.global irq4_asm
irq4_asm:
    push $0
    push $36
    jmp common_interrupt_handler

.global irq5_asm
irq5_asm:
    push $0
    push $37
    jmp common_interrupt_handler

.global irq6_asm
irq6_asm:
    push $0
    push $38
    jmp common_interrupt_handler

.global irq7_asm
irq7_asm:
    push $0
    push $39
    jmp common_interrupt_handler

.global irq8_asm
irq8_asm:
    push $0
    push $40
    jmp common_interrupt_handler

.global irq9_asm
irq9_asm:
    push $0
    push $41
    jmp common_interrupt_handler

.global irq10_asm
irq10_asm:
    push $0
    push $42
    jmp common_interrupt_handler

.global irq11_asm
irq11_asm:
    push $0
    push $43
    jmp common_interrupt_handler

.global irq12_asm
irq12_asm:
    push $0
    push $44
    jmp common_interrupt_handler

.global irq13_asm
irq13_asm:
    push $0
    push $45
    jmp common_interrupt_handler

.global irq14_asm
irq14_asm:
    push $0
    push $46
    jmp common_interrupt_handler

.global irq15_asm
irq15_asm:
    push $0
    push $47
    jmp common_interrupt_handler

# Syscall entry via `int 0x80` (vector 128). Reuses the common interrupt path;
# interrupt_handler_wrapper dispatches vector 0x80 to the syscall handler and
# writes the return value back into the saved RAX before iretq.
.global syscall_int_asm
syscall_int_asm:
    push $0                          # dummy error code
    push $0x80                       # vector number (128)
    jmp common_interrupt_handler

# Common interrupt handler
common_interrupt_handler:
    # Save all registers
    push %rax
    push %rbx
    push %rcx
    push %rdx
    push %rsi
    push %rdi
    push %rbp
    push %r8
    push %r9
    push %r10
    push %r11
    push %r12
    push %r13
    push %r14
    push %r15
    
    # Call interrupt handler
    mov %rsp, %rdi                  # Pass stack frame
    call interrupt_handler_wrapper
    
    # Restore registers
    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %r11
    pop %r10
    pop %r9
    pop %r8
    pop %rbp
    pop %rdi
    pop %rsi
    pop %rdx
    pop %rcx
    pop %rbx
    pop %rax
    
    # Remove vector number and error code
    add $16, %rsp
    
    # Return from interrupt
    iretq

# System call entry point (SYSCALL instruction)
.global syscall_entry
syscall_entry:
    # SYSCALL instruction automatically:
    # - Saves RIP to RCX
    # - Saves RFLAGS to R11
    # - Loads RIP from LSTAR MSR
    # - Loads CS/SS from STAR MSR
    # - Masks RFLAGS with SFMASK MSR
    
    # Switch to kernel stack
    swapgs                          # Swap GS with kernel GS
    mov %rsp, %gs:0                 # Save user stack
    mov %gs:8, %rsp                 # Load kernel stack
    
    # Save user registers
    push %rcx                       # User RIP
    push %r11                       # User RFLAGS
    push %rax                       # Syscall number
    push %rdi                       # Arg 0
    push %rsi                       # Arg 1
    push %rdx                       # Arg 2
    push %r10                       # Arg 3 (note: R10, not RCX)
    push %r8                        # Arg 4
    push %r9                        # Arg 5

    # Call syscall handler
    # Arguments are already in correct registers
    call syscall_handler_wrapper

    # Restore user registers
    pop %r9
    pop %r8
    pop %r10
    pop %rdx
    pop %rsi
    pop %rdi
    add $8, %rsp                    # Skip syscall number
    pop %r11                        # User RFLAGS
    pop %rcx                        # User RIP
    
    # Switch back to user stack
    mov %gs:0, %rsp                 # Restore user stack
    swapgs                          # Restore user GS
    
    # Return to user mode
    sysretq

# Context switch function
.global context_switch
context_switch:
    # RDI = old context pointer
    # RSI = new context pointer
    
    # Save old context
    mov %rax, 0(%rdi)
    mov %rbx, 8(%rdi)
    mov %rcx, 16(%rdi)
    mov %rdx, 24(%rdi)
    mov %rsi, 32(%rdi)
    mov %r8, 64(%rdi)
    mov %r9, 72(%rdi)
    mov %r10, 80(%rdi)
    mov %r11, 88(%rdi)
    mov %r12, 96(%rdi)
    mov %r13, 104(%rdi)
    mov %r14, 112(%rdi)
    mov %r15, 120(%rdi)
    mov %rbp, 40(%rdi)
    mov %rsp, 48(%rdi)
    
    # Save RIP (return address)
    mov (%rsp), %rax
    mov %rax, 56(%rdi)
    
    # Save segment registers
    mov %cs, %ax
    mov %ax, 128(%rdi)
    mov %ds, %ax
    mov %ax, 130(%rdi)
    mov %es, %ax
    mov %ax, 132(%rdi)
    mov %fs, %ax
    mov %ax, 134(%rdi)
    mov %gs, %ax
    mov %ax, 136(%rdi)
    mov %ss, %ax
    mov %ax, 138(%rdi)
    
    # Save CR3 (page table)
    mov %cr3, %rax
    mov %rax, 144(%rdi)
    
    # Load new context
    mov 144(%rsi), %rax
    mov %rax, %cr3                  # Switch page table
    
    mov 0(%rsi), %rax
    mov 8(%rsi), %rbx
    mov 16(%rsi), %rcx
    mov 24(%rsi), %rdx
    mov 32(%rsi), %rdi              # Note: this overwrites old context pointer
    mov 64(%rsi), %r8
    mov 72(%rsi), %r9
    mov 80(%rsi), %r10
    mov 88(%rsi), %r11
    mov 96(%rsi), %r12
    mov 104(%rsi), %r13
    mov 112(%rsi), %r14
    mov 120(%rsi), %r15
    mov 40(%rsi), %rbp
    mov 48(%rsi), %rsp
    
    # Jump to new RIP
    push 56(%rsi)
    mov 32(%rsi), %rsi              # Restore RSI last
    ret

# Atomic operations
.global atomic_add
atomic_add:
    lock add %esi, (%rdi)
    ret

.global atomic_sub
atomic_sub:
    lock sub %esi, (%rdi)
    ret

.global atomic_inc
atomic_inc:
    lock incl (%rdi)
    ret

.global atomic_dec
atomic_dec:
    lock decl (%rdi)
    ret

.global atomic_cmpxchg
atomic_cmpxchg:
    # RDI = ptr, RSI = old, RDX = new
    mov %rsi, %rax
    lock cmpxchg %rdx, (%rdi)
    sete %al
    movzx %al, %rax
    ret

# Memory operations
.global memcpy_fast
memcpy_fast:
    # RDI = dest, RSI = src, RDX = count
    mov %rdx, %rcx
    rep movsb
    mov %rdi, %rax
    ret

.global memset_fast
memset_fast:
    # RDI = dest, RSI = value, RDX = count
    mov %rsi, %rax
    mov %rdx, %rcx
    rep stosb
    ret

# CPU feature detection
.global cpuid
cpuid:
    # RDI = leaf, RSI = output array
    push %rbx
    mov %rdi, %rax
    cpuid
    mov %eax, 0(%rsi)
    mov %ebx, 4(%rsi)
    mov %ecx, 8(%rsi)
    mov %edx, 12(%rsi)
    pop %rbx
    ret

# MSR operations
.global read_msr
read_msr:
    # RDI = MSR number
    mov %rdi, %rcx
    rdmsr
    shl $32, %rdx
    or %rdx, %rax
    ret

.global write_msr
write_msr:
    # RDI = MSR number, RSI = value
    mov %rdi, %rcx
    mov %rsi, %rax
    mov %rsi, %rdx
    shr $32, %rdx
    wrmsr
    ret

# End of file
