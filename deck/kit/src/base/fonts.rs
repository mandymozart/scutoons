//! The faces text is set in: a sans, regular and bold, for text, labels and titles, and the mono face for values and
//! codes. ABC Areal is licensed to the owner and never shipped (no repository, bundle or image may carry it): a machine
//! that has it keeps its files in the font folder, read at start. Without them the shipped open faces stand in: Noto
//! Sans (SIL OFL, `assets/NotoSans-OFL.txt`) and the deck's mono face.
use ab_glyph::{Font as _, FontRef};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

static SANS: &[u8] = include_bytes!("../../assets/NotoSans-Regular-latin.ttf");
static BOLD: &[u8] = include_bytes!("../../assets/NotoSans-Bold-latin.ttf");

/// The font folder's file names, without the extension (`.ttf` or `.otf`).
pub const AREAL: [&str; 2] = ["ABCAreal-Regular", "ABCAreal-Bold"];
/// the mono faces, the first found wins
pub const AREAL_MONO: [&str; 2] = ["ABCArealMono-Regular", "ABCArealMono-Bold"];

/// What a text is set in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Font {
    /// values and codes
    #[default]
    Mono,
    /// text and labels
    Sans,
    /// titles
    Bold,
}

/// The three faces as font files.
#[derive(Clone)]
pub struct Faces {
    pub mono: &'static [u8],
    pub sans: &'static [u8],
    pub bold: &'static [u8],
    /// which came from the font folder (the sans, the mono)
    pub from_folder: (bool, bool),
}

impl std::fmt::Debug for Faces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Faces").field("from_folder", &self.from_folder).finish()
    }
}

impl Faces {
    /// the shipped faces: the open sans and `mono`
    pub fn shipped(mono: &'static [u8]) -> Faces {
        Faces { mono, sans: SANS, bold: BOLD, from_folder: (false, false) }
    }

    /// The faces of a font folder: ABC Areal when both its weights are there and readable, ABC Areal Mono when one of
    /// its files is; what is missing is the shipped face.
    pub fn from_folder(dir: &Path, mono: &'static [u8]) -> Faces {
        let mut faces = Faces::shipped(mono);
        if let [Some(sans), Some(bold)] = AREAL.map(|name| read(dir, name)) {
            (faces.sans, faces.bold, faces.from_folder.0) = (sans, bold, true);
        }
        if let Some(mono) = AREAL_MONO.iter().find_map(|name| read(dir, name)) {
            (faces.mono, faces.from_folder.1) = (mono, true);
        }
        faces
    }

    /// the faces of this machine's font folder (`folder()`), or the shipped ones
    pub fn installed(mono: &'static [u8]) -> Faces {
        match folder() {
            Some(dir) => Faces::from_folder(&dir, mono),
            None => Faces::shipped(mono),
        }
    }

    pub fn file(&self, font: Font) -> &'static [u8] {
        match font {
            Font::Mono => self.mono,
            Font::Sans => self.sans,
            Font::Bold => self.bold,
        }
    }

    /// what lays text out in these faces
    pub fn widths(&self) -> Widths {
        Widths::new(self)
    }
}

/// The font folder: `%LOCALAPPDATA%\scutoons\fonts` on Windows, `/var/lib/deck/fonts` on a deck.
pub fn folder() -> Option<PathBuf> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    if cfg!(windows) {
        return std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("scutoons").join("fonts"));
    }
    Some(PathBuf::from("/var/lib/deck/fonts"))
}

/// a face's file from the folder, once read for the whole run; none when missing or no font
fn read(dir: &Path, name: &str) -> Option<&'static [u8]> {
    let bytes = ["ttf", "otf"].iter().find_map(|ext| std::fs::read(dir.join(format!("{name}.{ext}"))).ok())?;
    FontRef::try_from_slice(&bytes).ok()?;
    Some(Box::leak(bytes.into_boxed_slice()))
}

/// Each face's advance per pixel of font size, per character: what a layout measures a text with. A character a face
/// lacks is set in the mono face (egui falls back the same way).
#[derive(Debug, Clone, PartialEq)]
pub struct Widths {
    mono: f32,
    sans: HashMap<char, f32>,
    bold: HashMap<char, f32>,
}

impl Widths {
    fn new(faces: &Faces) -> Widths {
        let table = |file: &'static [u8]| -> HashMap<char, f32> {
            let Ok(f) = FontRef::try_from_slice(file) else { return HashMap::new() };
            let em = f.units_per_em().unwrap_or(1000.0);
            f.codepoint_ids().map(|(id, c)| (c, f.h_advance_unscaled(id) / em)).collect()
        };
        let mono = FontRef::try_from_slice(faces.mono).map_or(0.6, |f| f.h_advance_unscaled(f.glyph_id('M')) / f.units_per_em().unwrap_or(1000.0));
        Widths { mono, sans: table(faces.sans), bold: table(faces.bold) }
    }

    /// the mono face's advance per pixel
    pub fn mono(&self) -> f32 {
        self.mono
    }

    /// one character's advance per pixel in `font`
    pub fn of(&self, c: char, font: Font) -> f32 {
        let table = match font {
            Font::Mono => return self.mono,
            Font::Sans => &self.sans,
            Font::Bold => &self.bold,
        };
        table.get(&c).copied().unwrap_or(self.mono)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONO: &[u8] = include_bytes!("../../../outlet/assets/IBMPlexMono-Bold-latin.ttf");

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("deck-kit-fonts-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_folder_s_faces_are_picked_and_the_shipped_ones_stand_in() {
        // the shipped open faces play the folder's files: the loader goes by the names
        let dir = folder("pick");
        std::fs::write(dir.join("ABCAreal-Regular.ttf"), BOLD).unwrap();
        std::fs::write(dir.join("ABCAreal-Bold.otf"), SANS).unwrap();
        std::fs::write(dir.join("ABCArealMono-Bold.ttf"), SANS).unwrap();
        let f = Faces::from_folder(&dir, MONO);
        assert_eq!(f.from_folder, (true, true));
        assert_eq!((f.sans, f.bold, f.mono), (BOLD, SANS, SANS), "each by its name, .ttf or .otf");

        std::fs::write(dir.join("ABCArealMono-Regular.ttf"), BOLD).unwrap();
        assert_eq!(Faces::from_folder(&dir, MONO).mono, BOLD, "the regular mono before the bold");

        // one weight missing: the shipped sans, both weights; a file that is no font is missing
        std::fs::remove_file(dir.join("ABCAreal-Bold.otf")).unwrap();
        std::fs::write(dir.join("ABCAreal-Bold.ttf"), b"not a font").unwrap();
        let f = Faces::from_folder(&dir, MONO);
        assert_eq!((f.sans, f.bold, f.from_folder.0), (SANS, BOLD, false));

        let f = Faces::from_folder(&dir.join("absent"), MONO);
        assert_eq!((f.sans, f.bold, f.mono, f.from_folder), (SANS, BOLD, MONO, (false, false)), "no folder: the shipped faces");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_sans_is_proportional_and_a_missing_letter_takes_the_mono_width() {
        let w = Faces::shipped(MONO).widths();
        assert!((w.mono() - 0.6).abs() < 0.01);
        assert!(w.of('i', Font::Sans) < w.of('M', Font::Sans), "a proportional face");
        assert!(w.of('a', Font::Bold) > w.of('a', Font::Sans), "the bold runs wider");
        assert_eq!(w.of('\u{4e00}', Font::Sans), w.mono());
        assert_eq!(w.of('i', Font::Mono), w.mono());
    }
}
