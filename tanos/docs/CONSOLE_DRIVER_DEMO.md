# Userspace virtual-console driver demo

A second, optional demo that shows the core microkernel pattern: a system
service that a monolithic kernel would implement *in-kernel* (the tty/console)
instead runs as an **isolated ring-3 process**, reachable only through message
passing.

- **PID 1** is a ring-3 **console-driver server**. It blocks in synchronous IPC
  receiving one byte per round-trip and "renders" each byte to the console.
- **PID 2** is a ring-3 **client**. It sends a line of text to the driver, one
  byte per IPC `call`, then sends an end-of-stream marker.

Every character of the printed line therefore crosses an address-space boundary
through the kernel's synchronous IPC path before the driver emits it.

> **What this is and isn't.** A *hardware* driver would do port I/O directly
> from ring 3, which requires delegating I/O privilege (IOPL or a TSS
> I/O-permission bitmap) — not yet implemented. So this "virtual" console emits
> through the kernel's debug-output primitive. The point being demonstrated is
> the **isolation + IPC structure** of a userspace driver, not raw device
> access. The driver and client are separate ring-3 processes in separate
> address spaces; a crash in the driver could not corrupt the client or the
> kernel (see the fault-isolation demo).

## Build & run

The demo is a build-time feature of the `init` binary (the default build is the
fault-isolation/reincarnation demo). From `tanos/`:

```bash
# Build the console-demo init and the kernel
cargo build -p init --target x86_64-unknown-none --profile userspace --features console_demo
cargo +nightly-2024-01-15 build --package kernel --target x86_64-unknown-none --profile kernel --features x86_64

# Assemble the ISO
cp target/x86_64-unknown-none/userspace/init   build/iso/boot/initrd
cp target/x86_64-unknown-none/kernel/kernel     build/iso/boot/kernel
grub-mkrescue -o build/tanos.iso build/iso

# Boot under KVM
timeout 12 qemu-system-x86_64 -accel kvm -cpu host -cdrom build/tanos.iso \
  -boot d -serial file:/tmp/s.txt -display none -m 512M -no-reboot -no-shutdown
cat /tmp/s.txt
```

## Expected output (verified under QEMU/KVM)

Filtering out the kernel's own `INFO`/`ERROR` log lines, the serial console
shows:

```
console-driver (PID 1): ready, serving bytes over IPC
console-driver (PID 1): --- begin client output ---
console-client (PID 2): sending a line through the userspace driver
Hello from a ring-3 client, rendered by a ring-3 console driver via IPC!
WARN ... capability: PID 2 DENIED ipc_call on endpoint 99 (no send capability)
console-client (PID 2): correctly DENIED IPC on unauthorized endpoint 99 -- capabilities enforced
console-driver (PID 1): --- end of stream, exiting ---
console-client (PID 2): line sent, exiting
```

The line between the `begin`/`end` markers was produced one byte at a time by
the client, each byte handed to the driver over IPC and emitted by the driver.

The demo also exercises **capability enforcement**: every process is granted a
capability for the well-known console endpoint (0) at creation, so the client's
calls on endpoint 0 succeed — but when it deliberately attempts an IPC call on
endpoint 99 (which it holds no capability for), the kernel denies it. This is
the microkernel access-control model: IPC on an endpoint requires a capability
for that endpoint (READ to receive, WRITE to send), checked in the kernel's
`int 0x80` dispatch (`core/kernel/src/interrupt/mod.rs`).

## How it maps to the code

- `userspace/init/src/main.rs` — `console_driver()` (PID 1) and
  `console_client()` (PID 2), behind the `console_demo` feature.
- The IPC ABI used is the live kernel path: `int 0x80` with
  `SYS_IPC_CALL=0x02`, `SYS_IPC_RECEIVE=0x01`, `SYS_IPC_REPLY=0x03`,
  `SYS_IPC_REPLY_RECV=0x0C` (single-word register messages).
- Kernel side: `process::ipc_call` / `ipc_receive` / `ipc_reply` /
  `ipc_reply_recv` in `core/kernel/src/process/mod.rs`.
