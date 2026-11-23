//! The storm cloud's light (#780): the sandbox `cloud-light` is the rig `cloud-storm` flattened, held, tinted, with an
//! `[outputs]` that pulses the deck's RGB wash (the Thunder Wash 600 RGBW at DMX 1) in the cloud's colour at every
//! repeat – `storm_cloud.red` / `green` / `blue` → `wash.red` / `green` / `blue`, each gained by the top three quarters
//! of `storm_cloud.pulse` (1 at a repeat, fading by 0.12 s), so it is dark again before the next eighth; and a strobe
//! (the generic profile at DMX 10) triggered by `storm_cloud.gate` (1 for the first half of each period), so it flashes
//! once at every repeat at every rate – the pulse's fade would not re-arm it between repeats faster than about 1/8.
//! Through the whole engine with the real modules and deck-outputs' engine as the test sink (no widget): each DMX value =
//! colour × pulse in 8 bit at its 40 Hz frame, lit at each repeat, dark between; one strobe flash a repeat at 1/8 and 1/32.
//! The sandbox is written by `the_sandbox_is_the_rig_with_its_light`, never by hand:
//!   UPDATE_FIXTURES=1 cargo test -p deck-native --features modules --test cloud_light
//! `CLOUD_OUT_DIR=<dir>` also writes a held second at 1/32 of the wash's channels and the strobe as `<dir>/cloud-light.csv`.
#![cfg(all(feature = "modules", feature = "events", feature = "outputs"))]
use deck_engine::{DeckConfig, Engine, ModuleSource, Opened, Seams, Sequencing};
use deck_events::{Event, Time};
use deck_module_host::{Mode, Setup};
use deck_native::audio::{Build, QUANTUM};
use deck_native::graph::{Graph, PortType};
use deck_native::module_host::{Instance, Library};
use deck_native::modules;
use deck_native::scene::Scene;
use deck_outputs::{Engine as Outputs, Out, Timed};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

const RATE: f64 = 48_000.0;
const SANDBOX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scenes/sandbox/cloud-light.toml");
/// the wash's colour channels (address 1, profile 4CH_1)
const RGB: [u16; 3] = [1, 2, 3];
/// the strobe's flash channel (the generic profile's dimmer, address 10: clear of the wash's 1–4)
const FLASH: u16 = 10;

const HEADER: &str = "\
# A sandbox (#780): the storm cloud's light – the rig `cloud-storm` flattened, held (its repeater plays a thunder and a
# kick on every eighth) and tinted storm-blue, and the deck's RGB wash (Cameo Thunder Wash 600 RGBW, 4CH_1, DMX address 1 on the first USB-DMX
# widget) flashing in the cloud's colour at the repeater's speed: red / green / blue each gained by the cloud's pulse
# (its top three quarters, so the wash is dark again 0.17 s after a repeat); and a generic strobe (profile `strobe`, DMX
# address 10 – clear of the wash's 1–4) flashing 30 ms at every repeat, triggered by the cloud's gate (1 for the first half
# of each period, so it re-arms between repeats at every rate, where the pulse's fade would not).
# The cloud's period, phase, pulse, gate, red, green, blue are control paths (`storm_cloud.<port>`) a [[link]] or an
# [[outputs.map]] takes further (a MIDI CC or note, the conductor); the sound leaves through the mixer like any track.
# Written by deck-native's tests/cloud_light.rs `the_sandbox_is_the_rig_with_its_light` – never by hand:
#   UPDATE_FIXTURES=1 cargo test -p deck-native --features modules --test cloud_light
# From deck/ (modules: cloud, sampler, mixer in DECK_MODULES):
#   cargo run -p deck-native --release --features modules -- --dev --scene cloud-light --windowed

";

/// the light, in the flattened rig's paths
const OUTPUTS: &str = r#"
dmx = "auto"

[[fixture]]
id = "wash"
profile = "thunderwash-600-rgbw"
address = 1

[[map]]
from = "storm_cloud.red"
gain = [{ from = "storm_cloud.pulse", in = [0.25, 1] }]
to = [{ dmx = "wash.red" }]

