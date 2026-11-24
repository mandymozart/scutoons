//! The cue matrix's sizes in the editor: a column's right edge, a row's bottom edge or the row heads' edge dragged, a
//! double click fitting it to its content. The sizes and their limits are deck-show's (`sizes.rs`); they belong to this
//! viewer's view of each show, kept in the editor's cache beside the panels, never in the show.
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::cards;
use deck_kit::editor::table::Edge;
use deck_show::sizes::{Sizes, CELL};
use deck_show::view::Matrix;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// how near an edge the pointer grabs it, either side
pub const GRIP: f32 = 3.0;
const DOUBLE: Duration = Duration::from_millis(400);

/// An edge in the hand: where the drag started, the size then and now (pixels of this frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resize {
    pub edge: Edge,
    pub from: (f32, f32),
    pub start: f32,
    pub now: f32,
}

/// What the band's content asks for, measured as it was drawn (pixels): each column's widest cell, each row's height,
/// the row heads' widest; and a rem in pixels.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fits {
    pub columns: Vec<f32>,
    pub rows: Vec<f32>,
    pub heads: f32,
    pub rem: f32,
}

/// every show's sizes, as the cache keeps them
pub type Views = BTreeMap<String, Sizes>;

impl Host {
    /// the sizes of the show shown, as this viewer left them
    pub fn view_sizes(&self) -> Sizes {
        self.sizes.get(&self.shown().id).cloned().unwrap_or_default()
    }

    /// Reads the sizes kept in `file`; they are written back there whenever an edge is let go or fitted.
    pub fn keep_views_in(&mut self, file: PathBuf) {
        self.sizes = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        self.views_file = Some(file);
    }

    fn write_views(&self) {
        let Some(file) = &self.views_file else { return };
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let kept: Views = self.sizes.iter().filter(|(_, s)| !s.is_empty()).map(|(k, s)| (k.clone(), s.clone())).collect();
        if let Ok(s) = serde_json::to_string_pretty(&kept) {
            std::fs::write(file, s).ok();
        }
    }

    /// what `edge` is called on the badge, and the id its size is kept under (none: the new row's place)
    fn edge_names(&self, edge: Edge) -> Option<(String, String)> {
        let show = self.shown();
        match edge {
            Edge::Heads => Some(("rows".into(), String::new())),
            Edge::Column(c) => deck_show::view::plan(show).columns.get(c).map(|col| (col.name.clone(), col.id.clone())),
            Edge::Row(r) => show.rows.get(r).map(|row| (row.id.clone(), row.id.clone())),
        }
    }

    /// the badge over an edge in the hand: `right · 230 px`
    pub fn edge_badge(&self) -> Option<String> {
        let r = self.resizing?;
        let (name, _) = self.edge_names(r.edge)?;
        Some(format!("{name} · {} px", r.now.round()))
    }

    /// `edge` set to `rem` (deck-show holds it to its limits); the size it came to, in pixels
    fn set_edge(&mut self, edge: Edge, rem: f32) -> Option<f32> {
        let (_, id) = self.edge_names(edge)?;
        let unit = self.band_fits.rem.max(1.0);
        let show = self.shown().id.clone();
        let s = self.sizes.entry(show).or_default();
        let to = match edge {
            Edge::Heads => s.set_heads(rem),
            Edge::Column(_) => s.set_column(&id, rem),
            Edge::Row(_) => s.set_row(&id, rem),
        };
        Some(to * unit)
    }

    /// A press on an edge: it is taken in the hand; the second of a double click fits it to its content.
    pub(super) fn edge_press(&mut self, edge: Edge) {
        self.edge_press_at(edge, Instant::now());
    }

    /// `edge_press` at `now` (the tests say when the second press comes: a loaded machine's real gap is no double click's)
    pub(super) fn edge_press_at(&mut self, edge: Edge, now: Instant) {
        let Some(laid) = self.band.as_ref() else { return };
        let start = laid.edge_size(edge);
        if self.edge_names(edge).is_none() {
            return;
        }
        if self.last_edge.is_some_and(|(e, at)| e == edge && now.saturating_duration_since(at) < DOUBLE) {
            self.last_edge = None;
            return self.fit_edge(edge);
        }
        self.last_edge = Some((edge, now));
        self.resizing = Some(Resize { edge, from: self.cursor, start, now: start });
    }

