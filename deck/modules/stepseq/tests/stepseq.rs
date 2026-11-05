//! The proof (#417): the `stepseq` module's 16 sliders are the pitches it plays, on the tempo clock – a level is a pitch
//! (`base × 2^(octaves × level)`, snapped to semitones by default), `steps` shortens the loop, however the timeline is cut
//! into calls – and its wasm plays, bit for bit, what the same code plays natively through the SDK's `Rig` (the
//! `Rig` is the `rt` code the wasm exports call). Inside the engine, driving `analog`: deck-native's tests/modules.rs.
use deck_core::tempo::TempoMap;
use deck_module::host;
use deck_module::testing::Rig;
use deck_module::{Describe, Kind, QUANTUM};
use stepseq_module::StepSeq;

const RATE: f64 = 48_000.0;
const BPM: f64 = 120.0;
/// the default phrase in semitones above the base (levels 0 … 0.5 of 2 octaves)
const PHRASE: [i32; 16] = [0, 5, 7, 5, 10, 7, 5, 7, 0, 5, 7, 12, 10, 7, 5, 7];

/// one note as the module gives it: its frame in the call, id, pitch (Hz), length (beats)
type Note = (u32, u32, f32, f32);

fn module(params: &[(&str, f32)]) -> Rig<StepSeq> {
    let d = host::parse(StepSeq::DESCRIPTOR).unwrap();
    let mut rig = Rig::<StepSeq>::new(RATE, 1);
    for (name, v) in params {
        rig.set_param(d.param_index(name).unwrap_or_else(|| panic!("no param {name}")), *v);
    }
    rig.set_tempo(0.0, 0.0, BPM, 4, true);
    rig
}

/// the notes of `frames` frames cut into calls of `call` frames: (absolute frame, id, pitch, length)
fn play(m: &mut Rig<StepSeq>, frames: usize, call: usize) -> Vec<Note> {
    let mut notes = Vec::new();
    let mut start = 0;
    while start < frames {
        let n = call.min(frames - start);
        m.run(start as f64, n as u32);
        for e in m.events_out(StepSeq::NOTES) {
            assert_eq!(e.kind(), Some(Kind::NoteOn));
            notes.push((start as u32 + e.frame, e.id, e.pitch_hz(), e.len));
        }
        start += n;
    }
    notes
}

/// a pitch as semitones above `base`
fn semis(pitch: f32, base: f32) -> f32 {
    12.0 * (pitch / base).log2()
}

/// 120 bpm, 2 steps a beat: a step every 12000 frames; two loops of the 16 steps = 32 steps = 8 s
const TWO_LOOPS: usize = 32 * 12_000;

#[test]
fn the_default_phrase_plays_on_the_tempo_clock_and_loops() {
    let notes = play(&mut module(&[]), TWO_LOOPS, QUANTUM);
    assert_eq!(notes.len(), 32);
    for (k, n) in notes.iter().enumerate() {
        assert_eq!((n.0, n.1), (k as u32 * 12_000, k as u32), "step {k} on its beat");
        assert!((semis(n.2, 110.0) - PHRASE[k % 16] as f32).abs() < 1e-3, "step {k}: {} semitones, want {}", semis(n.2, 110.0), PHRASE[k % 16]);
        assert!((n.3 - 0.35).abs() < 1e-6, "gate 0.7 of a step of 0.5 beat: {}", n.3);
    }
}

#[test]
fn a_slider_is_the_pitch() {
    let at = |level: f32| play(&mut module(&[("step3", level)]), 3 * 12_000, QUANTUM)[2].2;
    let (low, high) = (at(0.25), at(0.75));
    // 0.25 of 2 octaves = 6 semitones, 0.75 = 18: an octave apart
    assert!((semis(low, 110.0) - 6.0).abs() < 1e-3 && (semis(high, 110.0) - 18.0).abs() < 1e-3, "{low} {high}");
    assert!((high / low - 2.0).abs() < 1e-5, "an octave: {}", high / low);
    // the base and the reach scale it
    let wide = play(&mut module(&[("step3", 0.5), ("base", 220.0), ("octaves", 4.0)]), 3 * 12_000, QUANTUM)[2].2;
    assert!((wide / 220.0 - 4.0).abs() < 1e-4, "half of four octaves over 220 Hz: {wide}");
}