[[map]]
from = "storm_cloud.green"
gain = [{ from = "storm_cloud.pulse", in = [0.25, 1] }]
to = [{ dmx = "wash.green" }]

[[map]]
from = "storm_cloud.blue"
gain = [{ from = "storm_cloud.pulse", in = [0.25, 1] }]
to = [{ dmx = "wash.blue" }]

[[fixture]]
id = "strobe"
profile = "strobe"
address = 10

[[strobe]]
fixture = "strobe"
trigger = "storm_cloud.gate"
duration = 30
"#;

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

struct Unbudgeted;

impl ModuleSource for Unbudgeted {
    fn open(&self, kind: deck_space::modreg::ModuleKind, rate: f64, seed: u64, assets: Vec<deck_module_host::AssetData>) -> anyhow::Result<Opened> {
        let loaded = Library::global().of_kind(kind).ok_or_else(|| anyhow::anyhow!("the module '{}' is not loaded", kind.spec().kind))?;
        let instance = Instance::with_setup(&loaded, assets.into_iter().fold(Setup::new(rate, seed).mode(Mode::Show).unbudgeted(), Setup::asset))?.into_host();
        Ok(Opened { instance: Box::new(instance), layout: loaded.layout.clone(), params: loaded.desc.params.len() })
    }
}

/// the rig flattened, its cloud held and tinted, with the light
fn from_rig() -> Scene {
    load_modules();
    let mut s = deck_native::rig::scene(Path::new("rig:cloud-storm")).unwrap();
    s.stage.name = "Cloud light".into();
    let cloud = s.nodes.iter_mut().find(|n| n.id == "storm_cloud").expect("the cloud");
    cloud.params.insert("hold".into(), toml::Value::Float(1.0));
    // a storm-blue cloud, so the wash shows its colour (the rig's own is a grey)
    cloud.params.insert("hue".into(), toml::Value::Float(0.62));
    cloud.params.insert("tint".into(), toml::Value::Float(60.0));
    s.outputs = Some(toml::from_str(OUTPUTS).unwrap());
    s
}

fn sandbox() -> Scene {
    load_modules();
    Scene::parse(&std::fs::read_to_string(SANDBOX).expect("the sandbox cloud-light.toml")).unwrap()
}

/// The sandbox is the rig as it plays, held, with its light; the cloud's time and colour are paths a scene can take on
#[test]
fn the_sandbox_is_the_rig_with_its_light() {
    let s = from_rig();
    let text = format!("{HEADER}{}", toml::to_string(&s).unwrap());
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(SANDBOX, &text).unwrap();
    }
    let on_disk = std::fs::read_to_string(SANDBOX).unwrap().replace("\r\n", "\n");
    assert_eq!(on_disk, text, "cloud-light.toml is written by this test (UPDATE_FIXTURES=1)");
    assert_eq!(sandbox(), s);
    modules::check(&s).unwrap();
    let g = Graph::build(&s).unwrap();
    let paths = deck_native::outputs::paths(&g);
    for port in ["period", "phase", "pulse", "gate", "red", "green", "blue"] {
        assert!(paths.contains(&format!("storm_cloud.{port}")), "storm_cloud.{port} in {paths:?}");
    }
    let plan = deck_outputs::parse(s.outputs.as_ref().unwrap(), Some(&paths), &|n| Err(n.to_owned())).unwrap();
    assert_eq!(plan.paths, ["storm_cloud.gate", "storm_cloud.red", "storm_cloud.pulse", "storm_cloud.green", "storm_cloud.blue"]);
    assert_eq!(plan.targets.len(), 3);
    assert_eq!(plan.strobes.len(), 1);
    assert_eq!(plan.strobes[0].flash, FLASH);
    assert!(plan.strobes[0].trigger.is_some(), "the gate's rise, not the grid");
    assert_eq!(plan.strobes[0].rate.max_hz, None, "the generic profile: no cap");
    for (t, ch) in plan.targets.iter().zip(RGB) {
        assert!(matches!(t.wire, deck_outputs::config::Wire::Dmx { ch: c, fine: None } if c == ch), "{:?}", t.wire);
        assert_eq!(t.gains.len(), 1);
        assert_eq!(t.smooth, 0.0, "a flash, not a fade");
    }
    let doc = deck_space::scene_document::SceneDocument::from_scene("cloud-light", vec!["sandbox".into()], &s);
    assert_eq!(doc.check(), Vec::<String>::new());
}

