//! The beings' heads' sprites (#188 phase 5): a character's portrait mood sprites, `<char>_portrait-<mood>.png` /
//! `.webp` (the asset naming, agents/ASSETS.md), loaded on a thread of their own, halved to `MAX` and mipmapped there –
//! the frame that uploads one only copies. A head asks for its character's moods once (`request`); a mood whose sprite
//! is missing shows the nearest one that is there (toward the calm end), a character without any shows a faceless head.
//!
//! #495: the same loader carries the stage's cards' pictures – a character's `full` view per mood, an item's picture
//! (cast.rs `Picture`): a slot is a picture, the heads ask for the portrait view.
//!
//! Where the sprites come from is a `Source`. Today `LocalDir`: `--portraits DIR` (`DECK_PORTRAITS`, default
//! `<cache>/portraits`), either `DIR/<char>/<char>_portrait-<mood>.<ext>` (the layout of media/characters/) or flat in
//! `DIR`. **#180:** the deck-token fetch from the CDN (cached on DECKDATA) becomes a second `Source` that downloads into
//! that same folder and then reads it – the heads don't change.
//!
//! Without threads (the browser, #198) the Portraits are `manual`: the host takes the requests (`requests`), fetches
//! and decodes the sprites itself and hands them in (`deliver`).
use super::glb::{half, Image};
use crate::cast::{Picture, MEDIA, PORTRAIT_VIEW};
#[cfg(feature = "native")]
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

/// a sprite's longest side on the GPU (a head is at most a third of the screen)
pub const MAX: u32 = 512;
/// a media picture's longest side (a background covers the screen)
pub const MEDIA_MAX: u32 = 2048;

fn max_side(picture: &Picture) -> u32 {
    if picture.kind == MEDIA {
        MEDIA_MAX
    } else {
        MAX
    }
}
#[cfg(feature = "native")]
const EXTENSIONS: [&str; 2] = ["png", "webp"];

/// where the pictures come from
#[cfg(feature = "native")]
pub trait Source: Send + 'static {
    /// `picture` – its file, decoded (RGBA8); Err = why there is none
    fn load(&self, picture: &Picture) -> Result<image::RgbaImage, String>;
    /// `picture`'s file as it is (a video's, for deck-media)
    fn bytes(&self, picture: &Picture) -> Result<Vec<u8>, String> {
        Err(format!("{}: no file to read here", picture.id))
    }
}

/// a local folder: a character's `<dir>/<char>/<char>_<view>-<mood>.<png|webp>`, else `<dir>/<char>_<view>-<mood>.<…>`;
/// an item's `<dir>/items/<id>.<…>`, else `<dir>/<id>.<…>`
#[cfg(feature = "native")]
pub struct LocalDir(pub PathBuf);

#[cfg(feature = "native")]
impl LocalDir {
    /// the first file that exists for this picture
    pub fn find(&self, picture: &Picture) -> Option<PathBuf> {
        if picture.kind == MEDIA {
            return Some(self.0.join(&picture.id)).filter(|p| p.is_file());
        }
        let name = picture.stem();
        let sub = if picture.kind == "items" { "items" } else { picture.id.as_str() };
        [self.0.join(sub), self.0.clone()]
            .into_iter()
            .flat_map(|d| EXTENSIONS.map(|e| d.join(format!("{name}.{e}"))))
            .find(|p| p.is_file())
    }
}

#[cfg(feature = "native")]
impl Source for LocalDir {
    fn load(&self, picture: &Picture) -> Result<image::RgbaImage, String> {
        let Some(path) = self.find(picture) else {
            return Err(format!("no {} in {}", picture.stem(), self.0.display()));
        };
        image::open(&path).map(|i| i.to_rgba8()).map_err(|e| format!("{}: {e}", path.display()))
    }

