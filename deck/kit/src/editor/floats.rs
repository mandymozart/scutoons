//! Floating panels over a full-window stage: a pure layout model (where each panel is, how large, what scrolls) and its
//! drawing. A panel opens at the size of its content, grows while there is room – a gap to every other panel and to the
//! walls – and scrolls inside only when it cannot grow further. It has no header: a drag handle (grip and icon) moves it,
//! a chevron folds it to a pill (the icon and its name), which is dragged by the same rules; a panel that is not essential
//! to its workspace also has a close, and comes back from a list of the closable ones. Panels float on top and never push
//! or snap (the creator's floats.ts, the same rules): a dragged panel follows the pointer and stays where it is let go,
//! only held inside the walls; it may lie over another, the last touched on top. A panel growing to its content stops the
//! gap before a panel clear of it. The place is kept per corner (the nearest one), so it can be remembered and comes back
//! where it was. An open panel resizes from every edge and corner, held only by the walls, and never moves another; the
//! size is kept with the place and replaces the content's size (a smaller one scrolls). The scrollbar takes the pointer:
//! a press on its track jumps the thumb there, the thumb drags. A panel can also lie on the stage (a cell the caller
//! gives, `set_stage`): it fills it, under every floating panel, and its handle drags it off again.
use crate::base::draw::{DrawList, Rect};
use crate::base::tokens::Tokens;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

/// where a panel is: its distance from its corner's walls, whether it is folded to its pill, whether it is closed
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Place {
    pub corner: Corner,
    pub dx: f32,
    pub dy: f32,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub closed: bool,
    /// the open size the viewer resized it to (none: its content's)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<(f32, f32)>,
}

impl Place {
    pub const fn at(corner: Corner, dx: f32, dy: f32) -> Place {
        Place { corner, dx, dy, collapsed: false, closed: false, size: None }
    }

    pub const fn folded(self) -> Place {
        Place { collapsed: true, ..self }
    }

    pub const fn closed(self) -> Place {
        Place { closed: true, ..self }
    }
}

/// what a panel is: its id (what its place is remembered by), its name and icon (the pill, the info), what it is for;
/// an essential one only folds, the others close too
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spec {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub about: &'static str,
    pub place: Place,
    pub essential: bool,
    /// a toolbar: grip and icon at the start of its content's one row, no head above; folded only its icon
    pub inline: bool,
}

impl Spec {
    pub const fn essential(self) -> Spec {
        Spec { essential: true, place: Place { closed: false, ..self.place }, ..self }
    }

    pub const fn inline(self) -> Spec {
        Spec { inline: true, ..self }
    }
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
    /// 1rem: the chrome's icons
    pub rem: f32,
    /// a panel's smallest open size before its content scrolls
    pub min: (f32, f32),
}

impl Metrics {
    pub fn of(t: &Tokens, d: &DrawList) -> Metrics {
        let s = &t.size;
        Metrics { gap: s.gap, pad: s.pad, head: s.row, pill: s.row + s.rem(0.29), char_w: d.advance(s.font), rem: s.base, min: (s.panel_w, s.panel_h) }
    }

    /// a folded panel's pill: grip, icon, name (`name_w` wide when measured), chevron and – when it closes – the close
    pub fn pill_size(&self, name: &str, name_w: Option<f32>, closes: bool) -> (f32, f32) {
        (2.0 * self.pad + self.rem * 4.14 + name_w.unwrap_or(self.char_w * name.chars().count() as f32) + if closes { self.button() } else { 0.0 }, self.pill)
    }

    /// the chevron's and the close's width in the chrome
    fn button(&self) -> f32 {
        14.0 + self.pad
    }

    /// an inline panel's grip and icon before its row
    fn lead(&self) -> f32 {
        self.pad + self.rem * 2.4
    }

    /// an inline panel's size around a row of `c`, its chevron and – when it closes – its close after it
    fn inline_frame(&self, c: (f32, f32), closes: bool) -> (f32, f32) {
        (self.lead() + c.0 + self.pad + self.button() * if closes { 2.0 } else { 1.0 }, c.1.max(self.head) + self.pad)
    }

    /// an open panel's size around a content of `c`
    pub fn frame(&self, c: (f32, f32)) -> (f32, f32) {
        (c.0 + 2.0 * self.pad, c.1 + self.head + self.pad)
    }
}

/// which edges a resize moves: -1 the left / top one, 1 the right / bottom one, 0 neither
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    pub x: i8,
    pub y: i8,
}

/// a part of a panel's chrome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chrome {
    Handle,
    Fold,
    Close,
    Edge(Edge),
    /// the scrollbar's track and thumb
    Scroll,
}

/// how far an edge's grip reaches out of the panel (into the gap) and into it
const EDGE_OUT: f32 = 6.0;
const EDGE_IN: f32 = 2.0;
/// how far along an edge from a corner the corner's grip reaches
const CORNER: f32 = 14.0;

/// where a shown panel landed
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub whole: Rect,
    pub handle: Rect,
    pub fold: Rect,
    /// empty for an essential panel
    pub close: Rect,
    /// the visible part of the content (open panels; empty when folded)
    pub content: Rect,
    pub scroll: f32,
    /// how much of the content is below the visible part at scroll 0
    pub over: f32,
    pub collapsed: bool,
    /// the scrollbar's track along the right edge and its thumb (empty when nothing scrolls)
    pub track: Rect,
    pub thumb: Rect,
    /// whether it resizes (open and not a toolbar)
    pub sizes: bool,
    /// it lies on the stage: the stage's ground, not the floating panel's
    pub staged: bool,
}

impl View {
    fn empty(whole: Rect, collapsed: bool) -> View {
        let none = Rect { w: 0.0, h: 0.0, ..whole };
        View { whole, handle: none, fold: none, close: none, content: none, scroll: 0.0, over: 0.0, collapsed, track: none, thumb: none, sizes: false, staged: false }
    }

    /// the edges whose grip is under the pointer
    pub fn edge_at(&self, x: f32, y: f32) -> Option<Edge> {
        if !self.sizes {
            return None;
        }
        let w = self.whole;
        let near = |p: f32, lo: f32, hi: f32| -> i8 {
            if p >= lo - EDGE_OUT && p < lo + EDGE_IN {
                -1
            } else if p > hi - EDGE_IN && p <= hi + EDGE_OUT {
                1
            } else {
                0
            }
        };
        let (ex, ey) = (near(x, w.x, right(&w)), near(y, w.y, bottom(&w)));
        let along_x = x >= w.x - EDGE_OUT && x <= right(&w) + EDGE_OUT;
        let along_y = y >= w.y - EDGE_OUT && y <= bottom(&w) + EDGE_OUT;
        // a corner's grip reaches along both of its edges
        let cx = if x < w.x + CORNER { -1 } else if x > right(&w) - CORNER { 1 } else { 0 };
        let cy = if y < w.y + CORNER { -1 } else if y > bottom(&w) - CORNER { 1 } else { 0 };
        match (ex, ey) {
            (0, 0) => None,
            (ex, 0) if along_y => Some(Edge { x: ex, y: cy }),
            (0, ey) if along_x => Some(Edge { x: cx, y: ey }),
            (ex, ey) if ex != 0 && ey != 0 => Some(Edge { x: ex, y: ey }),
            _ => None,
        }
    }

