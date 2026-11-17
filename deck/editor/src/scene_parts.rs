//! Build's scene: its slots (placeholders) and its own video nodes. A slot's filter is drawn by the one condition control
//! – its kinds as chips, a hand-picked list and a parent picked from the toon's address book – and its preview is a pick
//! from the book; a video node's knobs are scrub bars, where it plays a pick of the free video inputs. Every rule – what
//! a slot admits, which kinds a scene adds, the ranges – is the core's (deck-space scene_slots.rs, scene_video.rs); a
//! refusal is said in the core's words.
use crate::scene::{Field, SceneView};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::typeahead::{Item, Typeahead};
use deck_space::scene_document::{FillerRef, HOLDER_KINDS};
use deck_space::scene_edit::Part;
use deck_catalog::layer::Anchor;
use deck_space::scene_layers::{depth_words, layer_name};
use deck_space::scene_slots::{fill_words, FillFact};
use deck_space::scene_video::{about, video_kinds, SCREEN};
use protocol::address_book::AddressKind;

/// what a pick in the scene's typeahead does
pub enum Pick {
    /// Build's Add at the scene: a slot of a kind, or a video node of a kind
    Add(Vec<Adding>),
    /// a book entry for slot `.0`: its preview, a reference of its list, its parent
    Book(usize, Ask, Vec<String>),
    /// where a video node plays: the targets offered
    Plays(usize, Vec<String>),
}

