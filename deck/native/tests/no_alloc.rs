//! The audio callback never allocates (CYBERDECKS.md §8): a counting global allocator around Engine::render for the
//! default scene (music player with a track, the sequencer on the tempo clock, FM, noise, the click, mixer, FFT tap,
//! output) – zero allocations and frees over 20 seconds of audio in device-sized blocks, each stamped for the time base
//! (`Engine::mark`), with the click on and the tempo map changing under it (the frame thread's seqlock writes) – on the
//! batch runtime (#183) with its AVX2 loops (where the CPU has them) and its baseline loops.
use deck_native::audio::{Build, Engine, Track};
use deck_native::graph::Graph;
use deck_native::scene::{Scene, MADNESS_SCENE};
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

/// #236: the sequencer path – the audio block taking its events from the ring, the sequencer's windows, the
/// dispatcher routing an adapter's notes into the patch's `listen` input – allocates nowhere: 2000 blocks of 256
/// frames, the sequencer filled a horizon ahead, a note routed in every 50 blocks, all measured on this thread.
#[cfg(feature = "events")]
#[test]
fn the_sequencer_and_the_dispatcher_do_not_allocate() {
    use deck_events::Event;
    use deck_native::audio::dispatch::Dispatcher;
    use deck_native::audio::Sequencing;
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let scene = Scene::parse(
        "[[node]]\nid = \"tempo\"\nkind = \"tempo\"\n[[node]]\nid = \"arp\"\nkind = \"events\"\nb = 8.0\n\
         generator = [{ id = \"walk\", kind = \"random_walk\", seed = 3, rate = 1.0, spread = 1.0 }, \
         { id = \"arp\", kind = \"arp\", octaves = 3, mode = \"updown\", rate = 8.0 }]\n\
         edge = [{ from = \"walk.notes\", to = \"arp.notes\" }]\nout = \"arp.notes\"\nknobs = [\"walk.spread\", \"arp.rate\"]\n\
         listen = [{ to = \"arp.notes\", name = \"music.note\" }]\n\
         [[node]]\nid = \"voice\"\nkind = \"fm\"\ngain = 0.2\n[[node]]\nid = \"out\"\nkind = \"audio_out\"\n\
         [[link]]\nfrom = \"arp.cv\"\nto = \"voice.cv\"\n[[link]]\nfrom = \"voice\"\nto = \"out.in\"\n",
    )
    .unwrap();
    let graph = Graph::build(&scene).unwrap();
    let (mut engine, mut h) = Engine::build_seq(&graph, 48_000.0, 1.0, Build::detect(), Sequencing::Thread).unwrap();
    let mut seq = h.events.take_sequencer().expect("the host runs on the sequencer");
    let (mut dispatcher, inlet) = Dispatcher::new(h.events.take_routing(), h.events.counters.clone());
    let (note, pitch) = (h.events.names.get("music.note").unwrap(), h.events.names.get("pitch").unwrap());
    let mut buf = vec![0.0f32; 256 * 2];
    let ahead = deck_native::audio::sequencer::horizon(&h.glitches);
    measure(true);
    let before = CALLS.load(Ordering::SeqCst);
    for i in 0..2000u64 {
        if i % 50 == 0 {
            inlet.send(Event::now(note).with(pitch, 110.0 + i as f32 / 10.0));
        }
        dispatcher.drain();
        seq.fill(ahead);
        engine.mark(1_000_000_000 + i * 5_333_333);
        engine.render(&mut buf, 2, |x| x);
    }
    measure(false);
    assert_eq!(CALLS.load(Ordering::SeqCst) - before, 0, "allocations / frees on the sequencer path");
    let c = &h.events.counters;
    assert_eq!(c.routed.load(Ordering::Relaxed), 40, "every note routed");
    assert_eq!(c.dropped(), 0, "nothing late or lost");
    assert!(buf.iter().any(|x| *x != 0.0), "the patch plays through the ring");
}

/// #208: the render thread's fill (blocks into the ring, the adaptive lead) and the device callback's copy out of it
/// never allocate either – 2000 callbacks of device-sized blocks through the ring, the click on.
#[test]
fn the_ring_and_its_render_thread_do_not_allocate() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let scene = deck_native::modules::carried_part(&Scene::parse(MADNESS_SCENE).unwrap());
    let graph = Graph::build(&scene).unwrap();
    let (engine, mut handles) = Engine::build_with(&graph, 48_000.0, 1.0, Build::detect()).unwrap();
    if let Some(m) = handles.music.first_mut() {
        m.tracks.push(Arc::new(Track { rate: 44_100.0, l: vec![0.1; 44_100], r: vec![-0.1; 44_100] })).ok().unwrap();
    }
    let (mut feed, mut renderer) = deck_native::audio::ring::pair(engine, Some(480));
    let mut bufs: Vec<Vec<f32>> = [480usize, 128, 1056, 37].iter().map(|n| vec![0.0; n * 2]).collect();
    let (map, _) = handles.tempo.map().unwrap();
    handles.tempo.publish(&map, true);
    measure(true);
    let before = CALLS.load(Ordering::SeqCst);
    for i in 0..2000u64 {
        renderer.fill(1_000_000_000 + i * 10_000_000);
        let b = &mut bufs[i as usize % 4];
        feed.play(b, 2, 1_000_000_000 + i * 10_000_000, |x| x);
        while handles.taps[0].1.pop().is_ok() {}
    }
    measure(false);
    assert_eq!(CALLS.load(Ordering::SeqCst) - before, 0, "allocations / frees in the ring's two halves");
    assert!(bufs[0].iter().any(|x| *x != 0.0), "the scene makes sound through the ring");
}

