//! The beings' heads' sprites (#188 phase 5): a character's portrait mood sprites, `<char>_portrait-<mood>.png` /
//! `.webp` (the asset naming, agents/ASSETS.md), loaded on a thread of their own, halved to `MAX` and mipmapped there –
//! the frame that uploads one only copies. A head asks for its character's moods once (`request`); a mood whose sprite
//! is missing shows the nearest one that is there (toward the calm end), a character without any shows a faceless head.
//!
//! Where the sprites come from is a `Source`. Today `LocalDir`: `--portraits DIR` (`DECK_PORTRAITS`, default
//! `<cache>/portraits`), either `DIR/<char>/<char>_portrait-<mood>.<ext>` (the layout of media/characters/) or flat in
//! `DIR`. **#180:** the deck-token fetch from the CDN (cached on DECKDATA) becomes a second `Source` that downloads into
//! that same folder and then reads it – the heads don't change.
//!
//! Without threads (the browser, #198) the Portraits are `manual`: the host takes the requests (`requests`), fetches
//! and decodes the sprites itself and hands them in (`deliver`).
use super::glb::{half, Image};
#[cfg(feature = "native")]
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

/// a sprite's longest side on the GPU (a head is at most a third of the screen)
pub const MAX: u32 = 512;
#[cfg(feature = "native")]
const EXTENSIONS: [&str; 2] = ["png", "webp"];

/// where a character's portrait sprites come from
#[cfg(feature = "native")]
pub trait Source: Send + 'static {
    /// the sprite of `character` in `mood` – its file, decoded (RGBA8); Err = why there is none
    fn load(&self, character: &str, mood: &str) -> Result<image::RgbaImage, String>;
}

/// a local folder: `<dir>/<char>/<char>_portrait-<mood>.<png|webp>`, else `<dir>/<char>_portrait-<mood>.<…>`
#[cfg(feature = "native")]
pub struct LocalDir(pub PathBuf);

#[cfg(feature = "native")]
impl LocalDir {
    /// the first file that exists for this character and mood
    pub fn find(&self, character: &str, mood: &str) -> Option<PathBuf> {
        let name = format!("{character}_portrait-{mood}");
        [self.0.join(character), self.0.clone()]
            .into_iter()
            .flat_map(|d| EXTENSIONS.map(|e| d.join(format!("{name}.{e}"))))
            .find(|p| p.is_file())
    }
}

#[cfg(feature = "native")]
impl Source for LocalDir {
    fn load(&self, character: &str, mood: &str) -> Result<image::RgbaImage, String> {
        let Some(path) = self.find(character, mood) else {
            return Err(format!("no {character}_portrait-{mood} in {}", self.0.display()));
        };
        image::open(&path).map(|i| i.to_rgba8()).map_err(|e| format!("{}: {e}", path.display()))
    }
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
    pub character: String,
    pub mood: String,
    pub state: State,
}

/// The sprites every head asked for, by slot (the renderer keeps a texture per ready slot).
pub struct Portraits {
    pub slots: Vec<Slot>,
    /// the loader thread's inbox; None = manual (the host asks: `requests`)
    ask: Option<Sender<(usize, String, String)>>,
    asked: Vec<(usize, String, String)>,
    got: Receiver<(usize, Result<Image, String>)>,
    tell: Sender<(usize, Result<Image, String>)>,
    /// decoded, not yet on the GPU
    fresh: Vec<(usize, Image)>,
}

impl Portraits {
    /// the loader thread over `source` (it ends with the Portraits)
    #[cfg(feature = "native")]
    pub fn new(source: Box<dyn Source>) -> Portraits {
        let (ask, asked) = mpsc::channel::<(usize, String, String)>();
        let (tell, got) = mpsc::channel();
        let teller = tell.clone();
        std::thread::Builder::new()
            .name("portraits".into())
            .spawn(move || {
                while let Ok((slot, character, mood)) = asked.recv() {
                    let r = source.load(&character, &mood).map(prepare);
                    if let Err(e) = &r {
                        tracing::debug!("portrait: {e}");
                    }
                    if teller.send((slot, r)).is_err() {
                        break;
                    }
                }
            })
            .expect("a thread");
        Portraits { slots: Vec::new(), ask: Some(ask), asked: Vec::new(), got, tell, fresh: Vec::new() }
    }

    /// from a folder (`--portraits`)
    #[cfg(feature = "native")]
    pub fn local(dir: &Path) -> Portraits {
        Portraits::new(Box::new(LocalDir(dir.to_path_buf())))
    }

    /// no loader: the host takes the requests and delivers the sprites (the browser)
    pub fn manual() -> Portraits {
        let (tell, got) = mpsc::channel();
        Portraits { slots: Vec::new(), ask: None, asked: Vec::new(), got, tell, fresh: Vec::new() }
    }

    /// the sprites asked for since the last call (slot, character, mood) – manual Portraits only
    pub fn requests(&mut self) -> Vec<(usize, String, String)> {
        std::mem::take(&mut self.asked)
    }

    /// a requested sprite, decoded by the host (RGBA8), or why there is none – it shows from the next `poll`
    pub fn deliver(&mut self, slot: usize, sprite: Result<image::RgbaImage, String>) {
        let _ = self.tell.send((slot, sprite.map(prepare)));
    }

    /// A slot per mood of `character` (the same character and mood share one).
    pub fn request(&mut self, character: &str, moods: &[String]) -> Vec<usize> {
        if character.is_empty() {
            return Vec::new();
        }
        moods
            .iter()
            .map(|mood| match self.slots.iter().position(|s| s.character == character && s.mood == *mood) {
                Some(i) => i,
                None => {
                    let i = self.slots.len();
                    self.slots.push(Slot { character: character.into(), mood: mood.clone(), state: State::Loading });
                    match &self.ask {
                        Some(ask) => {
                            let _ = ask.send((i, character.into(), mood.clone()));
                        }
                        None => self.asked.push((i, character.into(), mood.clone())),
                    }
                    i
                }
            })
            .collect()
    }

    /// what the loader has finished (each frame; nothing new = no work)
    pub fn poll(&mut self) {
        while let Ok((slot, r)) = self.got.try_recv() {
            self.slots[slot].state = match r {
                Ok(img) => {
                    self.fresh.push((slot, img));
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
        let mine: Vec<&Slot> = self.slots.iter().filter(|s| s.character == character).collect();
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

fn prepare(img: image::RgbaImage) -> Image {
    let mut img = img;
    while img.width().max(img.height()) > MAX {
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
        fn load(&self, character: &str, mood: &str) -> Result<image::RgbaImage, String> {
            match (character, mood) {
                ("plf", "neutral" | "laughing") => Ok(image::RgbaImage::from_pixel(1024, 1024, image::Rgba([200, 100, 50, 255]))),
                _ => Err(format!("no {character} {mood}")),
            }
        }
    }

    fn settle(p: &mut Portraits) {
        let until = Instant::now() + Duration::from_secs(5);
        while p.slots.iter().any(|s| s.state == State::Loading) && Instant::now() < until {
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
        let src = LocalDir(dir.clone());
        assert_eq!(src.find("catpope", "happy"), Some(dir.join("catpope").join("catpope_portrait-happy.png")));
        assert_eq!(src.find("plf", "neutral"), Some(dir.join("plf_portrait-neutral.png")));
        assert!(src.load("catpope", "sad").unwrap_err().contains("catpope_portrait-sad"));
        assert_eq!(src.load("plf", "neutral").unwrap().dimensions(), (4, 4));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
