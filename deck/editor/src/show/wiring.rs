//! Cells wired to cells in the rows band (Show): a cell's typed ports sit on its foot – inlets from the left, outlets from
//! the right – and show on the selected or hovered cell, at a wire's ends and, while a wire is in the hand, on every inlet
//! it may go into. A drag from an outlet to an inlet wires them; a wired inlet dragged lifts its wire (let go on nothing:
//! it goes). A busy row hangs the wires between two cells as one bundle with its count (deck-show wiring.rs `simplified`).
//! Ports, signals, colours and the wire are the one grammar of deck-kit (signal.rs, wire.rs) and deck-show (wiring.rs).
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::signal::Signal;
use deck_kit::base::tokens::Tokens;
use deck_kit::base::wire::Wire;
use deck_kit::editor::controls::Rect;
use deck_show::wiring::{self, CellWire, Shown};
use deck_show::Show;

/// a wire in the hand: from `from`'s outlet `out` (its signal), lifted off an inlet or new
#[derive(Debug, Clone, PartialEq)]
pub struct Wiring {
    pub row: usize,
    pub from: String,
    pub out: String,
    pub signal: Signal,
    pub lifted: Option<CellWire>,
}

/// One row of the band as laid: its index, its id, each column's cell box (none: kept, empty or inside a question).
pub struct RowLay {
    pub i: usize,
    pub id: String,
    pub cells: Vec<Option<Rect>>,
}

/// where port `k` of `n` sits on the foot of `cell`: inlets from the left, outlets from the right
fn port_at(t: &Tokens, cell: &Rect, out: bool, k: usize, n: usize) -> (f32, f32) {
    let (pad, gap) = (t.size.pad, t.size.rem(0.85));
    let y = cell.y + cell.h;
    if out {
        (cell.x + cell.w - pad - (n - 1 - k) as f32 * gap, y)
    } else {
        (cell.x + pad + k as f32 * gap, y)
    }
}

/// a port to draw: where, its signal, outlet, lit as a place the wire in the hand may go, its place
type ShownPin = ((f32, f32), Signal, bool, bool, Hit);

