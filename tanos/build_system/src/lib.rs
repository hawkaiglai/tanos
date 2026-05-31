//! Build utilities for TanOS microkernel system
//! 
//! This crate provides utilities for building, linking, and testing
//! the TanOS microkernel and userspace components.

pub mod qemu_runner;
pub mod linker_script;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

/// Supported target architectures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    RiscV64,
}

impl Architecture {
    pub fn target_triple(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "x86_64-unknown-none",
            Architecture::RiscV64 => "riscv64imac-unknown-none-elf",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "x86_64" | "x86-64" | "amd64" => Ok(Architecture::X86_64),
            "riscv64" | "risc-v" | "riscv" => Ok(Architecture::RiscV64),
            _ => anyhow::bail!("Unsupported architecture: {}", s),
        }
    }
}

/// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub arch: Architecture,
    pub profile: BuildProfile,
    pub workspace_root: PathBuf,
    pub target_dir: PathBuf,
    pub verbose: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildProfile {
    Debug,
    Release,
    Kernel,
    Userspace,
}

impl BuildProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            BuildProfile::Debug => "debug",
            BuildProfile::Release => "release",
            BuildProfile::Kernel => "kernel",
            BuildProfile::Userspace => "userspace",
        }
    }
}

impl BuildConfig {
    pub fn new(arch: Architecture, workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        let target_dir = workspace_root.join("target").join(arch.target_triple());
        
        Self {
            arch,
            profile: BuildProfile::Release,
            workspace_root,
            target_dir,
            verbose: false,
        }
    }

