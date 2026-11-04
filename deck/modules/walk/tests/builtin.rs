//! The proof (#280; module ABI v2 #343): the `walk` module plays what the built-in `walk` node plays – the same notes
//! at the same frames with the same pitches, lengths and glides, and the same cv samples (pitch left, gate right) – for
//! the same params, seed and tempo, however the timeline is cut into blocks. The module runs natively here through the
//! SDK's `Rig` (the same `rt` code the wasm exports call); its wasm runs against `fixtures/walk.json` in tests/wasm.rs
//! (native ≡ wasm) and inside the engine in deck-native's tests/modules.rs.
use deck_core::tempo::TempoMap;
use deck_events::bridge::GateCv;
use deck_events::generator::{Inbox, Outbox, Span};
use deck_events::generators::RandomWalk;
use deck_events::names::Names;
use deck_events::rt::{resolve, RtEvent};
use deck_events::{Event, Generator, Time};
use deck_module::host;
use deck_module::testing::Rig;
use deck_module::{Describe, Kind, QUANTUM};
use walk_module::Walk;

const RATE: f64 = 48_000.0;

struct Builtin {
    walk: RandomWalk,
    cv: GateCv,
    tempo: TempoMap,
}

/// one note as both sides give it: its frame in the call, pitch, length (beats), glide
type Note = (u32, f32, f32, f32);

impl Builtin {
    fn new(seed: u64, spread: f32, per_beat: f64, glide: f32, base: f32, bpm: f64) -> Builtin {
        let mut n = Names::new();
        let (note, pitch, glide_key) = (n.event("music.note").unwrap(), n.key("pitch").unwrap(), n.key("glide").unwrap());
        Builtin {
            walk: RandomWalk { note, pitch, glide_key, seed, base_hz: base, spread, octaves: 2.0, per_beat, length: 1.0, glide },
            cv: GateCv::new(note, pitch, glide_key, base),
            tempo: TempoMap::new(bpm, 4, RATE),
        }
    }

    /// one call [start, start + n): its notes and its cv
    fn call(&mut self, start: f64, n: usize) -> (Vec<Note>, Vec<f32>, Vec<f32>) {
        let span = Span::frames(&self.tempo, start, start + n as f64);
        let mut out = Outbox::new(1, 64);
        self.walk.query(&span, &Inbox { ports: &[] }, &mut out);
        let notes = out
            .port(0)
            .iter()
            .map(|e: &Event| {
                let frame = (e.frame(&self.tempo, start) - start).max(0.0).round() as u32;
                let len = match e.dur {
                    Some(Time::Beats(b)) => b as f32,
                    _ => unreachable!(),
                };
                (frame, e.props.get(self.walk.pitch).unwrap(), len, e.props.get(self.walk.glide_key).unwrap())
            })
            .collect();
        let mut rt: Vec<RtEvent> = Vec::new();
        resolve(out.port(0), &span, &mut rt);
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        self.cv.render(&rt, &mut l, &mut r);
        (notes, l, r)
    }
}

fn module(seed: u64, spread: f32, per_beat: f32, glide: f32, base: f32, bpm: f64) -> Rig<Walk> {
    let d = host::parse(Walk::DESCRIPTOR).unwrap();
    let mut rig = Rig::<Walk>::new(RATE, seed);
    for (name, v) in [("spread", spread), ("rate", per_beat), ("glide", glide), ("base", base)] {
        rig.set_param(d.param_index(name).unwrap(), v);
    }
    rig.set_tempo(0.0, 0.0, bpm, 4, true);
    rig
}

/// one module call: its notes and its cv
fn run(m: &mut Rig<Walk>, start: f64, n: usize) -> (Vec<Note>, Vec<f32>, Vec<f32>) {
    m.run(start, n as u32);
    let notes = m
        .events_out(Walk::NOTES)
        .iter()
        .map(|e| {
            assert_eq!(e.kind(), Some(Kind::NoteOn));
            (e.frame, e.pitch_hz(), e.len, e.glide())
        })
        .collect();
    let cv = m.audio_out(Walk::CV);
    let (l, r) = cv.stereo();
    (notes, l.to_vec(), r.to_vec())
}

/// the module and the built-in over `blocks` (each a frame count, cut into calls of at most `QUANTUM`), compared call
/// by call; the notes in total
fn same(seed: u64, spread: f32, per_beat: f32, glide: f32, base: f32, bpm: f64, blocks: &[usize]) -> usize {
    let mut b = Builtin::new(seed, spread, f64::from(per_beat), glide, base, bpm);
    let mut m = module(seed, spread, per_beat, glide, base, bpm);
    let (mut start, mut notes_total) = (0usize, 0);
    for &block in blocks {
        let mut left = block;
        while left > 0 {
            let n = left.min(QUANTUM);
            let (want, wl, wr) = b.call(start as f64, n);
            let (got, ml, mr) = run(&mut m, start as f64, n);
            assert_eq!(got, want, "the notes of the call at {start} (frame, pitch, length, glide)");
            assert_eq!(ml, wl, "cv pitch, call at {start}");
            assert_eq!(mr, wr, "cv gate, call at {start}");
            notes_total += want.len();
            start += n;
            left -= n;
        }
    }
    notes_total
}

