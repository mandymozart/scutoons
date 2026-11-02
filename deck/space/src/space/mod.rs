//! The 3D space (#188 "Cathedral of decks"): a `space` node draws a set's 3D – its glTF (glb.rs, meshopt.rs; DECISIONS
//! 2026-10-30: place → set → assets, a set by id) – and the deck's
//! being (being.rs – its running graph as a floating body, its head the character's portrait – portrait.rs) through an
//! orbit camera (camera.rs) into its texture, with a depth buffer (gpu.rs, shaders/space.wgsl). The pointer picks the
//! being's 3D buttons by ray cast; a drag pulls one along its tether like a MIDI CC (`Knobs::set`), elsewhere it
//! turns the camera; a double click eases a button back to the scene's value; a finger on a touch screen does the
//! same with a bigger reach (`touch`).
//!
//! Phase 4 – the session (the space node's `join`, with a relay set): a **deck** streams its being to the relay
//! (stream.rs, link.rs); the **conductor**'s deck draws every deck's being in a circle around its own, bigger one in
//! the middle of the nave, and the hoses between them along the session's routes (decks.rs). Each deck shows only its
//! own being. This module holds the CPU side; the renderer draws what it says. The conductor pushes the scene's sync
//! (#195, `push_sync`) and shows each deck's answer over its being and in the HUD (sync.rs).
//!
//! deck-space (#198): the same code on the deck (deck-native) and in the browser (deck/space-web). What is native stays
//! behind the feature `native` (the set's loader thread and file, the portraits') or in deck-native (the relay link, a
//! `Wire`); a host without threads hands a set in with `set_arrived`.
pub mod being;
pub mod camera;
pub mod decks;
pub mod feel;
pub mod glb;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod label;
pub mod meshopt;
pub mod portrait;
pub mod post;
#[cfg(feature = "gpu")]
pub mod post_gpu;
pub mod stream;
pub mod sync;
pub mod values;
pub mod wire;

use crate::graph::Graph;
use crate::params;
use crate::tempo;
use anyhow::Result;
use being::{Being, Hit, Look, Sphere, TubeVertex};
use label::Label;
use camera::Orbit;
use decks::{Remote, Route};
use glam::Vec3;
use portrait::Portraits;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use values::{Knobs, Local};
use wire::{In, Wire};

/// How this deck's being takes part in a Live session (the space node's `join`, with a relay set – deck.toml's `relay`
/// or `--relay`): alone (no session), as a deck of the congregation, or as the conductor in the middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Join {
    Alone,
    Deck,
    Conductor,
}

impl Join {
    pub fn parse(s: &str) -> Option<Join> {
        match s {
            "" | "alone" => Some(Join::Alone),
            "deck" => Some(Join::Deck),
            "conductor" => Some(Join::Conductor),
            _ => None,
        }
    }
}

/// the circle's radius around the conductor when the space node names none (m)
const RING: f32 = 3.2;

/// where the camera looks: a little above the body's centre, so the head is in the picture
fn aim(b: &Being) -> Vec3 {
    b.anchor + Vec3::Y * b.look.size * 0.45
}

/// the photographer's focus: from this many sizes away the camera aims at the whole being, from this near at its head
const FOCUS_FAR: f32 = 7.0;
const FOCUS_NEAR: f32 = 2.2;
/// #224: a deselect frames the whole being this far (a share of the view's half) off the edges
const FRAME_MARGIN: f32 = 0.08;

/// where the being floats when the set has no such spot (or there is no set)
const FALLBACK_ANCHOR: Vec3 = Vec3::new(0.0, 1.4, 0.0);
/// the glow's light in the room (#204): reach (m) and strength by default, and how many beings light it at most
pub const GLOW_RANGE: f32 = 7.0;
pub const GLOW_LIGHT: f32 = 1.0;
pub const MAX_LIGHTS: usize = 8;

/// two presses on one button within this long (s) = a double click: back to the scene's value
const DOUBLE_CLICK: f32 = 0.4;
/// how much easier a finger hits a button than the mouse
const TOUCH_REACH: f32 = 2.0;
/// #209: a press that moved less than this (× the view's height) before it let go is a tap – it selects
const TAP: f32 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    None,
    /// turning the camera from the last pointer position
    Orbit(f32, f32),
    /// pulling button `button` along its tether: its 0..1 and the pointer when it started, the tether's 0 → 1 on the
    /// screen (pixels)
    Knob { button: usize, norm: f32, at: (f32, f32), axis: (f32, f32) },
    /// a hand on a button that doesn't turn (linked, or the double click that just reset it)
    Hold,
}

/// what the pointer should look like (main.rs sets the window's cursor)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    Default,
    /// over a button that turns
    Grab,
    /// a hand on it
    Grabbing,
    /// over a linked button (it can't be turned)
    Locked,
}

/// a touch's phase (winit's, without winit)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Touch {
    Started,
    Moved,
    Ended,
}

pub struct Space {
    /// the space node
    pub node: usize,
    pub being: Being,
    pub camera: Orbit,
    /// the set's 3D, from its loader until the renderer takes it
    set_rx: Option<mpsc::Receiver<Result<glb::Model>>>,
    model: Option<glb::Model>,
    /// the set's 3D file shown (or loading) and its spots (kept after the renderer took it – a Live cell moves the
    /// camera to one, #186)
    set_path: Option<PathBuf>,
    spots: Vec<(String, Vec3)>,
    max_texture: u32,
    /// the set's glTF nodes left out (the space node's `hide`: their meshes)
    hide: Vec<String>,
    /// waits for the sync before a set's file is read (sync.rs; None = read what is there)
    gate: Option<Gate>,
    /// what the HUD says about the set ("SET 96K TRIS", "LOADING …", the error)
    pub set_status: String,
    spot: String,
    lift: f32,
    /// an explicit place in the set (metres) instead of spot + lift
    at: Option<Vec3>,
    /// the set's light (the space node's `light`) this frame
    pub light: f32,
    /// the post stack the scene picked (#204, post.rs) and its knobs this frame
    pub post: post::Chain,
    pub post_knobs: post::Knobs,
    /// the glow as light in the room (#204): how far a being's light reaches (m) and how strong it is (0 = off) – the
    /// space node's `glow_range` / `glow_light`
    pub glow_range: f32,
    pub glow_light: f32,
    pointer: Option<(f32, f32)>,
    view: (f32, f32),
    drag: Drag,
    /// the last press on a button (which, when – the show's time) for the double click
    last_press: Option<(usize, f32)>,
    /// the finger that drives the pointer on a touch screen
    finger: Option<u64>,
    /// #209: the last press – where, and what it hit (a tap selects that) – and the taps not yet asked for (`take_tap`)
    pressed: Option<((f32, f32), Option<Hit>)>,
    tapped: Option<Option<Hit>>,
    /// the show's time at the last update
    clock: f32,
    /// a word for the HUD until a time (a linked button grabbed: what drives it)
    notice: Option<(String, f32)>,
    /// this deck's graph (the stream reads it; a remote's comes from the wire)
    graph: Graph,
    /// the heads' sprites
    pub portraits: Portraits,
    pub join: Join,
    /// a deck: what it streams of its being
    stream: Option<stream::Stream>,
    /// the session's relay link (deck-native's link.rs)
    link: Option<Box<dyn Wire>>,
    /// the session's state in a word (the HUD): alone, no relay, the link's status
    note: String,
    /// the conductor: the decks' beings and the hoses between beings
    pub others: Vec<Remote>,
    pub routes: Vec<Route>,
    /// presence's routes as client ids, and this stage's own id
    route_ids: Vec<(String, String)>,
    me: Option<String>,
    ring: f32,
    /// bumped when beings or routes come or go (the renderer sizes its buffers anew)
    pub generation: u64,
    places: Vec<Vec3>,
    /// #195: the conductor's push and the decks' answers
    pub sync: sync::Board,
}

/// Called on the loader thread with a set's file before it is read: the sync (sync.rs) returns once that file is
/// settled – downloaded, already cached, or offline. Never on the frame.
pub type Gate = std::sync::Arc<dyn Fn(&Path) + Send + Sync>;

/// the 3D file of the set a space node names: `set` with an extension = a path (relative to `sets`), else the set's id
/// at `quality` (low · medium · high, default low – the X1's UHD 620): `scene::set_file`
pub fn set_path(graph: &Graph, node: usize, sets: &Path) -> Option<PathBuf> {
    let n = &graph.nodes[node];
    let set = n.str("set")?;
    let p = Path::new(set);
    Some(if p.extension().is_some() { sets.join(p) } else { crate::scene::set_file(sets, set, &quality(graph, node)) })
}

/// the space node's set id and quality (None: no set, or a path) – what the sync fetches
pub fn set_need(graph: &Graph, node: usize) -> Option<(String, String)> {
    let set = graph.nodes[node].str("set")?;
    Path::new(set).extension().is_none().then(|| (set.to_string(), quality(graph, node)))
}

fn quality(graph: &Graph, node: usize) -> String {
    graph.nodes[node].str("quality").unwrap_or("low").to_string()
}

