/* TanOS userspace linker script.
 *
 * Links userspace binaries at 1GB (0x40000000). User space must begin at or
 * above 1GB because every address space identity-maps the first 1GB as a
 * supervisor (kernel-only) region — see KERNEL_IDENTITY_SIZE in the kernel's
 * memory/page.rs. Each section is page-aligned so the resulting PT_LOAD
 * segments fall on clean 4KB boundaries for the kernel ELF loader.
 */

ENTRY(_start)

SECTIONS {
    . = 0x40000000;

    .text : ALIGN(4K) {
        *(.text._start)
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K) {
        *(.data .data.*)
    }

    .bss : ALIGN(4K) {
        *(.bss .bss.*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note .note.*)
        *(.comment)
    }
}
