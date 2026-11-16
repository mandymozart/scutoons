//! Floating panels over a full-window stage: a pure layout model (where each panel is, how large, what scrolls) and its
//! drawing. A panel opens at the size of its content, grows while there is room – a gap to every other panel and to the
//! walls – and scrolls inside only when it cannot grow further. It has no header: a drag handle (grip and icon) moves it,
//! a chevron folds it to a pill (the icon and its name), which is dragged by the same rules. A dragged panel stops at a
//! wall or another panel, the gap away; nothing overlaps, also after the window changes size. The place is kept per
//! corner (the nearest one), so it can be remembered and comes back where it was.
use crate::base::draw::{DrawList, Rect};
use crate::base::tokens::Tokens;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// a panel's smallest open size before its content scrolls
const MIN_W: f32 = 160.0;
const MIN_H: f32 = 48.0;
/// the wheel's step
const STEP: f32 = 40.0;
const EPS: f32 = 0.01;

/// the corner a panel keeps its distance to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Corner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    fn right(self) -> bool {
        matches!(self, Corner::TopRight | Corner::BottomRight)
    }
    fn bottom(self) -> bool {
        matches!(self, Corner::BottomLeft | Corner::BottomRight)
    }
}

/// where a panel is: its distance from its corner's walls, and whether it is folded to its pill
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Place {
    pub corner: Corner,
    pub dx: f32,
    pub dy: f32,
    #[serde(default)]
    pub collapsed: bool,
}

impl Place {
    pub const fn at(corner: Corner, dx: f32, dy: f32) -> Place {
        Place { corner, dx, dy, collapsed: false }
    }

    pub const fn folded(self) -> Place {
        Place { collapsed: true, ..self }
    }
}

/// what a panel is: its id (what its place is remembered by), its name and icon (the pill, the info), what it is for
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spec {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub about: &'static str,
    pub place: Place,
}

/// a panel shown this frame and the size its content asks for
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shown {
    pub id: &'static str,
    pub content: (f32, f32),
}

/// the sizes the model lays out with (from the tokens and the face)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// between panels and to the walls (1rem)
    pub gap: f32,
    pub pad: f32,
    /// the handle's strip at the top of an open panel
    pub head: f32,
    /// a pill's height
    pub pill: f32,
    /// a character's advance in a pill's name
    pub char_w: f32,
}

impl Metrics {
    pub fn of(t: &Tokens, d: &DrawList) -> Metrics {
        let rem = 16.0;
        Metrics { gap: rem, pad: t.size.pad, head: t.size.row, pill: t.size.row + 4.0, char_w: d.advance(t.size.font) }
    }

    /// a folded panel's pill: grip, icon, name, chevron
    pub fn pill_size(&self, name: &str) -> (f32, f32) {
        (self.pad + 12.0 + 4.0 + 16.0 + 6.0 + self.char_w * name.chars().count() as f32 + 6.0 + 14.0 + self.pad, self.pill)
    }

    /// an open panel's size around a content of `c`
    pub fn frame(&self, c: (f32, f32)) -> (f32, f32) {
        (c.0 + 2.0 * self.pad, c.1 + self.head + self.pad)
    }
}

/// a part of a panel's chrome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chrome {
    Handle,
    Fold,
}

/// where a shown panel landed
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub whole: Rect,
    pub handle: Rect,
    pub fold: Rect,
    /// the visible part of the content (open panels; empty when folded)
    pub content: Rect,
    pub scroll: f32,
    /// how much of the content is below the visible part at scroll 0
    pub over: f32,
    pub collapsed: bool,
}

#[derive(Debug, Clone)]
struct Panel {
    spec: Spec,
    place: Place,
    scroll: f32,
    shown: bool,
    content: (f32, f32),
    want: (f32, f32),
    min: (f32, f32),
    /// the corner it grows from this frame and that corner's point; its top left
    corner: Corner,
    anchor: (f32, f32),
    at: (f32, f32),
    view: View,
}

/// the panels of one workspace
#[derive(Debug, Clone)]
pub struct Floats {
    panels: Vec<Panel>,
    area: Rect,
    fixed: Vec<Rect>,
    m: Metrics,
    /// the panel in the hand and where the pointer holds it (from its top left)
    drag: Option<(usize, f32, f32)>,
}

