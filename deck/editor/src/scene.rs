//! Build edits a scene as well as a rig: the scene on the stage, its cast in the tree, the inspector of a member (its rig,
//! its spot, its knobs as the scene sets them), a member opened as its rig one level down, a save = the scene's draft.
//! The document, its members and the rule for their values are the core's (deck-space scene_edit.rs); here the rigs are
//! looked up (the toon's catalog, what scene-content gave the cached copy, the deck's core rigs) and the panel is drawn.
//! Slots and video nodes: scene_parts.rs.
use crate::rigs::Catalog;
use crate::show::build::SceneOpen;
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_space::published::Opened;
use deck_space::rig::bundle::Bundle;
use deck_space::scene::Scene;
use deck_space::scene_edit::{Part, SceneEdit};
use deck_space::scene_resolve::core_rigs;
use std::sync::Arc;

/// A scene open in Build.
pub struct SceneView {
    pub edit: SceneEdit,
    kept: Option<Opened>,
    catalog: Option<Arc<Catalog>>,
    pub sel: Part,
    /// the member opened as its rig (the stage plays its rig alone)
    pub inside: Option<usize>,
    /// what the tree's first lines stand for this frame
    pub rows: Vec<Part>,
    /// the stage plays it as edited (Show's sheet may have played another meanwhile)
    pub shown: bool,
    /// it saves itself as the scene's draft; a save carries back the document sent
    pub auto: crate::autosave::Autosave<deck_space::scene_document::SceneDocument>,
    /// where the stage drew its slots and video nodes this frame
    pub marks: Vec<(Rect, crate::scene_stage::Mark)>,
    /// the DOP panel's selection and the move it is making
    pub dop: crate::dop::Sel,
}

/// `bundle` with these knob values written into its tracks (a slot no track's node names is left)
fn with_values(bundle: &Bundle, positions: &[(String, f32)]) -> Bundle {
    let mut out = bundle.clone();
    for (slot, v) in positions {
        let Some(key) = deck_space::scene_edit::value_key(&out.rig, slot) else { continue };
        let Some((track, local)) = key.split_once(':') else { continue };
        if let Ok(b) = crate::wires::set_value(&out, track, local, *v) {
            out = b;
        }
    }
    out
}

/// a member's rig: the toon's catalog, else scene-content's answer the cache kept, else the deck's core rig
fn rig_from(catalog: Option<&Catalog>, kept: Option<&Opened>, subject: &str, version: Option<&str>) -> Option<Bundle> {
    catalog
        .and_then(|c| c.rig(subject).and_then(|e| c.bundle(e.content.clone()).ok()))
        .or_else(|| kept.and_then(|k| k.rig(subject)))
        .or_else(|| core_rigs(subject, version))
}

impl SceneView {
    /// The scene opened: the view and what the stage plays, with the core's notes (and the opening's own). Err: the
    /// core's reasons it does not play.
    pub fn open(o: SceneOpen, catalog: Option<Arc<Catalog>>) -> Result<(SceneView, Scene, Vec<String>), String> {
        let SceneOpen { doc, base, kept, note, save } = o;
        let id = doc.id.clone();
        let (cat, k) = (catalog.as_deref(), kept.as_ref());
        let rig = |s: &str, v: Option<&str>| rig_from(cat, k, s, v);
        let set = |l: &str| k.and_then(|k| k.set(l));
        let (edit, resolved) = SceneEdit::open(doc, base, &rig, &set).map_err(|p| format!("scene:{id}: {}", p.join("; ")))?;
        let notes = note.into_iter().chain(resolved.notes).collect();
        Ok((SceneView { edit, kept, catalog, sel: Part::Scene, inside: None, rows: Vec::new(), shown: true, auto: save.unwrap_or_else(|| crate::autosave::Autosave::new(base)), marks: Vec::new(), dop: Default::default() }, resolved.scene, notes))
    }

