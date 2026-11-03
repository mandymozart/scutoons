//! (b) live-core native vs `live_core.wasm` run in wasmtime, and (the engine move, #317) deck-engine itself: every kind's
//! scene native ≡ wasm32 ≡ wasm32 + simd128, bit for bit, for the block patterns 128 / 37 / 1024 – the determinism bar of
//! plan 3.4. The wasm files are built here (cargo, into `deck/target/conformance-wasm*`), once per source change.
mod common;
use common::*;
use std::path::PathBuf;
use std::process::Command;
use wasmtime::{Engine, Instance, Module, Store};

fn deck_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Build `pkg` for wasm32 (`simd`: +simd128) and return the .wasm path.
fn build_wasm(pkg: &str, file: &str, simd: bool) -> PathBuf {
    let dir = deck_dir().join("target").join(if simd { "conformance-wasm-simd" } else { "conformance-wasm" });
    let mut cmd = Command::new("cargo");
    cmd.current_dir(deck_dir()).args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", pkg, "--target-dir"]).arg(&dir);
    if simd {
        cmd.env("RUSTFLAGS", "-C target-feature=+simd128");
    } else {
        cmd.env_remove("RUSTFLAGS");
    }
    let out = cmd.output().expect("cargo runs");
    assert!(out.status.success(), "wasm build of {pkg} failed:\n{}", String::from_utf8_lossy(&out.stderr));
    dir.join("wasm32-unknown-unknown").join("release").join(file)
}

struct Wasm {
    store: Store<()>,
    inst: Instance,
}

impl Wasm {
    fn load(path: &PathBuf) -> Wasm {
        let engine = Engine::default();
        let module = Module::from_file(&engine, path).expect("module compiles (simd128 is on in wasmtime)");
        let mut store = Store::new(&engine, ());
        let inst = Instance::new(&mut store, &module, &[]).expect("no imports needed");
        Wasm { store, inst }
    }
    fn f<P: wasmtime::WasmParams, R: wasmtime::WasmResults>(&mut self, name: &str) -> wasmtime::TypedFunc<P, R> {
        self.inst.get_typed_func(&mut self.store, name).unwrap_or_else(|e| panic!("export {name}: {e}"))
    }
    fn read_f32(&mut self, ptr: u32, n: usize) -> Vec<f32> {
        let mem = self.inst.get_memory(&mut self.store, "memory").expect("memory");
        let data = mem.data(&self.store);
        data[ptr as usize..ptr as usize + n * 4].chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
    }
    fn write(&mut self, ptr: u32, bytes: &[u8]) {
        let mem = self.inst.get_memory(&mut self.store, "memory").expect("memory");
        mem.data_mut(&mut self.store)[ptr as usize..ptr as usize + bytes.len()].copy_from_slice(bytes);
    }
}

/// live-core in wasmtime: the scene (engine TOML), `frames` in blocks of `block`, the control stepped before each.
fn core_wasm_render(w: &mut Wasm, engine_toml: &str, block: usize, frames: usize) -> Vec<f32> {
    let init = w.f::<f32, ()>("lc_init");
    let inbox = w.f::<u32, u32>("lc_inbox");
    let load = w.f::<u32, i32>("lc_load");
    let step = w.f::<f32, ()>("lc_step");
    let render = w.f::<u32, u32>("lc_render");
    init.call(&mut w.store, RATE).unwrap();
    let bytes = engine_toml.as_bytes();
    let ptr = inbox.call(&mut w.store, bytes.len() as u32).unwrap();
    w.write(ptr, bytes);
    assert!(load.call(&mut w.store, bytes.len() as u32).unwrap() >= 0, "live-core.wasm loads the scene");
    let mut out = Vec::with_capacity(frames);
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        step.call(&mut w.store, n as f32 / RATE).unwrap();
        let p = render.call(&mut w.store, n as u32).unwrap();
        out.extend(w.read_f32(p, n));
        done += n;
    }
    out
}

#[test]
fn live_core_native_vs_wasm() {
    let plain = build_wasm("live-core", "live_core.wasm", false);
    let simd = build_wasm("live-core", "live_core.wasm", true);
    let mut wp = Wasm::load(&plain);
    let mut ws = Wasm::load(&simd);
    let mut md = String::from("| kind | block | native ≡ wasm32 | native ≡ wasm32 + simd128 | max abs (wasm32) | first differing sample (wasm32) |\n|---|---|---|---|---|---|\n");
    let mut played = 0;
    for case in CASES {
        let low = lower(case.toml);
        let native = core_render(&low.engine_toml, 128, FRAMES);
        if peak(&native) < 1e-6 {
            continue; // live-core plays nothing of this kind: nothing to compare
        }
        played += 1;
        for &b in &PATTERNS {
            let native = core_render(&low.engine_toml, b, FRAMES);
            let a = core_wasm_render(&mut wp, &low.engine_toml, b, FRAMES);
            let s = core_wasm_render(&mut ws, &low.engine_toml, b, FRAMES);
            let (da, ds) = (diff(&native, &a), diff(&native, &s));
            md.push_str(&format!(
                "| {} | {b} | {} | {} | {:.3e} | {} |\n",
                case.name,
                if da.exact { "yes" } else { "no" },
                if ds.exact { "yes" } else { "no" },
                da.max_abs,
                da.first.map_or("–".into(), |f| f.to_string())
            ));
        }
    }
    assert!(played >= 8, "the kinds live-core plays");
    std::fs::write(report_path("wasm-core.md"), &md).unwrap();
    println!("{md}");
}