    pub fn with_profile(mut self, profile: BuildProfile) -> Self {
        self.profile = profile;
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn kernel_binary_path(&self) -> PathBuf {
        self.target_dir.join(self.profile.as_str()).join("kernel")
    }

    pub fn userspace_binary_path(&self, name: &str) -> PathBuf {
        self.target_dir.join(self.profile.as_str()).join(name)
    }
}

/// Build the kernel
pub fn build_kernel(config: &BuildConfig) -> Result<PathBuf> {
    println!("Building kernel for {} architecture...", config.arch.target_triple());

    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--package")
        .arg("kernel")
        .arg("--target")
        .arg(config.arch.target_triple())
        .arg("--profile")
        .arg("kernel")
        .current_dir(&config.workspace_root);

    // Add architecture-specific features
    match config.arch {
        Architecture::X86_64 => {
            cmd.arg("--features").arg("x86_64");
        }
        Architecture::RiscV64 => {
            cmd.arg("--features").arg("riscv64");
        }
    }

    if config.verbose {
        cmd.arg("--verbose");
    }

    let output = cmd.output()
        .context("Failed to execute cargo build for kernel")?;

    if !output.status.success() {
        anyhow::bail!(
            "Kernel build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let kernel_path = config.kernel_binary_path();
    if !kernel_path.exists() {
        anyhow::bail!("Kernel binary not found at expected location: {}", kernel_path.display());
    }

    // Print kernel size information
    let metadata = fs::metadata(&kernel_path)?;
    println!("Kernel size: {} bytes ({:.1} KB)", metadata.len(), metadata.len() as f64 / 1024.0);

    // Verify kernel size constraint (< 64KB as per spec)
    if metadata.len() > 65536 {
        anyhow::bail!("Kernel too large: {} bytes (max: 64KB)", metadata.len());
    }

    Ok(kernel_path)
}

/// Build userspace components
pub fn build_userspace(config: &BuildConfig, components: &[&str]) -> Result<Vec<PathBuf>> {
    println!("Building userspace components for {}...", config.arch.target_triple());

    let mut built_binaries = Vec::new();

    for component in components {
        println!("Building {}...", component);

        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--package")
            .arg(component)
            .arg("--target")
            .arg(config.arch.target_triple())
            .arg("--profile")
            .arg("userspace")
            .current_dir(&config.workspace_root);

        if config.verbose {
            cmd.arg("--verbose");
        }

        let output = cmd.output()
            .with_context(|| format!("Failed to execute cargo build for {}", component))?;

        if !output.status.success() {
            anyhow::bail!(
                "Build failed for {}:\n{}",
                component,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let binary_path = config.userspace_binary_path(component);
        if binary_path.exists() {
            built_binaries.push(binary_path);
        }
    }

    Ok(built_binaries)
}

/// Create initrd image from userspace binaries
pub fn create_initrd(config: &BuildConfig, binaries: &[PathBuf]) -> Result<PathBuf> {
    let initrd_path = config.target_dir.join("initrd.img");
    
    println!("Creating initrd image: {}", initrd_path.display());

    // Create temporary directory for initrd contents
    let temp_dir = tempfile::TempDir::new()?;
    let initrd_root = temp_dir.path();

    // Create directory structure
    fs::create_dir_all(initrd_root.join("bin"))?;
    fs::create_dir_all(initrd_root.join("sbin"))?;
    fs::create_dir_all(initrd_root.join("etc"))?;
    fs::create_dir_all(initrd_root.join("dev"))?;

    // Copy binaries
    for binary in binaries {
        if let Some(name) = binary.file_name() {
            let dest = match name.to_str().unwrap() {
                "init" => initrd_root.join("sbin").join(name),
                name if name.ends_with("_driver") => initrd_root.join("sbin").join(name),
                name if name.ends_with("_server") => initrd_root.join("sbin").join(name),
                _ => initrd_root.join("bin").join(name),
            };
            fs::copy(binary, dest)?;
        }
    }

    // Create simple init configuration
    let init_config = r#"# TanOS Init Configuration
[processes]
keyboard_driver = { path = "/sbin/keyboard_driver", restart = true }
vga_driver = { path = "/sbin/vga_driver", restart = true }
process_server = { path = "/sbin/process_server", restart = true }
vfs_server = { path = "/sbin/vfs_server", restart = true }

[boot]
shell = "/bin/shell"
"#;
    fs::write(initrd_root.join("etc").join("init.toml"), init_config)?;

    // Create cpio archive
    let mut cmd = Command::new("find");
    cmd.arg(".")
        .arg("-print0")
        .current_dir(initrd_root);

    let find_output = cmd.output()
        .context("Failed to list initrd contents")?;

    let mut cpio_cmd = Command::new("cpio");
    cpio_cmd.arg("-o")
        .arg("-H")
        .arg("newc")
        .arg("-0")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .current_dir(initrd_root);

    let mut cpio_process = cpio_cmd.spawn()
        .context("Failed to start cpio")?;

    if let Some(stdin) = cpio_process.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(&find_output.stdout)?;
    }

    let cpio_output = cpio_process.wait_with_output()
        .context("Failed to create cpio archive")?;

    if !cpio_output.status.success() {
        anyhow::bail!("cpio failed: {}", String::from_utf8_lossy(&cpio_output.stderr));
    }

    // Compress with gzip
    fs::write(&initrd_path, &cpio_output.stdout)?;

    println!("Initrd created: {} bytes", fs::metadata(&initrd_path)?.len());

    Ok(initrd_path)
}

/// Check if required build tools are available
pub fn check_build_tools() -> Result<()> {
    let required_tools = ["rustc", "cargo", "ld", "objcopy", "qemu-system-x86_64"];

    for tool in &required_tools {
        if which::which(tool).is_err() {
            anyhow::bail!("Required tool not found: {}", tool);
        }
    }

    // Check Rust toolchain
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .context("Failed to check Rust version")?;

    let version_str = String::from_utf8(output.stdout)?;
    if !version_str.contains("nightly") {
        anyhow::bail!("Nightly Rust toolchain required, found: {}", version_str.trim());
    }

    Ok(())
}

/// Extract version information from Cargo.toml
pub fn get_version_info(workspace_root: &Path) -> Result<String> {
    let cargo_toml = workspace_root.join("Cargo.toml");
    let content = fs::read_to_string(cargo_toml)?;
    let parsed: toml::Value = toml::from_str(&content)?;

    if let Some(workspace) = parsed.get("workspace") {
        if let Some(package) = workspace.get("package") {
            if let Some(version) = package.get("version") {
                return Ok(version.as_str().unwrap_or("unknown").to_string());
            }
        }
    }

    Ok("unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_target_triple() {
        assert_eq!(Architecture::X86_64.target_triple(), "x86_64-unknown-none");
        assert_eq!(Architecture::RiscV64.target_triple(), "riscv64imac-unknown-none-elf");
    }

    #[test]
    fn test_architecture_from_str() {
        assert_eq!(Architecture::from_str("x86_64").unwrap(), Architecture::X86_64);
        assert_eq!(Architecture::from_str("riscv64").unwrap(), Architecture::RiscV64);
        assert!(Architecture::from_str("invalid").is_err());
    }

    #[test]
    fn test_build_profile_str() {
        assert_eq!(BuildProfile::Debug.as_str(), "debug");
        assert_eq!(BuildProfile::Kernel.as_str(), "kernel");
    }
}
