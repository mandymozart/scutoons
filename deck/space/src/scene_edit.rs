//! A scene opened in an editor's Build (research/SCENES.md §9.3): its document, its cast as members with each one's rig,
//! the tree an editor shows of it, and the one rule for a member's values in the scene – a value sets a knob of the
//! member's rig, inside the knob's range, stored as `<track>:<node>.<port>` on its holder; a value equal to the rig's own
//! leaves the document. Its slots (placeholders, §9.2) and its video nodes are edited too (scene_slots.rs, scene_video.rs);
//! every change is refused when the core finds a new problem in it. In Build a slot plays its preview. What plays is the
//! resolve step's (scene_resolve.rs); the host only draws and saves the document.
use crate::rig::address::{Address, Leaf};
use crate::rig::bundle::{Bundle, Knob};
use crate::rig::Rig;
use crate::scene_document::{FillerRef, HolderValue, SceneDocument};
use crate::scene_resolve::{no_rigs, no_sets, resolve, Resolved, Rigs, Sets};
use std::collections::BTreeMap;

/// One filled holder of the scene and the rig it plays.
#[derive(Debug, Clone)]
pub struct Member {
    pub holder: String,
    /// `character:catpope`, as the holder names it
    pub filler: String,
    pub place: String,
    /// none = the scene's own spot
    pub spot: Option<String>,
    /// `character.catpope`; none = its filler plays no rig
    pub subject: Option<String>,
    /// the rig as the host had it; none = not at hand (the member stands silent)
    pub bundle: Option<Bundle>,
    /// the rig's knobs as the rig alone sets them (its slots `<track>_<node>.<port>`)
    pub knobs: Vec<Knob>,
    /// the prefix of its rig's slots in the played scene (resolve's `Member::slots`)
    pub prefix: Option<String>,
}

/// What a line of the tree stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Scene,
    /// an authored member, by its place in `members`
    Member(usize),
    /// a placeholder, by its holder's place in the document
    Slot(usize),
    /// a video node of the scene's own, by its place in the document's nodes
    Video(usize),
}

/// A line of the tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub depth: usize,
    pub label: String,
    pub detail: String,
    pub part: Part,
}

/// A knob of a member as the scene sets it.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneKnob {
    pub knob: Knob,
    /// the scene's value, else the rig's own
    pub value: f32,
    /// the scene sets it
    pub set: bool,
}

/// A scene open for editing: the document as edited, the version a save names as `base`, its members.
#[derive(Debug, Clone)]
pub struct SceneEdit {
    pub doc: SceneDocument,
    pub base: Option<u64>,
    pub members: Vec<Member>,
    /// what plays less than written (a rig not at hand)
    pub notes: Vec<String>,
    saved: SceneDocument,
}

impl SceneEdit {
    /// `doc` (read at version `base`) resolved with `rigs` and `sets`, its slots playing their previews: its members and
    /// the scene the stage plays. Err: the core's reasons it does not play.
    pub fn open(doc: SceneDocument, base: Option<u64>, rigs: Rigs, sets: Sets) -> Result<(SceneEdit, Resolved), Vec<String>> {
        let mut edit = SceneEdit { saved: doc.clone(), doc, base, members: Vec::new(), notes: Vec::new() };
        let resolved = edit.refresh(rigs, sets)?;
        Ok((edit, resolved))
    }

    /// The scene as edited now resolved again, its members with it (a slot filled or emptied changes them). Err: the
    /// core's reasons it does not play; the members stay.
    pub fn refresh(&mut self, rigs: Rigs, sets: Sets) -> Result<Resolved, Vec<String>> {
        let resolved = resolve(&self.doc, &BTreeMap::new(), true, rigs, sets)?;
        let doc = &self.doc;
        self.members = resolved
            .cast
            .iter()
            .map(|m| {
                let holder = doc.holders.iter().find(|h| h.id == m.holder);
                let f = FillerRef::parse(&m.filler).ok();
                let subject = holder.and_then(|h| h.rig.as_ref()?.subject().map(String::from)).or_else(|| f.as_ref()?.rig_subject());
                let version = holder.and_then(|h| h.rig.as_ref()?.version.clone());
                let bundle = subject.as_deref().and_then(|s| rigs(s, version.as_deref()));
                let persona = f.and_then(|f| f.persona);
                let knobs = bundle.as_ref().and_then(|b| b.play(persona.as_deref(), None).ok()).map(|p| p.knobs).unwrap_or_default();
                Member { holder: m.holder.clone(), filler: m.filler.clone(), place: m.place.clone(), spot: m.spot.clone(), subject, bundle, knobs, prefix: m.slots.clone() }
            })
            .collect();
        self.notes.clone_from(&resolved.notes);
        Ok(resolved)
    }

