//! A local build (`local::Local`, the editor's) against the real toolchain: the walk module builds, a default changed in its
//! code reaches the descriptor, a broken line comes back as rustc's problem at its place. Needs the wasm32 target and the
//! SDK's dependencies in cargo's cache – skipped with a message otherwise.
use std::path::Path;

use scutoons_build::files::Files;
use scutoons_build::local::Local;

fn deck() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn wasm_target_present() -> bool {
    let Ok(out) = std::process::Command::new("rustc").args(["--print", "sysroot"]).output() else { return false };
    Path::new(String::from_utf8_lossy(&out.stdout).trim()).join("lib/rustlib/wasm32-unknown-unknown/lib").is_dir()
}

fn default_of(descriptor: &serde_json::Value, param: &str) -> f64 {
    descriptor["params"].as_array().unwrap().iter().find(|p| p["name"] == param).unwrap()["default"].as_f64().unwrap()
}

#[test]
fn a_changed_default_builds_locally_and_a_broken_line_is_a_problem() {
    if !wasm_target_present() {
        eprintln!("SKIPPED: no wasm32-unknown-unknown target here");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let local = Local::new(deck(), root.path()).unwrap();
    let walk = std::fs::read_to_string(deck().join("modules/walk/src/lib.rs")).unwrap();
    let built = match local.build(&Files::from([("lib.rs".to_string(), walk.clone())])) {
        Ok(b) => b,
        Err(p) if p.iter().any(|d| d.message.contains("offline") || d.message.contains("failed to get")) => {
            eprintln!("SKIPPED: the SDK's dependencies are not in cargo's cache: {p:?}");
            return;
        }
        Err(p) => panic!("walk does not build locally: {p:?}"),
    };
    assert_eq!(built.descriptor["name"], "walk");
    assert_eq!(default_of(&built.descriptor, "glide"), 0.5);

    let changed = walk.replacen("range = 0.0..1.0, default = 0.5", "range = 0.0..1.0, default = 0.25", 1);
    assert_ne!(changed, walk, "the walk's glide default is in its source");
    let again = local.build(&Files::from([("lib.rs".to_string(), changed.clone())])).expect("the changed walk builds");
    assert_eq!(default_of(&again.descriptor, "glide"), 0.25);
    assert_ne!(again.hash, built.hash);

    let broken = changed.replacen("impl Module for", "impl Module fr", 1);
    let problems = local.build(&Files::from([("lib.rs".to_string(), broken)])).unwrap_err();
    let first = problems.iter().find(|d| d.level == "error").expect("an error");
    assert_eq!(first.file, "lib.rs");
    assert!(first.line > 0, "{first:?}");
}
