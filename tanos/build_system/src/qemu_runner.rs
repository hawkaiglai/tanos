//! QEMU integration for TanOS testing and development

use anyhow::{Context, Result};
use clap::{Arg, Command as ClapCommand};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::fs;
use std::time::Duration;
use crate::{Architecture, BuildConfig};

/// QEMU configuration for different architectures
#[derive(Debug, Clone)]
pub struct QemuConfig {
    pub arch: Architecture,
    pub memory_mb: u32,
    pub kernel_path: PathBuf,
    pub initrd_path: Option<PathBuf>,
    pub enable_kvm: bool,
    pub enable_graphics: bool,
    pub enable_networking: bool,
    pub monitor_port: Option<u16>,
    pub gdb_port: Option<u16>,
    pub serial_output: SerialOutput,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum SerialOutput {
    Stdio,
    File(PathBuf),
    None,
}

impl QemuConfig {
    pub fn new(arch: Architecture, kernel_path: impl Into<PathBuf>) -> Self {
        Self {
            arch,
            memory_mb: 256,
            kernel_path: kernel_path.into(),
            initrd_path: None,
            enable_kvm: false,
            enable_graphics: false,
            enable_networking: false,
            monitor_port: None,
            gdb_port: None,
            serial_output: SerialOutput::Stdio,
            extra_args: Vec::new(),
        }
    }

    pub fn with_initrd(mut self, initrd_path: impl Into<PathBuf>) -> Self {
        self.initrd_path = Some(initrd_path.into());
        self
    }

    pub fn with_memory(mut self, memory_mb: u32) -> Self {
        self.memory_mb = memory_mb;
        self
    }

    pub fn with_kvm(mut self, enable: bool) -> Self {
        self.enable_kvm = enable;
        self
    }

    pub fn with_graphics(mut self, enable: bool) -> Self {
        self.enable_graphics = enable;
        self
    }

    pub fn with_gdb(mut self, port: u16) -> Self {
        self.gdb_port = Some(port);
        self
    }

    pub fn with_monitor(mut self, port: u16) -> Self {
        self.monitor_port = Some(port);
        self
    }

    pub fn qemu_binary(&self) -> &'static str {
        match self.arch {
            Architecture::X86_64 => "qemu-system-x86_64",
            Architecture::RiscV64 => "qemu-system-riscv64",
        }
    }
}

/// Run TanOS in QEMU
pub fn run_qemu(config: &QemuConfig) -> Result<()> {
    let qemu_bin = config.qemu_binary();
    
    // Check if QEMU is available
    if which::which(qemu_bin).is_err() {
        anyhow::bail!("QEMU not found: {}. Please install QEMU.", qemu_bin);
    }

    // Verify kernel exists
    if !config.kernel_path.exists() {
        anyhow::bail!("Kernel not found: {}", config.kernel_path.display());
    }

    let mut cmd = Command::new(qemu_bin);

    // Architecture-specific configuration
    match config.arch {
        Architecture::X86_64 => configure_x86_64(&mut cmd, config)?,
        Architecture::RiscV64 => configure_riscv64(&mut cmd, config)?,
    }

    // Common QEMU options
    cmd.arg("-m").arg(config.memory_mb.to_string());

    // Graphics
    if config.enable_graphics {
        cmd.arg("-vga").arg("std");
    } else {
        cmd.arg("-nographic");
    }

    // Networking
    if config.enable_networking {
        cmd.arg("-netdev")
            .arg("user,id=net0")
            .arg("-device")
            .arg("e1000,netdev=net0");
    }

    // Serial output
    match config.serial_output {
        SerialOutput::Stdio => {
            cmd.arg("-serial").arg("stdio");
        }
        SerialOutput::File(ref path) => {
            cmd.arg("-serial").arg(format!("file:{}", path.display()));
        }
        SerialOutput::None => {
            cmd.arg("-serial").arg("null");
        }
    }

    // Monitor
    if let Some(port) = config.monitor_port {
        cmd.arg("-monitor").arg(format!("tcp::{},server,nowait", port));
    }

    // GDB server
    if let Some(port) = config.gdb_port {
        cmd.arg("-gdb").arg(format!("tcp::{}", port));
        cmd.arg("-S"); // Start paused
    }

    // KVM acceleration (if available and requested)
    if config.enable_kvm && is_kvm_available() {
        cmd.arg("-enable-kvm");
        cmd.arg("-cpu").arg("host");
    }

    // Extra arguments
    for arg in &config.extra_args {
        cmd.arg(arg);
    }

    println!("Starting QEMU with command:");
    println!("{:?}", cmd);

    // Run QEMU
    let mut child = cmd.spawn()
        .context("Failed to start QEMU")?;

    // Wait for QEMU to exit
    let exit_status = child.wait()
        .context("Failed to wait for QEMU process")?;

    if !exit_status.success() {
        anyhow::bail!("QEMU exited with error: {:?}", exit_status.code());
    }

    Ok(())
}

