//! A planned cell → what the outlet node draws (#186): the flat layers (slices, panels, pictures, a video's still,
//! text, surtitles, colour) painted back → front into one plane at the outlet's size – the way the chapter loader sets
//! the dialog into a slice (CPU, ab_glyph). The 3D set is not painted: it is the space node's job (`SpaceTarget`), seen
//! through the plane's transparent parts. A layer that can't be drawn adds its reason to the HUD and leaves its box empty.
//! What the composer reads – a chapter's slices and dialog, a preloaded picture, a set's 3D file – comes through its
//! `Shelf` (deck-native: the library API + disk cache and the preloaded media, decoded in Rust).
use crate::frame::{Frame, SpaceTarget};
use crate::plan::{LayerNode, MediaRef, Plan, Planned, Rect, SCROLL_FADE_MS};
use crate::text::{self, Dialog};
use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};
use protocol::Fit;
use std::path::PathBuf;

/// What the composer reads; each `Err` is the reason the HUD shows for the layer.
pub trait Shelf {
    /// how many slices chapter `n` of `toon` has (reading order)
    fn slices(&mut self, toon: &str, n: u32) -> Result<usize, String>;
    /// slice `index`'s picture, decoded (without its dialog)
    fn picture(&mut self, toon: &str, n: u32, index: usize) -> Result<RgbaImage, String>;
    /// slice `index`'s dialog (none past the chapter's end)
    fn dialog(&mut self, toon: &str, n: u32, index: usize) -> Result<&[Dialog], String>;
    /// a preloaded picture or a set spot's still, decoded
    fn image(&mut self, media: &MediaRef) -> Result<RgbaImage, String>;
    /// a 3D set's file (for the space node)
    fn set(&mut self, media: &MediaRef) -> Result<PathBuf, String>;
}

pub struct Composed {
    pub frame: Frame,
    /// the space node's set and spot when the cell has a 3D set
    pub space: Option<SpaceTarget>,
    /// what this cell can't show, by layer (with the plan's own problems)
    pub problems: Vec<String>,
}

pub struct Composer<S> {
    pub shelf: S,
    /// the outlet's size in pixels (the window's render size; 1920 × 1080 until it is known)
    pub size: (u32, u32),
}

impl<S: Shelf> Composer<S> {
    pub fn new(shelf: S, size: (u32, u32)) -> Composer<S> {
        Composer { shelf, size }
    }

    /// slice `index` of a chapter (reading order, wrapping for a scroll) with its dialog set in
    fn slice(&mut self, toon: &str, n: u32, index: usize, wrap: bool) -> Result<RgbaImage, String> {
        let count = self.shelf.slices(toon, n)?;
        if count == 0 {
            return Err(format!("{toon} {n} has no slices"));
        }
        let i = if wrap { index % count } else { index };
        if i >= count {
            return Err(format!("{toon} {n} has {count} slices, not {}", i + 1));
        }
        let mut img = self.shelf.picture(toon, n, i)?;
        text::bake_dialog(&mut img, self.shelf.dialog(toon, n, i)?);
        Ok(img)
    }

    /// the dialog lines of slice `index` in `locale` (surtitles)
    fn dialog(&mut self, toon: &str, n: u32, index: usize, locale: &str) -> Result<String, String> {
        let lines: Vec<String> = self
            .shelf
            .dialog(toon, n, index)?
            .iter()
            .filter_map(|d| match &d.text {
                serde_json::Value::String(t) => Some(t.clone()),
                serde_json::Value::Object(m) => m.get(locale).or_else(|| m.get("en")).or_else(|| m.values().next()).and_then(|v| v.as_str()).map(str::to_owned),
                _ => None,
            })
            .filter(|t| !t.trim().is_empty())
            .collect();
        Ok(lines.join("\n"))
    }

    /// Paint `plan` – `slice` = the show's current slice (surtitles), `scroll` = the running chapter's step.
    pub fn compose(&mut self, plan: &Plan, slice: Option<usize>, scroll: usize) -> Composed {
        let (w, h) = self.size;
        let mut plane = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
        let mut problems = plan.problems.clone();
        let mut space = None;
        for l in &plan.layers {
            let drawn = match &l.node {
                LayerNode::Chapter { toon, chapter } => self.slice(toon, *chapter, scroll, true).map(|img| paint(&mut plane, img, l)),
                LayerNode::Slice { toon, chapter, slice, crop } => self.slice(toon, *chapter, *slice, false).map(|img| {
                    let img = match crop {
                        Some(c) => crop_to(&img, c),
                        None => img,
                    };
                    paint(&mut plane, img, l)
                }),
                LayerNode::Image { media } => self.shelf.image(media).map(|img| paint(&mut plane, img, l)),
                LayerNode::Video { still, .. } => match still {
                    // the video itself is S4; its problem is the plan's – the still stands in where there is one
                    Some(s) => self.shelf.image(s).map(|img| paint(&mut plane, img, l)),
                    None => Ok(()),
                },
                LayerNode::Space { set, spot } => {
                    let set = self.shelf.set(set);
                    let r = set.as_ref().map(|_| ()).map_err(Clone::clone);
                    space = Some(SpaceTarget { set: set.ok(), spot: spot.clone() });
                    r
                }
                LayerNode::Text { text, size, colour } => {
                    write(&mut plane, l, text, *size, *colour);
                    Ok(())
                }
                LayerNode::Surtitles { toon, chapter, locale } => match slice {
                    Some(i) => self.dialog(toon, *chapter, i, locale).map(|t| write(&mut plane, l, &t, 5.0, [255; 4])),
                    None => Ok(()),
                },
                LayerNode::Solid { rgba } => {
                    fill(&mut plane, l, *rgba);
                    Ok(())
                }
                LayerNode::VideoIn { .. } => Ok(()),
            };
            if let Err(why) = drawn {
                problems.push(format!("{}: {why}", l.id));
            }
        }
        let (enter_ms, exit_ms) = plan.fades();
        // a 3D set, or a show's piece the deck plays (#91): the space shows through
        let frame = Frame { plane, backdrop: space.is_some() || plan.piece, enter_ms, exit_ms };
        Composed { frame, space, problems }
    }

