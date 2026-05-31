//! Linker script generation for TanOS kernel and userspace

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use crate::Architecture;

/// Linker script configuration
#[derive(Debug, Clone)]
pub struct LinkerConfig {
    pub arch: Architecture,
    pub target_type: TargetType,
    pub base_address: u64,
    pub stack_size: u64,
    pub heap_size: Option<u64>,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    Kernel,
    Userspace,
}

impl LinkerConfig {
    pub fn kernel(arch: Architecture) -> Self {
        let base_address = match arch {
            Architecture::X86_64 => 0xFFFFFFFF80000000, // Higher half kernel
            Architecture::RiscV64 => 0xFFFFFFFF80000000,
        };

        Self {
            arch,
            target_type: TargetType::Kernel,
            base_address,
            stack_size: 16 * 1024, // 16KB kernel stack
            heap_size: Some(1024 * 1024), // 1MB kernel heap
            output_path: PathBuf::from("kernel.ld"),
        }
    }

    pub fn userspace(arch: Architecture) -> Self {
        let base_address = match arch {
            Architecture::X86_64 => 0x400000, // Standard userspace load address
            Architecture::RiscV64 => 0x10000,
        };

        Self {
            arch,
            target_type: TargetType::Userspace,
            base_address,
            stack_size: 64 * 1024, // 64KB user stack
            heap_size: None, // Managed by userspace allocator
            output_path: PathBuf::from("userspace.ld"),
        }
    }

    pub fn with_output_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.output_path = path.into();
        self
    }

    pub fn with_stack_size(mut self, size: u64) -> Self {
        self.stack_size = size;
        self
    }

    pub fn with_heap_size(mut self, size: Option<u64>) -> Self {
        self.heap_size = size;
        self
    }
}

/// Generate linker script
pub fn generate_linker_script(config: &LinkerConfig) -> Result<String> {
    match config.target_type {
        TargetType::Kernel => generate_kernel_linker_script(config),
        TargetType::Userspace => generate_userspace_linker_script(config),
    }
}

fn generate_kernel_linker_script(config: &LinkerConfig) -> Result<String> {
    let mut script = String::new();

    // Entry point
    script.push_str("ENTRY(_start)\n\n");

    // Architecture-specific configuration
    match config.arch {
        Architecture::X86_64 => {
            script.push_str("OUTPUT_FORMAT(elf64-x86-64)\n");
            script.push_str("OUTPUT_ARCH(i386:x86-64)\n\n");
        }
        Architecture::RiscV64 => {
            script.push_str("OUTPUT_FORMAT(elf64-littleriscv)\n");
            script.push_str("OUTPUT_ARCH(riscv)\n\n");
        }
    }

    // Memory layout
    script.push_str("MEMORY\n{\n");
    script.push_str(&format!(
        "    RAM : ORIGIN = 0x{:x}, LENGTH = 256M\n",
        config.base_address
    ));
    script.push_str("}\n\n");

    // Constants
    script.push_str(&format!("PROVIDE(_kernel_base = 0x{:x});\n", config.base_address));
    script.push_str(&format!("PROVIDE(_stack_size = 0x{:x});\n", config.stack_size));
    if let Some(heap_size) = config.heap_size {
        script.push_str(&format!("PROVIDE(_heap_size = 0x{:x});\n", heap_size));
    }
    script.push_str("\n");

    // Sections
    script.push_str("SECTIONS\n{\n");
    script.push_str(&format!("    . = 0x{:x};\n\n", config.base_address));

    // Text section
    script.push_str("    .text ALIGN(4K) : {\n");
    script.push_str("        _text_start = .;\n");
    script.push_str("        KEEP(*(.text.boot))\n");
    script.push_str("        *(.text .text.*)\n");
    script.push_str("        . = ALIGN(4K);\n");
    script.push_str("        _text_end = .;\n");
    script.push_str("    } > RAM\n\n");

    // Read-only data section
    script.push_str("    .rodata ALIGN(4K) : {\n");
    script.push_str("        _rodata_start = .;\n");
    script.push_str("        *(.rodata .rodata.*)\n");
    script.push_str("        *(.srodata .srodata.*)\n");
    script.push_str("        . = ALIGN(4K);\n");
    script.push_str("        _rodata_end = .;\n");
    script.push_str("    } > RAM\n\n");

    // Data section
    script.push_str("    .data ALIGN(4K) : {\n");
    script.push_str("        _data_start = .;\n");
    script.push_str("        *(.data .data.*)\n");
    script.push_str("        *(.sdata .sdata.*)\n");
    script.push_str("        . = ALIGN(4K);\n");
    script.push_str("        _data_end = .;\n");
    script.push_str("    } > RAM\n\n");

    // BSS section
    script.push_str("    .bss ALIGN(4K) : {\n");
    script.push_str("        _bss_start = .;\n");
    script.push_str("        *(.bss .bss.*)\n");
    script.push_str("        *(.sbss .sbss.*)\n");
    script.push_str("        *(COMMON)\n");
    script.push_str("        . = ALIGN(4K);\n");
    script.push_str("        _bss_end = .;\n");
    script.push_str("    } > RAM\n\n");

    // Kernel heap (if specified)
    if config.heap_size.is_some() {
        script.push_str("    .heap ALIGN(4K) : {\n");
        script.push_str("        _heap_start = .;\n");
        script.push_str(&format!("        . += 0x{:x};\n", config.heap_size.unwrap()));
        script.push_str("        _heap_end = .;\n");
        script.push_str("    } > RAM\n\n");
    }

    // Stack
    script.push_str("    .stack ALIGN(4K) : {\n");
    script.push_str("        _stack_bottom = .;\n");
    script.push_str(&format!("        . += 0x{:x};\n", config.stack_size));
    script.push_str("        _stack_top = .;\n");
    script.push_str("    } > RAM\n\n");

    // End marker
    script.push_str("    _kernel_end = .;\n\n");

    // Debug sections (discarded in release builds)
    script.push_str("    /DISCARD/ : {\n");
    script.push_str("        *(.debug*)\n");
    script.push_str("        *(.comment)\n");
    script.push_str("        *(.note*)\n");
    script.push_str("        *(.eh_frame*)\n");
    script.push_str("    }\n");

    script.push_str("}\n");

    Ok(script)
}

