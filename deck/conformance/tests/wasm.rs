//! (b) live-core native vs `live_core.wasm` run in wasmtime, (c) the lab spike (the stage's voice batches lifted to
//! wasm32 + simd128: size, speed in V8 against BatchGraph), (d) the lab's wasm half: today's stereo scene and the
//! 8-channel routing graph native ≡ wasm32 + simd128, bit for bit.
//! The wasm files are built here (cargo, into `deck/target/conformance-wasm*`), once per source change.
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

/// The lab's output natively and in wasm32 (+simd128), per scenario: the hashes and whether they are equal.
#[test]
fn lab_native_vs_wasm_simd() {
    use deck_engine_lab::{scenario_run, Exp};
    let start = fundsp::math::rnd1(0) as f32;
    let simd = build_wasm("deck-engine-lab", "deck_engine_lab.wasm", true);
    let mut w = Wasm::load(&simd);
    let run = w.f::<(u32, u32, u32, f32, u32), u32>("lab_run");
    let out = w.f::<(), u32>("lab_out");
    let mut md = String::from("| scenario | block | exp | native ≡ wasm32 + simd128 | first differing sample | max abs |\n|---|---|---|---|---|---|\n");
    let names = ["stereo scene (4 voices, mixer, out, limiter)", "8-channel routing graph", "saw PolyBLEP", "saw wavetable"];
    for (sc, name) in names.iter().enumerate() {
        for &block in &[128usize, 37, 1024] {
            for (exp, label) in [(Exp::Std, "std"), (Exp::Libm, "libm")] {
                let frames = FRAMES;
                let native = scenario_run(sc as u32, frames, block, start, exp);
                let len = run.call(&mut w.store, (sc as u32, frames as u32, block as u32, start, u32::from(exp == Exp::Libm))).unwrap() as usize;
                let ptr = out.call(&mut w.store, ()).unwrap();
                let wasm = w.read_f32(ptr, len);
                assert_eq!(native.len(), wasm.len(), "{name}");
                let d = diff(&native, &wasm);
                md.push_str(&format!("| {name} | {block} | {label} | {} | {} | {:.3e} |\n", if d.exact { "yes" } else { "NO" }, d.first.map_or("–".into(), |f| f.to_string()), d.max_abs));
            }
        }
    }
    std::fs::write(report_path("wasm-lab.md"), &md).unwrap();
    println!("{md}");
}

/// (c) the spike's size and speed: the lab (wasm32 + simd128) against live-core's BatchGraph (with and without simd128),
/// FM16 × 128 frames in node (V8). Needs node; skipped (with a note) without.
#[test]
fn spike_size_and_speed_in_v8() {
    let lab = build_wasm("deck-engine-lab", "deck_engine_lab.wasm", true);
    let core = build_wasm("live-core", "live_core.wasm", true);
    let base = build_wasm("live-core", "live_core.wasm", false);
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("js").join("bench.mjs");
    let out = match Command::new("node").arg(&script).arg(&lab).arg(&core).arg(&base).output() {
        Ok(o) => o,
        Err(e) => {
            println!("node is not available ({e}): the spike's V8 numbers are skipped");
            return;
        }
    };
    assert!(out.status.success(), "bench.mjs failed:\n{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    std::fs::write(report_path("spike.json"), &text).unwrap();
    println!("{text}");
}
