//! The Show workspace's node view: the selected row as deck-show's graph (nodes.rs there), laid out left → right or top →
//! bottom (the workspace's setting) by deck-kit's graph, its pins and wires typed by deck-kit's signal table. Zoom and
//! pan as the stage: the wheel zooms round the pointer, a middle drag or space + drag pans, F frames everything.
use super::{Hit, Host};
use deck_kit::base::canvas::View;
use deck_kit::base::draw::DrawList;
use deck_kit::base::graph::{self, Direction, Graph, Metrics, Node, Placed};
use deck_kit::base::signal::Signal;
use deck_kit::base::tokens::Tokens;
use deck_kit::base::wire::Wire;
use deck_kit::editor::controls::{self, Rect};

/// the view's state: its zoom and pan (kept while the editor runs), the direction, the drag in the hand
#[derive(Debug, Clone, Default)]
pub struct NodeView {
    pub view: View,
    pub direction: Direction,
    /// framed for this direction, canvas size and graph extent: a new direction frames again, so does a new size or
    /// graph (another row) until the view is zoomed or panned by hand
    framed: Option<(Direction, (f32, f32), (f32, f32))>,
    touched: bool,
    /// the canvas drawn last, on screen
    canvas: Rect,
    bounds: Rect,
    grab: Option<(f32, f32)>,
    /// space is held: a left drag pans
    pub space: bool,
    /// a check run's zoom, round the canvas's middle whenever it frames itself
    pub zoom_once: Option<f32>,
}

impl NodeView {
    fn frame(&mut self) {
        self.view = View::framing(self.bounds, (self.canvas.w, self.canvas.h), self.canvas.w.min(self.canvas.h) * 0.06);
    }

    fn over(&self, x: f32, y: f32) -> bool {
        self.canvas.w > 0.0 && self.canvas.has(x, y)
    }

    fn local(&self, x: f32, y: f32) -> (f32, f32) {
        (x - self.canvas.x, y - self.canvas.y)
    }
}

impl Host {
    /// the wheel over the canvas: a notch (or a pinch) zooms round the pointer, a touchpad's two fingers pan
    pub fn nodes_wheel(&mut self, (x, y): (f32, f32), (dx, dy): (f32, f32), fingers: bool, pinch: bool) -> bool {
        let n = &mut self.nodes;
        if !self.two_d || self.sheet.is_some() || !n.over(x, y) {
            return false;
        }
        n.touched = true;
        match fingers && !pinch {
            true => n.view.pan_by((dx * 40.0, dy * 40.0)),
            false => n.view.zoom_at(n.local(x, y), dy),
        }
        true
    }

    /// a press that pans (the middle button, or the left with space held) taken on the canvas
    pub fn nodes_grab(&mut self, x: f32, y: f32) -> bool {
        let n = &mut self.nodes;
        if !self.two_d || self.sheet.is_some() || !n.over(x, y) {
            return false;
        }
        n.grab = Some((x, y));
        n.touched = true;
        true
    }

    pub(super) fn nodes_drag(&mut self, x: f32, y: f32) {
        if let Some((gx, gy)) = self.nodes.grab {
            self.nodes.view.pan_by((x - gx, y - gy));
            self.nodes.grab = Some((x, y));
        }
    }

    /// space over the canvas (nothing typed): a left drag pans while it is held
    pub fn nodes_space(&mut self, down: bool) -> bool {
        let (x, y) = self.cursor;
        if !down {
            return std::mem::take(&mut self.nodes.space);
        }
        let free = self.typing.is_none() && self.context.is_none() && self.flashing.is_none() && self.asking.is_none();
        self.nodes.space = free && self.two_d && self.sheet.is_none() && self.nodes.over(x, y);
        self.nodes.space
    }

    pub(super) fn nodes_release(&mut self) {
        self.nodes.grab = None;
    }

    /// F over the canvas frames everything
    pub(super) fn nodes_frame(&mut self) -> bool {
        let (x, y) = self.cursor;
        if !self.two_d || self.sheet.is_some() || !self.nodes.over(x, y) {
            return false;
        }
        self.nodes.frame();
        true
    }

