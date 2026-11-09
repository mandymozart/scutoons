//! What a frame of UI draws (#456): a list of primitives in logical pixels, in painter's order. The kit has no GPU: an
//! adapter turns the list into draws. Shapes are exact at any size (a distance per pixel in the adapter), so a dot does not
//! depend on a texture. The text of a frame lives in one buffer, so a frame allocates nothing once it has run.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Prim {
    /// a filled rectangle, its corners rounded by `r`
    Rect { x: f32, y: f32, w: f32, h: f32, r: f32, color: [f32; 4] },
    /// the outline of one, `thick` wide
    Outline { x: f32, y: f32, w: f32, h: f32, r: f32, thick: f32, color: [f32; 4] },
    /// `start..end` = a byte range of the list's text buffer
    Text { x: f32, y: f32, px: f32, start: usize, end: usize, color: [f32; 4] },
    /// an icon by name, in a square of `size`
    Icon { x: f32, y: f32, size: f32, name: &'static str, color: [f32; 4] },
    /// a frame of the animated logo, in a square of `size`
    Logo { x: f32, y: f32, size: f32, frame: usize },
}

pub struct DrawList {
    prims: Vec<Prim>,
    text: String,
    /// a character's advance per pixel of font size (the mono face's)
    per_px: f32,
}

impl DrawList {
    pub fn new(per_px: f32) -> DrawList {
        DrawList { prims: Vec::with_capacity(4096), text: String::with_capacity(8192), per_px }
    }

    pub fn clear(&mut self) {
        self.prims.clear();
        self.text.clear();
    }

    pub fn prims(&self) -> &[Prim] {
        &self.prims
    }

    /// the text of a `Prim::Text`
    pub fn text_of(&self, start: usize, end: usize) -> &str {
        &self.text[start..end]
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
        let start = self.text.len();
        self.text.push_str(text);
        self.prims.push(Prim::Text { x, y, px, start, end: self.text.len(), color });
        self.advance(px) * text.chars().count() as f32
    }

    pub fn icon(&mut self, x: f32, y: f32, size: f32, name: &'static str, color: [f32; 4]) {
        self.prims.push(Prim::Icon { x, y, size, name, color });
    }

    pub fn logo(&mut self, x: f32, y: f32, size: f32, frame: usize) {
        self.prims.push(Prim::Logo { x, y, size, frame });
    }

    /// a filled circle around (x, y)
    pub fn disc(&mut self, x: f32, y: f32, r: f32, color: [f32; 4]) {
        self.round_rect(x - r, y - r, 2.0 * r, 2.0 * r, r, color);
    }

    /// a circle's outline around (x, y)
    pub fn ring(&mut self, x: f32, y: f32, r: f32, color: [f32; 4]) {
        self.outline(x - r, y - r, 2.0 * r, 2.0 * r, (r, 2.0), color);
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
    fn a_disc_is_a_square_rounded_all_the_way() {
        let mut d = DrawList::new(0.5);
        d.disc(10.0, 10.0, 4.0, [1.0; 4]);
        assert_eq!(d.prims()[0], Prim::Rect { x: 6.0, y: 6.0, w: 8.0, h: 8.0, r: 4.0, color: [1.0; 4] });
    }
}
