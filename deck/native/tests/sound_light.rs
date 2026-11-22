//! Sound → light and picture (research/audio-nodes-foundation.md §3 and §5 step 14): the sandbox scene `sound-light`
//! through the whole engine with the real modules (drumseq, analog, env, stepseq as wasm in wasmtime) and its `[outputs]`
//! through deck-outputs' engine, stepped on the audio clock as a host steps it – no device: the DMX messages are collected.
//! The bass's `env` drives the space node's glow (a link), its `level` a DMX dimmer (a map), and drum lane 1 – the kick –
//! through an `env` one-shot the strobe's trigger (one flash a kick). `SOUND_LIGHT_OUT_DIR=<dir>` also writes the scene
//! document as `<dir>/sound-light.scene.json` (the editor opens it) and the glow per block as `<dir>/sound-light.glow.csv`.
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
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

const RATE: f64 = 48_000.0;
const BPM: f64 = 160.0;
/// the strobe's flash channel (fixture `strobe` at 1) and the dimmer's (fixture `wash` at 10)
const FLASH: u16 = 1;
const DIMMER: u16 = 10;

fn load_modules() {
    static LOADED: OnceLock<()> = OnceLock::new();
    LOADED.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_owned();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .parent()
            .unwrap()
            .join("wasm-modules");
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let crates = ["analog", "drumseq", "env", "stepseq"];
        let mut args: Vec<String> = ["build", "--release", "--target", "wasm32-unknown-unknown"]
            .map(String::from)
            .to_vec();
        for c in crates {
            args.extend(["-p".into(), format!("{c}-module")]);
        }
        let out = std::process::Command::new(cargo)
            .current_dir(&root)
            .args(&args)
            .arg("--target-dir")
            .arg(&dir)
            .output()
            .expect("running cargo");
        assert!(
            out.status.success(),
            "building the modules for wasm32 failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        for c in crates {
            let wasm = std::fs::read(
                dir.join("wasm32-unknown-unknown/release")
                    .join(format!("{c}_module.wasm")),
            )
            .unwrap();
            Library::global().load(&wasm).unwrap();
        }
    });
}

/// the library's modules without a wall-clock deadline (what they play, not how fast)
struct Unbudgeted;

impl ModuleSource for Unbudgeted {
    fn open(
        &self,
        kind: deck_space::modreg::ModuleKind,
        rate: f64,
        seed: u64,
    ) -> anyhow::Result<Opened> {
        let loaded = Library::global()
            .of_kind(kind)
            .ok_or_else(|| anyhow::anyhow!("the module '{}' is not loaded", kind.spec().kind))?;
        let instance = Instance::with_setup(
            &loaded,
            Setup::new(rate, seed).mode(Mode::Show).unbudgeted(),
        )?
        .into_host();
        Ok(Opened {
            instance: Box::new(instance),
            layout: loaded.layout.clone(),
            params: loaded.desc.params.len(),
        })
    }
}

fn scene() -> Scene {
    Scene::parse(deck_space::scene::sandbox("sound-light").expect("the sandbox")).unwrap()
}