    pub(super) fn nodes_press(&mut self, hit: Hit) {
        match hit {
            Hit::NodeFlow(d) => self.nodes.direction = d,
            Hit::NodeFrame => self.nodes.frame(),
            _ => {}
        }
    }
}

/// `text` cut to `room` in the sans face
fn sans_fit(d: &DrawList, text: &str, px: f32, room: f32) -> String {
    let mut n = text.chars().count();
    loop {
        let s = controls::fit(text, n);
        if n == 0 || d.sans_width(px, &s) <= room {
            return s;
        }
        n -= 1;
    }
}

/// the canvas's place on screen and its view
struct Cam {
    at: (f32, f32),
    view: View,
}

impl Cam {
    fn s(&self, p: (f32, f32)) -> (f32, f32) {
        let q = self.view.to_screen(p);
        (self.at.0 + q.0, self.at.1 + q.1)
    }
}

/// a colour at `a` of its alpha
fn faded(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

/// The view in `a` (the panel's content): the head with the direction and the legend, the canvas under it.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, g: &Graph, a: Rect) {
    let (pad, small) = (t.size.pad, t.size.small);
    let head = t.size.row;
    let dir = host.nodes.direction;
    // the head: the direction switch, framing, the zoom, then what each signal looks like
    let names = [Direction::Lr.word(), Direction::Tb.word()];
    let pills = controls::pills(d, t, a.x, a.y + 2.0, head - 8.0, &names, if dir == Direction::Lr { 0 } else { 1 });
    for (r, dir) in pills.iter().zip([Direction::Lr, Direction::Tb]) {
        host.hits.push((*r, Hit::NodeFlow(dir)));
    }
    let mut x = pills.last().map_or(a.x, |r| r.x + r.w) + pad;
    let fit = controls::pills(d, t, x, a.y + 2.0, head - 8.0, &["Fit"], usize::MAX);
    host.hits.push((fit[0], Hit::NodeFrame));
    x = fit[0].x + fit[0].w + pad;
    x += d.text(x, a.y + (head - small * 1.25) * 0.5 - 2.0, small, &format!("{:.0} %", host.nodes.view.zoom * 100.0), t.color.muted) + 2.0 * pad;
    let mut used: Vec<Signal> = g.nodes.iter().flat_map(|n| n.ins.iter().chain(&n.outs)).map(|p| p.signal).collect();
    used.sort();
    used.dedup();
    let r = small * 0.45;
    for s in used {
        let w = d.advance(small) * s.word().chars().count() as f32;
        if x + 3.0 * r + w > a.x + a.w {
            break;
        }
        d.pin((x + r, a.y + head * 0.5 - 2.0), r, (s.shape(), (1.0, 0.0)), t.color.signals.of(s), t.color.surface);
        x += 2.0 * r + pad * 0.5;
        x += d.text(x, a.y + (head - small * 1.25) * 0.5 - 2.0, small, s.word(), t.color.muted) + pad * 1.5;
    }

    let canvas = Rect { x: a.x, y: a.y + head, w: a.w, h: (a.h - head).max(0.0) };
    let unit = t.size.rem(1.0);
    let placed = graph::layout(g, dir, &Metrics::of(unit));
    let n = &mut host.nodes;
    n.canvas = canvas;
    n.bounds = graph::bounds(&placed);
    let size = (canvas.w, canvas.h);
    let extent = (n.bounds.w, n.bounds.h);
    let new_dir = n.framed.map(|f| f.0) != Some(dir);
    if new_dir || (n.framed != Some((dir, size, extent)) && !n.touched) {
        n.frame();
        n.framed = Some((dir, size, extent));
        n.touched &= !new_dir;
        if let Some(z) = n.zoom_once {
            n.view.zoom_to((canvas.w * 0.5, canvas.h * 0.5), z);
        }
    }
    d.rect(canvas.x, canvas.y, canvas.w, 1.0, t.color.line);
    d.clip(Some(canvas));
    let cam = Cam { at: (canvas.x, canvas.y), view: n.view };
    let s = |p| cam.s(p);
    let z = cam.view.zoom;
    for l in &g.links {
        let (Some(from), Some(to)) = (placed.get(l.from.0).and_then(|p| p.outs.get(l.from.1)), placed.get(l.to.0).and_then(|p| p.ins.get(l.to.1))) else { continue };
        let (a, b) = (s(*from), s(*to));
        let w = match dir {
            Direction::Lr => Wire::hang(a, b, unit * z),
            Direction::Tb => Wire::drop(a, b, unit * z),
        };
        let c = t.color.signals.of(l.signal);
        let c = if l.live { c } else { faded(c, 0.35) };
        let beads = host.flow.filter(|_| l.live).map(|secs| w.flowing(unit * z, secs)).unwrap_or_default();
        d.wire(&w, 1.4 * z * l.signal.weight(), faded(c, 0.85), &beads, faded(c, 0.5));
    }
    for (node, p) in g.nodes.iter().zip(&placed) {
        box_of(d, t, node, p, &cam, dir);
    }
    d.clip(None);
    if g.nodes.is_empty() {
        d.text(canvas.x + pad, canvas.y + pad, small, "No row selected: pick one in the rows band.", t.color.muted);
    }
}

