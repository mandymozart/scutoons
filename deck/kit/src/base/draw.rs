//! What a frame of UI draws (#456): a list of primitives in logical pixels, in painter's order. The kit has no GPU: an
//! adapter turns the list into draws. Shapes are exact at any size (a distance per pixel in the adapter), so a dot does not
//! depend on a texture. The text of a frame lives in one buffer, so a frame allocates nothing once it has run.
use crate::base::tokens::contrast;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Prim {
    /// a filled rectangle, its corners rounded by `r`
    Rect { x: f32, y: f32, w: f32, h: f32, r: f32, color: [f32; 4] },
    /// the outline of one, `thick` wide
    Outline { x: f32, y: f32, w: f32, h: f32, r: f32, thick: f32, color: [f32; 4] },
    /// `start..end` = a byte range of the list's text buffer; `spacing` = extra room after each letter, in em; `halo` =
    /// its own ground on a see-through panel (alpha 0: none): light ink is ringed with it, dark ink sits on a strip of it
    Text { x: f32, y: f32, px: f32, start: usize, end: usize, color: [f32; 4], spacing: f32, halo: [f32; 4] },
    /// an icon by name, in a square of `size`
    Icon { x: f32, y: f32, size: f32, name: &'static str, color: [f32; 4], halo: [f32; 4] },
    /// a frame of the animated logo, in a square of `size`
    Logo { x: f32, y: f32, size: f32, frame: usize },
    /// what follows shows only inside `[x, y, w, h]` (a scrolled panel's content); none = the whole screen again
    Clip(Option<[f32; 4]>),
    /// a straight stroke from `a` to `b`, `thick` wide
    Segment { a: (f32, f32), b: (f32, f32), thick: f32, color: [f32; 4] },
}

/// Where a control landed, in logical pixels (what can be pressed is kept by its caller as these).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn has(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// A frame's primitives and their text (a draw list moves between threads: deck-show paints a dialog on the flash side's).
#[derive(Debug, Clone, PartialEq)]
pub struct DrawList {
    prims: Vec<Prim>,
    text: String,
    /// a character's advance per pixel of font size (the mono face's)
    per_px: f32,
    halo: [f32; 4],
    /// the first primitive drawn on the see-through ground
    halo_from: usize,
}

/// no halo
pub const NO_HALO: [f32; 4] = [0.0; 4];
/// below this contrast to the ink a halo would not set it apart: the ink is on a ground of its own
const HALO_CONTRAST: f32 = 3.0;

impl DrawList {
    pub fn new(per_px: f32) -> DrawList {
        DrawList { prims: Vec::with_capacity(4096), text: String::with_capacity(8192), per_px, halo: NO_HALO, halo_from: 0 }
    }

    pub fn clear(&mut self) {
        self.prims.clear();
        self.text.clear();
        self.halo = NO_HALO;
    }

    /// what is drawn next on a see-through ground: its text and icons get an outline of `ground` (opaque), unless their
    /// ink is on a ground of its own; `None` ends it
    pub fn halo(&mut self, ground: Option<[f32; 4]>) {
        self.halo = ground.map_or(NO_HALO, |g| [g[0], g[1], g[2], 1.0]);
        self.halo_from = self.prims.len();
    }

    /// ink at (x, y) gets the halo when it is on the see-through ground: not on an opaque ground drawn since, and set apart by it
    fn halo_for(&self, ink: [f32; 4], (x, y): (f32, f32)) -> [f32; 4] {
        if self.halo[3] <= 0.0 || contrast(ink, self.halo) < HALO_CONTRAST {
            return NO_HALO;
        }
        let covered = self.prims[self.halo_from..].iter().any(|p| matches!(*p, Prim::Rect { x: rx, y: ry, w, h, color, .. } if color[3] >= 0.99 && Rect { x: rx, y: ry, w, h }.has(x, y)));
        if covered {
            NO_HALO
        } else {
            self.halo
        }
    }

    /// back to the first `n` primitives (a part drawn again: its text stays in the buffer until the frame is cleared)
    pub fn truncate(&mut self, n: usize) {
        self.prims.truncate(n);
    }

    pub fn prims(&self) -> &[Prim] {
        &self.prims
    }

    /// the text of a `Prim::Text`
    pub fn text_of(&self, start: usize, end: usize) -> &str {
        &self.text[start..end]
    }

    /// every text of the list, in painter's order: what a layer says (tests, the headless host)
    pub fn texts(&self) -> Vec<&str> {
        self.prims.iter().filter_map(|p| if let Prim::Text { start, end, .. } = *p { Some(self.text_of(start, end)) } else { None }).collect()
    }

    /// a character's advance at font size `px`
    pub fn advance(&self, px: f32) -> f32 {
        self.per_px * px
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        self.prims.push(Prim::Rect { x, y, w, h, r: 0.0, color });
    }

    pub fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: [f32; 4]) {
        self.prims.push(Prim::Rect { x, y, w, h, r, color });
    }

    pub fn outline(&mut self, x: f32, y: f32, w: f32, h: f32, (r, thick): (f32, f32), color: [f32; 4]) {
        self.prims.push(Prim::Outline { x, y, w, h, r, thick, color });
    }

    /// `text` from (x, y), its top left, at font size `px`; the width drawn
    pub fn text(&mut self, x: f32, y: f32, px: f32, text: &str, color: [f32; 4]) -> f32 {
        self.spaced(x, y, px, text, color, 0.0)
    }

    /// `text` with `spacing` em of extra room after each letter; the width drawn
    pub fn spaced(&mut self, x: f32, y: f32, px: f32, text: &str, color: [f32; 4], spacing: f32) -> f32 {
        let start = self.text.len();
        self.text.push_str(text);
        let halo = self.halo_for(color, (x + px * 0.3, y + px * 0.6));
        self.prims.push(Prim::Text { x, y, px, start, end: self.text.len(), color, spacing, halo });
        self.step(px, spacing) * text.chars().count() as f32
    }

    /// a letter's step at font size `px` with `spacing` em between letters
    pub fn step(&self, px: f32, spacing: f32) -> f32 {
        self.advance(px) + spacing * px
    }

    pub fn icon(&mut self, x: f32, y: f32, size: f32, name: &'static str, color: [f32; 4]) {
        let halo = self.halo_for(color, (x + size * 0.5, y + size * 0.5));
        self.prims.push(Prim::Icon { x, y, size, name, color, halo });
    }

    pub fn logo(&mut self, x: f32, y: f32, size: f32, frame: usize) {
        self.prims.push(Prim::Logo { x, y, size, frame });
    }

    /// the rectangle the primitives from `from` on cover (a text as its line, `LINE` × its size high): what a content took
    pub fn extent(&self, from: usize) -> Option<Rect> {
        let mut out: Option<(f32, f32, f32, f32)> = None;
        for p in self.prims.get(from..).unwrap_or_default() {
            let r = match *p {
                Prim::Rect { x, y, w, h, .. } | Prim::Outline { x, y, w, h, .. } => (x, y, x + w, y + h),
                Prim::Text { x, y, px, start, end, spacing, .. } => (x, y, x + self.step(px, spacing) * self.text[start..end].chars().count() as f32, y + px * 1.25),
                Prim::Icon { x, y, size, .. } | Prim::Logo { x, y, size, .. } => (x, y, x + size, y + size),
                Prim::Segment { a, b, .. } => (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)),
                Prim::Clip(_) => continue,
            };
            out = Some(out.map_or(r, |o| (o.0.min(r.0), o.1.min(r.1), o.2.max(r.2), o.3.max(r.3))));
        }
        out.map(|(x0, y0, x1, y1)| Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
    }

    /// what is drawn next shows only inside `r` (none: everywhere again)
    pub fn clip(&mut self, r: Option<Rect>) {
        self.prims.push(Prim::Clip(r.map(|r| [r.x, r.y, r.w, r.h])));
    }

    /// a filled circle around (x, y)
    pub fn disc(&mut self, x: f32, y: f32, r: f32, color: [f32; 4]) {
        self.round_rect(x - r, y - r, 2.0 * r, 2.0 * r, r, color);
    }

    /// a circle's outline around (x, y)
    pub fn ring(&mut self, x: f32, y: f32, r: f32, color: [f32; 4]) {
        self.outline(x - r, y - r, 2.0 * r, 2.0 * r, (r, 2.0), color);
    }

    /// a straight stroke, however long (a grid's line)
    pub fn segment(&mut self, a: (f32, f32), b: (f32, f32), thick: f32, color: [f32; 4]) {
        self.prims.push(Prim::Segment { a, b, thick, color });
    }

    /// a thin straight line of beads
    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), r: f32, color: [f32; 4]) {
        let n = (((b.0 - a.0).hypot(b.1 - a.1) / r) as usize).clamp(2, 64);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            self.disc(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, r, color);
        }
    }

    /// a cable being dragged: beads along a hanging curve
    pub fn cable(&mut self, a: (f32, f32), b: (f32, f32), color: [f32; 4]) {
        const BEADS: usize = 56;
        let sag = 40.0 + (b.0 - a.0).abs().max((b.1 - a.1).abs()) * 0.35;
        for i in 0..=BEADS {
            let t = i as f32 / BEADS as f32;
            let s = 1.0 - t;
            let (c1, c2) = ((a.0, a.1 + sag), (b.0, b.1 + sag));
            let x = s * s * s * a.0 + 3.0 * s * s * t * c1.0 + 3.0 * s * t * t * c2.0 + t * t * t * b.0;
            let y = s * s * s * a.1 + 3.0 * s * s * t * c1.1 + 3.0 * s * t * t * c2.1 + t * t * t * b.1;
            self.disc(x, y, 3.0, color);
        }
    }
}

