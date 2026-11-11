//! The stored scene document (#243, research/SCENES.md §3): `scene:<id>` in the API's `entity_versions`. Format 1 is
//! today's `Scene` (scene.rs) as JSON – nothing new invented – beside a header: `format`, `id`, `version` (stamped by the
//! server on every change, as a show's) and `toons`. The one Rust form of it serves
//! - the core's check before a publish (rig/validate.rs kind `scenes`, the build service's `POST /validate`): it parses
//!   here and builds as a graph, or the API refuses it with these words;
//! - the JSON Schema the API guards a save with (deck-schema → server/schema/content/scene.schema.json, `[contract]`)
//!   and, from that schema, the TS and PHP types (server/schema/generate.ts → shared/api, server/src/Generated).
//!
//! Holders and world nodes (SCENE-PACKAGE §2) are format 2, after #462 (proposal, SCENES.md §7: v1 without holders).
//! A scene names its set and spot on its `space` node, as the TOML does (`set` = the staging, `spot` = a point in it;
//! SCENES.md §8.4) – the document adds no third word for either.
use crate::scene::Scene;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::Value;

/// The format this core reads and writes. A deck refuses a format it does not know (RULES A9: no back-compat).
pub const FORMAT: u32 = 1;

/// One `scene:<id>` document as the API stores it: the header, then the scene's own fields at the top level
/// (`stage`, `node`, `link`, `outputs`, `being`).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(deny_unknown_fields))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneDocument {
    /// the document's format: 1 (today's scene graph, no holders)
    #[cfg_attr(feature = "schema", schemars(extend("const" = 1)))]
    pub format: u32,
    /// the key's id: `nave` for `scene:nave` – platform-wide, no toon in it (a scene can serve several toons and shows)
    pub id: String,
    /// the number the server stamps on every change (as a show's); absent until the first save
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// the toons whose address books list it; empty = reachable by its id only (proposal, SCENES.md §7 question 5)
    #[serde(default)]
    pub toons: Vec<String>,
    /// the scene itself, its fields beside the header
    #[serde(flatten)]
    pub scene: Scene,
}

impl SceneDocument {
    /// A scene as a document of this format (no version yet: the server stamps it)
    pub fn new(id: &str, toons: Vec<String>, scene: Scene) -> SceneDocument {
        SceneDocument { format: FORMAT, id: id.to_string(), version: None, toons, scene }
    }

    /// The document from its JSON: the header read and taken off, the rest is a `Scene` with its unknown fields refused
    /// (serde's `flatten` would let them through, so the header is read by hand).
    pub fn from_value(value: Value) -> Result<SceneDocument> {
        let Value::Object(mut fields) = value else { bail!("a scene document is a JSON object") };
        match fields.remove("format") {
            Some(v) if v.as_u64() == Some(u64::from(FORMAT)) => {}
            Some(v) => bail!("format {v} is not one this core reads (it reads {FORMAT})"),
            None => bail!("`format` is missing ({FORMAT})"),
        }
        let id = match fields.remove("id") {
            Some(Value::String(id)) if !id.is_empty() => id,
            _ => bail!("`id` is missing: the id of the key `scene:<id>`"),
        };
        let version = match fields.remove("version") {
            None | Some(Value::Null) => None,
            Some(v) => Some(v.as_u64().and_then(|n| u32::try_from(n).ok()).context("`version` is a whole number ≥ 0")?),
        };
        let toons = match fields.remove("toons") {
            None => Vec::new(),
            Some(v) => serde_json::from_value(v).context("`toons` is a list of toon ids")?,
        };
        let scene: Scene = serde_json::from_value(Value::Object(fields)).context("the scene")?;
        Ok(SceneDocument { format: FORMAT, id, version, toons, scene })
    }

    pub fn parse_json(text: &str) -> Result<SceneDocument> {
        SceneDocument::from_value(serde_json::from_str(text).context("a scene document is JSON")?)
    }

    /// The document as JSON text (f32 values written as f32, so `0.15` stays `0.15`; a `serde_json::Value` would widen
    /// them first)
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::scene::{builtin, BUILTIN};

    /// every scene the repo carries survives TOML → document JSON → document unchanged (the being's knob labels are no
    /// scene key: none of these has them)
    #[test]
    fn every_bundled_scene_round_trips_through_json() {
        let scenes = BUILTIN.iter().map(|n| (*n, builtin(n).unwrap())).chain([("silent", crate::scene::SILENT_SCENE)]);
        for (name, text) in scenes {
            let scene = Scene::parse(text).unwrap();
            let doc = SceneDocument::new(name, vec!["catpope".into()], scene);
            let back = SceneDocument::parse_json(&doc.to_json().unwrap()).unwrap();
            assert_eq!(back, doc, "{name}");
        }
    }