/// The being's frame (#188) never allocates either: the control step, the body's pose, the hoses' bubbles, the buttons'
/// values, the head's energy and mood, picking, a button turned like a CC and the geometry written into the renderer's
/// preallocated buffers; and a deck's being at the conductor on its stream's values (#188 phase 4, `Mirror`); the
/// scene's feelings – every rule tested, moods turning, their nudges pushed into the control domain (feel.rs); the
/// buttons' feedback (#188, owner 2026-10-31): their tethers, hover, a hand on one, a linked one shaking, a double
/// click's ease back; #188: the value rings, a linked knob turned as its base, the points' levels, the head's strain;
/// #209: the knobs facing an orbiting camera, the selection's rings.
#[cfg(feature = "space")]
#[test]
fn the_being_steps_without_allocating() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    use deck_native::control::Controls;
    use deck_native::scene::CATHEDRAL_SCENE;
    use deck_native::space::being::{Being, Hit};
    use deck_native::space::camera::Orbit;
    use deck_native::space::values::Local;
    use deck_native::tempo;
    let scene = deck_native::modules::carried_part(&Scene::parse(CATHEDRAL_SCENE).unwrap());
    let graph = Graph::build(&scene).unwrap();
    let mut ctl = Controls::new(&graph, None);
    let mut being = Being::new(&graph, &scene.being);
    // phase 4: a deck's being at the conductor, on its stream's values
    let mut remote = Being::new(&graph, &scene.being);
    let mut mirror = deck_native::space::values::Mirror::new(&graph);
    assert!(mirror.apply(&graph, "fft.level", &[0.8]) && mirror.apply(&graph, "voice.index", &[3.0]));
    let (mut rs, mut rt) = { let (a, b) = remote.capacity(); (Vec::with_capacity(a), Vec::with_capacity(b)) };
    let (ns, nv) = being.capacity();
    let (mut spheres, mut tubes) = (Vec::with_capacity(ns), Vec::with_capacity(nv));
    let mut cam = Orbit::new(being.anchor, 0.3, 0.1, 2.0);
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
        // #209: the camera orbits – the knobs re-fan toward it and keep out of the core's and the head's shadow; the
        // selection's ring (a knob, then the whole being)
        cam.yaw = 0.3 + f as f32 * 0.02;
        being.eye = Some(cam.eye());
        remote.eye = being.eye;
        being.selected = match f {
            0..200 => Some(Hit::Button(hand)),
            200..400 => Some(Hit::Point(0)),
            _ => None,
        };
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
        shown += spheres.iter().filter(|s| s.extra[0] >= deck_native::space::being::kind::GAUGE).count();
    }
    measure(false);
    assert_eq!(CALLS.load(Ordering::SeqCst) - before, 0, "allocations / frees in the being's frame");
    assert!(being.hoses.iter().any(|h| h.bubbles.iter().any(|b| b.size > 0.0)), "it bubbles");
    assert!(turns >= 3, "the feelings turn the mood: {turns}");
    assert!(shown >= 600 * being.buttons.len(), "every knob wears its value ring every frame: {shown}");
}

