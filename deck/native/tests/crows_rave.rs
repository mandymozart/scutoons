//! The three crows rave (#747, research/audio-nodes-foundation.md §4): the sandbox scene `crows-rave` through the whole
//! engine with the real modules (arp, analog, drumseq, delay, reverb as wasm in wasmtime). A held A3 on MIDI input 1,
//! channel 1 plays three crows in eighths a diminished triad apart on the 160 BPM clock; the drum machine plays the groove
//! with the transport; Stop stops all four; two runs render the same samples. The headless host has no module runtime (#290), so this is the scene's scenario. `CROWS_OUT_DIR=<dir>` also
//! writes 8 s of it as `<dir>/crows-rave.wav` and its scene document as `<dir>/crows-rave.scene.json` (the editor opens it).
#![cfg(all(feature = "modules", feature = "events"))]
use deck_engine::{DeckConfig, Engine, ModuleSource, Opened, Seams, Sequencing};
use deck_events::adapters::midi::Midi;
use deck_events::{Event, Time};
use deck_module_host::{Mode, Setup};
use deck_native::audio::Build;
use deck_native::graph::Graph;
use deck_native::module_host::{Instance, Library};
use deck_native::modules;
use deck_native::scene::Scene;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

const RATE: f64 = 48_000.0;
const BLOCK: usize = 512;

/// arp, analog, drumseq, delay and reverb as release wasm, built once, loaded into the library
fn load_modules() {
    static LOADED: OnceLock<()> = OnceLock::new();
    LOADED.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).parent().unwrap().join("wasm-modules");
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let crates = ["arp", "analog", "drumseq", "delay", "reverb"];
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

/// the library's modules without a wall-clock deadline (as tests/modules.rs: what they play, not how fast)
struct Unbudgeted;

impl ModuleSource for Unbudgeted {
    fn open(&self, kind: deck_space::modreg::ModuleKind, rate: f64, seed: u64, assets: Vec<deck_module_host::AssetData>) -> anyhow::Result<Opened> {
        let loaded = Library::global().of_kind(kind).ok_or_else(|| anyhow::anyhow!("the module '{}' is not loaded", kind.spec().kind))?;
        let instance = Instance::with_setup(&loaded, assets.into_iter().fold(Setup::new(rate, seed).mode(Mode::Show).unbudgeted(), Setup::asset))?.into_host();
        Ok(Opened { instance: Box::new(instance), layout: loaded.layout.clone(), params: loaded.desc.params.len() })
    }
}

fn scene() -> Scene {
    Scene::parse(deck_space::scene::sandbox("crows-rave").expect("the sandbox")).unwrap()
}

fn param(scene: &Scene, node: &str, key: &str) -> f64 {
    let n = scene.nodes.iter().find(|n| n.id == node).unwrap_or_else(|| panic!("no node {node}"));
    let v = n.params.get(key).unwrap_or_else(|| panic!("{node}.{key}"));
    v.as_float().or_else(|| v.as_integer().map(|i| i as f64)).unwrap_or_else(|| panic!("{node}.{key} is no number"))
}

/// The scene as the plan draws it: 160 BPM, the MIDI world node on input 1 channel 1 cabled to the three arps, the arps a
/// minor third and a tritone apart, the voices' and drums' values analog.preset.json's presets, the strips' levels and pans.
#[test]
fn the_scene_is_the_plan_s() {
    let s = scene();
    assert_eq!(param(&s, "tempo", "bpm"), 160.0);
    assert_eq!((param(&s, "midi", "port"), param(&s, "midi", "channel")), (1.0, 1.0));
    let file: serde_json::Value = serde_json::from_str(include_str!("../../modules/analog/analog.preset.json")).unwrap();
    let preset = |name: &str| -> BTreeMap<String, f64> {
        let p = file["presets"].as_array().unwrap().iter().find(|p| p["name"] == name).unwrap_or_else(|| panic!("no preset {name}"));
        p["values"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_f64().unwrap())).collect()
    };
    let values = |node: &str| -> BTreeMap<String, f64> { s.nodes.iter().find(|n| n.id == node).unwrap().params.iter().map(|(k, v)| (k.clone(), v.as_float().unwrap())).collect() };
    for (crow, transpose) in [("wolf", 0.0), ("lizard", 3.0), ("fox", 6.0)] {
        assert_eq!(values(&format!("{crow}-voice")), preset("hoover-dissonant"), "{crow}: the hoover preset, written out");
        assert_eq!(param(&s, &format!("{crow}-arp"), "transpose"), transpose);
        assert_eq!((param(&s, &format!("{crow}-arp"), "rate"), param(&s, &format!("{crow}-arp"), "octaves")), (4.0, 2.0), "1/8, two octaves");
        assert!((20.0 * param(&s, &format!("{crow}-strip"), "gain").log10() + 9.0).abs() < 0.01, "{crow}: −9 dB");
        assert!(s.links.iter().any(|l| l.from == "midi.events" && l.to == format!("{crow}-arp.notes")), "{crow} hears MIDI");
        for bus in ["fx-delay", "fx-space"] {
            assert!(s.links.iter().any(|l| l.from == format!("{crow}-strip.out") && l.to == format!("{bus}.in")), "{crow} sends to {bus}");
        }
    }
    assert_eq!((param(&s, "lizard-pan", "azimuth"), param(&s, "fox-pan", "azimuth")), (-15.0, 15.0), "pans −0.5 / +0.5 as the rig strip's ±30°");
    for (i, lane) in ["kick", "snare", "hat-closed", "hat-open"].into_iter().enumerate() {
        assert_eq!(values(&format!("drum-{lane}")), preset(lane), "the {lane} lane");
        assert!(s.links.iter().any(|l| l.from == format!("drumseq.lane{}", i + 1) && l.to == format!("drum-{lane}.notes")), "lane {} plays {lane}", i + 1);
    }
    assert!((20.0 * param(&s, "drums", "gain").log10() + 6.0).abs() < 0.01, "drums −6 dB");
    // the same graph is a scene document the API takes (the production form, saved as a draft)
    let doc = deck_space::scene_document::SceneDocument::from_scene("crows-rave", vec!["sandbox".into()], &s);
    assert_eq!(doc.check(), Vec::<String>::new());
}

