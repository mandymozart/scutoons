//! Native ≡ wasm (#343): the module built for `wasm32-unknown-unknown` and run by the module host plays, bit for bit,
//! what the same code plays natively through the SDK's `Rig` – the notes and the cv of `fixtures/walk.json` (written by
//! tests/builtin.rs). Generic over the host's `Backend`, so the browser's backend proves itself by adding one line.
use deck_core::tempo::TempoMap;
use deck_module::QUANTUM;
use deck_module_host::{Backend, Instance, Loaded, Setup};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

fn wasm() -> Vec<u8> {
    static BUILD: OnceLock<()> = OnceLock::new();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_owned();
    let dir = root.join("target").join("wasm-modules");
    BUILD.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let out = std::process::Command::new(cargo)
            .current_dir(&root)
            .args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "walk-module", "--target-dir"])
            .arg(&dir)
            .output()
            .expect("running cargo");
        assert!(out.status.success(), "building the walk module for wasm32 failed (rustup target add wasm32-unknown-unknown?):\n{}", String::from_utf8_lossy(&out.stderr));
    });
    std::fs::read(dir.join("wasm32-unknown-unknown/release/walk_module.wasm")).unwrap()
}

fn the_wasm_plays_the_native_fixture<B: Backend>(e: &B::Engine) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/walk.json");
    let fixture: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let (rate, block, blocks) = (fixture["rate"].as_f64().unwrap(), fixture["block"].as_u64().unwrap() as usize, fixture["blocks"].as_u64().unwrap() as usize);
    let loaded = Arc::new(Loaded::<B>::load(e, &wasm()).expect("the walk module loads"));
    assert_eq!(loaded.desc.name, "walk");
    let mut i = Instance::new(&loaded, Setup::new(rate, fixture["seed"].as_u64().unwrap()).unbudgeted()).unwrap();
    for (name, v) in fixture["params"].as_object().unwrap() {
        i.set_param(loaded.param_index(name).unwrap(), v.as_f64().unwrap() as f32);
    }
    i.set_tempo(&TempoMap::new(fixture["bpm"].as_f64().unwrap(), 4, rate));
    let (mut notes, mut cv) = (Vec::new(), Vec::new());
    for b in 0..blocks {
        let (mut l, mut r) = (Vec::new(), Vec::new());
        for c in 0..block / QUANTUM {
            let at = c * QUANTUM;
            assert!(i.process((b * block + at) as f64, QUANTUM as u32), "{:?}", i.silenced());
            for n in i.events_out(0) {
                notes.push(serde_json::json!({ "block": b, "frame": at as u32 + n.frame, "pitch": n.pitch_hz(), "len": n.len, "glide": n.glide() }));
            }
            let out = i.audio_out(0);
            l.extend_from_slice(out.chan(0));
            r.extend_from_slice(out.chan(1));
        }
        cv.push(serde_json::json!([l[0], l[l.len() - 1], r.iter().sum::<f32>()]));
    }
    // compared as the f32 bits they are (a JSON number read back may sit an f64 ulp off)
    assert_eq!(bits(&serde_json::Value::from(notes)), bits(&fixture["notes"]), "the notes, wasm against native");
    assert_eq!(bits(&serde_json::Value::from(cv)), bits(&fixture["cv"]), "the cv, wasm against native");
}

/// every number of a JSON tree as f32 bits, in document order
fn bits(v: &serde_json::Value) -> Vec<u32> {
    match v {
        serde_json::Value::Number(n) => vec![(n.as_f64().unwrap() as f32).to_bits()],
        serde_json::Value::Array(a) => a.iter().flat_map(bits).collect(),
        serde_json::Value::Object(o) => o.values().flat_map(bits).collect(),
        _ => vec![],
    }
}

mod wasmtime_backend {
    use super::*;
    use deck_module_host::wasmtime_backend::{Wasmtime, WasmtimeEngine};

    #[test]
    fn the_wasm_plays_the_native_fixture() {
        static E: OnceLock<WasmtimeEngine> = OnceLock::new();
        super::the_wasm_plays_the_native_fixture::<Wasmtime>(E.get_or_init(|| WasmtimeEngine::new(None)));
    }
}
