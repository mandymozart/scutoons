//! The library browser (owner 2026-11-16 [design]): one component for whatever list it is handed – the server's Library
//! (deck-catalog `library::served`), a workspace's own entries. A find field, the places as chips, a chip per kind there
//! is (and "related" when a character is open), cards in a grid or rows in a list, each with the preview the core chose
//! for its content. A thing stepped into shows its levels, what it uses as cards and what uses it. A card pressed and
//! moved is dragged (its owner puts it in the one hand, `carry`); let go, its owner is told where; a second press on it
//! soon after opens it. A file whose family has more than one version (deck-catalog `library::shelf`) is drawn as a stack.
use crate::base::draw::{DrawList, Rect};
use crate::base::text::Field;
use crate::base::tokens::Tokens;
use crate::editor::controls;
use deck_catalog::library::served::Inside;
use deck_catalog::library::shelf::{select, Shelf};
use deck_catalog::library::{facets, find, Entry, Filter, Kind, Preview};

/// a card's width and its preview's height in the grid; a row's preview in the list
const CARD_W: f32 = 132.0;
const CARD_PREVIEW: f32 = 84.0;
const ROW_PREVIEW: (f32, f32) = (44.0, 28.0);
/// a press moved this far is a drag
const DRAG: f32 = 5.0;
/// a second press within this many seconds opens
const OPEN: f64 = 0.45;
/// a level hangs this far in from its parent
const INDENT: f32 = 12.0;

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
    /// the selection's cross: none chosen
    Unselect,
    /// one of the owner's places, by its index
    Place(usize),
    /// out of the thing stepped into
    Back,
    /// the thing stepped into, opened in its workspace
    OpenAt,
    /// a level inside it, by its index
    Level(usize),
    /// a thing that uses it, by its index
    UsedIn(usize),
    /// a card, by its entry's index
    Card(usize),
}

/// what the browser lists: its owner's entries, the toon's families for the stacks, the places, a thing stepped into
#[derive(Clone, Copy)]
pub struct Source<'a> {
    pub entries: &'a [Entry],
    pub shelf: Option<&'a Shelf>,
    /// each place's words and whether it is the one shown
    pub places: &'a [(String, bool)],
    pub inside: Option<&'a Inside>,
    /// one muted line over the cards (what an offline Library shows)
    pub note: Option<&'a str>,
}

