//! Native ≡ wasm: the module built for `wasm32-unknown-unknown` and run by the module host renders, bit for bit, what the
//! same code renders natively through the SDK's `Rig`. The case is generic over the host's `Backend`, so the browser's
//! backend proves itself against the same script by adding one line at the bottom (the way `deck-module-host`'s
//! conformance suite takes its fixtures).
//!
//! The script: four lanes of notes (events by id, a bend), steals, the pedal, noise, a synced LFO, hard sync, serial
//! filters, parameters turned mid-run, a tempo change – in blocks of 256 frames (two calls of 128).
mod common;
use common::*;
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
            .args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "analog-module", "--target-dir"])
            .arg(&dir)
            .output()
            .expect("running cargo");
        assert!(
            out.status.success(),
            "building the analog module for wasm32 failed (rustup target add wasm32-unknown-unknown?):\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    });
    std::fs::read(dir.join("wasm32-unknown-unknown/release/analog_module.wasm")).unwrap()
}

const BLOCKS: usize = 200;
const BLOCK: usize = 256;

/// what the lanes do in block `step` of the script, applied alike to the native bench and to the wasm instance
fn lane_state(step: usize, lanes: &mut [(f32, f32); 4]) {
    for (k, lane) in lanes.iter_mut().enumerate() {
        // lane k plays a note of 7 + k blocks, then rests 3 blocks; the pitch bends within the held note on lane 2
        let period = 10 + k;
        let phase = step % period;
        let n = step / period;
        if phase == 0 {
            lane.0 = 110.0 * 2f32.powf(((n * 5 + k * 3) % 24) as f32 / 12.0);
        }
        lane.1 = if phase < 7 + k { 0.4 + 0.15 * ((n + k) % 4) as f32 } else { 0.0 };
        if k == 2 && phase == 4 {
            lane.0 *= 1.25;
        }
    }
}

fn params(step: usize) -> Vec<(usize, f32)> {
    let mut v = vec![];
    if step == 0 {
        for (k, val) in preset("sync lead").iter().chain(
            [("noise_level", 0.15f32), ("noise_route", 2.0), ("f_serial", 1.0), ("l1_sync", 4.0), ("l1_shape", 4.0), ("l1_f1", 1.5), ("o1_level", 0.4), ("o1_sub", 0.4), ("sub1_route", 1.0), ("a2_level", 0.7), ("a2_pan", 0.6), ("f2_mode", 1.0), ("f2_cutoff", 1500.0), ("ae1_r", 0.4), ("ae2_r", 0.4)]
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect::<Vec<_>>()
                .iter(),
        ) {
            v.push((idx(k), *val));
        }
    }
    if step == 60 {
        v.push((idx("f1_cutoff"), 900.0));
        v.push((idx("o2_semi"), 3.0));
    }
    if step == 120 {
        v.push((idx("o2_sync"), 0.0));
        v.push((idx("f1_mode"), 2.0));
    }
    v
}

fn native(seed: u64) -> (Vec<f32>, Vec<f32>) {
    let mut b = Bench::seeded(seed);
    let (mut l, mut r) = (vec![], vec![]);
    for step in 0..BLOCKS {
        for (i, v) in params(step) {
            b.rig.set_param(i, v);
        }
        if step == 100 {
            b.rig.set_tempo((step * BLOCK) as f64, 6.0, 93.0, 4, true);
        }
        lane_state(step, &mut b.lanes.state);
        b.sustain = if (30..45).contains(&(step % 90)) { 1.0 } else { 0.0 };
        let (a, c) = b.run_in(BLOCK, BLOCK);
        l.extend(a);
        r.extend(c);
    }
    (l, r)
}

