//! The scene document: a TOML node graph (CYBERDECKS.md §8 – "a scene is a graph document, TOML now, later an entity on
//! scutoons.com"). `[[node]]` = id + kind + the kind's params, `[[link]]` = `from = "node.port"` → `to = "node.port"`
//! (a bare node id means its port `out`); control links map the value with `scale` and `offset`. `[being]` = the
//! being's look when the scene has a `space` node (space/being.rs, #188 – with the graph it is what a being preset
//! saves, #191).
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// The scene the stage plays when none is given: catpope 1's music + a synth, the FFT drives the test pattern.
pub const DEFAULT_SCENE: &str = include_str!("../scenes/default.toml");
/// The Cathedral of decks (#188): the set `cathedral` from Cat Pope chapter 2 and this deck's being in it.
pub const CATHEDRAL_SCENE: &str = include_str!("../scenes/cathedral.toml");
/// Its conductor (#188 phase 4): the bigger being in the middle of the nave, PLF's portrait as its head, the decks' beings
/// around it over the relay.
pub const CONDUCTOR_SCENE: &str = include_str!("../scenes/conductor.toml");
/// A Live outlet (#186): the relay's cells for this deck's outlet over the space (the `live` and `space` modules).
pub const OUTLET_SCENE: &str = include_str!("../scenes/outlet.toml");
/// The core alone (#185): an LFO, the test pattern, the screen – what a stage shows when a scene can't be played.
pub const TEST_PATTERN_SCENE: &str = include_str!("../scenes/test-pattern.toml");

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    #[serde(default)]
    pub stage: StageSettings,
    #[serde(default, rename = "node")]
    pub nodes: Vec<NodeDef>,
    #[serde(default, rename = "link")]
    pub links: Vec<LinkDef>,
    #[cfg(feature = "space")]
    #[serde(default)]
    pub being: crate::space::being::Look,
    /// without the `space` module the being's look is kept as it is (the scene still parses; its space node is refused)
    #[cfg(not(feature = "space"))]
    #[serde(default)]
    pub being: toml::Table,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct StageSettings {
    pub name: String,
    /// the render size relative to the window (0.25–1); a software adapter halves it by itself
    pub render_scale: f32,
}

impl Default for StageSettings {
    fn default() -> Self {
        Self { name: "untitled".into(), render_scale: 1.0 }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeDef {
    pub id: String,
    pub kind: String,
    /// everything else on the node: the kind's params (graph.rs reads them by name)
    #[serde(flatten)]
    pub params: toml::Table,
}

#[derive(Debug, Clone, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_scene_parses() {
        let s = Scene::parse(DEFAULT_SCENE).unwrap();
        assert!(s.nodes.iter().any(|n| n.kind == "chapter_music"));
        assert!(s.nodes.iter().any(|n| n.kind == "screen"));
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