/// The wires and ports of the laid rows, over the cells; their ports' places go to the host's hits.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, (show, columns): (&Show, &[String]), rows: &[RowLay], (region, pointer): (&Rect, (f32, f32))) {
    let unit = t.size.rem(1.0);
    let r = t.size.rem(0.3);
    let held = host.wiring.clone();
    let mut pins: Vec<ShownPin> = Vec::new();
    for lay in rows {
        let Some(row) = show.rows.iter().find(|x| x.id == lay.id) else { continue };
        let at = |id: &str| columns.iter().position(|c| c == id);
        let rect = |c: usize| lay.cells.get(c).copied().flatten();
        let ports: Vec<_> = columns.iter().map(|c| row.cells.get(c).map(|cell| wiring::ports(show, c, cell)).unwrap_or_default()).collect();
        let selected: Vec<&str> = if host.row == lay.i { host.cells.iter().map(String::as_str).collect() } else { Vec::new() };
        let hovered = (0..columns.len()).find(|c| rect(*c).is_some_and(|b| b.has(pointer.0, pointer.1)));
        let mut focus = selected;
        if let Some(c) = hovered {
            focus.push(columns[c].as_str());
        }
        let placed = wiring::placed(show, row);
        let shown_pin = |c: usize, out: bool, k: usize, lit: bool, pins: &mut Vec<_>| {
            let (Some(b), Some((ins, outs))) = (rect(c), ports.get(c)) else { return };
            let list = if out { outs } else { ins };
            let Some(p) = list.get(k) else { return };
            let hit = Hit::Port(lay.i, c, out, k);
            if !pins.iter().any(|x: &ShownPin| x.4 == hit) {
                pins.push((port_at(t, &b, out, k, list.len()), p.signal, out, lit, hit));
            }
        };
        // the focused cells' own ports, ready to wire
        for f in &focus {
            let Some(c) = at(f) else { continue };
            let (ins, outs) = &ports[c];
            for k in (0..ins.len()).filter(|k| wiring::wireable(&ins[*k])) {
                shown_pin(c, false, k, false, &mut pins);
            }
            for k in 0..outs.len() {
                shown_pin(c, true, k, false, &mut pins);
            }
        }
        // a wire in the hand: every inlet of the row it may go into
        if let Some(w) = held.as_ref().filter(|w| w.row == lay.i) {
            for (c, (ins, _)) in ports.iter().enumerate() {
                if columns[c] == w.from {
                    continue;
                }
                for k in (0..ins.len()).filter(|k| wiring::wireable(&ins[*k]) && w.signal.feeds(ins[*k].signal)) {
                    shown_pin(c, false, k, true, &mut pins);
                }
            }
        }
        for s in wiring::simplified(&placed, columns, &focus) {
            match s {
                Shown::One(n) => {
                    let p = &placed[n];
                    if held.as_ref().is_some_and(|h| h.lifted.as_ref() == Some(&p.wire)) {
                        continue;
                    }
                    let (Some(a), Some(b)) = (at(&p.wire.from), at(&p.wire.to)) else { continue };
                    let (Some(ra), Some(rb)) = (rect(a), rect(b)) else { continue };
                    let from = port_at(t, &ra, true, p.out_at, ports[a].1.len());
                    let to = port_at(t, &rb, false, p.in_at, ports[b].0.len());
                    let lit = focus.contains(&p.wire.from.as_str()) || focus.contains(&p.wire.to.as_str());
                    let c = t.color.signals.of(p.signal);
                    let c = if lit || placed.len() < wiring::BUSY { c } else { fade(c, 0.6) };
                    hang(d, (from, to), (unit, host.flow), 1.4 * p.signal.weight(), c);
                    shown_pin(a, true, p.out_at, false, &mut pins);
                    shown_pin(b, false, p.in_at, false, &mut pins);
                }
                Shown::Bundle { a, b, wires, signal } => {
                    let (Some(ra), Some(rb)) = (at(&a).and_then(rect), at(&b).and_then(rect)) else { continue };
                    let foot = |r: Rect| (r.x + r.w * 0.5, r.y + r.h);
                    let c = signal.map_or(t.color.muted, |s| t.color.signals.of(s));
                    let w = Wire::hang(foot(ra), foot(rb), unit);
                    d.wire(&w, 1.4 + 0.4 * (wires.len().min(5) as f32), fade(c, 0.75), &[]);
                    for p in [foot(ra), foot(rb)] {
                        d.port(p, r, c, t.color.surface);
                    }
                    let (x, y) = w.lowest();
                    let text = format!("{}", wires.len());
                    let tw = d.sans_width(t.size.small, &text) + t.size.rem(0.6);
                    let h = t.size.small * 1.5;
                    d.round_rect(x - tw * 0.5, y - h * 0.5, tw, h, h * 0.5, t.color.surface);
                    d.outline(x - tw * 0.5, y - h * 0.5, tw, h, (h * 0.5, 1.0), fade(c, 0.8));
                    d.text(x - tw * 0.5 + t.size.rem(0.3), y - t.size.small * 0.62, t.size.small, &text, t.color.text);
                }
            }
        }
        // the wire in the hand follows the pointer from its outlet
        if let Some(w) = held.as_ref().filter(|w| w.row == lay.i) {
            if let Some(c) = at(&w.from) {
                if let (Some(b), Some(k)) = (rect(c), ports[c].1.iter().position(|p| p.label == w.out)) {
                    let from = port_at(t, &b, true, k, ports[c].1.len());
                    hang(d, (from, host.cursor), (unit, None), 1.4 * w.signal.weight(), t.color.signals.of(w.signal));
                    shown_pin(c, true, k, false, &mut pins);
                }
            }
        }
    }
    for (p, signal, out, lit, hit) in pins {
        let over = Rect { x: p.0 - r * 2.0, y: p.1 - r * 2.0, w: r * 4.0, h: r * 4.0 };
        let hot = over.has(pointer.0, pointer.1);
        let c = t.color.signals.of(signal);
        if lit || hot {
            d.disc(p.0, p.1, r * 2.0, fade(c, 0.22));
        }
        let ground = if out { t.color.surface } else { t.color.control };
        d.pin(p, if hot { r * 1.3 } else { r }, (signal.shape(), (0.0, 1.0)), c, ground);
        if let Some(over) = deck_kit::editor::table::Laid::shown(&over, region) {
            host.hits.push((over, hit));
        }
    }
}