    /// `f` made on a copy of the document; refused with the core's words when the copy has a problem the document did not
    /// have – its checks, and whether it still resolves (no rigs needed for that)
    pub(crate) fn change(&mut self, f: impl FnOnce(&mut SceneDocument) -> Result<(), String>) -> Result<(), String> {
        let problems = |d: &SceneDocument| resolve(d, &BTreeMap::new(), true, &no_rigs, &no_sets).err().unwrap_or_default();
        let before = problems(&self.doc);
        let mut next = self.doc.clone();
        f(&mut next)?;
        let new: Vec<String> = problems(&next).into_iter().filter(|p| !before.contains(p)).collect();
        if !new.is_empty() {
            return Err(new.join("; "));
        }
        self.doc = next;
        Ok(())
    }

    /// the document's key (`scene:<id>`)
    pub fn key(&self) -> String {
        format!("scene:{}", self.doc.id)
    }

    /// The scene as edited now, for the stage (its slots play their previews). Err: the core's reasons it does not play.
    pub fn resolved(&self, rigs: Rigs, sets: Sets) -> Result<Resolved, Vec<String>> {
        resolve(&self.doc, &BTreeMap::new(), true, rigs, sets)
    }

    /// the tree: the scene, its authored members in the scene's order, its slots, its video nodes
    pub fn tree(&self) -> Vec<Line> {
        let name = if self.doc.stage.name.is_empty() { String::new() } else { self.doc.stage.name.clone() };
        let mut out = vec![Line { depth: 0, label: self.key(), detail: name, part: Part::Scene }];
        let placeholder = |id: &str| self.doc.holders.iter().any(|h| h.id == id && h.is_placeholder());
        for (i, m) in self.members.iter().enumerate().filter(|(_, m)| !placeholder(&m.holder)) {
            let kind = m.place.split_once(':').map_or("", |(k, _)| k);
            let detail = match (&m.subject, &m.bundle) {
                (Some(_), Some(_)) => kind.to_string(),
                (Some(_), None) => "rig not at hand".into(),
                (None, _) => format!("{kind}, no rig"),
            };
            out.push(Line { depth: 1, label: m.holder.clone(), detail, part: Part::Member(i) });
        }
        out.extend(self.slot_lines());
        out.extend(self.video_lines());
        out
    }

    /// whether the document differs from the one read or last saved
    pub fn changed(&self) -> bool {
        self.doc != self.saved
    }

    /// saved as version `version`: the next save goes on it
    pub fn saved_as(&mut self, version: u64) {
        self.base = Some(version);
        self.saved = self.doc.clone();
    }

    fn holder_values(&self, m: usize) -> Option<&BTreeMap<String, HolderValue>> {
        let id = &self.members.get(m)?.holder;
        self.doc.holders.iter().find(|h| &h.id == id).map(|h| &h.values)
    }

    /// member `m`'s knobs with the values the scene gives them
    pub fn knobs(&self, m: usize) -> Vec<SceneKnob> {
        let Some(member) = self.members.get(m) else { return Vec::new() };
        let set = self.positions(m);
        member
            .knobs
            .iter()
            .map(|k| match set.iter().find(|(s, _)| *s == k.path) {
                Some((_, v)) => SceneKnob { knob: k.clone(), value: *v, set: true },
                None => SceneKnob { knob: k.clone(), value: k.value, set: false },
            })
            .collect()
    }

