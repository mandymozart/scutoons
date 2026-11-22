//! The checks of an instrument, a preset or a rig as the catalog stores them (#313): what the API asks before a write to
//! the catalog (deck/build `POST /validate`) and what the creator's wasm can ask as a hint on a draft (space-web
//! `rig_problems`) – the code the decks play with, no second copy: serde's forms (unknown fields refused), then
//! - an instrument: `Instrument::check`;
//! - a preset: its instrument known, `PresetBook::lineage` / `resolve` with this preset in the book, `Instrument::patch`;
//! - a rig: `Rig::check` (a code rig's policy too, #408: its module, targets and who writes which input), then
//!   `Bundle::play` (flatten, the knobs and sticks, the engine's lowering) without a variant, for every persona and for
//!   every mood; `Rig::warnings` (a preset's value on an input the policy owns is dropped – said, not refused);
//! - a scene (#243, kind `scenes`, the `scene:<id>` document the API publishes): `SceneDocument` format 2 (#489: id = the
//!   request's, unknown fields refused), its `check` (the location, holders' ids, fillers and filters – a template's
//!   placeholders and previews well formed –, spawn zones, rigs and values), then resolved with its previews
//!   (scene_resolve.rs; no rigs travel with the request, so the holders stand silent) into `Graph::build` (kinds, ids,
//!   links, at most one sink of each kind, no cycle) and the browser engine's split (`engine::lower`) – what every deck
//!   does before it plays a scene. Its `module:<id>` nodes take their kinds from the descriptors in `modules` (registered
//!   as a deck's library registers them); a module none of them names is not in the catalog yet. Shared vectors:
//!   server/tests/fixtures/scenes/modules.json. The outputs table is checked as a shape only here (the API's JSON Schema);
//! - a use of a scene (#504, kind `uses`, `id` = the scene's id): `{ scene, with }` (scene_resolve.rs `Use`, unknown fields
//!   refused) with its scene's published document in `scenes`, its modules' descriptors in `modules` and the toon's book
//!   in `book` – `Use::problems`: the scene
//!   and every filler in the book, each filler against its placeholder's `fill`, the scene resolved with them. The
//!   requests as the API builds them and the core's answers: server/tests/fixtures/scenes/uses.json (shared vectors);
//! - a device preset (kind `device-presets`): `DevicePreset` (unknown fields refused, id = the request's), its `problems`;
//! - a rig preset (kind `rig-presets`): `RigPreset` (unknown fields refused, id = the request's), its `problems`;
//! - a model's spots (kind `spots`, `id` = the asset's id, the document a version's `spots`): deck-catalog `Spots`
//!   (unknown fields refused), its `problems` – names, doubles, hand edits of what the file does not have, a bone or node
//!   the model does not have – and, with the descriptor of the module this model is the body of in `modules`, its port
//!   spots against the module's ports. Shared vectors: server/tests/fixtures/assets/spots.json.
//!
//! The lookups travel by value: the instruments it names, the presets it names with their ancestors, the descriptors of
//! the modules it names (the API reads them from its catalog). Every problem a check lists is in the answer, each naming
//! its cable, node or track (problem.rs, #432); a far world node is a warning – valid, and it plays only while the deck
//! it names is in the session (#442).
use super::bundle::Bundle;
use super::{Instrument, Library, Preset, PresetBook, Rig};
use crate::problem::sentences;
use anyhow::{Context, Result};
use deck_catalog::Descriptor;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One document to check with what it names.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// the catalog kind: `instruments` · `presets` · `rigs` · `scenes` (#243: a `scene:<id>` document, not catalog) ·
    /// `uses` (#504: a use `{ scene, with }` of scene `id`)
    pub kind: String,
    /// the catalog id: an instrument's or a preset's id, a rig's subject
    pub id: String,
    pub document: Value,
    /// the instruments it names (a preset's, a rig's tracks'), by value
    #[serde(default)]
    pub instruments: Vec<Value>,
    /// the presets it names and their ancestors (a preset's lineage without itself, a rig's), by value
    #[serde(default)]
    pub presets: Vec<Value>,
    /// the descriptors of the `module:<id>` documents it names, by value: a code rig's policy module (#408), a scene's
    /// module nodes
    #[serde(default)]
    pub modules: Vec<Value>,
    /// a use's scene (#504, kind `uses`): the published `scene:<id>` document it names, by value
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenes: Vec<Value>,
    /// a use's toon book (#504, kind `uses`): the references of the toon's address book; none = not asked
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book: Option<Vec<String>>,
}

