//! The native row (#396): fm16 and analog compiled as plain Rust for this machine (their crates' `rlib`), run by the
//! SDK's `testing::Rig` – the module's own code with no wasm engine, no sandbox, no budget. The workloads, parameters,
//! notes and the hash of every output sample are the suite's, so its bits are checked against the wasm runtimes'
//! (ABI-V2 §6: native ≡ wasm32). The ceiling a wasm runtime can approach, not a way to run members' modules.
use crate::{stats, Fnv, Workload, QUANTUM, RATE};
use deck_module::testing::Rig;
use deck_module::{Ev, Module};

fn hash_out<M: Module>(r: &Rig<M>, h: &mut Fnv) {
    let out = r.audio_out(0);
    for c in 0..out.ch() {
        h.f32s(out.chan(c));
    }
}

/// `index`: the position of fm16's `index` parameter (the host's `param_index("index")`)
pub fn fm16(frames: u32, n: usize, index: usize) -> Workload {
    let mut w = Workload {
        name: "fm16".into(),
        frames,
        ..Workload::default()
    };
    let t = crate::now_us();
    let mut r = Rig::<fm16_module::Fm16>::new(RATE, 1);
    w.instantiate_us = crate::now_us() - t;
    r.set_param(index, 3.0);
    let calls = u64::from(frames) / QUANTUM as u64;
    let mut samples = Vec::with_capacity(n);
    let mut h = Fnv::new();
    let mut k = 0u64;
    for _ in 0..n {
        let t = crate::now_us();
        for _ in 0..calls {
            r.run(k as f64 * QUANTUM as f64, QUANTUM as u32);
            k += 1;
            hash_out(&r, &mut h);
        }
        samples.push(crate::now_us() - t);
    }
    w.hash = h.hex();
    stats(&mut w, &samples, &[]);
    w
}

/// two instances, eight notes held on each, a sample is both calls (the suite's `analog`)
pub fn analog(n: usize) -> Workload {
    let mut w = Workload {
        name: "analog x2".into(),
        frames: QUANTUM as u32,
        ..Workload::default()
    };
    let mut synths: Vec<Rig<analog_module::Analog>> =
        (0..2u64).map(|s| Rig::new(RATE, 1 + s)).collect();
    let mut samples = Vec::with_capacity(n);
    let mut h = Fnv::new();
    for k in 0..n as u64 {
        if k == 0 {
            for (s, r) in synths.iter_mut().enumerate() {
                let notes: Vec<Ev> = (0..8u32)
                    .map(|v| {
                        Ev::note_on(
                            0,
                            s as u32 * 8 + v,
                            110.0 * (1.0 + (s as f32 * 8.0 + v as f32) / 5.0),
                            f32::NAN,
                            0.8,
                        )
                    })
                    .collect();
                r.set_events_in(0, &notes);
            }
        }
        let t = crate::now_us();
        for r in synths.iter_mut() {
            r.run(k as f64 * QUANTUM as f64, QUANTUM as u32);
        }
        samples.push(crate::now_us() - t);
        for r in &synths {
            hash_out(r, &mut h);
        }
    }
    w.hash = h.hex();
    stats(&mut w, &samples, &[]);
    w
}
