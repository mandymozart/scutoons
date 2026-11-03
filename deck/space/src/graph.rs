//! The node graph core (CYBERDECKS.md §8, ossia score's model): nodes with typed ports – video (a GPU texture), audio
//! (a stereo block), control (one f32 per frame) – checked and ordered once at load. Each kind runs in one domain:
//! video nodes on the render thread per frame, audio nodes in the realtime callback per block, control nodes on the
//! frame thread. Control links into audio nodes cross threads as atomics (audio::Param); the FFT is an audio node with
//! control outputs (its samples cross through a lock-free ring).
use crate::modreg::{self, ModuleKind};
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
    /// an audio port's channel count from the catalog (`any` = what the links carry, plan 2.6); None = not declared (a
    /// module's ports: the stereo pair of ABI v1)
    pub channels: Option<deck_catalog::Channels>,
}

/// Every node kind the stage knows, built or not – a kind's implementation lives in its module (a cargo feature, #185:
/// modules.rs, deck-core's stage.rs table). A new kind = a variant here + its name + its ports + its block
/// (render.rs / audio / control.rs) + its line in that table.
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
    /// a 3D space (space/, #188): a set's glTF and the deck's being – the running graph as a floating body – seen by an
    /// orbit camera; its inputs colour the being (hue, glow), light the set and kick its breath
    Space,
    /// a Live outlet (live/, #186): this deck's outlet's cells from the relay, drawn natively – the flat layers (slices,
    /// panels, pictures, text, colour) as one plane over `in` (the backdrop: a space node, where a cell has a 3D set),
    /// cross-faded on each cell change; `opacity` dims it
    Outlet,
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
    /// a generated melody (#236, deck-events): a seeded random walk on a continuous pitch scale, on the tempo clock,
    /// written as seq's cv (pitch left, gate right) – `spread` 0 = the base held, 1 = ±`octaves`; `rate` = steps a
    /// beat; `glide` rides along for the voice. The same seed plays the same melody on every deck.
    Walk,
    /// generators rigged together (#236, deck-events' patch): its params hold the patch (`generator`, `edge`, `out`,
    /// `knobs`) – an arp on a walk, a pulse … –, its inputs a–d are the patch's knobs (unset = the patch's own values),
    /// its notes are written as seq's cv
    Events,
    /// a chapter's music track from the library API (cached, decoded with symphonia)
    ChapterMusic,
    /// four stereo inputs, a gain each
    Mixer,
    /// sound → control values: level, bass, mid, high (0–1)
    Fft,
    /// the device output – the audio sink
    AudioOut,
    /// a bus (plan 2.6): sums its inputs, a fader, mute / solo; sends are links out of `pre` / `out`, a folder bus sums its children
    Bus,
    /// an outlet: writes its input to an outlet group's / the device's channels
    Out,
    /// a panner: a mono source onto a speaker layout (equal power, VBAP, DBAP)
    Pan,
    /// a gain matrix between channel layouts (ITU-R BS.775 / Web Audio tables, or a custom matrix)
    Downmix,
    /// the explicit loop: `in` one block late
    Feedback,
    /// a compiled module from the library (#280, modreg.rs): its ports come from its descriptor, not from this table;
    /// its scene name is `module:<name>`. Not in `ALL` – those are the kinds graph.rs defines.
    Module(ModuleKind),
}

