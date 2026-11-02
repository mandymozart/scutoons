//! The proof (#280): the `walk` module plays what the built-in `walk` node plays – the same notes at the same beats
//! with the same pitches, and the same cv samples (pitch left, gate right) – for the same params, seed and tempo,
//! however the timeline is cut into blocks. The module runs natively here through the SDK's `Rig` (the same `rt` code the
//! wasm exports call); the stage's tests run the compiled .wasm against the same built-in (deck-stage, feature `modules`).
use deck_core::tempo::TempoMap;
use deck_events::bridge::GateCv;
use deck_events::generator::{Inbox, Outbox, Span};
use deck_events::generators::RandomWalk;
use deck_events::names::Names;
use deck_events::rt::{resolve, RtEvent};
use deck_events::{Event, Generator, Time};
use deck_module::host::Descriptor;
use deck_module::testing::Rig;
use deck_module::Describe;
use walk_module::Walk;

const RATE: f64 = 48_000.0;

struct Builtin {
    walk: RandomWalk,
    cv: GateCv,
    tempo: TempoMap,
}

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

    /// one block [start, start + n): its notes (beat, pitch, glide) and its cv
    fn block(&mut self, start: f64, n: usize) -> (Vec<(f64, f32)>, Vec<f32>, Vec<f32>) {
        let span = Span::frames(&self.tempo, start, start + n as f64);
        let mut out = Outbox::new(1, 64);
        self.walk.query(&span, &Inbox { ports: &[] }, &mut out);
        let notes = out
            .port(0)
            .iter()
            .map(|e: &Event| match e.start {
                Time::Beats(b) => (b, e.props.get(self.walk.pitch).unwrap()),
                _ => unreachable!(),
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
    let d = Descriptor::parse(Walk::DESCRIPTOR).unwrap();
    let mut rig = Rig::<Walk>::new(RATE, seed);
    for (name, v) in [("spread", spread), ("rate", per_beat), ("glide", glide), ("base", base)] {
        rig.set_param(d.param_index(name).unwrap(), v);
    }
    rig.set_tempo(0.0, 0.0, bpm, 4, true);
    rig
}

/// the module and the built-in over `blocks` (each a frame count), compared block by block
fn same(seed: u64, spread: f32, per_beat: f32, glide: f32, base: f32, bpm: f64, blocks: &[usize]) -> usize {
    let mut b = Builtin::new(seed, spread, f64::from(per_beat), glide, base, bpm);
    let mut m = module(seed, spread, per_beat, glide, base, bpm);
    let (mut start, mut notes_total) = (0usize, 0);
    for &n in blocks {
        let (notes, l, r) = b.block(start as f64, n);
        m.process(start as f64, n as u32);
        let events = m.events();
        assert_eq!(events.len(), notes.len(), "notes in block at {start}");
        for (e, (beat, pitch)) in events.iter().zip(&notes) {
            assert_eq!(e.name, "music.note");
            assert_eq!(e.start, *beat, "the beat of a step");
            assert_eq!(e.prop("pitch"), Some(*pitch), "the pitch of a step");
            assert_eq!(e.prop("glide"), Some(glide));
        }
        let (ml, mr) = m.audio_out(0);
        assert_eq!(ml, &l[..], "cv pitch, block at {start}");
        assert_eq!(mr, &r[..], "cv gate, block at {start}");
        notes_total += notes.len();
        start += n;
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

#[test]
fn spread_zero_is_a_drone_and_another_seed_another_melody() {
    let mut a = module(1, 0.0, 4.0, 0.0, 147.0, 120.0);
    a.process(0.0, 1024);
    assert!(a.events().iter().all(|e| e.prop("pitch") == Some(147.0)));
    let melody = |seed| {
        let mut m = module(seed, 1.0, 4.0, 0.0, 147.0, 120.0);
        m.process(0.0, 1024);
        m.events().iter().filter_map(|e| e.prop("pitch")).collect::<Vec<_>>()
    };
    assert_ne!(melody(1), melody(2));
    assert_eq!(melody(3), melody(3), "the same seed, the same melody");
}

#[test]
fn a_stopped_tempo_plays_no_steps() {
    let mut m = module(1, 1.0, 4.0, 0.0, 147.0, 120.0);
    m.set_tempo(0.0, 0.0, 120.0, 4, false);
    m.process(0.0, 512);
    assert!(m.events().is_empty());
}

#[test]
fn the_descriptor_declares_the_params_and_ports() {
    let d = Descriptor::parse(Walk::DESCRIPTOR).unwrap();
    assert_eq!(d.name, "walk");
    let names: Vec<&str> = d.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["spread", "rate", "glide", "base", "octaves", "length"]);
    assert_eq!((d.params[1].min, d.params[1].max, d.params[1].default), (0.0, 16.0, 2.0));
    assert_eq!(d.ports.len(), 2);
}

/// The fixture the browser host's test reads (shared/live-core/test/modules.test.ts runs the wasm in V8 and compares):
/// the module's events and a few cv samples per block, made natively here. `UPDATE_FIXTURES=1 cargo test -p walk-module
/// fixture` writes it, a plain run fails when it is stale.
#[test]
fn fixture_for_the_browser_host_is_current() {
    const BLOCK: u32 = 1024;
    let mut m = module(42, 1.0, 3.0, 0.5, 147.0, 84.0);
    let (mut events, mut cv) = (Vec::new(), Vec::new());
    for b in 0..200u32 {
        m.process(f64::from(b) * f64::from(BLOCK), BLOCK);
        for e in m.events() {
            events.push(serde_json::json!({ "block": b, "name": e.name, "start": e.start, "len": e.len, "pitch": e.prop("pitch"), "glide": e.prop("glide") }));
        }
        let (l, r) = m.audio_out(0);
        cv.push(serde_json::json!([l[0], l[l.len() - 1], r.iter().sum::<f32>()]));
    }
    let doc = serde_json::json!({
        "what": "walk-module natively: seed 42, 84 bpm, spread 1, rate 3, glide 0.5, base 147 – 200 blocks of 1024 frames at 48 kHz; cv = [left first, left last, gate sum] per block",
        "rate": RATE, "seed": 42, "bpm": 84, "params": { "spread": 1, "rate": 3, "glide": 0.5, "base": 147 },
        "block": BLOCK, "blocks": 200, "descriptor": Walk::DESCRIPTOR, "events": events, "cv": cv,
    });
    let text = serde_json::to_string_pretty(&doc).unwrap() + "\n";
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/walk.json");
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n"), text, "fixtures/walk.json is stale – UPDATE_FIXTURES=1 cargo test -p walk-module fixture");
    assert!(events.len() >= 15);
}
