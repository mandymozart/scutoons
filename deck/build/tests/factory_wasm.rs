//! Native ≡ wasm for the small audio modules (env, vca, filter, noise, osc, mixer, transpose, delay, reverb, drumseq): each built for
//! `wasm32-unknown-unknown` and run by the module host writes, bit for bit, what the same code writes natively through
//! the SDK's `Rig` – every audio output, control output and event. One script for all, read off the module's shape:
//! every parameter turned through its range, noise on two channels of every audio input, values on the control
//! inputs, notes on and off and a reset on every event input, in calls of changing length. The host is strict, so
//! the wasm that allocated in a call would fail here too.
use deck_module::dsp::noise::White;
use deck_module::testing::Rig;
use deck_module::{Ev, Module, QUANTUM};
use deck_module_host::wasmtime_backend::{Wasmtime, WasmtimeEngine};
use deck_module_host::{Instance, Loaded, Setup};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

const RATE: f64 = 48_000.0;
const CALLS: usize = 1500;

fn wasm(name: &str) -> Vec<u8> {
    static BUILD: OnceLock<()> = OnceLock::new();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).parent().unwrap().join("wasm-modules");
    BUILD.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let mut args = vec!["build", "--release", "--target", "wasm32-unknown-unknown"];
        for p in ["env-module", "vca-module", "filter-module", "noise-module", "osc-module", "mixer-module", "transpose-module", "delay-module", "reverb-module", "drumseq-module"] {
            args.extend(["-p", p]);
        }
        let out = std::process::Command::new(cargo).current_dir(&root).args(args).arg("--target-dir").arg(&dir).output().expect("running cargo");
        assert!(out.status.success(), "building the modules for wasm32 failed (rustup target add wasm32-unknown-unknown?):\n{}", String::from_utf8_lossy(&out.stderr));
    });
    std::fs::read(dir.join(format!("wasm32-unknown-unknown/release/{name}_module.wasm"))).unwrap()
}

/// what a call wrote, as bits (an absent event value as one fixed pattern: a NaN's payload is not the module's)
#[derive(Default, PartialEq)]
struct Written {
    audio: Vec<u32>,
    control: Vec<(u32, bool)>,
    events: Vec<(u32, u8, u32, [u32; 6])>,
}

impl Written {
    fn event(&mut self, e: &Ev) {
        self.events.push((e.frame, e.kind, e.id, e.v.map(|v| if v.is_nan() { u32::MAX } else { v.to_bits() })));
    }
}

/// what the script does before call `k`; `n` frames
struct Step {
    n: usize,
    param: (usize, f32),
    audio: Vec<f32>,
    control: f32,
    events: Vec<Ev>,
}

fn script<M: Module>() -> Vec<Step> {
    let mut signal = White::new(11);
    let inputs: usize = M::SHAPE.audio_in.iter().map(|ch| usize::from(*ch).min(2)).sum();
    (0..CALLS)
        .map(|k| {
            let n = [128, 64, 100, 37][k % 4];
            let i = k % M::PARAMS.len();
            let p = M::PARAMS[i];
            let mut events = vec![];
            match k % 6 {
                0 => events.push(Ev::note_on(5, k as u32, 110.0 * (1 + k % 5) as f32, f32::NAN, 0.2 + 0.2 * (k % 5) as f32)),
                2 => events.push(Ev::note_expr(n as u32 / 2, k as u32 - 2, deck_module::Expr::Bend, (k % 7) as f32 - 3.0)),
                4 => events.push(Ev::note_off(n as u32 - 1, k as u32 - 4)),
                _ => {}
            }
            if k % 250 == 249 {
                events.push(Ev::reset(0));
            }
            Step {
                n,
                param: (i, p.min + (p.max - p.min) * (k % 13) as f32 / 12.0),
                audio: (0..inputs * n).map(|_| signal.sample() * 0.5).collect(),
                control: (k % 5) as f32 * 0.25,
                events,
            }
        })
        .collect()
}

fn native<M: Module>(steps: &[Step], preset: bool) -> Written {
    let s = M::SHAPE;
    let mut rig = Rig::<M>::new(RATE, 3);
    if preset {
        rig.apply_preset();
    }
    for p in 0..s.audio_in_ports() {
        rig.set_channels(true, p, s.audio_in[p].min(2));
    }
    for p in 0..s.audio_out_ports() {
        rig.set_channels(false, p, s.audio_out[p].min(2));
    }
    let (mut w, mut frame) = (Written::default(), 0);
    for step in steps {
        rig.set_param(step.param.0, step.param.1);
        let mut audio = step.audio.chunks(step.n);
        for p in 0..s.audio_in_ports() {
            for c in 0..usize::from(s.audio_in[p].min(2)) {
                rig.set_audio_in(p, c, audio.next().unwrap());
            }
        }
        for c in 0..usize::from(s.control_in) {
            rig.set_control_in(c, step.control);
        }
        for p in 0..s.event_in_ports() {
            rig.set_events_in(p, &step.events);
        }
        rig.run(frame as f64, step.n as u32);
        frame += step.n;
        for p in 0..s.audio_out_ports() {
            let out = rig.audio_out(p);
            for c in 0..out.ch() {
                w.audio.extend(out.chan(c).iter().map(|v| v.to_bits()));
            }
        }
        for c in 0..usize::from(s.control_out) {
            let (v, written) = rig.control_out(c);
            w.control.push((v.to_bits(), written));
        }
        for p in 0..s.event_out_ports() {
            rig.events_out(p).iter().for_each(|e| w.event(e));
        }
    }
    w
}

