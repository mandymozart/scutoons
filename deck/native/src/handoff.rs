//! F9: the deck and the editor hand over to each other (#609) – two apps, one switch key on every platform.
//!
//! The leaving app writes what the other needs into a hand-off file (the session's relay, the show, the scene, and how
//! each app was started) and then either
//! - **swaps under deck-session** (the Deck OS, `DECK_SWAP` set): the file goes to `DECK_HANDOFF`, the app exits with
//!   [`EXIT_SWAP`] and the session starts the other with `--handoff "$DECK_HANDOFF"`, or
//! - **starts the sibling itself** (Windows, macOS, a Linux desktop): `deck-editor` next to `deck` (the same folder),
//!   with its own last arguments and `--handoff FILE`, then quits.
//!
//! Both ways the receiving app reads the same file the same way ([`Handoff::take`]), so the result is the same. No
//! sibling (a lean deck carries no editor) or deck.toml's `editor = "locked"` keep the app where it is.
use deck_ui::handoff::Refusal;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// the exit code that asks deck-session for the other app (F9 under `DECK_SWAP`)
pub const EXIT_SWAP: i32 = 11;

/// the two apps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum App {
    Deck,
    Editor,
}

impl App {
    /// the binary's name, without the platform's suffix
    pub fn bin(self) -> &'static str {
        match self {
            App::Deck => "deck",
            App::Editor => "deck-editor",
        }
    }

    pub fn other(self) -> App {
        match self {
            App::Deck => App::Editor,
            App::Editor => App::Deck,
        }
    }
}

/// What travels from one app to the other. Unset = the receiver keeps its own (its flags, deck.toml).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Handoff {
    /// the Live session: the relay with its code
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<String>,
    /// the show the editor has open (a file or `show:<id>`; the deck carries it through)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    /// the scene the stage plays (the editor carries it through)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    /// how each app was last started (its arguments without the hand-off's own): the sibling starts the other the same way
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deck_args: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub editor_args: Vec<String>,
}

impl Handoff {
    /// Reads the file a `--handoff` names and removes it (a hand-off is used once); a broken one is logged and empty.
    pub fn take(path: &Path) -> Handoff {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("hand-off {}: {e}", path.display());
                return Handoff::default();
            }
        };
        let _ = std::fs::remove_file(path);
        let h = toml::from_str(&text).unwrap_or_else(|e| {
            tracing::warn!("hand-off {}: {e}", path.display());
            Handoff::default()
        });
        tracing::info!("hand-off from {}: {h:?}", path.display());
        h
    }

    pub fn args_of(&self, app: App) -> &[String] {
        match app {
            App::Deck => &self.deck_args,
            App::Editor => &self.editor_args,
        }
    }

    pub fn set_args(&mut self, app: App, args: Vec<String>) {
        match app {
            App::Deck => self.deck_args = args,
            App::Editor => self.editor_args = args,
        }
    }
}

/// An app's own arguments without the hand-off's (`--handoff FILE`, a check run's `--f9 N`): what starts it again.
pub fn own_args(args: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "--handoff" || a == "--f9" {
            it.next();
        } else {
            out.push(a);
        }
    }
    out
}

/// The other app's binary next to this one's (the same folder), if it is installed there.
pub fn sibling(app: App) -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    sibling_in(&dir, app)
}

fn sibling_in(dir: &Path, app: App) -> Option<PathBuf> {
    let p = dir.join(format!("{}{}", app.bin(), std::env::consts::EXE_SUFFIX));
    p.is_file().then_some(p)
}

/// deck.toml's `editor`: "allowed" (the default) or "locked" – F9 never drops this deck into the editor. Another value
/// locks too (a typo must not open the editor in a show) and is said in the log.
pub fn editor_locked(deck_toml: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(deck_toml) else { return false };
    locked_in(&text)
}

