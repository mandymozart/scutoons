//! The scene switcher (#250, F5): a list of the built-in scenes and every `*.toml` in the scenes folder, the arrows move,
//! Enter loads the scene in place (main.rs's `switch`), and the pick is written into deck.toml as `scene = "…"` so the
//! next boot plays it. Kept as basic as the settings page (quality.rs) – this file has no window, no GPU.
use std::path::{Path, PathBuf};

/// where the deck keeps its scenes (the image installs `deck/native/scenes` here)
pub const SCENES_DIR: &str = "/usr/share/deck/scenes";

pub use crate::scene::{builtin, BUILTIN};

/// one line of the list: what the HUD shows and what `--scene` / deck.toml's `scene` is
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub label: String,
    pub value: String,
}

/// the built-ins, then the `*.toml` files of `dir` by name (a file named like a built-in is skipped: the built-in wins)
pub fn entries(dir: &Path) -> Vec<Entry> {
    let mut out: Vec<Entry> = BUILTIN.iter().map(|n| Entry { label: n.to_string(), value: n.to_string() }).collect();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "toml")).collect())
        .unwrap_or_default();
    files.sort();
    for p in files {
        let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if !stem.is_empty() && !BUILTIN.contains(&stem.as_str()) {
            out.push(Entry { label: stem, value: p.to_string_lossy().into_owned() });
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Enter,
}

const VISIBLE: usize = 9;
const WIDTH: usize = 39;

/// The list: open or not, the cursor, the last result.
#[derive(Debug, Clone, Default)]
pub struct Switcher {
    pub open: bool,
    list: Vec<Entry>,
    cursor: usize,
    /// what the last pick did (loaded · the error)
    pub status: String,
}

impl Switcher {
    /// Opens the list on `dir`'s scenes with the cursor on the playing one (`current` = `--scene`'s value).
    pub fn open(&mut self, dir: &Path, current: Option<&str>) {
        self.list = entries(dir);
        let now = current.unwrap_or(BUILTIN[0]);
        self.cursor = self.list.iter().position(|e| e.value == now).unwrap_or(0);
        self.open = true;
    }

    /// One key; Enter returns the entry to load.
    pub fn key(&mut self, k: Key) -> Option<Entry> {
        match k {
            Key::Up => self.cursor = self.cursor.saturating_sub(1),
            Key::Down => self.cursor = (self.cursor + 1).min(self.list.len().saturating_sub(1)),
            Key::Enter => return self.list.get(self.cursor).cloned(),
        }
        None
    }

    /// The list as HUD lines (≤ 13 × 39), the playing scene starred; also the cursor's line.
    pub fn lines(&self, current: Option<&str>) -> (Vec<String>, usize) {
        let now = current.unwrap_or(BUILTIN[0]);
        let mut out = vec!["SCENES  F5 CLOSE  ENTER=LOAD".to_string()];
        let start = (self.cursor + 1).saturating_sub(VISIBLE);
        let mut cursor = 1;
        for (i, e) in self.list.iter().enumerate().skip(start).take(VISIBLE) {
            if i == self.cursor {
                cursor = out.len();
            }
            let mark = if e.value == now { '*' } else { ' ' };
            let line = format!("{}{mark}{}", if i == self.cursor { '>' } else { ' ' }, e.label);
            out.push(line.to_uppercase().chars().take(WIDTH).collect());
        }
        let status: Vec<char> = self.status.to_uppercase().chars().collect();
        for part in status.chunks(WIDTH).take(3) {
            out.push(part.iter().collect());
        }
        (out, cursor)
    }
}

/// Writes `scene = "value"` into the deck.toml text: the first `scene` line is replaced, further ones go (deck-session
/// reads the first; deck-set does the same), else the line is added.
pub fn with_scene(text: &str, value: &str) -> String {
    crate::live::config::with_key(text, "scene", value)
}

/// #473: the piece side's file under the cache (live/piece.rs): a show's piece plays from it for a while – it is no
/// scene to boot into
pub const PIECE_FILE: [&str; 2] = ["live", "piece.toml"];

/// Remembers the pick in the deck's file, so the next boot plays it. A show's piece (`PIECE_FILE`) is not remembered:
/// the deck boots into its own scene, the piece comes again with the show (found in #481's end-to-end test).
pub fn remember(file: &Path, value: &str) -> std::io::Result<()> {
    if Path::new(value).ends_with(PIECE_FILE.iter().collect::<PathBuf>()) {
        return Ok(());
    }
    if value.contains('"') {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "a double quote in the scene"));
    }
    let text = std::fs::read_to_string(file).unwrap_or_default();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(file, with_scene(&text, value))
}

/// the deck's file (`DECK_TOML` names another – a dev machine's)
pub fn deck_toml() -> PathBuf {
    std::env::var_os("DECK_TOML").map(PathBuf::from).unwrap_or_else(|| crate::live::config::DECK_TOML.into())
}