    fn bytes(&self, picture: &Picture) -> Result<Vec<u8>, String> {
        let path = self.find(picture).ok_or_else(|| format!("no {} in {}", picture.stem(), self.0.display()))?;
        std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// A folder whose files arrive from elsewhere (a deck's sync): `wait` until a picture's file is there, then read it.
#[cfg(feature = "native")]
pub struct Arriving<W> {
    pub dir: LocalDir,
    pub wait: W,
}

#[cfg(feature = "native")]
impl<W: Fn(&Picture) + Send + 'static> Source for Arriving<W> {
    fn load(&self, picture: &Picture) -> Result<image::RgbaImage, String> {
        (self.wait)(picture);
        self.dir.load(picture)
    }

    fn bytes(&self, picture: &Picture) -> Result<Vec<u8>, String> {
        (self.wait)(picture);
        self.dir.bytes(picture)
    }
}

/// whether a picture is a video (by its media path) – it plays on deck-media, frame by frame into its slot
#[cfg(feature = "native")]
fn moving(picture: &Picture) -> bool {
    picture.kind == MEDIA && deck_catalog::background::is_video(&picture.id)
}

/// what the loader hands back for a slot
enum Got {
    Image(Image),
    #[cfg(feature = "media")]
    Movie(crate::video::Movie),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Loading,
    /// decoded; uploaded once the renderer took it
    Ready,
    Missing(String),
}

#[derive(Debug, Clone)]
pub struct Slot {
    pub picture: Picture,
    pub state: State,
    /// its width over its height once it came (a card's width follows it)
    pub aspect: f32,
}

/// The sprites every head asked for, by slot (the renderer keeps a texture per ready slot).
pub struct Portraits {
    pub slots: Vec<Slot>,
    /// the loader thread's inbox; None = manual (the host asks: `requests`)
    ask: Option<Sender<(usize, Picture)>>,
    asked: Vec<(usize, Picture)>,
    got: Receiver<(usize, Result<Got, String>)>,
    tell: Sender<(usize, Result<Got, String>)>,
    /// decoded, not yet on the GPU
    fresh: Vec<(usize, Image)>,
    /// the video slots' players
    #[cfg(feature = "media")]
    movies: Vec<(usize, crate::video::Movie)>,
    /// video frames for their slots' textures, not yet on the GPU
    #[cfg(feature = "media")]
    frames: Vec<(usize, std::sync::Arc<deck_media::Picture>)>,
}

/// a picture's file → what its slot holds
#[cfg(feature = "native")]
fn load(source: &dyn Source, picture: &Picture) -> Result<Got, String> {
    if moving(picture) {
        #[cfg(feature = "media")]
        return crate::video::Movie::spawn(source.bytes(picture)?, &picture.id).map(Got::Movie);
        #[cfg(not(feature = "media"))]
        return Err(format!("{}: this deck is built without video (the media module)", picture.id));
    }
    source.load(picture).map(|i| Got::Image(prepare(i, max_side(picture))))
}

impl Portraits {
    /// the loader thread over `source` (it ends with the Portraits)
    #[cfg(feature = "native")]
    pub fn new(source: Box<dyn Source>) -> Portraits {
        let (ask, asked) = mpsc::channel::<(usize, Picture)>();
        let (tell, got) = mpsc::channel();
        let teller = tell.clone();
        std::thread::Builder::new()
            .name("portraits".into())
            .spawn(move || {
                while let Ok((slot, picture)) = asked.recv() {
                    let r = load(&*source, &picture);
                    if let Err(e) = &r {
                        tracing::debug!("portrait: {e}");
                    }
                    if teller.send((slot, r)).is_err() {
                        break;
                    }
                }
            })
            .expect("a thread");
        Portraits::with(Some(ask), got, tell)
    }

    fn with(ask: Option<Sender<(usize, Picture)>>, got: Receiver<(usize, Result<Got, String>)>, tell: Sender<(usize, Result<Got, String>)>) -> Portraits {
        Portraits {
            slots: Vec::new(),
            ask,
            asked: Vec::new(),
            got,
            tell,
            fresh: Vec::new(),
            #[cfg(feature = "media")]
            movies: Vec::new(),
            #[cfg(feature = "media")]
            frames: Vec::new(),
        }
    }

    /// from a folder (`--portraits`)
    #[cfg(feature = "native")]
    pub fn local(dir: &Path) -> Portraits {
        Portraits::new(Box::new(LocalDir(dir.to_path_buf())))
    }

    /// no loader: the host takes the requests and delivers the sprites (the browser)
    pub fn manual() -> Portraits {
        let (tell, got) = mpsc::channel();
        Portraits::with(None, got, tell)
    }

    /// the pictures asked for since the last call – manual Portraits only
    pub fn requests(&mut self) -> Vec<(usize, Picture)> {
        std::mem::take(&mut self.asked)
    }

    /// a requested sprite, decoded by the host (RGBA8), or why there is none – it shows from the next `poll`
    pub fn deliver(&mut self, slot: usize, sprite: Result<image::RgbaImage, String>) {
        let max = self.slots.get(slot).map_or(MAX, |s| max_side(&s.picture));
        let _ = self.tell.send((slot, sprite.map(|i| Got::Image(prepare(i, max)))));
    }

