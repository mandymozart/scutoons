//! The storm cloud (#780): the module rig `cloud-storm` through the whole engine with the real modules (cloud, sampler,
//! mixer as wasm in wasmtime; the samplers' sounds the stand-ins, from a cache through the deck's loader, #804). Held, the cloud's repeater plays a thunder and a kick on its note value's grid of the tempo
//! clock and the kick's onsets are heard on those frames; free, the repeats follow the rate; let go, it is silent; a run
//! is the same twice. The headless host has no module runtime (#290), so this is the rig's scenario. `CLOUD_OUT_DIR=<dir>`
//! also writes 10 s of it played by hand (held, the rate pushed up, synced and then free, let go) as `<dir>/cloud-storm.wav`.
#![cfg(all(feature = "modules", feature = "events", feature = "chapter"))]
mod stand_ins;
use deck_engine::{DeckConfig, Engine, ModuleSource, Opened, Seams, Sequencing};
use deck_events::{Event, Time};
use deck_module_host::{Mode, Setup};
use deck_native::audio::Build;
use deck_native::control::Controls;
use deck_native::graph::Graph;
use deck_native::module_host::{Instance, Library};
use deck_native::modules;
use deck_native::scene::Scene;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

const RATE: f64 = 48_000.0;
const BLOCK: usize = 512;

/// cloud, sampler and mixer as release wasm, built once, loaded into the library
fn load_modules() {
    static LOADED: OnceLock<()> = OnceLock::new();
    LOADED.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).parent().unwrap().join("wasm-modules");
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let crates = ["cloud", "sampler", "mixer"];
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
    fn open(&self, kind: deck_space::modreg::ModuleKind, rate: f64, seed: u64, assets: Vec<deck_module_host::AssetData>) -> anyhow::Result<Opened> {
        let loaded = Library::global().of_kind(kind).ok_or_else(|| anyhow::anyhow!("the module '{}' is not loaded", kind.spec().kind))?;
        let instance = Instance::with_setup(&loaded, assets.into_iter().fold(Setup::new(rate, seed).mode(Mode::Show).unbudgeted(), Setup::asset))?.into_host();
        Ok(Opened { instance: Box::new(instance), layout: loaded.layout.clone(), params: loaded.desc.params.len() })
    }
}

fn scene() -> Scene {
    load_modules();
    deck_native::rig::scene(Path::new("rig:cloud-storm")).unwrap()
}

/// What one run heard: each repeat's note (start in beats) per event output, the samples, the frame of each beat
struct Run {
    notes: BTreeMap<String, Vec<f64>>,
    out: Vec<f32>,
    frames_a_beat: f64,
}

/// `seconds` of the rig; `play(seconds, controls, graph)` before each block turns its knobs (`storm_cloud.hold` …)
fn run(seconds: f64, mut play: impl FnMut(f64, &mut Controls, &Graph)) -> Run {
    let s = scene();
    modules::check(&s).unwrap();
    let g = Graph::build(&s).unwrap();
    let mut seams = Seams::with_deck(DeckConfig::device(2));
    seams.modules = Some(Arc::new(Unbudgeted));
    seams.assets = Some(stand_ins::loader("cloud-storm"));
    let (mut e, mut h) = Engine::build_seams(&g, RATE as f32, 1.0, Build::detect(), Sequencing::InBlock, &seams).unwrap();
    let mut boxes = h.events.take_outbox_slots();
    let (m, _) = h.tempo.map().expect("a tempo");
    let frames_a_beat = m.frame_of(1.0).unwrap() - m.frame_of(0.0).unwrap();
    let mut ctl = Controls::new(&g, Some(&mut h));
    let mut notes: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut out = Vec::new();
    for b in 0..(seconds * RATE) as usize / BLOCK {
        play(b as f64 * BLOCK as f64 / RATE, &mut ctl, &g);
        let mut buf = vec![0.0f32; BLOCK * 2];
        e.render(&mut buf, 2, |x| x);
        out.extend_from_slice(&buf);
        for (slot, ring) in &mut boxes {
            while let Ok(n) = ring.pop() {
                let n: Event = n;
                let Time::Beats(start) = n.start else { panic!("{slot}: {n:?}") };
                notes.entry(slot.clone()).or_default().push(start);
            }
        }
    }
    Run { notes, out, frames_a_beat }
}

fn set(ctl: &mut Controls, g: &Graph, path: &str, v: f32) {
    assert!(ctl.set_path(g, path, v), "no knob {path}");
}

