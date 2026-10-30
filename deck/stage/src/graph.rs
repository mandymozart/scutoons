//! The node graph core (CYBERDECKS.md §8, ossia score's model): nodes with typed ports – video (a GPU texture), audio
//! (a stereo block), control (one f32 per frame) – checked and ordered once at load. Each kind runs in one domain:
//! video nodes on the render thread per frame, audio nodes in the realtime callback per block, control nodes on the
//! frame thread. Control links into audio nodes cross threads as atomics (audio::Param); the FFT is an audio node with
//! control outputs (its samples cross through a lock-free ring).
use crate::scene::{split_ref, Scene};
use anyhow::{bail, Result};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortType {
    Video,
    Audio,
    Control,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Video,
    Audio,
    Control,
}

#[derive(Debug, Clone, Copy)]
pub struct Port {
    pub name: &'static str,
    pub ty: PortType,
}

/// a port literal (a struct literal, so the port tables below are promoted to statics)
macro_rules! p {
    ($name:literal, $ty:ident) => {
        Port { name: $name, ty: $ty }
    };
}
use PortType::{Audio as A, Control as C, Video as V};

/// Every node kind the stage knows. A new kind = a variant here + its ports + its block (render.rs / audio.rs / control.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// procedural colour bars, rings and a grid – the S1 picture
    TestPattern,
    /// a fragment shader over its input (a built-in effect or a WGSL file)
    Shader,
    /// the window – the video sink
    Screen,
    /// a chapter's slices from the library API, one after the other (its dialog set in), over its `in` (or a dark,
    /// stretched copy of the slice); `text` = the current slice's caption on transparent, for an overlay
    Chapter,
    /// `top` over `in` (alpha), the top popping in when it changes, shaken by `shake`
    Overlay,
    /// a 3D space (space/, #188): a glTF room and the deck's being – the running graph as a floating body – seen by an
    /// orbit camera; its inputs colour the being (hue, glow), light the room and kick its breath
    Space,
    /// a low-frequency oscillator (control)
    Lfo,
    /// the tempo clock (tempo/, #181): bpm, beat, bar, phase, bar_phase, pulse – one per scene, its params set the
    /// clock (bpm, beats per bar, the source, the click, MIDI / OSC ports)
    Tempo,
    /// a metronome on the tempo clock: a short tick per beat, higher on the bar's first (while the click is on)
    Click,
    /// a band-limited oscillator (sine / saw / square / triangle), pitched by its `cv` input or `freq`
    Osc,
    /// white or pink noise through a lowpass
    Noise,
    /// two-operator FM
    Fm,
    /// a step sequencer: `cv` = pitch in Hz on the left channel, gate 0 / 1 on the right (modular CV in an audio port)
    Seq,
    /// a chapter's music track from the library API (cached, decoded with symphonia)
    ChapterMusic,
    /// four stereo inputs, a gain each
    Mixer,
    /// sound → control values: level, bass, mid, high (0–1)
    Fft,
    /// the device output – the audio sink
    AudioOut,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "test_pattern" => Kind::TestPattern,
            "shader" => Kind::Shader,
            "screen" => Kind::Screen,
            "chapter" => Kind::Chapter,
            "overlay" => Kind::Overlay,
            "space" => Kind::Space,
            "lfo" => Kind::Lfo,
            "tempo" => Kind::Tempo,
            "click" => Kind::Click,
            "osc" => Kind::Osc,
            "noise" => Kind::Noise,
            "fm" => Kind::Fm,
            "seq" => Kind::Seq,
            "chapter_music" => Kind::ChapterMusic,
            "mixer" => Kind::Mixer,
            "fft" => Kind::Fft,
            "audio_out" => Kind::AudioOut,
            _ => return None,
        })
    }

    pub fn domain(self) -> Domain {
        match self {
            Kind::TestPattern | Kind::Shader | Kind::Screen | Kind::Chapter | Kind::Overlay | Kind::Space => Domain::Video,
            Kind::Lfo | Kind::Tempo => Domain::Control,
            _ => Domain::Audio,
        }
    }

    pub fn inputs(self) -> &'static [Port] {
        match self {
            Kind::TestPattern => &[p!("speed", C), p!("zoom", C), p!("hue", C), p!("pulse", C)],
            Kind::Shader => &[p!("in", V), p!("a", C), p!("b", C), p!("c", C), p!("d", C)],
            Kind::Screen => &[p!("in", V)],
            Kind::Chapter => &[p!("in", V), p!("next", C), p!("hold", C), p!("zoom", C), p!("pulse", C)],
            Kind::Overlay => &[p!("in", V), p!("top", V), p!("opacity", C), p!("pulse", C), p!("shake", C)],
            Kind::Space => &[p!("hue", C), p!("glow", C), p!("light", C), p!("pulse", C)],
            Kind::Lfo => &[p!("rate", C), p!("depth", C)],
            Kind::Tempo => &[],
            Kind::Click => &[p!("gain", C)],
            Kind::Osc => &[p!("cv", A), p!("freq", C), p!("gain", C)],
            Kind::Noise => &[p!("cutoff", C), p!("gain", C)],
            Kind::Fm => &[p!("cv", A), p!("freq", C), p!("ratio", C), p!("index", C), p!("gain", C)],
            Kind::Seq => &[p!("rate", C)],
            Kind::ChapterMusic => &[p!("gain", C)],
            Kind::Mixer => &[
                p!("in1", A), p!("in2", A), p!("in3", A), p!("in4", A),
                p!("gain1", C), p!("gain2", C), p!("gain3", C), p!("gain4", C),
            ],
            Kind::Fft => &[p!("in", A)],
            Kind::AudioOut => &[p!("in", A), p!("volume", C)],
        }
    }

    pub fn outputs(self) -> &'static [Port] {
        match self {
            Kind::TestPattern | Kind::Shader | Kind::Overlay | Kind::Space => &[p!("out", V)],
            Kind::Chapter => &[p!("out", V), p!("text", V)],
            Kind::Screen | Kind::AudioOut => &[],
            Kind::Lfo => &[p!("out", C)],
            Kind::Tempo => &[p!("bpm", C), p!("beat", C), p!("bar", C), p!("phase", C), p!("bar_phase", C), p!("pulse", C)],
            Kind::Seq => &[p!("cv", A)],
            Kind::Fft => &[p!("level", C), p!("bass", C), p!("mid", C), p!("high", C)],
            _ => &[p!("out", A)],
        }
    }

    pub fn input(self, name: &str) -> Option<usize> {
        self.inputs().iter().position(|p| p.name == name)
    }

    pub fn output(self, name: &str) -> Option<usize> {
        self.outputs().iter().position(|p| p.name == name)
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub kind: Kind,
    pub params: toml::Table,
}

