# TanOS Build System
# Supports x86_64 and RISC-V architectures

.PHONY: all kernel userspace drivers servers apps test run clean install help
.PHONY: debug release bench fmt clippy check docs
.PHONY: qemu qemu-debug qemu-gdb iso usb
.SILENT: help

# Configuration
ARCH ?= x86_64
PROFILE ?= kernel
TARGET = $(ARCH)-unknown-none
BUILD_DIR = target/$(TARGET)/$(PROFILE)
KERNEL_DIR = $(BUILD_DIR)
ISO_DIR = build/iso
USB_IMAGE = build/tanos.img
QEMU_MEMORY ?= 512M
QEMU_CPUS ?= 2

# Tools
CARGO = cargo +nightly-2024-01-15
RUSTFMT = rustfmt +nightly-2024-01-15
CLIPPY = clippy +nightly-2024-01-15
OBJCOPY = rust-objcopy
OBJDUMP = rust-objdump
QEMU_X86 = qemu-system-x86_64
QEMU_RISCV = qemu-system-riscv64
GRUB_MKRESCUE = grub-mkrescue
DD = dd

# Architecture-specific settings
ifeq ($(ARCH),x86_64)
    QEMU = $(QEMU_X86)
    QEMU_MACHINE = q35
    QEMU_CPU = qemu64,+x2apic
    KERNEL_TARGET = x86_64-unknown-none
    BOOTLOADER_FEATURES = x86_64
else ifeq ($(ARCH),riscv64)
    QEMU = $(QEMU_RISCV)
    QEMU_MACHINE = virt
    QEMU_CPU = rv64
    KERNEL_TARGET = riscv64gc-unknown-none-elf
    BOOTLOADER_FEATURES = riscv64
else
    $(error Unsupported architecture: $(ARCH))
endif

# QEMU flags
QEMU_FLAGS = -machine $(QEMU_MACHINE) \
             -cpu $(QEMU_CPU) \
             -m $(QEMU_MEMORY) \
             -smp $(QEMU_CPUS) \
             -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
             -serial stdio \
             -display none

QEMU_DEBUG_FLAGS = $(QEMU_FLAGS) -s -S
QEMU_KVM_FLAGS = $(QEMU_FLAGS) -enable-kvm

