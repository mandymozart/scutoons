//! The module host (#280, feature `modules`; module ABI v2 #342, #343): real wasm built with deck-module, run in
//! wasmtime inside the engine. The `walk` module is proven against the built-in `walk` node through the whole engine;
//! `walk` plays `analog` through the event host (module to module, by name); the fixture module misbehaves on request to
//! show that a panic never cuts the rest of the scene. The host's own cases (the io staging, a panic, a runaway loop,
//! hostile imports, memory) are the conformance suite of deck-module-host, generic over its backend. The wasm is built
//! here with the wasm32 target (`rustup target add wasm32-unknown-unknown`), into its own target dir.
#![cfg(feature = "modules")]
use deck_engine::{DeckConfig, Engine, ModuleSource, Opened, Seams, Sequencing};
use deck_module_host::{Mode, Setup};
use deck_native::audio::{detect_build, detect_sequencing, Build};
use deck_native::graph::{Graph, Kind};
use deck_native::module_host::{Instance, Library};
use deck_native::modules;
use deck_native::scene::Scene;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

fn deck_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned()
}

/// the release wasm of a module crate (built once for all the tests)
fn wasm(file: &str) -> Vec<u8> {
    static BUILD: OnceLock<()> = OnceLock::new();
    let root = deck_root();
    let dir = root.join("target").join("wasm-modules");
    BUILD.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let out = std::process::Command::new(cargo)
            .current_dir(&root)
            .args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "walk-module", "-p", "fixture-module", "-p", "analog-module", "--target-dir"])
            .arg(&dir)
            .output()
            .expect("running cargo");
        assert!(
            out.status.success(),
            "building the example modules for wasm32 failed (rustup target add wasm32-unknown-unknown?):\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    });
    std::fs::read(dir.join("wasm32-unknown-unknown/release").join(file)).unwrap()
}

fn walk() -> Vec<u8> {
    wasm("walk_module.wasm")
}

fn fixture() -> Vec<u8> {
    wasm("fixture_module.wasm")
}

fn analog() -> Vec<u8> {
    wasm("analog_module.wasm")
}

const RATE: f64 = 48_000.0;

/// The module source of these tests (#382): what the deck's `audio::modules::Global` does - the show mode, the library's
/// modules - but every instance without a wall-clock deadline (`Setup::unbudgeted`). The tests here prove what the modules
/// play, not how fast; with the deck's 4 x quantum ceiling a loaded machine (three builds at once) traps a call and
/// silences the module. Only the test support builds engines this way: a deck opens its modules with `Global`.
struct Unbudgeted;

impl ModuleSource for Unbudgeted {
    fn open(&self, kind: deck_space::modreg::ModuleKind, rate: f64, seed: u64) -> anyhow::Result<Opened> {
        let loaded = Library::global().of_kind(kind).ok_or_else(|| anyhow::anyhow!("the module '{}' is not loaded", kind.spec().kind))?;
        let instance = Instance::with_setup(&loaded, Setup::new(rate, seed).mode(Mode::Show).unbudgeted())?.into_host();
        Ok(Opened { instance: Box::new(instance), layout: loaded.layout.clone(), params: loaded.desc.params.len() })
    }
}

/// an engine on `channels` device channels with the unbudgeted modules, and the restart loader a deck runs
fn engine(graph: &Graph, build: Build, seq: Sequencing, channels: usize) -> (Engine, deck_engine::Handles) {
    let mut seams = Seams::with_deck(DeckConfig::device(channels));
    seams.modules = Some(Arc::new(Unbudgeted));
    let (e, mut h) = Engine::build_seams(graph, RATE as f32, 1.0, build, seq, &seams).unwrap();
    let _ = deck_native::audio::modules::spawn_loader(std::mem::take(&mut h.restarts));
    (e, h)
}

/// `Engine::build_for` of the deck's audio (its detected build and sequencing) with the unbudgeted modules
fn engine_for(graph: &Graph, channels: usize) -> (Engine, deck_engine::Handles) {
    engine(graph, detect_build(), detect_sequencing(), channels)
}

