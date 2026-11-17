//! The library browser (owner 2026-11-16 [design]): one component for whatever source it is handed – modules,
//! instruments, the catalog, scenes, assets (deck-catalog `library`). A find field, a chip per kind there is (and
//! "related" when a character is open), cards in a grid or rows in a list, each with the preview the core chose for its
//! content. A card pressed and moved is dragged; let go, its owner is told where; a second press on it soon after opens it.
use crate::base::draw::{DrawList, Rect};
use crate::base::text::Field;
use crate::base::tokens::Tokens;
use crate::editor::controls;
use deck_catalog::library::{facets, find, Entry, Filter, Kind, Preview};

/// a card's width and its preview's height in the grid; a row's preview in the list
const CARD_W: f32 = 132.0;
const CARD_PREVIEW: f32 = 84.0;
const ROW_PREVIEW: (f32, f32) = (44.0, 28.0);
/// a press moved this far is a drag
const DRAG: f32 = 5.0;
/// a second press within this many seconds opens
const OPEN: f64 = 0.45;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Grid,
    List,
}

/// what a place in the browser does when pressed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Find,
    Facet(Kind),
    Related,
    Mode,
    /// the owner's one button (Show's "new scene")
    Action,
    /// a card, by its entry's index
    Card(usize),
}

/// a card in the hand
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Held {
    pub entry: usize,
    pub from: (f32, f32),
    pub at: (f32, f32),
    pub dragging: bool,
}

/// what a press or a release came to
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Did {
    /// it was the browser's, nothing for its owner
    Taken,
    Open(usize),
    /// a card pressed and let go without moving
    Click(usize),
    Action,
    /// a card let go over (x, y)
    Drop { entry: usize, at: (f32, f32) },
}

/// the browser's state; the entries stay their owner's
#[derive(Debug, Clone)]
pub struct Browser {
    pub filter: Filter,
    pub mode: Mode,
    /// the find field while it is typed into
    pub typing: Option<Field>,
    /// the character "related" filters by (its owner sets it: Build's open rig)
    pub character: Option<String>,
    pub held: Option<Held>,
    last: Option<(usize, f64)>,
    pub hits: Vec<(Rect, Hit)>,
}

impl Default for Browser {
    fn default() -> Browser {
        Browser { filter: Filter::default(), mode: Mode::Grid, typing: None, character: None, held: None, last: None, hits: Vec::new() }
    }
}

impl Browser {
    /// the filter as it stands, with what is being typed
    pub fn filter_now(&self) -> Filter {
        let mut f = self.filter.clone();
        if let Some(t) = &self.typing {
            f.query = t.text.clone();
        }
        if f.related.is_some() {
            f.related = self.character.clone();
        }
        f
    }

    /// the entries shown, in their order
    pub fn shown(&self, entries: &[Entry]) -> Vec<usize> {
        find(entries, &self.filter_now())
    }

    pub fn hit(&self, x: f32, y: f32) -> Option<Hit> {
        self.hits.iter().rev().find(|(r, _)| r.has(x, y)).map(|(_, h)| *h)
    }

    /// the find field's typing ends, its text kept
    pub fn stop_typing(&mut self) {
        if let Some(f) = self.typing.take() {
            self.filter.query = f.text;
        }
    }

    /// A press at (x, y) at `now` seconds: None = not the browser's.
    pub fn press(&mut self, x: f32, y: f32, now: f64) -> Option<Did> {
        let hit = self.hit(x, y)?;
        if hit != Hit::Find {
            self.stop_typing();
        }
        Some(match hit {
            Hit::Find => {
                if self.typing.is_none() {
                    self.typing = Some(Field::with(&self.filter.query));
                }
                Did::Taken
            }
            Hit::Facet(k) => {
                if let Some(i) = self.filter.kinds.iter().position(|x| *x == k) {
                    self.filter.kinds.remove(i);
                } else {
                    self.filter.kinds.push(k);
                }
                Did::Taken
            }
            Hit::Related => {
                self.filter.related = match self.filter.related {
                    Some(_) => None,
                    None => self.character.clone(),
                };
                Did::Taken
            }
            Hit::Mode => {
                self.mode = if self.mode == Mode::Grid { Mode::List } else { Mode::Grid };
                Did::Taken
            }
            Hit::Action => Did::Action,
            Hit::Card(i) => {
                if self.last.is_some_and(|(j, t)| j == i && now - t <= OPEN) {
                    self.last = None;
                    return Some(Did::Open(i));
                }
                self.last = Some((i, now));
                self.held = Some(Held { entry: i, from: (x, y), at: (x, y), dragging: false });
                Did::Taken
            }
        })
    }

