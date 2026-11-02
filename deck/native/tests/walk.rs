//! The `walk` node (#236, deck-events in the stage): a seeded melody as seq's cv into an FM voice. Two engines with
//! the same seed render the same samples (two decks in step without sending notes); a spread changes the melody; a
//! spread of 0 holds the base. The sequencer thread plays the same samples as the in-block path; the dispatcher routes
//! an adapter's note into a patch's input.
#![cfg(feature = "events")]
use deck_events::{Event, Names};
use deck_native::audio::dispatch::{Dispatcher, Inlet};
use deck_native::audio::{Build, Engine, Sequencing};
use deck_native::graph::Graph;
use deck_native::scene::Scene;

fn scene(seed: i64, spread: f32) -> String {
    format!(
        r#"
[[node]]
id = "tempo"
kind = "tempo"
bpm = 96
[[node]]
id = "melody"
kind = "walk"
seed = {seed}
base = 147.0
spread = {spread}
rate = 2.0
[[node]]
id = "voice"
kind = "fm"
ratio = 1.5
index = 2.0
gain = 0.2
[[node]]
id = "fft"
kind = "fft"
[[node]]
id = "out"
kind = "audio_out"
[[link]]
from = "melody.cv"
to = "voice.cv"
[[link]]
from = "voice"
to = "fft.in"
[[link]]
from = "voice"
to = "out.in"
"#
    )
}

fn render(seed: i64, spread: f32, build: Build) -> Vec<f32> {
    let graph = Graph::build(&Scene::parse(&scene(seed, spread)).unwrap()).unwrap();
    let (mut engine, _handles) = Engine::build_with(&graph, 48_000.0, 1.0, build).unwrap();
    let mut all = Vec::new();
    let mut buf = vec![0.0f32; 256 * 2];
    for i in 0..375 {
        // 2 s in 256-frame blocks
        engine.mark(i as u64 * 5_333_333);
        engine.render(&mut buf, 2, |x| x);
        all.extend_from_slice(&buf);
    }
    all
}

#[test]
fn the_same_seed_renders_the_same_melody_and_a_spread_moves_it() {
    for build in [Build::detect(), Build { batch: true, avx2: false }] {
        let a = render(42, 1.0, build);
        assert!(a.iter().any(|x| x.abs() > 0.01), "the voice plays the walk ({})", build.label());
        assert_eq!(a, render(42, 1.0, build), "two decks, one seed: the same samples ({})", build.label());
        assert_ne!(a, render(7, 1.0, build), "another seed ({})", build.label());
        assert_ne!(render(42, 0.0, build), a, "spread 0 holds the base ({})", build.label());
    }
}

/// #236: the `events` node – a patch: an arp on a walk; its input `b` drives the arp's rate
fn arp_scene(seed: i64, b: Option<f32>) -> String {
    let b = b.map_or(String::new(), |v| format!("b = {v}\n"));
    format!(
        r#"
[[node]]
id = "tempo"
kind = "tempo"
bpm = 96
[[node]]
id = "arp"
kind = "events"
{b}generator = [
  {{ id = "walk", kind = "random_walk", seed = {seed}, rate = 0.5, spread = 0.6 }},
  {{ id = "arp", kind = "arp", ratios = [1.0, 1.2, 1.5], octaves = 2, mode = "updown", rate = 4.0, length = 0.6 }},
]
edge = [{{ from = "walk.notes", to = "arp.notes" }}]
out = "arp.notes"
knobs = ["walk.spread", "arp.rate"]
[[node]]
id = "voice"
kind = "fm"
ratio = 2.0
index = 1.0
gain = 0.2
[[node]]
id = "out"
kind = "audio_out"
[[link]]
from = "arp.cv"
to = "voice.cv"
[[link]]
from = "voice"
to = "out.in"
"#
    )
}

fn render_text(text: &str, build: Build) -> Vec<f32> {
    let graph = Graph::build(&Scene::parse(text).unwrap()).unwrap();
    let (mut engine, _handles) = Engine::build_with(&graph, 48_000.0, 1.0, build).unwrap();
    let mut all = Vec::new();
    let mut buf = vec![0.0f32; 256 * 2];
    for i in 0..375 {
        engine.mark(i as u64 * 5_333_333);
        engine.render(&mut buf, 2, |x| x);
        all.extend_from_slice(&buf);
    }
    all
}

#[test]
fn an_events_node_plays_its_patch_and_its_knobs_drive_it() {
    for build in [Build::detect(), Build { batch: true, avx2: false }] {
        let a = render_text(&arp_scene(42, None), build);
        assert!(a.iter().any(|x| x.abs() > 0.01), "the arp plays ({})", build.label());
        assert_eq!(a, render_text(&arp_scene(42, None), build), "one seed, the same samples ({})", build.label());
        assert_ne!(a, render_text(&arp_scene(7, None), build), "another seed ({})", build.label());
        assert_ne!(a, render_text(&arp_scene(42, Some(8.0)), build), "knob b: the arp's rate ({})", build.label());
    }
    let bad = arp_scene(1, None).replace("mode = \"updown\"", "mode = \"sideways\"");
    let graph = Graph::build(&Scene::parse(&bad).unwrap()).unwrap();
    let err = Engine::build(&graph, 48_000.0, 1.0).err().expect("a bad patch is refused");
    assert!(format!("{err:#}").contains("unknown mode"), "{err:#}");
}

