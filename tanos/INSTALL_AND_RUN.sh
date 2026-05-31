#!/bin/bash

echo "🚀 TanOS Installation Script"
echo "Installing everything needed to run TanOS..."
echo ""

# Update package list
echo "📦 Updating package lists..."
sudo apt update

# Install QEMU for running TanOS
echo "🖥️  Installing QEMU..."
sudo apt install -y qemu-system-x86 qemu-system-misc qemu-utils

# Install build dependencies
echo "🔧 Installing build dependencies..."
sudo apt install -y build-essential curl git

# Install Rust if not present
if ! command -v rustup &> /dev/null; then
    echo "🦀 Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
else
    echo "✅ Rust already installed"
fi

# Setup Rust for OS development
echo "🎯 Setting up Rust for OS development..."
source ~/.cargo/env
rustup toolchain install nightly-2024-01-15
rustup component add rust-src --toolchain nightly-2024-01-15
rustup target add x86_64-unknown-none --toolchain nightly-2024-01-15
rustup target add riscv64gc-unknown-none-elf --toolchain nightly-2024-01-15

# Optional: Install GRUB tools for ISO creation
echo "💿 Installing GRUB tools for bootable ISOs..."
sudo apt install -y grub-pc-bin grub-efi-amd64-bin mtools xorriso

echo ""
echo "✅ Installation complete!"
echo ""
echo "🚀 Ready to run TanOS!"
echo ""
echo "Commands to run TanOS:"
echo "  cd tanos"
echo "  make run              # Build and run in QEMU"
echo "  make iso              # Create bootable ISO"
echo "  make qemu-debug       # Run with debug output"
echo ""