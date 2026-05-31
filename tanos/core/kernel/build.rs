use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();

    println!("cargo:rerun-if-changed=asm/");
    println!("cargo:rerun-if-changed=linker.ld");

    // Generate linker script based on target architecture
    let linker_script = if target.contains("x86_64") {
        generate_x86_64_linker_script()
    } else if target.contains("riscv") {
        generate_riscv_linker_script()
    } else {
        panic!("Unsupported target architecture: {}", target);
    };

    let linker_script_path = out_dir.join("linker.ld");
    fs::write(&linker_script_path, linker_script).unwrap();

    println!("cargo:rustc-link-arg=-T{}", linker_script_path.display());
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-static");
    println!("cargo:rustc-link-arg=-no-pie");
    println!("cargo:rustc-link-arg=--gc-sections");

    // Assemble architecture-specific files
    if target.contains("x86_64") {
        // entry.S is GAS syntax — use cc to assemble it
        cc::Build::new()
            .file("asm/entry.S")
            .flag("-c")
            .flag("-nostdlib")
            .flag("-ffreestanding")
            .flag("-no-pie")
            .compile("kernel_asm");
    }
}

fn generate_x86_64_linker_script() -> String {
    r#"
ENTRY(_start)

MEMORY
{
    RAM : ORIGIN = 0x100000, LENGTH = 128M
}

SECTIONS
{
    . = 0x100000;

    .text ALIGN(4K) : {
        KEEP(*(.multiboot_header))
        KEEP(*(.multiboot))
        *(.text .text.*)
    } > RAM

    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
    } > RAM

    .data ALIGN(4K) : {
        *(.data .data.*)
    } > RAM

    .got ALIGN(4K) : {
        *(.got .got.*)
    } > RAM

    .bss ALIGN(4K) : {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } > RAM

    /DISCARD/ : {
        *(.note .note.*)
        *(.eh_frame .eh_frame.*)
        *(.eh_frame_hdr .eh_frame_hdr.*)
        *(.comment)
    }

    __kernel_start = 0x100000;
    __kernel_end = .;
    __kernel_size = __kernel_end - __kernel_start;
}
"#.to_string()
}

fn generate_riscv_linker_script() -> String {
    r#"
ENTRY(_start)

MEMORY
{
    RAM : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS
{
    . = 0x80000000;

    .text ALIGN(4K) : {
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) : {
        *(.data .data.*)
    }

    .bss ALIGN(4K) : {
        *(.bss .bss.*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.note .note.*)
        *(.eh_frame .eh_frame.*)
        *(.comment)
    }

    __kernel_start = 0x80000000;
    __kernel_end = .;
    __kernel_size = __kernel_end - __kernel_start;
}
"#.to_string()
}
