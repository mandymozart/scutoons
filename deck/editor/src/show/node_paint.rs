//! The node view's painting (nodes.rs holds its state and input): group frames on a dotted grid in their colour, the
//! latent grid under what the pointer is on (dots of a fixed size whose opacity fades, no blur), the wires as the core's
//! activity says, the nodes with their pins, fold, focus flash and preview, the notes, a wire's value.
use super::nodes::{NodeHit, NodeView, FADE_IN, FADE_OUT};
use deck_kit::base::canvas::{self, View};
use deck_kit::base::draw::DrawList;
use deck_kit::base::graph::{self, Activity, Direction, Graph, Link, Node, Placed};
use deck_kit::base::tokens::Tokens;
use deck_kit::base::wire::Wire;
use deck_kit::editor::colour::{self, Rgb};
use deck_kit::editor::controls::{self, Rect};
use deck_show::canvas::Canvas;

/// the canvas's place on screen, its view, the grid's unit and the direction
pub struct Cam {
    pub at: (f32, f32),
    pub view: View,
    pub unit: f32,
    pub dir: Direction,
}

impl Cam {
    pub fn s(&self, p: (f32, f32)) -> (f32, f32) {
        let q = self.view.to_screen(p);
        (self.at.0 + q.0, self.at.1 + q.1)
    }

    pub fn rect(&self, r: Rect) -> Rect {
        let (o, e) = (self.s((r.x, r.y)), self.s((r.x + r.w, r.y + r.h)));
        Rect { x: o.0, y: o.1, w: e.0 - o.0, h: e.1 - o.1 }
    }

    fn z(&self) -> f32 {
        self.view.zoom
    }
}

/// a colour at `a` of its alpha
pub fn faded(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

/// `a` mixed into `b` by `k`
fn mix(a: [f32; 4], b: [f32; 4], k: f32) -> [f32; 4] {
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k, 1.0]
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

/// the colour of what a node gives (else takes)
pub fn main_colour(t: &Tokens, n: &Node) -> [f32; 4] {
    n.outs.first().or(n.ins.first()).map(|p| t.color.signals.of(p.signal)).unwrap_or(t.color.muted)
}

/// an open group's frame on screen
pub struct Frame {
    pub k: usize,
    pub id: String,
    pub label: String,
    pub colour: [f32; 4],
    pub rect: Rect,
}

/// the frame of group `k` round its nodes `members`, a grid step out and room for its label on top; a folded group has
/// none (its pill is a node)
pub fn frame_of(doc: &Canvas, k: usize, members: &[usize], g: &Graph, placed: &[Placed], cam: &Cam) -> Option<Frame> {
    let grp = doc.groups.get(k).filter(|x| !x.folded)?;
    if members.iter().any(|&i| g.nodes.get(i).is_none_or(|n| n.id.starts_with(deck_show::canvas::GROUP))) {
        return None;
    }
    let b = graph::bounds_of(members.iter().filter_map(|&i| placed.get(i)).map(|p| p.rect));
    let u = cam.unit;
    let world = Rect { x: b.x - u, y: b.y - 2.0 * u, w: b.w + 2.0 * u, h: b.h + 3.0 * u };
    let colour = colour::rgb(&grp.colour).map_or([0.6, 0.6, 0.6, 1.0], |c| colour::rgba(c, 1.0));
    Some(Frame { k, id: grp.id.clone(), label: grp.label.clone(), colour, rect: cam.rect(world) })
}

/// every wire on screen, by its link's index
pub fn wires(g: &Graph, placed: &[Placed], cam: &Cam) -> Vec<(usize, Wire)> {
    let step = cam.unit * cam.z();
    g.links
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            let from = placed.get(l.from.0).and_then(|p| p.outs.get(l.from.1))?;
            let to = placed.get(l.to.0).and_then(|p| p.ins.get(l.to.1))?;
            let (a, b) = (cam.s(*from), cam.s(*to));
            Some((i, if cam.dir == Direction::Lr { Wire::hang(a, b, step) } else { Wire::drop(a, b, step) }))
        })
        .collect()
}

