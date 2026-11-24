//! The colour picker, after the OSCT editor's: a swatch beside the hex opens a popover – a palette of the usual keys, the
//! recent colours, a saturation / brightness area over a hue strip, the hex and the eyedropper. The kit holds the model and
//! the look; the value, its checks and where a pick goes stay with the caller (a background's colour: deck-catalog).
use crate::base::draw::{DrawList, Rect};
use crate::base::tokens::Tokens;
use crate::editor::controls;

pub type Rgb = [u8; 3];

/// `#rrggbb`, lower case
pub fn hex([r, g, b]: Rgb) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// `#rrggbb` (or `rrggbb`) as bytes; anything else: none
pub fn rgb(hex: &str) -> Option<Rgb> {
    let h = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
    if h.len() != 6 || !h.is_ascii() {
        return None;
    }
    let at = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some([at(0)?, at(2)?, at(4)?])
}

/// display values 0–1 (deck-catalog's reading of a hex) as bytes
pub fn bytes(c: [f32; 3]) -> Rgb {
    c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

pub fn rgba(c: Rgb, alpha: f32) -> [f32; 4] {
    [f32::from(c[0]) / 255.0, f32::from(c[1]) / 255.0, f32::from(c[2]) / 255.0, alpha]
}

/// a colour as hue (0–360), saturation and brightness (0–1)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

impl Hsv {
    pub fn of(c: Rgb) -> Hsv {
        let [r, g, b] = c.map(|v| f32::from(v) / 255.0);
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        let d = max - min;
        let h = if d == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        Hsv { h, s: if max == 0.0 { 0.0 } else { d / max }, v: max }
    }

    pub fn rgb(self) -> Rgb {
        let f = |n: f32| {
            let k = (n + self.h / 60.0).rem_euclid(6.0);
            self.v - self.v * self.s * k.min(4.0 - k).clamp(0.0, 1.0)
        };
        bytes([f(5.0), f(3.0), f(1.0)])
    }
}

/// the colours offered first: the keys a chroma key meets, white and black, the plain primaries
pub const PALETTE: [(&str, Rgb); 10] = [
    ("green screen", [0x00, 0xff, 0x00]),
    ("chroma green", [0x00, 0xb1, 0x40]),
    ("blue screen", [0x00, 0x00, 0xff]),
    ("chroma blue", [0x00, 0x47, 0xbb]),
    ("white", [0xff, 0xff, 0xff]),
    ("black", [0x00, 0x00, 0x00]),
    ("grey", [0x80, 0x80, 0x80]),
    ("red", [0xff, 0x00, 0x00]),
    ("yellow", [0xff, 0xff, 0x00]),
    ("magenta", [0xff, 0x00, 0xff]),
];

/// the last picked colours, newest first
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Recent(pub Vec<Rgb>);

impl Recent {
    pub const MAX: usize = 8;

    pub fn remember(&mut self, c: Rgb) {
        self.0.retain(|x| *x != c);
        self.0.insert(0, c);
        self.0.truncate(Self::MAX);
    }
}

/// a part of the popover you can press
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Part {
    Swatch(Rgb),
    Area,
    Hue,
    Hex,
    Dropper,
}

/// what a press or a drag in the popover asks for
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Out {
    /// a whole colour chosen (a swatch, a drag let go)
    Pick(Rgb),
    /// the area or the hue moving: show it, keep it when let go
    Moving(Rgb),
    /// type the hex
    Hex,
    /// pick from the stage
    Dropper,
}

/// where the popover landed
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Placed {
    pub whole: Rect,
    pub parts: Vec<(Rect, Part)>,
}

impl Placed {
    pub fn at(&self, x: f32, y: f32) -> Option<(Rect, Part)> {
        self.parts.iter().rev().find(|(r, _)| r.has(x, y)).copied()
    }

    fn rect(&self, part: Part) -> Option<Rect> {
        self.parts.iter().find(|(_, p)| *p == part).map(|(r, _)| *r)
    }
}

/// The popover's own state: it keeps the hue while saturation or brightness is 0.
#[derive(Debug, Clone, PartialEq)]
pub struct Picker {
    pub hsv: Hsv,
    /// what it picked last: an echo of its own pick does not move it
    last: Option<Rgb>,
    drag: Option<Part>,
}

impl Picker {
    pub fn new(value: Option<Rgb>) -> Picker {
        Picker { hsv: Hsv::of(value.unwrap_or_default()), last: value, drag: None }
    }

    /// a colour set from outside (typed, an undo) moves it
    pub fn follow(&mut self, value: Option<Rgb>) {
        if let Some(c) = value.filter(|c| Some(*c) != self.last && *c != self.hsv.rgb()) {
            self.hsv = Hsv::of(c);
            self.last = Some(c);
        }
    }

    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub fn press(&mut self, placed: &Placed, x: f32, y: f32) -> Option<Out> {
        let (_, part) = placed.at(x, y)?;
        match part {
            Part::Swatch(c) => {
                self.hsv = Hsv::of(c);
                Some(self.picked(c))
            }
            Part::Area | Part::Hue => {
                self.drag = Some(part);
                self.drag_to(placed, x, y)
            }
            Part::Hex => Some(Out::Hex),
            Part::Dropper => Some(Out::Dropper),
        }
    }

    pub fn drag_to(&mut self, placed: &Placed, x: f32, y: f32) -> Option<Out> {
        let part = self.drag?;
        let r = placed.rect(part)?;
        let fx = ((x - r.x) / r.w).clamp(0.0, 1.0);
        match part {
            Part::Area => {
                self.hsv.s = fx;
                self.hsv.v = 1.0 - ((y - r.y) / r.h).clamp(0.0, 1.0);
            }
            _ => self.hsv.h = fx * 360.0,
        }
        Some(Out::Moving(self.hsv.rgb()))
    }

    /// the drag let go: the colour it ended on
    pub fn release(&mut self) -> Option<Out> {
        self.drag.take()?;
        let c = self.hsv.rgb();
        Some(self.picked(c))
    }

    fn picked(&mut self, c: Rgb) -> Out {
        self.last = Some(c);
        Out::Pick(c)
    }
}

/// The swatch row: its label, a round swatch (`None`: no colour, struck through) and the value beside it; the swatch's
/// place and the value's (a press types it). Moves `y` on.
pub fn swatch_field(d: &mut DrawList, t: &Tokens, area: Rect, y: &mut f32, label: &str, value: &str, swatch: Option<Rgb>) -> (Rect, Rect) {
    let (font, h) = (t.size.font, t.size.tree_row);
    let at = *y + (h - font * 1.25) * 0.5;
    d.sans(area.x, at, font, label, t.color.muted);
    let x = area.x + (area.w * 0.42).min(96.0);
    let s = h * 0.7;
    let sw = Rect { x, y: *y + (h - s) * 0.5, w: s, h: s };
    swatch_mark(d, t, sw, swatch, false);
    let vx = x + s + t.size.pad * 0.75;
    let room = ((area.x + area.w - vx) / d.advance(font)).max(0.0) as usize;
    d.text(vx, at, font, &controls::fit(value, room), t.color.text);
    *y += h;
    (sw, Rect { x: vx, y: at - 2.0, w: area.x + area.w - vx, h: font * 1.25 + 4.0 })
}

fn swatch_mark(d: &mut DrawList, t: &Tokens, r: Rect, c: Option<Rgb>, on: bool) {
    let round = r.h * 0.5;
    match c {
        Some(c) => d.round_rect(r.x, r.y, r.w, r.h, round, rgba(c, 1.0)),
        None => d.segment((r.x + r.w * 0.25, r.y + r.h * 0.75), (r.x + r.w * 0.75, r.y + r.h * 0.25), t.size.edge.max(1.0), t.color.muted),
    }
    d.outline(r.x, r.y, r.w, r.h, (round, if on { 2.0 } else { 1.0 }), if on { t.color.text } else { t.color.line });
}

/// The popover under `anchor` (above it when the room ends), kept inside `room`; where its parts landed.
pub fn popover(d: &mut DrawList, t: &Tokens, (anchor, room): (Rect, (f32, f32)), picker: &Picker, recent: &[Rgb], current: Option<Rgb>, pointer: (f32, f32)) -> Placed {
    let (pad, small, cell) = (t.size.pad, t.size.small, t.size.tree_row * 0.8);
    let gap = pad * 0.4;
    // the palette on one line
    let w = (2.0 * pad + PALETTE.len() as f32 * (cell + gap) - gap).max(t.size.rem(15.0));
    let per_line = ((w - 2.0 * pad + gap) / (cell + gap)).floor().max(1.0) as usize;
    let lines = |n: usize| n.div_ceil(per_line) as f32;
    let head = small * 1.5;
    let swatch_block = |n: usize| head + lines(n) * (cell + gap);
    let (area_h, hue_h, hex_h) = (t.size.rem(8.0), t.size.rem(1.0), t.size.row);
    let h = pad + swatch_block(PALETTE.len()) + if recent.is_empty() { 0.0 } else { swatch_block(recent.len()) } + area_h + gap * 2.0 + hue_h + gap * 2.0 + hex_h + pad;
    let x = anchor.x.min(room.0 - w - 8.0).max(8.0);
    let below = anchor.y + anchor.h + 4.0;
    let y = if below + h > room.1 - 8.0 { (anchor.y - 4.0 - h).max(8.0) } else { below };
    let whole = Rect { x, y, w, h };
    d.round_rect(x, y, w, h, t.size.radius * 2.0, t.color.popover());
    d.outline(x, y, w, h, (t.size.radius * 2.0, 1.0), t.color.line);
    let mut out = Placed { whole, parts: Vec::new() };
    let (ix, iw) = (x + pad, w - 2.0 * pad);
    let mut cy = y + pad;
    let palette: Vec<Rgb> = PALETTE.iter().map(|(_, c)| *c).collect();
    for (name, list) in [("palette", palette.as_slice()), ("recent", recent)] {
        if list.is_empty() {
            continue;
        }
        controls::label(d, t, ix, cy, name, t.color.muted);
        cy += head;
        for (i, c) in list.iter().enumerate() {
            let r = Rect { x: ix + (i % per_line) as f32 * (cell + gap), y: cy + (i / per_line) as f32 * (cell + gap), w: cell, h: cell };
            if r.has(pointer.0, pointer.1) {
                d.round_rect(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0, t.size.radius + 2.0, t.color.hover);
            }
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, rgba(*c, 1.0));
            let on = current == Some(*c);
            d.outline(r.x, r.y, r.w, r.h, (t.size.radius, if on { 2.0 } else { 1.0 }), if on { t.color.text } else { t.color.line });
            out.parts.push((r, Part::Swatch(*c)));
        }
        cy += lines(list.len()) * (cell + gap);
    }
    let hsv = picker.hsv;
    let area = Rect { x: ix, y: cy, w: iw, h: area_h };
    let pure = rgba(Hsv { h: hsv.h, s: 1.0, v: 1.0 }.rgb(), 1.0);
    let (white, clear, black) = ([1.0; 4], [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]);
    d.gradient(area.x, area.y, area.w, area.h, [white, pure, pure, white]);
    d.gradient(area.x, area.y, area.w, area.h, [clear, clear, black, black]);
    d.outline(area.x, area.y, area.w, area.h, (0.0, 1.0), t.color.line);
    let (dx, dy) = (area.x + hsv.s * area.w, area.y + (1.0 - hsv.v) * area.h);
    d.ring(dx, dy, 6.0, [0.0, 0.0, 0.0, 1.0]);
    d.ring(dx, dy, 5.0, [1.0; 4]);
    out.parts.push((area, Part::Area));
    cy += area_h + gap * 2.0;
    let hue = Rect { x: ix, y: cy, w: iw, h: hue_h };
    let step = hue.w / 6.0;
    for k in 0..6 {
        let a = rgba(Hsv { h: k as f32 * 60.0, s: 1.0, v: 1.0 }.rgb(), 1.0);
        let b = rgba(Hsv { h: (k + 1) as f32 * 60.0 % 360.0, s: 1.0, v: 1.0 }.rgb(), 1.0);
        d.gradient(hue.x + k as f32 * step, hue.y, step, hue.h, [a, b, b, a]);
    }
    d.outline(hue.x, hue.y, hue.w, hue.h, (0.0, 1.0), t.color.line);
    let hx = hue.x + hsv.h / 360.0 * hue.w;
    d.round_rect(hx - 3.0, hue.y - 3.0, 6.0, hue.h + 6.0, 2.0, t.color.surface);
    d.outline(hx - 3.0, hue.y - 3.0, 6.0, hue.h + 6.0, (2.0, 1.0), t.color.text);
    // the area and the strip take a press a little outside them, as a slider's thumb would
    out.parts.push((Rect { x: hue.x - 4.0, y: hue.y - 4.0, w: hue.w + 8.0, h: hue.h + 8.0 }, Part::Hue));
    cy += hue_h + gap * 2.0;
    let dropper = Rect { x: ix, y: cy, w: hex_h, h: hex_h };
    controls::icon_button(d, t, dropper, "droplet", t.color.text, pointer);
    d.outline(dropper.x, dropper.y, dropper.w, dropper.h, (t.size.radius, 1.0), t.color.line);
    out.parts.push((dropper, Part::Dropper));
    let field = Rect { x: ix + hex_h + gap * 2.0, y: cy, w: iw - hex_h - gap * 2.0, h: hex_h };
    d.round_rect(field.x, field.y, field.w, field.h, t.size.radius, t.color.control);
    d.outline(field.x, field.y, field.w, field.h, (t.size.radius, t.size.edge), t.color.line);
    let text = current.map_or_else(String::new, hex);
    d.text(field.x + pad * 0.75, field.y + (field.h - t.size.font * 1.25) * 0.5, t.size.font, &text, t.color.text);
    out.parts.push((field, Part::Hex));
    out
}

