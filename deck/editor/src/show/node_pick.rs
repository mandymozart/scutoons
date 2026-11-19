//! The node view's picking: a drag on the empty canvas draws a box that selects the nodes it touches (shift adds to the
//! selection), a click on a wire pins its value beside it and a click again lets it go, and a node focused or selected
//! asks for its tone (deck-show focus.rs; main.rs plays it when the settings allow).
use super::nodes::NodeView;
use deck_kit::base::draw::DrawList;
use deck_kit::base::graph::Graph;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::Rect;

/// how far the pointer goes before a press on the canvas is a drag, px
pub const DRAG: f32 = 4.0;

/// a box being drawn: where it started and where the pointer is, whether it adds, what was selected before it, the
/// wire under the press
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Boxing {
    pub from: (f32, f32),
    pub to: (f32, f32),
    add: bool,
    before: Vec<String>,
    wire: Option<String>,
}

impl Boxing {
    pub fn rect(&self) -> Rect {
        let (x, y) = (self.from.0.min(self.to.0), self.from.1.min(self.to.1));
        Rect { x, y, w: (self.from.0 - self.to.0).abs(), h: (self.from.1 - self.to.1).abs() }
    }

    fn dragged(&self) -> bool {
        (self.to.0 - self.from.0).hypot(self.to.1 - self.from.1) >= DRAG
    }
}

/// a wire's key, kept while the graph is laid out again: its ends by node id and pin; None for a wire without a value
pub fn wire_key(g: &Graph, i: usize) -> Option<String> {
    let l = g.links.get(i).filter(|l| l.value.is_some())?;
    let (a, b) = (g.nodes.get(l.from.0)?, g.nodes.get(l.to.0)?);
    Some(format!("{}.{}>{}.{}", a.id, l.from.1, b.id, l.to.1))
}

