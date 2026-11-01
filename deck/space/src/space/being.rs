//! The being (#188 phases 2–3): a deck's running graph as a floating body – a translucent core (the `space` node, the
//! being's own eye, sits inside it), one node point per graph node, a hose per link (a gently curved tube) and thin
//! tendrils from the core to every point. Water bubbles run along a hose when its value changes: in the data's
//! direction, more and bigger the more it changes (control links: the change against the value's recent range; audio
//! links: the block peak of the source's output buffer; video links: a steady trickle). Every point carries its node's
//! control inputs as 3D buttons (crate::params ranges); a button on a linked input shows the hose's value and can't be
//! turned. The body drifts slowly and breathes on the tempo clock.
//!
//! **The head** (phase 5, owner 2026-10-30: "the portrait becomes the being's head … the interface between organic and
//! synthetic", Ghost in the Shell): the character's portrait mood sprite on a curved face plate over a dark machine
//! layer, inside a translucent synthetic skin; cables run from the core and the body's highest node points up into
//! sockets in its neck, their bands rising into the head. The mood follows the energy (the FFT level and the tempo):
//! `look.moods` calm → wild, held a moment before it changes, cross-faded (portrait.rs loads the sprites).
//!
//! **Tethered buttons** (owner 2026-10-31: "the buttons changed their relative position to the previous point; some
//! kind of hose in the colour could connect them"): a button hangs on a tether from its node point – the core's as
//! spokes, every other node's as a chain, each button from the one before it – and its value is its distance from
//! that point (`tether_len`: low = close, high = far, clamped), so a deck turned up wide sprawls and a quiet one stays
//! compact. The tether is a thin hose in the parameter's own colour (`param_hue`; a colour control – a hue – wears
//! the colour it is set to, its node point too), thicker and brighter with the value; while the value moves bubbles
//! run along it toward the node, the node flashes and its hoses glow (what the button reaches). Feedback (owner:
//! "very poor visual feedback"): hover swells it with a thin ring, a grip lights it fully, a double click eases it
//! back to the scene's value (`reset`).
//!
//! Owner 2026-10-31 after trying /live/deck: **no numbers** (no labels, no range ends – the knob, its tether and the
//! ring are enough), **longer tethers** (easier to reach, on a phone above all), **the value at rest**: every knob
//! always wears a thin 270° ring lit up to its value (`kind::GAUGE`, brighter and wider under the hand). **A linked
//! knob turns as its base** (graph semantics: a linked input is `source · scale + base`, the knob sets the base – the
//! link's offset – and the modulation adds on top, as synths do): its ring shows the base lit, the swing the
//! modulation covers lately as a band and the value now as a tick; a mirrored being's linked knob (its base not
//! streamed) can't be turned – it wears the dashed lock and shakes. **Level and overload**: a node point glows with its
//! audio level, the head flushes red with the output limiter's gain reduction (`strain`).
//!
//! Data-driven so a being can later be saved as a preset (#191): the body is the graph + the scene's `[being]` look;
//! the colours are the space node's inputs (hue, glow). Values come through `Values` (values.rs): this deck's control
//! domain and audio meters, or another deck's stream (phase 4) – no per-node copies; `update` and `write` never
//! allocate (tests/no_alloc.rs).
use crate::graph::{Domain, Graph, Kind, PortType};
use crate::space::camera::{Frame, Ray};
use crate::space::feel::{self, Feel, Senses};
use crate::space::values::{Knobs, Path, Values};
use crate::params::{self, Spec};
use crate::tempo;
use glam::Vec3;
use serde::Deserialize;

/// bubbles per hose at most (a fixed pool – full = no new bubble)
pub const BUBBLES: usize = 12;
/// a tube's rings along its curve and vertices around a ring
pub const SEGMENTS: usize = 18;
pub const SIDES: usize = 8;
/// neck cables at most: the core's spine + the body's highest points
pub const NECK: usize = 6;
/// the head's moods from calm to wild when the scene names none (the sprites' `<char>_portrait-<mood>`)
pub const MOODS: [&str; 5] = ["neutral", "thinking", "happy", "laughing", "surprised"];
/// bubbles per button's tether
pub const TETHER_BUBBLES: usize = 4;
/// a linked button's shake (s) and a double click's ease back to the scene's value (s)
pub const SHAKE_S: f32 = 0.4;
pub const RESET_S: f32 = 0.5;
/// the at-rest ring around a knob (× its radius), and under the hand
pub const RING_REST: f32 = 1.8;
pub const RING_GRIP: f32 = 2.5;
/// a bubble's radius at most, against its tube's (never touching the walls)
pub const BUBBLE_FILL: f32 = 0.6;
/// how long a linked knob's swing band remembers the modulation's reach (s)
const SWING: f32 = 3.0;
/// how long a new mood must hold before the head shows it (s), and the cross-fade (s)
const MOOD_HOLD: f32 = 1.2;
const MOOD_FADE: f32 = 0.5;
/// #209 – the knobs face the camera: how quickly the layout follows the eye as it orbits (s), the core's spokes'
/// angle off the view axis (a crown around the core, toward the eye), a chain's least lean toward the eye (cos), and
/// the margin a knob keeps clear of the core and the head in the view (× its radius)
const FACE_S: f32 = 0.35;
const CROWN: f32 = 1.05;
const LEAN: f32 = 0.35;
const CLEAR: f32 = 1.25;
/// #224 – every knob stays reachable close up: knobs further out than KNEE of the view's half fold in toward its edge
/// (softly, keeping their order), and stay EDGE of the half off it (the corner sticks, a thumb's room)
const KNEE: f32 = 0.7;
const EDGE: f32 = 0.12;
/// #224: folded knobs' centres keep this many × the sum of their radii apart on the screen (a little air between them)
const SPREAD: f32 = 1.3;
/// #224: a knob near the eye shows at most this share of the view's narrower half as its radius (the portrait puts
/// the eye a hand from the face – the knobs in front of it would fill the picture; a phone held upright is narrow)
const MAX_SCREEN: f32 = 0.09;
const TAU: f32 = std::f32::consts::TAU;

/// The scene's `[being]` table – the body's shape (the colours are the space node's inputs).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Look {
    /// the body's radius (m)
    pub size: f32,
    /// how far it floats around its spot (m)
    pub drift: f32,
    /// how much it swells on the beat (a fraction of its size)
    pub breath: f32,
    /// each point's own small sway (a fraction of the size)
    pub wobble: f32,
    /// hose thickness × this
    pub hose: f32,
    /// bubble rate × this
    pub bubbles: f32,
    /// the parameters shown as buttons: "node" (all its inputs) or "node.port"; empty = every control input
    pub expose: Vec<String>,
    /// the head's character (its portrait sprites `<character>_portrait-<mood>`); empty = a head without a face
    pub character: String,
    /// the moods the energy walks through, calm → wild (default MOODS)
    pub moods: Vec<String>,
    /// feedback → emotion: `[[being.feel]]` rules (feel.rs)
    pub feel: Vec<feel::Rule>,
    /// emotion → sound: `[[being.react]]` nudges (feel.rs)
    pub react: Vec<feel::Nudge>,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            size: 0.5,
            drift: 0.06,
            breath: 0.035,
            wobble: 0.025,
            hose: 1.0,
            bubbles: 1.0,
            expose: Vec::new(),
            character: String::new(),
            moods: MOODS.iter().map(|m| m.to_string()).collect(),
            feel: Vec::new(),
            react: Vec::new(),
        }
    }
}

/// The head (phase 5): where it floats, its size, the energy and the mood it shows (an index into the being's `moods`
/// – `look.moods`, then the moods its feelings name –, cross-faded from `prev` by `fade`).
#[derive(Debug, Clone)]
pub struct Head {
    pub pos: Vec3,
    pub radius: f32,
    /// 0..1: the FFT level and the tempo, smoothed
    pub energy: f32,
    pub mood: usize,
    pub prev: usize,
    /// 0 = `prev` shows, 1 = `mood`
    pub fade: f32,
    /// the mood the energy asks for, and since when
    candidate: usize,
    held: f32,
    /// the portrait sprite per mood (portrait.rs slots); empty = no face
    pub sprites: Vec<usize>,
    /// per mood: the moods to show instead while its sprite is missing, nearest first (portrait::near)
    pub near: Vec<Vec<usize>>,
    /// a mood set from outside (another deck's stream says its mood): the energy doesn't pick it then
    pub fixed: Option<usize>,
}

impl Head {
    /// the neck's socket `i` of `n`: a ring under the head's centre
    pub fn socket(&self, i: usize, n: usize) -> Vec3 {
        let a = TAU * (i as f32 + 0.5) / n.max(1) as f32 + 0.4;
        self.pos + Vec3::new(a.cos() * 0.38, -0.8, a.sin() * 0.38) * self.radius
    }
}

/// where a hose's flow is read
#[derive(Debug, Clone, Copy, PartialEq)]
enum Flow {
    /// the source's control output (node, port)
    Control(usize, usize),
    /// the source's audio output buffer's peak (node, port)
    Audio(usize, usize),
    Video,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Bubble {
    /// 0..1 along the hose, from its source to its target
    pub t: f32,
    /// hose lengths per second
    pub speed: f32,
    /// 0 = free slot, else 0.3..1
    pub size: f32,
}

#[derive(Debug, Clone)]
pub struct Point {
    pub node: usize,
    pub rest: Vec3,
    /// this frame's position (world)
    pub pos: Vec3,
    pub radius: f32,
    /// hue offset from the being's hue (by the node's domain)
    pub tint: f32,
    phase: f32,
    /// 0..1: one of its buttons moves (the node flashes – what the button reaches)
    pub flash: f32,
    /// 0..1: its audio level (its outputs' and incoming audio hoses' block peaks, eased) – it glows with it
    pub level: f32,
    /// a colour control on this node (a hue button): the point wears the colour it is set to
    pub colour: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Hose {
    /// the graph link it shows
    pub link: usize,
    pub from: usize,
    pub to: usize,
    pub ty: PortType,
    pub radius: f32,
    /// the curve's four control points (world, this frame)
    pub ctrl: [Vec3; 4],
    /// 0..1, how much flows (the shader's glow)
    pub activity: f32,
    pub bubbles: [Bubble; BUBBLES],
    flow: Flow,
    last: f32,
    lo: f32,
    hi: f32,
    acc: f32,
}

#[derive(Debug, Clone)]
pub struct Button {
    pub point: usize,
    pub node: usize,
    /// the input port's index and name
    pub port: usize,
    pub name: &'static str,
    pub spec: Spec,
    /// driven by a hose: the knob is its base, the hose's modulation on top (#188)
    pub linked: bool,
    /// a hand can turn it (an unlinked one, or a linked one whose base this being's values know – a mirrored deck's
    /// linked knob doesn't: it shakes)
    pub turnable: bool,
    /// the input's value now (a linked one: the base + the modulation) and its 0..1; the swing the modulation covers
    /// lately (0..1, lo ≤ hi – an unlinked knob's is its value)
    pub live: f32,
    pub live_norm: f32,
    pub lo: f32,
    pub hi: f32,
    /// the LFO's waveform (`port` is meaningless then; the value is an index into params::LFO_SHAPES)
    pub shape: bool,
    pub pos: Vec3,
    /// its radius this frame: `base`, capped close to the eye (MAX_SCREEN, #224) – what is drawn and picked
    pub radius: f32,
    pub base: f32,
    /// this frame's value (the knob's: a linked one's base) and its 0..1
    pub value: f32,
    pub norm: f32,
    /// its place among its node's buttons, and how many the node has
    pub rank: usize,
    pub count: usize,
    /// the parameter's own colour (a hue per parameter name; the hue button shows its value)
    pub hue: f32,
    /// what it hangs from: None = its node point, Some(j) = button j before it on the same node (a chain)
    pub prev: Option<usize>,
    /// this frame's tether: its direction, where its length is measured from, where the hose ends (the node point
    /// or the button before it) and the hose's curve from the button (t 0) to that end (t 1)
    pub dir: Vec3,
    pub root: Vec3,
    pub tether: [Vec3; 4],
    /// #224: where the knob sits before it folds into the view (`pos` is where it is shown and picked): the next knob of
    /// its chain grows from here, the camera's focus goes here when it is selected
    pub free: Vec3,
    /// the 0..1 shown – eased toward `norm`, so a 30 / s stream moves it smoothly
    pub shown: f32,
    /// how fast it moves lately (0..1): the tether pulses, bubbles run, the node flashes, the label shows
    pub motion: f32,
    /// eased 0..1: the pointer over it, a hand on it
    pub hot: f32,
    pub grip: f32,
    /// a button that can't be turned grabbed: seconds of shaking left
    pub shake: f32,
    /// the scene's value (a linked one's: the link's offset) – a double click eases back to it
    pub scene: f32,
    /// a linked input's source node (its id – the HUD's `INCENSE.CUTOFF 600 < SWAY`)
    pub source: String,
    pub bubbles: [Bubble; TETHER_BUBBLES],
    acc: f32,
}

/// A double click's ease back to the scene's value (`Being::reset`): the button, from / to (0..1), how far (0..1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reset {
    pub button: usize,
    pub from: f32,
    pub to: f32,
    pub t: f32,
}

/// The parameter's own colour: a fixed hue for the common ones (each its own, apart enough to tell), a stable hash
/// of the name for the rest.
pub fn param_hue(name: &str) -> f32 {
    match name {
        "freq" => 0.0,
        "ratio" => 0.07,
        "glow" => 0.12,
        "light" => 0.17,
        "speed" | "hold" => 0.24,
        "rate" => 0.31,
        "depth" => 0.40,
        "gain" | "volume" => 0.50,
        "gain1" => 0.46,
        "gain2" => 0.53,
        "gain3" => 0.66,
        "gain4" => 0.69,
        "index" => 0.60,
        "cutoff" => 0.75,
        "zoom" | "opacity" => 0.82,
        "pulse" | "shake" => 0.90,
        _ => hash(name),
    }
}

/// a colour control (owner 2026-10-31: "controls that are colours … show their value as that colour"): a hue
pub fn is_colour(name: &str) -> bool {
    name == "hue" || name.ends_with("_hue")
}

/// a button's colour now: a colour control's value, else the parameter's own (`param_hue`)
pub fn colour_of(b: &Button) -> f32 {
    if is_colour(b.name) {
        b.value
    } else {
        b.hue
    }
}

/// a tether's (minimum, span) × the being's size: the core's spokes, a chain's links (owner 2026-10-31: "longer
/// tethers … easier to reach, on a phone above all" – ~1.5× the first version's, the silhouette still reads)
const CORE_TETHER: (f32, f32) = (0.14, 0.48);
const CHAIN_TETHER: (f32, f32) = (0.10, 0.32);

/// A tether's length (m) at 0..1 for a being of `size`: low = close, high = far – clamped, so a body turned up wide
/// still reads. The core's spokes reach a little further than a chain's links.
pub fn tether_len(size: f32, core: bool, n: f32) -> f32 {
    let (min, span) = if core { CORE_TETHER } else { CHAIN_TETHER };
    size * (min + span * n.clamp(0.0, 1.0))
}

/// the 0..1 a tether of length `len` shows (`tether_len` backwards, clamped)
pub fn tether_norm(size: f32, core: bool, len: f32) -> f32 {
    let (min, span) = if core { CORE_TETHER } else { CHAIN_TETHER };
    ((len / size.max(1e-6) - min) / span).clamp(0.0, 1.0)
}

/// a tether's tube radius at the value 0..1 shown (m)
pub fn tether_radius(size: f32, hose: f32, shown: f32) -> f32 {
    size * (0.005 + 0.011 * shown) * hose.max(0.2)
}

/// what the pointer is over
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    Button(usize),
    Point(usize),
}