    /// the members again after a slot or a video node changed. Err: the core's reasons it does not play.
    pub fn refresh(&mut self) -> Result<(), String> {
        let (cat, k) = (self.catalog.as_deref(), self.kept.as_ref());
        let rig = |s: &str, v: Option<&str>| rig_from(cat, k, s, v);
        let set = |l: &str| k.and_then(|k| k.set(l));
        self.edit.refresh(&rig, &set).map(|_| ()).map_err(|p| format!("{}: {}", self.edit.key(), p.join("; ")))
    }

    /// the scene as edited now, for the stage
    pub fn scene(&self) -> Result<Scene, String> {
        let (cat, k) = (self.catalog.as_deref(), self.kept.as_ref());
        let rig = |s: &str, v: Option<&str>| rig_from(cat, k, s, v);
        let set = |l: &str| k.and_then(|k| k.set(l));
        self.edit.resolved(&rig, &set).map(|r| r.scene).map_err(|p| format!("{}: {}", self.edit.key(), p.join("; ")))
    }

    /// the line in the rig picker's place: the scene, its draft's version, whether it changed
    pub fn line(&self) -> String {
        let version = self.edit.base.map_or_else(|| "never saved".to_string(), |v| format!("draft v{v}"));
        format!("{} · {version}{}", self.edit.key(), if self.edit.changed() { " · changed" } else { "" })
    }

    /// the top bar's title: the scene, and the member open as its rig
    pub fn title(&self) -> String {
        match self.inside.and_then(|m| self.edit.members.get(m)) {
            Some(m) => format!("{} › {}", self.edit.key(), m.holder),
            None => self.edit.key(),
        }
    }

    /// the stage's first crumb: the scene, or the member open as its rig
    pub fn crumb(&self) -> String {
        match self.inside.and_then(|m| self.edit.members.get(m)) {
            Some(m) => m.holder.clone(),
            None => self.edit.key(),
        }
    }

    /// the member selected in the tree
    pub fn member(&self) -> Option<usize> {
        match self.sel {
            Part::Member(m) => Some(m),
            _ => None,
        }
    }
}

impl SceneView {
    /// The tree's first lines and what each one stands for (`rows`): at the scene, the scene and its members; inside a
    /// member, the scene and that member, the rig's own lines follow under it.
    pub fn tree_rows(&mut self) -> Vec<controls::TreeRow> {
        let lines = self.edit.tree();
        let shown: Vec<_> = match self.inside {
            None => lines,
            Some(m) => lines.into_iter().filter(|l| matches!(l.part, Part::Scene) || l.part == Part::Member(m)).collect(),
        };
        self.rows = shown.iter().map(|l| l.part).collect();
        shown
            .into_iter()
            .map(|l| {
                let (open, icon) = match l.part {
                    Part::Scene => (Some(true), "nodes"),
                    Part::Member(m) => (Some(self.inside == Some(m)), "folder"),
                    Part::Slot(_) => (None, "target"),
                    Part::Video(_) => (None, "video"),
                };
                controls::TreeRow { depth: l.depth, label: l.label, detail: l.detail, icon, open, selected: self.inside.is_none() && self.sel == l.part }
            })
            .collect()
    }
}

/// a field of the scene's inspector
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Field {
    /// a knob of the selected member, by its index among the rig's knobs: dragged, it sets the scene's value
    Knob(usize),
    /// back to the rig's own value
    Clear(usize),
    /// the member opened as its rig
    Into,
    /// a slot's filter chip, by its place in the question
    Fact(usize),
    /// a reference for a slot's list, its parent, picked from the book
    Refs,
    Parent,
    /// a slot's preview picked from the book, or taken off
    Fill,
    Empty,
    /// where a video node's picture plays, picked
    Plays,
    /// the slot or the video node taken out
    Remove,
    /// the scene's stage background: a click on its value types it, on its swatch opens the colour picker
    Background,
    BackgroundColour,
}