fn generate_userspace_linker_script(config: &LinkerConfig) -> Result<String> {
    let mut script = String::new();

    // Entry point
    script.push_str("ENTRY(_start)\n\n");

    // Architecture-specific configuration
    match config.arch {
        Architecture::X86_64 => {
            script.push_str("OUTPUT_FORMAT(elf64-x86-64)\n");
            script.push_str("OUTPUT_ARCH(i386:x86-64)\n\n");
        }
        Architecture::RiscV64 => {
            script.push_str("OUTPUT_FORMAT(elf64-littleriscv)\n");
            script.push_str("OUTPUT_ARCH(riscv)\n\n");
        }
    }

    // Memory layout
    script.push_str("MEMORY\n{\n");
    script.push_str(&format!(
        "    RAM : ORIGIN = 0x{:x}, LENGTH = 64M\n",
        config.base_address
    ));
    script.push_str("}\n\n");

    // Constants
    script.push_str(&format!("PROVIDE(_base_address = 0x{:x});\n", config.base_address));
    script.push_str(&format!("PROVIDE(_stack_size = 0x{:x});\n", config.stack_size));
    script.push_str("\n");

    // Sections
    script.push_str("SECTIONS\n{\n");
    script.push_str(&format!("    . = 0x{:x};\n\n", config.base_address));

    // Program headers
    script.push_str("    .text : {\n");
    script.push_str("        _text_start = .;\n");
    script.push_str("        KEEP(*(.text.start))\n");
    script.push_str("        *(.text .text.*)\n");
    script.push_str("        *(.gnu.linkonce.t.*)\n");
    script.push_str("        . = ALIGN(16);\n");
    script.push_str("        _text_end = .;\n");
    script.push_str("    } > RAM\n\n");

    script.push_str("    .rodata : {\n");
    script.push_str("        _rodata_start = .;\n");
    script.push_str("        *(.rodata .rodata.*)\n");
    script.push_str("        *(.srodata .srodata.*)\n");
    script.push_str("        *(.gnu.linkonce.r.*)\n");
    script.push_str("        . = ALIGN(16);\n");
    script.push_str("        _rodata_end = .;\n");
    script.push_str("    } > RAM\n\n");

    script.push_str("    .data : {\n");
    script.push_str("        _data_start = .;\n");
    script.push_str("        *(.data .data.*)\n");
    script.push_str("        *(.sdata .sdata.*)\n");
    script.push_str("        *(.gnu.linkonce.d.*)\n");
    script.push_str("        . = ALIGN(16);\n");
    script.push_str("        _data_end = .;\n");
    script.push_str("    } > RAM\n\n");

    script.push_str("    .bss : {\n");
    script.push_str("        _bss_start = .;\n");
    script.push_str("        *(.bss .bss.*)\n");
    script.push_str("        *(.sbss .sbss.*)\n");
    script.push_str("        *(.gnu.linkonce.b.*)\n");
    script.push_str("        *(COMMON)\n");
    script.push_str("        . = ALIGN(16);\n");
    script.push_str("        _bss_end = .;\n");
    script.push_str("    } > RAM\n\n");

    // Thread-local storage
    script.push_str("    .tdata : {\n");
    script.push_str("        _tdata_start = .;\n");
    script.push_str("        *(.tdata .tdata.*)\n");
    script.push_str("        *(.gnu.linkonce.td.*)\n");
    script.push_str("        _tdata_end = .;\n");
    script.push_str("    } > RAM\n\n");

    script.push_str("    .tbss : {\n");
    script.push_str("        _tbss_start = .;\n");
    script.push_str("        *(.tbss .tbss.*)\n");
    script.push_str("        *(.gnu.linkonce.tb.*)\n");
    script.push_str("        _tbss_end = .;\n");
    script.push_str("    } > RAM\n\n");

    // Dynamic sections (for future shared library support)
    script.push_str("    .dynamic : {\n");
    script.push_str("        *(.dynamic)\n");
    script.push_str("    } > RAM\n\n");

    script.push_str("    .got : {\n");
    script.push_str("        *(.got.plt)\n");
    script.push_str("        *(.got)\n");
    script.push_str("    } > RAM\n\n");

    // End of program
    script.push_str("    _end = .;\n");
    script.push_str("    PROVIDE(end = .);\n\n");

    // Stack (grows downward from high addresses)
    let stack_base = 0x7FFFFFFF - config.stack_size;
    script.push_str(&format!("    PROVIDE(_stack_bottom = 0x{:x});\n", stack_base));
    script.push_str(&format!("    PROVIDE(_stack_top = 0x{:x});\n", 0x7FFFFFFF));

    // Debug sections (keep for debugging)
    script.push_str("\n    /* Debug sections */\n");
    script.push_str("    .debug_info     0 : { *(.debug_info) }\n");
    script.push_str("    .debug_abbrev   0 : { *(.debug_abbrev) }\n");
    script.push_str("    .debug_line     0 : { *(.debug_line) }\n");
    script.push_str("    .debug_frame    0 : { *(.debug_frame) }\n");
    script.push_str("    .debug_str      0 : { *(.debug_str) }\n");
    script.push_str("    .debug_loc      0 : { *(.debug_loc) }\n");
    script.push_str("    .debug_ranges   0 : { *(.debug_ranges) }\n");

    script.push_str("}\n");

    Ok(script)
}

