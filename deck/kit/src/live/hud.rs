//! The deck's arcade look (#386; owner 2026-11-11: "I like the game style btw. The Hud does alright as menus for the
//! moment we can iterate on that"): the HUD's pages as the kit's controls – black ground, upper case in the deck's pixel
//! face (`pixel.rs`), the red bar on the chosen row, a dimmed row for what comes later, a QR. Sizes in logical pixels
//! where one is a cell of the pixel face at its size (`GLYPH`): a row is 10 high as the HUD's lines are, 20 for a finger
//! (a value under its label, `SMALL`). A control draws itself and answers where it landed; what a press means is the
//! caller's (deck-ui `menu.rs`). No layout file of our own: a page is these calls, top down.
use crate::controls::Rect;
use crate::draw::DrawList;
use crate::tokens::{Colors, Sizes, Tokens};

const fn rgb(v: u32) -> [f32; 4] {
    [((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0, 1.0]
}

/// the pixel face's size, and the small one (a value under its label on a finger's row)
pub const GLYPH: f32 = 8.0;
pub const SMALL: f32 = 4.0;
/// a row's height, and a finger's
pub const ROW: f32 = 10.0;
pub const FINGER: f32 = 20.0;

/// The HUD's look as the kit's tokens: the black ground (a little see-through over the scene), the HUD's white, a muted
/// grey for what is not there yet, the red selection bar (`selected`), the pairing code's mint (`accent`), the dimmed
/// scene behind a page (`stage`). A `tokens.toml` overrides them like any of the kit's.
pub fn tokens() -> Tokens {
    let black = rgb(0x000000);
    Tokens {
        color: Colors {
            stage: [0.0, 0.0, 0.0, 0.55],
            surface: [0.0, 0.0, 0.0, 0.92],
            surface_head: black,
            line: rgb(0x3a3a3a),
            text: rgb(0xebebeb),
            muted: rgb(0x7a7a7a),
            control: black,
            hover: rgb(0x262626),
            pressed: rgb(0x262626),
            selected: rgb(0xe8302a),
            within: rgb(0x262626),
            accent: rgb(0x5ef2c8),
            ..Colors::default()
        },
        size: Sizes { edge: 1.0, pad: 4.0, font: GLYPH, small: SMALL, row: ROW, tree_row: FINGER, radius: 0.0, ..Sizes::default() },
    }
}

/// how many characters of the pixel face fit `w`
pub fn cols(d: &DrawList, w: f32) -> usize {
    (w / d.advance(GLYPH)).max(0.0) as usize
}

/// `text` cut to `cols` characters, upper case
pub fn fit(text: &str, cols: usize) -> String {
    text.to_uppercase().chars().take(cols).collect()
}

/// A page's ground: the dimmed scene over the whole `screen` (none: the scene stays as it is), then the page's box.
pub fn ground(d: &mut DrawList, t: &Tokens, screen: Option<(f32, f32)>, at: Rect) {
    if let Some((w, h)) = screen {
        d.rect(0.0, 0.0, w, h, t.color.stage);
    }
    d.rect(at.x, at.y, at.w, at.h, t.color.surface);
    d.outline(at.x, at.y, at.w, at.h, (0.0, t.size.edge), t.color.line);
}

/// One line of text across `w` from (x, y): `left` at the start, `right` at the end, both cut to fit; muted or not. Its height.
pub fn line(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), left: &str, right: &str, muted: bool) -> f32 {
    let color = if muted { t.color.muted } else { t.color.text };
    let n = cols(d, w);
    let right = fit(right, n);
    let left = fit(left, n.saturating_sub(right.chars().count() + usize::from(!right.is_empty())));
    d.text(x, y, GLYPH, &left, color);
    let rw = d.advance(GLYPH) * right.chars().count() as f32;
    d.text(x + w - rw, y, GLYPH, &right, color);
    ROW
}

/// A line of `px`-sized text centred across `w`. Its height.
pub fn centred(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), px: f32, text: &str, muted: bool) -> f32 {
    let text = text.to_uppercase();
    let tw = d.advance(px) * text.chars().count() as f32;
    d.text(x + ((w - tw) * 0.5).max(0.0), y, px, &text, if muted { t.color.muted } else { t.color.text });
    px * 1.25
}

/// A code to type (a pairing code): twice the glyph, in the accent, from (x, y). Its height.
pub fn code(d: &mut DrawList, t: &Tokens, (x, y): (f32, f32), text: &str) -> f32 {
    d.text(x, y, 2.0 * GLYPH, &text.to_uppercase(), t.color.accent);
    2.0 * ROW + 2.0
}

/// A rule across `w`. Its height (the line and room around it).
pub fn rule(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32)) -> f32 {
    d.rect(x, y + 2.0, w, t.size.edge, t.color.line);
    5.0
}

/// One row of a menu: what it is, what it says on the right.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Item<'a> {
    pub label: &'a str,
    pub value: &'a str,
    /// the keys are on it: the red bar, the `>` before it
    pub chosen: bool,
    /// nothing behind it yet: muted
    pub dim: bool,
}

