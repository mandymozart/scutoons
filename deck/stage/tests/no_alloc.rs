//! The audio callback never allocates (CYBERDECKS.md §8): a counting global allocator around Engine::render for the
//! default scene (music player with a track, the sequencer on the tempo clock, FM, noise, the click, mixer, FFT tap,
//! output) – zero allocations and frees over 20 seconds of audio in device-sized blocks, each stamped for the time base
//! (`Engine::mark`), with the click on and the tempo map changing under it (the frame thread's seqlock writes) – on the
//! batch runtime (#183) with its AVX2 loops (where the CPU has them) and its baseline loops.
use deck_stage::audio::{Build, Engine, Track};
use deck_stage::graph::Graph;
use deck_stage::scene::{Scene, DEFAULT_SCENE};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct Counting;
static CALLS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// counting on this thread only: the test harness's main thread prints the previous test's result ("test … ok")
    /// while the next one measures – process-wide, those allocations landed in CALLS (#186, CI on Linux and macOS). A
    /// const Cell needs no lazy init and no destructor, so reading it never allocates.
    static MEASURED: Cell<bool> = const { Cell::new(false) };
}

fn counting() -> bool {
    MEASURED.try_with(Cell::get).unwrap_or(false)
}

/// count this thread's allocations from now on (true) or no longer (false)
fn measure(on: bool) {
    MEASURED.with(|m| m.set(on));
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
/// CALLS is shared: the tests take turns
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
    measure(true);
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
    measure(false);
    assert_eq!(CALLS.load(Ordering::SeqCst), 0, "allocations / frees in the audio path ({})", build.label());
    assert!(drained > 0);
    assert!(bufs[0].iter().any(|x| *x != 0.0), "the scene makes sound");
}

