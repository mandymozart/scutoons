//! The buttons' 3D labels (#188, owner 2026-10-31: "the buttons give very poor visual feedback"): `glow 0.60` next to
//! the hovered / turned button, `index 3.20 < sway` on a linked one, the range under a gripped one – always facing the
//! camera, a fixed size on the screen. The text is set without allocating (a fixed byte buffer, `fmt::Write`); the GPU
//! draws it as one quad per character from a glyph atlas made once at the start (IBM Plex Mono Bold, text.rs) – no
//! texture work per frame (gpu.rs).
use std::fmt::{self, Write};

/// labels per frame at most (all beings), and characters per label
pub const LABELS: usize = 8;
pub const CHARS: usize = 24;
/// the atlas: printable ASCII from ' ' (95 glyphs), then one solid cell (the labels' backing); 16 cells a row
pub const FIRST: u8 = b' ';
pub const GLYPHS: usize = 95;
pub const SOLID: usize = GLYPHS;
pub const COLS: usize = 16;

/// A label's text: at most CHARS bytes (more is cut), set with `write!` – never allocates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Text {
    pub bytes: [u8; CHARS],
    pub len: usize,
}

impl Default for Text {
    fn default() -> Self {
        Text { bytes: [b' '; CHARS], len: 0 }
    }
}

impl Text {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("?")
    }
}

impl Write for Text {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if self.len >= CHARS {
                break;
            }
            // the atlas is ASCII: anything else shows as '?'
            self.bytes[self.len] = if c.is_ascii() && !c.is_ascii_control() { c as u8 } else { b'?' };
            self.len += 1;
        }
        Ok(())
    }
}

/// One label: where (the button's centre – the GPU sets the text beside it, `offset` metres to the camera's right),
/// the text, its colour (alpha = how much it shows), its size (1 = a value's label) and its line (0, 1 = below).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Label {
    pub pos: [f32; 3],
    pub offset: f32,
    pub text: Text,
    pub color: [f32; 4],
    pub scale: f32,
    pub line: f32,
}

/// a value in ≤ 6 characters (as the HUD writes it): 1234 · 12.3 · 0.60
pub fn write_num(w: &mut impl Write, v: f32) -> fmt::Result {
    match v.abs() {
        a if a >= 1000.0 => write!(w, "{v:.0}"),
        a if a >= 10.0 => write!(w, "{v:.1}"),
        _ => write!(w, "{v:.2}"),
    }
}

/// The glyph atlas (RGBA: white, the coverage in alpha): `cell` = one character's box in pixels, the glyph `i`'s cell
/// at column i % COLS, row i / COLS; the last cell solid.
pub struct Atlas {
    pub w: u32,
    pub h: u32,
    pub cell: (u32, u32),
    pub pixels: Vec<u8>,
}

impl Atlas {
    /// `px` = the font's size in pixels (the glyphs are drawn smaller or larger on the screen)
    pub fn new(px: f32) -> Atlas {
        use ab_glyph::{point, Font, PxScale, ScaleFont};
        let font = crate::text::mono();
        let sf = font.as_scaled(PxScale::from(px));
        let cw = sf.h_advance(font.glyph_id('0')).ceil() as u32 + 2;
        let ch = (sf.ascent() - sf.descent()).ceil() as u32 + 2;
        let rows = (GLYPHS + 1).div_ceil(COLS) as u32;
        let (w, h) = (cw * COLS as u32, ch * rows);
        let mut pixels = vec![255u8; (w * h * 4) as usize];
        pixels.chunks_exact_mut(4).for_each(|p| p[3] = 0);
        for i in 0..GLYPHS {
            let c = (FIRST + i as u8) as char;
            let (x0, y0) = ((i % COLS) as u32 * cw, (i / COLS) as u32 * ch);
            let g = font.glyph_id(c).with_scale_and_position(PxScale::from(px), point(x0 as f32 + 1.0, y0 as f32 + 1.0 + sf.ascent()));
            if let Some(o) = font.outline_glyph(g) {
                let b = o.px_bounds();
                o.draw(|x, y, cov| {
                    let (px, py) = (b.min.x as i32 + x as i32, b.min.y as i32 + y as i32);
                    // inside its own cell only (a glyph that reaches out is cut, never bleeds into the next)
                    if px >= x0 as i32 && py >= y0 as i32 && px < (x0 + cw) as i32 && py < (y0 + ch) as i32 {
                        let a = &mut pixels[((py as u32 * w + px as u32) * 4 + 3) as usize];
                        *a = (*a).max((cov.clamp(0.0, 1.0) * 255.0) as u8);
                    }
                });
            }
        }
        let (x0, y0) = ((SOLID % COLS) as u32 * cw, (SOLID / COLS) as u32 * ch);
        for y in y0..y0 + ch {
            for x in x0..x0 + cw {
                pixels[((y * w + x) * 4 + 3) as usize] = 255;
            }
        }
        Atlas { w, h, cell: (cw, ch), pixels }
    }

    /// cell `i`'s uv rectangle (u0, v0, u1, v1), a little inset so the filter never reaches the next cell
    pub fn uv(&self, i: usize) -> [f32; 4] {
        let (cw, ch) = (self.cell.0 as f32, self.cell.1 as f32);
        let (x0, y0) = ((i % COLS) as f32 * cw, (i / COLS) as f32 * ch);
        let inset = if i == SOLID { cw * 0.4 } else { 0.5 };
        [(x0 + inset) / self.w as f32, (y0 + inset) / self.h as f32, (x0 + cw - inset) / self.w as f32, (y0 + ch - inset) / self.h as f32]
    }
}

/// a byte's atlas cell ('?' for what the atlas lacks)
pub fn glyph(b: u8) -> usize {
    if (FIRST..FIRST + GLYPHS as u8).contains(&b) {
        (b - FIRST) as usize
    } else {
        (b'?' - FIRST) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_cut_not_grown_and_numbers_are_short() {
        let mut t = Text::default();
        write!(t, "glow ").unwrap();
        write_num(&mut t, 0.6).unwrap();
        assert_eq!(t.as_str(), "glow 0.60");
        let mut t = Text::default();
        write!(t, "{}", "x".repeat(40)).unwrap();
        assert_eq!(t.len, CHARS);
        let mut t = Text::default();
        write!(t, "é 1").unwrap();
        assert_eq!(t.as_str(), "? 1");
        for (v, s) in [(1234.4, "1234"), (12.34, "12.3"), (-0.5, "-0.50")] {
            let mut t = Text::default();
            write_num(&mut t, v).unwrap();
            assert_eq!(t.as_str(), s);
        }
    }

    #[test]
    fn the_atlas_has_every_glyph_and_a_solid_cell() {
        let a = Atlas::new(28.0);
        let alpha = |i: usize| -> u32 {
            let (cw, ch) = a.cell;
            let (x0, y0) = ((i % COLS) as u32 * cw, (i / COLS) as u32 * ch);
            (y0..y0 + ch).flat_map(|y| (x0..x0 + cw).map(move |x| (x, y))).map(|(x, y)| a.pixels[((y * a.w + x) * 4 + 3) as usize] as u32).sum()
        };
        assert_eq!(alpha(glyph(b' ')), 0, "a space is empty");
        for c in *b"g0.<W" {
            assert!(alpha(glyph(c)) > 0, "{}", c as char);
        }
        assert_eq!(alpha(SOLID), a.cell.0 * a.cell.1 * 255);
        assert_eq!(glyph(200), glyph(b'?'));
        let uv = a.uv(SOLID);
        assert!(uv[0] < uv[2] && uv[1] < uv[3] && uv[3] <= 1.0);
    }
}