fn locked_in(text: &str) -> bool {
    #[derive(Deserialize)]
    struct Keys {
        editor: Option<String>,
    }
    match toml::from_str::<Keys>(text).map(|k| k.editor) {
        Ok(None) => false,
        Ok(Some(v)) if v == "allowed" => false,
        Ok(Some(v)) if v == "locked" => true,
        Ok(Some(v)) => {
            tracing::warn!("deck.toml: editor = \"{v}\" – allowed or locked; locked");
            true
        }
        // a broken deck.toml is said where the deck reads its other keys; the lock reads it as unset
        Err(_) => false,
    }
}

/// F9 in `from`: hands `h` over to the other app – deck-session's swap when `DECK_SWAP` is set, else the sibling started
/// here. `own` = this app's own arguments ([`own_args`]); `deck_toml` = the deck's settings (the lock). Ok = the exit
/// code to quit with now (deck-session's [`EXIT_SWAP`], or 0 once the sibling runs).
pub fn go(from: App, mut h: Handoff, own: Vec<String>, deck_toml: &Path) -> Result<i32, Refusal> {
    let to = from.other();
    if from == App::Deck && editor_locked(deck_toml) {
        return Err(Refusal::Locked);
    }
    let Some(bin) = sibling(to) else {
        return Err(match to {
            App::Editor => Refusal::NoEditor,
            App::Deck => Refusal::NoDeck,
        });
    };
    h.set_args(from, own);
    if std::env::var_os("DECK_SWAP").is_some() {
        if let Some(file) = std::env::var_os("DECK_HANDOFF").map(PathBuf::from) {
            write(&file, &h)?;
        }
        tracing::info!("F9: {} → {} (deck-session)", from.bin(), to.bin());
        return Ok(EXIT_SWAP);
    }
    let file = std::env::temp_dir().join(format!("deck-handoff-{}.toml", std::process::id()));
    write(&file, &h)?;
    let args: Vec<String> = h.args_of(to).iter().cloned().chain(["--handoff".into(), file.display().to_string()]).collect();
    tracing::info!("F9: {} → {} {}", from.bin(), bin.display(), args.join(" "));
    match std::process::Command::new(&bin).args(&args).spawn() {
        Ok(_) => Ok(0),
        Err(e) => {
            let _ = std::fs::remove_file(&file);
            Err(Refusal::Failed(format!("{}: {e}", bin.display())))
        }
    }
}

fn write(file: &Path, h: &Handoff) -> Result<(), Refusal> {
    let text = toml::to_string(h).map_err(|e| Refusal::Failed(e.to_string()))?;
    std::fs::write(file, text).map_err(|e| Refusal::Failed(format!("{}: {e}", file.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_args_leave_the_hand_off_out() {
        let a = own_args(["--windowed", "--handoff", "x.toml", "--f9", "30", "--scene", "idle"].map(String::from));
        assert_eq!(a, ["--windowed", "--scene", "idle"]);
    }

    #[test]
    fn the_lock() {
        assert!(!locked_in(""));
        assert!(!locked_in("relay = \"ws://x\"\neditor = \"allowed\"\n"));
        assert!(locked_in("editor = \"locked\"\n"));
        assert!(locked_in("editor = \"lockd\"\n"), "a typo locks");
    }

    #[test]
    fn a_hand_off_round_trips_through_its_file() {
        let dir = std::env::temp_dir().join(format!("handoff-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("h.toml");
        let h = Handoff { relay: Some("ws://127.0.0.1:7070/live/AB12".into()), show: Some("show:7".into()), scene: Some("idle".into()), deck_args: vec!["--windowed".into()], editor_args: vec![] };
        write(&f, &h).unwrap();
        assert_eq!(Handoff::take(&f), h);
        assert!(!f.exists(), "used once");
        assert_eq!(Handoff::take(&f), Handoff::default());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_sibling_is_the_binary_in_the_same_folder() {
        let dir = std::env::temp_dir().join(format!("handoff-sib-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(sibling_in(&dir, App::Editor), None);
        let p = dir.join(format!("deck-editor{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&p, b"").unwrap();
        assert_eq!(sibling_in(&dir, App::Editor), Some(p));
        let _ = std::fs::remove_dir_all(dir);
    }
}
