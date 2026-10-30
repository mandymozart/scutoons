//! A planned cell → what the outlet node draws (#186): the flat layers (slices, panels, pictures, a video's still,
//! text, surtitles, colour) painted back → front into one plane at the outlet's size, on the composer's own thread –
//! the way the chapter loader sets the dialog into a slice (CPU, ab_glyph). The 3D set is not painted: it is the
//! space node's job (`SpaceTarget`), seen through the plane's transparent parts. A layer that can't be drawn adds its
//! reason to the HUD and leaves its box empty.
use super::frame::{Frame, SpaceTarget};
use super::media::Media;
use super::plan::{LayerNode, Plan, Planned, Rect, SCROLL_FADE_MS};
use crate::library::{Chapter, Library};
use crate::text;
use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};
use protocol::Fit;
use std::collections::HashMap;
use std::sync::Arc;

/// what the slices are fetched at (1170 px wide: enough for a 1080p outlet)
const QUALITY: &str = "high";

pub struct Composed {
    pub frame: Frame,
    /// the space node's room and spot when the cell has a 3D set
    pub space: Option<SpaceTarget>,
    /// what this cell can't show, by layer (with the plan's own problems)
    pub problems: Vec<String>,
}

pub struct Composer {
    pub lib: Library,
    pub media: Arc<Media>,
    /// the outlet's size in pixels (the window's render size; 1920 × 1080 until it is known)
    pub size: (u32, u32),
    chapters: HashMap<(String, u32), Arc<Chapter>>,
}

impl Composer {
    pub fn new(lib: Library, media: Arc<Media>, size: (u32, u32)) -> Composer {
        Composer { lib, media, size, chapters: HashMap::new() }
    }

    /// the chapter once per composer (the API answers it once; the library's disk cache when offline)
    fn chapter(&mut self, toon: &str, n: u32) -> Result<Arc<Chapter>, String> {
        if let Some(c) = self.chapters.get(&(toon.to_owned(), n)) {
            return Ok(c.clone());
        }
        let (c, _) = self.lib.chapter(toon, n).map_err(|e| format!("{toon} {n}: {e:#}"))?;
        let c = Arc::new(c);
        self.chapters.insert((toon.to_owned(), n), c.clone());
        Ok(c)
    }

    /// slice `index` of a chapter (reading order, wrapping for a scroll) with its dialog set in
    fn slice(&mut self, toon: &str, n: u32, index: usize, wrap: bool) -> Result<RgbaImage, String> {
        let c = self.chapter(toon, n)?;
        let count = c.slices().count();
        if count == 0 {
            return Err(format!("{toon} {n} has no slices"));
        }
        let i = if wrap { index % count } else { index };
        let s = c.slices().nth(i).ok_or_else(|| format!("{toon} {n} has {count} slices, not {}", i + 1))?;
        let path = s.image_path(QUALITY).ok_or("a slice without an image")?;
        let file = self.lib.file(toon, path).map_err(|e| format!("{e:#}"))?;
        let mut img = image::open(&file).map_err(|e| format!("{path}: {e}"))?.to_rgba8();
        text::bake_dialog(&mut img, &s.dialog);
        Ok(img)
    }

