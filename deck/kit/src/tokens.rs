//! Design tokens (#456; owner 2026-11-09: "the ui can use the styles we have in the osct editor, but apply a design token
//! system we can reuitlize"): the names are what a thing is, not where it is used; the values are the OSCT editor's dark
//! ones. Every control reads them; a `tokens.toml` overrides them live.
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct Colors {
    /// behind the stage
    pub stage: [f32; 4],
    pub surface: [f32; 4],
    pub surface_head: [f32; 4],
    pub line: [f32; 4],
    pub text: [f32; 4],
    pub muted: [f32; 4],
    /// a field's or a framed control's ground
    pub control: [f32; 4],
    pub hover: [f32; 4],
    pub pressed: [f32; 4],
    pub selected: [f32; 4],
    pub within: [f32; 4],
    /// the one colour with a meaning of its own: live, an out, a cable in the hand
    pub accent: [f32; 4],
    pub selection: [f32; 4],
    pub danger: [f32; 4],
    /// the play button: go, and stop
    pub go: [f32; 4],
    pub warn: [f32; 4],
    pub stop: [f32; 4],
    /// the view gizmo's x, y, z
    pub axis: [[f32; 4]; 3],
}

/// `#rrggbb`
const fn rgb(v: u32) -> [f32; 4] {
    [((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0, 1.0]
}

impl Default for Colors {
    fn default() -> Colors {
        Colors {
            stage: rgb(0x0c0c0c),
            surface: rgb(0x2c2c2c),
            surface_head: rgb(0x1e1e1e),
            line: rgb(0x444444),
            text: rgb(0xffffff),
            muted: rgb(0xa3a3a3),
            control: rgb(0x383838),
            hover: rgb(0x3f3f3f),
            pressed: rgb(0x4f4f4f),
            selected: rgb(0x5e5e5e),
            within: rgb(0x363636),
            accent: [0.0, 0.82, 1.0, 1.0],
            selection: [0.3, 0.45, 0.7, 0.6],
            danger: rgb(0xc0392b),
            go: rgb(0x4cc35a),
            warn: rgb(0xf0a030),
            stop: rgb(0xe5483b),
            axis: [[0.93, 0.27, 0.32, 1.0], [0.47, 0.75, 0.2, 1.0], [0.25, 0.53, 0.95, 1.0]],
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct Sizes {
    /// the top bar
    pub top: f32,
    /// the tree on the left
    pub tree: f32,
    /// the inspector on the right
    pub inspector: f32,
    /// a region's edge
    pub edge: f32,
    pub pad: f32,
    pub font: f32,
    pub small: f32,
    /// a control's height
    pub row: f32,
    /// a tree row's height
    pub tree_row: f32,
    /// a control's corners
    pub radius: f32,
}

impl Default for Sizes {
    fn default() -> Sizes {
        Sizes { top: 36.0, tree: 220.0, inspector: 280.0, edge: 1.0, pad: 8.0, font: 12.0, small: 11.0, row: 28.0, tree_row: 24.0, radius: 5.0 }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Tokens {
    pub color: Colors,
    pub size: Sizes,
}
