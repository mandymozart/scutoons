//! What a page or a deck gets for one character (research/INSTRUMENTS.md §6, §7 step 2): the rig with every instrument
//! and preset it names (ancestors included) – scene-content's answer, or a core set's TOML (`CORE`) – and what playing
//! it hands the page: the flattened scene (TOML, a stage scene; nodes and links for the wire graph), the being's
//! character, the knobs (each exposed parameter with its range, curve, label and current value – shared/live-deck's
//! `Spec`), which preset each track plays, and the scene lowered for the browser's engine (engine.rs). deck/space-web's
//! `rig_play` is this behind wasm-bindgen; the page calls it again on a mood change and applies what differs.
use super::{Instrument, Library, Preset, Rig, Text};
use crate::graph::{Graph, Kind, PortType};
use crate::params::{self, Curve};
use crate::scene::{split_ref, LinkDef, NodeDef, Scene, CATHEDRAL_SCENE, SILENT_SCENE};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The core rigs a deck carries without an account (#191 "core"): subject → the bundle's TOML (deck/native/rigs/).
pub const CORE: [(&str, &str); 2] = [
    ("character.catpope", include_str!("../../../native/rigs/catpope.toml")),
    ("item.incense_burner", include_str!("../../../native/rigs/incense_burner.toml")),
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
    /// a code rig's policy module: its descriptor (the `module:<id>` document's, #408)
    #[serde(default, rename = "module", skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<deck_catalog::Descriptor>,
}

/// A code rig's `@preset` target (#408): the policy's output (`policy.<out>`, streamed) chooses the track's preset by
/// its index into `choices` – the deck replays the rig with it (`Bundle::play_with`, #410).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PresetChoice {
    pub track: String,
    pub path: String,
    pub choices: Vec<String>,
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

impl Knob {
    /// its label in `locale`, else English, else any, else its path
    pub fn label_in(&self, locale: &str) -> &str {
        self.label.get(locale).or_else(|| self.label.get("en")).or_else(|| self.label.values().next()).map_or(&self.path, String::as_str)
    }

    /// its range as a parameter (#369: value ↔ 0..1 on its curve, params.rs)
    pub fn spec(&self) -> crate::params::Spec {
        crate::params::Spec::knob(self.min, self.max, self.value, self.log)
    }
}

/// How a streamed path moves (#368, the deck stream's epsilon per kind – the relay client's `Values`, relay.rs): a knob's
/// input with its range, a control output, an audio output's peak, an LFO's shape.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    Input([f32; 2]),
    Control,
    Audio,
    Shape,
}

impl Stream {
    /// how much a value must move before the stream sends it again: a thousandth of an input's range, 0.002 for control
    /// outputs (0–1 mostly; the tempo's counts move by whole beats anyway), 0.01 for an audio peak, half a shape
    pub fn epsilon(self) -> f32 {
        match self {
            Stream::Input([min, max]) => ((max - min).abs() * 1e-3).max(1e-5),
            Stream::Control => 2e-3,
            Stream::Audio => 0.01,
            Stream::Shape => 0.5,
        }
    }
}

/// What a mood's replay changes (#368): the values that moved – every knob and every numeric param the preset wrote,
/// shown or not – and whether the engine must load the new scene (another shape: nodes or links).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diff {
    pub values: Vec<(String, f32)>,
    pub reload: bool,
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
    /// the same scene for the browser's sound engine (live-core) and the nodes it does not carry yet (engine.rs)
    pub engine: super::engine::Engine,
    /// #368: every path the deck streams (the being's hoses read them) – the knobs' inputs, the analyser's level and
    /// bass, a rig's sum (the mixer feeding `fft`) – in the knobs' order, then the outputs
    pub streamed: Vec<(String, Stream)>,
    /// #368: the knobs the being shows as buttons (its `expose` on the wire)
    pub expose: Vec<String>,
    /// a code rig's preset choices (#408): which policy output chooses which track's preset
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub presets: Vec<PresetChoice>,
    /// #410: the engine's inputs a code rig's `set` / `mul` target owns (`<track>_<node>.<port>`, a strip's `mixN.gainK`) –
    /// a replay's diff leaves them out: a base under such a link would sum with it (decision 2)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owned: Vec<String>,
}

