//! The assignment (#283, research/RIG-EDITING.md §2 tab 4, §6): which character plays which rig – what the creator's
//! Assignment tab and its list of characters ask the core (space-web `rig_coverage`, `rig_catalog_bundle`):
//! - `coverage`: every character with the state of its default rig (`character.<id>`) – it plays (it is in the catalog,
//!   what scene-content serves), it is only drafted, or it is missing – and the personas' own rigs
//!   (`character.<id>.<persona>`, which scene-content prefers);
//! - `catalog_bundle`: a rig document with what it names picked from the catalog's instruments and presets (the presets'
//!   ancestors included) – the bundle `rig_play` plays, so a draft is heard before it is published.
//!
//! The standard rig per instrument kind (§9.4, the fallback of a character without a default rig) does not exist yet:
//! until it does, a deck without a rig plays its own scene (`Played::deck_scene`). A code rig (#314) is a policy module
//! over the same document (DECISIONS 2026-11-03 [rigs]); the form edits the document, the code drives its parameters.
use super::{Bundle, Instrument, Preset, PresetBook, Rig};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// A character as the registry has it: its id and its personas' ids.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Character {
    pub id: String,
    #[serde(default)]
    pub personas: Vec<String>,
}

/// A stored rig as the catalog lists it (GET /api/catalog/docs/rigs): its subject and its state.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RigState {
    /// the subject (`character.<id>[.<persona>]`, `item.<id>`)
    pub id: String,
    /// in the catalog table – what scene-content serves, so a deck plays it
    #[serde(default)]
    pub live: bool,
    /// a newer version than the published one is waiting
    #[serde(default)]
    pub draft: bool,
    /// the newest version is a removal
    #[serde(default)]
    pub removed: bool,
}

/// How a subject's rig stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// published: decks play it
    Plays,
    /// only a draft: the creator hears it, decks do not
    Drafted,
    Missing,
}

impl Status {
    fn of(rig: Option<&RigState>) -> Status {
        match rig {
            Some(r) if r.live => Status::Plays,
            Some(r) if r.draft && !r.removed => Status::Drafted,
            _ => Status::Missing,
        }
    }
}

/// One character's assignment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Coverage {
    pub character: String,
    /// the default rig's subject, `character.<id>`
    pub subject: String,
    pub default: Status,
    /// the personas with a rig of their own (drafted or playing) and its state
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub personas: BTreeMap<String, Status>,
    /// no published default rig: a deck plays the fallback for it (today the deck's own scene, §9.4)
    pub missing: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageRequest {
    characters: Vec<Character>,
    #[serde(default)]
    rigs: Vec<RigState>,
}

/// Every character's assignment, in the order given.
pub fn coverage(characters: &[Character], rigs: &[RigState]) -> Vec<Coverage> {
    let by_id: BTreeMap<&str, &RigState> = rigs.iter().map(|r| (r.id.as_str(), r)).collect();
    characters
        .iter()
        .map(|c| {
            let subject = format!("character.{}", c.id);
            let default = Status::of(by_id.get(subject.as_str()).copied());
            let personas = c
                .personas
                .iter()
                .map(|p| (p.clone(), Status::of(by_id.get(format!("{subject}.{p}").as_str()).copied())))
                .filter(|(_, s)| *s != Status::Missing)
                .collect();
            Coverage { character: c.id.clone(), missing: default != Status::Plays, subject, default, personas }
        })
        .collect()
}