fn right(r: &Rect) -> f32 {
    r.x + r.w
}

fn bottom(r: &Rect) -> f32 {
    r.y + r.h
}

/// do two rectangles come closer than `gap`
fn crowd(a: &Rect, b: &Rect, gap: f32) -> bool {
    a.x < right(b) + gap - EPS && b.x < right(a) + gap - EPS && a.y < bottom(b) + gap - EPS && b.y < bottom(a) + gap - EPS
}

fn inside(r: &Rect, area: &Rect) -> bool {
    r.x >= area.x - EPS && r.y >= area.y - EPS && right(r) <= right(area) + EPS && bottom(r) <= bottom(area) + EPS
}

/// `r` moved into `area` as far as it fits (its top left first)
fn clamp_into(r: Rect, area: &Rect) -> Rect {
    let x = r.x.min(right(area) - r.w).max(area.x);
    let y = r.y.min(bottom(area) - r.h).max(area.y);
    Rect { x, y, ..r }
}

/// the free place for `r` nearest to it: beside a wall or the gap away from an obstacle
fn nearest_free(r: Rect, taken: &[Rect], area: &Rect, gap: f32) -> Rect {
    let mut xs = vec![r.x, area.x, right(area) - r.w];
    let mut ys = vec![r.y, area.y, bottom(area) - r.h];
    for o in taken {
        xs.extend([right(o) + gap, o.x - gap - r.w, o.x]);
        ys.extend([bottom(o) + gap, o.y - gap - r.h, o.y]);
    }
    let mut best: Option<(f32, Rect)> = None;
    for &x in &xs {
        for &y in &ys {
            let c = Rect { x, y, ..r };
            if !inside(&c, area) || taken.iter().any(|o| crowd(&c, o, gap)) {
                continue;
            }
            let d = (x - r.x).abs() + (y - r.y).abs();
            if best.is_none_or(|(b, _)| d < b) {
                best = Some((d, c));
            }
        }
    }
    best.map_or(r, |(_, c)| c)
}

/// how far a panel may reach along an axis from its anchored `edge` (away from it: up or left when `back`), inside the
/// band `lo..hi` across: to the wall or the gap before an obstacle
fn reach(edge: f32, back: bool, (lo, hi): (f32, f32), obstacles: &[Rect], area: &Rect, gap: f32, horizontal: bool) -> f32 {
    let along = |o: &Rect| if horizontal { (o.x, right(o)) } else { (o.y, bottom(o)) };
    let across = |o: &Rect| if horizontal { (o.y, bottom(o)) } else { (o.x, right(o)) };
    let (wall_lo, wall_hi) = if horizontal { (area.x, right(area)) } else { (area.y, bottom(area)) };
    let in_band = |o: &Rect| across(o).0 < hi + gap - EPS && across(o).1 + gap > lo + EPS;
    if back {
        obstacles.iter().filter(|o| in_band(o) && along(o).0 < edge - EPS).fold(edge - wall_lo, |l, o| l.min(edge - along(o).1 - gap))
    } else {
        obstacles.iter().filter(|o| in_band(o) && along(o).1 > edge + EPS).fold(wall_hi - edge, |l, o| l.min(along(o).0 - gap - edge))
    }
}

/// a rectangle of `size` held at `anchor` by its corner
fn from_corner(anchor: (f32, f32), corner: Corner, (w, h): (f32, f32)) -> Rect {
    Rect { x: if corner.right() { anchor.0 - w } else { anchor.0 }, y: if corner.bottom() { anchor.1 - h } else { anchor.1 }, w, h }
}

/// the corner point of `r` a panel anchored at `corner` keeps
fn corner_of(r: &Rect, corner: Corner) -> (f32, f32) {
    (if corner.right() { right(r) } else { r.x }, if corner.bottom() { bottom(r) } else { r.y })
}

fn intersect(a: &Rect, b: &Rect) -> Rect {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    Rect { x: x0, y: y0, w: (right(a).min(right(b)) - x0).max(0.0), h: (bottom(a).min(bottom(b)) - y0).max(0.0) }
}

