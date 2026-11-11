//! The deck's mono face as the screen's own glyph atlas (#485 slice 3; owner 2026-11-11 on slice 2's +893 KB: "Slim
//! down"). egui 0.37 sets text with harfrust, skrifa and vello_cpu's rasteriser – 3.2 MB of the browser deck's code, and
//! linked as soon as an egui `Context` exists (its debug text). The decks set one face, a monospace one, on one line at a
//! time: the screen lays it out the way egui does (egui's line metrics, the face's advance, a quarter-pixel bins of x) and
//! rasters each glyph with ab_glyph (the deck's text crate already: deck-outlet, deck-space's labels) into this atlas,
//! coverage turned into alpha as egui's dark mode does (2c − c²). epaint's meshes, textures and tessellator draw it as
//! before. The editor keeps egui's text (`screen.rs`, `Screen::with_egui_text`).
//!
//! The atlas is a texture of `WIDTH` columns that grows in rows as glyphs come; texel (0, 0) is white (epaint's
//! `WHITE_UV`: the tessellator's flat shapes sample it). Full at `MAX_HEIGHT`: emptied, and the frame painted again.
use ab_glyph::{Font, FontRef, PxScale};
use egui::epaint::{Color32, ColorImage, ImageDelta, Mesh};
use egui::{pos2, vec2, Rect, TextureOptions};
use std::collections::HashMap;

/// the atlas's width (WebGL2 guarantees textures of 2048)
pub const WIDTH: usize = 1024;
/// its largest height
const MAX_HEIGHT: usize = 1024;
/// rows added at a time
const STEP: usize = 128;
/// x positions of a glyph a pixel is split into (egui's subpixel bins)
const BINS: f32 = 4.0;

/// One glyph in the atlas: its texels, and where they lie from the pen (left, top) in pixels.
#[derive(Clone, Copy)]
struct Placed {
    at: [usize; 2],
    size: [usize; 2],
    offset: [f32; 2],
}

/// A line's measures at a font size, as egui sets them (`StyledMetrics`): to 1/32 of a pixel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub ascent: f32,
    pub row_height: f32,
    pub advance: f32,
}

pub struct Atlas {
    font: FontRef<'static>,
    pixels: Vec<Color32>,
    height: usize,
    /// (glyph, size in 1/64 px, bin) → its place (None: nothing to draw, a space)
    glyphs: HashMap<(char, u32, u8), Option<Placed>>,
    /// the shelf being filled: its left, top and height
    shelf: [usize; 3],
    /// rows written since the last delta (none: nothing), or the whole texture again
    rows: Option<(usize, usize)>,
    whole: bool,
    /// a glyph found no room in this frame
    full: bool,
}

fn ui(v: f32) -> f32 {
    (v * 32.0).round() / 32.0
}

impl Atlas {
    /// `mono`: the deck's mono face (a TrueType file); None if it is no font
    pub fn new(mono: &'static [u8]) -> Option<Atlas> {
        let font = FontRef::try_from_slice(mono).ok()?;
        let mut atlas = Atlas { font, pixels: Vec::new(), height: 0, glyphs: HashMap::new(), shelf: [0; 3], rows: None, whole: true, full: false };
        atlas.clear();
        Some(atlas)
    }

