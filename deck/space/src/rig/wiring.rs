//! The wiring of a played scene (#282, the rig designer's Nodes tab): its nodes with their kinds, the rig's track each
//! belongs to, their ports (the kind catalog's, a module's descriptor's) and their depth in the graph, and its links
//! resolved to ports (a bare `node` is its first output) with the path whose value flows on the cable. It is the checked
//! graph (`Graph::build`: a module's bound path outputs are links too) as the editor draws it – the editor only places
//! it; the live values on the cables are the engine's analysis frame's (live-core `value(path)`, the native deck's
//! value table), read by the paths named here.
use super::Played;
use crate::graph::{Domain, Graph, Kind, PortType};
use crate::problem::Problem;
use crate::scene::Scene;
use anyhow::{Context, Result};
use serde::Serialize;

/// The graph of what a deck plays, for drawing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Wiring {
    pub nodes: Vec<WireNode>,
    pub links: Vec<WireLink>,
    /// why there is no graph ("" = there is one), in one sentence
    #[serde(skip_serializing_if = "String::is_empty")]
    pub problem: String,
    /// each problem at its cable or node (#432): the ones behind `problem`, or on a graph that built, what is listed but
    /// not refused – a far world node that depends on another deck being in the session (#442)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub problems: Vec<Problem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WireNode {
    pub id: String,
    pub kind: String,
    /// the rig's track it belongs to ("" = the scene's own: the mixers summing the tracks, the outlet, the analyser,
    /// the being – or any node of a deck's own scene)
    pub track: String,
    pub domain: &'static str,
    /// 0 = fed by nothing before it; else one more than the deepest node feeding it (the links that go forward in the
    /// graph's order – a loop's way back adds none)
    pub depth: u32,
    pub inputs: Vec<WirePort>,
    pub outputs: Vec<WirePort>,
    /// the browser's engine plays it (false: a module the browser does not carry yet – no values on its cables there)
    pub played: bool,
    /// a world node's source and name filter (#430)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world: Option<WireWorld>,
}

impl WireNode {
    /// the address a rig's route names this node's `port` by (`voice:melody.cv`); None for the scene's own nodes (the
    /// mixers summing the tracks, the outlet, the analyser, the being): no route ends there
    pub fn address(&self, port: &str) -> Option<String> {
        let node = self.id.strip_prefix(self.track.as_str())?.strip_prefix('_')?;
        (!self.track.is_empty()).then(|| format!("{}:{node}.{port}", self.track))
    }
}

/// What a world node shows (#430): where its events come from, and the names it lets through.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WireWorld {
    /// `midi` · `generator` · `far`
    pub source: &'static str,
    pub listen: Vec<String>,
    /// a far source's address (#432)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WirePort {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WireLink {
    /// the nodes' indices in `nodes`, and the ports' names
    pub from: usize,
    pub out: String,
    pub to: usize,
    pub input: String,
    #[serde(rename = "type")]
    pub ty: &'static str,
    pub scale: f32,
    pub offset: f32,
    /// the path whose value flows on the cable – the output's `node.port` (an audio output's is its block's peak)
    pub value: String,
    /// a module's path output bound to the input (ABI-V2 §5), not one of the scene's links
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bound: bool,
}

fn port_type(t: PortType) -> &'static str {
    match t {
        PortType::Audio => "audio",
        PortType::Control => "control",
        PortType::Video => "video",
        PortType::Event => "event",
    }
}

fn domain(d: Domain) -> &'static str {
    match d {
        Domain::Audio => "audio",
        Domain::Control => "control",
        Domain::Video => "video",
    }
}

impl Wiring {
    /// no graph, and why: the one sentence, and each problem at its cable or node
    pub fn of_error(e: &anyhow::Error) -> Wiring {
        Wiring { nodes: Vec::new(), links: Vec::new(), problem: format!("{e:#}"), problems: crate::problem::listed(e) }
    }
}

/// Where a cable of the played scene is written (#433): the document a gesture on it would change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Home {
    /// this track's instrument's `link`
    Instrument { track: String },
    /// a preset's re-route (`link`) on this track – shown, marked, not edited here
    Preset { track: String, preset: String },
    /// the rig's `route` at this index
    Route { index: usize },
    /// flatten made it: the mixer sum, the default `fft.level → space.pulse`
    Compiled,
    /// a module's path output bound by `Graph::build` (a code rig's policy): the module's Rust names it
    Bound,
}

