//! The engine's render never allocates (CYBERDECKS.md §8; moved into deck-engine with the engine, #317): a counting global
//! allocator around Engine::render for the default scene (music player with a track, the sequencer on the tempo clock, FM,
//! noise, the click, mixer, FFT tap, output) – zero allocations and frees over 20 seconds of audio in device-sized blocks,
//! each stamped for the time base (`Engine::mark`), with the click on and the tempo map changing under it (the frame
//! thread's seqlock writes) – on the batch runtime (#183) with its AVX2 loops (where the CPU has them) and its baseline
//! loops, the saw / square / triangle wavetables, the noise and its lowpass included. The control half steps inside the
//! render (#319): the FFT's analysis, the LFOs, the helpers, the snapshot ring and the frame thread's commands (bases,
//! sends, an LFO's shape) cost no allocation either. The ring, the sequencer's thread and the dispatcher are the native
//! deck's: its tests/no_alloc.rs keeps those.
use deck_engine::control::{Context, Controls};
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

/// the singing voice – its glide, vibrato, breath noise and formant updates on a seq's notes – allocates nothing either
#[test]
fn a_singing_voice_renders_without_allocating() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let scene = Scene::parse(
        "[[node]]\nid = \"s\"\nkind = \"seq\"\nbpm = 150.0\nsteps = [64, 0, 67, 71, 76]\n\
         [[node]]\nid = \"sing\"\nkind = \"voice\"\nvowel = 1.5\nglide = 0.08\nbreath = 0.3\ngain = 0.2\n\
         [[node]]\nid = \"sway\"\nkind = \"lfo\"\nrate = 0.7\ndepth = 2.5\n\
         [[node]]\nid = \"v\"\nkind = \"fm\"\ngain = 0.1\n[[node]]\nid = \"fft\"\nkind = \"fft\"\n[[node]]\nid = \"m\"\nkind = \"mixer\"\n[[node]]\nid = \"out\"\nkind = \"audio_out\"\n\
         [[link]]\nfrom = \"s.cv\"\nto = \"sing.cv\"\n[[link]]\nfrom = \"sway\"\nto = \"sing.vowel\"\noffset = 2.5\n\
         [[link]]\nfrom = \"sing\"\nto = \"m.in1\"\n[[link]]\nfrom = \"v\"\nto = \"m.in2\"\n[[link]]\nfrom = \"m\"\nto = \"fft.in\"\n[[link]]\nfrom = \"m\"\nto = \"out.in\"\n",
    )
    .unwrap();
    let graph = Graph::build(&scene).unwrap();
    for build in [Build::detect(), Build { batch: true, avx2: false }] {
        renders_without_allocating(&graph, build);
    }
}

