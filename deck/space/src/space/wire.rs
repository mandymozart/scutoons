//! A deck's being on the wire (#188 phase 4, LIVE.md B.6): `deck.graph` from a graph and its look (`deck_graph`) and
//! back (`from_wire`) – the conductor builds every deck's body from it, the browser deck (#198) its own; what a
//! session's link hands the frame (`In`, the relay client's – relay.rs) and the link itself as the space sees it (`Wire` –
//! deck-native's link.rs drives the relay client on a thread).
use protocol::{DeckGraph, DeckLink, DeckMidi, DeckNode, DeckQuality, DeckValues, SceneSync};

use super::being::Look;
use crate::graph::Graph;

/// what the conductor's frame thread learns (the relay client's, relay.rs)
pub use crate::relay::In;

/// a session's link as the frame sees it: a deck's changes out, the session's events in, a word for the HUD
pub trait Wire: Send {
    /// a deck's changes (the link folds them, sends them in order)
    fn send(&self, values: DeckValues);
    /// #195: the conductor pushes a scene sync (the link sends it once joined, and again when the relay forgot it)
    fn push(&self, sync: SceneSync);
    /// #220: this deck's render quality (the link keeps the last one and sends it again after a rejoin)
    fn quality(&self, _quality: DeckQuality) {}
    /// #211: the deck's MIDI outputs for the conductor (`deck.midi`); a wire without the relay drops them
    fn midi(&self, _midi: DeckMidi) {}
    /// the next event, if any (never blocks)
    fn recv(&self) -> Option<In>;
    /// the link's state in a word (the HUD)
    fn status(&self) -> String;
}

/// a deck's being for the wire: its graph's nodes and links and the look's head – `moods` = the head's whole palette
/// (the energy's ladder + the feelings' moods), so the conductor knows every mood the deck may stream
pub fn deck_graph(graph: &Graph, look: &Look, moods: &[String]) -> DeckGraph {
    let port = |(n, p): (usize, usize), out: bool| {
        let node = &graph.nodes[n];
        let name = if out { node.kind.outputs()[p].name } else { node.kind.inputs()[p].name };
        format!("{}.{name}", node.id)
    };
    DeckGraph {
        nodes: graph.nodes.iter().map(|n| DeckNode { id: n.id.clone(), kind: n.kind.name().into() }).collect(),
        links: graph
            .links
            .iter()
            .map(|l| DeckLink {
                from: port(l.from, true),
                to: port(l.to, false),
                scale: (l.scale != 1.0).then_some(l.scale as f64),
                offset: (l.offset != 0.0).then_some(l.offset as f64),
            })
            .collect(),
        character: (!look.character.is_empty()).then(|| look.character.clone()),
        size: Some(look.size as f64),
        moods: Some(moods.to_vec()),
        expose: (!look.expose.is_empty()).then(|| look.expose.clone()),
    }
}

/// a deck's being from the wire: its graph (refused when it names a kind the stage doesn't know) and its look
pub fn from_wire(g: &DeckGraph) -> anyhow::Result<(Graph, Look)> {
    use crate::scene::{LinkDef, NodeDef, Scene};
    let mut scene = Scene::parse("")?;
    scene.nodes = g.nodes.iter().map(|n| NodeDef { id: n.id.clone(), kind: n.kind.clone(), params: toml::Table::new() }).collect();
    scene.links = g
        .links
        .iter()
        .map(|l| LinkDef { from: l.from.clone(), to: l.to.clone(), scale: l.scale.unwrap_or(1.0) as f32, offset: l.offset.unwrap_or(0.0) as f32, map: Vec::new() })
        .collect();
    let mut look = Look { size: g.size.unwrap_or(0.5).clamp(0.1, 2.0) as f32, character: g.character.clone().unwrap_or_default(), ..Look::default() };
    if let Some(m) = g.moods.as_ref().filter(|m| !m.is_empty()) {
        look.moods = m.clone();
    }
    look.expose = g.expose.clone().unwrap_or_default();
    Ok((Graph::build(&scene)?, look))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Scene, CATHEDRAL_SCENE};
    use protocol::Message;

    #[test]
    fn a_being_survives_the_wire() {
        let scene = Scene::parse(CATHEDRAL_SCENE).unwrap();
        let g = Graph::build(&scene).unwrap();
        let wire = deck_graph(&g, &scene.being, &scene.being.moods);
        let json = serde_json::to_string(&Message::DeckGraph(wire.clone())).unwrap();
        let Message::DeckGraph(back) = serde_json::from_str(&json).unwrap() else { panic!() };
        let (g2, look) = from_wire(&back).unwrap();
        assert_eq!(g2.nodes.iter().map(|n| (&n.id, n.kind)).collect::<Vec<_>>(), g.nodes.iter().map(|n| (&n.id, n.kind)).collect::<Vec<_>>());
        assert_eq!(g2.links.len(), g.links.len());
        for (a, b) in g.links.iter().zip(&g2.links) {
            assert_eq!((a.from, a.to, a.scale, a.offset), (b.from, b.to, b.scale, b.offset));
        }
        assert_eq!((look.character.as_str(), look.size, &look.moods), (scene.being.character.as_str(), scene.being.size, &scene.being.moods));
        // a kind this stage doesn't know: refused with its name
        let mut odd = wire;
        odd.nodes[0].kind = "teleporter".into();
        assert!(from_wire(&odd).unwrap_err().to_string().contains("teleporter"));
    }

    /// #197: the browser deck's being (deck/lab/www/deck-scene.ts, its fixture kept equal by deck-client.test.ts)
    /// builds on the conductor like a native deck's, its exposed parameters as buttons
    #[test]
    fn the_browser_decks_being_builds() {
        let Message::DeckGraph(g) = serde_json::from_str(include_str!("../../../lab/fixtures/deck-graph.json")).unwrap() else { panic!() };
        let (graph, look) = from_wire(&g).unwrap();
        assert_eq!((graph.nodes.len(), graph.links.len()), (g.nodes.len(), g.links.len()));
        assert_eq!(look.character, "yellowmouse");
        let being = super::super::being::Being::new(&graph, &look);
        assert_eq!(being.buttons.len(), g.expose.as_ref().unwrap().len(), "one button per exposed parameter");
    }
}
