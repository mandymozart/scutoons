// A bigger main-thread stack on Windows (MSVC): the default 1 MiB overflows while the module library compiles its wasm
// (wasmtime / Cranelift recurse deep on a big module such as `analog`) – `deck --scene rig:catpope-analog` died with
// STATUS_STACK_OVERFLOW before its first frame (2026-11-05). Linux and macOS main threads have 8 MiB.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CPAL_ASIO_DIR");
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if windows && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/STACK:16777216");
    }
    // asio-sys would fetch the SDK into %TEMP% on its own; the deck's build names the folder it builds against
    if windows && std::env::var_os("CARGO_FEATURE_ASIO").is_some() {
        let dir = std::env::var("CPAL_ASIO_DIR").unwrap_or_default();
        if dir.is_empty() || !std::path::Path::new(&dir).join("common").join("asio.h").is_file() {
            panic!(
                "\n\nASIO is in the Windows build by default and needs Steinberg's ASIO SDK, but CPAL_ASIO_DIR = '{dir}' holds no common\\asio.h.\n\
                 Unpack the SDK (steinberg.net) and set CPAL_ASIO_DIR to it, with LLVM (libclang) installed – deck/native/README.md \"Audio\".\n\
                 A build without ASIO: `--no-default-features --features bundled` (deck-native) or `--no-default-features` (deck-editor).\n\n"
            );
        }
    }
}