/// One level of the wiring (research/MODULE-WIRING.md §4): the rig (its tracks, policy and being as boxes), one track
/// (its instrument's nodes), one node (its ports). The context is an address prefix relative to the rig: "" · `<track>` ·
/// `<track>:<node>` (a node of the rig's own: `policy`, `fft`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Level {
    Rig,
    Track(String),
    /// the flattened node id
    Node(String),
}

/// What one level shows: boxes with the ports cables touch, the cables inside it with their home and whether a gesture
/// may change them, the cables crossing its edge as stubs named by the other end (one hop: its rig-level leaf), and the
/// levels above it (the rig first) so a GUI can step up.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct View {
    pub context: String,
    pub levels: Vec<String>,
    pub nodes: Vec<ViewNode>,
    pub links: Vec<ViewLink>,
    pub stubs: Vec<Stub>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ViewNode {
    /// a track's id, `policy`, `being`; at a track level the node's own id (without the track's prefix)
    pub id: String,
    /// `track` for a track's box, else the node's kind
    pub kind: String,
    pub inputs: Vec<WirePort>,
    pub outputs: Vec<WirePort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world: Option<WireWorld>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ViewLink {
    /// the cable's index in the played scene's links (a bound one counts on after them): what an edit names
    pub id: usize,
    pub from: usize,
    pub out: String,
    pub to: usize,
    pub input: String,
    #[serde(rename = "type")]
    pub ty: &'static str,
    pub scale: f32,
    pub offset: f32,
    /// the slot whose value flows on it (`WireLink::value`)
    pub value: String,
    pub home: Home,
    /// why a gesture may not change it (§4 rule 3): `compiled` · `bound` · `owned` · `preset` · `module`; none = editable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<&'static str>,
}

/// A cable crossing the level's edge: the port here, and the other end named by its rig-level leaf (`voice:melody.cv`,
/// `policy:heat`, `being:pulse`, `sum` for the compiled mix).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stub {
    pub id: usize,
    pub node: usize,
    pub port: String,
    /// the cable comes in here (else it leaves)
    pub input: bool,
    pub other: String,
    #[serde(rename = "type")]
    pub ty: &'static str,
    pub value: String,
    pub home: Home,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<&'static str>,
}

/// a flattened node's place at the rig level
#[derive(Debug, Clone, PartialEq, Eq)]
enum Group {
    Track(String),
    Policy,
    Being,
    /// the compiled sum: the mixers, `out`, `fft`
    Sum,
    /// a node of a scene that is no rig (a deck's own)
    Own(String),
}

fn port(ports: &mut Vec<WirePort>, name: &str, ty: &'static str) {
    if !ports.iter().any(|p| p.name == name) {
        ports.push(WirePort { name: name.to_string(), ty });
    }
}

impl Played {
    /// the level `context` names (see `Level`): a track's id, `<track>:<node>`, a node of the rig's own, "" = the rig
    fn level(&self, context: &str) -> Result<Level> {
        let context = context.trim();
        if context.is_empty() {
            return Ok(Level::Rig);
        }
        if let Some((track, node)) = context.split_once(':') {
            let id = format!("{track}_{node}");
            return if self.nodes.iter().any(|n| n.id == id) { Ok(Level::Node(id)) } else { anyhow::bail!("no node '{node}' in track '{track}'") };
        }
        if self.tracks.iter().any(|t| t.track == context) {
            return Ok(Level::Track(context.to_string()));
        }
        if self.nodes.iter().any(|n| n.id == context) {
            return Ok(Level::Node(context.to_string()));
        }
        anyhow::bail!("no track or node '{context}' (a level is \"\", <track> or <track>:<node>)")
    }

