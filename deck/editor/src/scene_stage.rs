//! Build's scene on the stage: each slot marked where it stands – an empty one a dashed card with its filter, a filled
//! one a tag under its preview's feet – and the scene's video nodes as cards; a press on a mark selects it, a library
//! card let go on a slot fills it (elsewhere on the stage: the first slot that admits it). The parallax layers are
//! outlined where they stand in the 3D, each with a handle: pressed it selects the layer, dragged up or down it moves it
//! in depth, a picture let go on it fills it. Where a slot or a layer stands, what it admits and every refusal are the
//! core's (deck-space place.rs, scene_slots.rs, space/layers.rs, scene_layers.rs).
use crate::scene::SceneView;
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_space::scene_edit::Part;
use deck_space::scene_video::SCREEN;
use deck_space::space::Space;

/// what a mark on the stage stands for
// TODO(#582): the trolley track becomes a mark of its own
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mark {
    Slot(usize),
    Video(usize),
    /// a parallax layer's handle
    Layer(usize),
}

impl Mark {
    pub fn part(self) -> Part {
        match self {
            Mark::Slot(s) => Part::Slot(s),
            Mark::Video(v) => Part::Video(v),
            Mark::Layer(l) => Part::Layer(l),
        }
    }
}

const DASH: f32 = 6.0;

fn alpha(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

/// a label and a muted line centred under `x` from `y`: the box they take
fn caption(d: &mut DrawList, t: &Tokens, (x, y): (f32, f32), name: &str, words: &str, color: [f32; 4]) -> Rect {
    let w = controls::label_width(d, t, name).max(d.sans_width(t.size.small, words));
    controls::label(d, t, x - controls::label_width(d, t, name) * 0.5, y, name, color);
    let wy = y + t.size.label * 1.6;
    d.sans(x - d.sans_width(t.size.small, words) * 0.5, wy, t.size.small, words, t.color.muted);
    Rect { x: x - w * 0.5, y, w, h: wy + t.size.small * 1.3 - y }
}

/// The marks of the scene open at the scene's level over the stage (`area`; `scale` = view pixels per logical one);
/// `carry` = where a carried card is: a slot under it lights up. Where each landed goes into `view.marks`.
pub fn draw(d: &mut DrawList, t: &Tokens, view: &mut SceneView, space: &Space, area: Rect, scale: f32, carry: Option<(f32, f32)>) {
    view.marks.clear();
    if view.inside.is_some() {
        return;
    }
    let logical = |x: f32, y: f32| (area.x + x / scale, area.y + y / scale);
    let slots = view.edit.slot_marks();
    let of = slots.iter().filter(|m| m.previews.is_empty()).count();
    let mut k = 0;
    for m in &slots {
        let sel = view.sel == Part::Slot(m.slot);
        if m.previews.is_empty() {
            let at = (k, of);
            k += 1;
            let Some([x, y, w, h]) = space.standee_pixels(m.spot.as_deref(), m.at, at, m.height) else { continue };
            let (x, y) = logical(x, y);
            let r = Rect { x, y, w: w / scale, h: h / scale };
            let hot = carry.is_some_and(|(px, py)| r.has(px, py));
            let line = if sel || hot { t.color.accent } else { alpha(t.color.bright, 0.6) };
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, alpha(if hot { t.color.selection } else { t.color.surface }, 0.35));
            controls::dashed(d, r, DASH, if sel || hot { 2.0 } else { 1.25 }, line);
            let icon = (r.w * 0.5).min(t.size.row);
            d.icon(r.x + (r.w - icon) * 0.5, r.y + (r.h - icon) * 0.5, icon, "target", line);
            let above = t.size.label * 1.6 + t.size.small * 1.3 + 6.0;
            let c = caption(d, t, (r.x + r.w * 0.5, r.y.max(area.y + above) - above), &m.id, &m.words, line);
            view.marks.push((Rect { x: r.x.min(c.x), y: c.y, w: r.w.max(c.w), h: r.y + r.h - c.y }, Mark::Slot(m.slot)));
        } else {
            let Some(feet) = space.cast.iter().find(|c| c.holder == m.id).map(|c| c.feet) else { continue };
            let Some((x, y)) = space.to_pixel(feet).map(|(x, y)| logical(x, y)) else { continue };
            let color = if sel { t.color.accent } else { alpha(t.color.bright, 0.7) };
            let tall = t.size.label * 1.6 + t.size.small * 1.3 + t.size.pad * 2.0;
            let c = caption(d, t, (x, (y + 6.0).min(area.y + area.h - tall)), &m.id, &m.words, color);
            let r = Rect { x: c.x - 6.0, y: c.y - 4.0, w: c.w + 12.0, h: c.h + 6.0 };
            controls::dashed(d, r, DASH * 0.5, if sel { 1.5 } else { 1.0 }, color);
            view.marks.push((r, Mark::Slot(m.slot)));
        }
    }
    videos(d, t, view, area);
    layers(d, t, view, space, area, scale, carry);
}