/// `{ characters: [{ id, personas }], rigs: [{ id, live, draft, removed }] }` → `[Coverage]` as JSON (the wasm boundary)
pub fn coverage_json(request: &str) -> Result<String> {
    let r: CoverageRequest = serde_json::from_str(request).context("not a coverage request { characters, rigs }")?;
    Ok(serde_json::to_string(&coverage(&r.characters, &r.rigs))?)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleRequest {
    rig: Value,
    #[serde(default)]
    instruments: Vec<Value>,
    #[serde(default)]
    presets: Vec<Value>,
}

fn id_of(v: &Value) -> Option<&str> {
    v.get("id").and_then(Value::as_str)
}

/// The rig with what it names from the catalog: its tracks' instruments, the presets its tracks, personas and moods name
/// with their ancestors. Only what it names is read – a broken entry it does not name is not its problem.
pub fn catalog_bundle(rig: Rig, instruments: &[Value], presets: &[Value]) -> Result<Bundle> {
    let mut picked: Vec<Instrument> = Vec::new();
    for t in &rig.tracks {
        if picked.iter().any(|i| i.id == t.instrument) {
            continue;
        }
        let Some(v) = instruments.iter().find(|v| id_of(v) == Some(t.instrument.as_str())) else {
            bail!("rig {}: the instrument \"{}\" is not in the catalog", rig.subject, t.instrument)
        };
        picked.push(serde_json::from_value(v.clone()).with_context(|| format!("instrument {}", t.instrument))?);
    }
    // the book of every preset that reads; a named one that does not read says why
    let book = PresetBook::new(presets.iter().filter_map(|v| serde_json::from_value::<Preset>(v.clone()).ok()));
    let named = rig
        .tracks
        .iter()
        .filter_map(|t| t.preset.as_ref())
        .chain(rig.personas.values().chain(rig.moods.values()).flat_map(|v| v.presets.values()));
    let mut chosen: BTreeMap<String, Preset> = BTreeMap::new();
    for id in named {
        if !book.presets.contains_key(id) {
            match presets.iter().find(|v| id_of(v) == Some(id.as_str())) {
                Some(v) => serde_json::from_value::<Preset>(v.clone()).map(|_| ()).with_context(|| format!("preset {id}"))?,
                None => bail!("rig {}: the preset \"{id}\" is not in the catalog", rig.subject),
            }
        }
        for p in book.lineage(id)? {
            chosen.entry(p.id.clone()).or_insert_with(|| p.clone());
        }
    }
    // a code rig's policy module (#408) is not looked up here yet: its descriptor and its preset choices come with T6
    Ok(Bundle { rig, instruments: picked, presets: chosen.into_values().collect(), modules: Vec::new() })
}

/// `{ rig, instruments: [..], presets: [..] }` (the catalog's documents) → the bundle as JSON for `rig_play`
pub fn catalog_bundle_json(request: &str) -> Result<String> {
    let r: BundleRequest = serde_json::from_str(request).context("not a bundle request { rig, instruments, presets }")?;
    let rig: Rig = serde_json::from_value(r.rig).context("the rig")?;
    Ok(serde_json::to_string(&catalog_bundle(rig, &r.instruments, &r.presets)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rig(id: &str, live: bool, draft: bool, removed: bool) -> RigState {
        RigState { id: id.into(), live, draft, removed }
    }

    #[test]
    fn a_character_without_a_published_default_rig_is_missing() {
        let characters = [
            Character { id: "catpope".into(), personas: vec!["regular".into(), "dj".into()] },
            Character { id: "boy".into(), personas: vec!["regular".into()] },
            Character { id: "plf".into(), personas: vec![] },
            Character { id: "gone".into(), personas: vec![] },
        ];
        let rigs = [
            rig("character.catpope", true, true, false),
            rig("character.catpope.dj", false, true, false),
            rig("character.boy", false, true, false),
            rig("character.gone", false, true, true),
            rig("item.incense_burner", true, false, false),
        ];
        let c = coverage(&characters, &rigs);
        assert_eq!(c.iter().map(|c| (c.character.as_str(), c.default, c.missing)).collect::<Vec<_>>(), [
            ("catpope", Status::Plays, false),
            ("boy", Status::Drafted, true),
            ("plf", Status::Missing, true),
            ("gone", Status::Missing, true),
        ]);
        assert_eq!(c[0].subject, "character.catpope");
        assert_eq!(c[0].personas, BTreeMap::from([("dj".to_string(), Status::Drafted)]), "only personas with a rig of their own");
        // a published rig whose removal waits plays until the removal is published
        assert_eq!(coverage(&characters[3..], &[rig("character.gone", true, true, true)])[0].default, Status::Plays);
    }

    #[test]
    fn coverage_json_reads_the_catalog_list_form() {
        let out = coverage_json(r#"{"characters":[{"id":"catpope","personas":["regular"]},{"id":"boy"}],"rigs":[{"id":"character.catpope","live":true,"published":12}]}"#)
            .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v, json!([
            { "character": "catpope", "subject": "character.catpope", "default": "plays", "missing": false },
            { "character": "boy", "subject": "character.boy", "default": "missing", "missing": true },
        ]));
        assert!(coverage_json(r#"{"rigs":[]}"#).is_err(), "characters are required");
    }

    fn v<T: Serialize>(x: &T) -> Value {
        serde_json::to_value(x).unwrap()
    }

    /// the core rig of catpope as catalog documents
    fn catpope() -> (Value, Vec<Value>, Vec<Value>) {
        let b = Bundle::core("character.catpope").unwrap().unwrap();
        (v(&b.rig), b.instruments.iter().map(v).collect(), b.presets.iter().map(v).collect())
    }

    #[test]
    fn a_catalog_bundle_holds_what_the_rig_names_and_plays() {
        let (rig_doc, mut instruments, mut presets) = catpope();
        // catalog entries the rig does not name, one of them broken: not its problem
        instruments.push(json!({ "id": "kazoo", "kind": "lead", "color": "red" }));
        presets.push(json!({ "id": "kazoo.base", "instrument": "kazoo", "values": "loud" }));
        let request = json!({ "rig": rig_doc, "instruments": instruments, "presets": presets }).to_string();
        let b = Bundle::parse_json(&catalog_bundle_json(&request).unwrap()).unwrap();
        assert_eq!(b.instruments.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), ["cathedral_voice", "drone"]);
        assert!(b.presets.iter().all(|p| p.id != "kazoo.base"));
        let core = Bundle::core("character.catpope").unwrap().unwrap();
        let mut want: Vec<&str> = core.presets.iter().map(|p| p.id.as_str()).collect();
        want.sort();
        assert_eq!(b.presets.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), want, "every named preset with its ancestors");
        for mood in [None, Some("happy"), Some("sad")] {
            assert_eq!(b.play(None, mood).unwrap(), core.play(None, mood).unwrap(), "{mood:?}");
        }
    }

    #[test]
    fn a_catalog_bundle_names_what_is_missing_or_broken() {
        let (rig_doc, instruments, presets) = catpope();
        let ask = |rig: &Value, instruments: &[Value], presets: &[Value]| {
            catalog_bundle_json(&json!({ "rig": rig, "instruments": instruments, "presets": presets }).to_string()).map_err(|e| format!("{e:#}"))
        };
        let e = ask(&rig_doc, &instruments[1..], &presets).unwrap_err();
        assert_eq!(e, "rig character.catpope: the instrument \"cathedral_voice\" is not in the catalog");
        let mut broken = presets.clone();
        let at = broken.iter().position(|p| id_of(p) == Some("cathedral_voice.bright")).unwrap();
        broken[at]["values"] = json!("loud");
        assert!(ask(&rig_doc, &instruments, &broken).unwrap_err().starts_with("preset cathedral_voice.bright: "));
        let orphan: Vec<Value> = presets.iter().filter(|p| id_of(p) != Some("cathedral_voice.bright")).cloned().collect();
        assert_eq!(ask(&rig_doc, &instruments, &orphan).unwrap_err(), "rig character.catpope: the preset \"cathedral_voice.bright\" is not in the catalog");
        assert!(ask(&json!({ "subject": "character.x" }), &[], &[]).unwrap_err().starts_with("the rig"));
    }
}