    /// One level of the wiring (#433, MODULE-WIRING §4): see `View`. Errors when the scene does not build as a graph
    /// or `context` names nothing.
    pub fn wire_view(&self, context: &str) -> Result<View> {
        let w = self.wiring()?;
        let level = self.level(context)?;
        let is_rig = !self.tracks.is_empty();
        let group = |n: &WireNode| -> Group {
            if !n.track.is_empty() {
                Group::Track(n.track.clone())
            } else if n.id == super::policy::NODE {
                Group::Policy
            } else if n.kind == "space" {
                Group::Being
            } else if is_rig && (n.kind == "mixer" || n.kind == "audio_out" || n.kind == "fft") {
                Group::Sum
            } else {
                Group::Own(n.id.clone())
            }
        };
        let local = |n: &WireNode| -> String { n.id.strip_prefix(&format!("{}_", n.track)).filter(|_| !n.track.is_empty()).unwrap_or(&n.id).to_string() };
        // the rig-level leaf of a node's port – a stub's name
        let leaf = |i: usize, p: &str| -> String {
            let n = &w.nodes[i];
            match group(n) {
                Group::Track(t) => format!("{t}:{}.{p}", local(n)),
                Group::Policy => format!("{}:{p}", super::policy::NODE),
                Group::Being => format!("{}:{p}", super::address::BEING),
                Group::Sum => "sum".to_string(),
                Group::Own(id) => format!("{id}.{p}"),
            }
        };
        let scene_links = self.links.len();
        let home = |k: usize| -> Home { if k < scene_links { self.homes.get(k).cloned().unwrap_or(Home::Compiled) } else { Home::Bound } };
        let readonly = |l: &WireLink, h: &Home| -> Option<&'static str> {
            if l.bound {
                Some("bound")
            } else if *h == Home::Compiled {
                Some("compiled")
            } else if matches!(h, Home::Preset { .. }) {
                Some("preset")
            } else if self.owned.contains(&format!("{}.{}", w.nodes[l.to].id, l.input)) {
                Some("owned")
            } else {
                None
            }
        };
        let mut view = View { context: context.trim().to_string(), levels: vec!["rig".to_string()], nodes: Vec::new(), links: Vec::new(), stubs: Vec::new() };
        match &level {
            Level::Rig => {
                // the boxes: the tracks in their order, the policy, the being, a plain scene's own nodes
                let mut boxes: Vec<(Group, ViewNode)> = self.tracks.iter().map(|t| (Group::Track(t.track.clone()), ViewNode { id: t.track.clone(), kind: "track".into(), inputs: Vec::new(), outputs: Vec::new(), world: None })).collect();
                for n in &w.nodes {
                    let g = group(n);
                    if g != Group::Sum && !boxes.iter().any(|(b, _)| *b == g) {
                        let id = match &g {
                            Group::Policy => super::policy::NODE.to_string(),
                            Group::Being => super::address::BEING.to_string(),
                            _ => n.id.clone(),
                        };
                        let own = matches!(g, Group::Own(_));
                        boxes.push((g, ViewNode { id, kind: n.kind.clone(), inputs: if own { n.inputs.clone() } else { Vec::new() }, outputs: if own { n.outputs.clone() } else { Vec::new() }, world: n.world.clone() }));
                    }
                }
                let groups: Vec<Group> = boxes.iter().map(|(g, _)| g.clone()).collect();
                let at = |i: usize| groups.iter().position(|g| *g == group(&w.nodes[i]));
                // a port's name on its box: `<node>.<port>` in a track, the port alone on the policy and the being
                let name = |i: usize, p: &str| -> String { if matches!(group(&w.nodes[i]), Group::Track(_)) { format!("{}.{p}", local(&w.nodes[i])) } else { p.to_string() } };
                for (k, l) in w.links.iter().enumerate() {
                    let (h, ro) = (home(k), readonly(l, &home(k)));
                    match (at(l.from), at(l.to)) {
                        (Some(a), Some(b)) if a != b => {
                            let (out, input) = (name(l.from, &l.out), name(l.to, &l.input));
                            port(&mut boxes[a].1.outputs, &out, l.ty);
                            port(&mut boxes[b].1.inputs, &input, l.ty);
                            view.links.push(ViewLink { id: k, from: a, out, to: b, input, ty: l.ty, scale: l.scale, offset: l.offset, value: l.value.clone(), home: h, readonly: ro });
                        }
                        (Some(a), None) => {
                            let out = name(l.from, &l.out);
                            port(&mut boxes[a].1.outputs, &out, l.ty);
                            view.stubs.push(Stub { id: k, node: a, port: out, input: false, other: leaf(l.to, &l.input), ty: l.ty, value: l.value.clone(), home: h, readonly: ro });
                        }
                        (None, Some(b)) => {
                            let input = name(l.to, &l.input);
                            port(&mut boxes[b].1.inputs, &input, l.ty);
                            view.stubs.push(Stub { id: k, node: b, port: input, input: true, other: leaf(l.from, &l.out), ty: l.ty, value: l.value.clone(), home: h, readonly: ro });
                        }
                        // inside one box, or inside the sum
                        _ => {}
                    }
                }
                view.nodes = boxes.into_iter().map(|(_, n)| n).collect();
            }
            Level::Track(t) => {
                view.levels.push(format!("track:{t}"));
                let inside: Vec<usize> = (0..w.nodes.len()).filter(|&i| group(&w.nodes[i]) == Group::Track(t.clone())).collect();
                view.nodes = inside.iter().map(|&i| ViewNode { id: local(&w.nodes[i]), kind: w.nodes[i].kind.clone(), inputs: w.nodes[i].inputs.clone(), outputs: w.nodes[i].outputs.clone(), world: w.nodes[i].world.clone() }).collect();
                let at = |i: usize| inside.iter().position(|&x| x == i);
                for (k, l) in w.links.iter().enumerate() {
                    let (h, ro) = (home(k), readonly(l, &home(k)));
                    match (at(l.from), at(l.to)) {
                        (Some(a), Some(b)) => view.links.push(ViewLink { id: k, from: a, out: l.out.clone(), to: b, input: l.input.clone(), ty: l.ty, scale: l.scale, offset: l.offset, value: l.value.clone(), home: h, readonly: ro }),
                        (Some(a), None) => view.stubs.push(Stub { id: k, node: a, port: l.out.clone(), input: false, other: leaf(l.to, &l.input), ty: l.ty, value: l.value.clone(), home: h, readonly: ro }),
                        (None, Some(b)) => view.stubs.push(Stub { id: k, node: b, port: l.input.clone(), input: true, other: leaf(l.from, &l.out), ty: l.ty, value: l.value.clone(), home: h, readonly: ro }),
                        (None, None) => {}
                    }
                }
            }
            Level::Node(id) => {
                let i = w.nodes.iter().position(|n| &n.id == id).context("the node is in the graph")?;
                let n = &w.nodes[i];
                if !n.track.is_empty() {
                    view.levels.push(format!("track:{}", n.track));
                    view.levels.push(format!("module:{}:{}", n.track, local(n)));
                } else {
                    view.levels.push(format!("module:{}", n.id));
                }
                view.nodes.push(ViewNode { id: local(n), kind: n.kind.clone(), inputs: n.inputs.clone(), outputs: n.outputs.clone(), world: n.world.clone() });
                // the other end of a cable: inside the same track its own `node.port`, else its rig-level leaf
                let other = |j: usize, p: &str| -> String { if !n.track.is_empty() && w.nodes[j].track == n.track { format!("{}.{p}", local(&w.nodes[j])) } else { leaf(j, p) } };
                for (k, l) in w.links.iter().enumerate() {
                    let h = home(k);
                    // a node's insides are the engine's (a module's: its Rust): nothing here is edited
                    let ro = Some("module");
                    if l.from == i {
                        view.stubs.push(Stub { id: k, node: 0, port: l.out.clone(), input: false, other: other(l.to, &l.input), ty: l.ty, value: l.value.clone(), home: h, readonly: ro });
                    } else if l.to == i {
                        view.stubs.push(Stub { id: k, node: 0, port: l.input.clone(), input: true, other: other(l.from, &l.out), ty: l.ty, value: l.value.clone(), home: h, readonly: ro });
                    }
                }
            }
        }
        Ok(view)
    }
}