/// #319: the control half at block rate – LFOs of every shape on two levels, the helpers, a smoother on an FFT, an LFO
/// into an oscillator's pitch and gain – with the frame thread turning bases, sending values and switching LFO shapes
/// between the blocks
#[test]
fn the_control_half_steps_without_allocating() {
    let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let scene = Scene::parse(
        "[[node]]
id = \"t\"
kind = \"tempo\"
[[node]]
id = \"a\"
kind = \"lfo\"
rate = 3.0
[[node]]
id = \"b\"
kind = \"lfo\"
shape = \"saw\"
         [[node]]
id = \"c\"
kind = \"lfo\"
shape = \"square\"
[[node]]
id = \"m\"
kind = \"mul\"
b = 200.0
[[node]]
id = \"s\"
kind = \"smooth\"
time = 0.2
         [[node]]
id = \"p\"
kind = \"add\"
b = 0.1
[[node]]
id = \"o\"
kind = \"osc\"
[[node]]
id = \"fft\"
kind = \"fft\"
[[node]]
id = \"out\"
kind = \"audio_out\"
         [[link]]
from = \"a\"
to = \"b.rate\"
[[link]]
from = \"t.phase\"
to = \"c.depth\"
[[link]]
from = \"b\"
to = \"m.a\"
[[link]]
from = \"m\"
to = \"o.freq\"
offset = 220.0
         [[link]]
from = \"fft.level\"
to = \"s.in\"
[[link]]
from = \"s\"
to = \"p.a\"
[[link]]
from = \"p\"
to = \"o.gain\"
         [[link]]
from = \"o\"
to = \"fft.in\"
[[link]]
from = \"o\"
to = \"out.in\"
",
    )
    .unwrap();
    let graph = Graph::build(&scene).unwrap();
    for build in [Build::detect(), Build { batch: true, avx2: false }] {
        let (mut engine, mut handles) = Engine::build_with(&graph, 48_000.0, 1.0, build).unwrap();
        let mut face = Controls::new(&graph, Some(&mut handles));
        let mut buf = vec![0.0f32; 480 * 2];
        // the face's first commands (their ring is made at build; a push never allocates)
        measure(true);
        for i in 0..1000 {
            match i % 5 {
                0 => face.set_base(7, 0, 200.0 + i as f32),
                1 => face.send(4, 1, 100.0, (i % 7) as f32 / 7.0),
                2 => face.set_lfo_shape(1 + i % 3, i / 4 % 3),
                // #409: the live context through the same ring, handed to the blocks at the next block's start
                3 => {
                    face.set_context(Context::new(if i % 10 == 3 { "sad" } else { "laughing" }, "dream", (i % 9) as f32 / 9.0, 0.0, 0.0));
                    true
                }
                _ => face.set_base(5, 1, 0.05),
            };
            engine.render(&mut buf, 2, |x| x);
        }
        measure(false);
        assert_eq!(CALLS.load(Ordering::SeqCst), 0, "allocations / frees in the control half ({})", build.label());
        assert!(engine.control().outputs(8)[0] > 0.1, "the FFT hears the oscillator");
        assert!(buf.iter().any(|x| *x != 0.0));
    }
}

fn renders_without_allocating(graph: &Graph, build: Build) {
    let (mut engine, mut handles) = Engine::build_with(graph, 48_000.0, 1.0, build).unwrap();
    let fft = graph.first(deck_space::graph::Kind::Fft).expect("an fft node");
    assert!(engine.shape().1 > 0, "the FM voice runs as a batch");
    let track = Track { rate: 44_100.0, l: vec![0.1; 44_100], r: vec![-0.1; 44_100] };
    if let Some(m) = handles.music.first_mut() {
        m.tracks.push(Arc::new(track)).ok().unwrap();
    }
    // 128 frames like the deck's PipeWire quantum, plus odd sizes and one bigger than MAX_BLOCK
    let mut bufs: Vec<Vec<f32>> = [128usize, 37, 480, 3000].iter().map(|n| vec![0.0; n * 2]).collect();
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
    }
    measure(false);
    assert_eq!(CALLS.load(Ordering::SeqCst), 0, "allocations / frees in the audio path ({})", build.label());
    assert!(engine.control().outputs(fft)[0] > 0.0, "the control half's analyzer drained the FFT's ring");
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

/// A held MIDI chord through a MIDI world node's port and channel into a module's note input: the event host's live
/// notes (on at key down, off at key up) cost no allocation – a stand-in module that counts what it is given
#[cfg(all(feature = "events", feature = "modules"))]
mod live_notes {
    use super::*;
    use deck_engine::{register_module, DeckConfig, ModuleInstance, ModuleSource, Opened, Seams, Sequencing};
    use deck_module_host::{Ev, EvKind, InBind, Layout};
    use std::sync::atomic::AtomicU32;

