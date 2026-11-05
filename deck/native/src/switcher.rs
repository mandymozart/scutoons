//! The scene switcher (#250, F5): a list of the built-in scenes and every `*.toml` in the scenes folder, the arrows move,
//! Enter loads the scene in place (main.rs's `switch`), and the pick is written into deck.toml as `scene = "…"` so the
//! next boot plays it. Kept as basic as the settings page (quality.rs) – this file has no window, no GPU.
use crate::scene;
use std::path::{Path, PathBuf};

/// where the deck keeps its scenes (the image installs `deck/native/scenes` here)
pub const SCENES_DIR: &str = "/usr/share/deck/scenes";

/// the built-in scenes in the list's order (the first is the stage's default)
pub const BUILTIN: [&str; 10] = ["idle", "madness", "cathedral", "catpope-analog", "conductor", "outlet", "strobe", "pulse-thunderwash", "lamps-off", "test-pattern"];

/// the source of a built-in scene by name
pub fn builtin(name: &str) -> Option<&'static str> {
    Some(match name {
        "idle" => scene::IDLE_SCENE,
        "madness" => scene::MADNESS_SCENE,
        "cathedral" => scene::CATHEDRAL_SCENE,
        "catpope-analog" => include_str!("../scenes/catpope-analog.toml"),
        "conductor" => scene::CONDUCTOR_SCENE,
        "outlet" => scene::OUTLET_SCENE,
        "strobe" => include_str!("../scenes/strobe.toml"),
        "pulse-thunderwash" => include_str!("../scenes/pulse-thunderwash.toml"),
        "lamps-off" => include_str!("../scenes/lamps-off.toml"),
        "test-pattern" => scene::TEST_PATTERN_SCENE,
        _ => return None,
    })
}

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
    let line = format!("scene = \"{value}\"");
    let mut out = Vec::new();
    let mut done = false;
    for l in text.lines() {
        let key = l.split('=').next().map(str::trim).filter(|_| l.contains('='));
        if key == Some("scene") {
            if !done {
                out.push(line.clone());
                done = true;
            }
        } else {
            out.push(l.to_string());
        }
    }
    if !done {
        out.push(line);
    }
    out.join("\n") + "\n"
}

/// Remembers the pick in the deck's file, so the next boot plays it.
pub fn remember(file: &Path, value: &str) -> std::io::Result<()> {
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

/// seconds the picture takes to fade to black and back (the audio fades out as long, backend.rs)
pub const FADE_OUT: f32 = 0.25;
pub const FADE_IN: f32 = 0.4;

/// What the transition asks of the stage on a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    None,
    /// black now: build the new show (once)
    Load,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Out,
    Hold,
    In,
}

/// A scene change through black (#250): the picture fades out, the stage loads the new show behind the black, it fades
/// in once the show has what it waits for. `veil` is the black's opacity.
#[derive(Debug, Clone)]
pub struct Transition {
    pub to: String,
    pub veil: f32,
    phase: Phase,
}

impl Transition {
    pub fn new(to: &str) -> Transition {
        Transition { to: to.to_owned(), veil: 0.0, phase: Phase::Out }
    }

    /// One frame of `dt` seconds; `ready` = the loaded show waits for nothing. Returns true when the veil is gone.
    pub fn step(&mut self, dt: f32, ready: bool) -> (Step, bool) {
        match self.phase {
            Phase::Out => {
                self.veil = (self.veil + dt / FADE_OUT).min(1.0);
                if self.veil >= 1.0 {
                    self.phase = Phase::Hold;
                    return (Step::Load, false);
                }
            }
            Phase::Hold => {
                if ready {
                    self.phase = Phase::In;
                }
            }
            Phase::In => {
                self.veil = (self.veil - dt / FADE_IN).max(0.0);
                return (Step::None, self.veil <= 0.0);
            }
        }
        (Step::None, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_scene_builds() {
        for name in BUILTIN {
            let src = builtin(name).unwrap_or_else(|| panic!("{name}: no source"));
            let s = scene::Scene::parse(src).unwrap_or_else(|e| panic!("{name}: {e:#}"));
            // a scene of wasm modules builds only with its modules loaded (DECK_MODULES): tests/modules.rs builds it with them
            if s.nodes.iter().any(|n| n.kind.starts_with("module:")) {
                continue;
            }
            // a scene needing a module this build lacks is refused with the feature's name – everything else must build
            match crate::modules::check(&s) {
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
        s.key(Key::Up);
        s.key(Key::Up);
        s.key(Key::Up);
        s.key(Key::Up);
        assert_eq!(s.key(Key::Enter).unwrap().value, "idle");
        for _ in 0..20 {
            s.key(Key::Down);
        }
        assert_eq!(s.key(Key::Enter).unwrap().value, "test-pattern");
        let (lines, cursor) = s.lines(Some("cathedral"));
        assert!(lines[cursor].starts_with('>') && lines[cursor].contains("TEST-PATTERN"));
        assert!(lines.iter().any(|l| l.contains("*CATHEDRAL")));
        assert!(lines.len() <= 13 && lines.iter().all(|l| l.chars().count() <= WIDTH));
    }

    #[test]
    fn a_transition_goes_black_loads_once_then_clears() {
        let mut t = Transition::new("madness");
        let mut loads = 0;
        let mut frames = 0;
        let mut peak = 0.0f32;
        loop {
            // not ready for 10 frames after the load
            let (step, done) = t.step(1.0 / 60.0, loads > 0 && frames > 40);
            loads += (step == Step::Load) as u32;
            peak = peak.max(t.veil);
            frames += 1;
            assert!(frames < 600, "never ends");
            if done {
                break;
            }
        }
        assert_eq!((loads, peak), (1, 1.0));
        assert_eq!(t.veil, 0.0);
    }

    #[test]
    fn the_default_is_the_playing_one_without_a_scene() {
        let mut s = Switcher::default();
        s.open(Path::new("/none"), None);
        assert_eq!(s.key(Key::Enter).unwrap().value, "idle");
    }

    #[test]
    fn one_scene_line_is_replaced_never_doubled() {
        let t = "kiosk = \"stage\"\nscene = \"cathedral\"\nrelay = \"ws://x\"\nscene = \"old\"\n";
        assert_eq!(with_scene(t, "madness"), "kiosk = \"stage\"\nscene = \"madness\"\nrelay = \"ws://x\"\n");
        assert_eq!(with_scene("", "idle"), "scene = \"idle\"\n");
        assert_eq!(with_scene("relay = \"a\"\n", "idle"), "relay = \"a\"\nscene = \"idle\"\n");
        // a key that only starts with scene stays
        assert_eq!(with_scene("scenery = 1\n", "idle"), "scenery = 1\nscene = \"idle\"\n");
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
