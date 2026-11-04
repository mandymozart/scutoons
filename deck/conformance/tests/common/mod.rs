//! Shared by the probes: the kinds' scenes, the two renderers (the native deck's engine, live-core after `lower()`),
//! the comparison numbers and the hash. Everything renders 2 s at 48 kHz.
#![allow(dead_code)]
use deck_native::audio::{Build, Engine};
use deck_native::graph::Graph;
use deck_native::scene::Scene;

pub const RATE: f32 = 48_000.0;
pub const FRAMES: usize = 2 * 48_000;
/// the block patterns of the plan: the browser's quantum, an odd one, the stage's largest
pub const PATTERNS: [usize; 3] = [128, 37, 1024];

/// One kind's scene (the stage's TOML) and what the plan expects of it.
pub struct Case {
    pub name: &'static str,
    pub toml: &'static str,
}

const OUT: &str = "[[node]]\nid = \"out\"\nkind = \"audio_out\"\nvolume = 1.0\n";

macro_rules! scene {
    ($body:expr) => {
        concat!($body, "[[node]]\nid = \"out\"\nkind = \"audio_out\"\nvolume = 1.0\n")
    };
}

pub const CASES: &[Case] = &[
    Case { name: "osc sine", toml: scene!("[[node]]\nid = \"o\"\nkind = \"osc\"\nfreq = 220.0\ngain = 0.2\n[[link]]\nfrom = \"o\"\nto = \"out.in\"\n") },
    Case { name: "osc saw", toml: scene!("[[node]]\nid = \"o\"\nkind = \"osc\"\nwave = \"saw\"\nfreq = 220.0\ngain = 0.2\n[[link]]\nfrom = \"o\"\nto = \"out.in\"\n") },
    Case { name: "osc square", toml: scene!("[[node]]\nid = \"o\"\nkind = \"osc\"\nwave = \"square\"\nfreq = 220.0\ngain = 0.2\n[[link]]\nfrom = \"o\"\nto = \"out.in\"\n") },
    Case { name: "osc triangle", toml: scene!("[[node]]\nid = \"o\"\nkind = \"osc\"\nwave = \"triangle\"\nfreq = 220.0\ngain = 0.2\n[[link]]\nfrom = \"o\"\nto = \"out.in\"\n") },
    Case { name: "fm", toml: scene!("[[node]]\nid = \"f\"\nkind = \"fm\"\nfreq = 220.0\nratio = 2.0\nindex = 1.5\ngain = 0.2\n[[link]]\nfrom = \"f\"\nto = \"out.in\"\n") },
    Case {
        name: "lfo sine -> osc.freq",
        toml: scene!("[[node]]\nid = \"l\"\nkind = \"lfo\"\nrate = 2.0\ndepth = 100.0\n[[node]]\nid = \"o\"\nkind = \"osc\"\ngain = 0.2\n[[link]]\nfrom = \"l\"\nto = \"o.freq\"\noffset = 300.0\n[[link]]\nfrom = \"o\"\nto = \"out.in\"\n"),
    },
    Case {
        name: "lfo saw -> osc.freq",
        toml: scene!("[[node]]\nid = \"l\"\nkind = \"lfo\"\nshape = \"saw\"\nrate = 2.0\ndepth = 100.0\n[[node]]\nid = \"o\"\nkind = \"osc\"\ngain = 0.2\n[[link]]\nfrom = \"l\"\nto = \"o.freq\"\noffset = 300.0\n[[link]]\nfrom = \"o\"\nto = \"out.in\"\n"),
    },
    Case { name: "noise white", toml: scene!("[[node]]\nid = \"n\"\nkind = \"noise\"\ncutoff = 4000.0\ngain = 0.1\n[[link]]\nfrom = \"n\"\nto = \"out.in\"\n") },
    Case { name: "noise pink", toml: scene!("[[node]]\nid = \"n\"\nkind = \"noise\"\ncolor = \"pink\"\ncutoff = 4000.0\ngain = 0.1\n[[link]]\nfrom = \"n\"\nto = \"out.in\"\n") },
    Case {
        name: "seq -> osc.cv",
        toml: scene!("[[node]]\nid = \"s\"\nkind = \"seq\"\nbpm = 120.0\ndivision = 4\ngate = 0.5\nsteps = [57, 0, 60, 64]\n[[node]]\nid = \"o\"\nkind = \"osc\"\ngain = 0.2\n[[link]]\nfrom = \"s.cv\"\nto = \"o.cv\"\n[[link]]\nfrom = \"o\"\nto = \"out.in\"\n"),
    },
    Case {
        name: "click",
        toml: scene!("[[node]]\nid = \"t\"\nkind = \"tempo\"\nbpm = 120.0\nclick = true\n[[node]]\nid = \"k\"\nkind = \"click\"\ngain = 0.5\n[[link]]\nfrom = \"k\"\nto = \"out.in\"\n"),
    },
    Case {
        name: "walk -> fm.cv",
        toml: scene!("[[node]]\nid = \"t\"\nkind = \"tempo\"\nbpm = 96\n[[node]]\nid = \"w\"\nkind = \"walk\"\nseed = 3\nbase = 147.0\nspread = 1.0\nrate = 2.0\n[[node]]\nid = \"v\"\nkind = \"fm\"\nratio = 1.5\nindex = 2.0\ngain = 0.2\n[[link]]\nfrom = \"w.cv\"\nto = \"v.cv\"\n[[link]]\nfrom = \"v\"\nto = \"out.in\"\n"),
    },
    Case {
        name: "mixer (2 osc, gains .8 .5)",
        toml: scene!("[[node]]\nid = \"a\"\nkind = \"osc\"\nfreq = 220.0\ngain = 0.2\n[[node]]\nid = \"b\"\nkind = \"osc\"\nfreq = 330.0\ngain = 0.2\n[[node]]\nid = \"m\"\nkind = \"mixer\"\ngain1 = 0.8\ngain2 = 0.5\n[[link]]\nfrom = \"a\"\nto = \"m.in1\"\n[[link]]\nfrom = \"b\"\nto = \"m.in2\"\n[[link]]\nfrom = \"m\"\nto = \"out.in\"\n"),
    },
    Case {
        name: "fft tap (osc -> fft, osc -> out)",
        toml: scene!("[[node]]\nid = \"o\"\nkind = \"osc\"\nfreq = 220.0\ngain = 0.2\n[[node]]\nid = \"f\"\nkind = \"fft\"\n[[link]]\nfrom = \"o\"\nto = \"f.in\"\n[[link]]\nfrom = \"o\"\nto = \"out.in\"\n"),
    },
];