/// a group's ground: its grid's dots in its colour, lighter than a reveal's, and its edge
pub fn ground(d: &mut DrawList, t: &Tokens, f: &Frame, cam: &Cam, canvas: Rect) {
    let r = f.rect;
    let inside = Rect { x: r.x.max(canvas.x), y: r.y.max(canvas.y), w: (r.x + r.w).min(canvas.x + canvas.w) - r.x.max(canvas.x), h: (r.y + r.h).min(canvas.y + canvas.h) - r.y.max(canvas.y) };
    if inside.w > 0.0 && inside.h > 0.0 {
        for (p, a) in canvas::dots(inside, cam.at, cam.view, cam.unit, |_| 0.32) {
            d.disc(p.0, p.1, 1.0, faded(f.colour, a));
        }
    }
    d.outline(r.x, r.y, r.w, r.h, (t.size.radius * 2.0 * cam.z(), 1.0), faded(f.colour, 0.45));
}

/// a group's head: its fold, its swatch (the next colour), its label (renamed in place); the presses on them
pub fn label(d: &mut DrawList, t: &Tokens, f: &Frame, cam: &Cam) -> Vec<(Rect, NodeHit)> {
    let z = cam.z();
    if z < 0.3 {
        return Vec::new();
    }
    let (px, pad) = (t.size.small * z.max(0.7), t.size.pad * z);
    let y = f.rect.y + cam.unit * z * 0.4;
    let s = px * 1.1;
    let fold = Rect { x: f.rect.x + pad, y, w: s, h: s };
    d.icon(fold.x, fold.y, s, "down", f.colour);
    let swatch = Rect { x: fold.x + s + pad * 0.6, y, w: s, h: s };
    d.disc(swatch.x + s * 0.5, swatch.y + s * 0.5, s * 0.36, f.colour);
    let tx = swatch.x + s + pad * 0.6;
    let room = (f.rect.x + f.rect.w - tx - pad).max(0.0);
    let text = sans_fit(d, &f.label, px, room);
    let w = d.sans(tx, y + (s - px * 1.25) * 0.5, px, &text, f.colour);
    vec![(fold, NodeHit::GroupFold(f.k)), (swatch, NodeHit::GroupColour(f.k)), (Rect { x: tx, y, w: w.max(s), h: s }, NodeHit::GroupName(f.k))]
}

/// a group's label while it is typed
pub fn typed_label(d: &mut DrawList, t: &Tokens, f: &Frame, text: &str) {
    let px = t.size.small;
    let (x, y) = (f.rect.x + t.size.pad, f.rect.y - px * 1.9);
    let w = d.sans_width(px, text) + 2.0 * t.size.pad + px;
    d.round_rect(x, y, w, px * 1.7, t.size.radius, t.color.control);
    d.outline(x, y, w, px * 1.7, (t.size.radius, 1.0), f.colour);
    d.sans(x + t.size.pad, y + px * 0.2, px, &format!("{text}|"), t.color.text);
}

/// one reveal of the latent grid: what it lies under (`node:<id>`, `group:<id>`, `wire:<n>`), its colour, since when it
/// shows and since when it fades
#[derive(Debug, Clone, PartialEq)]
pub struct Reveal {
    pub key: String,
    pub colour: [f32; 4],
    pub since: f32,
    pub left: Option<f32>,
}

impl Reveal {
    /// how strong it shows at `now`: fading in, then out once the pointer left
    pub fn strength(&self, now: f32) -> f32 {
        let fade_in = ((now - self.since) / FADE_IN).clamp(0.0, 1.0);
        let fade_out = self.left.map_or(1.0, |l| 1.0 - ((now - l) / FADE_OUT).clamp(0.0, 1.0));
        fade_in * fade_out
    }
}