/// one node: its ground, a stripe in the colour of what it gives, the title and detail, its pins and their labels
fn box_of(d: &mut DrawList, t: &Tokens, n: &Node, p: &Placed, cam: &Cam, dir: Direction) {
    let (s, z) = (|p| cam.s(p), cam.view.zoom);
    let (o, e) = (s((p.rect.x, p.rect.y)), s((p.rect.x + p.rect.w, p.rect.y + p.rect.h)));
    let (w, h) = (e.0 - o.0, e.1 - o.1);
    let radius = if n.collapsed { h * 0.5 } else { t.size.radius * 1.5 * z };
    let main = n.outs.first().or(n.ins.first()).map(|p| t.color.signals.of(p.signal)).unwrap_or(t.color.muted);
    d.round_rect(o.0, o.1, w, h, radius, if n.collapsed { t.color.within } else { t.color.control });
    d.outline(o.0, o.1, w, h, (radius, 1.0), if n.far { t.color.muted } else { t.color.line });
    let (font, small, pad) = (t.size.font * z, t.size.small * z, t.size.pad * z);
    let head = t.size.rem(2.0) * z;
    if !n.collapsed {
        d.round_rect(o.0, o.1, w, 3.0 * z, radius.min(1.5 * z), main);
        d.rect(o.0, o.1 + head, w, 1.0, t.color.line);
    }
    if z >= 0.3 {
        let room = (w - 2.0 * pad).max(0.0);
        let detail_w = (d.advance(small) * n.detail.chars().count() as f32).min(room * 0.35);
        let title_room = if n.collapsed { room } else { room - detail_w - pad };
        let ty = o.1 + ((if n.collapsed { h } else { head }) - font * 1.25) * 0.5;
        d.sans(o.0 + pad, ty, font, &sans_fit(d, &n.title, font, title_room), if n.far { t.color.muted } else { t.color.text });
        if !n.collapsed {
            let detail = controls::fit(&n.detail, ((room - title_room - pad).max(0.0) / d.advance(small)) as usize);
            let dw = d.advance(small) * detail.chars().count() as f32;
            d.text(o.0 + w - pad - dw, o.1 + (head - small * 1.25) * 0.5, small, &detail, t.color.muted);
        }
    }
    let r = t.size.rem(0.32) * z;
    let flow = dir.flow();
    let labels = z >= 0.55 && !n.collapsed;
    for (pins, at, inlet) in [(&n.ins, &p.ins, true), (&n.outs, &p.outs, false)] {
        let mut drawn: Vec<(f32, f32)> = Vec::new();
        for (pin, q) in pins.iter().zip(at) {
            let q = s(*q);
            if !drawn.contains(&q) {
                d.pin(q, r, (pin.signal.shape(), flow), t.color.signals.of(pin.signal), t.color.control);
                drawn.push(q);
            }
            if !labels {
                continue;
            }
            // a label keeps to its pin's share of the edge
            let max = if dir == Direction::Lr { w * 0.45 } else { w / pins.len() as f32 * 0.95 };
            let label = controls::fit(&pin.label, (max / d.advance(small)) as usize);
            let lw = d.advance(small) * label.chars().count() as f32;
            let (lx, ly) = match (dir, inlet) {
                (Direction::Lr, true) => (q.0 + r + pad * 0.6, q.1 - small * 0.62),
                (Direction::Lr, false) => (q.0 - r - pad * 0.6 - lw, q.1 - small * 0.62),
                (Direction::Tb, true) => (q.0 - lw * 0.5, q.1 - r - small * 1.4),
                (Direction::Tb, false) => (q.0 - lw * 0.5, q.1 + r + small * 0.2),
            };
            d.text(lx, ly, small, &label, t.color.muted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::author;
    use super::super::rows::tests::{host_with, H, W};
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
        host
    }

    #[test]
    fn the_rows_nodes_show_with_typed_pins_and_wires() {
        let mut host = toy();
        let d = frame(&mut host);
        let texts = d.texts();
        for want in ["reader", "clock", "this row sends", "on event", "LR", "TB", "event", "address"] {
            assert!(texts.contains(&want), "{want}: {texts:?}");
        }
        let event = night().color.signals.event;
        assert!(d.prims().iter().any(|p| matches!(p, Prim::Poly { n: 3, color, .. } if *color == event)), "an event's triangle pin");
        let wire = d.prims().iter().filter(|p| matches!(p, Prim::Segment { color, .. } if color[..3] == event[..3])).count();
        assert!(wire >= deck_kit::base::wire::SAMPLES, "event wires drawn: {wire}");
    }

    #[test]
    fn the_direction_switches_and_frames_again() {
        let mut host = toy();
        frame(&mut host);
        let lr = host.nodes.bounds;
        assert!(host.hits.iter().any(|(r, h)| *h == Hit::NodeFlow(Direction::Tb) && r.w > 0.0), "the TB pill");
        host.press(Hit::NodeFlow(Direction::Tb));
        frame(&mut host);
        assert_eq!(host.nodes.direction, Direction::Tb);
        let tb = host.nodes.bounds;
        assert_ne!(tb, lr, "laid out again");
    }

    #[test]
    fn the_wheel_zooms_round_the_pointer_and_a_drag_pans() {
        let mut host = toy();
        frame(&mut host);
        let c = host.nodes.canvas;
        let at = (c.x + c.w * 0.5, c.y + c.h * 0.5);
        let (z0, under) = (host.nodes.view.zoom, host.nodes.view.to_world(host.nodes.local(at.0, at.1)));
        assert!(host.nodes_wheel(at, (0.0, 2.0), false, false));
        assert!(host.nodes.view.zoom > z0);
        let back = host.nodes.view.to_screen(under);
        assert!((back.0 - (at.0 - c.x)).abs() < 1e-2 && (back.1 - (at.1 - c.y)).abs() < 1e-2, "the point stays under the pointer");
        assert!(!host.nodes_wheel((c.x - 50.0, c.y), (0.0, 1.0), false, false), "beside the canvas the wheel is not the view's");
        let pan = host.nodes.view.pan;
        assert!(host.nodes_grab(at.0, at.1));
        host.pointer(at.0 + 30.0, at.1 - 10.0);
        host.release();
        host.pointer(at.0 + 90.0, at.1);
        assert_eq!(host.nodes.view.pan, (pan.0 + 30.0, pan.1 - 10.0), "panned while held, not after");
        host.cursor = at;
        assert!(host.nodes_frame());
        assert!(host.nodes.view.zoom <= deck_kit::base::canvas::FRAME_MAX);
    }
}
