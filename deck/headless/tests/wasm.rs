//! The headless deck host as wasm (#363): deck-headless built for wasm32 and wasm32 + simd128 (the cdylib's `hl_*`
//! exports), every scenario run in wasmtime – the report (audio hashes, beats, lines, quality levels, the messages that
//! left) is byte for byte the native one. What the browser runs is that core; its bridge adds nothing a scenario can see.
//! The wasm files are built into the conformance suite's target dirs (`deck/target/conformance-wasm*`), once per change.
mod common;
use std::path::PathBuf;
use std::process::Command;
use wasmtime::{Engine, Instance, Module, Store};

fn build(simd: bool) -> PathBuf {
    let dir = common::deck_dir().join("target").join(if simd { "conformance-wasm-simd" } else { "conformance-wasm" });
    let mut cmd = Command::new("cargo");
    cmd.current_dir(common::deck_dir()).args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "deck-headless", "--lib", "--target-dir"]).arg(&dir);
    if simd {
        cmd.env("RUSTFLAGS", "-C target-feature=+simd128");
    } else {
        cmd.env_remove("RUSTFLAGS");
    }
    let out = cmd.output().expect("cargo runs");
    assert!(out.status.success(), "the wasm build failed:\n{}", String::from_utf8_lossy(&out.stderr));
    dir.join("wasm32-unknown-unknown").join("release").join("deck_headless.wasm")
}

/// one bundle through the module's `hl_run` → the report's JSON
fn run_wasm(module: &Module, engine: &Engine, bundle: &[u8]) -> String {
    let mut store = Store::new(engine, ());
    let inst = Instance::new(&mut store, module, &[]).expect("no imports");
    let inbox = inst.get_typed_func::<u32, u32>(&mut store, "hl_inbox").unwrap();
    let run = inst.get_typed_func::<u32, i32>(&mut store, "hl_run").unwrap();
    let outbox = inst.get_typed_func::<(), u32>(&mut store, "hl_outbox").unwrap();
    let at = inbox.call(&mut store, bundle.len() as u32).unwrap() as usize;
    let mem = inst.get_memory(&mut store, "memory").unwrap();
    mem.data_mut(&mut store)[at..at + bundle.len()].copy_from_slice(bundle);
    let n = run.call(&mut store, bundle.len() as u32).unwrap();
    let ptr = outbox.call(&mut store, ()).unwrap() as usize;
    let text = String::from_utf8(mem.data(&store)[ptr..ptr + n.unsigned_abs() as usize].to_vec()).unwrap();
    assert!(n > 0, "hl_run: {text}");
    text
}

/// the fields two reports differ in: step, key, native → wasm
fn diff(native: &str, wasm: &str) -> String {
    let (a, b): (serde_json::Value, serde_json::Value) = (serde_json::from_str(native).unwrap(), serde_json::from_str(wasm).unwrap());
    let mut out = Vec::new();
    if a["status"] != b["status"] {
        out.push(format!("status {} → {} ({})", a["status"], b["status"], b["reason"]));
    }
    for (i, (x, y)) in a["steps"].as_array().unwrap().iter().zip(b["steps"].as_array().unwrap()).enumerate() {
        for (k, v) in x["out"].as_object().into_iter().flatten() {
            if y["out"].get(k) != Some(v) {
                out.push(format!("step {} {}: {k} {v} → {}", i + 1, x["do"], y["out"].get(k).unwrap_or(&serde_json::Value::Null)));
            }
        }
    }
    out.join("\n")
}

/// Scenarios whose audio differs between the targets today, with the ticket that makes them bit-exact: only their
/// `audio` hashes may differ (every other field – beats, lines, levels, messages – must still be the same). One that
/// matches again fails the test until its line goes (an expected difference that is gone).
const KNOWN_DIVERGENT: &[(&str, &str)] = &[
    // the cathedral's fft.bass → drone.gain feeds the FFT's values into the sound; those values differ native vs wasm
    ("01-cathedral-boot", "#381"),
];

#[test]
fn every_scenario_reports_the_same_native_and_in_wasm32_with_and_without_simd128() {
    let bundles = common::bundles();
    let native: Vec<String> = bundles.iter().map(|b| serde_json::to_string(&deck_headless::run(b)).unwrap()).collect();
    let engine = Engine::default();
    for simd in [false, true] {
        let target = if simd { "wasm32 + simd128" } else { "wasm32" };
        let module = Module::from_file(&engine, build(simd)).expect("the module compiles");
        let mut wrong = Vec::new();
        let mut same = 0;
        for (b, want) in bundles.iter().zip(&native) {
            let got = run_wasm(&module, &engine, &serde_json::to_vec(b).unwrap());
            let known = KNOWN_DIVERGENT.iter().find(|(n, _)| *n == b.name);
            let d = diff(want, &got);
            match known {
                None if &got != want => wrong.push(format!("{} differs in {target}:\n{d}", b.name)),
                None => same += 1,
                Some((_, t)) if &got == want => wrong.push(format!("{} is the same in {target} now – {t} is done: remove its KNOWN_DIVERGENT line", b.name)),
                Some((_, t)) if d.lines().any(|l| !l.contains(": audio ")) => wrong.push(format!("{} ({t}) may differ only in its audio hash in {target}:\n{d}", b.name)),
                Some((_, t)) => println!("{target}: {} differs only in its audio ({t})", b.name),
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
        println!("{target}: {same} of {} scenarios report exactly what native does", bundles.len());
    }
}
