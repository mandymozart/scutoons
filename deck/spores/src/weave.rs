//! A weave (the spore contract: protocol::spore_network) as the graph the widget lays out and paints: a spore a node with
//! its ports as pins (inlets and outlets in their order), a thread a link between them, the canvas's groups as frames
//! and its folds on the nodes. The head's small word is the spore's family (its type stays in the detail).
use deck_kit::base::graph::{Graph, Link, Node, Pin};
use deck_kit::base::signal::Signal;
use protocol::spore_network::{self as sp, Direction, Weave};

/// the contract's signal as the one table's
pub fn signal(s: sp::Signal) -> Signal {
    match s {
        sp::Signal::Event => Signal::Event,
        sp::Signal::Control => Signal::Control,
        sp::Signal::Clock => Signal::Clock,
        sp::Signal::Audio => Signal::Audio,
        sp::Signal::Picture => Signal::Picture,
        sp::Signal::Address => Signal::Address,
        sp::Signal::Vote => Signal::Vote,
        sp::Signal::Scene => Signal::Scene,
        sp::Signal::Far => Signal::Far,
        sp::Signal::Job => Signal::Job,
        sp::Signal::Asset => Signal::Asset,
    }
}

/// a family's word: the card's small word and its icon
pub fn family(f: sp::Family) -> &'static str {
    match f {
        sp::Family::Event => "event",
        sp::Family::Rule => "rule",
        sp::Family::Action => "action",
        sp::Family::Variable => "variable",
        sp::Family::Entity => "entity",
        sp::Family::Scene => "scene",
        sp::Family::Plugin => "plugin",
        sp::Family::Container => "group",
        sp::Family::Sound => "sound",
        sp::Family::World => "world",
        sp::Family::Free => "free",
        sp::Family::Far => "far",
    }
}

/// The graph of `w`, and its canvas's groups as frames (a group's index, its spores' indices). A thread whose end is not
/// a port of its spore is left out.
pub fn graph(w: &Weave) -> (Graph, Vec<(usize, Vec<usize>)>) {
    let folded = w.canvas.as_ref().and_then(|c| c.folded.as_ref());
    let groups = w.canvas.as_ref().and_then(|c| c.groups.as_deref()).unwrap_or_default();
    let group_of = |id: &str| groups.iter().position(|g| g.spores.iter().any(|s| s == id));
    let nodes: Vec<Node> = w
        .spores
        .iter()
        .map(|s| {
            let pins = |dir: Direction| s.ports.iter().filter(|p| p.dir == dir).map(|p| Pin::new(p.label.clone(), signal(p.signal))).collect();
            Node {
                id: s.id.clone(),
                title: s.label.clone(),
                kind: family(s.family),
                detail: s.detail.clone().unwrap_or_else(|| s.kind.clone()),
                ins: pins(Direction::In),
                outs: pins(Direction::Out),
                far: s.family == sp::Family::Far,
                collapsed: folded.and_then(|f| f.get(&s.id)).copied().unwrap_or(false),
                picture: s.picture.clone(),
                group: group_of(&s.id),
            }
        })
        .collect();
    let end = |e: &sp::End, dir: Direction| -> Option<(usize, usize)> {
        let i = w.spores.iter().position(|s| s.id == e.spore)?;
        let k = w.spores[i].ports.iter().filter(|p| p.dir == dir).position(|p| p.id == e.port)?;
        Some((i, k))
    };
    let links = w
        .threads
        .iter()
        .filter_map(|t| {
            let mut l = Link::new(end(&t.from, Direction::Out)?, end(&t.to, Direction::In)?, signal(t.signal));
            l.value = t.label.clone().filter(|v| !v.is_empty());
            Some(l)
        })
        .collect();
    let frames = (0..groups.len()).map(|k| (k, nodes.iter().enumerate().filter(|(_, n)| n.group == Some(k)).map(|(i, _)| i).collect())).collect();
    (Graph { nodes, links }, frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VECTORS: &str = include_str!("../../../server/schema/api/spore-network.vectors.json");

    fn weave(scope: &str) -> Weave {
        let file: serde_json::Value = serde_json::from_str(VECTORS).unwrap();
        let raw = file["weaves"].as_array().unwrap().iter().find(|w| w["scope"] == scope).unwrap().clone();
        serde_json::from_value(raw).unwrap()
    }

    #[test]
    fn every_shared_weave_becomes_a_graph_with_all_its_threads() {
        let file: serde_json::Value = serde_json::from_str(VECTORS).unwrap();
        for raw in file["weaves"].as_array().unwrap() {
            let w: Weave = serde_json::from_value(raw.clone()).unwrap();
            let (g, _) = graph(&w);
            assert_eq!((g.nodes.len(), g.links.len()), (w.spores.len(), w.threads.len()), "{}", w.context);
        }
    }

    #[test]
    fn a_rig_keeps_its_ports_in_order_its_group_and_its_signals() {
        let (g, frames) = graph(&weave("rig"));
        let osc = g.nodes.iter().find(|n| n.id == "voice_osc").unwrap();
        assert_eq!((osc.kind, osc.ins.len(), osc.outs.len(), osc.outs[0].signal), ("sound", 2, 1, Signal::Audio));
        assert_eq!(frames, vec![(0, vec![1, 2])], "the voice group frames osc and sum");
        let l = &g.links[0];
        assert_eq!((l.from, l.to), ((0, 0), (1, 0)), "midi's note into osc's note");
        assert_eq!(l.signal, Signal::Event);
    }
}