/// metres a layer moves per logical pixel its handle is dragged
pub const DEPTH_PER_PX: f32 = 0.05;

/// The parallax layers outlined where they stand (an empty place brighter), each with its handle at its left edge –
/// its picture's name and its depth; handles of layers that stand alike are stacked far to near.
fn layers(d: &mut DrawList, t: &Tokens, view: &mut SceneView, space: &Space, area: Rect, scale: f32, carry: Option<(f32, f32)>) {
    let mut planes = Vec::new();
    space.layer_planes(true, &mut planes);
    let stack = view.edit.doc.stage.layers.len() as f32;
    let row = t.size.row;
    for (rank, p) in planes.iter().enumerate() {
        let Some(layer) = view.edit.doc.stage.layers.get(p.layer) else { continue };
        let sel = view.sel == Part::Layer(p.layer);
        let px = |v| space.to_pixel(v).map(|(x, y)| (area.x + x / scale, area.y + y / scale));
        let Some(c) = p.corners().map(px).into_iter().collect::<Option<Vec<_>>>() else { continue };
        let line = if sel { t.color.accent } else { alpha(t.color.bright, if layer.image.is_none() { 0.55 } else { 0.25 }) };
        for i in 0..4 {
            d.segment(c[i], c[(i + 1) % 4], if sel { 2.0 } else { 1.0 }, line);
        }
        let Some((ex, ey)) = px(p.middle - p.across) else { continue };
        let name = crate::scene_parts::layer_handle(layer);
        let w = d.sans_width(t.size.small, &name) + t.size.pad * 2.0 + row * 0.6;
        let x = ex.clamp(area.x + t.size.pad, area.x + area.w - w - t.size.pad);
        let y = (ey + (rank as f32 - stack * 0.5) * (row + 2.0)).clamp(area.y + t.size.pad, area.y + area.h - row - t.size.pad);
        let r = Rect { x, y, w, h: row - 4.0 };
        let hot = carry.is_some_and(|(cx, cy)| r.has(cx, cy));
        d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, alpha(if hot { t.color.selection } else { t.color.surface }, 0.85));
        d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.0), if sel || hot { t.color.accent } else { line });
        d.icon(r.x + t.size.pad * 0.5, r.y + (r.h - row * 0.5) * 0.5, row * 0.5, "layers", if sel { t.color.accent } else { t.color.muted });
        d.sans(r.x + t.size.pad + row * 0.5, r.y + (r.h - t.size.small * 1.25) * 0.5, t.size.small, &name, t.color.text);
        view.marks.push((r, Mark::Layer(p.layer)));
    }
}

