//! The scene document (deck-space, #198 – deck-native plays it, the space builds its being from it): a TOML node graph (CYBERDECKS.md §8 – "a scene is a graph document, TOML now, later an entity on
//! scutoons.com"). `[[node]]` = id + kind + the kind's params, `[[link]]` = `from = "node.port"` → `to = "node.port"`
//! (a bare node id means its port `out`); control links map the value with `scale` and `offset`. `[being]` = the
//! being's look when the scene has a `space` node (space/being.rs, #188 – with the graph it is what a being preset
//! saves, #191).
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The scene the stage plays when none is given (#252): fireflies on black, no sound, no network – the deck idles.
pub const IDLE_SCENE: &str = include_str!("../../native/scenes/idle.toml");
/// "AV madness" (`--scene madness`; the default until #252): catpope 1's music + a synth, the FFT drives the test pattern.
pub const MADNESS_SCENE: &str = include_str!("../../native/scenes/madness.toml");
/// The Cathedral of decks (#188): the set `cathedral` from Cat Pope chapter 2 and this deck's being in it.
pub const CATHEDRAL_SCENE: &str = include_str!("../../native/scenes/cathedral.toml");
/// The silent scene (#425): the being in its space, no audio graph – what the deck plays when no rig does.
pub const SILENT_SCENE: &str = include_str!("../../native/scenes/silent.toml");
/// Its conductor (#188 phase 4): the bigger being in the middle of the nave, PLF's portrait as its head, the decks' beings
/// around it over the relay.
pub const CONDUCTOR_SCENE: &str = include_str!("../../native/scenes/conductor.toml");
/// A Live outlet (#186): the relay's cells for this deck's outlet over the space (the `live` and `space` modules).
pub const OUTLET_SCENE: &str = include_str!("../../native/scenes/outlet.toml");
/// The core alone (#185): an LFO, the test pattern, the screen – what a stage shows when a scene can't be played.
pub const TEST_PATTERN_SCENE: &str = include_str!("../../native/scenes/test-pattern.toml");

/// the built-in scenes in the list's order (the first is the stage's default)
pub const BUILTIN: [&str; 12] = ["idle", "madness", "cathedral", "catpope-analog", "conductor", "outlet", "strobe", "pulse-thunderwash", "lamps-off", "test-pattern", "module-walk", "module-analog"];

/// the source of a built-in scene by name (the deck's F5 list, the browser's `?scene=`)
pub fn builtin(name: &str) -> Option<&'static str> {
    Some(match name {
        "idle" => IDLE_SCENE,
        "madness" => MADNESS_SCENE,
        "cathedral" => CATHEDRAL_SCENE,
        "catpope-analog" => include_str!("../../native/scenes/catpope-analog.toml"),
        "conductor" => CONDUCTOR_SCENE,
        "outlet" => OUTLET_SCENE,
        "strobe" => include_str!("../../native/scenes/strobe.toml"),
        "pulse-thunderwash" => include_str!("../../native/scenes/pulse-thunderwash.toml"),
        "lamps-off" => include_str!("../../native/scenes/lamps-off.toml"),
        "test-pattern" => TEST_PATTERN_SCENE,
        // #406: they play compiled rig modules, so they need the `modules` build and the modules' wasm in `DECK_MODULES`
        "module-walk" => include_str!("../../native/scenes/module-walk.toml"),
        "module-analog" => include_str!("../../native/scenes/module-analog.toml"),
        _ => return None,
    })
}

