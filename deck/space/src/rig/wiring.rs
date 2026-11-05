//! The wiring of a played scene (#282, the rig designer's Nodes tab): its nodes with their kinds, the rig's track each
//! belongs to, their ports (the kind catalog's, a module's descriptor's) and their depth in the graph, and its links
//! resolved to ports (a bare `node` is its first output) with the path whose value flows on the cable. It is the checked
//! graph (`Graph::build`: a module's bound path outputs are links too) as the editor draws it – the editor only places
//! it; the live values on the cables are the engine's analysis frame's (live-core `value(path)`, the native deck's
//! value table), read by the paths named here.
use super::Played;
use crate::graph::{Domain, Graph, PortType};
use crate::scene::Scene;
use anyhow::Result;
use serde::Serialize;

/// The graph of what a deck plays, for drawing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Wiring {
    pub nodes: Vec<WireNode>,
    pub links: Vec<WireLink>,
    /// why there is no graph ("" = there is one)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub problem: String,
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
    /// no graph, and why
    pub fn failed(problem: String) -> Wiring {
        Wiring { nodes: Vec::new(), links: Vec::new(), problem }
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
            })
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
        Ok(Wiring { nodes, links, problem: String::new() })
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

    #[test]
    fn the_deck_s_own_scene_marks_what_the_browser_does_not_play() {
        let own = Played::deck_scene().unwrap();
        let w = own.wiring().unwrap();
        assert_eq!(w.nodes.len(), own.nodes.len());
        assert!(w.nodes.iter().all(|n| n.track.is_empty()));
        for n in &w.nodes {
            assert_eq!(n.played, !own.engine.page.iter().any(|p| p.id == n.id), "{}", n.id);
        }
        assert!(w.nodes.iter().any(|n| !n.played), "chapter_music is the page's");
    }
}
