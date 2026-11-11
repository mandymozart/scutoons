//! The deck's pixel face (#386; owner 2026-11-11: "The Hud does alright as menus for the moment"): the HUD's font8x8
//! (deck-space hud.rs), so the menus drawn with the kit wear the HUD's arcade look on both decks. #485: egui takes only
//! outline fonts (TrueType / OpenType through ab_glyph), so the face is a bitmap atlas instead – one 8 × 8 cell per
//! character in a texture egui samples nearest (`screen.rs`), a glyph one quad. Basic Latin and Latin-1 (the `·`);
//! anything else is a `?`. The atlas's first cell is solid: the rectangles of a pixel layer are quads on it, so a layer is
//! one mesh in painter's order.

/// a character's 8 rows of 8 cells (bit 0 = the left cell)
pub fn glyph(c: char) -> [u8; 8] {
    use font8x8::UnicodeFonts;
    font8x8::BASIC_FONTS.get(c).or_else(|| font8x8::LATIN_FONTS.get(c)).or_else(|| font8x8::BASIC_FONTS.get('?')).unwrap_or([0; 8])
}

/// the atlas: 16 × 16 cells of 8 × 8 pixels, a cell per code point below 256
pub const SIDE: usize = 128;
const COLS: usize = 16;

/// the cell a character is drawn from: its code point when the face has it, else the `?`'s (0 is the solid cell)
pub fn cell(c: char) -> usize {
    match c as u32 {
        n @ (32..=126 | 160..=255) => n as usize,
        _ => '?' as usize,
    }
}

/// a cell's top left in the atlas, in pixels
pub fn origin(cell: usize) -> (usize, usize) {
    ((cell % COLS) * 8, (cell / COLS) * 8)
}

/// The atlas as straight RGBA, `SIDE × SIDE`: white where a glyph has a cell, clear elsewhere; cell 0 all white.
pub fn atlas() -> Vec<u8> {
    let mut rgba = vec![0u8; SIDE * SIDE * 4];
    let mut set = |x: usize, y: usize| rgba[(y * SIDE + x) * 4..][..4].copy_from_slice(&[255; 4]);
    for (gy, gx) in (0..8).flat_map(|y| (0..8).map(move |x| (y, x))) {
        set(gx, gy);
    }
    for code in (32..=126).chain(160..=255) {
        let (ox, oy) = origin(code as usize);
        let bits = glyph(char::from_u32(code).unwrap_or('?'));
        for (gy, row) in bits.iter().enumerate() {
            for gx in (0..8).filter(|gx| row & (1 << gx) != 0) {
                set(ox + gx, oy + gy);
            }
        }
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_middle_dot_is_latin_one_and_an_unknown_one_is_a_question_mark() {
        assert_ne!(glyph('·'), glyph('?'));
        assert_eq!(glyph('漢'), glyph('?'));
        assert_eq!((cell('A'), cell('·'), cell('漢'), cell('\n')), (65, 183, 63, 63));
    }

    #[test]
    fn the_atlas_holds_each_glyph_in_its_cell_and_a_solid_first_cell() {
        let px = atlas();
        let at = |x: usize, y: usize| px[(y * SIDE + x) * 4 + 3];
        assert!((0..8).all(|y| (0..8).all(|x| at(x, y) == 255)), "the solid cell");
        let (ox, oy) = origin(cell('A'));
        let set: u32 = glyph('A').iter().map(|b| b.count_ones()).sum();
        let painted = (0..8).flat_map(|y| (0..8).map(move |x| (x, y))).filter(|&(x, y)| at(ox + x, oy + y) == 255).count();
        assert_eq!(painted as u32, set);
        assert_eq!(at(ox + 7, oy), 0, "A's top right is clear");
    }
}