    pub fn drag_to(&mut self, x: f32, y: f32) {
        if let Some(h) = self.held.as_mut() {
            h.at = (x, y);
            h.dragging |= (x - h.from.0).hypot(y - h.from.1) > DRAG;
        }
    }

    /// whether a card is being dragged
    pub fn dragging(&self) -> Option<usize> {
        self.held.filter(|h| h.dragging).map(|h| h.entry)
    }

    /// The button let go: a dragged card is dropped where the pointer is (its owner decides what takes it).
    pub fn release(&mut self, x: f32, y: f32) -> Option<Did> {
        let h = self.held.take()?;
        Some(if h.dragging { Did::Drop { entry: h.entry, at: (x, y) } } else { Did::Click(h.entry) })
    }

    /// The browser in `a`: the find field, the chips, the cards. `picture` answers which of the screen's pictures shows
    /// a preview (a still or a picture already loaded); `action` is the owner's one button.
    pub fn draw(&mut self, d: &mut DrawList, t: &Tokens, (a, pointer): (Rect, (f32, f32)), entries: &[Entry], picture: &dyn Fn(&Preview) -> Option<u64>, action: Option<&str>) {
        self.hits.clear();
        let (pad, small, row) = (t.size.pad, t.size.small, t.size.tree_row);
        let mut y = a.y;
        // the find field, the mode and the owner's button on one line
        let side = row - 4.0;
        let mut right = a.x + a.w;
        if let Some(label) = action {
            let w = d.sans_width(small, label) + 2.0 * pad;
            let r = Rect { x: right - w, y: y + 2.0, w, h: side };
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, if r.has(pointer.0, pointer.1) { t.color.hover } else { t.color.control });
            d.sans(r.x + pad, r.y + (r.h - small * 1.25) * 0.5, small, label, t.color.text);
            self.hits.push((r, Hit::Action));
            right = r.x - 4.0;
        }
        let mode = controls::icon_button(d, t, Rect { x: right - side, y: y + 2.0, w: side, h: side }, if self.mode == Mode::Grid { "list" } else { "grid" }, t.color.muted, pointer);
        self.hits.push((mode, Hit::Mode));
        let field = Rect { x: a.x, y: y + 2.0, w: (mode.x - 4.0 - a.x).max(40.0), h: side };
        d.round_rect(field.x, field.y, field.w, field.h, t.size.radius, t.color.control);
        if self.typing.is_some() {
            d.outline(field.x, field.y, field.w, field.h, (t.size.radius, 1.0), t.color.accent);
        }
        d.icon(field.x + 6.0, field.y + (field.h - 13.0) * 0.5, 13.0, "search", t.color.muted);
        let (text, ink) = match &self.typing {
            Some(f) => (format!("{}|", f.shown().0), t.color.text),
            None if self.filter.query.is_empty() => ("find".to_string(), t.color.muted),
            None => (self.filter.query.clone(), t.color.text),
        };
        d.sans(field.x + 24.0, field.y + (field.h - t.size.font * 1.25) * 0.5, t.size.font, &controls::fit_sans(d, t.size.font, &text, field.w - 30.0), ink);
        self.hits.push((field, Hit::Find));
        y += row + 4.0;
        // a chip per kind there is, then "related" when a character is open
        let kinds = facets(entries);
        let mut names: Vec<String> = kinds.iter().map(|(k, n)| format!("{} {n}", k.word())).collect();
        let mut on: Vec<bool> = kinds.iter().map(|(k, _)| self.filter.kinds.contains(k)).collect();
        if let Some(c) = &self.character {
            names.push(format!("related to {}", c.strip_prefix("character:").unwrap_or(c)));
            on.push(self.filter.related.is_some());
        }
        if names.len() > 1 || self.character.is_some() {
            let refs: Vec<&str> = names.iter().map(String::as_str).collect();
            let (places, used) = controls::chips(d, t, (a.x, y, a.w), &refs, &on);
            for (i, r) in places.into_iter().enumerate() {
                self.hits.push((r, kinds.get(i).map_or(Hit::Related, |(k, _)| Hit::Facet(*k))));
            }
            y += used + 6.0;
        }
        let shown = self.shown(entries);
        if shown.is_empty() {
            let why = if entries.is_empty() { "nothing here yet" } else { "nothing found" };
            d.sans(a.x, y + 4.0, small, why, t.color.muted);
            return;
        }
        let held = self.held.map(|h| h.entry);
        match self.mode {
            Mode::Grid => {
                let gap = 8.0;
                let cols = (((a.w + gap) / (CARD_W + gap)).floor() as usize).max(1);
                let cw = (a.w - gap * (cols - 1) as f32) / cols as f32;
                let ch = CARD_PREVIEW + small * 1.4 * 2.0 + 8.0;
                for (n, &i) in shown.iter().enumerate() {
                    let e = &entries[i];
                    let r = Rect { x: a.x + (n % cols) as f32 * (cw + gap), y: y + (n / cols) as f32 * (ch + gap), w: cw, h: ch };
                    let lit = held == Some(i);
                    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, if lit { t.color.selected } else if r.has(pointer.0, pointer.1) { t.color.hover } else { t.color.surface_head });
                    preview(d, t, Rect { x: r.x + 4.0, y: r.y + 4.0, w: r.w - 8.0, h: CARD_PREVIEW - 4.0 }, e, picture);
                    let ty = r.y + CARD_PREVIEW + 4.0;
                    d.sans(r.x + 6.0, ty, t.size.font, &controls::fit_sans(d, t.size.font, &e.label, r.w - 12.0), t.color.text);
                    d.sans(r.x + 6.0, ty + small * 1.45, small, &controls::fit_sans(d, small, &e.detail, r.w - 12.0), t.color.muted);
                    self.hits.push((r, Hit::Card(i)));
                }
            }
            Mode::List => {
                let h = ROW_PREVIEW.1 + 6.0;
                for (n, &i) in shown.iter().enumerate() {
                    let e = &entries[i];
                    let r = Rect { x: a.x, y: y + n as f32 * h, w: a.w, h };
                    if held == Some(i) {
                        d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.selected);
                    } else if r.has(pointer.0, pointer.1) {
                        d.rect(r.x, r.y, r.w, r.h, t.color.hover);
                    }
                    preview(d, t, Rect { x: r.x + 3.0, y: r.y + 3.0, w: ROW_PREVIEW.0, h: ROW_PREVIEW.1 }, e, picture);
                    let x = r.x + ROW_PREVIEW.0 + 10.0;
                    let dw = d.sans_width(small, &e.detail).min(r.w * 0.4);
                    d.sans(x, r.y + (h - t.size.font * 1.25) * 0.5, t.size.font, &controls::fit_sans(d, t.size.font, &e.label, r.x + r.w - dw - 12.0 - x), t.color.text);
                    d.sans(r.x + r.w - dw - 4.0, r.y + (h - small * 1.25) * 0.5, small, &controls::fit_sans(d, small, &e.detail, dw), t.color.muted);
                    self.hits.push((r, Hit::Card(i)));
                }
            }
        }
    }

    /// Over everything: the card being dragged at the pointer.
    pub fn over(&self, d: &mut DrawList, t: &Tokens, entries: &[Entry], max_x: f32) {
        if let Some(h) = self.held.filter(|h| h.dragging) {
            if let Some(e) = entries.get(h.entry) {
                controls::chip(d, t, (h.at.0 + 12.0, h.at.1 + 8.0), max_x, &e.label, false);
            }
        }
    }
}

