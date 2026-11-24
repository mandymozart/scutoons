//! Design tokens: the names are what a thing is, not where it is used. The editor has two looks, day and night, with the
//! same roles; its sizes are multiples of one base size (its rem), so the whole UI scales like a browser zoom. Every value
//! is the kit's one source, `kit/tokens.toml` at the repo's top, built in as the base (build.rs); a `tokens.toml` beside
//! the editor overrides it live. The components' states (`[component.<name>.<state>]`) resolve to colours here
//! (`Tokens::kit`). The `live` module brings its own sets (`live/tokens.rs`).
use crate::base::signal::Signals;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// the kit's tokens, built in
pub const KIT: &str = include_str!(env!("DECK_KIT_TOKENS"));

pub type Rgba = [f32; 4];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Colors {
    /// behind the stage
    pub stage: Rgba,
    /// the stage colour, opaque: at `panel_opacity` the ground of a panel, the bar, a menu, a popover; opaque the ink ring
    pub surface: Rgba,
    pub surface_head: Rgba,
    pub line: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    /// a field's or a framed control's ground
    pub control: Rgba,
    pub hover: Rgba,
    /// a pressed segment or tab, the one primary action, a selected row – and the ink on it
    pub pressed: Rgba,
    pub pressed_text: Rgba,
    pub selected: Rgba,
    pub within: Rgba,
    /// light ink: text over the stage and on a coloured ground (danger, go)
    pub bright: Rgba,
    pub selection: Rgba,
    pub danger: Rgba,
    /// ok, warning, error: the only colours the UI's own parts say
    pub go: Rgba,
    pub warn: Rgba,
    pub stop: Rgba,
    /// the view gizmo's x, y, z
    pub axis: [Rgba; 3],
    /// a strip's stripe, round the list
    pub strips: [Rgba; 6],
    /// what a port carries (signal.rs)
    pub signals: Signals,
    /// where a thing plays
    pub outlets: Outlets,
    /// a group of spores: one of six, a new group takes the first unused
    pub groups: [Rgba; 6],
    /// ok, warning, error of the colour-blind setting
    pub colorblind: Status,
    /// how much a surface's ground covers the stage behind it (0..1)
    pub panel_opacity: f32,
    /// a surface's text and icons get the surface's colour, opaque, as an outline (`DrawList::halo`), so they read over any
    /// stage (no blur: it is heavy on the GPU); without it the see-through ground alone has to hold them
    pub ink_ring: bool,
}

/// where a thing plays: the reader's page, a deck's screen or speaker, an XR scene, a device (DMX …)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Outlets {
    pub page: Rgba,
    pub stage: Rgba,
    pub target: Rgba,
    pub device: Rgba,
}

/// ok, warning, error
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub go: Rgba,
    pub warn: Rgba,
    pub stop: Rgba,
}

/// `#rrggbb`
pub const fn rgb(v: u32) -> Rgba {
    [((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0, 1.0]
}

/// `#rrggbb` or `#rrggbbaa`
pub fn hex(s: &str) -> Option<Rgba> {
    let h = s.strip_prefix('#')?;
    if !(h.len() == 6 || h.len() == 8) || !h.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok().map(|b| b as f32 / 255.0);
    Some([byte(0)?, byte(2)?, byte(4)?, if h.len() == 8 { byte(6)? } else { 1.0 }])
}

/// `#rrggbb` (the alpha left out)
pub fn to_hex(c: Rgba) -> String {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(c[0]), b(c[1]), b(c[2]))
}

/// `c` at `share` of its strength
pub fn alpha(c: Rgba, share: f32) -> Rgba {
    [c[0], c[1], c[2], c[3] * share]
}

/// the editor's two looks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Day,
    #[default]
    Night,
}

impl Colors {
    pub fn day() -> Colors {
        kit().day.clone()
    }

    pub fn night() -> Colors {
        kit().night.clone()
    }

    pub fn of(theme: Theme) -> Colors {
        match theme {
            Theme::Day => Colors::day(),
            Theme::Night => Colors::night(),
        }
    }

    /// the halo a surface's content takes (`DrawList::halo`)
    pub fn ring(&self) -> Option<Rgba> {
        self.ink_ring.then_some(self.surface)
    }