/// The inspector of the scene or of its selected member; where its fields landed.
pub fn inspect(d: &mut DrawList, t: &Tokens, area: Rect, view: &SceneView, hot: Option<Field>) -> Vec<(Rect, Field)> {
    let mut out = Vec::new();
    let mut y = area.y;
    let e = &view.edit;
    match view.sel {
        Part::Slot(s) => return crate::scene_parts::inspect_slot(d, t, area, view, s),
        Part::Video(v) => return crate::scene_parts::inspect_video(d, t, area, view, v, hot),
        _ => {}
    }
    let Some(m) = view.member() else {
        y += controls::section(d, t, area.x, y, area.w, "SCENE", true);
        controls::row(d, t, area, &mut y, "id", &e.doc.id);
        controls::row(d, t, area, &mut y, "name", &e.doc.stage.name);
        controls::row(d, t, area, &mut y, "toons", &e.doc.toons.join(" "));
        controls::row(d, t, area, &mut y, "cast", &e.members.len().to_string());
        controls::row(d, t, area, &mut y, "version", &view.line());
        let row = crate::inspector::background_row(e.doc.stage.background.as_ref(), "its rigs', else black");
        let (swatch, at) = crate::inspector::background(d, t, area, &mut y, &row);
        out.push((swatch, Field::BackgroundColour));
        out.push((at, Field::Background));
        for n in &e.notes {
            if y + t.size.row > area.y + area.h {
                break;
            }
            d.text(area.x, y + 4.0, t.size.small, &controls::fit(n, (area.w / d.advance(t.size.small)) as usize), t.color.warn);
            y += t.size.small * 1.6;
        }
        return out;
    };
    let Some(member) = e.members.get(m) else { return out };
    y += controls::section(d, t, area.x, y, area.w, "MEMBER", true);
    controls::row(d, t, area, &mut y, "holder", &member.holder);
    controls::row(d, t, area, &mut y, "filler", &member.filler);
    controls::row(d, t, area, &mut y, "spot", member.spot.as_deref().unwrap_or("the scene's spot"));
    let rig = match (&member.subject, &member.bundle) {
        (Some(s), Some(_)) => format!("rig:{s}"),
        (Some(s), None) => format!("rig:{s} – not at hand"),
        (None, _) => "none".into(),
    };
    controls::row(d, t, area, &mut y, "rig", &rig);
    if member.bundle.is_some() {
        if let Some(r) = controls::pills(d, t, area.x, y, t.size.row - 4.0, &["Open as rig"], usize::MAX).into_iter().next() {
            out.push((r, Field::Into));
        }
        y += t.size.row;
    }
    let knobs = e.knobs(m);
    if knobs.is_empty() {
        return out;
    }
    y += controls::section(d, t, area.x, y, area.w, "VALUES IN THE SCENE", false);
    let row = t.size.row;
    for (i, k) in knobs.iter().enumerate() {
        if y + row > area.y + area.h {
            break;
        }
        let x_w = if k.set { row } else { 0.0 };
        let f = crate::fraction(k.value, k.knob.min, k.knob.max, k.knob.log);
        let bar = controls::scrub(d, t, Rect { x: area.x, y, w: area.w - x_w, h: row }, k.knob.label_in("en"), &crate::number(k.value), f, hot == Some(Field::Knob(i)));
        out.push((bar, Field::Knob(i)));
        if k.set {
            let r = Rect { x: area.x + area.w - row, y, w: row, h: row };
            d.text(r.x + row * 0.35, y + (row - t.size.font * 1.25) * 0.5, t.size.font, "×", t.color.muted);
            out.push((r, Field::Clear(i)));
        }
        y += row;
    }
    out
}

