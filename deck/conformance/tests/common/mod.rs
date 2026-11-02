//! Shared by the probes: the kinds' scenes, the two renderers (the native deck's engine, live-core after `lower()`),
//! the comparison numbers and the hash. Everything renders 2 s at 48 kHz.
#![allow(dead_code)]
use deck_stage::audio::{Build, Engine};
use deck_stage::control::Controls;
use deck_stage::graph::Graph;
use deck_stage::scene::Scene;
use deck_stage::tempo::Now;

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

pub fn stage_graph(toml: &str) -> Graph {
    Graph::build(&Scene::parse(toml).expect("scene parses")).expect("graph builds")
}

/// The native deck's engine: the scene rendered in `block`-frame blocks, the control domain stepped before each block
/// (the frame thread's `Controls::update`), `frames` frames; the left channel after master and limiter.
pub fn stage_render(toml: &str, block: usize, frames: usize, build: Build) -> Vec<f32> {
    let graph = stage_graph(toml);
    let (mut engine, mut handles) = Engine::build_with(&graph, RATE, 1.0, build).expect("engine builds");
    let mut controls = Controls::new(&graph, Some(&mut handles));
    let now = Now::default();
    let mut out = Vec::with_capacity(frames);
    let mut buf = vec![0.0f32; block * 2];
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        controls.update(n as f32 / RATE, 0, &now);
        engine.render(&mut buf[..n * 2], 2, |x| x);
        out.extend(buf[..n * 2].chunks_exact(2).map(|f| f[0]));
        done += n;
    }
    out
}

/// What `lower()` made of a scene.
pub struct Lowered {
    pub engine_toml: String,
    /// the nodes handed to the page (not played by the engine)
    pub page: Vec<String>,
    /// per original node: the params the lowering dropped
    pub dropped_params: Vec<String>,
    /// links the engine does not carry (mixers, audio_out and others)
    pub dropped_links: usize,
}

pub fn lower(toml: &str) -> Lowered {
    let scene = Scene::parse(toml).expect("scene parses");
    let e = deck_space::rig::engine::lower(&scene).expect("lowers");
    let doc: toml::Table = toml::from_str(&e.scene).expect("the lowered scene is TOML");
    let lowered_nodes: Vec<&toml::Table> = doc.get("node").and_then(toml::Value::as_array).map(|a| a.iter().filter_map(toml::Value::as_table).collect()).unwrap_or_default();
    let mut dropped = Vec::new();
    for n in &scene.nodes {
        if e.page.iter().any(|p| p.node.id == n.id) {
            continue;
        }
        let keep = lowered_nodes.iter().find(|t| t.get("id").and_then(toml::Value::as_str) == Some(n.id.as_str()));
        for key in n.params.keys() {
            if key == "volume" {
                continue;
            }
            if let Some(t) = keep {
                if !t.contains_key(key) {
                    dropped.push(format!("{}.{key}", n.id));
                }
            }
        }
    }
    Lowered { engine_toml: e.scene, page: e.page.iter().map(|p| format!("{} ({})", p.node.id, p.node.kind)).collect(), dropped_params: dropped, dropped_links: e.dropped.len() }
}

/// live-core after `lower()`: mono master through the limiter, the control domain stepped before each block.
pub fn core_render(engine_toml: &str, block: usize, frames: usize) -> Vec<f32> {
    let mut core = live_core::Core::load(engine_toml, RATE).unwrap_or_else(|e| panic!("live-core loads: {e}\n{engine_toml}"));
    let mut out = Vec::with_capacity(frames);
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(block);
        core.step(n as f32 / RATE);
        out.extend_from_slice(core.render(n));
        done += n;
    }
    out
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

pub const _OUT: &str = OUT;
