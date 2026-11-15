//! A cell's values in the cell inspector (SHOW.md §5): the knobs of the cell's piece as Build's scrub numbers (deck-kit
//! `controls::scrub`, main.rs `scrub` / `fraction`), a turn let go written into the cell, a set value's × back to the
//! piece's own, "take current", the path warning's sentence. What a value may be, the knobs' words, what plays now and the
//! warning are deck-show's (values.rs); the knobs are the core's (deck-space `piece::knobs`). The loader here is glue: on a
//! thread, a published scene through the core's loader (deck-native `published::load`, kept in `<cache>/scenes/`), one
//! answer per piece kept for the session.
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_show::rows::{self, Edit};
use deck_show::values::{self, Knob};
use deck_show::Show;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

/// a press on a cell's values: a knob's scrub number (a drag turns it), a set value's ×, "take current"
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KnobHit {
    Turn(usize),
    Clear(usize),
    Take,
}

type Answer = Result<Arc<Vec<Knob>>, String>;

/// The knobs of the pieces the inspector shows, asked once each.
pub struct Loader {
    got: HashMap<String, Option<Answer>>,
    tx: Sender<(String, Answer)>,
    rx: Receiver<(String, Answer)>,
    base: String,
    cache: PathBuf,
}

impl Loader {
    pub fn new(base: String) -> Loader {
        let (tx, rx) = channel();
        Loader { got: HashMap::new(), tx, rx, base, cache: deck_native::cache::default_dir() }
    }

    /// `piece`'s knobs, or why not; none while they are on their way (the first ask starts it)
    pub fn get(&mut self, piece: &str) -> Option<Answer> {
        if let Some(a) = self.got.get(piece) {
            return a.clone();
        }
        self.got.insert(piece.to_string(), None);
        let (tx, piece, base, cache) = (self.tx.clone(), piece.to_string(), self.base.clone(), self.cache.clone());
        std::thread::spawn(move || {
            let answer = load(&piece, &base, &cache).map(Arc::new);
            let _ = tx.send((piece, answer));
        });
        None
    }

    pub fn poll(&mut self) {
        while let Ok((piece, a)) = self.rx.try_recv() {
            self.got.insert(piece, Some(a));
        }
    }

    #[cfg(test)]
    pub fn put(&mut self, piece: &str, knobs: Vec<Knob>) {
        self.got.insert(piece.to_string(), Some(Ok(Arc::new(knobs))));
    }
}

/// the core loads the piece as a cell would; a published scene comes through the core's loader, the API asked with the
/// editor's token
fn load(piece: &str, base: &str, cache: &std::path::Path) -> Result<Vec<Knob>, String> {
    deck_space::piece::knobs(piece, |place| {
        if place.kind != "scene" {
            return None;
        }
        let r = match deck_space::published::SceneRef::parse(piece)? {
            Ok(r) => r,
            Err(why) => return Some(deck_space::piece::Found::Missing(why)),
        };
        let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(8)).build();
        let got = deck_native::published::load(r, &cache.join("scenes"), |path| deck_native::published::get_with(&agent, base, path, super::api::token()));
        Some(match got.map_err(|e| format!("{e:#}")).and_then(|l| l.text()) {
            Ok(text) => deck_space::piece::Found::Scene(text),
            Err(why) => deck_space::piece::Found::Missing(why),
        })
    })
}

/// a knob turned by a drag: its index, where the drag began, the value then and now
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KnobDrag {
    pub i: usize,
    x0: f32,
    v0: f32,
    pub v: f32,
}

impl Host {
    /// the selected cell's row, column and piece with the piece's knobs; none (and why, in the status) while not known
    fn cell_knobs(&mut self) -> Option<(String, String, String, Arc<Vec<Knob>>)> {
        let (Some(row), Some(column)) = self.selected() else { return None };
        let piece = rows::cell_inspector(self.shown(), &row, &column)?.knobs_of?;
        match self.knobs.get(&piece) {
            Some(Ok(k)) => Some((row, column, piece, k)),
            Some(Err(why)) => {
                self.status = why;
                None
            }
            None => {
                self.status = values::LOADING.into();
                None
            }
        }
    }

    pub(super) fn knob_press(&mut self, h: KnobHit) {
        let Some((row, column, piece, knobs)) = self.cell_knobs() else { return };
        let views = rows::cell_knobs(self.shown(), &row, &column, &knobs, "en");
        match h {
            KnobHit::Turn(i) => {
                if let Some(k) = views.get(i) {
                    self.knob_drag = Some(KnobDrag { i, x0: self.cursor.0, v0: k.value, v: k.value });
                }
            }
            KnobHit::Clear(i) => {
                if let Some(k) = views.get(i) {
                    self.edit_rows_with(&Edit::Value { row, column, path: k.path.clone(), to: None }, Some(&knobs));
                }
            }
            KnobHit::Take => match values::current(&self.sub, &column, &piece, &knobs) {
                Ok(positions) => {
                    self.edit_rows_with(&Edit::Take { row, column, positions }, Some(&knobs));
                }
                Err(why) => self.status = why,
            },
        }
    }

