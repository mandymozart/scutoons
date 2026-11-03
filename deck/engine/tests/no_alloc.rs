//! The engine's render never allocates (CYBERDECKS.md §8; moved into deck-engine with the engine, #317): a counting global
//! allocator around Engine::render for the default scene (music player with a track, the sequencer on the tempo clock, FM,
//! noise, the click, mixer, FFT tap, output) – zero allocations and frees over 20 seconds of audio in device-sized blocks,
//! each stamped for the time base (`Engine::mark`), with the click on and the tempo map changing under it (the frame
//! thread's seqlock writes) – on the batch runtime (#183) with its AVX2 loops (where the CPU has them) and its baseline
//! loops, the saw / square / triangle wavetables, the noise and its lowpass included. The ring, the sequencer's thread
//! and the dispatcher are the native deck's: its tests/no_alloc.rs keeps those.
use deck_engine::{Build, Engine, Track};
use deck_space::graph::Graph;
use deck_space::scene::{Scene, MADNESS_SCENE};
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
    // the engine plays the audio nodes of the scene and ignores the picture's
    let graph = Graph::build(&Scene::parse(MADNESS_SCENE).unwrap()).unwrap();
    for build in [Build::detect(), Build { batch: true, avx2: false }] {
        renders_without_allocating(&graph, build);
    }
}

/// #236: the `walk` node – deck-events' graph, its real-time events and the gate_cv bridge inside the callback –
/// doesn't allocate either
#[cfg(feature = "events")]
#[test]
fn a_walk_into_an_fm_voice_renders_without_allocating() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let scene = Scene::parse(
        "[[node]]\nid = \"tempo\"\nkind = \"tempo\"\n[[node]]\nid = \"melody\"\nkind = \"walk\"\nseed = 3\nspread = 1.0\nrate = 4.0\n\
         [[node]]\nid = \"voice\"\nkind = \"fm\"\ngain = 0.2\n[[node]]\nid = \"fft\"\nkind = \"fft\"\n[[node]]\nid = \"out\"\nkind = \"audio_out\"\n\
         [[link]]\nfrom = \"melody.cv\"\nto = \"voice.cv\"\n[[link]]\nfrom = \"voice\"\nto = \"fft.in\"\n[[link]]\nfrom = \"voice\"\nto = \"out.in\"\n",
    )
    .unwrap();
    let graph = Graph::build(&scene).unwrap();
    for build in [Build::detect(), Build { batch: true, avx2: false }] {
        renders_without_allocating(&graph, build);
    }
}

/// #236: an `events` node – an arp on a walk, a patch – doesn't allocate either
#[cfg(feature = "events")]
#[test]
fn an_events_patch_into_an_fm_voice_renders_without_allocating() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let scene = Scene::parse(
        "[[node]]\nid = \"tempo\"\nkind = \"tempo\"\n[[node]]\nid = \"arp\"\nkind = \"events\"\nb = 8.0\n\
         generator = [{ id = \"walk\", kind = \"random_walk\", seed = 3, rate = 1.0, spread = 1.0 }, \
         { id = \"arp\", kind = \"arp\", octaves = 3, mode = \"updown\", rate = 8.0 }]\n\
         edge = [{ from = \"walk.notes\", to = \"arp.notes\" }]\nout = \"arp.notes\"\nknobs = [\"walk.spread\", \"arp.rate\"]\n\
         [[node]]\nid = \"voice\"\nkind = \"fm\"\ngain = 0.2\n[[node]]\nid = \"fft\"\nkind = \"fft\"\n[[node]]\nid = \"out\"\nkind = \"audio_out\"\n\
         [[link]]\nfrom = \"arp.cv\"\nto = \"voice.cv\"\n[[link]]\nfrom = \"voice\"\nto = \"fft.in\"\n[[link]]\nfrom = \"voice\"\nto = \"out.in\"\n",
    )
    .unwrap();
    let graph = Graph::build(&scene).unwrap();
    for build in [Build::detect(), Build { batch: true, avx2: false }] {
        renders_without_allocating(&graph, build);
    }
}