/// one sphere for the GPU (48 bytes): position, radius, colour (rgb + alpha), extra = (kind, glow, hover, value)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Sphere {
    pub pos: [f32; 3],
    pub radius: f32,
    pub color: [f32; 4],
    pub extra: [f32; 4],
}

/// the sphere kinds (extra[0])
pub mod kind {
    pub const NUCLEUS: f32 = 0.0;
    pub const BUTTON: f32 = 1.0;
    pub const SHELL: f32 = 2.0;
    pub const BUBBLE: f32 = 3.0;
    pub const CORE: f32 = 4.0;
    /// the head's translucent synthetic skin
    pub const SKIN: f32 = 5.0;
    /// a thin ring around a button, facing the eye: extra.w 0 = hover, 1 = dashed (it can't be turned – locked)
    pub const RING: f32 = 6.0;
    /// every button's value ring (#188, always on – the value at rest): a 270° arc facing the eye lit up to extra.w
    /// (the knob's 0..1), the swing band extra.y..extra.z (a linked knob's modulation lately), the value now as a tick
    /// at (extra.x − 7) / 0.9; colour alpha 0.5 at rest → 1 under the hand (thin → wide)
    pub const GAUGE: f32 = 7.0;
}

/// one tube vertex (40 bytes): position, normal, t along the tube, its flow (activity), hue tint, kind (0 hose,
/// 1 tendril, 2 neck cable, 3 a hose between beings, 4 + 0.9 × motion a button's tether – its flow = its value, its
/// tint the parameter's own hue, not the being's)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TubeVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub t: f32,
    pub flow: f32,
    pub tint: f32,
    pub kind: f32,
}

pub struct Being {
    pub look: Look,
    pub points: Vec<Point>,
    pub hoses: Vec<Hose>,
    pub buttons: Vec<Button>,
    /// the point of the space node (the core's nucleus), if the graph has one
    pub core_point: Option<usize>,
    /// where it floats (world) – the spot + lift
    pub anchor: Vec3,
    /// this frame's centre (anchor + drift) and swell
    pub center: Vec3,
    pub swell: f32,
    pub core_radius: f32,
    /// the being's colours this frame (the space node's hue / glow)
    pub hue: f32,
    pub glow: f32,
    /// the space node's `pulse` this frame (the level's kick – the core's light swells with it, #204)
    pub pulse: f32,
    /// the hovered / dragged part (the renderer lights it)
    pub hover: Option<Hit>,
    /// #209: the selection the sticks drive (the last tapped): a button, or `Hit::Point` – the whole being (any of its
    /// points or its head); the renderer rings it until something else is tapped
    pub selected: Option<Hit>,
    /// #209: where the camera's eye is (world; the space sets it every frame) – the knobs turn toward it; None = the
    /// fixed layout (no camera: the tests, a headless check)
    pub eye: Option<Vec3>,
    /// #224: the camera's view (the space sets it every frame) – the knobs fold into it close up; None = no folding
    pub frame: Option<Frame>,
    /// the way to the eye from the centre, eased (FACE_S) – what the knobs' layout follows
    pub face: Vec3,
    /// `face` has been set once (the first frame with an eye takes it at once)
    faced: bool,
    pub dragging: bool,
    pub head: Head,
    /// the points whose cables run up into the neck (the core first, then the highest points)
    pub neck: Vec<usize>,
    /// the FFT node (the head's energy)
    fft: Option<usize>,
    /// the head's palette: `look.moods` (the energy's ladder) first, then every other mood the feelings name
    pub moods: Vec<String>,
    /// the feelings: rules that pick the mood, nudges that push it back into the graph (feel.rs)
    pub feel: Feel,
    /// a double click's ease back to the scene's value, pushed into the graph by `push`
    pub reset: Option<Reset>,
    /// 0..1: how hard the output's limiter works (its gain reduction, eased – `strain`): the head's red flush (#188)
    pub strain: f32,
    /// the last frame's step (s) – `push` eases with it
    dt: f32,
}

impl Being {
    pub fn new(graph: &Graph, look: &Look) -> Being {
        let s = look.size.max(0.05);
        let n = graph.nodes.len();
        // depth along the ordering links: data rises from the sources (low) to the sinks (high, toward the head)
        let mut depth = vec![0usize; n];
        for &i in &graph.order {
            for l in graph.links.iter().filter(|l| l.from.0 == i && l.orders(&graph.nodes)) {
                depth[l.to.0] = depth[l.to.0].max(depth[i] + 1);
            }
        }
        let max_depth = depth.iter().copied().max().unwrap_or(0).max(1) as f32;
        let core_node = graph.first(Kind::Space);
        let mut points = Vec::with_capacity(n);
        let mut k = 0usize;
        for &i in &graph.order {
            let node = &graph.nodes[i];
            let h = hash(&node.id);
            let (rest, radius) = if Some(i) == core_node {
                (Vec3::ZERO, s * 0.12)
            } else {
                let y = -0.85 + 1.6 * depth[i] as f32 / max_depth + (h - 0.5) * 0.12;
                let ring = 0.62 + 0.38 * (1.0 - y * y).max(0.0).sqrt();
                let a = k as f32 * 2.399_963 + h * 0.6; // the golden angle, a little jitter
                k += 1;
                let r = match node.kind.domain() {
                    Domain::Audio => 0.095,
                    Domain::Video => 0.09,
                    Domain::Control => 0.08,
                };
                (Vec3::new(ring * a.cos(), y, ring * a.sin()) * s, s * r)
            };
            let tint = match node.kind.domain() {
                _ if Some(i) == core_node => 0.0,
                Domain::Audio => 0.36,
                Domain::Video => 0.0,
                Domain::Control => -0.18,
            };
            points.push(Point { node: i, rest, pos: rest, radius, tint, phase: h * TAU, flash: 0.0, level: 0.0, colour: None });
        }
        let point_of = |node: usize| points.iter().position(|p| p.node == node).expect("a point per node");
        let hoses = graph
            .links
            .iter()
            .enumerate()
            .map(|(li, l)| {
                let (flow, r) = match l.ty {
                    PortType::Control => (Flow::Control(l.from.0, l.from.1), 0.016),
                    PortType::Audio => (Flow::Audio(l.from.0, l.from.1), 0.028),
                    PortType::Video => (Flow::Video, 0.026),
                };
                Hose {
                    link: li,
                    from: point_of(l.from.0),
                    to: point_of(l.to.0),
                    ty: l.ty,
                    radius: s * r * look.hose,
                    ctrl: [Vec3::ZERO; 4],
                    activity: 0.0,
                    bubbles: [Bubble::default(); BUBBLES],
                    flow,
                    last: f32::NAN,
                    lo: 0.0,
                    hi: 0.0,
                    acc: 0.0,
                }
            })
            .collect();
        let mut buttons = Vec::new();
        for (pi, p) in points.iter().enumerate() {
            let node = &graph.nodes[p.node];
            let mut ports: Vec<usize> = node
                .kind
                .inputs()
                .iter()
                .enumerate()
                .filter(|(_, port)| port.ty == PortType::Control && exposed(&look.expose, &node.id, port.name))
                // the post stack's knobs (#204) are no buttons on the core unless `expose` names them
                .filter(|(_, port)| !(node.kind == Kind::Space && super::post::is_port(port.name)) || named(&look.expose, &node.id, port.name))
                .map(|(k, _)| k)
                .collect();
            // the waveform: an LFO's shape (an audio oscillator's `wave` can't change while it plays yet)
            let shape = node.kind == Kind::Lfo && exposed(&look.expose, &node.id, "shape");
            if shape {
                ports.push(usize::MAX);
            }
            let count = ports.len();
            let core = Some(p.node) == core_node;
            for (rank, &k) in ports.iter().enumerate() {
                let (name, spec, linked, scene) = if k == usize::MAX {
                    let names = crate::params::LFO_SHAPES;
                    let scene = node.str("shape").and_then(|n| names.iter().position(|x| *x == n)).unwrap_or(0) as f32;
                    ("shape", Spec { min: 0.0, max: (names.len() - 1) as f32, default: 0.0, curve: params::Curve::Lin }, false, scene)
                } else {
                    let name = node.kind.inputs()[k].name;
                    let spec = params::spec(node.kind, name);
                    // a linked knob's scene value is its base: the link's offset
                    match graph.link_into(p.node, k) {
                        Some(l) => (name, spec, true, l.offset),
                        None => (name, spec, false, node.f32(name, spec.default)),
                    }
                };
                let source = graph.link_into(p.node, k).map(|l| graph.nodes[l.from.0].id.clone()).unwrap_or_default();
                let n0 = spec.norm(scene);
                buttons.push(Button {
                    point: pi,
                    node: p.node,
                    port: k,
                    name,
                    spec,
                    linked,
                    turnable: !linked,
                    live: scene,
                    live_norm: n0,
                    lo: n0,
                    hi: n0,
                    shape: k == usize::MAX,
                    pos: p.rest,
                    radius: s * 0.058,
                    base: s * 0.058,
                    value: scene,
                    norm: n0,
                    rank,
                    count,
                    hue: param_hue(name),
                    // the core's buttons are spokes around it; any other node's hang one from the other
                    prev: (rank > 0 && !core).then(|| buttons.len() - 1),
                    dir: Vec3::Y,
                    root: p.rest,
                    tether: [p.rest; 4],
                    free: p.rest,
                    shown: spec.norm(scene),
                    motion: 0.0,
                    hot: 0.0,
                    grip: 0.0,
                    shake: 0.0,
                    scene,
                    source,
                    bubbles: [Bubble::default(); TETHER_BUBBLES],
                    acc: 0.0,
                });
            }
        }
        let core_point = core_node.map(point_of);
        // a colour control's node point wears the colour it is set to
        for (i, b) in buttons.iter().enumerate() {
            if is_colour(b.name) && points[b.point].colour.is_none() {
                points[b.point].colour = Some(i);
            }
        }
        // the neck: the core's spine, then the points nearest the head (the sinks – where the data has risen to)
        let mut by_height: Vec<usize> = (0..points.len()).filter(|&i| Some(i) != core_point).collect();
        by_height.sort_by(|&a, &b| points[b].rest.y.total_cmp(&points[a].rest.y));
        let neck: Vec<usize> = core_point.into_iter().chain(by_height).take(NECK).collect();
        let mut moods = look.moods.clone();
        let feel = Feel::new(graph, &look.feel, &look.react, &mut moods, (hash(&look.character) * 16_777_216.0) as u32 ^ 0x9e37_79b9);
        Being {
            look: look.clone(),
            points,
            hoses,
            buttons,
            core_point,
            anchor: Vec3::ZERO,
            center: Vec3::ZERO,
            swell: 1.0,
            core_radius: s * 0.3,
            hue: params::default(Kind::Space, "hue"),
            glow: params::default(Kind::Space, "glow"),
            pulse: 0.0,
            hover: None,
            selected: None,
            eye: None,
            frame: None,
            face: Vec3::Z,
            faced: false,
            dragging: false,
            head: Head {
                pos: Vec3::ZERO,
                radius: s * 0.55,
                energy: 0.0,
                mood: 0,
                prev: 0,
                fade: 1.0,
                candidate: 0,
                held: 0.0,
                sprites: Vec::new(),
                near: super::portrait::near(&moods),
                fixed: None,
            },
            neck,
            fft: graph.first(Kind::Fft),
            moods,
            feel,
            reset: None,
            strain: 0.0,
            dt: 0.0,
        }
    }

