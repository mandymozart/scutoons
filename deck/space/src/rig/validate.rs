//! The checks of an instrument, a preset or a rig as the catalog stores them (#313): what the API asks before a write to
//! the catalog (deck/build `POST /validate`) and what the creator's wasm can ask as a hint on a draft (space-web
//! `rig_problems`) – the code the decks play with, no second copy: serde's forms (unknown fields refused), then
//! - an instrument: `Instrument::check`;
//! - a preset: its instrument known, `PresetBook::lineage` / `resolve` with this preset in the book, `Instrument::patch`;
//! - a rig: `Rig::check`, then `Bundle::play` (flatten, the knobs and sticks, the engine's lowering) without a variant,
//!   for every persona and for every mood.
//!
//! The lookups travel by value: the instruments it names, the presets it names with their ancestors (the API reads them
//! from its catalog). The core stops at the first problem, so the answer holds at most one.
use super::bundle::Bundle;
use super::{Instrument, Library, Preset, PresetBook, Rig};
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One document to check with what it names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// the catalog kind: `instruments` · `presets` · `rigs`
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
}

/// The answer: the problems, none = valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    pub problems: Vec<String>,
}

fn parse<T: DeserializeOwned>(who: &str, v: &Value) -> Result<T, String> {
    serde_json::from_value(v.clone()).map_err(|e| format!("{who}: {e}"))
}

fn lookups<T: DeserializeOwned>(label: &str, list: &[Value]) -> Result<Vec<T>, String> {
    list.iter().map(|v| parse(&format!("{label} {}", v.get("id").and_then(Value::as_str).unwrap_or("?")), v)).collect()
}

fn said(e: anyhow::Error) -> String {
    format!("{e:#}")
}

impl Request {
    /// The document's problems (empty = valid); `Err` for a kind that is none of the three.
    pub fn problems(&self) -> Result<Vec<String>, String> {
        let label = match self.kind.as_str() {
            "instruments" => "instrument",
            "presets" => "preset",
            "rigs" => "rig",
            other => return Err(format!("unknown kind '{other}' (instruments, presets, rigs)")),
        };
        let who = format!("{label} {}", self.id);
        let checked = match label {
            "instrument" => self.instrument(&who),
            "preset" => self.preset(&who),
            _ => self.rig(&who),
        };
        Ok(checked.err().into_iter().collect())
    }

    fn instrument(&self, who: &str) -> Result<(), String> {
        let i: Instrument = parse(who, &self.document)?;
        i.check().map(|_| ()).map_err(said)
    }

    fn preset(&self, who: &str) -> Result<(), String> {
        let p: Preset = parse(who, &self.document)?;
        let instruments: Vec<Instrument> = lookups("instrument", &self.instruments)?;
        let Some(instrument) = instruments.iter().find(|i| i.id == p.instrument) else {
            return Err(format!("{who}: unknown instrument \"{}\"", p.instrument));
        };
        // the book as it will be: this preset counts with its new data
        let mut presets: Vec<Preset> = lookups::<Preset>("preset", &self.presets)?.into_iter().filter(|q| q.id != p.id).collect();
        presets.push(p.clone());
        let book = PresetBook::new(presets);
        let resolved = book.resolve(&p.id).map_err(said)?;
        instrument.patch(&resolved).map(|_| ()).map_err(said)
    }

    fn rig(&self, who: &str) -> Result<(), String> {
        let rig: Rig = parse(who, &self.document)?;
        let instruments: Vec<Instrument> = lookups("instrument", &self.instruments)?;
        let presets: Vec<Preset> = lookups("preset", &self.presets)?;
        rig.check(&Library::new(instruments.clone(), presets.clone())).map_err(said)?;
        let personas: Vec<String> = rig.personas.keys().cloned().collect();
        let moods: Vec<String> = rig.moods.keys().cloned().collect();
        let bundle = Bundle { rig, instruments, presets };
        let variants = std::iter::once((None, None))
            .chain(personas.iter().map(|p| (Some(p.as_str()), None)))
            .chain(moods.iter().map(|m| (None, Some(m.as_str()))));
        for (persona, mood) in variants {
            bundle.play(persona, mood).map_err(said)?;
        }
        Ok(())
    }
}

/// `Request` as JSON → `Answer` as JSON – the wasm boundary's and the build service's form
pub fn problems_json(request: &str) -> Result<String> {
    let request: Request = serde_json::from_str(request).context("not a validate request { kind, id, document, instruments?, presets? }")?;
    let problems = request.problems().map_err(anyhow::Error::msg)?;
    Ok(serde_json::to_string(&Answer { problems })?)
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
                instruments: b.instruments.iter().map(|i| v(i)).collect(),
                presets: b.presets.iter().map(|p| v(p)).collect(),
            };
            assert_eq!(request.problems().unwrap(), Vec::<String>::new(), "{subject}");
            for i in &b.instruments {
                let r = Request { kind: "instruments".into(), id: i.id.clone(), document: v(i), instruments: vec![], presets: vec![] };
                assert_eq!(r.problems().unwrap(), Vec::<String>::new(), "{}", i.id);
            }
            for p in &b.presets {
                let others = b.presets.iter().filter(|q| q.id != p.id).map(|q| v(q)).collect();
                let r = Request { kind: "presets".into(), id: p.id.clone(), document: v(p), instruments: b.instruments.iter().map(|i| v(i)).collect(), presets: others };
                assert_eq!(r.problems().unwrap(), Vec::<String>::new(), "{}", p.id);
            }
        }
    }
}