/// The rig is the plan's: the cloud, its two voices, the mixer, the event cables from the cloud's thunder and kick;
/// the left stick plays the storm (x the rate, y the hold).
#[test]
fn the_rig_is_a_cloud_with_two_voices() {
    let s = scene();
    let ids: Vec<&str> = s.nodes.iter().map(|n| n.id.as_str()).collect();
    for id in ["storm_cloud", "storm_thunder", "storm_kick", "storm_mix"] {
        assert!(ids.contains(&id), "{id} in {ids:?}");
    }
    for (from, to) in [("storm_cloud.thunder", "storm_thunder.notes"), ("storm_cloud.kick", "storm_kick.notes")] {
        assert!(s.links.iter().any(|l| l.from == from && l.to == to), "{from} → {to}");
    }
    for (id, sound) in [("storm_thunder", "scutoons/sounds/cloud-thunder.wav"), ("storm_kick", "scutoons/sounds/cloud-kick.wav")] {
        let n = s.nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!(n.kind, "module:sampler", "{id}");
        assert_eq!(n.params["assets"]["sample"].as_str(), Some(sound), "{id} plays its sound");
    }
    let b: deck_space::rig::Bundle = toml::from_str(deck_native::rig::MODULE_RIGS[2].1).unwrap();
    assert_eq!(b.rig.subject, "item.cloud");
    let sticks = b.rig.sticks.clone().unwrap();
    assert_eq!(sticks[0], ["storm:cloud.rate".to_string(), "storm:cloud.hold".to_string()]);
}

