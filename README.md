# TanOS: A Rust Microkernel

A from-scratch x86-64 microkernel in Rust, demonstrating isolated userspace processes, synchronous cross-address-space IPC, and automatic recovery from process crashes. Built to put Tanenbaum's microkernel principles on actual hardware (QEMU/KVM).

**Author:** Stephen Ifeanyichukwu Chuks-Onah  
**License:** MIT  
**Status:** Verified working, reproducible.

## What it demonstrates

- **Real 4-level paging**: each process gets its own address space; the kernel identity-maps the low 1 GB so it survives CR3 switches.
- **Ring 3 userspace**: loads ELF binaries from an initrd, sets up a GDT (user segments + TSS), and runs processes in ring 3 via `iretq`.
- **Synchronous IPC**: cross-address-space message passing (call/receive/reply rendezvous) between isolated processes.
- **Fault isolation + reincarnation**: when a ring-3 process crashes, the kernel kills *only* that process and restarts it from its image. Other processes and the kernel survive.

## Build & boot (verified working)

### Prerequisites
- Rust nightly (tested with `nightly-2024-01-15`)
- QEMU with KVM support (`qemu-system-x86_64`)
- grub-mkrescue + xorriso
- x86_64 Linux host

### Build the kernel

```bash
cargo +nightly-2024-01-15 build --package kernel \
  --target x86_64-unknown-none \
  --profile kernel \
  --features x86_64
```

The kernel binary: `target/x86_64-unknown-none/kernel/kernel` (~57 KB stripped)

### Build the init process

```bash
cargo build -p init --target x86_64-unknown-none --profile userspace
cp target/x86_64-unknown-none/userspace/init build/iso/boot/initrd
cp target/x86_64-unknown-none/kernel/kernel build/iso/boot/kernel
```

### Create the bootable ISO

```bash
grub-mkrescue -o build/tanos.iso build/iso
```

### Boot in QEMU/KVM

```bash
timeout 12 qemu-system-x86_64 \
  -accel kvm \
  -cpu host \
  -cdrom build/tanos.iso \
  -boot d \
  -serial file:/tmp/s.txt \
  -display none \
  -m 512M \
  -no-reboot \
  -no-shutdown
```

### Check the output

```bash
cat /tmp/s.txt
```

You should see:
```
survivor (PID 1): running
driver: starting (generation 0)
driver: hit a bug -- crashing (ud2)!
CPU Exception: vector=6 ... cs=0x1b ... (from ring 3)
fault isolation: PID 2 (gen 0) crashed (CPU exception 6) - kernel survives
reincarnation: restarting crashed PID 2 as PID 3 (generation 1)
driver: REINCARNATED and running normally -- recovery works!
driver: did useful work after recovery, exiting cleanly
survivor (PID 1): STILL ALIVE the whole time -- fault isolation works!
```

## Measuring IPC latency

The `init` process shipped here runs the **fault-isolation demo** (above). The IPC benchmark variant (server/client, 20,000 round-trips timed with `rdtsc`) is documented in the init source; you can swap it back to reproduce the ~3,800-cycle measurement.

**Result: ~3,800 cycles/round-trip** under QEMU KVM with real host TSC (`-accel kvm -cpu host`), with high run-to-run variance (~3,700–4,500). This is unoptimized: two full save/restore transitions, two CR3 reloads (TLB flush, no PCID), and spinlocks per round-trip.

Always benchmark under KVM or bare metal; plain TCG `rdtsc` reports host time, not guest cycles, and is meaningless here.

## A note on AI assistance

This project was built with heavy AI assistance (Anthropic's Claude). The kernel code, the init process, and the writeup are AI-generated or AI-assisted. The right way to evaluate the claims is **not to trust the prose** — it's to **build and run the code yourself**. Every claim in this README has a corresponding command above. Run them, check the output on your serial console. Reproducibility is the only credibility that matters.

## Design notes

### Context switching (no kernel-stack switching)

Every `int 0x80` syscall already saves the full register frame on a single, reused kernel stack. To switch processes, the kernel copies that frame into the current process's PCB, then restores another process's saved frame + CR3 and `iretq`s. Because the syscall gate clears IF (no nested entries) and each syscall fully resolves before the next, one kernel stack suffices.

### Memory layout

- **Kernel identity region**: low 1 GB, 2 MB huge pages, reachable in every address space.
- **Userspace**: starts at 0x40000000 (1 GB), avoids kernel region collisions.
- **User stack**: 0xC0000000 (canonical).

### Three bugs worth mentioning

1. **Stack overflow zeroed the heap**: `MemoryManager` embedded a `#[repr(align(4096))]` page table (4 KB aligned), overflowing the 16 KB kernel stack and zeroing the global allocator's metadata. Fix: `Box` the page table.
2. **Userspace jumped to address 0**: PIE codegen routed cross-crate calls through zero-valued GOT entries. Fix: build with `relocation-model=static` (direct calls, no GOT).
3. **Non-canonical stack pointer**: user stack top at 0x800000000000 (bit 47 set with high bits clear) caused #GP. Moved to 0xC0000000 (canonical).

## What's not done

- Not seL4-competitive on IPC (see above; closing that gap requires PCID + register fastpath).
- Capability-based security is largely stubbed.
- Reincarnation is in-kernel for the demo; the pure design runs a userspace reincarnation server.
- No formal verification yet.
- Memory-safe Rust, but contains `unsafe` without extensive safety comments.

## Useful references

- **Toolchain**: Rust nightly-2024-01-15, `x86_64-unknown-none` target, multiboot2 spec, x86-64 CPU manual.
- **Inspiration**: Tanenbaum's *Modern Operating Systems*, MINIX-3 (reincarnation idea), seL4 (IPC design patterns).

## License

MIT. See [LICENSE](LICENSE).

---

Questions? Issues? Reach out: stephen.find@proton.me
