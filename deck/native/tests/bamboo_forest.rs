//! The bamboo forest (#780): the module rig `bamboo-forest` – a grove track (the bamboo module, the wind's sequencer) and
//! a perc track (an analog knock its hits play) – and the sandbox `bamboo-delay`, the rig with the perc track's sound sent
//! at scene level to a delay bus (`fx-delay`, as crows-rave sends its crows). Through the whole engine with the real
//! modules (bamboo, analog, delay as wasm in wasmtime): every hit is heard on the perc track at its frame, the delay
//! echoes the hits a dotted eighth later, a run is the same twice.
//! The sandbox is written by `the_sandbox_is_the_rig_with_a_delay_bus`, never by hand:
//!   UPDATE_FIXTURES=1 cargo test -p deck-native --features modules --test bamboo_forest
//! `BAMBOO_OUT_DIR=<dir>` also writes 16 s of the sandbox (the wind rising and dying away) as `<dir>/bamboo-delay.wav`.
#![cfg(all(feature = "modules", feature = "events"))]
use deck_engine::{DeckConfig, Engine, ModuleSource, Opened, Seams, Sequencing};
use deck_events::{Event, Time};
use deck_module_host::{Mode, Setup};
use deck_native::audio::Build;
use deck_native::control::Controls;
use deck_native::graph::Graph;
use deck_native::module_host::{Instance, Library};
use deck_native::modules;
use deck_native::scene::{LinkDef, NodeDef, Scene};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

const RATE: f64 = 48_000.0;
const BLOCK: usize = 512;
const SANDBOX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scenes/sandbox/bamboo-delay.toml");
/// the delay's time: a dotted eighth (sync 8), in beats
const ECHO_BEATS: f64 = 0.75;

const HEADER: &str = "\
# A sandbox (#780): the bamboo forest with a delay bus – the rig `bamboo-forest` flattened (the grove's wind sequencer
# `bamboo_forest`, its hits cabled into the perc track's knock `perc_knock`), in a gusty wind, and the perc track's
# sound sent at scene level to the bus `fx-delay` into the delay module (dotted eighths bouncing left and right), whose
# echoes join the mix on its second input – the scene's own spores, as crows-rave sends its crows.
# Written by deck-native's tests/bamboo_forest.rs `the_sandbox_is_the_rig_with_a_delay_bus` – never by hand:
#   UPDATE_FIXTURES=1 cargo test -p deck-native --features modules --test bamboo_forest
# From deck/ (modules: bamboo, analog, delay in DECK_MODULES):
#   cargo run -p deck-native --release --features modules -- --dev --scene bamboo-delay --windowed

";

/// bamboo, analog and delay as release wasm, built once, loaded into the library
fn load_modules() {
    static LOADED: OnceLock<()> = OnceLock::new();
    LOADED.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).parent().unwrap().join("wasm-modules");
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let crates = ["bamboo", "analog", "delay"];
        let mut args: Vec<String> = ["build", "--release", "--target", "wasm32-unknown-unknown"].map(String::from).to_vec();
        for c in crates {
            args.extend(["-p".into(), format!("{c}-module")]);
        }
        let out = std::process::Command::new(cargo).current_dir(&root).args(&args).arg("--target-dir").arg(&dir).output().expect("running cargo");
        assert!(out.status.success(), "building the modules for wasm32 failed:\n{}", String::from_utf8_lossy(&out.stderr));
        for c in crates {
            let wasm = std::fs::read(dir.join("wasm32-unknown-unknown/release").join(format!("{c}_module.wasm"))).unwrap();
            Library::global().load(&wasm).unwrap();
        }
    });
}

/// the library's modules without a wall-clock deadline (what they play, not how fast)
struct Unbudgeted;