/// how far `r` may move by `d` along an axis before it meets a wall or an obstacle the gap away
fn sweep(r: &Rect, d: f32, obstacles: &[Rect], area: &Rect, gap: f32, horizontal: bool) -> f32 {
    let (pos, len, lo, hi) = if horizontal { (r.x, r.w, area.x, right(area)) } else { (r.y, r.h, area.y, bottom(area)) };
    let across = |o: &Rect| if horizontal { o.y < bottom(r) + gap - EPS && bottom(o) + gap > r.y + EPS } else { o.x < right(r) + gap - EPS && right(o) + gap > r.x + EPS };
    let span = |o: &Rect| if horizontal { (o.x, o.w) } else { (o.y, o.h) };
    if d > 0.0 {
        let limit = obstacles.iter().filter(|o| across(o) && span(o).0 >= pos + len - EPS).fold(hi - (pos + len), |l, o| l.min(span(o).0 - gap - (pos + len)));
        d.min(limit.max(0.0))
    } else {
        let limit = obstacles.iter().filter(|o| across(o) && span(o).0 + span(o).1 <= pos + EPS).fold(lo - pos, |l, o| l.max(span(o).0 + span(o).1 + gap - pos));
        d.max(limit.min(0.0))
    }
}

impl Floats {
    pub fn new(specs: &[Spec]) -> Floats {
        let empty = View { whole: Rect::default(), handle: Rect::default(), fold: Rect::default(), content: Rect::default(), scroll: 0.0, over: 0.0, collapsed: false };
        let panels = specs.iter().map(|s| Panel { spec: *s, place: s.place, scroll: 0.0, shown: false, content: (0.0, 0.0), want: (0.0, 0.0), min: (0.0, 0.0), corner: s.place.corner, anchor: (0.0, 0.0), at: (0.0, 0.0), view: empty }).collect();
        Floats { panels, area: Rect::default(), fixed: Vec::new(), m: Metrics { gap: 16.0, pad: 8.0, head: 28.0, pill: 32.0, char_w: 7.0 }, drag: None }
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.panels.iter().position(|p| p.spec.id == id)
    }

    /// the places to remember
    pub fn places(&self) -> BTreeMap<String, Place> {
        self.panels.iter().map(|p| (p.spec.id.to_string(), p.place)).collect()
    }

    /// places remembered before (ids no panel has are left)
    pub fn restore(&mut self, places: &BTreeMap<String, Place>) {
        for p in &mut self.panels {
            if let Some(place) = places.get(p.spec.id) {
                p.place = *place;
            }
        }
    }

