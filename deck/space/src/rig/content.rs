//! scene-content's rigs (#271, server docs/API.md "Library") → a `Bundle` per character / item. The answer names each
//! holder's rig as `{ id, version, data, instruments: [ids], presets: [ids] }` and holds every instrument / preset once
//! in its top-level maps (`{ id, version, data }`, presets with their ancestors); `version` = 16 hex of the stored data's
//! SHA-256, what a deck's cache compares. The data stays JSON here until a bundle is built, so one broken rig never
//! hides the others, and a cache can store the entries as they came (deck/native sync, shared/live-deck).
use super::Bundle;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// One stored definition as served: its id, its version, its data (an `Instrument` / `Preset`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stored {
    pub id: String,
    pub version: String,
    pub data: Value,
}

/// A holder's rig as served: the stored `Rig` and the ids of what it names in the top-level maps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub version: String,
    pub data: Value,
    #[serde(default)]
    pub instruments: Vec<String>,
    #[serde(default)]
    pub presets: Vec<String>,
}

/// a character or item of the answer: only its rig matters here (portraits, look … are the caller's)
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Holder {
    #[serde(default)]
    pub rig: Option<Entry>,
}

/// The rig part of scene-content's answer.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SceneRigs {
    #[serde(default)]
    pub characters: BTreeMap<String, Holder>,
    #[serde(default)]
    pub items: BTreeMap<String, Holder>,
    #[serde(default)]
    pub instruments: BTreeMap<String, Stored>,
    #[serde(default)]
    pub presets: BTreeMap<String, Stored>,
}

/// an id a cache may use as a file name: letters, digits, `.`, `_`, `-`, not starting with a dot
pub fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 160 && !id.starts_with('.') && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

impl SceneRigs {
    pub fn parse_json(text: &str) -> Result<SceneRigs> {
        serde_json::from_str(text).context("scene-content's answer is not valid JSON for its rigs")
    }

    /// the rig of the asked character (the id as asked: `catpope`, `plf`, `catpope-regular`), if the answer has one
    pub fn character(&self, id: &str) -> Option<&Entry> {
        self.characters.get(id)?.rig.as_ref()
    }

    pub fn item(&self, id: &str) -> Option<&Entry> {
        self.items.get(id)?.rig.as_ref()
    }

    /// every rig of the answer: (`characters` / `items`, the asked id, the entry)
    pub fn rigs(&self) -> impl Iterator<Item = (&'static str, &str, &Entry)> {
        self.characters
            .iter()
            .filter_map(|(k, h)| Some(("characters", k.as_str(), h.rig.as_ref()?)))
            .chain(self.items.iter().filter_map(|(k, h)| Some(("items", k.as_str(), h.rig.as_ref()?))))
    }

    /// `entry` with what it names from the answer's maps, as a bundle (unknown fields refused, a missing one named)
    pub fn bundle(&self, entry: &Entry) -> Result<Bundle> {
        bundle_of(entry, |id| self.instruments.get(id).cloned(), |id| self.presets.get(id).cloned())
    }
}

/// A bundle from a rig entry and a lookup for what it names (the answer's maps, or a cache's files): each named
/// instrument / preset must be there and carry the id it is filed under.
pub fn bundle_of(entry: &Entry, instrument: impl Fn(&str) -> Option<Stored>, preset: impl Fn(&str) -> Option<Stored>) -> Result<Bundle> {
    let pick = |what: &str, ids: &[String], get: &dyn Fn(&str) -> Option<Stored>| -> Result<Vec<Value>> {
        ids.iter()
            .map(|id| match get(id) {
                Some(s) if s.id == *id => Ok(s.data),
                Some(s) => bail!("rig '{}': {what} '{id}' is filed as '{}'", entry.id, s.id),
                None => bail!("rig '{}' names the {what} '{id}', which is not there", entry.id),
            })
            .collect()
    };
    let doc = serde_json::json!({
        "rig": entry.data,
        "instrument": pick("instrument", &entry.instruments, &instrument)?,
        "preset": pick("preset", &entry.presets, &preset)?,
    });
    let b: Bundle = serde_json::from_value(doc).with_context(|| format!("rig '{}' is not a valid bundle", entry.id))?;
    if b.rig.subject != entry.id {
        bail!("rig '{}' carries the subject '{}'", entry.id, b.rig.subject);
    }
    Ok(b)
}

/// scene-content's answer → the bundle of one holder as JSON (`group` = `characters` / `items`), empty when it has no
/// rig – the wasm boundary's form (deck/space-web `scene_rig`)
pub fn scene_rig_json(answer: &str, group: &str, id: &str) -> Result<String> {
    let rigs = SceneRigs::parse_json(answer)?;
    let entry = match group {
        "characters" => rigs.character(id),
        "items" => rigs.item(id),
        _ => bail!("no group '{group}' (characters, items)"),
    };
    match entry {
        Some(e) => Ok(serde_json::to_string(&rigs.bundle(e)?)?),
        None => Ok(String::new()),
    }
}