fn hosted<M: Module>(engine: &WasmtimeEngine, steps: &[Step], preset: bool) -> Written {
    let bytes = wasm(M::NAME);
    assert_eq!(deck_module::host::read_descriptor(&bytes), Some(M::DESCRIPTOR), "{}: the wasm carries the descriptor", M::NAME);
    let loaded = Arc::new(Loaded::<Wasmtime>::load(engine, &bytes).unwrap_or_else(|e| panic!("{}: {e:#}", M::NAME)));
    let mut inst = Instance::new(&loaded, Setup::new(RATE, 3).unbudgeted()).unwrap_or_else(|e| panic!("{}: {e:#}", M::NAME));
    if preset {
        assert!(inst.apply_factory_preset(), "{}: a factory preset", M::NAME);
    }
    let s = M::SHAPE;
    for p in 0..s.audio_in_ports() {
        inst.set_channels(true, p, s.audio_in[p].min(2));
    }
    for p in 0..s.audio_out_ports() {
        inst.set_channels(false, p, s.audio_out[p].min(2));
    }
    let (mut w, mut frame) = (Written::default(), 0);
    for (k, step) in steps.iter().enumerate() {
        inst.set_param(step.param.0, step.param.1);
        let mut audio = step.audio.chunks(step.n);
        for p in 0..s.audio_in_ports() {
            for c in 0..usize::from(s.audio_in[p].min(2)) {
                inst.audio_in_mut(p, c)[..step.n].copy_from_slice(audio.next().unwrap());
            }
        }
        for c in 0..usize::from(s.control_in) {
            inst.set_control_in(c, step.control);
        }
        for p in 0..s.event_in_ports() {
            assert_eq!(inst.set_events_in(p, &step.events), step.events.len());
        }
        assert!(inst.run(frame as f64, step.n as u32), "{} call {k}: {:?}", M::NAME, inst.silenced());
        frame += step.n;
        for p in 0..s.audio_out_ports() {
            let out = inst.audio_out(p);
            for c in 0..out.ch() {
                w.audio.extend(out.chan(c)[..step.n].iter().map(|v| v.to_bits()));
            }
        }
        for c in 0..usize::from(s.control_out) {
            let (v, written) = inst.control_out(c);
            w.control.push((v.to_bits(), written));
        }
        for p in 0..s.event_out_ports() {
            inst.events_out(p).iter().for_each(|e| w.event(e));
        }
    }
    w
}

fn the_wasm_writes_what_the_native_code_writes<M: Module>() {
    static ENGINE: OnceLock<WasmtimeEngine> = OnceLock::new();
    let engine = ENGINE.get_or_init(|| WasmtimeEngine::new(None));
    const { assert!(QUANTUM >= 128) };
    let steps = script::<M>();
    for preset in [false, true] {
        let (n, h) = (native::<M>(&steps, preset), hosted::<M>(engine, &steps, preset));
        assert!(n.audio.iter().any(|b| *b << 1 != 0) || n.events.len() > 100, "{}: the script is not silent", M::NAME);
        let first = n.audio.iter().zip(&h.audio).position(|(a, b)| a != b);
        assert_eq!((first, n.audio.len()), (None, h.audio.len()), "{} (preset {preset}): the audio differs native ≡ wasm", M::NAME);
        assert!(n.control == h.control, "{} (preset {preset}): the control outputs differ", M::NAME);
        assert!(n.events == h.events, "{} (preset {preset}): the events differ", M::NAME);
    }
}

macro_rules! modules {
    ($($test:ident: $module:ty,)*) => {$(
        #[test]
        fn $test() {
            the_wasm_writes_what_the_native_code_writes::<$module>();
        }
    )*};
}

modules! {
    env: env_module::Env,
    vca: vca_module::Vca,
    filter: filter_module::Filter,
    noise: noise_module::Noise,
    osc: osc_module::Osc,
    mixer: mixer_module::Mixer,
    transpose: transpose_module::Transpose,
    delay: delay_module::Delay,
    reverb: reverb_module::Reverb,
    drumseq: drumseq_module::DrumSeq,
}

/// wasm has no flush-to-zero: the reverb's tail, played out in the host to silence, never passes through the denormals
#[test]
fn the_reverb_tail_ends_at_zero_in_wasm() {
    let engine = WasmtimeEngine::new(None);
    let loaded = Arc::new(Loaded::<Wasmtime>::load(&engine, &wasm("reverb")).unwrap());
    let mut inst = Instance::new(&loaded, Setup::new(RATE, 3).unbudgeted()).unwrap();
    let d = deck_module::host::parse(<reverb_module::Reverb as deck_module::Describe>::DESCRIPTOR).unwrap();
    for (name, v) in [("size", 0.0), ("decay", 0.2), ("damp", 2000.0), ("mix", 100.0), ("predelay", 0.0)] {
        inst.set_param(d.params.iter().position(|p| p.name == name).unwrap(), v);
    }
    inst.set_channels(true, 0, 2);
    inst.set_channels(false, 0, 2);
    let mut last = 1.0f32;
    for k in 0..(12.0 * RATE / 128.0) as usize {
        for c in 0..2 {
            let x = inst.audio_in_mut(0, c);
            x[..128].fill(0.0);
            x[0] = if k == 400 { 1.0 } else { 0.0 };
        }
        assert!(inst.run((k * 128) as f64, 128), "call {k}: {:?}", inst.silenced());
        let out = inst.audio_out(0);
        for v in out.chan(0)[..128].iter().chain(&out.chan(1)[..128]) {
            assert!(*v == 0.0 || v.is_normal(), "call {k}: {v:e}");
        }
        last = out.chan(0)[..128].iter().chain(&out.chan(1)[..128]).fold(0.0, |m, v| m.max(v.abs()));
    }
    assert_eq!(last, 0.0, "silent in the end");
}