    /// the thumb's place for a scroll
    fn thumb_at(track: Rect, over: f32, scroll: f32, min: f32) -> Rect {
        if over <= 0.0 {
            return Rect { w: 0.0, h: 0.0, ..track };
        }
        let h = (track.h * track.h / (track.h + over)).max(min).min(track.h);
        Rect { y: track.y + (track.h - h) * (scroll / over).clamp(0.0, 1.0), h, ..track }
    }
}

/// what the pointer holds
#[derive(Debug, Clone, Copy, PartialEq)]
enum Hold {
    /// the panel moved, where the pointer holds it (from its top left)
    Move(usize, f32, f32),
    /// the panel resized by these edges, its rectangle and the pointer when the press began
    Size(usize, Edge, Rect, (f32, f32)),
    /// the scrollbar's thumb, where the pointer holds it (from the thumb's top)
    Scroll(usize, f32),
}

impl Hold {
    fn panel(self) -> usize {
        match self {
            Hold::Move(i, ..) | Hold::Size(i, ..) | Hold::Scroll(i, _) => i,
        }
    }
}

#[derive(Debug, Clone)]
struct Panel {
    spec: Spec,
    /// its name's width in the pill, when measured
    name_w: Option<f32>,
    place: Place,
    scroll: f32,
    /// its content scrolls itself (a table under sticky heads): the panel only grows and clips
    own: bool,
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
    hold: Option<Hold>,
    /// the floating panels' paint order, the last on top
    z: Vec<usize>,
    /// the panels lying on the stage and their cells
    staged: Vec<(usize, Rect)>,
    /// a panel dragged off the stage by its handle, until the caller asks (`left_stage`)
    unstaged: Option<&'static str>,
    /// where the press began and whether the pointer has left it since (a press that stays is a click)
    pressed: (f32, f32),
    travelled: bool,
}

/// how far a press may wander and still be a click
const CLICK: f32 = 4.0;

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

#[cfg(test)]
fn inside(r: &Rect, area: &Rect) -> bool {
    r.x >= area.x - EPS && r.y >= area.y - EPS && right(r) <= right(area) + EPS && bottom(r) <= bottom(area) + EPS
}

