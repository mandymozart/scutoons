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

/// A one-port module whose `process` runs `body` for every sample `s` of the output (the shape of the stub the image primes with)
fn probe(body: &str) -> String {
    format!(
        "use deck_module::{{Block, Ctx, Module}};

#[derive(Module)]
#[module(name = \"probe\")]
#[port(output, audio, name = \"out\")]
struct Probe {{
    #[param(range = 0.0..1.0, default = 0.5)]
    level: f32,
}}

impl Module for Probe {{
    fn process(&mut self, block: &mut Block, _ctx: &Ctx) {{
        let mut out = block.audio_out(0);
        for c in 0..out.ch() {{
            for s in out.chan(c).iter_mut() {{
                {body}
            }}
        }}
    }}
}}
"
    )
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
    assert_eq!(built.descriptor["abi"], 2, "module ABI v2 (#343)");
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

    // 5a. the template: the stub the image primes with (`scutoons-build stub`) is a v2 module, builds clean and plays strictly
    let stub = compiler.compile(&files(scutoons_build::compile::STUB_SOURCE)).await.unwrap_or_else(|e| panic!("the stub does not compile: {e:?}"));
    assert_eq!((stub.descriptor["abi"].as_u64(), stub.descriptor["name"].as_str(), stub.warnings.len()), (Some(2), Some("stub"), 0));

    // 5b. the determinism rules (#344, ABI-V2 §6): the stored modules of the repository compile, play strictly, and say nothing
    // about std's float functions (walk and analog use libm); the conformance fixture is a module like any other
    assert!(built.warnings.iter().all(|w| !w.message.contains("disallowed")), "walk uses deck_module::math: {:?}", built.warnings);
    let fixture = std::fs::read_to_string(deck().join("modules/fixture/src/lib.rs")).unwrap();
    let fixture_built = compiler.compile(&files(&fixture)).await.unwrap_or_else(|e| panic!("the fixture does not compile or play: {e:?}"));
    assert_eq!(fixture_built.descriptor["name"], "fixture");

    // a module calling std's float transcendentals: built, with a warning for each call that says what to call instead
    let sin = probe("*s = self.level.sin() + (self.level as f64).powf(2.0) as f32;");
    let built_sin = compiler.compile(&files(&sin)).await.unwrap();
    let lints: Vec<_> = built_sin.warnings.iter().filter(|w| w.message.contains("disallowed method")).collect();
    assert_eq!(lints.len(), 2, "{:?}", built_sin.warnings);
    assert_eq!((lints[0].file.as_str(), lints[0].line, lints[0].level.as_str()), ("lib.rs", 16, "warning"), "{lints:?}");
    assert!(lints[0].message.contains("`f32::sin`") && lints[0].message.contains("use deck_module::math::sin"), "{lints:?}");
    assert!(lints[1].message.contains("`f64::powf`") && lints[1].message.contains("deck_module::math::libm::pow"), "{lints:?}");
    // the same through deck_module::math: nothing to say
    let libm = compiler.compile(&files(&probe("*s = deck_module::math::sin(self.level) + deck_module::math::pow(self.level, 2.0);"))).await.unwrap();
    assert!(libm.warnings.is_empty(), "{:?}", libm.warnings);

    // the strict test run: an allocation inside process is an error that says so; so are a panic and a runaway
    let CompileError::Failed(errors) = compiler.compile(&files(&probe("core::hint::black_box(vec![1u8; 16]); *s = 0.0;"))).await.unwrap_err() else { panic!("expected the strict run to stop it") };
    assert!(errors[0].level == "error" && errors[0].message.contains("allocated inside process / control") && errors[0].message.contains("allocate in init"), "{errors:?}");
    let CompileError::Failed(errors) = compiler.compile(&files(&probe("if self.level > 0.1 { panic!(\"boom\"); } *s = 0.0;"))).await.unwrap_err() else { panic!("expected the strict run to stop it") };
    assert!(errors[0].message.contains("strict test run") && errors[0].message.contains("boom") && !errors[0].message.contains("allocated"), "{errors:?}");
    let CompileError::Failed(errors) = compiler.compile(&files(&probe("loop { core::hint::black_box(0); }"))).await.unwrap_err() else { panic!("expected the strict run to stop it") };
    assert!(errors[0].message.contains("time budget") && errors[0].message.contains("too much work for one block"), "{errors:?}");
    // relaxed SIMD (its results are the CPU's) compiles when forced per function, and the deck's host refuses it at load
    let relaxed = format!(
        "{}\n#[target_feature(enable = \"relaxed-simd\")]\nfn relaxed(x: f32) -> f32 {{\n    use core::arch::wasm32::*;\n    let v = f32x4_splat(x);\n    f32x4_extract_lane::<0>(f32x4_relaxed_madd(v, v, v))\n}}\n",
        probe("*s = unsafe { relaxed(self.level) };")
    );
    let relaxed_result = compiler.compile(&files(&relaxed)).await;
    let CompileError::Failed(errors) = relaxed_result.unwrap_err() else { panic!("expected the strict run to refuse it") };
    assert!(errors[0].message.contains("the module does not load"), "{errors:?}");
    // an error brings the warnings along (the creator fixes both in one go)
    let CompileError::Failed(errors) = compiler.compile(&files(&probe("*s = self.level.cos(); core::hint::black_box(vec![1u8; 16]);"))).await.unwrap_err() else { panic!("expected the strict run to stop it") };
    assert_eq!(errors.iter().map(|d| d.level.as_str()).collect::<Vec<_>>(), ["error", "warning"], "{errors:?}");

    // 5c. rebuild over a folder (`scutoons-build rebuild`): the sources of the repository's modules as a cargo layout, the
    // old API as a single file; the failures are the list
    let folder = root.path().join("modules");
    copy_dir(&deck().join("modules/walk/src"), &folder.join("walk/src"));
    copy_dir(&deck().join("modules/fixture/src"), &folder.join("fixture/src"));
    // analog is flat files since #350 (the catalog's factory module:analog stores them): it builds and plays strictly too
    copy_dir(&deck().join("modules/analog/src"), &folder.join("analog/src"));
    std::fs::write(folder.join("old.rs"), "use deck_module::rt::Instance;\n").unwrap();
    let rows = scutoons_build::rebuild::rebuild_all(&compiler, scutoons_build::rebuild::read_modules(&folder).unwrap()).await;
    let (text, failed) = scutoons_build::rebuild::report(&rows);
    assert_eq!(failed, 1, "{text}");
    assert!(text.contains("fixture\tOK\tABI 2") && text.contains("walk\tOK\tABI 2"), "{text}");
    assert!(text.contains("old\tFAILED\n    lib.rs:1:"), "{text}");
    assert!(text.ends_with("rebuild: 4 module(s), 1 failed: old\n"), "{text}");

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

    // 7. the factory modules as the API sends them (#350, #416: ModuleCompiler posts the catalog entry's flat `files`): every
    // answer's descriptor uses only the fields and values the API's ModuleSource::descriptorProblems knows – a field the core
    // adds to the descriptor must be added there too, or the compile answers 502. With CAPTURE_SERVICE_ANSWERS=1 the answers
    // are written to server/tests/fixtures/modules/service-answer-<id>.json (the API's regression fixtures).
    let php = std::fs::read_to_string(deck().join("../server/src/Domain/Modules/ModuleSource.php")).unwrap();
    for id in ["walk", "fixture", "analog"] {
        let mut files = serde_json::Map::new();
        for entry in std::fs::read_dir(deck().join(format!("modules/{id}/src"))).unwrap().flatten() {
            files.insert(entry.file_name().to_string_lossy().into_owned(), std::fs::read_to_string(entry.path()).unwrap().into());
        }
        let (status, answer) = post(addr, Some("sesame"), &serde_json::json!({ "files": files }).to_string()).await;
        assert_eq!(status, 200, "{id}: {answer}");
        let problems = api_descriptor_problems(&php, &answer["descriptor"]);
        assert!(problems.is_empty(), "{id}: the API would refuse this descriptor (ModuleSource.php): {problems:?}");
        if std::env::var_os("CAPTURE_SERVICE_ANSWERS").is_some() {
            let out = deck().join(format!("../server/tests/fixtures/modules/service-answer-{id}.json"));
            std::fs::create_dir_all(out.parent().unwrap()).unwrap();
            std::fs::write(out, serde_json::to_string_pretty(&answer).unwrap() + "\n").unwrap();
        }
    }
    server.abort();
}