    /// the pointer moved: a knob being dragged turns (shown at once, written when let go)
    pub(super) fn knob_moved(&mut self, x: f32) {
        let Some(d) = self.knob_drag else { return };
        let Some((row, column, _, knobs)) = self.cell_knobs() else { return };
        if let Some(k) = rows::cell_knobs(self.shown(), &row, &column, &knobs, "en").get(d.i) {
            self.knob_drag = Some(KnobDrag { v: crate::scrub(d.v0, x - d.x0, k.min, k.max, k.log), ..d });
        }
    }

    /// The button let go: a knob dragged to a new value is written into the cell (deck-show checks it).
    pub fn release(&mut self) {
        let Some(d) = self.knob_drag.take() else { return };
        if d.v == d.v0 {
            return;
        }
        let Some((row, column, _, knobs)) = self.cell_knobs() else { return };
        if let Some(k) = rows::cell_knobs(self.shown(), &row, &column, &knobs, "en").get(d.i) {
            self.edit_rows_with(&Edit::Value { row, column, path: k.path.clone(), to: Some(d.v) }, Some(&knobs));
        }
    }

    /// the wheel over a knob turns it a step, written at once
    pub(super) fn knob_wheel(&mut self, i: usize, steps: f32) {
        let Some((row, column, _, knobs)) = self.cell_knobs() else { return };
        if let Some(k) = rows::cell_knobs(self.shown(), &row, &column, &knobs, "en").get(i) {
            let v = crate::scrub(k.value, steps * 12.0, k.min, k.max, k.log);
            self.edit_rows_with(&Edit::Value { row, column, path: k.path.clone(), to: Some(v) }, Some(&knobs));
        }
    }
}