    /// The output limiter's gain reduction this frame (dB, the deepest since the last frame): `strain` rises at once
    /// (full at 9 dB) and eases off over a second – the head's red flush, the feelings' `strain` (#188, owner
    /// 2026-10-31: "when the output distorts … the head and avatar turn red-faced").
    pub fn strain(&mut self, dt: f32, reduction_db: f32) {
        let target = (reduction_db / 9.0).clamp(0.0, 1.0);
        let tau = if target > self.strain { 0.05 } else { 0.8 };
        self.strain += (target - self.strain) * (dt / tau).min(1.0);
    }

    /// Where the camera aims (camera::focus_target, owner 2026-10-31: "like a photographer"): the whole body's centre –
    /// the bounds of its node points' rest places and its head, around the anchor (not the drift, not the knobs: the view
    /// holds still while a hand turns them) – and the head's.
    pub fn focus(&self) -> (Vec3, Vec3) {
        let head = self.anchor + Vec3::Y * self.look.size * 1.3;
        let r = Vec3::splat(self.head.radius);
        let (mut lo, mut hi) = (head - r, head + r);
        for p in &self.points {
            let q = self.anchor + p.rest;
            (lo, hi) = (lo.min(q - Vec3::splat(p.radius)), hi.max(q + Vec3::splat(p.radius)));
        }
        ((lo + hi) * 0.5, head)
    }

    /// the core's light this frame (#204): the glow and the level's kick on it
    pub fn light_power(&self) -> f32 {
        (self.glow + self.pulse).max(0.0)
    }

    /// the core's light colour: its hue, a little more saturated than the core looks (linear 0..1)
    pub fn light_rgb(&self) -> [f32; 3] {
        hsv(self.hue, 0.7, 1.0)
    }

    /// where the head floats: above the body's top
    pub fn head(&self) -> Vec3 {
        self.center + Vec3::Y * self.look.size * 1.3 * self.swell
    }

    /// One frame: the body's pose (drift, breath, sway), the hoses' curves and bubbles, the buttons' values, the head's
    /// energy and mood. Never allocates.
    pub fn update(&mut self, dt: f32, time: f32, vals: &dyn Values, beat: &tempo::Now) {
        let look = &self.look;
        let s = look.size;
        if let Some(c) = self.core_point {
            let node = self.points[c].node;
            self.hue = vals.input(node, "hue", params::default(Kind::Space, "hue"));
            self.glow = vals.input(node, "glow", params::default(Kind::Space, "glow"));
        }
        // breath: a swell on each beat and a slow wave over the bar; without a running clock a calm 0.2 Hz
        let b = if beat.running {
            let on = (1.0 - beat.pos.phase as f32).powi(3);
            0.6 * on + 0.4 * (0.5 - 0.5 * (TAU * beat.pos.bar_phase as f32).cos())
        } else {
            0.5 - 0.5 * (TAU * 0.2 * time).cos()
        };
        let pulse = self.core_point.map_or(0.0, |c| vals.input(self.points[c].node, "pulse", 0.0));
        self.pulse = pulse;
        self.swell = 1.0 + look.breath * (b + 0.5 * pulse);
        let d = look.drift;
        self.center = self.anchor + Vec3::new(d * (0.13 * time).sin(), 0.6 * d * (0.21 * time + 1.0).sin(), d * (0.17 * time + 2.0).sin());
        for p in &mut self.points {
            let w = look.wobble * s;
            let ph = p.phase;
            let sway = Vec3::new((1.3 * time + ph).sin(), (1.7 * time + 2.0 * ph).sin(), (1.1 * time + 3.0 * ph).sin()) * w;
            p.pos = self.center + p.rest * self.swell + if p.rest == Vec3::ZERO { Vec3::ZERO } else { sway };
        }
        let center = self.center;
        // the points' audio level: up at once with an audio hose's peak at either end, down over ~0.3 s
        let fall = (-dt / 0.3).exp();
        for p in &mut self.points {
            p.level *= fall;
        }
        for h in &mut self.hoses {
            let (a, b) = (self.points[h.from].pos, self.points[h.to].pos);
            let out = |p: Vec3| (p - center).try_normalize().unwrap_or(Vec3::Y);
            let bow = 0.35 * (b - a).length() + 0.1 * s;
            h.ctrl = [a, a + out(a) * bow, b + out(b) * bow, b];
            // how much changes → bubbles per second and their size
            let (rate, size) = match h.flow {
                Flow::Control(n, p) => {
                    let v = vals.output(n, p);
                    if h.last.is_nan() {
                        (h.last, h.lo, h.hi) = (v, v, v);
                    }
                    let change = (v - h.last).abs();
                    h.last = v;
                    // the recent range: min / max easing toward the value (~4 s)
                    let k = (dt / 4.0).min(1.0);
                    h.hi = (h.hi + (v - h.hi) * k).max(v);
                    h.lo = (h.lo + (v - h.lo) * k).min(v);
                    let rel = change / (h.hi - h.lo).max(1e-3);
                    ((rel / dt.max(1e-3) * 2.5).min(14.0), 0.35 + 0.65 * (rel * 6.0).min(1.0))
                }
                Flow::Audio(n, p) => {
                    let level = vals.meter(n, p).clamp(0.0, 1.0);
                    for at in [h.from, h.to] {
                        let pt = &mut self.points[at];
                        pt.level = pt.level.max(level.sqrt());
                    }
                    (level.sqrt() * 9.0, 0.3 + 0.7 * level.sqrt())
                }
                Flow::Video => (1.5, 0.35),
            };
            let rate = rate * look.bubbles;
            h.activity += ((rate / 8.0).min(1.0) - h.activity) * (dt * 3.0).min(1.0);
            h.acc += rate * dt;
            let speed = 0.35 + 0.5 * (rate / 10.0).min(1.0);
            for b in h.bubbles.iter_mut().filter(|b| b.size > 0.0) {
                b.t += b.speed * dt;
                if b.t >= 1.0 {
                    b.size = 0.0;
                }
            }
            while h.acc >= 1.0 {
                h.acc -= 1.0;
                if let Some(free) = h.bubbles.iter_mut().find(|b| b.size == 0.0) {
                    *free = Bubble { t: 0.0, speed, size };
                }
            }
            h.acc = h.acc.min(2.0);
        }
        self.dt = dt;
        self.step_buttons(dt, vals);
        self.step_head(dt, vals, beat);
    }