#[test]
fn the_module_plays_the_built_in_walk_note_for_note_and_sample_for_sample() {
    // 84 bpm, 3 steps a beat, ~4 s in 256-frame blocks: a melody wide in range with glides
    let blocks = vec![256; 800];
    let notes = same(42, 1.0, 3.0, 0.5, 147.0, 84.0, &blocks);
    assert!(notes >= 12, "{notes} notes");
}

#[test]
fn it_holds_however_the_timeline_is_cut() {
    // odd block lengths, a different seed, other params: still the same as the built-in
    let blocks: Vec<usize> = (0..600).map(|i| [64, 100, 333, 17, 1024, 480][i % 6]).collect();
    let notes = same(7, 0.6, 2.0, 1.0, 220.0, 133.0, &blocks);
    assert!(notes >= 10, "{notes} notes");
}

/// the pitches of the notes over `calls` calls of `QUANTUM` frames
fn melody(seed: u64, spread: f32) -> Vec<f32> {
    let mut m = module(seed, spread, 4.0, 0.0, 147.0, 120.0);
    (0..64).flat_map(|k| run(&mut m, (k * QUANTUM) as f64, QUANTUM).0).map(|n| n.1).collect()
}

#[test]
fn spread_zero_is_a_drone_and_another_seed_another_melody() {
    let drone = melody(1, 0.0);
    assert!(!drone.is_empty() && drone.iter().all(|&p| p == 147.0));
    assert_ne!(melody(1, 1.0), melody(2, 1.0));
    assert_eq!(melody(3, 1.0), melody(3, 1.0), "the same seed, the same melody");
}

#[test]
fn a_stopped_tempo_plays_no_steps() {
    let mut m = module(1, 1.0, 4.0, 0.0, 147.0, 120.0);
    m.set_tempo(0.0, 0.0, 120.0, 4, false);
    for k in 0..4 {
        assert!(run(&mut m, (k * QUANTUM) as f64, QUANTUM).0.is_empty());
    }
}

#[test]
fn the_descriptor_declares_the_params_and_ports() {
    let d = host::parse(Walk::DESCRIPTOR).unwrap();
    assert_eq!(d.name, "walk");
    let names: Vec<&str> = d.params.iter().map(|p| p.name.as_ref()).collect();
    assert_eq!(names, ["spread", "rate", "glide", "base", "octaves", "length"]);
    assert_eq!((d.params[1].min, d.params[1].max, d.params[1].default), (0.0, 16.0, Some(2.0)));
    assert!(d.params.iter().all(|p| !p.doc.is_empty()), "every parameter documented");
    assert_eq!(d.ports.iter().map(|p| (p.name.as_ref(), p.ty)).collect::<Vec<_>>(), [("cv", deck_catalog::PortType::Cv), ("notes", deck_catalog::PortType::Event)]);
    let notes = &d.ports[1];
    assert_eq!((notes.names[0].as_ref(), notes.props.iter().map(|p| p.as_ref()).collect::<Vec<_>>()), ("music.note", vec!["pitch", "vel", "glide"]));
}

/// The native run the wasm is held against (tests/wasm.rs, native ≡ wasm): the module's notes and a few cv samples per
/// block of 1024 frames (eight calls), made natively here. `UPDATE_FIXTURES=1 cargo test -p walk-module fixture` writes
/// it, a plain run fails when it is stale.
#[test]
fn fixture_of_the_native_run_is_current() {
    const BLOCK: usize = 1024;
    let mut m = module(42, 1.0, 3.0, 0.5, 147.0, 84.0);
    let (mut events, mut cv) = (Vec::new(), Vec::new());
    for b in 0..200usize {
        let (mut l, mut r) = (Vec::new(), Vec::new());
        for c in 0..BLOCK / QUANTUM {
            let at = c * QUANTUM;
            let (notes, cl, cr) = run(&mut m, (b * BLOCK + at) as f64, QUANTUM);
            for (frame, pitch, len, glide) in notes {
                events.push(serde_json::json!({ "block": b, "frame": at as u32 + frame, "pitch": pitch, "len": len, "glide": glide }));
            }
            l.extend(cl);
            r.extend(cr);
        }
        cv.push(serde_json::json!([l[0], l[l.len() - 1], r.iter().sum::<f32>()]));
    }
    let doc = serde_json::json!({
        "what": "walk-module natively: seed 42, 84 bpm, spread 1, rate 3, glide 0.5, base 147 – 200 blocks of 1024 frames (eight calls of 128) at 48 kHz; notes = note ons (frame in the block, pitch Hz, length beats, glide); cv = [left first, left last, gate sum] per block",
        "rate": RATE, "seed": 42, "bpm": 84, "params": { "spread": 1, "rate": 3, "glide": 0.5, "base": 147 },
        "block": BLOCK, "blocks": 200, "descriptor": Walk::DESCRIPTOR, "notes": events, "cv": cv,
    });
    let text = serde_json::to_string_pretty(&doc).unwrap() + "\n";
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/walk.json");
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n"), text, "fixtures/walk.json is stale – UPDATE_FIXTURES=1 cargo test -p walk-module fixture");
    assert!(events.len() >= 15);
}