fn hosted<B: Backend>(e: &B::Engine, seed: u64) -> (Vec<f32>, Vec<f32>) {
    let loaded = Arc::new(Loaded::<B>::load(e, &wasm()).expect("the analog module loads"));
    assert_eq!(loaded.desc.name, "analog");
    let mut inst = Instance::new(&loaded, Setup::new(RATE, seed).unbudgeted()).expect("instance");
    let (mut l, mut r) = (vec![], vec![]);
    let mut lanes = Lanes::new();
    let (mut ol, mut or) = (vec![0.0f32; QUANTUM], vec![0.0f32; QUANTUM]);
    for step in 0..BLOCKS {
        for (i, v) in params(step) {
            inst.set_param(i, v);
        }
        if step == 100 {
            inst.set_tempo(&TempoMap::new(93.0, 4, RATE).with_beat((step * BLOCK) as f64, 6.0));
        }
        lane_state(step, &mut lanes.state);
        let sustain = if (30..45).contains(&(step % 90)) { 1.0 } else { 0.0 };
        for c in 0..BLOCK / QUANTUM {
            let events = lanes.events();
            assert_eq!(inst.set_events_in(0, &events), events.len());
            inst.set_control_in(0, sustain);
            assert!(inst.process((step * BLOCK + c * QUANTUM) as f64, QUANTUM as u32), "block {step}: {:?}", inst.silenced());
            inst.read_audio_out(0, &mut ol, &mut or);
            l.extend_from_slice(&ol);
            r.extend_from_slice(&or);
        }
    }
    let (last, max) = inst.cpu_us();
    eprintln!("analog under the module host: {QUANTUM} frames in {last} µs (slowest {max} µs) of a {} µs call", QUANTUM as f64 / RATE * 1e6);
    (l, r)
}

fn the_wasm_renders_what_the_native_code_renders<B: Backend>(e: &B::Engine) {
    for seed in [1u64, 99] {
        let (nl, nr) = native(seed);
        let (wl, wr) = hosted::<B>(e, seed);
        assert!(rms(&nl) > 0.01 && peak(&nl) < 2.0, "the script is audible: rms {}", rms(&nl));
        let bits = |x: &[f32]| x.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
        let first = bits(&nl).iter().zip(bits(&wl)).position(|(a, b)| *a != b);
        assert_eq!(first, None, "seed {seed}: left channel differs native ≡ wasm (first at {first:?})");
        let first = bits(&nr).iter().zip(bits(&wr)).position(|(a, b)| *a != b);
        assert_eq!(first, None, "seed {seed}: right channel differs native ≡ wasm (first at {first:?})");
    }
    let (a, _) = native(1);
    let (b, _) = native(99);
    assert_ne!(a, b, "the seed reaches the noise");
}

mod wasmtime_backend {
    use super::*;
    use deck_module_host::wasmtime_backend::{Wasmtime, WasmtimeEngine};

    #[test]
    fn the_wasm_renders_what_the_native_code_renders() {
        static E: OnceLock<WasmtimeEngine> = OnceLock::new();
        super::the_wasm_renders_what_the_native_code_renders::<Wasmtime>(E.get_or_init(|| WasmtimeEngine::new(None)));
    }
}

#[test]
fn the_wasm_carries_its_descriptor_with_the_parameter_docs_and_the_factory_preset() {
    let bytes = wasm();
    let json = deck_module::host::read_descriptor(&bytes).expect("deck.module");
    assert_eq!(custom_section(&bytes, "deck.module").as_deref(), Some(json));
    assert_eq!(json, <analog_module::Analog as deck_module::Describe>::DESCRIPTOR);
    let d = deck_module::host::parse(json).unwrap();
    assert_eq!(d.params.len(), analog_module::PARAM_NAMES.len());
    assert!(d.params.iter().all(|p| !p.doc.is_empty()), "every parameter documented in the descriptor");
    assert_eq!(d.preset.as_ref().map(|p| p.name.as_str()), Some("pad"));
    // the extra sections of ABI v1 are gone: the descriptor is the one place
    assert_eq!((custom_section(&bytes, "deck.params"), custom_section(&bytes, "deck.presets")), (None, None));
    // small: the whole synth, every parameter doc and its preset
    assert!(bytes.len() < 250_000, "{} bytes (the SDK alone is about 100 KB: walk)", bytes.len());
    eprintln!("analog_module.wasm: {} bytes", bytes.len());
}

/// the payload of the first custom section called `name` in a wasm binary
fn custom_section(wasm: &[u8], name: &str) -> Option<String> {
    fn leb(b: &[u8], at: &mut usize) -> usize {
        let (mut v, mut shift) = (0usize, 0);
        loop {
            let byte = b[*at];
            *at += 1;
            v |= usize::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return v;
            }
            shift += 7;
        }
    }
    let mut at = 8;
    while at < wasm.len() {
        let id = wasm[at];
        at += 1;
        let size = leb(wasm, &mut at);
        let end = at + size;
        if id == 0 {
            let mut p = at;
            let n = leb(wasm, &mut p);
            if &wasm[p..p + n] == name.as_bytes() {
                return Some(String::from_utf8(wasm[p + n..end].to_vec()).unwrap());
            }
        }
        at = end;
    }
    None
}