/// #319: the control half at work – what moves at control rate into an audio input (the golden hashes, native ≡ wasm, the
/// frame-vs-block diff)
pub const CONTROL_CASES: &[Case] = &[
    Case {
        name: "lfo sine -> osc.gain (tremolo)",
        toml: scene!("[[node]]
id = \"l\"
kind = \"lfo\"
rate = 5.0
depth = 0.2
[[node]]
id = \"o\"
kind = \"osc\"
freq = 1000.0
[[link]]
from = \"l\"
to = \"o.gain\"
offset = 0.05
[[link]]
from = \"o\"
to = \"out.in\"
"),
    },
    Case {
        name: "lfo square -> osc.freq (trill)",
        toml: scene!("[[node]]
id = \"l\"
kind = \"lfo\"
shape = \"square\"
rate = 4.0
depth = 110.0
[[node]]
id = \"o\"
kind = \"osc\"
wave = \"saw\"
gain = 0.2
[[link]]
from = \"l\"
to = \"o.freq\"
offset = 330.0
[[link]]
from = \"o\"
to = \"out.in\"
"),
    },
    Case {
        name: "lfo -> lfo.rate -> fm.index (two levels)",
        toml: scene!("[[node]]
id = \"a\"
kind = \"lfo\"
rate = 0.3
depth = 6.0
[[node]]
id = \"b\"
kind = \"lfo\"
shape = \"saw\"
depth = 4.0
[[node]]
id = \"f\"
kind = \"fm\"
freq = 165.0
ratio = 2.0
gain = 0.2
[[link]]
from = \"a\"
to = \"b.rate\"
offset = 0.5
[[link]]
from = \"b\"
to = \"f.index\"
[[link]]
from = \"f\"
to = \"out.in\"
"),
    },
    Case {
        name: "lfo -> mul -> add -> smooth -> osc.freq (helpers)",
        toml: scene!("[[node]]
id = \"l\"
kind = \"lfo\"
shape = \"square\"
rate = 2.0
[[node]]
id = \"m\"
kind = \"mul\"
b = 220.0
[[node]]
id = \"a\"
kind = \"add\"
b = 220.0
[[node]]
id = \"s\"
kind = \"smooth\"
time = 0.05
[[node]]
id = \"o\"
kind = \"osc\"
gain = 0.2
[[link]]
from = \"l\"
to = \"m.a\"
[[link]]
from = \"m\"
to = \"a.a\"
[[link]]
from = \"a\"
to = \"s.in\"
[[link]]
from = \"s\"
to = \"o.freq\"
[[link]]
from = \"o\"
to = \"out.in\"
"),
    },
    Case {
        name: "tempo.pulse -> osc.gain (the beat)",
        toml: scene!("[[node]]
id = \"t\"
kind = \"tempo\"
bpm = 132.0
[[node]]
id = \"o\"
kind = \"osc\"
freq = 110.0
[[link]]
from = \"t.pulse\"
to = \"o.gain\"
scale = 0.3
[[link]]
from = \"o\"
to = \"out.in\"
"),
    },
    Case {
        name: "fft.level -> smooth -> osc.gain (sound -> control -> sound)",
        toml: scene!("[[node]]
id = \"l\"
kind = \"lfo\"
shape = \"square\"
rate = 1.5
depth = 0.3
[[node]]
id = \"a\"
kind = \"osc\"
freq = 220.0
[[node]]
id = \"f\"
kind = \"fft\"
[[node]]
id = \"s\"
kind = \"smooth\"
time = 0.1
[[node]]
id = \"b\"
kind = \"osc\"
freq = 660.0
[[link]]
from = \"l\"
to = \"a.gain\"
[[link]]
from = \"a\"
to = \"f.in\"
[[link]]
from = \"f.level\"
to = \"s.in\"
[[link]]
from = \"s\"
to = \"b.gain\"
scale = 0.2
[[node]]
id = \"mix\"
kind = \"mixer\"
[[link]]
from = \"a\"
to = \"mix.in1\"
[[link]]
from = \"b\"
to = \"mix.in2\"
[[link]]
from = \"mix\"
to = \"out.in\"
"),
    },
];

pub fn stage_graph(toml: &str) -> Graph {
    Graph::build(&Scene::parse(toml).expect("scene parses")).expect("graph builds")
}

/// The native deck's engine: the scene rendered in `block`-frame blocks (the control half steps inside, once per 128-frame
/// quantum – #319), `frames` frames; the left channel after master and limiter.
pub fn stage_render(toml: &str, block: usize, frames: usize, build: Build) -> Vec<f32> {
    stage_render_at(toml, block, frames, build, None)
}

/// `stage_render` with the control half stepped at `hz` instead (`Engine::control_at`: the native deck's video frame rate
/// before #319 – the P5 golden diff's reference)
pub fn stage_render_at(toml: &str, block: usize, frames: usize, build: Build, hz: Option<f32>) -> Vec<f32> {
    let scene = deck_native::modules::carried_part(&Scene::parse(toml).expect("scene parses"));
    let graph = Graph::build(&scene).expect("graph builds");
    let (mut engine, _handles) = Engine::build_with(&graph, RATE, 1.0, build).expect("engine builds");
    if let Some(hz) = hz {
        engine.control_at(hz);
    }
    let mut out = Vec::with_capacity(frames);
    let mut buf = vec![0.0f32; block * 2];
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        engine.render(&mut buf[..n * 2], 2, |x| x);
        out.extend(buf[..n * 2].chunks_exact(2).map(|f| f[0]));
        done += n;
    }
    out
}

/// #322: the native deck's path for a rig – the scene `--scene rig:<subject>@<mood>` loads (deck-native rig.rs: deck-space's
/// `Deck` plays the core rig), checked as the stage checks it (`modules::check`), its graph on deck-engine with the hosts in
/// the block, `frames` frames in blocks of `block`: both channels of a stereo device, interleaved.
pub fn native_rig_render(subject: &str, mood: &str, block: usize, frames: usize) -> Vec<f32> {
    let scene = deck_native::rig::scene(std::path::Path::new(&format!("rig:{subject}@{mood}"))).expect("the rig's scene");
    deck_native::modules::check(&scene).expect("the stage plays the rig's scene");
    let graph = Graph::build(&scene).expect("graph builds");
    let (mut engine, _handles) = Engine::build_with(&graph, RATE, 1.0, Build::detect()).expect("engine builds");
    let mut out = Vec::with_capacity(frames * 2);
    let mut buf = vec![0.0f32; block * 2];
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        engine.render(&mut buf[..n * 2], 2, |x| x);
        out.extend_from_slice(&buf[..n * 2]);
        done += n;
    }
    out
}

/// #322: the browser deck's side of the same rig and mood – the scene deck-space's `Deck` hands live-core (space-web
/// `DeckState.play_core`, the headless host's `load rig`)
pub fn browser_rig_scene(subject: &str, mood: &str) -> String {
    let mut d = deck_space::deck::Deck::new();
    d.ladder.pin(Some(mood));
    let t = d.play_core(subject, "");
    assert!(t.rig && !t.failed, "{subject}: {}", t.note);
    assert!(t.played.engine.page.is_empty(), "{subject}: the browser carries every kind of it");
    t.played.engine.scene
}

/// the core rigs and the moods the rig conformance plays (#322)
pub const RIGS: [&str; 2] = ["catpope", "item.incense_burner"];
pub const RIG_MOODS: [&str; 4] = ["neutral", "happy", "laughing", "sad"];

/// What `lower()` made of a scene (P6, #320: a split – the kinds the browser carries stay as they are).
pub struct Lowered {
    pub engine_toml: String,
    /// the nodes handed to the page (not played by the engine)
    pub page: Vec<String>,
}

pub fn lower(toml: &str) -> Lowered {
    let scene = Scene::parse(toml).expect("scene parses");
    let e = deck_space::rig::engine::lower(&scene).expect("lowers");
    Lowered { engine_toml: e.scene, page: e.page.iter().map(|n| format!("{} ({})", n.id, n.kind)).collect() }
}

/// The browser deck: live-core (deck-engine behind the worker's exports) on a device of `channels` channels, `frames`
/// frames in blocks of `block`; every channel, interleaved.
pub fn core_render_device(engine_toml: &str, channels: usize, block: usize, frames: usize) -> Vec<f32> {
    let mut core = live_core::Core::load(engine_toml, RATE, channels).unwrap_or_else(|e| panic!("live-core loads: {e}\n{engine_toml}"));
    let mut out = Vec::with_capacity(frames * channels);
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        out.extend_from_slice(core.render(n));
        done += n;
    }
    out
}

/// `core_render_device` on a stereo device, the left channel (what `stage_render` returns)
pub fn core_render(engine_toml: &str, block: usize, frames: usize) -> Vec<f32> {
    core_render_device(engine_toml, 2, block, frames).chunks_exact(2).map(|f| f[0]).collect()
}

#[derive(Debug, Clone, Copy)]
pub struct Diff {
    pub max_abs: f32,
    pub rms: f32,
    pub first: Option<usize>,
    pub exact: bool,
}

pub fn diff(a: &[f32], b: &[f32]) -> Diff {
    assert_eq!(a.len(), b.len());
    let mut max_abs = 0.0f32;
    let mut sum = 0.0f64;
    let mut first = None;
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        let d = (x - y).abs();
        if x.to_bits() != y.to_bits() && first.is_none() {
            first = Some(i);
        }
        max_abs = max_abs.max(d);
        sum += f64::from(d) * f64::from(d);
    }
    Diff { max_abs, rms: (sum / a.len().max(1) as f64).sqrt() as f32, first, exact: first.is_none() }
}

/// FNV-1a 64 over the f32 bit patterns
pub fn hash(x: &[f32]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for v in x {
        for b in v.to_bits().to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

pub fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0, |p, v| p.max(v.abs()))
}

pub fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| f64::from(*v) * f64::from(*v)).sum::<f64>() / x.len().max(1) as f64).sqrt() as f32
}