fn configure_x86_64(cmd: &mut Command, config: &QemuConfig) -> Result<()> {
    // Machine type
    cmd.arg("-machine").arg("q35");

    // CPU
    cmd.arg("-cpu").arg("qemu64,+x2apic,+fsgsbase");

    // Kernel
    cmd.arg("-kernel").arg(&config.kernel_path);

    // Initrd
    if let Some(ref initrd_path) = config.initrd_path {
        if !initrd_path.exists() {
            anyhow::bail!("Initrd not found: {}", initrd_path.display());
        }
        cmd.arg("-initrd").arg(initrd_path);
    }

    // Boot options
    cmd.arg("-append").arg("console=ttyS0 quiet");

    Ok(())
}

fn configure_riscv64(cmd: &mut Command, config: &QemuConfig) -> Result<()> {
    // Machine type
    cmd.arg("-machine").arg("virt");

    // CPU
    cmd.arg("-cpu").arg("rv64");

    // Kernel (RISC-V uses -bios for bootloader, -kernel for kernel)
    cmd.arg("-kernel").arg(&config.kernel_path);

    // Initrd
    if let Some(ref initrd_path) = config.initrd_path {
        if !initrd_path.exists() {
            anyhow::bail!("Initrd not found: {}", initrd_path.display());
        }
        cmd.arg("-initrd").arg(initrd_path);
    }

    Ok(())
}

fn is_kvm_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new("/dev/kvm").exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// QEMU test runner for automated testing
pub struct QemuTestRunner {
    config: QemuConfig,
    timeout: Duration,
    expected_output: Vec<String>,
}

impl QemuTestRunner {
    pub fn new(config: QemuConfig) -> Self {
        Self {
            config,
            timeout: Duration::from_secs(30),
            expected_output: Vec::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn expect_output(mut self, output: impl Into<String>) -> Self {
        self.expected_output.push(output.into());
        self
    }

    pub fn run_test(&self) -> Result<TestResult> {
        use std::io::{BufRead, BufReader};
        use std::sync::mpsc;
        use std::thread;

        let qemu_bin = self.config.qemu_binary();
        let mut cmd = Command::new(qemu_bin);

        // Configure QEMU for testing
        match self.config.arch {
            Architecture::X86_64 => configure_x86_64(&mut cmd, &self.config)?,
            Architecture::RiscV64 => configure_riscv64(&mut cmd, &self.config)?,
        }

        cmd.arg("-m").arg(self.config.memory_mb.to_string())
            .arg("-nographic")
            .arg("-serial").arg("stdio")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()
            .context("Failed to start QEMU for testing")?;

        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();

        // Spawn thread to read output
        let expected_output = self.expected_output.clone();
        let output_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            let mut captured_output = Vec::new();
            let mut found_outputs = vec![false; expected_output.len()];

            for line in reader.lines() {
                if let Ok(line) = line {
                    captured_output.push(line.clone());
                    
                    // Check for expected outputs
                    for (i, expected) in expected_output.iter().enumerate() {
                        if line.contains(expected) {
                            found_outputs[i] = true;
                        }
                    }

                    // If all expected outputs found, signal success
                    if found_outputs.iter().all(|&found| found) {
                        let _ = tx.send(TestResult::Success { 
                            output: captured_output,
                            duration: std::time::Instant::now().elapsed(),
                        });
                        break;
                    }
                }
            }

            TestResult::Timeout { 
                output: captured_output,
                missing_outputs: expected_output.into_iter()
                    .zip(found_outputs.iter())
                    .filter_map(|(expected, &found)| if !found { Some(expected) } else { None })
                    .collect()
            }
        });

        // Wait for timeout or completion
        let result = match rx.recv_timeout(self.timeout) {
            Ok(result) => result,
            Err(_) => {
                // Timeout - kill QEMU
                let _ = child.kill();
                output_thread.join().unwrap_or_else(|_| TestResult::Error { 
                    message: "Output thread panicked".to_string()
                })
            }
        };

        // Clean up QEMU process
        let _ = child.kill();
        let _ = child.wait();

        Ok(result)
    }
}