impl crate::App {
    /// a scene Show asked Build to edit takes the stage
    pub(crate) fn open_scene(&mut self, o: SceneOpen) {
        self.leave_view();
        let (x, y) = self.cursor;
        self.take_member_out();
        match SceneView::open(o, self.show.compose.catalog.clone()) {
            Ok((view, scene, notes)) => {
                let id = view.edit.doc.id.clone();
                self.drop_scene();
                self.scene = Some(view);
                if let Err(err) = self.stage.play_scene(&id, scene) {
                    return self.say(format!("scene:{id}: {err:#}"), x, y, true);
                }
                self.stage.space.open_on_scene_camera();
                let line = std::iter::once(format!("scene:{id} in Build")).chain(notes).collect::<Vec<_>>().join(" – ");
                self.say(line, x, y, false);
            }
            Err(why) => self.say(why, x, y, true),
        }
    }

    /// the scene as edited plays on the stage again
    pub(crate) fn replay_scene(&mut self) {
        let Some(view) = self.scene.as_mut() else { return };
        view.shown = true;
        let view = &*view;
        let (x, y) = self.cursor;
        let id = view.edit.doc.id.clone();
        match view.scene() {
            Ok(scene) => {
                if let Err(err) = self.stage.play_scene(&id, scene) {
                    self.say(format!("scene:{id}: {err:#}"), x, y, true);
                }
            }
            Err(why) => self.say(why, x, y, true),
        }
    }

    /// a line of the scene's tree pressed: selected; a member pressed twice opens as its rig; the scene from inside a member
    /// takes Build's knobs into the scene and goes back up
    pub(crate) fn scene_row(&mut self, part: Part) {
        let double = self.last_row.is_some_and(|(p, at)| p == part && at.elapsed().as_millis() < 400);
        self.last_row = Some((part, std::time::Instant::now()));
        let Some(inside) = self.scene.as_ref().map(|v| v.inside) else { return };
        match (part, inside) {
            (Part::Scene, Some(_)) => self.scene_up(),
            (Part::Member(m), Some(i)) if m == i => self.sel = crate::inspector::Sel::Rig,
            (Part::Member(m), None) if double => self.scene_into(m),
            (p, _) => {
                if let Some(v) = self.scene.as_mut() {
                    v.sel = p;
                }
            }
        }
    }

    /// member `m` opened as its rig, one level down: the stage plays its rig alone with the scene's values on its knobs
    pub(crate) fn scene_into(&mut self, m: usize) {
        let (x, y) = self.cursor;
        let Some(view) = self.scene.as_mut() else { return };
        let Some(member) = view.edit.members.get(m).cloned() else { return };
        let Some(own) = member.bundle.clone() else { return self.say(format!("{}: its rig is not at hand", member.holder), x, y, true) };
        let bundle = with_values(&own, &view.edit.positions(m));
        (view.inside, view.sel) = (Some(m), Part::Member(m));
        self.stage.path.clear();
        if let Err(err) = self.stage.replay(bundle) {
            return self.say(format!("{err:#}"), x, y, true);
        }
        self.stage.base = own;
        (self.undo, self.sel, self.armed) = (Vec::new(), crate::inspector::Sel::Rig, None);
        // its knobs' turns stay out of the rig's own working copy
        self.draft = crate::rigs::Copy::Nothing;
        if let Some(l) = self.show.build.as_mut() {
            (l.place, l.prefix) = (member.place.clone(), member.prefix.clone().unwrap_or_default());
        }
        self.say(format!("{} · {} as its rig: its knobs go into the scene when you go back up", member.holder, member.filler), x, y, false);
    }

    /// Build's knobs of the member open go into the scene (the core says which it takes); the member leaves the stage
    fn take_member_out(&mut self) -> Option<String> {
        let view = self.scene.as_mut()?;
        let m = view.inside.take()?;
        let member = view.edit.members[m].clone();
        let persona = deck_space::scene_document::FillerRef::parse(&member.filler).ok().and_then(|f| f.persona);
        let positions: Vec<(String, f32)> = self.stage.bundle.play(persona.as_deref(), None).map(|p| p.knobs.into_iter().map(|k| (k.path, k.value)).collect()).unwrap_or_default();
        let (changed, refused) = view.edit.take(m, &positions);
        let own: Vec<(String, f32)> = member.knobs.iter().map(|k| (k.path.clone(), k.value)).collect();
        let rewired = member.bundle.as_ref().is_some_and(|b| with_values(b, &own) != with_values(&self.stage.bundle, &own));
        let mut said = format!("{}: {changed} value{} into the scene", member.holder, if changed == 1 { "" } else { "s" });
        if !refused.is_empty() {
            said.push_str(&format!(" – not taken: {}", refused.join("; ")));
        }
        if rewired {
            said.push_str(" – its rig's wiring is the rig's, not the scene's: open the rig itself to save it");
        }
        if let Some(l) = self.show.build.as_mut() {
            l.place.clone_from(&l.piece);
            l.prefix.clear();
        }
        Some(said)
    }