/// An entry's preview in `r`: its picture when the screen has it, its wiring, its code's face, else its kind.
pub fn preview(d: &mut DrawList, t: &Tokens, r: Rect, e: &Entry, picture: &dyn Fn(&Preview) -> Option<u64>) {
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.stage);
    if let Some(key) = picture(&e.preview) {
        d.picture(r.x, r.y, r.w, r.h, key);
        return;
    }
    let small = t.size.small.min(r.h * 0.3);
    match &e.preview {
        Preview::Graph { dots, tubes } => {
            let inset = (r.h * 0.12).max(3.0);
            let at = |i: usize| dots.get(i).map(|p| (r.x + inset + p.x * (r.w - 2.0 * inset), r.y + inset + p.y * (r.h - 2.0 * inset)));
            for &(a, b) in tubes {
                if let (Some(p), Some(q)) = (at(a), at(b)) {
                    d.segment(p, q, 1.5, t.color.muted);
                }
            }
            let rad = (r.h * 0.06).clamp(1.5, 4.0);
            for (i, dot) in dots.iter().enumerate() {
                if let Some(p) = at(i) {
                    d.disc(p.0, p.1, rad, if dot.audio { t.color.accent } else { t.color.text });
                }
            }
        }
        Preview::Code { params, ins, outs } if r.h >= 40.0 => {
            d.icon(r.x + 6.0, r.y + 6.0, 14.0, e.kind.icon(), t.color.muted);
            d.sans(r.x + 24.0, r.y + 6.0, small, &format!("{} parameters", params.len()), t.color.muted);
            for (n, p) in params.iter().take(((r.h - 26.0) / (small * 1.3)).max(0.0) as usize).enumerate() {
                d.sans(r.x + 12.0, r.y + 24.0 + n as f32 * small * 1.3, small, &controls::fit_sans(d, small, p, r.w - 24.0), t.color.text);
            }
            ports(d, t, r, *ins, r.x + 3.0);
            ports(d, t, r, *outs, r.x + r.w - 3.0);
        }
        Preview::Code { ins, outs, .. } => {
            let s = (r.h * 0.6).min(16.0);
            d.icon(r.x + (r.w - s) * 0.5, r.y + (r.h - s) * 0.5, s, e.kind.icon(), t.color.muted);
            ports(d, t, r, *ins, r.x + 3.0);
            ports(d, t, r, *outs, r.x + r.w - 3.0);
        }
        _ => {
            let s = (r.h * 0.42).min(28.0);
            d.icon(r.x + (r.w - s) * 0.5, r.y + (r.h - s) * 0.5, s, e.kind.icon(), t.color.muted);
        }
    }
}