impl Played {
    /// the facts a page needs beside the scene: what streams (the knobs' inputs, the analyser, a rig's sum) and what the
    /// being shows
    fn with_facts(mut self) -> Played {
        let mut streamed: Vec<(String, Stream)> = self.knobs.iter().map(|k| (k.path.clone(), Stream::Input([k.min, k.max]))).collect();
        if self.nodes.iter().any(|n| n.id == "fft") {
            streamed.extend(["fft.level", "fft.bass"].map(|p| (p.to_string(), Stream::Control)));
        }
        // a rig's sum: the last of the mixers Rig::flatten chains (`mix1`, `mix2` …) – it feeds `fft`
        if !self.tracks.is_empty() {
            if let Some(sum) = self.links.iter().find(|l| l.to == "fft.in").and_then(|l| l.from.strip_suffix(".out")) {
                if self.nodes.iter().any(|n| n.id == sum && n.kind == "mixer") {
                    streamed.push((format!("{sum}.out"), Stream::Audio));
                }
            }
        }
        // a code rig's preset choices: the deck reads them each frame (#410)
        streamed.extend(self.presets.iter().map(|c| (c.path.clone(), Stream::Control)));
        self.expose = self.knobs.iter().map(|k| k.path.clone()).collect();
        self.streamed = streamed;
        self
    }

    /// #368: where a knob's (or a preset's param's) value goes in the engine: live-core plays the scene as it is, so the
    /// path is the engine's – unless its node is one the browser's engine does not carry yet (`engine.page`), or none.
    pub fn target<'a>(&self, path: &'a str) -> Option<&'a str> {
        let node = path.rsplit_once('.').map_or(path, |(n, _)| n);
        let played = self.nodes.iter().any(|n| n.id == node) && !self.engine.page.iter().any(|n| n.id == node);
        played.then_some(path)
    }

    /// every numeric param of the nodes as `node.param`, then the knobs' values (a knob over a param wins)
    fn values(&self) -> Vec<(String, f32)> {
        let mut out: Vec<(String, f32)> = Vec::new();
        let mut put = |path: String, v: f32| match out.iter_mut().find(|(p, _)| *p == path) {
            Some(slot) => slot.1 = v,
            None => out.push((path, v)),
        };
        for n in &self.nodes {
            for (k, v) in &n.params {
                if let Some(x) = v.as_float().or_else(|| v.as_integer().map(|i| i as f64)) {
                    put(format!("{}.{k}", n.id), x as f32);
                }
            }
        }
        for k in &self.knobs {
            put(k.path.clone(), k.value);
        }
        out
    }

    /// #368: this scene → `to` (the same rig replayed for another mood): the values that moved and whether the engine
    /// loads the new scene – a node's id or kind, or a link (ends, scale, offset) that differ. An input a code rig's policy
    /// owns (`owned`, #410) is no value of the diff: its link is its only writer.
    pub fn diff(&self, to: &Played) -> Diff {
        let was = self.values();
        let values = to.values().into_iter().filter(|(p, v)| !to.owned.contains(p) && !was.iter().any(|(q, w)| q == p && w == v)).collect();
        let shape = |p: &Played| (p.nodes.iter().map(|n| (n.id.clone(), n.kind.clone())).collect::<Vec<_>>(), p.links.iter().map(|l| (l.from.clone(), l.to.clone(), l.scale, l.offset)).collect::<Vec<_>>());
        Diff { values, reload: shape(self) != shape(to) }
    }

    /// A deck's scene played as the page needs it (#360) – what the browser deck plays when its character has no rig: the
    /// scene as it is, live-core's part of it (`engine::lower`), its knobs the being's buttons (`params::shown`, the
    /// native deck's 3D buttons) on the nodes that part plays (a linked input's knob turns its base, the link's offset;
    /// an input no param or link drives is none), the sticks its first four knobs.
    pub fn scene(text: &str) -> Result<Played> {
        let scene = Scene::parse(text)?;
        let graph = Graph::build(&scene)?;
        let engine = super::engine::lower(&scene)?;
        let expose = scene.being_expose();
        let mut knobs = Vec::new();
        for (i, n) in graph.nodes.iter().enumerate().filter(|(_, n)| !engine.page.iter().any(|p| p.id == n.id)) {
            for (k, port) in n.kind.inputs().iter().enumerate() {
                if port.ty != PortType::Control || !params::shown(n.kind, &n.id, port.name, &expose) {
                    continue;
                }
                let spec = params::spec(n.kind, port.name);
                let value = graph.link_into(i, k).map_or_else(|| n.f32(port.name, spec.default), |l| l.offset);
                if value.is_finite() {
                    let log = spec.curve == Curve::Log;
                    knobs.push(Knob { path: format!("{}.{}", n.id, port.name), label: Text::new(), min: spec.min, max: spec.max, value, log });
                }
            }
        }
        let mut it = knobs.iter().map(|k| k.path.clone());
        let sticks = [[it.next().unwrap_or_default(), it.next().unwrap_or_default()], [it.next().unwrap_or_default(), it.next().unwrap_or_default()]];
        let character = scene.being_character();
        let played = Played { scene: text.to_string(), nodes: scene.nodes, links: scene.links, character, knobs, tracks: Vec::new(), sticks, engine, streamed: Vec::new(), expose: Vec::new(), presets: Vec::new(), owned: Vec::new() };
        Ok(played.with_facts())
    }

    /// The deck's own scene (#425): what plays when no rig does – the being in its space, no audio graph at all, so
    /// no sound (owner 2026-11-08: "eine figur ohne rig, sollte nicht klingen"). Every deck has it from here (native,
    /// browser, headless: `Deck::new`), never a copy.
    pub fn silent() -> Result<Played> {
        Played::scene(SILENT_SCENE)
    }

    /// The Cathedral of decks (#360), a TEST scene since #425 (owner 2026-11-08: "die scene cathedral is ein test
    /// gewesen"): the X1's old boot scene, the one `--scene cathedral` and F5 play and the room a native rig is staged
    /// in. No deck plays it as a fallback any more; `Deck::testing` plays it for the tests that need knobs and a sound.
    pub fn deck_scene() -> Result<Played> {
        Played::scene(CATHEDRAL_SCENE)
    }

    /// `deck_scene` as JSON – the wasm boundary's form
    pub fn deck_scene_json() -> Result<String> {
        Ok(serde_json::to_string(&Played::deck_scene()?)?)
    }
}