/// The reveals after the pointer moved onto `hover` (none: nothing) at `now`: the one under it fades in (again), the
/// others fade out and go once faded.
pub fn follow(reveals: &mut Vec<Reveal>, hover: Option<(String, [f32; 4])>, now: f32) {
    let key = hover.as_ref().map(|(k, _)| k.as_str());
    for r in reveals.iter_mut() {
        match (Some(r.key.as_str()) == key, r.left) {
            (true, Some(_)) => {
                let s = r.strength(now);
                (r.since, r.left) = (now - FADE_IN * s, None);
            }
            (false, None) => r.left = Some(now),
            _ => {}
        }
    }
    reveals.retain(|r| r.left.is_none_or(|l| now - l < FADE_OUT));
    if let Some((k, colour)) = hover {
        if !reveals.iter().any(|r| r.key == k) {
            reveals.push(Reveal { key: k, colour, since: now, left: None });
        }
    }
}

/// the shape a reveal lies under, on screen
enum Shape {
    Rect(Rect),
    Line(Vec<(f32, f32)>),
}

/// A reveal: the grid's dots round what it lies under in its colour, each the same small size, its opacity fading with
/// the distance and following the pointer slowly (the blob); without blur.
pub fn reveal(d: &mut DrawList, r: &Reveal, (n, cam, canvas): (&NodeView, &Cam, Rect), (frames, wires, g, placed): (&[Frame], &[(usize, Wire)], &Graph, &[Placed])) {
    let s = r.strength(n.secs);
    if s <= 0.0 {
        return;
    }
    let shape = if let Some(id) = r.key.strip_prefix("node:") {
        g.node(id).and_then(|i| placed.get(i)).map(|p| Shape::Rect(cam.rect(p.rect)))
    } else if let Some(id) = r.key.strip_prefix("group:") {
        frames.iter().find(|f| f.id == id).map(|f| Shape::Rect(f.rect))
    } else {
        let i: Option<usize> = r.key.strip_prefix("wire:").and_then(|i| i.parse().ok());
        wires.iter().find(|(k, _)| Some(*k) == i).map(|(_, w)| Shape::Line(w.points()))
    };
    let Some(shape) = shape else { return };
    let z = cam.z();
    let reach = 3.0 * cam.unit * z;
    let b = match &shape {
        Shape::Rect(r) => *r,
        Shape::Line(p) => {
            let (x0, y0, x1, y1) = p.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |o, q| (o.0.min(q.0), o.1.min(q.1), o.2.max(q.0), o.3.max(q.1)));
            Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
        }
    };
    let x0 = (b.x - reach).max(canvas.x);
    let y0 = (b.y - reach).max(canvas.y);
    let area = Rect { x: x0, y: y0, w: (b.x + b.w + reach).min(canvas.x + canvas.w) - x0, h: (b.y + b.h + reach).min(canvas.y + canvas.h) - y0 };
    if area.w <= 0.0 || area.h <= 0.0 {
        return;
    }
    let blob = n.blob.map(|p| cam.s(p));
    let near = |p: (f32, f32)| {
        let d = match &shape {
            Shape::Rect(r) => canvas::to_rect(*r, p),
            Shape::Line(pts) => canvas::to_line(pts, p),
        };
        let follow = blob.map_or(1.0, |q| 0.5 + 0.5 * canvas::falloff(((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt(), 8.0 * cam.unit * z));
        s * canvas::falloff(d, reach) * follow * 0.6
    };
    for (p, a) in canvas::dots(area, cam.at, cam.view, cam.unit, near) {
        d.disc(p.0, p.1, 1.1, faded(r.colour, a));
    }
}

/// A wire as the core's activity says (`activity` on): idle faint and thin, a stream glowing with its beads, a pulse
/// lit with a bead on its way to the inlet. Off: every wire alike. Hovered: a halo.
pub fn wire(d: &mut DrawList, t: &Tokens, (l, w): (&Link, &Wire), (cam, activity, flow): (&Cam, bool, Option<f32>), hovered: bool) {
    let z = cam.z();
    let c = t.color.signals.of(l.signal);
    let thick = 1.4 * z * l.signal.weight();
    if hovered {
        let pts = w.points();
        for p in pts.windows(2) {
            d.segment(p[0], p[1], thick * 5.0, faded(c, 0.18));
        }
    }
    if !activity {
        return d.wire(w, thick, faded(c, 0.85), &[]);
    }
    match l.activity {
        Activity::Idle => {
            let pts = w.points();
            for p in pts.windows(2) {
                d.segment(p[0], p[1], thick * 0.8, faded(c, 0.32));
            }
        }
        Activity::Stream => {
            let beads = flow.map(|secs| w.flowing(cam.unit * z, secs)).unwrap_or_default();
            d.wire(w, thick * 1.2, c, &beads);
        }
        Activity::Pulse(p) => {
            d.wire(w, thick * 1.3, faded(c, 1.0 - 0.45 * p), &[]);
            let at = w.at(p);
            let r = 3.0 * z.max(0.6);
            d.disc(at.0, at.1, r * 2.2, faded(c, 0.22));
            d.disc(at.0, at.1, r, c);
        }
    }
}

/// what a node's paint left to press: its body and its fold
pub struct Boxed {
    pub body: Rect,
    pub fold: Option<Rect>,
}

/// One node: its ground (a folded group's pill on its dotted grid), a stripe in the colour of what it gives, the title
/// and detail, its fold, its pins and their labels, its preview; selected a ring in its colour, focused a flash and a
/// skeleton shimmer.
pub fn box_of(d: &mut DrawList, t: &Tokens, (n, p, tint): (&Node, &Placed, Option<Rgb>), cam: &Cam, v: &NodeView) -> Boxed {
    let z = cam.z();
    let body = cam.rect(p.rect);
    let (o, w, h) = ((body.x, body.y), body.w, body.h);
    let radius = if n.collapsed { h * 0.5 } else { t.size.radius * 1.5 * z };
    let main = tint.map_or_else(|| main_colour(t, n), |c| colour::rgba(c, 1.0));
    d.round_rect(o.0, o.1, w, h, radius, if n.collapsed { t.color.within } else { t.color.control });
    if tint.is_some() {
        for (q, a) in canvas::dots(body, cam.at, cam.view, cam.unit, |q| if canvas::to_rect(Rect { x: body.x + radius * 0.5, y: body.y + 2.0, w: (body.w - radius).max(0.0), h: (body.h - 4.0).max(0.0) }, q) <= 0.0 { 0.32 } else { 0.0 }) {
            d.disc(q.0, q.1, 1.0, faded(main, a));
        }
    }
    let edge = if tint.is_some() { faded(main, 0.6) } else if n.far { t.color.muted } else { t.color.line };
    d.outline(o.0, o.1, w, h, (radius, 1.0), edge);
    let (font, small, pad) = (t.size.font * z, t.size.small * z, t.size.pad * z);
    let head = t.size.rem(2.0) * z;
    if !n.collapsed {
        d.round_rect(o.0, o.1, w, 3.0 * z, radius.min(1.5 * z), main);
        d.rect(o.0, o.1 + head, w, 1.0, t.color.line);
    }
    // the fold, at the head's end (a pill's end)
    let fs = (12.0 * z).clamp(9.0, 18.0);
    let fold = (z >= 0.45).then(|| {
        let line = if n.collapsed { h } else { head };
        Rect { x: o.0 + w - pad - fs - if n.collapsed { radius * 0.3 } else { 0.0 }, y: o.1 + (line - fs) * 0.5, w: fs, h: fs }
    });
    if let Some(f) = fold {
        d.icon(f.x, f.y, f.w, if n.collapsed { "next" } else { "down" }, t.color.muted);
    }
    let fold_w = fold.map_or(0.0, |f| f.w + pad * 0.5);
    if z >= 0.3 {
        let room = (w - 2.0 * pad - fold_w).max(0.0);
        let detail_w = (d.advance(small) * n.detail.chars().count() as f32).min(room * 0.35);
        let title_room = if n.collapsed { room } else { room - detail_w - pad };
        let ty = o.1 + ((if n.collapsed { h } else { head }) - font * 1.25) * 0.5;
        d.sans(o.0 + pad, ty, font, &sans_fit(d, &n.title, font, title_room), if n.far { t.color.muted } else { t.color.text });
        if !n.collapsed {
            let detail = controls::fit(&n.detail, ((room - title_room - pad).max(0.0) / d.advance(small)) as usize);
            let dw = d.advance(small) * detail.chars().count() as f32;
            d.text(o.0 + w - pad - fold_w - dw, o.1 + (head - small * 1.25) * 0.5, small, &detail, t.color.muted);
        }
    }
    // the preview: the picture of what it plays, under its pins
    if let (true, false, Some(key)) = (v.motion.previews, n.collapsed, n.picture.as_deref()) {
        let ph = (5.0 * cam.unit - t.size.pad) * z;
        let pw = (ph * 264.0 / 168.0).min(w - 2.0 * pad);
        let (px, py) = (o.0 + (w - pw) * 0.5, o.1 + h - ph - pad * 0.5);
        match v.pictures.get(key) {
            Some(id) => d.picture(px, py, pw, ph, *id),
            None => {
                d.round_rect(px, py, pw, ph, t.size.radius * z, t.color.within);
                if z >= 0.55 {
                    d.text(px + pad * 0.6, py + pad * 0.4, small, &controls::fit(key, (pw / d.advance(small)) as usize), t.color.muted);
                }
            }
        }
    }
    let r = t.size.rem(0.32) * z;
    let flow = cam.dir.flow();
    let labels = z >= 0.55 && !n.collapsed;
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
            // a label keeps to its pin's share of the edge
            let max = if cam.dir == Direction::Lr { w * 0.45 } else { w / pins.len() as f32 * 0.95 };
            let label = controls::fit(&pin.label, (max / d.advance(small)) as usize);
            let lw = d.advance(small) * label.chars().count() as f32;
            let (lx, ly) = match (cam.dir, inlet) {
                (Direction::Lr, true) => (q.0 + r + pad * 0.6, q.1 - small * 0.62),
                (Direction::Lr, false) => (q.0 - r - pad * 0.6 - lw, q.1 - small * 0.62),
                (Direction::Tb, true) => (q.0 - lw * 0.5, q.1 - r - small * 1.4),
                (Direction::Tb, false) => (q.0 - lw * 0.5, q.1 + r + small * 0.2),
            };
            d.text(lx, ly, small, &label, t.color.muted);
        }
    }
    if v.selected.contains(&n.id) {
        d.outline(o.0 - 1.5, o.1 - 1.5, w + 3.0, h + 3.0, (radius + 1.5, 1.5), main);
    }
    if let Some(a) = v.flashing(&n.id) {
        let k = 1.0 - a;
        d.outline(o.0 - 3.0 * k, o.1 - 3.0 * k, w + 6.0 * k, h + 6.0 * k, (radius + 3.0 * k, 1.0 + 2.5 * k), faded(main, k));
        // the skeleton shimmer: a light band sweeping across once
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
    Boxed { body, fold }
}