#[derive(Debug)]
pub enum TestResult {
    Success {
        output: Vec<String>,
        duration: Duration,
    },
    Timeout {
        output: Vec<String>,
        missing_outputs: Vec<String>,
    },
    Error {
        message: String,
    },
}

impl TestResult {
    pub fn is_success(&self) -> bool {
        matches!(self, TestResult::Success { .. })
    }
}

/// Create QEMU runner from command line arguments
pub fn create_qemu_runner() -> Result<()> {
    let matches = ClapCommand::new("qemu_runner")
        .version("3.0.0")
        .about("QEMU runner for TanOS")
        .arg(Arg::new("kernel")
            .long("kernel")
            .value_name("FILE")
            .help("Path to kernel binary")
            .required(true))
        .arg(Arg::new("initrd")
            .long("initrd")
            .value_name("FILE")
            .help("Path to initrd image"))
        .arg(Arg::new("arch")
            .long("arch")
            .value_name("ARCH")
            .help("Target architecture")
            .default_value("x86_64"))
        .arg(Arg::new("memory")
            .long("memory")
            .value_name("MB")
            .help("Amount of memory in MB")
            .default_value("256"))
        .arg(Arg::new("kvm")
            .long("kvm")
            .help("Enable KVM acceleration")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("graphics")
            .long("graphics")
            .help("Enable graphics output")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("gdb")
            .long("gdb")
            .value_name("PORT")
            .help("Enable GDB server on port"))
        .arg(Arg::new("monitor")
            .long("monitor")
            .value_name("PORT")
            .help("Enable monitor on port"))
        .arg(Arg::new("test")
            .long("test")
            .help("Run in test mode")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("timeout")
            .long("timeout")
            .value_name("SECONDS")
            .help("Test timeout in seconds")
            .default_value("30"))
        .get_matches();

    let arch = Architecture::from_str(matches.get_one::<String>("arch").unwrap())?;
    let kernel_path = PathBuf::from(matches.get_one::<String>("kernel").unwrap());
    let memory_mb: u32 = matches.get_one::<String>("memory").unwrap().parse()?;

    let mut config = QemuConfig::new(arch, kernel_path)
        .with_memory(memory_mb)
        .with_kvm(matches.get_flag("kvm"))
        .with_graphics(matches.get_flag("graphics"));

    if let Some(initrd) = matches.get_one::<String>("initrd") {
        config = config.with_initrd(PathBuf::from(initrd));
    }

    if let Some(gdb_port) = matches.get_one::<String>("gdb") {
        config = config.with_gdb(gdb_port.parse()?);
    }

    if let Some(monitor_port) = matches.get_one::<String>("monitor") {
        config = config.with_monitor(monitor_port.parse()?);
    }

    if matches.get_flag("test") {
        let timeout_secs: u64 = matches.get_one::<String>("timeout").unwrap().parse()?;
        let timeout = Duration::from_secs(timeout_secs);

        let test_runner = QemuTestRunner::new(config)
            .with_timeout(timeout)
            .expect_output("TanOS initialized")
            .expect_output("Shell ready");

        let result = test_runner.run_test()?;
        
        match result {
            TestResult::Success { duration, .. } => {
                println!("Test passed in {:?}", duration);
                std::process::exit(0);
            }
            TestResult::Timeout { missing_outputs, .. } => {
                println!("Test timed out. Missing outputs: {:?}", missing_outputs);
                std::process::exit(1);
            }
            TestResult::Error { message } => {
                println!("Test error: {}", message);
                std::process::exit(1);
            }
        }
    } else {
        run_qemu(&config)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qemu_config_creation() {
        let config = QemuConfig::new(Architecture::X86_64, "/path/to/kernel");
        assert_eq!(config.arch, Architecture::X86_64);
        assert_eq!(config.memory_mb, 256);
        assert_eq!(config.qemu_binary(), "qemu-system-x86_64");
    }

    #[test]
    fn test_qemu_binary_selection() {
        assert_eq!(
            QemuConfig::new(Architecture::X86_64, "").qemu_binary(),
            "qemu-system-x86_64"
        );
        assert_eq!(
            QemuConfig::new(Architecture::RiscV64, "").qemu_binary(),
            "qemu-system-riscv64"
        );
    }
}