    /// the pointer moved: the edge in the hand follows it
    pub(super) fn edge_moved(&mut self, x: f32, y: f32) {
        let Some(r) = self.resizing else { return };
        let delta = match r.edge {
            Edge::Row(_) => y - r.from.1,
            _ => x - r.from.0,
        };
        let unit = self.band_fits.rem.max(1.0);
        if let Some(now) = self.set_edge(r.edge, (r.start + delta) / unit) {
            self.resizing = Some(Resize { now, ..r });
        }
    }

    /// the button let go: the size is kept
    pub(super) fn edge_release(&mut self) {
        if self.resizing.take().is_some() {
            self.write_views();
        }
    }

    /// `edge` fitted to what its content asks for (the widest cell of a column, a row's lines, the widest row head)
    fn fit_edge(&mut self, edge: Edge) {
        let f = &self.band_fits;
        let want = match edge {
            Edge::Heads => Some(f.heads),
            Edge::Column(c) => f.columns.get(c).copied(),
            Edge::Row(r) => f.rows.get(r).copied(),
        };
        let unit = f.rem.max(1.0);
        if let Some(px) = want.and_then(|w| self.set_edge(edge, w / unit)) {
            let name = self.edge_names(edge).map(|(n, _)| n).unwrap_or_default();
            self.status = format!("{name} fits its content: {} px", px.round());
            self.write_views();
        }
    }

    /// The row at `at` was renamed from `from`: its height goes with it.
    pub(super) fn row_renamed(&mut self, at: Option<usize>, from: &str) {
        let Some(to) = at.and_then(|i| self.shown().rows.get(i)).map(|r| r.id.clone()).filter(|to| to != from) else { return };
        let show = self.shown().id.clone();
        if let Some(s) = self.sizes.get_mut(&show) {
            s.renamed_row(from, &to);
            self.write_views();
        }
    }

    /// A check run's drag: `column:ID`, `row:ID` or `heads` taken at its middle and moved by `by` pixels (across for the
    /// heads and a column, down for a row), held.
    pub(super) fn script_band_drag(&mut self, edge: &str, by: f32) {
        let show = self.shown();
        let found = match edge.split_once(':') {
            Some(("column", id)) => deck_show::view::plan(show).columns.iter().position(|c| c.id == id).map(Edge::Column),
            Some(("row", id)) => show.rows.iter().position(|r| r.id == id).map(Edge::Row),
            _ if edge == "heads" => Some(Edge::Heads),
            _ => None,
        };
        let Some(e) = found else {
            self.status = format!("--band-drag: no edge {edge}");
            return;
        };
        let Some(r) = self.hits.iter().rev().find(|(_, h)| *h == Hit::Edge(e)).map(|(r, _)| *r) else { return };
        let (x, y) = (r.x + r.w * 0.5, r.y + r.h * 0.5);
        self.pointer(x, y);
        self.press(Hit::Edge(e));
        let to = if matches!(e, Edge::Row(_)) { (x, y + by) } else { (x + by, y) };
        self.pointer(to.0, to.1);
    }

    /// the pointer's look over the band: the arrows of an edge it is on or drags
    pub fn band_cursor(&self, x: f32, y: f32) -> Option<winit::window::CursorIcon> {
        let edge = self.resizing.map(|r| r.edge).or_else(|| match self.hit(x, y) {
            Some(Hit::Edge(e)) => Some(e),
            _ => None,
        })?;
        Some(match edge {
            Edge::Row(_) => winit::window::CursorIcon::NsResize,
            _ => winit::window::CursorIcon::EwResize,
        })
    }
}

