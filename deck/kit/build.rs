//! The kit's one source, `kit/tokens.toml` at the repo's top, is built into deck-kit as its base (base/tokens.rs). A build
//! whose context holds only deck/ (a container, `cross`) hands the folder in with `DECK_KIT` (Cross.toml, the Dockerfiles).
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=DECK_KIT");
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let dir = std::env::var_os("DECK_KIT").map(PathBuf::from).unwrap_or_else(|| manifest.join("../../kit"));
    let file = dir.join("tokens.toml");
    if !file.is_file() {
        panic!("deck-kit needs the kit's tokens at {} (the repo's kit/ beside deck/, or DECK_KIT=<that folder>)", file.display());
    }
    println!("cargo:rerun-if-changed={}", file.display());
    println!("cargo:rustc-env=DECK_KIT_TOKENS={}", file.display());
}
