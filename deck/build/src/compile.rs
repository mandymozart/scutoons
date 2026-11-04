//! One compile: the member's files in a crate the service writes itself, `cargo build --offline --release --target
//! wasm32-unknown-unknown`, the wasm and its descriptor out (RIG-EDITING.md §4). The rules that make it safe to compile
//! strangers' code:
//! - the `Cargo.toml` is ours: one path dependency (`deck-module`, whose only dependency is its derive crate), a `cdylib`,
//!   a fixed profile. The member's files are `src/*.rs`, nothing else – no `build.rs` (`build = false`), no dependency, no
//!   proc-macro of their own (the scan), no network (`--offline`, a vendored closure).
//! - rustc runs under a wall-clock, a memory and an output limit, `nice`d, with `CARGO_BUILD_JOBS` 2; the whole process tree
//!   dies with it. A fresh folder per compile, deleted after.
//! - the member's code is compiled, never run natively: the descriptor is read out of the wasm's custom section, and the
//!   wasm is played once in wasmtime's sandbox by the deck's own host (`strict`, #344).
//! - what makes a native run equal a wasm run is pinned by the service (`determinism`): the compiler flags, and a
//!   `clippy.toml` whose `disallowed-methods` (std's float transcendentals) a `cargo clippy` pass after the build reports
//!   as warnings.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine;
use deck_module_host::wasmtime_backend::WasmtimeEngine;
use sha2::{Digest, Sha256};
use tokio::process::Command;

use crate::config::Config;
use crate::determinism;
use crate::diag::{parse_cargo_json, parse_lints, Diag};
use crate::files::Files;
use crate::live::LiveCheck;
use crate::runner::{run_limited, BoxFuture, Killed, Limits, Outcome};
use crate::scan::scan;
use crate::strict;

/// the member crate's name: its wasm is `member.wasm`
pub const PACKAGE: &str = "member";
const TARGET: &str = "wasm32-unknown-unknown";
/// What the image primes the build folder with, and what `scutoons-build stub` writes
pub const STUB_SOURCE: &str = "use deck_module::{Block, Ctx, Module};\n\n#[derive(Module)]\n#[module(name = \"stub\")]\n#[port(output, audio, name = \"out\")]\nstruct Stub {\n    #[param(range = 0.0..1.0, default = 0.5)]\n    level: f32,\n}\n\nimpl Module for Stub {\n    fn process(&mut self, _block: &mut Block, _ctx: &Ctx) {}\n}\n";

#[derive(Debug)]
pub struct Built {
    pub hash: String,
    pub descriptor: serde_json::Value,
    pub wasm: Vec<u8>,
    pub warnings: Vec<Diag>,
}

impl Built {
    pub fn wasm_b64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(&self.wasm)
    }
}

#[derive(Debug)]
pub enum CompileError {
    /// refused before rustc ran (the scan)
    Rejected(Vec<Diag>),
    /// rustc's errors, or a limit that stopped it
    Failed(Vec<Diag>),
    /// the relay reports a live session
    LiveShow,
    /// our own trouble (cargo could not start, the disk): logged, not the member's to fix
    Internal(String),
}

/// The `Cargo.toml` of a member crate – ours, never the member's
pub fn manifest(sdk_dir: &Path) -> String {
    let sdk = sdk_dir.join("module");
    let sdk = sdk.to_string_lossy().replace('\\', "/");
    format!(
        "[package]\nname = \"{PACKAGE}\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\nbuild = false\nautobins = false\nautoexamples = false\nautotests = false\nautobenches = false\n\n\
         [lib]\npath = \"src/lib.rs\"\ncrate-type = [\"cdylib\"]\n\n\
         [dependencies]\ndeck-module = {{ path = '{sdk}' }}\n\n\
         # its own workspace: no Cargo.toml above the temp folder may adopt it\n[workspace]\n\n\
         [profile.release]\nopt-level = 3\nlto = \"fat\"\ncodegen-units = 1\npanic = \"abort\"\nstrip = true\ndebug = false\noverflow-checks = false\nincremental = false\n"
    )
}

pub struct Compiler {
    config: Arc<Config>,
    live: Option<LiveCheck>,
    toolchain: Option<String>,
    gate: tokio::sync::Mutex<()>,
    /// wasmtime for the strict test run, made at the first compile (its epoch ticker is a thread of its own)
    engine: std::sync::OnceLock<Arc<WasmtimeEngine>>,
}

