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
/// Its conductor (#188 phase 4): the bigger being in the middle of the nave, PLF's portrait as its head, the decks' beings
/// around it over the relay.
pub const CONDUCTOR_SCENE: &str = include_str!("../../native/scenes/conductor.toml");
/// A Live outlet (#186): the relay's cells for this deck's outlet over the space (the `live` and `space` modules).
pub const OUTLET_SCENE: &str = include_str!("../../native/scenes/outlet.toml");
/// The core alone (#185): an LFO, the test pattern, the screen – what a stage shows when a scene can't be played.
pub const TEST_PATTERN_SCENE: &str = include_str!("../../native/scenes/test-pattern.toml");

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    #[serde(default)]
    pub stage: StageSettings,
    #[serde(default, rename = "node")]
    pub nodes: Vec<NodeDef>,
    #[serde(default, rename = "link")]
    pub links: Vec<LinkDef>,
    /// #211: the parameters as outputs – MIDI out, the conductor, DMX fixtures and the clock-synced strobe; parsed and
    /// checked by deck-outputs (its config.rs documents the table), played by the host (deck-native's outputs.rs)
    #[serde(default)]
    pub outputs: Option<toml::Table>,
    #[cfg(feature = "being")]
    #[serde(default)]
    pub being: crate::space::being::Look,
    /// without the space (deck-space's `being`, deck-native's `space` module) the being's look is kept as it is (the
    /// scene still parses; deck-native refuses its space node)
    #[cfg(not(feature = "being"))]
    #[serde(default)]
    pub being: toml::Table,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct StageSettings {
    pub name: String,
    /// the render size relative to the window (0.25–1); a software adapter halves it by itself
    pub render_scale: f32,
    /// the audio device's buffer in frames (#208, deck-native's audio/backend.rs): a heavy scene may ask for more;
    /// `--audio-buffer` and deck.toml's `audio_buffer` (the machine's) win over it. None = the device's default
    pub audio_buffer: Option<u32>,
    /// #264: how the stage hands over TO this scene on a switch (F5): the DMX / MIDI outputs of the scene that was playing
    /// fade into this one's (`fade`, the default), snap (`cut`) or go dark first (`blackout`); a stop is always dark
    pub transition: SceneTransition,
    /// the hand-over's length in s (`fade`); the same time base as the picture's fade in (deck-native switcher.rs `FADE_IN`)
    pub transition_secs: f32,
}

/// What a switch does to the outputs (the lamps) between two scenes (#264).
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
}

/// `"node.port"` → (node, port); a bare `"node"` → (node, "out")
pub fn split_ref(r: &str) -> (&str, &str) {
    match r.split_once('.') {
        Some((n, p)) => (n, p),
        None => (r, "out"),
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
        for text in [CATHEDRAL_SCENE, CONDUCTOR_SCENE, OUTLET_SCENE, TEST_PATTERN_SCENE] {
            let s = Scene::parse(text).unwrap();
            assert!(s.nodes.iter().any(|n| n.kind == "screen"));
        }
    }

    #[test]
    fn refs_default_to_out() {
        assert_eq!(split_ref("fft.bass"), ("fft", "bass"));
        assert_eq!(split_ref("music"), ("music", "out"));
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
