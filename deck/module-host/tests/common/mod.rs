//! What the conformance suite shares: the repository modules built for wasm32 once, loading, error text.
#![allow(dead_code)]
use deck_module_host::{Backend, Loaded};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

pub const RATE: f64 = 48_000.0;

pub fn deck_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned()
}

/// the release wasm of a module crate (`walk`, `fixture`; built once for all the tests – `rustup target
/// add wasm32-unknown-unknown`)
pub fn wasm(file: &str) -> Vec<u8> {
    static BUILD: OnceLock<()> = OnceLock::new();
    let root = deck_root();
    let dir = root.join("target").join("wasm-modules");
    BUILD.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let out = std::process::Command::new(cargo)
            .current_dir(&root)
            .args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "walk-module", "-p", "fixture-module", "--target-dir"])
            .arg(&dir)
            .output()
            .expect("running cargo");
        assert!(
            out.status.success(),
            "building the example modules for wasm32 failed (rustup target add wasm32-unknown-unknown?):\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    });
    std::fs::read(dir.join("wasm32-unknown-unknown/release").join(file)).unwrap()
}

pub fn load<B: Backend>(e: &B::Engine, bytes: &[u8]) -> anyhow::Result<Arc<Loaded<B>>> {
    Loaded::<B>::load(e, bytes).map(Arc::new)
}

pub fn err_text<T>(r: anyhow::Result<T>) -> String {
    format!("{:#}", r.err().expect("an error"))
}
