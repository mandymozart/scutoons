//! `wiring` (#393, agents/research/NATIVE-EDITOR-GUI.md §8): does hard wiring make a rig faster or slower?
//!
//! **Modules.** The same chain of K filter stages three ways, each output hash compared (the same bits = the same work):
//! - `wired`: K `chain_stage` modules, the host carrying each block from one to the next in the order and with the calls
//!   of the engine's `ModuleBlock` (audio in, process, audio out, a pool buffer per output); without its tempo and event
//!   staging, so a real node costs a little more than this row;
//! - `merged`: one `chain_merged` module with the K stages inside, the crossings gone;
//! - `native`: `chain_merged` compiled for this machine (no wasm), the ceiling.
//!
//! A `light` stage (1 section) shows the crossing itself, a `heavy` one (16 sections) a stage that does real work. Each
//! on the deck's wasmtime settings and on the tuned ones (#396: NaN canonicalization and the epoch off, EXPERIMENTS).
//!
//! **Native nodes.** Sixteen FM voices through the real engine: all on one sequencer (one level, one batch), split so that
//! twelve are free and four sequenced (two levels, two batches), and the patcher build (no batches).
use anyhow::{anyhow, Result};
use deck_engine::{Build, Engine};
use deck_module::testing::Rig;
use deck_module::QUANTUM;
use deck_module_host::wasmtime_backend::{Wasmtime, WasmtimeEngine};
use deck_module_host::{Instance, Loaded, Setup};
use deck_space::graph::Graph;
use deck_space::scene::Scene;
use module_bench::{now_us, BUDGET_US, RATE};
use std::fmt::Write as _;
use std::sync::Arc;

const STAGES: [usize; 3] = [2, 8, 16];
const WEIGHTS: [(&str, f32); 2] = [("light", 1.0), ("heavy", 16.0)];
const WARMUP: u64 = 200;

