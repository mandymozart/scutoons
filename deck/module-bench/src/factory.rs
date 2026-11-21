//! The small audio modules' rows: env, vca, filter, noise, osc, mixer, transpose and reverb, each alone under one
//! runtime and as busy as a rig makes it – its factory preset, noise on two channels of every audio input, 0.5 on its
//! control inputs, and on every event input a note on in each call (its off in the next), so an envelope retriggers
//! and transpose has notes to move. A sample is one call of 128 frames; the table is the suite's, a row a module.
use super::{call, instance, load, now_us, stats, Fnv, Runtime, Workload, RATE};
use anyhow::Result;
use deck_module::dsp::noise::White;
use deck_module::{Ev, QUANTUM};
use deck_module_host::Setup;
use std::fmt::Write;

pub const NAMES: [&str; 8] = ["env", "vca", "filter", "noise", "osc", "mixer", "transpose", "reverb"];

/// `n` calls of the module in `wasm`
pub fn workload<B: Runtime>(engine: &B::Engine, name: &str, wasm: &[u8], n: usize) -> Result<Workload> {
    let mut w = Workload { name: name.into(), frames: QUANTUM as u32, ..Workload::default() };
    let loaded = load::<B>(engine, wasm)?;
    w.compile_us = loaded.compile.as_secs_f64() * 1e6;
    let (mut i, inst) = instance(&loaded, Setup::new(RATE, 1).unbudgeted())?;
    w.instantiate_us = inst;
    i.apply_factory_preset();
    let shape = i.io_layout().shape;
    for p in 0..shape.audio_in_ports() {
        i.set_channels(true, p, shape.audio_in[p].min(2));
    }
    for p in 0..shape.audio_out_ports() {
        i.set_channels(false, p, shape.audio_out[p].min(2));
    }
    let mut signal = White::new(1);
    let mut samples = Vec::with_capacity(n);
    let mut h = Fnv::new();
    for k in 0..n as u64 {
        for p in 0..shape.audio_in_ports() {
            for c in 0..shape.audio_in[p].min(2) as usize {
                for s in i.audio_in_mut(p, c) {
                    *s = signal.sample() * 0.5;
                }
            }
        }
        for c in 0..shape.control_in as usize {
            i.set_control_in(c, 0.5);
        }
        let id = k as u32;
        let notes = [Ev::note_off(0, id.wrapping_sub(1)), Ev::note_on(8, id, 110.0 * (1 + k % 8) as f32, 45.0 + (k % 8) as f32, 0.8)];
        for p in 0..shape.event_in_ports() {
            i.set_events_in(p, &notes);
        }
        let t = now_us();
        call(&mut i, k)?;
        samples.push(now_us() - t);
        for p in 0..shape.audio_out_ports() {
            let out = i.audio_out(p);
            for c in 0..out.ch() {
                h.f32s(out.chan(c));
            }
        }
        for c in 0..shape.control_out as usize {
            h.f32s(&[i.control_out(c).0]);
        }
        for p in 0..shape.event_out_ports() {
            for e in i.events_out(p) {
                h.f32s(&[e.frame as f32, e.id as f32, e.pitch_hz()]);
            }
        }
        w.memory_bytes = w.memory_bytes.max(i.memory_size() as u64);
    }
    w.hash = h.hex();
    stats(&mut w, &samples, &[]);
    Ok(w)
}

/// the rows as a table
pub fn markdown(label: &str, runtime: &str, rows: &[Workload]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "| device | runtime | module | frames | compile | inst. | first µs | p50 µs | p99 µs | max µs | mean µs | p99 % budget | mem KB | hash |");
    let _ = writeln!(s, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for w in rows {
        if let Some(e) = &w.error {
            let _ = writeln!(s, "| {label} | {runtime} | {} | {} | error: {} ||||||||||", w.name, w.frames, e.replace('|', "/"));
            continue;
        }
        let _ = writeln!(
            s,
            "| {label} | {runtime} | {} | {} | {:.1} ms | {:.0} µs | {:.0} | {:.1} | {:.1} | {:.0} | {:.1} | {:.2} % | {} | {} |",
            w.name,
            w.frames,
            w.compile_us / 1000.0,
            w.instantiate_us,
            w.first_us,
            w.p50_us,
            w.p99_us,
            w.max_us,
            w.mean_us,
            w.p99_pct,
            w.memory_bytes / 1024,
            &w.hash[..w.hash.len().min(8)]
        );
    }
    s
}
