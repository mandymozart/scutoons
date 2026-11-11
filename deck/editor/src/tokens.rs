//! The design tokens' file (#444): `tokens.toml` next to the binary (or `--tokens FILE`), watched by its modification time
//! and applied in the next frame – a changed token shows in under a second without a restart. The tokens are the kit's.
pub use deck_kit::base::tokens::Tokens;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// the tokens file, watched
pub struct Watched {
    pub path: PathBuf,
    pub tokens: Tokens,
    seen: Option<SystemTime>,
    /// the last problem reading it (shown in the panel)
    pub problem: Option<String>,
}

impl Watched {
    pub fn new(path: PathBuf) -> Watched {
        let mut w = Watched { path, tokens: Tokens::default(), seen: None, problem: None };
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
        match std::fs::read_to_string(&self.path).map_err(|e| e.to_string()).and_then(|t| toml::from_str::<Tokens>(&t).map_err(|e| e.to_string())) {
            Ok(t) => {
                self.problem = None;
                if t != self.tokens {
                    self.tokens = t;
                    return true;
                }
                false
            }
            Err(e) => {
                self.problem = Some(e);
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