/// the wavetable oscillators, the white and pink noise and their lowpass allocate nothing either
#[test]
fn the_waves_and_the_noise_render_without_allocating() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut text = String::new();
    for (i, wave) in ["saw", "square", "triangle"].iter().enumerate() {
        text += &format!("[[node]]\nid = \"w{i}\"\nkind = \"osc\"\nwave = \"{wave}\"\nfreq = {}\ngain = 0.1\n[[link]]\nfrom = \"w{i}\"\nto = \"m.in{}\"\n", 55 * (i + 1), i + 1);
    }
    text += "[[node]]\nid = \"n\"\nkind = \"noise\"\ncolor = \"pink\"\ncutoff = 3000.0\ngain = 0.1\n[[link]]\nfrom = \"n\"\nto = \"m.in4\"\n\
             [[node]]\nid = \"v\"\nkind = \"fm\"\ngain = 0.1\n[[node]]\nid = \"fft\"\nkind = \"fft\"\n[[node]]\nid = \"m\"\nkind = \"mixer\"\n[[node]]\nid = \"out\"\nkind = \"audio_out\"\n\
             [[link]]\nfrom = \"m\"\nto = \"fft.in\"\n[[link]]\nfrom = \"m\"\nto = \"out.in\"\n";
    let graph = Graph::build(&Scene::parse(&text).unwrap()).unwrap();
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

/// P4b (#318): 32 channels cost no allocation either – a panner over 32 speakers into a 32-wide bus, a send with a channel
/// map, a 32-channel feedback loop, a gain matrix 32 → 2, outlets on a 32-channel device, the limiter groups, the meters
#[test]
fn thirty_two_channels_render_without_allocating() {
    use deck_engine::{DeckConfig, Seams, Sequencing};
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let az: Vec<String> = (0..32).map(|k| format!("{}", -180.0 + k as f32 * 360.0 / 32.0)).collect();
    let matrix: Vec<String> = (0..64).map(|k| if k % 32 == k / 32 { "0.5".to_string() } else { "0.0".to_string() }).collect();
    let text = format!(
        "[[node]]\nid = \"a\"\nkind = \"osc\"\nfreq = 220.0\ngain = 0.05\n[[node]]\nid = \"b\"\nkind = \"fm\"\nfreq = 110.0\ngain = 0.05\n\
         [[node]]\nid = \"pan\"\nkind = \"pan\"\nmode = \"vbap\"\nazimuth = 33.0\naz = [{az}]\n\
         [[node]]\nid = \"bed\"\nkind = \"bus\"\nchannels = 32\n[[node]]\nid = \"loop\"\nkind = \"feedback\"\nchannels = 32\n\
         [[node]]\nid = \"send\"\nkind = \"bus\"\n[[node]]\nid = \"dm\"\nkind = \"downmix\"\ninputs = 32\nmatrix = [{matrix}]\n\
         [[node]]\nid = \"main\"\nkind = \"out\"\nfirst = 1\n[[node]]\nid = \"cue\"\nkind = \"out\"\ngroup = \"cue\"\nfirst = 1\n\
         [[link]]\nfrom = \"a\"\nto = \"pan.in\"\n[[link]]\nfrom = \"pan.out\"\nto = \"bed.in\"\n[[link]]\nfrom = \"loop.out\"\nto = \"bed.in\"\nscale = 0.1\n\
         [[link]]\nfrom = \"bed.out\"\nto = \"loop.in\"\n[[link]]\nfrom = \"b\"\nto = \"send.in\"\nmap = [[1, 30], [2, 31]]\n[[link]]\nfrom = \"send.out\"\nto = \"bed.in\"\n\
         [[link]]\nfrom = \"bed.out\"\nto = \"main.in\"\n[[link]]\nfrom = \"bed.pre\"\nto = \"dm.in\"\n[[link]]\nfrom = \"dm.out\"\nto = \"cue.in\"\n",
        az = az.join(", "),
        matrix = matrix.join(", ")
    );
    let graph = Graph::build(&Scene::parse(&text).unwrap()).unwrap();
    for build in [Build::detect(), Build { batch: true, avx2: false }] {
        let seams = Seams::with_deck(DeckConfig::device(32));
        let (mut engine, handles) = Engine::build_seams(&graph, 48_000.0, 1.0, build, Sequencing::InBlock, &seams).unwrap();
        assert_eq!(engine.device_channels(), 32);
        let mut bufs: Vec<Vec<f32>> = [128usize, 37, 480, 1000].iter().map(|n| vec![0.0; n * 32]).collect();
        measure(true);
        for i in 0..1000 {
            let b = &mut bufs[i % 4];
            engine.mark(i as u64 * 1_000_000);
            engine.render(b, 32, |x| x);
        }
        measure(false);
        assert_eq!(CALLS.load(Ordering::SeqCst), 0, "allocations / frees with 32 channels ({})", build.label());
        let loud = (0..32).filter(|&c| handles.device.peak(c) > 1e-4).count();
        assert!(loud >= 3, "the 32 speakers carry the panned source (and the send): {loud} channels are loud");
    }
}