/// A QR code (style-free – both component modules draw it: the deck's pairing, the editor's Session panel, #531): `n × n` modules (`dark(x, y)`), `module` logical pixels each, black on a white square with the quiet zone
/// (4 modules) inside it, from its top left. Its side.
pub fn qr(d: &mut DrawList, (x, y): (f32, f32), module: f32, n: usize, dark: &dyn Fn(usize, usize) -> bool) -> f32 {
    let side = (n + 8) as f32 * module;
    d.rect(x, y, side, side, [1.0; 4]);
    for my in 0..n {
        for mx in 0..n {
            if dark(mx, my) {
                d.rect(x + (mx + 4) as f32 * module, y + (my + 4) as f32 * module, module, module, [0.0, 0.0, 0.0, 1.0]);
            }
        }
    }
    side
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_comes_back_from_the_buffer_and_tells_its_width() {
        let mut d = DrawList::new(0.5);
        assert_eq!(d.text(0.0, 0.0, 10.0, "Bühne", [1.0; 4]), 25.0);
        let Prim::Text { start, end, .. } = d.prims()[0] else { panic!("a text") };
        assert_eq!(d.text_of(start, end), "Bühne");
    }

    #[test]
    fn clearing_starts_a_frame_over() {
        let mut d = DrawList::new(0.5);
        d.rect(0.0, 0.0, 1.0, 1.0, [1.0; 4]);
        d.text(0.0, 0.0, 10.0, "x", [1.0; 4]);
        d.clear();
        assert!(d.prims().is_empty());
        d.text(0.0, 0.0, 10.0, "y", [1.0; 4]);
        let Prim::Text { start, end, .. } = d.prims()[0] else { panic!("a text") };
        assert_eq!(d.text_of(start, end), "y");
    }

    #[test]
    fn the_extent_covers_what_was_drawn_from_a_point_on() {
        let mut d = DrawList::new(0.5);
        d.rect(-50.0, -50.0, 1.0, 1.0, [1.0; 4]);
        let from = d.prims().len();
        assert_eq!(d.extent(from), None);
        d.rect(10.0, 10.0, 5.0, 5.0, [1.0; 4]);
        d.clip(None);
        d.text(0.0, 30.0, 10.0, "abcd", [1.0; 4]);
        assert_eq!(d.extent(from), Some(Rect { x: 0.0, y: 10.0, w: 20.0, h: 32.5 }));
    }

    #[test]
    fn a_halo_rings_ink_that_its_ground_sets_apart_and_ends() {
        let (ground, light, dark) = ([0.1, 0.1, 0.1, 0.3], [0.95, 0.95, 0.95, 1.0], [0.09, 0.1, 0.09, 1.0]);
        let mut d = DrawList::new(0.5);
        d.halo(Some(ground));
        d.text(0.0, 0.0, 10.0, "a", light);
        d.text(0.0, 0.0, 10.0, "b", dark);
        d.icon(0.0, 0.0, 10.0, "grip", light);
        d.rect(0.0, 0.0, 20.0, 20.0, [0.2, 0.2, 0.2, 1.0]);
        d.text(2.0, 2.0, 10.0, "in a field", light);
        d.halo(None);
        d.text(0.0, 0.0, 10.0, "c", light);
        let halos: Vec<[f32; 4]> = d.prims().iter().filter_map(|p| match *p {
            Prim::Text { halo, .. } | Prim::Icon { halo, .. } => Some(halo),
            _ => None,
        }).collect();
        let g = [0.1, 0.1, 0.1, 1.0];
        assert_eq!(halos, [g, NO_HALO, g, NO_HALO, NO_HALO], "opaque round light ink; none round ink the ground does not set apart, on an opaque field, or after the end");
    }

    #[test]
    fn a_disc_is_a_square_rounded_all_the_way() {
        let mut d = DrawList::new(0.5);
        d.disc(10.0, 10.0, 4.0, [1.0; 4]);
        assert_eq!(d.prims()[0], Prim::Rect { x: 6.0, y: 6.0, w: 8.0, h: 8.0, r: 4.0, color: [1.0; 4] });
    }

    #[test]
    fn a_qr_is_a_white_square_with_its_quiet_zone() {
        let mut d = DrawList::new(1.0);
        let side = qr(&mut d, (10.0, 10.0), 2.0, 3, &|x, y| x == y);
        assert_eq!(side, 22.0);
        assert_eq!(d.prims().len(), 4, "the square and three dark modules");
        assert_eq!(d.prims()[1], Prim::Rect { x: 18.0, y: 18.0, w: 2.0, h: 2.0, r: 0.0, color: [0.0, 0.0, 0.0, 1.0] });
    }
}