impl Kind {
    /// every kind with its name in a scene
    const NAMES: [(&'static str, Kind); 25] = [
        ("test_pattern", Kind::TestPattern),
        ("shader", Kind::Shader),
        ("screen", Kind::Screen),
        ("chapter", Kind::Chapter),
        ("overlay", Kind::Overlay),
        ("space", Kind::Space),
        ("outlet", Kind::Outlet),
        ("lfo", Kind::Lfo),
        ("tempo", Kind::Tempo),
        ("click", Kind::Click),
        ("osc", Kind::Osc),
        ("noise", Kind::Noise),
        ("fm", Kind::Fm),
        ("seq", Kind::Seq),
        ("walk", Kind::Walk),
        ("events", Kind::Events),
        ("chapter_music", Kind::ChapterMusic),
        ("mixer", Kind::Mixer),
        ("fft", Kind::Fft),
        ("audio_out", Kind::AudioOut),
        ("bus", Kind::Bus),
        ("out", Kind::Out),
        ("pan", Kind::Pan),
        ("downmix", Kind::Downmix),
        ("feedback", Kind::Feedback),
    ];
    pub const ALL: [Kind; 25] = {
        let mut all = [Kind::Screen; 25];
        let mut i = 0;
        while i < 25 {
            all[i] = Kind::NAMES[i].1;
            i += 1;
        }
        all
    };

    pub fn parse(s: &str) -> Option<Kind> {
        match Kind::NAMES.iter().find(|(n, _)| *n == s) {
            Some(&(_, k)) => Some(k),
            None if s.starts_with(modreg::PREFIX) => modreg::lookup(s).map(Kind::Module),
            None => None,
        }
    }

    /// the kind's name in a scene
    pub fn name(self) -> &'static str {
        if let Kind::Module(m) = self {
            return m.spec().kind;
        }
        Kind::NAMES.iter().find(|&&(_, k)| k == self).map(|&(n, _)| n).expect("every kind has a name")
    }

    /// the kind's catalog entry (deck-catalog, #315): its domain, ports and parameters are the catalog's
    pub fn descriptor(self) -> Option<&'static deck_catalog::Descriptor> {
        match self {
            Kind::Module(_) => None,
            _ => deck_catalog::builtin(self.name()),
        }
    }

    pub fn domain(self) -> Domain {
        match self.descriptor().map(|d| d.domain) {
            Some(deck_catalog::Domain::Video) => Domain::Video,
            Some(deck_catalog::Domain::Control) => Domain::Control,
            _ => Domain::Audio,
        }
    }

    /// both port lists of every kind, built once from the catalog (indexed like `NAMES`)
    fn port_lists() -> &'static [(Vec<Port>, Vec<Port>)] {
        static LISTS: std::sync::OnceLock<Vec<(Vec<Port>, Vec<Port>)>> = std::sync::OnceLock::new();
        LISTS.get_or_init(|| {
            use deck_catalog::{static_str, Dir, PortType as T};
            let list = |d: &'static deck_catalog::Descriptor, dir: Dir| -> Vec<Port> {
                d.ports
                    .iter()
                    .filter(|p| p.dir == dir)
                    .map(|p| Port {
                        name: static_str(&p.name),
                        channels: p.channels,
                        ty: match p.ty {
                            T::Audio | T::Cv => PortType::Audio,
                            T::Control | T::Event => PortType::Control,
                            T::Video => PortType::Video,
                        },
                    })
                    .collect()
            };
            Kind::NAMES
                .iter()
                .map(|(name, _)| {
                    let d = deck_catalog::builtin(name).unwrap_or_else(|| panic!("kind '{name}' is not in the catalog (deck-catalog)"));
                    (list(d, Dir::In), list(d, Dir::Out))
                })
                .collect()
        })
    }

    fn index(self) -> usize {
        Kind::NAMES.iter().position(|&(_, k)| k == self).expect("every built-in kind has a name")
    }

    pub fn inputs(self) -> &'static [Port] {
        match self {
            Kind::Module(m) => m.spec().inputs,
            _ => &Kind::port_lists()[self.index()].0,
        }
    }

    pub fn outputs(self) -> &'static [Port] {
        match self {
            Kind::Module(m) => m.spec().outputs,
            _ => &Kind::port_lists()[self.index()].1,
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

    /// a list of strings (`hide = ["SM_Weihrauch"]`); anything else = empty
    pub fn strs(&self, key: &str) -> Vec<String> {
        let Some(toml::Value::Array(a)) = self.params.get(key) else { return Vec::new() };
        a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect()
    }

    pub fn f32s(&self, key: &str) -> Vec<f32> {
        let Some(toml::Value::Array(a)) = self.params.get(key) else { return Vec::new() };
        a.iter()
            .filter_map(|v| v.as_float().map(|f| f as f32).or_else(|| v.as_integer().map(|i| i as f32)))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub ty: PortType,
    pub scale: f32,
    pub offset: f32,
    /// an audio link's channel map as 0-based (source, target) pairs; empty = channel n to channel n (the scene's
    /// `map`, counted from 1)
    pub map: Vec<(u8, u8)>,
}

/// the most channels a port carries (plan 2.6)
pub const MAX_CHANNELS: usize = 32;

/// A checked graph: every link joins an output to an input of the same type, a control input has at most one link, an
/// audio input sums its links (in the scene's order), no cycles but through a `feedback` node; `order` is a
/// topological order (sources first).
#[derive(Debug, Clone)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
    pub order: Vec<usize>,
}

