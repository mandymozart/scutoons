//! The library browser (owner 2026-11-16 [design]): one component for whatever source it is handed – modules,
//! instruments, the catalog, scenes, assets (deck-catalog `library`). A find field, a chip per kind there is (and
//! "related" when a character is open), cards in a grid or rows in a list, each with the preview the core chose for its
//! content. A card pressed and moved is dragged (its owner puts it in the one hand, `carry`); let go, its owner is told
//! where; a second press on it soon after opens it. The toon's files come as the shelf's families (deck-catalog
//! `library::shelf`): a family with more than one version is drawn as a stack.
use crate::base::draw::{DrawList, Rect};
use crate::base::text::Field;
use crate::base::tokens::Tokens;
use crate::editor::controls;
use deck_catalog::library::shelf::{select, Group, GroupKind, Grouping, Query, Scope, Shelf, Sort, Special, View};
use std::collections::HashMap;
use deck_catalog::library::{facets, find, Entry, Filter, Kind, Preview};

/// a card's width and its preview's height in the grid; a row's preview in the list
const CARD_W: f32 = 132.0;
const CARD_PREVIEW: f32 = 84.0;
const ROW_PREVIEW: (f32, f32) = (44.0, 28.0);
/// a press moved this far is a drag
const DRAG: f32 = 5.0;
/// a second press within this many seconds opens
const OPEN: f64 = 0.45;
/// "recently added" is asked again once a minute, not every frame
const MINUTE: f64 = 60_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Grid,
    List,
}

/// a place among the toon's files beyond the scope (the shelf's specials)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// added within the last week
    Recent,
    /// no credits line
    Credits,
    /// lacking a version its kind and place should have (on demand only: no count, no warning)
    Missing,
}

impl Place {
    fn special(self) -> Special {
        match self {
            Place::Recent => Special::Recent,
            Place::Credits => Special::Credits,
            Place::Missing => Special::Missing,
        }
    }
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
    /// the toon's files: what is shown of them
    Scope(Scope),
    /// a group's heading, by its place among the groups drawn
    Fold(usize),
    /// a place, or back to all of the scope
    Place(Place),
    /// the selection's cross: none chosen
    Unselect,
    /// the places per item: open or fold their list
    Items,
    /// one item's files, by its place in the list drawn (again: all of the scope)
    Item(usize),
    /// the next grouping: by use, by kind, none
    Grouping,
    /// the next order: name, newest, biggest, most used
    Sort,
    /// the levels stacked (one card per family) or single (every file its own)
    Levels,
    /// a card, by its entry's index
    Card(usize),
}

/// what the browser lists: its owner's entries and, when the workspace lists the toon's files, their shelf (an entry
/// of a file kind is a family's card, its address the family's main file)
#[derive(Clone, Copy)]
pub struct Source<'a> {
    pub entries: &'a [Entry],
    pub shelf: Option<&'a Shelf>,
    /// changes whenever the shelf or its context does (the view is asked again)
    pub stamp: u64,
    /// now, in ms since 1970
    pub now_ms: f64,
}

