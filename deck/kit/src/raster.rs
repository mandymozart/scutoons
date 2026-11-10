//! A draw list as pixels (#481): the deck's own UI for the person at a deck is painted on the CPU, so the native deck and
//! the browser deck show the same panel as one more overlay of their stage. The face comes from the caller (the deck's
//! IBM Plex Mono Bold, deck-outlet's assets). No GPU here; the editor has its own adapter (deck-editor ui.rs). Icons and
//! the logo are the editor's and are not painted.
use crate::draw::{DrawList, Prim};
use ab_glyph::{Font, PxScale, ScaleFont};

/// a line's height per pixel of font size: a text sits in the middle of it (panel.rs)
const LINE: f32 = 1.35;

/// Straight RGBA, `w × h`.
#[derive(Debug, Clone, PartialEq)]
pub struct Pixels {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl Pixels {
    pub fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.w + x) * 4) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }

    /// `rgb` over the pixel with coverage `a`
    fn blend(&mut self, x: i32, y: i32, rgb: [u8; 3], a: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 || a <= 0.0 {
            return;
        }
        let a = a.min(1.0);
        let p = &mut self.rgba[((y as u32 * self.w + x as u32) * 4) as usize..][..4];
        let under = p[3] as f32 / 255.0;
        let out = a + under * (1.0 - a);
        for k in 0..3 {
            p[k] = ((rgb[k] as f32 * a + p[k] as f32 * under * (1.0 - a)) / out.max(1e-6)).round() as u8;
        }
        p[3] = (out * 255.0).round() as u8;
    }

    /// a rectangle on whole pixels
    fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let (x0, y0, x1, y1) = (x.round() as i32, y.round() as i32, (x + w).round() as i32, (y + h).round() as i32);
        for py in y0.max(0)..y1.min(self.h as i32) {
            for px in x0.max(0)..x1.min(self.w as i32) {
                self.blend(px, py, rgb(color), color[3]);
            }
        }
    }
}

fn rgb(color: [f32; 4]) -> [u8; 3] {
    [(color[0] * 255.0).round() as u8, (color[1] * 255.0).round() as u8, (color[2] * 255.0).round() as u8]
}

/// a font size (the em, as in the reader's CSS: a 14 px text is 14 px per em) as the face's scale (its whole height)
fn scaled(font: &impl Font, px: f32) -> PxScale {
    PxScale::from(px * font.height_unscaled() / font.units_per_em().unwrap_or(1000.0))
}

/// A mono face's advance per pixel of font size: what a `DrawList` for this painter is made with.
pub fn per_px(font: &impl Font) -> f32 {
    font.h_advance_unscaled(font.glyph_id('M')) / font.units_per_em().unwrap_or(1000.0)
}

/// The part of `list` from `origin`, `size` logical pixels, at `scale` device pixels per logical one, in `font`.
pub fn raster(list: &DrawList, font: &impl Font, origin: (f32, f32), size: (f32, f32), scale: f32) -> Pixels {
    let (w, h) = ((size.0 * scale).ceil().max(1.0) as u32, (size.1 * scale).ceil().max(1.0) as u32);
    let mut out = Pixels { w, h, rgba: vec![0; (w * h * 4) as usize] };
    let at = |x: f32, y: f32| ((x - origin.0) * scale, (y - origin.1) * scale);
    for p in list.prims() {
        match *p {
            Prim::Rect { x, y, w, h, color, .. } => {
                let (x, y) = at(x, y);
                out.fill(x, y, w * scale, h * scale, color);
            }
            Prim::Outline { x, y, w, h, thick, color, .. } => {
                let ((x, y), (w, h), t) = (at(x, y), (w * scale, h * scale), (thick * scale).max(1.0));
                out.fill(x, y, w, t, color);
                out.fill(x, y + h - t, w, t, color);
                out.fill(x, y + t, t, h - 2.0 * t, color);
                out.fill(x + w - t, y + t, t, h - 2.0 * t, color);
            }
            Prim::Text { x, y, px, start, end, color } => {
                let (x, y) = at(x, y);
                text(&mut out, font, (x, y), px * scale, list.text_of(start, end), color);
            }
            Prim::Icon { .. } | Prim::Logo { .. } => {}
        }
    }
    out
}

