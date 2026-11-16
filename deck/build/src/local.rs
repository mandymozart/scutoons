//! A build on the creator's own machine (research/NATIVE-EDITOR-GUI.md §5 "Compile"): the service's `Compiler` with the
//! same rules – our manifest, the pinned flags, the clippy pass, the strict run – on folders of the editor's cache, without
//! the secret, the queue or the live-show check. A local wasm is for audition only: the compile service stays the only
//! producer of a published module.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::compile::{manifest, Built, CompileError, Compiler};
use crate::config::Config;
use crate::diag::Diag;
use crate::files::{self, Files};

/// What a local build ends in: the module, or the problems to show (rustc's, the scan's, the strict run's, ours)
pub type Outcome = Result<Built, Vec<Diag>>;

/// The SDK as the image bakes it (deck/build/Dockerfile): `module/`, `module-macros/`, `catalog/` with their sources and
/// the standalone manifests of deck/build/sdk. Written again only where a file differs, so cargo keeps its build.
pub fn bake_sdk(deck: &Path, into: &Path) -> std::io::Result<PathBuf> {
    for (crate_dir, manifest) in [("module", "module.Cargo.toml"), ("module-macros", "macros.Cargo.toml"), ("catalog", "catalog.Cargo.toml")] {
        sync_dir(&deck.join(crate_dir).join("src"), &into.join(crate_dir).join("src"))?;
        sync_file(&deck.join("build/sdk").join(manifest), &into.join(crate_dir).join("Cargo.toml"))?;
    }
    Ok(into.to_path_buf())
}

fn sync_file(from: &Path, to: &Path) -> std::io::Result<()> {
    let bytes = std::fs::read(from)?;
    if std::fs::read(to).ok().as_deref() != Some(bytes.as_slice()) {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(to, bytes)?;
    }
    Ok(())
}

fn sync_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)?.flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            sync_dir(&entry.path(), &target)?;
        } else {
            sync_file(&entry.path(), &target)?;
        }
    }
    Ok(())
}

impl Config {
    /// A local build's settings: the SDK baked at `sdk`, cargo's folders under `root`, the deck workspace's lock (the
    /// versions the deck builds with, so `--offline` finds them in cargo's cache); no secret, no relay, no memory cap,
    /// a wall long enough for the first build of the SDK's dependencies.
    pub fn local(sdk: &Path, root: &Path, lock: Option<PathBuf>) -> Config {
        Config {
            listen: String::new(),
            token: String::new(),
            sdk_dir: sdk.to_path_buf(),
            work_dir: root.join("work"),
            target_dir: root.join("target"),
            cargo_home: None,
            lock_file: lock.filter(|p| p.is_file()),
            cargo: "cargo".into(),
            status_url: None,
            jobs: std::thread::available_parallelism().map_or(2, |n| (n.get() as u32 / 2).max(1)),
            wall: Duration::from_secs(600),
            memory_mb: 0,
            max_wasm: 2048 * 1024,
            max_queue: 0,
            queue_wait: Duration::ZERO,
        }
    }
}

/// The local builder: one `Compiler` (one build at a time, as the service) and its folders
pub struct Local {
    compiler: Compiler,
    sdk: PathBuf,
}

impl Local {
    /// `deck` is the deck workspace (its SDK sources and lock), `root` the cache folder the builds live in
    pub fn new(deck: &Path, root: &Path) -> std::io::Result<Local> {
        let sdk = bake_sdk(deck, &root.join("sdk"))?;
        std::fs::create_dir_all(root.join("work"))?;
        let config = Config::local(&sdk, root, Some(deck.join("Cargo.lock")));
        Ok(Local { compiler: Compiler::new(Arc::new(config)), sdk })
    }

    /// The manifest a working copy carries, so rust-analyzer in the external editor reads it as the service does
    pub fn manifest(&self) -> String {
        manifest(&self.sdk)
    }

    /// One build of `files` (the limits first, as the API holds them); blocks until it ends
    pub fn build(&self, files: &Files) -> Outcome {
        let problems = files::check(files);
        if !problems.is_empty() {
            return Err(problems);
        }
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| vec![Diag::general(format!("no runtime for the build: {e}"))])?;
        runtime.block_on(self.compiler.compile(files)).map_err(problems_of)
    }
}

/// A compile error as the problems a creator reads
pub fn problems_of(e: CompileError) -> Vec<Diag> {
    match e {
        CompileError::Rejected(d) | CompileError::Failed(d) => d,
        CompileError::LiveShow => vec![Diag::general("a show is live: no builds now")],
        CompileError::Internal(m) => vec![Diag::general(format!("the build could not run: {m}"))],
    }
}

/// The sources of a working copy: its `src/*.rs` whose names a module may have, in name order
pub fn read_sources(dir: &Path) -> std::io::Result<Files> {
    let mut out = Files::new();
    for entry in std::fs::read_dir(dir.join("src"))?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if files::valid_name(&name) && entry.path().is_file() {
            out.insert(name, std::fs::read_to_string(entry.path())?);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_config_keeps_the_rules_and_drops_the_service() {
        let root = Path::new("/cache/modules");
        let c = Config::local(Path::new("/cache/modules/sdk"), root, None);
        assert_eq!((c.status_url, c.memory_mb, c.token.as_str()), (None, 0, ""));
        assert_eq!(c.target_dir, root.join("target"));
        assert!(c.wall >= Duration::from_secs(60) && c.jobs >= 1);
    }

    #[test]
    fn baking_writes_the_sdk_once_and_keeps_unchanged_files() {
        let deck = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let root = tempfile::tempdir().unwrap();
        let sdk = bake_sdk(deck, &root.path().join("sdk")).unwrap();
        let lib = sdk.join("module/src/lib.rs");
        assert!(lib.is_file() && sdk.join("module-macros/Cargo.toml").is_file() && sdk.join("catalog/Cargo.toml").is_file());
        let first = std::fs::metadata(&lib).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        bake_sdk(deck, &sdk).unwrap();
        assert_eq!(std::fs::metadata(&lib).unwrap().modified().unwrap(), first, "an unchanged file is not written again");
    }

    #[test]
    fn a_working_copy_reads_only_module_files_and_the_limits_come_first() {
        let root = tempfile::tempdir().unwrap();
        let src = root.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "// lib").unwrap();
        std::fs::write(src.join("osc.rs"), "// osc").unwrap();
        std::fs::write(src.join("Notes.md"), "no").unwrap();
        let files = read_sources(root.path()).unwrap();
        assert_eq!(files.keys().collect::<Vec<_>>(), ["lib.rs", "osc.rs"]);
        let local = Local::new(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap(), &root.path().join("cache")).unwrap();
        let problems = local.build(&Files::from([("osc.rs".to_string(), String::new())])).unwrap_err();
        assert!(problems.iter().any(|d| d.message.contains("lib.rs")), "{problems:?}");
        assert!(local.manifest().contains("crate-type = [\"cdylib\"]"));
    }
}
