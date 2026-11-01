//! scene-content's rigs (#271, research/INSTRUMENTS.md §6): what `GET /api/library/scene-content` answers after #258 –
//! per character and per item a `rig` `{ id, version, data, instruments, presets }`, and the top-level `instruments` /
//! `presets` maps holding every referenced one with its ancestors, each `{ id, version, data }` (`version` = a hash of
//! the stored data; a deck's cache skips what it has). `SceneRigs` reads that answer (the sets and portraits are left
//! alone), `Bundle::from_scene_content` picks one subject's bundle out of it – the same `Bundle` a core TOML gives.
//! Platform-free: the native stage (deck-stage sync.rs) and the browser (space-web) read the same answer with it.
use super::{Bundle, Instrument, Preset, Rig};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// an instrument or preset as the API serves and the deck's cache keeps it
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Versioned<T> {
    pub id: String,
    pub version: String,
    pub data: T,
}

/// a character's or item's rig in the answer: the rig and the ids it needs (ancestors included, root first)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RigEntry {
    pub id: String,
    pub version: String,
    pub data: Rig,
    #[serde(default)]
    pub instruments: Vec<String>,
    #[serde(default)]
    pub presets: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Rigged {
    #[serde(default)]
    pub rig: Option<RigEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct MissingRigs {
    #[serde(default)]
    pub characters: Vec<String>,
    #[serde(default)]
    pub items: Vec<String>,
}

/// The rig part of scene-content's answer (the rest – sets, portraits – is ignored here).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct SceneRigs {
    #[serde(default)]
    pub characters: BTreeMap<String, Rigged>,
    #[serde(default)]
    pub items: BTreeMap<String, Rigged>,
    #[serde(default)]
    pub instruments: BTreeMap<String, Versioned<Instrument>>,
    #[serde(default)]
    pub presets: BTreeMap<String, Versioned<Preset>>,
    #[serde(default)]
    pub missing: MissingRigs,
}

/// `character.<id>` → the character's key, `item.<id>` → the item's; None for anything else
pub fn split_subject(subject: &str) -> Option<(bool, &str)> {
    if let Some(c) = subject.strip_prefix("character.") {
        // `character.<id>.<persona>` is the persona's own rig – it hangs on the character's entry
        return Some((true, c.split('.').next().unwrap_or(c)));
    }
    subject.strip_prefix("item.").map(|i| (false, i))
}

impl SceneRigs {
    pub fn parse(json: &str) -> Result<SceneRigs> {
        serde_json::from_str(json).context("scene-content is not an answer with rigs")
    }

    /// the rig for `subject`: `character.<id>` = the answer's character `<id>` (its rig may be the persona's own,
    /// `character.<id>.<persona>`), `item.<id>` = the item; None when the answer has none
    pub fn rig(&self, subject: &str) -> Option<&RigEntry> {
        let (character, key) = split_subject(subject)?;
        let map = if character { &self.characters } else { &self.items };
        map.get(key).and_then(|r| r.rig.as_ref()).or_else(|| map.values().filter_map(|r| r.rig.as_ref()).find(|r| r.id == subject))
    }

    /// is `subject` one the API said it has nothing for (`missing`, or a character without a rig)?
    pub fn names(&self, subject: &str) -> bool {
        match split_subject(subject) {
            Some((true, c)) => self.characters.contains_key(c) || self.missing.characters.iter().any(|m| m == c),
            Some((false, i)) => self.items.contains_key(i) || self.missing.items.iter().any(|m| m == i),
            None => false,
        }
    }

    /// every rig in the answer (characters first, then items)
    pub fn rigs(&self) -> impl Iterator<Item = &RigEntry> {
        self.characters.values().chain(self.items.values()).filter_map(|r| r.rig.as_ref())
    }

    /// `entry`'s instruments and presets out of the maps – every id it names must be there
    pub fn bundle_of(&self, entry: &RigEntry) -> Result<Bundle> {
        let mut instruments = Vec::with_capacity(entry.instruments.len());
        for id in &entry.instruments {
            instruments.push(self.instruments.get(id).with_context(|| format!("rig '{}' names instrument '{id}', the answer has none", entry.id))?.data.clone());
        }
        let mut presets = Vec::with_capacity(entry.presets.len());
        for id in &entry.presets {
            presets.push(self.presets.get(id).with_context(|| format!("rig '{}' names preset '{id}', the answer has none", entry.id))?.data.clone());
        }
        Ok(Bundle { rig: entry.data.clone(), instruments, presets })
    }

    /// `subject`'s bundle: an error when the answer has no rig for it
    pub fn bundle(&self, subject: &str) -> Result<Bundle> {
        match self.rig(subject) {
            Some(e) => self.bundle_of(e),
            None => bail!("scene-content has no rig for '{subject}'"),
        }
    }
}