fn hash(h: &mut u64, s: &[f32]) {
    for x in s {
        for b in x.to_bits().to_le_bytes() {
            *h = (*h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
}

/// the block's input: the same noise for every case, made outside the timing
fn input(k: u64, l: &mut [f32], r: &mut [f32]) {
    let mut x = (k as u32).wrapping_mul(2_654_435_761) | 1;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        (x as i32 as f32) / 2_147_483_648.0 * 0.5
    };
    for (a, b) in l.iter_mut().zip(r.iter_mut()) {
        *a = next();
        *b = next();
    }
}

#[derive(Clone, Copy, Default)]
struct Timing {
    mean: f64,
    p99: f64,
}

fn timing(mut samples: Vec<f64>) -> Timing {
    samples.sort_by(|a, b| a.total_cmp(b));
    let p = ((samples.len() - 1) as f64 * 0.99).round() as usize;
    Timing {
        mean: samples.iter().sum::<f64>() / samples.len() as f64,
        p99: samples[p],
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

fn open(loaded: &Arc<Loaded<Wasmtime>>, values: &[(&str, f32)]) -> Result<Instance<Wasmtime>> {
    let mut i = Instance::new(loaded, Setup::new(RATE, 1).unbudgeted())?;
    for (name, v) in values {
        let k = loaded
            .param_index(name)
            .ok_or_else(|| anyhow!("no parameter {name}"))?;
        i.set_param(k, *v);
    }
    i.set_out_channels(0, 2);
    Ok(i)
}

fn call(i: &mut Instance<Wasmtime>, k: u64) -> Result<()> {
    if i.process(k as f64 * QUANTUM as f64, QUANTUM as u32) {
        Ok(())
    } else {
        Err(anyhow!("call {k}: {}", i.silenced().unwrap_or("?")))
    }
}

fn wired(
    loaded: &Arc<Loaded<Wasmtime>>,
    stages: usize,
    sections: f32,
    blocks: u64,
) -> Result<(Timing, u64)> {
    let mut chain = (0..stages)
        .map(|_| open(loaded, &[("sections", sections)]))
        .collect::<Result<Vec<_>>>()?;
    let mut pool = vec![[0f32; QUANTUM]; stages * 2];
    let (mut l, mut r) = ([0f32; QUANTUM], [0f32; QUANTUM]);
    let mut h = 0xcbf2_9ce4_8422_2325;
    let mut samples = Vec::with_capacity(blocks as usize);
    for k in 0..WARMUP + blocks {
        input(k, &mut l, &mut r);
        let t = now_us();
        for (s, inst) in chain.iter_mut().enumerate() {
            let (before, after) = pool.split_at_mut(s * 2);
            match s {
                0 => inst.set_audio_chans(0, &[&l, &r]),
                _ => inst.set_audio_chans(0, &[&before[s * 2 - 2], &before[s * 2 - 1]]),
            }
            call(inst, k)?;
            inst.read_audio_chan(0, 0, &mut after[0]);
            inst.read_audio_chan(0, 1, &mut after[1]);
        }
        let d = now_us() - t;
        if k >= WARMUP {
            samples.push(d);
            hash(&mut h, &pool[stages * 2 - 2]);
            hash(&mut h, &pool[stages * 2 - 1]);
        }
    }
    Ok((timing(samples), h))
}

fn merged(
    loaded: &Arc<Loaded<Wasmtime>>,
    stages: usize,
    sections: f32,
    blocks: u64,
) -> Result<(Timing, u64)> {
    let mut inst = open(loaded, &[("sections", sections), ("stages", stages as f32)])?;
    let (mut l, mut r) = ([0f32; QUANTUM], [0f32; QUANTUM]);
    let (mut a, mut b) = ([0f32; QUANTUM], [0f32; QUANTUM]);
    let mut h = 0xcbf2_9ce4_8422_2325;
    let mut samples = Vec::with_capacity(blocks as usize);
    for k in 0..WARMUP + blocks {
        input(k, &mut l, &mut r);
        let t = now_us();
        inst.set_audio_chans(0, &[&l, &r]);
        call(&mut inst, k)?;
        inst.read_audio_chan(0, 0, &mut a);
        inst.read_audio_chan(0, 1, &mut b);
        let d = now_us() - t;
        if k >= WARMUP {
            samples.push(d);
            hash(&mut h, &a);
            hash(&mut h, &b);
        }
    }
    Ok((timing(samples), h))
}

fn native(stages: usize, sections: f32, blocks: u64) -> (Timing, u64) {
    let mut rig = Rig::<chain_merged_module::ChainMerged>::new(RATE, 1);
    rig.set_param(0, sections);
    rig.set_param(1, stages as f32);
    rig.set_channels(true, 0, 2);
    rig.set_channels(false, 0, 2);
    rig.set_in_silent(0, false);
    let (mut l, mut r) = ([0f32; QUANTUM], [0f32; QUANTUM]);
    let (mut a, mut b) = ([0f32; QUANTUM], [0f32; QUANTUM]);
    let mut h = 0xcbf2_9ce4_8422_2325;
    let mut samples = Vec::with_capacity(blocks as usize);
    for k in 0..WARMUP + blocks {
        input(k, &mut l, &mut r);
        let t = now_us();
        rig.set_audio_in(0, 0, &l);
        rig.set_audio_in(0, 1, &r);
        rig.run(k as f64 * QUANTUM as f64, QUANTUM as u32);
        {
            let out = rig.audio_out(0);
            a.copy_from_slice(out.chan(0));
            b.copy_from_slice(out.chan(1));
        }
        let d = now_us() - t;
        if k >= WARMUP {
            samples.push(d);
            hash(&mut h, &a);
            hash(&mut h, &b);
        }
    }
    (timing(samples), h)
}

/// sixteen FM voices into mixers and out; the voices below `free` have no cv, the others ride the sequencer
fn fm_scene(free: usize) -> String {
    let mut s = String::from("[[node]]\nid=\"seq\"\nkind=\"seq\"\nbpm=133\ndivision=4\ngate=0.9\nsteps=[45, 52, 57, 55, 60, 64]\n");
    for i in 0..16 {
        let f = 97.0 + 41.3 * i as f32;
        let _ = write!(s, "[[node]]\nid=\"v{i}\"\nkind=\"fm\"\nfreq={f}\ngain=0.06\nratio=1.5\nindex=2.0\nattack=0.001\nrelease=0.2\n");
        let _ = write!(
            s,
            "[[link]]\nfrom=\"v{i}\"\nto=\"m{}.in{}\"\n",
            i / 4,
            i % 4 + 1
        );
        if i >= free {
            let _ = write!(s, "[[link]]\nfrom=\"seq.cv\"\nto=\"v{i}.cv\"\n");
        }
    }
    for m in 0..4 {
        let _ = write!(
            s,
            "[[node]]\nid=\"m{m}\"\nkind=\"mixer\"\n[[link]]\nfrom=\"m{m}\"\nto=\"sum.in{}\"\n",
            m + 1
        );
    }
    s + "[[node]]\nid=\"sum\"\nkind=\"mixer\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n[[link]]\nfrom=\"sum\"\nto=\"out.in\"\n"
}

fn engine_case(text: &str, build: Build, blocks: u64) -> Result<Timing> {
    let g = Graph::build(&Scene::parse(text)?)?;
    let (mut e, _h) = Engine::build_with(&g, RATE as f32, 1.0, build)?;
    let mut out = vec![0f32; QUANTUM * 2];
    for _ in 0..WARMUP {
        e.render(&mut out, 2, |x| x);
    }
    let mut samples = Vec::with_capacity(blocks as usize);
    for _ in 0..blocks {
        let t = now_us();
        e.render(&mut out, 2, |x| x);
        samples.push(now_us() - t);
    }
    Ok(timing(samples))
}

fn configs() -> Vec<(&'static str, wasmtime::Config)> {
    let deck = WasmtimeEngine::deck_config();
    let mut tuned = WasmtimeEngine::deck_config();
    tuned.cranelift_nan_canonicalization(false);
    tuned.epoch_interruption(false);
    vec![("deck", deck), ("tuned", tuned)]
}

pub fn run(label: &str, stage: &[u8], whole: &[u8], blocks: u64, rounds: usize) -> Result<String> {
    let mut md = String::new();
    writeln!(md, "# wiring (#393, {label}): median of {rounds} rounds of {blocks} blocks; mean µs per 128-frame block (budget {BUDGET_US:.0} µs), p99 in brackets")?;
    writeln!(md)?;
    writeln!(md, "| stage | K | wasmtime | wired modules | merged in one | native | wired − merged | per crossing | of budget | wired ÷ merged | bits equal |")?;
    writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|")?;
    let configs: Vec<_> = configs()
        .into_iter()
        .map(|(name, c)| {
            let e = WasmtimeEngine::from_config(&c, None);
            let s = Arc::new(Loaded::<Wasmtime>::load(&e, stage)?);
            let w = Arc::new(Loaded::<Wasmtime>::load(&e, whole)?);
            Ok((name, e, s, w))
        })
        .collect::<Result<Vec<_>>>()?;
    for (weight, sections) in WEIGHTS {
        for k in STAGES {
            let (mut base, mut base_p) = (Vec::new(), Vec::new());
            let mut native_hash = 0;
            for _ in 0..rounds {
                let (t, h) = native(k, sections, blocks);
                base.push(t.mean);
                base_p.push(t.p99);
                native_hash = h;
            }
            let (nat, nat_p) = (median(base), median(base_p));
            for (name, _engine, s, w) in &configs {
                let (mut a, mut ap, mut b, mut bp) =
                    (Vec::new(), Vec::new(), Vec::new(), Vec::new());
                let mut equal = true;
                for _ in 0..rounds {
                    let (t, ha) = wired(s, k, sections, blocks)?;
                    a.push(t.mean);
                    ap.push(t.p99);
                    let (t, hb) = merged(w, k, sections, blocks)?;
                    b.push(t.mean);
                    bp.push(t.p99);
                    equal &= ha == hb && hb == native_hash;
                }
                let (x, xp, y, yp) = (median(a), median(ap), median(b), median(bp));
                let d = x - y;
                writeln!(
                    md,
                    "| {weight} | {k} | {name} | {x:.1} ({xp:.1}) | {y:.1} ({yp:.1}) | {nat:.1} ({nat_p:.1}) | {d:.1} | {:.2} | {:.1} % | {:.2}× | {} |",
                    d / (k - 1) as f64,
                    100.0 * d / BUDGET_US,
                    x / y,
                    if equal { "yes" } else { "NO" }
                )?;
            }
        }
    }
    writeln!(md)?;
    writeln!(md, "`per crossing` = (wired − merged) / (K − 1). `wired` has no tempo or event staging; a real module node costs a little more.")?;
    writeln!(md)?;
    writeln!(
        md,
        "## Native nodes through the engine: sixteen FM voices (batches by level)"
    )?;
    writeln!(md)?;
    writeln!(md, "| wiring | mean µs | p99 µs | of budget |")?;
    writeln!(md, "|---|---|---|---|")?;
    let batched = Build::detect();
    let scalar = Build {
        batch: false,
        avx2: false,
    };
    for (name, free, build) in [
        (
            "all 16 on the sequencer: one level, one batch",
            0usize,
            batched,
        ),
        (
            "12 free + 4 sequenced: two levels, two batches",
            12,
            batched,
        ),
        (
            "all 16 on the sequencer, patcher build: no batch",
            0,
            scalar,
        ),
    ] {
        let text = fm_scene(free);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for _ in 0..rounds {
            let t = engine_case(&text, build, blocks)?;
            a.push(t.mean);
            b.push(t.p99);
        }
        let (x, y) = (median(a), median(b));
        writeln!(
            md,
            "| {name} | {x:.1} | {y:.1} | {:.1} % |",
            100.0 * x / BUDGET_US
        )?;
    }
    Ok(md)
}