/// a set's 3D file → its loader thread (after the gate); the receiver and the HUD's "LOADING …"
#[cfg(feature = "native")]
fn load_set(path: PathBuf, max: u32, hide: Vec<String>, gate: Option<Gate>) -> (Option<mpsc::Receiver<Result<glb::Model>>>, String) {
    let (tx, rx) = mpsc::channel();
    let shown = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    std::thread::Builder::new()
        .name("set".into())
        .spawn(move || {
            if let Some(gate) = gate {
                gate(&path);
            }
            let t = std::time::Instant::now();
            let r = glb::load(&path, max, &hide);
            if let Ok(set) = &r {
                tracing::info!("set {}: {} triangles, {} images, {:.0} ms", path.display(), set.triangles(), set.images.len(), t.elapsed().as_secs_f32() * 1000.0);
            }
            let _ = tx.send(r);
        })
        .expect("a thread");
    (Some(rx), format!("LOADING {shown}"))
}

/// without threads (the browser): the host fetches the set and hands it in (`Space::set_arrived`)
#[cfg(not(feature = "native"))]
fn load_set(path: PathBuf, _max: u32, _hide: Vec<String>, _gate: Option<Gate>) -> (Option<mpsc::Receiver<Result<glb::Model>>>, String) {
    (None, format!("LOADING {}", path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()))
}

impl Space {
    /// The space node `node` of `graph`: its being from the graph + `look`, its set's 3D loading on a thread of its own
    /// (from `sets`, the `--sets` folder), its head's sprites from `portraits`.
    pub fn new(graph: &Graph, node: usize, look: &Look, sets: &Path, portraits: Portraits) -> Space {
        Space::with_gate(graph, node, look, sets, portraits, None)
    }

    /// `new`, with the sync's gate before every set file is read (sync.rs)
    pub fn with_gate(graph: &Graph, node: usize, look: &Look, sets: &Path, mut portraits: Portraits, gate: Option<Gate>) -> Space {
        let n = &graph.nodes[node];
        let mut being = Being::new(graph, look);
        being.head.sprites = portraits.request(&look.character, &being.moods);
        for p in &being.feel.problems {
            tracing::warn!("being: {p} – left out");
        }
        let join = n.str("join").map_or(Some(Join::Alone), Join::parse).unwrap_or_else(|| {
            tracing::warn!("space: join = '{}' – deck, conductor or alone; alone", n.str("join").unwrap_or_default());
            Join::Alone
        });
        let at = match n.f32s("at").as_slice() {
            [x, y, z] => Some(Vec3::new(*x, *y, *z)),
            _ => None,
        };
        being.anchor = at.unwrap_or(FALLBACK_ANCHOR);
        let max_texture = n.f32("max_texture", 2048.0) as u32;
        let hide = n.strs("hide");
        let path = set_path(graph, node, sets);
        let (set_rx, set_status) = match &path {
            Some(path) => load_set(path.clone(), max_texture, hide.clone(), gate.clone()),
            None => (None, "NO SET".into()),
        };
        let (post, problems) = post::Chain::of(n);
        for p in &problems {
            tracing::warn!("space: {p} – left out");
        }
        let mut camera = Orbit::new(aim(&being), n.f32("yaw", 20.0).to_radians(), n.f32("pitch", 8.0).to_radians(), n.f32("distance", 2.4));
        camera.auto = n.bool("auto_turn", false);
        Space {
            node,
            being,
            camera,
            set_rx,
            model: None,
            set_path: path,
            spots: Vec::new(),
            max_texture,
            hide,
            gate,
            set_status,
            spot: n.str("spot").unwrap_or("spot_aisle").to_string(),
            lift: n.f32("lift", 1.5),
            at,
            light: 1.0,
            post,
            post_knobs: post::Knobs::default(),
            glow_range: n.f32("glow_range", GLOW_RANGE).max(0.1),
            glow_light: n.f32("glow_light", GLOW_LIGHT).max(0.0),
            pointer: None,
            view: (1.0, 1.0),
            drag: Drag::None,
            last_press: None,
            finger: None,
            pressed: None,
            tapped: None,
            clock: 0.0,
            notice: None,
            graph: graph.clone(),
            portraits,
            join,
            stream: None,
            link: None,
            note: String::new(),
            others: Vec::new(),
            routes: Vec::new(),
            route_ids: Vec::new(),
            me: None,
            ring: n.f32("ring", RING),
            generation: 0,
            places: Vec::new(),
            sync: sync::Board::default(),
        }
    }

    /// what a deck announces of its being on the wire (`deck.graph`); None unless the space node joins as a deck
    pub fn wire_graph(&self) -> Option<protocol::DeckGraph> {
        (self.join == Join::Deck).then(|| wire::deck_graph(&self.graph, &self.being.look, &self.being.moods))
    }

    /// The session's link (deck-native's link.rs: the relay at `ws://host:port/live/<code>`, joined as the space node's
    /// `join` says – a deck streams its being, the conductor gathers the decks'), or why there is none (the HUD).
    pub fn attach(&mut self, link: Result<Box<dyn Wire>, String>) {
        match link {
            Ok(l) => {
                if self.join == Join::Deck {
                    self.stream = Some(stream::Stream::new(&self.graph, &self.being));
                }
                self.link = Some(l);
            }
            Err(e) => {
                tracing::warn!("cathedral: {e}");
                self.note = e.to_uppercase();
            }
        }
    }