    /// back up from a member to the scene
    pub(crate) fn scene_up(&mut self) {
        let (x, y) = self.cursor;
        let Some(said) = self.take_member_out() else { return };
        self.stage.path.clear();
        self.replay_scene();
        self.say(said, x, y, false);
    }

    /// the scene's draft on its way (saving.rs): Build's knobs of the member open go into it first
    pub(crate) fn scene_save(&mut self, now: std::time::Instant) {
        let inside = self.scene.as_ref().and_then(|v| v.inside);
        self.take_member_out();
        let Some(view) = self.scene.as_mut() else { return };
        view.auto.look(crate::autosave::fingerprint(&view.edit.doc), now);
        crate::show::saving::send_scene(self.show.api(), &mut view.auto, &view.edit.doc, now);
        if let Some(m) = inside {
            self.scene_into(m);
        }
    }

    /// a scene knob dragged to `x`: the core sets the selected member's value
    pub(crate) fn scene_knob_to(&mut self, i: usize, x0: f32, v0: f32, x: f32) {
        let Some(view) = self.scene.as_mut() else { return };
        if let Part::Video(v) = view.sel {
            return crate::scene_parts::video_knob_to(view, v, i, x - x0, v0);
        }
        let Some(m) = view.member() else { return };
        let Some(k) = view.edit.members.get(m).and_then(|mm| mm.knobs.get(i)).cloned() else { return };
        let v = crate::scrub(v0, x - x0, k.min, k.max, k.log);
        let _ = view.edit.set(m, &k.path, v);
    }

    /// a scene knob let go, or a value cleared: the stage plays the scene as edited
    pub(crate) fn scene_settled(&mut self) {
        self.replay_scene();
    }

    pub(crate) fn scene_clear(&mut self, i: usize) {
        let Some(view) = self.scene.as_mut() else { return };
        if let Part::Video(v) = view.sel {
            if let Some(param) = view.edit.video_knobs(v).get(i).map(|k| k.param.clone()) {
                view.edit.clear_video(v, &param);
            }
            return self.replay_scene();
        }
        let Some(m) = view.member() else { return };
        if let Some(path) = view.edit.members.get(m).and_then(|mm| mm.knobs.get(i)).map(|k| k.path.clone()) {
            view.edit.clear(m, &path);
        }
        self.replay_scene();
    }

    /// a check run's (`--member HOLDER[,inside]`): the member selected, its first knob turned to four fifths as a drag
    /// would, then opened as its rig if asked
    pub(crate) fn script_scene(&mut self) {
        let Some((holder, inside)) = self.args.member.clone() else { return };
        if holder.contains(['+', ':']) {
            return self.script_parts(&holder);
        }
        let Some(m) = self.scene.as_ref().and_then(|v| v.edit.members.iter().position(|x| x.holder == holder)) else { return };
        match self.frames {
            75 => self.scene_row(Part::Member(m)),
            80 => {
                if let Some(k) = self.scene.as_ref().and_then(|v| v.edit.members[m].knobs.first().cloned()) {
                    let x0 = self.cursor.0;
                    let v0 = self.scene_knob(0).unwrap_or(k.value);
                    let target = k.min + (k.max - k.min) * 0.8;
                    let dx = (crate::fraction(target, k.min, k.max, k.log) - crate::fraction(v0, k.min, k.max, k.log)) * crate::SCRUB_PX;
                    self.scene_knob_to(0, x0, v0, x0 + dx);
                    self.scene_settled();
                }
            }
            85 if inside => self.scene_into(m),
            _ => {}
        }
    }

