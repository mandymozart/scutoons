//! What a module with nothing to see drives: a light controller, the `fixture` – each control output it writes
//! as a lamp lit by its value, the value, and its trace over the last seconds. The numbers are the core's (the stage's
//! control outputs of the auditioned node); this only keeps what it read for the trace.
use crate::stage::Stage;
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::Rect;
use deck_space::graph::PortType;
use std::collections::VecDeque;

/// how many frames a trace holds (≈ 4 s at 60 fps)
const KEEP: usize = 240;

/// one output's trace
#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    pub name: String,
    /// its index among the node's outputs
    port: usize,
    pub values: VecDeque<f32>,
}

impl Trace {
    pub fn now(&self) -> f32 {
        self.values.back().copied().unwrap_or(0.0)
    }

    /// the range the trace is drawn in: what it held, and at least 0..1
    pub fn range(&self) -> (f32, f32) {
        self.values.iter().fold((0.0f32, 1.0f32), |(lo, hi), &v| (lo.min(v), hi.max(v)))
    }

    /// how lit the lamp is: the value inside the range, 0..1
    pub fn lit(&self) -> f32 {
        let (lo, hi) = self.range();
        ((self.now() - lo) / (hi - lo).max(1e-6)).clamp(0.0, 1.0)
    }

    fn push(&mut self, v: f32) {
        if self.values.len() == KEEP {
            self.values.pop_front();
        }
        self.values.push_back(if v.is_finite() { v } else { 0.0 });
    }
}

/// what a module drives, read each frame while it is heard
#[derive(Debug, Default)]
pub struct Lamps {
    pub traces: Vec<Trace>,
    node: Option<usize>,
}

impl Lamps {
    /// The heard node `node`'s control outputs this frame onto their traces; a new node starts them again. None: not
    /// heard, the traces go.
    pub fn read(&mut self, stage: &Stage, node: Option<usize>) {
        let Some(n) = node else {
            self.traces.clear();
            self.node = None;
            return;
        };
        if self.node != Some(n) {
            let Some(kind) = stage.kinds.get(n) else { return };
            self.traces = kind.outputs().iter().enumerate().filter(|(_, p)| p.ty == PortType::Control).map(|(port, p)| Trace { name: p.name.to_string(), port, values: VecDeque::with_capacity(KEEP) }).collect();
            self.node = Some(n);
        }
        let outs = stage.controls.outputs(n);
        for t in &mut self.traces {
            t.push(outs.get(t.port).copied().unwrap_or(0.0));
        }
    }
}

/// The panel's body: one row per output – its lamp, its name and value, its trace.
pub fn draw(d: &mut DrawList, t: &Tokens, lamps: &Lamps, heard: bool, a: Rect) {
    let (font, small, row) = (t.size.font, t.size.small, t.size.row * 1.4);
    let mut y = a.y;
    if !heard {
        d.text(a.x, y + 4.0, small, "Build it: it plays on the stage, its outputs light here.", t.color.muted);
        return;
    }
    if lamps.traces.is_empty() {
        d.text(a.x, y + 4.0, small, "It drives no control output.", t.color.muted);
        return;
    }
    for tr in &lamps.traces {
        let r = row * 0.36;
        let (cx, cy) = (a.x + r + 2.0, y + row * 0.5);
        let lit = tr.lit();
        let [cr, cg, cb, _] = t.color.warn;
        d.disc(cx, cy, r + 3.0, [cr, cg, cb, 0.18 * lit]);
        d.disc(cx, cy, r, [cr * (0.25 + 0.75 * lit), cg * (0.25 + 0.75 * lit), cb * (0.25 + 0.75 * lit), 1.0]);
        let x = cx + r + 10.0;
        d.text(x, y + 2.0, font, &tr.name, t.color.text);
        d.text(x, y + 2.0 + font * 1.2, small, &format!("{:.3}", tr.now()), t.color.muted);
        let gx = x + 90.0;
        let g = Rect { x: gx, y: y + 3.0, w: (a.x + a.w - gx).max(10.0), h: row - 6.0 };
        trace(d, t, tr, g);
        y += row;
    }
}

fn trace(d: &mut DrawList, t: &Tokens, tr: &Trace, g: Rect) {
    d.rect(g.x, g.y, g.w, g.h, t.color.control);
    let (lo, hi) = tr.range();
    let n = tr.values.len();
    if n < 2 {
        return;
    }
    let px = |i: usize, v: f32| (g.x + i as f32 / (KEEP - 1) as f32 * g.w, g.y + g.h - (v - lo) / (hi - lo).max(1e-6) * g.h);
    let start = KEEP - n;
    for (i, w) in tr.values.iter().collect::<Vec<_>>().windows(2).enumerate() {
        d.line(px(start + i, *w[0]), px(start + i + 1, *w[1]), 1.0, t.color.warn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn traced(values: &[f32]) -> Trace {
        let mut t = Trace { name: "level".into(), port: 0, values: VecDeque::new() };
        for &v in values {
            t.push(v);
        }
        t
    }

    #[test]
    fn a_lamp_is_lit_by_its_value_inside_what_it_held() {
        assert_eq!(traced(&[0.0, 0.5]).lit(), 0.5);
        let t = traced(&[0.0, 4.0, 2.0]);
        assert_eq!((t.range(), t.lit()), ((0.0, 4.0), 0.5), "a value past 1 widens the range");
        assert_eq!(traced(&[]).lit(), 0.0);
        assert_eq!(traced(&[f32::NAN]).now(), 0.0);
    }

    #[test]
    fn a_trace_keeps_the_last_seconds() {
        let t = traced(&vec![1.0; KEEP + 10]);
        assert_eq!(t.values.len(), KEEP);
    }
}