/// note lines: `text` broken into lines of at most `per` characters, at spaces where it can
fn wrap(text: &str, per: usize) -> Vec<String> {
    let per = per.max(4);
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > per {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
        while line.chars().count() > per {
            let rest: String = line.chars().skip(per).collect();
            lines.push(line.chars().take(per).collect());
            line = rest;
        }
    }
    lines.push(line);
    lines
}

/// The row's notes: neutral grey (the theme's ground and ink mixed: light at day, dark at night), the author small
/// above; beside their node, else in a column after the graph. `editing` = the text while a note is typed. The presses.
pub fn notes(d: &mut DrawList, t: &Tokens, (doc, drawn, bounds): (&Canvas, &[(String, Rect)], Rect), cam: &Cam, editing: &dyn Fn(&str) -> Option<String>) -> Vec<(usize, Rect)> {
    let (z, u) = (cam.z(), cam.unit);
    let (small, pad) = (t.size.small * z, t.size.pad * z);
    let ground = mix(t.color.surface, t.color.text, 0.13);
    let mut free_y = bounds.y;
    let mut out = Vec::new();
    for (k, note) in doc.notes.iter().enumerate() {
        let typed = editing(&note.id);
        let text = typed.clone().map_or_else(|| note.text.clone(), |s| format!("{s}|"));
        let w = 11.0 * u;
        let lines = wrap(if text.is_empty() { "…" } else { &text }, ((w * z - 2.0 * pad) / d.advance(small)) as usize);
        let hgt = (pad * 2.0 + small * 1.3 * (lines.len() as f32 + 1.0)) / z;
        let at = note.node.as_ref().and_then(|id| drawn.iter().find(|(x, _)| x == id)).map(|(_, r)| (r.x + u, r.y + r.h + 0.5 * u));
        let at = at.unwrap_or_else(|| {
            let p = (bounds.x + bounds.w + 2.0 * u, free_y);
            free_y += hgt + u;
            p
        });
        let r = cam.rect(Rect { x: at.0, y: at.1, w, h: hgt });
        d.round_rect(r.x, r.y, r.w, r.h, t.size.radius * z, ground);
        d.outline(r.x, r.y, r.w, r.h, (t.size.radius * z, 1.0), if typed.is_some() { t.color.text } else { t.color.line });
        if z >= 0.4 {
            let author = if note.author.is_empty() { "note" } else { note.author.as_str() };
            d.text(r.x + pad, r.y + pad * 0.7, small * 0.85, author, t.color.muted);
            for (i, line) in lines.iter().enumerate() {
                d.sans(r.x + pad, r.y + pad + small * 1.3 * (i as f32 + 1.0), small, line, t.color.text);
            }
        }
        out.push((k, r));
    }
    out
}