/// Write linker script to file
pub fn write_linker_script(config: &LinkerConfig) -> Result<()> {
    let script = generate_linker_script(config)?;
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = config.output_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create output directory")?;
    }

    fs::write(&config.output_path, script)
        .with_context(|| format!("Failed to write linker script to {}", config.output_path.display()))?;

    println!("Generated linker script: {}", config.output_path.display());
    Ok(())
}

/// Generate linker scripts from templates
pub fn generate_from_template(
    template_path: &Path,
    config: &LinkerConfig,
) -> Result<String> {
    let template = fs::read_to_string(template_path)
        .with_context(|| format!("Failed to read template: {}", template_path.display()))?;

    let mut result = template;

    // Replace template variables
    result = result.replace("{{BASE_ADDRESS}}", &format!("0x{:x}", config.base_address));
    result = result.replace("{{STACK_SIZE}}", &format!("0x{:x}", config.stack_size));
    
    if let Some(heap_size) = config.heap_size {
        result = result.replace("{{HEAP_SIZE}}", &format!("0x{:x}", heap_size));
    } else {
        result = result.replace("{{HEAP_SIZE}}", "0");
    }

    // Architecture-specific replacements
    match config.arch {
        Architecture::X86_64 => {
            result = result.replace("{{OUTPUT_FORMAT}}", "elf64-x86-64");
            result = result.replace("{{OUTPUT_ARCH}}", "i386:x86-64");
        }
        Architecture::RiscV64 => {
            result = result.replace("{{OUTPUT_FORMAT}}", "elf64-littleriscv");
            result = result.replace("{{OUTPUT_ARCH}}", "riscv");
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_linker_config() {
        let config = LinkerConfig::kernel(Architecture::X86_64);
        assert_eq!(config.target_type, TargetType::Kernel);
        assert_eq!(config.base_address, 0xFFFFFFFF80000000);
        assert_eq!(config.stack_size, 16 * 1024);
        assert_eq!(config.heap_size, Some(1024 * 1024));
    }

    #[test]
    fn test_userspace_linker_config() {
        let config = LinkerConfig::userspace(Architecture::X86_64);
        assert_eq!(config.target_type, TargetType::Userspace);
        assert_eq!(config.base_address, 0x400000);
        assert_eq!(config.stack_size, 64 * 1024);
        assert_eq!(config.heap_size, None);
    }

    #[test]
    fn test_generate_kernel_linker_script() {
        let config = LinkerConfig::kernel(Architecture::X86_64);
        let script = generate_kernel_linker_script(&config).unwrap();
        
        assert!(script.contains("ENTRY(_start)"));
        assert!(script.contains("OUTPUT_FORMAT(elf64-x86-64)"));
        assert!(script.contains("0xFFFFFFFF80000000"));
        assert!(script.contains(".text"));
        assert!(script.contains(".data"));
        assert!(script.contains(".bss"));
    }

    #[test]
    fn test_generate_userspace_linker_script() {
        let config = LinkerConfig::userspace(Architecture::X86_64);
        let script = generate_userspace_linker_script(&config).unwrap();
        
        assert!(script.contains("ENTRY(_start)"));
        assert!(script.contains("OUTPUT_FORMAT(elf64-x86-64)"));
        assert!(script.contains("0x400000"));
        assert!(script.contains(".text"));
        assert!(script.contains(".data"));
        assert!(script.contains(".bss"));
    }
}