/// Where an outputs path's value is in the engine's control half: a node's control input or one of its control outputs.
enum Slot {
    In(usize, &'static str),
    Out(usize, usize),
}

fn slot(g: &Graph, path: &str) -> Slot {
    let (id, port) = path.split_once('.').unwrap();
    let i = g
        .nodes
        .iter()
        .position(|n| n.id == id)
        .unwrap_or_else(|| panic!("no node {id}"));
    let kind = &g.nodes[i].kind;
    if let Some(k) = kind
        .input(port)
        .filter(|&k| kind.inputs()[k].ty == PortType::Control)
    {
        return Slot::In(i, kind.inputs()[k].name);
    }
    Slot::Out(
        i,
        kind.output(port)
            .filter(|&k| kind.outputs()[k].ty == PortType::Control)
            .unwrap_or_else(|| panic!("no control {path}")),
    )
}

/// What one run saw, a value per block (`QUANTUM` frames): the bass's env and level, the space node's glow; the kicks lane 1
/// sent (s); every message the outputs sent; the dimmer's value per block (what the lamp shows).
#[derive(PartialEq, Debug)]
struct Run {
    env: Vec<f32>,
    level: Vec<f32>,
    glow: Vec<f32>,
    kicks: Vec<f64>,
    sent: Vec<Timed>,
    /// the block whose values each sent message came from
    from: Vec<usize>,
    dimmer: Vec<u8>,
}

fn play(seconds: f64) -> Run {
    load_modules();
    let s = scene();
    modules::check(&s).unwrap();
    let g = Graph::build(&s).unwrap();
    let plan = deck_outputs::parse(
        s.outputs.as_ref().unwrap(),
        Some(&deck_native::outputs::paths(&g)),
        &|n| Err(n.to_owned()),
    )
    .unwrap();
    let slots: Vec<Slot> = plan.paths.iter().map(|p| slot(&g, p)).collect();
    let mut outputs = Outputs::new(plan);
    let mut seams = Seams::with_deck(DeckConfig::device(2));
    seams.modules = Some(Arc::new(Unbudgeted));
    let (mut e, mut h) = Engine::build_seams(
        &g,
        RATE as f32,
        1.0,
        Build::detect(),
        Sequencing::InBlock,
        &seams,
    )
    .unwrap();
    let mut boxes = h.events.take_outbox_slots();
    let at = |id: &str| g.nodes.iter().position(|n| n.id == id).unwrap();
    let (bass, space) = (at("bass"), at("space"));
    let port = |name: &str| g.nodes[bass].kind.output(name).unwrap();
    let (env_k, level_k) = (port("env"), port("level"));
    let mut run = Run {
        env: vec![],
        level: vec![],
        glow: vec![],
        kicks: vec![],
        sent: vec![],
        from: vec![],
        dimmer: vec![],
    };
    let (mut values, mut wake, mut dimmer) = (vec![0.0f32; slots.len()], 0.0f64, 0u8);
    for b in 0..(seconds * RATE) as usize / QUANTUM {
        let mut buf = vec![0.0f32; QUANTUM * 2];
        e.render(&mut buf, 2, |x| x);
        assert!(buf.iter().all(|x| x.is_finite()));
        let c = e.control();
        run.env.push(c.outputs(bass)[env_k]);
        run.level.push(c.outputs(bass)[level_k]);
        run.glow.push(c.input(space, "glow", -1.0));
        for (slot, ring) in &mut boxes {
            while let Ok(n) = ring.pop() {
                let n: Event = n;
                if slot == "drumseq.lane1" {
                    let Time::Beats(start) = n.start else {
                        panic!("{n:?}")
                    };
                    run.kicks.push(start * 60.0 / BPM);
                }
            }
        }
        // the outputs on the audio clock: the end of this block, when the engine is due (a host's wake)
        let now = (b + 1) as f64 * QUANTUM as f64 / RATE;
        if now >= wake - 1e-9 {
            for (v, s) in values.iter_mut().zip(&slots) {
                *v = match *s {
                    Slot::In(n, p) => c.input(n, p, 0.0),
                    Slot::Out(n, k) => c.outputs(n)[k],
                };
            }
            let from = run.sent.len();
            wake = outputs.step(now, &values, None, &mut run.sent);
            run.from.resize(run.sent.len(), b);
            for m in &run.sent[from..] {
                if let Out::Dmx {
                    ch: DIMMER, value, ..
                } = m.out
                {
                    dimmer = value;
                }
            }
        }
        run.dimmer.push(dimmer);
    }
    run
}

fn flashes(sent: &[Timed]) -> Vec<f64> {
    sent.iter()
        .filter_map(|m| {
            matches!(m.out, Out::Dmx { ch: FLASH, value, now: true } if value > 0).then_some(m.at)
        })
        .collect()
}

/// The scene draws the three hooks as wires, none by default: env → glow a link, level → the dimmer a map, the kick's
/// gate → the strobe's trigger; and it is a scene document the API takes
#[test]
fn the_scene_draws_the_three_hooks() {
    load_modules();
    let s = scene();
    assert!(s
        .links
        .iter()
        .any(|l| l.from == "bass.env" && l.to == "space.glow"));
    assert!(
        s.links
            .iter()
            .any(|l| l.from == "drumseq.lane1" && l.to == "kickgate.gate")
            && s.links
                .iter()
                .any(|l| l.from == "drumseq.lane1" && l.to == "kick.notes")
    );
    let g = Graph::build(&s).unwrap();
    let plan = deck_outputs::parse(
        s.outputs.as_ref().unwrap(),
        Some(&deck_native::outputs::paths(&g)),
        &|n| Err(n.to_owned()),
    )
    .unwrap();
    assert_eq!(plan.paths, ["kickgate.env", "bass.level"]);
    assert_eq!(plan.strobes[0].flash, FLASH);
    assert!(matches!(
        plan.targets[0].wire,
        deck_outputs::config::Wire::Dmx {
            ch: DIMMER,
            fine: None
        }
    ));
    let doc = deck_space::scene_document::SceneDocument::from_scene(
        "sound-light",
        vec!["sandbox".into()],
        &s,
    );
    assert_eq!(doc.check(), Vec::<String>::new());
}

/// Two bars at 160 BPM (3 s): the kick plays lane 1's groove – beats 0, 2 and 2.5 a bar, six kicks – and the strobe
/// flashes exactly once a kick, each within 11 ms after it (the gate's rise: a block behind the lane, seen within POLL_S), 30 ms long
#[test]
fn one_flash_per_kick_over_two_bars() {
    let run = play(3.0);
    let want: Vec<f64> = [0.0, 2.0, 2.5, 4.0, 6.0, 6.5]
        .iter()
        .map(|b| b * 60.0 / BPM)
        .collect();
    assert!(
        run.kicks.len() == want.len()
            && run
                .kicks
                .iter()
                .zip(&want)
                .all(|(a, b)| (a - b).abs() < 1e-6),
        "kicks {:?}",
        run.kicks
    );
    let on = flashes(&run.sent);
    assert_eq!(
        on.len(),
        run.kicks.len(),
        "flashes {on:?} for kicks {:?}",
        run.kicks
    );
    // the gate's env is a block behind the lane, read at a block's end, looked at every POLL_S
    let late = deck_outputs::engine::POLL_S + 2.0 * QUANTUM as f64 / RATE + 1e-6;
    for (f, k) in on.iter().zip(&run.kicks) {
        assert!(
            *f >= *k && *f - *k < late,
            "the flash at {f} s for the kick at {k} s"
        );
        let off = run
            .sent
            .iter()
            .find(|m| {
                m.at > *f
                    && matches!(
                        m.out,
                        Out::Dmx {
                            ch: FLASH,
                            value: 0,
                            now: true
                        }
                    )
            })
            .expect("the flash ends");
        assert!(
            (off.at - f - 0.030).abs() < 0.003,
            "30 ms (to the block): {}",
            off.at - f
        );
    }
}

/// The glow is the bass's env (a control link: the space node reads what the block left on it), and the env moves – note by note, up past 0.5 and down
/// near 0 between the phrase's rests – so the glow does
#[test]
fn the_glow_follows_the_env() {
    let run = play(3.0);
    for k in 0..run.glow.len() {
        assert!(
            (run.glow[k] - run.env[k]).abs() < 1e-6,
            "block {k}: glow {} env {}",
            run.glow[k],
            run.env[k]
        );
    }
    let (lo, hi) = run.glow[10..]
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    assert!(hi > 0.5 && lo < 0.2, "the glow moves: {lo} … {hi}");
}

/// The dimmer follows the bass's level: every DMX value the map sent is the level of its frame in 8 bit (no smoothing),
/// at most one a 25 ms frame; over the run the lamp is within 4 % of the level on average and never off by more than
/// the level moves in a frame and a block
#[test]
fn the_dmx_dimmer_follows_the_level() {
    let run = play(3.0);
    let block = QUANTUM as f64 / RATE;
    let sent: Vec<&Timed> = run
        .sent
        .iter()
        .filter(|m| matches!(m.out, Out::Dmx { ch: DIMMER, .. }))
        .collect();
    assert!(sent.len() > 20, "the dimmer moved {} times", sent.len());
    for w in sent.windows(2) {
        assert!(
            w[1].at - w[0].at > deck_outputs::engine::FRAME_S - block,
            "one a frame: {} → {}",
            w[0].at,
            w[1].at
        );
    }
    for (m, &b) in run.sent.iter().zip(&run.from) {
        if let Out::Dmx {
            ch: DIMMER, value, ..
        } = m.out
        {
            assert_eq!(
                value,
                deck_outputs::shape::to_8bit(run.level[b]),
                "at {} s",
                m.at
            );
        }
    }
    let frame = (deck_outputs::engine::FRAME_S / block).ceil() as usize + 1;
    let mut err = 0.0;
    for k in frame..run.level.len() {
        let lamp = f32::from(run.dimmer[k]) / 255.0;
        let window = &run.level[k - frame..=k];
        let (lo, hi) = window
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        assert!(
            lamp >= lo - 1.0 / 255.0 && lamp <= hi + 1.0 / 255.0,
            "block {k}: the lamp {lamp}, the level {lo} … {hi}"
        );
        err += (lamp - run.level[k]).abs();
    }
    let mean = err / (run.level.len() - frame) as f32;
    assert!(mean < 0.04, "mean distance {mean}");
    assert!(
        run.level.iter().copied().fold(0.0f32, f32::max) > 0.1,
        "the bass sounds"
    );
}

/// Two decks, one scene: the same env, glow, level, kicks, DMX messages and lamp, block for block
#[test]
fn deterministic() {
    assert_eq!(play(2.0), play(2.0));
}

/// `SOUND_LIGHT_OUT_DIR=<dir>`: the scene document and 3 s of the glow, the env and the lamps per block as CSV
#[test]
fn write_the_scene_when_asked() {
    let Ok(dir) = std::env::var("SOUND_LIGHT_OUT_DIR") else {
        return;
    };
    let run = play(3.0);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = deck_space::scene_document::SceneDocument::from_scene(
        "sound-light",
        vec!["sandbox".into()],
        &scene(),
    );
    std::fs::write(
        std::path::Path::new(&dir).join("sound-light.scene.json"),
        doc.to_json().unwrap(),
    )
    .unwrap();
    let block = QUANTUM as f64 / RATE;
    let on = flashes(&run.sent);
    let mut csv = String::from("t,env,glow,level,dimmer,flash\n");
    for k in 0..run.glow.len() {
        let t = (k + 1) as f64 * block;
        let lit = on.iter().any(|&f| t >= f - 1e-9 && t < f + 0.030);
        csv += &format!(
            "{t:.5},{:.4},{:.4},{:.4},{},{}\n",
            run.env[k],
            run.glow[k],
            run.level[k],
            run.dimmer[k],
            u8::from(lit)
        );
    }
    std::fs::write(std::path::Path::new(&dir).join("sound-light.glow.csv"), csv).unwrap();
}