    /// the knob `i` of the selected member as the scene has it (a drag starts from it)
    pub(crate) fn scene_knob(&self, i: usize) -> Option<f32> {
        let view = self.scene.as_ref()?;
        if let Part::Video(v) = view.sel {
            return view.edit.video_knobs(v).get(i).map(|k| k.value);
        }
        view.edit.knobs(view.member()?).get(i).map(|k| k.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_space::scene_document::SceneDocument;

    fn duet() -> SceneOpen {
        let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        doc.recast(&["character:catpope".into(), "item:incense_burner".into()]);
        SceneOpen { doc, base: Some(3), kept: None, note: None, save: None }
    }

    /// the panel model: the scene opens with its cast in the tree, a member selected shows its knobs as the scene sets
    /// them, a value set through the core shows as set and changes the scene the stage plays
    #[test]
    fn a_scene_opens_and_a_members_values_are_edited() {
        let (mut view, scene, _) = SceneView::open(duet(), None).expect("it plays");
        deck_space::graph::Graph::build(&scene).expect("the stage builds it");
        assert_eq!(view.line(), "scene:duet · draft v3");
        let tree = view.edit.tree();
        assert_eq!(tree.iter().map(|l| l.part).collect::<Vec<_>>(), [Part::Scene, Part::Member(0), Part::Member(1)]);
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        let area = Rect { x: 0.0, y: 0.0, w: 320.0, h: 2000.0 };
        let fields: Vec<Field> = inspect(&mut d, &t, area, &view, None).into_iter().map(|(_, f)| f).collect();
        assert_eq!(fields, [Field::BackgroundColour, Field::Background], "the scene's one field: its background, its swatch first");
        view.edit.set_background("#336699").unwrap();
        assert_eq!(view.scene().unwrap().stage.background, Some(deck_space::Background::colour("#336699")), "the stage plays it");
        view.edit.set_background("").unwrap();
        view.sel = Part::Member(0);
        let fields = inspect(&mut d, &t, area, &view, None);
        assert!(fields.iter().any(|(_, f)| *f == Field::Into));
        assert!(fields.iter().any(|(_, f)| *f == Field::Knob(0)) && !fields.iter().any(|(_, f)| matches!(f, Field::Clear(_))));
        let k = view.edit.members[0].knobs[0].clone();
        let v = if k.value == k.min { k.max } else { k.min };
        view.edit.set(0, &k.path, v).unwrap();
        assert!(view.line().ends_with(" · changed"));
        assert!(inspect(&mut d, &t, area, &view, None).iter().any(|(_, f)| *f == Field::Clear(0)));
        let played = view.scene().unwrap();
        let (node, port) = k.path.split_once('.').unwrap();
        let n = played.nodes.iter().find(|n| n.id == format!("catpope_{node}")).unwrap();
        assert_eq!(n.params.get(port).and_then(toml::Value::as_float).map(|x| x as f32), Some(v));
        // one level down the member's rig carries the scene's values; back up unchanged, nothing changes
        let own = view.edit.members[0].bundle.clone().unwrap();
        let down = with_values(&own, &view.edit.positions(0));
        let knobs: Vec<(String, f32)> = down.play(None, None).unwrap().knobs.into_iter().map(|k| (k.path, k.value)).collect();
        assert!(knobs.contains(&(k.path.clone(), v)));
        assert_eq!(view.edit.take(0, &knobs), (0, Vec::new()));
        let rig: Vec<(String, f32)> = view.edit.members[0].knobs.iter().map(|k| (k.path.clone(), k.value)).collect();
        assert_eq!(with_values(&own, &rig), with_values(&down, &rig), "a knob turned is no rewiring");
    }
}