#[test]
fn a_module_is_read_checked_and_registered_without_running_it() {
    let l = Library::global().load(&walk()).unwrap();
    assert_eq!(l.desc.name, "walk");
    assert_eq!(l.desc.params.iter().map(|p| p.name.as_ref()).collect::<Vec<_>>(), ["spread", "rate", "glide", "base", "octaves", "length"]);
    // its kind is a graph kind: ports from the descriptor, checked like any
    let kind = Kind::parse("module:walk").expect("registered");
    assert_eq!(kind, Kind::Module(l.kind));
    assert_eq!(kind.inputs().iter().map(|p| p.name).collect::<Vec<_>>(), ["spread", "rate", "glide", "base", "octaves", "length"]);
    assert_eq!(kind.outputs().iter().map(|p| p.name).collect::<Vec<_>>(), ["cv"]);
    assert_eq!(deck_native::params::default_for(kind, "rate"), 2.0);
    let same = Library::global().load(&walk()).unwrap();
    assert!(std::sync::Arc::ptr_eq(&l, &same), "the compile is cached");
    // the same bytes as a file in DECK_MODULES' folder
    let scene = Scene::parse("[[node]]\nid=\"w\"\nkind=\"module:walk\"\n").unwrap();
    assert!(modules::check(&scene).is_ok());
    assert!(modules::check_with(&scene, |f| f == "core").unwrap_err().to_string().contains("'modules' (node w)"));
    // it does not read as a module when it isn't one
    assert!(Library::global().load(b"\0asm\x01\0\0\0").unwrap_err().to_string().contains("deck.module"));
    assert!(Library::global().load(b"hello").is_err());
}

