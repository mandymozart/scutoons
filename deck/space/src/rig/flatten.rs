//! A rig as one playable scene (#191, research/INSTRUMENTS.md §7 step 1): every played track's patch with its node ids
//! prefixed `<track>_<node>`, the tracks summed by a chain of four-input mixers (a track's `gain` = its mixer gain) into
//! the scene's `audio_out`, an `fft` on the sum (the being's energy and feelings read it), the rig's routes as links,
//! and a bare `space` node the caller completes (its set, spot, camera) – the being, breathing with the sum's level
//! (`fft.level` → `space.pulse`, as the phone deck's patch did) unless a route drives `being:pulse` itself.
//! A code rig (#408, policy.rs) adds its policy: the node `policy` (`module:<id>`, registered from the library's
//! descriptor), its parameters and seed, the context it starts with (character, persona, mood – the live one is #409's),
//! and the strips its `@gain` targets drive; routes reach it as `policy:<port>`.
//! The being's `character` (from the subject) and `expose` (the instruments' shown parameters) come along.
use super::policy::{self, Member};
use super::{scoped, Library, Rig, TrackPatch};
use crate::modreg::{self, ModuleDef};
use crate::scene::{split_ref, LinkDef, NodeDef, Scene};
use anyhow::{bail, Result};
use std::collections::BTreeMap;

/// a mixer's inputs (graph.rs `Kind::Mixer`)
const MIXER_INPUTS: usize = 4;

fn node(id: &str, kind: &str, params: &[(&str, f32)]) -> NodeDef {
    let mut t = toml::Table::new();
    for (k, v) in params {
        t.insert((*k).to_string(), toml::Value::Float(f64::from(*v)));
    }
    NodeDef { id: id.to_string(), kind: kind.to_string(), params: t }
}

fn link(from: String, to: String, scale: f32, offset: f32) -> LinkDef {
    LinkDef { from, to, scale, offset, map: Vec::new() }
}

/// `node.port` inside track `t` → `<t>_<node>.<port>`
pub(crate) fn prefixed(track: &str, r: &str) -> String {
    let (n, p) = split_ref(r);
    format!("{track}_{n}.{p}")
}

impl Rig {
    /// The rig played for `persona` and `mood` (`Rig::play`) as one scene; checked as a graph.
    pub fn flatten(&self, lib: &Library, persona: Option<&str>, mood: Option<&str>) -> Result<Scene> {
        self.flatten_with(lib, persona, mood, &BTreeMap::new())
    }

    /// `flatten` with presets chosen per track (`Rig::play_with`: a code rig's `@preset` outputs, #410)
    pub fn flatten_with(&self, lib: &Library, persona: Option<&str>, mood: Option<&str>, chosen: &BTreeMap<String, String>) -> Result<Scene> {
        self.check(lib)?;
        let played = self.play_with(lib, persona, mood, chosen)?;
        if played.is_empty() {
            bail!("rig {}: every track is muted", self.subject);
        }
        let mut scene = Scene::parse("")?;
        scene.stage.name = self.subject.clone();
        for TrackPatch { track, patch, .. } in &played {
            for n in &patch.nodes {
                scene.nodes.push(NodeDef { id: format!("{track}_{}", n.id), ..n.clone() });
            }
            for l in &patch.links {
                scene.links.push(LinkDef { map: l.map.clone(), ..link(prefixed(track, &l.from), prefixed(track, &l.to), l.scale, l.offset) });
            }
        }
        self.mix(&played, &mut scene);
        self.policy_node(lib, &played, persona, mood, &mut scene)?;
        let playing = |t: &str| played.iter().any(|p| p.track == t) || (t == policy::NODE && self.policy.is_some());
        let mut pulse = false;
        for r in &self.routes {
            let (Some((ft, from)), Some((tt, to))) = (scoped(&r.from), scoped(&r.to)) else { continue };
            // a route from or to a muted track is off with it
            if !playing(ft) || (tt != "being" && !playing(tt)) {
                continue;
            }
            let end = |track: &str, r: &str| if track == policy::NODE { format!("{}.{r}", policy::NODE) } else { prefixed(track, r) };
            let to = if tt == "being" {
                pulse |= to == "pulse";
                format!("space.{to}")
            } else {
                end(tt, to)
            };
            scene.links.push(link(end(ft, from), to, r.scale, r.offset));
        }
        scene.nodes.push(node("space", "space", &[]));
        if !pulse {
            scene.links.push(link("fft.level".into(), "space.pulse".into(), 0.6, 0.0));
        }
        #[cfg(feature = "being")]
        {
            if let Some(c) = self.subject.strip_prefix("character.") {
                scene.being.character = c.split('.').next().unwrap_or(c).to_string();
            }
            for TrackPatch { track, .. } in &played {
                let inst = lib.instrument(&self.tracks.iter().find(|t| &t.id == track).expect("played").instrument)?;
                scene.being.expose.extend(inst.expose.iter().map(|e| prefixed(track, &super::instrument::canonical(&e.path))));
                // #424: the knobs' hover names – the labels the instrument gave its exposed parameters
                for e in inst.expose.iter().filter(|e| !e.label.is_empty()) {
                    scene.being.labels.insert(prefixed(track, &super::instrument::canonical(&e.path)), e.label.clone());
                }
            }
            if let Some((_, d)) = self.policy_module(lib)? {
                scene.being.expose.extend(d.params.iter().map(|p| format!("{}.{}", policy::NODE, p.name)));
            }
        }
        crate::graph::Graph::build(&scene).map_err(|e| e.context(format!("rig {} flattened", self.subject)))?;
        Ok(scene)
    }

