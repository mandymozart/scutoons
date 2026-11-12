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
//!   does before it plays a scene. The outputs table is checked as a shape only here (the API's JSON Schema).
//!
//! The lookups travel by value: the instruments it names, the presets it names with their ancestors, a code rig's policy
//! module's descriptor (the API reads them from its catalog). Every problem a check lists is in the answer, each naming
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// the catalog kind: `instruments` · `presets` · `rigs` · `scenes` (#243: a `scene:<id>` document, not catalog)
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
    /// a code rig's policy module: the descriptor of the `module:<id>` document it names, by value (#408)
    #[serde(default)]
    pub modules: Vec<Value>,
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
            other => return Err(format!("unknown kind '{other}' (instruments, presets, rigs, scenes)")),
        };
        let who = format!("{label} {}", self.id);
        let checked = match label {
            "instrument" => self.instrument(&who),
            "preset" => self.preset(&who).map(|_| Vec::new()),
            "scene" => self.scene(&who).map(|_| Vec::new()),
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
        i.check().map_err(said)?;
        Ok(i.warnings())
    }

    fn preset(&self, who: &str) -> Result<(), Problems> {
        let p: Preset = parse(who, &self.document)?;
        let instruments: Vec<Instrument> = lookups("instrument", &self.instruments)?;
        let Some(instrument) = instruments.iter().find(|i| i.id == p.instrument) else {
            return Err(vec![format!("{who}: unknown instrument \"{}\"", p.instrument)]);
        };
        // the book as it will be: this preset counts with its new data
        let mut presets: Vec<Preset> = lookups::<Preset>("preset", &self.presets)?.into_iter().filter(|q| q.id != p.id).collect();
        presets.push(p.clone());
        let book = PresetBook::new(presets);
        let resolved = book.resolve(&p.id).map_err(said)?;
        instrument.patch(&resolved).map(|_| ()).map_err(said)
    }

    /// a scene document's problems (#243): what a deck refuses to load
    fn scene(&self, who: &str) -> Result<(), Problems> {
        let doc = crate::scene_document::SceneDocument::from_value(self.document.clone()).map_err(|e| vec![format!("{who}: {e:#}")])?;
        if doc.id != self.id {
            return Err(vec![format!("{who}: the document's id is \"{}\", the key's \"{}\"", doc.id, self.id)]);
        }
        let resolved = crate::scene_resolve::resolve(&doc, &Default::default(), true, &crate::scene_resolve::no_rigs)
            .map_err(|problems| problems.into_iter().map(|p| format!("{who}: {p}")).collect::<Vec<_>>())?;
        super::engine::lower(&resolved.scene).map_err(said)?;
        Ok(())
    }

    /// a rig's problems, or its warnings
    fn rig(&self, who: &str) -> Result<Vec<String>, Problems> {
        let rig: Rig = parse(who, &self.document)?;
        let instruments: Vec<Instrument> = lookups("instrument", &self.instruments)?;
        let presets: Vec<Preset> = lookups("preset", &self.presets)?;
        let modules: Vec<Descriptor> =
            self.modules.iter().map(|v| parse(&format!("module {}", v.get("name").and_then(Value::as_str).unwrap_or("?")), v)).collect::<Result<_, _>>()?;
        let lib = Library::new(instruments.clone(), presets.clone()).with_modules(modules.clone());
        rig.check(&lib).map_err(said)?;
        let warnings = rig.warnings(&lib).map_err(said)?;
        let personas: Vec<String> = rig.personas.keys().cloned().collect();
        let moods: Vec<String> = rig.moods.keys().cloned().collect();
        let bundle = Bundle { rig, instruments, presets, modules };
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
    }

    /// #243: a scene document – the first real scene passes; what a deck could not load is refused with its reasons
    #[test]
    fn scenes() {
        let nave = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/nave.json")).unwrap();
        let ask = |id: &str, document: &str| Request { kind: "scenes".into(), id: id.into(), document: serde_json::from_str(document).unwrap(), instruments: vec![], presets: vec![], modules: vec![] }.problems().unwrap();
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
            let r = Request { kind: "scenes".into(), id: name.into(), document: serde_json::from_str(&text).unwrap(), instruments: vec![], presets: vec![], modules: vec![] };
            assert_eq!(r.problems().unwrap(), Vec::<String>::new(), "{name}");
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
            };
            assert_eq!(request.problems().unwrap(), Vec::<String>::new(), "{subject}");
            for i in &b.instruments {
                let r = Request { kind: "instruments".into(), id: i.id.clone(), document: v(i), instruments: vec![], presets: vec![], modules: vec![] };
                assert_eq!(r.problems().unwrap(), Vec::<String>::new(), "{}", i.id);
            }
            for p in &b.presets {
                let others = b.presets.iter().filter(|q| q.id != p.id).map(v).collect();
                let r = Request { kind: "presets".into(), id: p.id.clone(), document: v(p), instruments: b.instruments.iter().map(v).collect(), presets: others, modules: vec![] };
                assert_eq!(r.problems().unwrap(), Vec::<String>::new(), "{}", p.id);
            }
        }
    }
}
