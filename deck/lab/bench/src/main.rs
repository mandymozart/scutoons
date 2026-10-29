//! `cargo run --release -p bench` (after building core-raw for both targets, deck/lab/README.md): the deck-core
//! workloads on the native paths – compiled in, a dylib through libloading, a wasm module in wasmtime – as ns per
//! call (the median of 7 runs). The browser paths are measured by deck/lab/www and deck/lab/ts. Prints a table and
//! writes `results/native.json`.
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use deck_core::analysis::{Bands, SIZE};
use deck_core::batch::BatchGraph;
use deck_core::{ControlGraph, Registry, SceneDef};

const RATE: f32 = 48_000.0;
/// the workload sizes – the same in every path (www/worker.ts, ts/bench.ts)
const CHAINS: u32 = 100; // 300 control nodes
const VOICES: u32 = 16;
const FRAMES: u32 = 128; // an AudioWorklet quantum
pub const ENVELOPE: &str = r#"{"v":1,"seq":4,"at":1761700000250,"from":"c1","to":{"group":"left"},"kind":"cue","p":{"id":"cue-7","actions":[{"say":{"outlet":"left","text":{"de":"Aufstehen","en":"Stand up"},"haptic":true}},{"stage":{"outlet":"left","layer":"l1","show":true}},{"send":{"outlet":"ableton","midi":{"type":"note","ch":1,"n":60,"v":100}}}]}}"#;

/// ns per call: `iters` calls per run, 7 runs, the median.
fn time(iters: u32, mut f: impl FnMut()) -> f64 {
    for _ in 0..iters / 10 + 1 {
        f();
    }
    let mut runs: Vec<f64> = (0..7)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..iters {
                f();
            }
            t.elapsed().as_nanos() as f64 / iters as f64
        })
        .collect();
    runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    runs[3]
}

struct Row {
    path: &'static str,
    workload: &'static str,
    ns: f64,
}

