//! Design tokens (#444): colours and sizes from `tokens.toml` next to the binary (or `--tokens FILE`), watched by its
//! modification time and applied in the next frame – a changed token shows in under a second without a restart. Later
//! these come from shared/tokens; the file is the proof's stand-in.
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct Colors {
    pub background: [f32; 4],
    pub panel: [f32; 4],
    pub divider: [f32; 4],
    pub text: [f32; 4],
    pub muted: [f32; 4],
    pub accent: [f32; 4],
    pub field: [f32; 4],
    pub selection: [f32; 4],
    pub tab: [f32; 4],
    pub tab_active: [f32; 4],
    /// an output port and a cable being dragged
    pub cable: [f32; 4],
    /// a refused gesture's sentence
    pub refused: [f32; 4],
    /// the view gizmo's x, y, z
    pub axis: [[f32; 4]; 3],
}

impl Default for Colors {
    fn default() -> Colors {
        Colors {
            background: [0.07, 0.07, 0.08, 1.0],
            panel: [0.12, 0.12, 0.14, 1.0],
            divider: [0.22, 0.22, 0.26, 1.0],
            text: [0.92, 0.92, 0.9, 1.0],
            muted: [0.55, 0.55, 0.6, 1.0],
            accent: [0.98, 0.62, 0.2, 1.0],
            field: [0.17, 0.17, 0.2, 1.0],
            selection: [0.3, 0.45, 0.7, 0.6],
            tab: [0.16, 0.16, 0.19, 1.0],
            tab_active: [0.22, 0.22, 0.26, 1.0],
            cable: [0.0, 0.82, 1.0, 1.0],
            refused: [0.8, 0.1, 0.0, 1.0],
            axis: [[0.93, 0.27, 0.32, 1.0], [0.47, 0.75, 0.2, 1.0], [0.25, 0.53, 0.95, 1.0]],
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct Sizes {
    pub panel: f32,
    pub divider: f32,
    pub pad: f32,
    pub font: f32,
    pub row: f32,
    /// the height of the window's top bar
    pub bar: f32,
}

impl Default for Sizes {
    fn default() -> Sizes {
        Sizes { panel: 320.0, divider: 6.0, pad: 12.0, font: 15.0, row: 28.0, bar: 26.0 }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Tokens {
    pub color: Colors,
    pub size: Sizes,
}

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