/// The scene's video nodes as cards across the back of the stage, and the window framed while one that plays there is
/// selected.
fn videos(d: &mut DrawList, t: &Tokens, view: &mut SceneView, area: Rect) {
    let vids = view.edit.videos();
    if vids.is_empty() {
        return;
    }
    let gap = t.size.gap * 2.0;
    let w = t.size.rem(18.0).min(area.w * 0.3);
    let h = w * 9.0 / 16.0;
    let total = vids.len() as f32 * (w + gap) - gap;
    let (mut x, y) = (area.x + (area.w - total) * 0.5, area.y + area.h * 0.3);
    for v in vids {
        let node = &view.edit.doc.nodes[v];
        let sel = view.sel == Part::Video(v);
        let plays = view.edit.plays_at(v);
        let line = if sel { t.color.accent } else { alpha(t.color.bright, 0.6) };
        let r = Rect { x, y, w, h };
        // TODO(#326): the node's poster once the deck decodes video; until then the placeholder says so
        d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, alpha(t.color.surface, 0.85));
        controls::dashed(d, r, DASH, if sel { 2.0 } else { 1.25 }, line);
        let icon = t.size.row;
        d.icon(r.x + (r.w - icon) * 0.5, r.y + (r.h - icon) * 0.5 - t.size.small, icon, "video", line);
        let pad = t.size.pad;
        let room = r.w - pad * 2.0;
        let used = controls::label(d, t, r.x + pad, r.y + pad, &controls::fit(&node.id, 24), line);
        if node.kind != node.id {
            let kind = controls::fit_sans(d, t.size.small, &node.kind, room - used - pad);
            d.sans(r.x + r.w - pad - d.sans_width(t.size.small, &kind), r.y + pad, t.size.small, &kind, t.color.muted);
        }
        let place = match plays.as_deref() {
            Some(p) if p.starts_with(SCREEN) => "on the window".to_string(),
            Some(p) => format!("into {p}"),
            None => "nowhere yet".to_string(),
        };
        let lines = [format!("plays {place}"), "no poster: the deck plays no video yet".to_string()];
        for (i, l) in lines.iter().enumerate() {
            let ly = r.y + r.h - pad - t.size.small * 1.3 * (2 - i) as f32;
            d.sans(r.x + pad, ly, t.size.small, &controls::fit_sans(d, t.size.small, l, room), if i == 0 { t.color.text } else { t.color.muted });
        }
        if sel && plays.as_deref().is_some_and(|p| p.starts_with(SCREEN)) {
            controls::dashed(d, Rect { x: area.x + 4.0, y: area.y + 4.0, w: area.w - 8.0, h: area.h - 8.0 }, DASH * 2.0, 1.0, alpha(t.color.accent, 0.6));
        }
        view.marks.push((r, Mark::Video(v)));
        x += w + gap;
    }
}

/// the mark under (`x`, `y`), the one drawn last first
pub fn under(marks: &[(Rect, Mark)], x: f32, y: f32) -> Option<Mark> {
    marks.iter().rev().find(|(r, _)| r.has(x, y)).map(|(_, m)| *m)
}

impl crate::App {
    /// A press on the stage of a scene: a mark under it is selected. true = there was one.
    pub(crate) fn scene_mark_press(&mut self, x: f32, y: f32) -> bool {
        let Some(view) = self.scene.as_mut().filter(|v| v.inside.is_none()) else { return false };
        let Some(m) = under(&view.marks, x, y) else { return false };
        view.sel = m.part();
        if let Mark::Layer(l) = m {
            if let Some(depth) = view.edit.doc.stage.layers.get(l).map(|l| l.depth) {
                self.panel.drag = Some(crate::Drag::LayerDepth(l, y, depth));
            }
        }
        true
    }

    /// a layer's handle dragged from `y0` (its depth then `d0`) to `y`: up is farther; the stage shows it at once
    pub(crate) fn layer_depth_to(&mut self, l: usize, y0: f32, d0: f32, y: f32) {
        let Some(view) = self.scene.as_mut() else { return };
        let depth = (d0 + (y0 - y) * DEPTH_PER_PX).clamp(deck_space::scene_layers::NEAREST_KNOB, deck_space::scene_layers::DEEPEST_KNOB);
        if view.edit.move_layer(l, (depth * 10.0).round() / 10.0).is_ok() {
            let layers = view.edit.doc.stage.layers.clone();
            self.stage.space.set_layers(&layers);
        }
    }