    /// the dialog lines of slice `index` in `locale` (surtitles)
    fn dialog(&mut self, toon: &str, n: u32, index: usize, locale: &str) -> Result<String, String> {
        let c = self.chapter(toon, n)?;
        let Some(s) = c.slices().nth(index) else { return Ok(String::new()) };
        let lines: Vec<String> = s
            .dialog
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
                LayerNode::Image { media } => self.media.file_of(media).and_then(|p| decode(&p)).map(|img| paint(&mut plane, img, l)),
                LayerNode::Video { still, .. } => match still {
                    // the video itself is S4; its problem is the plan's – the still stands in where there is one
                    Some(s) => self.media.file_of(s).and_then(|p| decode(&p)).map(|img| paint(&mut plane, img, l)),
                    None => Ok(()),
                },
                LayerNode::Space { room, spot } => {
                    let room = self.media.file_of(room);
                    let r = room.as_ref().map(|_| ()).map_err(Clone::clone);
                    space = Some(SpaceTarget { room: room.ok(), spot: spot.clone() });
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
        let frame = Frame { plane, backdrop: space.is_some(), enter_ms, exit_ms };
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

fn decode(path: &std::path::Path) -> Result<RgbaImage, String> {
    image::open(path).map(|i| i.to_rgba8()).map_err(|e| format!("{}: {e}", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()))
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
    use super::super::media::tests::{temp_dir, FolderFetch};
    use super::super::media::{LocalSets, NoToken};
    use super::super::plan::plan;
    use super::*;
    use std::sync::Mutex;

    fn png(path: &std::path::Path, w: u32, h: u32, rgba: [u8; 4]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        RgbaImage::from_pixel(w, h, Rgba(rgba)).save(path).unwrap();
    }

    /// a cached chapter (the library's disk cache – the API at a closed port answers nothing) and the cathedral's files
    fn setup(name: &str) -> (std::path::PathBuf, Composer) {
        let root = temp_dir(name);
        let cache = root.join("cache");
        let chapter = r#"{ "title": "s1c2", "groups": [ { "slices": [
            { "image": "s1c2/a.png", "width": 390, "height": 624, "dialog": [ { "rect": { "x": 10, "y": 10, "width": 80 }, "text": { "de": "Die Predigt stand bevor", "en": "The sermon was at hand" } } ] },
            { "image": "s1c2/b.png", "width": 390, "height": 624 } ] } ] }"#;
        let ch = cache.join("api/toons/catpope/chapters/2.json");
        std::fs::create_dir_all(ch.parent().unwrap()).unwrap();
        std::fs::write(ch, chapter).unwrap();
        png(&cache.join("cdn/catpope/s1c2/a.png"), 39, 62, [200, 0, 0, 255]);
        png(&cache.join("cdn/catpope/s1c2/b.png"), 39, 62, [0, 0, 200, 255]);
        png(&root.join("sets/cathedral/choir.still.png"), 16, 9, [0, 200, 0, 255]);
        std::fs::create_dir_all(root.join("rooms")).unwrap();
        std::fs::write(root.join("rooms/cathedral.low.glb"), "glb").unwrap();
        let lib = Library { api: "offline://api".into(), cdn: "offline://cdn".into(), cache: cache.clone() };
        let media = Media {
            cache: cache.clone(),
            resolve: Box::new(LocalSets { rooms: root.join("rooms"), sets: root.join("sets"), quality: "low".into() }),
            fetch: Box::new(FolderFetch { root: root.join("cdn"), secret: None, calls: Mutex::new(vec![]) }),
            token: Box::new(NoToken),
            local: Mutex::new(HashMap::new()),
        };
        (root, Composer::new(lib, Arc::new(media), (160, 90)))
    }

    fn cell(json: &str) -> protocol::Cell {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_sermon_cell_paints_its_flat_layers_and_names_the_space() {
        let (root, mut c) = setup("sermon");
        let sermon = cell(
            r##"{ "mode": "own", "layers": [
            { "id": "room", "src": { "kind": "set", "p": { "set": "cathedral", "spot": "nave-centre", "mode": "model3d" } }, "enter": { "ms": 1200 } },
            { "id": "choir", "src": { "kind": "set", "p": { "set": "cathedral", "spot": "choir", "mode": "video" } }, "box": { "x": 0, "y": 0, "w": 25, "h": 25 } },
            { "id": "panel", "src": { "kind": "panel", "p": { "toon": "catpope", "chapter": 2, "slice": 0, "crop": { "x": 0, "y": 50, "w": 100, "h": 50 } } },
              "box": { "x": 50, "y": 0, "w": 50, "h": 50 }, "fit": "contain" },
            { "id": "line", "src": { "kind": "text", "p": { "text": { "de": "Liebe Gemeinde" }, "size": 12, "colour": "#ffd700" } }, "box": { "x": 0, "y": 70, "w": 100, "h": 30 } },
            { "id": "gone", "src": { "kind": "image", "p": { "media": "nope" } } }
        ] }"##,
        );
        let p = plan(&sermon, "de", |_| true);
        let out = c.compose(&p, None, 0);
        let f = &out.frame;
        assert_eq!((f.plane.width(), f.plane.height(), f.backdrop, f.enter_ms), (160, 90, true, 1200));
        assert_eq!(out.space, Some(SpaceTarget { room: Some(root.join("rooms/cathedral.low.glb")), spot: Some("nave-centre".into()) }));
        assert_eq!(f.plane.get_pixel(5, 5).0, [0, 200, 0, 255], "the choir's still stands in for its video");
        assert_eq!(f.plane.get_pixel(120, 20).0, [200, 0, 0, 255], "the panel of slice 0, contained in its box");
        assert_eq!(f.plane.get_pixel(85, 20).0[3], 0, "beside the contained panel: the backdrop shows");
        assert!((0..160).any(|x| (70..90).any(|y| f.plane.get_pixel(x, y).0 == [255, 215, 0, 255])), "the line in gold");
        assert_eq!(out.problems, ["choir: the stage plays no video yet (S4, #179) – its still instead", "gone: 'nope' is not preloaded (yet)"]);
    }

    #[test]
    fn scroll_surtitles_colour_and_a_missing_chapter() {
        let (_root, mut c) = setup("scroll");
        let p = plan(
            &cell(
                r##"{ "mode": "own", "layers": [
                { "id": "s", "src": { "kind": "scroll", "p": { "toon": "catpope", "chapter": 2 } } },
                { "id": "sur", "src": { "kind": "surtitles", "p": { "toon": "catpope", "chapter": 2, "locale": "en" } }, "box": { "x": 0, "y": 80, "w": 100, "h": 20 } },
                { "id": "wash", "src": { "kind": "colour", "p": { "colour": "#00000080" } }, "box": { "x": 0, "y": 0, "w": 10, "h": 10 } },
                { "id": "lost", "src": { "kind": "slice", "p": { "toon": "catpope", "chapter": 9, "slice": 0 } } }
            ] }"##,
            ),
            "de",
            |_| true,
        );
        let first = c.compose(&p, Some(0), 0);
        assert!(!first.frame.backdrop && first.space.is_none());
        assert_eq!(first.frame.plane.get_pixel(80, 40).0, [200, 0, 0, 255], "scroll step 0 = slice a (cover)");
        let wash = first.frame.plane.get_pixel(2, 2).0;
        assert!(wash[0] < 120 && wash[3] >= 250, "the wash darkens the slice: {wash:?}");
        assert!((0..160).any(|x| (72..90).any(|y| first.frame.plane.get_pixel(x, y).0[1] > 150)), "the surtitle line (white over the red slice)");
        assert_eq!(first.problems.len(), 1);
        assert!(first.problems[0].starts_with("lost: catpope 9:"), "{:?}", first.problems);
        let next = c.scroll_step(&p, Some(1), 1);
        assert_eq!(next.frame.plane.get_pixel(80, 40).0, [0, 0, 200, 255], "step 1 = slice b");
        assert_eq!((next.frame.enter_ms, next.frame.exit_ms), (SCROLL_FADE_MS, SCROLL_FADE_MS));
        let wrapped = c.compose(&p, None, 2);
        assert_eq!(wrapped.frame.plane.get_pixel(80, 40).0, [200, 0, 0, 255], "a scroll wraps");
    }

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
