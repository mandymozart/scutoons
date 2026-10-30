//! The audio callback never allocates (CYBERDECKS.md §8): a counting global allocator around Engine::render for the
//! default scene (music player with a track, the sequencer on the tempo clock, FM, noise, the click, mixer, FFT tap,
//! output) – zero allocations and frees over 20 seconds of audio in device-sized blocks, each stamped for the time base
//! (`Engine::mark`), with the click on and the tempo map changing under it (the frame thread's seqlock writes) – on the
//! batch runtime (#183) with its AVX2 loops (where the CPU has them) and its baseline loops.
use deck_stage::audio::{Build, Engine, Track};
use deck_stage::graph::Graph;
use deck_stage::scene::{Scene, DEFAULT_SCENE};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

struct Counting;
static COUNTING: AtomicBool = AtomicBool::new(false);
static CALLS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if COUNTING.load(Ordering::Relaxed) {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        System.realloc(p, l, n)
    }
}

#[global_allocator]
static A: Counting = Counting;
/// the counter is global: the tests take turns
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn the_default_scene_renders_without_allocating() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    // a build without a module plays the scene without that module's nodes (#185: Graph::build refuses them)
    let scene = deck_stage::modules::carried_part(&Scene::parse(DEFAULT_SCENE).unwrap());
    let graph = Graph::build(&scene).unwrap();
    for build in [Build::detect(), Build { batch: true, avx2: false }] {
        renders_without_allocating(&graph, build);
    }
}

fn renders_without_allocating(graph: &Graph, build: Build) {
    let (mut engine, mut handles) = Engine::build_with(graph, 48_000.0, 1.0, build).unwrap();
    assert!(engine.shape().1 > 0, "the FM voice runs as a batch");
    let track = Track { rate: 44_100.0, l: vec![0.1; 44_100], r: vec![-0.1; 44_100] };
    if let Some(m) = handles.music.first_mut() {
        m.tracks.push(Arc::new(track)).ok().unwrap();
    }
    // 128 frames like the deck's PipeWire quantum, plus odd sizes and one bigger than MAX_BLOCK
    let mut bufs: Vec<Vec<f32>> = [128usize, 37, 480, 3000].iter().map(|n| vec![0.0; n * 2]).collect();
    let mut drained = 0usize;
    let (map, _) = handles.tempo.map().unwrap();
    handles.tempo.publish(&map, true); // the click on
    COUNTING.store(true, Ordering::SeqCst);
    for i in 0..2000 {
        if i % 250 == 0 {
            // what the frame thread does: a new tempo from the frontier on
            let (m, click) = handles.tempo.map().unwrap();
            let frontier = handles.clock.rendered() as f64;
            handles.tempo.publish(&m.with_bpm(frontier, 80.0 + (i / 25) as f64), click);
        }
        let b = &mut bufs[i % 4];
        engine.mark(i as u64 * 1_000_000); // the block's stamp (the time base's seqlock)
        engine.render(b, 2, |x| x);
        // the frame thread's side: the tap ring is drained (pop never allocates either)
        while handles.taps[0].1.pop().is_ok() {
            drained += 1;
        }
    }
    COUNTING.store(false, Ordering::SeqCst);
    assert_eq!(CALLS.load(Ordering::SeqCst), 0, "allocations / frees in the audio path ({})", build.label());
    assert!(drained > 0);
    assert!(bufs[0].iter().any(|x| *x != 0.0), "the scene makes sound");
}

/// The being's frame (#188) never allocates either: the control step, the body's pose, the hoses' bubbles, the buttons'
/// values, picking, a button turned like a CC and the geometry written into the renderer's preallocated buffers.
#[cfg(feature = "space")]
#[test]
fn the_being_steps_without_allocating() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    use deck_stage::control::Controls;
    use deck_stage::scene::CATHEDRAL_SCENE;
    use deck_stage::space::being::Being;
    use deck_stage::space::camera::Orbit;
    use deck_stage::tempo;
    let scene = deck_stage::modules::carried_part(&Scene::parse(CATHEDRAL_SCENE).unwrap());
    let graph = Graph::build(&scene).unwrap();
    let mut ctl = Controls::new(&graph, None);
    let mut being = Being::new(&graph, &scene.being);
    let (ns, nv) = being.capacity();
    let (mut spheres, mut tubes) = (Vec::with_capacity(ns), Vec::with_capacity(nv));
    let cam = Orbit::new(being.anchor, 0.3, 0.1, 2.0);
    let knob = being.buttons.iter().find(|b| !b.linked).map(|b| (b.node, b.port)).unwrap();
    let meter = |n: usize, p: usize| ((n * 7 + p) % 5) as f32 * 0.2;
    COUNTING.store(true, Ordering::SeqCst);
    let before = CALLS.load(Ordering::SeqCst);
    for f in 0..600 {
        let beat = tempo::Now { bpm: 84.0, pos: tempo::position(f as f64 * 0.023, 4), per_bar: 4, running: true };
        ctl.update(1.0 / 60.0, 0, &beat);
        ctl.set(knob.0, knob.1, (f % 100) as f32 * 0.01);
        being.update(1.0 / 60.0, f as f32 / 60.0, &ctl, &beat, &meter);
        let _ = being.pick(&cam.ray(640.0 + f as f32, 360.0, 1280.0, 720.0));
        being.write(&mut spheres, &mut tubes);
    }
    COUNTING.store(false, Ordering::SeqCst);
    assert_eq!(CALLS.load(Ordering::SeqCst) - before, 0, "allocations / frees in the being's frame");
    assert!(being.hoses.iter().any(|h| h.bubbles.iter().any(|b| b.size > 0.0)), "it bubbles");
}
