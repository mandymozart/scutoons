//! The node view's picking: a drag on the empty canvas draws a box that selects the nodes it touches (shift adds to the
//! selection), a click on a wire pins its value beside it and a click again lets it go, and a node focused or selected
//! asks for its tone (deck-show focus.rs; main.rs plays it when the settings allow).
// the box select, the pinned values and the tone request are the Spores widget's (deck-spores pick.rs, state.rs)
pub use deck_spores::pick::DRAG;

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


#[cfg(test)]
mod tests {
    use super::super::author;
    use super::super::nodes::NodeHit;
    use super::super::rows::tests::{host_with, H, W};
    use super::super::{Hit, Host};
    use deck_kit::base::draw::DrawList;
    use deck_kit::base::tokens::Tokens;
    use deck_kit::editor::controls::Rect;
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
        assert_eq!(host.nodes.take_tone(), Some(deck_spores::focus::pitch(deck_kit::base::signal::Signal::Clock)));
        assert_eq!(host.nodes.take_tone(), None, "taken once");
        host.key(&winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab), false);
        assert!(host.nodes.take_tone().is_some(), "Tab too, a kept cell without pins as well");
    }
}
