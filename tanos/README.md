# TanOS

A from-scratch x86-64 microkernel in Rust, demonstrating Tanenbaum's
microkernel principles on real hardware (QEMU/KVM): isolated ring-3 processes,
synchronous cross-address-space IPC, fault isolation with automatic
reincarnation, and capability-checked IPC.

> The canonical, full README is at the repository root:
> [../README.md](../README.md). This file is a short, honest overview of the
> code in this directory. Every claim below is reproducible with the commands
> in the root README — don't trust the prose, build and run it.

## What actually works (verified under QEMU/KVM)

- **4-level paging** with a private address space per process; the kernel
  identity-maps the low 1 GB so it survives CR3 switches.
- **Ring-3 userspace**: ELF binaries loaded from an initrd, run in ring 3 via a
  GDT/TSS and an `int 0x80` syscall gate.
- **Synchronous IPC** (call/receive/reply) between isolated processes.
- **Fault isolation + reincarnation**: a ring-3 crash kills only that process;
  the kernel restarts it from its image and other processes are unaffected. The
  dead process's frames are reclaimed (no leak).
- **Capability-checked IPC**: a process needs a READ/WRITE capability for an
  endpoint to receive/send on it; the kernel denies otherwise. See
  [docs/CONSOLE_DRIVER_DEMO.md](docs/CONSOLE_DRIVER_DEMO.md).

## Honest status

- **IPC latency: ~3,800 cycles/round-trip** under KVM (real host TSC),
  unoptimized — two CR3/TLB flushes per round-trip, no PCID. This is **not**
  seL4-competitive (~600); closing the gap would need PCID + a register
  fastpath. Don't trust any other number; the older docs' "measured" cycle
  tables were aspirational and have been removed.
- **Kernel TCB**: roughly 5,700 lines of Rust; the stripped kernel binary is
  ~57 KB.
- **Capabilities** are enforced for IPC endpoints only; the other resource types
  (memory/IRQ/I/O-port/process) are defined but not yet enforced.
- **Not done**: no formal verification; some peripheral subsystems
  (network stack, driver registry) are scaffolding; reincarnation is driven
  in-kernel for the demo rather than by a userspace reincarnation server.
- Built with heavy AI assistance (Claude). The point of open-sourcing is that
  you can verify the claims yourself rather than take the writeup's word.

## Build & run

See [../README.md](../README.md) for the exact, verified build/boot/measure
commands (Rust `nightly-2024-01-15`, `x86_64-unknown-none`, GRUB ISO,
QEMU/KVM). The default boot runs the fault-isolation/reincarnation demo; build
the `init` crate with `--features console_demo` for the userspace
console-driver + capability demo.

## Layout

- `core/kernel/` — the kernel (boot, memory/paging, interrupts, processes,
  scheduler, IPC, capabilities).
- `userspace/` — `libmicro` (userspace runtime) and `init` (the demo program).
- `docs/` — the OSDEV writeup and the console-driver demo.

## License

MIT. See [../LICENSE](../LICENSE).