impl ModuleSource for Unbudgeted {
    fn open(&self, kind: deck_space::modreg::ModuleKind, rate: f64, seed: u64) -> anyhow::Result<Opened> {
        let loaded = Library::global().of_kind(kind).ok_or_else(|| anyhow::anyhow!("the module '{}' is not loaded", kind.spec().kind))?;
        let instance = Instance::with_setup(&loaded, Setup::new(rate, seed).mode(Mode::Show).unbudgeted())?.into_host();
        Ok(Opened { instance: Box::new(instance), layout: loaded.layout.clone(), params: loaded.desc.params.len() })
    }
}

fn rig() -> Scene {
    load_modules();
    deck_native::rig::scene(Path::new("rig:bamboo-forest")).unwrap()
}

fn node(id: &str, kind: &str, params: &[(&str, f64)]) -> NodeDef {
    let mut n: NodeDef = toml::from_str(&format!("id = \"{id}\"\nkind = \"{kind}\"")).unwrap();
    for (k, v) in params {
        n.params.insert((*k).into(), toml::Value::Float(*v));
    }
    n
}

fn link(from: &str, to: &str, scale: f32) -> LinkDef {
    toml::from_str(&format!("from = \"{from}\"\nto = \"{to}\"\nscale = {scale:?}")).unwrap()
}

/// the rig flattened in a gusty wind, the perc track sent to a delay bus whose echoes come back on the mix's second input
fn from_rig() -> Scene {
    let mut s = rig();
    s.stage.name = "Bamboo delay".into();
    let forest = s.nodes.iter_mut().find(|n| n.id == "bamboo_forest").expect("the grove");
    for (k, v) in [("thin", 2.0), ("thick", 1.5), ("wind", 0.7), ("gust", 0.7)] {
        forest.params.insert(k.into(), toml::Value::Float(v));
    }
    let mix = s.nodes.iter_mut().find(|n| n.id == "mix1").expect("the sum");
    mix.params.insert("gain2".into(), toml::Value::Float(1.0));
    s.nodes.push(node("fx-delay", "bus", &[]));
    s.nodes.push(node("delay", "module:delay", &[("sync", 8.0), ("feedback", 45.0), ("damp", 3000.0), ("pingpong", 100.0), ("mix", 100.0)]));
    s.links.push(link("perc_knock.out", "fx-delay.in", 0.5));
    s.links.push(link("fx-delay.out", "delay.in", 1.0));
    s.links.push(link("delay.out", "mix1.in2", 1.0));
    s
}

fn sandbox() -> Scene {
    load_modules();
    Scene::parse(&std::fs::read_to_string(SANDBOX).expect("the sandbox bamboo-delay.toml")).unwrap()
}

fn has_link(s: &Scene, from: &str, to: &str) -> bool {
    s.links.iter().any(|l| l.from == from && l.to == to)
}

/// The rig is the plan's: the grove a silent track whose hits are cabled into the perc track's knock, the knock alone on
/// the strips; the left stick the weather, the right the two multipliers
#[test]
fn the_rig_is_a_grove_and_its_knock_on_two_tracks() {
    let s = rig();
    let ids: Vec<&str> = s.nodes.iter().map(|n| n.id.as_str()).collect();
    for id in ["bamboo_forest", "perc_knock", "mix1", "out"] {
        assert!(ids.contains(&id), "{id} in {ids:?}");
    }
    assert!(has_link(&s, "bamboo_forest.hits", "perc_knock.notes"), "the hits play the knock");
    assert!(has_link(&s, "perc_knock.out", "mix1.in1"), "the perc track on its strip");
    assert!(!s.links.iter().any(|l| l.from.starts_with("bamboo_forest.") && l.to.starts_with("mix")), "the grove is silent");
    let b: deck_space::rig::Bundle = toml::from_str(deck_native::rig::MODULE_RIGS[3].1).unwrap();
    assert_eq!((b.rig.subject.as_str(), b.rig.tracks.len()), ("item.bamboo", 2));
    let sticks = b.rig.sticks.clone().unwrap();
    assert_eq!(sticks, [["bamboo:forest.wind", "bamboo:forest.gust"], ["bamboo:forest.thin", "bamboo:forest.thick"]].map(|p| p.map(String::from)));
    modules::check(&s).unwrap();
    Graph::build(&s).unwrap();
}