/// What the band's content asks for at these tokens: each column's widest cell or head, each row's lines (one, or two
/// with a status line or a fire tag), the widest row head with its tag; `rad` is a cell's ▶.
pub(super) fn measure(d: &DrawList, t: &Tokens, host: &Host, m: &Matrix, rad: f32) -> Fits {
    let (pad, font, small, lpx) = (t.size.pad, t.size.font, t.size.small, t.size.label * 1.2);
    let w = |px: f32, s: &str| d.advance(px) * s.chars().count() as f32;
    let h0 = t.size.rem(CELL.1);
    let stop = super::band::stop_width(t);
    let mut columns: Vec<f32> = m
        .columns
        .iter()
        .map(|col| {
            let kind = col.filter.as_ref().map_or_else(|| col.detail.clone(), |f| format!("{} · {f}", col.detail));
            (w(font, &col.name) + 3.0 * pad + stop).max(w(lpx, &kind) + 2.0 * pad)
        })
        .collect();
    let mut rows = Vec::with_capacity(m.rows.len());
    let mut heads: f32 = 0.0;
    let rr = (h0 * 0.26).min(t.size.rem(1.0));
    for r in &m.rows {
        let mut two = false;
        for (c, cell) in r.cells.iter().enumerate() {
            let need = if cell.empty {
                2.0 * rad + 2.0 * pad
            } else if cell.keep {
                w(small, &cell.text) + 2.0 * pad
            } else {
                let words = cell.ask.as_ref().map_or_else(|| Some(cell.text.clone()), |a| a.words()).unwrap_or_default();
                let note = super::band::note(t, host, (r, c), m, cell);
                two |= note.is_some();
                let line = pad + 2.0 * rad + pad * 0.75 + w(font, &words) + pad;
                let status = note.map_or(0.0, |(s, _)| pad + lpx + w(lpx, &s) + pad);
                line.max(status) + 6.0
            };
            if let Some(x) = columns.get_mut(c) {
                *x = x.max(need);
            }
        }
        let card = pad * 0.6 + 2.0 * rad + pad * 0.4 + lpx * 1.25 + pad * 0.5 + 6.0;
        let head = font * 1.2 + 3.0 + cards::tag_height(t) + pad;
        rows.push(match (two, r.chip.is_some()) {
            (false, false) => h0,
            (true, false) => card.max(h0),
            (false, true) => head.max(h0),
            (true, true) => card.max(head).max(h0),
        });
        let waits = if r.chip.as_ref().is_some_and(|c| c.waiting) { w(lpx, deck_show::view::WAITS) + pad } else { 0.0 };
        let tag = r.chip.as_ref().map_or(0.0, |c| cards::tag_width(d, t, &super::band::fire_tag_of(t, c)));
        let lead = pad + 3.0 + 2.0 * rr + pad + w(small, "00") + pad * 0.75;
        heads = heads.max(lead + (w(font, &r.id) + waits).max(tag) + 2.0 * pad);
    }
    Fits { columns, rows, heads, rem: t.size.rem(1.0) }
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{host_with, H, W};
    use super::super::{author, Hit, Host};
    use super::Edge;
    use deck_kit::base::draw::DrawList;
    use deck_kit::base::tokens::Tokens;
    use deck_kit::editor::controls::Rect;
    use deck_show::sizes::{CELL, MIN};

    const DOC: &str = r#"{ "id": "sizes", "version": 1,
      "toons": ["catpope"], "outlets": [{ "id": "left" }, { "id": "right" }, { "id": "lights", "kind": "device" }],
      "rows": [
        { "id": "dialog", "fire": { "event": "ch3.dialog", "how": "auto", "moment": "bar" }, "cells": { "left": { "mode": "play", "piece": "scene:nave" } } },
        { "id": "yellowmouse", "fire": { "event": "ch3.yellowmouse-enters", "how": "suggest" }, "cells": { "right": { "mode": "play", "piece": "scene:a-scene-with-a-really-long-name-indeed" } } }
      ] }"#;

    fn frame(host: &mut Host) -> Vec<String> {
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        host.hits.clear();
        author::draw(&mut d, &t, host, W, H, host.cursor);
        d.texts().into_iter().map(str::to_string).collect()
    }

    fn place(host: &Host, want: Hit) -> Rect {
        host.hits.iter().rev().find(|(_, h)| *h == want).map(|(r, _)| *r).unwrap_or_else(|| panic!("no {want:?}"))
    }

    /// a press on `edge`'s middle, the pointer moved by `by`, let go
    fn drag(host: &mut Host, edge: Edge, by: (f32, f32)) {
        frame(host);
        let r = place(host, Hit::Edge(edge));
        let (x, y) = (r.x + r.w * 0.5, r.y + r.h * 0.5);
        host.pointer(x, y);
        assert_eq!(host.hit(x, y), Some(Hit::Edge(edge)), "the edge takes the press before what is under it");
        host.press(Hit::Edge(edge));
        host.pointer(x + by.0, y + by.1);
        host.release();
        host.last_edge = None;
    }

    #[test]
    fn a_column_dragged_by_its_edge_widens_the_row_heads_and_rows_too_and_the_smallest_cell_is_a_square() {
        let mut host = host_with(DOC);
        host.two_d = false;
        frame(&mut host);
        let before = place(&host, Hit::Cell(0, 1));
        let rem = Tokens::default().size.rem(1.0);
        drag(&mut host, Edge::Column(1), (80.0, 0.0));
        assert!((host.view_sizes().column("right") - (CELL.0 + 80.0 / rem)).abs() < 0.01);
        frame(&mut host);
        assert!((place(&host, Hit::Cell(0, 1)).w - (before.w + 80.0)).abs() < 0.5, "the cells follow");
        assert!((place(&host, Hit::Column(1)).w - (before.w + 86.0)).abs() < 0.5, "and the head");
        // as narrow as it goes: a square of a row's height as it comes
        drag(&mut host, Edge::Column(1), (-2000.0, 0.0));
        assert_eq!(host.view_sizes().column("right"), MIN);
        drag(&mut host, Edge::Row(0), (0.0, 40.0));
        assert!(host.view_sizes().row("dialog") > CELL.1);
        drag(&mut host, Edge::Heads, (60.0, 0.0));
        assert!(host.view_sizes().heads() > deck_show::sizes::HEADS);
        assert_eq!(host.view_sizes().column("left"), CELL.0, "the others stay");
    }

    #[test]
    fn a_double_click_on_an_edge_fits_the_content_and_the_sizes_are_kept_per_show_in_the_view_file() {
        let file = std::env::temp_dir().join(format!("deck-editor-views-{}", std::process::id())).join("views.json");
        let mut host = host_with(DOC);
        host.two_d = false;
        host.views_file = Some(file.clone());
        frame(&mut host);
        let r = place(&host, Hit::Edge(Edge::Column(1)));
        host.pointer(r.x + 1.0, r.y + 10.0);
        host.press(Hit::Edge(Edge::Column(1)));
        host.release();
        // the second press 100 ms after the first on the host's own clock, whatever the machine's load made of the real gap
        let first = host.last_edge.expect("the first press is kept").1;
        host.edge_press_at(Edge::Column(1), first + std::time::Duration::from_millis(100));
        host.release();
        let fitted = host.view_sizes().column("right");
        assert!(fitted > CELL.0, "the long scene name asks for more: {fitted}");
        assert!(host.status.starts_with("right fits its content"), "{}", host.status);
        // read back by another editor; another show has its own
        let mut other = host_with(DOC);
        other.keep_views_in(file.clone());
        assert_eq!(other.view_sizes().column("right"), fitted);
        assert!(!std::fs::read_to_string(&file).unwrap().contains("\"rows\""), "only what was changed is kept");
        other.sub = deck_show::Subtree::new(deck_show::parse(&DOC.replace(r#""id": "sizes""#, r#""id": "other""#)).unwrap(), deck_show::ClockChoice::Parent);
        assert_eq!(other.view_sizes().column("right"), CELL.0);
        std::fs::remove_dir_all(file.parent().unwrap()).ok();
    }

    #[test]
    fn a_rows_fire_tag_says_only_what_its_name_does_not_and_a_row_renamed_keeps_its_height() {
        let mut host = host_with(DOC);
        host.two_d = false;
        let texts = frame(&mut host);
        assert!(texts.iter().any(|t| t == "ch3"), "named after ch3.dialog: the chapter alone: {texts:?}");
        assert!(!texts.iter().any(|t| t.contains("· dialog")), "the row's name is not said twice: {texts:?}");
        assert!(texts.iter().any(|t| t.starts_with("ch3 · yel")), "another name: the whole event: {texts:?}");
        drag(&mut host, Edge::Row(0), (0.0, 30.0));
        let tall = host.view_sizes().row("dialog");
        host.press(Hit::Row(0));
        host.typing = Some((super::super::roles::Typing::RowName("dialog".into()), deck_kit::base::text::Field::with("dialogue")));
        host.type_key(&winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter));
        assert_eq!(host.shown().rows[0].id, "dialogue");
        assert_eq!(host.view_sizes().row("dialogue"), tall);
    }

    #[test]
    fn the_pointer_on_an_edge_shows_its_arrows_and_the_handle_with_the_size_while_dragged() {
        let mut host = host_with(DOC);
        host.two_d = false;
        frame(&mut host);
        let r = place(&host, Hit::Edge(Edge::Column(0)));
        let (x, y) = (r.x + r.w * 0.5, r.y + 30.0);
        host.pointer(x, y);
        assert_eq!(host.band_cursor(x, y), Some(winit::window::CursorIcon::EwResize));
        let row = place(&host, Hit::Edge(Edge::Row(0)));
        assert_eq!(host.band_cursor(row.x + 40.0, row.y + row.h * 0.5), Some(winit::window::CursorIcon::NsResize));
        host.press(Hit::Edge(Edge::Column(0)));
        host.pointer(x + 30.0, y);
        let texts = frame(&mut host);
        assert!(texts.iter().any(|t| t.starts_with("left · ") && t.ends_with(" px")), "the badge: {texts:?}");
        host.release();
        assert!(!frame(&mut host).iter().any(|t| t.ends_with(" px")));
    }
}