    /// a requested video's file (`moving` pictures), fetched by the host; it plays from the next `poll`, decoded on the
    /// frame's thread
    #[cfg(feature = "media")]
    pub fn deliver_video(&mut self, slot: usize, file: Result<Vec<u8>, String>) {
        let _ = self.tell.send((slot, file.and_then(crate::video::Movie::open).map(Got::Movie)));
    }

    /// Each frame: every video slot's frame for the heard time `heard` (s), when it changed, waits for the GPU.
    #[cfg(feature = "media")]
    pub fn play(&mut self, heard: f64) {
        for (slot, m) in &mut self.movies {
            if let Some(p) = m.next_picture(heard) {
                self.frames.push((*slot, p));
            }
        }
    }

    /// the video frames for their slots' textures, once
    #[cfg(feature = "media")]
    pub fn take_frames(&mut self) -> Vec<(usize, std::sync::Arc<deck_media::Picture>)> {
        std::mem::take(&mut self.frames)
    }

    /// A portrait slot per mood of `character` (the same character and mood share one).
    pub fn request(&mut self, character: &str, moods: &[String]) -> Vec<usize> {
        self.request_view(character, PORTRAIT_VIEW, moods)
    }

    /// A slot per mood of `character`'s `view` (#495: a card's `full`).
    pub fn request_view(&mut self, character: &str, view: &'static str, moods: &[String]) -> Vec<usize> {
        if character.is_empty() {
            return Vec::new();
        }
        moods.iter().map(|mood| self.picture(Picture::character(character, view, mood))).collect()
    }

    /// The slot of `picture` (asked for once).
    pub fn picture(&mut self, picture: Picture) -> usize {
        if let Some(i) = self.slots.iter().position(|s| s.picture == picture) {
            return i;
        }
        let i = self.slots.len();
        match &self.ask {
            Some(ask) => {
                let _ = ask.send((i, picture.clone()));
            }
            None => self.asked.push((i, picture.clone())),
        }
        self.slots.push(Slot { picture, state: State::Loading, aspect: 1.0 });
        i
    }

    /// what the loader has finished (each frame; nothing new = no work)
    pub fn poll(&mut self) {
        while let Ok((slot, r)) = self.got.try_recv() {
            self.slots[slot].state = match r {
                Ok(Got::Image(img)) => {
                    let (w, h) = img.size();
                    self.slots[slot].aspect = w as f32 / h.max(1) as f32;
                    self.fresh.push((slot, img));
                    State::Ready
                }
                #[cfg(feature = "media")]
                Ok(Got::Movie(m)) => {
                    self.slots[slot].aspect = m.aspect();
                    self.movies.push((slot, m));
                    State::Ready
                }
                Err(e) => State::Missing(e),
            };
        }
    }

    /// the decoded sprites for the renderer, once
    pub fn take(&mut self) -> Vec<(usize, Image)> {
        std::mem::take(&mut self.fresh)
    }

    pub fn ready(&self, slot: usize) -> bool {
        self.slots.get(slot).is_some_and(|s| s.state == State::Ready)
    }

    /// The slot a head shows for mood `mood` of its `sprites`: that mood's when it is ready, else the nearest ready one
    /// toward the calm end, then toward the wild end; None = no face (yet). No allocation.
    pub fn pick(&self, sprites: &[usize], mood: usize) -> Option<usize> {
        let m = mood.min(sprites.len().checked_sub(1)?);
        (0..=m).rev().chain(m + 1..sprites.len()).map(|i| sprites[i]).find(|&s| self.ready(s))
    }

    /// The slot a head shows for mood `mood`: its own sprite, else the first ready one of `near` (the related moods,
    /// portrait::near), else `pick`'s walk toward calm. No allocation.
    pub fn pick_near(&self, sprites: &[usize], near: Option<&Vec<usize>>, mood: usize) -> Option<usize> {
        near.into_iter().flatten().filter_map(|&i| sprites.get(i).copied()).find(|&s| self.ready(s)).or_else(|| self.pick(sprites, mood))
    }