/// The eyedropper at the pointer: a ring around it, a chip with the colour under it and its hex (none yet: a dash).
pub fn dropper(d: &mut DrawList, t: &Tokens, (x, y): (f32, f32), room: (f32, f32), under: Option<Rgb>) {
    let ink = under.map_or(t.color.muted, |c| rgba(c, 1.0));
    d.ring(x, y, 9.0, [0.0, 0.0, 0.0, 1.0]);
    d.ring(x, y, 7.5, [1.0; 4]);
    let text = under.map_or_else(|| "–".to_string(), hex);
    let (font, s) = (t.size.font, t.size.tree_row * 0.6);
    let w = s + t.size.pad * 1.5 + d.advance(font) * text.chars().count() as f32 + t.size.pad;
    let h = t.size.tree_row;
    let cx = (x + 16.0).min(room.0 - w - 8.0).max(8.0);
    let cy = if y + 16.0 + h > room.1 - 8.0 { y - 16.0 - h } else { y + 16.0 };
    d.round_rect(cx, cy, w, h, t.size.radius, t.color.surface_head);
    d.outline(cx, cy, w, h, (t.size.radius, 1.0), t.color.line);
    d.round_rect(cx + t.size.pad * 0.5, cy + (h - s) * 0.5, s, s, s * 0.5, ink);
    d.text(cx + s + t.size.pad, cy + (h - font * 1.25) * 0.5, font, &text, t.color.text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_and_hsv_go_both_ways() {
        assert_eq!(hex([255, 128, 0]), "#ff8000");
        assert_eq!(rgb("#ff8000"), Some([255, 128, 0]));
        assert_eq!(rgb("5FB3FF"), Some([0x5f, 0xb3, 0xff]));
        assert_eq!(rgb("#ff80"), None);
        assert_eq!(rgb("#gg8000"), None);
        assert_eq!(bytes([1.0, 128.0 / 255.0, 0.0]), [255, 128, 0]);
        let red = Hsv::of([255, 0, 0]);
        assert_eq!((red.h, red.s, red.v), (0.0, 1.0, 1.0));
        assert!((Hsv::of([0, 0, 255]).h - 240.0).abs() < 1e-3);
        assert_eq!(Hsv::of([0, 0, 0]), Hsv { h: 0.0, s: 0.0, v: 0.0 });
        for c in [[0x33, 0x66, 0x99], [0, 0xb1, 0x40], [0xff, 0xff, 0xff], [1, 2, 3], [0xf3, 0xcc, 0x94], [0xff, 0x00, 0xfe]] {
            assert_eq!(Hsv::of(c).rgb(), c, "{}", hex(c));
        }
    }

    #[test]
    fn recent_colours_are_newest_first_without_repeats() {
        let mut r = Recent::default();
        for i in 0..10u8 {
            r.remember([i, 0, 0]);
        }
        assert_eq!(r.0.len(), Recent::MAX);
        assert_eq!(r.0[0], [9, 0, 0]);
        r.remember([5, 0, 0]);
        assert_eq!(r.0[..2], [[5, 0, 0], [9, 0, 0]]);
        assert_eq!(r.0.iter().filter(|c| **c == [5, 0, 0]).count(), 1);
    }

    fn placed(picker: &Picker, recent: &[Rgb]) -> Placed {
        popover(&mut DrawList::new(0.6), &Tokens::default(), (Rect { x: 100.0, y: 100.0, w: 20.0, h: 20.0 }, (1920.0, 1080.0)), picker, recent, None, (0.0, 0.0))
    }

    #[test]
    fn a_swatch_picks_and_the_area_drags() {
        let mut p = Picker::new(Some([0x33, 0x66, 0x99]));
        let at = placed(&p, &[[1, 2, 3]]);
        let swatches = at.parts.iter().filter(|(_, x)| matches!(x, Part::Swatch(_))).count();
        assert_eq!(swatches, PALETTE.len() + 1, "the palette and the recent one");
        let (r, _) = at.parts.iter().find(|(_, x)| *x == Part::Swatch(PALETTE[0].1)).copied().unwrap();
        assert_eq!(p.press(&at, r.x + 1.0, r.y + 1.0), Some(Out::Pick(PALETTE[0].1)));
        let area = at.rect(Part::Area).unwrap();
        assert_eq!(p.press(&at, area.x + area.w - 0.01, area.y), Some(Out::Moving(Hsv { h: p.hsv.h, s: 1.0, v: 1.0 }.rgb())));
        assert!(p.dragging());
        assert!(matches!(p.drag_to(&at, area.x - 50.0, area.y + area.h + 50.0), Some(Out::Moving([0, 0, 0]))), "held to its edges");
        assert_eq!(p.release(), Some(Out::Pick([0, 0, 0])));
        assert_eq!(p.release(), None);
        assert!((p.hsv.h - 120.0).abs() < 1e-3, "black keeps the hue it had");
    }

    #[test]
    fn the_hue_strip_turns_the_hue_and_an_echo_does_not_move_it() {
        let mut p = Picker::new(Some([255, 0, 0]));
        let at = placed(&p, &[]);
        let hue = at.rect(Part::Hue).unwrap();
        let half = hue.x + hue.w * 0.5;
        assert!(matches!(p.press(&at, half, hue.y + 5.0), Some(Out::Moving(_))));
        let Some(Out::Pick(c)) = p.release() else { panic!("a pick") };
        let h = p.hsv.h;
        p.follow(Some(c));
        assert_eq!(p.hsv.h, h, "its own pick comes back");
        p.follow(Some([0, 0, 255]));
        assert!((p.hsv.h - 240.0).abs() < 1e-3, "a colour from outside moves it");
    }

    #[test]
    fn it_stays_on_screen_and_offers_the_hex_and_the_dropper() {
        let p = Picker::new(None);
        let mut d = DrawList::new(0.6);
        let at = popover(&mut d, &Tokens::default(), (Rect { x: 1900.0, y: 1060.0, w: 20.0, h: 20.0 }, (1920.0, 1080.0)), &p, &[], None, (0.0, 0.0));
        assert!(at.whole.x + at.whole.w <= 1920.0 && at.whole.y + at.whole.h <= 1080.0 && at.whole.y >= 0.0, "{:?}", at.whole);
        assert!(at.rect(Part::Hex).is_some() && at.rect(Part::Dropper).is_some());
        assert_eq!(p.clone().press(&at, at.whole.x + 1.0, at.whole.y + 1.0), None, "its ground is no part");
    }

    #[test]
    fn the_swatch_field_has_a_swatch_and_a_value() {
        let (t, mut d, mut y) = (Tokens::default(), DrawList::new(0.6), 10.0);
        let (sw, value) = swatch_field(&mut d, &t, Rect { x: 0.0, y: 0.0, w: 300.0, h: 400.0 }, &mut y, "background", "#336699", Some([0x33, 0x66, 0x99]));
        assert!(sw.x + sw.w < value.x, "the swatch, then the hex");
        assert_eq!(y, 10.0 + t.size.tree_row);
        assert!(d.texts().contains(&"#336699"));
    }
}