    /// this frame's layout: the room inside the walls, rectangles nothing may cover, the panels shown with their content
    pub fn layout(&mut self, area: Rect, fixed: &[Rect], shown: &[Shown], m: Metrics) {
        let gap = m.gap;
        self.area = area;
        self.m = m;
        self.fixed = fixed.to_vec();
        for p in &mut self.panels {
            p.shown = false;
        }
        for s in shown {
            if let Some(i) = self.index(s.id) {
                self.panels[i].shown = true;
                self.panels[i].content = (s.content.0.max(0.0), s.content.1.max(0.0));
            }
        }
        let order: Vec<usize> = (0..self.panels.len()).filter(|&i| self.panels[i].shown).collect();
        for &i in &order {
            let p = &mut self.panels[i];
            let pill = m.pill_size(p.spec.name);
            if p.place.collapsed {
                p.want = (pill.0.min(area.w), pill.1.min(area.h));
                p.min = p.want;
            } else {
                let full = m.frame(p.content);
                p.want = (full.0.max(pill.0).min(area.w), full.1.min(area.h));
                p.min = (p.want.0.min(MIN_W.max(pill.0)), p.want.1.min(m.head + MIN_H));
            }
            // the one in the hand is held by its top left where the pointer has it
            let held = matches!(self.drag, Some((d, ..)) if d == i);
            if held {
                p.corner = Corner::TopLeft;
                p.anchor = p.at;
            } else {
                let Place { corner, dx, dy, .. } = p.place;
                p.corner = corner;
                p.anchor = (if corner.right() { right(&area) - dx } else { area.x + dx }, if corner.bottom() { bottom(&area) - dy } else { area.y + dy });
            }
        }
        // the smallest rectangles first: each inside the walls, none crowding another
        let mut mins: Vec<Rect> = Vec::with_capacity(order.len());
        for &i in &order {
            let p = &self.panels[i];
            let r = clamp_into(from_corner(p.anchor, p.corner, p.min), &area);
            let taken: Vec<Rect> = self.fixed.iter().chain(&mins).copied().collect();
            let r = if taken.iter().any(|o| crowd(&r, o, gap)) { nearest_free(r, &taken, &area, gap) } else { r };
            let p = &mut self.panels[i];
            p.anchor = corner_of(&r, p.corner);
            mins.push(r);
        }
        // then each grows from its corner to its content while there is room: the ones before it as they grew, the
        // later ones as large as they want to be (they take what is left)
        let wants: Vec<Rect> = order.iter().map(|&i| intersect(&from_corner(self.panels[i].anchor, self.panels[i].corner, self.panels[i].want), &area)).collect();
        let mut grown: Vec<Rect> = Vec::with_capacity(order.len());
        for (k, &i) in order.iter().enumerate() {
            let obstacles: Vec<Rect> = self.fixed.iter().chain(&grown).chain(&wants[k + 1..]).copied().collect();
            let p = &self.panels[i];
            let (c, m0) = (p.corner, mins[k]);
            let w = p.want.0.min(reach(p.anchor.0, c.right(), (m0.y, bottom(&m0)), &obstacles, &area, gap, true)).max(p.min.0);
            let band = from_corner(p.anchor, c, (w, p.min.1));
            let h = p.want.1.min(reach(p.anchor.1, c.bottom(), (band.x, right(&band)), &obstacles, &area, gap, false)).max(p.min.1);
            let whole = from_corner(p.anchor, c, (w, h));
            let p = &mut self.panels[i];
            p.at = (whole.x, whole.y);
            grown.push(whole);
            let collapsed = p.place.collapsed;
            let fold_w = 14.0 + m.pad;
            let (handle, fold, content) = if collapsed {
                (Rect { w: (w - fold_w).max(0.0), ..whole }, Rect { x: right(&whole) - fold_w - m.pad * 0.5, w: fold_w + m.pad * 0.5, ..whole }, Rect { x: whole.x, y: whole.y, w: 0.0, h: 0.0 })
            } else {
                let head = Rect { h: m.head, ..whole };
                (
                    Rect { w: (w - fold_w - m.pad).max(0.0), ..head },
                    Rect { x: right(&whole) - fold_w - m.pad, w: fold_w + m.pad, ..head },
                    Rect { x: whole.x + m.pad, y: whole.y + m.head, w: (w - 2.0 * m.pad).max(0.0), h: (h - m.head - m.pad).max(0.0) },
                )
            };
            let over = if collapsed { 0.0 } else { (p.content.1 - content.h).max(0.0) };
            p.scroll = p.scroll.clamp(0.0, over);
            p.view = View { whole, handle, fold, content, scroll: p.scroll, over, collapsed };
        }
    }

    /// where a shown panel landed this frame
    pub fn view(&self, id: &str) -> Option<View> {
        self.index(id).map(|i| &self.panels[i]).filter(|p| p.shown).map(|p| p.view)
    }

    pub fn spec(&self, id: &str) -> Option<&Spec> {
        self.index(id).map(|i| &self.panels[i].spec)
    }

    fn shown(&self) -> impl Iterator<Item = (usize, &Panel)> {
        self.panels.iter().enumerate().filter(|(_, p)| p.shown)
    }

    /// the chrome under the pointer
    pub fn chrome_at(&self, x: f32, y: f32) -> Option<(&Spec, Chrome)> {
        self.shown().find_map(|(_, p)| {
            if p.view.fold.has(x, y) {
                Some((&p.spec, Chrome::Fold))
            } else if p.view.handle.has(x, y) {
                Some((&p.spec, Chrome::Handle))
            } else {
                None
            }
        })
    }

    /// is a shown panel under the pointer (its ground keeps the press from the stage)
    pub fn covers(&self, x: f32, y: f32) -> bool {
        self.shown().any(|(_, p)| p.view.whole.has(x, y))
    }

