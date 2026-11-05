// A bigger main-thread stack on Windows (MSVC): the default 1 MiB overflows while the module library compiles its wasm
// (wasmtime / Cranelift recurse deep on a big module such as `analog`) – `deck --scene rig:catpope-analog` died with
// STATUS_STACK_OVERFLOW before its first frame (2026-11-05). Linux and macOS main threads have 8 MiB.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/STACK:16777216");
    }
}
