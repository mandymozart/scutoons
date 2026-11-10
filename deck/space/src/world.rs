//! A world node (#430, research/EVENTS-VS-PARAMS.md §5): a source from outside a rig as a node of its graph, with ordinary
//! event outputs, cabled like a module's. It holds the name filter (`listen`) – the one place a document names a world
//! event. Module to module there are only cables; an event input with no cable hears nothing.
//!
//! Three sources now, the others (carrier bag, VN state) are later tickets:
//! - `midi` – the MIDI adapter's events through the dispatcher, `listen = ["midi.note"]`
//! - `generator` – the deck-events patch in its `patch` table run by the node itself (the engine's `Host`), its notes
//!   going down its cables and nowhere else, so two generators stay apart; `listen` names what leaves it (`music.note`)
//! - `far` – another scene, character, rig or deck, named by its `address` (rig/address.rs: a place before the leaf,
//!   `character:catpope/analog:seq.notes`, or absolute `/deck:left/scene:nave/…`; #432, owner: reach is wide from the
//!   start). The deck's link asks the named deck over the relay and feeds what it sends into the cable's ring (far.rs,
//!   #442); the wiring and the validator list that it plays only while that deck is in the session. `listen` names
//!   what it lets through.
use crate::graph::Node;
use crate::rig::address::Address;
use anyhow::{anyhow, bail, Result};
use deck_events::patch::{NoteNames, Patch};
use deck_events::Names;

/// What a far source's holder may be from the toon's address book (#477; owner 2026-11-10: an address is real only where a
/// receiver of that type exists): the kinds a deck answers as a holder today – far.rs `offers` matches a `character` or an
/// `item` place against the subject the deck plays. No receiver takes a location yet; a persona or a look is no place in an
/// address (address.rs `Place`); chapters, groups and variables are the creator's references, not addresses.
pub const FAR_HOLDERS: [protocol::address_book::AddressKind; 2] = {
    use protocol::address_book::AddressKind::*;
    [Character, Item]
};

/// the names a note comes as: the generators' `music.note`, the MIDI adapter's `midi.note` – a note is a note on any cable
pub const NOTE_NAMES: [&str; 2] = ["music.note", "midi.note"];

pub fn is_note(name: &str) -> bool {
    NOTE_NAMES.contains(&name)
}

/// whether an event port sending `sends` can reach one taking `takes`: a name in common, or notes on both sides
pub fn shares_a_name(sends: &[String], takes: &[String]) -> bool {
    sends.iter().any(|s| takes.contains(s)) || (sends.iter().any(|s| is_note(s)) && takes.iter().any(|t| is_note(t)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Midi,
    Generator,
    /// another place's events (#432), carried over the relay by the deck's link (far.rs, #442)
    Far(Address),
}

/// A world node's meaning: where its events come from and its name filter.
#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub source: Source,
    pub listen: Vec<String>,
}

impl World {
    /// `node`'s world params, checked: a known `source`, a `listen` of valid names that this source can send, a
    /// generator's `patch` that builds, a far source's `address` that parses and names a place
    pub fn of(node: &Node) -> Result<World> {
        World::read(&node.id, &node.params)
    }

    /// `of` from a document's node (its id and params), before it is a graph node
    pub fn read(id: &str, params: &toml::Table) -> Result<World> {
        let at = |what: String| anyhow!("world node '{id}': {what}");
        let str = |key: &str| params.get(key).and_then(|v| v.as_str());
        let source = match str("source") {
            Some("midi") => Source::Midi,
            Some("generator") => Source::Generator,
            Some("far") => match str("address") {
                Some(text) => {
                    let a = Address::parse(text).map_err(|e| at(format!("`address` '{text}': {e}")))?;
                    if a.is_relative() {
                        return Err(at(format!("`address` '{text}' names no place – a far source is another scene, character, rig or deck (character:<id>/<track>:<node>.<port>, /deck:<id>/scene:<id>/…); inside the rig a cable joins the ports")));
                    }
                    Source::Far(a)
                }
                None => return Err(at("a far source needs an `address` (character:<id>/<track>:<node>.<port>, /deck:<id>/scene:<id>/…)".into())),
            },
            Some(other) => return Err(at(format!("unknown source '{other}' (midi, generator, far)"))),
            None => return Err(at("it needs a `source` (midi, generator, far)".into())),
        };
        let listen: Vec<String> = match params.get("listen") {
            Some(toml::Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
            _ => Vec::new(),
        };
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
                    bail!("world node '{id}': MIDI sends midi.* names, not '{n}'");
                }
            }
            Source::Generator => {
                if listen != ["music.note"] {
                    bail!("world node '{id}': a generator sends music.note, so `listen` is [\"music.note\"]");
                }
                let Some(table) = params.get("patch").and_then(|p| p.as_table()) else {
                    bail!("world node '{id}': a generator needs a `patch` table (`generator`, `edge`, `out`)")
                };
                let notes = NoteNames::new(&mut names);
                Patch::from_table(table).and_then(|p| p.build(notes, &mut names).map(|_| ())).map_err(|e| at(format!("its patch: {e}")))?;
            }
            // what a far place sends is its own business; the filter says what this node lets through
            Source::Far(_) => {}
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

    /// #432: a far source names a place outside the rig; it builds, and sends nothing until #442 carries it
    #[test]
    fn a_far_world_node_names_a_place_and_builds() {
        let g = build("[[node]]\nid=\"far\"\nkind=\"world\"\nsource=\"far\"\naddress=\"character:catpope/analog:seq.notes\"\nlisten=[\"music.note\"]\n").unwrap();
        let w = World::of(&g.nodes[0]).unwrap();
        assert_eq!(w.source, Source::Far(Address::parse("character:catpope/analog:seq.notes").unwrap()));
        assert_eq!(w.listen, ["music.note"]);
        let absolute = build("[[node]]\nid=\"far\"\nkind=\"world\"\nsource=\"far\"\naddress=\"/deck:left/scene:nave/character:catpope/@energy\"\nlisten=[\"story.cue\"]\n").unwrap();
        assert!(matches!(World::of(&absolute.nodes[0]).unwrap().source, Source::Far(a) if a.absolute));
        for (text, says) in [
            ("source=\"far\"\nlisten=[\"music.note\"]\n", "needs an `address`"),
            ("source=\"far\"\naddress=\"seq.notes\"\nlisten=[\"music.note\"]\n", "names no place"),
            ("source=\"far\"\naddress=\"nave/seq.notes\"\nlisten=[\"music.note\"]\n", "not a place"),
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
