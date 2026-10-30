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
//! Data-driven so a being can later be saved as a preset (#191): the body is the graph + the scene's `[being]` look;
//! the colours are the space node's inputs (hue, glow). Values come through `Values` (values.rs): this deck's control
//! domain and audio meters, or another deck's stream (phase 4) – no per-node copies; `update` and `write` never
//! allocate (tests/no_alloc.rs).
use crate::graph::{Domain, Graph, Kind, PortType};
use crate::space::camera::Ray;
use crate::space::values::{Path, Values};
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
/// how long a new mood must hold before the head shows it (s), and the cross-fade (s)
const MOOD_HOLD: f32 = 1.2;
const MOOD_FADE: f32 = 0.5;
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
        }
    }
}

/// The head (phase 5): where it floats, its size, the energy and the mood it shows (an index into `look.moods`,
/// cross-faded from `prev` by `fade`).
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
    /// driven by a hose (read-only)
    pub linked: bool,
    /// the LFO's waveform (`port` is meaningless then; the value is an index into control::LFO_SHAPES)
    pub shape: bool,
    pub pos: Vec3,
    pub radius: f32,
    /// this frame's value and its 0..1
    pub value: f32,
    pub norm: f32,
    slot: (f32, usize),
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
}

/// one tube vertex (40 bytes): position, normal, t along the tube, its flow (activity), hue tint, kind (0 hose,
/// 1 tendril, 2 neck cable, 3 a hose between beings)
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
    /// the hovered / dragged part (the renderer lights it)
    pub hover: Option<Hit>,
    pub dragging: bool,
    pub head: Head,
    /// the points whose cables run up into the neck (the core first, then the highest points)
    pub neck: Vec<usize>,
    /// the FFT node (the head's energy)
    fft: Option<usize>,
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
            points.push(Point { node: i, rest, pos: rest, radius, tint, phase: h * TAU });
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
            let ports: Vec<usize> = node
                .kind
                .inputs()
                .iter()
                .enumerate()
                .filter(|(_, port)| port.ty == PortType::Control && exposed(&look.expose, &node.id, port.name))
                .map(|(k, _)| k)
                .collect();
            for (j, &k) in ports.iter().enumerate() {
                let name = node.kind.inputs()[k].name;
                buttons.push(Button {
                    point: pi,
                    node: p.node,
                    port: k,
                    name,
                    spec: params::spec(node.kind, name),
                    linked: graph.link_into(p.node, k).is_some(),
                    shape: false,
                    pos: p.rest,
                    radius: s * 0.05,
                    value: 0.0,
                    norm: 0.0,
                    slot: (TAU * j as f32 / ports.len() as f32, ports.len()),
                });
            }
            // the waveform: an LFO's shape (an audio oscillator's `wave` can't change while it plays yet)
            if node.kind == Kind::Lfo && exposed(&look.expose, &node.id, "shape") {
                let count = ports.len() + 1;
                for b in buttons.iter_mut().filter(|b| b.point == pi) {
                    b.slot.1 = count;
                    b.slot.0 = b.slot.0 * ports.len() as f32 / count as f32;
                }
                buttons.push(Button {
                    point: pi,
                    node: p.node,
                    port: usize::MAX,
                    name: "shape",
                    spec: Spec { min: 0.0, max: (crate::control::LFO_SHAPES.len() - 1) as f32, default: 0.0, curve: params::Curve::Lin },
                    linked: false,
                    shape: true,
                    pos: p.rest,
                    radius: s * 0.05,
                    value: 0.0,
                    norm: 0.0,
                    slot: (TAU * ports.len() as f32 / count as f32, count),
                });
            }
        }
        let core_point = core_node.map(point_of);
        // the neck: the core's spine, then the points nearest the head (the sinks – where the data has risen to)
        let mut by_height: Vec<usize> = (0..points.len()).filter(|&i| Some(i) != core_point).collect();
        by_height.sort_by(|&a, &b| points[b].rest.y.total_cmp(&points[a].rest.y));
        let neck: Vec<usize> = core_point.into_iter().chain(by_height).take(NECK).collect();
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
            hover: None,
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
                fixed: None,
            },
            neck,
            fft: graph.first(Kind::Fft),
        }
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
        for bt in &mut self.buttons {
            let p = &self.points[bt.point];
            let node = p.node;
            bt.value = if bt.shape { vals.lfo_shape(node).unwrap_or(0) as f32 } else { vals.input(node, bt.name, bt.spec.default) };
            bt.norm = bt.spec.norm(bt.value);
            let (angle, count) = bt.slot;
            bt.pos = if Some(bt.point) == self.core_point {
                // around the core's equator, facing out
                let r = self.core_radius * self.swell * 1.3;
                p.pos + Vec3::new(angle.cos(), 0.0, angle.sin()) * r
            } else {
                let out = (p.pos - center).try_normalize().unwrap_or(Vec3::Y);
                let u = out.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
                let v = out.cross(u);
                let ring = p.radius * if count > 1 { 1.9 } else { 0.0 };
                p.pos + out * p.radius * 1.9 + (u * angle.cos() + v * angle.sin()) * ring
            };
        }
        self.step_head(dt, vals, beat);
    }

    /// The head's place, its energy (the FFT level – this deck's sound – and the tempo: a fast clock is a little
    /// wilder) and its mood: the energy's mood must hold MOOD_HOLD before it shows, then it cross-fades. A fixed mood
    /// (another deck's stream says it) shows at once.
    fn step_head(&mut self, dt: f32, vals: &dyn Values, beat: &tempo::Now) {
        let pos = self.head();
        let h = &mut self.head;
        h.pos = pos;
        let level = self.fft.map_or(0.0, |n| vals.output(n, 0)).clamp(0.0, 1.0);
        let bpm = if beat.running { beat.bpm as f32 } else { 0.0 };
        let target = (0.8 * (level * 1.3).min(1.0) + 0.2 * ((bpm - 60.0) / 100.0).clamp(0.0, 1.0)).clamp(0.0, 1.0);
        // quick up, slow down: a crescendo shows at once, the calm comes back gently
        let k = if target > h.energy { dt / 0.3 } else { dt / 1.5 };
        h.energy += (target - h.energy) * k.min(1.0);
        let n = self.look.moods.len().max(1);
        let want = h.fixed.unwrap_or(((h.energy * (n - 1) as f32).round() as usize).min(n - 1));
        if want != h.candidate {
            (h.candidate, h.held) = (want, 0.0);
        }
        h.held += dt;
        if h.candidate != h.mood && (h.held >= MOOD_HOLD || h.fixed.is_some()) {
            (h.prev, h.mood, h.fade) = (h.mood, h.candidate, 0.0);
        }
        h.fade = (h.fade + dt / MOOD_FADE).min(1.0);
    }

    /// the nearest button under the ray (a little easier to hit than it looks), else the nearest node point
    pub fn pick(&self, ray: &Ray) -> Option<Hit> {
        let nearest = |hits: &mut dyn Iterator<Item = (Option<f32>, Hit)>| {
            hits.filter_map(|(t, h)| t.map(|t| (t, h))).min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, h)| h)
        };
        // the buttons first – they are what a hand reaches for; a node point (its nucleus) only where no button is
        nearest(&mut self.buttons.iter().enumerate().map(|(i, b)| (ray.sphere(b.pos, b.radius * 1.5), Hit::Button(i))))
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
        let spheres = 2 + 2 * self.points.len() + self.buttons.len() + self.hoses.len() * BUBBLES + self.neck.len();
        (spheres, self.tubes() * (SEGMENTS + 1) * SIDES)
    }

    /// hoses + tendrils + neck cables
    pub fn tubes(&self) -> usize {
        self.hoses.len() + self.points.len() - self.core_point.map_or(0, |_| 1) + self.neck.len()
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

    /// the solid spheres: nuclei, buttons, the neck's sockets
    pub fn write_solid(&self, spheres: &mut Vec<Sphere>) {
        let hover = |h: Hit| match self.hover {
            Some(x) if x == h => {
                if self.dragging {
                    2.0
                } else {
                    1.0
                }
            }
            _ => 0.0,
        };
        let rgb = |tint: f32, sat: f32, val: f32| hsv(self.hue + tint, sat, val);
        for (i, p) in self.points.iter().enumerate() {
            let c = rgb(p.tint, 0.55, 1.0);
            spheres.push(Sphere { pos: p.pos.into(), radius: p.radius * 0.45, color: [c[0], c[1], c[2], 1.0], extra: [kind::NUCLEUS, self.glow, hover(Hit::Point(i)), 0.0] });
        }
        for (i, b) in self.buttons.iter().enumerate() {
            let p = &self.points[b.point];
            let c = match (b.name, b.linked) {
                ("hue", _) => hsv(b.value, 0.8, 1.0),
                (_, true) => rgb(p.tint, 0.15, 0.55 + 0.45 * b.norm),
                _ => rgb(p.tint + 0.08, 0.7, 0.35 + 0.65 * b.norm),
            };
            let link = if b.linked { 1.0 } else { 0.0 };
            spheres.push(Sphere { pos: b.pos.into(), radius: b.radius, color: [c[0], c[1], c[2], link], extra: [kind::BUTTON, self.glow, hover(Hit::Button(i)), b.norm] });
        }
        // the sockets: dark metal where the cables plug into the neck, lit up by the energy
        let c = rgb(0.0, 0.3, 0.3 + 0.7 * self.head.energy);
        for i in 0..self.neck.len() {
            let pos = self.head.socket(i, self.neck.len());
            spheres.push(Sphere { pos: pos.into(), radius: self.look.size * 0.03, color: [c[0], c[1], c[2], 1.0], extra: [kind::NUCLEUS, self.glow, 0.0, 0.0] });
        }
    }

    /// the translucent spheres: core, shells, bubbles, the head's synthetic skin
    pub fn write_ghosts(&self, spheres: &mut Vec<Sphere>) {
        let rgb = |tint: f32, sat: f32, val: f32| hsv(self.hue + tint, sat, val);
        let c = rgb(0.0, 0.5, 1.0);
        spheres.push(Sphere { pos: self.center.into(), radius: self.core_radius * self.swell, color: [c[0], c[1], c[2], 0.5], extra: [kind::CORE, self.glow, 0.0, 0.0] });
        for p in self.points.iter().filter(|p| p.rest != Vec3::ZERO) {
            let c = rgb(p.tint, 0.6, 1.0);
            spheres.push(Sphere { pos: p.pos.into(), radius: p.radius, color: [c[0], c[1], c[2], 0.45], extra: [kind::SHELL, self.glow, 0.0, 0.0] });
        }
        for h in &self.hoses {
            let tint = self.points[h.from].tint;
            let c = rgb(tint + 0.04, 0.25, 1.0);
            for b in h.bubbles.iter().filter(|b| b.size > 0.0) {
                let pos = bezier(&h.ctrl, b.t);
                let fade = (b.t * 8.0).min(1.0) * ((1.0 - b.t) * 8.0).min(1.0);
                spheres.push(Sphere { pos: pos.into(), radius: h.radius * (0.6 + 0.9 * b.size) * fade.max(0.2), color: [c[0], c[1], c[2], 0.9], extra: [kind::BUBBLE, self.glow, 0.0, b.size] });
            }
        }
        let c = rgb(-0.05, 0.35, 1.0);
        spheres.push(Sphere { pos: self.head.pos.into(), radius: self.head.radius, color: [c[0], c[1], c[2], 0.5], extra: [kind::SKIN, self.glow, 0.0, self.head.energy] });
    }

    /// the tubes: the hoses, the tendrils, the neck's cables
    pub fn write_tubes(&self, tubes: &mut Vec<TubeVertex>) {
        for h in &self.hoses {
            tube(tubes, &h.ctrl, h.radius, h.radius, h.activity, self.points[h.from].tint, 0.0);
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
    }
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
    list.is_empty() || list.iter().any(|e| e == node || e.split_once('.') == Some((node, port)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::Controls;
    use crate::scene::Scene;
    use crate::space::values::Local;

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
        assert_eq!(b.tubes(), 3 + 4 + 5, "hoses + a tendril per non-core point + the neck's cables");
        assert_eq!(verts, 12 * (SEGMENTS + 1) * SIDES);
        assert_eq!(spheres, 1 + 10 + 12 + 3 * BUBBLES + 5 + 1, "core, nuclei + shells, buttons, bubbles, sockets, skin");
        assert_eq!(b.tube_indices().len(), 12 * SEGMENTS * SIDES * 6);
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
        let mut c = Controls::new(&g, None);
        let lfo_hose = b.hoses.iter().position(|h| h.ty == PortType::Control).unwrap();
        let silent = |_: usize, _: usize| 0.0;
        for f in 0..120 {
            c.update(1.0 / 60.0, 0, &now(f as f64 / 30.0));
            b.update(1.0 / 60.0, f as f32 / 60.0, &Local { ctl: &c, meter: &silent }, &now(f as f64 / 30.0));
        }
        let h = &b.hoses[lfo_hose];
        assert!(h.bubbles.iter().any(|x| x.size > 0.0), "a 2 Hz LFO keeps the hose bubbling");
        // the curve runs from the source's point (t 0) to the target's (t 1): the LFO → the FM voice
        assert_eq!((b.points[h.from].node, b.points[h.to].node), (2, 3));
        assert_eq!((h.ctrl[0], h.ctrl[3]), (b.points[h.from].pos, b.points[h.to].pos));
        let before = h.bubbles;
        c.update(1.0 / 60.0, 0, &now(4.0));
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
        let mut c = Controls::new(&g, None);
        for f in 0..300 {
            c.update(1.0 / 60.0, 0, &now(0.0));
            b.update(1.0 / 60.0, f as f32 / 60.0, &Local { ctl: &c, meter: &|_, _| 0.0 }, &now(0.0));
        }
        assert!(b.hoses[0].bubbles.iter().all(|x| x.size == 0.0));
        assert!(b.hoses[0].activity < 0.01);
    }

    #[test]
    fn picking_finds_the_nearest_button() {
        let g = graph(SCENE);
        let mut b = Being::new(&g, &Look::default());
        let c = Controls::new(&g, None);
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
        let mut c = Controls::new(&g, None);
        let (ns, nv) = b.capacity();
        let (mut s, mut v) = (Vec::with_capacity(ns), Vec::with_capacity(nv));
        for f in 0..200 {
            c.update(1.0 / 60.0, 0, &now(f as f64 / 20.0));
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
        let c = Controls::new(&g, None);
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

    #[test]
    fn hsv_primaries() {
        assert_eq!(hsv(0.0, 1.0, 1.0), [1.0, 0.0, 0.0]);
        assert_eq!(hsv(1.0 / 3.0, 1.0, 1.0).map(|x| (x * 1e4).round() / 1e4), [0.0, 1.0, 0.0]);
        assert_eq!(hsv(0.5, 0.0, 0.5), [0.5, 0.5, 0.5]);
    }
}
