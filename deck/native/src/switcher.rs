//! The scene switcher (#250, F5): the system scenes and, in dev mode, the sandboxes (#427: the files of the sandbox
//! folder, apart under their own heading); the arrows move, Enter loads the scene in place (main.rs's `switch`) and
//! nothing else, S makes the one under the cursor the start scene (deck.toml's `scene = "…"`, the next boot plays it;
//! unset, the deck starts with `idle`). Kept as basic as the settings page (quality.rs) – this file has no window, no GPU.
use std::path::{Path, PathBuf};

/// where the dev image keeps the sandboxes (#427: only the dev image and the dev update channel install them)
pub const SANDBOX_DIR: &str = "/usr/share/deck/scenes/sandbox";

pub use crate::scene::{builtin, BUILTIN};

/// The sandbox folder in dev mode (#427), none outside it: `--scenes DIR`, else the dev image's (`SANDBOX_DIR`, there
/// only on a dev image), else with `--dev` the repo's deck/native/scenes/sandbox (a developer's PC).
pub fn sandboxes(given: Option<PathBuf>, dev: bool) -> Option<PathBuf> {
    let image = Path::new(SANDBOX_DIR);
    given.or_else(|| image.is_dir().then(|| image.into())).or_else(|| dev.then(|| concat!(env!("CARGO_MANIFEST_DIR"), "/scenes/sandbox").into()))
}

/// The scene a `--scene` value (or deck.toml's `scene`) names, a rig's aside (rig.rs): an existing file; a system scene's
/// name; a sandbox's name in dev mode (its file in `sandboxes`), refused outside it; else the file (an error that says so).
pub fn read(value: &Path, sandboxes: Option<&Path>) -> anyhow::Result<crate::scene::Scene> {
    use deck_space::scene::{named, sandbox_refused, Named, Scene};
    let name = value.to_str().filter(|_| !value.exists()).unwrap_or_default();
    match named(name) {
        Named::System(text) => Scene::parse(text),
        Named::Sandbox => match sandboxes.map(|d| d.join(format!("{name}.toml"))).filter(|f| f.is_file()) {
            Some(file) => Scene::load(&file),
            None => Err(sandbox_refused(name)),
        },
        Named::Other => Scene::load(value),
    }
}

/// one line of the list: what the HUD shows and what `--scene` / deck.toml's `scene` is
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub label: String,
    pub value: String,
    /// a sandbox (#427), listed apart after the system scenes
    pub sandbox: bool,
}

/// the system scenes, then in dev mode the `*.toml` files of the sandbox folder by name (a file named like a system scene
/// is skipped: the system scene wins)
pub fn entries(sandboxes: Option<&Path>) -> Vec<Entry> {
    let mut out: Vec<Entry> = BUILTIN.iter().map(|n| Entry { label: n.to_string(), value: n.to_string(), sandbox: false }).collect();
    let mut files: Vec<PathBuf> = sandboxes
        .and_then(|dir| std::fs::read_dir(dir).ok())
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "toml")).collect())
        .unwrap_or_default();
    files.sort();
    for p in files {
        let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if !stem.is_empty() && !BUILTIN.contains(&stem.as_str()) {
            out.push(Entry { label: stem, value: p.to_string_lossy().into_owned(), sandbox: true });
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Enter,
    /// #427: "set as start" – the cursor's scene into deck.toml
    Start,
}

const VISIBLE: usize = 9;
const WIDTH: usize = 39;

/// The list: open or not, the cursor, the last result.
#[derive(Debug, Clone, Default)]
pub struct Switcher {
    pub open: bool,
    list: Vec<Entry>,
    cursor: usize,
    /// what the last pick did (loaded · the start scene · the error)
    pub status: String,
}

impl Switcher {
    /// Opens the list (the sandboxes only in dev mode: `sandboxes`) with the cursor on the playing one (`current` =
    /// `--scene`'s value).
    pub fn open(&mut self, sandboxes: Option<&Path>, current: Option<&str>) {
        self.list = entries(sandboxes);
        let now = current.unwrap_or(BUILTIN[0]);
        self.cursor = self.list.iter().position(|e| e.value == now).unwrap_or(0);
        self.open = true;
    }

    /// One key; Enter returns the entry to load (loading only – S sets the start scene, into the deck's file).
    pub fn key(&mut self, k: Key) -> Option<Entry> {
        match k {
            Key::Up => self.cursor = self.cursor.saturating_sub(1),
            Key::Down => self.cursor = (self.cursor + 1).min(self.list.len().saturating_sub(1)),
            Key::Enter => return self.list.get(self.cursor).cloned(),
            Key::Start => self.set_start(&deck_toml()),
        }
        None
    }

    /// #427: "set as start", apart from loading – the cursor's scene into `file`'s `scene` (the next boot plays it)
    pub fn set_start(&mut self, file: &Path) {
        let Some(e) = self.list.get(self.cursor) else { return };
        self.status = match remember(file, &e.value) {
            Ok(()) => format!("START SCENE: {}", e.label),
            Err(err) => format!("START NOT SAVED: {err}"),
        };
    }

    /// The list as HUD lines (≤ 13 × 39), the playing scene starred, the sandboxes under their heading; also the cursor's
    /// line.
    pub fn lines(&self, current: Option<&str>) -> (Vec<String>, usize) {
        let now = current.unwrap_or(BUILTIN[0]);
        let mut out = vec!["SCENES  ENTER=LOAD  S=START  F5=CLOSE".to_string()];
        // the rows: an entry's index, or the sandboxes' heading (None) before the first of them
        let rows: Vec<Option<usize>> = (0..self.list.len())
            .flat_map(|i| {
                let heading = self.list[i].sandbox && (i == 0 || !self.list[i - 1].sandbox);
                heading.then_some(None).into_iter().chain([Some(i)])
            })
            .collect();
        let at = rows.iter().position(|r| *r == Some(self.cursor)).unwrap_or(0);
        let start = (at + 1).saturating_sub(VISIBLE);
        let mut cursor = 1;
        for row in rows.iter().skip(start).take(VISIBLE) {
            let Some(i) = *row else {
                out.push(" -- SANDBOX (DEV) --".to_string());
                continue;
            };
            let e = &self.list[i];
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

/// Remembers the start scene in the deck's file (F5's S, #427), so the next boot plays it. A show's piece (`PIECE_FILE`)
/// is not remembered: the deck boots into its own scene, the piece comes again with the show (#481's end-to-end test).
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

    /// the system scenes and every sandbox build (a sandbox's text from deck-space: the tests' build carries them)
    #[test]
    fn every_system_scene_and_sandbox_builds() {
        let sandboxes = deck_space::scene::SANDBOX.map(|n| (n, deck_space::scene::sandbox(n).unwrap_or_else(|| panic!("{n}: no source"))));
        for (name, src) in BUILTIN.map(|n| (n, builtin(n).unwrap_or_else(|| panic!("{n}: no source")))).into_iter().chain(sandboxes) {
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
    fn the_default_is_the_playing_one_without_a_scene() {
        let mut s = Switcher::default();
        s.open(None, None);
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