/// the member crate's folder, removed when the compile ends
struct Folder(PathBuf);

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Compiler {
    pub fn new(config: Arc<Config>) -> Compiler {
        let live = config.status_url.as_deref().and_then(|u| {
            let parsed = LiveCheck::parse(u);
            if parsed.is_none() {
                tracing::warn!(url = u, "RELAY_STATUS_URL is not an http:// url: no live-show check");
            }
            parsed
        });
        let toolchain = std::process::Command::new("rustc").arg("--version").output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
        Compiler { config, live, toolchain, gate: tokio::sync::Mutex::new(()), engine: std::sync::OnceLock::new() }
    }

    pub fn toolchain(&self) -> Option<&str> {
        self.toolchain.as_deref()
    }

    /// Is a show live right now?
    pub async fn live_show(&self) -> bool {
        match &self.live {
            Some(live) => live.is_live().await,
            None => false,
        }
    }

    pub async fn compile(&self, files: &Files) -> Result<Built, CompileError> {
        let found = scan(files);
        if !found.rejected.is_empty() {
            return Err(CompileError::Rejected(found.rejected));
        }
        // one compile at a time, whoever asks (the service queues in front of this; a fixed folder needs it)
        let _one = self.gate.lock().await;
        let io = |what: &str, e: std::io::Error| CompileError::Internal(format!("{what}: {e}"));
        // the same folder every time: cargo's package id (and with it the module's symbol hashes) holds the path, and the
        // same source + toolchain should give the same file. Made fresh, deleted after, whatever happens.
        let dir = Folder(self.config.work_dir.join(PACKAGE));
        let _ = std::fs::remove_dir_all(&dir.0);
        self.write_crate(&dir.0, files).map_err(|e| io("writing the crate", e))?;

        let live = self.live.clone();
        let veto = move || -> BoxFuture<'static, bool> {
            let live = live.clone();
            Box::pin(async move {
                match live {
                    Some(l) => l.is_live().await,
                    None => false,
                }
            })
        };
        let limits = Limits { wall: self.config.wall, memory: self.config.memory_mb << 20, output: 16 << 20, tick: std::time::Duration::from_secs(1) };
        let outcome = run_limited(self.cargo(&dir.0, "build"), &limits, Some(&veto)).await.map_err(|e| CompileError::Internal(format!("cargo did not start: {e}")))?;
        let result = match self.interpret(outcome, !found.unlexable.is_empty()).await {
            Ok(mut built) => match self.lint(&dir.0, &limits, &veto).await {
                // the determinism lints are warnings; the strict run may still stop the module
                Ok(lints) => {
                    built.warnings.extend(lints);
                    self.strict_run(built).await
                }
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        };
        self.clean_member_artifacts();
        result
    }

    /// `cargo clippy` over the member, for the `clippy::disallowed_methods` warnings of clippy.toml (determinism.rs). Clippy
    /// missing or failing is logged and costs the member nothing: no warnings.
    async fn lint(&self, dir: &Path, limits: &Limits, veto: &(dyn Fn() -> BoxFuture<'static, bool> + Send + Sync)) -> Result<Vec<Diag>, CompileError> {
        let outcome = match run_limited(self.cargo(dir, "clippy"), limits, Some(veto)).await {
            Ok(outcome) => outcome,
            Err(e) => {
                tracing::warn!(error = %e, "cargo clippy did not start: no determinism lints");
                return Ok(vec![]);
            }
        };
        match outcome.killed {
            Some(Killed::Vetoed) => return Err(CompileError::LiveShow),
            Some(_) => {
                tracing::warn!("cargo clippy was stopped by a limit: no determinism lints");
                return Ok(vec![]);
            }
            None => {}
        }
        if !outcome.success() {
            let tail: String = outcome.stderr.chars().rev().take(500).collect::<Vec<_>>().into_iter().rev().collect();
            tracing::warn!(%tail, "cargo clippy failed: no determinism lints");
            return Ok(vec![]);
        }
        Ok(parse_lints(&outcome.stdout))
    }

    /// The strict test run of the built module (strict.rs): its error stops the module, the warnings so far go with it
    async fn strict_run(&self, built: Built) -> Result<Built, CompileError> {
        let engine = self.engine.get_or_init(|| Arc::new(WasmtimeEngine::new(None))).clone();
        let wasm = built.wasm.clone();
        let ran = tokio::task::spawn_blocking(move || strict::run(&engine, &wasm)).await.map_err(|e| CompileError::Internal(format!("the strict test run died: {e}")))?;
        match ran {
            Ok(()) => Ok(built),
            Err(message) => {
                let mut diags = vec![Diag::general(message)];
                diags.extend(built.warnings);
                Err(CompileError::Failed(diags))
            }
        }
    }

    fn write_crate(&self, dir: &Path, files: &Files) -> std::io::Result<()> {
        std::fs::create_dir_all(dir.join("src"))?;
        std::fs::write(dir.join("Cargo.toml"), manifest(&self.config.sdk_dir))?;
        std::fs::write(dir.join("clippy.toml"), determinism::clippy_toml())?;
        for (name, source) in files {
            // names were checked (files::check): [a-z0-9_]+.rs, so nothing leaves src/
            std::fs::write(dir.join("src").join(name), source)?;
        }
        if let Some(lock) = &self.config.lock_file {
            std::fs::copy(lock, dir.join("Cargo.lock"))?;
        }
        Ok(())
    }

    /// `cargo <build|clippy> --offline --release --target wasm32-unknown-unknown`, in a clean environment of our own making
    fn cargo(&self, dir: &Path, subcommand: &str) -> Command {
        let mut cmd = Command::new(&self.config.cargo);
        cmd.args([subcommand, "--offline", "--release", "--target", TARGET, "--message-format=json", "--color", "never"]).current_dir(dir);
        // nothing of the service's environment steers the build (and the secret never reaches a child)
        for var in ["RUSTFLAGS", "CARGO_BUILD_RUSTFLAGS", "CARGO_BUILD_TARGET", "CARGO_BUILD_RUSTC", "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTDOC", "CLIPPY_CONF_DIR", "CLIPPY_ARGS", "MODULE_BUILD_TOKEN"] {
            cmd.env_remove(var);
        }
        // the pinned flags (determinism.rs): cargo reads this one before any other place flags can come from
        cmd.env("CARGO_ENCODED_RUSTFLAGS", determinism::encoded_rustflags());
        cmd.env("CARGO_BUILD_JOBS", self.config.jobs.to_string())
            .env("CARGO_TARGET_DIR", &self.config.target_dir)
            .env("CARGO_NET_OFFLINE", "true")
            .env("CARGO_INCREMENTAL", "0")
            .env("CARGO_TERM_COLOR", "never");
        if let Some(home) = &self.config.cargo_home {
            cmd.env("CARGO_HOME", home);
        }
        #[cfg(unix)]
        cmd.env("TMPDIR", dir);
        cmd
    }

    async fn interpret(&self, outcome: Outcome, unlexable: bool) -> Result<Built, CompileError> {
        match outcome.killed {
            Some(Killed::Vetoed) => return Err(CompileError::LiveShow),
            Some(Killed::Timeout) => return Err(CompileError::Failed(vec![Diag::general(format!("the compile took longer than {} s and was stopped", self.config.wall.as_secs()))])),
            Some(Killed::Memory) => return Err(CompileError::Failed(vec![Diag::general(format!("the compile used more than {} MB of memory and was stopped", self.config.memory_mb))])),
            Some(Killed::Output) => return Err(CompileError::Failed(vec![Diag::general("the compiler printed more than it may and was stopped")])),
            None => {}
        }
        let diags = parse_cargo_json(&outcome.stdout);
        if !outcome.success() {
            if diags.iter().any(|d| d.level == "error") {
                return Err(CompileError::Failed(diags));
            }
            let tail: String = outcome.stderr.chars().rev().take(2000).collect::<Vec<_>>().into_iter().rev().collect();
            return Err(CompileError::Internal(format!("cargo failed without a diagnostic: {tail}")));
        }
        if unlexable {
            // rustc accepted a file our lexer could not read: not something we can vouch for
            return Err(CompileError::Rejected(vec![Diag::general("a source file could not be checked (its tokens are unreadable) – the module is refused")]));
        }
        let wasm_path = self.config.target_dir.join(TARGET).join("release").join(format!("{PACKAGE}.wasm"));
        let wasm = std::fs::read(&wasm_path).map_err(|e| CompileError::Internal(format!("the wasm is missing at {}: {e}", wasm_path.display())))?;
        if wasm.len() > self.config.max_wasm {
            return Err(CompileError::Failed(vec![Diag::general(format!("the module is {} bytes, at most {} are allowed", wasm.len(), self.config.max_wasm))]));
        }
        let descriptor = describe(&wasm).map_err(|e| CompileError::Failed(vec![Diag::general(e)]))?;
        Ok(Built { hash: hex(&Sha256::digest(&wasm)), descriptor, wasm, warnings: diags.into_iter().filter(|d| d.level == "warning").collect() })
    }

    /// The shared build folder keeps the SDK's artifacts (that is the point); the member's own go, so no compile leaves
    /// anything for the next and the folder does not grow
    fn clean_member_artifacts(&self) {
        let release = self.config.target_dir.join(TARGET).join("release");
        for (folder, prefix) in [(release.clone(), format!("{PACKAGE}.")), (release.join("deps"), format!("{PACKAGE}-")), (release.join("deps"), format!("lib{PACKAGE}-")), (release.join(".fingerprint"), format!("{PACKAGE}-")), (release.join("incremental"), format!("{PACKAGE}-"))] {
            let Ok(entries) = std::fs::read_dir(&folder) else { continue };
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with(&prefix) {
                    let path: PathBuf = entry.path();
                    let _ = if path.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
                }
            }
        }
    }
}

