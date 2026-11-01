//! The post stack (#204, OSCT's stackable post-processing without the plugin layer): hard-coded passes after the space
//! is drawn – the scene picks and orders them on its space node (`post = ["bloom", "curves", "pixelate", "fxaa"]`),
//! their parameters are the space node's control inputs (knobs, the feelings' nudges, the conductor's sends – like any
//! node's). This file is the data side (which passes, their knobs this frame, which of them run); the GPU side is
//! post_gpu.rs, one WGSL fragment shader per pass under shaders/post/ (no compute – the same passes run on WebGL2).
//!
//! Performance first: a pass the scene doesn't list doesn't exist (no pipeline, no texture); a listed pass whose knobs
//! stand at neutral (bloom 0, curves at identity, pixelate at 1 px without levels or palette) is skipped this frame
//! (`Chain::plan`); with nothing to run the space draws straight into its target as before. `plan` and `Knobs::read`
//! never allocate (tests/no_alloc.rs).
use super::values::Knobs as Values;
use crate::graph::{Kind, Node};
use crate::params;

/// the passes there are, in the order a scene may list them in any way
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    /// threshold, a down / up-sampled blur chain at half, quarter and eighth size (dual filter), added back
    Bloom,
    /// colour grading: lift / gamma / gain
    Curves,
    /// blocks of `pixel` render pixels, optional quantise (`levels` per channel) or a palette
    Pixelate,
    /// FXAA as a post pass (one draw)
    Fxaa,
}

/// at most one of each
pub const MAX: usize = 4;
/// a pixelate palette's colours at most (the uniform's array)
pub const PALETTE: usize = 16;

impl Effect {
    pub const ALL: [Effect; MAX] = [Effect::Bloom, Effect::Curves, Effect::Pixelate, Effect::Fxaa];

    pub fn parse(name: &str) -> Option<Effect> {
        Effect::ALL.into_iter().find(|e| e.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            Effect::Bloom => "bloom",
            Effect::Curves => "curves",
            Effect::Pixelate => "pixelate",
            Effect::Fxaa => "fxaa",
        }
    }

    /// its WGSL file under shaders/post/ (the stage's `--dev` reloads it from there)
    pub fn file(self) -> &'static str {
        match self {
            Effect::Bloom => "bloom.wgsl",
            Effect::Curves => "curves.wgsl",
            Effect::Pixelate => "pixelate.wgsl",
            Effect::Fxaa => "fxaa.wgsl",
        }
    }

    /// the built-in source (without the prelude, `COMMON`)
    pub fn source(self) -> &'static str {
        match self {
            Effect::Bloom => include_str!("../shaders/post/bloom.wgsl"),
            Effect::Curves => include_str!("../shaders/post/curves.wgsl"),
            Effect::Pixelate => include_str!("../shaders/post/pixelate.wgsl"),
            Effect::Fxaa => include_str!("../shaders/post/fxaa.wgsl"),
        }
    }
}

/// the prelude every pass is compiled with: the uniform, the input textures, the fullscreen triangle
pub const COMMON: &str = include_str!("../shaders/post/common.wgsl");

/// Where the passes' WGSL lives in this checkout (the `--dev` hot reload watches these files; a deck without the
/// sources keeps the built-in ones).
pub fn source_path(file: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shaders/post").join(file)
}

/// the space node's control inputs that are post knobs (not the being's 3D buttons unless `expose` names them)
pub const PORTS: [&str; 7] = ["bloom", "threshold", "lift", "gamma", "gain", "pixel", "levels"];

pub fn is_port(name: &str) -> bool {
    PORTS.contains(&name)
}

/// The knobs this frame: the space node's post inputs (linked, set, or the defaults of params.rs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Knobs {
    /// bloom: how much of the blur is added (0 = off), and from which brightness on (display values 0..1)
    pub bloom: f32,
    pub threshold: f32,
    /// curves: lift raises the blacks, gamma bends the middle (>1 brighter), gain scales the whites
    pub lift: f32,
    pub gamma: f32,
    pub gain: f32,
    /// pixelate: the block's size in render pixels (1 = none), quantise to `levels` per channel (< 2 = off)
    pub pixel: f32,
    pub levels: f32,
}