    /// Empty: only the white texel; the whole texture goes to the GPU again.
    pub fn clear(&mut self) {
        self.height = STEP;
        self.pixels = vec![Color32::TRANSPARENT; WIDTH * self.height];
        for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            self.pixels[y * WIDTH + x] = Color32::WHITE;
        }
        self.glyphs.clear();
        // the white texels and a pixel of room
        self.shelf = [3, 0, 3];
        (self.rows, self.whole, self.full) = (None, true, false);
    }

    pub fn size(&self) -> [usize; 2] {
        [WIDTH, self.height]
    }

    /// Whether a glyph of this frame found no room: `clear` and paint again.
    pub fn full(&self) -> bool {
        self.full
    }

    /// the face's line at `size` pixels
    pub fn metrics(&self, size: f32) -> Metrics {
        let f = size / self.font.units_per_em().unwrap_or(1000.0);
        let (ascent, descent, gap) = (ui(self.font.ascent_unscaled() * f), ui(self.font.descent_unscaled() * f), ui(self.font.line_gap_unscaled() * f));
        let advance = self.font.h_advance_unscaled(self.font.glyph_id('M')) * f;
        Metrics { ascent, row_height: ascent - descent + gap, advance }
    }

    /// Whether the face has every character of `text`.
    pub fn has_glyphs(&self, text: &str) -> bool {
        text.chars().all(|c| c == ' ' || self.font.glyph_id(c).0 != 0)
    }

    /// `text` on one line from (x, top) at `size` pixels in `color`: a quad a glyph on the atlas, in device pixels
    /// (egui's galley: its top at a whole pixel, each glyph's left binned to a quarter pixel, its texels on whole pixels).
    pub fn text(&mut self, mesh: &mut Mesh, text: &str, (x, top): (f32, f32), size: f32, color: Color32) {
        let m = self.metrics(size);
        let (x, base) = (x.round(), top.round() + m.ascent.round());
        let mut pen = 0.0;
        for c in text.chars() {
            let at = x + pen;
            pen += m.advance;
            let (whole, bin) = (at.floor(), ((at - at.floor()) * BINS).round());
            let (whole, bin) = if bin >= BINS { (whole + 1.0, 0.0) } else { (whole, bin) };
            let Some(g) = self.glyph(c, size, bin as u8) else { continue };
            let (w, h) = (WIDTH as f32, self.height as f32);
            let uv = Rect::from_min_size(pos2(g.at[0] as f32 / w, g.at[1] as f32 / h), vec2(g.size[0] as f32 / w, g.size[1] as f32 / h));
            let rect = Rect::from_min_size(pos2(whole + g.offset[0], base + g.offset[1]), vec2(g.size[0] as f32, g.size[1] as f32));
            mesh.add_rect_with_uv(rect, uv, color);
        }
    }

    /// a glyph's place, rastered and added the first time
    fn glyph(&mut self, c: char, size: f32, bin: u8) -> Option<Placed> {
        let key = (c, (size * 64.0).round() as u32, bin);
        if let Some(g) = self.glyphs.get(&key) {
            return *g;
        }
        let scale = PxScale::from(size * self.font.height_unscaled() / self.font.units_per_em().unwrap_or(1000.0));
        let glyph = self.font.glyph_id(c).with_scale_and_position(scale, ab_glyph::point(bin as f32 / BINS, 0.0));
        let Some(outline) = self.font.outline_glyph(glyph) else {
            self.glyphs.insert(key, None);
            return None;
        };
        let b = outline.px_bounds();
        let (w, h) = (b.width() as usize, b.height() as usize);
        let at = self.room(w, h)?;
        outline.draw(|x, y, cover| {
            let (x, y) = (at[0] + x as usize, at[1] + y as usize);
            if x < WIDTH && y < self.height {
                let c = cover.clamp(0.0, 1.0);
                self.pixels[y * WIDTH + x] = Color32::from_white_alpha(((2.0 * c - c * c) * 255.0).round() as u8);
            }
        });
        let rows = self.rows.map_or((at[1], at[1] + h), |(a, b)| (a.min(at[1]), b.max(at[1] + h)));
        self.rows = Some(rows);
        let placed = Some(Placed { at, size: [w, h], offset: [b.min.x, b.min.y] });
        self.glyphs.insert(key, placed);
        placed
    }

    /// room for w × h texels and a pixel around: on the shelf, on a new one, in new rows – or none (`full`)
    fn room(&mut self, w: usize, h: usize) -> Option<[usize; 2]> {
        let [mut x, mut y, mut row] = self.shelf;
        if x + w + 1 > WIDTH {
            (x, y, row) = (0, y + row + 1, 0);
        }
        if w + 1 > WIDTH || y + h + 1 > MAX_HEIGHT {
            self.full = true;
            return None;
        }
        while y + h + 1 > self.height {
            self.height += STEP;
            self.pixels.resize(WIDTH * self.height, Color32::TRANSPARENT);
            self.whole = true;
        }
        self.shelf = [x + w + 1, y, row.max(h)];
        Some([x, y])
    }

    /// What the GPU's copy lacks: the whole texture, or the rows written since the last delta.
    pub fn delta(&mut self) -> Option<ImageDelta> {
        let rows = self.rows.take();
        if std::mem::take(&mut self.whole) {
            return Some(ImageDelta::full(ColorImage::new([WIDTH, self.height], self.pixels.clone()), TextureOptions::LINEAR));
        }
        let (a, b) = rows?;
        let image = ColorImage::new([WIDTH, b - a], self.pixels[a * WIDTH..b * WIDTH].to_vec());
        Some(ImageDelta::partial([0, a], image, TextureOptions::LINEAR))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONO: &[u8] = include_bytes!("../../../outlet/assets/IBMPlexMono-Bold-latin.ttf");

    #[test]
    fn a_glyph_is_rastered_once_and_only_its_rows_are_sent_again() {
        let mut a = Atlas::new(MONO).expect("the mono face");
        assert!(a.delta().is_some_and(|d| d.pos.is_none()), "the whole texture first");
        assert!(a.delta().is_none(), "then nothing");
        let mut mesh = Mesh::default();
        a.text(&mut mesh, "AA A", (0.0, 0.0), 28.0, Color32::WHITE);
        assert_eq!(mesh.vertices.len() / 4, 3, "a quad a letter, none for the space");
        let d = a.delta().expect("the new glyph's rows");
        assert_eq!(d.pos, Some([0, 0]));
        let glyphs = a.glyphs.len();
        a.text(&mut mesh, "A", (0.0, 0.0), 28.0, Color32::WHITE);
        assert_eq!((a.glyphs.len(), a.delta().is_none()), (glyphs, true), "the same glyph from the atlas");
        assert_eq!(a.pixels[0], Color32::WHITE, "the white texel stays");
    }

    #[test]
    fn the_face_advances_its_0_6_em_and_a_full_atlas_says_so() {
        let mut a = Atlas::new(MONO).expect("the mono face");
        assert!((a.metrics(20.0).advance - 12.0).abs() < 0.01);
        let mut mesh = Mesh::default();
        let all: String = (33u8..127).map(char::from).collect();
        for size in (40..400).step_by(20) {
            a.text(&mut mesh, &all, (0.0, 0.0), size as f32, Color32::WHITE);
        }
        assert!(a.full(), "big glyphs fill it");
        a.clear();
        assert!(!a.full() && a.size() == [WIDTH, STEP]);
    }
}