/// The module's descriptor, read from the wasm's `deck.module` section and validated (ABI version, names, ranges,
/// counts); handed on as the module wrote it (the stored form the server checks, ModuleSource.php)
pub fn describe(wasm: &[u8]) -> Result<serde_json::Value, String> {
    let json = deck_module::host::read_descriptor(wasm).ok_or("the wasm has no module descriptor: derive `Module` on a struct (`#[derive(Module)]`)")?;
    deck_module::host::parse(json)?;
    serde_json::from_str(json).map_err(|e| e.to_string())
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_is_fixed() {
        let m = manifest(Path::new("/opt/sdk"));
        assert!(m.contains("deck-module = { path = '/opt/sdk/module' }"));
        assert!(m.contains("crate-type = [\"cdylib\"]"));
        assert!(m.contains("build = false"));
        assert!(m.contains("panic = \"abort\""));
        // one dependency, nothing a member could add
        let parsed: toml_like::Doc = toml_like::Doc::of(&m);
        assert_eq!(parsed.section("dependencies"), ["deck-module = { path = '/opt/sdk/module' }"]);
        // a Windows path stays a literal TOML string
        assert!(manifest(Path::new("C:\\sdk")).contains("path = 'C:/sdk/module'"));
    }

    /// the lines under a `[section]` (enough TOML for the test above)
    mod toml_like {
        pub struct Doc(Vec<(String, Vec<String>)>);
        impl Doc {
            pub fn of(text: &str) -> Doc {
                let mut sections: Vec<(String, Vec<String>)> = vec![(String::new(), vec![])];
                for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
                    if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                        sections.push((name.to_string(), vec![]));
                    } else {
                        sections.last_mut().unwrap().1.push(line.to_string());
                    }
                }
                Doc(sections)
            }
            pub fn section(&self, name: &str) -> Vec<&str> {
                self.0.iter().filter(|(n, _)| n == name).flat_map(|(_, l)| l.iter().map(String::as_str)).collect()
            }
        }
    }

    /// the compiler flags are ours: whatever flags the service's environment carries, cargo gets the pinned ones
    #[test]
    fn cargo_runs_with_the_pinned_flags_and_the_environments_are_dropped() {
        let env = |k: &str| match k {
            "MODULE_BUILD_TOKEN" => Some("t".to_string()),
            _ => None,
        };
        let compiler = Compiler::new(Arc::new(Config::from_env(env).unwrap()));
        for subcommand in ["build", "clippy"] {
            let cmd = compiler.cargo(Path::new("/w/member"), subcommand);
            let cmd = cmd.as_std();
            let envs: std::collections::HashMap<_, _> = cmd.get_envs().map(|(k, v)| (k.to_string_lossy().to_string(), v.map(|v| v.to_string_lossy().to_string()))).collect();
            assert_eq!(envs["CARGO_ENCODED_RUSTFLAGS"].as_deref(), Some(determinism::encoded_rustflags().as_str()));
            for gone in ["RUSTFLAGS", "CARGO_BUILD_RUSTFLAGS", "RUSTC_WRAPPER", "CLIPPY_CONF_DIR", "CLIPPY_ARGS", "MODULE_BUILD_TOKEN"] {
                assert_eq!(envs[gone], None, "{gone} is removed from the child's environment");
            }
            let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy().to_string()).collect();
            assert_eq!(args[0], subcommand);
            assert!(args.contains(&"--offline".to_string()) && args.contains(&"--release".to_string()) && args.contains(&TARGET.to_string()));
        }
    }

    #[test]
    fn hex_is_lowercase() {
        assert_eq!(hex(&[0, 15, 16, 255]), "000f10ff");
    }

    #[test]
    fn a_wasm_without_a_descriptor_says_so() {
        assert!(describe(b"\0asm\x01\0\0\0").unwrap_err().contains("derive `Module`"));
        assert!(describe(b"not wasm").is_err());
    }
}