/// `deck/target/conformance/<name>`: where the probes leave their tables
pub fn report_path(name: &str) -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("target").join("conformance");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

pub fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}

/// osc 220 · fm 330 (2, 1.5) · osc 440 · fm 110 (3, 2) → a mixer (unity gains) → audio_out → master → limiter: a stereo mix
/// of four voices (the P0 base scene)
pub const STEREO_TOML: &str = r#"
[[node]]
id = "a"
kind = "osc"
freq = 220.0
gain = 0.2
[[node]]
id = "b"
kind = "fm"
freq = 330.0
ratio = 2.0
index = 1.5
gain = 0.2
[[node]]
id = "c"
kind = "osc"
freq = 440.0
gain = 0.15
[[node]]
id = "d"
kind = "fm"
freq = 110.0
ratio = 3.0
index = 2.0
gain = 0.2
[[node]]
id = "mix"
kind = "mixer"
gain1 = 1.0
gain2 = 1.0
gain3 = 1.0
gain4 = 1.0
[[node]]
id = "out"
kind = "audio_out"
volume = 1.0
[[link]]
from = "a"
to = "mix.in1"
[[link]]
from = "b"
to = "mix.in2"
[[link]]
from = "c"
to = "mix.in3"
[[link]]
from = "d"
to = "mix.in4"
[[link]]
from = "mix"
to = "out.in"
"#;


pub const _OUT: &str = OUT;

pub mod routing;