impl<'a> Source<'a> {
    pub fn plain(entries: &'a [Entry]) -> Source<'a> {
        Source { entries, shelf: None, places: &[], inside: None, note: None }
    }

    /// how many versions the card of `e` stacks (1: a single file or no family)
    pub fn stack(&self, e: &Entry) -> usize {
        self.shelf.filter(|_| e.kind.is_file()).and_then(|s| s.family_in(e.path(), false)).map_or(1, |f| f.versions.len())
    }
}

/// the sheets drawn behind a stacked card (two at most) and how far each stands out
const SHEETS: usize = 2;
const SHEET_STEP: f32 = 3.0;

/// the sheets of a stack of `n` versions behind `r`, up and to the right
fn sheets(d: &mut DrawList, t: &Tokens, r: Rect, n: usize) {
    for k in (1..=SHEETS.min(n.saturating_sub(1))).rev() {
        let o = SHEET_STEP * k as f32;
        d.round_rect(r.x + o, r.y - o, r.w, r.h, t.size.radius, t.color.surface_head);
        d.outline(r.x + o, r.y - o, r.w, r.h, (t.size.radius, 1.0), t.color.line);
    }
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
    Place(usize),
    Back,
    OpenAt,
    Level(usize),
    UsedIn(usize),
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
    /// the cards the last drop let go of, in the order shown (several when a chosen card was dragged)
    dropped: Vec<usize>,
    /// the cards chosen (by address) and the one a Shift run starts from
    pub chosen: Vec<String>,
    anchor: Option<String>,
    /// Shift and Ctrl / ⌘ as the owner saw them at the press
    pub keys: (bool, bool),
    /// the cards drawn, in their order: entry and address (a Shift run's order)
    order: Vec<(usize, String)>,
}

impl Default for Browser {
    fn default() -> Browser {
        Browser { filter: Filter::default(), mode: Mode::Grid, typing: None, character: None, held: None, last: None, hits: Vec::new(), dropped: Vec::new(), chosen: Vec::new(), anchor: None, keys: (false, false), order: Vec::new() }
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
            Hit::Unselect => {
                (self.chosen, self.anchor) = (Vec::new(), None);
                Did::Taken
            }
            Hit::Place(n) => {
                (self.chosen, self.anchor) = (Vec::new(), None);
                Did::Place(n)
            }
            Hit::Back => Did::Back,
            Hit::OpenAt => Did::OpenAt,
            Hit::Level(n) => Did::Level(n),
            Hit::UsedIn(n) => Did::UsedIn(n),
            Hit::Card(i) => {
                let (shift, toggle) = self.keys;
                // a plain press on one of several chosen cards keeps them: it may drag them all (a click narrows on release)
                let keep = |b: &Browser, id: &str| !shift && !toggle && b.chosen.len() > 1 && b.chosen.iter().any(|c| c == id);
                if let Some(id) = self.order.iter().find(|(j, _)| *j == i).map(|(_, a)| a.clone()).filter(|id| !keep(self, id)) {
                    let order: Vec<String> = self.order.iter().map(|(_, a)| a.clone()).collect();
                    (self.chosen, self.anchor) = select(&self.chosen, self.anchor.as_deref(), &id, &order, shift, toggle);
                }
                if shift || toggle {
                    self.last = None;
                    return Some(Did::Taken);
                }
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

    /// The cards in the hand: every chosen card in the order shown when the one held is among several chosen, else the
    /// one held.
    pub fn carried(&self) -> Vec<usize> {
        let Some(h) = self.held else { return Vec::new() };
        let address = self.order.iter().find(|(i, _)| *i == h.entry).map(|(_, a)| a.as_str());
        if self.chosen.len() < 2 || !address.is_some_and(|a| self.chosen.iter().any(|c| c == a)) {
            return vec![h.entry];
        }
        let mut out: Vec<usize> = Vec::new();
        for (i, a) in &self.order {
            if self.chosen.contains(a) && !out.contains(i) {
                out.push(*i);
            }
        }
        out
    }

    /// the cards the last drop let go of (`Did::Drop` names the one under the pointer)
    pub fn dropped(&self) -> &[usize] {
        &self.dropped
    }

    /// The button let go: dragged cards are dropped where the pointer is (its owner decides what takes them); a press on
    /// one of several chosen cards that did not move chooses that one.
    pub fn release(&mut self, x: f32, y: f32) -> Option<Did> {
        self.dropped = self.carried();
        let h = self.held.take()?;
        if h.dragging {
            return Some(Did::Drop { entry: h.entry, at: (x, y) });
        }
        self.dropped.clear();
        if let Some(id) = self.order.iter().find(|(i, _)| *i == h.entry).map(|(_, a)| a.clone()).filter(|_| self.chosen.len() > 1) {
            (self.chosen, self.anchor) = (vec![id.clone()], Some(id));
        }
        Some(Did::Click(h.entry))
    }

    /// The browser in `a`: the find field, the places and kinds as chips (or the thing stepped into: back, its name, Open,
    /// its levels), the cards, what uses the thing. `picture` answers which of the screen's pictures shows a preview (a
    /// still or a picture already loaded); `action` is the owner's one button.
    pub fn draw(&mut self, d: &mut DrawList, t: &Tokens, (a, pointer): (Rect, (f32, f32)), src: &Source, picture: &dyn Fn(&Preview) -> Option<u64>, action: Option<&str>) {
        let entries = src.entries;
        self.hits.clear();
        let (small, row) = (t.size.small, t.size.tree_row);
        let mut y = a.y;
        // the find field, the mode and the owner's button on one line
        let side = row - 4.0;
        let mut right = a.x + a.w;
        if let Some(label) = action {
            let r = self.button(d, t, (right, y + 2.0, side), label, pointer, Hit::Action);
            right = r.x - 4.0;
        }
        let mode = controls::icon_button(d, t, Rect { x: right - side, y: y + 2.0, w: side, h: side }, if self.mode == Mode::Grid { "list" } else { "grid" }, t.color.muted, pointer);
        self.hits.push((mode, Hit::Mode));
        let field = Rect { x: a.x, y: y + 2.0, w: (mode.x - 4.0 - a.x).max(40.0), h: side };
        d.round_rect(field.x, field.y, field.w, field.h, t.size.radius, t.color.control);
        if self.typing.is_some() {
            d.outline(field.x, field.y, field.w, field.h, (t.size.radius, 1.0), t.kit("field", "on").border.unwrap_or(t.color.text));
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
        if let Some(inside) = src.inside {
            y = self.inside(d, t, (a, pointer), y, inside);
        }
        let f = self.filter_now();
        let shown = find(entries, &f);
        // the places (not inside a thing), a chip per kind there is, "related" when a character is open
        let mut chips: Vec<(String, bool, Hit)> = Vec::new();
        if src.inside.is_none() {
            chips.extend(src.places.iter().enumerate().map(|(n, (w, on))| (w.clone(), *on, Hit::Place(n))));
        }
        let kinds = facets(entries);
        if kinds.len() > 1 {
            chips.extend(kinds.iter().map(|(k, n)| (format!("{} {n}", k.word()), self.filter.kinds.contains(k), Hit::Facet(*k))));
        }
        if let Some(c) = &self.character {
            chips.push((format!("related to {}", c.strip_prefix("character:").unwrap_or(c)), self.filter.related.is_some(), Hit::Related));
        }
        if !chips.is_empty() {
            let names: Vec<&str> = chips.iter().map(|(n, _, _)| n.as_str()).collect();
            let on: Vec<bool> = chips.iter().map(|(_, o, _)| *o).collect();
            let (places, used) = controls::chips(d, t, (a.x, y, a.w), &names, &on);
            for (r, (_, _, hit)) in places.into_iter().zip(&chips) {
                self.hits.push((r, *hit));
            }
            y += used + 6.0;
        }
        if let Some(text) = src.note {
            let h = small * 1.4 + 4.0;
            d.sans(a.x, y + (h - small * 1.25) * 0.5, small, &controls::fit_sans(d, small, text, a.w), t.color.muted);
            y += h + 2.0;
        }
        if self.chosen.len() > 1 {
            let h = small * 1.4 + 6.0;
            let text = format!("{} selected", self.chosen.len());
            d.sans(a.x, y + (h - small * 1.25) * 0.5, small, &text, t.color.text);
            let x = controls::icon_button(d, t, Rect { x: a.x + d.sans_width(small, &text) + 6.0, y, w: h, h }, "close", t.color.muted, pointer);
            self.hits.push((x, Hit::Unselect));
            y += h + 4.0;
        }
        self.order.clear();
        if shown.is_empty() && src.inside.is_none() {
            let why = if entries.is_empty() { "nothing here yet" } else { "nothing found" };
            d.sans(a.x, y + 4.0, small, why, t.color.muted);
            return;
        }
        y = self.cards(d, t, (a, pointer), y, src, &shown, picture);
        if let Some(inside) = src.inside {
            self.used_in(d, t, (a, pointer), y, inside);
        }
    }

    /// a small button ending at `right` on the line at `y`, `side` high; its place
    fn button(&mut self, d: &mut DrawList, t: &Tokens, (right, y, side): (f32, f32, f32), label: &str, pointer: (f32, f32), hit: Hit) -> Rect {
        let (pad, small) = (t.size.pad, t.size.small);
        let w = d.sans_width(small, label) + 2.0 * pad;
        let r = Rect { x: right - w, y, w, h: side };
        d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, if r.has(pointer.0, pointer.1) { t.color.hover } else { t.color.control });
        d.sans(r.x + pad, r.y + (r.h - small * 1.25) * 0.5, small, label, t.color.text);
        self.hits.push((r, hit));
        r
    }

    /// One row of a list in `a` at `y`: its words, a muted word at the right, hung in by `depth`; where it ends.
    #[allow(clippy::too_many_arguments)]
    fn row(&mut self, d: &mut DrawList, t: &Tokens, (a, pointer): (Rect, (f32, f32)), y: f32, depth: usize, (words, side): (&str, &str), hit: Hit) -> f32 {
        let small = t.size.small;
        let h = small * 1.4 + 6.0;
        let r = Rect { x: a.x, y, w: a.w, h };
        if r.has(pointer.0, pointer.1) {
            controls::list_row(d, t, r, "hover");
        }
        let sw = d.sans_width(small, side);
        let x = r.x + 4.0 + INDENT * depth as f32;
        d.sans(x, r.y + (h - t.size.font * 1.25) * 0.5, t.size.font, &controls::fit_sans(d, t.size.font, words, (r.x + r.w - sw - 12.0 - x).max(0.0)), t.color.text);
        d.sans(r.x + r.w - sw - 4.0, r.y + (h - small * 1.25) * 0.5, small, side, t.color.muted);
        self.hits.push((r, hit));
        y + h
    }

    /// The thing stepped into: back, its name and Open on one line, then its levels hung under their parents.
    fn inside(&mut self, d: &mut DrawList, t: &Tokens, (a, pointer): (Rect, (f32, f32)), mut y: f32, inside: &Inside) -> f32 {
        let side = t.size.tree_row - 4.0;
        let back = controls::icon_button(d, t, Rect { x: a.x, y: y + 2.0, w: side, h: side }, "previous", t.color.muted, pointer);
        self.hits.push((back, Hit::Back));
        let open = self.button(d, t, (a.x + a.w, y + 2.0, side), "open", pointer, Hit::OpenAt);
        let x = back.x + back.w + 6.0;
        let name = format!("{} · {}", inside.at.label, inside.at.kind.word());
        d.sans(x, y + 2.0 + (side - t.size.font * 1.25) * 0.5, t.size.font, &controls::fit_sans(d, t.size.font, &name, (open.x - 6.0 - x).max(0.0)), t.color.text);
        y += t.size.tree_row + 2.0;
        // a look names how many moods it has: a mood is opened from its look, the list stays short
        for (n, l) in inside.levels.iter().enumerate().filter(|(_, l)| l.mood.is_none()) {
            let moods = inside.levels.iter().filter(|m| m.mood.is_some() && m.parent == l.address).count();
            let side = match moods {
                0 => l.level.clone(),
                1 => format!("{} · 1 mood", l.level),
                n => format!("{} · {n} moods", l.level),
            };
            y = self.row(d, t, (a, pointer), y, l.depth, (&l.name, &side), Hit::Level(n));
        }
        y + 6.0
    }

    /// what uses the thing stepped into: a heading, then a row each, how it uses it at the right
    fn used_in(&mut self, d: &mut DrawList, t: &Tokens, (a, pointer): (Rect, (f32, f32)), mut y: f32, inside: &Inside) {
        if inside.used_in.is_empty() {
            return;
        }
        let small = t.size.small;
        y += 6.0;
        d.sans(a.x, y, small, &format!("used in {}", inside.used_in.len()), t.color.muted);
        y += small * 1.4 + 4.0;
        for (n, l) in inside.used_in.iter().enumerate() {
            y = self.row(d, t, (a, pointer), y, 0, (&l.name, &l.role), Hit::UsedIn(n));
        }
    }

    /// The cards of `list` (indices into the entries) from `y` on, in the grid or the list; where they end.
    #[allow(clippy::too_many_arguments)]
    fn cards(&mut self, d: &mut DrawList, t: &Tokens, (a, pointer): (Rect, (f32, f32)), mut y: f32, src: &Source, list: &[usize], picture: &dyn Fn(&Preview) -> Option<u64>) -> f32 {
        if list.is_empty() {
            return y;
        }
        let small = t.size.small;
        let (gap, row_h) = (8.0, ROW_PREVIEW.1 + 6.0);
        let grid = self.mode == Mode::Grid;
        let cols = if grid { (((a.w + gap) / (CARD_W + gap)).floor() as usize).max(1) } else { 1 };
        if grid && src.shelf.is_some() {
            y += SHEET_STEP * SHEETS as f32;
        }
        let cw = (a.w - gap * (cols - 1) as f32) / cols as f32;
        let ch = if grid { CARD_PREVIEW + small * 1.4 * 2.0 + 8.0 } else { row_h };
        let step = if grid { ch + gap } else { row_h };
        for (n, &i) in list.iter().enumerate() {
            let r = Rect { x: a.x + (n % cols) as f32 * (cw + gap), y: y + (n / cols) as f32 * step, w: cw, h: ch };
            // a card outside the area is not drawn (a toon lists thousands); it keeps its place in the order
            if r.y > a.y + a.h || r.y + r.h < a.y {
                self.order.push((i, src.entries[i].address.clone()));
                continue;
            }
            self.card(d, t, (r, pointer), src, i, picture);
        }
        y + list.len().div_ceil(cols) as f32 * step + if grid { 0.0 } else { 4.0 }
    }

    /// the card of entry `i` in `r`: pressed, dragged, chosen
    fn card(&mut self, d: &mut DrawList, t: &Tokens, (r, pointer): (Rect, (f32, f32)), src: &Source, i: usize, picture: &dyn Fn(&Preview) -> Option<u64>) {
        let small = t.size.small;
        let e = &src.entries[i];
        let lit = self.held.map(|h| h.entry) == Some(i) || self.chosen.contains(&e.address);
        let hover = r.has(pointer.0, pointer.1);
        let stack = src.stack(e);
        match self.mode {
            Mode::Grid => {
                sheets(d, t, r, stack);
                d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, if lit { t.color.selected } else if hover { t.color.hover } else { t.color.surface_head });
                preview(d, t, Rect { x: r.x + 4.0, y: r.y + 4.0, w: r.w - 8.0, h: CARD_PREVIEW - 4.0 }, e, picture);
                let ty = r.y + CARD_PREVIEW + 4.0;
                d.sans(r.x + 6.0, ty, t.size.font, &controls::fit_sans(d, t.size.font, &e.label, r.w - 12.0), t.color.text);
                d.sans(r.x + 6.0, ty + small * 1.45, small, &controls::fit_sans(d, small, &e.detail, r.w - 12.0), t.color.muted);
            }
            Mode::List => {
                if lit {
                    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.selected);
                } else if hover {
                    controls::list_row(d, t, r, "hover");
                }
                let p = Rect { x: r.x + 3.0, y: r.y + 3.0, w: ROW_PREVIEW.0, h: ROW_PREVIEW.1 };
                sheets(d, t, Rect { w: p.w - SHEET_STEP * SHEETS as f32, ..p }, stack);
                preview(d, t, p, e, picture);
                let x = r.x + ROW_PREVIEW.0 + 10.0;
                let dw = d.sans_width(small, &e.detail).min(r.w * 0.4);
                d.sans(x, r.y + (r.h - t.size.font * 1.25) * 0.5, t.size.font, &controls::fit_sans(d, t.size.font, &e.label, r.x + r.w - dw - 12.0 - x), t.color.text);
                d.sans(r.x + r.w - dw - 4.0, r.y + (r.h - small * 1.25) * 0.5, small, &controls::fit_sans(d, small, &e.detail, dw), t.color.muted);
            }
        }
        self.order.push((i, e.address.clone()));
        self.hits.push((r, Hit::Card(i)));
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
                    d.disc(p.0, p.1, rad, if dot.audio { t.color.signals.audio } else { t.color.text });
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
            // a still not taken yet: that it draws, else its kind
            let icon = if matches!(e.preview, Preview::Still { .. }) && e.kind == Kind::Module { "model" } else { e.kind.icon() };
            d.icon(r.x + (r.w - s) * 0.5, r.y + (r.h - s) * 0.5, s, icon, t.color.muted);
        }
    }
}

