//! A scene's own video nodes in an editor: a node of a video kind the catalog has built (deck-catalog: the picture
//! domain, a graph kind, not one a scene owns once), where its picture plays (a free video input of the scene's nodes,
//! or the window – the `screen`, made when the scene has none), its knobs as the catalog's params in range; a value
//! equal to the kind's own leaves the document. Every change goes through `SceneEdit::change`.
use crate::scene::{LinkDef, NodeDef};
use crate::scene_edit::{Line, Part, SceneEdit};
use deck_catalog::{Curve, Descriptor, Dir, Domain, PortType, Scope};

/// the window: where a picture plays when nothing else takes it
pub const SCREEN: &str = "screen.in";

/// the video kinds a scene may add, in the catalog's order
pub fn video_kinds() -> Vec<&'static str> {
    deck_catalog::builtin_kinds().iter().filter(|d| d.domain == Domain::Video && d.scope == Scope::Graph && !d.scene_only).map(|d| d.name.as_ref()).collect()
}

/// what a video kind is, in the catalog's words
pub fn about(kind: &str) -> &'static str {
    descriptor(kind).map_or("", |d| d.doc.as_ref())
}

/// A knob of a video node as the scene sets it.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoKnob {
    pub param: String,
    pub min: f32,
    pub max: f32,
    pub log: bool,
    pub unit: String,
    /// the scene's value, else the kind's own
    pub value: f32,
    pub set: bool,
}

fn descriptor(kind: &str) -> Option<&'static Descriptor> {
    deck_catalog::builtin(kind).filter(|d| d.domain == Domain::Video)
}

fn video_port(d: &Descriptor, dir: Dir) -> impl Iterator<Item = &str> {
    d.ports.iter().filter(move |p| p.dir == dir && p.ty == PortType::Video).map(|p| p.name.as_ref())
}

impl SceneEdit {
    /// the scene's own video nodes a scene may add (not its space, its screen or an outlet): their places in the nodes
    pub fn videos(&self) -> Vec<usize> {
        let kinds = video_kinds();
        (0..self.doc.nodes.len()).filter(|&i| kinds.contains(&self.doc.nodes[i].kind.as_str())).collect()
    }

    /// where node `v`'s picture plays (`screen.in`, `overlay.top`); none = nowhere yet
    pub fn plays_at(&self, v: usize) -> Option<String> {
        let id = &self.doc.nodes.get(v)?.id;
        self.doc.links.iter().find(|l| l.from.split_once('.').map_or(l.from.as_str(), |(n, _)| n) == id && is_video_input(&self.doc.nodes, &l.to)).map(|l| l.to.clone())
    }

    pub(crate) fn video_lines(&self) -> Vec<Line> {
        self.videos()
            .into_iter()
            .map(|i| {
                let n = &self.doc.nodes[i];
                let detail = self.plays_at(i).map_or_else(|| "nowhere".to_string(), |t| t.split_once('.').map_or(t.as_str(), |(n, _)| n).to_string());
                Line { depth: 1, label: n.id.clone(), detail, part: Part::Video(i) }
            })
            .collect()
    }

    /// A node of video kind `kind`, its picture on the window when nothing plays there yet: its place in the nodes. Err:
    /// no video kind a scene adds, or the core's words.
    pub fn add_video(&mut self, kind: &str) -> Result<usize, String> {
        if !video_kinds().contains(&kind) {
            return Err(format!("'{kind}' is no video kind a scene adds ({})", video_kinds().join(", ")));
        }
        let id = (1..).map(|n| if n == 1 { kind.to_string() } else { format!("{kind}_{n}") }).find(|id| !self.doc.nodes.iter().any(|n| &n.id == id)).unwrap_or_default();
        let screen_free = !self.doc.links.iter().any(|l| l.to == SCREEN);
        let out = descriptor(kind).and_then(|d| video_port(d, Dir::Out).next()).map(String::from);
        self.change(|d| {
            d.nodes.push(NodeDef { id: id.clone(), kind: kind.to_string(), params: toml::Table::new() });
            if let (true, Some(out)) = (screen_free, out) {
                wire(d, &id, &out, SCREEN);
            }
            Ok(())
        })?;
        Ok(self.doc.nodes.iter().position(|n| n.id == id).expect("added"))
    }

    /// where node `v`'s picture may play: the window, and each video input of the scene's other nodes no link feeds
    pub fn targets(&self, v: usize) -> Vec<String> {
        let Some(me) = self.doc.nodes.get(v) else {
            return Vec::new();
        };
        let fed = |to: &str| self.doc.links.iter().any(|l| l.to == to);
        let mut out = vec![SCREEN.to_string()];
        for n in self.doc.nodes.iter().filter(|n| n.id != me.id) {
            let Some(d) = descriptor(&n.kind).filter(|d| !d.scene_only) else {
                continue;
            };
            out.extend(video_port(d, Dir::In).map(|p| format!("{}.{p}", n.id)).filter(|t| !fed(t)));
        }
        out
    }