#[test]
fn snap_lands_on_semitones_and_off_it_is_continuous() {
    let level = ("step1", 0.3); // 0.3 × 24 = 7.2 semitones
    let snapped = play(&mut module(&[level]), 1, QUANTUM)[0].2;
    let free = play(&mut module(&[level, ("snap", 0.0)]), 1, QUANTUM)[0].2;
    assert!((semis(snapped, 110.0) - 7.0).abs() < 1e-3, "{}", semis(snapped, 110.0));
    assert!((semis(free, 110.0) - 7.2).abs() < 1e-3, "{}", semis(free, 110.0));
}

#[test]
fn steps_shortens_the_loop() {
    let notes = play(&mut module(&[("steps", 5.0)]), 12 * 12_000, QUANTUM);
    let got: Vec<i32> = notes.iter().map(|n| semis(n.2, 110.0).round() as i32).collect();
    assert_eq!(got.len(), 12);
    let want: Vec<i32> = (0..12).map(|k| PHRASE[k % 5]).collect();
    assert_eq!(got, want, "the first five steps, over and over");
    // one step: a drone on step 1
    let one = play(&mut module(&[("steps", 1.0)]), 6 * 12_000, QUANTUM);
    assert!(one.iter().all(|n| n.2 == 110.0));
}

#[test]
fn it_holds_however_the_timeline_is_cut() {
    let whole = play(&mut module(&[("rate", 3.0), ("gate", 0.4)]), 200_000, QUANTUM);
    assert!(whole.len() >= 10);
    for call in [1, 17, 100] {
        assert_eq!(play(&mut module(&[("rate", 3.0), ("gate", 0.4)]), 200_000, call), whole, "calls of {call} frames");
    }
}

#[test]
fn it_is_silent_when_the_clock_stands_or_the_rate_is_zero() {
    assert!(play(&mut module(&[("rate", 0.0)]), 48_000, QUANTUM).is_empty());
    let mut stopped = module(&[]);
    stopped.set_tempo(0.0, 0.0, BPM, 4, false);
    assert!(play(&mut stopped, 48_000, QUANTUM).is_empty());
}

mod wasm {
    use super::*;
    use deck_module_host::wasmtime_backend::{Wasmtime, WasmtimeEngine};
    use deck_module_host::{Instance, Loaded, Setup};
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
                .args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "stepseq-module", "--target-dir"])
                .arg(&dir)
                .output()
                .expect("running cargo");
            assert!(out.status.success(), "building the stepseq module for wasm32 failed (rustup target add wasm32-unknown-unknown?):\n{}", String::from_utf8_lossy(&out.stderr));
        });
        std::fs::read(dir.join("wasm32-unknown-unknown/release/stepseq_module.wasm")).unwrap()
    }

    /// the wasm in the module host plays, bit for bit, what the native run plays – default phrase, a rewritten step, a
    /// shorter loop and continuous pitch
    #[test]
    fn the_wasm_plays_what_the_native_run_plays() {
        static E: OnceLock<WasmtimeEngine> = OnceLock::new();
        let loaded = Arc::new(Loaded::<Wasmtime>::load(E.get_or_init(|| WasmtimeEngine::new(None)), &wasm()).expect("the stepseq module loads"));
        assert_eq!(loaded.desc.name, "stepseq");
        assert_eq!(loaded.desc.params.len(), 22);
        for params in [&[][..], &[("step7", 0.91), ("rate", 3.0)][..], &[("steps", 11.0), ("snap", 0.0), ("base", 97.0)][..]] {
            let want = play(&mut module(params), 100_000, QUANTUM);
            let mut i = Instance::new(&loaded, Setup::new(RATE, 1).unbudgeted()).unwrap();
            for (name, v) in params {
                i.set_param(loaded.param_index(name).unwrap(), *v);
            }
            i.set_tempo(&TempoMap::new(BPM, 4, RATE));
            let mut got = Vec::new();
            for c in 0..100_000usize.div_ceil(QUANTUM) {
                let (at, n) = (c * QUANTUM, QUANTUM.min(100_000 - c * QUANTUM));
                assert!(i.process(at as f64, n as u32), "{:?}", i.silenced());
                got.extend(i.events_out(0).iter().map(|e| ((at as u32 + e.frame), e.id, e.pitch_hz(), e.len)));
            }
            let bits = |v: &[Note]| v.iter().map(|n| (n.0, n.1, n.2.to_bits(), n.3.to_bits())).collect::<Vec<_>>();
            assert!(!want.is_empty());
            assert_eq!(bits(&got), bits(&want), "{params:?}");
        }
    }
}