fn meets(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

impl NodeView {
    /// a press on the empty canvas at `at`: a box starts (a click without a drag is decided on release)
    pub(super) fn box_start(&mut self, at: (f32, f32)) {
        let wire = self.hover.as_deref().and_then(|h| h.strip_prefix("wire:")).and_then(|i| i.parse::<usize>().ok()).and_then(|i| self.wire_keys.get(i).cloned().flatten());
        self.boxing = Some(Boxing { from: at, to: at, add: self.shift, before: self.selected.clone(), wire });
    }

    /// the pointer moved with the box in the hand: what it touches is selected as it grows
    pub(super) fn box_move(&mut self, at: (f32, f32)) {
        let Some(b) = self.boxing.as_mut() else { return };
        b.to = at;
        if !b.dragged() {
            return;
        }
        let (r, add, before) = (b.rect(), b.add, b.before.clone());
        let inside: Vec<String> = self.drawn.iter().filter(|(_, n)| meets(self.on_screen(*n), r)).map(|(id, _)| id.clone()).collect();
        let mut picked = if add { before } else { Vec::new() };
        for id in inside {
            if !picked.contains(&id) {
                picked.push(id);
            }
        }
        self.selected = picked;
    }

    /// the button let go: a drag keeps what the box selected and focuses the first new node in reading order; a click
    /// pins or unpins the wire under it, else lets the selection go (unless shift)
    pub(super) fn box_end(&mut self) {
        let Some(b) = self.boxing.take() else { return };
        if b.dragged() {
            let new = self.order.iter().filter_map(|&i| self.drawn.get(i)).map(|(id, _)| id).find(|id| self.selected.contains(id) && !b.before.contains(id)).cloned();
            if let Some(id) = new {
                self.flash = Some((id.clone(), self.secs));
                self.ask_tone(&id);
                self.focus = Some(id);
            }
            return;
        }
        match b.wire {
            Some(key) => match self.pinned.iter().position(|k| *k == key) {
                Some(k) => {
                    self.pinned.remove(k);
                }
                None => self.pinned.push(key),
            },
            None if !b.add => {
                self.selected.clear();
                self.focus = None;
            }
            None => {}
        }
    }

    /// a node's place on the canvas, on screen
    pub(super) fn on_screen(&self, r: Rect) -> Rect {
        let (o, e) = (self.view.to_screen((r.x, r.y)), self.view.to_screen((r.x + r.w, r.y + r.h)));
        Rect { x: self.canvas.x + o.0, y: self.canvas.y + o.1, w: e.0 - o.0, h: e.1 - o.1 }
    }

    /// the node `id` was focused or selected: its tone is wanted
    pub(super) fn ask_tone(&mut self, id: &str) {
        if let Some(hz) = self.drawn.iter().position(|(x, _)| x == id).and_then(|i| self.tones.get(i).copied()) {
            self.tone = Some(hz);
        }
    }

    /// the tone asked for since the last call, Hz
    pub fn take_tone(&mut self) -> Option<f32> {
        self.tone.take()
    }
}

impl super::Host {
    /// a check run: a box from and to these fractions of the canvas, held; the `pin`-th wire with a value pinned
    pub(super) fn script_pick(&mut self, frac: Option<[f32; 4]>, pin: Option<usize>) {
        let c = self.nodes.canvas;
        let at = |fx: f32, fy: f32| (c.x + c.w * fx, c.y + c.h * fy);
        if let Some(k) = pin {
            let wire = self.nodes.wire_keys.iter().enumerate().filter(|(_, w)| w.is_some()).nth(k).map(|(i, _)| i);
            if let Some(mid) = wire.and_then(|i| self.nodes.wire_mids[i].map(|m| (i, m))) {
                self.nodes.hover = Some(format!("wire:{}", mid.0));
                self.cursor = mid.1;
                self.press(super::Hit::Nodes(super::nodes::NodeHit::Canvas));
                self.release();
            }
        }
        if let Some([x0, y0, x1, y1]) = frac {
            self.cursor = at(x0, y0);
            self.press(super::Hit::Nodes(super::nodes::NodeHit::Canvas));
            let to = at(x1, y1);
            self.pointer(to.0, to.1);
        }
    }
}

/// the box as it is drawn: a faint fill and a thin edge in the selection's colour
pub fn draw_box(d: &mut DrawList, t: &Tokens, b: &Boxing) {
    let r = b.rect();
    if !b.dragged() {
        return;
    }
    let c = t.color.selection;
    d.rect(r.x, r.y, r.w, r.h, [c[0], c[1], c[2], c[3] * 0.18]);
    d.outline(r.x, r.y, r.w, r.h, (0.0, 1.0), [c[0], c[1], c[2], c[3] * 0.9]);
}

#[cfg(test)]
mod tests {
    use super::super::author;
    use super::super::nodes::NodeHit;
    use super::super::rows::tests::{host_with, H, W};
    use super::super::{Hit, Host};
    use super::*;
    use deck_kit::base::draw::Prim;
    use deck_kit::base::tokens::Theme;

    const TOY: &str = include_str!("../../../show/fixtures/top-toy.show.json");

    fn night() -> Tokens {
        Tokens::new(Theme::Night, 16.0)
    }

    fn frame(host: &mut Host) -> DrawList {
        let mut d = DrawList::new(0.6);
        host.hits.clear();
        author::draw(&mut d, &night(), host, W, H, (0.0, 0.0));
        d
    }

    fn toy() -> Host {
        let mut host = host_with(TOY);
        host.row = host.shown().rows.iter().position(|r| r.id == "yellowmouse-enters").unwrap();
        frame(&mut host);
        host
    }

    fn screen(host: &Host, id: &str) -> Rect {
        let r = host.nodes.drawn.iter().find(|(x, _)| x == id).unwrap().1;
        host.nodes.on_screen(r)
    }

    /// a left press on the canvas at `a`, dragged to `b`, let go
    fn drag(host: &mut Host, a: (f32, f32), b: (f32, f32)) {
        host.cursor = a;
        host.press(Hit::Nodes(NodeHit::Canvas));
        host.pointer(b.0, b.1);
        host.release();
    }

    #[test]
    fn a_drag_on_the_canvas_selects_what_the_box_touches_and_shift_adds() {
        let mut host = toy();
        let (reader, clock) = (screen(&host, "reader"), screen(&host, "clock"));
        let a = (reader.x - 3.0, reader.y - 3.0);
        drag(&mut host, a, (reader.x + 4.0, reader.y + 4.0));
        assert_eq!(host.nodes.selected, ["reader"]);
        assert_eq!(host.nodes.focus.as_deref(), Some("reader"));
        assert!(host.nodes.take_tone().is_some(), "the new node's tone");
        host.nodes.shift = true;
        drag(&mut host, (clock.x - 3.0, clock.y - 3.0), (clock.x + 4.0, clock.y + 4.0));
        assert_eq!(host.nodes.selected, ["reader", "clock"], "shift adds");
        host.nodes.shift = false;
        drag(&mut host, (clock.x - 3.0, clock.y - 3.0), (clock.x + 4.0, clock.y + 4.0));
        assert_eq!(host.nodes.selected, ["clock"], "without shift the box starts again");
    }

    #[test]
    fn the_box_shows_while_it_is_drawn_and_selects_as_it_grows() {
        let mut host = toy();
        let r = screen(&host, "row");
        host.cursor = (r.x - 6.0, r.y - 6.0);
        host.press(Hit::Nodes(NodeHit::Canvas));
        host.pointer(r.x + r.w * 0.5, r.y + r.h * 0.5);
        assert_eq!(host.nodes.selected, ["row"], "selected while the box is in the hand");
        let sel = night().color.selection;
        let d = frame(&mut host);
        assert!(d.prims().iter().any(|p| matches!(p, Prim::Rect { color, .. } if color[..3] == sel[..3])), "the box's fill");
        host.release();
        assert!(host.nodes.boxing.is_none());
    }

    #[test]
    fn a_click_on_the_canvas_lets_the_selection_go() {
        let mut host = toy();
        let r = screen(&host, "row");
        drag(&mut host, (r.x - 3.0, r.y - 3.0), (r.x + 5.0, r.y + 5.0));
        assert!(!host.nodes.selected.is_empty());
        let c = host.nodes.canvas;
        drag(&mut host, (c.x + 2.0, c.y + c.h - 2.0), (c.x + 3.0, c.y + c.h - 2.0));
        assert!(host.nodes.selected.is_empty() && host.nodes.focus.is_none());
    }

    #[test]
    fn a_click_on_a_wire_pins_its_value_and_a_click_again_lets_it_go() {
        let mut host = toy();
        let i = host.nodes.wire_keys.iter().position(Option::is_some).expect("a wire with a value");
        let at = host.nodes.wire_mids[i].expect("the wire on screen");
        let away = |host: &mut Host| {
            host.cursor = (0.0, 0.0);
            frame(host).texts().len()
        };
        let before = away(&mut host);
        host.cursor = at;
        frame(&mut host);
        assert_eq!(host.nodes.hover, Some(format!("wire:{i}")), "the pointer on the wire");
        drag(&mut host, at, at);
        assert_eq!(host.nodes.pinned.len(), 1);
        assert_eq!(away(&mut host), before + 1, "pinned: the value shows with the pointer away");
        host.cursor = at;
        frame(&mut host);
        drag(&mut host, at, at);
        assert!(host.nodes.pinned.is_empty(), "unpinned");
        assert_eq!(away(&mut host), before);
    }

    #[test]
    fn a_focused_node_asks_for_its_signals_tone() {
        let mut host = toy();
        assert_eq!(host.nodes.take_tone(), None);
        let i = host.nodes.drawn.iter().position(|(x, _)| x == "clock").unwrap();
        host.press(Hit::Nodes(NodeHit::Node(i)));
        assert_eq!(host.nodes.take_tone(), Some(deck_show::focus::pitch(deck_kit::base::signal::Signal::Clock)));
        assert_eq!(host.nodes.take_tone(), None, "taken once");
        host.key(&winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab), false, true);
        assert!(host.nodes.take_tone().is_some(), "Tab too, a kept cell without pins as well");
    }
}