/// `n` ports down an edge at `x`
fn ports(d: &mut DrawList, t: &Tokens, r: Rect, n: usize, x: f32) {
    let n = n.min(8);
    for i in 0..n {
        d.disc(x, r.y + (i as f32 + 0.5) / n as f32 * r.h, 2.0, t.color.accent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::draw::Prim;

    fn entry(address: &str, kind: Kind, preview: Preview, receivers: &[&str]) -> Entry {
        Entry { address: address.into(), label: address.split(':').nth(1).unwrap_or(address).into(), detail: String::new(), kind, preview, receivers: receivers.iter().map(|s| s.to_string()).collect() }
    }

    fn list() -> Vec<Entry> {
        vec![
            entry("module:walk", Kind::Module, Preview::Code { params: vec!["rate".into()], ins: 0, outs: 1 }, &[]),
            entry("module:torso", Kind::Module, Preview::Still { key: "module:torso".into() }, &["character:catpope"]),
            entry("instrument:keys", Kind::Instrument, deck_catalog::library::graph(&[("osc".into(), true), ("out".into(), true)], &[("osc".into(), "out".into())]), &[]),
            entry("scene:nave", Kind::Scene, Preview::Kind, &[]),
        ]
    }

    fn drawn(b: &mut Browser, entries: &[Entry], has: &dyn Fn(&Preview) -> Option<u64>) -> DrawList {
        let mut d = DrawList::new(8.0);
        b.draw(&mut d, &Tokens::default(), (Rect { x: 0.0, y: 0.0, w: 420.0, h: 2000.0 }, (-1.0, -1.0)), entries, has, Some("new scene"));
        d
    }

    fn at(b: &Browser, want: Hit) -> (f32, f32) {
        let (r, _) = b.hits.iter().find(|(_, h)| *h == want).unwrap_or_else(|| panic!("{want:?} in {:?}", b.hits));
        (r.x + r.w * 0.5, r.y + r.h * 0.5)
    }

    #[test]
    fn chips_filter_by_kind_and_related_and_the_field_by_words() {
        let e = list();
        let mut b = Browser::default();
        drawn(&mut b, &e, &|_| None);
        assert!(!b.hits.iter().any(|(_, h)| *h == Hit::Related), "no character open: no related chip");
        assert_eq!(b.shown(&e), [0, 1, 2, 3]);
        let (x, y) = at(&b, Hit::Facet(Kind::Module));
        assert_eq!(b.press(x, y, 0.0), Some(Did::Taken));
        assert_eq!(b.shown(&e), [0, 1]);
        b.character = Some("character:catpope".into());
        drawn(&mut b, &e, &|_| None);
        let (x, y) = at(&b, Hit::Related);
        b.press(x, y, 1.0);
        assert_eq!(b.shown(&e), [1]);
        b.press(x, y, 2.0);
        let (x, y) = at(&b, Hit::Find);
        b.press(x, y, 3.0);
        b.typing.as_mut().unwrap().insert("wa");
        assert_eq!(b.shown(&e), [0], "found as typed");
        let (x, y) = at(&b, Hit::Mode);
        b.press(x, y, 4.0);
        assert_eq!((b.mode, b.filter.query.as_str(), b.typing.is_none()), (Mode::List, "wa", true), "a press elsewhere keeps the query");
        drawn(&mut b, &e, &|_| None);
        assert_eq!(b.hits.iter().filter(|(_, h)| matches!(h, Hit::Card(_))).count(), 1);
    }

    #[test]
    fn a_card_is_dragged_and_dropped_or_opened_by_a_second_press() {
        let e = list();
        let mut b = Browser::default();
        drawn(&mut b, &e, &|_| None);
        let (x, y) = at(&b, Hit::Card(3));
        b.press(x, y, 10.0);
        b.drag_to(x + 2.0, y);
        assert_eq!(b.dragging(), None, "a jitter is no drag");
        b.drag_to(x + 300.0, y + 200.0);
        assert_eq!(b.dragging(), Some(3));
        assert_eq!(b.release(x + 300.0, y + 200.0), Some(Did::Drop { entry: 3, at: (x + 300.0, y + 200.0) }));
        assert_eq!(b.release(0.0, 0.0), None, "nothing held");
        b.press(x, y, 20.0);
        assert_eq!(b.release(x, y), Some(Did::Click(3)));
        assert_eq!(b.press(x, y, 20.3), Some(Did::Open(3)));
        b.press(x, y, 30.0);
        b.release(x, y);
        assert_eq!(b.press(x, y, 31.0), Some(Did::Taken), "too late for an open");
        let (ax, ay) = at(&b, Hit::Action);
        assert_eq!(b.press(ax, ay, 40.0), Some(Did::Action));
        assert_eq!(b.press(-50.0, -50.0, 41.0), None, "not the browser's");
    }

    #[test]
    fn each_preview_draws_by_its_content() {
        let e = list();
        let mut b = Browser::default();
        let d = drawn(&mut b, &e, &|_| None);
        let segments = d.prims().iter().filter(|p| matches!(p, Prim::Segment { .. })).count();
        assert!(segments >= 1, "the instrument's tube");
        assert!(d.texts().iter().any(|s| *s == "1 parameters" || *s == "rate"), "the code's face");
        assert!(!d.prims().iter().any(|p| matches!(p, Prim::Picture { .. })), "no still yet: its kind");
        let d = drawn(&mut b, &e, &|p| matches!(p, Preview::Still { .. }).then_some(7));
        assert!(d.prims().iter().any(|p| matches!(p, Prim::Picture { key: 7, .. })), "the still once the screen has it");
    }
}