impl Played {
    /// The played scene's wiring (see the module's doc); an error when it does not build as a graph.
    pub fn wiring(&self) -> Result<Wiring> {
        let mut scene = Scene::parse("")?;
        scene.nodes = self.nodes.clone();
        scene.links = self.links.clone();
        let g = Graph::build(&scene)?;
        let mut pos = vec![0; g.nodes.len()];
        for (at, &i) in g.order.iter().enumerate() {
            pos[i] = at;
        }
        let mut depth = vec![0u32; g.nodes.len()];
        for &i in &g.order {
            for l in g.links.iter().filter(|l| l.to.0 == i && pos[l.from.0] < pos[i]) {
                depth[i] = depth[i].max(depth[l.from.0] + 1);
            }
        }
        // a node's track: the longest track id that prefixes it as `<track>_` (a track's id may hold a `_` itself)
        let track = |id: &str| -> String {
            self.tracks
                .iter()
                .map(|t| t.track.as_str())
                .filter(|t| id.strip_prefix(t).is_some_and(|rest| rest.starts_with('_')))
                .max_by_key(|t| t.len())
                .unwrap_or_default()
                .to_string()
        };
        let ports = |list: &[crate::graph::Port]| list.iter().map(|p| WirePort { name: p.name.to_string(), ty: port_type(p.ty) }).collect();
        let nodes = g
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| WireNode {
                id: n.id.clone(),
                kind: n.kind.name().to_string(),
                track: track(&n.id),
                domain: domain(n.kind.domain()),
                depth: depth[i],
                inputs: ports(n.kind.inputs()),
                outputs: ports(n.kind.outputs()),
                played: !self.engine.page.iter().any(|p| p.id == n.id),
                world: (n.kind == Kind::World).then(|| crate::world::World::of(n)).transpose().ok().flatten().map(|w| {
                    let (source, address) = match &w.source {
                        crate::world::Source::Midi => ("midi", None),
                        crate::world::Source::Generator => ("generator", None),
                        crate::world::Source::Far(a) => ("far", Some(a.to_string())),
                    };
                    WireWorld { source, listen: w.listen, address }
                }),
            })
            .collect::<Vec<WireNode>>();
        // #432: a far world node is drawn and cabled, and listed as not connected until #442 carries its events
        let problems = nodes
            .iter()
            .filter_map(|n| n.world.as_ref().and_then(|w| w.address.as_ref()).map(|a| Problem::new(format!("world node '{}'", n.id), format!("listens over the relay for '{a}' – it plays only while the deck it names is in the session (#442)"))))
            .collect();
        // the scene's links come first in the graph's list, a module's bound outputs after them
        let links = g
            .links
            .iter()
            .enumerate()
            .map(|(k, l)| {
                let (f, t) = (&g.nodes[l.from.0], &g.nodes[l.to.0]);
                let out = f.kind.outputs()[l.from.1].name;
                WireLink {
                    from: l.from.0,
                    out: out.to_string(),
                    to: l.to.0,
                    input: t.kind.inputs()[l.to.1].name.to_string(),
                    ty: port_type(l.ty),
                    scale: l.scale,
                    offset: l.offset,
                    value: format!("{}.{out}", f.id),
                    bound: k >= scene.links.len(),
                }
            })
            .collect();
        Ok(Wiring { nodes, links, problem: String::new(), problems })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::Bundle;