    /// a press on a handle takes the panel in the hand, on a chevron folds or opens it: whether the press was the chrome's
    pub fn press(&mut self, x: f32, y: f32) -> bool {
        let Some((id, chrome)) = self.chrome_at(x, y).map(|(s, c)| (s.id, c)) else { return false };
        let Some(i) = self.index(id) else { return false };
        match chrome {
            Chrome::Fold => self.toggle(id),
            Chrome::Handle => {
                let at = self.panels[i].view.whole;
                self.drag = Some((i, x - at.x, y - at.y));
            }
        }
        true
    }

    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// the panel in the hand follows the pointer until a wall or another panel stops it
    pub fn drag_to(&mut self, x: f32, y: f32) -> bool {
        let Some((i, ox, oy)) = self.drag else { return false };
        let gap = self.m.gap;
        let obstacles: Vec<Rect> = self.fixed.iter().copied().chain(self.shown().filter(|(j, _)| *j != i).map(|(_, p)| p.view.whole)).collect();
        let mut r = self.panels[i].view.whole;
        r.x += sweep(&r, x - ox - r.x, &obstacles, &self.area, gap, true);
        r.y += sweep(&r, y - oy - r.y, &obstacles, &self.area, gap, false);
        let p = &mut self.panels[i];
        p.at = (r.x, r.y);
        p.view.whole.x = r.x;
        p.view.whole.y = r.y;
        true
    }

    /// let go: the place is kept from the nearest corner; whether a panel was in the hand (its place changed)
    pub fn release(&mut self) -> bool {
        let Some((i, ..)) = self.drag.take() else { return false };
        let area = self.area;
        let p = &mut self.panels[i];
        let seen = p.view.whole;
        let (cx, cy) = (seen.x + seen.w * 0.5, seen.y + seen.h * 0.5);
        let (r, b) = (cx > area.x + area.w * 0.5, cy > area.y + area.h * 0.5);
        p.place.corner = match (r, b) {
            (false, false) => Corner::TopLeft,
            (true, false) => Corner::TopRight,
            (false, true) => Corner::BottomLeft,
            (true, true) => Corner::BottomRight,
        };
        p.place.dx = if r { right(&area) - right(&seen) } else { seen.x - area.x }.max(0.0);
        p.place.dy = if b { bottom(&area) - bottom(&seen) } else { seen.y - area.y }.max(0.0);
        true
    }

    /// fold a panel to its pill or open it again
    pub fn toggle(&mut self, id: &str) {
        if let Some(i) = self.index(id) {
            let p = &mut self.panels[i];
            p.place.collapsed = !p.place.collapsed;
        }
    }

    pub fn set_collapsed(&mut self, id: &str, collapsed: bool) {
        if let Some(i) = self.index(id) {
            self.panels[i].place.collapsed = collapsed;
        }
    }

    pub fn collapsed(&self, id: &str) -> bool {
        self.index(id).is_some_and(|i| self.panels[i].place.collapsed)
    }

    /// the wheel over an open panel whose content is taller than it: whether it scrolled
    pub fn wheel(&mut self, x: f32, y: f32, steps: f32) -> bool {
        let Some(i) = self.shown().find(|(_, p)| p.view.whole.has(x, y) && p.view.over > 0.0).map(|(i, _)| i) else { return false };
        let p = &mut self.panels[i];
        p.scroll = (p.scroll - steps * STEP).clamp(0.0, p.view.over);
        p.view.scroll = p.scroll;
        true
    }

    /// the content's own rectangle of a shown open panel: as wide as it shows, as tall as it is, moved up by the scroll
    pub fn content(&self, id: &str) -> Option<Rect> {
        let v = self.view(id).filter(|v| !v.collapsed)?;
        let p = &self.panels[self.index(id)?];
        Some(Rect { x: v.content.x, y: v.content.y - v.scroll, w: v.content.w, h: p.content.1.max(v.content.h) })
    }
}

/// the places a panel's content pressed this frame, from `from` on, cut to what shows of it
pub fn clip_hits<H>(view: &View, hits: &mut Vec<(Rect, H)>, from: usize) {
    let c = view.content;
    let kept: Vec<(Rect, H)> = hits
        .drain(from..)
        .filter_map(|(r, h)| {
            let (x0, y0) = (r.x.max(c.x), r.y.max(c.y));
            let (x1, y1) = (right(&r).min(right(&c)), bottom(&r).min(bottom(&c)));
            (x1 > x0 && y1 > y0).then_some((Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }, h))
        })
        .collect();
    hits.extend(kept);
}