    /// The buttons this frame: their values (a linked one rides its source), the tethers (each button at its value's
    /// distance from the point it hangs from), hover / grip / shake eased, how fast each moves (bubbles along the
    /// tether, the node's flash). Never allocates.
    fn step_buttons(&mut self, dt: f32, vals: &dyn Values) {
        let s = self.look.size;
        let center = self.center;
        let hovered = match self.hover {
            Some(Hit::Button(i)) => Some(i),
            _ => None,
        };
        let ease = |x: &mut f32, to: f32, tau: f32| *x += (to - *x) * (dt / tau).min(1.0);
        for p in &mut self.points {
            p.flash = (p.flash - dt * 2.5).max(0.0);
        }
        // #209: the knobs face the camera – the way to the eye, eased, so the layout re-fans as the camera orbits (the
        // first frame takes it at once); what the knobs keep clear of in the view: the core and the head
        let eye = self.eye;
        let facing = eye.and_then(|e| (e - center).try_normalize()).map(|want| {
            let k = if self.faced && dt > 0.0 { (dt / FACE_S).min(1.0) } else { 1.0 };
            (self.face + (want - self.face) * k).try_normalize().unwrap_or(want)
        });
        if let Some(f) = facing {
            (self.face, self.faced) = (f, true);
        }
        let shadows = [(center, self.core_radius * self.swell), (self.head(), self.head.radius)];
        let frame = self.frame;
        for i in 0..self.buttons.len() {
            // the chain's knob before: where it grows from (unfolded), where it is shown (the tether's end)
            let prev = self.buttons[i].prev.map(|j| (self.buttons[j].free, self.buttons[j].pos));
            let b = &mut self.buttons[i];
            let p = &self.points[b.point];
            // the knob: an unlinked input's value, a linked one's base (the modulation rides on top – `live`)
            b.live = if b.shape { vals.lfo_shape(b.node).unwrap_or(0) as f32 } else { vals.input(b.node, b.name, b.spec.default) };
            let base = if b.linked { vals.base(b.node, b.port) } else { None };
            b.turnable = !b.linked || base.is_some();
            b.value = base.unwrap_or(b.live);
            b.norm = b.spec.norm(b.value);
            b.live_norm = b.spec.norm(b.live);
            // the swing band: where the modulation reached lately (its edges ease back over SWING s)
            if b.linked {
                let k = (dt / SWING).min(1.0);
                b.lo = (b.lo + (b.live_norm - b.lo) * k).min(b.live_norm).min(b.norm);
                b.hi = (b.hi + (b.live_norm - b.hi) * k).max(b.live_norm).max(b.norm);
            } else {
                (b.lo, b.hi) = (b.norm, b.norm);
            }
            // a hand on it – one that can't be turned only shakes
            let gripped = self.dragging && hovered == Some(i) && b.turnable;
            ease(&mut b.hot, if hovered == Some(i) { 1.0 } else { 0.0 }, 0.08);
            ease(&mut b.grip, if gripped { 1.0 } else { 0.0 }, 0.06);
            // shown follows the value: at once under the hand, eased otherwise (a stream's steps, a reset's glide)
            let before = b.shown;
            if gripped || dt <= 0.0 {
                b.shown = b.norm;
            } else {
                ease(&mut b.shown, b.norm, 0.07);
            }
            let speed = if dt > 0.0 { (b.shown - before).abs() / dt } else { 0.0 };
            let target = (speed * 2.5).min(1.0);
            let tau = if target > b.motion { 0.04 } else { 0.35 };
            ease(&mut b.motion, target, tau);
            b.shake = (b.shake - dt).max(0.0);
            // the tether: the core's spokes around its equator, any other node's chain curling out of its point
            let core = Some(b.point) == self.core_point;
            let out = (p.pos - center).try_normalize().unwrap_or(Vec3::Y);
            // (#209: the core's spokes as a crown around the view axis, a chain leaning toward the eye)
            let (dir, root, end) = if core {
                let a = TAU * b.rank as f32 / b.count.max(1) as f32;
                let dir = match facing {
                    Some(f) => crown(f, a),
                    None => Vec3::new(a.cos(), 0.12, a.sin()).normalize(),
                };
                (dir, p.pos + dir * self.core_radius * self.swell, p.pos)
            } else {
                let u = out.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
                let v = out.cross(u);
                let a = p.phase + b.rank as f32 * 1.1;
                let dir = (out + (u * a.cos() + v * a.sin()) * 0.65).normalize();
                let dir = facing.map_or(dir, |f| lean(dir, out, f));
                match prev {
                    Some((from, shown)) => (dir, from, shown),
                    None => (dir, p.pos + dir * p.radius, p.pos),
                }
            };
            let len = tether_len(s, core, b.shown);
            // a linked button grabbed shakes across its tether
            let shake = if b.shake > 0.0 {
                let k = b.shake / SHAKE_S;
                dir.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X) * (b.shake * 75.0).sin() * k * b.base * 0.9
            } else {
                Vec3::ZERO
            };
            b.dir = dir;
            b.root = root;
            b.pos = root + dir * len + shake;
            // #209: never hidden behind the core or the head – out of their shadow, sideways in the view
            // (a few rounds: a knob pushed out of the head's shadow may land in the core's, seen from high above)
            if let Some(e) = eye {
                for _ in 0..3 {
                    for (c, r) in shadows {
                        b.pos = clear_of(b.pos, e, c, r, b.base);
                    }
                }
            }
            // #224: close up, the knobs out of the picture fold in toward its edge – the tether runs out to its node
            b.free = b.pos;
            b.radius = b.base;
            if let Some(f) = &frame {
                b.pos = fold(b.pos, f, b.base);
                b.radius = b.base.min((b.pos - f.eye).dot(f.fwd) * f.tan_x.min(f.tan_y) * MAX_SCREEN);
            }
            b.tether = tether(b.pos, end);
            // bubbles toward the node while it moves
            for x in b.bubbles.iter_mut().filter(|x| x.size > 0.0) {
                x.t += x.speed * dt;
                if x.t >= 1.0 {
                    x.size = 0.0;
                }
            }
            b.acc = (b.acc + b.motion * 12.0 * dt).min(2.0);
            while b.acc >= 1.0 {
                b.acc -= 1.0;
                if let Some(free) = b.bubbles.iter_mut().find(|x| x.size == 0.0) {
                    *free = Bubble { t: 0.0, speed: 1.6, size: 0.4 + 0.6 * b.motion };
                }
            }
            // what a hand (or a feeling's nudge, a reset) moves flashes its node – a linked knob's base too; the
            // modulation riding its hose doesn't (the hose's bubbles show that already)
            let flash = b.motion.max(b.grip * 0.5);
            let pt = &mut self.points[b.point];
            pt.flash = pt.flash.max(flash);
        }
        if let Some(f) = &frame {
            self.spread(f);
        }
    }

    /// #224: the knobs folded into the view crowd its edge – a few rounds pushing apart those that overlap on the screen
    /// (in the view's tangents, SPREAD × the sum of their radii apart, each of a folded pair half the way), kept inside the fold's
    /// frame; then the tethers to where they are now. Only folded knobs move; fixed rounds, so it is smooth frame to
    /// frame. Never allocates.
    fn spread(&mut self, f: &Frame) {
        let n = self.buttons.len();
        let at = |b: &Button| {
            let q = b.pos - f.eye;
            let z = q.dot(f.fwd).max(1e-4);
            (q.dot(f.right) / z, q.dot(f.up) / z, z, b.radius / z)
        };
        for _ in 0..12 {
            for i in 0..n {
                if self.buttons[i].pos == self.buttons[i].free {
                    continue;
                }
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    let (xi, yi, zi, ri) = at(&self.buttons[i]);
                    let (xj, yj, _, rj) = at(&self.buttons[j]);
                    let (dx, dy) = (xi - xj, yi - yj);
                    let d = (dx * dx + dy * dy).sqrt();
                    let need = (ri + rj) * SPREAD;
                    if d >= need {
                        continue;
                    }
                    // apart along their line (a fixed way when they coincide); the whole step if j stays where it is
                    let (ux, uy) = if d > 1e-6 { (dx / d, dy / d) } else if i < j { (0.0, 1.0) } else { (0.0, -1.0) };
                    let share = if self.buttons[j].pos == self.buttons[j].free { 1.0 } else { 0.5 };
                    let (x, y) = inside(xi + ux * (need - d) * share, yi + uy * (need - d) * share, f, self.buttons[i].radius / zi * CLEAR);
                    self.buttons[i].pos = f.eye + (f.fwd + f.right * x + f.up * y) * zi;
                }
            }
        }
        for i in 0..n {
            let end = match self.buttons[i].prev {
                Some(j) => self.buttons[j].pos,
                None => self.points[self.buttons[i].point].pos,
            };
            self.buttons[i].tether = tether(self.buttons[i].pos, end);
        }
    }

    /// A button that can't be turned grabbed (a mirrored deck's linked knob): it shakes.
    pub fn shake(&mut self, i: usize) {
        if let Some(b) = self.buttons.get_mut(i) {
            b.shake = SHAKE_S;
        }
    }

    /// A double click: button `i` eases back to the scene's value over RESET_S (`push` sets it) – a linked knob to the
    /// link's offset; one that can't be turned doesn't.
    pub fn reset(&mut self, i: usize) {
        let Some(b) = self.buttons.get(i).filter(|b| b.turnable) else { return };
        self.reset = Some(Reset { button: i, from: b.norm, to: b.spec.norm(b.scene), t: 0.0 });
    }

    /// The head's place, its energy (the FFT level – this deck's sound – and the tempo: a fast clock is a little
    /// wilder) and its mood: a feeling's rule that fires shows its mood at once (feel.rs); else a fixed mood (another
    /// deck's stream says it) shows at once; else the energy's mood, once it has held MOOD_HOLD. Each turn cross-fades
    /// and kicks the new mood's nudges.
    fn step_head(&mut self, dt: f32, vals: &dyn Values, beat: &tempo::Now) {
        let pos = self.head();
        let felt = self.feel.step(dt, vals, &Senses { energy: self.head.energy, touching: self.dragging, beat, strain: self.strain });
        let h = &mut self.head;
        h.pos = pos;
        let level = self.fft.map_or(0.0, |n| vals.output(n, 0)).clamp(0.0, 1.0);
        let bpm = if beat.running { beat.bpm as f32 } else { 0.0 };
        let target = (0.8 * (level * 1.3).min(1.0) + 0.2 * ((bpm - 60.0) / 100.0).clamp(0.0, 1.0)).clamp(0.0, 1.0);
        // quick up, slow down: a crescendo shows at once, the calm comes back gently
        let k = if target > h.energy { dt / 0.3 } else { dt / 1.5 };
        h.energy += (target - h.energy) * k.min(1.0);
        let n = self.look.moods.len().max(1);
        let want = felt.or(h.fixed).unwrap_or(((h.energy * (n - 1) as f32).round() as usize).min(n - 1));
        if want != h.candidate {
            (h.candidate, h.held) = (want, 0.0);
        }
        h.held += dt;
        if h.candidate != h.mood && (h.held >= MOOD_HOLD || h.fixed.is_some() || felt.is_some()) {
            (h.prev, h.mood, h.fade) = (h.mood, h.candidate, 0.0);
            self.feel.turned(h.mood);
            tracing::debug!("mood {} < {}", self.moods[h.mood], self.feel.because().unwrap_or(if h.fixed.is_some() { "stream" } else { "energy" }));
        }
        h.fade = (h.fade + dt / MOOD_FADE).min(1.0);
    }

    /// The feelings' nudges into this deck's graph (after `update`; a mirrored being has none of its own). Never
    /// allocates.
    pub fn push(&mut self, ctl: &mut dyn Knobs) {
        self.feel.push(ctl);
        // a double click's ease back (smoothstep: it leaves gently and lands gently); a waveform jumps at once
        if let Some(r) = self.reset.as_mut() {
            r.t = (r.t + self.dt / RESET_S).min(1.0);
            let b = &self.buttons[r.button];
            if b.shape {
                ctl.set_lfo_shape(b.node, b.scene as usize);
                r.t = 1.0;
            } else {
                let k = r.t * r.t * (3.0 - 2.0 * r.t);
                ctl.set(b.node, b.port, b.spec.value(r.from + (r.to - r.from) * k));
            }
            if r.t >= 1.0 {
                self.reset = None;
            }
        }
    }

    /// The HUD's word on the mood: `MOOD FURIOUS < FM.INDEX +2` (the rule that picked it, the nudges moving the graph),
    /// `MOOD HAPPY < ENERGY 0.62` or `< STREAM`. ≤ 39 characters.
    pub fn mood_line(&self) -> String {
        let mood = self.moods.get(self.head.mood).map_or("-", String::as_str);
        let why = match (self.feel.because(), self.head.fixed) {
            (Some(w), _) => w.to_string(),
            (None, Some(_)) => "stream".into(),
            _ => format!("energy {:.2}", self.head.energy),
        };
        let kicks = match self.feel.nudging() {
            0 => String::new(),
            n => format!(" +{n}"),
        };
        format!("MOOD {mood} < {why}{kicks}").to_uppercase().chars().take(39).collect()
    }

    /// the nearest button under the ray (a little easier to hit than it looks), else the nearest node point
    pub fn pick(&self, ray: &Ray) -> Option<Hit> {
        self.pick_reach(ray, 1.0)
    }

    /// `pick` with the buttons `reach` times easier to hit (a finger on a touch screen: 2)
    pub fn pick_reach(&self, ray: &Ray, reach: f32) -> Option<Hit> {
        let nearest = |hits: &mut dyn Iterator<Item = (Option<f32>, Hit)>| {
            hits.filter_map(|(t, h)| t.map(|t| (t, h))).min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, h)| h)
        };
        // the buttons first – they are what a hand reaches for: the nearest one the ray really hits, else (within the
        // reach) the one it passes closest to, against its size – on a chain the neighbour behind never steals it; a
        // node point (its nucleus) only where no button is
        let off = |c: Vec3| (c - ray.origin).cross(ray.dir).length();
        nearest(&mut self.buttons.iter().enumerate().map(|(i, b)| (ray.sphere(b.pos, b.radius), Hit::Button(i))))
            .or_else(|| {
                self.buttons
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| ray.sphere(b.pos, b.radius * 1.5 * reach).is_some())
                    .min_by(|(_, a), (_, b)| (off(a.pos) / a.radius).total_cmp(&(off(b.pos) / b.radius)))
                    .map(|(i, _)| Hit::Button(i))
            })
            .or_else(|| nearest(&mut self.points.iter().enumerate().map(|(i, p)| (ray.sphere(p.pos, p.radius * 0.7), Hit::Point(i)))))
    }

    /// What the body shows of the graph's values: every button's input (or an LFO's shape) and every hose's source
    /// output – what a deck streams (phase 4), each once.
    pub fn shown(&self) -> Vec<Path> {
        let mut out: Vec<Path> = Vec::new();
        let buttons = self.buttons.iter().map(|b| if b.shape { Path::Shape(b.node) } else { Path::Input(b.node, b.port) });
        let hoses = self.hoses.iter().filter_map(|h| match h.flow {
            Flow::Control(n, p) | Flow::Audio(n, p) => Some(Path::Output(n, p)),
            Flow::Video => None,
        });
        for p in buttons.chain(hoses) {
            if !out.contains(&p) {
                out.push(p);
            }
        }
        out
    }

    /// how many spheres / tube vertices `write` produces at most (the GPU buffers' sizes)
    pub fn capacity(&self) -> (usize, usize) {
        // a button: itself, its ring, its gauge, its tether's bubbles; the core's selection ring (#209)
        let spheres = 3 + 2 * self.points.len() + self.buttons.len() * (3 + TETHER_BUBBLES) + self.hoses.len() * BUBBLES + self.neck.len();
        (spheres, self.tubes() * (SEGMENTS + 1) * SIDES)
    }

    /// hoses + tendrils + neck cables + the buttons' tethers
    pub fn tubes(&self) -> usize {
        self.hoses.len() + self.points.len() - self.core_point.map_or(0, |_| 1) + self.neck.len() + self.buttons.len()
    }

    /// the tubes' triangle list (fixed: only the vertices move)
    pub fn tube_indices(&self) -> Vec<u32> {
        tube_indices(self.tubes())
    }

    /// This frame's geometry into the caller's buffers (cleared first; their capacity from `capacity` – no allocation):
    /// the solid spheres first, then the translucent ones; returns how many are solid. Several beings (phase 4) write
    /// their parts one after the other instead: `write_solid` of all, then `write_ghosts`, and `write_tubes`.
    pub fn write(&self, spheres: &mut Vec<Sphere>, tubes: &mut Vec<TubeVertex>) -> usize {
        spheres.clear();
        tubes.clear();
        self.write_solid(spheres);
        let solid = spheres.len();
        self.write_ghosts(spheres);
        self.write_tubes(tubes);
        solid
    }

    /// a point's colour: a colour control's value on its node (the hue it is set to), else the being's hue + its tint
    fn point_rgb(&self, p: &Point, sat: f32) -> [f32; 3] {
        match p.colour {
            Some(b) => hsv(colour_of(&self.buttons[b]), sat, 1.0),
            None => hsv(self.hue + p.tint, sat, 1.0),
        }
    }

    /// the solid spheres: nuclei (they glow with their audio level), buttons, the neck's sockets
    pub fn write_solid(&self, spheres: &mut Vec<Sphere>) {
        let rgb = |tint: f32, sat: f32, val: f32| hsv(self.hue + tint, sat, val);
        for (i, p) in self.points.iter().enumerate() {
            let c = self.point_rgb(p, 0.55);
            // hovered: lit; one of its buttons moving: it flashes (what the button reaches); its sound: it glows
            let lit: f32 = if self.hover == Some(Hit::Point(i)) { 1.0 } else { 0.0 };
            let r = p.radius * 0.45 * (1.0 + 0.3 * p.flash + 0.35 * p.level);
            spheres.push(Sphere { pos: p.pos.into(), radius: r, color: [c[0], c[1], c[2], 1.0], extra: [kind::NUCLEUS, self.glow, lit.max(p.flash * 0.8).max(p.level * 0.9), 0.0] });
        }
        for b in &self.buttons {
            let c = button_rgb(b);
            let locked = if b.turnable { 0.0 } else { 1.0 };
            // hover swells it, a grip a little more; extra.z: 0..1 hover, 1..2 the hand on it
            let r = b.radius * (1.0 + 0.35 * b.hot + 0.15 * b.grip);
            spheres.push(Sphere { pos: b.pos.into(), radius: r, color: [c[0], c[1], c[2], locked], extra: [kind::BUTTON, self.glow, b.hot + b.grip, b.shown] });
        }
        // the sockets: dark metal where the cables plug into the neck, lit up by the energy
        let c = rgb(0.0, 0.3, 0.3 + 0.7 * self.head.energy);
        for i in 0..self.neck.len() {
            let pos = self.head.socket(i, self.neck.len());
            spheres.push(Sphere { pos: pos.into(), radius: self.look.size * 0.03, color: [c[0], c[1], c[2], 1.0], extra: [kind::NUCLEUS, self.glow, 0.0, 0.0] });
        }
    }

    /// Button `i`'s value ring (always on – the value at rest, #188): (radius, colour with its alpha, extra) – the
    /// knob's 0..1 lit, the swing band lo..hi, the value now as a tick; thin and quiet at rest, wide and bright under
    /// the hand.
    pub fn ring(&self, i: usize) -> (f32, [f32; 4], [f32; 4]) {
        let b = &self.buttons[i];
        let c = hsv(colour_of(b), 0.6, 1.0);
        let r = b.radius * (1.0 + 0.35 * b.hot + 0.15 * b.grip) * (RING_REST + (RING_GRIP - RING_REST) * b.grip);
        let a = (0.5 + 0.2 * b.hot + 0.5 * b.grip).min(1.0);
        (r, [c[0], c[1], c[2], a], [kind::GAUGE + 0.9 * b.live_norm, b.lo, b.hi, b.shown])
    }

    /// the translucent spheres: core, shells, the buttons' rings, bubbles, the head's synthetic skin
    pub fn write_ghosts(&self, spheres: &mut Vec<Sphere>) {
        let rgb = |tint: f32, sat: f32, val: f32| hsv(self.hue + tint, sat, val);
        let c = rgb(0.0, 0.5, 1.0);
        spheres.push(Sphere { pos: self.center.into(), radius: self.core_radius * self.swell, color: [c[0], c[1], c[2], 0.5], extra: [kind::CORE, self.glow, 0.0, 0.0] });
        // #209: the whole being selected (the sticks drive its main pairs) – a white ring around the core
        if matches!(self.selected, Some(Hit::Point(_))) {
            spheres.push(Sphere { pos: self.center.into(), radius: self.core_radius * self.swell * 1.35, color: [1.0, 1.0, 1.0, 0.8], extra: [kind::RING, self.glow, 1.0, 0.0] });
        }
        for p in self.points.iter().filter(|p| p.rest != Vec3::ZERO) {
            let c = self.point_rgb(p, 0.6);
            spheres.push(Sphere { pos: p.pos.into(), radius: p.radius, color: [c[0], c[1], c[2], 0.45], extra: [kind::SHELL, self.glow, p.flash.max(p.level * 0.5), 0.0] });
        }
        let hose = self.look.hose;
        for (i, b) in self.buttons.iter().enumerate() {
            let c = hsv(colour_of(b), 0.6, 1.0);
            let r = b.radius * (1.0 + 0.35 * b.hot + 0.15 * b.grip);
            // the thin ring: hover's, the selection's (#209: the sticks drive it – always, white), or the dashed lock
            // of one that can't be turned (always)
            let chosen = self.selected == Some(Hit::Button(i));
            if chosen {
                spheres.push(Sphere { pos: b.pos.into(), radius: r * 1.55, color: [1.0, 1.0, 1.0, 0.9], extra: [kind::RING, self.glow, 1.0, 0.0] });
            } else if !b.turnable || b.hot > 0.02 {
                let a = if b.turnable { b.hot } else { 0.55 + 0.45 * b.hot };
                spheres.push(Sphere { pos: b.pos.into(), radius: r * 1.4, color: [c[0], c[1], c[2], a], extra: [kind::RING, self.glow, b.hot, if b.turnable { 0.0 } else { 1.0 }] });
            }
            let (radius, color, extra) = self.ring(i);
            spheres.push(Sphere { pos: b.pos.into(), radius, color, extra });
            // the tether's bubbles: well inside its tube (BUBBLE_FILL of its radius at most)
            let bc = hsv(colour_of(b), 0.3, 1.0);
            let tube_r = tether_radius(self.look.size, hose, b.shown);
            for x in b.bubbles.iter().filter(|x| x.size > 0.0) {
                let t = x.t;
                let pos = bezier(&b.tether, t);
                let fade = (t * 6.0).min(1.0) * ((1.0 - t) * 6.0).min(1.0);
                // the tether narrows to 0.7 of its radius toward its end (write_tubes)
                let r = tube_r * (1.0 - 0.3 * t) * BUBBLE_FILL * (0.5 + 0.5 * x.size) * fade.max(0.25);
                spheres.push(Sphere { pos: pos.into(), radius: r, color: [bc[0], bc[1], bc[2], 0.9], extra: [kind::BUBBLE, self.glow, 0.0, x.size] });
            }
        }
        for h in &self.hoses {
            let tint = self.points[h.from].tint;
            let c = rgb(tint + 0.04, 0.25, 1.0);
            for b in h.bubbles.iter().filter(|b| b.size > 0.0) {
                let pos = bezier(&h.ctrl, b.t);
                let fade = (b.t * 8.0).min(1.0) * ((1.0 - b.t) * 8.0).min(1.0);
                let r = h.radius * BUBBLE_FILL * (0.45 + 0.55 * b.size) * fade.max(0.2);
                spheres.push(Sphere { pos: pos.into(), radius: r, color: [c[0], c[1], c[2], 0.9], extra: [kind::BUBBLE, self.glow, 0.0, b.size] });
            }
        }
        // the skin: redder and denser as the limiter strains (the face plate flushes too – gpu.rs)
        let k = self.strain * 0.85;
        let s = rgb(-0.05, 0.35, 1.0);
        let c = [s[0] + (1.0 - s[0]) * k, s[1] + (0.2 - s[1]) * k, s[2] + (0.15 - s[2]) * k];
        spheres.push(Sphere { pos: self.head.pos.into(), radius: self.head.radius, color: [c[0], c[1], c[2], 0.5 + 0.3 * self.strain], extra: [kind::SKIN, self.glow, 0.0, self.head.energy.max(self.strain)] });
    }

    /// the tubes: the hoses, the tendrils, the neck's cables
    pub fn write_tubes(&self, tubes: &mut Vec<TubeVertex>) {
        for h in &self.hoses {
            // a button moving on its source node: the hoses out of it glow too (what the button reaches)
            let flow = h.activity.max(self.points[h.from].flash * 0.5);
            tube(tubes, &h.ctrl, h.radius, h.radius, flow, self.points[h.from].tint, 0.0);
        }
        let center = self.center;
        for p in self.points.iter().filter(|p| p.rest != Vec3::ZERO) {
            let up = (p.pos - center).try_normalize().unwrap_or(Vec3::Y);
            let len = (p.pos - center).length();
            let ctrl = [center, center + up * len * 0.4 + Vec3::Y * len * 0.15, p.pos - up * len * 0.25, p.pos];
            let r = self.look.size * 0.009;
            tube(tubes, &ctrl, r * 1.8, r, 0.0, p.tint, 1.0);
        }
        // the neck: from each point up into its socket (t 0 → 1: the data rising into the head)
        let n = self.neck.len();
        for (i, &pi) in self.neck.iter().enumerate() {
            let (a, b) = (self.points[pi].pos, self.head.socket(i, n));
            let len = (b - a).length();
            let out = (a - center).try_normalize().unwrap_or(Vec3::Y);
            let ctrl = [a, a + (out * 0.35 + Vec3::Y * 0.5) * len * 0.5, b - Vec3::Y * len * 0.45, b];
            let r = self.look.size * if Some(pi) == self.core_point { 0.05 } else { 0.028 };
            tube(tubes, &ctrl, r, r * 0.8, 0.25 + 0.75 * self.head.energy, self.points[pi].tint, 2.0);
        }
        // the tethers: thin hoses in the parameter's colour (a colour control's: its value), thicker and brighter with
        // the value, pulsing while it moves
        for b in &self.buttons {
            let r = tether_radius(self.look.size, self.look.hose, b.shown);
            let flow = (0.25 + 0.75 * b.shown) * (1.0 + 0.8 * b.hot);
            tube(tubes, &b.tether, r, r * 0.7, flow, colour_of(b), 4.0 + 0.9 * b.motion.max(b.grip * 0.6).min(1.0));
        }
    }

    /// Where button `i`'s value 0 and 1 sit on its tether this frame (a drag pulls it along the line between).
    pub fn span(&self, i: usize) -> (Vec3, Vec3) {
        let b = &self.buttons[i];
        let core = Some(b.point) == self.core_point;
        let s = self.look.size;
        let (lo, hi) = (b.root + b.dir * tether_len(s, core, 0.0), b.root + b.dir * tether_len(s, core, 1.0));
        // #224: as it is shown – folded into the view like the knob
        match &self.frame {
            Some(f) => (fold(lo, f, b.base), fold(hi, f, b.base)),
            None => (lo, hi),
        }
    }
}