    /// The next step of a running chapter (a scroll): the same cell, a short cut.
    pub fn scroll_step(&mut self, plan: &Plan, slice: Option<usize>, scroll: usize) -> Composed {
        let mut c = self.compose(plan, slice, scroll);
        c.frame.enter_ms = SCROLL_FADE_MS;
        c.frame.exit_ms = SCROLL_FADE_MS;
        c
    }
}

/// the layer's box in pixels of `plane`
fn px(plane: &RgbaImage, r: &Rect) -> (u32, u32, u32, u32) {
    let (w, h) = (plane.width() as f32, plane.height() as f32);
    let x = (r.x * w).round() as u32;
    let y = (r.y * h).round() as u32;
    ((x), (y), ((r.w * w).round() as u32).max(1), ((r.h * h).round() as u32).max(1))
}

/// a crop in 0–1 of an image
pub fn crop_to(img: &RgbaImage, c: &Rect) -> RgbaImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let (x, y) = ((c.x * w) as u32, (c.y * h) as u32);
    let (cw, ch) = (((c.w * w) as u32).max(1).min(img.width() - x.min(img.width() - 1)), ((c.h * h) as u32).max(1).min(img.height() - y.min(img.height() - 1)));
    imageops::crop_imm(img, x.min(img.width() - 1), y.min(img.height() - 1), cw, ch).to_image()
}

/// `img` into the layer's box: contain = whole, centred; cover = filling the box, the overflow cut off centred
pub fn fit_into(img: &RgbaImage, bw: u32, bh: u32, fit: Fit) -> (RgbaImage, u32, u32) {
    let (iw, ih) = (img.width().max(1) as f32, img.height().max(1) as f32);
    let k = match fit {
        Fit::Contain => (bw as f32 / iw).min(bh as f32 / ih),
        Fit::Cover => (bw as f32 / iw).max(bh as f32 / ih),
    };
    let (sw, sh) = (((iw * k).round() as u32).max(1), ((ih * k).round() as u32).max(1));
    let scaled = imageops::resize(img, sw, sh, FilterType::Triangle);
    match fit {
        Fit::Contain => (scaled, (bw - sw.min(bw)) / 2, (bh - sh.min(bh)) / 2),
        Fit::Cover => {
            let (cx, cy) = ((sw - bw.min(sw)) / 2, (sh - bh.min(sh)) / 2);
            (imageops::crop_imm(&scaled, cx, cy, bw.min(sw), bh.min(sh)).to_image(), 0, 0)
        }
    }
}

/// a picture into its box, with the layer's opacity
fn paint(plane: &mut RgbaImage, img: RgbaImage, l: &Planned) {
    let (x, y, bw, bh) = px(plane, &l.rect);
    let (mut img, dx, dy) = fit_into(&img, bw, bh, l.fit);
    if l.opacity < 1.0 {
        for p in img.pixels_mut() {
            p.0[3] = (p.0[3] as f32 * l.opacity).round() as u8;
        }
    }
    imageops::overlay(plane, &img, (x + dx) as i64, (y + dy) as i64);
}

/// text into its box: the line height `size` % of the outlet's height, centred in the box
fn write(plane: &mut RgbaImage, l: &Planned, text: &str, size: f32, colour: [u8; 4]) {
    let (x, y, bw, bh) = px(plane, &l.rect);
    let line = size / 100.0 * plane.height() as f32;
    let a = (colour[3] as f32 * l.opacity).round() as u8;
    if let Some(img) = text::text_lines(text, bw, line, [colour[0], colour[1], colour[2], a], bh) {
        let dy = (bh - img.height().min(bh)) / 2;
        imageops::overlay(plane, &img, x as i64, (y + dy) as i64);
    }
}

/// a colour over the box (alpha-blended)
fn fill(plane: &mut RgbaImage, l: &Planned, rgba: [u8; 4]) {
    let (x, y, bw, bh) = px(plane, &l.rect);
    let a = (rgba[3] as f32 * l.opacity).round() as u8;
    let solid = RgbaImage::from_pixel(bw, bh, Rgba([rgba[0], rgba[1], rgba[2], a]));
    imageops::overlay(plane, &solid, x as i64, y as i64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_contain_and_cover() {
        let img = RgbaImage::from_pixel(40, 20, Rgba([1, 2, 3, 255]));
        let (c, dx, dy) = fit_into(&img, 20, 20, Fit::Contain);
        assert_eq!((c.width(), c.height(), dx, dy), (20, 10, 0, 5));
        let (c, dx, dy) = fit_into(&img, 20, 20, Fit::Cover);
        assert_eq!((c.width(), c.height(), dx, dy), (20, 20, 0, 0));
        let half = crop_to(&img, &Rect { x: 0.5, y: 0.0, w: 0.5, h: 1.0 });
        assert_eq!((half.width(), half.height()), (20, 20));
    }
}
