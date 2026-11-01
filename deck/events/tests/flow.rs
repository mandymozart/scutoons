//! End to end (#236): random_walk → arp → real-time events → gate_cv, the way the stage's host node will run it.
//! Cut into 256-frame blocks or played as one window, two decks with the same seed: the same samples.
use deck_core::tempo::TempoMap;
use deck_events::bridge::GateCv;
use deck_events::generators::{Arp, ArpMode, RandomWalk};
use deck_events::rt::{resolve, RtEvent};
use deck_events::{Event, EventGraph, Names, Span};

const RATE: f64 = 48_000.0;

struct Deck {
    graph: EventGraph,
    arp: usize,
    cv: GateCv,
    tempo: TempoMap,
    note: deck_events::Sym,
    pitch: deck_events::Sym,
}

fn deck(seed: u64) -> Deck {
    let mut n = Names::new();
    let (note, pitch, glide) = (n.event("music.note").unwrap(), n.key("pitch").unwrap(), n.key("glide").unwrap());
    let mut graph = EventGraph::new();
    let walk = graph.add(RandomWalk {
        note,
        pitch,
        glide_key: glide,
        seed,
        base_hz: 147.0,
        spread: 0.6,
        octaves: 2.0,
        per_beat: 0.5,
        length: 1.0,
        glide: 0.5,
    });
    let arp = graph.add(Arp {
        note,
        pitch,
        ratios: vec![1.0, 1.2, 1.5],
        octaves: 2,
        mode: ArpMode::UpDown,
        per_beat: 4.0,
        length: 0.6,
        root: None,
    });
    graph.connect((walk, "notes"), (arp, "notes"), false).unwrap();
    Deck { graph, arp, cv: GateCv::new(note, pitch, 147.0), tempo: TempoMap::new(96.0, 4, RATE), note, pitch }
}

/// play `frames` in windows of `block` frames; the cv's left and right channels
fn play(d: &mut Deck, frames: usize, block: usize) -> (Vec<f32>, Vec<f32>) {
    let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
    let mut rt: Vec<RtEvent> = Vec::new();
    let mut at = 0;
    while at < frames {
        let n = block.min(frames - at);
        let span = Span::frames(&d.tempo, at as f64, (at + n) as f64);
        d.graph.run(&span).unwrap();
        rt.clear();
        resolve(d.graph.outputs(d.arp, "notes").unwrap(), &span, &mut rt);
        d.cv.render(&rt, &mut l[at..at + n], &mut r[at..at + n]);
        at += n;
    }
    (l, r)
}

#[test]
fn blocks_or_one_window_two_decks_one_seed_the_same_samples() {
    let frames = 4 * RATE as usize; // 4 s at 96 bpm = 6.4 beats
    let one = play(&mut deck(42), frames, frames);
    let blocks = play(&mut deck(42), frames, 256);
    let other_deck = play(&mut deck(42), frames, 1000);
    assert_eq!(one, blocks, "one window = 256-frame blocks, sample for sample");
    assert_eq!(blocks, other_deck, "another deck, other blocks, the same seed: the same samples");
    assert!(one.1.contains(&1.0), "the gate opens");
    let pitches: std::collections::BTreeSet<u32> = one.0.iter().map(|p| p.round() as u32).collect();
    assert!(pitches.len() > 4, "the arp walks a chord on a moving root: {pitches:?}");
    assert_ne!(play(&mut deck(7), frames, 256), one, "another seed, another melody");
}

#[test]
fn an_injected_note_moves_the_root_on_the_next_window() {
    let mut d = deck(42);
    let (note, pitch) = (d.note, d.pitch);
    let span = Span::frames(&d.tempo, 0.0, 256.0);
    d.graph.run(&span).unwrap();
    // a MIDI key (or a story trigger's note): a trigger into the arp, heard from the next window on
    d.graph.inject(d.arp, "notes", Event::now(note).with(pitch, 1000.0)).unwrap();
    // the next beat (96 bpm at 48 kHz: 30 000 frames a beat): the arp's steps walk the new root's chord
    let span = Span::frames(&d.tempo, 256.0, 256.0 + 30_000.0);
    d.graph.run(&span).unwrap();
    let ladder = [1.0, 1.2, 1.5, 2.0, 2.4, 3.0].map(|r| 1000.0 * r);
    let out = d.graph.outputs(d.arp, "notes").unwrap();
    assert_eq!(out.len(), 4, "four steps a beat");
    assert!(
        out.iter().all(|e| ladder.iter().any(|l| (e.props.get(pitch).unwrap() - l).abs() < 1e-2)),
        "every step on the injected root's chord"
    );
}
