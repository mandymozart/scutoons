//! The deck's pixel face (#386; owner 2026-11-11: "The Hud does alright as menus for the moment"): the HUD's font8x8
//! (deck-space hud.rs) as a raster `Face`, so the menus drawn with the kit wear the HUD's arcade look on both decks. A
//! glyph is 8 × 8 cells; at font size `px` a cell is `px / 8` device pixels – whole pixels at the sizes the menus use
//! (8 and 4 logical pixels at an integer scale). Basic Latin and Latin-1 (the `·`); anything else is a `?`.
use crate::raster::{Face, Pixels, LINE};
use font8x8::UnicodeFonts;

/// font8x8 as a face: one advance per glyph, as wide as its size
#[derive(Debug, Clone, Copy, Default)]
pub struct Pixel8;

/// a character's 8 rows of 8 cells (bit 0 = the left cell)
pub fn glyph(c: char) -> [u8; 8] {
    font8x8::BASIC_FONTS.get(c).or_else(|| font8x8::LATIN_FONTS.get(c)).or_else(|| font8x8::BASIC_FONTS.get('?')).unwrap_or([0; 8])
}

impl Face for Pixel8 {
    fn per_px(&self) -> f32 {
        1.0
    }

    fn paint(&self, out: &mut Pixels, (x, y): (f32, f32), px: f32, s: &str, color: [f32; 4]) {
        let cell = px / 8.0;
        // the glyph's 8 cells in the middle of the line, on a whole pixel
        let top = (y + (px * LINE - px) / 2.0).round();
        for (i, c) in s.chars().enumerate() {
            let left = x + i as f32 * px;
            for (gy, bits) in glyph(c).iter().enumerate() {
                for gx in 0..8 {
                    if bits & (1 << gx) != 0 {
                        out.fill(left + gx as f32 * cell, top + gy as f32 * cell, cell, cell, color);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawList;
    use crate::raster::raster_with;

    #[test]
    fn a_glyph_is_whole_pixels_at_twice_its_size() {
        let mut d = DrawList::new(Pixel8.per_px());
        assert_eq!(d.text(0.0, 0.0, 8.0, "AB", [1.0; 4]), 16.0, "8 a glyph");
        let img = raster_with(&d, &Pixel8, (0.0, 0.0), (16.0, 11.0), 2.0);
        let white = (0..img.w * img.h).filter(|i| img.at(i % img.w, i / img.w) == [255, 255, 255, 255]).count();
        let set: u32 = glyph('A').iter().chain(glyph('B').iter()).map(|b| b.count_ones()).sum();
        assert_eq!(white as u32, set * 4, "each cell 2 × 2 pixels, nothing half covered");
        assert_eq!(img.at(0, 0)[3], 0, "the line's top is free: the glyph sits in its middle");
    }

    #[test]
    fn the_middle_dot_is_latin_one_and_an_unknown_one_is_a_question_mark() {
        assert_ne!(glyph('·'), glyph('?'));
        assert_eq!(glyph('漢'), glyph('?'));
    }
}