/// A menu row across `w` from (x, y): the HUD's (10 high, the value at the right) or a finger's (20 high, the value
/// under the label, a `>` at the end). Its place.
pub fn item(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), it: Item, finger: bool) -> Rect {
    let h = if finger { FINGER } else { ROW };
    let at = Rect { x, y, w, h };
    if it.chosen {
        d.rect(x, y, w, h, t.color.selected);
    }
    let color = if it.dim && !it.chosen { t.color.muted } else { t.color.text };
    let adv = d.advance(GLYPH);
    let n = cols(d, w);
    if it.chosen {
        d.text(x + 2.0, y + (h - ROW) * 0.5 + 1.0, GLYPH, ">", t.color.text);
    }
    let lx = x + 2.0 + 2.0 * adv;
    if finger {
        let label = fit(it.label, n.saturating_sub(4));
        let y0 = if it.value.is_empty() { y + (h - ROW) * 0.5 + 1.0 } else { y + 2.0 };
        d.text(lx, y0, GLYPH, &label, color);
        if !it.value.is_empty() {
            d.text(lx, y0 + ROW, SMALL, &fit(it.value, ((w - 2.0 * adv) / d.advance(SMALL)) as usize), if it.chosen { t.color.text } else { t.color.muted });
        }
        d.text(x + w - adv - 2.0, y + (h - ROW) * 0.5 + 1.0, GLYPH, ">", color);
    } else {
        let value = fit(it.value, n.saturating_sub(3));
        let label = fit(it.label, n.saturating_sub(3 + value.chars().count() + usize::from(!value.is_empty())));
        d.text(lx, y + 1.0, GLYPH, &label, color);
        d.text(x + w - 2.0 - adv * value.chars().count() as f32, y + 1.0, GLYPH, &value, color);
    }
    at
}

/// The page drawn into `list` as pixels: `size` logical pixels from the top left at `scale` device pixels each, in the
/// deck's pixel face. The one way a page of this look becomes pixels: its owner never names the face or the painter.
#[cfg(feature = "pixel")]
pub fn paint(list: &DrawList, size: (f32, f32), scale: f32) -> crate::raster::Pixels {
    crate::raster::raster_with(list, &crate::pixel::Pixel8, (0.0, 0.0), size, scale)
}

/// a draw list for this look: the pixel face's advance (one per pixel of size)
pub fn list() -> DrawList {
    DrawList::new(1.0)
}

/// A QR code: `n × n` modules (`dark(x, y)`), `module` logical pixels each, black on a white square with the quiet zone
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
    use crate::draw::Prim;

    fn texts(d: &DrawList) -> Vec<String> {
        d.prims().iter().filter_map(|p| if let Prim::Text { start, end, .. } = *p { Some(d.text_of(start, end).to_string()) } else { None }).collect()
    }

    #[test]
    fn a_chosen_row_wears_the_red_bar_and_its_marker() {
        let (mut d, t) = (DrawList::new(1.0), tokens());
        let at = item(&mut d, &t, (0.0, 0.0, 320.0), Item { label: "Continue", value: "", chosen: true, dim: false }, false);
        assert_eq!((at.w, at.h), (320.0, ROW));
        assert_eq!(d.prims()[0], Prim::Rect { x: 0.0, y: 0.0, w: 320.0, h: ROW, r: 0.0, color: t.color.selected });
        assert_eq!(texts(&d), [">", "CONTINUE", ""]);
    }

    #[test]
    fn a_finger_row_is_twice_as_high_with_its_value_under_its_label() {
        let (mut d, t) = (DrawList::new(1.0), tokens());
        let at = item(&mut d, &t, (0.0, 0.0, 176.0), Item { label: "Live", value: "tenor: anna", chosen: false, dim: false }, true);
        assert_eq!(at.h, FINGER);
        let small: Vec<(f32, f32)> = d.prims().iter().filter_map(|p| if let Prim::Text { px, y, .. } = *p { Some((px, y)) } else { None }).collect();
        assert_eq!(small, [(GLYPH, 2.0), (SMALL, 12.0), (GLYPH, 6.0)], "the label, its value under it, the arrow");
        assert_eq!(texts(&d), ["LIVE", "TENOR: ANNA", ">"]);
    }

    #[test]
    fn a_line_keeps_its_right_part_and_cuts_the_left() {
        let (mut d, t) = (DrawList::new(1.0), tokens());
        line(&mut d, &t, (0.0, 0.0, 80.0), "scutoons deck", "esc close", false);
        assert_eq!(texts(&d), ["", "ESC CLOSE"], "nine of ten columns are the right part's: nothing is left for the left one");
        d.clear();
        line(&mut d, &t, (0.0, 0.0, 160.0), "scutoons deck", "esc close", false);
        assert_eq!(texts(&d), ["SCUTOONS D", "ESC CLOSE"], "20 columns: nine, a space, ten");
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