/// the quoted names of `const <name> = [...]` in ModuleSource.php
fn php_list(php: &str, name: &str) -> Vec<String> {
    let start = php.find(&format!("const {name} = [")).unwrap_or_else(|| panic!("ModuleSource.php has no {name}")) + name.len() + 10;
    let list = &php[start..start + php[start..].find(']').unwrap()];
    list.split(',').map(|s| s.trim().trim_matches('\'').to_string()).filter(|s| !s.is_empty()).collect()
}

/// The keys and values of a service descriptor that ModuleSource.php does not know (its FIELDS, PARAM_FIELDS, PORT_FIELDS,
/// DIRS, TYPES and ABI)
fn api_descriptor_problems(php: &str, d: &serde_json::Value) -> Vec<String> {
    let unknown = |v: &serde_json::Value, list: &str, at: &str| -> Vec<String> {
        let known = php_list(php, list);
        v.as_object().map(|o| o.keys().filter(|k| !known.contains(k)).map(|k| format!("{at}.{k} (not in {list})")).collect()).unwrap_or_else(|| vec![format!("{at} is no object")])
    };
    let mut p = unknown(d, "FIELDS", "descriptor");
    let abi = php.split("const ABI = ").nth(1).and_then(|s| s.split(';').next()).unwrap().trim().to_string();
    if d["abi"].as_u64().map(|n| n.to_string()).as_deref() != Some(abi.as_str()) {
        p.push(format!("descriptor.abi {} (the API expects {abi})", d["abi"]));
    }
    for (i, param) in d["params"].as_array().into_iter().flatten().enumerate() {
        p.extend(unknown(param, "PARAM_FIELDS", &format!("params[{i}]")));
        for key in ["min", "max", "default"] {
            if !param[key].is_number() {
                p.push(format!("params[{i}].{key} = {} is no number", param[key]));
            }
        }
    }
    let (dirs, types) = (php_list(php, "DIRS"), php_list(php, "TYPES"));
    for (i, port) in d["ports"].as_array().into_iter().flatten().enumerate() {
        p.extend(unknown(port, "PORT_FIELDS", &format!("ports[{i}]")));
        for (key, known) in [("dir", &dirs), ("type", &types)] {
            if !port[key].as_str().is_some_and(|v| known.iter().any(|k| k == v)) {
                p.push(format!("ports[{i}].{key} = {}", port[key]));
            }
        }
    }
    p
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