    /// Node `v`'s picture to `target` (one of `targets`): its old link goes, the screen is made when the scene has none,
    /// whatever fed the window before leaves it. Err: the core's words (a loop, a port it lacks).
    pub fn play_at(&mut self, v: usize, target: &str) -> Result<(), String> {
        let node = self.doc.nodes.get(v).filter(|_| self.videos().contains(&v)).ok_or("no video node of the scene's")?.clone();
        let out = descriptor(&node.kind).and_then(|d| video_port(d, Dir::Out).next()).map(String::from).ok_or_else(|| format!("{}: a {} has no picture out", node.id, node.kind))?;
        if !self.targets(v).iter().any(|t| t == target) && self.plays_at(v).as_deref() != Some(target) {
            return Err(format!("{}: {target} is no free video input of this scene", node.id));
        }
        self.change(|d| {
            let from = format!("{}.{out}", node.id);
            d.links.retain(|l| l.from != from && l.to != target);
            wire(d, &node.id, &out, target);
            Ok(())
        })
    }

    /// node `v`'s knobs: the kind's number params, the scene's value or the kind's own
    pub fn video_knobs(&self, v: usize) -> Vec<VideoKnob> {
        let Some(n) = self.doc.nodes.get(v) else {
            return Vec::new();
        };
        let Some(d) = descriptor(&n.kind) else {
            return Vec::new();
        };
        d.params
            .iter()
            .filter(|p| p.choices.is_empty() && p.max > p.min)
            .map(|p| {
                let own = p.default.unwrap_or(p.min);
                let set = n.params.get(p.name.as_ref()).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64))).map(|x| x as f32);
                VideoKnob { param: p.name.to_string(), min: p.min, max: p.max, log: p.curve == Curve::Log, unit: p.unit.to_string(), value: set.unwrap_or(own), set: set.is_some() }
            })
            .collect()
    }

    /// Node `v`'s knob `param` set to `value`; the kind's own value takes it out again. Err: no knob of the kind, a value
    /// outside its range.
    pub fn set_video(&mut self, v: usize, param: &str, value: f32) -> Result<(), String> {
        let id = self.doc.nodes.get(v).map(|n| n.id.clone()).ok_or("no such node")?;
        let k = self.video_knobs(v).into_iter().find(|k| k.param == param).ok_or_else(|| format!("{id}: no knob {param}"))?;
        if !value.is_finite() || value < k.min || value > k.max {
            return Err(format!("{id}: {param} goes from {} to {}", k.min, k.max));
        }
        let own = deck_catalog::builtin(&self.doc.nodes[v].kind).and_then(|d| d.params.iter().find(|p| p.name == param)?.default).unwrap_or(k.min);
        self.change(|d| {
            let params = &mut d.nodes[v].params;
            if (value - own).abs() <= f32::EPSILON * own.abs().max(1.0) {
                params.remove(param);
            } else {
                params.insert(param.to_string(), toml::Value::Float(f64::from(value)));
            }
            Ok(())
        })
    }

    /// what a `video` node plays: its file's media path
    pub fn video_src(&self, v: usize) -> Option<&str> {
        self.doc.nodes.get(v)?.params.get("src")?.as_str()
    }

    /// a `video` node's in and out points on its file (`start`, `end`): this use's trim, the file untouched
    #[cfg(feature = "media")]
    pub fn video_trim(&self, v: usize) -> deck_media::Trim {
        let num = |k: &str| self.doc.nodes.get(v).and_then(|n| n.params.get(k)).and_then(|p| p.as_float().or_else(|| p.as_integer().map(|i| i as f64))).unwrap_or(0.0);
        deck_media::Trim::new(num("start"), num("end"))
    }

    /// A `video` node's file. Err: not a video node, or no video file (MP4, WebM, MOV, MKV).
    pub fn set_video_src(&mut self, v: usize, src: &str) -> Result<(), String> {
        let id = self.doc.nodes.get(v).filter(|n| n.kind == "video").map(|n| n.id.clone()).ok_or("no video node")?;
        if !deck_catalog::background::is_video(src) {
            return Err(format!("{id}: '{src}' is no video file (.mp4, .webm, .mov, .mkv)"));
        }
        self.change(|d| {
            d.nodes[v].params.insert("src".into(), toml::Value::String(src.trim().to_string()));
            Ok(())
        })
    }

    /// node `v`'s knob `param` back to the kind's own value
    pub fn clear_video(&mut self, v: usize, param: &str) {
        if let Some(n) = self.doc.nodes.get_mut(v) {
            n.params.remove(param);
        }
    }

    /// node `v` taken out of the scene with its links (a screen it made stays: the scene's window)
    pub fn remove_video(&mut self, v: usize) -> Result<(), String> {
        let id = self.doc.nodes.get(v).filter(|_| self.videos().contains(&v)).map(|n| n.id.clone()).ok_or("no video node of the scene's")?;
        let ends = |end: &str| end.split_once('.').map_or(end, |(n, _)| n) == id;
        self.change(|d| {
            d.nodes.remove(v);
            d.links.retain(|l| !ends(&l.from) && !ends(&l.to));
            Ok(())
        })
    }
}