/// How a test drives the hosts' generators.
#[derive(Clone, Copy, Debug)]
enum Drive {
    /// inside the audio block (the fallback)
    InBlock,
    /// the sequencer filled by hand one block ahead before each render (no thread)
    ByHand,
    /// the sequencer on its own thread; each render waits until the thread is a block ahead
    Thread,
}

/// 2 s in 256-frame blocks (the render thread's), `send` called before each block (an adapter's events)
fn render_driven(text: &str, drive: Drive, mut send: impl FnMut(usize, &Names, &Inlet)) -> (Vec<f32>, u64) {
    let graph = Graph::build(&Scene::parse(text).unwrap()).unwrap();
    let seq = match drive {
        Drive::InBlock => Sequencing::InBlock,
        _ => Sequencing::Thread,
    };
    let (mut engine, mut h) = Engine::build_seq(&graph, 48_000.0, 1.0, Build::detect(), seq).unwrap();
    let mut sequencer = h.events.take_sequencer();
    assert_eq!(sequencer.is_some(), seq == Sequencing::Thread, "a sequencer only on the thread");
    let (mut dispatcher, inlet) = Dispatcher::new(h.events.take_routing(), h.events.counters.clone());
    let (progress, _thread) = match (drive, sequencer.take()) {
        (Drive::Thread, Some(s)) => (Some(s.progress()), Some(deck_native::audio::sequencer::spawn(s, h.glitches.clone()).unwrap())),
        (_, s) => {
            sequencer = s;
            (None, None)
        }
    };
    let mut all = Vec::new();
    let mut buf = vec![0.0f32; 256 * 2];
    for i in 0..375 {
        send(i, &h.events.names, &inlet);
        dispatcher.drain();
        if let Some(s) = sequencer.as_mut() {
            s.fill(256);
        }
        if let Some(p) = &progress {
            // the thread fills to its horizon on its own; the render may not pass it
            let want = h.clock.rendered() + 256;
            let t0 = std::time::Instant::now();
            while p.load(std::sync::atomic::Ordering::Acquire) < want {
                assert!(t0.elapsed().as_secs() < 5, "the sequencer thread keeps up");
                std::thread::yield_now();
            }
        }
        engine.mark(i as u64 * 5_333_333);
        engine.render(&mut buf, 2, |x| x);
        all.extend_from_slice(&buf);
    }
    (all, h.events.counters.dropped())
}

/// #236: the sequencer thread plays exactly what the in-block path plays – a walk and an arp on a walk, by hand and on
/// the real thread
#[test]
fn the_sequencer_renders_the_same_samples_as_the_block() {
    for text in [scene(42, 1.0), arp_scene(42, Some(6.0))] {
        let (a, dropped) = render_driven(&text, Drive::InBlock, |_, _, _| {});
        assert!(a.iter().any(|x| x.abs() > 0.01), "it plays");
        assert_eq!(dropped, 0);
        for drive in [Drive::ByHand, Drive::Thread] {
            let (b, dropped) = render_driven(&text, drive, |_, _, _| {});
            assert_eq!(dropped, 0, "{drive:?}: nothing late, nothing lost");
            assert!(a == b, "{drive:?}: the same samples as in the block");
        }
    }
}

/// an arp with nothing to play until the dispatcher routes a note into it (the patch's `listen`; a MIDI adapter turns
/// its keys into `music.note` at the edge)
const LISTENING_ARP: &str = r#"
[[node]]
id = "tempo"
kind = "tempo"
bpm = 120
[[node]]
id = "arp"
kind = "events"
generator = [{ id = "arp", kind = "arp", ratios = [1.0, 1.5], rate = 4.0 }]
out = "arp.notes"
listen = [{ to = "arp.notes", name = "music.note" }]
[[node]]
id = "voice"
kind = "fm"
gain = 0.2
[[node]]
id = "out"
kind = "audio_out"
[[link]]
from = "arp.cv"
to = "voice.cv"
[[link]]
from = "voice"
to = "out.in"
"#;

/// #236: the dispatcher routes an adapter's events into a host's input by name – the arp starts on the note, plays
/// its pitch, the same on both paths
#[test]
fn a_routed_note_starts_the_arp_on_both_paths() {
    let send = |i: usize, names: &Names, inlet: &Inlet| {
        let (note, pitch) = (names.get("music.note").unwrap(), names.get("pitch").unwrap());
        match i {
            40 => assert!(inlet.send(Event::now(note).with(pitch, 220.0))),
            200 => assert!(inlet.send(Event::now(note).with(pitch, 330.0))),
            _ => {}
        }
    };
    let (a, _) = render_driven(LISTENING_ARP, Drive::InBlock, send);
    let silent = |s: &[f32]| s.iter().all(|x| x.abs() < 1e-6);
    // the voice's gate stays shut until the first note reaches the arp (block 40)
    assert!(silent(&a[..2 * 256 * 40]), "nothing before the note");
    assert!(!silent(&a[2 * 256 * 41..2 * 256 * 80]), "the arp plays after it");
    for drive in [Drive::ByHand, Drive::Thread] {
        let (b, _) = render_driven(LISTENING_ARP, drive, send);
        assert!(a == b, "{drive:?}: the routed note lands in the same window");
    }
    let (b, _) = render_driven(LISTENING_ARP, Drive::InBlock, |i, n, inl| send(if i == 200 { 0 } else { i }, n, inl));
    assert!(a != b, "the second note changes the pitch");
}