/// The answer: the problems, none = valid; the warnings (RULES H7: they can be accepted).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    pub problems: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// the problems of a check: every sentence listed
type Problems = Vec<String>;

fn parse<T: DeserializeOwned>(who: &str, v: &Value) -> Result<T, Problems> {
    serde_json::from_value(v.clone()).map_err(|e| vec![format!("{who}: {e}")])
}

fn lookups<T: DeserializeOwned>(label: &str, list: &[Value]) -> Result<Vec<T>, Problems> {
    list.iter().map(|v| parse(&format!("{label} {}", v.get("id").and_then(Value::as_str).unwrap_or("?")), v)).collect()
}

fn said(e: anyhow::Error) -> Problems {
    sentences(&e)
}

impl Request {
    /// the module descriptors it carries
    fn descriptors(&self) -> Result<Vec<Descriptor>, Problems> {
        self.modules.iter().map(|v| parse(&format!("module {}", v.get("name").and_then(Value::as_str).unwrap_or("?")), v)).collect()
    }

    /// a scene's module nodes as node kinds: the descriptors registered as a deck's library registers them; a node whose
    /// module has no descriptor here is named – the process-wide registry may know it from another request, the answer
    /// must not depend on that
    fn scene_modules(&self, doc: &crate::scene_document::SceneDocument, who: &str) -> Result<(), Problems> {
        self.node_modules(doc.nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str())), who)
    }

    /// #803: the module kinds these nodes (id, kind) name, from the descriptors in `modules` – each one there is registered,
    /// one that isn't is named; an instrument's and a preset's instrument's nodes as a scene's (never what another request
    /// registered in this process)
    fn node_modules<'a>(&self, nodes: impl Iterator<Item = (&'a str, &'a str)>, who: &str) -> Result<(), Problems> {
        let lib = Library::new(Vec::new(), Vec::new()).with_modules(self.descriptors()?);
        let missing: Problems = nodes
            .filter_map(|(id, kind)| {
                let m = kind.strip_prefix(crate::modreg::PREFIX)?;
                (!lib.modules.contains_key(m)).then(|| format!("{who}: node '{id}': module:{m} isn't in the catalog yet"))
            })
            .collect();
        if !missing.is_empty() {
            return Err(missing);
        }
        lib.register_modules();
        Ok(())
    }

    /// The document's problems (empty = valid); `Err` for a kind that is none of the three.
    pub fn problems(&self) -> Result<Vec<String>, String> {
        self.answer().map(|a| a.problems)
    }

    /// The document's problems and warnings; `Err` for a kind that is none of the three.
    pub fn answer(&self) -> Result<Answer, String> {
        let label = match self.kind.as_str() {
            "instruments" => "instrument",
            "presets" => "preset",
            "rigs" => "rig",
            "scenes" => "scene",
            "uses" => "use of scene",
            "device-presets" => "device preset",
            "rig-presets" => "rig preset",
            "spots" => "spots of asset",
            other => return Err(format!("unknown kind '{other}' (instruments, presets, rigs, scenes, uses, device-presets, rig-presets, spots)")),
        };
        let who = format!("{label} {}", self.id);
        let checked = match label {
            "instrument" => self.instrument(&who),
            "preset" => self.preset(&who).map(|_| Vec::new()),
            "scene" => self.scene(&who).map(|_| Vec::new()),
            "use of scene" => self.scene_use(&who).map(|_| Vec::new()),
            "device preset" => self.device_preset(&who).map(|_| Vec::new()),
            "rig preset" => self.rig_preset(&who).map(|_| Vec::new()),
            "spots of asset" => self.spots(&who).map(|_| Vec::new()),
            _ => self.rig(&who),
        };
        Ok(match checked {
            Ok(warnings) => Answer { problems: Vec::new(), warnings },
            Err(problems) => Answer { problems, warnings: Vec::new() },
        })
    }

    /// an instrument's problems, or its warnings
    fn instrument(&self, who: &str) -> Result<Vec<String>, Problems> {
        let i: Instrument = parse(who, &self.document)?;
        self.node_modules(i.nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str())), who)?;
        i.check().map_err(said)?;
        Ok(i.warnings())
    }

    fn preset(&self, who: &str) -> Result<(), Problems> {
        let p: Preset = parse(who, &self.document)?;
        let instruments: Vec<Instrument> = lookups("instrument", &self.instruments)?;
        let Some(instrument) = instruments.iter().find(|i| i.id == p.instrument) else {
            return Err(vec![format!("{who}: unknown instrument \"{}\"", p.instrument)]);
        };
        self.node_modules(instrument.nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str())), who)?;
        // the book as it will be: this preset counts with its new data
        let mut presets: Vec<Preset> = lookups::<Preset>("preset", &self.presets)?.into_iter().filter(|q| q.id != p.id).collect();
        presets.push(p.clone());
        let book = PresetBook::new(presets);
        let resolved = book.resolve(&p.id).map_err(said)?;
        instrument.patch(&resolved).map(|_| ()).map_err(said)
    }

    /// a device preset's problems
    fn device_preset(&self, who: &str) -> Result<(), Problems> {
        let p: crate::device_preset::DevicePreset = parse(who, &self.document)?;
        let mut problems: Problems = p.problems().into_iter().map(|s| format!("{who}: {s}")).collect();
        if p.id != self.id {
            problems.push(format!("{who}: the document's id is \"{}\", the key's \"{}\"", p.id, self.id));
        }
        if problems.is_empty() { Ok(()) } else { Err(problems) }
    }

    /// a rig preset's problems
    fn rig_preset(&self, who: &str) -> Result<(), Problems> {
        let p: super::RigPreset = parse(who, &self.document)?;
        let mut problems: Problems = p.problems().into_iter().map(|s| format!("{who}: {s}")).collect();
        if p.id != self.id {
            problems.push(format!("{who}: the document's id is \"{}\", the key's \"{}\"", p.id, self.id));
        }
        if problems.is_empty() { Ok(()) } else { Err(problems) }
    }

    /// a model's spots' problems; `modules` holds the descriptor of the module this model is the body of, when it is one
    fn spots(&self, who: &str) -> Result<(), Problems> {
        let spots: deck_catalog::Spots = parse(who, &self.document)?;
        let module: Option<Descriptor> = self.descriptors()?.into_iter().next();
        let problems: Problems = spots.problems(module.as_ref()).into_iter().map(|s| format!("{who}: {s}")).collect();
        if problems.is_empty() { Ok(()) } else { Err(problems) }
    }

    /// a scene document's problems (#243): what a deck refuses to load
    fn scene(&self, who: &str) -> Result<(), Problems> {
        let doc = crate::scene_document::SceneDocument::from_value(self.document.clone()).map_err(|e| vec![format!("{who}: {e:#}")])?;
        if doc.id != self.id {
            return Err(vec![format!("{who}: the document's id is \"{}\", the key's \"{}\"", doc.id, self.id)]);
        }
        self.scene_modules(&doc, who)?;
        let resolved = crate::scene_resolve::resolve(&doc, &Default::default(), true, &crate::scene_resolve::no_rigs, &crate::scene_resolve::no_sets)
            .map_err(|problems| problems.into_iter().map(|p| format!("{who}: {p}")).collect::<Vec<_>>())?;
        super::engine::lower(&resolved.scene).map_err(said)?;
        Ok(())
    }

    /// a use's problems (#504): `id` is its scene's id, `scenes` holds that scene's published document, `book` the toon's
    /// references – scene_resolve.rs `Use::problems` (in the book, each filler against its `fill`, the scene resolved)
    fn scene_use(&self, who: &str) -> Result<(), Problems> {
        let u: crate::scene_resolve::Use = parse(who, &self.document)?;
        let Some(doc) = self.scenes.iter().find(|s| s.get("id").and_then(Value::as_str) == Some(self.id.as_str())) else {
            return Err(vec![format!("{who}: no published scene:{} to play", self.id)]);
        };
        let doc = crate::scene_document::SceneDocument::from_value(doc.clone()).map_err(|e| vec![format!("{who}: {e:#}")])?;
        self.scene_modules(&doc, who)?;
        let problems = u.problems(&doc, self.book.as_deref());
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.into_iter().map(|p| format!("{who}: {p}")).collect())
        }
    }

    /// a rig's problems, or its warnings
    fn rig(&self, who: &str) -> Result<Vec<String>, Problems> {
        let rig: Rig = parse(who, &self.document)?;
        let instruments: Vec<Instrument> = lookups("instrument", &self.instruments)?;
        let presets: Vec<Preset> = lookups("preset", &self.presets)?;
        for i in &instruments {
            self.node_modules(i.nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str())), who)?;
        }
        let modules = self.descriptors()?;
        let lib = Library::new(instruments.clone(), presets.clone()).with_modules(modules.clone());
        rig.check(&lib).map_err(said)?;
        let warnings = rig.warnings(&lib).map_err(said)?;
        let personas: Vec<String> = rig.personas.keys().cloned().collect();
        let moods: Vec<String> = rig.moods.keys().cloned().collect();
        let bundle = Bundle { rig, instruments, presets, modules, rig_presets: Vec::new() };
        let variants = std::iter::once((None, None))
            .chain(personas.iter().map(|p| (Some(p.as_str()), None)))
            .chain(moods.iter().map(|m| (None, Some(m.as_str()))));
        for (persona, mood) in variants {
            bundle.play(persona, mood).map_err(said)?;
        }
        Ok(warnings)
    }
}