    /// A book entry let go on the scene: a slot under it takes it as its preview, elsewhere the first slot that admits
    /// it; the core's words where it was let go when none does.
    pub(crate) fn scene_drop(&mut self, address: &str, kind: deck_catalog::library::Kind, x: f32, y: f32) {
        let Some(view) = self.scene.as_mut() else { return };
        if kind == deck_catalog::library::Kind::Image {
            let layer = match under(&view.marks, x, y) {
                Some(Mark::Layer(l)) => Some(l),
                _ => view.edit.empty_layer(),
            };
            let Some(l) = layer else {
                self.library.refused = Some((x, y, "no empty layer: add the parallax stack or a layer first".into()));
                return;
            };
            let done = view.edit.fill_layer(l, Some(address));
            view.sel = Part::Layer(l);
            return self.scene_changed(done, format!("{address} fills the layer"), x, y);
        }
        let slot = match under(&view.marks, x, y) {
            Some(Mark::Slot(s)) => Ok(s),
            _ => view.edit.slot_for(address),
        };
        match slot.and_then(|s| view.edit.fill(s, address).map(|()| s)) {
            Ok(s) => {
                view.sel = Part::Slot(s);
                let id = view.edit.doc.holders[s].id.clone();
                self.scene_changed(Ok(()), format!("{address} previews {id}"), x, y);
            }
            Err(why) => self.library.refused = Some((x, y, why)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::show::build::SceneOpen;
    use deck_space::graph::{Graph, Kind};
    use deck_space::scene_document::SceneDocument;
    use deck_space::space::portrait::Portraits;
    use deck_space::space::values::Table;
    use protocol::address_book::AddressKind;

    fn stage(view: &SceneView) -> Space {
        let scene = view.scene().expect("it plays");
        let g = Graph::build(&scene).unwrap();
        let mut space = Space::new(&g, g.first(Kind::Space).unwrap(), &scene.being, std::path::Path::new("/nowhere"), Portraits::manual());
        space.set_view(1280.0, 720.0);
        let mut ctl = Table::new(&g);
        for k in 0..30 {
            space.update(1.0 / 60.0, k as f32 / 60.0, &mut ctl, &deck_space::tempo::Now::default(), &|_, _| 0.0);
        }
        space
    }

    /// the panel model: an empty slot stands as a dashed card a press finds and a carried card lights, a filled one has
    /// its tag, a video node its card with the placeholder; inside a member nothing is marked
    #[test]
    fn the_stage_marks_the_slots_and_the_video_nodes_of_the_scene() {
        let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        doc.recast(&["character:catpope".into()]);
        let (mut view, _, _) = SceneView::open(SceneOpen { doc, base: Some(3), kept: None, note: None, save: None }, None).expect("it plays");
        let empty = view.edit.add_slot(AddressKind::Character).unwrap();
        let filled = view.edit.add_slot(AddressKind::Item).unwrap();
        view.edit.fill(filled, "item:incense_burner").unwrap();
        view.refresh().unwrap();
        let v = view.edit.add_video("test_pattern").unwrap();
        let space = stage(&view);
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        let area = Rect { x: 0.0, y: 0.0, w: 1280.0, h: 720.0 };
        draw(&mut d, &t, &mut view, &space, area, 1.0, None);
        let at = |m: Mark| view.marks.iter().find(|(_, x)| *x == m).map(|(r, _)| *r);
        let card = at(Mark::Slot(empty)).expect("the empty slot is marked");
        assert!(at(Mark::Slot(filled)).is_some(), "the filled slot has its tag");
        let screen = at(Mark::Video(v)).expect("the video node's card");
        assert!(d.texts().iter().any(|s| s.contains("no poster")), "the placeholder says why");
        let (cx, cy) = (card.x + card.w * 0.5, card.y + card.h * 0.3);
        assert_eq!(under(&view.marks, cx, cy), Some(Mark::Slot(empty)));
        assert_eq!(under(&view.marks, screen.x + 2.0, screen.y + 2.0).map(Mark::part), Some(Part::Video(v)));
        let accent = |d: &DrawList| d.prims().iter().filter(|p| matches!(p, deck_kit::base::draw::Prim::Segment { color, .. } if *color == t.color.accent)).count();
        let quiet = accent(&d);
        let mut lit = DrawList::new(0.6);
        draw(&mut lit, &t, &mut view, &space, area, 1.0, Some((cx, cy)));
        assert!(accent(&lit) > quiet, "a carried card lights the slot under it");
        view.inside = Some(0);
        draw(&mut d, &t, &mut view, &space, area, 1.0, None);
        assert!(view.marks.is_empty());
    }
}