/// `n` ports down an edge at `x`
fn ports(d: &mut DrawList, t: &Tokens, r: Rect, n: usize, x: f32) {
    let n = n.min(8);
    for i in 0..n {
        d.disc(x, r.y + (i as f32 + 0.5) / n as f32 * r.h, 2.0, t.color.muted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::draw::Prim;

    fn entry(address: &str, kind: Kind, preview: Preview, made_for: &[&str]) -> Entry {
        Entry { address: address.into(), label: address.split(':').nth(1).unwrap_or(address).into(), detail: String::new(), kind, preview, made_for: made_for.iter().map(|s| s.to_string()).collect() }
    }

    fn list() -> Vec<Entry> {
        vec![
            entry("module:walk", Kind::Module, Preview::Code { params: vec!["rate".into()], ins: 0, outs: 1 }, &[]),
            entry("module:torso", Kind::Module, Preview::Still { key: "module:torso".into() }, &["catpope"]),
            entry("instrument:keys", Kind::Instrument, deck_catalog::library::graph(&[("osc".into(), true), ("out".into(), true)], &[("osc".into(), "out".into())]), &[]),
            entry("scene:nave", Kind::Scene, Preview::Kind, &[]),
        ]
    }

    fn drawn(b: &mut Browser, entries: &[Entry], has: &dyn Fn(&Preview) -> Option<u64>) -> DrawList {
        let mut d = DrawList::new(8.0);
        b.draw(&mut d, &Tokens::default(), (Rect { x: 0.0, y: 0.0, w: 420.0, h: 2000.0 }, (-1.0, -1.0)), &Source::plain(entries), has, Some("new scene"));
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

    /// the toon's files as the shelf's families: two versions of one picture are one card, drawn as a stack
    fn shelf() -> Shelf {
        let file = |path: &str| format!(r#"{{ "path": "{path}", "size": 10 }}"#);
        let asset = |id: &str, kind: &str, files: &[&str]| format!(r#"{{ "id": "{id}", "kind": "{kind}", "usages": 1, "renditions": [{}] }}"#, files.iter().map(|f| file(f)).collect::<Vec<_>>().join(","));
        let items = [asset("a1", "image", &["cat/s1/c1/hills.webp", "cat/s1/c1/hills.2x.webp", "cat/s1/c1/hills.3x.webp"]), asset("a2", "image", &["cat/s1/c2/sky.webp"])];
        Shelf::from_json(&format!(r#"{{ "items": [{}] }}"#, items.join(","))).expect("the answer")
    }

    fn draw_src(b: &mut Browser, src: &Source) -> DrawList {
        // narrow letters: the words fit and are read whole
        let mut d = DrawList::new(0.5);
        b.draw(&mut d, &Tokens::default(), (Rect { x: 0.0, y: 0.0, w: 420.0, h: 4000.0 }, (-1.0, -1.0)), src, &|_| None, None);
        d
    }

    #[test]
    fn a_file_with_versions_is_drawn_as_a_stack() {
        let shelf = shelf();
        let entries = vec![entry("image:cat/s1/c1/hills.webp", Kind::Image, Preview::Kind, &[]), entry("image:cat/s1/c2/sky.webp", Kind::Image, Preview::Kind, &[])];
        let src = Source { shelf: Some(&shelf), ..Source::plain(&entries) };
        assert_eq!((src.stack(&entries[0]), src.stack(&entries[1])), (3, 1), "the server's file address, its family by its path");
        let mut b = Browser::default();
        let d = draw_src(&mut b, &src);
        let (r, _) = *b.hits.iter().find(|(_, h)| *h == Hit::Card(0)).expect("the hills' card");
        let behind = |k: f32| d.prims().iter().any(|p| matches!(p, Prim::Outline { x, y, .. } if *x == r.x + SHEET_STEP * k && *y == r.y - SHEET_STEP * k));
        assert!(behind(1.0) && behind(2.0) && !behind(3.0), "two sheets behind the three versions' card");
    }

    #[test]
    fn the_places_are_chips_and_a_note_says_what_is_shown() {
        let e = list();
        let places = [("this toon".to_string(), true), ("everything".to_string(), false)];
        let src = Source { places: &places, note: Some("offline: the Library as last read"), ..Source::plain(&e) };
        let mut b = Browser { chosen: vec!["module:walk".into()], ..Browser::default() };
        let d = draw_src(&mut b, &src);
        assert!(d.texts().contains(&"offline: the Library as last read"));
        let (x, y) = at(&b, Hit::Place(1));
        assert_eq!(b.press(x, y, 0.0), Some(Did::Place(1)), "the owner asks the place");
        assert!(b.chosen.is_empty(), "another place: nothing chosen");
    }

    #[test]
    fn a_thing_stepped_into_shows_its_levels_what_it_uses_and_what_uses_it() {
        use deck_catalog::library::served::{Inside, Level, Link};
        let level = |depth: usize, address: &str, level: &str, name: &str, parent: &str| Level { depth, address: address.into(), level: level.into(), name: name.into(), parent: parent.into(), mood: None };
        let step = Inside {
            at: entry("character:catpope", Kind::Character, Preview::Kind, &[]),
            levels: vec![level(0, "character:catpope.human", "persona", "human", "character:catpope"), level(1, "character:catpope.human.helmet", "look", "helmet", "character:catpope.human")],
            uses: vec![entry("image:c/a.webp", Kind::Image, Preview::Kind, &[])],
            used_in: vec![Link { address: "season:catpope:1".into(), thing: false, name: "Season 1".into(), role: "cast".into() }],
        };
        let places = [("this toon".to_string(), true)];
        let src = Source { places: &places, inside: Some(&step), ..Source::plain(&step.uses) };
        let mut b = Browser::default();
        let d = draw_src(&mut b, &src);
        assert!(d.texts().iter().any(|s| s.starts_with("catpope · characters")) && d.texts().contains(&"helmet") && d.texts().contains(&"cast"));
        assert!(!b.hits.iter().any(|(_, h)| matches!(h, Hit::Place(_))), "inside a thing: no places");
        assert_eq!(cards(&b), [0], "the files it uses are cards");
        for (hit, did) in [(Hit::Back, Did::Back), (Hit::OpenAt, Did::OpenAt), (Hit::Level(1), Did::Level(1)), (Hit::UsedIn(0), Did::UsedIn(0))] {
            let (x, y) = at(&b, hit);
            assert_eq!(b.press(x, y, 0.0), Some(did));
        }
    }

    fn cards(b: &Browser) -> Vec<usize> {
        b.hits.iter().filter_map(|(_, h)| if let Hit::Card(i) = h { Some(*i) } else { None }).collect()
    }

    /// several chosen cards go into the hand together when one of them is dragged; a click on one narrows to it
    #[test]
    fn a_drag_of_a_chosen_card_carries_every_chosen_card() {
        let e = list();
        let mut b = Browser::default();
        drawn(&mut b, &e, &|_| None);
        let pick = |b: &mut Browser, i: usize, keys: (bool, bool)| {
            let (x, y) = at(b, Hit::Card(i));
            b.keys = keys;
            b.press(x, y, 0.0);
            b.release(x, y);
        };
        pick(&mut b, 0, (false, false));
        pick(&mut b, 2, (false, true));
        pick(&mut b, 3, (false, true));
        assert_eq!(b.chosen.len(), 3);
        b.keys = (false, false);
        let (x, y) = at(&b, Hit::Card(3));
        b.press(x, y, 100.0);
        assert_eq!(b.chosen.len(), 3, "a plain press on a chosen card keeps the others");
        b.drag_to(x + 200.0, y + 100.0);
        assert_eq!(b.carried(), [0, 2, 3], "every chosen card, in the order shown");
        assert_eq!(b.release(x + 200.0, y + 100.0), Some(Did::Drop { entry: 3, at: (x + 200.0, y + 100.0) }));
        assert_eq!(b.dropped(), [0, 2, 3]);
        let (x1, y1) = at(&b, Hit::Card(1));
        b.press(x1, y1, 200.0);
        b.drag_to(x1 + 200.0, y1);
        assert_eq!(b.carried(), [1], "a card not chosen: only itself, the choice starts again");
        b.release(x1 + 200.0, y1);
        assert_eq!(b.dropped(), [1]);
        pick(&mut b, 0, (false, true));
        assert_eq!(b.chosen, ["module:torso", "module:walk"]);
        let (x, y) = at(&b, Hit::Card(0));
        b.keys = (false, false);
        b.press(x, y, 300.0);
        assert_eq!(b.release(x, y), Some(Did::Click(0)));
        assert_eq!((b.chosen.as_slice(), b.dropped()), (["module:walk".to_string()].as_slice(), [].as_slice()), "a click without a move narrows to that card");
    }

    #[test]
    fn ctrl_adds_a_card_to_the_selection_and_shift_the_run_in_the_order_shown() {
        let e = list();
        let mut b = Browser::default();
        drawn(&mut b, &e, &|_| None);
        let press = |b: &mut Browser, i: usize, keys: (bool, bool), now: f64| {
            let (x, y) = at(b, Hit::Card(i));
            b.keys = keys;
            let did = b.press(x, y, now);
            b.release(x, y);
            did
        };
        press(&mut b, 0, (false, false), 0.0);
        assert_eq!(b.chosen, ["module:walk"], "a plain click: that one");
        assert_eq!(press(&mut b, 2, (false, true), 10.0), Some(Did::Taken), "Ctrl: added, nothing held or opened");
        assert_eq!(b.chosen, ["module:walk", "instrument:keys"]);
        press(&mut b, 2, (false, true), 20.0);
        assert_eq!(b.chosen, ["module:walk"], "Ctrl again: taken out");
        press(&mut b, 3, (true, false), 30.0);
        assert_eq!(b.chosen, ["module:walk", "instrument:keys", "scene:nave"], "Shift: the run from the anchor (the last Ctrl press) in the order shown");
        let d = drawn(&mut b, &e, &|_| None);
        assert!(d.texts().contains(&"3 selected"));
        let (x, y) = at(&b, Hit::Unselect);
        b.keys = (false, false);
        b.press(x, y, 40.0);
        assert!(b.chosen.is_empty(), "the cross clears it");
    }
}
