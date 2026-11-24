//! One spore as a card: head and body share one border, the whole border thin in the kind's colour, the
//! head faintly tinted with the kind's icon, the name, the kind's word and the fold; the pins one row each on the card's
//! edge in their signal's colour, a held value in mono beside its label. Folded, the card is its head alone.
use crate::paint::{faded, main_colour, sans_fit, Cam};
use crate::Seen;
use deck_kit::base::draw::DrawList;
use deck_kit::base::graph::{Direction, Node, Placed};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::colour::{self, Rgb};
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::icons;

/// how strong the kind's colour is: in the border, the head's tint, the line under the head
const BORDER: f32 = 0.55;
const BORDER_FAR: f32 = 0.35;
const TINT: f32 = 0.09;
const DIVIDE: f32 = 0.22;

/// what a node's paint left to press: its body and its fold
pub struct Boxed {
    pub body: Rect,
    pub fold: Option<Rect>,
}

/// One node (`tint`: a folded group's colour instead of its kind's): its outline in its kind's colour, whole under the
/// pointer; selected the text colour round the whole card on the selected ground; focused a flash and a skeleton shimmer.
pub fn card(d: &mut DrawList, t: &Tokens, (n, p, tint): (&Node, &Placed, Option<Rgb>), cam: &Cam, v: &dyn Seen) -> Boxed {
    let z = cam.view.zoom;
    let body = cam.rect(p.rect);
    let (o, w, h) = ((body.x, body.y), body.w, body.h);
    let radius = t.size.radius * 1.5 * z;
    let kind = tint.map_or_else(|| main_colour(t, n), |c| colour::rgba(c, 1.0));
    let head = if n.collapsed { h } else { t.size.rem(2.0) * z };
    let (selected, hovered) = (v.selected(&n.id), v.hovered(&n.id));
    d.round_rect(o.0, o.1, w, h, radius, if selected { t.color.selected } else { t.color.control });
    // the head's tint, its top corners the card's: drawn whole and clipped at the head's foot
    let before = d.current_clip();
    let head_box = Rect { x: o.0, y: o.1, w, h: head };
    d.clip(Some(before.map_or(head_box, |c| meet(c, head_box))));
    d.round_rect(o.0, o.1, w, head + radius, radius, faded(kind, TINT));
    d.clip(before);
    if !n.collapsed {
        d.rect(o.0, o.1 + head, w, 1.0, faded(kind, DIVIDE));
    }
    let edge = if selected {
        t.color.text
    } else if hovered {
        kind
    } else {
        faded(kind, if n.far { BORDER_FAR } else { BORDER })
    };
    d.outline(o.0, o.1, w, h, (radius, if selected { 1.5 } else { 1.0 }), edge);
    let fold = head_line(d, t, n, (o, w, head), kind, z);
    preview(d, t, n, (o, w, h), z, v);
    pins(d, t, n, p, cam, w);
    if let Some(a) = v.flashing(&n.id) {
        flash(d, t, (o, w, h), radius, kind, a);
    }
    Boxed { body, fold }
}

/// where two rectangles overlap (none: an empty one)
fn meet(a: Rect, b: Rect) -> Rect {
    let (x, y) = (a.x.max(b.x), a.y.max(b.y));
    Rect { x, y, w: ((a.x + a.w).min(b.x + b.w) - x).max(0.0), h: ((a.y + a.h).min(b.y + b.h) - y).max(0.0) }
}