impl Graph {
    /// The scene's graph, checked as a graph. Whether a build carries the modules its kinds need is deck-native's check
    /// (its modules.rs, before it builds a scene); the space draws any kind.
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
        for sink in [Kind::Screen, Kind::AudioOut, Kind::Tempo, Kind::Space, Kind::Outlet] {
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
            // several audio links into one input are summed (a bus, a mixer's channel); a control input takes one
            if fty != PortType::Audio && links.iter().any(|x: &Link| x.to == (ti, tp)) {
                bail!("link {} → {}: the input already has a link", l.from, l.to);
            }
            if !l.map.is_empty() && fty != PortType::Audio {
                bail!("link {} → {}: only an audio link has a channel map", l.from, l.to);
            }
            let mut map = Vec::with_capacity(l.map.len());
            for &[s, t] in &l.map {
                if s == 0 || t == 0 || usize::from(s) > MAX_CHANNELS || usize::from(t) > MAX_CHANNELS {
                    bail!("link {} → {}: the channel map names channels 1..={MAX_CHANNELS}, not {s} → {t}", l.from, l.to);
                }
                map.push((s - 1, t - 1));
            }
            links.push(Link { from: (fi, fp), to: (ti, tp), ty: fty, scale: l.scale, offset: l.offset, map });
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
        // a link into a `feedback` node is read one block late: it orders nothing, so it closes a loop
        if self.ty == PortType::Audio && nodes[self.to.0].kind == Kind::Feedback {
            return false;
        }
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
        let Some(next) = (0..n).find(|&i| !done[i] && indeg[i] == 0) else {
            // name the loop: from a node that waits, walk back along waiting predecessors until one repeats
            let mut walk = vec![(0..n).find(|&i| !done[i]).expect("a node is left")];
            let ring = loop {
                let cur = *walk.last().expect("not empty");
                let before = links.iter().find(|l| l.to.0 == cur && !done[l.from.0]).map(|l| l.from.0).expect("a waiting node has a waiting predecessor");
                if let Some(at) = walk.iter().position(|&w| w == before) {
                    break walk[at..].to_vec();
                }
                walk.push(before);
            };
            let ids: Vec<&str> = ring.iter().rev().map(|&i| nodes[i].id.as_str()).collect();
            bail!("the graph has a cycle: {} → {} (a loop on purpose goes through a `feedback` node)", ids.join(" → "), ids[0]);
        };
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

    /// #252: the default scene idles – a picture, no sound
    #[test]
    fn the_idle_scene_has_no_audio() {
        let g = Graph::build(&Scene::parse(crate::scene::IDLE_SCENE).unwrap()).unwrap();
        assert!(g.nodes.iter().all(|n| n.kind.domain() != Domain::Audio));
        assert!(g.nodes.iter().any(|n| n.str("effect") == Some("fireflies")));
    }

    #[test]
    fn the_madness_scene_builds_in_order() {
        let g = Graph::build(&Scene::parse(crate::scene::MADNESS_SCENE).unwrap()).unwrap();
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