impl Node {
    pub fn f32(&self, key: &str, default: f32) -> f32 {
        match self.params.get(key) {
            Some(toml::Value::Float(f)) => *f as f32,
            Some(toml::Value::Integer(i)) => *i as f32,
            _ => default,
        }
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        self.params.get(key).and_then(|v| v.as_str())
    }

    pub fn bool(&self, key: &str, default: bool) -> bool {
        self.params.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    pub fn f32s(&self, key: &str) -> Vec<f32> {
        let Some(toml::Value::Array(a)) = self.params.get(key) else { return Vec::new() };
        a.iter()
            .filter_map(|v| v.as_float().map(|f| f as f32).or_else(|| v.as_integer().map(|i| i as f32)))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Link {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub ty: PortType,
    pub scale: f32,
    pub offset: f32,
}

/// A checked graph: every link joins an output to an input of the same type, an input has at most one link, no cycles;
/// `order` is a topological order (sources first).
#[derive(Debug, Clone)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
    pub order: Vec<usize>,
}

impl Graph {
    pub fn build(scene: &Scene) -> Result<Graph> {
        let mut nodes = Vec::new();
        let mut by_id = HashMap::new();
        for n in &scene.nodes {
            let Some(kind) = Kind::parse(&n.kind) else { bail!("node '{}': unknown kind '{}'", n.id, n.kind) };
            if by_id.insert(n.id.clone(), nodes.len()).is_some() {
                bail!("node id '{}' is used twice", n.id);
            }
            nodes.push(Node { id: n.id.clone(), kind, params: n.params.clone() });
        }
        for sink in [Kind::Screen, Kind::AudioOut, Kind::Tempo, Kind::Space] {
            if nodes.iter().filter(|n| n.kind == sink).count() > 1 {
                bail!("a scene has at most one {sink:?} node");
            }
        }
        let mut links = Vec::new();
        for l in &scene.links {
            let (fnode, fport) = split_ref(&l.from);
            let (tnode, tport) = split_ref(&l.to);
            let Some(&fi) = by_id.get(fnode) else { bail!("link {} → {}: no node '{fnode}'", l.from, l.to) };
            let Some(&ti) = by_id.get(tnode) else { bail!("link {} → {}: no node '{tnode}'", l.from, l.to) };
            let Some(fp) = nodes[fi].kind.output(fport) else {
                bail!("link {} → {}: {:?} has no output '{fport}'", l.from, l.to, nodes[fi].kind)
            };
            let Some(tp) = nodes[ti].kind.input(tport) else {
                bail!("link {} → {}: {:?} has no input '{tport}'", l.from, l.to, nodes[ti].kind)
            };
            let (fty, tty) = (nodes[fi].kind.outputs()[fp].ty, nodes[ti].kind.inputs()[tp].ty);
            if fty != tty {
                bail!("link {} → {}: a {fty:?} output cannot feed a {tty:?} input", l.from, l.to);
            }
            if links.iter().any(|x: &Link| x.to == (ti, tp)) {
                bail!("link {} → {}: the input already has a link", l.from, l.to);
            }
            links.push(Link { from: (fi, fp), to: (ti, tp), ty: fty, scale: l.scale, offset: l.offset });
        }
        let order = topo_order(nodes.len(), &links, &nodes)?;
        Ok(Graph { nodes, links, order })
    }

    /// the link into `node`'s input `port`, if any
    pub fn link_into(&self, node: usize, port: usize) -> Option<&Link> {
        self.links.iter().find(|l| l.to == (node, port))
    }

    pub fn first(&self, kind: Kind) -> Option<usize> {
        self.nodes.iter().position(|n| n.kind == kind)
    }
}

impl Link {
    /// Whether the link orders its two nodes. A control link out of an audio node (the FFT) or into one (a synth's
    /// param) crosses threads a frame late – the analyzer runs before the frame's control nodes, params are written
    /// after them – so it never orders, and feedback through it (FFT → synth → mix → FFT) is allowed, like a patch cable.
    pub fn orders(&self, nodes: &[Node]) -> bool {
        self.ty != PortType::Control
            || (nodes[self.from.0].kind.domain() != Domain::Audio && nodes[self.to.0].kind.domain() != Domain::Audio)
    }
}

/// Kahn's algorithm over the ordering links, stable by declaration order (the scene's order wins among independent nodes)
fn topo_order(n: usize, links: &[Link], nodes: &[Node]) -> Result<Vec<usize>> {
    let links: Vec<&Link> = links.iter().filter(|l| l.orders(nodes)).collect();
    let mut indeg = vec![0usize; n];
    for l in &links {
        indeg[l.to.0] += 1;
    }
    let mut order = Vec::with_capacity(n);
    let mut done = vec![false; n];
    while order.len() < n {
        let Some(next) = (0..n).find(|&i| !done[i] && indeg[i] == 0) else { bail!("the graph has a cycle") };
        done[next] = true;
        order.push(next);
        for l in links.iter().filter(|l| l.from.0 == next) {
            indeg[l.to.0] -= 1;
        }
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(text: &str) -> Result<Graph> {
        Graph::build(&Scene::parse(text)?)
    }

    #[test]
    fn the_default_scene_builds_in_order() {
        let g = Graph::build(&Scene::parse(crate::scene::DEFAULT_SCENE).unwrap()).unwrap();
        assert!(g.links.iter().any(|l| !l.orders(&g.nodes)), "the scene's FFT feedback");
        for l in g.links.iter().filter(|l| l.orders(&g.nodes)) {
            let pos = |i| g.order.iter().position(|&x| x == i).unwrap();
            assert!(pos(l.from.0) < pos(l.to.0), "{l:?}");
        }
    }

    #[test]
    fn type_mismatch_is_an_error() {
        let e = graph(
            "[[node]]\nid=\"o\"\nkind=\"osc\"\n[[node]]\nid=\"s\"\nkind=\"screen\"\n[[link]]\nfrom=\"o\"\nto=\"s.in\"\n",
        )
        .unwrap_err();
        assert!(e.to_string().contains("cannot feed"), "{e}");
    }

    #[test]
    fn cycles_are_errors() {
        let e = graph(
            "[[node]]\nid=\"a\"\nkind=\"lfo\"\n[[node]]\nid=\"b\"\nkind=\"lfo\"\n\
             [[link]]\nfrom=\"a\"\nto=\"b.rate\"\n[[link]]\nfrom=\"b\"\nto=\"a.rate\"\n",
        )
        .unwrap_err();
        assert!(e.to_string().contains("cycle"), "{e}");
    }

    #[test]
    fn unknown_ports_and_double_links_are_errors() {
        let two = "[[node]]\nid=\"a\"\nkind=\"lfo\"\n[[node]]\nid=\"b\"\nkind=\"lfo\"\n";
        assert!(graph(&format!("{two}[[link]]\nfrom=\"a.nope\"\nto=\"b.rate\"\n")).is_err());
        assert!(graph(&format!("{two}[[link]]\nfrom=\"a\"\nto=\"b.rate\"\n[[link]]\nfrom=\"a\"\nto=\"b.rate\"\n")).is_err());
        assert!(graph("[[node]]\nid=\"a\"\nkind=\"blender\"\n").is_err());
    }

    #[test]
    fn params_read_ints_as_floats() {
        let g = graph("[[node]]\nid=\"s\"\nkind=\"seq\"\nbpm=120\nsteps=[60, 0, 62.5]\n").unwrap();
        assert_eq!(g.nodes[0].f32("bpm", 0.0), 120.0);
        assert_eq!(g.nodes[0].f32s("steps"), vec![60.0, 0.0, 62.5]);
    }
}
