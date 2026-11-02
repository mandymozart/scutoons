//! What a page or a deck gets for one character (research/INSTRUMENTS.md §6, §7 step 2): the rig with every instrument
//! and preset it names (ancestors included) – scene-content's answer, or a core set's TOML (`CORE`) – and what playing
//! it hands the page: the flattened scene (TOML, a stage scene; nodes and links for the wire graph), the being's
//! character, the knobs (each exposed parameter with its range, curve, label and current value – shared/live-deck's
//! `Spec`), which preset each track plays, and the scene lowered for the browser's engine (engine.rs). deck/space-web's
//! `rig_play` is this behind wasm-bindgen; the page calls it again on a mood change and applies what differs.
use super::{Instrument, Library, Preset, Rig, Text};
use crate::graph::Kind;
use crate::params::{self, Curve};
use crate::scene::{split_ref, LinkDef, NodeDef};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// The core rigs a deck carries without an account (#191 "core"): subject → the bundle's TOML (deck/stage/rigs/).
pub const CORE: [(&str, &str); 2] = [
    ("character.catpope", include_str!("../../../stage/rigs/catpope.toml")),
    ("item.incense_burner", include_str!("../../../stage/rigs/incense_burner.toml")),
];

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub rig: Rig,
    #[serde(default, rename = "instrument")]
    pub instruments: Vec<Instrument>,
    #[serde(default, rename = "preset")]
    pub presets: Vec<Preset>,
}

/// A knob the page shows: an exposed parameter of the flattened scene (`<track>_<node>.<port>`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Knob {
    pub path: String,
    #[serde(skip_serializing_if = "Text::is_empty")]
    pub label: Text,
    pub min: f32,
    pub max: f32,
    /// the patched param, else the kind's default (params.rs)
    pub value: f32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub log: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrackPlays {
    pub track: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
}

/// The rig played: everything the page needs to build its sound, its wire graph and its knobs.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Played {
    /// a stage scene (`[stage]`, `[[node]]`, `[[link]]`, `[being]`): the space node bare – the caller sets its set
    pub scene: String,
    pub nodes: Vec<NodeDef>,
    pub links: Vec<LinkDef>,
    /// the head's character (from `character.<id>[.<persona>]`), empty for an item's rig
    pub character: String,
    pub knobs: Vec<Knob>,
    pub tracks: Vec<TrackPlays>,
    /// the corner sticks' pairs, `[[x, y], [x, y]]` (left, right) as knob paths: the rig's `sticks`, else its first four
    /// knobs in order ("" = no knob for that axis)
    pub sticks: [[String; 2]; 2],
    /// the same scene for the browser's sound engine (live-core) and what the page plays itself (engine.rs)
    pub engine: super::engine::Engine,
}

/// the scene document as TOML (scene.rs's form, so deck-stage and live-core read it)
#[derive(Serialize)]
struct SceneDoc<'a> {
    stage: StageDoc<'a>,
    node: &'a [NodeDef],
    link: &'a [LinkDef],
    being: BeingDoc<'a>,
}

#[derive(Serialize)]
struct StageDoc<'a> {
    name: &'a str,
}

#[derive(Serialize)]
struct BeingDoc<'a> {
    #[serde(skip_serializing_if = "str::is_empty")]
    character: &'a str,
    expose: Vec<&'a str>,
}

impl Bundle {
    /// the core rig of `subject` (`character.<id>` / `item.<id>`), if the deck carries one
    pub fn core(subject: &str) -> Option<Result<Bundle>> {
        let (_, text) = CORE.iter().find(|(s, _)| *s == subject)?;
        Some(toml::from_str(text).with_context(|| format!("the core rig {subject}")))
    }

    /// `core` as JSON – the wasm boundary's form; empty when the deck carries none
    pub fn core_json(subject: &str) -> Result<String> {
        match Bundle::core(subject) {
            Some(b) => Ok(serde_json::to_string(&b?)?),
            None => Ok(String::new()),
        }
    }

    pub fn parse_json(text: &str) -> Result<Bundle> {
        serde_json::from_str(text).context("the rig bundle is not valid JSON for a bundle")
    }

    pub fn library(&self) -> Library {
        Library::new(self.instruments.clone(), self.presets.clone())
    }

    pub fn play(&self, persona: Option<&str>, mood: Option<&str>) -> Result<Played> {
        let lib = self.library();
        let scene = self.rig.flatten(&lib, persona, mood)?;
        let tracks = self
            .rig
            .play(&lib, persona, mood)?
            .into_iter()
            .map(|t| TrackPlays { track: t.track, preset: t.preset })
            .collect();
        let character = self
            .rig
            .subject
            .strip_prefix("character.")
            .map(|c| c.split('.').next().unwrap_or(c).to_string())
            .unwrap_or_default();
        let mut knobs = Vec::new();
        for t in self.rig.tracks.iter().filter(|t| !t.mute) {
            let inst = lib.instrument(&t.instrument)?;
            for e in &inst.expose {
                let (node, port) = split_ref(&e.path);
                let id = format!("{}_{node}", t.id);
                let n = scene.nodes.iter().find(|n| n.id == id).context("an exposed node is in the scene")?;
                let kind = Kind::parse(&n.kind).context("a checked kind")?;
                let spec = params::spec(kind, port);
                let [min, max] = e.range.unwrap_or([spec.min, spec.max]);
                let v = n.params.get(port);
                let value = v
                    .and_then(|v| v.as_float().map(|f| f as f32).or_else(|| v.as_integer().map(|i| i as f32)))
                    .unwrap_or(spec.default);
                knobs.push(Knob {
                    path: format!("{id}.{port}"),
                    label: e.label.clone(),
                    min,
                    max,
                    value,
                    log: spec.curve == Curve::Log,
                });
            }
        }
        let sticks = match &self.rig.sticks {
            Some(pairs) => {
                for p in pairs.iter().flatten() {
                    if !p.is_empty() && !knobs.iter().any(|k| &k.path == p) {
                        bail!("rig '{}': stick '{p}' is not one of its knobs", self.rig.subject);
                    }
                }
                pairs.clone()
            }
            None => {
                let mut it = knobs.iter().map(|k| k.path.clone());
                [[it.next().unwrap_or_default(), it.next().unwrap_or_default()], [it.next().unwrap_or_default(), it.next().unwrap_or_default()]]
            }
        };
        let doc = SceneDoc {
            stage: StageDoc { name: &self.rig.subject },
            node: &scene.nodes,
            link: &scene.links,
            being: BeingDoc { character: &character, expose: knobs.iter().map(|k| k.path.as_str()).collect() },
        };
        let text = toml::to_string(&doc).context("writing the flattened scene")?;
        let engine = super::engine::lower(&scene)?;
        Ok(Played { scene: text, nodes: scene.nodes, links: scene.links, character, knobs, tracks, sticks, engine })
    }

    /// `play` as JSON – the wasm boundary's form (`persona` / `mood` empty = none)
    pub fn play_json(bundle_json: &str, persona: &str, mood: &str) -> Result<String> {
        let none = |s: &str| (!s.is_empty()).then(|| s.to_string());
        let played = Bundle::parse_json(bundle_json)?.play(none(persona).as_deref(), none(mood).as_deref())?;
        Ok(serde_json::to_string(&played)?)
    }
}
