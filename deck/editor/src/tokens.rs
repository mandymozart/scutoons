//! The design tokens of the look (day or night) at the UI size (the base size, 1rem), both settings (settings.rs);
//! `tokens.toml` next to the binary (or `--tokens FILE`) overrides tokens, watched by its modification time and applied
//! in the next frame – without a restart.
pub use deck_kit::base::tokens::{Theme, Tokens, BASE, BASE_RANGE};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// the setting: which look, how large
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Look {
    pub theme: Theme,
    pub base: f32,
    /// ok, warning and error in the colour-blind set (the kit's `[day|night.colorblind]`)
    pub colorblind: bool,
}

impl Default for Look {
    fn default() -> Look {
        Look { theme: Theme::Night, base: BASE, colorblind: false }
    }
}

impl Look {
    /// a step of the UI size: −1 smaller, +1 larger, 0 back to 100 %
    pub fn sized(self, step: i32) -> Look {
        let base = if step == 0 { BASE } else { (self.base + step as f32).clamp(BASE_RANGE.0, BASE_RANGE.1) };
        Look { base, ..self }
    }

    /// the UI size in percent of the default
    pub fn percent(&self) -> u32 {
        (self.base / BASE * 100.0).round() as u32
    }
}

/// the tokens file, watched, and the look
pub struct Watched {
    pub path: PathBuf,
    pub tokens: Tokens,
    pub look: Look,
    text: String,
    seen: Option<SystemTime>,
}

impl Watched {
    pub fn new(path: PathBuf, look: Look) -> Watched {
        let mut w = Watched { path, tokens: Tokens::new(look.theme, look.base).colorblind(look.colorblind), look, text: String::new(), seen: None };
        w.poll();
        w
    }

    /// the file's modification time changed: read it again; true when the tokens changed
    pub fn poll(&mut self) -> bool {
        let modified = std::fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        if modified.is_none() || modified == self.seen {
            return false;
        }
        self.seen = modified;
        match std::fs::read_to_string(&self.path) {
            Ok(t) => {
                self.text = t;
                self.apply()
            }
            Err(e) => {
                crate::log::post(deck_ui::messages::Level::Warn, "tokens", format!("{}: {e}", self.path.display()));
                false
            }
        }
    }

    /// another look, applied at once; true when the tokens changed
    pub fn set(&mut self, look: Look) -> bool {
        if look == self.look {
            return false;
        }
        self.look = look;
        self.apply()
    }

    fn apply(&mut self) -> bool {
        match Tokens::from_toml(&self.text, self.look.theme, self.look.base).map(|t| t.colorblind(self.look.colorblind)) {
            Ok(t) => {
                if t != self.tokens {
                    self.tokens = t;
                    return true;
                }
                false
            }
            Err(e) => {
                crate::log::post(deck_ui::messages::Level::Warn, "tokens", format!("{}: {e}", self.path.display()));
                false
            }
        }
    }
}

/// `tokens.toml` beside the binary, else the crate's own
pub fn default_path() -> PathBuf {
    let beside = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("tokens.toml"))).filter(|p| p.is_file());
    beside.unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tokens.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ui_size_steps_by_a_pixel_and_stays_in_range() {
        let l = Look::default();
        assert_eq!(l.percent(), 100);
        assert_eq!(l.sized(1).base, BASE + 1.0);
        assert_eq!(l.sized(1).sized(0), l);
        let mut big = l;
        for _ in 0..100 {
            big = big.sized(1);
        }
        assert_eq!(big.base, BASE_RANGE.1);
    }

    #[test]
    fn a_look_switches_live() {
        let dir = std::env::temp_dir().join(format!("deck-editor-look-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let toml = dir.join("tokens.toml");
        std::fs::write(&toml, "[night.color]\nline = [1.0, 0.0, 0.0, 1.0]\n").unwrap();
        let mut w = Watched::new(toml.clone(), Look { theme: Theme::Day, base: BASE, colorblind: false });
        let day = w.tokens.clone();
        assert!(w.set(Look { theme: Theme::Night, base: 20.0, colorblind: false }));
        assert_ne!(w.tokens.color.text, day.color.text);
        assert_eq!(w.tokens.color.line, [1.0, 0.0, 0.0, 1.0], "the file's night override");
        assert_eq!(w.tokens.size.font, 20.0);
        assert!(w.set(Look { theme: Theme::Night, base: 20.0, colorblind: true }));
        assert_eq!(w.tokens.color.stop, w.tokens.color.colorblind.stop, "the colour-blind set");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