# Colors for output
RED = \033[0;31m
GREEN = \033[0;32m
YELLOW = \033[0;33m
BLUE = \033[0;34m
PURPLE = \033[0;35m
CYAN = \033[0;36m
NC = \033[0m # No Color

# Default target
all: check kernel userspace

# Help target
help:
	echo "$(CYAN)TanOS Build System$(NC)"
	echo ""
	echo "$(YELLOW)Build Targets:$(NC)"
	echo "  all         - Build kernel and userspace (default)"
	echo "  kernel      - Build only the kernel"
	echo "  userspace   - Build userspace components"
	echo "  drivers     - Build device drivers"
	echo "  servers     - Build system servers"
	echo "  apps        - Build applications"
	echo ""
	echo "$(YELLOW)Development:$(NC)"
	echo "  check       - Run cargo check on all components"
	echo "  fmt         - Format all code"
	echo "  clippy      - Run clippy lints"
	echo "  test        - Run all tests"
	echo "  bench       - Run benchmarks"
	echo "  docs        - Generate documentation"
	echo ""
	echo "$(YELLOW)Execution:$(NC)"
	echo "  run         - Build and run in QEMU"
	echo "  qemu        - Run with QEMU (requires pre-built image)"
	echo "  qemu-debug  - Run with QEMU in debug mode"
	echo "  qemu-gdb    - Run with QEMU waiting for GDB"
	echo ""
	echo "$(YELLOW)Distribution:$(NC)"
	echo "  iso         - Create bootable ISO image"
	echo "  usb         - Create bootable USB image"
	echo ""
	echo "$(YELLOW)Maintenance:$(NC)"
	echo "  clean       - Clean build artifacts"
	echo "  install     - Install system tools"
	echo ""
	echo "$(YELLOW)Variables:$(NC)"
	echo "  ARCH={x86_64,riscv64}  - Target architecture (default: x86_64)"
	echo "  PROFILE={kernel,userspace,debug,release} - Build profile"
	echo "  QEMU_MEMORY=SIZE       - QEMU memory size (default: 512M)"
	echo "  QEMU_CPUS=N           - Number of QEMU CPUs (default: 2)"

# Check all code
check:
	@echo "$(BLUE)Checking all components...$(NC)"
	$(CARGO) check --workspace --target $(KERNEL_TARGET)

# Format code
fmt:
	@echo "$(BLUE)Formatting code...$(NC)"
	$(CARGO) fmt --all

# Run clippy
clippy:
	@echo "$(BLUE)Running clippy...$(NC)"
	$(CARGO) clippy --workspace --target $(KERNEL_TARGET) -- -D warnings

# Build kernel
kernel:
	@echo "$(GREEN)Building kernel for $(ARCH)...$(NC)"
	$(CARGO) build --package kernel \
		--target $(KERNEL_TARGET) \
		--profile $(PROFILE) \
		--features $(ARCH)
	@echo "$(GREEN)Kernel built successfully$(NC)"
	@echo "$(CYAN)Kernel size: $$(du -h $(KERNEL_DIR)/kernel | cut -f1)$(NC)"
	@$(OBJDUMP) -h $(KERNEL_DIR)/kernel | grep -E '\.text|\.data|\.bss'

# Build userspace
userspace: drivers servers apps

# Build drivers
drivers:
	@echo "$(GREEN)Building device drivers...$(NC)"
	@for driver in keyboard vga storage network; do \
		echo "  Building $$driver driver..."; \
		$(CARGO) build --package $$driver \
			--target $(KERNEL_TARGET) \
			--profile userspace || exit 1; \
	done

# Build servers
servers:
	@echo "$(GREEN)Building system servers...$(NC)"
	@for server in process_server memory_server vfs network_stack; do \
		echo "  Building $$server..."; \
		$(CARGO) build --package $$server \
			--target $(KERNEL_TARGET) \
			--profile userspace || exit 1; \
	done

# Build init process
init:
	@echo "$(GREEN)Building init process...$(NC)"
	$(CARGO) build --package init \
		--target $(KERNEL_TARGET) \
		--profile userspace

# Build applications
apps: init
	@echo "$(GREEN)Building applications...$(NC)"
	@for app in shell; do \
		echo "  Building $$app..."; \
		$(CARGO) build --package $$app \
			--target $(KERNEL_TARGET) \
			--profile userspace || exit 1; \
	done

# Create initrd image
initrd: userspace
	@echo "$(BLUE)Creating initrd...$(NC)"
	@mkdir -p $(BUILD_DIR)
	@mkdir -p build/initrd/{bin,drivers,servers}
	
	# Copy binaries
	@cp $(BUILD_DIR)/init build/initrd/bin/
	@cp $(BUILD_DIR)/shell build/initrd/bin/
	
	# Copy drivers
	@cp $(BUILD_DIR)/keyboard build/initrd/drivers/
	@cp $(BUILD_DIR)/vga build/initrd/drivers/
	@cp $(BUILD_DIR)/storage build/initrd/drivers/
	@cp $(BUILD_DIR)/network build/initrd/drivers/
	
	# Copy servers
	@cp $(BUILD_DIR)/process_server build/initrd/servers/
	@cp $(BUILD_DIR)/memory_server build/initrd/servers/
	@cp $(BUILD_DIR)/vfs build/initrd/servers/
	@cp $(BUILD_DIR)/network_stack build/initrd/servers/
	
	# Create initrd archive
	@cd build/initrd && find . | cpio -o -H newc | gzip > ../initrd.img
	@echo "$(GREEN)Initrd created: build/initrd.img$(NC)"

# Run tests
test: unit-tests integration-tests

unit-tests:
	@echo "$(BLUE)Running unit tests...$(NC)"
	$(CARGO) test --lib --workspace

integration-tests: all initrd
	@echo "$(BLUE)Running integration tests...$(NC)"
	$(CARGO) test --test '*' --workspace --features integration

# Run benchmarks
bench: all
	@echo "$(BLUE)Running benchmarks...$(NC)"
	$(CARGO) bench --workspace --features benchmark

# Generate documentation
docs:
	@echo "$(BLUE)Generating documentation...$(NC)"
	$(CARGO) doc --workspace --no-deps --target $(KERNEL_TARGET)

# Build everything and run in QEMU
run: all initrd
	@echo "$(GREEN)Launching TanOS in QEMU...$(NC)"
	$(QEMU) $(QEMU_FLAGS) \
		-kernel $(KERNEL_DIR)/kernel \
		-initrd build/initrd.img

# Run in QEMU (requires pre-built image)
qemu:
	@echo "$(GREEN)Running TanOS in QEMU...$(NC)"
	$(QEMU) $(QEMU_FLAGS) \
		-kernel $(KERNEL_DIR)/kernel \
		-initrd build/initrd.img

# Run in QEMU with KVM acceleration
qemu-kvm: all initrd
	@echo "$(GREEN)Running TanOS in QEMU with KVM...$(NC)"
	$(QEMU) $(QEMU_KVM_FLAGS) \
		-kernel $(KERNEL_DIR)/kernel \
		-initrd build/initrd.img

# Debug in QEMU
qemu-debug: all initrd
	@echo "$(YELLOW)Running TanOS in QEMU debug mode...$(NC)"
	@echo "$(YELLOW)Connect GDB to localhost:1234$(NC)"
	$(QEMU) $(QEMU_DEBUG_FLAGS) \
		-kernel $(KERNEL_DIR)/kernel \
		-initrd build/initrd.img

# Run QEMU waiting for GDB
qemu-gdb: all initrd
	@echo "$(YELLOW)Starting QEMU, waiting for GDB connection...$(NC)"
	@echo "$(YELLOW)Run: gdb $(KERNEL_DIR)/kernel$(NC)"
	@echo "$(YELLOW)Then: target remote :1234$(NC)"
	$(QEMU) $(QEMU_DEBUG_FLAGS) \
		-kernel $(KERNEL_DIR)/kernel \
		-initrd build/initrd.img

# Create bootable ISO
iso: all initrd
	@echo "$(BLUE)Creating bootable ISO...$(NC)"
	@mkdir -p $(ISO_DIR)/boot/grub
	
	# Copy kernel and initrd
	@cp $(KERNEL_DIR)/kernel $(ISO_DIR)/boot/
	@cp build/initrd.img $(ISO_DIR)/boot/
	
	# Create GRUB config
	@echo 'set timeout=0' > $(ISO_DIR)/boot/grub/grub.cfg
	@echo 'set default=0' >> $(ISO_DIR)/boot/grub/grub.cfg
	@echo '' >> $(ISO_DIR)/boot/grub/grub.cfg
	@echo 'menuentry "TanOS" {' >> $(ISO_DIR)/boot/grub/grub.cfg
	@echo '    multiboot2 /boot/kernel' >> $(ISO_DIR)/boot/grub/grub.cfg
	@echo '    module2 /boot/initrd.img initrd' >> $(ISO_DIR)/boot/grub/grub.cfg
	@echo '    boot' >> $(ISO_DIR)/boot/grub/grub.cfg
	@echo '}' >> $(ISO_DIR)/boot/grub/grub.cfg
	
	# Create ISO
	@mkdir -p build
	$(GRUB_MKRESCUE) -o build/tanos.iso $(ISO_DIR)
	@echo "$(GREEN)ISO created: build/tanos.iso$(NC)"

# Create bootable USB image
usb: iso
	@echo "$(BLUE)Creating USB image...$(NC)"
	@if [ ! -f build/tanos.iso ]; then \
		echo "$(RED)ISO not found, building...$(NC)"; \
		$(MAKE) iso; \
	fi
	
	# Create disk image (16MB)
	$(DD) if=/dev/zero of=$(USB_IMAGE) bs=1M count=16
	
	# Copy ISO content
	@echo "$(YELLOW)Use 'dd if=build/tanos.img of=/dev/sdX' to write to USB$(NC)"
	@echo "$(GREEN)USB image created: $(USB_IMAGE)$(NC)"

# Performance analysis
perf: all initrd
	@echo "$(BLUE)Running performance analysis...$(NC)"
	perf record -g $(QEMU) $(QEMU_FLAGS) \
		-kernel $(KERNEL_DIR)/kernel \
		-initrd build/initrd.img &
	@sleep 10
	@pkill qemu
	perf report

# Code coverage
coverage:
	@echo "$(BLUE)Generating code coverage...$(NC)"
	$(CARGO) tarpaulin --workspace --out Html --output-dir coverage/

# Security audit
audit:
	@echo "$(BLUE)Running security audit...$(NC)"
	$(CARGO) audit

# Check kernel size
check-size: kernel
	@SIZE=$$(stat -c%s $(KERNEL_DIR)/kernel); \
	if [ $$SIZE -gt 65536 ]; then \
		echo "$(RED)Error: Kernel too large: $$SIZE bytes (limit: 64KB)$(NC)"; \
		exit 1; \
	else \
		echo "$(GREEN)Kernel size OK: $$SIZE bytes$(NC)"; \
	fi

# Install development tools
install:
	@echo "$(BLUE)Installing development tools...$(NC)"
	rustup component add rust-src llvm-tools-preview
	cargo install cargo-binutils
	cargo install bootimage
	@if command -v apt-get >/dev/null 2>&1; then \
		sudo apt-get update; \
		sudo apt-get install -y qemu-system-x86 qemu-system-misc grub-pc-bin xorriso; \
	elif command -v dnf >/dev/null 2>&1; then \
		sudo dnf install -y qemu-system-x86 qemu-system-riscv grub2-tools xorriso; \
	elif command -v pacman >/dev/null 2>&1; then \
		sudo pacman -S qemu-system-x86 qemu-system-riscv grub xorriso; \
	else \
		echo "$(YELLOW)Please install QEMU and GRUB manually$(NC)"; \
	fi

# Clean build artifacts
clean:
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	$(CARGO) clean
	@rm -rf build/
	@rm -rf coverage/
	@echo "$(GREEN)Clean completed$(NC)"

# Continuous integration target
ci: fmt clippy check test check-size
	@echo "$(GREEN)All CI checks passed$(NC)"

# Release build
release:
	@$(MAKE) PROFILE=release all

# Debug build
debug:
	@$(MAKE) PROFILE=debug all

# Show build information
info:
	@echo "$(CYAN)TanOS Build Information$(NC)"
	@echo "Architecture: $(ARCH)"
	@echo "Target: $(KERNEL_TARGET)"
	@echo "Profile: $(PROFILE)"
	@echo "Build Directory: $(BUILD_DIR)"
	@echo "QEMU: $(QEMU)"
	@echo "Memory: $(QEMU_MEMORY)"
	@echo "CPUs: $(QEMU_CPUS)"
	@echo ""
	@echo "$(CYAN)Rust Toolchain:$(NC)"
	@rustc --version
	@cargo --version
