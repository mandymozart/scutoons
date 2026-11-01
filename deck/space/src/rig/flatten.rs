//! A rig as one playable scene (#191, research/INSTRUMENTS.md §7 step 1): every played track's patch with its node ids
//! prefixed `<track>_<node>`, the tracks summed by a chain of four-input mixers (a track's `gain` = its mixer gain) into
//! the scene's `audio_out`, an `fft` on the sum (the being's energy and feelings read it), the rig's routes as links,
//! and – when a route goes to `being:<input>` – a bare `space` node the caller completes (its set, spot, camera).
//! The being's `character` (from the subject) and `expose` (the instruments' shown parameters) come along.
use super::{scoped, Library, Rig, TrackPatch};
use crate::scene::{split_ref, LinkDef, NodeDef, Scene};
use anyhow::{bail, Result};

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
    LinkDef { from, to, scale, offset }
}

/// `node.port` inside track `t` → `<t>_<node>.<port>`
fn prefixed(track: &str, r: &str) -> String {
    let (n, p) = split_ref(r);
    format!("{track}_{n}.{p}")
}

impl Rig {
    /// The rig played for `persona` and `mood` (`Rig::play`) as one scene; checked as a graph.
    pub fn flatten(&self, lib: &Library, persona: Option<&str>, mood: Option<&str>) -> Result<Scene> {
        self.check(lib)?;
        let played = self.play(lib, persona, mood)?;
        if played.is_empty() {
            bail!("rig '{}': every track is muted", self.subject);
        }
        let mut scene = Scene::parse("")?;
        scene.stage.name = self.subject.clone();
        for TrackPatch { track, patch, .. } in &played {
            for n in &patch.nodes {
                scene.nodes.push(NodeDef { id: format!("{track}_{}", n.id), ..n.clone() });
            }
            for l in &patch.links {
                scene.links.push(link(prefixed(track, &l.from), prefixed(track, &l.to), l.scale, l.offset));
            }
        }
        self.mix(&played, &mut scene);
        let playing = |t: &str| played.iter().any(|p| p.track == t);
        let mut space = false;
        for r in &self.routes {
            let (Some((ft, from)), Some((tt, to))) = (scoped(&r.from), scoped(&r.to)) else { continue };
            // a route from or to a muted track is off with it
            if !playing(ft) || (tt != "being" && !playing(tt)) {
                continue;
            }
            let to = if tt == "being" {
                space = true;
                format!("space.{to}")
            } else {
                prefixed(tt, to)
            };
            scene.links.push(link(prefixed(ft, from), to, r.scale, r.offset));
        }
        if space {
            scene.nodes.push(node("space", "space", &[]));
        }
        #[cfg(feature = "being")]
        {
            if let Some(c) = self.subject.strip_prefix("character.") {
                scene.being.character = c.split('.').next().unwrap_or(c).to_string();
            }
            for TrackPatch { track, .. } in &played {
                let inst = lib.instrument(&self.tracks.iter().find(|t| &t.id == track).expect("played").instrument)?;
                scene.being.expose.extend(inst.expose.iter().map(|e| prefixed(track, &super::instrument::canonical(&e.path))));
            }
        }
        crate::graph::Graph::build(&scene).map_err(|e| e.context(format!("rig '{}' flattened", self.subject)))?;
        Ok(scene)
    }

    /// `mix1` takes the first four tracks; each next mixer takes the one before on `in1` and three more tracks; the
    /// last feeds `out` (audio_out) and `fft`.
    fn mix(&self, played: &[TrackPatch], scene: &mut Scene) {
        let mut inputs: Vec<(String, f32)> = played
            .iter()
            .map(|p| {
                let gain = self.tracks.iter().find(|t| t.id == p.track).map_or(1.0, |t| t.gain);
                (prefixed(&p.track, &p.patch.out), gain)
            })
            .collect();
        let mut n = 0;
        loop {
            n += 1;
            let id = format!("mix{n}");
            let take: Vec<(String, f32)> = inputs.drain(..inputs.len().min(MIXER_INPUTS)).collect();
            let gains: Vec<(String, f32)> =
                take.iter().enumerate().map(|(i, (_, g))| (format!("gain{}", i + 1), *g)).collect();
            let params: Vec<(&str, f32)> = gains.iter().map(|(k, g)| (k.as_str(), *g)).collect();
            scene.nodes.push(node(&id, "mixer", &params));
            for (i, (from, _)) in take.into_iter().enumerate() {
                scene.links.push(link(from, format!("{id}.in{}", i + 1), 1.0, 0.0));
            }
            if inputs.is_empty() {
                scene.nodes.push(node("out", "audio_out", &[]));
                scene.nodes.push(node("fft", "fft", &[]));
                scene.links.push(link(format!("{id}.out"), "out.in".into(), 1.0, 0.0));
                scene.links.push(link(format!("{id}.out"), "fft.in".into(), 1.0, 0.0));
                return;
            }
            inputs.insert(0, (format!("{id}.out"), 1.0));
        }
    }
}