/// What one run heard: the notes each arp sent out (key, start in beats, length in beats) and the samples.
struct Run {
    notes: BTreeMap<String, Vec<(f64, f64, f64)>>,
    out: Vec<f32>,
}

/// `seconds` of the scene with A3 held on input 1 channel 1 from frame 0; the transport stops at `stop` s when given
fn play(seconds: f64, stop: Option<f64>) -> Run {
    load_modules();
    let s = scene();
    modules::check(&s).unwrap();
    let g = Graph::build(&s).unwrap();
    let mut seams = Seams::with_deck(DeckConfig::device(2));
    seams.modules = Some(Arc::new(Unbudgeted));
    let (mut e, mut h) = Engine::build_seams(&g, RATE as f32, 1.0, Build::detect(), Sequencing::InBlock, &seams).unwrap();
    let mut midi = Midi::new(&mut h.events.names, Some("midi:keys"), 1);
    let routing = h.events.take_routing();
    let mut outlets = routing.outlets;
    let mut boxes = h.events.take_outbox_slots();
    let mut feed = |bytes: [u8; 3]| {
        let mut evs = Vec::new();
        midi.feed(&bytes, 0.0, &mut evs);
        for ev in evs {
            for &t in routing.table.targets(ev.name) {
                outlets[t as usize].push(ev.clone()).unwrap();
            }
        }
    };
    feed([0x90, 57, 100]); // A3 down
    feed([0x91, 45, 100]); // channel 2: not the crows'
    let (key, names) = (h.events.names.get("key").unwrap(), &h.events.names);
    let mut notes: BTreeMap<String, Vec<(f64, f64, f64)>> = BTreeMap::new();
    let mut out = Vec::new();
    let blocks = (seconds * RATE) as usize / BLOCK;
    for b in 0..blocks {
        if let Some(at) = stop.filter(|&t| b == (t * RATE) as usize / BLOCK) {
            let (m, c) = h.tempo.map().unwrap();
            h.tempo.publish(&m.with_running(at * RATE, false), c);
        }
        let mut buf = vec![0.0f32; BLOCK * 2];
        e.render(&mut buf, 2, |x| x);
        out.extend_from_slice(&buf);
        for (slot, ring) in &mut boxes {
            while let Ok(n) = ring.pop() {
                let n: Event = n;
                assert_eq!(names.name(n.name), "music.note");
                let (Time::Beats(start), Some(Time::Beats(dur))) = (n.start, n.dur) else { panic!("{slot}: {n:?}") };
                notes.entry(slot.clone()).or_default().push((n.props.get(key).unwrap_or(f32::NAN) as f64, start, dur));
            }
        }
    }
    Run { notes, out }
}

fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

