//! A world node (#430, research/EVENTS-VS-PARAMS.md §5): a source from outside a rig as a node of its graph, with ordinary
//! event outputs, cabled like a module's. It holds the name filter (`listen`) – the one place a document names a world
//! event. Module to module there are only cables; an event input with no cable hears nothing.
//!
//! Two sources now, the others (carrier bag, VN state, other scenes and decks) are later tickets:
//! - `midi` – the MIDI adapter's events through the dispatcher, `listen = ["midi.note"]`
//! - `generator` – the deck-events patch in its `patch` table run by the node itself (the engine's `Host`), its notes
//!   going down its cables and nowhere else, so two generators stay apart; `listen` names what leaves it (`music.note`)
use crate::graph::Node;
use anyhow::{anyhow, bail, Result};
use deck_events::patch::{NoteNames, Patch};
use deck_events::Names;

/// the names a note comes as: the generators' `music.note`, the MIDI adapter's `midi.note` – a note is a note on any cable
pub const NOTE_NAMES: [&str; 2] = ["music.note", "midi.note"];

pub fn is_note(name: &str) -> bool {
    NOTE_NAMES.contains(&name)
}

/// whether an event port sending `sends` can reach one taking `takes`: a name in common, or notes on both sides
pub fn shares_a_name(sends: &[String], takes: &[String]) -> bool {
    sends.iter().any(|s| takes.contains(s)) || (sends.iter().any(|s| is_note(s)) && takes.iter().any(|t| is_note(t)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Midi,
    Generator,
}

/// A world node's meaning: where its events come from and its name filter.
#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub source: Source,
    pub listen: Vec<String>,
}

impl World {
    /// `node`'s world params, checked: a known `source`, a `listen` of valid names that this source can send, a
    /// generator's `patch` that builds
    pub fn of(node: &Node) -> Result<World> {
        let at = |what: String| anyhow!("world node '{}': {what}", node.id);
        let source = match node.str("source") {
            Some("midi") => Source::Midi,
            Some("generator") => Source::Generator,
            Some(other) => return Err(at(format!("unknown source '{other}' (midi, generator)"))),
            None => return Err(at("it needs a `source` (midi, generator)".into())),
        };
        let listen = node.strs("listen");
        if listen.is_empty() {
            return Err(at("it needs a `listen`: the names it lets through".into()));
        }
        let mut names = Names::new();
        for n in &listen {
            names.event(n).map_err(|e| at(format!("`listen` names '{n}': {e:?}")))?;
        }
        match source {
            Source::Midi => {
                if let Some(n) = listen.iter().find(|n| !n.starts_with("midi.")) {
                    bail!("world node '{}': MIDI sends midi.* names, not '{n}'", node.id);
                }
            }
            Source::Generator => {
                if listen != ["music.note"] {
                    bail!("world node '{}': a generator sends music.note, so `listen` is [\"music.note\"]", node.id);
                }
                let Some(table) = node.params.get("patch").and_then(|p| p.as_table()) else {
                    bail!("world node '{}': a generator needs a `patch` table (`generator`, `edge`, `out`)", node.id)
                };
                let notes = NoteNames::new(&mut names);
                Patch::from_table(table).and_then(|p| p.build(notes, &mut names).map(|_| ())).map_err(|e| at(format!("its patch: {e}")))?;
            }
        }
        Ok(World { source, listen })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::scene::Scene;

    fn build(text: &str) -> Result<Graph> {
        Graph::build(&Scene::parse(text).unwrap())
    }

    #[test]
    fn a_midi_world_node_holds_its_filter_and_has_one_event_output() {
        let g = build("[[node]]\nid=\"keys\"\nkind=\"world\"\nsource=\"midi\"\nlisten=[\"midi.note\"]\n").unwrap();
        assert_eq!(g.nodes[0].kind.outputs().iter().map(|p| (p.name, p.ty)).collect::<Vec<_>>(), [("events", crate::graph::PortType::Event)]);
        assert_eq!(World::of(&g.nodes[0]).unwrap(), World { source: Source::Midi, listen: vec!["midi.note".to_string()] });
    }

    #[test]
    fn a_world_node_is_refused_with_a_sentence() {
        for (text, says) in [
            ("source=\"radio\"\nlisten=[\"midi.note\"]\n", "unknown source"),
            ("listen=[\"midi.note\"]\n", "needs a `source`"),
            ("source=\"midi\"\n", "needs a `listen`"),
            ("source=\"midi\"\nlisten=[\"story.fx\"]\n", "MIDI sends midi.*"),
            ("source=\"generator\"\nlisten=[\"midi.note\"]\n", "a generator sends music.note"),
            ("source=\"generator\"\nlisten=[\"music.note\"]\n", "needs a `patch`"),
        ] {
            let e = build(&format!("[[node]]\nid=\"w\"\nkind=\"world\"\n{text}")).unwrap_err().to_string();
            assert!(e.contains("world node 'w'") && e.contains(says), "{e}");
        }
    }

    #[test]
    fn a_generator_holds_a_patch_that_builds() {
        let good = "[[node]]\nid=\"g\"\nkind=\"world\"\nsource=\"generator\"\nlisten=[\"music.note\"]\npatch = { out = \"w.notes\", generator = [{ id = \"w\", kind = \"random_walk\", seed = 3 }] }\n";
        assert_eq!(World::of(&build(good).unwrap().nodes[0]).unwrap().source, Source::Generator);
        let bad = good.replace("w.notes", "nobody.notes");
        assert!(build(&bad).unwrap_err().to_string().contains("its patch"));
    }

    #[test]
    fn a_note_is_a_note_on_any_cable() {
        let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert!(shares_a_name(&v(&["midi.note"]), &v(&["music.note"])));
        assert!(shares_a_name(&v(&["story.cue", "music.note"]), &v(&["story.cue"])));
        assert!(!shares_a_name(&v(&["story.cue"]), &v(&["music.note"])));
    }
}
