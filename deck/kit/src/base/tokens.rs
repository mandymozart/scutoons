//! Design tokens: the names are what a thing is, not where it is used. The editor has two colour sets, day (light) and
//! night (dark), with the same roles; its sizes are multiples of one base size (its rem), so the whole UI scales like a
//! browser zoom. A `tokens.toml` overrides them live. The `live` module brings its own sets (`live/tokens.rs`).
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// a pressed segment or tab, and the text on it
    pub pressed: [f32; 4],
    pub pressed_text: [f32; 4],
    pub selected: [f32; 4],
    pub within: [f32; 4],
    /// the one colour with a meaning of its own: live, an out, a cable in the hand
    pub accent: [f32; 4],
    /// the one primary action of a view, its edge and its text
    pub primary: [f32; 4],
    pub primary_line: [f32; 4],
    pub primary_ink: [f32; 4],
    /// light ink: text over the stage and on a coloured ground (danger, go)
    pub bright: [f32; 4],
    pub selection: [f32; 4],
    pub danger: [f32; 4],
    /// the play button: go, and stop
    pub go: [f32; 4],
    pub warn: [f32; 4],
    pub stop: [f32; 4],
    /// the view gizmo's x, y, z
    pub axis: [[f32; 4]; 3],
    /// a strip's stripe, round the list
    pub strips: [[f32; 4]; 6],
    /// how much a floating panel's ground covers the stage behind it (0..1)
    pub panel_opacity: f32,
}