impl Default for Knobs {
    fn default() -> Self {
        let d = |p| params::default(Kind::Space, p);
        Knobs { bloom: d("bloom"), threshold: d("threshold"), lift: d("lift"), gamma: d("gamma"), gain: d("gain"), pixel: d("pixel"), levels: d("levels") }
    }
}

impl Knobs {
    /// the space node `node`'s post inputs from `ctl` (no allocation)
    pub fn read(ctl: &dyn Values, node: usize) -> Knobs {
        let v = |p| ctl.input(node, p, params::default(Kind::Space, p));
        Knobs { bloom: v("bloom"), threshold: v("threshold"), lift: v("lift"), gamma: v("gamma"), gain: v("gain"), pixel: v("pixel"), levels: v("levels") }
    }

    fn neutral(&self, e: Effect, palette: bool) -> bool {
        match e {
            Effect::Bloom => self.bloom <= 1e-3,
            Effect::Curves => self.lift.abs() < 1e-3 && (self.gamma - 1.0).abs() < 1e-3 && (self.gain - 1.0).abs() < 1e-3,
            Effect::Pixelate => self.pixel < 1.5 && self.levels < 2.0 && !palette,
            Effect::Fxaa => false,
        }
    }
}

/// The scene's stack: the space node's `post` list (in its order, each pass once) and pixelate's `palette`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Chain {
    effects: [Option<Effect>; MAX],
    len: usize,
    /// linear 0..1 RGB (the display values the passes see), `palette_len` of them
    pub palette: [[f32; 4]; PALETTE],
    pub palette_len: usize,
}

impl Chain {
    /// From the space node's params: `post` (names; an unknown one or a repeat is left out with a warning – the scene
    /// still plays) and `palette` (hex colours "#rrggbb", at most 16). Returns the chain and what was left out.
    pub fn of(node: &Node) -> (Chain, Vec<String>) {
        let mut c = Chain::default();
        let mut problems = Vec::new();
        for name in node.strs("post") {
            match Effect::parse(&name) {
                Some(e) if c.effects().any(|x| x == e) => problems.push(format!("post: '{name}' twice")),
                Some(e) => {
                    c.effects[c.len] = Some(e);
                    c.len += 1;
                }
                None => problems.push(format!("post: unknown pass '{name}' (bloom, curves, pixelate, fxaa)")),
            }
        }
        for hex in node.strs("palette") {
            if c.palette_len == PALETTE {
                problems.push(format!("palette: more than {PALETTE} colours"));
                break;
            }
            match parse_hex(&hex) {
                Some(rgb) => {
                    c.palette[c.palette_len] = [rgb[0], rgb[1], rgb[2], 1.0];
                    c.palette_len += 1;
                }
                None => problems.push(format!("palette: '{hex}' is not #rrggbb")),
            }
        }
        (c, problems)
    }

    /// a chain of these passes (tests, the browser)
    pub fn new(list: &[Effect]) -> Chain {
        let mut c = Chain::default();
        for &e in list.iter().take(MAX) {
            if !c.effects().any(|x| x == e) {
                c.effects[c.len] = Some(e);
                c.len += 1;
            }
        }
        c
    }

    /// the listed passes, in order
    pub fn effects(&self) -> impl Iterator<Item = Effect> + '_ {
        self.effects[..self.len].iter().flatten().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn contains(&self, e: Effect) -> bool {
        self.effects().any(|x| x == e)
    }

    /// This frame's passes: the listed ones whose knobs aren't neutral, in order (no allocation).
    pub fn plan(&self, k: &Knobs) -> Plan {
        self.plan_quality(k, true, true)
    }

    /// `plan` at a quality rung (#220, quality.rs): bloom and FXAA only where the rung allows them – curves and pixelate
    /// are the scene's look and cost one cheap pass, they always run.
    pub fn plan_quality(&self, k: &Knobs, bloom: bool, fxaa: bool) -> Plan {
        let mut p = Plan::default();
        for e in self.effects() {
            let allowed = match e {
                Effect::Bloom => bloom,
                Effect::Fxaa => fxaa,
                Effect::Curves | Effect::Pixelate => true,
            };
            if allowed && !k.neutral(e, self.palette_len > 0) {
                p.run[p.len] = e;
                p.len += 1;
            }
        }
        p
    }
}

/// the passes that run this frame
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    run: [Effect; MAX],
    len: usize,
}

impl Default for Plan {
    fn default() -> Self {
        Plan { run: [Effect::Bloom; MAX], len: 0 }
    }
}

