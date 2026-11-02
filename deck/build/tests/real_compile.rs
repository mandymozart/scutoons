//! The service against a real toolchain: the walk module's source compiles to a wasm whose descriptor is read back, a
//! broken source gives rustc's errors with their lines, the rejects stop before rustc runs, and the whole thing works over
//! HTTP behind the token. Needs `rustc` with the wasm32-unknown-unknown target and the SDK's dependencies in cargo's
//! cache (any machine that built the deck workspace) – skipped with a message otherwise. One test, one build folder: cargo
//! builds one thing at a time there, as the service does.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use scutoons_build::compile::{describe, CompileError, Compiler};
use scutoons_build::config::Config;
use scutoons_build::files::Files;
use scutoons_build::service::{serve, Service};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn deck() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// does the toolchain build wasm32-unknown-unknown?
fn wasm_target_present() -> bool {
    let Ok(out) = std::process::Command::new("rustc").args(["--print", "sysroot"]).output() else { return false };
    let sysroot = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Path::new(&sysroot).join("lib/rustlib/wasm32-unknown-unknown/lib").is_dir()
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// the SDK as the image bakes it: deck/module/src + deck/module-macros/src with the standalone manifests
fn bake_sdk(root: &Path) -> PathBuf {
    let sdk = root.join("sdk");
    let deck = deck();
    copy_dir(&deck.join("module/src"), &sdk.join("module/src"));
    copy_dir(&deck.join("module-macros/src"), &sdk.join("module-macros/src"));
    std::fs::copy(deck.join("build/sdk/module.Cargo.toml"), sdk.join("module/Cargo.toml")).unwrap();
    std::fs::copy(deck.join("build/sdk/macros.Cargo.toml"), sdk.join("module-macros/Cargo.toml")).unwrap();
    sdk
}

fn config(root: &Path, sdk: &Path, extra: &[(&str, &str)]) -> Arc<Config> {
    let lock = deck().join("Cargo.lock");
    let pairs: Vec<(String, String)> = [
        ("MODULE_BUILD_TOKEN", "sesame".to_string()),
        ("BUILD_SDK_DIR", sdk.display().to_string()),
        ("BUILD_WORK_DIR", root.join("work").display().to_string()),
        ("BUILD_TARGET_DIR", root.join("target").display().to_string()),
        ("BUILD_LOCK", lock.display().to_string()),
        ("BUILD_WALL_SECS", "900".to_string()),
        ("BUILD_MEMORY_MB", "0".to_string()),
        ("RELAY_STATUS_URL", "off".to_string()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .chain(extra.iter().map(|(k, v)| (k.to_string(), v.to_string())))
    .collect();
    Arc::new(Config::from_env(|k| pairs.iter().rev().find(|(n, _)| n == k).map(|(_, v)| v.clone())).unwrap())
}

fn files(lib: &str) -> Files {
    Files::from([("lib.rs".to_string(), lib.to_string())])
}

async fn post(addr: std::net::SocketAddr, token: Option<&str>, body: &str) -> (u16, serde_json::Value) {
    let mut s = TcpStream::connect(addr).await.unwrap();
    let auth = token.map(|t| format!("X-Build-Token: {t}\r\n")).unwrap_or_default();
    s.write_all(format!("POST /compile HTTP/1.1\r\nHost: x\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
    let mut raw = String::new();
    s.read_to_string(&mut raw).await.unwrap();
    let (head, json) = raw.split_once("\r\n\r\n").unwrap();
    (head.split(' ').nth(1).unwrap().parse().unwrap(), serde_json::from_str(json).unwrap())
}

#[tokio::test]
async fn the_walk_module_compiles_and_the_rest_is_refused() {
    if !wasm_target_present() {
        eprintln!("SKIPPED: no rustc with the wasm32-unknown-unknown target here (rustup target add wasm32-unknown-unknown)");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let sdk = bake_sdk(root.path());
    let compiler = Arc::new(Compiler::new(config(root.path(), &sdk, &[])));
    let walk = std::fs::read_to_string(deck().join("modules/walk/src/lib.rs")).unwrap();

    // 1. the walk module
    let built = match compiler.compile(&files(&walk)).await {
        Ok(b) => b,
        Err(CompileError::Internal(m)) if m.contains("offline") || m.contains("no matching package") || m.contains("failed to get") => {
            eprintln!("SKIPPED: the SDK's dependencies are not in cargo's cache here: {m}");
            return;
        }
        Err(e) => panic!("walk does not compile: {e:?}"),
    };
    assert!(built.wasm.starts_with(b"\0asm"));
    assert_eq!(built.hash, scutoons_build::compile::hex(&Sha256::digest(&built.wasm)));
    assert_eq!(built.descriptor["name"], "walk");
    assert_eq!(built.descriptor["abi"], 1);
    let params: Vec<&str> = built.descriptor["params"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(params, ["spread", "rate", "glide", "base", "octaves", "length"]);
    assert_eq!(describe(&built.wasm).unwrap(), built.descriptor, "the descriptor is the one in the wasm");
    // the member's artifacts do not stay behind; the work folder is empty again
    assert_eq!(std::fs::read_dir(root.path().join("work")).unwrap().count(), 0);
    let release = root.path().join("target/wasm32-unknown-unknown/release");
    assert!(!release.join("member.wasm").exists());

    // 2. the same sources, the same hash (the SDK's dependencies are built already: this is the fast path)
    let again = compiler.compile(&files(&walk)).await.unwrap();
    assert_eq!(again.hash, built.hash, "same source + toolchain = the same file");

    // 3. rustc's errors, with their places
    let broken = "use deck_module::Module;\n\n#[derive(Module)]\nstruct M {}\n\nfn f() -> u32 {\n    undefined_thing\n}\n";
    let CompileError::Failed(errors) = compiler.compile(&files(broken)).await.unwrap_err() else { panic!("expected rustc's errors") };
    let e = errors.iter().find(|d| d.message.contains("undefined_thing")).unwrap_or_else(|| panic!("{errors:?}"));
    assert_eq!((e.file.as_str(), e.line, e.column, e.level.as_str()), ("lib.rs", 7, 5, "error"));

    // 4. no descriptor: it compiles but is no module
    let CompileError::Failed(errors) = compiler.compile(&files("pub fn x() {}\n")).await.unwrap_err() else { panic!("expected the missing descriptor") };
    assert!(errors[0].message.contains("derive `Module`"), "{errors:?}");

    // 5. the rejects stop before rustc runs (a failing compile would say something else)
    for (source, part) in [
        ("const X: &str = include_str!(\"/etc/passwd\");", "include_str!"),
        ("extern crate alloc;", "extern crate"),
        ("#[path = \"x.rs\"]\nmod a;", "path"),
    ] {
        let CompileError::Rejected(found) = compiler.compile(&files(source)).await.unwrap_err() else { panic!("{source}") };
        assert!(found[0].message.contains(part) && found[0].file == "lib.rs", "{found:?}");
    }
    // a second file the way a module splits itself
    let two = Files::from([("lib.rs".to_string(), walk.replace("use deck_module::cv::GateCv;", "mod nothing;\nuse deck_module::cv::GateCv;")), ("nothing.rs".to_string(), "// empty\n".to_string())]);
    assert_eq!(compiler.compile(&two).await.unwrap().descriptor["name"], "walk");

    // 6. over HTTP behind the token
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let service = Arc::new(Service::new(config(root.path(), &sdk, &[]), compiler.clone()));
    let server = tokio::spawn(serve(listener, service));
    let body = serde_json::json!({ "files": { "lib.rs": walk } }).to_string();
    assert_eq!(post(addr, None, &body).await.0, 401);
    assert_eq!(post(addr, Some("wrong"), &body).await.0, 401);
    let (status, answer) = post(addr, Some("sesame"), &body).await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["hash"], built.hash.as_str());
    assert_eq!(answer["descriptor"]["name"], "walk");
    use base64::Engine;
    assert_eq!(base64::engine::general_purpose::STANDARD.decode(answer["wasm_b64"].as_str().unwrap()).unwrap(), built.wasm);
    assert!(answer["toolchain"].as_str().unwrap().starts_with("rustc "));
    let (status, answer) = post(addr, Some("sesame"), &serde_json::json!({ "files": { "lib.rs": "fn f( {" } }).to_string()).await;
    assert_eq!(status, 422);
    assert_eq!(answer["errors"][0]["file"], "lib.rs");
    assert!(answer["errors"][0]["line"].as_u64().unwrap() >= 1);
    assert_eq!(post(addr, Some("sesame"), "{}").await.0, 422);
    server.abort();
}

#[tokio::test]
async fn a_compile_past_the_wall_clock_is_stopped() {
    if !wasm_target_present() {
        eprintln!("SKIPPED: no wasm32-unknown-unknown target");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let sdk = bake_sdk(root.path());
    // one second is less than cargo needs to start and resolve; whatever it was doing dies with it
    let compiler = Compiler::new(config(root.path(), &sdk, &[("BUILD_WALL_SECS", "1")]));
    let started = std::time::Instant::now();
    match compiler.compile(&files("// nothing\n")).await {
        Err(CompileError::Failed(e)) => assert!(e[0].message.contains("took longer than 1 s"), "{e:?}"),
        other => panic!("expected the wall clock to stop it, got {other:?}"),
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    assert_eq!(std::fs::read_dir(root.path().join("work")).unwrap().count(), 0, "the temporary folder is gone");
}