    /// One frame: the set's arrival, the camera, the being (its feelings' nudges into `ctl`), what the pointer is
    /// over; a deck's stream, the conductor's congregation.
    pub fn update(&mut self, dt: f32, time: f32, ctl: &mut dyn Knobs, beat: &tempo::Now, meter: &dyn Fn(usize, usize) -> f32) {
        if let Some(rx) = &self.set_rx {
            match rx.try_recv() {
                Ok(r) => {
                    self.set_rx = None;
                    self.set_arrived(r);
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.set_rx = None,
            }
        }
        self.portraits.poll();
        self.clock = time;
        self.light = ctl.input(self.node, "light", params::default(crate::graph::Kind::Space, "light"));
        self.post_knobs = post::Knobs::read(&*ctl, self.node);
        // the focus follows the zoom: the whole being far out, its head close up (owner 2026-10-31) – or, with a knob
        // selected, that knob (#224: "zooming then brings you closer to that point"); the camera glides there – held
        // still while a hand is on a knob (a press selects it: the knob would slide away under the finger)
        if !matches!(self.drag, Drag::Knob { .. } | Drag::Hold) {
            self.camera.goal = self.focus_goal(self.camera.distance);
        }
        self.camera.step(dt);
        // #209: the knobs face the camera; #224: and fold into its view close up
        self.being.eye = Some(self.camera.eye());
        self.being.frame = Some(self.camera.frame(self.view.0 / self.view.1));
        self.being.update(dt, time, &Local { ctl, meter }, beat);
        self.being.push(ctl);
        let local = Local { ctl, meter };
        if self.drag == Drag::None {
            let reach = if self.finger.is_some() { TOUCH_REACH } else { 1.0 };
            self.being.hover = self.pointer.and_then(|(x, y)| self.being.pick_reach(&self.camera.ray(x, y, self.view.0, self.view.1), reach));
        }
        self.being.dragging = matches!(self.drag, Drag::Knob { .. } | Drag::Hold);
        self.session(time, &local);
        self.congregation(dt, time, beat);
    }

    /// Where the camera's focus goes at `distance` (camera::focus_target): from the whole being's centre far out to its
    /// head close up – to the selected knob instead of the head while one is (#224; where it sits unfolded, so the
    /// knob comes to the middle of the picture as the camera nears it).
    fn focus_goal(&self, distance: f32) -> Vec3 {
        let (body, head) = self.being.focus();
        let near = match self.being.selected {
            Some(Hit::Button(i)) => self.being.buttons[i].free,
            _ => head,
        };
        let s = self.being.look.size;
        camera::focus_target(body, near, distance, s * FOCUS_NEAR, s * FOCUS_FAR)
    }

    /// #224 (owner 2026-10-31: "when it's deselected again, the whole figure is framed at the zoom level that
    /// allows"): the closest distance at which the whole being – its points, its head, its knobs where they sit
    /// unfolded – fits the view, aiming where the focus rule aims there (a bisection: the fit only grows with the
    /// distance); the farthest the camera goes if even that is too close
    pub fn frame_distance(&self) -> f32 {
        let b = &self.being;
        let spheres = || {
            let points = b.points.iter().map(|p| (p.pos, p.radius));
            let knobs = b.buttons.iter().map(|k| (k.free, k.radius * being::RING_REST));
            points.chain(knobs).chain(std::iter::once((b.head.pos, b.head.radius)))
        };
        let aspect = self.view.0 / self.view.1;
        let fits = |d: f32| {
            let mut c = self.camera;
            c.distance = d;
            let (body, head) = b.focus();
            let s = b.look.size;
            c.target = camera::focus_target(body, head, d, s * FOCUS_NEAR, s * FOCUS_FAR);
            c.fits(aspect, spheres(), FRAME_MARGIN)
        };
        let (mut lo, mut hi) = (self.camera.min_distance, self.camera.max_distance);
        if fits(lo) {
            return lo;
        }
        if !fits(hi) {
            return hi;
        }
        for _ in 0..24 {
            let mid = (lo * hi).sqrt();
            if fits(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        hi
    }

    /// The set's 3D has arrived (its loader thread on the deck, the page's fetch in the browser): the being moves to
    /// its spot, the renderer uploads it (`take_set`).
    pub fn set_arrived(&mut self, r: Result<glb::Model>) {
        match r {
            Ok(model) => {
                let spot = model.spot(&self.spot);
                if spot.is_none() && self.at.is_none() {
                    tracing::warn!("set: no spot '{}' – the being floats over the set's centre", self.spot);
                }
                let base = spot.unwrap_or_else(|| Vec3::new((model.min.x + model.max.x) * 0.5, model.min.y, (model.min.z + model.max.z) * 0.5));
                self.being.anchor = self.at.unwrap_or(base + Vec3::Y * self.lift);
                self.camera.look_at(aim(&self.being));
                self.set_status = format!("SET {}K TRIS", model.triangles() / 1000);
                self.spots = model.spots.clone();
                self.model = Some(model);
            }
            Err(e) => {
                tracing::error!("set: {e:#}");
                self.set_status = format!("SET ERROR {e}");
            }
        }
    }

    /// Another being for this deck (the browser's character switcher, #198): its graph and look, floating where the old
    /// one did; the set and the camera stay, the renderer sizes its buffers anew.
    pub fn replace_being(&mut self, graph: &Graph, look: &Look) {
        let mut being = Being::new(graph, look);
        being.head.sprites = self.portraits.request(&look.character, &being.moods);
        being.anchor = self.being.anchor;
        self.being = being;
        self.graph = graph.clone();
        if let Some(n) = graph.first(crate::graph::Kind::Space) {
            self.node = n;
        }
        self.drag = Drag::None;
        self.last_press = None;
        self.pressed = None;
        self.generation += 1;
    }

    /// the longest texture side the set loader keeps, and the set's nodes left out (the space node's `hide`)
    pub fn set_options(&self) -> (u32, &[String]) {
        (self.max_texture, &self.hide)
    }

    /// a deck: its changes to the relay (≤ 30 / s); the conductor: the decks' beings, their values, presence
    fn session(&mut self, time: f32, local: &Local) {
        let Some(l) = &self.link else { return };
        if let Some(s) = self.stream.as_mut() {
            if let Some(c) = s.sample(time, &self.graph, local, &self.being) {
                let values = c.values.into_iter().map(|(k, v)| (k, v.into_iter().map(f64::from).collect())).collect();
                l.send(protocol::DeckValues { values, mood: c.mood });
            }
        }
        while let Some(ev) = self.link.as_ref().and_then(|l| l.recv()) {
            match ev {
                In::Me(id) => self.me = Some(id),
                In::Deck { client, graph, values, mood } => match wire::from_wire(&graph) {
                    Ok((g, mut look)) => {
                        // the conductor's feelings read every deck's values (their nudges stay with the decks)
                        look.feel = self.being.look.feel.clone();
                        tracing::info!("cathedral: deck {client} joined ({} nodes, {})", g.nodes.len(), if look.character.is_empty() { "no face" } else { &look.character });
                        let mut r = Remote::new(client.clone(), g, &look);
                        r.being.head.sprites = self.portraits.request(&look.character, &r.being.moods);
                        r.apply(&values, mood.as_deref());
                        self.add(r);
                    }
                    Err(e) => tracing::warn!("cathedral: deck {client}'s being: {e:#}"),
                },
                In::Values { client, values, mood } => {
                    if let Some(r) = self.others.iter_mut().find(|r| r.client == client) {
                        r.apply(&values, mood.as_deref());
                    }
                }
                In::Pushed(s) => self.sync.pushed(s),
                In::Synced { client, synced } => self.sync.answer(client, synced),
                In::Presence { decks, routes, links, quality } => {
                    self.sync.synced.retain(|c, _| decks.contains(c));
                    for r in &mut self.others {
                        r.link = links.get(&r.client).cloned();
                        r.quality = quality.get(&r.client).cloned();
                    }
                    let before = self.others.len();
                    self.others.retain(|r| decks.contains(&r.client));
                    if self.others.len() != before || routes != self.route_ids {
                        self.route_ids = routes;
                        self.rebuild();
                    }
                }
            }
        }
        if let Some(l) = &self.link {
            self.note = l.status();
        }
    }

    /// #220: this deck's render quality to the session (a deck's link sends it; the conductor sees it next to the link)
    pub fn report_quality(&self, quality: protocol::DeckQuality) {
        if let Some(l) = self.link.as_ref().filter(|_| self.join == Join::Deck) {
            l.quality(quality);
        }
    }

    /// #195: the conductor pushes a scene sync to the decks (the link sends it once joined); false = this space is no
    /// conductor or has no link (the HUD says why)
    pub fn push_sync(&mut self, sync: protocol::SceneSync) -> bool {
        let Some(link) = self.link.as_ref().filter(|_| self.join == Join::Conductor) else { return false };
        tracing::info!("cathedral: pushing the sync of '{}'", sync.scene);
        link.push(sync.clone());
        self.sync.pushed(sync);
        true
    }

    /// #195: what the scene needs, as a push: its set and the characters on stage – this being's and every deck's
    pub fn scene_push(&self, scene: &str) -> protocol::SceneSync {
        let sets: Vec<String> = set_need(&self.graph, self.node).map(|(id, _)| id).into_iter().collect();
        let mut characters: Vec<String> = Vec::new();
        for c in self.beings().map(|b| &b.look.character).filter(|c| !c.is_empty()) {
            if !characters.contains(c) {
                characters.push(c.clone());
            }
        }
        protocol::SceneSync { scene: scene.into(), sets: Some(sets), characters: Some(characters), beings: None, decks: None }
    }

    /// #195, the cue's wait: every deck the push is for is ready (at least one is there)
    pub fn decks_ready(&self) -> bool {
        self.sync.all_ready(self.others.iter().map(|r| r.client.as_str()))
    }

    /// a deck's being joins (or replaces its old one), placed where it will float so it doesn't fly across the nave
    pub fn add(&mut self, r: Remote) {
        let client = r.client.clone();
        self.others.retain(|o| o.client != client);
        self.others.push(r);
        self.others.sort_by(|a, b| a.client.cmp(&b.client));
        self.rebuild();
        // it comes down from the vault into its place
        if let Some(r) = self.others.iter_mut().find(|o| o.client == client) {
            r.being.anchor = r.place + Vec3::Y * 2.0;
        }
    }

    /// the decks' places and the routes between beings (after a join, a leave, new routes)
    fn rebuild(&mut self) {
        self.places.resize(self.others.len(), Vec3::ZERO);
        self.layout();
        let index = |id: &str| -> Option<Option<usize>> {
            if self.me.as_deref() == Some(id) {
                return Some(None);
            }
            self.others.iter().position(|r| r.client == id).map(Some)
        };
        self.routes = self.route_ids.iter().filter_map(|(a, b)| Some(Route::new(index(a)?, index(b)?))).filter(|r| r.from != r.to).collect();
        self.generation += 1;
    }

    /// the decks' places: a circle around this being's anchor (it moves with the set's spot), the first between two
    /// decks toward the camera's start – the view falls on the conductor between them
    fn layout(&mut self) {
        let n = self.others.len().min(self.places.len());
        let facing = std::f32::consts::FRAC_PI_2 - self.camera.home().0 + std::f32::consts::PI / n.max(1) as f32;
        decks::circle(self.being.anchor, self.ring, n, facing, &mut self.places);
        for (r, p) in self.others.iter_mut().zip(&self.places) {
            r.place = *p;
        }
    }

    /// the conductor's frame: every deck's being on its own values and this clock, the hoses between them
    fn congregation(&mut self, dt: f32, time: f32, beat: &tempo::Now) {
        if self.others.is_empty() {
            return;
        }
        self.layout();
        let eye = self.camera.eye();
        for r in &mut self.others {
            r.being.eye = Some(eye);
            r.step(dt, time, beat);
        }
        let center = |i: Option<usize>| i.map_or(self.being.center, |i| self.others[i].being.center);
        for k in 0..self.routes.len() {
            let (a, b) = (center(self.routes[k].from), center(self.routes[k].to));
            let rate = self.routes[k].from.map_or(0.0, |i| self.others[i].rate);
            self.routes[k].step(dt, a, b, rate);
        }
    }

    /// The beings' cores as point lights in the room (#204): at most `out.len()`, the strongest at the eye first –
    /// a being's light (`glow` + its level pulse, × `glow_light`) over 1 + its squared distance to the eye. Each light
    /// = [x, y, z, range, r, g, b, 0] (colour × intensity, linear). Returns how many; never allocates.
    pub fn write_lights(&self, eye: Vec3, out: &mut [[f32; 8]]) -> usize {
        let cap = out.len();
        if cap == 0 || self.glow_light <= 0.0 {
            return 0;
        }
        // (score, the light) kept sorted strongest first in `out`'s own slots
        let mut scores = [0f32; MAX_LIGHTS];
        let cap = cap.min(MAX_LIGHTS);
        let mut n = 0;
        for b in self.beings() {
            let power = b.light_power() * self.glow_light;
            if power <= 1e-4 {
                continue;
            }
            let score = power / (1.0 + (b.center - eye).length_squared());
            let mut at = n;
            while at > 0 && scores[at - 1] < score {
                at -= 1;
            }
            if at >= cap {
                continue;
            }
            let last = n.min(cap - 1);
            for k in (at..last).rev() {
                scores[k + 1] = scores[k];
                out[k + 1] = out[k];
            }
            let c = b.light_rgb();
            scores[at] = score;
            out[at] = [b.center.x, b.center.y, b.center.z, self.glow_range, c[0] * power, c[1] * power, c[2] * power, 0.0];
            n = (n + 1).min(cap);
        }
        n
    }

    /// every being: this deck's own first, then the decks'
    pub fn beings(&self) -> impl Iterator<Item = &Being> {
        std::iter::once(&self.being).chain(self.others.iter().map(|r| &r.being))
    }

    /// This frame's labels (label.rs): the conductor's word over each deck's being (the buttons carry no numbers – owner
    /// 2026-10-31) – at most label::LABELS (`out` cleared first; its capacity from the renderer – no allocation).
    pub fn write_labels(&self, out: &mut Vec<Label>) {
        out.clear();
        // #195: the conductor's word over each deck's being – its sync for the push (first: the decks' state matters), then
        // its render quality (#220) and its link (#223), a line each
        for r in &self.others {
            let head = &r.being.head;
            let mut line = 0.0;
            let mut put = |out: &mut Vec<Label>, text: label::Text, hue: f32| {
                if out.len() < label::LABELS {
                    let c = being::hsv(hue, 0.55, 1.0);
                    out.push(Label { pos: (head.pos + Vec3::Y * head.radius * 1.4).into(), offset: head.radius, text, color: [c[0], c[1], c[2], 0.9], scale: 0.85, line });
                }
                line += 1.0;
            };
            if let Some(view) = self.sync.view(&r.client) {
                let mut text = label::Text::default();
                let _ = view.write(&mut text);
                put(out, text, view.hue());
            }
            let mut text = label::Text::default();
            if let Some(hue) = r.write_quality(&mut text) {
                put(out, text, hue);
            }
            let mut text = label::Text::default();
            if let Some(hue) = r.write_link(&mut text) {
                put(out, text, hue);
            }
        }
    }

    /// what the pointer should look like now
    pub fn cursor(&self) -> Cursor {
        match (self.drag, self.being.hover) {
            (Drag::Knob { .. }, _) => Cursor::Grabbing,
            (Drag::Hold, _) => Cursor::Locked,
            (Drag::None, Some(Hit::Button(i))) if !self.being.buttons[i].turnable => Cursor::Locked,
            (Drag::None, Some(Hit::Button(_))) => Cursor::Grab,
            _ => Cursor::Default,
        }
    }

    /// a world point in window pixels (None behind the eye)
    pub fn to_pixel(&self, p: Vec3) -> Option<(f32, f32)> {
        let clip = self.camera.view_proj(self.view.0 / self.view.1) * p.extend(1.0);
        (clip.w > 1e-4).then(|| ((clip.x / clip.w + 1.0) * 0.5 * self.view.0, (1.0 - clip.y / clip.w) * 0.5 * self.view.1))
    }

    /// this deck's button `node.port` (or `node.shape`), by name – a check run's `--pointer` / `--drag`
    pub fn button(&self, graph: &Graph, name: &str) -> Option<usize> {
        let (id, port) = name.split_once('.')?;
        self.being.buttons.iter().position(|b| graph.nodes[b.node].id == id && b.name == port)
    }

    /// button `i`'s tether on the screen: where its 0 sits and the way to its 1 (pixels); a tether that points at the
    /// eye (shorter than a tenth of the window) turns into a plain upward drag over 40 % of the window's height
    pub fn drag_axis(&self, i: usize) -> ((f32, f32), (f32, f32)) {
        let (lo, hi) = self.being.span(i);
        let up = (0.0, -0.4 * self.view.1);
        match (self.to_pixel(lo), self.to_pixel(hi)) {
            (Some(a), Some(b)) if ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt() > 0.1 * self.view.1 => (a, (b.0 - a.0, b.1 - a.1)),
            (Some(a), _) => (a, up),
            _ => ((0.0, 0.0), up),
        }
    }

    /// how many spheres / tube vertices `write` produces at most
    pub fn capacity(&self) -> (usize, usize) {
        let (mut s, mut v) = (0, 0);
        for b in self.beings() {
            let (bs, bv) = b.capacity();
            s += bs;
            v += bv;
        }
        (s + self.routes.len() * being::BUBBLES, v + self.routes.len() * (being::SEGMENTS + 1) * being::SIDES)
    }

    /// the tubes' count (the index buffer)
    pub fn tubes(&self) -> usize {
        self.beings().map(Being::tubes).sum::<usize>() + self.routes.len()
    }

    /// This frame's geometry (the buffers cleared first, sized by `capacity` – no allocation): the solid spheres of
    /// every being, then the translucent ones; returns how many are solid.
    pub fn write(&self, spheres: &mut Vec<Sphere>, tubes: &mut Vec<TubeVertex>) -> usize {
        spheres.clear();
        tubes.clear();
        for b in self.beings() {
            b.write_solid(spheres);
        }
        let solid = spheres.len();
        let r = Route::radius(self.being.look.size);
        for b in self.beings() {
            b.write_ghosts(spheres);
            b.write_tubes(tubes);
        }
        for route in &self.routes {
            route.write_ghosts(self.being.hue, self.being.glow, r, spheres);
            route.write_tube(r, tubes);
        }
        solid
    }

    /// #211: MIDI from the deck's outputs for the conductor (`deck.midi` over the relay) – a deck with a link only
    pub fn send_midi(&self, msgs: &[[u8; 3]]) {
        if msgs.is_empty() || self.join == Join::Conductor {
            return;
        }
        if let Some(l) = &self.link {
            l.midi(protocol::DeckMidi { msgs: msgs.iter().map(|b| protocol::MidiBytes { bytes: b.to_vec() }).collect() });
        }
    }

    /// the HUD's lines about the session: who this deck is to it, the decks, the faces
    pub fn session_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        let who = match self.join {
            Join::Alone => return vec![format!("FACE {}", self.portraits.status(&self.being.look.character)).to_uppercase()],
            Join::Deck => "DECK",
            Join::Conductor => "CONDUCTOR",
        };
        // the link's status names the role once joined ("CONDUCTOR c6 @ CATH"); the HUD has room for one line
        let first = if self.note.is_empty() { who.to_string() } else if self.note.starts_with(who) { self.note.clone() } else { format!("{who} {}", self.note) };
        out.push(match self.join {
            Join::Conductor => format!("{first} {} DECKS {} ROUTES", self.others.len(), self.routes.len()),
            _ => first,
        });
        if let Some(line) = self.sync.line(self.others.iter().map(|r| r.client.as_str())) {
            out.push(line);
        }
        out.push(format!("FACE {}", self.portraits.status(&self.being.look.character)));
        out.iter().map(|l| l.replace(['–', '—'], "-").to_uppercase().chars().take(40).collect()).collect()
    }

    /// A Live cell's 3D set (#186, #193: set + spot): another set's file loads on its thread (the old one stays until
    /// it is there), and the being and the camera go to `spot` (in the new set once it has arrived). A spot the set
    /// lacks keeps the place – the set's status says so.
    pub fn show(&mut self, set: Option<&Path>, spot: Option<&str>) {
        if let Some(spot) = spot {
            self.spot = spot.to_owned();
            self.at = None;
        }
        match set {
            Some(p) if self.set_path.as_deref() != Some(p) => {
                (self.set_rx, self.set_status) = load_set(p.to_path_buf(), self.max_texture, self.hide.clone(), self.gate.clone());
                self.set_path = Some(p.to_path_buf());
            }
            _ => {
                if let Some(pos) = self.spots.iter().find(|(n, _)| *n == self.spot).map(|(_, p)| *p) {
                    self.being.anchor = pos + Vec3::Y * self.lift;
                    self.camera.look_at(aim(&self.being));
                } else if spot.is_some() && self.set_rx.is_none() {
                    tracing::warn!("set: no spot '{}' – the being stays", self.spot);
                }
            }
        }
    }

    /// the set's 3D, once – the renderer uploads it
    pub fn take_set(&mut self) -> Option<glb::Model> {
        self.model.take()
    }

    /// the window's size in pixels (the pointer's frame)
    pub fn set_view(&mut self, w: f32, h: f32) {
        self.view = (w.max(1.0), h.max(1.0));
    }

    pub fn pointer_moved(&mut self, x: f32, y: f32, ctl: &mut dyn Knobs) {
        match self.drag {
            Drag::Orbit(lx, ly) => {
                self.camera.drag(x - lx, y - ly);
                self.drag = Drag::Orbit(x, y);
            }
            Drag::Knob { button, norm, at, axis } => {
                let b = &self.being.buttons[button];
                // the pointer's way along the tether's screen line: out = up, in = down, the button under the hand
                let along = ((x - at.0) * axis.0 + (y - at.1) * axis.1) / (axis.0 * axis.0 + axis.1 * axis.1).max(1.0);
                let n = (norm + along).clamp(0.0, 1.0);
                if b.shape {
                    ctl.set_lfo_shape(b.node, b.spec.value(n).round() as usize);
                } else {
                    ctl.set(b.node, b.port, b.spec.value(n));
                }
                self.camera.idle = 0.0;
            }
            Drag::Hold | Drag::None => {}
        }
        self.pointer = Some((x, y));
    }

    pub fn pointer_left(&mut self) {
        self.pointer = None;
        self.being.hover = None;
    }

    /// the primary button down: over an unlinked button = pull it (twice quickly: back to the scene's value), over a
    /// linked one = it shakes and the HUD says what drives it, else turn the camera
    pub fn press(&mut self) {
        // a touch screen's own mouse emulation: the finger already pressed
        if self.finger.is_none() {
            self.press_reach(1.0);
        }
    }

    fn press_reach(&mut self, reach: f32) {
        let Some((x, y)) = self.pointer else { return };
        let ray = self.camera.ray(x, y, self.view.0, self.view.1);
        let hit = self.being.pick_reach(&ray, reach);
        self.being.hover = hit;
        // #209: what a tap here selects – a button, the being (a point, the head), or nothing (the master section); a
        // hand on a button selects it at once (the last touched knob owns the sticks)
        let head = self.being.core_point.filter(|_| ray.sphere(self.being.head.pos, self.being.head.radius).is_some()).map(Hit::Point);
        let chosen = hit.or(head);
        self.pressed = Some(((x, y), chosen));
        if let Some(Hit::Button(i)) = chosen {
            self.select(Some(Hit::Button(i)));
        }
        self.drag = match hit {
            Some(Hit::Button(i)) if !self.being.buttons[i].turnable => {
                self.being.shake(i);
                let b = &self.being.buttons[i];
                let id = &self.graph_id(b.node);
                self.notice = Some((format!("{id}.{} IS LINKED < {}", b.name, b.source), self.clock + 2.5));
                Drag::Hold
            }
            Some(Hit::Button(i)) => {
                let double = self.last_press.is_some_and(|(j, t)| j == i && self.clock - t < DOUBLE_CLICK);
                if double {
                    self.last_press = None;
                    self.being.reset(i);
                    Drag::Hold
                } else {
                    self.last_press = Some((i, self.clock));
                    self.being.reset = None;
                    // the hand starts where the value sits on the tether (not where it hit the sphere)
                    let (lo, axis) = self.drag_axis(i);
                    let n = self.being.buttons[i].norm;
                    let at = (x - (lo.0 + axis.0 * n) + lo.0, y - (lo.1 + axis.1 * n) + lo.1);
                    Drag::Knob { button: i, norm: 0.0, at, axis }
                }
            }
            _ => Drag::Orbit(x, y),
        };
    }

    pub fn release(&mut self) {
        if self.finger.is_none() {
            self.let_go();
        }
    }

    /// the hand lifts: a press that barely moved is a tap (#209) – it selects what it hit (nothing: deselect)
    fn let_go(&mut self) {
        if let (Some(((x0, y0), chosen)), Some((x, y))) = (self.pressed.take(), self.pointer) {
            if (x - x0).hypot(y - y0) < TAP * self.view.1 {
                self.select(chosen);
                self.tapped = Some(self.being.selected);
            }
        }
        self.drag = Drag::None;
    }

    /// #209: select a button, the being (`Hit::Point` – any), or nothing (None: the master section) – the renderer rings
    /// it; the page asks for taps with `take_tap`
    /// #224: a knob let go (the being or nothing selected now) frames the whole being – the camera glides out (or in)
    /// to `frame_distance`, the focus back to the being
    pub fn select(&mut self, what: Option<Hit>) {
        let was_knob = matches!(self.being.selected, Some(Hit::Button(_)));
        self.being.selected = match what {
            Some(Hit::Point(_)) => Some(Hit::Point(self.being.core_point.unwrap_or(0))),
            Some(Hit::Button(i)) if i < self.being.buttons.len() => Some(Hit::Button(i)),
            _ => None,
        };
        if was_knob && !matches!(self.being.selected, Some(Hit::Button(_))) {
            self.camera.want = self.frame_distance();
            self.camera.idle = 0.0;
        }
    }

    /// #209: the selection a tap made since the last call (Some(None) = a tap on empty space: deselected)
    pub fn take_tap(&mut self) -> Option<Option<Hit>> {
        self.tapped.take()
    }

    /// #209: a selection by name – `""` nothing, `"being"`, `"knob:node.port"` (`node.shape`) – what the page's
    /// controls (crate::controls) speak; an unknown knob deselects
    pub fn select_named(&mut self, name: &str) {
        let what = match name {
            "being" => Some(Hit::Point(0)),
            _ => name.strip_prefix("knob:").and_then(|p| self.button(&self.graph, p)).map(Hit::Button),
        };
        self.select(what);
    }

    /// #209: the selection's name (`select_named` backwards)
    pub fn selection_name(&self) -> String {
        match self.being.selected {
            Some(Hit::Point(_)) => "being".into(),
            Some(Hit::Button(i)) => {
                let b = &self.being.buttons[i];
                format!("knob:{}.{}", self.graph_id(b.node), b.name)
            }
            None => String::new(),
        }
    }

    /// A finger on a touch screen (winit's `Touch`): the first one drives the pointer – down = a press with a bigger
    /// reach, up = a release; any other finger meanwhile is ignored.
    pub fn touch(&mut self, id: u64, phase: Touch, x: f32, y: f32, ctl: &mut dyn Knobs) {
        match phase {
            Touch::Started if self.finger.is_none() => {
                self.pointer_moved(x, y, ctl);
                self.finger = Some(id);
                self.press_reach(TOUCH_REACH);
            }
            Touch::Moved if self.finger == Some(id) => self.pointer_moved(x, y, ctl),
            Touch::Ended if self.finger == Some(id) => {
                self.pointer_moved(x, y, ctl);
                self.finger = None;
                self.let_go();
                self.pointer_left();
            }
            _ => {}
        }
    }

    /// a node's id in this deck's graph (the HUD)
    fn graph_id(&self, node: usize) -> String {
        self.graph.nodes.get(node).map_or_else(String::new, |n| n.id.clone())
    }

    /// The HUD's line: the hovered button (its node, name, value – a linked one's base and where its hose comes from;
    /// no range – owner 2026-10-31), the hovered
    /// node, else the set's state. ≤ 40 characters.
    pub fn hud_line(&self, graph: &Graph) -> String {
        if let Some((word, until)) = &self.notice {
            if self.clock < *until {
                return word.replace(['–', '—'], "-").to_uppercase().chars().take(40).collect();
            }
        }
        let line = match self.being.hover {
            Some(Hit::Button(i)) => {
                let b = &self.being.buttons[i];
                let id = &graph.nodes[b.node].id;
                if b.shape {
                    let names = crate::params::LFO_SHAPES;
                    return format!("{id}.shape {} [{}]", names[(b.value as usize).min(names.len() - 1)], names.join(" ")).to_uppercase().chars().take(40).collect();
                }
                match graph.link_into(b.node, b.port) {
                    Some(l) => format!("{id}.{} {} < {}{}", b.name, num(b.value), graph.nodes[l.from.0].id, if self.being.dragging { " *" } else { "" }),
                    None => format!("{id}.{} {}{}", b.name, num(b.value), if self.being.dragging { " *" } else { "" }),
                }
            }
            Some(Hit::Point(i)) => {
                let n = &graph.nodes[self.being.points[i].node];
                format!("NODE {} ({:?})", n.id, n.kind)
            }
            None => self.set_status.clone(),
        };
        line.to_uppercase().chars().take(40).collect()
    }
}

/// a value in ≤ 6 characters
fn num(v: f32) -> String {
    let mut s = String::new();
    let _ = label::write_num(&mut s, v);
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Scene, CATHEDRAL_SCENE};
    use values::Table;

    #[test]
    fn set_paths_follow_the_quality() {
        let g = Graph::build(&Scene::parse("[[node]]\nid=\"s\"\nkind=\"space\"\nset=\"cathedral\"\nquality=\"medium\"\n").unwrap()).unwrap();
        assert_eq!(set_path(&g, 0, Path::new("/r")), Some(Path::new("/r").join("cathedral").join("cathedral.medium.glb")));
        assert_eq!(set_need(&g, 0), Some(("cathedral".into(), "medium".into())));
        let g = Graph::build(&Scene::parse("[[node]]\nid=\"s\"\nkind=\"space\"\nset=\"x/plain.glb\"\n").unwrap()).unwrap();
        assert_eq!(set_path(&g, 0, Path::new("/r")), Some(Path::new("/r").join("x/plain.glb")));
        assert_eq!(set_need(&g, 0), None, "a path is not synced");
        let g = Graph::build(&Scene::parse("[[node]]\nid=\"s\"\nkind=\"space\"\n").unwrap()).unwrap();
        assert_eq!(set_path(&g, 0, Path::new("/r")), None);
    }

    /// the owner (2026-10-30): the whole mood set, every mood reachable – each has a rule and a nudge in both bundled
    /// scenes, all of them resolve; the conductor's rules read a deck's graph too
    #[test]
    fn every_mood_has_a_rule_and_a_nudge() {
        use crate::scene::CONDUCTOR_SCENE;
        let cathedral = Scene::parse(CATHEDRAL_SCENE).unwrap();
        let deck = Graph::build(&cathedral).unwrap();
        for text in [CATHEDRAL_SCENE, CONDUCTOR_SCENE] {
            let scene = Scene::parse(text).unwrap();
            let g = Graph::build(&scene).unwrap();
            let b = Being::new(&g, &scene.being);
            assert!(b.feel.problems.is_empty(), "{}: {:?}", scene.stage.name, b.feel.problems);
            for (mood, _) in portrait::NEAR {
                assert!(scene.being.feel.iter().any(|r| r.mood == mood), "{}: no rule for {mood}", scene.stage.name);
                assert!(scene.being.react.iter().any(|n| n.mood == mood), "{}: no nudge for {mood}", scene.stage.name);
            }
            assert_eq!(b.moods.len(), portrait::NEAR.len(), "the palette is the whole set");
        }
        let conductor = Scene::parse(CONDUCTOR_SCENE).unwrap();
        let look = being::Look { feel: conductor.being.feel.clone(), ..cathedral.being.clone() };
        let mirrored = Being::new(&deck, &look);
        assert!(mirrored.feel.problems.is_empty(), "the conductor's rules on a deck's graph: {:?}", mirrored.feel.problems);
    }

    /// #188 (owner 2026-10-31: the nudges sounded like dropouts): in both bundled scenes no nudge ducks a gain deeper than
    /// 6 dB below where it rests, and every nudge on the sound takes ≥ 0.15 s to arrive
    #[test]
    fn the_nudges_are_gentle_on_the_sound() {
        use crate::scene::CONDUCTOR_SCENE;
        for text in [CATHEDRAL_SCENE, CONDUCTOR_SCENE] {
            let scene = Scene::parse(text).unwrap();
            let g = Graph::build(&scene).unwrap();
            for n in &scene.being.react {
                let Some(values::Path::Input(node, port)) = values::resolve_loose(&g, &n.set) else { panic!("{}", n.set) };
                let kind = g.nodes[node].kind;
                if kind.domain() != crate::graph::Domain::Audio {
                    continue;
                }
                let name = kind.inputs()[port].name;
                assert!(n.attack >= 0.15, "{}: {} {} attacks in {} s", scene.stage.name, n.mood, n.set, n.attack);
                if name.starts_with("gain") || name == "volume" {
                    let rest = g.nodes[node].f32(name, params::default(kind, name));
                    let to = n.to.unwrap_or(rest + n.by.unwrap_or(0.0));
                    assert!(to >= rest * 0.5 - 1e-6, "{}: {} ducks {} from {rest} to {to}", scene.stage.name, n.mood, n.set);
                }
            }
        }
    }

    fn cathedral() -> (Graph, Space, Table) {
        let scene = Scene::parse(CATHEDRAL_SCENE).unwrap();
        let g = Graph::build(&scene).unwrap();
        let node = g.first(crate::graph::Kind::Space).unwrap();
        let mut space = Space::new(&g, node, &scene.being, Path::new("/nowhere"), Portraits::manual());
        let mut ctl = Table::new(&g);
        space.set_view(1280.0, 720.0);
        space.update(0.016, 0.0, &mut ctl, &tempo::Now::default(), &|_, _| 0.0);
        (g, space, ctl)
    }

    /// a frame at the show's time `t` (the double click reads it)
    fn frame(space: &mut Space, ctl: &mut Table, t: f32) {
        space.update(1.0 / 60.0, t, ctl, &tempo::Now::default(), &|_, _| 0.0);
    }

    /// #224: three seconds of frames from the show's time `t` – the glides arrive
    fn settle(space: &mut Space, ctl: &mut Table, t: f32) {
        for k in 0..180 {
            frame(space, ctl, t + k as f32 / 60.0);
        }
    }

    /// #224 (owner 2026-10-31: "when you tap one, that node is selected, and zooming then brings you closer to that
    /// point. When it's deselected again, the whole figure is framed"): a selected knob draws the focus as the camera
    /// zooms in and holds it there steadily; a deselect glides back – the focus to the being, the distance out to the
    /// closest one where the whole being fits – without overshooting
    #[test]
    fn the_focus_follows_a_selected_knob_and_a_deselect_frames_the_whole_being() {
        let (g, mut space, mut ctl) = cathedral();
        let i = space.button(&g, "voice.index").unwrap();
        space.select(Some(Hit::Button(i)));
        let before = space.camera.target;
        frame(&mut space, &mut ctl, 0.0);
        assert!(space.camera.target.distance(before) < 0.05, "no jump at the tap: {}", space.camera.target.distance(before));
        space.camera.zoom(100.0);
        settle(&mut space, &mut ctl, 0.1);
        let knob = space.being.buttons[i].free;
        let s = space.being.look.size;
        assert!(space.camera.target.distance(knob) < 0.02 * s, "close up: the knob ({})", space.camera.target.distance(knob));
        let (x, y) = space.to_pixel(space.being.buttons[i].pos).unwrap();
        assert!((x - 640.0).abs() < 64.0 && (y - 360.0).abs() < 36.0, "in the middle of the picture: {x} {y}");
        // it holds there: no hunting between the camera and the knob's camera-facing layout
        let mut last = space.camera.target;
        for k in 0..60 {
            frame(&mut space, &mut ctl, 3.2 + k as f32 / 60.0);
            assert!(space.camera.target.distance(last) < 0.004 * s, "steady: {}", space.camera.target.distance(last));
            last = space.camera.target;
        }
        // deselect (a tap on empty space): framed – the closest distance where everything fits
        let close = space.camera.distance;
        space.select(None);
        let want = space.camera.want;
        assert!(want > close * 1.5 && want < space.camera.max_distance, "out to the frame: {close} → {want}");
        assert_eq!(want, space.frame_distance());
        let mut d = space.camera.distance;
        for k in 0..240 {
            frame(&mut space, &mut ctl, 5.0 + k as f32 / 60.0);
            assert!(space.camera.distance >= d && space.camera.distance <= want, "out, never past: {d} → {}", space.camera.distance);
            d = space.camera.distance;
        }
        assert_eq!(space.camera.distance, want);
        let (body, head) = space.being.focus();
        let goal = camera::focus_target(body, head, want, s * FOCUS_NEAR, s * FOCUS_FAR);
        assert!(space.camera.target.distance(goal) < 0.01 * s, "the focus back on the being");
        let b = &space.being;
        let all = b.points.iter().map(|p| (p.pos, p.radius)).chain(b.buttons.iter().map(|k| (k.free, k.radius))).chain([(b.head.pos, b.head.radius)]);
        assert!(space.camera.fits(16.0 / 9.0, all, 0.0), "the whole being in the picture");
        // and no further than needed: a step closer something leaves the frame
        let mut c = space.camera;
        c.distance = want * 0.9;
        c.target = camera::focus_target(body, head, c.distance, s * FOCUS_NEAR, s * FOCUS_FAR);
        let all = b.points.iter().map(|p| (p.pos, p.radius)).chain(b.buttons.iter().map(|k| (k.free, k.radius * being::RING_REST))).chain([(b.head.pos, b.head.radius)]);
        assert!(!c.fits(16.0 / 9.0, all, FRAME_MARGIN), "the closest such distance");
        // selecting the being after a knob frames it too; the being after the being changes nothing
        space.select(Some(Hit::Button(i)));
        space.camera.zoom(100.0);
        space.select(Some(Hit::Point(0)));
        assert_eq!(space.camera.want, space.frame_distance());
        space.camera.zoom(3.0);
        let w = space.camera.want;
        space.select(None);
        assert_eq!(space.camera.want, w, "no knob was let go");
    }

    /// #224 (owner 2026-10-31: "When we zoom in on the character, to the portrait, we can't reach some parameters any
    /// more"): at the portrait every knob is on the screen – the ones that would sit outside fold in toward the edge,
    /// off it by a thumb's room – on a wide window and on a phone held upright; and each can still be tapped
    #[test]
    fn at_the_portrait_every_knob_is_on_the_screen() {
        for (w, h) in [(1280.0, 720.0), (390.0, 845.0)] {
            let (_, mut space, mut ctl) = cathedral();
            space.set_view(w, h);
            space.camera.zoom(100.0);
            settle(&mut space, &mut ctl, 0.1);
            let mut folded = 0;
            let bs = &space.being.buttons;
            for (i, a) in bs.iter().enumerate() {
                // (only where one of them folded – the spread's job; the free layout is #209's)
                for b in bs[i + 1..].iter().filter(|b| a.pos != a.free || b.pos != b.free) {
                    let (p, q) = (space.to_pixel(a.pos).unwrap(), space.to_pixel(b.pos).unwrap());
                    let gap = ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();
                    let r = |k: &being::Button| space.to_pixel(k.pos + space.camera.frame(1.0).up * k.radius).map_or(0.0, |t| (t.1 - space.to_pixel(k.pos).unwrap().1).abs());
                    assert!(gap > (r(a) + r(b)) * 0.9, "{w}×{h}: {} and {} overlap ({gap:.0} px)", a.name, b.name);
                }
            }
            for (i, b) in space.being.buttons.iter().enumerate() {
                let (x, y) = space.to_pixel(b.pos).expect("in front of the eye");
                let inside = |x: f32, y: f32, m: f32| x > w * m && x < w * (1.0 - m) && y > h * m && y < h * (1.0 - m);
                assert!(inside(x, y, 0.04), "{w}×{h}: knob {i} ({}) at {x:.0} {y:.0}", b.name);
                if !space.to_pixel(b.free).is_some_and(|(x, y)| inside(x, y, 0.0)) {
                    folded += 1;
                }
            }
            assert!(folded > 0, "{w}×{h}: some knobs would be off the screen unfolded");
            // every one is tapped where it is shown
            for i in 0..space.being.buttons.len() {
                let (x, y) = space.to_pixel(space.being.buttons[i].pos).unwrap();
                space.pointer_moved(x, y, &mut ctl);
                space.press();
                space.release();
                assert_eq!(space.being.selected, Some(Hit::Button(i)), "{w}×{h}: knob {i} ({}) is tapped", space.being.buttons[i].name);
                space.select(None);
                space.camera.zoom(100.0);
                settle(&mut space, &mut ctl, 4.0 + i as f32 * 3.0);
            }
        }
        // at the start view #209's layout stays: most knobs untouched, those near the edge nudged a little, none shrunk
        let (_, space, _) = cathedral();
        let mut still = 0;
        for b in &space.being.buttons {
            let (p, q) = (space.to_pixel(b.pos).unwrap(), space.to_pixel(b.free).unwrap());
            let moved = ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();
            assert!(moved < 0.05 * 720.0 && b.radius == b.base, "{} moved {moved:.0} px at home", b.name);
            still += (moved < 0.5) as usize;
        }
        assert!(still * 2 > space.being.buttons.len(), "most stay where #209 put them: {still}");
    }

    #[test]
    fn a_drag_pulls_a_button_along_its_tether_and_elsewhere_turns_the_camera() {
        let (g, mut space, mut ctl) = cathedral();
        let i = space.button(&g, "voice.index").expect("the FM voice's index");
        assert!(!space.being.buttons[i].linked);
        let (x, y) = space.to_pixel(space.being.buttons[i].pos).unwrap();
        space.pointer_moved(x, y, &mut ctl);
        frame(&mut space, &mut ctl, 0.0);
        assert_eq!(space.being.hover, Some(Hit::Button(i)), "the button under the pointer");
        assert_eq!(space.cursor(), Cursor::Grab);
        let (node, port, before) = (space.being.buttons[i].node, space.being.buttons[i].port, space.being.buttons[i].norm);
        let (_, axis) = space.drag_axis(i);
        assert!((axis.0 * axis.0 + axis.1 * axis.1).sqrt() > 72.0, "the tether reads on the screen: {axis:?}");
        space.press();
        assert_eq!(space.cursor(), Cursor::Grabbing);
        // #224: the press selected it, but the focus holds still under the hand
        let goal = space.camera.goal;
        frame(&mut space, &mut ctl, 0.05);
        assert_eq!(space.camera.goal, goal, "the focus holds while the knob is held");
        // a quarter of the tether's way out on the screen = a quarter of the range up
        space.pointer_moved(x + axis.0 * 0.25, y + axis.1 * 0.25, &mut ctl);
        let spec = params::spec(g.nodes[node].kind, "index");
        let want = spec.value((before + 0.25).min(1.0));
        assert!((ctl.input(node, "index", -1.0) - want).abs() < 1e-3 * want.abs().max(1.0));
        // the other way, past its end: clamped at the range's floor
        space.pointer_moved(x - axis.0 * 3.0, y - axis.1 * 3.0, &mut ctl);
        assert_eq!(ctl.input(node, "index", -1.0), spec.min);
        let _ = port;
        frame(&mut space, &mut ctl, 0.1);
        assert!(space.hud_line(&g).starts_with("VOICE.INDEX"));
        space.release();
        // an empty spot: the camera turns
        let yaw = space.camera.yaw;
        space.pointer_moved(5.0, 5.0, &mut ctl);
        frame(&mut space, &mut ctl, 0.2);
        space.press();
        space.pointer_moved(105.0, 5.0, &mut ctl);
        assert!(space.camera.yaw != yaw);
        space.release();
        assert!(space.hud_line(&g).starts_with("SET ERROR") || space.hud_line(&g).starts_with("LOADING"));
    }

    /// two presses on a button within DOUBLE_CLICK: it eases back to the scene's value (cathedral: voice.index = 2)
    #[test]
    fn a_double_click_resets_a_button_to_the_scene() {
        let (g, mut space, mut ctl) = cathedral();
        let i = space.button(&g, "voice.index").unwrap();
        let (node, port, scene) = (space.being.buttons[i].node, space.being.buttons[i].port, space.being.buttons[i].scene);
        assert_eq!(scene, g.nodes[node].f32("index", -1.0));
        ctl.set(node, port, 8.0);
        frame(&mut space, &mut ctl, 1.0);
        let at = |space: &Space| space.to_pixel(space.being.buttons[i].pos).unwrap();
        let (x, y) = at(&space);
        space.pointer_moved(x, y, &mut ctl);
        space.press();
        space.release();
        frame(&mut space, &mut ctl, 1.1);
        assert!(space.being.reset.is_none(), "one click is a grab");
        let (x, y) = at(&space);
        space.pointer_moved(x, y, &mut ctl);
        space.press();
        assert!(space.being.reset.is_some(), "the second, quickly: back to the scene");
        space.release();
        for f in 0..40 {
            frame(&mut space, &mut ctl, 1.2 + f as f32 / 60.0);
        }
        assert!((ctl.input(node, "index", -1.0) - scene).abs() < 1e-3);
        // two clicks far apart: no reset
        ctl.set(node, port, 8.0);
        frame(&mut space, &mut ctl, 5.0);
        let (x, y) = at(&space);
        space.pointer_moved(x, y, &mut ctl);
        space.press();
        space.release();
        frame(&mut space, &mut ctl, 6.0);
        space.press();
        space.release();
        assert!(space.being.reset.is_none());
    }

    /// owner 2026-10-31: the focus follows the zoom – far out the whole being, close up its head; R home still works
    #[test]
    fn the_camera_focuses_by_the_zoom() {
        let (_, mut space, mut ctl) = cathedral();
        let (body, head) = space.being.focus();
        assert!(head.y > body.y + 0.2, "the head above the body's centre");
        space.camera.zoom(100.0);
        settle(&mut space, &mut ctl, 0.1); // the zoom's and the focus' glides (#224)
        let (_, head) = space.being.focus();
        assert!(space.camera.target.distance(head) < 1e-3, "close up: a portrait");
        space.camera.zoom(-100.0);
        settle(&mut space, &mut ctl, 3.0);
        let (body, _) = space.being.focus();
        assert!(space.camera.target.distance(body) < 1e-3, "far out: the whole being");
        space.camera.reset();
        settle(&mut space, &mut ctl, 6.0);
        let t = space.camera.target;
        assert!(t.y > body.y && t.y < head.y, "home: between, nearer the body ({} {} {})", body.y, t.y, head.y);
        assert_eq!(space.camera.distance, space.camera.home().2);
    }

    /// #188: a linked knob grabbed turns its base – the HUD names its base and what drives it, the camera stays
    #[test]
    fn a_linked_knob_drags_its_base() {
        let (g, mut space, mut ctl) = cathedral();
        let i = space.button(&g, "incense.cutoff").unwrap();
        assert!(space.being.buttons[i].linked);
        let (node, port) = (space.being.buttons[i].node, space.being.buttons[i].port);
        frame(&mut space, &mut ctl, 0.4);
        let (x, y) = space.to_pixel(space.being.buttons[i].pos).unwrap();
        space.pointer_moved(x, y, &mut ctl);
        frame(&mut space, &mut ctl, 0.5);
        assert_eq!(space.cursor(), Cursor::Grab, "it turns");
        assert!(space.hud_line(&g).starts_with("INCENSE.CUTOFF 600.0 < SWAY"), "{}", space.hud_line(&g));
        let yaw = space.camera.yaw;
        space.press();
        assert_eq!(space.cursor(), Cursor::Grabbing);
        // #224: the press selected it, but the focus holds still under the hand
        let goal = space.camera.goal;
        frame(&mut space, &mut ctl, 0.05);
        assert_eq!(space.camera.goal, goal, "the focus holds while the knob is held");
        let (_, axis) = space.drag_axis(i);
        space.pointer_moved(x + axis.0 * 0.3, y + axis.1 * 0.3, &mut ctl);
        let base = ctl.base(node, port).unwrap();
        assert!(base > 600.0, "the base went up: {base}");
        assert_eq!(space.camera.yaw, yaw);
        space.release();
        // the modulation still on top of the new base
        ctl.step(1.0);
        assert!(ctl.input(node, "cutoff", -1.0) >= base);
        // a mirrored deck's linked knob (no base on the stream) can't be turned: it shakes and says why
        let mirror = values::Mirror::new(&g);
        let mut m = Being::new(&g, &space.being.look);
        m.update(0.016, 0.0, &mirror, &tempo::Now::default());
        assert!(!m.buttons[i].turnable);
    }

    /// a finger reaches a button the mouse would miss; it drags like the mouse; the screen's mouse emulation is ignored
    #[test]
    fn a_finger_has_a_bigger_reach() {
        let (g, mut space, mut ctl) = cathedral();
        let i = space.button(&g, "voice.index").unwrap();
        let b = &space.being.buttons[i];
        // beside the button, across its chain on the side away from its neighbours: outside the mouse's reach (1.5
        // radii), inside a finger's (3)
        let across = (b.pos - space.camera.eye()).cross(b.dir).normalize();
        let clear = |p: Vec3| space.being.buttons.iter().enumerate().filter(|(k, _)| *k != i).map(|(_, q)| q.pos.distance(p)).fold(f32::MAX, f32::min);
        let side = [across, -across].into_iter().max_by(|a, c| clear(b.pos + *a * b.radius * 2.3).total_cmp(&clear(b.pos + *c * b.radius * 2.3))).unwrap();
        let (x, y) = space.to_pixel(b.pos + side * b.radius * 2.3).unwrap();
        space.pointer_moved(x, y, &mut ctl);
        assert_ne!(space.being.pick(&space.camera.ray(x, y, 1280.0, 720.0)), Some(Hit::Button(i)), "the mouse misses it");
        space.touch(7, Touch::Started, x, y, &mut ctl);
        assert_eq!(space.being.hover, Some(Hit::Button(i)), "the finger got it");
        assert_eq!(space.cursor(), Cursor::Grabbing);
        space.press(); // the emulated mouse press: ignored
        let (node, before) = (space.being.buttons[i].node, ctl.input(space.being.buttons[i].node, "index", -1.0));
        let (_, axis) = space.drag_axis(i);
        space.touch(8, Touch::Moved, x + axis.0, y + axis.1, &mut ctl); // another finger: ignored
        assert_eq!(ctl.input(node, "index", -1.0), before);
        space.touch(7, Touch::Moved, x + axis.0 * 0.5, y + axis.1 * 0.5, &mut ctl);
        assert!(ctl.input(node, "index", -1.0) > before);
        space.touch(7, Touch::Ended, x, y, &mut ctl);
        assert_eq!((space.cursor(), space.being.hover), (Cursor::Default, None));
    }

    /// #204: every being's core lights the room – the strongest at the eye first, at most as many as there are slots,
    /// coloured by its hue, stronger with its glow and its level's pulse; `glow_light = 0` turns it off
    #[test]
    fn the_glow_lights_the_room() {
        let (g, mut space, _) = cathedral();
        let eye = space.being.center + Vec3::new(0.0, 0.0, 3.0);
        let mut out = [[0.0; 8]; MAX_LIGHTS];
        assert_eq!(space.write_lights(eye, &mut out), 1);
        let power = space.being.glow * space.glow_light;
        let c = space.being.light_rgb();
        assert!((out[0][4] - c[0] * power).abs() < 1e-5 && out[0][3] == space.glow_range);
        // a congregation: ten decks' beings, near and far, bright and dim
        let scene = Scene::parse(CATHEDRAL_SCENE).unwrap();
        for k in 0..10 {
            let mut r = decks::Remote::new(format!("deck{k}"), g.clone(), &scene.being);
            r.being.center = eye + Vec3::new(k as f32, 0.0, 0.0);
            r.being.glow = if k == 9 { 50.0 } else { 0.5 };
            space.others.push(r);
        }
        let n = space.write_lights(eye, &mut out);
        assert_eq!(n, MAX_LIGHTS, "capped");
        let score = |l: &[f32; 8]| (l[4] + l[5] + l[6]) / (1.0 + (Vec3::new(l[0], l[1], l[2]) - eye).length_squared());
        assert!(out.windows(2).all(|w| score(&w[0]) >= score(&w[1]) - 1e-6), "strongest first");
        assert!(out.iter().any(|l| (l[0] - (eye.x + 9.0)).abs() < 1e-4), "the far but bright one is in");
        assert!(!out.iter().any(|l| (l[0] - (eye.x + 8.0)).abs() < 1e-4), "the far dim one is out");
        let mut few = [[0.0; 8]; 2];
        assert_eq!(space.write_lights(eye, &mut few), 2);
        assert_eq!(few[0], out[0]);
        // the pulse swells it
        let before = space.being.light_power();
        space.being.pulse = 0.5;
        assert!(space.being.light_power() > before);
        space.glow_light = 0.0;
        assert_eq!(space.write_lights(eye, &mut out), 0);
    }

    /// #204: the post knobs are the space node's control inputs, but no buttons on the core unless `expose` names them
    #[test]
    fn the_post_knobs_are_no_buttons_unless_exposed() {
        let (g, space, _) = cathedral();
        assert!(space.post.contains(post::Effect::Bloom), "the cathedral lists bloom");
        let names: Vec<&str> = space.being.buttons.iter().filter(|b| b.node == space.node).map(|b| b.name).collect();
        assert_eq!(names, ["hue", "glow", "light", "pulse"]);
        let scene = Scene::parse(CATHEDRAL_SCENE).unwrap();
        let look = being::Look { expose: vec!["space".into(), "space.bloom".into()], ..scene.being };
        let b = Being::new(&g, &look);
        let names: Vec<&str> = b.buttons.iter().filter(|b| b.node == space.node).map(|b| b.name).collect();
        assert_eq!(names, ["hue", "glow", "light", "pulse", "bloom"]);
    }

    /// #209 (owner 2026-10-31: "the last clicked / tapped thing controls the x / y sticks … tapping empty space
    /// deselects"): a tap on a knob selects it, one on the head the being, one on empty space nothing; a drag that
    /// orbits changes nothing; the page reads the taps by name
    #[test]
    fn a_tap_selects_and_empty_space_deselects() {
        let (g, mut space, mut ctl) = cathedral();
        let i = space.button(&g, "voice.index").unwrap();
        let (x, y) = space.to_pixel(space.being.buttons[i].pos).unwrap();
        space.pointer_moved(x, y, &mut ctl);
        space.press();
        space.release();
        assert_eq!(space.being.selected, Some(Hit::Button(i)));
        assert_eq!(space.take_tap(), Some(Some(Hit::Button(i))));
        assert_eq!(space.take_tap(), None, "a tap is told once");
        assert_eq!(space.selection_name(), "knob:voice.index");
        // the head: the whole being
        let (hx, hy) = space.to_pixel(space.being.head.pos + Vec3::Y * space.being.head.radius * 0.5).unwrap();
        space.pointer_moved(hx, hy, &mut ctl);
        space.press();
        space.release();
        assert_eq!(space.selection_name(), "being");
        // an orbit from empty space keeps it; a tap there deselects
        space.pointer_moved(5.0, 5.0, &mut ctl);
        space.press();
        space.pointer_moved(200.0, 40.0, &mut ctl);
        space.release();
        assert_eq!(space.selection_name(), "being", "a drag is no tap");
        space.pointer_moved(5.0, 5.0, &mut ctl);
        space.press();
        space.release();
        assert_eq!((space.selection_name(), space.take_tap()), (String::new(), Some(None)));
        space.select_named("knob:incense.cutoff");
        assert_eq!(space.selection_name(), "knob:incense.cutoff");
        frame(&mut space, &mut ctl, 1.0);
        let mut spheres = Vec::with_capacity(space.capacity().0);
        let mut tubes = Vec::with_capacity(space.capacity().1);
        space.write(&mut spheres, &mut tubes);
        assert!(spheres.iter().any(|s| s.extra[0] == being::kind::RING && s.color == [1.0, 1.0, 1.0, 0.9]), "the selection's white ring");
        assert!(spheres.len() <= space.capacity().0);
    }
}