    /// the see-through ground of every surface: a panel, the bar, a card, a menu, a popover
    pub fn panel(&self) -> Rgba {
        alpha(self.surface, self.panel_opacity.clamp(0.0, 1.0))
    }

    /// the colour of the group at `i`, round the six
    pub fn group(&self, i: usize) -> Rgba {
        self.groups[i % self.groups.len()]
    }

    /// ok, warning and error swapped for the colour-blind set
    pub fn colorblind(mut self) -> Colors {
        let Status { go, warn, stop } = self.colorblind;
        (self.go, self.warn, self.stop) = (go, warn, stop);
        self
    }
}

impl Default for Colors {
    fn default() -> Colors {
        Colors::day()
    }
}

/// the sizes in rem: multiples of the base size
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rems {
    /// 1rem at 100 %, px
    pub base: f32,
    /// the top bar
    pub top: f32,
    /// the tree on the left
    pub tree: f32,
    /// the inspector on the right
    pub inspector: f32,
    /// a line or a region's edge, px
    pub edge: f32,
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
    /// the labels' letter spacing, em
    pub spacing: f32,
}

impl Default for Rems {
    fn default() -> Rems {
        kit().size.clone()
    }
}

/// the editor's base size at 100 % (the kit's `[size] base`: a test holds them equal)
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
            edge: self.edge,
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
            spacing: self.spacing,
        }
    }
}

impl Default for Sizes {
    fn default() -> Sizes {
        Rems::default().at(BASE)
    }
}

/// a component in one state as the kit draws it: each part a colour, `None` where the part is not drawn
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct State {
    pub text: Option<Rgba>,
    pub ground: Option<Rgba>,
    pub border: Option<Rgba>,
    /// the focus outline
    pub ring: Option<Rgba>,
    pub knob: Option<Rgba>,
    /// a tick, a dot, a chevron, the insert line
    pub mark: Option<Rgba>,
    pub opacity: f32,
    /// rem; the overlay's alone
    pub blur: f32,
}

impl Default for State {
    fn default() -> State {
        State { text: None, ground: None, border: None, ring: None, knob: None, mark: None, opacity: 1.0, blur: 0.0 }
    }
}

impl State {
    /// `c` faded by the state's opacity (a disabled control)
    pub fn faded(&self, c: Rgba) -> Rgba {
        alpha(c, self.opacity)
    }
}

/// every component's states, resolved against one look
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Kit {
    states: BTreeMap<String, State>,
}

impl Kit {
    /// `name` in `state` (over the component's rest); a state the kit does not write is its rest
    pub fn get(&self, name: &str, state: &str) -> State {
        self.states.get(&format!("{name}.{state}")).or_else(|| self.states.get(&format!("{name}.{}", rest_of(name)))).copied().unwrap_or_default()
    }

    /// the names of the states a component has, as written
    pub fn states(&self, name: &str) -> Vec<&str> {
        let prefix = format!("{name}.");
        self.states.keys().filter_map(|k| k.strip_prefix(&prefix)).collect()
    }
}