/// The being's frame (#188) never allocates either: the control step, the body's pose, the hoses' bubbles, the buttons'
/// values, the head's energy and mood, picking, a button turned like a CC and the geometry written into the renderer's
/// preallocated buffers; and a deck's being at the conductor on its stream's values (#188 phase 4, `Mirror`); the
/// scene's feelings – every rule tested, moods turning, their nudges pushed into the control domain (feel.rs); the
/// buttons' feedback (#188, owner 2026-10-31): their tethers, hover, a hand on one, a linked one shaking, a double
/// click's ease back; #188: the value rings, a linked knob turned as its base, the points' levels, the head's strain.
#[cfg(feature = "space")]
#[test]
fn the_being_steps_without_allocating() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    use deck_stage::control::Controls;
    use deck_stage::scene::CATHEDRAL_SCENE;
    use deck_stage::space::being::{Being, Hit};
    use deck_stage::space::camera::Orbit;
    use deck_stage::space::values::Local;
    use deck_stage::tempo;
    let scene = deck_stage::modules::carried_part(&Scene::parse(CATHEDRAL_SCENE).unwrap());
    let graph = Graph::build(&scene).unwrap();
    let mut ctl = Controls::new(&graph, None);
    let mut being = Being::new(&graph, &scene.being);
    // phase 4: a deck's being at the conductor, on its stream's values
    let mut remote = Being::new(&graph, &scene.being);
    let mut mirror = deck_stage::space::values::Mirror::new(&graph);
    assert!(mirror.apply(&graph, "fft.level", &[0.8]) && mirror.apply(&graph, "voice.index", &[3.0]));
    let (mut rs, mut rt) = { let (a, b) = remote.capacity(); (Vec::with_capacity(a), Vec::with_capacity(b)) };
    let (ns, nv) = being.capacity();
    let (mut spheres, mut tubes) = (Vec::with_capacity(ns), Vec::with_capacity(nv));
    let cam = Orbit::new(being.anchor, 0.3, 0.1, 2.0);
    let knob = being.buttons.iter().find(|b| !b.linked).map(|b| (b.node, b.port)).unwrap();
    let hand = being.buttons.iter().position(|b| !b.linked && (b.node, b.port) != knob).unwrap();
    let linked = being.buttons.iter().position(|b| b.linked).unwrap();
    let mut shown = 0;
    let meter = |n: usize, p: usize| ((n * 7 + p) % 5) as f32 * 0.2;
    let (rules, nudges) = being.feel.counts();
    assert!(rules >= 20 && nudges >= 20 && being.feel.problems.is_empty(), "{:?}", being.feel.problems);
    let mut turns = 0;
    measure(true);
    let before = CALLS.load(Ordering::SeqCst);
    for f in 0..600 {
        let beat = tempo::Now { bpm: 84.0, pos: tempo::position(f as f64 * 0.023, 4), per_bar: 4, running: true };
        ctl.update(1.0 / 60.0, 0, &beat);
        ctl.set(knob.0, knob.1, (f % 100) as f32 * 0.01);
        let before = (being.head.mood, remote.head.mood);
        // a hand on a button for half a second; before it the pointer over it; a linked one grabbed; a double click
        being.hover = match f {
            150..230 => Some(Hit::Button(hand)),
            300..330 => Some(Hit::Button(linked)),
            _ => None,
        };
        being.dragging = (200..230).contains(&f) || (300..330).contains(&f);
        if f == 300 {
            being.shake(linked);
        }
        if f == 400 {
            being.reset(hand);
        }
        if (300..330).contains(&f) {
            // #188: the linked knob turns as its base
            let b = &being.buttons[linked];
            ctl.set(b.node, b.port, b.spec.value((f - 300) as f32 / 30.0));
        }
        being.strain(1.0 / 60.0, if (100..140).contains(&f) { 9.0 } else { 0.0 });
        being.update(1.0 / 60.0, f as f32 / 60.0, &Local { ctl: &ctl, meter: &meter }, &beat);
        being.push(&mut ctl);
        // the deck's sound swells and drops: its rules see jumps, quiet and noise
        let loud = ((f / 90) % 2) as f32;
        mirror.apply(&graph, "fft.level", &[0.9 * loud]);
        mirror.apply(&graph, "fft.bass", &[0.8 * loud]);
        turns += usize::from(being.head.mood != before.0) + usize::from(remote.head.mood != before.1);
        let _ = being.pick(&cam.ray(640.0 + f as f32, 360.0, 1280.0, 720.0));
        being.write(&mut spheres, &mut tubes);
        // the deck's knob jumps on the stream: the mirrored being moves it
        mirror.apply(&graph, "voice.index", &[if (f / 60) % 2 == 0 { 1.0 } else { 7.0 }]);
        remote.update(1.0 / 60.0, f as f32 / 60.0, &mirror, &beat);
        remote.write(&mut rs, &mut rt);
        shown += spheres.iter().filter(|s| s.extra[0] >= deck_stage::space::being::kind::GAUGE).count();
    }
    measure(false);
    assert_eq!(CALLS.load(Ordering::SeqCst) - before, 0, "allocations / frees in the being's frame");
    assert!(being.hoses.iter().any(|h| h.bubbles.iter().any(|b| b.size > 0.0)), "it bubbles");
    assert!(turns >= 3, "the feelings turn the mood: {turns}");
    assert!(shown >= 600 * being.buttons.len(), "every knob wears its value ring every frame: {shown}");
}

/// The outlet's frame (#186) never allocates: the crossfade's numbers per frame and the empty polls of the composer's
/// channels (a new plane is an upload, like a chapter's cut – not every frame).
#[test]
fn the_outlet_frame_does_not_allocate() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    use deck_stage::live::frame::{Crossfade, Frame, SpaceTarget};
    let (_frames_tx, frames) = std::sync::mpsc::sync_channel::<Frame>(2);
    let (_space_tx, space) = std::sync::mpsc::channel::<SpaceTarget>();
    let mut fade = Crossfade::default();
    fade.cut(0.0, true, 1200, 300);
    let mut sum = 0.0;
    measure(true);
    let before = CALLS.load(Ordering::SeqCst);
    for f in 0..600 {
        let t = f as f32 / 60.0;
        if f == 300 {
            fade.cut(t, false, 600, 600);
        }
        assert!(frames.try_recv().is_err() && space.try_recv().is_err());
        sum += fade.mix(t).iter().sum::<f32>();
    }
    measure(false);
    assert_eq!(CALLS.load(Ordering::SeqCst) - before, 0, "allocations / frees in the outlet's frame");
    assert!(sum > 0.0);
}