/// §4.5: holding one key plays three crows in eighths a diminished triad apart, on the clock – step k at beat k / 2, half
/// a step long, the note and its octave; what channel 2 plays stays out; the mix sounds, finite and below full scale.
#[test]
fn a_held_a3_plays_three_crows_in_eighths_a_diminished_triad_apart() {
    let run = play(3.0, None);
    let steps = (3.0 * 160.0 / 60.0 * 2.0) as usize; // eighths in 3 s
    for (crow, root) in [("wolf", 57.0), ("lizard", 60.0), ("fox", 63.0)] {
        let heard = run.notes.get(&format!("{crow}-arp.out")).unwrap_or_else(|| panic!("{crow} played nothing: {:?}", run.notes.keys()));
        assert!(heard.len() + 1 >= steps, "{crow}: {} steps of {steps}", heard.len());
        for (k, &(key, start, dur)) in heard.iter().enumerate() {
            let want = root + if k % 2 == 1 { 12.0 } else { 0.0 };
            assert_eq!(key, want, "{crow} step {k}: {heard:?}");
            assert!((start - k as f64 * 0.5).abs() < 1e-6, "{crow} step {k} at beat {start}");
            assert!((dur - 0.25).abs() < 1e-6, "{crow} step {k}: gate 50 % of an eighth, {dur} beats");
        }
    }
    assert!(run.out.iter().all(|x| x.is_finite()));
    let p = peak(&run.out);
    assert!(p > 0.05 && p < 1.0, "the crows sound, below full scale: peak {p}");
    assert_eq!(run.out, play(3.0, None).out, "two decks, one scene: the same samples");
}

/// §4.4: the drum machine plays the punk groove with the transport, one bar of 16ths again and again: the kick on 1 and 3
/// with a push on the "and" of 3, the snare on 2 and 4, closed hats on the eighths but the last, the open hat on the last "and"
#[test]
fn the_drum_machine_plays_the_groove_with_the_transport() {
    let run = play(2.95, None); // two bars at 160 BPM are 3 s: the last step heard is bar 2's open hat
    let lane = |n: usize, bar: &[f64]| {
        let heard: Vec<f64> = run.notes.get(&format!("drumseq.lane{n}")).unwrap_or_else(|| panic!("lane {n} played nothing")).iter().map(|n| n.1).collect();
        let want: Vec<f64> = bar.iter().chain(bar.iter().map(|b| b + 4.0).collect::<Vec<_>>().iter()).copied().collect();
        assert!(heard.len() == want.len() && heard.iter().zip(&want).all(|(h, w)| (h - w).abs() < 1e-6), "lane {n}: {heard:?}, not {want:?}");
    };
    lane(1, &[0.0, 2.0, 2.5]); // kick
    lane(2, &[1.0, 3.0]); // snare
    lane(3, &[0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0]); // closed hat
    lane(4, &[3.5]); // open hat
}

/// §4.5: Stop stops all four – no crow step and no drum after the transport stops, and the mix (voices, echoes, reverb)
/// dies away
#[test]
fn stop_stops_every_crow_and_the_drums() {
    let run = play(7.0, Some(1.0));
    for slot in ["wolf-arp.out", "lizard-arp.out", "fox-arp.out", "drumseq.lane1", "drumseq.lane3"] {
        let last = run.notes[slot].last().unwrap().1;
        assert!(last < 160.0 / 60.0 + 1e-6, "{slot}: a note at beat {last} after the stop at beat 2.67");
    }
    let tail = &run.out[(6.0 * RATE) as usize * 2..];
    assert!(peak(tail) < 1e-3, "silent 5 s after the stop: {}", peak(tail));
}

/// `CROWS_OUT_DIR=<dir>`: 8 s of A3 held, the drums running, as `<dir>/crows-rave.wav` (48 kHz, stereo, 16-bit), the document as JSON
#[test]
fn render_the_scene_when_asked() {
    let Ok(dir) = std::env::var("CROWS_OUT_DIR") else { return };
    let run = play(8.0, None);
    let data: Vec<u8> = run.out.iter().flat_map(|v| ((v.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes()).collect();
    let mut wav = Vec::with_capacity(44 + data.len());
    for part in [&b"RIFF"[..], &(36 + data.len() as u32).to_le_bytes(), b"WAVEfmt ", &16u32.to_le_bytes(), &1u16.to_le_bytes(), &2u16.to_le_bytes()] {
        wav.extend_from_slice(part);
    }
    for part in [&48_000u32.to_le_bytes()[..], &(48_000u32 * 4).to_le_bytes(), &4u16.to_le_bytes(), &16u16.to_le_bytes(), b"data", &(data.len() as u32).to_le_bytes()] {
        wav.extend_from_slice(part);
    }
    wav.extend_from_slice(&data);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(std::path::Path::new(&dir).join("crows-rave.wav"), wav).unwrap();
    let doc = deck_space::scene_document::SceneDocument::from_scene("crows-rave", vec!["sandbox".into()], &scene());
    std::fs::write(std::path::Path::new(&dir).join("crows-rave.scene.json"), doc.to_json().unwrap()).unwrap();
}