/// the head: the kind's icon, the name (cut with an ellipsis), the kind's word small, the fold at its end; the fold's place
fn head_line(d: &mut DrawList, t: &Tokens, n: &Node, (o, w, head): ((f32, f32), f32, f32), kind: [f32; 4], z: f32) -> Option<Rect> {
    let (font, small, pad) = (t.size.font * z, t.size.small * z, t.size.pad * z);
    let fs = (12.0 * z).clamp(9.0, 18.0);
    let fold = (z >= 0.45).then_some(Rect { x: o.0 + w - pad - fs, y: o.1 + (head - fs) * 0.5, w: fs, h: fs });
    if let Some(f) = fold {
        d.icon(f.x, f.y, f.w, if n.collapsed { "next" } else { "down" }, t.color.muted);
    }
    if z < 0.3 {
        return fold;
    }
    let is = font * 1.1;
    d.icon(o.0 + pad, o.1 + (head - is) * 0.5, is, icons::of_kind(n.kind), kind);
    let x = o.0 + pad + is + pad * 0.7;
    let end = fold.map_or(o.0 + w - pad, |f| f.x - pad * 0.6);
    let word_w = if n.kind.is_empty() { 0.0 } else { d.sans_width(small, n.kind) };
    let room = (end - x).max(0.0);
    let word_w = if word_w <= room * 0.45 { word_w } else { 0.0 };
    let name_room = (room - word_w - if word_w > 0.0 { pad } else { 0.0 }).max(0.0);
    d.sans(x, o.1 + (head - font * 1.25) * 0.5, font, &sans_fit(d, &n.title, font, name_room), t.color.text);
    if word_w > 0.0 {
        d.sans(end - word_w, o.1 + (head - small * 1.25) * 0.5, small, n.kind, t.color.muted);
    }
    fold
}

/// the picture of what it plays, under its pins
fn preview(d: &mut DrawList, t: &Tokens, n: &Node, (o, w, h): ((f32, f32), f32, f32), z: f32, v: &dyn Seen) {
    let (true, false, Some(key)) = (v.previews(), n.collapsed, n.picture.as_deref()) else { return };
    let (small, pad) = (t.size.small * z, t.size.pad * z);
    let ph = (5.0 * t.size.rem(1.0) - t.size.pad) * z;
    let pw = (ph * 264.0 / 168.0).min(w - 2.0 * pad);
    let (px, py) = (o.0 + (w - pw) * 0.5, o.1 + h - ph - pad * 0.5);
    match v.picture(key) {
        Some(id) => d.picture(px, py, pw, ph, id),
        None => {
            d.round_rect(px, py, pw, ph, t.size.radius * z, t.color.within);
            if z >= 0.55 {
                d.text(px + pad * 0.6, py + pad * 0.4, small, &controls::fit(key, (pw / d.advance(small)) as usize), t.color.muted);
            }
        }
    }
}

/// the pins on the card's edge in their signal's colour; open and near enough, each its label and its value in mono
fn pins(d: &mut DrawList, t: &Tokens, n: &Node, p: &Placed, cam: &Cam, w: f32) {
    let z = cam.view.zoom;
    let (small, pad) = (t.size.small * z, t.size.pad * z);
    let r = t.size.rem(0.32) * z;
    let flow = cam.dir.flow();
    let labels = z >= 0.55 && !n.collapsed;
    let ink = faded(t.color.text, 0.8);
    for (pins, at, inlet) in [(&n.ins, &p.ins, true), (&n.outs, &p.outs, false)] {
        let mut drawn: Vec<(f32, f32)> = Vec::new();
        for (pin, q) in pins.iter().zip(at) {
            let q = cam.s(*q);
            if !drawn.contains(&q) {
                d.pin(q, r, (pin.signal.shape(), flow), t.color.signals.of(pin.signal), t.color.control);
                drawn.push(q);
            }
            if !labels {
                continue;
            }
            if cam.dir == Direction::Tb {
                // a label keeps to its pin's share of the edge
                let max = w / pins.len() as f32 * 0.95;
                let label = sans_fit(d, &pin.label, small, max);
                let lw = d.sans_width(small, &label);
                let ly = if inlet { q.1 - r - small * 1.4 } else { q.1 + r + small * 0.2 };
                d.sans(q.0 - lw * 0.5, ly, small, &label, ink);
                continue;
            }
            let ly = q.1 - small * 0.65;
            let value = pin.value.as_deref().map(|x| controls::fit(x, ((w * 0.3) / d.advance(small)) as usize)).unwrap_or_default();
            let vw = if value.is_empty() { 0.0 } else { d.advance(small) * value.chars().count() as f32 + pad * 0.6 };
            let label = sans_fit(d, &pin.label, small, (w * 0.5 - vw).max(0.0));
            let lw = d.sans_width(small, &label);
            if inlet {
                let lx = q.0 + r + pad * 0.6;
                d.sans(lx, ly, small, &label, ink);
                if !value.is_empty() {
                    d.text(lx + lw + pad * 0.6, ly, small, &value, t.color.muted);
                }
            } else {
                let lx = q.0 - r - pad * 0.6 - lw;
                d.sans(lx, ly, small, &label, ink);
                if !value.is_empty() {
                    d.text(lx - vw, ly, small, &value, t.color.muted);
                }
            }
        }
    }
}