// the change through black itself is the core's (#473: every deck fades the same way)
pub use deck_space::veil::{Change, Step, Transition, FADE_IN, FADE_OUT};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_scene_builds() {
        for name in BUILTIN {
            let src = builtin(name).unwrap_or_else(|| panic!("{name}: no source"));
            let s = crate::scene::Scene::parse(src).unwrap_or_else(|e| panic!("{name}: {e:#}"));
            // a scene needing a module this build lacks is refused with the feature's name – everything else must build
            match crate::modules::check(&s) {
                Ok(()) if s.nodes.iter().any(|n| n.kind.starts_with("module:")) => {} // its wasm: tests/modules.rs
                Ok(()) => {
                    crate::graph::Graph::build(&s).unwrap_or_else(|e| panic!("{name}: {e:#}"));
                }
                Err(e) => assert!(format!("{e:#}").contains("build"), "{name}: {e:#}"),
            }
        }
    }

    #[test]
    fn the_list_has_the_built_ins_then_the_folder() {
        let dir = std::env::temp_dir().join(format!("deck-switcher-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["zeta.toml", "alpha.toml", "idle.toml", "notes.txt"] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        let l = entries(&dir);
        let labels: Vec<&str> = l.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(&labels[..BUILTIN.len()], BUILTIN);
        assert_eq!(&labels[BUILTIN.len()..], ["alpha", "zeta"]);
        assert!(l[BUILTIN.len()].value.ends_with("alpha.toml"));
        // no folder: the built-ins alone
        assert_eq!(entries(&dir.join("none")).len(), BUILTIN.len());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_cursor_moves_and_enter_picks() {
        let mut s = Switcher::default();
        s.open(Path::new("/none"), Some("cathedral"));
        assert_eq!(s.key(Key::Enter).unwrap().value, "cathedral");
        (0..4).for_each(|_| drop(s.key(Key::Up)));
        assert_eq!(s.key(Key::Enter).unwrap().value, "idle");
        for _ in 0..20 {
            s.key(Key::Down);
        }
        assert_eq!(s.key(Key::Enter).unwrap().value, "module-analog");
        let (lines, cursor) = s.lines(Some("cathedral"));
        assert!(lines[cursor].starts_with('>') && lines[cursor].contains("MODULE-ANALOG"));
        assert!(s.lines(Some("module-analog")).0.iter().any(|l| l.contains("*MODULE-ANALOG"))); // the playing scene is starred (cathedral has scrolled out of the 9 visible lines)
        assert!(lines.len() <= 13 && lines.iter().all(|l| l.chars().count() <= WIDTH));
    }

    #[test]
    fn the_default_is_the_playing_one_without_a_scene() {
        let mut s = Switcher::default();
        s.open(Path::new("/none"), None);
        assert_eq!(s.key(Key::Enter).unwrap().value, "idle");
    }

    #[test]
    fn a_shows_piece_is_not_remembered_the_operators_pick_is() {
        let file = std::env::temp_dir().join(format!("deck-remember-{}.toml", std::process::id()));
        std::fs::write(&file, "audio_host = \"asio\"\n").unwrap();
        let piece = std::env::temp_dir().join("cache").join(PIECE_FILE[0]).join(PIECE_FILE[1]);
        remember(&file, &piece.to_string_lossy()).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "audio_host = \"asio\"\n", "the piece's file is no boot scene");
        remember(&file, "madness").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "audio_host = \"asio\"\nscene = \"madness\"\n");
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn one_scene_line_is_replaced_never_doubled() {
        let t = "kiosk = \"stage\"\nscene = \"cathedral\"\nrelay = \"ws://x\"\nscene = \"old\"\n";
        assert_eq!(with_scene(t, "madness"), "kiosk = \"stage\"\nscene = \"madness\"\nrelay = \"ws://x\"\n");
        assert_eq!(with_scene("", "idle"), "scene = \"idle\"\n");
        assert_eq!(with_scene("relay = \"a\"\n", "idle"), "relay = \"a\"\nscene = \"idle\"\n");
        // a key that only starts with scene stays
        assert_eq!(with_scene("scenery = 1\n", "idle"), "scenery = 1\nscene = \"idle\"\n");
        // a Windows path stays a TOML string the deck reads back
        let path = r"C:\Users\deck\cache\live\piece.toml";
        let text = with_scene("audio_host = \"asio\"\n", path);
        assert_eq!(text.parse::<toml::Table>().expect("valid TOML")["scene"].as_str(), Some(path));
    }

    #[test]
    fn remembering_writes_the_file() {
        let dir = std::env::temp_dir().join(format!("deck-remember-{}", std::process::id()));
        let f = dir.join("deck.toml");
        remember(&f, "madness").unwrap();
        remember(&f, "/usr/share/deck/scenes/x.toml").unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "scene = \"/usr/share/deck/scenes/x.toml\"\n");
        assert!(remember(&f, "a\"b").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