/// `s` from its top left: its glyph box in the middle of a line `px × LINE` tall
fn text(out: &mut Pixels, font: &impl Font, (x, y): (f32, f32), px: f32, s: &str, color: [f32; 4]) {
    let scale = scaled(font, px);
    let sf = font.as_scaled(scale);
    let base = y + (px * LINE - (sf.ascent() - sf.descent())) / 2.0 + sf.ascent();
    let mut cx = x;
    for c in s.chars() {
        let id = sf.glyph_id(c);
        if let Some(o) = font.outline_glyph(id.with_scale_and_position(scale, ab_glyph::point(cx, base))) {
            let b = o.px_bounds();
            o.draw(|gx, gy, cover| out.blend(b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32, rgb(color), cover * color[3]));
        }
        cx += sf.h_advance(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel::{self, Part};
    use ab_glyph::FontRef;

    /// the deck's mono face (deck-outlet's assets)
    fn mono() -> FontRef<'static> {
        FontRef::try_from_slice(include_bytes!("../../outlet/assets/IBMPlexMono-Bold-latin.ttf")).expect("IBM Plex Mono")
    }

    #[test]
    fn a_panel_is_painted_black_with_its_edge_its_buttons_and_its_text() {
        let font = mono();
        assert!((per_px(&font) - 0.6).abs() < 0.01, "a mono face, 0.6 em wide: {}", per_px(&font));
        let (mut d, t) = (DrawList::new(per_px(&font)), panel::tokens());
        let parts = [Part::Caption("Flash · Vorschlag", "von ana"), Part::Title("top · Version 3"), Part::Buttons(&["Annehmen", "Ablehnen"], None, None)];
        let screen = (320.0, 240.0);
        let h = panel::height(&d, &t, screen.0, &parts);
        let places = panel::draw(&mut d, &t, screen, &parts, false);
        // the panel alone, at two device pixels per logical one
        let img = raster(&d, &font, (0.0, screen.1 - h), (screen.0, h), 2.0);
        assert_eq!((img.w, img.h), (640, (h * 2.0).ceil() as u32));
        let px = |x: f32, y: f32| img.at((x * 2.0) as u32, ((y - (screen.1 - h)) * 2.0) as u32);
        assert_eq!(px(100.0, screen.1 - h + 0.5), [0, 209, 255, 255], "the accent on the top edge");
        assert_eq!(px(4.0, screen.1 - 4.0), [0, 0, 0, 255], "the ground");
        // the primary button is filled with the ink, the second one only framed
        let (a, b) = (places[0], places[1]);
        assert_eq!(px(a.x + 3.0, a.y + 3.0), [255, 255, 255, 255]);
        assert_eq!(px(b.x + 6.0, b.y + 6.0), [0, 0, 0, 255]);
        assert_eq!(px(b.x + 0.5, b.y + 10.0), [85, 85, 85, 255], "its frame");
        // the title is white type on the black
        let white = (0..img.w * img.h).filter(|i| img.at(i % img.w, i / img.w) == [255, 255, 255, 255]).count();
        assert!(white > 2000, "{white} white pixels");
    }

    #[test]
    fn what_lies_outside_is_cut_and_nothing_is_painted_where_nothing_is() {
        let font = mono();
        let mut d = DrawList::new(per_px(&font));
        d.rect(-10.0, -10.0, 15.0, 15.0, [1.0, 0.0, 0.0, 0.5]);
        let img = raster(&d, &font, (0.0, 0.0), (10.0, 10.0), 1.0);
        assert_eq!(img.at(2, 2), [255, 0, 0, 128], "half covered, over nothing");
        assert_eq!(img.at(8, 8), [0, 0, 0, 0]);
    }
}