/// The sandbox is the rig as it plays plus the scene's delay bus on the perc track
#[test]
fn the_sandbox_is_the_rig_with_a_delay_bus() {
    let s = from_rig();
    let text = format!("{HEADER}{}", toml::to_string(&s).unwrap());
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(SANDBOX, &text).unwrap();
    }
    let on_disk = std::fs::read_to_string(SANDBOX).unwrap().replace("\r\n", "\n");
    assert_eq!(on_disk, text, "bamboo-delay.toml is written by this test (UPDATE_FIXTURES=1)");
    assert_eq!(sandbox(), s);
    for (from, to) in [("perc_knock.out", "fx-delay.in"), ("fx-delay.out", "delay.in"), ("delay.out", "mix1.in2"), ("perc_knock.out", "mix1.in1")] {
        assert!(has_link(&s, from, to), "{from} → {to}");
    }
    modules::check(&s).unwrap();
    Graph::build(&s).unwrap();
    let doc = deck_space::scene_document::SceneDocument::from_scene("bamboo-delay", vec!["sandbox".into()], &s);
    assert_eq!(doc.check(), Vec::<String>::new());
}

/// What one run heard: the hits (s), the samples
#[derive(PartialEq, Debug)]
struct Run {
    hits: Vec<f64>,
    out: Vec<f32>,
}

/// `seconds` of `s`; `play(t, controls, graph)` before each block turns its knobs
fn run(s: &Scene, seconds: f64, mut play: impl FnMut(f64, &mut Controls, &Graph)) -> Run {
    modules::check(s).unwrap();
    let g = Graph::build(s).unwrap();
    let mut seams = Seams::with_deck(DeckConfig::device(2));
    seams.modules = Some(Arc::new(Unbudgeted));
    let (mut e, mut h) = Engine::build_seams(&g, RATE as f32, 1.0, Build::detect(), Sequencing::InBlock, &seams).unwrap();
    let mut boxes = h.events.take_outbox_slots();
    let (m, _) = h.tempo.map().expect("a tempo");
    let spb = (m.frame_of(1.0).unwrap() - m.frame_of(0.0).unwrap()) / RATE;
    let mut ctl = Controls::new(&g, Some(&mut h));
    let mut r = Run { hits: vec![], out: vec![] };
    for b in 0..(seconds * RATE) as usize / BLOCK {
        play(b as f64 * BLOCK as f64 / RATE, &mut ctl, &g);
        let mut buf = vec![0.0f32; BLOCK * 2];
        e.render(&mut buf, 2, |x| x);
        r.out.extend_from_slice(&buf);
        for (slot, ring) in &mut boxes {
            while let Ok(n) = ring.pop() {
                let n: Event = n;
                if slot == "bamboo_forest.hits" {
                    let Time::Beats(start) = n.start else { panic!("{n:?}") };
                    r.hits.push(start * spb);
                }
            }
        }
    }
    r
}

/// the sandbox with its send to the delay bus at `send`
fn sent(send: f32) -> Scene {
    let mut s = sandbox();
    s.links.iter_mut().find(|l| l.from == "perc_knock.out" && l.to == "fx-delay.in").unwrap().scale = send;
    s
}