    /// The tracks summed on their strips (`policy::strip`: `mix1` takes the first four tracks, each next mixer the one
    /// before on `in1` – gain 1 – and three more tracks); the last mixer feeds `out` (audio_out) and `fft`.
    fn mix(&self, played: &[TrackPatch], scene: &mut Scene) {
        let last = policy::strip(played.len() - 1).0;
        for n in 1..=last {
            let id = format!("mix{n}");
            let (mut gains, mut links) = (Vec::new(), Vec::new());
            if n > 1 {
                gains.push(("gain1".to_string(), 1.0));
                links.push(link(format!("mix{}.out", n - 1), format!("{id}.in1"), 1.0, 0.0));
            }
            for (i, p) in played.iter().enumerate().filter(|(i, _)| policy::strip(*i).0 == n) {
                let k = policy::strip(i).1;
                debug_assert!(k <= MIXER_INPUTS);
                gains.push((format!("gain{k}"), self.tracks.iter().find(|t| t.id == p.track).map_or(1.0, |t| t.gain)));
                links.push(link(prefixed(&p.track, &p.patch.out), format!("{id}.in{k}"), 1.0, 0.0));
            }
            let params: Vec<(&str, f32)> = gains.iter().map(|(k, g)| (k.as_str(), *g)).collect();
            scene.nodes.push(node(&id, "mixer", &params));
            scene.links.extend(links);
        }
        scene.nodes.push(node("out", "audio_out", &[]));
        scene.nodes.push(node("fft", "fft", &[]));
        scene.links.push(link(format!("mix{last}.out"), "out.in".into(), 1.0, 0.0));
        scene.links.push(link(format!("mix{last}.out"), "fft.in".into(), 1.0, 0.0));
    }

    /// A code rig's policy as the node `policy`: its module registered as a node kind (the library's descriptor, the
    /// mapping the native library uses too), its seed and parameters, the context it starts with, and the strip of
    /// each `@gain` target (`@gain = { <track> = "mixN.gainK" }`, bound by graph.rs). `@preset` stays unbound: the deck
    /// applies it (#410).
    fn policy_node(&self, lib: &Library, played: &[TrackPatch], persona: Option<&str>, mood: Option<&str>, scene: &mut Scene) -> Result<()> {
        let Some((policy, d)) = self.policy_module(lib)? else { return Ok(()) };
        modreg::register(&d.name, &ModuleDef::of(d));
        let mut params = toml::Table::new();
        if let Some(seed) = policy.seed {
            params.insert("seed".into(), toml::Value::Integer(i64::from(seed)));
        }
        for (k, v) in self.policy_values(persona, mood) {
            params.insert(k, toml::Value::Float(f64::from(v)));
        }
        if let Some(c) = self.subject.strip_prefix("character.") {
            params.insert("character".into(), toml::Value::String(c.split('.').next().unwrap_or(c).to_string()));
        }
        for (k, v) in [("persona", persona), ("mood", mood)] {
            if let Some(v) = v {
                params.insert(k.into(), toml::Value::String(v.to_string()));
            }
        }
        let mut gains = toml::Table::new();
        for t in self.targets(lib)?.iter().filter(|t| t.member == Member::Gain) {
            if let Some(i) = played.iter().position(|p| p.track == t.track) {
                let (n, k) = policy::strip(i);
                gains.insert(t.track.clone(), toml::Value::String(format!("mix{n}.gain{k}")));
            }
        }
        if !gains.is_empty() {
            params.insert(policy::GAIN_TABLE.into(), toml::Value::Table(gains));
        }
        scene.nodes.push(NodeDef { id: policy::NODE.to_string(), kind: format!("{}{}", modreg::PREFIX, d.name), params });
        Ok(())
    }
}