/// a panel's ground and chrome; open, the content is clipped to its place until `end`
pub fn begin(d: &mut DrawList, t: &Tokens, spec: &Spec, v: &View, pointer: (f32, f32)) {
    let w = v.whole;
    let r = t.size.radius * 2.0;
    d.round_rect(w.x, w.y, w.w, w.h, if v.collapsed { w.h * 0.5 } else { r }, t.color.surface);
    d.outline(w.x, w.y, w.w, w.h, (if v.collapsed { w.h * 0.5 } else { r }, 1.0), t.color.line);
    let hot = v.handle.has(pointer.0, pointer.1);
    let mid = v.handle.y + v.handle.h * 0.5;
    let x = w.x + t.size.pad;
    d.icon(x, mid - 6.0, 12.0, "grip", if hot { t.color.text } else { t.color.muted });
    d.icon(x + 16.0, mid - 8.0, 16.0, spec.icon, t.color.text);
    if v.collapsed {
        d.text(x + 38.0, mid - t.size.font * 0.62, t.size.font, spec.name, t.color.text);
    }
    let fold_hot = v.fold.has(pointer.0, pointer.1);
    d.icon(v.fold.x + (v.fold.w - 14.0) * 0.5, mid - 7.0, 14.0, if v.collapsed { "down" } else { "up" }, if fold_hot { t.color.text } else { t.color.muted });
    if !v.collapsed {
        d.clip(Some(v.content));
    }
}

