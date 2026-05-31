TanOS: a Rust microkernel that runs isolated processes, does measured IPC, and auto-recovers from crashes

A from-scratch x86-64 microkernel in Rust, built to put Tanenbaum's microkernel principles on actual hardware (well - QEMU/KVM). This is a writeup of what *actually works*, with honest numbers and the bugs that were interesting along the way. No vapor: every claim below was observed on a serial console.

What it does (all verified in QEMU/KVM)

- Boots via GRUB + multiboot2, parses the real e820 memory map, and brings up its subsystems (memory, interrupts, IPC, capabilities) on x86-64.
- Real 4-level paging. Each process gets its own address space; the kernel identity-maps the low 1 GB into every address space (2 MB pages) so it keeps  running across CR3 switches.
- Runs userspace in ring 3. It loads ELF binaries from an initrd module, maps their `PT_LOAD` segments, sets up a GDT (user segments + TSS) and an `int 0x80` syscall gate, and `iretq`s into ring 3.
- Synchronous IPC between two *isolated* processes (separate page tables): a `call`/`receive`/`reply` rendezvous, message in a register.
- Fault isolation + reincarnation. When a ring-3 process executes an illegal instruction (or any fatal fault), the kernel catches it, kills *only* that process, and - the MINIX-3 idea - restarts it from its image. Other processes are untouched and the kernel never goes down.

Here's the actual fault-recovery transcript:

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

 The IPC number (and honest caveats)

20,000 synchronous cross-address-space call/reply round-trips between two isolated userspace processes, timed end-to-end with `rdtsc`:

-~3,800 cycles / round-trip under QEMU KVM (`-cpu host`, real host TSC), with high run-to-run variance (~3,700–4,500 - short benchmark + KVM noise; a rigorous number needs many more iterations and ideally bare metal).
- (Under plain QEMU TCG it reports ~49,948 "cycles" - meaningless, since TCG's `rdtsc` tracks host time, not guest cycles. Always benchmark a kernel under KVM or on metal.)

That's roughly 6 seL4's published ~600-cycle fastpath, and I want to be clear about why and not over-claim:

- This is the unoptimized slow path: two full `int 0x80` save/restore transitions, two CR3 reloads (full TLB flush - no PCID), a process-manager spinlock, and a rendezvous mutex, *per round-trip*.
- It is genuinely cross-address-space (real isolation), so the TLB cost is inherent without PCID/tagged-TLB.

So: TanOS does not claim seL4-competitive IPC. It claims working, honestly-measured cross-AS IPC. Closing the gap would need PCID plus a hand-tuned register fastpath - a real project, not a footnote.

 Three bugs that were worth the debugging

1. An 8 KB allocation killed the boot. The `MemoryManager` struct was 12 KB because it embedded a `#[repr(align(4096))]` page table by value; moving it into a `Once` static overflowed the 16 KB kernel stack and *zeroed the global allocator's metadata* in adjacent `.bss`. Symptom: a heap that worked one line earlier reported `size=0`. Fix: `Box` the page table (struct → 104 B).
   Lesson: never move page-table-sized aligned structs through a small kernel stack.

2. Userspace jumped to address 0. Once `init` made cross-crate calls into its support library, the PIE codegen routed them through GOT entries - which were *zero*, because nothing relocates them. `call *%rbx` → jump to 0 → page fault. Fix: build the bare-metal target with `relocation-model=static` (direct calls, no GOT).

3. A non-canonical stack pointer. The initial user stack top was `0x800000000000` - bit 47 set with the high bits clear, i.e. *non-canonical*, which `#GP`s the moment it's loaded into RSP. Moved user space to a canonical range.

 Design note: how context switching works

Rather than fragile kernel-stack switching, every `int 0x80`/interrupt already saves the full register frame on a (single, reused) kernel stack. To switch processes the kernel just copies that frame into the current process's PCB, then restores another process's saved frame + CR3 and `iretq`s. Because the syscall gate clears IF (no nested entries) and each syscall fully resolves before the next, one kernel stack suffices. Simple, and it hasn't triple-faulted.

 What's honestly *not* done

- Not seL4-competitive on IPC (see above).
- Capability-based security is largely stubbed.
- Reincarnation is driven in-kernel for the demo; the "pure" design runs a
  userspace reincarnation server that the kernel notifies.
- No formal verification yet (seL4's actual moat).
- Memory-safe Rust, but there's still `unsafe` without safety comments.

 Why bother / what's the point

It's a clean, small (~5,700-line kernel) demonstration that the microkernel
story - isolation, message passing, and *crash recovery* - works end-to-end in
safe-ish Rust, with numbers you can reproduce. Toolchain: Rust
nightly-2024-01-15, `x86_64-unknown-none`, GRUB ISO, QEMU/KVM.

Happy to answer questions about any of the above.