    const KEYS: &str = r#"{"abi":2,"name":"t_keys","version":"1","params":[],"ports":[
        {"name":"notes","dir":"in","type":"event","names":["music.note"],"capacity":16},
        {"name":"out","dir":"out","type":"audio","channels":2}]}"#;

    static ONS: AtomicU32 = AtomicU32::new(0);
    static OFFS: AtomicU32 = AtomicU32::new(0);

    struct Keys;

    impl ModuleInstance for Keys {
        fn silenced(&self) -> bool {
            false
        }
        fn set_character(&mut self, _: &str, _: &str, _: &str) {}
        fn set_vars(&mut self, _: &[(String, f32)]) {}
        fn set_tempo(&mut self, _: &deck_core::tempo::TempoMap) {}
        fn set_param(&mut self, _: usize, _: f32) {}
        fn set_control_in(&mut self, _: usize, _: f32) {}
        fn set_audio_in(&mut self, _: usize, _: &[&[f32]]) {}
        fn process(&mut self, _: f64, _: u32) -> bool {
            true
        }
        fn read_audio_out(&mut self, _: usize, _: usize, dst: &mut [f32]) {
            dst.fill(0.0);
        }
        fn set_events_in(&mut self, _: usize, events: &[Ev]) -> usize {
            for e in events {
                match e.kind() {
                    Some(EvKind::NoteOn) => ONS.fetch_add(1, Ordering::Relaxed),
                    Some(EvKind::NoteOff) => OFFS.fetch_add(1, Ordering::Relaxed),
                    _ => 0,
                };
            }
            events.len()
        }
    }

    struct Source(Layout);

    impl ModuleSource for Source {
        fn open(&self, _: deck_space::modreg::ModuleKind, _: f64, _: u64, _: Vec<deck_module_host::AssetData>) -> anyhow::Result<Opened> {
            let params = self.0.inputs.iter().filter(|b| matches!(b, InBind::Param(_))).count();
            Ok(Opened { instance: Box::new(Keys), layout: self.0.clone(), params })
        }
    }

    #[test]
    fn a_held_midi_chord_into_a_module_renders_without_allocating() {
        use deck_events::adapters::midi::Midi;
        let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let d: deck_catalog::Descriptor = serde_json::from_str(KEYS).unwrap();
        let layout = Layout::of(&d).unwrap();
        register_module(&d, &layout);
        let scene = "[[node]]\nid=\"w\"\nkind=\"world\"\nsource=\"midi\"\nlisten=[\"midi.note\"]\nport=1\nchannel=1\n\
                     [[node]]\nid=\"k\"\nkind=\"module:t_keys\"\n[[link]]\nfrom=\"w.events\"\nto=\"k.notes\"\n";
        let graph = Graph::build(&Scene::parse(scene).unwrap()).unwrap();
        let mut seams = Seams::with_deck(DeckConfig::device(2));
        seams.modules = Some(Arc::new(Source(layout)));
        let (mut engine, mut handles) = Engine::build_seams(&graph, 48_000.0, 1.0, Build::detect(), Sequencing::InBlock, &seams).unwrap();
        let mut midi = Midi::new(&mut handles.events.names, None, 1);
        let routing = handles.events.take_routing();
        let mut outlets = routing.outlets;
        let mut buf = vec![0.0f32; 128 * 2];
        let mut frame = 0usize;
        CALLS.store(0, Ordering::SeqCst);
        for round in 0..20 {
            // the platform's side, outside the measure: the bytes through the adapter into the dispatcher's rings
            let mut out = Vec::new();
            for key in [57, 60, 64, 72] {
                let status = if round % 2 == 0 { 0x90 } else { 0x80 } | u8::from(key == 72);
                midi.feed(&[status, key, 100], (frame + 5) as f64 / 48_000.0, &mut out);
            }
            for ev in out {
                for &t in routing.table.targets(ev.name) {
                    outlets[t as usize].push(ev.clone()).unwrap();
                }
            }
            measure(true);
            for _ in 0..8 {
                engine.render(&mut buf, 2, |x| x);
                frame += 128;
            }
            measure(false);
        }
        assert_eq!(CALLS.load(Ordering::SeqCst), 0, "allocations / frees in the event host's live notes");
        assert_eq!((ONS.load(Ordering::Relaxed), OFFS.load(Ordering::Relaxed)), (30, 30), "ten chords on channel 1, each held and released; channel 2 stays out");
    }
}