/// `text` in lines that fit `w`
fn wrap(d: &DrawList, text: &str, px: f32, w: f32) -> Vec<String> {
    let n = (w / d.advance(px)).max(8.0) as usize;
    let mut lines: Vec<String> = vec![];
    for word in text.split(' ') {
        match lines.last_mut() {
            Some(l) if l.chars().count() + 1 + word.chars().count() <= n => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// The cell's knobs under its piece field (`piece`'s, the cell's values over its own), "take current", and the path
/// warning when the cell has one. The height used.
pub fn side(d: &mut DrawList, t: &Tokens, host: &mut Host, (x, y, w): (f32, f32, f32), show: &Show, (row, column): (&str, &str), (piece, warning): (&str, Option<&str>)) -> f32 {
    let (pad, font, small, h) = (t.size.pad, t.size.font, t.size.small, t.size.row);
    let top = y;
    let mut y = y + controls::section(d, t, x, y, w, &values::KNOBS.to_uppercase(), false);
    match host.knobs.get(piece) {
        None => {
            d.text(x, y + 2.0, small, values::LOADING, t.color.muted);
            y += small * 1.8;
        }
        Some(Err(why)) => {
            for l in wrap(d, &why, small, w) {
                d.text(x, y + 2.0, small, &l, t.color.warn);
                y += small * 1.5;
            }
        }
        Some(Ok(knobs)) => {
            let clear_w = h;
            for (i, k) in rows::cell_knobs(show, row, column, &knobs, "en").iter().enumerate() {
                let dragged = host.knob_drag.filter(|g| g.i == i);
                let v = dragged.map_or(k.value, |g| g.v);
                let r = Rect { x, y, w: w - clear_w - 4.0, h };
                let bar = controls::scrub(d, t, r, &k.label, &crate::number(v), crate::fraction(v, k.min, k.max, k.log), dragged.is_some() || k.set);
                host.hits.push((bar, Hit::Knob(KnobHit::Turn(i))));
                if k.set {
                    let c = Rect { x: x + w - clear_w, y: y + 1.0, w: clear_w, h: h - 3.0 };
                    d.text(c.x + (c.w - d.advance(font)) * 0.5, y + (h - font * 1.25) * 0.5, font, "×", t.color.muted);
                    host.hits.push((c, Hit::Knob(KnobHit::Clear(i))));
                }
                y += h;
            }
            let bw = d.advance(small) * values::TAKE.chars().count() as f32 + 2.0 * pad;
            let b = Rect { x, y: y + 2.0, w: bw, h: h - 4.0 };
            super::perform::button(d, t, b, values::TAKE, t.color.control, t.color.text);
            host.hits.push((b, Hit::Knob(KnobHit::Take)));
            y += h;
        }
    }
    if let Some(warn) = warning {
        for l in wrap(d, warn, small, w) {
            d.text(x, y + 2.0, small, &l, t.color.warn);
            y += small * 1.5;
        }
    }
    y + pad * 0.5 - top
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{host_with, press};
    use super::super::Hit;
    use super::KnobHit;
    use deck_show::values::Knob;

    fn nave() -> Vec<Knob> {
        let k = |path: &str, label: &str, value: f32| Knob { path: path.into(), label: [("en".to_string(), label.to_string())].into(), min: 0.0, max: 1.0, value, log: false };
        vec![k("light.gain", "Light", 0.3), k("dim.gain", "Dim", 0.6)]
    }

    fn value(host: &super::Host, key: &str) -> Option<f64> {
        match host.shown().row("dialog")?.cells.get("left")? {
            deck_show::Cell::Values { values, .. } => values.get(key)?.as_f64(),
            _ => None,
        }
    }

    /// the cell inspector's knobs as the operator uses them: a drag let go writes the value, × gives it back to the piece,
    /// "take current" copies what plays
    #[test]
    fn a_cells_knobs_are_turned_cleared_and_taken_from_what_plays() {
        let mut host = host_with(include_str!("../../../show/fixtures/top-toy.show.json"));
        host.row = 1;
        let left = deck_show::view::plan(host.shown()).columns.iter().position(|c| c.id == "left").unwrap();
        press(&mut host, false, |h| *h == Hit::Cell(1, left));
        // the knobs not known yet: the inspector says so, a press on nothing happens
        host.knobs.put("scene:nave", nave());
        press(&mut host, false, |h| *h == Hit::Knob(KnobHit::Turn(0)));
        assert!(host.knob_drag.is_some(), "a press on light starts its drag");
        host.pointer(crate::SCRUB_PX * 0.5, 0.0);
        assert!((host.knob_drag.unwrap().v - 0.8).abs() < 1e-4, "half the travel: 0.3 → 0.8");
        host.release();
        assert!(host.status.starts_with("row 2 dialog, left: light.gain 0.8"), "{}", host.status);
        assert!((value(&host, "light.gain").unwrap() - 0.8).abs() < 1e-6);
        // dim is set by the cell (`dim 0.5`): its × gives it back to the piece
        press(&mut host, false, |h| *h == Hit::Knob(KnobHit::Clear(1)));
        assert_eq!(host.status, "row 2 dialog, left: dim.gain is the piece's own again");
        assert_eq!(value(&host, "dim"), None);
        // take current: nothing plays yet, then opening fired (light 0.2, dim the piece's own)
        press(&mut host, false, |h| *h == Hit::Knob(KnobHit::Take));
        assert!(host.status.starts_with("nothing plays on left"), "{}", host.status);
        host.sub.fire("opening", 1000);
        press(&mut host, false, |h| *h == Hit::Knob(KnobHit::Take));
        assert_eq!(host.status, "row 2 dialog, left: took 2 knobs – values · dim.gain 0.6 · light.gain 0.2");
        // the wheel over a knob turns it a step
        let (r, _) = *host.hits.iter().find(|(_, h)| *h == Hit::Knob(KnobHit::Turn(1))).unwrap();
        assert!(host.wheel(r.x + 4.0, r.y + 4.0, 2.0));
        assert!((value(&host, "dim.gain").unwrap() - 0.7).abs() < 1e-6, "{}", host.status);
    }

    /// a values cell on another piece: the inspector carries the path warning
    #[test]
    fn the_inspector_warns_where_a_values_cell_meets_another_piece() {
        let mut host = host_with(include_str!("../../../show/fixtures/top-toy.show.json"));
        host.row = 1;
        let left = deck_show::view::plan(host.shown()).columns.iter().position(|c| c.id == "left").unwrap();
        press(&mut host, false, |h| *h == Hit::Cell(1, left));
        host.edit_rows(&deck_show::rows::Edit::Piece { row: "dialog".into(), column: "left".into(), piece: "scene:vigil".into() });
        let i = deck_show::rows::cell_inspector(host.shown(), "dialog", "left").unwrap();
        assert_eq!(i.knobs_of.as_deref(), Some("scene:vigil"));
        assert!(i.warning.unwrap().contains("scene:nave plays there"));
        host.knobs.put("scene:vigil", nave());
        press(&mut host, false, |h| *h == Hit::Knob(KnobHit::Take));
        assert!(host.status.contains("plays scene:nave, not scene:vigil") || host.status.starts_with("nothing plays"), "{}", host.status);
    }
}
