//! The composer's shelf on this deck (#186, #378): deck-outlet's composer paints the cell; what it reads comes from here –
//! a chapter from the library (the API once per composer, the disk cache when offline), its slices fetched at the outlet's
//! quality and decoded in Rust, the preloaded media and the sets' files (media.rs).
use super::media::Media;
use crate::library::{Chapter, Library};
use deck_outlet::compose::Shelf;
use deck_outlet::plan::MediaRef;
use deck_outlet::text::Dialog;
use image::RgbaImage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// what the slices are fetched at (1170 px wide: enough for a 1080p outlet)
const QUALITY: &str = "high";

/// The outlet's composer on this deck.
pub type Composer = deck_outlet::compose::Composer<LibraryShelf>;

/// a composer of `size` pixels reading `lib` and `media`
pub fn composer(lib: Library, media: Arc<Media>, size: (u32, u32)) -> Composer {
    Composer::new(LibraryShelf { lib, media, chapters: HashMap::new() }, size)
}

pub struct LibraryShelf {
    pub lib: Library,
    pub media: Arc<Media>,
    chapters: HashMap<(String, u32), Chapter>,
}

impl LibraryShelf {
    /// the chapter once per composer (the API answers it once; the library's disk cache when offline)
    fn chapter(&mut self, toon: &str, n: u32) -> Result<&Chapter, String> {
        let key = (toon.to_owned(), n);
        if !self.chapters.contains_key(&key) {
            let (c, _) = self.lib.chapter(toon, n).map_err(|e| format!("{toon} {n}: {e:#}"))?;
            self.chapters.insert(key.clone(), c);
        }
        Ok(&self.chapters[&key])
    }
}

impl Shelf for LibraryShelf {
    fn slices(&mut self, toon: &str, n: u32) -> Result<usize, String> {
        Ok(self.chapter(toon, n)?.slices().count())
    }

    fn picture(&mut self, toon: &str, n: u32, index: usize) -> Result<RgbaImage, String> {
        let c = self.chapter(toon, n)?;
        let count = c.slices().count();
        let s = c.slices().nth(index).ok_or_else(|| format!("{toon} {n} has {count} slices, not {}", index + 1))?;
        let path = s.image_path(QUALITY).ok_or("a slice without an image")?.to_owned();
        let file = self.lib.file(toon, &path).map_err(|e| format!("{e:#}"))?;
        Ok(image::open(&file).map_err(|e| format!("{path}: {e}"))?.to_rgba8())
    }

    fn dialog(&mut self, toon: &str, n: u32, index: usize) -> Result<&[Dialog], String> {
        Ok(self.chapter(toon, n)?.slices().nth(index).map_or(&[][..], |s| &s.dialog[..]))
    }

    fn image(&mut self, media: &MediaRef) -> Result<RgbaImage, String> {
        self.media.file_of(media).and_then(|p| decode(&p))
    }

    fn set(&mut self, media: &MediaRef) -> Result<PathBuf, String> {
        self.media.file_of(media)
    }
}

fn decode(path: &std::path::Path) -> Result<RgbaImage, String> {
    image::open(path).map(|i| i.to_rgba8()).map_err(|e| format!("{}: {e}", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::super::media::tests::{temp_dir, FolderFetch};
    use super::super::media::{LocalSets, NoToken};
    use super::*;
    use deck_outlet::frame::SpaceTarget;
    use deck_outlet::plan::{plan, SCROLL_FADE_MS};
    use image::Rgba;
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
        std::fs::create_dir_all(root.join("sets/cathedral")).unwrap();
        std::fs::write(root.join("sets/cathedral/cathedral.low.glb"), "glb").unwrap();
        let lib = Library { api: "offline://api".into(), cdn: "offline://cdn".into(), cache: cache.clone() };
        let media = Media {
            cache: cache.clone(),
            resolve: Box::new(LocalSets { sets: root.join("sets"), quality: "low".into(), synced: false }),
            fetch: Box::new(FolderFetch { root: root.join("cdn"), secret: None, calls: Mutex::new(vec![]) }),
            token: Box::new(NoToken),
            local: Mutex::new(HashMap::new()),
        };
        (root, composer(lib, Arc::new(media), (160, 90)))
    }

    fn cell(json: &str) -> protocol::Cell {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_sermon_cell_paints_its_flat_layers_and_names_the_space() {
        let (root, mut c) = setup("sermon");
        let sermon = cell(
            r##"{ "mode": "own", "layers": [
            { "id": "set", "src": { "kind": "set", "p": { "set": "cathedral", "spot": "nave-centre", "mode": "model3d" } }, "enter": { "ms": 1200 } },
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
        assert_eq!(out.space, Some(SpaceTarget { set: Some(root.join("sets/cathedral/cathedral.low.glb")), spot: Some("nave-centre".into()) }));
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
}