impl Bundle {
    /// A subject's bundle out of scene-content's answer (`characters.<id>.rig` / `items.<id>.rig` with the top-level
    /// `instruments` / `presets`): `character.<id>` or `item.<id>`. An error when the answer has no rig for it or lacks
    /// something the rig names.
    pub fn from_scene_content(json: &str, subject: &str) -> Result<Bundle> {
        SceneRigs::parse(json)?.bundle(subject)
    }

    /// `from_scene_content` as JSON – the wasm boundary's form; empty when the answer has no rig for `subject`
    pub fn from_scene_content_json(json: &str, subject: &str) -> Result<String> {
        let rigs = SceneRigs::parse(json)?;
        match rigs.rig(subject) {
            Some(e) => Ok(serde_json::to_string(&rigs.bundle_of(e)?)?),
            None => Ok(String::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::bundle::CORE;

    /// the server's contract fixture (SceneContentRigsTest writes it; `the_servers_scene_content_reads_as_bundles` checks
    /// every rig in it plays)
    pub fn fixture() -> String {
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/rigs/scene-content.json")).unwrap()
    }

    fn sorted(mut b: Bundle) -> Bundle {
        b.instruments.sort_by(|a, b| a.id.cmp(&b.id));
        b.presets.sort_by(|a, b| a.id.cmp(&b.id));
        b
    }

    #[test]
    fn the_servers_answer_gives_every_core_rig_s_bundle() {
        let json = fixture();
        for (subject, _) in CORE {
            let b = Bundle::from_scene_content(&json, subject).unwrap_or_else(|e| panic!("{subject}: {e:#}"));
            assert_eq!(sorted(b), sorted(Bundle::core(subject).unwrap().unwrap()), "{subject}: the catalog's rig = the image's");
        }
        let rigs = SceneRigs::parse(&json).unwrap();
        let e = rigs.rig("character.catpope").unwrap();
        assert_eq!(e.id, "character.catpope");
        assert_eq!(e.version.len(), 16, "a version for the cache");
        assert_eq!(e.presets[0], "cathedral_voice.base", "ancestors first");
        assert!(e.presets.iter().all(|p| rigs.presets[p].version.len() == 16));
        assert_eq!(rigs.rigs().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["character.catpope", "item.incense_burner"]);
        let as_json = Bundle::from_scene_content_json(&json, "item.incense_burner").unwrap();
        assert_eq!(sorted(Bundle::parse_json(&as_json).unwrap()), sorted(Bundle::core("item.incense_burner").unwrap().unwrap()));
        assert_eq!(Bundle::from_scene_content_json(&json, "character.boy").unwrap(), "", "a character without a rig");
    }

    #[test]
    fn a_subject_the_answer_has_no_rig_for_is_an_error() {
        let json = fixture();
        let rigs = SceneRigs::parse(&json).unwrap();
        assert!(rigs.rig("character.boy").is_none() && rigs.names("character.boy"), "boy is answered, without a rig");
        assert!(rigs.rig("character.ghost").is_none() && !rigs.names("character.ghost"), "ghost wasn't asked for");
        assert!(rigs.rig("item.ghost").is_none());
        assert_eq!(split_subject("character.catpope.regular"), Some((true, "catpope")), "a persona's own rig hangs on its character");
        assert_eq!(split_subject("set.cathedral"), None);
        let e = Bundle::from_scene_content(&json, "character.boy").unwrap_err().to_string();
        assert!(e.contains("no rig for 'character.boy'"), "{e}");
        // the persona's own rig id is found through its character
        let mut persona: serde_json::Value = serde_json::from_str(&json).unwrap();
        persona["characters"]["catpope"]["rig"]["id"] = "character.catpope.regular".into();
        let rigs = SceneRigs::parse(&persona.to_string()).unwrap();
        assert_eq!(rigs.rig("character.catpope").unwrap().id, "character.catpope.regular");
        assert_eq!(rigs.rig("character.catpope.regular").unwrap().id, "character.catpope.regular");
    }

    #[test]
    fn a_rig_naming_what_the_answer_lacks_is_refused() {
        let mut v: serde_json::Value = serde_json::from_str(&fixture()).unwrap();
        v["presets"].as_object_mut().unwrap().remove("cathedral_voice.bright");
        let e = format!("{:#}", Bundle::from_scene_content(&v.to_string(), "character.catpope").unwrap_err());
        assert!(e.contains("names preset 'cathedral_voice.bright'"), "{e}");
        let mut v: serde_json::Value = serde_json::from_str(&fixture()).unwrap();
        v["instruments"].as_object_mut().unwrap().remove("drone");
        let e = format!("{:#}", Bundle::from_scene_content(&v.to_string(), "character.catpope").unwrap_err());
        assert!(e.contains("names instrument 'drone'"), "{e}");
        // the sets and portraits of a full answer don't disturb it; an answer without rigs is empty
        assert!(SceneRigs::parse(r#"{"sets":{"x":{"model":{}}},"characters":{"a":{"portraits":{}}},"missing":{"sets":[]}}"#).unwrap().rigs().next().is_none());
        assert!(SceneRigs::parse("nope").is_err());
    }
}