#[test]
fn the_compile_is_cached_on_disk_by_the_wasm_hash() {
    let dir = std::env::temp_dir().join(format!("deck-module-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let a = Library::with_cache(&dir).load(&walk()).unwrap();
    assert!(!a.from_disk);
    let files: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).collect();
    assert_eq!(files.len(), 1);
    assert!(files[0].file_name().to_string_lossy().starts_with(&a.hash));
    let b = Library::with_cache(&dir).load(&walk()).unwrap();
    assert!(b.from_disk, "a second library takes it from the cache");
    // a damaged cache file is compiled again, not trusted (the mapped file is let go first: Windows locks it)
    drop((a, b));
    std::fs::write(files[0].path(), b"not a compiled module").unwrap();
    assert!(!Library::with_cache(&dir).load(&walk()).unwrap().from_disk);
    let _ = std::fs::remove_dir_all(&dir);
}

/// the scene of the proof: a walk (the built-in or the module) playing an oscillator through the output
fn walk_scene(kind: &str) -> Scene {
    Scene::parse(&format!(
        "[[node]]\nid=\"w\"\nkind=\"{kind}\"\nseed=42\nspread=1.0\nrate=3.0\nglide=0.5\n\
         [[node]]\nid=\"o\"\nkind=\"osc\"\ngain=0.5\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n\
         [[link]]\nfrom=\"w.cv\"\nto=\"o.cv\"\n[[link]]\nfrom=\"o\"\nto=\"out.in\"\n"
    ))
    .unwrap()
}

fn render(scene: &Scene, seconds: usize) -> Vec<f32> {
    modules::check(scene).unwrap();
    let g = Graph::build(scene).unwrap();
    let (mut e, _h) = engine(&g, Build::detect(), Sequencing::InBlock, 2);
    let mut out = Vec::new();
    // device-sized buffers, as the callback gets them
    for _ in 0..seconds * 48_000 / 512 {
        let mut buf = vec![0.0f32; 512 * 2];
        e.render(&mut buf, 2, |x| x);
        out.extend(buf);
    }
    out
}

/// the proof (#280, #343): the module, compiled to wasm and run in wasmtime inside the engine, plays what the built-in
/// `walk` node plays – the same melody through the same voice (its `cv`; its notes are an event output). Natively the
/// two are bit-equal (modules/walk/tests) and the wasm equals the native run (modules/walk/tests/wasm.rs); the
/// tolerance covers the voice's rendering of the cv.
#[test]
fn the_walk_module_plays_what_the_built_in_walk_plays() {
    Library::global().load(&walk()).unwrap();
    let builtin = render(&walk_scene("walk"), 4);
    let module = render(&walk_scene("module:walk"), 4);
    assert_eq!(builtin.len(), module.len());
    let peak = builtin.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(peak > 0.1, "the built-in plays: {peak}");
    let worst = builtin.iter().zip(&module).fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
    assert!(worst < 2e-3, "the module is the built-in's melody: worst sample difference {worst}");
    // and it is not the same by silence or by accident: another seed plays another melody
    let other = render(&walk_scene("module:walk").with_seed(7), 4);
    let diff = builtin.iter().zip(&other).fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
    assert!(diff > 0.05, "another seed, another melody ({diff})");
}

trait WithSeed {
    fn with_seed(self, seed: i64) -> Scene;
}

impl WithSeed for Scene {
    fn with_seed(mut self, seed: i64) -> Scene {
        for n in &mut self.nodes {
            if n.id == "w" {
                n.params.insert("seed".into(), toml::Value::Integer(seed));
            }
        }
        self
    }
}

/// through the engine: a module that panics on every call never cuts the rest of the scene (a show restarts it with a
/// growing pause – `a_trapped_module_restarts_in_a_show`)
#[test]
fn a_trapped_node_never_cuts_the_show() {
    Library::global().load(&fixture()).unwrap();
    let scene = Scene::parse(
        "[[node]]\nid=\"f\"\nkind=\"module:fixture\"\nmode=1\n[[node]]\nid=\"o\"\nkind=\"osc\"\nfreq=440\ngain=0.5\n\
         [[node]]\nid=\"m\"\nkind=\"mixer\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n\
         [[link]]\nfrom=\"f.out\"\nto=\"m.in1\"\n[[link]]\nfrom=\"o\"\nto=\"m.in2\"\n[[link]]\nfrom=\"m\"\nto=\"out.in\"\n",
    )
    .unwrap();
    let out = render(&scene, 1);
    let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(peak > 0.3 && peak < 0.6, "the oscillator plays through the mixer: {peak}");
}

/// walk → analog (#343): the walk's note events reach the synth's note input through the engine's event host (module
/// to module, by the name `music.note`) – a melody, the same every run, another with another seed
#[test]
fn the_walk_plays_the_analog_synth_through_note_events() {
    Library::global().load(&walk()).unwrap();
    Library::global().load(&analog()).unwrap();
    let scene = |seed: i64| {
        Scene::parse(&format!(
            "[[node]]\nid=\"w\"\nkind=\"module:walk\"\nseed={seed}\nspread=1.0\nrate=3.0\n\
             [[node]]\nid=\"a\"\nkind=\"module:analog\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n\
             [[link]]\nfrom=\"a.out\"\nto=\"out.in\"\n"
        ))
        .unwrap()
    };
    let a = render(&scene(42), 2);
    let peak = a.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(peak > 0.05, "the synth plays the walk's notes: peak {peak}");
    assert_eq!(a, render(&scene(42), 2), "the same every run");
    let diff = a.iter().zip(&render(&scene(7), 2)).fold(0.0f32, |m, (x, y)| m.max((x - y).abs()));
    assert!(diff > 0.01, "another seed, another melody ({diff})");
}

/// the demo scenes (#406, deck/native/scenes): the files the deck's F5 list offers parse, pass the module check and play
/// non-silent, quiet audio through the engine – `module-walk` (the walk module's cv into an FM voice) and `module-analog`
/// (the walk's notes into the analog synth). The scenes carry a tempo clock and a picture besides; the audio engine plays
/// its part of the graph.
#[test]
fn the_demo_scenes_play_the_modules() {
    Library::global().load(&walk()).unwrap();
    Library::global().load(&analog()).unwrap();
    let dir = deck_root().join("native").join("scenes");
    for name in ["module-walk", "module-analog"] {
        let scene = Scene::parse(&std::fs::read_to_string(dir.join(format!("{name}.toml"))).unwrap()).unwrap();
        // refused by name on a build without the modules feature, accepted with it
        assert!(modules::check_with(&scene, |f| f == "core").unwrap_err().to_string().contains("'modules'"), "{name}");
        let out = render(&scene, 4);
        assert!(out.iter().all(|x| x.is_finite()), "{name}: finite samples");
        let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        // audible, and quiet by default (the X1 plays to speakers or a mixer)
        assert!(peak > 0.02, "{name} plays: peak {peak}");
        assert!(peak < 0.5, "{name} is quiet by default: peak {peak}");
        // it keeps sounding over the four seconds (not one note and then silence)
        let blocks: Vec<f32> = out.chunks(512 * 2).map(|b| b.iter().fold(0.0f32, |m, x| m.max(x.abs()))).collect();
        let (lo, hi) = blocks.iter().fold((f32::MAX, 0.0f32), |(lo, hi), &p| (lo.min(p), hi.max(p)));
        assert!(blocks.iter().filter(|&&p| p > 0.01).count() > blocks.len() / 4, "{name}: sounds through the run");
        println!("{name}: peak {peak:.3}, block peaks {lo:.3}..{hi:.3}");
    }
}

/// numbers for the record (`cargo test -p deck-native --features modules -- --nocapture module_costs`)
#[test]
fn module_costs() {
    let l = Library::global().load(&walk()).unwrap();
    let t = Instant::now();
    let mut i = Instance::with_setup(&l, Setup::new(RATE, 42).unbudgeted()).unwrap();
    let inst = t.elapsed();
    i.set_param(0, 1.0);
    i.set_param(1, 8.0);
    let q = deck_native::audio::QUANTUM;
    i.process(0.0, q as u32);
    let mut total = Duration::ZERO;
    for k in 1..=1600 {
        let t = Instant::now();
        i.process((k * q) as f64, q as u32);
        total += t.elapsed();
    }
    let per = total / 1600;
    println!("walk: compile {:?}, instantiate {:?}, a {q}-frame call {:?} ({:.2}% of its {:.2} ms)", l.compile, inst, per, per.as_secs_f64() / (q as f64 / RATE) * 100.0, q as f64 / RATE * 1000.0);
    assert!(i.silenced().is_none());
}

// --- the module ABI in the engine (#342): the conformance fixture (deck/modules/fixture) as a node of a scene

/// an oscillator spread onto five channels by a bus into the fixture's `wide_in`, its `wide_out` onto a five-channel
/// device, its `info` control output as a cable into a second oscillator's `freq`
fn v2_scene(extra: &str) -> Graph {
    Library::global().load(&fixture()).unwrap();
    let scene = Scene::parse(&format!(
        "[[node]]\nid=\"o\"\nkind=\"osc\"\nfreq=440\ngain=0.02\n[[node]]\nid=\"b\"\nkind=\"bus\"\nchannels=5\n\
         [[node]]\nid=\"fx\"\nkind=\"module:fixture\"\n{extra}\n[[node]]\nid=\"dev\"\nkind=\"out\"\n[[node]]\nid=\"o2\"\nkind=\"osc\"\n\
         [[link]]\nfrom=\"o\"\nto=\"b.in\"\nmap=[[1, 1], [1, 2], [1, 3], [1, 4], [1, 5]]\n[[link]]\nfrom=\"b\"\nto=\"fx.wide_in\"\n\
         [[link]]\nfrom=\"fx.wide_out\"\nto=\"dev.in\"\n[[link]]\nfrom=\"fx.info\"\nto=\"o2.freq\"\n"
    ))
    .unwrap();
    modules::check(&scene).unwrap();
    Graph::build(&scene).unwrap()
}

/// one quantum of the five device channels, interleaved
fn quantum5(e: &mut Engine) -> Vec<f32> {
    let mut buf = vec![0.0f32; deck_native::audio::QUANTUM * 5];
    e.render(&mut buf, 5, |x| x);
    buf
}

/// frames whose first channel sounds, each checked as `wide_out` channel c = `wide_in` channel c × (c + 1)
fn wide_frames(out: &[f32]) -> usize {
    let mut heard = 0;
    for f in out.chunks(5).filter(|f| f[0].abs() > 1e-4) {
        for c in 1..5 {
            assert!((f[c] / f[0] - (c + 1) as f32).abs() < 1e-3, "channel {c} = in × {}: {f:?}", c + 1);
        }
        heard += 1;
    }
    heard
}

#[test]
fn a_module_is_a_node_with_channel_slices_and_control_outputs() {
    let g = v2_scene("");
    let fx = g.nodes.iter().position(|n| n.id == "fx").unwrap();
    let kind = g.nodes[fx].kind;
    assert_eq!(kind.inputs().iter().map(|p| p.name).collect::<Vec<_>>(), ["in", "wide_in", "push", "mode", "bias", "work"]);
    assert_eq!(kind.outputs().iter().map(|p| p.name).collect::<Vec<_>>(), ["out", "wide_out", "level", "tilt", "info"]);
    assert_eq!(kind.inputs()[1].channels, Some(deck_catalog::Channels::Count(5)));
    let o2 = g.nodes.iter().position(|n| n.id == "o2").unwrap();
    let (mut e, _h) = engine_for(&g, 5);
    let mut heard = 0;
    let mut info = Vec::new();
    for _ in 0..16 {
        heard += wide_frames(&quantum5(&mut e));
        info.push(e.control().input(o2, "freq", -1.0));
    }
    assert!(heard > 1000, "five channels through the module ({heard} frames)");
    // `info` = bpm / 100 + the beat / 1000, read one block late: the hold (0) first, then 1.2 and a little
    assert_eq!(info[0], 0.0);
    assert!(info[1..].iter().all(|&v| (1.2..1.21).contains(&v)), "{info:?}");
}

/// a trap in a show (the deck opens its instances so) restarts the module: the loader thread rebuilds it, it plays again
#[test]
fn a_trapped_module_restarts_in_a_show() {
    let g = v2_scene("mode=1");
    let fx = g.nodes.iter().position(|n| n.id == "fx").unwrap();
    let mode = g.nodes[fx].kind.input("mode").unwrap();
    let (mut e, _h) = engine_for(&g, 5);
    assert_eq!(wide_frames(&quantum5(&mut e)), 0, "it panics on its first call");
    assert!(e.control_mut().set_base(fx, mode, 0.0), "play again from the next call");
    let t = Instant::now();
    let mut heard = 0;
    while heard < 1000 && t.elapsed() < Duration::from_secs(60) {
        heard += wide_frames(&quantum5(&mut e));
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(heard >= 1000, "the rebuilt instance plays ({heard} frames in {:?})", t.elapsed());
}

/// a note from the deck's events into the fixture's `notes`, its echo out as a deck-events note with its duration
#[cfg(feature = "events")]
#[test]
fn a_module_takes_and_gives_events() {
    use deck_events::{Event, Time};
    let g = v2_scene("");
    let (mut e, mut h) = engine_for(&g, 5);
    let names = &h.events.names;
    let (note, pitch) = (names.get("music.note").unwrap(), names.get("pitch").unwrap());
    let routing = h.events.take_routing();
    let mut outlets = routing.outlets;
    // 120 bpm: beat 0.01 = frame 240, 0.02 beats long
    for &t in routing.table.targets(note) {
        outlets[t as usize].push(Event::beats(note, 0.01).lasting(Time::Beats(0.02)).with(pitch, 440.0)).unwrap();
    }
    let mut outbox = h.events.take_outbox();
    assert_eq!(outbox.len(), 1, "the fixture's `echo`");
    for _ in 0..8 {
        quantum5(&mut e);
    }
    let echo = outbox[0].pop().expect("the echo: one note with its length, sent when its off came");
    let names = &h.events.names;
    assert_eq!(names.name(echo.name), "music.note");
    let (Time::Beats(start), Some(Time::Beats(dur))) = (echo.start, echo.dur) else { panic!("{echo:?}") };
    assert!((start - 0.01).abs() < 1e-4 && (dur - 0.02).abs() < 1e-4, "start {start}, dur {dur}");
    let get = |k: &str| echo.props.get(names.get(k).unwrap());
    assert_eq!((get("pitch"), get("key")), (Some(440.0), Some(69.0)), "the key filled from the deck's tuning");
    let more = outbox[0].pop();
    assert!(more.is_err(), "one note: {:?} then {more:?}", echo);
}