/// the scene document as TOML (scene.rs's form, so deck-native and live-core read it)
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
        Library::new(self.instruments.clone(), self.presets.clone()).with_modules(self.modules.clone())
    }

    pub fn play(&self, persona: Option<&str>, mood: Option<&str>) -> Result<Played> {
        self.play_with(persona, mood, &BTreeMap::new())
    }

    /// `play` with presets chosen per track (track → preset: a code rig's `@preset` outputs, the deck's replay #410)
    pub fn play_with(&self, persona: Option<&str>, mood: Option<&str>, chosen: &BTreeMap<String, String>) -> Result<Played> {
        let lib = self.library();
        let scene = self.rig.flatten_with(&lib, persona, mood, chosen)?;
        let tracks = self
            .rig
            .play_with(&lib, persona, mood, chosen)?
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
        // a code rig's parameters are rig knobs too (`policy.<param>`, #408), and its preset choices are facts
        let (mut presets, mut owned) = (Vec::new(), Vec::new());
        if let Some((_, d)) = self.rig.policy_module(&lib)? {
            use super::policy::Member;
            let node = scene.nodes.iter().find(|n| n.id == super::policy::NODE).context("the policy node is in the scene")?;
            for p in d.params.iter().filter(|p| p.choices.is_empty()) {
                let value = node.params.get(p.name.as_ref()).and_then(|v| v.as_float()).map_or(p.default.unwrap_or(p.min), |f| f as f32);
                let label = if p.doc.is_empty() { Text::new() } else { Text::from([("en".to_string(), p.doc.to_string())]) };
                knobs.push(Knob { path: format!("{}.{}", super::policy::NODE, p.name), label, min: p.min, max: p.max, value, log: p.curve == deck_catalog::Curve::Log });
            }
            let strip = |track: &str| node.params.get(super::policy::GAIN_TABLE).and_then(|t| t.get(track)).and_then(|v| v.as_str()).map(String::from);
            for t in self.rig.targets(&lib)? {
                let owns = t.owns();
                match t.member {
                    Member::Preset(choices) => presets.push(PresetChoice { track: t.track, path: format!("{}.{}", super::policy::NODE, t.output), choices }),
                    Member::Input(path) if owns => owned.push(super::flatten::prefixed(&t.track, &path)),
                    Member::Gain if owns => owned.extend(strip(&t.track)),
                    _ => {}
                }
            }
        }
        let sticks = match &self.rig.sticks {
            Some(pairs) => {
                for p in pairs.iter().flatten() {
                    if !p.is_empty() && !knobs.iter().any(|k| &k.path == p) {
                        bail!("rig {}: stick '{p}' is not one of its knobs", self.rig.subject);
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
        let played = Played { scene: text, nodes: scene.nodes, links: scene.links, character, knobs, tracks, sticks, engine, streamed: Vec::new(), expose: Vec::new(), presets, owned };
        Ok(played.with_facts())
    }

    /// `play` as JSON – the wasm boundary's form (`persona` / `mood` empty = none)
    pub fn play_json(bundle_json: &str, persona: &str, mood: &str) -> Result<String> {
        let none = |s: &str| (!s.is_empty()).then(|| s.to_string());
        let played = Bundle::parse_json(bundle_json)?.play(none(persona).as_deref(), none(mood).as_deref())?;
        Ok(serde_json::to_string(&played)?)
    }
}