/// the focus flash: a ring that closes in, and the skeleton shimmer, a light band sweeping across once
fn flash(d: &mut DrawList, t: &Tokens, (o, w, h): ((f32, f32), f32, f32), radius: f32, kind: [f32; 4], a: f32) {
    let k = 1.0 - a;
    d.outline(o.0 - 3.0 * k, o.1 - 3.0 * k, w + 6.0 * k, h + 6.0 * k, (radius + 3.0 * k, 1.0 + 2.5 * k), faded(kind, k));
    let band = w * 0.35;
    let x = o.0 - band + (w + band) * a;
    let light = faded(t.color.text, 0.12 * k);
    let clear = faded(t.color.text, 0.0);
    let (l0, l1) = (x.max(o.0), (x + band * 0.5).min(o.0 + w));
    if l1 > l0 {
        d.gradient(l0, o.1, l1 - l0, h, [clear, light, light, clear]);
    }
    let (r0, r1) = ((x + band * 0.5).max(o.0), (x + band).min(o.0 + w));
    if r1 > r0 {
        d.gradient(r0, o.1, r1 - r0, h, [light, clear, clear, light]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_kit::base::canvas::View;
    use deck_kit::base::graph::Pin;
    use deck_kit::base::signal::Signal;
    use deck_kit::base::tokens::Theme;

    /// a widget's state with nothing on but the selection
    #[derive(Default)]
    struct Still {
        selected: Vec<String>,
    }

    impl Seen for Still {
        fn selected(&self, id: &str) -> bool {
            self.selected.iter().any(|s| s == id)
        }
        fn flashing(&self, _: &str) -> Option<f32> {
            None
        }
        fn previews(&self) -> bool {
            false
        }
        fn picture(&self, _: &str) -> Option<u64> {
            None
        }
        fn secs(&self) -> f32 {
            0.0
        }
        fn blob(&self) -> Option<(f32, f32)> {
            None
        }
    }

    fn paint(n: &Node, selected: bool) -> DrawList {
        let t = Tokens::new(Theme::Night, 16.0);
        let mut d = DrawList::new(0.6);
        let cam = Cam { at: (0.0, 0.0), view: View::default(), unit: 16.0, dir: Direction::Lr };
        let p = Placed { rect: Rect { x: 0.0, y: 0.0, w: 320.0, h: 96.0 }, ins: vec![(0.0, 48.0); n.ins.len()], outs: vec![(320.0, 48.0); n.outs.len()] };
        let mut v = Still::default();
        if selected {
            v.selected.push(n.id.clone());
        }
        card(&mut d, &t, (n, &p, None), &cam, &v);
        d
    }

    #[test]
    fn the_head_says_the_name_and_the_kind_and_a_pin_its_value() {
        let n = Node { id: "left".into(), title: "left".into(), kind: "character", ins: vec![Pin::new("mood", Signal::Control).valued("shy")], ..Node::default() };
        let d = paint(&n, false);
        assert_eq!(d.texts(), ["left", "character", "mood", "shy"]);
    }

    #[test]
    fn a_long_name_ends_in_an_ellipsis_and_the_kind_stays() {
        let n = Node { id: "row".into(), title: "ch3.yellowmouse-enters-the-stage-from-far-away".into(), kind: "event", ..Node::default() };
        let d = paint(&n, false);
        let texts = d.texts();
        assert!(texts[0].ends_with('…') && texts[0].starts_with("ch3."), "{texts:?}");
        assert_eq!(texts[1], "event");
    }
}