/// #204: the post stack's frame (its knobs read from the controls, the plan of the passes that run) and the glow's
/// lights (the beings' cores chosen strongest first) never allocate – the conductor with ten decks' beings.
#[cfg(feature = "space")]
#[test]
fn the_post_plan_and_the_lights_do_not_allocate() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    use deck_native::control::Controls;
    use deck_native::graph::Kind;
    use deck_native::scene::CONDUCTOR_SCENE;
    use deck_native::space::decks::Remote;
    use deck_native::space::portrait::Portraits;
    use deck_native::space::post::Knobs;
    use deck_native::space::{Space, MAX_LIGHTS};
    use deck_native::tempo;
    let scene = deck_native::modules::carried_part(&Scene::parse(CONDUCTOR_SCENE).unwrap());
    let graph = Graph::build(&scene).unwrap();
    let node = graph.first(deck_native::graph::Kind::Space).unwrap();
    let mut ctl = Controls::new(&graph, None);
    let mut space = Space::new(&graph, node, &scene.being, std::path::Path::new("/nowhere"), Portraits::manual());
    for k in 0..10 {
        let mut r = Remote::new(format!("deck{k}"), graph.clone(), &scene.being);
        r.being.center.x = k as f32;
        space.others.push(r);
    }
    assert_eq!(space.post.effects().count(), 4, "the conductor lists the four passes");
    let mut lights = [[0.0f32; 8]; MAX_LIGHTS];
    let (mut ran, mut lit) = (0, 0);
    measure(true);
    let before = CALLS.load(Ordering::SeqCst);
    for f in 0..600 {
        let beat = tempo::Now { bpm: 84.0, pos: tempo::position(f as f64 * 0.023, 4), per_bar: 4, running: true };
        ctl.update(1.0 / 60.0, 0, &beat);
        // a knob through the passes: bloom off and on, the pixels coarser
        ctl.set(node, Kind::Space.input("bloom").unwrap(), (f % 120) as f32 / 60.0);
        ctl.set(node, Kind::Space.input("pixel").unwrap(), 1.0 + (f % 7) as f32);
        let k = Knobs::read(&ctl, node);
        ran += space.post.plan(&k).passes().len();
        space.being.glow = (f % 50) as f32 * 0.04;
        let mut eye = space.camera.eye();
        eye.x = (f % 13) as f32;
        lit += space.write_lights(eye, &mut lights);
    }
    measure(false);
    assert_eq!(CALLS.load(Ordering::SeqCst) - before, 0, "allocations / frees in the post plan and the lights");
    assert!(ran > 600, "passes ran: {ran}");
    assert_eq!(lit, 600 * MAX_LIGHTS, "eight lights every frame");
}

/// #220: the quality governor's frame, its reports, the post plan at a rung and the conductor's labels (each deck's sync,
/// quality and link) never allocate – a load that steps it down and back up over ten minutes of frames.
#[cfg(feature = "space")]
#[test]
fn the_quality_governor_and_the_conductors_labels_do_not_allocate() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    use deck_space::quality::{Governor, Level};
    use deck_native::scene::CONDUCTOR_SCENE;
    use deck_native::space::decks::Remote;
    use deck_native::space::label::{Label, LABELS};
    use deck_native::space::portrait::Portraits;
    use deck_native::space::post::Knobs;
    use deck_native::space::Space;
    let scene = deck_native::modules::carried_part(&Scene::parse(CONDUCTOR_SCENE).unwrap());
    let graph = Graph::build(&scene).unwrap();
    let node = graph.first(deck_native::graph::Kind::Space).unwrap();
    let mut space = Space::new(&graph, node, &scene.being, std::path::Path::new("/nowhere"), Portraits::manual());
    for k in 0..8 {
        let mut r = Remote::new(format!("deck{k}"), graph.clone(), &scene.being);
        r.quality = Some(protocol::DeckQuality { level: protocol::QualityLevel::Medium, scale: 0.8, auto: true, frame: 17.0 + k as f64 * 4.0, ceiling: Some(protocol::QualityLevel::High) });
        r.link = Some(protocol::LinkQuality { rtt: Some(3 + k * 20), hz: 30, skipped: Some(0) });
        space.others.push(r);
    }
    let mut g = Governor::new(Level::High, true);
    let mut labels: Vec<Label> = Vec::with_capacity(LABELS);
    let k = Knobs { gamma: 1.1, ..Knobs::default() };
    let (mut steps, mut reports, mut ran) = (0, 0, 0);
    let mut t = 0.0f64;
    measure(true);
    let before = CALLS.load(Ordering::SeqCst);
    for f in 0..36_000u32 {
        // 60 fps, with a minute at 40 fps in the middle
        let ms = if (12_000..15_600).contains(&f) { 25.0 } else { 16.7 };
        t += ms as f64 / 1000.0;
        steps += g.frame(t, ms, 0) as usize;
        reports += g.report_due(t).is_some() as usize;
        let r = g.rung();
        ran += space.post.plan_quality(&k, r.bloom, r.fxaa).passes().len();
        if f % 60 == 0 {
            space.write_labels(&mut labels);
        }
    }
    measure(false);
    assert_eq!(CALLS.load(Ordering::SeqCst) - before, 0, "allocations / frees in the governor, its reports, the plan, the labels");
    assert!(steps >= 2, "it stepped down and back up: {steps}");
    assert_eq!(g.level(), Level::High, "back at the level after the calm");
    assert!(reports >= steps && ran > 0);
    assert_eq!(labels.len(), 16, "each deck's quality and link");
}

/// The outlet's frame (#186) never allocates: the crossfade's numbers per frame and the empty polls of the composer's
/// channels (a new plane is an upload, like a chapter's cut – not every frame).
#[test]
fn the_outlet_frame_does_not_allocate() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    use deck_native::live::frame::{Crossfade, Frame, SpaceTarget};
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