    /// the HUD's word on a character's sprites: how many are there, the first reason one is missing
    pub fn status(&self, character: &str) -> String {
        let mine: Vec<&Slot> = self.slots.iter().filter(|s| s.picture.kind == "characters" && s.picture.view == PORTRAIT_VIEW && s.picture.id == character).collect();
        let ready = mine.iter().filter(|s| s.state == State::Ready).count();
        let why = mine.iter().find_map(|s| if let State::Missing(w) = &s.state { Some(w.as_str()) } else { None });
        match why {
            Some(w) if ready == 0 => format!("{character} NO FACE: {w}"),
            _ => format!("{character} {ready}/{} MOODS", mine.len()),
        }
    }
}

/// a sprite → at most MAX, with its mip chain
/// The complete mood set of the characters' sprites (production 2026-10-30: 17 every full set has, + `dizzy` (the
/// cat popes, the helmet mouse), `dying` (billthefrog), `tripping` (yellowmouse)), each with the moods nearest to it –
/// what a head shows while that mood's sprite is missing.
pub const NEAR: [(&str, &[&str]); 20] = [
    ("neutral", &["thinking", "acceptance"]),
    ("thinking", &["confused", "neutral"]),
    ("happy", &["laughing", "smug", "neutral"]),
    ("laughing", &["happy", "smug"]),
    ("smug", &["happy", "laughing"]),
    ("surprised", &["scared", "confused"]),
    ("scared", &["surprised", "crying"]),
    ("furious", &["angry", "disgusted"]),
    ("angry", &["furious", "disgusted"]),
    ("disgusted", &["angry", "embarrassed"]),
    ("embarrassed", &["smug", "confused"]),
    ("confused", &["thinking", "dizzy", "surprised"]),
    ("dizzy", &["confused", "tripping", "surprised"]),
    ("tripping", &["dizzy", "confused", "laughing"]),
    ("sad", &["crying", "acceptance"]),
    ("crying", &["sad", "dying"]),
    ("dying", &["crying", "sad", "scared"]),
    ("denial", &["angry", "bargaining"]),
    ("bargaining", &["denial", "thinking"]),
    ("acceptance", &["sad", "neutral"]),
];

/// per mood of `moods`: itself, then its NEAR moods that are in `moods` (indices into it)
pub fn near(moods: &[String]) -> Vec<Vec<usize>> {
    moods
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let related = NEAR.iter().find(|(n, _)| n == m).map_or(&[][..], |(_, r)| *r);
            std::iter::once(i).chain(related.iter().filter_map(|r| moods.iter().position(|x| x == r))).collect()
        })
        .collect()
}