fn fade(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

/// one hanging wire with its beads (none while the flow is off)
fn hang(d: &mut DrawList, (a, b): ((f32, f32), (f32, f32)), (unit, flow): (f32, Option<f32>), thick: f32, color: [f32; 4]) {
    let w = Wire::hang(a, b, unit);
    let beads = flow.map(|s| w.flowing(unit, s)).unwrap_or_default();
    d.wire(&w, thick, color, &beads);
}

impl Host {
    /// a port pressed: an outlet takes a new wire in the hand, a wired inlet lifts its wire
    pub(super) fn port_press(&mut self, row: usize, col: usize, out: bool, k: usize) {
        let show = self.shown().clone();
        let m = deck_show::view::matrix(&self.sub);
        let (Some(r), Some(column)) = (show.rows.get(row), m.columns.get(col).map(|c| c.id.clone())) else { return };
        let Some(cell) = r.cells.get(&column) else { return };
        let (ins, outs) = wiring::ports(&show, &column, cell);
        self.row = row;
        if out {
            let Some(p) = outs.get(k) else { return };
            self.wiring = Some(Wiring { row, from: column, out: p.label.clone(), signal: p.signal, lifted: None });
            return;
        }
        let Some(p) = ins.get(k) else { return };
        let Some(w) = r.wires.iter().rev().find(|w| w.to == column && w.into == p.label).cloned() else {
            self.status = format!("{column} {}: drag from an outlet into it to wire it", p.label);
            return;
        };
        let signal = wiring::ports(&show, &w.from, r.cells.get(&w.from).unwrap_or(&deck_show::Cell::Keep)).1.iter().find(|o| o.label == w.out).map_or(p.signal, |o| o.signal);
        self.wiring = Some(Wiring { row, from: w.from.clone(), out: w.out.clone(), signal, lifted: Some(w) });
    }

    /// The button let go with a wire in the hand: on an inlet it may go into it is wired (deck-show checks it); a lifted
    /// wire let go elsewhere goes.
    pub(super) fn wire_release(&mut self) {
        let Some(w) = self.wiring.take() else { return };
        let row = match self.shown().rows.get(w.row) {
            Some(r) => r.id.clone(),
            None => return,
        };
        let (x, y) = self.cursor;
        let m = deck_show::view::matrix(&self.sub);
        let into = self.hits.iter().rev().find(|(r, _)| r.has(x, y)).and_then(|(_, h)| match *h {
            Hit::Port(i, c, false, k) if i == w.row => Some((c, k)),
            _ => None,
        });
        let target = into.and_then(|(c, k)| {
            let column = m.columns.get(c)?.id.clone();
            let cell = self.shown().rows.get(w.row)?.cells.get(&column)?.clone();
            let pin = wiring::ports(self.shown(), &column, &cell).0.get(k)?.clone();
            Some(CellWire { from: w.from.clone(), out: w.out.clone(), to: column, into: pin.label })
        });
        match (target, w.lifted) {
            (Some(t), lifted) if lifted.as_ref() != Some(&t) => {
                self.edit_rows(&deck_show::rows::Edit::Wire { row, wire: t, replaces: lifted });
            }
            (Some(_), _) => {}
            (None, Some(lifted)) => {
                self.edit_rows(&deck_show::rows::Edit::Unwire { row, wire: lifted });
            }
            (None, None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{host_with, H, W};
    use super::super::{author, Hit, Host};
    use deck_kit::base::draw::DrawList;
    use deck_kit::base::tokens::Tokens;
    use deck_show::wiring::CellWire;

    const SHOW: &str = r#"{ "id": "wired", "version": 1, "toons": ["catpope"],
        "outlets": [{ "id": "wall" }, { "id": "floor" }, { "id": "sky" }],
        "rows": [{ "id": "one", "cells": {
            "wall": { "mode": "play", "piece": "scene:keys", "values": { "speed": 0.5 } },
            "floor": { "mode": "play", "piece": "scene:bells", "values": { "glow": 1.0 } },
            "sky": { "mode": "play", "piece": "scene:keys" } } }] }"#;

    fn frame(host: &mut Host) -> Vec<String> {
        let (mut d, t) = (DrawList::new(0.6), Tokens::new(deck_kit::base::tokens::Theme::Night, 16.0));
        host.hits.clear();
        author::draw(&mut d, &t, host, W, H, (-10.0, -10.0));
        d.texts().into_iter().map(str::to_string).collect()
    }

    fn port(host: &Host, want: Hit) -> Option<(f32, f32)> {
        host.hits.iter().rev().find(|(_, h)| *h == want).map(|(r, _)| (r.x + r.w * 0.5, r.y + r.h * 0.5))
    }

    fn wires(host: &Host) -> Vec<CellWire> {
        host.shown().rows[0].wires.clone()
    }

    #[test]
    fn a_selected_cells_ports_show_and_a_drag_from_its_outlet_into_an_inlet_wires_them() {
        let mut host = host_with(SHOW);
        frame(&mut host);
        assert!(!host.hits.iter().any(|(_, h)| matches!(h, Hit::Port(..))), "no ports while no cell is in focus");
        host.cells = vec!["wall".into()];
        frame(&mut host);
        // wall's outlets: plays, ends
        assert!(port(&host, Hit::Port(0, 0, true, 1)).is_some());
        host.press(Hit::Port(0, 0, true, 1));
        assert!(host.wiring.is_some());
        frame(&mut host);
        // in the hand: floor's and sky's fire inlets show, not floor's value (an event feeds no control)
        assert!(port(&host, Hit::Port(0, 1, false, 0)).is_some() && port(&host, Hit::Port(0, 2, false, 0)).is_some());
        assert!(port(&host, Hit::Port(0, 1, false, 1)).is_none());
        host.cursor = port(&host, Hit::Port(0, 1, false, 0)).unwrap();
        host.wire_release();
        assert_eq!(wires(&host), [CellWire::new("wall", "ends", "floor", "fire")], "{}", host.status);
    }

    #[test]
    fn a_wired_inlet_dragged_off_onto_another_rewires_and_into_nothing_unwires() {
        let mut host = host_with(SHOW);
        let mut s = host.shown().clone();
        s.rows[0].wires.push(CellWire::new("wall", "ends", "floor", "fire"));
        host.changed(s);
        frame(&mut host);
        assert!(port(&host, Hit::Port(0, 1, false, 0)).is_some(), "a wire's ends show");
        host.press(Hit::Port(0, 1, false, 0));
        assert!(host.wiring.as_ref().is_some_and(|w| w.lifted.is_some()));
        frame(&mut host);
        host.cursor = port(&host, Hit::Port(0, 2, false, 0)).expect("sky's fire takes it");
        host.wire_release();
        assert_eq!(wires(&host), [CellWire::new("wall", "ends", "sky", "fire")], "{}", host.status);
        frame(&mut host);
        host.press(Hit::Port(0, 2, false, 0));
        host.cursor = (-50.0, -50.0);
        host.wire_release();
        assert!(wires(&host).is_empty(), "{}", host.status);
    }

    #[test]
    fn a_busy_row_hangs_one_bundle_with_its_count_until_a_cell_is_in_focus() {
        let mut host = host_with(SHOW);
        let mut s = host.shown().clone();
        s.rows[0].wires = vec![CellWire::new("wall", "ends", "floor", "fire"), CellWire::new("floor", "ends", "wall", "fire"), CellWire::new("sky", "ends", "wall", "fire")];
        host.changed(s);
        assert_eq!(wires(&host).len(), 3);
        let quiet = frame(&mut host);
        assert_eq!(quiet.iter().filter(|t| *t == "2").count(), 1, "wall and floor folded to a count");
        host.cells = vec!["floor".into()];
        let open = frame(&mut host);
        assert_eq!(open.iter().filter(|t| *t == "2").count(), 0, "the selected cell's wires on their own");
    }
}
