//! deck-core behind a C ABI: opaque handles, f32 buffers by pointer + length. The same source is a dynamic module two
//! ways (#182 §4): `--target wasm32-unknown-unknown` → a sandboxed module a native host runs with wasmtime; natively →
//! a `.dll` / `.so` the host loads with libloading (no sandbox, no stable Rust ABI – only these C functions cross).
#![allow(clippy::missing_safety_doc)]

use deck_core::analysis::{Bands, SIZE};
use deck_core::{ControlGraph, Registry, SceneDef};

const RATE: f32 = 48_000.0;

/// A float buffer the host fills / reads (in wasm: inside the module's memory).
#[no_mangle]
pub extern "C" fn buf_new(len: u32) -> *mut f32 {
    Box::into_raw(vec![0.0f32; len as usize].into_boxed_slice()) as *mut f32
}

#[no_mangle]
pub extern "C" fn graph_new(n: u32, voices: u32) -> *mut ControlGraph {
    let scene = SceneDef::parse(&deck_core::graph::bench_scene(n as usize, voices as usize)).expect("bench scene");
    Box::into_raw(Box::new(ControlGraph::build(&scene, &Registry::compiled(), RATE).expect("builds")))
}

#[no_mangle]
pub unsafe extern "C" fn graph_step(g: *mut ControlGraph, dt: f32) {
    (*g).step(dt);
}

#[no_mangle]
pub unsafe extern "C" fn graph_render(g: *mut ControlGraph, out: *mut f32, len: u32) {
    (*g).render(std::slice::from_raw_parts_mut(out, len as usize));
}

#[no_mangle]
pub unsafe extern "C" fn graph_bench_steps(g: *mut ControlGraph, iters: u32) {
    for _ in 0..iters {
        (*g).step(1.0 / 60.0);
    }
}

#[no_mangle]
pub unsafe extern "C" fn graph_bench_render(g: *mut ControlGraph, blocks: u32, frames: u32) -> f32 {
    let mut buf = vec![0.0; frames as usize];
    let mut sum = 0.0;
    for _ in 0..blocks {
        (*g).render(&mut buf);
        sum += buf[0];
    }
    sum
}

pub struct Analyzer {
    b: Bands,
    window: Vec<f32>,
}

#[no_mangle]
pub extern "C" fn bands_new() -> *mut Analyzer {
    let window = (0..SIZE).map(|i| 0.5 * (std::f32::consts::TAU * 440.0 * i as f32 / RATE).sin()).collect();
    Box::into_raw(Box::new(Analyzer { b: Bands::new(RATE), window }))
}

#[no_mangle]
pub unsafe extern "C" fn bands_bench(a: *mut Analyzer, iters: u32) -> f32 {
    let a = &mut *a;
    let mut sum = 0.0;
    for _ in 0..iters {
        sum += a.b.analyse(&a.window)[0];
    }
    sum
}