    /// the numbers the scene sets on member `m`, as its rig's slots (`tone_osc.freq`): what its rig plays alone with
    pub fn positions(&self, m: usize) -> Vec<(String, f32)> {
        let Some(values) = self.holder_values(m) else { return Vec::new() };
        values
            .iter()
            .filter_map(|(key, v)| match (Address::leaf(key).ok()?, v) {
                (leaf @ Leaf::Port { track: Some(_), .. }, HolderValue::Number(x)) => Some((leaf.slot()?, *x)),
                _ => None,
            })
            .collect()
    }

    /// Member `m`'s knob `slot` set to `value` in the scene; the rig's own value takes it out again. Err: no knob of its
    /// rig, a value outside its range, or a knob a holder's value cannot name.
    pub fn set(&mut self, m: usize, slot: &str, value: f32) -> Result<(), String> {
        let member = self.members.get(m).ok_or("no such member")?;
        let who = format!("{} ({})", member.holder, member.filler);
        let knob = member.knobs.iter().find(|k| k.path == slot).ok_or_else(|| format!("{who}: its rig has no knob {slot}"))?;
        if !value.is_finite() || value < knob.min || value > knob.max {
            return Err(format!("{who}: {slot} goes from {} to {}", knob.min, knob.max));
        }
        let rig = member.bundle.as_ref().map(|b| &b.rig).ok_or_else(|| format!("{who}: its rig is not at hand"))?;
        let key = value_key(rig, slot).ok_or_else(|| format!("{who}: {slot} is no knob of a track – a scene sets a track's knobs only"))?;
        let own = knob.value;
        let id = member.holder.clone();
        let holder = self.doc.holders.iter_mut().find(|h| h.id == id).ok_or_else(|| format!("{who}: no holder"))?;
        if (value - own).abs() <= f32::EPSILON * own.abs().max(1.0) {
            holder.values.remove(&key);
        } else {
            holder.values.insert(key, HolderValue::Number(value));
        }
        Ok(())
    }

    /// member `m`'s knob `slot` back to its rig's own value
    pub fn clear(&mut self, m: usize, slot: &str) {
        let Some(member) = self.members.get(m) else { return };
        let Some(key) = member.bundle.as_ref().and_then(|b| value_key(&b.rig, slot)) else { return };
        let id = member.holder.clone();
        if let Some(h) = self.doc.holders.iter_mut().find(|h| h.id == id) {
            h.values.remove(&key);
        }
    }

    /// The knob positions of member `m`'s rig as an editor turned them (its slots) into the scene: how many differ from
    /// what the scene had, and why the others were refused.
    pub fn take(&mut self, m: usize, positions: &[(String, f32)]) -> (usize, Vec<String>) {
        let before = self.holder_values(m).cloned();
        let mut refused = Vec::new();
        for (slot, v) in positions {
            if let Err(why) = self.set(m, slot, *v) {
                refused.push(why);
            }
        }
        let after = self.holder_values(m).cloned().unwrap_or_default();
        let before = before.unwrap_or_default();
        let changed = after.iter().filter(|(k, v)| before.get(*k) != Some(*v)).count() + before.keys().filter(|k| !after.contains_key(*k)).count();
        (changed, refused)
    }
}