/// The catalog's rig (server/tests/fixtures/rigs/cloud-storm.json – the rows migration 2026-11-24-cloud-light-sandbox
/// writes: `rig:item.cloud` and its instrument) is this rig, the puff's model in its cloud's slot. `UPDATE_FIXTURES=1` writes
/// the fixture from cloud_storm.toml.
#[test]
fn the_catalog_rig_is_this_rig() {
    let b: deck_space::rig::Bundle = toml::from_str(deck_native::rig::MODULE_RIGS[2].1).unwrap();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../server/tests/fixtures/rigs/cloud-storm.json");
    let text = serde_json::to_string_pretty(&b).unwrap() + "\n";
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let stored: deck_space::rig::Bundle = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(stored, b, "cloud-storm.json is the rig: UPDATE_FIXTURES=1 cargo test -p deck-native --features modules --test cloud_storm");
    let cloud = &b.instruments[0].nodes.iter().find(|n| n.id == "cloud").unwrap().params;
    let set: Vec<&str> = cloud["assets"]["puff"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(set, ["scutoons/3d/cloud-three-blobs.glb", "scutoons/3d/cloud-four-blobs.glb"]);
}

/// The cloud's table carries its puffs' set of models, loaded through the deck's loader like the samplers' sounds, in the
/// set's order: the space draws each puff with one of them (deck-space space/drawn.rs); the rain stays the placeholder segment.
#[test]
fn the_cloud_draws_its_puffs_with_the_library_model() {
    let s = scene();
    let g = Graph::build(&s).unwrap();
    let mut seams = Seams::with_deck(DeckConfig::device(2));
    seams.modules = Some(Arc::new(Unbudgeted));
    seams.assets = Some(stand_ins::loader("cloud-puff"));
    let (_e, h) = Engine::build_seams(&g, RATE as f32, 1.0, Build::detect(), Sequencing::InBlock, &seams).unwrap();
    let table = h.instances.iter().find(|t| t.node == "storm_cloud").expect("the cloud draws");
    let shapes: Vec<Option<u32>> = table.models.iter().map(|m| m.shape).collect();
    assert_eq!(shapes, [Some(0), Some(0)], "the puff (shape 0) only, two models");
    assert_eq!(&*table.models[0].bytes, include_bytes!("../../space/tests/fixtures/cloud-three-blobs.glb").as_slice());
    assert_eq!(&*table.models[1].bytes, include_bytes!("../../space/tests/fixtures/cloud-four-blobs.glb").as_slice());
}

/// Held, every repeat is a thunder and a kick on the 1/8 grid (the rig's rate 0.6); let go, nothing more
#[test]
fn held_it_repeats_on_the_grid_and_let_go_it_stops() {
    let r = run(3.0, |t, ctl, g| {
        if t == 0.0 {
            set(ctl, g, "storm_cloud.hold", 1.0);
        }
        if (2.0..2.0 + BLOCK as f64 / RATE).contains(&t) {
            set(ctl, g, "storm_cloud.hold", 0.0);
        }
    });
    let kick = &r.notes["storm_cloud.kick"];
    let thunder = &r.notes["storm_cloud.thunder"];
    assert_eq!(kick, thunder, "a thunder with every kick");
    assert!(kick.len() >= 6, "{kick:?}");
    for w in kick.windows(2) {
        assert!((w[1] - w[0] - 0.5).abs() < 1e-6, "eighths: {kick:?}");
    }
    assert!(kick.iter().all(|b| (b * 2.0).fract().abs() < 1e-6), "on the grid: {kick:?}");
    let stop = 2.0 * RATE / r.frames_a_beat;
    assert!(kick.iter().all(|&b| b < stop + 0.5), "silent once let go: {kick:?} (let go at beat {stop})");
    assert!(r.out.iter().all(|x| x.is_finite()));
}

/// The kick alone (no thunder) on quarter notes: an onset is heard at each repeat's frame and nowhere between
#[test]
fn the_kick_is_heard_on_the_repeats_frames() {
    let r = run(4.0, |t, ctl, g| {
        if t == 0.0 {
            set(ctl, g, "storm_cloud.rumble", 0.0);
            set(ctl, g, "storm_cloud.rate", 0.4);
            set(ctl, g, "storm_cloud.hold", 1.0);
        }
    });
    let mono: Vec<f32> = r.out.chunks(2).map(|c| c[0] + c[1]).collect();
    let rms = |a: usize, b: usize| (mono[a..b].iter().map(|v| v * v).sum::<f32>() / (b - a) as f32).sqrt();
    let kick = &r.notes["storm_cloud.kick"];
    assert!(kick.len() >= 4, "{kick:?}");
    // the rate glides from the rig's 0.6 to 0.4: the first repeats may still be eighths; from beat 2 on, quarters
    let onsets: Vec<usize> = kick.iter().filter(|&&b| b >= 2.0).map(|b| (b * r.frames_a_beat).round() as usize).filter(|&f| f + 2048 < mono.len()).collect();
    assert!(onsets.len() >= 3, "{kick:?}");
    for &f in &onsets {
        let (before, after) = (rms(f - 1024, f - 32), rms(f, f + 1024));
        assert!(after > 4.0 * before && after > 0.02, "an onset at frame {f}: {before} → {after}");
    }
    for w in onsets.windows(2) {
        let mid = (w[0] + w[1]) / 2;
        assert!(rms(mid, mid + 1024) < rms(w[0], w[0] + 1024) * 0.5, "no onset between {} and {}", w[0], w[1]);
    }
}

/// Free (sync off), the repeats follow the rate: 0.5 … 32 a second, here rate 0.5 = 4 a second
#[test]
fn free_it_follows_the_rate() {
    let r = run(3.0, |t, ctl, g| {
        if t == 0.0 {
            set(ctl, g, "storm_cloud.sync", 0.0);
            set(ctl, g, "storm_cloud.rate", 0.5);
        }
        if (1.0..1.0 + BLOCK as f64 / RATE).contains(&t) {
            set(ctl, g, "storm_cloud.hold", 1.0);
        }
    });
    let kick = &r.notes["storm_cloud.kick"];
    let seconds: Vec<f64> = kick.iter().map(|b| b * r.frames_a_beat / RATE).collect();
    // the rate glides from 0.6 to 0.5 over the first second, before the hold
    assert!(seconds.len() >= 6, "{seconds:?}");
    for w in seconds.windows(2) {
        assert!((w[1] - w[0] - 0.25).abs() < 0.002, "4 a second: {seconds:?}");
    }
}

#[test]
fn a_run_is_the_same_twice() {
    let hold = |t: f64, ctl: &mut Controls, g: &Graph| {
        if t == 0.0 {
            set(ctl, g, "storm_cloud.hold", 1.0);
        }
    };
    let (a, b) = (run(1.5, hold), run(1.5, hold));
    assert_eq!(a.notes, b.notes);
    assert_eq!(a.out, b.out, "two decks, one storm: the same samples");
    let peak = a.out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(peak > 0.05 && peak < 1.0, "it sounds, below full scale: {peak}");
}

/// `SAMPLER_OUT_DIR=<dir>`: the samplers' stand-in sounds (both rigs') as files, for upload under their CDN paths
#[test]
fn write_the_stand_ins_when_asked() {
    stand_ins::write_when_asked();
}

/// `CLOUD_OUT_DIR=<dir>`: 10 s played by hand as `<dir>/cloud-storm.wav` (48 kHz, stereo, 16-bit): held at 0.5 s on
/// quarters, the rate pushed up to 1/32 over 3 s, sync off at 5 s (free, the rate sliding down), let go at 8.5 s
#[test]
fn render_the_storm_when_asked() {
    let Ok(dir) = std::env::var("CLOUD_OUT_DIR") else { return };
    let r = run(10.0, |t, ctl, g| {
        if t == 0.0 {
            set(ctl, g, "storm_cloud.rate", 0.4);
        }
        if (0.5..5.0).contains(&t) {
            set(ctl, g, "storm_cloud.hold", 1.0);
            set(ctl, g, "storm_cloud.rate", (0.4 + 0.6 * ((t - 1.5) / 3.0)).clamp(0.4, 1.0) as f32);
        }
        if (5.0..8.5).contains(&t) {
            set(ctl, g, "storm_cloud.sync", 0.0);
            set(ctl, g, "storm_cloud.rate", (0.9 - 0.6 * (t - 5.0) / 3.5) as f32);
        }
        if t >= 8.5 {
            set(ctl, g, "storm_cloud.hold", 0.0);
        }
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
    std::fs::write(Path::new(&dir).join("cloud-storm.wav"), wav).unwrap();
}