/// `#rrggbb`
pub const fn rgb(v: u32) -> [f32; 4] {
    [((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0, 1.0]
}

/// the editor's two looks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Day,
    #[default]
    Night,
}

impl Theme {
    pub fn other(self) -> Theme {
        match self {
            Theme::Day => Theme::Night,
            Theme::Night => Theme::Day,
        }
    }
}

impl Colors {
    /// warm off-white panels, white fields, a sage line, olive greys, the near-black pressed segment, the lime primary
    pub fn day() -> Colors {
        Colors {
            stage: rgb(0x000000),
            surface: rgb(0xf3f3f0),
            surface_head: rgb(0xe8eae2),
            line: rgb(0xd9decf),
            text: rgb(0x171916),
            muted: rgb(0x4f5747),
            control: rgb(0xfffffb),
            hover: rgb(0xe4e8dc),
            pressed: rgb(0x22271e),
            pressed_text: rgb(0xffffff),
            selected: rgb(0xd5dbc9),
            within: rgb(0xe2e6da),
            accent: rgb(0x00728d),
            primary: rgb(0xb4ff32),
            primary_line: rgb(0x9ce327),
            primary_ink: rgb(0x171916),
            bright: rgb(0xf3f3f0),
            selection: [0.0, 0.45, 0.55, 0.35],
            danger: rgb(0xc0392b),
            go: rgb(0x2e8a3e),
            warn: rgb(0xb86e0c),
            stop: rgb(0xd63a2d),
            axis: [[0.93, 0.27, 0.32, 1.0], [0.47, 0.75, 0.2, 1.0], [0.25, 0.53, 0.95, 1.0]],
            strips: [rgb(0x00a6cc), rgb(0xe08e1c), rgb(0x9a52e6), rgb(0x3aa84a), rgb(0xe8577a), rgb(0xc9b42e)],
            panel_opacity: 0.86,
        }
    }

    /// the same roles dark: olive black panels, the light pressed segment, the same lime
    pub fn night() -> Colors {
        Colors {
            stage: rgb(0x000000),
            surface: rgb(0x1a1c18),
            surface_head: rgb(0x141612),
            line: rgb(0x3a4034),
            text: rgb(0xf3f3f0),
            muted: rgb(0xa7af9b),
            control: rgb(0x262922),
            hover: rgb(0x2e3229),
            pressed: rgb(0xf3f3f0),
            pressed_text: rgb(0x171916),
            selected: rgb(0x3a4132),
            within: rgb(0x23261f),
            accent: rgb(0x00d1ff),
            primary: rgb(0xb4ff32),
            primary_line: rgb(0x9ce327),
            primary_ink: rgb(0x171916),
            bright: rgb(0xf3f3f0),
            selection: [0.3, 0.45, 0.7, 0.6],
            danger: rgb(0xc0392b),
            go: rgb(0x4cc35a),
            warn: rgb(0xf0a030),
            stop: rgb(0xe5483b),
            axis: [[0.93, 0.27, 0.32, 1.0], [0.47, 0.75, 0.2, 1.0], [0.25, 0.53, 0.95, 1.0]],
            strips: [rgb(0x00d1ff), rgb(0xf0a030), rgb(0xb36bff), rgb(0x4cc35a), rgb(0xff6b8a), rgb(0xe6d24a)],
            panel_opacity: 0.84,
        }
    }

    pub fn of(theme: Theme) -> Colors {
        match theme {
            Theme::Day => Colors::day(),
            Theme::Night => Colors::night(),
        }
    }
}

impl Default for Colors {
    fn default() -> Colors {
        Colors::day()
    }
}

/// the sizes in rem: multiples of the base size
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Rems {
    /// the top bar
    pub top: f32,
    /// the tree on the left
    pub tree: f32,
    /// the inspector on the right
    pub inspector: f32,
    pub pad: f32,
    /// between floating panels and to the walls
    pub gap: f32,
    pub font: f32,
    /// values and small text
    pub small: f32,
    /// the small uppercase labels
    pub label: f32,
    pub title: f32,
    /// a control's height
    pub row: f32,
    /// a tree row's height
    pub tree_row: f32,
    /// a control's corners
    pub radius: f32,
    /// a floating panel's smallest open size before its content scrolls
    pub panel_w: f32,
    pub panel_h: f32,
}

impl Default for Rems {
    fn default() -> Rems {
        Rems {
            top: 2.6,
            tree: 20.0,
            inspector: 27.0,
            pad: 0.57,
            gap: 1.0,
            font: 1.0,
            small: 0.86,
            label: 0.72,
            title: 1.15,
            row: 2.15,
            tree_row: 1.8,
            radius: 0.36,
            panel_w: 11.5,
            panel_h: 3.4,
        }
    }
}

/// the editor's base size at 100 %
pub const BASE: f32 = 14.0;
/// the base sizes the UI size setting allows
pub const BASE_RANGE: (f32, f32) = (9.0, 28.0);

/// the sizes in pixels at scale 1, what every control reads
#[derive(Debug, Clone, PartialEq)]
pub struct Sizes {
    /// 1rem
    pub base: f32,
    pub top: f32,
    pub tree: f32,
    pub inspector: f32,
    /// a region's edge (a border width: pixels, not rem)
    pub edge: f32,
    pub pad: f32,
    pub gap: f32,
    pub font: f32,
    pub small: f32,
    pub label: f32,
    pub title: f32,
    pub row: f32,
    pub tree_row: f32,
    pub radius: f32,
    pub panel_w: f32,
    pub panel_h: f32,
    /// the labels' letter spacing, in em
    pub spacing: f32,
}

impl Sizes {
    /// `n` rem in pixels
    pub fn rem(&self, n: f32) -> f32 {
        n * self.base
    }
}

impl Rems {
    pub fn at(&self, base: f32) -> Sizes {
        let b = base.clamp(BASE_RANGE.0, BASE_RANGE.1);
        Sizes {
            base: b,
            top: self.top * b,
            tree: self.tree * b,
            inspector: self.inspector * b,
            edge: 1.0,
            pad: self.pad * b,
            gap: self.gap * b,
            font: self.font * b,
            small: self.small * b,
            label: self.label * b,
            title: self.title * b,
            row: self.row * b,
            tree_row: self.tree_row * b,
            radius: self.radius * b,
            panel_w: self.panel_w * b,
            panel_h: self.panel_h * b,
            spacing: 0.15,
        }
    }
}

impl Default for Sizes {
    fn default() -> Sizes {
        Rems::default().at(BASE)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tokens {
    pub color: Colors,
    pub size: Sizes,
}

/// what a `tokens.toml` may set: `[color]` and `[size]` (rem) for both looks, `[day.color]` / `[night.color]` for one
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct File {
    color: toml::Table,
    size: toml::Table,
    day: Part,
    night: Part,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct Part {
    color: toml::Table,
}

/// `over`'s keys replace `base`'s
fn layer(base: &mut toml::Table, over: &toml::Table) {
    for (k, v) in over {
        base.insert(k.clone(), v.clone());
    }
}

fn table<T: Serialize>(v: &T) -> toml::Table {
    toml::Table::try_from(v).unwrap_or_default()
}

impl Tokens {
    pub fn new(theme: Theme, base: f32) -> Tokens {
        Tokens { color: Colors::of(theme), size: Rems::default().at(base) }
    }

    /// the look `theme` at `base`, with a `tokens.toml`'s overrides
    pub fn from_toml(text: &str, theme: Theme, base: f32) -> Result<Tokens, String> {
        let f: File = toml::from_str(text).map_err(|e| e.to_string())?;
        let mut color = table(&Colors::of(theme));
        layer(&mut color, &f.color);
        layer(&mut color, &match theme {
            Theme::Day => f.day.color,
            Theme::Night => f.night.color,
        });
        let mut size = table(&Rems::default());
        layer(&mut size, &f.size);
        let color: Colors = color.try_into().map_err(|e: toml::de::Error| e.to_string())?;
        let size: Rems = size.try_into().map_err(|e: toml::de::Error| e.to_string())?;
        Ok(Tokens { color, size: size.at(base) })
    }
}

/// the WCAG contrast of two opaque colours (1..21)
pub fn contrast(a: [f32; 4], b: [f32; 4]) -> f32 {
    let lum = |c: [f32; 4]| {
        let ch = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
        0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
    };
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `top` at `alpha` over `under`
pub fn over(top: [f32; 4], alpha: f32, under: [f32; 4]) -> [f32; 4] {
    let m = |i: usize| top[i] * alpha + under[i] * (1.0 - alpha);
    [m(0), m(1), m(2), 1.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRIGHT_STAGE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    const DARK_STAGE: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

    #[test]
    fn both_looks_read_on_their_panels_over_a_bright_and_a_dark_stage() {
        for theme in [Theme::Day, Theme::Night] {
            let c = Colors::of(theme);
            for stage in [BRIGHT_STAGE, DARK_STAGE] {
                let panel = over(c.surface, c.panel_opacity, stage);
                assert!(contrast(c.text, panel) >= 7.0, "{theme:?} text on panel over {stage:?}: {}", contrast(c.text, panel));
                assert!(contrast(c.muted, panel) >= 4.5, "{theme:?} muted on panel over {stage:?}: {}", contrast(c.muted, panel));
                assert!(contrast(c.accent, panel) >= 3.0, "{theme:?} accent on panel over {stage:?}: {}", contrast(c.accent, panel));
            }
            assert!(contrast(c.text, c.control) >= 7.0, "{theme:?} text in a field");
            assert!(contrast(c.muted, c.control) >= 4.5, "{theme:?} muted in a field");
            assert!(contrast(c.primary_ink, c.primary) >= 7.0, "{theme:?} ink on lime");
            assert!(contrast(c.pressed_text, c.pressed) >= 7.0, "{theme:?} pressed text");
            assert!(contrast(c.text, c.selected) >= 4.5, "{theme:?} text on a selected row");
            assert!(contrast(c.text, c.within) >= 4.5, "{theme:?} text on a scrub's fill");
            assert!(contrast(c.bright, c.stage) >= 7.0, "{theme:?} labels over the stage");
            assert!(contrast(c.bright, c.danger) >= 4.5, "{theme:?} light ink on danger");
        }
    }

    #[test]
    fn the_stage_stays_black_in_both_looks() {
        assert_eq!(Colors::day().stage, Colors::night().stage);
    }

    #[test]
    fn every_size_follows_the_base() {
        let (a, b) = (Rems::default().at(14.0), Rems::default().at(21.0));
        for (x, y) in [(a.font, b.font), (a.row, b.row), (a.gap, b.gap), (a.radius, b.radius), (a.panel_w, b.panel_w), (a.label, b.label)] {
            assert!((y / x - 1.5).abs() < 1e-4, "{x} → {y}");
        }
        assert_eq!(a.font, 14.0);
        assert_eq!(a.gap, a.base, "the panels' gap is 1rem");
        assert_eq!(Rems::default().at(1000.0).base, BASE_RANGE.1);
    }

    #[test]
    fn a_file_overrides_both_looks_or_one() {
        let text = "[size]\nfont = 1.25\n[color]\nline = [1.0, 0.0, 0.0, 1.0]\n[night.color]\ntext = [0.5, 0.5, 0.5, 1.0]\n";
        let day = Tokens::from_toml(text, Theme::Day, 16.0).unwrap();
        let night = Tokens::from_toml(text, Theme::Night, 16.0).unwrap();
        assert_eq!(day.size.font, 20.0);
        assert_eq!(day.color.line, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(night.color.line, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(day.color.text, Colors::day().text);
        assert_eq!(night.color.text, [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(Tokens::from_toml("", Theme::Night, BASE).unwrap(), Tokens::new(Theme::Night, BASE));
        assert!(Tokens::from_toml("[size]\nfont = \"big\"", Theme::Day, BASE).is_err());
    }
}