/// A stage scene: the graph, the stage's settings, the outputs and the being. TOML in the repo's files, JSON in the stored
/// document (#243, scene_document.rs: the same struct, so TOML is only a view of it).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    #[serde(default)]
    pub stage: StageSettings,
    #[serde(default, rename = "node")]
    pub nodes: Vec<NodeDef>,
    #[serde(default, rename = "link")]
    pub links: Vec<LinkDef>,
    /// the parameters as outputs (#211) – MIDI out, the conductor, DMX fixtures and the clock-synced strobe; parsed and
    /// checked by deck-outputs (its config.rs documents the table), played by the host (deck-native's outputs.rs)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<std::collections::BTreeMap<String, serde_json::Value>>"))]
    pub outputs: Option<toml::Table>,
    #[cfg(feature = "being")]
    #[serde(default)]
    pub being: crate::space::being::Look,
    /// without the space (deck-space's `being`, deck-native's `space` module) the being's look is kept as it is (the
    /// scene still parses; deck-native refuses its space node)
    #[cfg(not(feature = "being"))]
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(with = "std::collections::BTreeMap<String, serde_json::Value>"))]
    pub being: toml::Table,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct StageSettings {
    pub name: String,
    /// the render size relative to the window (0.25–1); a software adapter halves it by itself
    pub render_scale: f32,
    /// the audio device's buffer in frames (#208, deck-native's audio/backend.rs): a heavy scene may ask for more;
    /// `--audio-buffer` and deck.toml's `audio_buffer` (the machine's) win over it. None = the device's default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_buffer: Option<u32>,
    /// how the stage hands over TO this scene on a switch (F5, #264): the DMX / MIDI outputs of the scene that was playing
    /// fade into this one's (`fade`, the default), snap (`cut`) or go dark first (`blackout`); a stop is always dark
    pub transition: SceneTransition,
    /// the hand-over's length in s (`fade`); the same time base as the picture's fade in (deck-native switcher.rs `FADE_IN`)
    pub transition_secs: f32,
}

/// What a switch does to the outputs (the lamps) between two scenes (#264).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneTransition {
    /// hold the last values, then crossfade to the new scene's over `transition_secs`; channels it doesn't drive fade to 0
    #[default]
    Fade,
    /// the new scene takes over at once; channels it doesn't drive go to 0
    Cut,
    /// the lamps go dark first (the outputs stop and start again), like a stop
    Blackout,
}

/// the default `transition_secs`
pub const DEFAULT_TRANSITION_SECS: f32 = 0.4;

impl Default for StageSettings {
    fn default() -> Self {
        Self { name: "untitled".into(), render_scale: 1.0, audio_buffer: None, transition: SceneTransition::Fade, transition_secs: DEFAULT_TRANSITION_SECS }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeDef {
    pub id: String,
    pub kind: String,
    /// everything else on the node: the kind's params (graph.rs reads them by name)
    #[serde(flatten)]
    #[cfg_attr(feature = "schema", schemars(with = "std::collections::BTreeMap<String, serde_json::Value>"))]
    pub params: toml::Table,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkDef {
    pub from: String,
    pub to: String,
    #[serde(default = "one")]
    pub scale: f32,
    #[serde(default)]
    pub offset: f32,
    /// an audio link's channel map (plan 2.6, #318): `[source, target]` pairs counted from 1 – `[[1, 3], [2, 4]]` lands a
    /// stereo source on channels 3 and 4 (a send's offset); one source may go to several targets, several sources into one
    /// target sum. Empty = channel n to channel n. `scale` on an audio link is the send level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub map: Vec<[u8; 2]>,
}

fn one() -> f32 {
    1.0
}

impl Scene {
    pub fn parse(text: &str) -> Result<Scene> {
        toml::from_str(text).context("the scene is not valid TOML for a stage scene")
    }

    pub fn load(path: &Path) -> Result<Scene> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading the scene {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("in {}", path.display()))
    }

    /// the being's `expose` (its look's; read from the table when the space is not built)
    pub fn being_expose(&self) -> Vec<String> {
        #[cfg(feature = "being")]
        return self.being.expose.clone();
        #[cfg(not(feature = "being"))]
        return self.being_strs("expose");
    }

    /// the being's head: its `character` ("" = none)
    pub fn being_character(&self) -> String {
        #[cfg(feature = "being")]
        return self.being.character.clone();
        #[cfg(not(feature = "being"))]
        return self.being.get("character").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    }

    #[cfg(not(feature = "being"))]
    fn being_strs(&self, key: &str) -> Vec<String> {
        let list = self.being.get(key).and_then(|v| v.as_array());
        list.map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default()
    }

    /// What of this scene a build that carries the modules `carries` plays (deck-core's `carried_part`, #360): the scene
    /// without the other nodes and the links that touch them – and those nodes and links.
    pub fn carried_part(&self, carries: impl Fn(&str) -> bool) -> (Scene, Vec<NodeDef>, Vec<LinkDef>) {
        let part = deck_core::stage::carried_part(
            self.nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str(), &n.params)),
            self.links.iter().map(|l| (l.from.as_str(), l.to.as_str())),
            carries,
        );
        let mut out = self.clone();
        let (nodes, page): (Vec<_>, Vec<_>) = self.nodes.iter().cloned().zip(&part.nodes).partition(|(_, &stays)| stays);
        let (links, dropped): (Vec<_>, Vec<_>) = self.links.iter().cloned().zip(&part.links).partition(|(_, &stays)| stays);
        out.nodes = nodes.into_iter().map(|(n, _)| n).collect();
        out.links = links.into_iter().map(|(l, _)| l).collect();
        (out, page.into_iter().map(|(n, _)| n).collect(), dropped.into_iter().map(|(l, _)| l).collect())
    }
}