/// A rig's slot (`tone_osc.freq`) as a holder's value names it (`tone:osc.freq`): the track whose id prefixes the node.
/// None: no track's node (a code rig's `policy.<param>`).
pub fn value_key(rig: &Rig, slot: &str) -> Option<String> {
    let (node, port) = slot.split_once('.')?;
    rig.tracks.iter().filter_map(|t| Some((t, node.strip_prefix(t.id.as_str())?.strip_prefix('_')?))).find(|(_, n)| !n.is_empty()).map(|(t, n)| format!("{}:{n}.{port}", t.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_resolve::{core_rigs, no_sets};

    fn duet() -> SceneDocument {
        let mut d = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        d.recast(&["character:catpope".into(), "item:incense_burner".into()]);
        d
    }

    #[test]
    fn a_scene_opens_with_its_cast_and_their_rigs() {
        let (edit, resolved) = SceneEdit::open(duet(), Some(7), &core_rigs, &no_sets).expect("it plays");
        assert_eq!(edit.members.len(), 2);
        assert!(edit.members.iter().all(|m| m.bundle.is_some() && !m.knobs.is_empty()), "both core rigs at hand");
        assert_eq!(edit.members[0].prefix.as_deref(), Some("catpope_"));
        let lines: Vec<_> = edit.tree().into_iter().map(|l| (l.depth, l.label, l.part)).collect();
        assert_eq!(lines[0], (0, "scene:duet".into(), Part::Scene));
        assert_eq!(lines[1], (1, "catpope".into(), Part::Member(0)));
        assert_eq!(edit.tree()[1].detail, "character");
        assert_eq!(lines[2].2, Part::Member(1));
        assert!(!edit.changed());
        assert!(!resolved.scene.nodes.is_empty());
        let none = |_: &str, _: Option<&str>| None;
        let (silent, _) = SceneEdit::open(duet(), None, &none, &no_sets).unwrap();
        assert_eq!(silent.tree()[1].detail, "rig not at hand");
        assert!(silent.notes.iter().any(|n| n.contains("stands silent")));
    }

    #[test]
    fn a_members_values_are_its_rigs_knobs_in_range_stored_on_its_holder() {
        let (mut edit, _) = SceneEdit::open(duet(), Some(7), &core_rigs, &no_sets).unwrap();
        let k = edit.members[0].knobs[0].clone();
        let key = value_key(&edit.members[0].bundle.as_ref().unwrap().rig, &k.path).expect("a track's knob");
        assert_eq!(Address::leaf(&key).unwrap().slot().as_deref(), Some(k.path.as_str()), "the key names the slot back");
        let v = k.min + (k.max - k.min) * 0.25;
        edit.set(0, &k.path, v).unwrap();
        assert!(edit.changed());
        assert_eq!(edit.doc.holders[0].values.get(&key), Some(&HolderValue::Number(v)));
        assert_eq!(edit.positions(0), [(k.path.clone(), v)]);
        assert!(edit.knobs(0)[0].set);
        assert!(edit.doc.check().is_empty(), "{:?}", edit.doc.check());
        let played = edit.resolved(&core_rigs, &no_sets).expect("still plays");
        let (node, port) = k.path.split_once('.').unwrap();
        let n = played.scene.nodes.iter().find(|n| n.id == format!("catpope_{node}")).expect("the member's node");
        assert_eq!(n.params.get(port).and_then(toml::Value::as_float).map(|x| x as f32), Some(v), "the stage plays the scene's value");
        assert!(edit.set(0, &k.path, k.max + 1.0).unwrap_err().contains("goes from"));
        assert!(edit.set(0, "nothing_here.x", 0.0).unwrap_err().contains("no knob"));
        edit.set(0, &k.path, k.value).unwrap();
        assert!(edit.doc.holders[0].values.is_empty(), "the rig's own value leaves the document");
        assert!(!edit.changed());
        edit.set(0, &k.path, v).unwrap();
        edit.clear(0, &k.path);
        assert!(!edit.changed());
    }

    #[test]
    fn build_s_knob_positions_go_into_the_scene_and_a_save_settles_them() {
        let (mut edit, _) = SceneEdit::open(duet(), Some(7), &core_rigs, &no_sets).unwrap();
        let ks = edit.members[1].knobs.clone();
        let mut positions: Vec<(String, f32)> = ks.iter().map(|k| (k.path.clone(), k.value)).collect();
        positions[0].1 = ks[0].min;
        positions.push(("policy.x".into(), 1.0));
        let (changed, refused) = edit.take(1, &positions);
        assert_eq!(changed, usize::from(ks[0].min != ks[0].value));
        assert_eq!(refused.len(), 1, "{refused:?}");
        edit.saved_as(8);
        assert_eq!(edit.base, Some(8));
        assert!(!edit.changed());
        let back = SceneDocument::parse_json(&edit.doc.to_json().unwrap()).unwrap();
        assert_eq!(back, edit.doc, "the values survive the document's JSON");
    }
}