enum Slot {
    In(usize, &'static str),
    Out(usize, usize),
}

fn slot(g: &Graph, path: &str) -> Slot {
    let (id, port) = path.split_once('.').unwrap();
    let i = g.nodes.iter().position(|n| n.id == id).unwrap_or_else(|| panic!("no node {id}"));
    let kind = &g.nodes[i].kind;
    if let Some(k) = kind.input(port).filter(|&k| kind.inputs()[k].ty == PortType::Control) {
        return Slot::In(i, kind.inputs()[k].name);
    }
    Slot::Out(i, kind.output(port).filter(|&k| kind.outputs()[k].ty == PortType::Control).unwrap_or_else(|| panic!("no control {path}")))
}

/// One step of the outputs: when, the cloud's [pulse, red, green, blue] and gate it saw, the wash's RGB and the strobe's
/// flash channel after it
#[derive(Debug, PartialEq)]
struct Frame {
    at: f64,
    seen: [f32; 4],
    gate: f32,
    wash: [u8; 3],
    strobe: u8,
    sent: Vec<Timed>,
}

#[derive(Debug, PartialEq)]
struct Run {
    frames: Vec<Frame>,
    /// the repeats (s), from the kick's event cable
    repeats: Vec<f64>,
    out: Vec<f32>,
}

fn play(seconds: f64) -> Run {
    run(sandbox(), seconds)
}

/// the sandbox with the cloud's `rate` and `hold` set
fn with(rate: f64, hold: f64) -> Scene {
    let mut s = sandbox();
    let cloud = s.nodes.iter_mut().find(|n| n.id == "storm_cloud").unwrap();
    cloud.params.insert("rate".into(), toml::Value::Float(rate));
    cloud.params.insert("hold".into(), toml::Value::Float(hold));
    s
}

fn run(s: Scene, seconds: f64) -> Run {
    modules::check(&s).unwrap();
    let g = Graph::build(&s).unwrap();
    let plan = deck_outputs::parse(s.outputs.as_ref().unwrap(), Some(&deck_native::outputs::paths(&g)), &|n| Err(n.to_owned())).unwrap();
    let slots: Vec<Slot> = plan.paths.iter().map(|p| slot(&g, p)).collect();
    let mut outputs = Outputs::new(plan);
    let mut seams = Seams::with_deck(DeckConfig::device(2));
    seams.modules = Some(Arc::new(Unbudgeted));
    let (mut e, mut h) = Engine::build_seams(&g, RATE as f32, 1.0, Build::detect(), Sequencing::InBlock, &seams).unwrap();
    let mut boxes = h.events.take_outbox_slots();
    let (m, _) = h.tempo.map().expect("a tempo");
    let spb = (m.frame_of(1.0).unwrap() - m.frame_of(0.0).unwrap()) / RATE;
    let cloud = g.nodes.iter().position(|n| n.id == "storm_cloud").unwrap();
    let port = |name: &str| g.nodes[cloud].kind.output(name).unwrap();
    let ports = [port("pulse"), port("red"), port("green"), port("blue")];
    let gate = port("gate");
    let mut run = Run { frames: vec![], repeats: vec![], out: vec![] };
    let (mut values, mut wake, mut wash, mut strobe) = (vec![0.0f32; slots.len()], 0.0f64, [0u8; 3], 0u8);
    for b in 0..(seconds * RATE) as usize / QUANTUM {
        let mut buf = vec![0.0f32; QUANTUM * 2];
        e.render(&mut buf, 2, |x| x);
        run.out.extend_from_slice(&buf);
        for (slot, ring) in &mut boxes {
            while let Ok(n) = ring.pop() {
                let n: Event = n;
                if slot == "storm_cloud.kick" {
                    let Time::Beats(start) = n.start else { panic!("{n:?}") };
                    run.repeats.push(start * spb);
                }
            }
        }
        let now = (b + 1) as f64 * QUANTUM as f64 / RATE;
        if now >= wake - 1e-9 {
            let c = e.control();
            for (v, s) in values.iter_mut().zip(&slots) {
                *v = match *s {
                    Slot::In(n, p) => c.input(n, p, 0.0),
                    Slot::Out(n, k) => c.outputs(n)[k],
                };
            }
            let seen = ports.map(|k| c.outputs(cloud)[k]);
            let mut sent = Vec::new();
            wake = outputs.step(now, &values, None, &mut sent);
            for m in &sent {
                if let Out::Dmx { ch, value, .. } = m.out {
                    if let Some(i) = RGB.iter().position(|&c| c == ch) {
                        wash[i] = value;
                    }
                    if ch == FLASH {
                        strobe = value;
                    }
                }
            }
            run.frames.push(Frame { at: now, seen, gate: c.outputs(cloud)[gate], wash, strobe, sent });
        }
    }
    run
}

/// the pulse as the light takes it: its top three quarters, so the flash is out 0.17 s after a repeat (the pulse's tail
/// would keep a fast storm from ever going dark)
fn gain(pulse: f32) -> f32 {
    ((pulse - 0.25) / 0.75).clamp(0.0, 1.0)
}

/// Each colour channel the wash gets is the cloud's colour × its pulse in 8 bit, at its frame
#[test]
fn the_wash_is_the_colour_times_the_pulse() {
    let run = play(2.0);
    let mut sent = 0;
    for f in &run.frames {
        for m in &f.sent {
            let Out::Dmx { ch, value, .. } = m.out else { continue };
            let Some(i) = RGB.iter().position(|&c| c == ch) else { continue };
            let want = (gain(f.seen[0]) * f.seen[1 + i].clamp(0.0, 1.0) * 255.0).round();
            assert!((f32::from(value) - want).abs() <= 1.0, "at {} s channel {ch}: {value}, want {want} from {:?}", f.at, f.seen);
            sent += 1;
        }
    }
    assert!(sent > 20, "the wash moved: {sent} values");
    // the colour is the cloud's: not grey, not black
    let lit = run.frames.iter().map(|f| f.wash).max_by_key(|w| w.iter().map(|&v| u32::from(v)).sum::<u32>()).unwrap();
    assert!(lit.iter().any(|&v| v > 100) && lit.iter().max() != lit.iter().min(), "a coloured flash: {lit:?}");
}

/// Held on eighths: the wash flashes at every repeat (within a frame and a block) and is dark before the next
#[test]
fn the_wash_flashes_at_each_repeat_and_is_dark_between() {
    let run = play(3.0);
    let r = &run.repeats;
    assert!(r.len() >= 8, "repeats {r:?}");
    let late = deck_outputs::engine::FRAME_S + 2.0 * QUANTUM as f64 / RATE + 1e-6;
    let bright = |f: &Frame| f.wash.iter().map(|&v| u32::from(v)).sum::<u32>();
    for w in r.windows(2) {
        let lit = run.frames.iter().find(|f| f.at >= w[0] && bright(f) > 0).unwrap_or_else(|| panic!("no flash after {}", w[0]));
        assert!(lit.at - w[0] < late, "the flash at {} s for the repeat at {} s", lit.at, w[0]);
        assert!(lit.seen[0] > 0.6, "lit near the pulse's top: {:?}", lit.seen);
        // the light's pulse is out after 0.12 s × ln 4 = 0.17 s; the next repeat is an eighth later
        let dark = run.frames.iter().filter(|f| f.at > w[0] + 0.17 + deck_outputs::engine::FRAME_S && f.at < w[1] - 1e-6);
        assert!(dark.clone().count() > 0 && dark.clone().all(|f| bright(f) == 0), "dark between {} and {}", w[0], w[1]);
    }
}

/// when the strobe's flashes went out (s)
fn flashes(run: &Run) -> Vec<f64> {
    run.frames.iter().flat_map(|f| &f.sent).filter_map(|m| matches!(m.out, Out::Dmx { ch: FLASH, value, now: true } if value > 0).then_some(m.at)).collect()
}

/// The strobe flashes once at every repeat – at 1/8 and at 1/32 (62.5 ms at 120 BPM, where the pulse never falls
/// below the trigger's 0.25 between repeats) – within a block and a poll after it
#[test]
fn the_strobe_flashes_once_at_each_repeat_at_1_8_and_1_32() {
    for (rate, what, least) in [(0.6, "1/8", 10), (1.0, "1/32", 40)] {
        let seconds = 3.0;
        let run = run(with(rate, 1.0), seconds);
        let r = &run.repeats;
        assert!(r.len() >= least, "{what}: repeats {r:?}");
        let on = flashes(&run);
        let late = 2.0 * QUANTUM as f64 / RATE + deck_outputs::engine::POLL_S + 1e-6;
        // the repeats a flash can follow inside the run
        let seen: Vec<f64> = r.iter().copied().filter(|&t| t + late < seconds - QUANTUM as f64 / RATE).collect();
        for (k, &t) in seen.iter().enumerate() {
            let next = seen.get(k + 1).copied().unwrap_or(f64::INFINITY);
            let mine: Vec<f64> = on.iter().copied().filter(|&f| f >= t - 1e-9 && f < next - 1e-9).collect();
            assert_eq!(mine.len(), 1, "{what}: one flash for the repeat at {t} s: {mine:?}");
            assert!(mine[0] - t < late, "{what}: the flash at {} s for the repeat at {t} s", mine[0]);
        }
        assert!(on.iter().all(|&f| f >= r[0] - 1e-9), "{what}: no flash before the first repeat");
        assert!(on.len() >= seen.len() && on.len() <= r.len(), "{what}: {} flashes for {} repeats", on.len(), r.len());
        let offs = run.frames.iter().flat_map(|f| &f.sent).filter(|m| matches!(m.out, Out::Dmx { ch: FLASH, value: 0, now: true })).count();
        assert!(offs + 1 >= on.len(), "{what}: every flash ends");
    }
}

/// Not held: no repeats, the gate low, the strobe dark
#[test]
fn no_flash_while_not_held() {
    let run = run(with(1.0, 0.0), 1.5);
    assert!(run.repeats.is_empty());
    assert!(run.frames.iter().all(|f| f.gate == 0.0 && f.strobe == 0));
    assert!(flashes(&run).is_empty());
}

/// two decks, one storm: the same light
#[test]
fn a_run_is_the_same_twice() {
    let (a, b) = (play(1.5), play(1.5));
    assert_eq!(a, b);
}

/// `CLOUD_OUT_DIR=<dir>`: a held second at 1/32 of the wash's channels and the strobe per step of the outputs as
/// `<dir>/cloud-light.csv`
#[test]
fn write_the_light_when_asked() {
    let Ok(dir) = std::env::var("CLOUD_OUT_DIR") else { return };
    let run = run(with(1.0, 1.0), 1.0);
    let mut csv = String::from("t_s,repeat,pulse,gate,red,green,blue,dmx1_red,dmx2_green,dmx3_blue,dmx10_strobe\n");
    let mut next = run.repeats.iter().peekable();
    for f in &run.frames {
        let mut repeat = String::new();
        while let Some(&&t) = next.peek().filter(|&&&t| t <= f.at) {
            repeat = format!("{t:.4}");
            next.next();
        }
        let [p, r, g, b] = f.seen;
        let [d1, d2, d3] = f.wash;
        csv += &format!("{:.4},{repeat},{p:.3},{:.0},{r:.3},{g:.3},{b:.3},{d1},{d2},{d3},{}\n", f.at, f.gate, f.strobe);
    }
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(Path::new(&dir).join("cloud-light.csv"), csv).unwrap();
}