/// after the content: the clip ends, a scrolled panel shows where its view stands
pub fn end(d: &mut DrawList, t: &Tokens, v: &View) {
    if v.collapsed {
        return;
    }
    d.clip(None);
    if v.over > 0.0 {
        let c = v.content;
        let whole = c.h + v.over;
        d.round_rect(right(&v.whole) - 5.0, c.y + c.h * v.scroll / whole, 3.0, c.h * c.h / whole, 1.5, t.color.muted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: Metrics = Metrics { gap: 16.0, pad: 8.0, head: 28.0, pill: 32.0, char_w: 7.0 };

    fn area(w: f32, h: f32) -> Rect {
        Rect { x: 16.0, y: 64.0, w: w - 32.0, h: h - 80.0 }
    }

    fn spec(id: &'static str, place: Place) -> Spec {
        Spec { id, name: id, icon: "list", about: "", place }
    }

    fn four() -> Floats {
        Floats::new(&[
            spec("tree", Place::at(Corner::TopLeft, 0.0, 0.0)),
            spec("inspector", Place::at(Corner::TopRight, 0.0, 0.0)),
            spec("mixer", Place::at(Corner::BottomRight, 0.0, 0.0)),
            spec("info", Place::at(Corner::BottomLeft, 0.0, 0.0).folded()),
        ])
    }

    fn shown(sizes: [(f32, f32); 4]) -> Vec<Shown> {
        ["tree", "inspector", "mixer", "info"].into_iter().zip(sizes).map(|(id, content)| Shown { id, content }).collect()
    }

    const SIZES: [(f32, f32); 4] = [(220.0, 300.0), (280.0, 600.0), (500.0, 120.0), (240.0, 100.0)];

    fn rects(f: &Floats) -> Vec<Rect> {
        f.panels.iter().filter(|p| p.shown).map(|p| p.view.whole).collect()
    }

    fn assert_valid(f: &Floats, a: &Rect) {
        let r = rects(f);
        for (i, x) in r.iter().enumerate() {
            assert!(inside(x, a), "panel {i} {x:?} inside {a:?}");
            for (j, y) in r.iter().enumerate().skip(i + 1) {
                assert!(!crowd(x, y, 16.0), "panels {i} {x:?} and {j} {y:?} keep the gap");
            }
        }
    }

    #[test]
    fn a_panel_opens_at_the_size_of_its_content_and_does_not_scroll() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("tree").unwrap();
        assert_eq!((v.whole.w, v.whole.h), M.frame(SIZES[0]));
        assert_eq!((v.whole.x, v.whole.y), (a.x, a.y), "in its corner, at the walls' gap");
        assert_eq!(v.over, 0.0);
        let m = f.view("mixer").unwrap();
        assert_eq!((right(&m.whole), bottom(&m.whole)), (right(&a), bottom(&a)), "the bottom right one in its corner");
    }

    #[test]
    fn a_full_panel_grows_to_the_room_then_scrolls_inside() {
        let mut f = four();
        let a = area(1920.0, 700.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("inspector").unwrap();
        let mixer = f.view("mixer").unwrap().whole;
        assert_eq!(bottom(&v.whole), mixer.y - 16.0, "it grows down to the panel under it, the gap away");
        assert!(v.over > 0.0, "and scrolls the rest");
        assert!(f.wheel(v.whole.x + 5.0, v.whole.y + 50.0, -100.0));
        assert_eq!(f.view("inspector").unwrap().scroll, v.over, "the wheel stops at the end");
        assert!(!f.wheel(f.view("tree").unwrap().whole.x + 5.0, a.y + 50.0, -1.0), "a panel that fits does not scroll");
    }

    #[test]
    fn a_dragged_panel_moves_with_the_pointer() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("tree").unwrap();
        assert!(f.press(v.handle.x + 4.0, v.handle.y + 4.0));
        f.drag_to(v.handle.x + 304.0, v.handle.y + 104.0);
        assert!(f.release());
        f.layout(a, &[], &shown(SIZES), M);
        let w = f.view("tree").unwrap().whole;
        assert_eq!((w.x, w.y), (v.whole.x + 300.0, v.whole.y + 100.0));
    }

    #[test]
    fn a_dragged_panel_stops_at_the_wall() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("tree").unwrap();
        f.press(v.handle.x + 4.0, v.handle.y + 4.0);
        f.drag_to(v.handle.x - 500.0, v.handle.y - 500.0);
        let w = f.view("tree").unwrap().whole;
        assert_eq!((w.x, w.y), (a.x, a.y), "the walls hold it, the gap from the window's edge");
    }

    #[test]
    fn a_dragged_panel_stops_the_gap_before_another() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("tree").unwrap();
        let other = f.view("inspector").unwrap().whole;
        f.press(v.handle.x + 4.0, v.handle.y + 4.0);
        f.drag_to(v.handle.x + 5000.0, v.handle.y);
        let w = f.view("tree").unwrap().whole;
        assert_eq!(right(&w), other.x - 16.0, "stopped 1rem before the inspector");
        f.release();
        f.layout(a, &[], &shown(SIZES), M);
        assert_eq!(right(&f.view("tree").unwrap().whole), other.x - 16.0, "and stays there");
        assert_valid(&f, &a);
    }

    #[test]
    fn a_fixed_place_stops_a_panel_too() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        let gizmo = Rect { x: 900.0, y: 64.0, w: 72.0, h: 72.0 };
        f.layout(a, &[gizmo], &shown(SIZES), M);
        let v = f.view("tree").unwrap();
        f.press(v.handle.x + 4.0, v.handle.y + 4.0);
        f.drag_to(v.handle.x + 5000.0, v.handle.y);
        assert_eq!(right(&f.view("tree").unwrap().whole), gizmo.x - 16.0);
    }

    #[test]
    fn a_panel_folds_to_its_pill_and_opens_again() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("tree").unwrap();
        assert!(f.press(v.fold.x + 2.0, v.fold.y + 2.0));
        f.layout(a, &[], &shown(SIZES), M);
        let p = f.view("tree").unwrap();
        assert!(p.collapsed);
        assert_eq!((p.whole.w, p.whole.h), M.pill_size("tree"));
        assert_eq!((p.whole.x, p.whole.y), (v.whole.x, v.whole.y), "in the same corner");
        let info = f.view("info").unwrap();
        assert!(info.collapsed && info.whole.h == M.pill, "a panel can start folded");
        f.press(p.fold.x + 2.0, p.fold.y + 2.0);
        f.layout(a, &[], &shown(SIZES), M);
        assert_eq!(f.view("tree").unwrap().whole, v.whole);
    }

    #[test]
    fn a_pill_is_dragged_by_the_same_rules() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.toggle("tree");
        f.layout(a, &[], &shown(SIZES), M);
        let p = f.view("tree").unwrap();
        let other = f.view("inspector").unwrap().whole;
        assert!(f.press(p.handle.x + 4.0, p.handle.y + 4.0));
        f.drag_to(p.handle.x + 5000.0, p.handle.y + 4.0);
        assert_eq!(right(&f.view("tree").unwrap().whole), other.x - 16.0);
    }

    #[test]
    fn nothing_overlaps_after_the_window_shrinks() {
        let mut f = four();
        f.layout(area(1920.0, 1080.0), &[], &shown(SIZES), M);
        for (w, h) in [(1280.0, 720.0), (900.0, 600.0), (640.0, 480.0), (2560.0, 1440.0)] {
            let a = area(w, h);
            f.layout(a, &[], &shown(SIZES), M);
            assert_valid(&f, &a);
        }
    }

    #[test]
    fn a_corner_panel_keeps_its_corner_when_the_window_changes() {
        let mut f = four();
        for (w, h) in [(1920.0, 1080.0), (1280.0, 720.0)] {
            let a = area(w, h);
            f.layout(a, &[], &shown(SIZES), M);
            let m = f.view("mixer").unwrap().whole;
            assert_eq!((right(&m), bottom(&m)), (right(&a), bottom(&a)));
        }
    }

    #[test]
    fn a_panel_put_on_another_finds_the_nearest_free_place() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        let mut places = f.places();
        places.insert("mixer".into(), Place::at(Corner::TopLeft, 10.0, 10.0));
        f.restore(&places);
        f.layout(a, &[], &shown(SIZES), M);
        assert_valid(&f, &a);
    }

    #[test]
    fn a_place_remembered_comes_back_where_it_was() {
        let mut f = four();
        let a = area(1600.0, 900.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("mixer").unwrap();
        f.press(v.handle.x + 4.0, v.handle.y + 4.0);
        f.drag_to(v.handle.x - 200.0, v.handle.y - 100.0);
        f.release();
        f.toggle("info");
        f.layout(a, &[], &shown(SIZES), M);
        let before = rects(&f);
        let mut g = four();
        g.restore(&f.places());
        g.layout(a, &[], &shown(SIZES), M);
        assert_eq!(rects(&g), before);
    }

    #[test]
    fn a_panel_not_shown_is_no_obstacle() {
        let mut f = four();
        let a = area(1920.0, 700.0);
        let only: Vec<Shown> = shown(SIZES).into_iter().filter(|s| s.id != "mixer").collect();
        f.layout(a, &[], &only, M);
        assert!(f.view("mixer").is_none());
        let v = f.view("inspector").unwrap();
        assert_eq!(bottom(&v.whole), bottom(&a), "the inspector takes the room the mixer leaves");
    }

    #[test]
    fn any_mix_of_places_and_sizes_keeps_every_panel_apart() {
        // a fixed sequence of pseudo-random places, contents and windows
        let mut seed = 7u32;
        let mut next = |n: f32| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (seed >> 8) as f32 / (1u32 << 24) as f32 * n
        };
        let corners = [Corner::TopLeft, Corner::TopRight, Corner::BottomLeft, Corner::BottomRight];
        for _ in 0..200 {
            let mut f = four();
            let mut places = BTreeMap::new();
            for id in ["tree", "inspector", "mixer", "info"] {
                let mut p = Place::at(corners[next(4.0) as usize % 4], next(600.0), next(400.0));
                p.collapsed = next(1.0) < 0.3;
                places.insert(id.to_string(), p);
            }
            f.restore(&places);
            let sizes = [(next(500.0), next(900.0)), (next(500.0), next(900.0)), (next(900.0), next(300.0)), (next(300.0), next(200.0))];
            let a = area(900.0 + next(1500.0), 640.0 + next(800.0));
            f.layout(a, &[], &shown(sizes), M);
            assert_valid(&f, &a);
        }
    }

    #[test]
    fn hits_of_a_scrolled_content_end_at_its_edge() {
        let v = View { whole: Rect::default(), handle: Rect::default(), fold: Rect::default(), content: Rect { x: 0.0, y: 100.0, w: 200.0, h: 100.0 }, scroll: 0.0, over: 50.0, collapsed: false };
        let mut hits = vec![(Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, 0), (Rect { x: 0.0, y: 90.0, w: 10.0, h: 20.0 }, 1), (Rect { x: 0.0, y: 150.0, w: 10.0, h: 10.0 }, 2), (Rect { x: 0.0, y: 250.0, w: 10.0, h: 10.0 }, 3)];
        clip_hits(&v, &mut hits, 1);
        assert_eq!(hits.iter().map(|(_, h)| *h).collect::<Vec<_>>(), [0, 1, 2], "the first is before `from`, the last below the edge");
        assert_eq!(hits[1].0, Rect { x: 0.0, y: 100.0, w: 10.0, h: 10.0 });
    }
}
