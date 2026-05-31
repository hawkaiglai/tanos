// Link the init binary with the shared userspace linker script, which places
// it at 1GB (above the kernel's identity-mapped region) with ENTRY(_start).
fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let script = format!("{}/../userspace.ld", manifest_dir);
    println!("cargo:rustc-link-arg=-T{}", script);
    println!("cargo:rerun-if-changed={}", script);
}