/// a wire's value beside the pointer, in its colour
pub fn value_label(d: &mut DrawList, t: &Tokens, l: &Link, cursor: (f32, f32)) {
    let Some(v) = l.value.as_deref() else { return };
    let px = t.size.small;
    let w = d.advance(px) * v.chars().count() as f32 + 2.0 * t.size.pad;
    let (x, y) = (cursor.0 + 12.0, cursor.1 - px * 2.4);
    let c = t.color.signals.of(l.signal);
    d.round_rect(x, y, w, px * 1.7, t.size.radius, t.color.control);
    d.outline(x, y, w, px * 1.7, (t.size.radius, 1.0), faded(c, 0.8));
    d.text(x + t.size.pad, y + px * 0.2, px, v, t.color.text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reveal_fades_in_then_out_and_goes() {
        let mut r: Vec<Reveal> = Vec::new();
        follow(&mut r, Some(("node:a".into(), [1.0; 4])), 0.0);
        assert_eq!(r[0].strength(0.0), 0.0);
        assert_eq!(r[0].strength(FADE_IN), 1.0);
        follow(&mut r, Some(("node:b".into(), [1.0; 4])), 1.0);
        assert_eq!(r.len(), 2, "a fades out while b fades in");
        assert!(r[0].strength(1.0 + FADE_OUT * 0.5) > 0.0 && r[0].strength(1.0 + FADE_OUT * 0.5) < 1.0);
        follow(&mut r, None, 1.0 + FADE_OUT + 0.01);
        assert!(r.iter().all(|x| x.key != "node:a"), "faded: gone");
        follow(&mut r, Some(("node:b".into(), [1.0; 4])), 1.0 + FADE_OUT + 0.1);
        assert!(r[0].left.is_none() && r[0].strength(1.0 + FADE_OUT + 0.1) > 0.0, "back on it: fades in from where it was");
    }

    #[test]
    fn a_hovered_wire_tells_its_value() {
        let t = Tokens::new(deck_kit::base::tokens::Theme::Night, 16.0);
        let mut d = DrawList::new(0.6);
        let mut l = Link::new((0, 0), (1, 1), deck_kit::base::signal::Signal::Control);
        value_label(&mut d, &t, &l, (100.0, 100.0));
        assert!(d.texts().is_empty(), "no value: no label");
        l.value = Some("mood = \"shy\"".into());
        value_label(&mut d, &t, &l, (100.0, 100.0));
        assert_eq!(d.texts(), ["mood = \"shy\""]);
    }

    #[test]
    fn a_note_breaks_at_spaces() {
        assert_eq!(wrap("louder here please", 8), ["louder", "here", "please"]);
        assert_eq!(wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(wrap("", 8), [""]);
    }
}