/// A set's 3D file at `quality` under `sets` (`--sets`, default `<cache>/sets`): `<sets>/<id>/<id>.<quality>.glb` (the
/// sync's layout), else `<sets>/<id>.<quality>.glb` (tools/rooms' build output, e.g. media/scutoons/rooms) – the first
/// that exists, else the first (where the sync puts it).
pub fn set_file(sets: &Path, id: &str, quality: &str) -> PathBuf {
    let name = format!("{id}.{quality}.glb");
    let canonical = sets.join(id).join(&name);
    if canonical.is_file() {
        return canonical;
    }
    let flat = sets.join(&name);
    if flat.is_file() {
        flat
    } else {
        canonical
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_scene_parses() {
        let s = Scene::parse(MADNESS_SCENE).unwrap();
        assert!(s.nodes.iter().any(|n| n.kind == "chapter_music"));
        assert!(s.nodes.iter().any(|n| n.kind == "screen"));
    }

    #[test]
    fn the_transition_is_a_fade_unless_the_scene_says_otherwise() {
        let s = Scene::parse("").unwrap();
        assert_eq!((s.stage.transition, s.stage.transition_secs), (SceneTransition::Fade, DEFAULT_TRANSITION_SECS));
        let s = Scene::parse("[stage]
transition = \"blackout\"
").unwrap();
        assert_eq!(s.stage.transition, SceneTransition::Blackout);
        let s = Scene::parse("[stage]
transition = \"fade\"
transition_secs = 2.5
").unwrap();
        assert_eq!((s.stage.transition, s.stage.transition_secs), (SceneTransition::Fade, 2.5));
        assert!(Scene::parse("[stage]
transition = \"wipe\"
").is_err());
    }

    #[test]
    fn the_bundled_scenes_parse() {
        for text in [CATHEDRAL_SCENE, SILENT_SCENE, CONDUCTOR_SCENE, OUTLET_SCENE, TEST_PATTERN_SCENE] {
            let s = Scene::parse(text).unwrap();
            assert!(s.nodes.iter().any(|n| n.kind == "screen"));
        }
    }

    #[test]
    fn links_default_to_identity() {
        let s = Scene::parse("[[link]]\nfrom = \"a\"\nto = \"b.in\"\n").unwrap();
        assert_eq!((s.links[0].scale, s.links[0].offset), (1.0, 0.0));
    }

    #[test]
    fn unknown_top_level_keys_are_errors() {
        assert!(Scene::parse("[stage]\nnmae = \"x\"\n").is_err());
    }
}