fn main() -> Result<()> {
    let mut rows: Vec<Row> = vec![];
    let mut push = |path, workload, ns| rows.push(Row { path, workload, ns });

    // the build's label: `BENCH_LABEL=x86-64-v3` for a run built with other RUSTFLAGS
    let label: &'static str = Box::leak(std::env::var("BENCH_LABEL").map(|l| format!("native {l}")).unwrap_or("native".into()).into_boxed_str());
    let batch_label: &'static str = Box::leak(format!("{label} · batch").into_boxed_str());

    // ---- the two architectures (#182 §4b): patcher (one virtual call per node) vs batch (per level, per kind, SoA)
    for (shape, text) in [
        ("deep", deck_core::graph::bench_scene(CHAINS as usize, VOICES as usize)),
        ("wide", deck_core::graph::bench_scene_wide(CHAINS as usize, VOICES as usize)),
    ] {
        let scene = SceneDef::parse(&text).map_err(anyhow::Error::msg)?;
        let mut g = ControlGraph::build(&scene, &Registry::compiled(), RATE).map_err(anyhow::Error::msg)?;
        let mut b = BatchGraph::build(&scene, RATE).map_err(anyhow::Error::msg)?;
        let w: &'static str = if shape == "deep" { "control step (300 nodes, deep)" } else { "control step (300 nodes, wide)" };
        push(label, w, time(20_000, || g.step(1.0 / 60.0)));
        push(batch_label, w, time(20_000, || b.step(1.0 / 60.0)));
        if shape == "deep" {
            let mut buf = vec![0.0f32; FRAMES as usize];
            push(label, "audio block (16 FM voices, 128 frames)", time(20_000, || g.render(&mut buf)));
            push(batch_label, "audio block (16 FM voices, 128 frames)", time(20_000, || b.render(&mut buf)));
        }
    }
    if label != "native" {
        return print(&rows, label);
    }

    // ---- compiled in (deck-core as a crate: the stage's path)
    let scene = SceneDef::parse(&deck_core::graph::bench_scene(CHAINS as usize, VOICES as usize)).map_err(anyhow::Error::msg)?;
    let mut g = ControlGraph::build(&scene, &Registry::compiled(), RATE).map_err(anyhow::Error::msg)?;
    let mut buf = vec![0.0f32; FRAMES as usize];
    let mut b = Bands::new(RATE);
    let window: Vec<f32> = (0..SIZE).map(|i| 0.5 * (std::f32::consts::TAU * 440.0 * i as f32 / RATE).sin()).collect();
    push("native", "FFT bands (1024)", time(20_000, || {
        std::hint::black_box(b.analyse(&window));
    }));
    push("native", "codec decode + encode (cue, 330 B)", time(50_000, || {
        let e: protocol::Envelope = serde_json::from_str(ENVELOPE).unwrap();
        std::hint::black_box(serde_json::to_string(&e).unwrap());
    }));
    let t = Instant::now();
    let _ = ControlGraph::build(&scene, &Registry::compiled(), RATE).map_err(anyhow::Error::msg)?;
    push("native", "graph build from TOML (348 nodes)", t.elapsed().as_nanos() as f64);

    // ---- a dylib (libloading): the same code behind the C ABI, one indirect call per use
    let lab = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let dll = lab.join("target/release").join(libloading::library_filename("core_raw"));
    unsafe {
        let lib = libloading::Library::new(&dll).with_context(|| format!("{} – build core-raw first", dll.display()))?;
        let graph_new: libloading::Symbol<extern "C" fn(u32, u32) -> *mut u8> = lib.get(b"graph_new")?;
        let graph_step: libloading::Symbol<unsafe extern "C" fn(*mut u8, f32)> = lib.get(b"graph_step")?;
        let graph_render: libloading::Symbol<unsafe extern "C" fn(*mut u8, *mut f32, u32)> = lib.get(b"graph_render")?;
        let bands_new: libloading::Symbol<extern "C" fn() -> *mut u8> = lib.get(b"bands_new")?;
        let bands_bench: libloading::Symbol<unsafe extern "C" fn(*mut u8, u32) -> f32> = lib.get(b"bands_bench")?;
        let g = graph_new(CHAINS, VOICES);
        push("dylib", "control step (300 nodes, deep)", time(20_000, || graph_step(g, 1.0 / 60.0)));
        let p = buf.as_mut_ptr();
        push("dylib", "audio block (16 FM voices, 128 frames)", time(20_000, || graph_render(g, p, FRAMES)));
        let a = bands_new();
        push("dylib", "FFT bands (1024)", time(20_000, || {
            std::hint::black_box(bands_bench(a, 1));
        }));
        std::mem::forget(lib); // the handles above live in it
    }

    // ---- a wasm module in wasmtime (Cranelift): sandboxed, its own memory
    let wasm = lab.join("target/wasm32-unknown-unknown/release/core_raw.wasm");
    let t = Instant::now();
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::from_file(&engine, &wasm).with_context(|| format!("{} – build core-raw for wasm32 first", wasm.display()))?;
    push("wasmtime", "module compile (Cranelift, cold)", t.elapsed().as_nanos() as f64);
    let t = Instant::now();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[])?;
    push("wasmtime", "instantiate", t.elapsed().as_nanos() as f64);
    let graph_new = instance.get_typed_func::<(u32, u32), u32>(&mut store, "graph_new")?;
    let graph_step = instance.get_typed_func::<(u32, f32), ()>(&mut store, "graph_step")?;
    let graph_render = instance.get_typed_func::<(u32, u32, u32), ()>(&mut store, "graph_render")?;
    let graph_bench_steps = instance.get_typed_func::<(u32, u32), ()>(&mut store, "graph_bench_steps")?;
    let buf_new = instance.get_typed_func::<u32, u32>(&mut store, "buf_new")?;
    let bands_new = instance.get_typed_func::<(), u32>(&mut store, "bands_new")?;
    let bands_bench = instance.get_typed_func::<(u32, u32), f32>(&mut store, "bands_bench")?;
    let memory = instance.get_memory(&mut store, "memory").context("memory export")?;
    let g = graph_new.call(&mut store, (CHAINS, VOICES))?;
    push("wasmtime", "control step (300 nodes, deep)", time(20_000, || graph_step.call(&mut store, (g, 1.0 / 60.0)).unwrap()));
    push("wasmtime (loop inside)", "control step (300 nodes, deep)", time(20, || graph_bench_steps.call(&mut store, (g, 1000)).unwrap()) / 1000.0);
    let out = buf_new.call(&mut store, FRAMES)?;
    push("wasmtime", "audio block (16 FM voices, 128 frames)", time(20_000, || {
        graph_render.call(&mut store, (g, out, FRAMES)).unwrap();
        // the host reads the block out of the module's memory, as an audio callback would
        let bytes = &memory.data(&store)[out as usize..out as usize + FRAMES as usize * 4];
        for (d, s) in buf.iter_mut().zip(bytes.chunks_exact(4)) {
            *d = f32::from_le_bytes([s[0], s[1], s[2], s[3]]);
        }
    }));
    let a = bands_new.call(&mut store, ())?;
    push("wasmtime", "FFT bands (1024)", time(20_000, || {
        std::hint::black_box(bands_bench.call(&mut store, (a, 1)).unwrap());
    }));

    print(&rows, "native")
}

fn print(rows: &[Row], label: &str) -> Result<()> {
    println!("| path | workload | µs |
|---|---|---|");
    for r in rows {
        println!("| {} | {} | {:.3} |", r.path, r.workload, r.ns / 1000.0);
    }
    let json: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| serde_json::json!({ "path": r.path, "workload": r.workload, "us": (r.ns / 10.0).round() / 100.0 }))
        .collect();
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../results");
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join(format!("{}.json", label.replace(' ', "-"))), serde_json::to_string_pretty(&json)?)?;
    Ok(())
}