/// `Request` as JSON → `Answer` as JSON – the wasm boundary's and the build service's form
pub fn problems_json(request: &str) -> Result<String> {
    let request: Request = serde_json::from_str(request).context("not a validate request { kind, id, document, instruments?, presets?, modules? }")?;
    let answer = request.answer().map_err(anyhow::Error::msg)?;
    Ok(serde_json::to_string(&answer)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #803: an instrument (and a preset of it) with a `module:` node is checked with the descriptors the request sends –
    /// not with whatever another request registered in this process; without the descriptor the node is named
    #[test]
    fn an_instrument_with_a_module_node_takes_the_requests_descriptor() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/modules.json");
        let set: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let analog = set["cases"][0]["request"]["modules"][0].clone();
        assert_eq!(analog["name"], "analog");
        let synth = serde_json::json!({"id": "synth803", "kind": "voice", "name": {"en": "Synth"}, "out": "voice",
            "node": [{"id": "voice", "kind": "module:analog"}], "link": [], "expose": []});
        let ask = |kind: &str, document: Value, instruments: Vec<Value>, modules: Vec<Value>| {
            let id = document["id"].as_str().unwrap().to_owned();
            Request { kind: kind.into(), id, document, instruments, presets: vec![], modules, ..Default::default() }.problems().unwrap()
        };
        assert_eq!(ask("instruments", synth.clone(), vec![], vec![analog.clone()]), Vec::<String>::new());
        let without = ask("instruments", synth.clone(), vec![], vec![]);
        assert!(without.iter().any(|p| p.contains("module:analog isn't in the catalog yet")), "{without:?}");
        let preset = serde_json::json!({"id": "synth803.base", "instrument": "synth803", "values": {}});
        assert_eq!(ask("presets", preset, vec![synth], vec![analog]), Vec::<String>::new());
    }

    /// The fixture set (server/tests/fixtures/rigs/validate.json): the requests as the API builds them and the problems
    /// the retired PHP RigValidator gave on them – the core says the same words.
    #[test]
    fn the_core_words_the_fixture_set_as_the_php_validator_did() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/rigs/validate.json");
        let set: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let cases = set["cases"].as_array().unwrap();
        assert!(cases.len() >= 3);
        for case in cases {
            let mut request = case.clone();
            let o = request.as_object_mut().unwrap();
            let name = o.remove("name").unwrap();
            let expected: Vec<String> = serde_json::from_value(o.remove("problems").unwrap()).unwrap();
            let request: Request = serde_json::from_value(request).unwrap();
            assert_eq!(request.problems().unwrap(), expected, "{name}");
        }
    }

    #[test]
    fn forms_and_kinds() {
        let ask = |json: &str| problems_json(json).map(|a| serde_json::from_str::<Answer>(&a).unwrap().problems);
        let p = ask(r#"{"kind":"instruments","id":"x","document":{"id":"x","kind":"pad","node":[],"out":"o","color":"red"}}"#).unwrap();
        assert_eq!(p.len(), 1);
        assert!(p[0].starts_with("instrument x: unknown field `color`"), "{p:?}");
        let p = ask(r#"{"kind":"presets","id":"x.a","document":{"id":"x.a","instrument":"kazoo"}}"#).unwrap();
        assert_eq!(p, ["preset x.a: unknown instrument \"kazoo\""]);
        let p = ask(r#"{"kind":"rigs","id":"character.boy","document":{"subject":"character.boy","track":[{"id":"v","instrument":"kazoo"}]}}"#).unwrap();
        assert_eq!(p, ["rig character.boy: unknown instrument \"kazoo\""]);
        assert!(ask(r#"{"kind":"items","id":"x","document":{}}"#).is_err());
        assert!(ask(r#"{"kind":"rigs","id":"x"}"#).is_err(), "a document is required");
        let good = r#"{"kind":"device-presets","id":"mine","document":{"id":"mine","name":"Mine","hardware":{"channels":2},"outlets":[{"group":"main","channels":[1,2]}]}}"#;
        assert_eq!(ask(good).unwrap(), Vec::<String>::new());
        let p = ask(r#"{"kind":"device-presets","id":"other","document":{"id":"mine","name":"Mine","hardware":{"channels":2},"outlets":[{"group":"main","channels":[3]}]}}"#).unwrap();
        assert_eq!(p, ["device preset other: 'main': channel 3 is not on the device (it has 1–2)", "device preset other: the document's id is \"mine\", the key's \"other\""]);
        assert!(ask(r#"{"kind":"device-presets","id":"m","document":{"id":"m","name":"M","hardware":{},"colour":1}}"#).unwrap()[0].contains("unknown field `colour`"));
        let soft = r#"{"kind":"rig-presets","id":"character.catpope.soft","document":{"id":"character.catpope.soft","rig":"character.catpope","name":"Soft","origin":"character.catpope.warm","values":{"voice:voice.index":4.0}}}"#;
        assert_eq!(ask(soft).unwrap(), Vec::<String>::new());
        let p = ask(r#"{"kind":"rig-presets","id":"x","document":{"id":"character.catpope.soft","rig":"character.catpope","name":"Soft","values":{"voice.index":4.0}}}"#).unwrap();
        assert_eq!(p, ["rig preset x: value 'voice.index' is no knob of a track (<track>:<node>.<port>)", "rig preset x: the document's id is \"character.catpope.soft\", the key's \"x\""]);
    }

    /// #243: a scene document – the first real scene passes; what a deck could not load is refused with its reasons
    #[test]
    fn scenes() {
        let nave = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/nave.json")).unwrap();
        let ask = |id: &str, document: &str| Request { kind: "scenes".into(), id: id.into(), document: serde_json::from_str(document).unwrap(), instruments: vec![], presets: vec![], modules: vec![], ..Default::default() }.problems().unwrap();
        assert_eq!(ask("nave", &nave), Vec::<String>::new());
        let p = ask("other", &nave);
        assert_eq!(p, ["scene other: the document's id is \"nave\", the key's \"other\""]);
        let p = ask("x", r#"{"format":2,"id":"x","node":[{"id":"a","kind":"blender"},{"id":"t","kind":"tempo"}],"link":[{"from":"t.beat","to":"nowhere.in"}]}"#);
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(p[0].contains("unknown kind 'blender'") && p[1].contains("nowhere"), "{p:?}");
        let p = ask("x", r#"{"format":1,"id":"x"}"#);
        assert!(p[0].starts_with("scene x: format 1"), "format 1 is migrated, not read: {p:?}");
        let p = ask("x", r#"{"format":2,"id":"x","nodes":[]}"#);
        assert!(p[0].contains("unknown field `nodes`"), "{p:?}");
        // #489: a holder's problems and a preview its filter refuses
        let p = ask("x", r#"{"format":2,"id":"x","holder":[{"id":"who","fill":{"kind":["character"]},"preview":"item:incense"}]}"#);
        assert_eq!(p, ["scene x: holder who: its preview: 'item:incense' is an item, the placeholder takes character"]);
    }

    /// #489: the four templates (SCENES.md §9.5, server/tests/fixtures/scenes/templates) pass as published scenes
    #[test]
    fn the_templates_pass() {
        for name in ["place", "solo", "stage", "avatar-home"] {
            let text = std::fs::read_to_string(format!("{}/../../server/tests/fixtures/scenes/templates/{name}.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
            let r = Request { kind: "scenes".into(), id: name.into(), document: serde_json::from_str(&text).unwrap(), instruments: vec![], presets: vec![], modules: vec![], ..Default::default() };
            assert_eq!(r.problems().unwrap(), Vec::<String>::new(), "{name}");
        }
    }

    /// #504: the uses as the API sends them (server/tests/fixtures/scenes/uses.json) – the core's words for each; a good one
    /// passes, the others name their reason. `UPDATE_FIXTURES=1` writes the problems.
    #[test]
    fn uses_vectors() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/uses.json");
        let stored = std::fs::read_to_string(path).unwrap().replace('\r', "");
        let mut set: Value = serde_json::from_str(&stored).unwrap();
        for case in set["cases"].as_array_mut().unwrap() {
            let request: Request = serde_json::from_value(case["request"].clone()).unwrap();
            case["problems"] = serde_json::to_value(request.problems().unwrap()).unwrap();
        }
        let cases = set["cases"].as_array().unwrap();
        assert_eq!(cases[0]["problems"], serde_json::json!([]), "the good use passes");
        assert!(cases[1]["problems"][0].as_str().unwrap().contains("'item:incense' is an item, the placeholder takes character"));
        assert!(cases[1..].iter().all(|c| !c["problems"].as_array().unwrap().is_empty()), "every other case is refused");
        let written = format!("{}\n", serde_json::to_string_pretty(&set).unwrap());
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(path, &written).unwrap();
        } else {
            assert_eq!(stored, written, "uses.json is stale: UPDATE_FIXTURES=1 cargo test -p deck-space uses_vectors");
        }
        let wire: Value = serde_json::from_str(&problems_json(r#"{"kind":"uses","id":"stage","document":{"scene":"scene:stage","extra":1}}"#).unwrap()).unwrap();
        assert!(wire["problems"][0].as_str().unwrap().contains("unknown field `extra`"), "{wire}");
    }

    /// a scene's module nodes as the API sends them (server/tests/fixtures/scenes/modules.json) – with the module's
    /// descriptor it passes, without it the node and the module are named, even once the process knows the module.
    /// `UPDATE_FIXTURES=1` writes the problems.
    #[test]
    fn scene_modules_vectors() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/modules.json");
        let stored = std::fs::read_to_string(path).unwrap().replace('\r', "");
        let mut set: Value = serde_json::from_str(&stored).unwrap();
        for case in set["cases"].as_array_mut().unwrap() {
            let request: Request = serde_json::from_value(case["request"].clone()).unwrap();
            case["problems"] = serde_json::to_value(request.problems().unwrap()).unwrap();
        }
        let problems = |i: usize| serde_json::from_value::<Vec<String>>(set["cases"][i]["problems"].clone()).unwrap();
        assert_eq!(problems(0), Vec::<String>::new(), "module:analog with its descriptor passes");
        assert_eq!(problems(1), ["scene rave: node 'voice': module:analog isn't in the catalog yet"], "registered by case 0, still refused");
        assert_eq!(problems(2), ["scene rave: node 'arp': module:arp isn't in the catalog yet"]);
        let written = format!("{}\n", serde_json::to_string_pretty(&set).unwrap());
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(path, &written).unwrap();
        } else {
            assert_eq!(stored, written, "modules.json is stale: UPDATE_FIXTURES=1 cargo test -p deck-space scene_modules_vectors");
        }
    }

    /// A model's spots as the API sends them (server/tests/fixtures/assets/spots.json) – the core's words for each; the
    /// good ones pass, the others name their reason. `UPDATE_FIXTURES=1` writes the problems.
    #[test]
    fn spots_vectors() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/assets/spots.json");
        let stored = std::fs::read_to_string(path).unwrap().replace('\r', "");
        let mut set: Value = serde_json::from_str(&stored).unwrap();
        for case in set["cases"].as_array_mut().unwrap() {
            let request: Request = serde_json::from_value(case["request"].clone()).unwrap();
            case["problems"] = serde_json::to_value(request.problems().unwrap()).unwrap();
        }
        let cases = set["cases"].as_array().unwrap();
        let said = |name: &str| cases.iter().find(|c| c["name"] == name).unwrap_or_else(|| panic!("no case {name}"))["problems"].to_string();
        assert_eq!(said("a character's spots: the file's and two hand edits"), "[]");
        assert_eq!(said("a module body whose port spots match"), "[]");
        assert!(said("a bad name").contains("is no spot name"));
        assert!(said("a missing bone").contains("the model has no bone 'wrist_l'"));
        assert!(said("a port mismatch").contains("renamed?"));
        let written = format!("{}\n", serde_json::to_string_pretty(&set).unwrap());
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(path, &written).unwrap();
        } else {
            assert_eq!(stored, written, "spots.json is stale: UPDATE_FIXTURES=1 cargo test -p deck-space spots_vectors");
        }
    }

    fn v<T: Serialize>(x: &T) -> Value {
        serde_json::to_value(x).unwrap()
    }

    /// The core rigs (deck/native/rigs) pass as catalog documents
    #[test]
    fn the_core_rigs_pass() {
        for (subject, _) in super::super::bundle::CORE {
            let b = Bundle::core(subject).unwrap().unwrap();
            let request = Request {
                kind: "rigs".into(),
                id: subject.into(),
                document: v(&b.rig),
                instruments: b.instruments.iter().map(v).collect(),
                presets: b.presets.iter().map(v).collect(),
                modules: vec![],
                ..Default::default()
            };
            assert_eq!(request.problems().unwrap(), Vec::<String>::new(), "{subject}");
            for i in &b.instruments {
                let r = Request { kind: "instruments".into(), id: i.id.clone(), document: v(i), instruments: vec![], presets: vec![], modules: vec![], ..Default::default() };
                assert_eq!(r.problems().unwrap(), Vec::<String>::new(), "{}", i.id);
            }
            for p in &b.presets {
                let others = b.presets.iter().filter(|q| q.id != p.id).map(v).collect();
                let r = Request { kind: "presets".into(), id: p.id.clone(), document: v(p), instruments: b.instruments.iter().map(v).collect(), presets: others, modules: vec![], ..Default::default() };
                assert_eq!(r.problems().unwrap(), Vec::<String>::new(), "{}", p.id);
            }
        }
    }
}
