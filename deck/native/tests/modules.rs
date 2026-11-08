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
            .args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "walk-module", "-p", "fixture-module", "-p", "analog-module", "-p", "stepseq-module", "-p", "context-probe-module", "-p", "catpope-rig-module", "--target-dir"])
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

fn stepseq() -> Vec<u8> {
    wasm("stepseq_module.wasm")
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

/// #406: the demo scene `module-walk` (the built-in the F5 list offers): the walk module's cv into an FM voice – parses, passes the
/// module check, plays non-silent and quiet audio through the engine and keeps sounding (not one note and then silence). A build
/// without the modules feature refuses it by name.
#[test]
fn the_module_walk_scene_plays_the_walk_module() {
    Library::global().load(&walk()).unwrap();
    let scene = Scene::parse(deck_native::switcher::builtin("module-walk").expect("a built-in scene")).unwrap();
    assert!(modules::check_with(&scene, |f| f == "core").unwrap_err().to_string().contains("'modules'"));
    let out = render(&scene, 4);
    assert!(out.iter().all(|x| x.is_finite()), "finite samples");
    let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(peak > 0.02, "it plays: peak {peak}");
    assert!(peak < 0.5, "quiet by default (the X1 plays to speakers or a mixer): peak {peak}");
    let blocks: Vec<f32> = out.chunks(512 * 2).map(|b| b.iter().fold(0.0f32, |m, x| m.max(x.abs()))).collect();
    assert!(blocks.iter().filter(|&&p| p > 0.01).count() > blocks.len() / 4, "sounds through the run");
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

/// #409 (T2 of #314): the deck's mood changes between blocks and a real module's output changes with it – the live
/// context through the face's command ring, applied at the next block's start, read by the module as
/// `ctx.character()`: the mood's index in its own `moods` (A1), the string, the energy, the touch
#[test]
fn a_module_reads_the_deck_s_live_mood() {
    use deck_native::control::{Context, Controls};
    Library::global().load(&wasm("context_probe_module.wasm")).unwrap();
    let scene = Scene::parse(
        "[[node]]\nid=\"p\"\nkind=\"module:context_probe\"\nmood=\"happy\"\n\
         [[node]]\nid=\"o\"\nkind=\"osc\"\ngain=0.01\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\n[[link]]\nfrom=\"o\"\nto=\"out.in\"\n",
    )
    .unwrap();
    let g = Graph::build(&scene).unwrap();
    let p = g.nodes.iter().position(|n| n.id == "p").unwrap();
    let (mut e, mut h) = engine_for(&g, 2);
    let mut face = Controls::new(&g, Some(&mut h));
    let mut seen = Vec::new();
    let mut quantum = |e: &mut Engine| {
        let mut buf = vec![0.0f32; deck_native::audio::QUANTUM * 2];
        e.render(&mut buf, 2, |x| x);
        // what the module wrote in the block before (its outputs act one block late): index, energy, touch, sad
        let o = e.control().outputs(p);
        seen.push([o[0], o[1], o[2], o[3]]);
    };
    quantum(&mut e);
    quantum(&mut e);
    face.set_context(Context::new("sad", "", 0.5, 0.25, 0.0));
    quantum(&mut e);
    quantum(&mut e);
    face.set_context(Context::new("laughing", "", 0.875, 0.0, 0.0));
    quantum(&mut e);
    quantum(&mut e);
    face.set_context(Context::new("neutral", "", 0.0, 0.0, 0.0));
    quantum(&mut e);
    quantum(&mut e);
    let want = [
        [0.0; 4],                 // the first block: nothing written yet (held)
        [3.0, 0.0, 0.0, 0.0],     // the node's `mood` param (happy, the third of its list) until the deck says one
        [3.0, 0.0, 0.0, 0.0],     // the block the context arrives on – written then, read one block late
        [2.0, 0.5, 0.25, 1.0],    // sad
        [2.0, 0.5, 0.25, 1.0],
        [4.0, 0.875, 0.0, 0.0],   // laughing
        [4.0, 0.875, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],     // neutral: a mood it does not list
    ];
    assert_eq!(seen, want);
}

/// #410 (T3 of #314): catpope as a code rig on the native deck – its policy the REAL compiled module
/// (deck/conformance/modules/catpope-rig, research/RIG-MODULE-GROUPS.md §3) in the engine, reading the deck's live mood
/// (#409); each frame the deck reads its `@preset` output from the heard block and the core replays the rig with the
/// preset it chose (`Deck::choose`). For all six moods, one frame after the mood turned: the preset the voice plays is the
/// declarative catpope's, and what the voice's and the drone's inputs read is what the tables give.
#[test]
fn catpope_as_a_code_rig_chooses_what_the_tables_choose() {
    use deck_native::control::{Context, Controls};
    use deck_native::rig::Player;
    use deck_space::deck::Deck;
    use deck_space::rig::Bundle;
    let loaded = Library::global().load(&wasm("catpope_rig_module.wasm")).unwrap();
    // the code rig: the core catpope's tracks, instruments and presets, its mood tables replaced by the policy – with the
    // descriptor the compiled module carries (what the build service stores with its module:<id> document)
    let mut bundle = Bundle::core("character.catpope").unwrap().unwrap();
    bundle.rig.moods.clear();
    bundle.rig.policy = Some(deck_space::rig::policy::Policy { module: "catpope_rig".into(), seed: None, values: Default::default() });
    bundle.modules = vec![loaded.desc.clone()];
    let mut player = Player::play(bundle, Some("neutral")).unwrap();
    let scene = player.scene().unwrap();
    modules::check(&scene).unwrap();
    let g = Graph::build(&scene).unwrap();
    let (mut e, mut h) = engine_for(&g, 2);
    let mut ctl = Controls::new(&g, Some(&mut h));
    let at = |id: &str| g.nodes.iter().position(|n| n.id == id).unwrap();
    let (voice, drone) = (at("voice_voice"), at("drone_drone"));
    let tempo = deck_native::tempo::Now::default();
    let voice_preset = |p: &Player| p.played().tracks.iter().find(|t| t.track == "voice").and_then(|t| t.preset.clone());
    // the declarative catpope: what its tables play for a mood (the voice's preset; index, ratio, drone gain)
    let mut tables = Deck::new();
    tables.play_core("catpope", "");
    let mut table = |mood: &str| {
        tables.ladder.pin(Some(mood));
        tables.step(0.0, 0.0, false, 0.0, 1.0 / 60.0);
        let p = tables.played();
        let param = |node: &str, k: &str| p.nodes.iter().find(|n| n.id == node).and_then(|n| n.params.get(k)).and_then(|v| v.as_float()).unwrap() as f32;
        let preset = p.tracks.iter().find(|t| t.track == "voice").and_then(|t| t.preset.clone());
        (preset, [param("voice_voice", "index"), param("voice_voice", "ratio"), param("drone_drone", "gain")])
    };
    // one deck frame as the native deck runs it (rig.rs `frame`): a frame's blocks, the heard values, then the being's
    // context to the module nodes and the rig's frame (the mood's replay, the policy's choice)
    let mut frame = |mood: &str, player: &mut Player, ctl: &mut Controls| {
        for _ in 0..4 {
            let mut buf = vec![0.0f32; deck_native::audio::QUANTUM * 2];
            e.render(&mut buf, 2, |x| x);
        }
        ctl.update(1.0 / 60.0, 0, &tempo);
        ctl.set_context(Context::new(mood, "", 0.0, 0.0, 0.0));
        player.frame(mood, &g, ctl, 1.0 / 60.0);
    };
    let mut before = None;
    for mood in ["neutral", "thinking", "sad", "happy", "laughing", "surprised"] {
        let (preset, values) = table(mood);
        frame(mood, &mut player, &mut ctl);
        if before.is_some() && before != Some(preset.clone()) {
            assert_eq!(Some(voice_preset(&player)), before, "{mood}: the frame the mood turned the policy has not heard it yet");
        }
        frame(mood, &mut player, &mut ctl);
        assert_eq!(voice_preset(&player), preset, "{mood}: one frame later the policy's choice plays");
        frame(mood, &mut player, &mut ctl);
        let read = [ctl.input(voice, "index", f32::NAN), ctl.input(voice, "ratio", f32::NAN), ctl.input(drone, "gain", f32::NAN)];
        assert_eq!(read, values, "{mood}: the voice's index and ratio, the drone's gain");
        before = Some(preset);
    }
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

/// #417: stepseq → analog through the engine's event host: the sequencer's notes sound, and a slider is a pitch – the
/// notes the module sends out to the world (their keys filled from the deck's tuning) move by the interval a slider moved
#[test]
fn the_stepseq_plays_the_analog_synth_and_a_slider_is_a_pitch() {
    Library::global().load(&stepseq()).unwrap();
    Library::global().load(&analog()).unwrap();
    let scene = |step3: f32| {
        Scene::parse(&format!(
            "[[node]]\nid=\"q\"\nkind=\"module:stepseq\"\nstep3={step3}\n\
             [[node]]\nid=\"a\"\nkind=\"module:analog\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n\
             [[link]]\nfrom=\"a.out\"\nto=\"out.in\"\n"
        ))
        .unwrap()
    };
    let a = render(&scene(0.25), 3);
    let peak = a.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(peak > 0.05, "the synth plays the sequencer's notes: peak {peak}");
    assert_eq!(a, render(&scene(0.25), 3), "the same every run: no randomness");
    let diff = a.iter().zip(&render(&scene(0.75), 3)).fold(0.0f32, |m, (x, y)| m.max((x - y).abs()));
    assert!(diff > 0.01, "another slider, another sound ({diff})");
    // the keys it sends out: step 3 is the third note (id 2) – 6 semitones above the base at 0.25, 18 at 0.75
    let key_of_third = |step3: f32| {
        let g = Graph::build(&scene(step3)).unwrap();
        let (mut e, mut h) = engine(&g, Build::detect(), Sequencing::InBlock, 2);
        let mut outbox = h.events.take_outbox();
        for _ in 0..(3 * 48_000 / 512) {
            e.render(&mut vec![0.0f32; 512 * 2], 2, |x| x);
        }
        let names = &h.events.names;
        let key = names.get("key").unwrap();
        let mut keys = Vec::new();
        while let Ok(n) = outbox[0].pop() {
            keys.push(n.props.get(key).expect("the key is filled from the tuning"));
        }
        assert!(keys.len() >= 5, "five steps in 3 s at 120 bpm, 2 a beat: {keys:?}");
        keys
    };
    let (low, high) = (key_of_third(0.25), key_of_third(0.75));
    assert_eq!(high[2] - low[2], 12.0, "an octave apart: {low:?} {high:?}");
    assert_eq!((low[0], low[1], low[3]), (high[0], high[1], high[3]), "the other steps do not move");
}

/// #417: `rig:catpope-analog` – catpope's rig with the analog module as one more track, the stepseq module playing it – in
/// every mood: the synth's own voices sound (the render minus the render with its volume at 0 – the rest is the same
/// voice and drone), and its volume follows the mood the way the drone's gain does
#[test]
fn catpope_plays_the_analog_synth_as_an_additional_track_in_every_mood() {
    Library::global().load(&stepseq()).unwrap();
    Library::global().load(&analog()).unwrap();
    let stats = |x: &[f32]| (x.iter().fold(0.0f32, |m, v| m.max(v.abs())), (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt());
    let mut rms = std::collections::BTreeMap::new();
    let b: deck_space::rig::Bundle = toml::from_str(deck_native::rig::MODULE_RIGS[0].1).unwrap();
    // the signal strip: a slider per step, a toggle per step, the loop length, the synth's volume
    let knobs: Vec<String> = b.play(None, None).unwrap().knobs.into_iter().map(|k| k.path).filter(|p| p.starts_with("analog_")).collect();
    let want: Vec<String> = (1..=16).map(|i| format!("analog_seq.step{i}")).chain((1..=16).map(|i| format!("analog_seq.on{i}"))).chain(["analog_seq.steps".into(), "analog_synth.volume".into()]).collect();
    assert_eq!(knobs, want);
    for mood in ["neutral", "thinking", "sad", "happy", "laughing", "surprised"] {
        let scene = deck_native::rig::scene(std::path::Path::new(&format!("rig:catpope-analog@{mood}"))).unwrap();
        let ids: Vec<&str> = scene.nodes.iter().map(|n| n.id.as_str()).collect();
        for id in ["voice_voice", "drone_drone", "drone_fifth", "analog_seq", "analog_synth"] {
            assert!(ids.contains(&id), "{mood}: {id} in {ids:?}");
        }
        let mut quiet = scene.clone();
        quiet.nodes.iter_mut().find(|n| n.id == "analog_synth").unwrap().params.insert("volume".into(), toml::Value::Float(0.0));
        let (with, without) = (render(&scene, 8), render(&quiet, 8));
        let (peak_all, rms_all) = stats(&with);
        let (_, rms_rest) = stats(&without);
        let only: Vec<f32> = with.iter().zip(&without).map(|(a, b)| a - b).collect();
        let (peak, r) = stats(&only);
        let (left, right): (Vec<f32>, Vec<f32>) = (only.iter().step_by(2).copied().collect(), only.iter().skip(1).step_by(2).copied().collect());
        println!("{mood}: all peak {peak_all:.3} rms {rms_all:.4}; without analog rms {rms_rest:.4}; analog alone peak {peak:.3} rms {r:.4} (L {:.4} R {:.4})", stats(&left).1, stats(&right).1);
        assert!(peak > 0.02 && r > 0.005, "{mood}: the synth's voices sound: peak {peak}, rms {r}");
        assert!(stats(&left).1 > 0.002 && stats(&right).1 > 0.002, "{mood}: both channels");
        assert!(r < rms_rest * 1.5, "{mood}: it does not bury the voice and the drone ({r} against {rms_rest})");
        assert!(peak_all < 1.0, "{mood}: the sum stays below full scale ({peak_all})");
        rms.insert(mood, r);
    }
    assert!(rms["sad"] > rms["neutral"] && rms["neutral"] > rms["surprised"], "the volume follows the mood like the drone's gain: {rms:?}");
}

/// #418: the built-in scene `catpope-analog` – the being (its room, catpope's head) with the stepseq module playing the
/// analog module: it loads, its sound is not silent, a slider is a pitch (step 3 up an octave moves that note by 12
/// semitones and no other), and the being's 3D buttons are the sliders, the switches, the loop and the synth's main controls
#[test]
fn the_catpope_analog_scene_plays_and_its_sliders_are_buttons() {
    Library::global().load(&stepseq()).unwrap();
    Library::global().load(&analog()).unwrap();
    let text = deck_native::switcher::builtin("catpope-analog").expect("a built-in scene");
    let base = Scene::parse(text).unwrap();
    let with_step3 = |v: f64| {
        let mut s = base.clone();
        s.nodes.iter_mut().find(|n| n.id == "seq").unwrap().params.insert("step3".into(), toml::Value::Float(v));
        s
    };
    // staged: the space draws on the window, the sound reaches the stereo outputs
    modules::check(&base).unwrap();
    assert!(base.nodes.iter().any(|n| n.kind == "space") && base.nodes.iter().any(|n| n.kind == "screen"));
    let low = render(&with_step3(0.25), 4);
    let peak = low.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    let rms = (low.iter().map(|v| v * v).sum::<f32>() / low.len() as f32).sqrt();
    assert!(peak > 0.01 && peak < 0.9 && rms > 0.002, "the scene sounds, quietly: peak {peak} rms {rms}");
    let (left, right): (f32, f32) = (low.iter().step_by(2).map(|v| v * v).sum(), low.iter().skip(1).step_by(2).map(|v| v * v).sum());
    assert!(left > 0.0 && right > 0.0, "both channels");
    let diff = low.iter().zip(&render(&with_step3(0.75), 4)).fold(0.0f32, |m, (x, y)| m.max((x - y).abs()));
    assert!(diff > 0.005, "another step 3, another sound ({diff})");
    // the keys sent out: step 3 is the third note – 6 semitones above the base at 0.25, 18 at 0.75
    let keys_of = |s: &Scene| {
        let g = Graph::build(s).unwrap();
        let (mut e, mut h) = engine(&g, Build::detect(), Sequencing::InBlock, 2);
        let mut outbox = h.events.take_outbox();
        for _ in 0..(4 * 48_000 / 512) {
            e.render(&mut vec![0.0f32; 512 * 2], 2, |x| x);
        }
        let key = h.events.names.get("key").unwrap();
        let mut keys = Vec::new();
        while let Ok(n) = outbox[0].pop() {
            keys.push(n.props.get(key).expect("the key is filled from the tuning"));
        }
        assert!(keys.len() >= 5, "the sequencer plays on the scene's tempo: {keys:?}");
        keys
    };
    let (low, high) = (keys_of(&with_step3(0.25)), keys_of(&with_step3(0.75)));
    assert_eq!(high[2] - low[2], 12.0, "an octave apart: {low:?} {high:?}");
    assert_eq!((low[0], low[1], low[3]), (high[0], high[1], high[3]), "the other steps do not move");
    // the being's 3D buttons, as the native deck builds them from the scene's graph and `[being] expose`
    let graph = Graph::build(&base).unwrap();
    let being = deck_native::space::being::Being::new(&graph, &base.being);
    let knobs: Vec<String> = being.buttons.iter().map(|b| format!("{}.{}", graph.nodes[b.node].id, b.name)).collect();
    let mut want: Vec<String> = (1..=16).map(|i| format!("seq.step{i}")).chain((1..=16).map(|i| format!("seq.on{i}"))).collect();
    want.extend(["seq.steps", "seq.rate", "seq.gate", "synth.volume", "synth.f1_cutoff", "synth.f1_res", "synth.f1_env", "synth.ae1_d", "synth.ae1_r", "synth.o2_fine", "synth.l1_rate", "cue.gain", "stem.gain"].map(String::from));
    let mut got = knobs.clone();
    got.sort();
    want.sort();
    assert_eq!(got, want, "the 16 sliders, the 16 switches, the loop, the clock and the synth's main knobs: {knobs:?}");
    assert_eq!(knobs[0], "seq.steps", "#424: the loop length is the very first knob");
}

/// #424 (owner 2026-11-08): the being shows only as many pitch knobs (and on/off switches) as the sequence has steps, the
/// loop length `steps` the first knob – as the native deck builds it from the real scene and the real modules, the
/// count live: the scene's `steps`, then another value pushed into the control table
#[test]
fn the_being_shows_only_as_many_step_knobs_as_the_sequence_has_steps() {
    use deck_native::space::being::{Being, Hit};
    use deck_native::space::values::{Knobs, Local, Table};
    Library::global().load(&stepseq()).unwrap();
    Library::global().load(&analog()).unwrap();
    let base = Scene::parse(deck_native::switcher::builtin("catpope-analog").expect("a built-in scene")).unwrap();
    let mut scene = base.clone();
    scene.nodes.iter_mut().find(|n| n.id == "seq").unwrap().params.insert("steps".into(), toml::Value::Float(6.0));
    let graph = Graph::build(&scene).unwrap();
    let mut being = Being::new(&graph, &scene.being);
    let mut ctl = Table::new(&graph);
    let silent = |_: usize, _: usize| 0.0;
    let beat = deck_native::tempo::Now { bpm: 100.0, pos: deck_native::tempo::position(0.0, 4), per_bar: 4, running: true };
    let shown = |being: &mut Being, ctl: &Table| -> Vec<String> {
        being.update(1.0 / 60.0, 0.0, &Local { ctl, meter: &silent }, &beat);
        being.buttons.iter().filter(|b| !b.hidden).map(|b| format!("{}.{}", graph.nodes[b.node].id, b.name)).collect()
    };
    let others = ["synth.volume", "synth.f1_cutoff", "synth.f1_res", "synth.f1_env", "synth.ae1_d", "synth.ae1_r", "synth.o2_fine", "synth.l1_rate", "cue.gain", "stem.gain"];
    let want = |n: usize| -> Vec<String> {
        let seq = ["seq.steps".to_string()].into_iter().chain((1..=n).map(|i| format!("seq.step{i}"))).chain((1..=n).map(|i| format!("seq.on{i}"))).chain(["seq.rate", "seq.gate"].map(String::from));
        seq.chain(others.iter().map(|s| s.to_string())).collect()
    };
    let six = shown(&mut being, &ctl);
    assert_eq!(six, want(6), "steps = 6: 6 pitch knobs, 6 switches, the length first, the others");
    // the hidden knobs are no longer buttons to pick (a flag, the list keeps its length) and keep their place in it
    assert_eq!(being.buttons.len(), 1 + 16 + 16 + 2 + 8 + 2);
    let hidden = being.buttons.iter().position(|b| b.name == "step9").unwrap();
    assert!(being.buttons[hidden].hidden && being.hover != Some(Hit::Button(hidden)));
    // live: the owner turns the length knob (the table's value) – 16 shows all of them, 3 the first three
    let (seq, port) = (being.buttons[0].node, being.buttons[0].port);
    assert_eq!(being.buttons[0].name, "steps", "the length is the first knob");
    assert!(ctl.set(seq, port, 16.0));
    assert_eq!(shown(&mut being, &ctl), want(16), "steps = 16: all of them");
    assert!(ctl.set(seq, port, 3.0));
    assert_eq!(shown(&mut being, &ctl), want(3));
}