/// `node.out` → `target`, a screen made for `screen.in` when the document has none
fn wire(d: &mut crate::scene_document::SceneDocument, node: &str, out: &str, target: &str) {
    if target == SCREEN && !d.nodes.iter().any(|n| n.id == "screen") {
        d.nodes.push(NodeDef { id: "screen".into(), kind: "screen".into(), params: toml::Table::new() });
    }
    d.links.push(LinkDef { from: format!("{node}.{out}"), to: target.to_string(), scale: 1.0, offset: 0.0, map: Vec::new() });
}

/// whether `end` (`node.port`) is a video input of one of `nodes`
fn is_video_input(nodes: &[NodeDef], end: &str) -> bool {
    let Some((n, p)) = end.split_once('.') else {
        return false;
    };
    nodes.iter().find(|x| x.id == n).and_then(|x| descriptor(&x.kind)).is_some_and(|d| video_port(d, Dir::In).any(|q| q == p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_document::SceneDocument;
    use crate::scene_resolve::{core_rigs, no_sets};

    fn solo() -> SceneEdit {
        let mut d = SceneDocument::new("clip", "Clip", vec!["catpope".into()]);
        d.recast(&["character:catpope".into()]);
        SceneEdit::open(d, None, &core_rigs, &no_sets).expect("it plays").0
    }

    #[test]
    fn the_video_kinds_are_the_catalogs_built_picture_kinds() {
        let kinds = video_kinds();
        assert!(kinds.contains(&"test_pattern") && kinds.contains(&"shader") && kinds.contains(&"video"), "{kinds:?}");
        assert!(!kinds.iter().any(|k| ["screen", "space", "outlet", "look", "model3d"].contains(k)), "no sink, no scene-owned node, nothing planned: {kinds:?}");
    }

    #[test]
    fn a_clip_is_a_video_node_with_its_file() {
        let mut edit = solo();
        let v = edit.add_video("video").unwrap();
        assert_eq!(edit.plays_at(v).as_deref(), Some(SCREEN));
        assert!(edit.set_video_src(v, "layers/sky.png").unwrap_err().contains("no video file"));
        edit.set_video_src(v, "live/x/loop.mov").unwrap();
        assert_eq!(edit.video_src(v), Some("live/x/loop.mov"));
        let played = edit.resolved(&core_rigs, &no_sets).expect("the stage plays it");
        let node = played.scene.nodes.iter().find(|n| n.kind == "video").unwrap();
        assert_eq!(node.params.get("src").and_then(|s| s.as_str()), Some("live/x/loop.mov"));
        assert!(edit.video_knobs(v).iter().any(|k| k.param == "speed" && k.value == 1.0), "as recorded");
    }

    #[test]
    fn a_video_node_is_added_plays_on_the_window_and_its_knobs_are_the_kinds_params() {
        let mut edit = solo();
        let v = edit.add_video("test_pattern").unwrap();
        assert_eq!(edit.plays_at(v).as_deref(), Some(SCREEN));
        assert!(edit.doc.nodes.iter().any(|n| n.kind == "screen"), "the window made");
        assert_eq!(edit.tree().last().map(|l| (l.part, l.detail.as_str())), Some((Part::Video(v), "screen")));
        let played = edit.resolved(&core_rigs, &no_sets).expect("the stage plays it");
        assert!(played.scene.nodes.iter().any(|n| n.id == "test_pattern"));
        let k = edit.video_knobs(v).into_iter().find(|k| k.param == "speed").expect("the catalog's speed");
        assert!(!k.set);
        edit.set_video(v, "speed", 2.0).unwrap();
        assert!(edit.video_knobs(v).iter().any(|k| k.param == "speed" && k.set && k.value == 2.0));
        assert!(edit.set_video(v, "speed", 99.0).unwrap_err().contains("goes from"));
        edit.set_video(v, "speed", k.value).unwrap();
        assert!(!edit.doc.nodes[v].params.contains_key("speed"), "the kind's own value leaves the document");
        // a shader over it: the pattern into the shader, the shader on the window
        let s = edit.add_video("shader").unwrap();
        assert_eq!(edit.plays_at(s), None, "the window is taken");
        assert!(edit.targets(v).contains(&"shader.in".to_string()));
        edit.play_at(v, "shader.in").unwrap();
        edit.play_at(s, SCREEN).unwrap();
        assert_eq!((edit.plays_at(v).as_deref(), edit.plays_at(s).as_deref()), (Some("shader.in"), Some(SCREEN)));
        assert!(edit.play_at(s, "shader.in").is_err(), "not into itself");
        edit.resolved(&core_rigs, &no_sets).expect("still plays");
        let back = SceneDocument::parse_json(&edit.doc.to_json().unwrap()).unwrap();
        assert_eq!(back, edit.doc, "saved with the draft");
        edit.remove_video(v).unwrap();
        assert!(!edit.doc.links.iter().any(|l| l.from.starts_with("test_pattern")));
        assert!(edit.remove_video(0).is_err(), "the space is the scene's");
    }
}