/// deck-engine in wasmtime: the scene through `ew_run`.
fn engine_wasm_render(w: &mut Wasm, toml: &str, block: usize, frames: usize, batch: bool) -> Vec<f32> {
    let inbox = w.f::<u32, u32>("ew_inbox");
    let run = w.f::<(f32, u32, u32, u32), u32>("ew_run");
    let out = w.f::<(), u32>("ew_out");
    let ptr = inbox.call(&mut w.store, toml.len() as u32).unwrap();
    w.write(ptr, toml.as_bytes());
    let n = run.call(&mut w.store, (RATE, block as u32, frames as u32, u32::from(batch))).unwrap() as usize;
    assert_eq!(n, frames, "the engine in wasm renders the scene");
    let ptr = out.call(&mut w.store, ()).unwrap();
    w.read_f32(ptr, n)
}

/// The determinism bar (plan 3.4, owner: native ≡ wasm bit for bit): every kind of the suite, the stereo scene too,
/// rendered by deck-engine natively and as wasm32 (plain and +simd128) in wasmtime – the same f32 bits. Also the
/// patcher build (scalar blocks) against the batches in wasm. The report goes to `deck/target/conformance/wasm-engine.md`.
#[test]
fn engine_native_vs_wasm() {
    let plain = build_wasm("deck-engine-wasm", "deck_engine_wasm.wasm", false);
    let simd = build_wasm("deck-engine-wasm", "deck_engine_wasm.wasm", true);
    let (mut wp, mut ws) = (Wasm::load(&plain), Wasm::load(&simd));
    let mut md = String::from("| scene | block | native ≡ wasm32 | native ≡ wasm32 + simd128 | patcher ≡ batch in wasm32 + simd128 | first differing sample |\n|---|---|---|---|---|---|\n");
    let mut scenes: Vec<(&str, &str)> = CASES.iter().map(|c| (c.name, c.toml)).collect();
    scenes.push(("stereo scene", STEREO_TOML));
    let mut wrong = Vec::new();
    for (name, toml) in scenes {
        for &block in &PATTERNS {
            let native = deck_engine_wasm::run(toml, RATE, block, FRAMES, deck_engine::Build::detect()).expect("native renders");
            let a = engine_wasm_render(&mut wp, toml, block, FRAMES, true);
            let b = engine_wasm_render(&mut ws, toml, block, FRAMES, true);
            let patcher = engine_wasm_render(&mut ws, toml, block, FRAMES, false);
            let (da, db, dp) = (diff(&native, &a), diff(&native, &b), diff(&b, &patcher));
            let yes = |d: &Diff| if d.exact { "yes" } else { "NO" };
            md.push_str(&format!("| {name} | {block} | {} | {} | {} | {} |\n", yes(&da), yes(&db), yes(&dp), da.first.or(db.first).map_or("–".into(), |f| f.to_string())));
            if !(da.exact && db.exact) {
                wrong.push(format!("{name}/{block}: wasm32 {:?}, simd128 {:?}", da.first, db.first));
            }
        }
    }
    // P4b (#318): the routing scenes on several device channels – sends with channel maps, folders, outlets, VBAP, the 5.1
    // fold, the group limiters, the feedback loop – through the same two modules
    md.push_str("\n| routing scene | device channels | block | native ≡ wasm32 | native ≡ wasm32 + simd128 | patcher ≡ batch in wasm32 + simd128 | first differing sample |\n|---|---|---|---|---|---|---|\n");
    for case in common::routing::cases() {
        for &block in &PATTERNS {
            let native = deck_engine_wasm::run_device(&case.toml, &case.deck, RATE, block, FRAMES, deck_engine::Build::detect()).expect("native renders");
            let a = engine_wasm_device_render(&mut wp, &case, block, FRAMES, true);
            let b = engine_wasm_device_render(&mut ws, &case, block, FRAMES, true);
            let patcher = engine_wasm_device_render(&mut ws, &case, block, FRAMES, false);
            let (da, db, dp) = (diff(&native, &a), diff(&native, &b), diff(&b, &patcher));
            let yes = |d: &Diff| if d.exact { "yes" } else { "NO" };
            md.push_str(&format!("| {} | {} | {block} | {} | {} | {} | {} |\n", case.name, case.deck.channels, yes(&da), yes(&db), yes(&dp), da.first.or(db.first).map_or("–".into(), |f| f.to_string())));
            if !(da.exact && db.exact) {
                wrong.push(format!("{}/{block}: wasm32 {:?}, simd128 {:?}", case.name, da.first, db.first));
            }
        }
    }
    std::fs::write(report_path("wasm-engine.md"), &md).unwrap();
    println!("{md}");
    assert!(wrong.is_empty(), "native and wasm renders differ:\n{}", wrong.join("\n"));
}

/// deck-engine in wasmtime on a deck configuration: every device channel, interleaved (`ew_config`, `ew_run_device`).
fn engine_wasm_device_render(w: &mut Wasm, case: &common::routing::RoutingCase, block: usize, frames: usize, batch: bool) -> Vec<f32> {
    let inbox = w.f::<u32, u32>("ew_inbox");
    let config = w.f::<u32, u32>("ew_config");
    let run = w.f::<(f32, u32, u32, u32), u32>("ew_run_device");
    let out = w.f::<(), u32>("ew_out");
    let ptr = inbox.call(&mut w.store, case.toml.len() as u32).unwrap();
    w.write(ptr, case.toml.as_bytes());
    let json = serde_json::to_string(&case.deck).unwrap();
    let ptr = config.call(&mut w.store, json.len() as u32).unwrap();
    w.write(ptr, json.as_bytes());
    let n = run.call(&mut w.store, (RATE, block as u32, frames as u32, u32::from(batch))).unwrap() as usize;
    assert_eq!(n, frames * case.deck.channels, "the engine in wasm renders the scene on the deck configuration");
    let ptr = out.call(&mut w.store, ()).unwrap();
    w.read_f32(ptr, n)
}