/// a component's resting state's name: `rest`, except the toggle's `off` and the status dot's `none`
fn rest_of(name: &str) -> &'static str {
    match name {
        "toggle" => "off",
        "status" => "none",
        _ => "rest",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tokens {
    pub color: Colors,
    pub size: Sizes,
    pub kit: Kit,
}

impl Default for Tokens {
    fn default() -> Tokens {
        Tokens::new(Theme::Day, BASE)
    }
}

/// what the kit file, or a `tokens.toml` over it, sets: `[color]` and `[size]` (rem) for both looks, `[day.*]` /
/// `[night.*]` for one, `[groups]`, the components' states
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct File {
    color: toml::Table,
    size: toml::Table,
    day: Part,
    night: Part,
    groups: toml::Table,
    component: BTreeMap<String, BTreeMap<String, toml::Table>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct Part {
    color: toml::Table,
    signals: toml::Table,
    outlets: toml::Table,
    colorblind: toml::Table,
}

impl File {
    fn part(&self, theme: Theme) -> &Part {
        match theme {
            Theme::Day => &self.day,
            Theme::Night => &self.night,
        }
    }
}

/// `over`'s keys replace `base`'s
fn layer(base: &mut toml::Table, over: &toml::Table) {
    for (k, v) in over {
        base.insert(k.clone(), v.clone());
    }
}

/// every `"#rrggbb(aa)"` string as four numbers
fn colours(v: &mut toml::Value) -> Result<(), String> {
    match v {
        toml::Value::String(s) if s.starts_with('#') => {
            let c = hex(s).ok_or_else(|| format!("not a colour: {s}"))?;
            *v = toml::Value::Array(c.iter().map(|&x| toml::Value::Float(x as f64)).collect());
        }
        toml::Value::Array(a) => a.iter_mut().try_for_each(colours)?,
        toml::Value::Table(t) => t.iter_mut().try_for_each(|(_, v)| colours(v))?,
        _ => {}
    }
    Ok(())
}

/// one look's colours from the files in order, each over the one before
fn compose(files: &[&File], theme: Theme) -> Result<toml::Table, String> {
    let mut color = toml::Table::new();
    for f in files {
        layer(&mut color, &f.color);
        let p = f.part(theme);
        layer(&mut color, &p.color);
        for (key, t) in [("signals", &p.signals), ("outlets", &p.outlets), ("colorblind", &p.colorblind)] {
            if let toml::Value::Table(nested) = color.entry(key).or_insert_with(|| toml::Value::Table(toml::Table::new())) {
                layer(nested, t);
            }
        }
        if let Some(g) = f.groups.get("colors") {
            color.insert("groups".into(), g.clone());
        }
    }
    let mut v = toml::Value::Table(color);
    colours(&mut v)?;
    let toml::Value::Table(mut color) = v else { unreachable!() };
    // the kit names the nine signals a person wires; a job takes the warning's colour, an asset the muted one
    let (warn, muted) = (color.get("warn").cloned(), color.get("muted").cloned());
    if let Some(toml::Value::Table(s)) = color.get_mut("signals") {
        for (k, v) in [("job", warn), ("asset", muted)] {
            if let (false, Some(v)) = (s.contains_key(k), v) {
                s.insert(k.into(), v);
            }
        }
    }
    Ok(color)
}

fn sizes(files: &[&File]) -> toml::Table {
    let mut size = toml::Table::new();
    for f in files {
        layer(&mut size, &f.size);
    }
    size
}

/// a part's token, `"text"` or `"text 12%"` (see-through at 12 %); `none` is not drawn
fn paint(c: &Colors, v: &str) -> Result<Option<Rgba>, String> {
    let (name, share) = match v.split_once(' ') {
        Some((n, p)) => (n, p.strip_suffix('%').and_then(|p| p.parse::<f32>().ok()).ok_or_else(|| format!("not a share: {v}"))? / 100.0),
        None => (v, 1.0),
    };
    let base = match name {
        "none" => return Ok(None),
        "panel" | "popover" => c.panel(),
        "surface" => c.surface,
        "surface_head" => c.surface_head,
        "line" => c.line,
        "text" => c.text,
        "muted" => c.muted,
        "bright" => c.bright,
        "control" => c.control,
        "hover" => c.hover,
        "pressed" => c.pressed,
        "pressed_text" => c.pressed_text,
        "selected" => c.selected,
        "within" => c.within,
        "selection" => c.selection,
        "go" => c.go,
        "warn" => c.warn,
        "stop" => c.stop,
        _ => return Err(format!("no token {name}")),
    };
    Ok(Some(alpha(base, share)))
}

type Specs = BTreeMap<String, BTreeMap<String, toml::Table>>;

fn state(c: &Colors, parts: &toml::Table) -> Result<State, String> {
    let mut s = State::default();
    for (k, v) in parts {
        let number = || v.as_float().or_else(|| v.as_integer().map(|i| i as f64)).map(|f| f as f32).ok_or_else(|| format!("{k}: not a number"));
        match k.as_str() {
            "opacity" => s.opacity = number()?,
            "blur" => s.blur = number()?,
            part => {
                let at = paint(c, v.as_str().ok_or_else(|| format!("{part}: not a token"))?)?;
                match part {
                    "text" => s.text = at,
                    "ground" => s.ground = at,
                    "border" => s.border = at,
                    "ring" => s.ring = at,
                    "knob" => s.knob = at,
                    "mark" => s.mark = at,
                    _ => return Err(format!("no part {part}")),
                }
            }
        }
    }
    Ok(s)
}

/// every state of `specs` over its component's rest, against `c`
fn resolve(specs: &Specs, c: &Colors) -> Result<Kit, String> {
    let mut states = BTreeMap::new();
    for (name, all) in specs {
        let rest = all.get(rest_of(name)).cloned().unwrap_or_default();
        for (st, parts) in all {
            let mut layered = rest.clone();
            layer(&mut layered, parts);
            states.insert(format!("{name}.{st}"), state(c, &layered).map_err(|e| format!("component.{name}.{st}: {e}"))?);
        }
    }
    Ok(Kit { states })
}

fn specs(files: &[&File]) -> Specs {
    let mut out = Specs::new();
    for f in files {
        for (name, all) in &f.component {
            let into = out.entry(name.clone()).or_default();
            for (st, parts) in all {
                layer(into.entry(st.clone()).or_default(), parts);
            }
        }
    }
    out
}

/// the built-in kit, read once
struct Looks {
    file: File,
    day: Colors,
    night: Colors,
    size: Rems,
    specs: Specs,
}

fn looks(text: &str) -> Result<Looks, String> {
    let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
    let colors = |theme| -> Result<Colors, String> { compose(&[&file], theme)?.try_into().map_err(|e: toml::de::Error| format!("{theme:?}: {e}")) };
    let (day, night) = (colors(Theme::Day)?, colors(Theme::Night)?);
    let size: Rems = sizes(&[&file]).try_into().map_err(|e: toml::de::Error| format!("size: {e}"))?;
    let specs = specs(&[&file]);
    resolve(&specs, &day)?;
    Ok(Looks { file, day, night, size, specs })
}

fn kit() -> &'static Looks {
    static KIT_LOOKS: OnceLock<Looks> = OnceLock::new();
    KIT_LOOKS.get_or_init(|| looks(KIT).unwrap_or_else(|e| panic!("kit/tokens.toml: {e}")))
}

impl Tokens {
    pub fn new(theme: Theme, base: f32) -> Tokens {
        Tokens::with(Colors::of(theme), Rems::default().at(base))
    }

    /// a look's colours and sizes, the kit's states resolved against them
    pub fn with(color: Colors, size: Sizes) -> Tokens {
        let kit = resolve(&kit().specs, &color).unwrap_or_default();
        Tokens { color, size, kit }
    }

    /// the look `theme` at `base`, with a `tokens.toml`'s overrides over the kit
    pub fn from_toml(text: &str, theme: Theme, base: f32) -> Result<Tokens, String> {
        let over: File = toml::from_str(text).map_err(|e| e.to_string())?;
        let files = [&kit().file, &over];
        let color: Colors = compose(&files, theme)?.try_into().map_err(|e: toml::de::Error| e.to_string())?;
        let size: Rems = sizes(&files).try_into().map_err(|e: toml::de::Error| e.to_string())?;
        let kit = resolve(&specs(&files), &color)?;
        Ok(Tokens { color, size: size.at(base), kit })
    }

    /// ok, warning and error in the colour-blind set (the setting), the states resolved again
    pub fn colorblind(self, on: bool) -> Tokens {
        if !on {
            return self;
        }
        let specs = kit().specs.clone();
        let color = self.color.colorblind();
        let kit = resolve(&specs, &color).unwrap_or(self.kit);
        Tokens { color, kit, ..self }
    }

    /// a component in a state (`[component.<name>.<state>]`)
    pub fn kit(&self, name: &str, state: &str) -> State {
        self.kit.get(name, state)
    }
}

/// the relative luminance of an opaque colour (0..1)
pub fn luminance(c: Rgba) -> f32 {
    let ch = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
}

/// the WCAG contrast of two opaque colours (1..21)
pub fn contrast(a: Rgba, b: Rgba) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `top` at `alpha` over `under`
pub fn over(top: Rgba, alpha: f32, under: Rgba) -> Rgba {
    let m = |i: usize| top[i] * alpha + under[i] * (1.0 - alpha);
    [m(0), m(1), m(2), 1.0]
}

/// a see-through colour laid over an opaque one
pub fn on(top: Rgba, under: Rgba) -> Rgba {
    over(top, top[3], under)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRIGHT_STAGE: Rgba = [1.0, 1.0, 1.0, 1.0];
    const DARK_STAGE: Rgba = [0.0, 0.0, 0.0, 1.0];

    /// bright, colourful scenes: white, sky, moss, periwinkle, yellow, pink, orange
    const COLOURFUL_STAGES: [u32; 7] = [0xffffff, 0x4fc3f7, 0x6aa84f, 0x8088d0, 0xffd400, 0xff6fa5, 0xff8a00];

    #[test]
    fn the_kit_file_gives_every_token_of_both_looks() {
        let k = looks(KIT).unwrap();
        assert_eq!(k.size.base, BASE, "BASE is the kit's [size] base");
        for theme in [Theme::Day, Theme::Night] {
            // every key the file writes is one the deck reads, and nothing the deck reads is missing (no defaults)
            let read = toml::Table::try_from(Colors::of(theme)).unwrap();
            let written = compose(&[&k.file], theme).unwrap();
            let keys = |t: &toml::Table| -> Vec<String> {
                let mut all: Vec<String> = t.iter().flat_map(|(k, v)| match v {
                    toml::Value::Table(n) => n.keys().map(|n| format!("{k}.{n}")).collect(),
                    _ => vec![k.clone()],
                }).collect();
                all.sort();
                all
            };
            assert_eq!(keys(&written), keys(&read), "{theme:?}");
        }
        let sorted = |t: toml::Table| { let mut k: Vec<String> = t.keys().cloned().collect(); k.sort(); k };
        assert_eq!(sorted(k.file.size.clone()), sorted(toml::Table::try_from(&k.size).unwrap()));
    }

    #[test]
    fn a_missing_key_fails() {
        for key in ["line = \"#3a4034\"\n", "page = \"#7cc4ff\"\n", "radius = 0.36\n", "colors = [\"#5fb3ff\", \"#7fd68a\", \"#f2b84b\", \"#e07ad6\", \"#ff7a6b\", \"#9a8cff\"]
"] {
            assert!(KIT.contains(key), "{key}");
            let cut = KIT.replacen(key, "", 1);
            assert!(looks(&cut).is_err(), "without {key}");
        }
        assert!(looks(&KIT.replace("[component.tab.hover]\ntext = \"text\"", "[component.tab.hover]\ntext = \"accent\"")).is_err(), "a state naming no token");
    }

    #[test]
    fn palette_d_has_no_signal_hue_in_the_ui_parts() {
        for theme in [Theme::Day, Theme::Night] {
            let t = Tokens::new(theme, BASE);
            let c = &t.color;
            assert_eq!(t.kit("focus", "rest").ring, Some(c.text), "focus is the text colour");
            assert_eq!(t.kit("tab", "on").text, Some(c.text));
            assert_eq!((t.kit("tab", "rest").border, t.kit("tab", "on").border), (None, None), "no border at rest");
            assert_eq!(t.kit("tab", "hover").border, Some(c.line));
            assert_eq!(t.kit("button_primary", "rest").ground, Some(c.pressed), "the primary is the pressed fill");
            assert_eq!(t.kit("toggle", "on").ground, Some(c.pressed), "no green switch");
            assert_eq!(t.kit("menu", "rest").ground, Some(c.panel()));
            assert_eq!(t.kit("tab", "disabled").text, Some(c.muted), "a state is over its rest");
            assert_eq!(t.kit("chip", "ok").border, Some(c.go));
            assert_eq!(t.color.signals.job, c.warn);
            assert_eq!(t.color.signals.asset, c.muted);
        }
    }

    #[test]
    fn no_ground_is_grey() {
        // a surface is the stage colour at panel_opacity; a control's grounds the text colour see-through
        for theme in [Theme::Day, Theme::Night] {
            let c = Colors::of(theme);
            assert_eq!(c.surface[3], 1.0);
            assert!(c.panel()[3] < 1.0, "{theme:?}: the stage shows through");
            for g in [c.control, c.hover, c.selected, c.within, c.surface_head] {
                assert!(g[3] < 0.5 && g[..3] == c.text[..3], "{theme:?}: {g:?}");
            }
        }
    }

    #[test]
    fn night_reads_on_its_panels_by_the_ring_over_a_bright_and_a_dark_stage() {
        let c = Colors::night();
        assert_eq!(c.ring(), Some(c.surface));
        for stage in [BRIGHT_STAGE, DARK_STAGE] {
            let ground = on(c.panel(), stage);
            // the ring (the surface, opaque) sits under every glyph
            assert!(contrast(c.text, c.surface) >= 7.0 && contrast(c.muted, c.surface) >= 4.5);
            assert!(contrast(c.text, ground) >= 4.5, "text on the panel over {stage:?}: {}", contrast(c.text, ground));
        }
    }

    #[test]
    fn day_reads_on_its_panel_alone_over_bright_colourful_and_dark_stages_so_its_ring_stays_off() {
        // day keeps ink_ring off: the white panel at panel_opacity holds the ink over the brightest and the darkest stage
        let c = Colors::day();
        assert_eq!(c.ring(), None);
        for stage in COLOURFUL_STAGES.iter().map(|&s| rgb(s)).chain([DARK_STAGE]) {
            let panel = on(c.panel(), stage);
            assert!(contrast(c.text, panel) >= 7.0, "text over {stage:?}: {}", contrast(c.text, panel));
            assert!(contrast(c.muted, panel) >= 4.5, "muted over {stage:?}: {}", contrast(c.muted, panel));
        }
    }

    #[test]
    fn both_looks_read_in_their_fields_and_on_their_grounds() {
        for theme in [Theme::Day, Theme::Night] {
            let c = Colors::of(theme);
            let panel = c.surface;
            assert!(contrast(c.text, on(c.control, panel)) >= 7.0, "{theme:?} text in a field");
            assert!(contrast(c.muted, on(c.control, panel)) >= 4.5, "{theme:?} muted in a field");
            assert!(contrast(c.pressed_text, c.pressed) >= 7.0, "{theme:?} pressed text");
            assert!(contrast(c.text, on(c.selected, panel)) >= 4.5, "{theme:?} text on a selected item");
            assert!(contrast(c.text, on(c.within, panel)) >= 4.5, "{theme:?} text on a scrub's fill");
            assert!(contrast(c.bright, c.danger) >= 4.5, "{theme:?} light ink on danger");
            for s in [c.go, c.warn, c.stop, c.colorblind.go, c.colorblind.warn, c.colorblind.stop] {
                assert!(contrast(s, panel) >= 3.0, "{theme:?} a status colour on its panel");
            }
        }
    }

    #[test]
    fn the_colour_blind_setting_swaps_ok_warning_and_error() {
        let t = Tokens::new(Theme::Night, BASE).colorblind(true);
        let c = Colors::night();
        assert_eq!((t.color.go, t.color.warn, t.color.stop), (c.colorblind.go, c.colorblind.warn, c.colorblind.stop));
        assert_eq!(t.kit("chip", "error").text, Some(c.colorblind.stop), "the states follow");
        assert_eq!(Tokens::new(Theme::Night, BASE).colorblind(false), Tokens::new(Theme::Night, BASE));
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
        let text = "[size]\nfont = 1.25\n[color]\nline = [1.0, 0.0, 0.0, 1.0]\n[night.color]\ntext = \"#808080\"\n[night.signals]\naudio = \"#ff0000\"\n[component.tab.rest]\ntext = \"text\"\n";
        let day = Tokens::from_toml(text, Theme::Day, 16.0).unwrap();
        let night = Tokens::from_toml(text, Theme::Night, 16.0).unwrap();
        assert_eq!(day.size.font, 20.0);
        assert_eq!(day.color.line, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(night.color.line, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(day.color.text, Colors::day().text);
        assert_eq!(night.color.text, hex("#808080").unwrap());
        assert_eq!(night.color.signals.audio, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(night.color.signals.picture, Colors::night().signals.picture, "the rest of the table stays");
        assert_eq!(night.kit("tab", "rest").text, Some(night.color.text));
        assert_eq!(Tokens::from_toml("", Theme::Night, BASE).unwrap(), Tokens::new(Theme::Night, BASE));
        assert!(Tokens::from_toml("[size]\nfont = \"big\"", Theme::Day, BASE).is_err());
        assert!(Tokens::from_toml("[color]\nline = \"#12\"", Theme::Day, BASE).is_err());
    }
}