impl<'a> Source<'a> {
    pub fn plain(entries: &'a [Entry]) -> Source<'a> {
        Source { entries, shelf: None, stamp: 0, now_ms: 0.0 }
    }

    /// how many versions the card of `e` stacks (1: a single file, single levels or no family)
    pub fn stack(&self, e: &Entry, single: bool) -> usize {
        self.shelf.filter(|_| e.kind.is_file()).and_then(|s| s.family_in(&e.address, single)).map_or(1, |f| f.versions.len())
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
    /// the toon's files: used by the open item, all (the open item's pinned on top, the rest folded until a search), unused
    pub scope: Scope,
    pub grouping: Grouping,
    pub sort: Sort,
    /// every file on its own instead of the families' stacks (its owner lists the single files then)
    pub single: bool,
    pub place: Option<Place>,
    /// the one item whose files are shown (a place of its own, beside `place`)
    pub item: Option<String>,
    /// the list of places per item is open
    pub items_open: bool,
    /// the items listed, in their order (an item row's press)
    items: Vec<String>,
    /// the cards the last drop let go of, in the order shown (several when a chosen card was dragged)
    dropped: Vec<usize>,
    /// the cards chosen (by address) and the one a Shift run starts from
    pub chosen: Vec<String>,
    anchor: Option<String>,
    /// Shift and Ctrl / ⌘ as the owner saw them at the press
    pub keys: (bool, bool),
    /// the cards drawn, in their order: entry and address (a Shift run's order)
    order: Vec<(usize, String)>,
    /// the groups the viewer folded or opened, by key (a search opens them all)
    pub folds: HashMap<String, bool>,
    /// the groups drawn: key and whether folded (a heading's press)
    folded: Vec<(String, bool)>,
    /// the shelf's last view: the stamp and query it answers
    view: Option<(u64, Query, View)>,
}

impl Default for Browser {
    fn default() -> Browser {
        Browser { filter: Filter::default(), mode: Mode::Grid, typing: None, character: None, held: None, last: None, hits: Vec::new(), scope: Scope::All, grouping: Grouping::Use, sort: Sort::Name, single: false, place: None, item: None, items_open: false, items: Vec::new(), dropped: Vec::new(), chosen: Vec::new(), anchor: None, keys: (false, false), order: Vec::new(), folds: HashMap::new(), folded: Vec::new(), view: None }
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
            Hit::Scope(scope) => {
                self.scope = scope;
                Did::Taken
            }
            Hit::Unselect => {
                (self.chosen, self.anchor) = (Vec::new(), None);
                Did::Taken
            }
            Hit::Place(p) => {
                self.place = if self.place == Some(p) { None } else { Some(p) };
                self.item = None;
                Did::Taken
            }
            Hit::Items => {
                self.items_open = !self.items_open;
                Did::Taken
            }
            Hit::Item(n) => {
                let picked = self.items.get(n).cloned();
                self.item = if self.item == picked { None } else { picked };
                self.place = None;
                Did::Taken
            }
            Hit::Grouping => {
                self.grouping = match self.grouping {
                    Grouping::Use => Grouping::Kind,
                    Grouping::Kind => Grouping::None,
                    Grouping::None => Grouping::Use,
                };
                Did::Taken
            }
            Hit::Sort => {
                self.sort = match self.sort {
                    Sort::Name => Sort::Recent,
                    Sort::Recent => Sort::Size,
                    Sort::Size => Sort::Uses,
                    Sort::Uses => Sort::Name,
                };
                Did::Taken
            }
            Hit::Levels => {
                self.single = !self.single;
                (self.chosen, self.anchor) = (Vec::new(), None);
                Did::Taken
            }
            Hit::Fold(n) => {
                if let Some((key, folded)) = self.folded.get(n).cloned() {
                    self.folds.insert(key, !folded);
                }
                Did::Taken
            }
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

    /// The browser in `a`: the find field, the chips, the cards – the owner's entries, then the toon's files in the
    /// shelf's groups. `picture` answers which of the screen's pictures shows a preview (a still or a picture already
    /// loaded); `action` is the owner's one button.
    pub fn draw(&mut self, d: &mut DrawList, t: &Tokens, (a, pointer): (Rect, (f32, f32)), src: &Source, picture: &dyn Fn(&Preview) -> Option<u64>, action: Option<&str>) {
        let entries = src.entries;
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
        let f = self.filter_now();
        let plain: Vec<usize> = find(entries, &f).into_iter().filter(|&i| src.shelf.is_none() || !entries[i].kind.is_file()).collect();
        let groups = self.groups(src, &f);
        // a chip per kind there is, "related" when a character is open, then the shelf's
        let kinds = facets(entries);
        let mut chips: Vec<(String, bool, Hit)> = kinds.iter().map(|(k, n)| (format!("{} {n}", k.word()), self.filter.kinds.contains(k), Hit::Facet(*k))).collect();
        if let Some(c) = &self.character {
            chips.push((format!("related to {}", c.strip_prefix("character:").unwrap_or(c)), self.filter.related.is_some(), Hit::Related));
        }
        let shelf_chips = self.shelf_chips();
        if chips.len() > 1 || self.character.is_some() || !shelf_chips.is_empty() {
            chips.extend(shelf_chips);
            let names: Vec<&str> = chips.iter().map(|(n, _, _)| n.as_str()).collect();
            let on: Vec<bool> = chips.iter().map(|(_, o, _)| *o).collect();
            let (places, used) = controls::chips(d, t, (a.x, y, a.w), &names, &on);
            for (r, (_, _, hit)) in places.into_iter().zip(&chips) {
                self.hits.push((r, *hit));
            }
            y += used + 6.0;
        }
        if let Some(text) = self.drop_line(src) {
            let h = small * 1.4 + 4.0;
            d.sans(a.x, y + (h - small * 1.25) * 0.5, small, &controls::fit_sans(d, small, &text, a.w), t.color.muted);
            y += h + 2.0;
        }
        y = self.item_places(d, t, (a, pointer), y);
        if self.chosen.len() > 1 {
            let h = small * 1.4 + 6.0;
            let text = format!("{} selected", self.chosen.len());
            d.sans(a.x, y + (h - small * 1.25) * 0.5, small, &text, t.color.text);
            let x = controls::icon_button(d, t, Rect { x: a.x + d.sans_width(small, &text) + 6.0, y, w: h, h }, "close", t.color.muted, pointer);
            self.hits.push((x, Hit::Unselect));
            y += h + 4.0;
        }
        self.order.clear();
        if plain.is_empty() && groups.is_empty() {
            let why = if entries.is_empty() { "nothing here yet" } else { "nothing found" };
            d.sans(a.x, y + 4.0, small, why, t.color.muted);
            return;
        }
        let plain: Vec<Cell> = plain.into_iter().map(|i| Cell { tiles: vec![Tile::Card(i)], frame: None }).collect();
        y = self.cards(d, t, (a, pointer), y, src, &plain, picture);
        self.folded.clear();
        for (n, g) in groups.iter().enumerate() {
            if g.header {
                let h = small * 1.4 + 8.0;
                let r = Rect { x: a.x, y, w: a.w, h };
                if r.has(pointer.0, pointer.1) {
                    d.rect(r.x, r.y, r.w, r.h, t.color.hover);
                }
                d.icon(r.x + 2.0, r.y + (h - 12.0) * 0.5, 12.0, if g.folded { "next" } else { "down" }, t.color.muted);
                let count = g.cells.iter().map(|c| c.tiles.len()).sum::<usize>().to_string();
                let cw = d.sans_width(small, &count);
                d.sans(r.x + 18.0, r.y + (h - small * 1.25) * 0.5, small, &controls::fit_sans(d, small, &g.title, r.w - cw - 28.0), t.color.text);
                d.sans(r.x + r.w - cw - 4.0, r.y + (h - small * 1.25) * 0.5, small, &count, t.color.muted);
                d.rect(r.x, r.y + h - t.size.edge, r.w, t.size.edge, t.color.line);
                self.hits.push((r, Hit::Fold(n)));
                y += h + 4.0;
            }
            self.folded.push((g.key.clone(), g.folded));
            if !g.folded {
                y = self.cards(d, t, (a, pointer), y, src, &g.cells, picture);
            }
        }
    }

    /// The toon's files the shelf shows for the filter, in its groups: its view asked again only when the query or the
    /// shelf changed; only the kinds the owner lists (`within`), the kind chips picked, each family the owner's entry.
    fn groups(&mut self, src: &Source, f: &Filter) -> Vec<Shown> {
        let Some(shelf) = src.shelf else {
            self.view = None;
            return Vec::new();
        };
        let within: Vec<Kind> = Kind::ALL.into_iter().filter(|k| k.is_file() && src.entries.iter().any(|e| e.kind == *k)).collect();
        if within.is_empty() {
            self.view = None;
            return Vec::new();
        }
        let q = Query { scope: self.scope, grouping: self.grouping, sort: self.sort, special: self.item.clone().map(Special::Item).or(self.place.map(Place::special)), query: f.query.clone(), related: f.related.clone(), single: self.single, now_ms: (src.now_ms / MINUTE).floor() * MINUTE, within, ..Query::default() };
        if !matches!(&self.view, Some((stamp, asked, _)) if *stamp == src.stamp && *asked == q) {
            self.view = Some((src.stamp, q.clone(), shelf.view(&q)));
        }
        let Some((_, _, view)) = &self.view else { return Vec::new() };
        let by_address: HashMap<&str, usize> = src.entries.iter().enumerate().filter(|(_, e)| e.kind.is_file()).map(|(i, e)| (e.address.as_str(), i)).collect();
        let searching = !q.query.trim().is_empty();
        view.groups
            .iter()
            .filter_map(|g| {
                let cells: Vec<Cell> = g.cells.iter().filter_map(|c| cell((shelf, self.single), c, &by_address, src.entries, f)).collect();
                let folded = !searching && self.folds.get(&g.key).copied().unwrap_or(g.folded);
                (!cells.is_empty()).then(|| Shown { key: g.key.clone(), title: title(g), header: g.what != GroupKind::All, folded, cells })
            })
            .collect()
    }

    /// The places per item (the shelf's one-item places): a heading that opens the list, each item's address and how many
    /// of the scope's files it has; the one shown is lit. Where it ends.
    fn item_places(&mut self, d: &mut DrawList, t: &Tokens, (a, pointer): (Rect, (f32, f32)), mut y: f32) -> f32 {
        let list: Vec<(String, usize)> = self.view.as_ref().map(|(_, _, v)| v.specials.items.clone()).unwrap_or_default();
        self.items = list.iter().map(|(i, _)| i.clone()).collect();
        if list.is_empty() {
            return y;
        }
        let small = t.size.small;
        let h = small * 1.4 + 6.0;
        let head = Rect { x: a.x, y, w: a.w, h };
        if head.has(pointer.0, pointer.1) {
            d.rect(head.x, head.y, head.w, head.h, t.color.hover);
        }
        d.icon(head.x + 2.0, head.y + (h - 12.0) * 0.5, 12.0, if self.items_open { "down" } else { "next" }, t.color.muted);
        let title = match &self.item {
            Some(i) => format!("by item: {i}"),
            None => format!("by item {}", list.len()),
        };
        d.sans(head.x + 18.0, head.y + (h - small * 1.25) * 0.5, small, &controls::fit_sans(d, small, &title, head.w - 22.0), if self.item.is_some() { t.color.text } else { t.color.muted });
        self.hits.push((head, Hit::Items));
        y += h;
        if self.items_open {
            for (n, (item, count)) in list.iter().enumerate() {
                let r = Rect { x: a.x, y, w: a.w, h };
                if self.item.as_deref() == Some(item.as_str()) {
                    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.selected);
                } else if r.has(pointer.0, pointer.1) {
                    d.rect(r.x, r.y, r.w, r.h, t.color.hover);
                }
                let c = count.to_string();
                let cw = d.sans_width(small, &c);
                d.sans(r.x + 18.0, r.y + (h - small * 1.25) * 0.5, small, &controls::fit_sans(d, small, item, r.w - cw - 28.0), t.color.text);
                d.sans(r.x + r.w - cw - 4.0, r.y + (h - small * 1.25) * 0.5, small, &c, t.color.muted);
                self.hits.push((r, Hit::Item(n)));
                y += h;
            }
        }
        y + 4.0
    }

    /// where a file dropped on the editor goes (the core's folder for the open item), when the workspace lists files
    fn drop_line(&self, src: &Source) -> Option<String> {
        let to = src.shelf.filter(|_| self.view.is_some()).and_then(Shelf::upload_folder)?;
        Some(format!("files dropped here go to {to}"))
    }

    /// the shelf's chips after the kinds: its scopes with their counts, the places, the grouping and the order
    fn shelf_chips(&self) -> Vec<(String, bool, Hit)> {
        let Some((_, _, v)) = &self.view else { return Vec::new() };
        let mut out = Vec::new();
        for (scope, word, n) in [(Scope::Context, "in use", v.scopes.context), (Scope::All, "all files", v.scopes.all), (Scope::Unused, "unused", v.scopes.unused)] {
            if n > 0 || self.scope == scope || scope == Scope::All {
                out.push((format!("{word} {n}"), self.scope == scope, Hit::Scope(scope)));
            }
        }
        let places = [
            (Place::Recent, format!("this week {}", v.specials.recent), v.specials.recent > 0),
            (Place::Credits, format!("no credits {}", v.specials.credits.unwrap_or(0)), v.specials.credits.is_some_and(|n| n > 0)),
            (Place::Missing, "missing versions".to_string(), v.specials.missing > 0),
        ];
        for (place, word, there) in places {
            if there || self.place == Some(place) {
                out.push((word, self.place == Some(place), Hit::Place(place)));
            }
        }
        let grouping = match self.grouping {
            Grouping::Use => "by use",
            Grouping::Kind => "by kind",
            Grouping::None => "no groups",
        };
        let sort = match self.sort {
            Sort::Name => "by name",
            Sort::Recent => "newest first",
            Sort::Size => "biggest first",
            Sort::Uses => "most used first",
        };
        out.push((grouping.to_string(), false, Hit::Grouping));
        out.push((sort.to_string(), false, Hit::Sort));
        out.push((if self.single { "levels single" } else { "levels stacked" }.to_string(), self.single, Hit::Levels));
        out
    }

    /// The cells of `list` from `y` on, in the grid or the list, a frame's tiles together inside its outline; where they end.
    #[allow(clippy::too_many_arguments)]
    fn cards(&mut self, d: &mut DrawList, t: &Tokens, (a, pointer): (Rect, (f32, f32)), mut y: f32, src: &Source, list: &[Cell], picture: &dyn Fn(&Preview) -> Option<u64>) -> f32 {
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
        let slot = |n: usize| Rect { x: a.x + (n % cols) as f32 * (cw + gap), y: y + (n / cols) as f32 * step, w: cw, h: ch };
        let mut n = 0;
        for c in list {
            let first = n;
            for tile in &c.tiles {
                self.tile(d, t, (slot(n), pointer), src, tile, picture);
                n += 1;
            }
            if c.frame.is_some() {
                frame(d, t, (first..n).map(slot).collect(), if grid { gap * 0.5 - 1.0 } else { 1.0 });
            }
        }
        y + n.div_ceil(cols) as f32 * step + if grid { 0.0 } else { 4.0 }
    }

    /// one tile in `r`: an owner's card (pressed, dragged, chosen) or a frame's part the owner does not list (drawn only)
    fn tile(&mut self, d: &mut DrawList, t: &Tokens, (r, pointer): (Rect, (f32, f32)), src: &Source, tile: &Tile, picture: &dyn Fn(&Preview) -> Option<u64>) {
        let small = t.size.small;
        let (e, i) = match tile {
            Tile::Card(i) => (&src.entries[*i], Some(*i)),
            Tile::Part(e) => (e, None),
        };
        let lit = i.is_some_and(|i| self.held.map(|h| h.entry) == Some(i) || self.chosen.contains(&e.address));
        let hover = i.is_some() && r.has(pointer.0, pointer.1);
        let stack = if i.is_some() { src.stack(e, self.single) } else { 1 };
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
                    d.rect(r.x, r.y, r.w, r.h, t.color.hover);
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
        if let Some(i) = i {
            self.order.push((i, e.address.clone()));
            self.hits.push((r, Hit::Card(i)));
        }
    }
}

/// what one place in the files shows: an owner's card, or a frame's part the owner does not list
#[derive(Debug, Clone, PartialEq)]
enum Tile {
    Card(usize),
    Part(Entry),
}

/// one cell: one card, or a frame's main and its parts
#[derive(Debug, Clone)]
struct Cell {
    tiles: Vec<Tile>,
    frame: Option<Kind>,
}

/// The shelf's cell as tiles: each family the owner's entry (by its main file); a frame's part the owner does not list is
/// drawn from its family. A kind chip picks by the cell's main. None = nothing of it is shown.
fn cell((shelf, single): (&Shelf, bool), c: &deck_catalog::library::shelf::Cell, by_address: &HashMap<&str, usize>, entries: &[Entry], f: &Filter) -> Option<Cell> {
    let mut tiles = Vec::new();
    for k in &c.keys {
        let Some(fam) = shelf.family_in(k, single) else { continue };
        match by_address.get(fam.main.as_str()).copied() {
            Some(i) => tiles.push(Tile::Card(i)),
            None if !tiles.is_empty() => tiles.push(Tile::Part(fam.entry())),
            None => return None,
        }
    }
    let Some(Tile::Card(main)) = tiles.first() else { return None };
    (f.kinds.is_empty() || f.kinds.contains(&entries[*main].kind)).then(|| Cell { frame: c.frame.filter(|_| tiles.len() > 1), tiles })
}

/// a frame's outline around its tiles: in the grid one per row they take, in the list one around its rows
fn frame(d: &mut DrawList, t: &Tokens, slots: Vec<Rect>, pad: f32) {
    let mut rows: Vec<Rect> = Vec::new();
    for r in slots {
        match rows.last_mut() {
            Some(last) if last.y == r.y => last.w = r.x + r.w - last.x,
            // the list: one row under the other
            Some(last) if last.x == r.x && last.w == r.w => last.h = r.y + r.h - last.y,
            _ => rows.push(r),
        }
    }
    for r in rows {
        d.outline(r.x - pad, r.y - pad, r.w + 2.0 * pad, r.h + 2.0 * pad, (t.size.radius + pad, 1.0), t.color.text);
    }
}

/// one group of the toon's files as the browser shows it
struct Shown {
    key: String,
    title: String,
    /// a heading over it (not over the one group of an ungrouped view)
    header: bool,
    folded: bool,
    /// the owner's entries in the shelf's order, a frame's together
    cells: Vec<Cell>,
}

/// a group's heading
fn title(g: &Group) -> String {
    let item = g.item.as_deref().unwrap_or("");
    match g.what {
        GroupKind::Open => format!("here: {item}"),
        GroupKind::Unused => "unused".into(),
        GroupKind::Item => item.to_string(),
        GroupKind::Toon => format!("the toon {}", item.strip_prefix("toon:").unwrap_or(item)),
        GroupKind::Other => "other files".into(),
        GroupKind::Kind => Kind::of_id(item).map_or_else(|| item.to_string(), |k| k.word().to_string()),
        GroupKind::All => "all files".into(),
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
    pub(crate) fn shelf() -> Shelf {
        let file = |path: &str| format!(r#"{{ "path": "{path}", "size": 10 }}"#);
        let asset = |id: &str, kind: &str, files: &[&str], extra: &str| format!(r#"{{ "id": "{id}", "kind": "{kind}", "usages": 1{extra}, "renditions": [{}] }}"#, files.iter().map(|f| file(f)).collect::<Vec<_>>().join(","));
        let items = [
            asset("a1", "image", &["cat/s1/c1/hills.webp", "cat/s1/c1/hills.2x.webp", "cat/s1/c1/hills.3x.webp"], r#", "updated_at": "2026-11-19 10:00:00""#),
            asset("a2", "image", &["cat/s1/c2/sky.webp"], r#", "credits": """#),
            asset("a3", "audio", &["cat/bells.128.mp3"], r#", "usages": 0"#),
        ];
        Shelf::from_json(&format!(r#"{{ "items": [{}] }}"#, items.join(","))).expect("the answer")
    }

    #[test]
    fn a_family_of_versions_is_one_card_drawn_as_a_stack() {
        let shelf = shelf();
        let entries: Vec<Entry> = shelf.families(false).iter().map(|f| f.entry()).collect();
        assert_eq!(entries.len(), 3, "one card per family, not per file");
        let src = Source { entries: &entries, shelf: Some(&shelf), stamp: 0, now_ms: 0.0 };
        let hills = entries.iter().position(|e| e.label == "hills").expect("hills");
        let sky = entries.iter().position(|e| e.label == "sky").expect("sky");
        assert_eq!((src.stack(&entries[hills], false), src.stack(&entries[sky], false)), (3, 1));
        assert_eq!(src.stack(&entries[hills], true), 1, "single levels: no stack");
        let mut b = Browser::default();
        b.filter.query = "hills".into();
        let d = draw_src(&mut b, &src);
        let (r, _) = *b.hits.iter().find(|(_, h)| *h == Hit::Card(hills)).expect("the hills' card");
        let behind = |k: f32| d.prims().iter().any(|p| matches!(p, Prim::Outline { x, y, .. } if *x == r.x + SHEET_STEP * k && *y == r.y - SHEET_STEP * k));
        assert!(behind(1.0) && behind(2.0) && !behind(3.0), "two sheets behind the three versions' card");
    }

    fn draw_src(b: &mut Browser, src: &Source) -> DrawList {
        let mut d = DrawList::new(8.0);
        b.draw(&mut d, &Tokens::default(), (Rect { x: 0.0, y: 0.0, w: 420.0, h: 4000.0 }, (-1.0, -1.0)), src, &|_| None, None);
        d
    }

    fn cards(b: &Browser) -> Vec<usize> {
        b.hits.iter().filter_map(|(_, h)| if let Hit::Card(i) = h { Some(*i) } else { None }).collect()
    }

    #[test]
    fn the_files_are_scoped_by_use_and_all_pins_the_open_item_and_folds_the_rest() {
        let mut shelf = shelf();
        shelf.set_context(deck_catalog::library::shelf::Context { open: Some("scene:nave".into()), toon: Some("cat".into()), uses: vec![deck_catalog::library::shelf::Use { path: "cat/s1/c2/sky.webp".into(), item: "scene:nave".into(), part: "layer 1".into(), at: String::new() }], ..Default::default() });
        let mut entries: Vec<Entry> = shelf.families(false).iter().map(|f| f.entry()).collect();
        entries.insert(0, entry("module:walk", Kind::Module, Preview::Kind, &[]));
        let label = |i: usize| entries[i].label.clone();
        let src = Source { entries: &entries, shelf: Some(&shelf), stamp: 1, now_ms: 0.0 };
        let mut b = Browser::default();
        draw_src(&mut b, &src);
        assert_eq!(cards(&b).into_iter().map(label).collect::<Vec<_>>(), ["walk", "sky"], "the owner's entries, then the open item's files pinned; the rest folded");
        let folds = b.hits.iter().filter(|(_, h)| matches!(h, Hit::Fold(_))).count();
        assert!(folds >= 2, "a heading per group");
        let (x, y) = at(&b, Hit::Fold(1));
        b.press(x, y, 0.0);
        draw_src(&mut b, &src);
        assert_eq!(cards(&b).len(), 3, "a heading opens its group");
        let (x, y) = at(&b, Hit::Scope(Scope::Context));
        b.press(x, y, 1.0);
        draw_src(&mut b, &src);
        assert_eq!(cards(&b).into_iter().map(label).collect::<Vec<_>>(), ["walk", "sky"], "in use: only what the scene uses");
        b.scope = Scope::All;
        b.filter.query = "bells".into();
        draw_src(&mut b, &src);
        assert_eq!(cards(&b).into_iter().map(label).collect::<Vec<_>>(), ["bells"], "a search opens every group");
        b.filter.query.clear();
        b.filter.kinds = vec![Kind::Image];
        draw_src(&mut b, &src);
        assert!(cards(&b).into_iter().all(|i| entries[i].kind == Kind::Image), "a kind chip narrows the files too");
    }

    #[test]
    fn the_files_group_by_use_kind_or_none_and_sort_by_name_newest_size_or_uses() {
        let shelf = shelf();
        let entries: Vec<Entry> = shelf.families(false).iter().map(|f| f.entry()).collect();
        let src = Source { entries: &entries, shelf: Some(&shelf), stamp: 1, now_ms: 0.0 };
        let mut b = Browser::default();
        draw_src(&mut b, &src);
        let (x, y) = at(&b, Hit::Grouping);
        b.press(x, y, 0.0);
        assert_eq!(b.grouping, Grouping::Kind);
        draw_src(&mut b, &src);
        assert_eq!(b.hits.iter().filter(|(_, h)| matches!(h, Hit::Fold(_))).count(), 2, "a heading per kind: images, audio");
        b.press(x, y, 1.0);
        let d = draw_src(&mut b, &src);
        assert_eq!(b.hits.iter().filter(|(_, h)| matches!(h, Hit::Fold(_))).count(), 0, "no groups: no headings");
        assert!(d.texts().contains(&"no groups"));
        let names = |b: &Browser| cards(b).into_iter().map(|i| entries[i].label.clone()).collect::<Vec<_>>();
        assert_eq!(names(&b), ["bells", "hills", "sky"], "by name");
        let (x, y) = at(&b, Hit::Sort);
        b.press(x, y, 2.0);
        draw_src(&mut b, &src);
        assert_eq!(names(&b)[0], "hills", "newest first");
        b.press(x, y, 3.0);
        draw_src(&mut b, &src);
        assert_eq!(names(&b)[0], "hills", "biggest first: three files");
        b.press(x, y, 4.0);
        draw_src(&mut b, &src);
        assert_eq!(names(&b).last().map(String::as_str), Some("bells"), "most used first: the unused last");
    }

    #[test]
    fn places_show_the_files_added_this_week_without_credits_or_missing_versions() {
        let shelf = shelf();
        let entries: Vec<Entry> = shelf.families(false).iter().map(|f| f.entry()).collect();
        let now = deck_catalog::library::shelf::epoch_ms("2026-11-20 10:00:00").unwrap();
        let src = Source { entries: &entries, shelf: Some(&shelf), stamp: 1, now_ms: now };
        let mut b = Browser { grouping: Grouping::None, ..Browser::default() };
        let d = draw_src(&mut b, &src);
        assert!(d.texts().contains(&"this week 1") && d.texts().contains(&"no credits 1"), "{:?}", d.texts());
        let names = |b: &Browser| cards(b).into_iter().map(|i| entries[i].label.clone()).collect::<Vec<_>>();
        let (x, y) = at(&b, Hit::Place(Place::Recent));
        b.press(x, y, 0.0);
        draw_src(&mut b, &src);
        assert_eq!(names(&b), ["hills"]);
        let (x, y) = at(&b, Hit::Place(Place::Credits));
        b.press(x, y, 1.0);
        draw_src(&mut b, &src);
        assert_eq!(names(&b), ["sky"]);
        let (x, y) = at(&b, Hit::Place(Place::Missing));
        b.press(x, y, 2.0);
        let d = draw_src(&mut b, &src);
        assert!(d.texts().contains(&"missing versions"), "on demand, without a count");
        assert!(names(&b).contains(&"bells".to_string()), "an audio lacking its levels: {:?}", names(&b));
        b.press(x, y, 3.0);
        draw_src(&mut b, &src);
        assert_eq!((b.place, names(&b).len()), (None, 3), "the same place again: all of the scope");
    }

    /// the toon's files with a video and its subtitles (a file kind no workspace lists)
    fn framed() -> Shelf {
        let items = r#"{ "items": [
            { "id": "v", "kind": "video", "usages": 1, "renditions": [{ "path": "cat/s1/speak.720p.mp4", "width": 1280, "height": 720 }, { "path": "cat/s1/speak.480p.mp4", "width": 854, "height": 480 }] },
            { "id": "s", "kind": "other", "usages": 1, "renditions": [{ "path": "cat/s1/speak.en.vtt" }] },
            { "id": "h", "kind": "image", "usages": 1, "renditions": [{ "path": "cat/s1/hills.webp" }, { "path": "cat/s1/hills.2x.webp" }] }
        ] }"#;
        Shelf::from_json(items).expect("the answer")
    }

    /// a video and its subtitles stand in one frame: the subtitles drawn from their family though the owner lists no
    /// files of their kind, outlined together with the video's card
    #[test]
    fn a_video_and_its_subtitles_stand_in_one_frame() {
        let shelf = framed();
        let entries: Vec<Entry> = shelf.families(false).iter().filter(|f| f.kind != Kind::File).map(|f| f.entry()).collect();
        let src = Source { entries: &entries, shelf: Some(&shelf), stamp: 1, now_ms: 0.0 };
        let mut b = Browser { grouping: Grouping::None, ..Browser::default() };
        let d = draw_src(&mut b, &src);
        assert_eq!(cards(&b).len(), 2, "the part is drawn, not a card of the owner's");
        let video = entries.iter().position(|e| e.kind == Kind::Video).unwrap();
        let (r, _) = *b.hits.iter().find(|(_, h)| *h == Hit::Card(video)).unwrap();
        let tiles = |d: &DrawList, r: Rect| d.prims().iter().filter(|p| matches!(p, Prim::Rect { y, w, h, .. } if *y == r.y && *w == r.w && *h == r.h)).count();
        assert_eq!(tiles(&d, r), 3, "the video, its subtitles' tile, the hills");
        assert!(d.prims().iter().any(|p| matches!(p, Prim::Icon { name, .. } if *name == Kind::File.icon())), "the subtitles show their kind");
        let framed = d.prims().iter().any(|p| matches!(p, Prim::Outline { x, y, w, .. } if *x < r.x && *y < r.y && *w > 2.0 * r.w));
        assert!(framed, "one outline around the video and its subtitles");
        b.mode = Mode::List;
        let d = draw_src(&mut b, &src);
        let (r, _) = *b.hits.iter().find(|(_, h)| *h == Hit::Card(video)).unwrap();
        assert!(d.prims().iter().any(|p| matches!(p, Prim::Outline { y, h, .. } if *y < r.y && *h > 2.0 * r.h)), "in the list: one outline around both rows");
    }

    /// the levels chip: stacked one card per family, single every file its own card (the owner lists the single files)
    #[test]
    fn the_levels_show_stacked_or_single() {
        let shelf = shelf();
        let stacked: Vec<Entry> = shelf.families(false).iter().map(|f| f.entry()).collect();
        let single: Vec<Entry> = shelf.families(true).iter().map(|f| f.entry()).collect();
        let mut b = Browser { grouping: Grouping::None, ..Browser::default() };
        let d = draw_src(&mut b, &Source { entries: &stacked, shelf: Some(&shelf), stamp: 1, now_ms: 0.0 });
        assert!(d.texts().contains(&"levels stacked"));
        assert_eq!(cards(&b).len(), 3);
        let (x, y) = at(&b, Hit::Levels);
        b.chosen = vec!["cat/bells.128.mp3".into()];
        b.press(x, y, 0.0);
        assert!(b.single && b.chosen.is_empty(), "single, and the stacked selection gone");
        let src = Source { entries: &single, shelf: Some(&shelf), stamp: 1, now_ms: 0.0 };
        let d = draw_src(&mut b, &src);
        assert!(d.texts().contains(&"levels single"));
        assert_eq!(cards(&b).len(), 5, "hills' 2x and 3x stand alone");
        let three = single.iter().position(|e| e.address == "cat/s1/c1/hills.3x.webp").expect("the 3x");
        let (r, _) = *b.hits.iter().find(|(_, h)| *h == Hit::Card(three)).unwrap();
        assert!(!d.prims().iter().any(|p| matches!(p, Prim::Outline { x, y, .. } if *x == r.x + SHEET_STEP && *y == r.y - SHEET_STEP)), "no sheets behind a single file");
    }

    #[test]
    fn the_browser_says_where_a_dropped_file_goes() {
        let mut shelf = shelf();
        let entries: Vec<Entry> = shelf.families(false).iter().map(|f| f.entry()).collect();
        let mut b = Browser::default();
        let src = Source { entries: &entries, shelf: Some(&shelf), stamp: 1, now_ms: 0.0 };
        let d = draw_src(&mut b, &src);
        assert_eq!(b.drop_line(&src), None, "nothing open, no toon: no folder");
        assert!(!d.texts().iter().any(|s| s.starts_with("fil")));
        shelf.set_context(deck_catalog::library::shelf::Context { open: Some("character:catpope".into()), toon: Some("cat".into()), by_home: true, ..Default::default() });
        let src = Source { entries: &entries, shelf: Some(&shelf), stamp: 2, now_ms: 0.0 };
        let d = draw_src(&mut b, &src);
        assert_eq!(b.drop_line(&src).as_deref(), Some("files dropped here go to characters/catpope/"));
        assert!(d.texts().iter().any(|s| s.starts_with("fil")), "drawn (cut to the panel's width): {:?}", d.texts());
        let modules = list();
        let src = Source { entries: &modules, shelf: Some(&shelf), stamp: 3, now_ms: 0.0 };
        draw_src(&mut b, &src);
        assert_eq!(b.drop_line(&src), None, "a workspace that lists no files takes none");
    }

    /// every item with files is a place of its own: listed with its count, pressed it shows only its files
    #[test]
    fn each_item_is_a_place_listed_with_its_count() {
        let mut shelf = shelf();
        let used = |path: &str, item: &str| deck_catalog::library::shelf::Use { path: path.into(), item: item.into(), part: String::new(), at: String::new() };
        shelf.set_context(deck_catalog::library::shelf::Context { open: Some("scene:nave".into()), toon: Some("cat".into()), uses: vec![used("cat/s1/c2/sky.webp", "scene:nave"), used("cat/s1/c1/hills.2x.webp", "scene:dunes")], ..Default::default() });
        let entries: Vec<Entry> = shelf.families(false).iter().map(|f| f.entry()).collect();
        let src = Source { entries: &entries, shelf: Some(&shelf), stamp: 1, now_ms: 0.0 };
        let mut b = Browser { grouping: Grouping::None, ..Browser::default() };
        draw_src(&mut b, &src);
        assert!(!b.hits.iter().any(|(_, h)| matches!(h, Hit::Item(_))), "folded at first");
        let (x, y) = at(&b, Hit::Items);
        b.press(x, y, 0.0);
        let d = draw_src(&mut b, &src);
        assert_eq!(b.items, ["chapter:cat:1", "chapter:cat:2", "scene:dunes", "scene:nave", "toon:cat"], "every item with files, in natural order");
        assert!(d.texts().iter().filter(|s| **s == "1").count() >= 4, "each with its count: {:?}", d.texts());
        let dunes = b.items.iter().position(|i| i == "scene:dunes").unwrap();
        let (x, y) = at(&b, Hit::Item(dunes));
        b.press(x, y, 1.0);
        draw_src(&mut b, &src);
        assert_eq!(cards(&b).into_iter().map(|i| entries[i].label.clone()).collect::<Vec<_>>(), ["hills"], "only the files scene:dunes uses");
        let (x, y) = at(&b, Hit::Place(Place::Credits));
        b.press(x, y, 2.0);
        assert_eq!((b.item.as_deref(), b.place), (None, Some(Place::Credits)), "a place replaces the item");
        let (x, y) = at(&b, Hit::Item(dunes));
        b.press(x, y, 3.0);
        b.press(x, y, 4.0);
        draw_src(&mut b, &src);
        assert_eq!((b.item.as_deref(), b.place, cards(&b).len()), (None, None, 3), "the same item again: all of the scope");
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