fn prepare(img: image::RgbaImage, max: u32) -> Image {
    let mut img = img;
    while img.width().max(img.height()) > max {
        img = image::imageops::resize(&img, (img.width() / 2).max(1), (img.height() / 2).max(1), image::imageops::FilterType::Triangle);
    }
    let mut levels = vec![(img.width(), img.height(), img.into_raw())];
    while let Some(&(w, h, ref px)) = levels.last().filter(|(w, h, _)| *w > 1 || *h > 1) {
        let next = half(w, h, px);
        levels.push(next);
    }
    Image { levels }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// a source that has only some moods
    struct Some2;
    impl Source for Some2 {
        fn load(&self, p: &Picture) -> Result<image::RgbaImage, String> {
            match (p.id.as_str(), p.view, p.mood.as_str()) {
                ("plf", PORTRAIT_VIEW, "neutral" | "laughing") => Ok(image::RgbaImage::from_pixel(1024, 1024, image::Rgba([200, 100, 50, 255]))),
                ("plf", crate::cast::CARD_VIEW, "neutral") => Ok(image::RgbaImage::from_pixel(400, 1000, image::Rgba([1, 2, 3, 255]))),
                _ => Err(format!("no {}", p.stem())),
            }
        }
    }

    fn settle(p: &mut Portraits) {
        // until the loader has answered every slot, however loaded the machine; only a hang runs into the minute, and fails
        let until = Instant::now() + Duration::from_secs(60);
        while p.slots.iter().any(|s| s.state == State::Loading) {
            assert!(Instant::now() < until, "the loader never answered");
            p.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn moods_fall_back_to_the_nearest_sprite_there_is() {
        let mut p = Portraits::new(Box::new(Some2));
        let moods: Vec<String> = crate::space::being::MOODS.iter().map(|m| m.to_string()).collect();
        let a = p.request("plf", &moods);
        let b = p.request("plf", &moods);
        assert_eq!(a, b, "one slot per character and mood");
        assert!(p.request("", &moods).is_empty(), "no character, no face");
        assert_eq!(p.pick(&a, 0), None, "nothing loaded yet");
        settle(&mut p);
        let up = p.take();
        assert_eq!(up.len(), 2);
        assert_eq!(up[0].1.size(), (MAX, MAX), "halved to MAX");
        assert_eq!(up[0].1.levels.last().unwrap().0, 1, "with its mips");
        assert!(p.take().is_empty(), "once");
        // neutral 0, thinking 1 (missing → neutral), happy 2 (→ neutral), laughing 3, surprised 4 (→ laughing)
        assert_eq!([0, 1, 2, 3, 4].map(|m| p.pick(&a, m)), [Some(a[0]), Some(a[0]), Some(a[0]), Some(a[3]), Some(a[3])]);
        assert_eq!(p.status("plf"), "plf 2/5 MOODS");
        let none = p.request("nobody", &moods[..1]);
        settle(&mut p);
        assert_eq!(p.pick(&none, 0), None, "a character without sprites: no face");
        assert!(p.status("nobody").starts_with("nobody NO FACE"));
        // #495: a card's view is a slot of its own, its aspect kept; the heads' count stays the portraits'
        let full = p.request_view("plf", crate::cast::CARD_VIEW, &moods[..2]);
        assert!(full.iter().all(|s| !a.contains(s)), "another view, other slots");
        settle(&mut p);
        assert_eq!(p.pick(&full, 1), Some(full[0]), "a mood without its full view shows the nearest one");
        assert!((p.slots[full[0]].aspect - 0.4).abs() < 1e-3, "{}", p.slots[full[0]].aspect);
        assert_eq!(p.status("plf"), "plf 2/5 MOODS");
    }

    #[test]
    fn a_missing_mood_shows_its_nearest() {
        let moods: Vec<String> = ["neutral", "happy", "dizzy", "confused", "tripping"].iter().map(|m| m.to_string()).collect();
        let n = near(&moods);
        assert_eq!(n[2], [2, 3, 4], "dizzy: itself, confused, tripping");
        assert_eq!(n[4], [4, 2, 3], "tripping: itself, dizzy, confused (laughing isn't in the palette)");
        assert_eq!(n[0], [0], "neutral's neighbours aren't there");
        assert!(NEAR.iter().all(|(m, r)| r.iter().all(|x| x != m && NEAR.iter().any(|(y, _)| y == x))), "every neighbour is a mood of the set");
    }

    #[test]
    fn a_local_folder_is_searched_per_character_then_flat() {
        let dir = std::env::temp_dir().join(format!("deck-portraits-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("catpope")).unwrap();
        let px = image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 2, 3, 255]));
        px.save(dir.join("catpope").join("catpope_portrait-happy.png")).unwrap();
        px.save(dir.join("plf_portrait-neutral.png")).unwrap();
        std::fs::create_dir_all(dir.join("items")).unwrap();
        px.save(dir.join("catpope").join("catpope_full-happy.png")).unwrap();
        px.save(dir.join("items").join("incense_burner.png")).unwrap();
        let src = LocalDir(dir.clone());
        let head = |c: &str, m: &str| Picture::character(c, PORTRAIT_VIEW, m);
        assert_eq!(src.find(&head("catpope", "happy")), Some(dir.join("catpope").join("catpope_portrait-happy.png")));
        assert_eq!(src.find(&head("plf", "neutral")), Some(dir.join("plf_portrait-neutral.png")));
        assert!(src.load(&head("catpope", "sad")).unwrap_err().contains("catpope_portrait-sad"));
        assert_eq!(src.load(&head("plf", "neutral")).unwrap().dimensions(), (4, 4));
        // #495: a card's full view and an item's picture
        assert_eq!(src.find(&Picture::character("catpope", crate::cast::CARD_VIEW, "happy")), Some(dir.join("catpope").join("catpope_full-happy.png")));
        assert_eq!(src.find(&Picture::item("incense_burner")), Some(dir.join("items").join("incense_burner.png")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "media")]
    #[test]
    fn a_video_slot_gets_the_frame_for_the_heard_time() {
        let clip = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../media/tests/fixtures/hap.mov")).unwrap();
        let mut p = Portraits::manual();
        let slot = p.picture(Picture::media("layers/sea.mov"));
        assert_eq!(p.requests().len(), 1, "the host fetches its file");
        p.deliver_video(slot, Ok(clip));
        p.poll();
        assert!(p.ready(slot) && (p.slots[slot].aspect - 64.0 / 48.0).abs() < 1e-6);
        p.play(0.0);
        p.play(0.01);
        assert_eq!(p.take_frames().len(), 1, "one frame for both times");
        p.play(0.5);
        let f = p.take_frames();
        assert_eq!((f.len(), f[0].0), (1, slot));
        assert!(p.take().is_empty(), "no still for a video slot");
        let bad = p.picture(Picture::media("layers/broken.webm"));
        p.deliver_video(bad, Ok(vec![0; 64]));
        p.poll();
        assert!(matches!(p.slots[bad].state, State::Missing(_)));
    }
}