/// what a slot asks the book for
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ask {
    Fill,
    Ref,
    Parent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Adding {
    Slot(AddressKind),
    Video(&'static str),
}

/// the rows of Build's Add at the scene
pub fn adding() -> (Vec<Adding>, Vec<Item>) {
    let mut list: Vec<Adding> = HOLDER_KINDS.iter().map(|k| Adding::Slot(*k)).collect();
    list.extend(video_kinds().into_iter().map(Adding::Video));
    let items = list
        .iter()
        .enumerate()
        .map(|(id, a)| match a {
            Adding::Slot(k) => Item { id, label: format!("slot · {}", k.as_str()), detail: "a placeholder: filled by a use, previewed here".into() },
            Adding::Video(k) => Item { id, label: format!("video · {k}"), detail: about(k).to_string() },
        })
        .collect();
    (list, items)
}

/// pills on one line; where each landed with its field
fn pills(d: &mut DrawList, t: &Tokens, area: Rect, y: &mut f32, names: &[&str], fields: &[Field], out: &mut Vec<(Rect, Field)>) {
    for (r, f) in controls::pills(d, t, area.x, *y, t.size.row - 4.0, names, usize::MAX).into_iter().zip(fields) {
        out.push((r, *f));
    }
    *y += t.size.row;
}

/// The inspector of slot `s`: its id and spot, its filter as the condition control, its preview.
pub fn inspect_slot(d: &mut DrawList, t: &Tokens, area: Rect, view: &SceneView, s: usize) -> Vec<(Rect, Field)> {
    let mut out = Vec::new();
    let Some(h) = view.edit.doc.holders.get(s) else {
        return out;
    };
    let mut y = area.y;
    y += controls::section(d, t, area.x, y, area.w, "SLOT", true);
    controls::row(d, t, area, &mut y, "id", &h.id);
    controls::row(d, t, area, &mut y, "spot", h.spot.as_deref().unwrap_or("the scene's spot"));
    let takes = match (h.take, h.max) {
        (deck_space::scene_document::Take::One, _) => "one".to_string(),
        (_, Some(m)) => format!("many, at most {m}"),
        _ => "many".to_string(),
    };
    controls::row(d, t, area, &mut y, "takes", &takes);
    if let Some(q) = view.edit.question(s, &[]) {
        let (names, on) = q.chips();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let (chips, used) = crate::show::condition::chips(d, t, (area.x, y, area.w), "fill", &q.words, &names, &on);
        out.extend(chips.into_iter().enumerate().map(|(i, r)| (r, Field::Fact(i))));
        y += used;
    }
    pills(d, t, area, &mut y, &["One of", "Below"], &[Field::Refs, Field::Parent], &mut out);
    let preview = h.preview.as_ref().map_or_else(|| "empty – its spot stays empty".to_string(), |p| p.list().join(", "));
    controls::row(d, t, area, &mut y, "preview", &preview);
    pills(d, t, area, &mut y, &["Remove", "Empty", "Fill"], &[Field::Remove, Field::Empty, Field::Fill], &mut out);
    out
}

/// The inspector of video node `v`: its kind, where it plays, its knobs as the scene sets them.
pub fn inspect_video(d: &mut DrawList, t: &Tokens, area: Rect, view: &SceneView, v: usize, hot: Option<Field>) -> Vec<(Rect, Field)> {
    let mut out = Vec::new();
    let e = &view.edit;
    let Some(n) = e.doc.nodes.get(v) else {
        return out;
    };
    let mut y = area.y;
    y += controls::section(d, t, area.x, y, area.w, "VIDEO", true);
    controls::row(d, t, area, &mut y, "id", &n.id);
    controls::row(d, t, area, &mut y, "kind", &n.kind);
    controls::row(d, t, area, &mut y, "plays at", e.plays_at(v).as_deref().unwrap_or("nowhere"));
    pills(d, t, area, &mut y, &["Remove", "Play at"], &[Field::Remove, Field::Plays], &mut out);
    let knobs = e.video_knobs(v);
    if knobs.is_empty() {
        return out;
    }
    y += controls::section(d, t, area.x, y, area.w, "KNOBS", false);
    let row = t.size.row;
    for (i, k) in knobs.iter().enumerate() {
        if y + row > area.y + area.h {
            break;
        }
        let x_w = if k.set { row } else { 0.0 };
        let f = crate::fraction(k.value, k.min, k.max, k.log);
        let value = format!("{}{}", crate::number(k.value), if k.unit.is_empty() { String::new() } else { format!(" {}", k.unit) });
        let bar = controls::scrub(d, t, Rect { x: area.x, y, w: area.w - x_w, h: row }, &k.param, &value, f, hot == Some(Field::Knob(i)));
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

/// a layer's handle on the stage: its picture's name and its depth
pub fn layer_handle(l: &deck_space::Layer) -> String {
    format!("{} · {}", layer_name(l), depth_words(l.depth))
}

/// The inspector of layer `l`: its picture (typed; empty = an empty place), its anchor, where it lies, Remove, its depth
/// as a scrub bar.
pub fn inspect_layer(d: &mut DrawList, t: &Tokens, area: Rect, view: &SceneView, l: usize, hot: Option<Field>) -> Vec<(Rect, Field)> {
    let mut out = Vec::new();
    let e = &view.edit;
    let Some(layer) = e.doc.stage.layers.get(l) else {
        return out;
    };
    let mut y = area.y;
    y += controls::section(d, t, area.x, y, area.w, "PARALLAX LAYER", true);
    for (name, value, field) in [("picture", layer.image.as_deref().unwrap_or("empty – drop a picture on it"), Field::Picture), ("covers", layer.anchor.word(), Field::Anchor)] {
        let top = y;
        controls::row(d, t, area, &mut y, name, value);
        out.push((Rect { x: area.x, y: top, w: area.w, h: y - top }, field));
    }
    let place = if layer.depth < 0.0 { "in front of the 3D" } else { "behind the 3D" };
    controls::row(d, t, area, &mut y, "lies", &format!("{} · {place}", depth_words(layer.depth)));
    pills(d, t, area, &mut y, &["Remove"], &[Field::Remove], &mut out);
    y += controls::section(d, t, area.x, y, area.w, "KNOBS", false);
    let row = t.size.row;
    for (i, k) in e.layer_knobs(l).iter().enumerate() {
        let f = crate::fraction(k.value, k.min, k.max, k.log);
        let bar = controls::scrub(d, t, Rect { x: area.x, y, w: area.w, h: row }, &k.param, &format!("{} {}", crate::number(k.value), k.unit), f, hot == Some(Field::Knob(i)));
        out.push((bar, Field::Knob(i)));
        y += row;
    }
    out
}

/// a layer's depth knob dragged by `dx` from `v0`: the core moves it
pub fn layer_knob_to(view: &mut SceneView, l: usize, i: usize, dx: f32, v0: f32) {
    let Some(k) = view.edit.layer_knobs(l).get(i).cloned() else {
        return;
    };
    let _ = view.edit.move_layer(l, crate::scrub(v0, dx, k.min, k.max, k.log));
}

impl crate::App {
    /// layer `l`'s picture as typed: a media path, or empty = an empty place again
    pub(crate) fn scene_layer(&mut self, l: usize, text: &str) {
        let (x, y) = self.cursor;
        let Some(view) = self.scene.as_mut() else {
            return;
        };
        let path = Some(text.trim()).filter(|p| !p.is_empty());
        let done = view.edit.fill_layer(l, path);
        self.scene_changed(done, path.map_or_else(|| "the layer is an empty place".to_string(), |p| format!("{p} fills the layer")), x, y);
    }
}

/// a video knob dragged by `dx` from `v0`: the core sets it
pub fn video_knob_to(view: &mut SceneView, v: usize, i: usize, dx: f32, v0: f32) {
    let Some(k) = view.edit.video_knobs(v).get(i).cloned() else {
        return;
    };
    let x = crate::scrub(v0, dx, k.min, k.max, k.log);
    let _ = view.edit.set_video(v, &k.param, x);
}

/// the fillers offered without a book: what the deck's core rigs stand for
fn fallback_fillers() -> Vec<String> {
    deck_space::rig::bundle::CORE.iter().filter_map(|(s, _)| FillerRef::of_rig(s)).collect()
}

impl crate::App {
    fn scene_menu(&mut self, ta: Typeahead, pick: Pick) {
        self.menu = Some(crate::Menu { ta, from: None, far: None, rigs: false, add: Vec::new(), far_new: None, scene: Some(pick) });
    }

    /// Build's Add at the scene: a slot or a video node
    pub(crate) fn scene_add(&mut self, x: f32, y: f32) {
        let (list, items) = adding();
        self.scene_menu(Typeahead::new(items, x, y), Pick::Add(list));
    }

    /// a change the core made or refused: the members again and the stage, or the core's words
    pub(crate) fn scene_changed(&mut self, done: Result<(), String>, said: String, x: f32, y: f32) {
        let Some(view) = self.scene.as_mut() else {
            return;
        };
        match done.and_then(|()| view.refresh()) {
            Ok(()) => {
                self.replay_scene();
                self.say(said, x, y, false);
            }
            Err(why) => self.say(why, x, y, true),
        }
    }

    /// a field of a slot's or a video node's inspector pressed
    pub(crate) fn scene_field(&mut self, f: Field, x: f32, y: f32) {
        let Some(view) = self.scene.as_mut() else {
            return;
        };
        match (f, view.sel) {
            (Field::Fact(i), Part::Slot(s)) => {
                let Some(fact) = view.edit.question(s, &[]).and_then(|q| q.facts.get(i).map(|(f, _)| f.clone())) else {
                    return;
                };
                let done = view.edit.flip(s, &fact);
                self.scene_changed(done, format!("fill: {}", fact.word()), x, y);
            }
            (Field::Fill, Part::Slot(s)) => self.scene_book(s, Ask::Fill, x, y),
            (Field::Refs, Part::Slot(s)) => self.scene_book(s, Ask::Ref, x, y),
            (Field::Parent, Part::Slot(s)) => self.scene_book(s, Ask::Parent, x, y),
            (Field::Empty, Part::Slot(s)) => {
                let done = view.edit.empty(s);
                self.scene_changed(done, "the slot is empty: its spot stays empty".into(), x, y);
            }
            (Field::Plays, Part::Video(v)) => {
                let targets = view.edit.targets(v);
                let items = targets.iter().enumerate().map(|(id, t)| Item { id, label: t.clone(), detail: if t == SCREEN { "the window".into() } else { "a video input".into() } }).collect();
                self.scene_menu(Typeahead::new(items, x, y), Pick::Plays(v, targets));
            }
            (Field::Remove, Part::Slot(s)) => {
                let done = view.edit.remove_slot(s);
                view.sel = Part::Scene;
                self.scene_changed(done, "the slot left the scene".into(), x, y);
            }
            (Field::Remove, Part::Video(v)) => {
                let done = view.edit.remove_video(v);
                view.sel = Part::Scene;
                self.scene_changed(done, "the video node left the scene".into(), x, y);
            }
            (Field::Remove, Part::Layer(l)) => {
                let done = view.edit.remove_layer(l);
                view.sel = Part::Scene;
                self.scene_changed(done, "the layer left the stage".into(), x, y);
            }
            (Field::Picture, Part::Layer(l)) => {
                let now = view.edit.doc.stage.layers.get(l).and_then(|layer| layer.image.clone()).unwrap_or_default();
                self.begin_edit(crate::Editing::SceneLayer(l), crate::Hit::SceneField(Field::Picture), now);
            }
            (Field::Anchor, Part::Layer(l)) => {
                let Some(now) = view.edit.doc.stage.layers.get(l).map(|layer| layer.anchor) else { return };
                let next = Anchor::ALL[(Anchor::ALL.iter().position(|a| *a == now).unwrap_or(0) + 1) % Anchor::ALL.len()];
                let done = view.edit.anchor_layer(l, next);
                self.scene_changed(done, format!("the layer covers {} of the view", if next == Anchor::Full { "all" } else { next.word() }), x, y);
            }
            (Field::AddStack, Part::Scene) => {
                let done = view.edit.add_stack();
                let said = done.as_ref().map_or_else(|_| String::new(), |n| format!("the parallax stack: {n} empty layers – drop pictures on their handles"));
                self.scene_changed(done.map(|_| ()), said, x, y);
            }
            (Field::AddLayer, Part::Scene) => {
                let done = view.edit.add_layer(4.0, Anchor::Full);
                if let Ok(l) = done {
                    view.sel = Part::Layer(l);
                }
                self.scene_changed(done.map(|_| ()), "an empty layer 4 m behind".into(), x, y);
            }
            (Field::BackgroundColour, Part::Scene) => self.colour_open(crate::colour::Target::Scene),
            (Field::Background, Part::Scene) => {
                let now = view.edit.doc.stage.background.as_ref().map_or("", |b| b.text()).to_string();
                self.begin_edit(crate::Editing::SceneBackground, crate::Hit::SceneField(Field::Background), now);
            }
            _ => {}
        }
    }

    /// The book's entries slot `s` asks for – of the kinds it takes (a preview, a reference of its list) or a character or
    /// persona (its parent) – with the core's sentence beside a preview it does not admit; without a book what the deck's
    /// core rigs stand for, and why inside.
    fn scene_book(&mut self, s: usize, ask: Ask, x: f32, y: f32) {
        self.book.poll();
        let why = self.book_why();
        let Some(fill) = self.scene.as_ref().and_then(|v| v.edit.doc.holders.get(s)?.fill.clone()) else {
            return;
        };
        let kinds = match ask {
            Ask::Parent => vec![AddressKind::Character, AddressKind::Persona],
            _ => fill.kind.clone(),
        };
        let (refs, labels): (Vec<String>, Vec<String>) = match self.book.book.as_ref() {
            Some(b) => b.find("", &kinds, crate::LOCALE).into_iter().filter_map(|i| Some((b.entry(i)?.r#ref.clone(), b.chain(i, crate::LOCALE)))).unzip(),
            None => fallback_fillers().into_iter().filter(|r| FillerRef::parse(r).is_ok_and(|f| kinds.contains(&f.kind))).map(|r| (r.clone(), r)).unzip(),
        };
        let detail = |r: &String| match ask {
            Ask::Fill => fill.admits(r).err().unwrap_or_else(|| r.clone()),
            Ask::Ref if fill.refs.contains(r) => format!("{r} · on the list"),
            Ask::Parent if fill.parent.as_ref() == Some(r) => format!("{r} · its parent"),
            _ => r.clone(),
        };
        let items = refs.iter().zip(labels).enumerate().map(|(id, (r, label))| Item { id, label, detail: detail(r) }).collect();
        let mut ta = Typeahead::new(items, x, y);
        if let Some(w) = why {
            ta = ta.with_note(w);
        }
        self.scene_menu(ta, Pick::Book(s, ask, refs));
    }

    /// a row of the scene's typeahead chosen
    pub(crate) fn scene_pick(&mut self, pick: Pick, id: usize, x: f32, y: f32) {
        let Some(view) = self.scene.as_mut() else {
            return;
        };
        let (done, said) = match pick {
            Pick::Add(list) => match list.get(id) {
                Some(Adding::Slot(k)) => match view.edit.add_slot(*k) {
                    Ok(s) => {
                        view.sel = Part::Slot(s);
                        (Ok(()), format!("a slot for {} – fill it to preview it", fill_words(view.edit.doc.holders[s].fill.as_ref().expect("a slot"))))
                    }
                    Err(e) => (Err(e), String::new()),
                },
                Some(Adding::Video(k)) => match view.edit.add_video(k) {
                    Ok(v) => {
                        view.sel = Part::Video(v);
                        let at = view.edit.plays_at(v).unwrap_or_else(|| "nowhere yet – Play at".into());
                        (Ok(()), format!("{k} plays at {at}"))
                    }
                    Err(e) => (Err(e), String::new()),
                },
                None => return,
            },
            Pick::Book(s, ask, refs) => {
                let Some(r) = refs.get(id).cloned() else {
                    return;
                };
                match ask {
                    Ask::Fill => (view.edit.fill(s, &r), format!("previewed by {r}")),
                    Ask::Ref => (view.edit.flip(s, &FillFact::Ref(r.clone())), format!("fill: one of … {r}")),
                    Ask::Parent => (view.edit.flip(s, &FillFact::Parent(r.clone())), format!("fill: below {r}")),
                }
            }
            Pick::Plays(v, targets) => {
                let Some(target) = targets.get(id) else {
                    return;
                };
                (view.edit.play_at(v, target), format!("plays at {target}"))
            }
        };
        self.scene_changed(done, said, x, y);
    }

    /// A check run's `--member` steps, `+` between: `slot:<kind>[=<filler>]` adds a slot (previewed), `video:<kind>` a
    /// video node, any other word selects that slot or video node by its id.
    pub(crate) fn script_parts(&mut self, spec: &str) {
        if self.frames != 75 {
            return;
        }
        let (x, y) = self.cursor;
        let (mut orbit, mut camera) = (None, false);
        for step in spec.split('+').map(str::trim) {
            let Some(view) = self.scene.as_mut() else {
                return;
            };
            let done = if step == "camera" {
                camera = true;
                Ok(())
            } else if step == "stack" {
                view.edit.add_stack().map(|_| ())
            } else if let Some(fill) = step.strip_prefix("fill:") {
                let (l, path) = fill.split_once('=').unwrap_or((fill, ""));
                l.parse::<usize>().map_err(|e| e.to_string()).and_then(|l| view.edit.fill_layer(l, Some(path)).map(|()| view.sel = Part::Layer(l)))
            } else if let Some(depth) = step.strip_prefix("layer:") {
                depth.parse::<f32>().map_err(|e| e.to_string()).and_then(|d| view.edit.add_layer(d, Anchor::Full)).map(|l| view.sel = Part::Layer(l))
            } else if let Some(typed) = step.strip_prefix("background:") {
                view.edit.set_background(typed).map(|_| ())
            } else if let Some(by) = step.strip_prefix("orbit:") {
                orbit = by.split_once(',').and_then(|(dx, dy)| Some((dx.trim().parse::<f32>().ok()?, dy.trim().parse::<f32>().ok()?)));
                Ok(())
            } else if let Some(rest) = step.strip_prefix("slot:") {
                let (kind, filler) = rest.split_once('=').map_or((rest, None), |(k, f)| (k, Some(f)));
                let kind = HOLDER_KINDS.iter().find(|k| k.as_str() == kind).copied().ok_or(format!("no slot kind {kind}"));
                kind.and_then(|k| view.edit.add_slot(k)).and_then(|s| filler.map_or(Ok(()), |f| view.edit.fill(s, f)))
            } else if let Some(kind) = step.strip_prefix("video:") {
                view.edit.add_video(kind).map(|_| ())
            } else {
                let e = &view.edit;
                let slot = e.doc.holders.iter().position(|h| h.id == step && h.is_placeholder()).map(Part::Slot);
                let video = e.videos().into_iter().find(|&i| e.doc.nodes[i].id == step).map(Part::Video);
                view.sel = slot.or(video).unwrap_or(Part::Scene);
                Ok(())
            };
            if let Err(why) = done {
                self.say(why, x, y, true);
            }
        }
        self.scene_changed(Ok(()), "the scene's slots, video and layers".into(), x, y);
        if camera {
            self.stage.space.look_through_scene();
        }
        if let Some((dx, dy)) = orbit {
            self.stage.space.camera.drag(dx, dy);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::show::build::SceneOpen;
    use deck_space::scene_document::SceneDocument;

    fn duet() -> SceneOpen {
        let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        doc.recast(&["character:catpope".into()]);
        SceneOpen { doc, base: Some(3), kept: None, note: None, save: None }
    }

    /// the panel model: Add offers the slot kinds and the built video kinds; a slot added shows its filter as chips and its
    /// preview, filled through the core it plays; a video node shows where it plays and its knobs
    #[test]
    fn a_slot_and_a_video_node_in_the_scenes_panel() {
        let (list, items) = adding();
        assert_eq!(list.len(), items.len());
        assert!(items.iter().any(|i| i.label == "slot · character") && items.iter().any(|i| i.label == "video · test_pattern"));
        let (mut view, _, _) = SceneView::open(duet(), None).expect("it plays");
        let s = view.edit.add_slot(AddressKind::Item).unwrap();
        view.sel = Part::Slot(s);
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        let area = Rect { x: 0.0, y: 0.0, w: 320.0, h: 2000.0 };
        let fields = crate::scene::inspect(&mut d, &t, area, &view, None);
        let facts = fields.iter().filter(|(_, f)| matches!(f, Field::Fact(_))).count();
        assert_eq!(facts, HOLDER_KINDS.len(), "one chip a kind, nothing named yet");
        assert!([Field::Fill, Field::Empty, Field::Remove, Field::Refs, Field::Parent].iter().all(|f| fields.iter().any(|(_, x)| x == f)));
        view.edit.flip(s, &FillFact::Ref("item:incense_burner".into())).unwrap();
        let named = crate::scene::inspect(&mut d, &t, area, &view, None).iter().filter(|(_, f)| matches!(f, Field::Fact(_))).count();
        assert_eq!(named, HOLDER_KINDS.len() + 1, "a reference on the list is a chip");
        assert!(view.edit.fill(s, "character:catpope").is_err(), "an item slot refuses a character");
        view.edit.fill(s, "item:incense_burner").unwrap();
        view.refresh().unwrap();
        let played = view.scene().unwrap();
        assert!(played.nodes.iter().any(|n| n.id.starts_with("slot_")), "the preview's rig plays: {:?}", played.nodes.iter().map(|n| &n.id).collect::<Vec<_>>());
        let v = view.edit.add_video("test_pattern").unwrap();
        view.sel = Part::Video(v);
        let fields = crate::scene::inspect(&mut d, &t, area, &view, None);
        assert!(fields.iter().any(|(_, f)| *f == Field::Plays) && fields.iter().any(|(_, f)| *f == Field::Knob(0)));
        let k = view.edit.video_knobs(v)[0].clone();
        video_knob_to(&mut view, v, 0, crate::SCRUB_PX, k.value);
        assert!(view.edit.video_knobs(v)[0].set, "a drag sets the scene's value");
        assert!(crate::scene::inspect(&mut d, &t, area, &view, None).iter().any(|(_, f)| *f == Field::Clear(0)));
        deck_space::graph::Graph::build(&view.scene().unwrap()).expect("the stage builds it");
    }
}
