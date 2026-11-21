//! The local working copy of a module (research/NATIVE-EDITOR-GUI.md §5): one folder per key in the cache, `src/*.rs` as
//! the external editor sees them, our `Cargo.toml` and `clippy.toml` beside them (rust-analyzer then checks as the build
//! does), and `.base.json` – the files and version it was taken from, so a local change is kept and told apart. The API
//! stays the source: the folder is rebuilt from it whenever it holds no change of its own.
use scutoons_build::files::Files;
use scutoons_build::local::read_sources;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const BASE: &str = ".base.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Base {
    key: String,
    version: Option<u64>,
    files: Files,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checkout {
    /// written from the server's files
    Fresh,
    /// the folder had changes of its own: they stay
    Kept,
}

/// `module:walk` → `<root>/module_walk`
pub fn folder(root: &Path, key: &str) -> PathBuf {
    root.join(key.replace([':', '/', '\\'], "_"))
}

fn base(dir: &Path) -> Option<Base> {
    serde_json::from_str(&std::fs::read_to_string(dir.join(BASE)).ok()?).ok()
}

/// the folder's sources differ from what it was taken from
pub fn changed(dir: &Path) -> bool {
    match (base(dir), read_sources(dir)) {
        (Some(b), Ok(now)) => b.files != now,
        _ => false,
    }
}

/// The server's `files` of `key` at `version` into the folder, unless it holds changes of its own
pub fn checkout(dir: &Path, key: &str, version: Option<u64>, files: &Files, manifest: &str, clippy: &str) -> std::io::Result<Checkout> {
    if changed(dir) {
        return Ok(Checkout::Kept);
    }
    replace(dir, key, version, files, manifest, clippy)?;
    Ok(Checkout::Fresh)
}

/// The server's `files` of `key` at `version` into the folder, whatever it holds (a revert's document comes back)
pub fn replace(dir: &Path, key: &str, version: Option<u64>, files: &Files, manifest: &str, clippy: &str) -> std::io::Result<()> {
    let src = dir.join("src");
    std::fs::create_dir_all(&src)?;
    for entry in std::fs::read_dir(&src)?.flatten() {
        if entry.path().extension().is_some_and(|x| x == "rs") {
            std::fs::remove_file(entry.path())?;
        }
    }
    for (name, text) in files {
        std::fs::write(src.join(name), text)?;
    }
    std::fs::write(dir.join("Cargo.toml"), manifest)?;
    std::fs::write(dir.join("clippy.toml"), clippy)?;
    std::fs::write(dir.join(BASE), serde_json::to_string_pretty(&Base { key: key.into(), version, files: files.clone() })?)
}

/// after a save: what was saved is the new base
pub fn saved(dir: &Path, key: &str, version: u64) -> std::io::Result<()> {
    let files = read_sources(dir)?;
    std::fs::write(dir.join(BASE), serde_json::to_string_pretty(&Base { key: key.into(), version: Some(version), files })?)
}

pub fn sources(dir: &Path) -> std::io::Result<Files> {
    read_sources(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_checkout_is_rebuilt_from_the_server_until_it_holds_a_change() {
        let root = std::env::temp_dir().join(format!("deck-editor-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = folder(&root, "module:walk");
        assert!(dir.ends_with("module_walk"));
        let v1 = Files::from([("lib.rs".to_string(), "// one".to_string())]);
        assert_eq!(checkout(&dir, "module:walk", Some(1), &v1, "[package]", "msrv").unwrap(), Checkout::Fresh);
        assert!(!changed(&dir) && dir.join("Cargo.toml").is_file() && dir.join("clippy.toml").is_file());
        assert_eq!(sources(&dir).unwrap(), v1);

        let v2 = Files::from([("lib.rs".to_string(), "// two".to_string())]);
        assert_eq!(checkout(&dir, "module:walk", Some(2), &v2, "[package]", "msrv").unwrap(), Checkout::Fresh, "no change of its own: the newer server files");
        std::fs::write(dir.join("src/lib.rs"), "// mine").unwrap();
        assert!(changed(&dir));
        assert_eq!(checkout(&dir, "module:walk", Some(3), &v1, "[package]", "msrv").unwrap(), Checkout::Kept);
        assert_eq!(sources(&dir).unwrap()["lib.rs"], "// mine");

        let v5 = Files::from([("lib.rs".to_string(), "// five".to_string())]);
        replace(&dir, "module:walk", Some(5), &v5, "[package]", "msrv").unwrap();
        assert!(!changed(&dir), "a revert's document replaces even a change of its own");
        assert_eq!(sources(&dir).unwrap(), v5);

        saved(&dir, "module:walk", 4).unwrap();
        assert!(!changed(&dir));
        let _ = std::fs::remove_dir_all(&root);
    }
}