    fn catpope() -> Wiring {
        Bundle::core("character.catpope").unwrap().unwrap().play(None, None).unwrap().wiring().unwrap()
    }

    fn node<'a>(w: &'a Wiring, id: &str) -> (usize, &'a WireNode) {
        w.nodes.iter().enumerate().find(|(_, n)| n.id == id).unwrap_or_else(|| panic!("no node {id}"))
    }

    #[test]
    fn catpope_s_rig_is_its_tracks_the_sum_and_the_being() {
        let w = catpope();
        assert!(w.problem.is_empty());
        let kinds: Vec<(&str, &str, &str)> = w.nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str(), n.track.as_str())).collect();
        for want in [
            ("voice_voice", "fm", "voice"),
            ("voice_melody", "walk", "voice"),
            ("drone_drone", "osc", "drone"),
            ("drone_mix", "mixer", "drone"),
            ("mix1", "mixer", ""),
            ("out", "audio_out", ""),
            ("fft", "fft", ""),
            ("space", "space", ""),
        ] {
            assert!(kinds.contains(&want), "{want:?} in {kinds:?}");
        }
        let (_, fm) = node(&w, "voice_voice");
        assert_eq!(fm.domain, "audio");
        assert!(fm.inputs.iter().any(|p| p.name == "ratio" && p.ty == "control"), "the catalog's ports: {:?}", fm.inputs);
        assert!(w.links.iter().all(|l| !l.bound), "no module in catpope's rig");
    }

    #[test]
    fn links_are_resolved_to_ports_with_the_path_of_their_value() {
        let w = catpope();
        let (melody, _) = node(&w, "voice_melody");
        let (voice, _) = node(&w, "voice_voice");
        let cv = w.links.iter().find(|l| l.from == melody && l.to == voice).expect("melody → voice");
        assert_eq!((cv.out.as_str(), cv.input.as_str(), cv.ty, cv.value.as_str()), ("cv", "cv", "audio", "voice_melody.cv"));
        // `from = "drone"`: a bare node is its first output
        let (drone, _) = node(&w, "drone_drone");
        let bare = w.links.iter().find(|l| l.from == drone).expect("drone → mix");
        assert_eq!((bare.out.as_str(), bare.input.as_str(), bare.value.as_str()), ("out", "in1", "drone_drone.out"));
        let (fft, _) = node(&w, "fft");
        let pulse = w.links.iter().find(|l| l.from == fft).expect("fft.level → space.pulse");
        assert_eq!((pulse.ty, pulse.scale), ("control", 0.6));
        for l in &w.links {
            assert!(w.nodes[l.from].outputs.iter().any(|p| p.name == l.out && p.ty == l.ty), "{l:?}");
            assert!(w.nodes[l.to].inputs.iter().any(|p| p.name == l.input && p.ty == l.ty), "{l:?}");
        }
    }

    #[test]
    fn depth_follows_the_signal() {
        let w = catpope();
        let d = |id: &str| node(&w, id).1.depth;
        assert_eq!(d("voice_melody"), 0);
        assert!(d("voice_melody") < d("voice_voice") && d("voice_voice") < d("mix1"));
        assert!(d("drone_drone") < d("drone_mix") && d("drone_mix") < d("mix1"));
        assert!(d("mix1") < d("out") && d("mix1") < d("fft") && d("fft") < d("space"));
    }

    /// #408: a code rig's policy is a node like any other – the scene's own (no track), in the control domain, its
    /// target links drawn as bound cables, the browser not playing it yet
    #[test]
    fn a_code_rig_s_policy_and_its_cables() {
        let w = crate::rig::policy::tests::code_catpope().play(None, None).unwrap().wiring().unwrap();
        let (policy, n) = node(&w, "policy");
        assert_eq!((n.kind.as_str(), n.track.as_str(), n.domain, n.played), ("module:catpope_rig", "", "control", false));
        assert_eq!(n.outputs.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["voice_preset", "voice_index", "drone_gain"]);
        assert!(n.inputs.iter().any(|p| p.name == "heat" && p.ty == "control"), "its parameter is an input");
        let cables: Vec<(String, &str, String, bool)> = w.links.iter().filter(|l| l.from == policy).map(|l| (l.value.clone(), w.nodes[l.to].id.as_str(), l.input.clone(), l.bound)).collect();
        assert_eq!(
            cables,
            [("policy.voice_index".to_string(), "voice_voice", "index".to_string(), true), ("policy.drone_gain".to_string(), "drone_drone", "gain".to_string(), true)]
        );
        // fed by nothing, it is a source; its cables into the audio half cross threads, so they order nothing
        assert_eq!(node(&w, "policy").1.depth, 0);
    }

    /// #433 §4: the rig level – the tracks and the being as boxes with the ports cables touch, the compiled sum as stubs
    /// (read-only), the default pulse into the being from the sum; the level chain is the rig alone
    #[test]
    fn the_rig_level_shows_tracks_the_being_and_the_sum_as_stubs() {
        let p = Bundle::core("character.catpope").unwrap().unwrap().play(None, None).unwrap();
        let v = p.wire_view("").unwrap();
        assert_eq!(v.levels, ["rig"]);
        assert_eq!(v.nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str())).collect::<Vec<_>>(), [("voice", "track"), ("drone", "track"), ("being", "space")]);
        assert!(v.links.is_empty(), "catpope's core rig has no route");
        let stubs: Vec<(&str, &str, bool, &str)> = v.stubs.iter().map(|s| (v.nodes[s.node].id.as_str(), s.port.as_str(), s.input, s.other.as_str())).collect();
        assert_eq!(stubs, [("voice", "voice.out", false, "sum"), ("drone", "mix.out", false, "sum"), ("being", "pulse", true, "sum")], "{stubs:?}");
        assert!(v.stubs.iter().all(|s| s.home == Home::Compiled && s.readonly == Some("compiled")));
        assert_eq!(v.nodes[0].outputs.iter().map(|p| (p.name.as_str(), p.ty)).collect::<Vec<_>>(), [("voice.out", "audio")]);
        assert_eq!(v.nodes[2].inputs.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["pulse"]);
        assert!(p.wire_view("nobody").is_err() && p.wire_view("voice:nobody").is_err());
    }

    /// the track level: the instrument's nodes by their own ids, its links with their home (editable), the sum as a stub
    #[test]
    fn the_track_level_shows_the_instrument_and_names_its_cables_home() {
        let p = Bundle::core("character.catpope").unwrap().unwrap().play(None, None).unwrap();
        let v = p.wire_view("voice").unwrap();
        assert_eq!(v.levels, ["rig", "track:voice"]);
        assert_eq!(v.nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str())).collect::<Vec<_>>(), [("voice", "fm"), ("melody", "walk")]);
        assert_eq!(v.links.len(), 1);
        let l = &v.links[0];
        assert_eq!((v.nodes[l.from].id.as_str(), l.out.as_str(), v.nodes[l.to].id.as_str(), l.input.as_str(), l.value.as_str()), ("melody", "cv", "voice", "cv", "voice_melody.cv"));
        assert_eq!((&l.home, l.readonly), (&Home::Instrument { track: "voice".into() }, None));
        assert_eq!(v.stubs.iter().map(|s| (v.nodes[s.node].id.as_str(), s.port.as_str(), s.input, s.other.as_str(), s.readonly)).collect::<Vec<_>>(), [("voice", "out", false, "sum", Some("compiled"))]);
        assert_eq!(p.links.len(), p.homes.len(), "one home per link");
    }

    /// the node level: its ports, every cable at them as a stub (inside the track by the node's own name), all read-only
    #[test]
    fn the_node_level_is_its_ports_and_stubs() {
        let p = Bundle::core("character.catpope").unwrap().unwrap().play(None, None).unwrap();
        let v = p.wire_view("voice:voice").unwrap();
        assert_eq!(v.levels, ["rig", "track:voice", "module:voice:voice"]);
        assert_eq!((v.nodes.len(), v.nodes[0].id.as_str(), v.nodes[0].kind.as_str()), (1, "voice", "fm"));
        assert!(v.nodes[0].inputs.iter().any(|p| p.name == "ratio"), "every port, cabled or not");
        let stubs: Vec<(&str, bool, &str)> = v.stubs.iter().map(|s| (s.port.as_str(), s.input, s.other.as_str())).collect();
        assert_eq!(stubs, [("cv", true, "melody.cv"), ("out", false, "sum")]);
        assert!(v.stubs.iter().all(|s| s.readonly == Some("module")));
        let own = p.wire_view("fft").unwrap();
        assert_eq!(own.levels, ["rig", "module:fft"]);
        assert!(own.stubs.iter().any(|s| s.port == "level" && !s.input && s.other == "being:pulse"));
    }

    #[test]
    fn the_deck_s_own_scene_marks_what_the_browser_does_not_play() {
        let own = Played::scene(crate::scene::CATHEDRAL_SCENE).unwrap();
        let w = own.wiring().unwrap();
        assert_eq!(w.nodes.len(), own.nodes.len());
        assert!(w.nodes.iter().all(|n| n.track.is_empty()));
        for n in &w.nodes {
            assert_eq!(n.played, !own.engine.page.iter().any(|p| p.id == n.id), "{}", n.id);
        }
        assert!(w.nodes.iter().any(|n| !n.played), "chapter_music is the page's");
    }
}