/// #209: an orthonormal pair across the view axis `f` – `u` level (the fan stays upright as the camera orbits), `v` up
fn across(f: Vec3) -> (Vec3, Vec3) {
    let u = f.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
    (u, u.cross(f))
}

/// #209: the core's spoke at angle `a` around the fan, on a crown around the view axis `f` (CROWN off it): every spoke
/// toward the eye's side of the core, spread evenly around it on the screen.
pub fn crown(f: Vec3, a: f32) -> Vec3 {
    let (u, v) = across(f);
    (f * CROWN.cos() + (u * a.cos() + v * a.sin()) * CROWN.sin()).normalize()
}

/// #209: a chain's direction `d` leaning toward the eye (`f`): one that faces it enough (cos ≥ LEAN) stays; one turned
/// away swings over to the eye's side, keeping its way around the view axis – and the further its point sits behind
/// the body, the more it heads out along that point's own way (`out`, from the centre) in the view: it rounds the body
/// instead of passing behind it. Continuous in `d` and `f` (the eased face moves it smoothly).
pub fn lean(d: Vec3, out: Vec3, f: Vec3) -> Vec3 {
    let k = d.dot(f);
    if k >= LEAN {
        return d;
    }
    let side = (out - f * out.dot(f)).try_normalize();
    let t0 = (d - f * k).try_normalize().or(side).unwrap_or_else(|| across(f).0);
    let w = ((LEAN - k) / (LEAN + 1.0)).clamp(0.0, 1.0) * (-out.dot(f)).clamp(0.0, 1.0);
    let t = side.and_then(|s| (t0 * (1.0 - w) + s * w).try_normalize()).unwrap_or(t0);
    (t * (1.0 - LEAN * LEAN).sqrt() + f * LEAN).normalize()
}

/// a tether from the knob at `pos` to `end` (its node point, or the chain's knob before), sagging a little
fn tether(pos: Vec3, end: Vec3) -> [Vec3; 4] {
    let sag = Vec3::Y * (-0.18 * (end - pos).length());
    [pos, pos + (end - pos) / 3.0 + sag, pos + (end - pos) * (2.0 / 3.0) + sag, end]
}

/// #224: the fold's frame at a depth, in the view's tangents: the half view less EDGE and a knob's margin `m` (tangent)
fn limits(f: &Frame, m: f32) -> (f32, f32) {
    ((f.tan_x * (1.0 - EDGE) - m).max(f.tan_x * 0.05), (f.tan_y * (1.0 - EDGE) - m).max(f.tan_y * 0.05))
}

/// #224: tangents (x, y) kept inside the fold's frame
fn inside(x: f32, y: f32, f: &Frame, m: f32) -> (f32, f32) {
    let (lx, ly) = limits(f, m);
    (x.clamp(-lx, lx), y.clamp(-ly, ly))
}

/// #224: `pos` (a knob of radius `kr`) folded into the view `f`. In the view's tangents scaled to the frame (the half
/// view less EDGE and the knob's size), a knob inside KNEE of it stays; one further out moves in along its own way
/// from the middle, softly toward the frame's edge without reaching it – so the knobs keep their order around the
/// picture. One behind the eye (or nearer than `f.min_depth`) comes in front at that depth and counts as further
/// out the more behind it is: it lands on the edge on its own side. Continuous; the same as `pos` for any knob well
/// inside the picture.
pub fn fold(pos: Vec3, f: &Frame, kr: f32) -> Vec3 {
    let q = pos - f.eye;
    let raw = q.dot(f.fwd);
    let z = raw.max(f.min_depth);
    let (lx, ly) = limits(f, kr * CLEAR / z);
    let (mut u, mut v) = (q.dot(f.right) / z / lx, q.dot(f.up) / z / ly);
    let mut r = u.abs().max(v.abs());
    if raw < f.min_depth {
        // behind: outward by how far behind (straight down when it sits right on the axis)
        if r < 1e-4 {
            (u, v, r) = (0.0, -1e-4, 1e-4);
        }
        let out = r + 2.0 * (f.min_depth - raw) / f.min_depth;
        (u, v, r) = (u * out / r, v * out / r, out);
    }
    if r > KNEE {
        // t / (1 + t): smooth at the knee (slope 1), and far out still in order (an exp would round off in f32)
        let room = 1.0 - KNEE;
        let t = (r - KNEE) / room;
        let k = (KNEE + room * t / (1.0 + t)) / r;
        (u, v) = (u * k, v * k);
    }
    f.eye + (f.fwd + f.right * (u * lx) + f.up * (v * ly)) * z
}

/// #209: `pos` (a knob of radius `kr`) moved out of the shadow the sphere (`c`, `r`) casts seen from `eye`: sideways in
/// the view until it clears the sphere's silhouette by CLEAR × `kr`. Weighted by its depth – nothing in front of the
/// sphere, fully from its centre back – so the push grows smoothly as the camera orbits.
pub fn clear_of(pos: Vec3, eye: Vec3, c: Vec3, r: f32, kr: f32) -> Vec3 {
    let to_c = c - eye;
    let dist = to_c.length();
    if dist <= r * 1.01 {
        return pos;
    }
    let d = to_c / dist;
    let q = pos - c;
    let depth = q.dot(d);
    let w = ((depth + r) / r.max(1e-6)).clamp(0.0, 1.0);
    if w <= 0.0 {
        return pos;
    }
    // the silhouette's cone, widened at the knob's depth (perspective), and the knob's own size
    let tan = r / (dist * dist - r * r).sqrt();
    let need = (dist + depth.max(0.0)) * tan + kr * CLEAR;
    let perp = q - d * depth;
    let off = perp.length();
    if off >= need {
        return pos;
    }
    let side = if off > 1e-5 { perp / off } else { across(d).0 };
    pos + side * (need - off) * w
}