fn mono(out: &[f32]) -> Vec<f32> {
    out.chunks(2).map(|c| c[0] + c[1]).collect()
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

/// Dry (no send), every lone hit is heard on the perc track at its frame: quiet before, a knock right after
#[test]
fn the_hits_are_heard_on_the_perc_track() {
    let r = run(&sent(0.0), 6.0, |_, _, _| {});
    assert!(r.hits.len() >= 8, "the wind knocks: {:?}", r.hits);
    let m = mono(&r.out);
    let first = (r.hits[0] * RATE) as usize;
    assert!(rms(&m[..first.saturating_sub(64)]) < 1e-6, "silent before the first hit");
    let mut lone = 0;
    for (i, &t) in r.hits.iter().enumerate() {
        let f = (t * RATE).round() as usize;
        let quiet = i == 0 || t - r.hits[i - 1] > 0.3;
        if !quiet || f < 2048 || f + 2048 > m.len() {
            continue;
        }
        let (before, after) = (rms(&m[f - 1024..f - 32]), rms(&m[f..f + 1024]));
        assert!(after > 4.0 * before && after > 0.005, "a knock at {t:.3} s: {before} → {after}");
        lone += 1;
    }
    assert!(lone >= 2, "lone hits to hear: {lone} of {:?}", r.hits);
}

/// Sent to the bus, the delay gets the hits: what the send adds starts a dotted eighth after the first hit, not before
#[test]
fn the_delay_echoes_the_hits() {
    let (dry, wet) = (run(&sent(0.0), 6.0, |_, _, _| {}), run(&sent(0.5), 6.0, |_, _, _| {}));
    assert_eq!(dry.hits, wet.hits, "the send changes nothing upstream");
    let echo: Vec<f32> = mono(&wet.out).iter().zip(mono(&dry.out)).map(|(w, d)| w - d).collect();
    let first = dry.hits[0] * RATE;
    // 120 BPM: a beat is half a second
    let late = (ECHO_BEATS * 0.5 * RATE) as usize;
    let begins = echo.iter().position(|v| v.abs() > 1e-4).expect("echoes");
    assert!(begins + 64 >= first as usize + late, "no echo before the delay's time: {begins} < {} + {late}", first as usize);
    assert!(begins < first as usize + late + 2048, "the first echo a dotted eighth after the first hit: {begins}, hit at {first}");
    assert!(rms(&echo[begins..]) > 0.002, "the echoes are heard: {}", rms(&echo[begins..]));
}

#[test]
fn a_run_is_the_same_twice() {
    let s = sandbox();
    let (a, b) = (run(&s, 2.0, |_, _, _| {}), run(&s, 2.0, |_, _, _| {}));
    assert_eq!(a, b, "two decks, one forest: the same hits and samples");
    let peak = a.out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(peak > 0.05 && peak < 1.0, "it sounds, below full scale: {peak}");
}

/// `BAMBOO_OUT_DIR=<dir>`: 16 s of the sandbox as `<dir>/bamboo-delay.wav` (48 kHz, stereo, 16-bit): the wind rising from
/// a breath to a gale over 10 s, then dying away by 13 s; the echoes ring out
#[test]
fn render_the_forest_when_asked() {
    let Ok(dir) = std::env::var("BAMBOO_OUT_DIR") else { return };
    let r = run(&sandbox(), 16.0, |t, ctl, g| {
        let wind = if t < 10.0 { 0.15 + 0.85 * t / 10.0 } else { (1.0 - (t - 10.0) / 3.0).max(0.0) };
        assert!(ctl.set_path(g, "bamboo_forest.wind", wind as f32));
    });
    let data: Vec<u8> = r.out.iter().flat_map(|v| ((v.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes()).collect();
    let mut wav = Vec::with_capacity(44 + data.len());
    for part in [&b"RIFF"[..], &(36 + data.len() as u32).to_le_bytes(), b"WAVEfmt ", &16u32.to_le_bytes(), &1u16.to_le_bytes(), &2u16.to_le_bytes()] {
        wav.extend_from_slice(part);
    }
    for part in [&48_000u32.to_le_bytes()[..], &(48_000u32 * 4).to_le_bytes(), &4u16.to_le_bytes(), &16u16.to_le_bytes(), b"data", &(data.len() as u32).to_le_bytes()] {
        wav.extend_from_slice(part);
    }
    wav.extend_from_slice(&data);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(Path::new(&dir).join("bamboo-delay.wav"), wav).unwrap();
    println!("{} hits", r.hits.len());
}