impl Plan {
    pub fn passes(&self) -> &[Effect] {
        &self.run[..self.len]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// "#rrggbb" (or "rrggbb") → 0..1
fn parse_hex(s: &str) -> Option<[f32; 3]> {
    let h = s.strip_prefix('#').unwrap_or(s);
    if h.len() != 6 || !h.is_ascii() {
        return None;
    }
    let ch = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok().map(|v| v as f32 / 255.0);
    Some([ch(0)?, ch(2)?, ch(4)?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::scene::Scene;

    fn space(params: &str) -> Graph {
        let text = format!("[[node]]\nid=\"space\"\nkind=\"space\"\n{params}\n[[node]]\nid=\"screen\"\nkind=\"screen\"\n[[link]]\nfrom=\"space\"\nto=\"screen.in\"\n");
        Graph::build(&Scene::parse(&text).unwrap()).unwrap()
    }

    #[test]
    fn the_scene_picks_and_orders_the_passes() {
        let g = space("post = [\"fxaa\", \"bloom\", \"glitter\", \"bloom\", \"pixelate\"]\npalette = [\"#ff0000\", \"00ff00\", \"#nope\"]");
        let (c, problems) = Chain::of(&g.nodes[0]);
        assert_eq!(c.effects().collect::<Vec<_>>(), [Effect::Fxaa, Effect::Bloom, Effect::Pixelate]);
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert_eq!(c.palette_len, 2);
        assert_eq!(c.palette[1], [0.0, 1.0, 0.0, 1.0]);
        let (none, problems) = Chain::of(&space("").nodes[0]);
        assert!(none.is_empty() && problems.is_empty());
    }

    #[test]
    fn neutral_knobs_skip_their_pass() {
        let c = Chain::new(&Effect::ALL);
        let mut k = Knobs::default();
        // the defaults: bloom on, curves at identity, no pixelation, FXAA always
        assert_eq!(c.plan(&k).passes(), [Effect::Bloom, Effect::Fxaa]);
        (k.bloom, k.gamma, k.pixel) = (0.0, 1.2, 4.0);
        assert_eq!(c.plan(&k).passes(), [Effect::Curves, Effect::Pixelate, Effect::Fxaa]);
        k.pixel = 1.0;
        k.levels = 4.0;
        assert_eq!(c.plan(&k).passes(), [Effect::Curves, Effect::Pixelate, Effect::Fxaa]);
        let mut pal = Chain::new(&[Effect::Pixelate]);
        k.levels = 0.0;
        assert!(pal.plan(&k).is_empty());
        pal.palette_len = 1;
        assert_eq!(pal.plan(&k).passes(), [Effect::Pixelate]);
        assert!(Chain::default().plan(&k).is_empty());
    }

    #[test]
    fn a_low_rung_drops_bloom_and_fxaa_but_keeps_the_look() {
        let c = Chain::new(&Effect::ALL);
        let k = Knobs { gamma: 1.2, ..Knobs::default() };
        assert_eq!(c.plan_quality(&k, true, true).passes(), [Effect::Bloom, Effect::Curves, Effect::Fxaa]);
        assert_eq!(c.plan_quality(&k, false, true).passes(), [Effect::Curves, Effect::Fxaa]);
        assert_eq!(c.plan_quality(&k, false, false).passes(), [Effect::Curves]);
        let lowest = crate::quality::LADDER[0];
        assert!(c.plan_quality(&Knobs::default(), lowest.bloom, lowest.fxaa).is_empty(), "the lowest rung: the space straight into its target");
    }

    #[test]
    fn the_knobs_are_control_inputs() {
        let g = space("post = [\"bloom\"]\nbloom = 1.4\ngamma = 0.8");
        let ctl = crate::space::values::Table::new(&g);
        let k = Knobs::read(&ctl, 0);
        assert_eq!((k.bloom, k.gamma, k.gain), (1.4, 0.8, 1.0));
        for p in PORTS {
            assert!(Kind::Space.input(p).is_some(), "{p}");
        }
    }

    #[test]
    fn every_pass_has_its_shader() {
        for e in Effect::ALL {
            assert!(e.source().contains("@fragment"), "{}", e.name());
            assert!(source_path(e.file()).exists(), "{}", e.file());
        }
    }
}