/// the triangle list of `count` tubes (fixed: only their vertices move)
pub fn tube_indices(count: usize) -> Vec<u32> {
    let mut idx = Vec::with_capacity(count * SEGMENTS * SIDES * 6);
    for t in 0..count {
        let base = (t * (SEGMENTS + 1) * SIDES) as u32;
        for s in 0..SEGMENTS as u32 {
            for k in 0..SIDES as u32 {
                let (a, b) = (base + s * SIDES as u32 + k, base + s * SIDES as u32 + (k + 1) % SIDES as u32);
                let (c, d) = (a + SIDES as u32, b + SIDES as u32);
                idx.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
    }
    idx
}

/// a tube along a cubic Bézier: SEGMENTS + 1 rings of SIDES vertices, radius r0 → r1
pub fn tube(out: &mut Vec<TubeVertex>, c: &[Vec3; 4], r0: f32, r1: f32, flow: f32, tint: f32, kind: f32) {
    let mut prev_n = Vec3::ZERO;
    for s in 0..=SEGMENTS {
        let t = s as f32 / SEGMENTS as f32;
        let p = bezier(c, t);
        let tan = bezier_tangent(c, t).try_normalize().unwrap_or(Vec3::Y);
        // a normal carried along the curve (parallel transport light): project the previous one, else any perpendicular
        let n = (prev_n - tan * prev_n.dot(tan)).try_normalize().unwrap_or_else(|| tan.any_orthonormal_vector());
        prev_n = n;
        let bn = tan.cross(n);
        let r = r0 + (r1 - r0) * t;
        for k in 0..SIDES {
            let a = TAU * k as f32 / SIDES as f32;
            let normal = n * a.cos() + bn * a.sin();
            out.push(TubeVertex { pos: (p + normal * r).into(), normal: normal.into(), t, flow, tint, kind });
        }
    }
}

pub fn bezier(c: &[Vec3; 4], t: f32) -> Vec3 {
    let u = 1.0 - t;
    c[0] * (u * u * u) + c[1] * (3.0 * u * u * t) + c[2] * (3.0 * u * t * t) + c[3] * (t * t * t)
}

fn bezier_tangent(c: &[Vec3; 4], t: f32) -> Vec3 {
    let u = 1.0 - t;
    (c[1] - c[0]) * (3.0 * u * u) + (c[2] - c[1]) * (6.0 * u * t) + (c[3] - c[2]) * (3.0 * t * t)
}

/// a button's colour: the parameter's own (a colour control: its value), brighter with the value; one that can't be
/// turned a pale metal
fn button_rgb(b: &Button) -> [f32; 3] {
    let hue = colour_of(b);
    if !b.turnable {
        hsv(hue, 0.18, 0.5 + 0.4 * b.shown)
    } else {
        hsv(hue, 0.75, 0.4 + 0.6 * b.shown)
    }
}

/// HSV → RGB (h wraps)
pub fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = (h - h.floor()) * 6.0;
    let f = |n: f32| {
        let k = (n + h) % 6.0;
        v - v * s * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    [f(5.0), f(3.0), f(1.0)]
}

/// a stable 0..1 from a node id (its place on the body must not change between runs)
fn hash(s: &str) -> f32 {
    let mut h: u32 = 2_166_136_261;
    for b in s.bytes() {
        h = (h ^ b as u32).wrapping_mul(16_777_619);
    }
    (h >> 8) as f32 / (1u32 << 24) as f32
}

fn exposed(list: &[String], node: &str, port: &str) -> bool {
    list.is_empty() || list.iter().any(|e| e == node) || named(list, node, port)
}

/// `expose` names this port itself ("space.bloom")
fn named(list: &[String], node: &str, port: &str) -> bool {
    list.iter().any(|e| e.split_once('.') == Some((node, port)))
}

#[cfg(test)]
mod tests {
    use super::*;
        use crate::scene::Scene;
    use crate::space::values::{Local, Table};

    const SCENE: &str = "[[node]]\nid=\"space\"\nkind=\"space\"\n[[node]]\nid=\"screen\"\nkind=\"screen\"\n\
        [[node]]\nid=\"lfo\"\nkind=\"lfo\"\nrate=2\n[[node]]\nid=\"fm\"\nkind=\"fm\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\n\
        [[link]]\nfrom=\"space\"\nto=\"screen.in\"\n[[link]]\nfrom=\"lfo\"\nto=\"fm.index\"\nscale=4\n[[link]]\nfrom=\"fm\"\nto=\"out.in\"\n";

    fn graph(text: &str) -> Graph {
        Graph::build(&Scene::parse(text).unwrap()).unwrap()
    }

    fn now(beat: f64) -> tempo::Now {
        tempo::Now { bpm: 120.0, pos: tempo::position(beat, 4), per_bar: 4, running: true }
    }

    #[test]
    fn the_body_has_a_point_per_node_a_hose_per_link_and_buttons_per_input() {
        let g = graph(SCENE);
        let b = Being::new(&g, &Look::default());
        assert_eq!(b.points.len(), 5);
        assert_eq!(b.hoses.len(), 3, "a hose per link");
        for (h, l) in b.hoses.iter().zip(&g.links) {
            assert_eq!((b.points[h.from].node, b.points[h.to].node), (l.from.0, l.to.0));
        }
        // control inputs: space hue / glow / light / pulse, lfo rate / depth + its shape, fm freq / ratio / index / gain,
        // out volume
        assert_eq!(b.buttons.len(), 4 + 3 + 4 + 1);
        assert!(b.buttons.iter().any(|x| x.shape && g.nodes[x.node].kind == Kind::Lfo));
        let index = b.buttons.iter().find(|x| x.name == "index").unwrap();
        assert!(index.linked, "fm.index is driven by the LFO's hose");
        assert_eq!(b.points[b.core_point.unwrap()].rest, Vec3::ZERO, "the space node is the core");
        // the neck: the core's spine first, then every other point (five nodes – all of them, under NECK)
        assert_eq!(b.neck.len(), 5);
        assert_eq!(b.neck[0], b.core_point.unwrap());
        let (spheres, verts) = b.capacity();
        assert_eq!(b.tubes(), 3 + 4 + 5 + 12, "hoses + a tendril per non-core point + the neck's cables + the tethers");
        assert_eq!(verts, 24 * (SEGMENTS + 1) * SIDES);
        assert_eq!(spheres, 2 + 10 + 12 * (3 + TETHER_BUBBLES) + 3 * BUBBLES + 5 + 1, "core (+ its selection ring), nuclei + shells, buttons (+ ring, gauge, bubbles), bubbles, sockets, skin");
        assert_eq!(b.tube_indices().len(), 24 * SEGMENTS * SIDES * 6);
        assert!(b.tube_indices().iter().all(|&i| (i as usize) < verts));
    }

    #[test]
    fn the_look_limits_the_buttons() {
        let g = graph(SCENE);
        let look = Look { expose: vec!["fm.index".into(), "space".into()], ..Look::default() };
        let b = Being::new(&g, &look);
        let names: Vec<_> = b.buttons.iter().map(|x| (g.nodes[x.node].id.as_str(), x.name)).collect();
        assert_eq!(names, [("space", "hue"), ("space", "glow"), ("space", "light"), ("space", "pulse"), ("fm", "index")]);
    }

    #[test]
    fn bubbles_flow_from_the_source_to_the_target_when_the_value_changes() {
        let g = graph(SCENE);
        let mut b = Being::new(&g, &Look::default());
        let mut c = Table::new(&g);
        let lfo_hose = b.hoses.iter().position(|h| h.ty == PortType::Control).unwrap();
        let silent = |_: usize, _: usize| 0.0;
        for f in 0..120 {
            c.step(1.0 / 60.0);
            b.update(1.0 / 60.0, f as f32 / 60.0, &Local { ctl: &c, meter: &silent }, &now(f as f64 / 30.0));
        }
        let h = &b.hoses[lfo_hose];
        assert!(h.bubbles.iter().any(|x| x.size > 0.0), "a 2 Hz LFO keeps the hose bubbling");
        // the curve runs from the source's point (t 0) to the target's (t 1): the LFO → the FM voice
        assert_eq!((b.points[h.from].node, b.points[h.to].node), (2, 3));
        assert_eq!((h.ctrl[0], h.ctrl[3]), (b.points[h.from].pos, b.points[h.to].pos));
        let before = h.bubbles;
        c.step(1.0 / 60.0);
        b.update(1.0 / 60.0, 2.0, &Local { ctl: &c, meter: &silent }, &now(4.0));
        let h = &b.hoses[lfo_hose];
        let mut moved = 0;
        for (x0, x) in before.iter().zip(&h.bubbles) {
            // a bubble keeps its slot until it arrives: it only moves on, toward the target
            if x0.size > 0.0 && x.size > 0.0 && x.size == x0.size && x.speed == x0.speed {
                assert!(x.t > x0.t, "{x0:?} → {x:?}");
                moved += 1;
            }
        }
        assert!(moved > 0);
        assert!(h.bubbles.iter().all(|x| x.size == 0.0 || (0.0..1.0).contains(&x.t)));
        // the audio hose: silent → nothing; a loud meter → bubbles
        let audio = b.hoses.iter().position(|h| h.ty == PortType::Audio).unwrap();
        assert!(b.hoses[audio].bubbles.iter().all(|x| x.size == 0.0));
        let loud = |_: usize, _: usize| 0.8;
        b.update(0.25, 2.25, &Local { ctl: &c, meter: &loud }, &now(4.5));
        assert!(b.hoses[audio].bubbles.iter().any(|x| x.size > 0.0));
    }

    #[test]
    fn a_steady_value_makes_no_bubbles() {
        let g = graph("[[node]]\nid=\"l\"\nkind=\"lfo\"\nrate=0\n[[node]]\nid=\"p\"\nkind=\"test_pattern\"\n[[link]]\nfrom=\"l\"\nto=\"p.hue\"\n");
        let mut b = Being::new(&g, &Look::default());
        let mut c = Table::new(&g);
        for f in 0..300 {
            c.step(1.0 / 60.0);
            b.update(1.0 / 60.0, f as f32 / 60.0, &Local { ctl: &c, meter: &|_, _| 0.0 }, &now(0.0));
        }
        assert!(b.hoses[0].bubbles.iter().all(|x| x.size == 0.0));
        assert!(b.hoses[0].activity < 0.01);
    }

    #[test]
    fn picking_finds_the_nearest_button() {
        let g = graph(SCENE);
        let mut b = Being::new(&g, &Look::default());
        let c = Table::new(&g);
        b.update(0.0, 0.0, &Local { ctl: &c, meter: &|_, _| 0.0 }, &now(0.0));
        let target = b.buttons[3].pos;
        let origin = target + Vec3::new(0.0, 0.0, 5.0);
        let ray = Ray { origin, dir: (target - origin).normalize() };
        // aimed at a button's centre: that button, or one in front of it on the same ray
        let Some(Hit::Button(i)) = b.pick(&ray) else { panic!("the ray goes through a button") };
        assert!(i == 3 || b.buttons[i].pos.z > target.z, "the nearest one");
        let away = Ray { origin, dir: Vec3::Z };
        assert_eq!(b.pick(&away), None);
    }

    #[test]
    fn write_fills_the_buffers_within_capacity() {
        let g = graph(SCENE);
        let mut b = Being::new(&g, &Look::default());
        let mut c = Table::new(&g);
        let (ns, nv) = b.capacity();
        let (mut s, mut v) = (Vec::with_capacity(ns), Vec::with_capacity(nv));
        for f in 0..200 {
            c.step(1.0 / 60.0);
            b.update(1.0 / 60.0, f as f32 / 60.0, &Local { ctl: &c, meter: &|_, _| 1.0 }, &now(f as f64 / 20.0));
            let solid = b.write(&mut s, &mut v);
            assert_eq!(solid, 5 + 12 + 5, "nuclei + buttons + sockets");
            assert!(s.len() <= ns && v.len() == nv);
            assert_eq!((s.capacity(), v.capacity()), (ns, nv), "no growth");
        }
        assert!(v.iter().all(|x| x.pos.iter().all(|f| f.is_finite())));
    }

    #[test]
    fn it_breathes_on_the_beat() {
        let g = graph(SCENE);
        let mut b = Being::new(&g, &Look::default());
        let c = Table::new(&g);
        b.update(0.0, 0.0, &Local { ctl: &c, meter: &|_, _| 0.0 }, &now(8.0));
        let on = b.swell;
        b.update(0.0, 0.0, &Local { ctl: &c, meter: &|_, _| 0.0 }, &now(8.9));
        assert!(on > b.swell, "fuller on the beat than just before the next");
        assert!(b.head().y > b.center.y + b.look.size);
    }

    /// phase 5: silence is the first mood; a loud, fast deck walks up to the last – a new mood only once it has held,
    /// then cross-faded; a mood fixed from outside (another deck's stream) shows at once
    #[test]
    fn the_mood_follows_the_energy_and_holds() {
        use crate::space::values::Mirror;
        let g = graph("[[node]]\nid=\"space\"\nkind=\"space\"\n[[node]]\nid=\"n\"\nkind=\"noise\"\n[[node]]\nid=\"fft\"\nkind=\"fft\"\n[[link]]\nfrom=\"n\"\nto=\"fft.in\"\n");
        let mut b = Being::new(&g, &Look::default());
        let mut m = Mirror::new(&g);
        let fast = tempo::Now { bpm: 170.0, pos: tempo::position(0.0, 4), per_bar: 4, running: true };
        let still = tempo::Now::default();
        let run = |b: &mut Being, m: &Mirror, beat: &tempo::Now, frames: usize| {
            for _ in 0..frames {
                b.update(1.0 / 60.0, 0.0, m, beat);
            }
        };
        run(&mut b, &m, &still, 60);
        assert_eq!((b.head.mood, b.head.energy), (0, 0.0), "silence");
        m.apply(&g, "fft.level", &[1.0]);
        run(&mut b, &m, &fast, 20);
        assert_eq!(b.head.mood, 0, "a third of a second is not yet a mood");
        run(&mut b, &m, &fast, 240);
        assert!(b.head.energy > 0.95, "{}", b.head.energy);
        assert_eq!(b.head.mood, b.look.moods.len() - 1);
        assert_eq!(b.head.fade, 1.0, "faded in");
        b.head.fixed = Some(1);
        run(&mut b, &m, &fast, 1);
        assert_eq!((b.head.mood, b.head.prev), (1, 4));
        assert!(b.head.fade < 1.0, "cross-fading");
        // the head floats above the body with its neck's sockets under it
        let h = &b.head;
        assert!(h.pos.y > b.center.y + b.look.size && (0..5).all(|i| h.socket(i, 5).y < h.pos.y));
    }

    fn step(b: &mut Being, c: &Table, frames: usize) {
        for f in 0..frames {
            b.update(1.0 / 60.0, f as f32 / 60.0, &Local { ctl: c, meter: &|_, _| 0.0 }, &now(0.0));
        }
    }

    /// owner 2026-10-31: a button's value is its distance from the point before it – low = close, high = far, clamped
    #[test]
    fn a_tether_is_as_long_as_its_value() {
        for core in [false, true] {
            for n in [0.0, 0.25, 0.5, 1.0] {
                assert!((tether_norm(0.5, core, tether_len(0.5, core, n)) - n).abs() < 1e-5);
            }
            assert_eq!(tether_len(0.5, core, 7.0), tether_len(0.5, core, 1.0), "clamped");
            assert!(tether_len(0.5, core, 0.0) > 0.0 && tether_len(0.5, core, 1.0) < 0.5, "never inside its point, never wider than the body");
        }
        let g = graph(SCENE);
        let mut b = Being::new(&g, &Look::default());
        let mut c = Table::new(&g);
        let fm = g.nodes.iter().position(|n| n.id == "fm").unwrap();
        let ratio = b.buttons.iter().position(|x| x.node == fm && x.name == "ratio").unwrap();
        let gain = b.buttons.iter().position(|x| x.node == fm && x.name == "gain").unwrap();
        // the FM voice's buttons hang one from the other: its first from the node point, the others from the one before
        assert_eq!(b.buttons[ratio].prev, Some(ratio - 1));
        assert_eq!(b.buttons[ratio - 1].prev, None);
        let space = b.buttons.iter().filter(|x| Some(x.point) == b.core_point);
        assert!(space.clone().count() == 4 && space.clone().all(|x| x.prev.is_none()), "the core's are spokes");
        let mut far = Vec::new();
        for n in [0.1, 0.5, 0.9] {
            c.set(fm, b.buttons[ratio].port, b.buttons[ratio].spec.value(n));
            step(&mut b, &c, 60);
            let x = &b.buttons[ratio];
            let d = x.pos.distance(x.root);
            assert!((tether_norm(b.look.size, false, d) - n).abs() < 0.01, "{n}: {d}");
            // the button after it rides along: it hangs from this one
            assert_eq!(b.buttons[ratio + 1].root, x.pos);
            far.push(b.buttons[gain].pos.distance(b.points[x.point].pos));
        }
        assert!(far[0] < far[1] && far[1] < far[2], "turned up, the chain reaches further: {far:?}");
        // the hose: from the button (t 0) to the point it hangs from (t 1)
        let x = &b.buttons[ratio];
        assert_eq!((x.tether[0], x.tether[3]), (x.pos, b.buttons[ratio - 1].pos));
    }

    /// a deck's being at the conductor (its values from the stream): its silhouette follows them
    #[test]
    fn a_mirrored_being_sprawls_with_its_values() {
        use crate::space::values::Mirror;
        let g = graph(SCENE);
        let mut b = Being::new(&g, &Look::default());
        let mut m = Mirror::new(&g);
        let i = b.buttons.iter().position(|x| x.name == "ratio").unwrap();
        let reach = |b: &Being| b.buttons[i].pos.distance(b.buttons[i].root);
        m.apply(&g, "fm.ratio", &[0.3]);
        for _ in 0..60 {
            b.update(1.0 / 60.0, 0.0, &m, &now(0.0));
        }
        let quiet = reach(&b);
        m.apply(&g, "fm.ratio", &[7.0]);
        b.update(1.0 / 60.0, 0.0, &m, &now(0.0));
        assert!(b.buttons[i].motion > 0.3, "a jump on the stream moves it: {}", b.buttons[i].motion);
        for _ in 0..90 {
            b.update(1.0 / 60.0, 0.0, &m, &now(0.0));
        }
        assert!(reach(&b) > quiet * 2.0, "{quiet} → {}", reach(&b));
        assert!(b.buttons[i].motion < 0.05, "and rests again: {}", b.buttons[i].motion);
    }

    /// hover swells a button and rings it; a hand on it lights it and widens its ring; one that can't be turned (a
    /// mirrored deck's linked knob) only shakes
    #[test]
    fn hover_and_grip_show_on_the_button() {
        let g = graph(SCENE);
        let mut b = Being::new(&g, &Look::default());
        let c = Table::new(&g);
        let (ns, nv) = b.capacity();
        let (mut s, mut v) = (Vec::with_capacity(ns), Vec::with_capacity(nv));
        let glow = b.buttons.iter().position(|x| x.name == "glow").unwrap();
        let kinds = |s: &[Sphere], k: f32| s.iter().filter(|x| x.extra[0] == k).count();
        step(&mut b, &c, 10);
        b.write(&mut s, &mut v);
        let rest = b.buttons[glow].radius;
        assert_eq!(kinds(&s, kind::RING), 0, "this deck's knobs all turn (a linked one as its base): no lock rings");
        b.hover = Some(Hit::Button(glow));
        step(&mut b, &c, 30);
        b.write(&mut s, &mut v);
        let me = s.iter().find(|x| x.extra[0] == kind::BUTTON && Vec3::from(x.pos) == b.buttons[glow].pos).unwrap();
        assert!(me.radius > rest * 1.3 && me.extra[2] > 0.95 && me.extra[2] < 1.05, "swollen, lit: {me:?}");
        assert_eq!(kinds(&s, kind::RING), 1, "the hover ring");
        let before = b.ring(glow);
        b.dragging = true;
        step(&mut b, &c, 30);
        let gripped = b.ring(glow);
        assert!(gripped.0 > before.0 * 1.2 && gripped.1[3] > 0.95 && b.buttons[glow].grip > 0.95, "wider and brighter under the hand");
        // a mirrored being (the stream carries no base): its linked knob can't be turned – grabbed, it shakes
        use crate::space::values::Mirror;
        let mut m = Being::new(&g, &Look::default());
        let mirror = Mirror::new(&g);
        let index = m.buttons.iter().position(|x| x.linked).unwrap();
        m.update(1.0 / 60.0, 0.0, &mirror, &now(0.0));
        assert!(!m.buttons[index].turnable);
        m.hover = Some(Hit::Button(index));
        m.dragging = true;
        m.shake(index);
        for _ in 0..3 {
            m.update(1.0 / 60.0, 0.0, &mirror, &now(0.0));
        }
        let x = &m.buttons[index];
        let still = x.root + x.dir * tether_len(m.look.size, false, x.shown);
        assert!(x.shake > 0.0 && x.pos.distance(still) > 1e-4, "it shakes");
        m.write(&mut s, &mut v);
        assert!(s.iter().any(|q| q.extra[0] == kind::RING && q.extra[3] == 1.0), "the dashed lock");
        for _ in 0..40 {
            m.update(1.0 / 60.0, 0.0, &mirror, &now(0.0));
        }
        assert_eq!((m.buttons[index].shake, m.buttons[index].grip < 0.01), (0.0, true), "still now, never gripped");
    }

    /// owner 2026-10-31: no numbers, the value at rest – every knob always wears its ring, centred on it, a little
    /// wider than the knob, lit up to its value; no labels at all
    #[test]
    fn every_knob_wears_its_value_ring_at_rest() {
        let g = graph(&SCENE.replace("kind=\"fm\"\n", "kind=\"fm\"\nratio=4\n"));
        let mut b = Being::new(&g, &Look::default());
        let c = Table::new(&g);
        let (ns, nv) = b.capacity();
        let (mut s, mut v) = (Vec::with_capacity(ns), Vec::with_capacity(nv));
        step(&mut b, &c, 30);
        b.write(&mut s, &mut v);
        let rings: Vec<&Sphere> = s.iter().filter(|x| x.extra[0] >= kind::GAUGE && x.extra[0] < kind::GAUGE + 1.0).collect();
        assert_eq!(rings.len(), b.buttons.len(), "one each, nobody touching");
        for (x, r) in b.buttons.iter().zip(&rings) {
            assert_eq!(Vec3::from(r.pos), x.pos, "centred on its knob");
            assert!((r.radius / x.radius - RING_REST).abs() < 1e-4, "{}: {}", x.name, r.radius / x.radius);
            assert_eq!(r.extra[3], x.shown, "lit up to the value");
            assert!((r.color[3] - 0.5).abs() < 1e-4, "quiet at rest");
        }
        let ratio = b.buttons.iter().position(|x| x.name == "ratio").unwrap();
        let spec = b.buttons[ratio].spec;
        assert!((b.ring(ratio).2[3] - spec.norm(4.0)).abs() < 1e-3, "ratio 4 lights {} of the arc", spec.norm(4.0));
        // an unlinked knob has no swing: lo = hi = its value, the tick on it
        let (_, _, e) = b.ring(ratio);
        assert!(e[1] == e[2] && e[1] == e[3] && ((e[0] - kind::GAUGE) / 0.9 - e[3]).abs() < 1e-5, "{e:?}");
    }

    /// owner 2026-10-31: a linked knob turns as its base – the knob sets where the modulation sways around (graph
    /// semantics: source · scale + base), its ring shows the base, the modulation's swing and the value now
    #[test]
    fn a_linked_knob_turns_as_its_base_and_shows_its_swing() {
        let g = graph(SCENE);
        let mut b = Being::new(&g, &Look::default());
        let mut c = Table::new(&g);
        let index = b.buttons.iter().position(|x| x.name == "index").unwrap();
        let (node, port) = (b.buttons[index].node, b.buttons[index].port);
        assert_eq!(b.buttons[index].scene, 0.0, "its scene value: the link's offset");
        assert!(c.set(node, port, 3.0), "the knob: its base");
        let mut seen = (f32::MAX, f32::MIN);
        for f in 0..240 {
            c.step(1.0 / 60.0);
            b.update(1.0 / 60.0, f as f32 / 60.0, &Local { ctl: &c, meter: &|_, _| 0.0 }, &now(0.0));
            let x = &b.buttons[index];
            seen = (seen.0.min(x.live), seen.1.max(x.live));
            assert!(x.turnable && x.value == 3.0, "the knob stays at its base: {}", x.value);
        }
        // the LFO (0..1 × 4) rides on top: 3 .. 7
        assert!(seen.0 < 3.2 && seen.1 > 6.8, "{seen:?}");
        let x = &b.buttons[index];
        assert!(x.lo <= x.norm && x.hi >= x.spec.norm(6.5) && x.lo <= x.live_norm && x.live_norm <= x.hi, "the swing band holds the modulation: {} {} {}", x.lo, x.hi, x.live_norm);
        // its tether is as long as its base, not the swaying value
        let d = x.pos.distance(x.root);
        assert!((tether_norm(b.look.size, false, d) - x.spec.norm(3.0)).abs() < 0.02);
        // a double click: back to the link's offset
        b.reset(index);
        for _ in 0..40 {
            step(&mut b, &c, 1);
            b.push(&mut c);
        }
        assert_eq!(c.base(node, port), Some(0.0));
    }

    /// owner 2026-10-31: the bubbles never touch the hose walls – each well inside its tube, scaled with its radius
    #[test]
    fn bubbles_stay_inside_their_tubes() {
        let g = graph(SCENE);
        let mut b = Being::new(&g, &Look::default());
        let mut c = Table::new(&g);
        let (ns, nv) = b.capacity();
        let (mut s, mut v) = (Vec::with_capacity(ns), Vec::with_capacity(nv));
        let knob = b.buttons.iter().position(|x| x.name == "ratio").unwrap();
        let mut checked = 0;
        for f in 0..300 {
            c.step(1.0 / 60.0);
            c.set(b.buttons[knob].node, b.buttons[knob].port, 0.3 + (f % 60) as f32 * 0.1);
            b.update(1.0 / 60.0, f as f32 / 60.0, &Local { ctl: &c, meter: &|_, _| 0.9 }, &now(f as f64 / 30.0));
            b.write(&mut s, &mut v);
            // each bubble against its tube where it is: a hose's radius, a tether's narrowing from the knob (t 0) to 0.7
            let radius_at = |pos: Vec3| s.iter().find(|q| q.extra[0] == kind::BUBBLE && Vec3::from(q.pos) == pos).map(|q| q.radius);
            for h in &b.hoses {
                for x in h.bubbles.iter().filter(|x| x.size > 0.0) {
                    let r = radius_at(bezier(&h.ctrl, x.t)).expect("drawn");
                    assert!(r <= h.radius * BUBBLE_FILL + 1e-6, "a bubble of {r} in a hose of {}", h.radius);
                    checked += 1;
                }
            }
            for k in &b.buttons {
                let tube_r = tether_radius(b.look.size, b.look.hose, k.shown);
                for x in k.bubbles.iter().filter(|x| x.size > 0.0) {
                    let r = radius_at(bezier(&k.tether, x.t)).expect("drawn");
                    assert!(r <= tube_r * (1.0 - 0.3 * x.t) * BUBBLE_FILL + 1e-6, "a bubble of {r} in a tether of {tube_r}");
                    checked += 1;
                }
            }
        }
        assert!(checked > 50, "bubbles ran: {checked}");
    }

    /// owner 2026-10-31: a colour control shows its value as that colour – its knob, tether, ring and node point; the
    /// others keep their own
    #[test]
    fn a_colour_control_wears_its_colour() {
        let g = graph(&SCENE.replace("[[node]]\nid=\"screen\"", "[[node]]\nid=\"p\"\nkind=\"test_pattern\"\nhue=0.3\n[[node]]\nid=\"screen\""));
        let mut b = Being::new(&g, &Look::default());
        let mut c = Table::new(&g);
        let p = g.nodes.iter().position(|n| n.id == "p").unwrap();
        let hue = b.buttons.iter().position(|x| x.node == p && x.name == "hue").unwrap();
        let pt = b.buttons[hue].point;
        assert_eq!(b.points[pt].colour, Some(hue));
        for v in [0.3f32, 0.8] {
            c.set(p, b.buttons[hue].port, v);
            step(&mut b, &c, 2);
            assert!((colour_of(&b.buttons[hue]) - v).abs() < 1e-6);
            assert_eq!(b.point_rgb(&b.points[pt], 0.55), hsv(v, 0.55, 1.0), "the node point too");
            assert_eq!(b.ring(hue).1[..3], hsv(v, 0.6, 1.0), "the ring");
        }
        let speed = b.buttons.iter().position(|x| x.node == p && x.name == "speed").unwrap();
        assert_eq!(colour_of(&b.buttons[speed]), param_hue("speed"), "the others keep their identity colour");
    }

    /// owner 2026-10-31: level and overload on the being – a node point glows with its sound, the head flushes red with
    /// the limiter's gain reduction
    #[test]
    fn points_glow_with_their_level_and_the_head_strains() {
        let g = graph(SCENE);
        let mut b = Being::new(&g, &Look::default());
        let c = Table::new(&g);
        let fm = b.points.iter().position(|p| g.nodes[p.node].id == "fm").unwrap();
        b.update(1.0 / 60.0, 0.0, &Local { ctl: &c, meter: &|_, _| 0.0 }, &now(0.0));
        assert_eq!(b.points[fm].level, 0.0, "silent");
        b.update(1.0 / 60.0, 0.0, &Local { ctl: &c, meter: &|_, _| 0.64 }, &now(0.0));
        assert!((b.points[fm].level - 0.8).abs() < 1e-5, "up at once: √0.64");
        for _ in 0..60 {
            b.update(1.0 / 60.0, 0.0, &Local { ctl: &c, meter: &|_, _| 0.0 }, &now(0.0));
        }
        assert!(b.points[fm].level < 0.05, "and down again");
        assert_eq!(b.strain, 0.0);
        for _ in 0..20 {
            b.strain(1.0 / 60.0, 12.0);
        }
        assert!(b.strain > 0.95, "12 dB of gain reduction: full flush {}", b.strain);
        let (ns, nv) = b.capacity();
        let (mut s, mut v) = (Vec::with_capacity(ns), Vec::with_capacity(nv));
        b.write(&mut s, &mut v);
        let skin = s.iter().find(|x| x.extra[0] == kind::SKIN).unwrap();
        assert!(skin.color[0] > 0.9 && skin.color[1] < 0.4, "a red skin: {:?}", skin.color);
        for _ in 0..120 {
            b.strain(1.0 / 60.0, 0.0);
        }
        assert!(b.strain < 0.1, "it calms down: {}", b.strain);
    }

    /// a double click eases the button back to the scene's value – gently, through the values between
    #[test]
    fn a_reset_eases_back_to_the_scene() {
        let g = graph(&SCENE.replace("kind=\"fm\"\n", "kind=\"fm\"\nratio=2.5\n"));
        let mut b = Being::new(&g, &Look::default());
        let mut c = Table::new(&g);
        let i = b.buttons.iter().position(|x| x.name == "ratio").unwrap();
        assert_eq!(b.buttons[i].scene, 2.5);
        let (node, port) = (b.buttons[i].node, b.buttons[i].port);
        c.set(node, port, 6.0);
        step(&mut b, &c, 2);
        b.reset(i);
        let mut seen = Vec::new();
        for _ in 0..40 {
            step(&mut b, &c, 1);
            b.push(&mut c);
            seen.push(c.input(node, "ratio", 0.0));
        }
        assert!((seen[39] - 2.5).abs() < 1e-3 && b.reset.is_none(), "{seen:?}");
        assert!(seen.windows(2).all(|w| w[1] <= w[0] + 1e-4), "one way down");
        assert!(seen.iter().any(|v| *v > 3.0 && *v < 5.5), "eased, not a jump: {seen:?}");
    }


    /// the cathedral's being (the phone's and the deck's), its values, at a camera orbiting it
    fn cathedral() -> (Being, Table) {
        let scene = Scene::parse(crate::scene::CATHEDRAL_SCENE).unwrap();
        let g = Graph::build(&scene).unwrap();
        let mut b = Being::new(&g, &scene.being);
        b.anchor = Vec3::new(0.0, 1.4, 0.0);
        (b, Table::new(&g))
    }

    fn orbit_eye(b: &Being, yaw: f32, pitch: f32, dist: f32) -> Vec3 {
        b.anchor + Vec3::Y * b.look.size * 0.45 + dist * Vec3::new(pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos())
    }

    /// #209 (owner 2026-10-31: "the control nodes align themselves so they're always in sight of the camera … never
    /// hidden behind the body or the head"): from all around and from above and below, once the layout has followed the
    /// eye, no knob sits in the core's or the head's shadow, and the core's spokes all point to the eye's side
    #[test]
    fn the_knobs_face_the_camera_and_never_hide_behind_the_core_or_the_head() {
        let (mut b, t) = cathedral();
        for pitch in [-1.1f32, -0.35, 0.1, 0.6, 1.1, 1.35] {
            for k in 0..12 {
                let yaw = TAU * k as f32 / 12.0;
                let eye = orbit_eye(&b, yaw, pitch, 2.2);
                b.eye = Some(eye);
                step(&mut b, &t, 90);
                let to_eye = (eye - b.center).normalize();
                assert!(b.face.dot(to_eye) > 0.999, "the layout has followed the eye");
                for (i, x) in b.buttons.iter().enumerate() {
                    let ray = Ray { origin: eye, dir: (x.pos - eye).normalize() };
                    let far = (x.pos - eye).length() - x.radius;
                    for (what, c, r) in [("core", b.center, b.core_radius * b.swell), ("head", b.head.pos, b.head.radius)] {
                        let hidden = ray.sphere(c, r).is_some_and(|hit| hit < far);
                        assert!(!hidden, "button {i} ({}) hidden behind the {what} at yaw {yaw:.2} pitch {pitch}", x.name);
                    }
                    if Some(x.point) == b.core_point {
                        assert!(x.dir.dot(to_eye) > 0.3, "the core's spoke {i} toward the eye: {}", x.dir.dot(to_eye));
                    }
                }
            }
        }
    }

    /// #209: the layout re-fans eased as the camera orbits, not jumping – the eye flipped to the far side moves the face
    /// a little each frame and arrives within a second or two
    #[test]
    fn the_layout_follows_the_eye_eased() {
        let (mut b, t) = cathedral();
        b.eye = Some(orbit_eye(&b, 0.0, 0.1, 2.2));
        step(&mut b, &t, 1);
        let front = b.face;
        let spoke = b.buttons.iter().position(|x| Some(x.point) == b.core_point).unwrap();
        let before = b.buttons[spoke].pos;
        b.eye = Some(orbit_eye(&b, 2.6, 0.1, 2.2));
        step(&mut b, &t, 1);
        let moved = b.face.angle_between(front);
        assert!(moved > 0.0 && moved < 0.25, "one frame turns the face a little: {moved}");
        assert!(b.buttons[spoke].pos.distance(before) < b.look.size * 0.2, "a spoke glides, it doesn't jump");
        step(&mut b, &t, 120);
        let want = (orbit_eye(&b, 2.6, 0.1, 2.2) - b.center).normalize();
        assert!(b.face.angle_between(want) < 0.02, "arrived: {}", b.face.angle_between(want));
    }

    /// #209: without a camera (no eye) the layout is the fixed one; the lean leaves a chain that faces the eye alone
    /// and is continuous where it starts to bend
    #[test]
    fn lean_and_crown_keep_their_shape() {
        let f = Vec3::Z;
        let d = Vec3::new(0.6, 0.0, 0.8);
        assert_eq!(lean(d, Vec3::X, f), d, "facing enough: unchanged");
        let edge = Vec3::new((1.0 - LEAN * LEAN).sqrt(), 0.0, LEAN);
        let below = Vec3::new((1.0 - (LEAN - 1e-3).powi(2)).sqrt(), 0.0, LEAN - 1e-3);
        assert!(lean(below, -Vec3::Z, f).distance(edge) < 1e-2, "continuous at the edge");
        let back = lean(-Vec3::Z, Vec3::new(0.3, 0.0, -1.0).normalize(), f);
        assert!((back.dot(f) - LEAN).abs() < 1e-4 && back.x > 0.5, "straight back: over to the eye's side, out its point's way: {back}");
        for k in 0..6 {
            let c = crown(f, TAU * k as f32 / 6.0);
            assert!((c.dot(f) - CROWN.cos()).abs() < 1e-5 && (c.length() - 1.0).abs() < 1e-5);
        }
        // the shadow: a point straight behind a sphere moves out of it, one beside it stays
        let eye = Vec3::new(0.0, 0.0, 5.0);
        let out = clear_of(Vec3::new(0.05, 0.0, -0.5), eye, Vec3::ZERO, 0.3, 0.02);
        let ray = Ray { origin: eye, dir: (out - eye).normalize() };
        assert!(ray.sphere(Vec3::ZERO, 0.3).is_none(), "out of the shadow: {out}");
        let beside = Vec3::new(1.0, 0.0, -0.5);
        assert_eq!(clear_of(beside, eye, Vec3::ZERO, 0.3, 0.02), beside);
    }

    #[test]
    fn hsv_primaries() {
        assert_eq!(hsv(0.0, 1.0, 1.0), [1.0, 0.0, 0.0]);
        assert_eq!(hsv(1.0 / 3.0, 1.0, 1.0).map(|x| (x * 1e4).round() / 1e4), [0.0, 1.0, 0.0]);
        assert_eq!(hsv(0.5, 0.0, 0.5), [0.5, 0.5, 0.5]);
    }

    /// #224: the fold leaves a knob inside the picture alone, brings any other in – never out to the edge, keeping the
    /// knobs' order across the picture and changing smoothly – and puts one behind the eye on the edge on its own side
    #[test]
    fn the_fold_brings_every_knob_into_the_picture_in_order() {
        use crate::space::camera::Orbit;
        let cam = Orbit::new(Vec3::ZERO, 0.0, 0.0, 1.0);
        let f = cam.frame(9.0 / 19.5);
        let kr = 0.03;
        let tan = |p: Vec3| {
            let q = p - f.eye;
            let z = q.dot(f.fwd);
            (q.dot(f.right) / z / f.tan_x, q.dot(f.up) / z / f.tan_y, z)
        };
        // the middle: untouched
        let mid = Vec3::new(0.05, -0.1, 0.0);
        assert_eq!(fold(mid, &f, kr), mid);
        // a row running out to the right, far past the edge: in the picture, in order, nearer the edge the further out
        let mut last = f32::MIN;
        let mut prev = fold(Vec3::new(0.0, -0.2, 0.0), &f, kr);
        for k in 0..400 {
            let p = Vec3::new(k as f32 * 0.01, -0.2, 0.0);
            let q = fold(p, &f, kr);
            let (x, y, _) = tan(q);
            assert!(x.abs() < 1.0 - EDGE && y.abs() < 1.0 - EDGE, "{k}: inside the frame: {x} {y}");
            assert!(x > last, "{k}: in order: {last} → {x}");
            assert!(q.distance(prev) < 0.02, "{k}: smooth – no jump");
            (last, prev) = (x, q);
        }
        // behind the eye, down and left: in front, on the lower left edge – the way it lies
        let q = fold(Vec3::new(-0.2, -0.3, 1.6), &f, kr);
        let (x, y, z) = tan(q);
        assert!(z >= f.min_depth - 1e-5, "in front of the eye: {z}");
        assert!(x < 0.0 && y < 0.0 && x.abs().max(y.abs()) > 0.5 && x.abs().max(y.abs()) < 1.0 - EDGE, "toward its edge: {x} {y}");
    }
}