/// `r` moved into `area` as far as it fits (its top left first)
fn clamp_into(r: Rect, area: &Rect) -> Rect {
    let x = r.x.min(right(area) - r.w).max(area.x);
    let y = r.y.min(bottom(area) - r.h).max(area.y);
    Rect { x, y, ..r }
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

/// an inline panel's chrome: folded the whole icon is its handle; open the grip and icon lead its one row, the chevron
/// and the close end it
fn inline_view(whole: Rect, collapsed: bool, closes: bool, m: &Metrics) -> View {
    let end = Rect { x: right(&whole), w: 0.0, ..whole };
    if collapsed {
        return View { handle: whole, fold: end, close: end, ..View::empty(whole, collapsed) };
    }
    let b = m.button();
    let close = if closes { Rect { x: right(&whole) - m.pad - b, w: b + m.pad, ..whole } } else { end };
    let fold = Rect { x: (if closes { close.x } else { right(&whole) - m.pad }) - b, w: if closes { b } else { b + m.pad }, ..whole };
    let handle = Rect { w: m.lead().min(whole.w), ..whole };
    let x = whole.x + handle.w;
    let content = Rect { x, y: whole.y + m.pad * 0.5, w: (fold.x - x).max(0.0), h: (whole.h - m.pad).max(0.0) };
    View { handle, fold, close, content, ..View::empty(whole, collapsed) }
}

impl Floats {
    pub fn new(specs: &[Spec]) -> Floats {
        let empty = View::empty(Rect::default(), false);
        let panels = specs.iter().map(|s| Panel { spec: *s, name_w: None, place: s.place, scroll: 0.0, own: false, shown: false, content: (0.0, 0.0), want: (0.0, 0.0), min: (0.0, 0.0), corner: s.place.corner, anchor: (0.0, 0.0), at: (0.0, 0.0), view: empty }).collect();
        let z = (0..specs.len()).collect();
        Floats { panels, area: Rect::default(), fixed: Vec::new(), m: Metrics { gap: 16.0, pad: 8.0, head: 28.0, pill: 32.0, char_w: 7.0, rem: 14.0, min: (160.0, 48.0) }, hold: None, pressed: (0.0, 0.0), travelled: false, z, staged: Vec::new(), unstaged: None }
    }

    /// `i` on top of the floating panels
    fn raise(&mut self, i: usize) {
        self.z.retain(|&j| j != i);
        self.z.push(i);
    }

    /// the panel under the pointer comes on top (a press on its ground); its id
    pub fn raise_at(&mut self, x: f32, y: f32) -> Option<&'static str> {
        let i = self.top_index(x, y)?;
        if !self.is_staged(i) {
            self.raise(i);
        }
        Some(self.panels[i].spec.id)
    }

    fn is_staged(&self, i: usize) -> bool {
        self.staged.iter().any(|(j, _)| *j == i)
    }

    /// the shown panels from the top down: the floating ones last touched first, then those on the stage
    fn top_down(&self) -> Vec<usize> {
        let floating = self.z.iter().rev().copied().filter(|&i| self.panels[i].shown && !self.is_staged(i));
        floating.chain(self.staged.iter().map(|(i, _)| *i).filter(|&i| self.panels[i].shown)).collect()
    }

    /// the panel whose ground or grip is uppermost at the pointer
    fn top_index(&self, x: f32, y: f32) -> Option<usize> {
        self.top_down().into_iter().find(|&i| self.panels[i].view.whole.has(x, y) || self.panels[i].view.edge_at(x, y).is_some())
    }

    /// the shown panels in the order they are painted: the ones on the stage, then the floating ones, the top last
    pub fn paint_order(&self) -> Vec<&'static str> {
        let mut out = self.top_down();
        out.reverse();
        out.into_iter().map(|i| self.panels[i].spec.id).collect()
    }

    /// the grounds of the shown panels painted over `id`
    pub fn above(&self, id: &str) -> Vec<Rect> {
        let order = self.top_down();
        let Some(at) = self.index(id).and_then(|i| order.iter().position(|&j| j == i)) else { return Vec::new() };
        order[..at].iter().map(|&i| self.panels[i].view.whole).collect()
    }

    /// The panels lying on the stage this frame and their cells (none: every panel floats); the one in the hand floats
    pub fn set_stage(&mut self, cells: &[(&str, Rect)]) {
        let held = self.hold.map(Hold::panel);
        self.staged = cells.iter().filter_map(|(id, r)| self.index(id).filter(|i| Some(*i) != held).map(|i| (i, *r))).collect();
    }

    /// whether a panel lies on the stage
    pub fn staged(&self, id: &str) -> bool {
        self.index(id).is_some_and(|i| self.is_staged(i))
    }

    /// a panel dragged off the stage by its handle since the last ask
    pub fn left_stage(&mut self) -> Option<&'static str> {
        self.unstaged.take()
    }

    /// the panel in the hand (moved by its handle)
    pub fn moving(&self) -> Option<&'static str> {
        match self.hold {
            Some(Hold::Move(i, ..)) => Some(self.panels[i].spec.id),
            _ => None,
        }
    }

    /// the smallest size an open panel resizes to: its chrome's row, a little of its content
    fn least(&self, p: &Panel) -> (f32, f32) {
        let m = &self.m;
        (m.pill_size(p.spec.name, p.name_w, !p.spec.essential).0, m.head + m.pad + m.min.1 * 0.5)
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.panels.iter().position(|p| p.spec.id == id)
    }

    /// the places to remember
    pub fn places(&self) -> BTreeMap<String, Place> {
        self.panels.iter().map(|p| (p.spec.id.to_string(), p.place)).collect()
    }

    /// places remembered before (ids no panel has are left; an essential panel is never closed)
    pub fn restore(&mut self, places: &BTreeMap<String, Place>) {
        for p in &mut self.panels {
            if let Some(place) = places.get(p.spec.id) {
                p.place = Place { closed: place.closed && !p.spec.essential, ..*place };
            }
        }
    }

    /// the panels' names measured as their pills set them
    pub fn measure_names(&mut self, width: impl Fn(&str) -> f32) {
        for p in &mut self.panels {
            p.name_w = Some(width(p.spec.name));
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
            if let Some(i) = self.index(s.id).filter(|&i| !self.panels[i].place.closed) {
                self.panels[i].shown = true;
                self.panels[i].content = (s.content.0.max(0.0), s.content.1.max(0.0));
            }
        }
        let order: Vec<usize> = (0..self.panels.len()).filter(|&i| self.panels[i].shown && !self.is_staged(i)).collect();
        for &i in &order {
            let p = &mut self.panels[i];
            let pill = m.pill_size(p.spec.name, p.name_w, !p.spec.essential);
            if p.spec.inline {
                p.want = if p.place.collapsed { (m.pill, m.pill) } else { m.inline_frame(p.content, !p.spec.essential) };
                p.want = (p.want.0.min(area.w), p.want.1.min(area.h));
                p.min = p.want;
            } else if p.place.collapsed {
                p.want = (pill.0.min(area.w), pill.1.min(area.h));
                p.min = p.want;
            } else {
                let full = p.place.size.unwrap_or_else(|| m.frame(p.content));
                p.want = (full.0.max(pill.0).min(area.w), full.1.min(area.h));
                p.min = (p.want.0.min(m.min.0.max(pill.0)), p.want.1.min(m.head + m.min.1));
            }
            // the one in the hand is held by its top left where the pointer has it
            let held = matches!(self.hold, Some(Hold::Move(d, ..) | Hold::Size(d, ..)) if d == i);
            if held {
                p.corner = Corner::TopLeft;
                p.anchor = p.at;
            } else {
                let Place { corner, dx, dy, .. } = p.place;
                p.corner = corner;
                p.anchor = (if corner.right() { right(&area) - dx } else { area.x + dx }, if corner.bottom() { bottom(&area) - dy } else { area.y + dy });
            }
        }
        // the smallest rectangles first, each only held inside the walls: nothing is pushed aside
        let mut mins: Vec<Rect> = Vec::with_capacity(order.len());
        for &i in &order {
            let p = &self.panels[i];
            let r = clamp_into(from_corner(p.anchor, p.corner, p.min), &area);
            let p = &mut self.panels[i];
            p.anchor = corner_of(&r, p.corner);
            mins.push(r);
        }
        // then each grows from its corner to its content while there is room: a size the viewer gave is taken as it is;
        // a content stops the gap before a panel clear of it (the ones before it as they grew, the later ones as large as
        // they want to be); one it already lies over is no wall
        let wants: Vec<Rect> = order.iter().map(|&i| intersect(&from_corner(self.panels[i].anchor, self.panels[i].corner, self.panels[i].want), &area)).collect();
        let mut grown: Vec<Rect> = Vec::with_capacity(order.len());
        for (k, &i) in order.iter().enumerate() {
            let p = &self.panels[i];
            let (c, m0) = (p.corner, mins[k]);
            let obstacles: Vec<Rect> = if p.place.size.is_some() { Vec::new() } else { self.fixed.iter().chain(&grown).chain(&wants[k + 1..]).filter(|o| !crowd(o, &m0, gap)).copied().collect() };
            let w = p.want.0.min(reach(p.anchor.0, c.right(), (m0.y, bottom(&m0)), &obstacles, &area, gap, true)).max(p.min.0);
            let band = from_corner(p.anchor, c, (w, p.min.1));
            let h = p.want.1.min(reach(p.anchor.1, c.bottom(), (band.x, right(&band)), &obstacles, &area, gap, false)).max(p.min.1);
            let whole = from_corner(p.anchor, c, (w, h));
            let p = &mut self.panels[i];
            p.at = (whole.x, whole.y);
            grown.push(whole);
            let collapsed = p.place.collapsed;
            if p.spec.inline {
                p.view = inline_view(whole, collapsed, !p.spec.essential, &m);
                p.scroll = 0.0;
                continue;
            }
            // the chrome's row: the handle, the chevron, the close last; the end's padding belongs to the last button
            let (b, end) = (m.button(), if collapsed { m.pad * 0.5 } else { m.pad });
            let head = if collapsed { whole } else { Rect { h: m.head, ..whole } };
            let close = if p.spec.essential { Rect { x: right(&whole) - end, w: 0.0, ..head } } else { Rect { x: right(&whole) - end - b, w: b + end, ..head } };
            let fold = Rect { x: close.x - b, w: if p.spec.essential { b + end } else { b }, ..head };
            let handle = Rect { w: (fold.x - whole.x).max(0.0), ..head };
            let content = if collapsed { Rect { x: whole.x, y: whole.y, w: 0.0, h: 0.0 } } else { Rect { x: whole.x + m.pad, y: whole.y + m.head, w: (w - 2.0 * m.pad).max(0.0), h: (h - m.head - m.pad).max(0.0) } };
            let over = if collapsed || p.own { 0.0 } else { Some(p.content.1 - content.h).filter(|o| *o > EPS).unwrap_or(0.0) };
            p.scroll = p.scroll.clamp(0.0, over);
            let track = if over > 0.0 { Rect { x: right(&whole) - m.pad, y: content.y, w: m.pad - EDGE_IN, h: content.h } } else { Rect { w: 0.0, h: 0.0, ..content } };
            let thumb = View::thumb_at(track, over, p.scroll, m.rem * 1.5);
            p.view = View { whole, handle, fold, close, content, scroll: p.scroll, over, collapsed, track, thumb, sizes: !collapsed, staged: false };
        }
        // the panels on the stage fill their cells: a head to drag them off, no fold, no close, no resize
        for k in 0..self.staged.len() {
            let (i, whole) = self.staged[k];
            let p = &mut self.panels[i];
            if !p.shown {
                continue;
            }
            let head = Rect { h: m.head.min(whole.h), ..whole };
            let end = Rect { x: right(&whole), w: 0.0, ..head };
            let content = Rect { x: whole.x + m.pad, y: whole.y + m.head, w: (whole.w - 2.0 * m.pad).max(0.0), h: (whole.h - m.head - m.pad).max(0.0) };
            let over = if p.own { 0.0 } else { Some(p.content.1 - content.h).filter(|o| *o > EPS).unwrap_or(0.0) };
            p.scroll = p.scroll.clamp(0.0, over);
            let track = if over > 0.0 { Rect { x: right(&whole) - m.pad, y: content.y, w: m.pad - EDGE_IN, h: content.h } } else { Rect { w: 0.0, h: 0.0, ..content } };
            let thumb = View::thumb_at(track, over, p.scroll, m.rem * 1.5);
            p.at = (whole.x, whole.y);
            p.view = View { whole, handle: head, fold: end, close: end, content, scroll: p.scroll, over, collapsed: false, track, thumb, sizes: false, staged: true };
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
        let p = &self.panels[self.top_index(x, y)?];
        {
            if let Some(e) = p.view.edge_at(x, y) {
                Some((&p.spec, Chrome::Edge(e)))
            } else if p.view.track.has(x, y) {
                Some((&p.spec, Chrome::Scroll))
            } else if p.view.close.has(x, y) {
                Some((&p.spec, Chrome::Close))
            } else if p.view.fold.has(x, y) {
                Some((&p.spec, Chrome::Fold))
            } else if p.view.handle.has(x, y) {
                Some((&p.spec, Chrome::Handle))
            } else {
                None
            }
        }
    }

    /// is a shown panel under the pointer (its ground keeps the press from the stage)
    /// is a shown panel or an edge's grip under the pointer (its ground keeps the press from the stage)
    pub fn covers(&self, x: f32, y: f32) -> bool {
        self.shown().any(|(_, p)| p.view.whole.has(x, y) || p.view.edge_at(x, y).is_some())
    }

    /// is a shown panel painted over `id` under the pointer
    pub fn covered(&self, id: &str, x: f32, y: f32) -> bool {
        self.above(id).iter().any(|r| r.has(x, y))
    }

    /// a press on a handle takes the panel in the hand, on an edge resizes it, on the scrollbar scrolls it (the track
    /// jumps the thumb there and holds it), on a chevron folds or opens it, on the close closes it: whether the press
    /// was the chrome's
    pub fn press(&mut self, x: f32, y: f32) -> bool {
        let Some((id, chrome)) = self.chrome_at(x, y).map(|(s, c)| (s.id, c)) else { return false };
        let Some(i) = self.index(id) else { return false };
        self.pressed = (x, y);
        self.travelled = false;
        if !self.is_staged(i) {
            self.raise(i);
        }
        match chrome {
            Chrome::Fold => self.toggle(id),
            Chrome::Close => {
                self.close(id);
            }
            Chrome::Handle if self.is_staged(i) => {
                // off the stage: it floats again at its own size, held where the pointer took its head
                self.staged.retain(|(j, _)| *j != i);
                self.unstaged = Some(self.panels[i].spec.id);
                self.raise(i);
                let at = self.panels[i].view.whole;
                let w = self.panels[i].want.0.min(at.w);
                let ox = (x - at.x).min(w * 0.5);
                self.panels[i].view.whole.w = w;
                self.panels[i].view.whole.h = self.panels[i].want.1.min(at.h);
                self.panels[i].view.whole.x = x - ox;
                self.panels[i].at = (x - ox, at.y);
                self.hold = Some(Hold::Move(i, ox, y - at.y));
            }
            Chrome::Handle => {
                let at = self.panels[i].view.whole;
                self.hold = Some(Hold::Move(i, x - at.x, y - at.y));
            }
            Chrome::Edge(e) => self.hold = Some(Hold::Size(i, e, self.panels[i].view.whole, (x, y))),
            Chrome::Scroll => {
                let thumb = self.panels[i].view.thumb;
                let grab = if thumb.has(x, y) { y - thumb.y } else { thumb.h * 0.5 };
                self.hold = Some(Hold::Scroll(i, grab));
                self.scroll_to(i, y);
            }
        }
        true
    }

    /// a panel moved, resized or scrolled by the pointer
    pub fn dragging(&self) -> bool {
        self.hold.is_some()
    }

    /// the edges of the panel being resized
    pub fn resizing(&self) -> Option<Edge> {
        match self.hold {
            Some(Hold::Size(_, e, ..)) => Some(e),
            _ => None,
        }
    }

    /// the panel in the hand follows the pointer, held inside the walls (it may pass over another); a resized one's edges
    /// stop only at the walls; a held thumb scrolls
    pub fn drag_to(&mut self, x: f32, y: f32) -> bool {
        let Some(hold) = self.hold else { return false };
        if (x - self.pressed.0).abs() + (y - self.pressed.1).abs() > CLICK {
            self.travelled = true;
        }
        let i = hold.panel();
        match hold {
            Hold::Move(_, ox, oy) => {
                let r = clamp_into(Rect { x: x - ox, y: y - oy, ..self.panels[i].view.whole }, &self.area);
                let p = &mut self.panels[i];
                p.at = (r.x, r.y);
                p.view.whole.x = r.x;
                p.view.whole.y = r.y;
            }
            Hold::Size(_, e, start, (px, py)) => {
                let r = self.resized(i, e, start, (x - px, y - py), &[]);
                let p = &mut self.panels[i];
                p.at = (r.x, r.y);
                p.place.size = Some((r.w, r.h));
                p.view.whole = r;
            }
            Hold::Scroll(..) => self.scroll_to(i, y),
        }
        true
    }

    /// `start` with the edges `e` moved by `d`: no smaller than the panel's least size, no further than a wall or the
    /// gap before an obstacle
    fn resized(&self, i: usize, e: Edge, start: Rect, (dx, dy): (f32, f32), obstacles: &[Rect]) -> Rect {
        let (least_w, least_h) = self.least(&self.panels[i]);
        let (area, gap) = (&self.area, self.m.gap);
        let mut r = start;
        if e.x != 0 {
            let back = e.x < 0;
            let edge = if back { right(&r) } else { r.x };
            let most = reach(edge, back, (r.y, bottom(&r)), obstacles, area, gap, true).max(least_w);
            r.w = (if back { start.w - dx } else { start.w + dx }).clamp(least_w, most);
            r.x = if back { edge - r.w } else { edge };
        }
        if e.y != 0 {
            let back = e.y < 0;
            let edge = if back { bottom(&r) } else { r.y };
            let most = reach(edge, back, (r.x, right(&r)), obstacles, area, gap, false).max(least_h);
            r.h = (if back { start.h - dy } else { start.h + dy }).clamp(least_h, most);
            r.y = if back { edge - r.h } else { edge };
        }
        r
    }

    /// the held thumb's top follows the pointer at `y`
    fn scroll_to(&mut self, i: usize, y: f32) {
        let Some(Hold::Scroll(_, grab)) = self.hold else { return };
        let v = self.panels[i].view;
        let room = (v.track.h - v.thumb.h).max(EPS);
        self.set_scroll(i, (y - grab - v.track.y) / room * v.over);
    }

    fn set_scroll(&mut self, i: usize, s: f32) {
        let p = &mut self.panels[i];
        p.scroll = s.clamp(0.0, p.view.over.max(0.0));
        p.view.scroll = p.scroll;
        p.view.thumb = View::thumb_at(p.view.track, p.view.over, p.scroll, self.m.rem * 1.5);
    }

    /// let go: the place (and a size given) is kept from the nearest corner; a folded inline panel clicked without moving
    /// opens; whether its place changed
    pub fn release(&mut self) -> bool {
        let Some(hold) = self.hold.take() else { return false };
        let i = hold.panel();
        if matches!(hold, Hold::Scroll(..)) {
            return false;
        }
        let area = self.area;
        let p = &mut self.panels[i];
        if matches!(hold, Hold::Move(..)) && p.spec.inline && p.place.collapsed && !self.travelled {
            p.place.collapsed = false;
            return true;
        }
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

    /// close a panel that is not essential: whether it closed
    pub fn close(&mut self, id: &str) -> bool {
        let Some(i) = self.index(id).filter(|&i| !self.panels[i].spec.essential) else { return false };
        self.panels[i].place.closed = true;
        if self.hold.is_some_and(|h| h.panel() == i) {
            self.hold = None;
        }
        true
    }

    /// a closed panel back, open: at its place, or the nearest free one the next layout finds
    pub fn reopen(&mut self, id: &str) {
        if let Some(i) = self.index(id) {
            let p = &mut self.panels[i].place;
            p.closed = false;
            p.collapsed = false;
        }
    }

    pub fn closed(&self, id: &str) -> bool {
        self.index(id).is_some_and(|i| self.panels[i].place.closed)
    }

    /// every panel's spec, in their order
    pub fn all(&self) -> Vec<&Spec> {
        self.panels.iter().map(|p| &p.spec).collect()
    }

    /// the panels that close, in their order, and whether each is open
    pub fn closable(&self) -> Vec<(&Spec, bool)> {
        self.panels.iter().filter(|p| !p.spec.essential).map(|p| (&p.spec, !p.place.closed)).collect()
    }

    /// the wheel over an open panel whose content is taller than it: whether it scrolled
    pub fn wheel(&mut self, x: f32, y: f32, steps: f32) -> bool {
        let Some(i) = self.shown().find(|(_, p)| p.view.whole.has(x, y) && p.view.over > 0.0).map(|(i, _)| i) else { return false };
        self.set_scroll(i, self.panels[i].scroll - steps * STEP);
        true
    }

    /// the content's own rectangle of a shown open panel: as wide as it shows, as tall as it is, moved up by the scroll
    pub fn content(&self, id: &str) -> Option<Rect> {
        let v = self.view(id).filter(|v| !v.collapsed)?;
        let p = &self.panels[self.index(id)?];
        let h = if p.own { v.content.h } else { p.content.1.max(v.content.h) };
        Some(Rect { x: v.content.x, y: v.content.y - v.scroll, w: v.content.w, h })
    }

    /// a panel whose content scrolls itself: it grows to its content and is clipped, it never scrolls
    pub fn scrolls_itself(&mut self, id: &str) {
        if let Some(i) = self.index(id) {
            self.panels[i].own = true;
        }
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

/// `r` less the part `o` covers: the largest side left (none: all covered)
fn trim(r: Rect, o: &Rect) -> Option<Rect> {
    if !crowd(&r, o, 0.0) {
        return Some(r);
    }
    let sides = [
        Rect { w: o.x - r.x, ..r },
        Rect { x: right(o), w: right(&r) - right(o), ..r },
        Rect { h: o.y - r.y, ..r },
        Rect { y: bottom(o), h: bottom(&r) - bottom(o), ..r },
    ];
    sides.into_iter().filter(|s| s.w > EPS && s.h > EPS).max_by(|a, b| (a.w * a.h).total_cmp(&(b.w * b.h)))
}

/// the places a panel's content pressed this frame, from `from` on, less what the panels painted over it cover
pub fn clip_hits_under<H>(above: &[Rect], hits: &mut Vec<(Rect, H)>, from: usize) {
    if above.is_empty() {
        return;
    }
    let kept: Vec<(Rect, H)> = hits.drain(from..).filter_map(|(r, h)| above.iter().try_fold(r, trim).map(|r| (r, h))).collect();
    hits.extend(kept);
}

/// a panel's ground and chrome; open, the content is clipped to its place until `end`
pub fn begin(d: &mut DrawList, t: &Tokens, spec: &Spec, v: &View, pointer: (f32, f32)) {
    let w = v.whole;
    let r = t.size.radius * 2.0;
    let s = t.kit("panel", "rest");
    let r = if v.collapsed || spec.inline { w.h * 0.5 } else { r };
    // a view on the stage lies on the stage's own ground, a hairline around it
    let ground = if v.staged { t.color.stage } else { s.ground.unwrap_or(t.color.panel()) };
    d.round_rect(w.x, w.y, w.w, w.h, r, ground);
    d.outline(w.x, w.y, w.w, w.h, (r, 1.0), s.border.unwrap_or(t.color.line));
    d.halo(t.color.ring());
    let hot = v.handle.has(pointer.0, pointer.1);
    let mid = v.handle.y + v.handle.h * 0.5;
    let x = w.x + t.size.pad;
    let (grip, icon, chevron) = (t.size.rem(0.86), t.size.rem(1.15), t.size.rem(1.0));
    if spec.inline && v.collapsed {
        d.icon(w.x + (w.w - icon) * 0.5, mid - icon * 0.5, icon, spec.icon, if hot { t.color.bright } else { t.color.text });
        d.halo(None);
        return;
    }
    // the title carries no border at rest; under the pointer (and so while it is dragged) a rounded one, as a tab's
    if hot {
        let named = v.collapsed && !spec.inline;
        let end = if named { x + t.size.rem(2.72) + d.sans_width(t.size.font, spec.name) } else { x + t.size.rem(1.15) + icon };
        let h = (v.handle.h - 4.0).clamp(0.0, t.size.row);
        let r = Rect { x: x - t.size.pad * 0.5, y: mid - h * 0.5, w: end - x + t.size.pad, h };
        if let Some(line) = t.kit("tab", "hover").border {
            d.outline(r.x, r.y, r.w, r.h, (h * 0.5, t.size.edge), line);
        }
    }
    d.icon(x, mid - grip * 0.5, grip, "grip", if hot { t.color.text } else { t.color.muted });
    d.icon(x + t.size.rem(1.15), mid - icon * 0.5, icon, spec.icon, t.color.text);
    if v.collapsed && !spec.inline {
        d.sans(x + t.size.rem(2.72), mid - t.size.font * 0.62, t.size.font, spec.name, t.color.text);
    }
    let fold_hot = v.fold.has(pointer.0, pointer.1);
    d.icon(v.fold.x + (v.fold.w - chevron) * 0.5, mid - chevron * 0.5, chevron, if v.collapsed { "down" } else { "up" }, if fold_hot { t.color.text } else { t.color.muted });
    if v.close.w > 0.0 {
        let hot = v.close.has(pointer.0, pointer.1);
        d.icon(v.close.x + (v.fold.w - chevron) * 0.5, mid - chevron * 0.5, chevron, "close", if hot { t.color.text } else { t.color.muted });
    }
    if v.over > 0.0 {
        // the scrollbar: thin at rest, wider under the pointer
        let (tr, th) = (v.track, v.thumb);
        let hot = tr.has(pointer.0, pointer.1);
        let w = if hot { 5.0 } else { 3.0 };
        let x = right(&tr) - w;
        if hot {
            d.round_rect(x, tr.y, w, tr.h, w * 0.5, t.color.line);
        }
        d.round_rect(x, th.y, w, th.h, w * 0.5, if hot { t.color.text } else { t.color.muted });
    }
    match v.collapsed {
        true => d.halo(None),
        false => d.clip(Some(v.content)),
    }
}

/// after the content: the clip ends
pub fn end(d: &mut DrawList, v: &View) {
    if v.collapsed {
        return;
    }
    d.halo(None);
    d.clip(None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::tokens::{Colors, Theme};

    const M: Metrics = Metrics { gap: 16.0, pad: 8.0, head: 28.0, pill: 32.0, char_w: 7.0, rem: 14.0, min: (160.0, 48.0) };

    fn area(w: f32, h: f32) -> Rect {
        Rect { x: 16.0, y: 64.0, w: w - 32.0, h: h - 80.0 }
    }

    fn spec(id: &'static str, place: Place) -> Spec {
        Spec { id, name: id, icon: "list", about: "", place, essential: false, inline: false }
    }

    /// the four and a toolbar at the top's middle
    fn with_bar() -> Floats {
        let mut specs: Vec<Spec> = four().panels.iter().map(|p| p.spec).collect();
        specs.push(spec("bar", Place::at(Corner::TopLeft, 600.0, 0.0)).inline().essential());
        Floats::new(&specs)
    }

    fn shown_bar() -> Vec<Shown> {
        let mut s = shown(SIZES);
        s.push(Shown { id: "bar", content: (300.0, 24.0) });
        s
    }

    #[test]
    fn an_inline_panel_is_one_row_its_chrome_beside_its_content() {
        let mut f = with_bar();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown_bar(), M);
        let v = f.view("bar").unwrap();
        assert_eq!(v.whole.h, M.head + M.pad, "one row");
        assert_eq!((v.handle.y, v.handle.h), (v.whole.y, v.whole.h), "the handle is beside, not above");
        assert!(right(&v.handle) <= v.content.x + EPS && right(&v.content) <= v.fold.x + EPS, "grip, row, chevron");
        assert!(v.content.y > v.whole.y && bottom(&v.content) < bottom(&v.whole));
        assert!(v.content.w >= 300.0 - EPS, "the row fits");
        assert_eq!(v.close.w, 0.0, "essential: it folds, never closes");
        assert_valid(&f, &a);
    }

    #[test]
    fn a_folded_inline_panel_is_its_icon_and_a_click_opens_it_a_drag_moves_it() {
        let mut f = with_bar();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown_bar(), M);
        let v = f.view("bar").unwrap();
        assert!(f.press(v.fold.x + 2.0, v.fold.y + 2.0));
        f.layout(a, &[], &shown_bar(), M);
        let p = f.view("bar").unwrap();
        assert!(p.collapsed);
        assert_eq!((p.whole.w, p.whole.h), (M.pill, M.pill), "only the icon");
        assert_eq!((p.fold.w, p.close.w), (0.0, 0.0), "no chevron, no close");
        assert_eq!(p.handle, p.whole, "all of it is the handle");
        // a drag moves it and leaves it folded
        assert!(f.press(p.whole.x + 4.0, p.whole.y + 4.0));
        f.drag_to(p.whole.x + 104.0, p.whole.y + 54.0);
        f.release();
        f.layout(a, &[], &shown_bar(), M);
        let moved = f.view("bar").unwrap();
        assert!(moved.collapsed && moved.whole.x > p.whole.x + 50.0, "{moved:?}");
        // a click opens it
        assert!(f.press(moved.whole.x + 4.0, moved.whole.y + 4.0));
        f.drag_to(moved.whole.x + 5.0, moved.whole.y + 5.0);
        f.release();
        f.layout(a, &[], &shown_bar(), M);
        assert!(!f.view("bar").unwrap().collapsed);
        assert_valid(&f, &a);
    }

    fn four() -> Floats {
        Floats::new(&[
            spec("tree", Place::at(Corner::TopLeft, 0.0, 0.0)),
            spec("inspector", Place::at(Corner::TopRight, 0.0, 0.0)).essential(),
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

    /// inside the walls and, laid out from their places, apart
    fn assert_valid(f: &Floats, a: &Rect) {
        let r = rects(f);
        assert_inside(f, a);
        for (i, x) in r.iter().enumerate() {
            for (j, y) in r.iter().enumerate().skip(i + 1) {
                assert!(!crowd(x, y, 16.0), "panels {i} {x:?} and {j} {y:?} keep the gap");
            }
        }
    }

    /// every panel inside the walls (they may lie over each other)
    fn assert_inside(f: &Floats, a: &Rect) {
        for (i, x) in rects(f).iter().enumerate() {
            assert!(inside(x, a), "panel {i} {x:?} inside {a:?}");
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
    fn a_panel_whose_content_scrolls_itself_grows_and_is_clipped() {
        let mut f = four();
        f.scrolls_itself("inspector");
        let a = area(1920.0, 700.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("inspector").unwrap();
        assert_eq!(bottom(&v.whole), f.view("mixer").unwrap().whole.y - 16.0, "it grows as any panel");
        assert_eq!(v.over, 0.0, "the panel itself never scrolls");
        assert!(!f.wheel(v.whole.x + 5.0, v.whole.y + 50.0, -1.0));
        assert_eq!(f.content("inspector").unwrap().h, v.content.h, "its content is what shows");
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
    fn a_dragged_panel_passes_over_another_stays_where_let_go_and_lies_on_top() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("tree").unwrap();
        let other = f.view("inspector").unwrap().whole;
        f.press(v.handle.x + 4.0, v.handle.y + 4.0);
        f.drag_to(other.x + 4.0, v.handle.y + 4.0);
        let w = f.view("tree").unwrap().whole;
        assert_eq!(w.x, other.x, "no stop before the inspector, no snap");
        f.release();
        f.layout(a, &[], &shown(SIZES), M);
        let w = f.view("tree").unwrap().whole;
        assert_eq!((w.x, w.y), (other.x, v.whole.y), "it stays where it was let go, over the inspector");
        assert_eq!(f.view("inspector").unwrap().whole, other, "the inspector is not pushed");
        assert_eq!(f.paint_order().last(), Some(&"tree"), "the last touched on top");
        assert!(f.covered("inspector", w.x + 10.0, w.y + 10.0) && !f.covered("tree", w.x + 10.0, w.y + 10.0));
        assert_eq!(f.chrome_at(w.x + 4.0, w.y + 4.0).map(|(s, c)| (s.id, c)), Some(("tree", Chrome::Handle)), "the one on top takes the press");
        let i = f.view("inspector").unwrap();
        assert_eq!(f.raise_at(right(&i.whole) - 4.0, bottom(&i.whole) - 4.0), Some("inspector"), "a press on its ground raises the other");
        assert_eq!(f.paint_order().last(), Some(&"inspector"));
        assert_inside(&f, &a);
    }

    #[test]
    fn the_hits_of_a_panel_under_another_end_where_the_other_begins() {
        let over = [Rect { x: 50.0, y: 0.0, w: 100.0, h: 100.0 }];
        let mut hits = vec![(Rect { x: 0.0, y: 10.0, w: 80.0, h: 10.0 }, 0), (Rect { x: 60.0, y: 10.0, w: 20.0, h: 10.0 }, 1), (Rect { x: 200.0, y: 0.0, w: 5.0, h: 5.0 }, 2)];
        clip_hits_under(&over, &mut hits, 0);
        assert_eq!(hits.iter().map(|(_, h)| *h).collect::<Vec<_>>(), [0, 2], "one all under it is gone");
        assert_eq!(hits[0].0, Rect { x: 0.0, y: 10.0, w: 50.0, h: 10.0 });
    }

    #[test]
    fn a_panel_on_the_stage_fills_its_cell_under_the_floating_ones_and_drags_off() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        let cell = Rect { x: 0.0, y: 50.0, w: 960.0, h: 1030.0 };
        f.set_stage(&[("mixer", cell)]);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("mixer").unwrap();
        assert_eq!(v.whole, cell);
        assert!(!v.sizes && v.close.w == 0.0 && v.fold.w == 0.0, "no resize, fold or close on the stage");
        assert!(f.staged("mixer"));
        assert_eq!(f.paint_order().first(), Some(&"mixer"), "under every floating panel");
        let tree = f.view("tree").unwrap().whole;
        assert!(f.covered("mixer", tree.x + 2.0, tree.y + 2.0));
        // its handle takes it off the stage, floating at its own size
        assert!(f.press(cell.x + 500.0, cell.y + 4.0));
        assert_eq!(f.left_stage(), Some("mixer"));
        assert_eq!(f.left_stage(), None, "said once");
        assert_eq!(f.moving(), Some("mixer"));
        f.drag_to(cell.x + 600.0, cell.y + 300.0);
        f.release();
        f.set_stage(&[]);
        f.layout(a, &[], &shown(SIZES), M);
        let w = f.view("mixer").unwrap().whole;
        assert_eq!((w.w, w.h), M.frame(SIZES[2]), "its own size again");
        assert!(!f.staged("mixer"));
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
        assert_eq!((p.whole.w, p.whole.h), M.pill_size("tree", None, true));
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
        let w = f.view("tree").unwrap().whole;
        assert!(right(&w) > other.x && (right(&w) - right(&a)).abs() < EPS, "over the inspector to the wall: {w:?}");
    }

    #[test]
    fn every_panel_stays_inside_the_walls_when_the_window_changes() {
        let mut f = four();
        f.layout(area(1920.0, 1080.0), &[], &shown(SIZES), M);
        for (w, h) in [(1280.0, 720.0), (900.0, 600.0), (640.0, 480.0), (2560.0, 1440.0)] {
            let a = area(w, h);
            f.layout(a, &[], &shown(SIZES), M);
            assert_inside(&f, &a);
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
    fn a_panel_put_on_another_stays_there_and_pushes_nothing() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let tree = f.view("tree").unwrap().whole;
        let mut places = f.places();
        places.insert("mixer".into(), Place::at(Corner::TopLeft, 10.0, 10.0));
        f.restore(&places);
        f.layout(a, &[], &shown(SIZES), M);
        let m = f.view("mixer").unwrap().whole;
        assert_eq!((m.x, m.y), (a.x + 10.0, a.y + 10.0), "no snap to a free place");
        assert_eq!(f.view("tree").unwrap().whole, tree, "the one under it stays");
        assert_inside(&f, &a);
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
    fn any_mix_of_places_and_sizes_keeps_every_panel_inside() {
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
            assert_inside(&f, &a);
        }
    }

    #[test]
    fn a_panel_that_is_not_essential_closes_and_comes_back_where_it_was() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("tree").unwrap();
        assert!(v.close.w > 0.0 && v.close.x >= right(&v.fold) - EPS, "the close after the chevron");
        assert!(f.press(v.close.x + 2.0, v.close.y + 2.0));
        assert!(f.closed("tree"));
        f.layout(a, &[], &shown(SIZES), M);
        assert!(f.view("tree").is_none(), "a closed panel is not shown");
        assert!(!f.covers(v.whole.x + 4.0, v.whole.y + 4.0));
        assert!(f.closable().iter().any(|(s, open)| s.id == "tree" && !open));
        f.reopen("tree");
        f.layout(a, &[], &shown(SIZES), M);
        assert_eq!(f.view("tree").unwrap().whole, v.whole);
    }

    #[test]
    fn an_essential_panel_only_folds() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("inspector").unwrap();
        assert_eq!(v.close.w, 0.0, "no close");
        assert!(!f.close("inspector"));
        let mut places = f.places();
        places.get_mut("inspector").unwrap().closed = true;
        f.restore(&places);
        assert!(!f.closed("inspector"), "not even from a cache");
        assert!(f.closable().iter().all(|(s, _)| s.id != "inspector"));
        f.toggle("inspector");
        f.layout(a, &[], &shown(SIZES), M);
        assert_eq!(f.view("inspector").unwrap().whole.w, M.pill_size("inspector", None, false).0);
    }

    #[test]
    fn a_reopened_panel_comes_back_at_its_own_place_even_when_another_lies_there() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let tree = f.view("tree").unwrap().whole;
        f.close("tree");
        f.layout(a, &[], &shown(SIZES), M);
        let i = f.view("inspector").unwrap();
        f.press(i.handle.x + 4.0, i.handle.y + 4.0);
        f.drag_to(a.x - 5000.0, i.handle.y + 4.0);
        f.release();
        f.reopen("tree");
        f.layout(a, &[], &shown(SIZES), M);
        assert_eq!((f.view("tree").unwrap().whole.x, f.view("tree").unwrap().whole.y), (tree.x, tree.y));
        assert_inside(&f, &a);
    }

    #[test]
    fn closed_is_remembered_with_the_place() {
        let mut f = four();
        f.close("mixer");
        let kept = toml::to_string(&f.places()).unwrap();
        let mut g = four();
        g.restore(&toml::from_str(&kept).unwrap());
        assert!(g.closed("mixer"));
        let old: Place = toml::from_str("corner = \"TopLeft\"\ndx = 0.0\ndy = 0.0").unwrap();
        assert!(!old.closed && !old.collapsed, "a place kept before closing existed is open");
    }

    #[test]
    fn a_panels_ground_lets_the_stage_show_through() {
        for (theme, ring) in [(Theme::Night, 1.0), (Theme::Day, 0.0)] {
            let t = Tokens { color: Colors::of(theme), ..Tokens::default() };
            let mut d = DrawList::new(0.6);
            let v = View::empty(Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }, false);
            begin(&mut d, &t, &spec("x", Place::default()), &v, (0.0, 0.0));
            let crate::base::draw::Prim::Rect { color, .. } = d.prims()[0] else { panic!("the ground first") };
            assert_eq!(color[3], t.color.panel_opacity);
            assert!(color[3] < 1.0);
            d.text(0.0, 0.0, 10.0, "in", t.color.text);
            end(&mut d, &v);
            d.text(0.0, 0.0, 10.0, "out", t.color.text);
            let halos: Vec<f32> = d.prims().iter().filter_map(|p| if let crate::base::draw::Prim::Text { halo, .. } = *p { Some(halo[3]) } else { None }).collect();
            assert_eq!(halos, [ring, 0.0], "{theme:?}: night rings the content's text in the panel's colour, day does not; nothing after the panel");
        }
    }

    /// a press on `v`'s grip of `e`, a drag by (dx, dy) and the release
    fn resize(f: &mut Floats, v: &View, e: Edge, (dx, dy): (f32, f32)) {
        let w = v.whole;
        let at = |side: i8, lo: f32, hi: f32| match side {
            -1 => lo - 2.0,
            1 => hi + 2.0,
            _ => (lo + hi) * 0.5,
        };
        let (x, y) = (at(e.x, w.x, right(&w)), at(e.y, w.y, bottom(&w)));
        assert_eq!(f.chrome_at(x, y).map(|(_, c)| c), Some(Chrome::Edge(e)), "the grip of {e:?} at {x},{y}");
        assert!(f.press(x, y));
        assert_eq!(f.resizing(), Some(e));
        f.drag_to(x + dx, y + dy);
        assert!(f.release(), "a resize is remembered");
    }

    #[test]
    fn a_panel_resizes_from_every_edge_and_corner() {
        let a = area(1920.0, 1080.0);
        for ex in [-1i8, 0, 1] {
            for ey in [-1i8, 0, 1] {
                if ex == 0 && ey == 0 {
                    continue;
                }
                let mut f = four();
                let mut places = f.places();
                places.insert("tree".into(), Place::at(Corner::TopLeft, 300.0, 200.0));
                f.restore(&places);
                f.layout(a, &[], &shown(SIZES), M);
                let v = f.view("tree").unwrap();
                let e = Edge { x: ex, y: ey };
                resize(&mut f, &v, e, (40.0 * ex as f32, 30.0 * ey as f32));
                f.layout(a, &[], &shown(SIZES), M);
                let w = f.view("tree").unwrap().whole;
                let grew = |side: i8, d: f32| if side == 0 { 0.0 } else { d };
                assert_eq!((w.w, w.h), (v.whole.w + grew(ex, 40.0), v.whole.h + grew(ey, 30.0)), "{e:?} grows outwards");
                assert_eq!(w.x, if ex < 0 { v.whole.x - 40.0 } else { v.whole.x }, "{e:?}: the left edge moves only from the left");
                assert_eq!(w.y, if ey < 0 { v.whole.y - 30.0 } else { v.whole.y }, "{e:?}: the top moves only from the top");
                assert_inside(&f, &a);
            }
        }
    }

    #[test]
    fn a_resized_panel_stops_only_at_the_walls_moves_no_other_and_keeps_a_least_size() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("tree").unwrap();
        let other = f.view("inspector").unwrap().whole;
        resize(&mut f, &v, Edge { x: 1, y: 0 }, (5000.0, 0.0));
        f.layout(a, &[], &shown(SIZES), M);
        assert_eq!(right(&f.view("tree").unwrap().whole), right(&a), "to the wall, over the inspector");
        assert_eq!(f.view("inspector").unwrap().whole, other, "the inspector neither moves nor shrinks");
        let v = f.view("tree").unwrap();
        resize(&mut f, &v, Edge { x: -1, y: -1 }, (-5000.0, -5000.0));
        f.layout(a, &[], &shown(SIZES), M);
        let w = f.view("tree").unwrap().whole;
        assert_eq!((w.x, w.y), (a.x, a.y), "the walls hold it");
        let v = f.view("tree").unwrap();
        resize(&mut f, &v, Edge { x: 1, y: 1 }, (-5000.0, -5000.0));
        f.layout(a, &[], &shown(SIZES), M);
        let w = f.view("tree").unwrap().whole;
        assert_eq!((w.w, w.h), f.least(&f.panels[0]), "no smaller than its chrome and a little content");
        assert_inside(&f, &a);
    }

    #[test]
    fn a_size_is_remembered_with_the_place_and_a_smaller_one_scrolls() {
        let mut f = four();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("tree").unwrap();
        assert_eq!(v.over, 0.0);
        resize(&mut f, &v, Edge { x: 1, y: 1 }, (60.0, -150.0));
        f.layout(a, &[], &shown(SIZES), M);
        let sized = f.view("tree").unwrap();
        assert_eq!((sized.whole.w, sized.whole.h), (v.whole.w + 60.0, v.whole.h - 150.0));
        assert!(sized.over > 0.0, "the content is taller than the panel now: it scrolls");
        let kept = toml::to_string(&f.places()).unwrap();
        let mut g = four();
        g.restore(&toml::from_str(&kept).unwrap());
        g.layout(a, &[], &shown(SIZES), M);
        assert_eq!(g.view("tree").unwrap().whole, sized.whole, "the size comes back with the place");
        let old: Place = toml::from_str("corner = \"TopLeft\"\ndx = 0.0\ndy = 0.0").unwrap();
        assert_eq!(old.size, None, "a place kept before sizes existed sizes to its content");
        // a smaller window still keeps every panel inside
        for (w, h) in [(1280.0, 720.0), (800.0, 500.0)] {
            g.layout(area(w, h), &[], &shown(SIZES), M);
            assert_inside(&g, &area(w, h));
        }
    }

    #[test]
    fn pills_and_toolbars_do_not_resize() {
        let mut f = with_bar();
        let a = area(1920.0, 1080.0);
        f.layout(a, &[], &shown_bar(), M);
        for id in ["bar", "info"] {
            let w = f.view(id).unwrap().whole;
            assert!(!matches!(f.chrome_at(right(&w) + 2.0, w.y + w.h * 0.5), Some((_, Chrome::Edge(_)))), "{id}");
            assert!(!matches!(f.chrome_at(w.x + w.w * 0.5, bottom(&w) + 2.0), Some((_, Chrome::Edge(_)))), "{id}");
        }
    }

    #[test]
    fn a_click_on_the_track_jumps_there_and_the_thumb_drags() {
        let mut f = four();
        let a = area(1920.0, 700.0);
        f.layout(a, &[], &shown(SIZES), M);
        let v = f.view("inspector").unwrap();
        assert!(v.over > 0.0 && v.thumb.h > 0.0 && v.thumb.y == v.track.y, "{v:?}");
        let x = v.track.x + v.track.w * 0.5;
        // the track's bottom: the thumb jumps to the end and stays in the hand
        assert_eq!(f.chrome_at(x, bottom(&v.track) - 1.0).map(|(_, c)| c), Some(Chrome::Scroll));
        assert!(f.press(x, bottom(&v.track) - 1.0));
        assert_eq!(f.view("inspector").unwrap().scroll, v.over);
        f.drag_to(x, v.track.y + v.track.h * 0.5);
        let mid = f.view("inspector").unwrap();
        assert!(mid.scroll > 0.0 && mid.scroll < v.over, "dragged back up: {}", mid.scroll);
        assert!(!f.release(), "scrolling changes no place");
        // the thumb itself drags from where it is held, without a jump
        let held = mid.thumb;
        assert!(f.press(x, held.y + 2.0));
        assert_eq!(f.view("inspector").unwrap().scroll, mid.scroll, "a press on the thumb does not jump");
        f.drag_to(x, held.y + 2.0 - 5000.0);
        assert_eq!(f.view("inspector").unwrap().scroll, 0.0, "the top stops it");
        f.release();
        f.layout(a, &[], &shown(SIZES), M);
        assert_eq!(f.view("inspector").unwrap().scroll, 0.0, "the scroll holds through the next layout");
        assert!(f.view("tree").unwrap().track.h == 0.0, "a panel that fits has no scrollbar");
    }

    #[test]
    fn any_mix_of_resizes_keeps_every_panel_inside() {
        let mut seed = 11u32;
        let mut next = |n: f32| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (seed >> 8) as f32 / (1u32 << 24) as f32 * n
        };
        let a = area(1600.0, 900.0);
        let mut f = four();
        f.layout(a, &[], &shown(SIZES), M);
        for _ in 0..300 {
            let id = ["tree", "inspector", "mixer"][next(3.0) as usize % 3];
            let v = f.view(id).unwrap();
            let e = Edge { x: next(3.0) as i8 - 1, y: next(3.0) as i8 - 1 };
            if e == (Edge { x: 0, y: 0 }) {
                continue;
            }
            let w = v.whole;
            let at = |side: i8, lo: f32, hi: f32| [lo - 2.0, (lo + hi) * 0.5, hi + 2.0][(side + 1) as usize];
            if f.press(at(e.x, w.x, right(&w)), at(e.y, w.y, bottom(&w))) {
                f.drag_to(next(1600.0), next(900.0));
                f.release();
            }
            f.layout(a, &[], &shown(SIZES), M);
            assert_inside(&f, &a);
        }
    }

    #[test]
    fn hits_of_a_scrolled_content_end_at_its_edge() {
        let v = View { content: Rect { x: 0.0, y: 100.0, w: 200.0, h: 100.0 }, over: 50.0, ..View::empty(Rect::default(), false) };
        let mut hits = vec![(Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, 0), (Rect { x: 0.0, y: 90.0, w: 10.0, h: 20.0 }, 1), (Rect { x: 0.0, y: 150.0, w: 10.0, h: 10.0 }, 2), (Rect { x: 0.0, y: 250.0, w: 10.0, h: 10.0 }, 3)];
        clip_hits(&v, &mut hits, 1);
        assert_eq!(hits.iter().map(|(_, h)| *h).collect::<Vec<_>>(), [0, 1, 2], "the first is before `from`, the last below the edge");
        assert_eq!(hits[1].0, Rect { x: 0.0, y: 100.0, w: 10.0, h: 10.0 });
    }
}