    #[test]
    fn the_header_is_checked_and_unknown_fields_are_refused() {
        let ok = r#"{"format":1,"id":"x","toons":[],"node":[{"id":"t","kind":"tempo","bpm":120}]}"#;
        let doc = SceneDocument::parse_json(ok).unwrap();
        assert_eq!((doc.format, doc.id.as_str(), doc.version, doc.scene.nodes.len()), (1, "x", None, 1));
        assert!(SceneDocument::parse_json(r#"{"format":2,"id":"x"}"#).unwrap_err().to_string().contains("format 2"));
        assert!(SceneDocument::parse_json(r#"{"id":"x"}"#).is_err(), "no format");
        assert!(SceneDocument::parse_json(r#"{"format":1}"#).is_err(), "no id");
        assert!(SceneDocument::parse_json(r#"{"format":1,"id":"x","nodes":[]}"#).is_err(), "a typo is refused, not dropped");
        assert!(SceneDocument::parse_json(r#"{"format":1,"id":"x","version":-1}"#).is_err());
        assert_eq!(SceneDocument::parse_json(r#"{"format":1,"id":"x","version":3}"#).unwrap().version, Some(3));
    }

    /// #243 / SCENES.md §8.2: a nudge names its control input as `input`; `set` in a scene is the staging alone
    #[cfg(feature = "being")]
    #[test]
    fn a_nudge_says_input_and_set_is_refused_there() {
        let doc = r#"{"format":1,"id":"x","being":{"react":[{"mood":"happy","input":"space.hue","by":0.1}]}}"#;
        assert_eq!(SceneDocument::parse_json(doc).unwrap().scene.being.react[0].input, "space.hue");
        let old = r#"{"format":1,"id":"x","being":{"react":[{"mood":"happy","set":"space.hue","by":0.1}]}}"#;
        assert!(SceneDocument::parse_json(old).is_err());
        assert!(Scene::parse("[being]\nreact = [{ mood = \"happy\", set = \"space.hue\", by = 0.1 }]\n").is_err(), "no alias in TOML either");
    }

    /// The first real scene (#243, SCENES.md §5; proposal, SCENES.md §7 question 4): `scene:nave` = the cathedral set with
    /// Cat Pope's rig. The fixture server/tests/fixtures/scenes/nave.json (the API's tests save and publish it) is this
    /// built from the core rig `character.catpope` (rig/bundle.rs `CORE`; the catalog's rig started as it) flattened for
    /// no persona and no mood, its `space` node staged as the cathedral sandbox stages it. It must round-trip into the same
    /// scene. `UPDATE_FIXTURES=1 cargo test -p deck-space nave` writes it.
    #[cfg(feature = "being")]
    #[test]
    fn nave_is_the_cathedral_set_with_catpopes_rig() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/nave.json");
        let built = nave().to_json().unwrap() + "\n";
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
            std::fs::write(path, &built).unwrap();
        }
        let stored = std::fs::read_to_string(path).unwrap().replace('\r', "");
        assert_eq!(stored, built, "the fixture is stale: UPDATE_FIXTURES=1 cargo test -p deck-space nave");
        let doc = SceneDocument::parse_json(&stored).unwrap();
        let mut scene = nave().scene;
        scene.being.labels.clear();
        assert_eq!(doc.scene, scene, "the same scene back");
        assert_eq!(doc.scene.nodes.iter().find(|n| n.kind == "space").unwrap().params["set"].as_str(), Some("cathedral"));
        Graph::build(&doc.scene).unwrap();
    }

    #[cfg(feature = "being")]
    fn nave() -> SceneDocument {
        use crate::rig::bundle::Bundle;
        let bundle = Bundle::core("character.catpope").unwrap().unwrap();
        let mut scene = bundle.rig.flatten(&bundle.library(), None, None).unwrap();
        scene.stage.name = "Nave".into();
        let cathedral = Scene::parse(crate::scene::CATHEDRAL_SCENE).unwrap();
        let room = cathedral.nodes.iter().find(|n| n.kind == "space").unwrap();
        let space = scene.nodes.iter_mut().find(|n| n.kind == "space").unwrap();
        for key in ["set", "quality", "at", "yaw", "pitch", "distance"] {
            space.params.insert(key.into(), room.params[key].clone());
        }
        SceneDocument::new("nave", vec!["catpope".into()], scene)
    }
}
