//! The event layer may run on the audio thread (the stage's host node, #236): after the graph is built, a run, the
//! resolve into real-time events and the gate_cv bridge never allocate – a counting allocator over 2,000 blocks of a
//! random walk → arp chain, with a knob turned and a trigger injected along the way.
use deck_core::tempo::TempoMap;
use deck_events::bridge::GateCv;
use deck_events::generators::{Arp, ArpMode, RandomWalk};
use deck_events::rt::{resolve, RtEvent};
use deck_events::{Event, EventGraph, Names, Span, EDGE_CAP};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static CALLS: AtomicUsize = AtomicUsize::new(0);
thread_local! { static ON: Cell<bool> = const { Cell::new(false) }; }

fn counting() -> bool {
    ON.try_with(Cell::get).unwrap_or(false)
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if counting() {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if counting() {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if counting() {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        System.realloc(p, l, n)
    }
}

#[global_allocator]
static A: Counting = Counting;

#[test]
fn a_built_graph_runs_resolves_and_bridges_without_allocating() {
    let mut n = Names::new();
    let (note, pitch, glide) = (n.event("music.note").unwrap(), n.key("pitch").unwrap(), n.key("glide").unwrap());
    let mut g = EventGraph::new();
    let walk = g.add(RandomWalk { note, pitch, glide_key: glide, seed: 3, base_hz: 147.0, spread: 1.0, octaves: 2.0, per_beat: 2.0, length: 0.9, glide: 0.5 });
    let arp = g.add(Arp { note, pitch, ratios: vec![1.0, 1.25, 1.5], octaves: 3, mode: ArpMode::UpDown, per_beat: 8.0, length: 0.5, root: None });
    g.connect((walk, "notes"), (arp, "notes"), false).unwrap();
    g.build().unwrap();
    let spread = g.param(walk, "spread").unwrap();
    let mut cv = GateCv::new(note, pitch, glide, 147.0);
    let mut rt: Vec<RtEvent> = Vec::with_capacity(EDGE_CAP);
    let (mut l, mut r) = (vec![0.0f32; 480], vec![0.0f32; 480]);
    let trigger = Event::now(note).with(pitch, 300.0);
    let tempo = TempoMap::new(140.0, 4, 48_000.0);
    let sizes = [128usize, 37, 480, 256];
    let mut at = 0usize;
    let mut gates = 0usize;
    ON.with(|o| o.set(true));
    for i in 0..2000 {
        let n = sizes[i % 4];
        if i % 300 == 0 {
            g.set(walk, spread, (i % 7) as f32 / 7.0);
            g.inject(arp, "notes", trigger.clone()).unwrap();
        }
        let span = Span::frames(&tempo, at as f64, (at + n) as f64);
        g.run(&span).unwrap();
        rt.clear();
        resolve(g.outputs(arp, "notes").unwrap(), &span, &mut rt);
        cv.render(&rt, &mut l[..n], &mut r[..n]);
        gates += r[..n].iter().filter(|&&x| x > 0.0).count();
        at += n;
    }
    ON.with(|o| o.set(false));
    assert_eq!(CALLS.load(Ordering::SeqCst), 0, "allocations / frees in a run");
    assert!(gates > 0, "it played");
}
