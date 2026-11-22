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
pub mod anatomy;
pub mod backdrop;
pub mod being;
pub mod camera;
pub mod decks;
pub mod dop;
pub mod feel;
pub mod focus;
pub mod glb;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod label;
pub mod layers;
pub mod meshopt;
pub mod motion;
pub mod pick;
pub mod place;
pub mod portrait;
pub mod post;
#[cfg(feature = "gpu")]
pub mod post_gpu;
pub mod spots;
pub mod stream;
pub mod sync;
pub mod values;
pub mod wire;

use crate::gesture::{Act, Fingers};
use crate::graph::Graph;
use crate::params;
use crate::tempo;
use anyhow::Result;
use crate::cast::{self, Form, Who};
use being::{Being, Hit, Look, Sphere, TubeVertex};
use label::Label;
use camera::{Orbit, Shot};
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
/// a view as the orbit camera looks through it
fn shot_of(v: &dop::View) -> Shot {
    Shot { target: v.target, yaw: v.yaw, pitch: v.pitch, distance: v.distance, fov_y: v.lens.fov_y(), ortho: v.lens.ortho().then_some(v.lens.size) }
}

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
/// #517: the address actions and story events kept for the playing scene until its host takes them
const CAST_CAP: usize = 64;
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

pub use crate::gesture::Touch;

/// One holder's being on a stage of several (#492): the scene's `look` node drawn at its holder's spot – as its 3D being
/// or, where there is no 3D for it, as its 2D card (#495, cast.rs).
pub struct Cast {
    pub being: Being,
    /// the holder (a placeholder's id, or a zone's)
    pub holder: String,
    /// the set's spot it stands at; none = the scene's spot
    pub spot: Option<String>,
    /// where it stands from that spot (a spawn zone's arrangement), metres
    pub at: Vec3,
    /// #495: who stands there, and as what (cast.rs `form`: the stage's 3D and who it is)
    pub who: Who,
    pub form: Form,
    /// the form its holder chose (its look node's `form`); none = its look's
    pub chosen: Option<Form>,
    /// the form its character's look takes (scene-content's `form`, `set_looks`); none = the stage's
    pub look: Option<Form>,
    /// where its feet are: the spot on the set's floor (the being floats `lift` above it)
    pub feet: Vec3,
    /// its card's pictures: a character's full view per mood of its being, an item's one picture
    pub pictures: Vec<usize>,
    /// where its motion module puts it this frame (none: its spot)
    pub moved: Option<motion::Pose>,
}

/// One card on the stage this frame (#495): an upright picture with its feet on its spot, turned to the eye around the
/// vertical – `right` is half its width, `up` its height; its being's light on it
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Card {
    pub feet: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    /// the picture's slot (Portraits); None = not here (yet): a blank standee in its being's colour
    pub picture: Option<usize>,
    pub glow: f32,
    pub hue: f32,
    pub pulse: f32,
}

/// between the cast's beings when the set has no spot for them (or there is no set): across the view, metres
const CAST_SPACING: f32 = 1.4;
/// #495: a card's width over its height while its picture is not here (a figure's standee)
const CARD_ASPECT: f32 = 0.45;
/// #495: how high a bounce of 1 lifts a card, a fraction of its height
const CARD_HOP: f32 = 0.15;

impl Cast {
    /// the cast of `graph`: a being per `look` node – the node's look (its `being` table), its body the nodes of its rig
    /// (the ids under its prefix, `<prefix>look`), its core the look node; empty = the scene's one being is the space's
    pub fn of(graph: &Graph, portraits: &mut Portraits) -> Vec<Cast> {
        let looks: Vec<(usize, &str)> = graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == crate::graph::Kind::Look)
            .map(|(i, n)| (i, n.id.strip_suffix(crate::scene::look_id("").as_str()).unwrap_or_default()))
            .collect();
        // a node is the being's whose prefix is the longest that names it (holders `a` and `a_b`: `a_b_voice` is a_b's)
        let owner = |k: usize| looks.iter().filter(|(_, p)| !p.is_empty() && graph.nodes[k].id.starts_with(p)).max_by_key(|(_, p)| p.len()).map(|(i, _)| *i);
        looks
            .iter()
            .map(|&(i, _)| {
                let n = &graph.nodes[i];
                let look: Look = n.params.get("being").and_then(|v| v.clone().try_into().ok()).unwrap_or_default();
                let mut being = Being::part(graph, &look, Some(i), &|k| owner(k) == Some(i));
                being.head.sprites = portraits.request(&look.character, &being.moods);
                let at = match n.f32s("at").as_slice() {
                    [x, y, z] => Vec3::new(*x, *y, *z),
                    _ => Vec3::ZERO,
                };
                // #495: who it is (a look node of #492 without `who` is its character's)
                let who = n.str("who").and_then(Who::parse).unwrap_or_else(|| Who::Character(look.character.clone()));
                let pictures = match &who {
                    Who::Character(c) => portraits.request_view(c, cast::CARD_VIEW, &being.moods),
                    Who::Item(id) => vec![portraits.picture(cast::Picture::item(id))],
                };
                let (holder, spot) = (n.str("holder").unwrap_or_default().to_string(), n.str("spot").map(String::from));
                let chosen = n.str("form").and_then(Form::parse);
                Cast { being, holder, spot, at, who, form: Form::Card, chosen, look: None, feet: Vec3::ZERO, pictures, moved: None }
            })
            .collect()
    }
}

pub struct Space {
    /// the space node
    pub node: usize,
    /// the scene's being – the space's own (a rig played alone, a scene's `[being]`); on a stage of several holders a
    /// bodiless one the camera aims by, the beings are the cast's
    pub being: Being,
    /// #492: one being per character holder on a stage of several (the scene's `look` nodes), each at its holder's spot
    /// and moved only by its own rig's links to its look; drawn and lit, turned by hand once picked (#496)
    pub cast: Vec<Cast>,
    /// #496: the cast member picked by a click on the stage (pick.rs) – its knobs are the hand's; None = the space's own
    picked: Option<usize>,
    pub camera: Orbit,
    /// the set's 3D, from its loader until the renderer takes it
    set_rx: Option<mpsc::Receiver<Result<glb::Model>>>,
    model: Option<glb::Model>,
    /// the set's 3D file shown (or loading) and its spots (kept after the renderer took it – a Live cell moves the
    /// camera to one, #186)
    set_path: Option<PathBuf>,
    /// #495: the set's 3D could not be had – the stage is not 3D, its cast stands as cards
    set_failed: bool,
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
    /// the fingers on a touch screen: the first drives the pointer, two pinch (gesture.rs)
    fingers: Fingers,
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
    /// the outlet (column) this deck plays for: an address filtered to a column's group reaches the link by it
    pub column: Option<String>,
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
    /// #517: what came for the deck's playing scene (`In::Address`, `In::Story`), oldest first, until its host takes it
    /// (`take_cast`); at most `CAST_CAP` – a host that takes nothing loses the oldest
    cast_in: std::collections::VecDeque<In>,
    /// #517: what the link was last told about the story (`ask_story`)
    story: bool,
    ring: f32,
    /// bumped when beings or routes come or go (the renderer sizes its buffers anew)
    pub generation: u64,
    places: Vec<Vec3>,
    /// #195: the conductor's push and the decks' answers
    pub sync: sync::Board,
    pub backdrop: backdrop::Backdrop,
    pub layers: layers::Layers,
    /// the scene's primary camera playing: its shots and moves
    pub dop: dop::Dop,
    /// a camera word for a scene not (yet) on this stage: tried once more when the next scene's camera arrives
    camera_pending: Option<(String, String, Option<f64>)>,
    /// the modules on the camera's and the members' slots
    pub motions: motion::Motions,
    /// when the scene's row started (s, beat): what a motion module counts its row time from
    row_from: Option<(f32, f64)>,
    /// the scene camera's focus this frame: metres from the eye
    pub focus: f32,
    /// the subjects whose rigs read the focus's outlets (`camera:focus`, `camera:distance`)
    focus_outlets: Vec<focus::Outlet>,
    /// a phone holds this deck's camera: the picture follows it (live_cameras.rs `Follow`), the deck's own view put
    /// aside, not changed – it returns when the phone lets go
    pub held: Option<crate::live_cameras::Follow>,
    /// the live camera it follows (its id: this scene's primary camera or one of its shots); a move played over it
    held_by: Option<String>,
    held_moved: bool,
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

/// the space node's set id and quality – what the sync fetches (#371: the push's core)
pub use crate::push::set_need;

fn quality(graph: &Graph, node: usize) -> String {
    graph.nodes[node].str("quality").unwrap_or(crate::push::DEFAULT_QUALITY).to_string()
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
        let cast = Cast::of(graph, &mut portraits);
        let mut being = if cast.is_empty() { Being::new(graph, look) } else { Being::part(graph, look, None, &|_| false) };
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
        let mut space = Space {
            node,
            being,
            cast,
            picked: None,
            camera,
            set_rx,
            model: None,
            set_path: path,
            set_failed: false,
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
            fingers: Fingers::new(),
            pressed: None,
            tapped: None,
            clock: 0.0,
            notice: None,
            graph: graph.clone(),
            portraits,
            join,
            column: None,
            stream: None,
            link: None,
            note: String::new(),
            others: Vec::new(),
            routes: Vec::new(),
            route_ids: Vec::new(),
            me: None,
            cast_in: std::collections::VecDeque::new(),
            story: false,
            ring: n.f32("ring", RING),
            generation: 0,
            places: Vec::new(),
            sync: sync::Board::default(),
            backdrop: backdrop::Backdrop::default(),
            layers: layers::Layers::default(),
            dop: dop::Dop::default(),
            camera_pending: None,
            motions: motion::Motions::default(),
            row_from: None,
            focus: 0.0,
            focus_outlets: focus::outlets(graph),
            held: None,
            held_by: None,
            held_moved: false,
        };
        space.place_cast();
        space
    }

    /// the stage's background that won (the played scene's `stage.background`)
    pub fn set_background(&mut self, background: Option<crate::Background>) {
        let ground = self.backdrop.ground;
        self.backdrop = backdrop::Backdrop { ground, ..backdrop::Backdrop::new(background, &mut self.portraits) };
    }

    /// what the played scene sets beside its graph: its stage (background, layers) and its camera
    pub fn set_scene(&mut self, scene: &crate::scene::Scene) {
        self.set_stage(&scene.stage);
        self.set_camera(None, scene.camera.clone());
        self.set_motions(scene);
    }

    /// The modules the played scene attaches: the camera's and its members'. The row's time counts from here.
    pub fn set_motions(&mut self, scene: &crate::scene::Scene) {
        use motion::Target;
        let camera = scene.camera.iter().flat_map(|c| c.modules.iter()).map(|a| (Target::Camera, a.clone()));
        let members = scene.modules.iter().map(|p| (Target::Holder(p.holder.clone()), p.attach()));
        self.motions.set(camera.chain(members).collect());
        self.row_from = None;
    }

    /// an editor's attachments changed: those still attached run on as they were, the row's time too
    pub fn edit_motions(&mut self, list: Vec<(motion::Target, deck_catalog::slot::Attach)>) {
        self.motions.set(list);
    }

    /// a phone's pose for the camera it holds (relay.rs `Out::HandPose` of the camera's holder): the first one takes
    /// the camera over from the view the deck has now; false: late or malformed
    pub fn held_pose(&mut self, t: f64, p: &[f64], q: &[f64]) -> bool {
        let eye = self.camera.eye();
        let deck = (eye, crate::live_cameras::look_rotation(eye, self.camera.aim()));
        self.held.get_or_insert_with(Default::default).pose(t, p, q, deck)
    }

    /// A camera of the session's phones (relay.rs `In::Held`): one of this scene's – its primary camera, or a shot, whose
    /// first pose starts from that shot's view – is followed while a phone holds it; one at a time, the first held. Another
    /// phone taking it goes on from the view held now; free again, the deck's own camera returns.
    pub fn held(&mut self, h: crate::live_cameras::Held) {
        use crate::live_cameras::{camera_of, look_rotation, Held};
        let (Held::Holder { camera, .. } | Held::Pose { camera, .. } | Held::Recenter { camera }) = &h;
        let (scene, shot) = camera_of(camera);
        let mine = match &self.dop.scene {
            Some(place) => crate::rig::address::Target::parse(scene).is_ok_and(|t| t.picks_up(place)),
            None => crate::rig::address::Target::parse(scene).is_ok_and(|t| t.kind == "scene"),
        };
        let from_shot = shot.map(|s| self.dop.camera.shot(s).map(dop::View::of_shot));
        if !mine || matches!(from_shot, Some(None)) || self.held_by.as_ref().is_some_and(|c| c != camera) {
            return;
        }
        match h {
            Held::Holder { holder: None, .. } => {
                self.held_by = None;
                self.held_release();
            }
            Held::Holder { camera, holder: Some(_) } => match self.held.as_mut() {
                Some(f) if self.held_by.is_some() => f.handover(),
                _ => self.held_by = Some(camera),
            },
            Held::Pose { camera, pose } => {
                let anchor = match from_shot.flatten() {
                    Some(v) => (v.eye(), look_rotation(v.eye(), v.target)),
                    None => (self.camera.eye(), look_rotation(self.camera.eye(), self.camera.aim())),
                };
                self.held_by = Some(camera);
                self.held.get_or_insert_with(Default::default).pose(pose.t, &pose.p, &pose.q, anchor);
            }
            Held::Recenter { .. } => self.held_recenter(),
        }
    }

    /// the live camera this deck follows now, if a phone holds one of its scene's
    pub fn held_camera(&self) -> Option<&str> {
        self.held_by.as_deref()
    }

    /// the holder's Recenter: its next pose is the view now
    pub fn held_recenter(&mut self) {
        if let Some(f) = self.held.as_mut() {
            f.recenter();
        }
    }

    /// the phone let go (or the camera returned to its owner): the deck's own camera again
    pub fn held_release(&mut self) {
        self.held = None;
    }

    /// the held view over everything the deck's camera did this frame, with the deck's lens. A move that plays meanwhile
    /// plays over it (LIVE-CAMERAS.md §6, [delegated]); when it ends the phone goes on from where the move ended.
    fn follow_held(&mut self, dt: f32) {
        let Some(f) = self.held.as_mut() else { return };
        if self.dop.director.moving() {
            self.held_moved = true;
            f.step(dt);
            return;
        }
        if std::mem::take(&mut self.held_moved) {
            let eye = self.camera.eye();
            f.rebase((eye, crate::live_cameras::look_rotation(eye, self.camera.aim())));
        }
        f.step(dt);
        let (fov_y, ortho) = self.camera.over.map_or((self.camera.fov_y, self.camera.ortho), |o| (o.fov_y, o.ortho));
        if let Some(over) = f.over(fov_y, ortho) {
            self.camera.over = Some(over);
        }
    }

    /// The motion modules this frame, on top of what the shot, the move or the spot made: the camera's while the view is
    /// the scene camera's (never the free one), each member's on its card.
    fn move_by_modules(&mut self, clock: &dop::Clock) {
        self.camera.over = None;
        for c in &mut self.cast {
            c.moved = None;
        }
        if self.motions.is_empty() {
            return;
        }
        let (from_s, from_b) = *self.row_from.get_or_insert((clock.secs as f32, clock.beat));
        let row = (clock.secs as f32 - from_s, (clock.beat - from_b) as f32);
        if self.dop.viewing != dop::Viewing::Free && self.motions.has(&motion::Target::Camera) {
            let input = self.camera_input(clock, row);
            if let Some((out, slot)) = self.motions.step(&motion::Target::Camera, clock, &input) {
                let c = &self.camera;
                let base = motion::Lensed { eye: c.base_eye(), target: c.target, focal: crate::scene_camera::focal_of(c.fov_y), ortho: c.ortho };
                let m = motion::camera(base, &out, slot);
                self.camera.over = Some(camera::Over { eye: m.eye, target: m.target, fov_y: crate::scene_camera::fov_of(m.focal), ortho: m.ortho });
            }
        }
        for i in 0..self.cast.len() {
            let c = &self.cast[i];
            if !self.motions.has_holder(&c.holder) {
                continue;
            }
            let base = motion::Pose::at(c.feet);
            let mut input = deck_module::TransformIn { kind: deck_module::motion::KIND_ENTITY, position: base.position.to_array(), marks: 1, ..deck_module::TransformIn::ZERO };
            input.mark[0] = [c.feet.x, c.feet.y, c.feet.z, 0.0];
            (input.row_valid, input.row_secs, input.row_beats) = (1, row.0, row.1);
            if let Some((out, slot)) = self.motions.step_holder(&c.holder, clock, &input) {
                self.cast[i].moved = Some(motion::entity(base, &out, slot));
            }
        }
    }

    /// what a camera's motion module reads: the view now, the shot it went to, the move under way, its shots' eyes
    fn camera_input(&self, clock: &dop::Clock, row: (f32, f32)) -> deck_module::TransformIn {
        let c = &self.camera;
        let shots = &self.dop.camera.shots;
        let mut t = deck_module::TransformIn {
            kind: deck_module::motion::KIND_CAMERA,
            row_valid: 1,
            row_secs: row.0,
            row_beats: row.1,
            position: c.base_eye().to_array(),
            target: c.target.to_array(),
            focal: crate::scene_camera::focal_of(c.fov_y),
            size: c.ortho.unwrap_or(0.0),
            projection: u32::from(c.ortho.is_some()),
            shot: self.dop.current.as_deref().and_then(|id| shots.iter().position(|s| s.id == id)).map_or(0, |i| i as u32 + 1),
            shots: shots.len() as u32,
            ..deck_module::TransformIn::ZERO
        };
        if let Some((_, p)) = self.dop.director.playing(clock) {
            (t.progress, t.moving) = (p, 1);
        }
        for (m, s) in t.mark.iter_mut().zip(shots.iter()) {
            let e = dop::View::of_shot(s).eye();
            *m = [e.x, e.y, e.z, s.lens.focal];
        }
        t.marks = shots.len().min(deck_module::motion::MARKS) as u32;
        t
    }

    /// the stage's parallax layers (the played scene's `stage.layers`), their pictures asked for
    pub fn set_layers(&mut self, layers: &[crate::Layer]) {
        self.layers = layers::Layers::new(layers, &mut self.portraits);
    }

    /// The layers' planes in the 3D this frame: fitted to the scene's primary camera, far to near from the eye (into
    /// `out`, cleared first). `empty`: the empty places too (an editor outlines them).
    pub fn layer_planes(&self, empty: bool, out: &mut Vec<layers::Plane>) {
        let shot = self.scene_camera();
        self.layers.planes(&shot, self.view.0 / self.view.1.max(1.0), &self.portraits, self.camera.eye(), empty, out);
    }

    /// what a played scene's stage shows around the 3D: its background and its layers
    pub fn set_stage(&mut self, stage: &crate::scene::StageSettings) {
        self.set_background(stage.background.clone());
        self.set_layers(&stage.layers);
    }

    /// what shows when no background sets a colour (display values; black unless the editor's `stage` token says else)
    pub fn set_ground(&mut self, rgb: [f32; 3]) {
        self.backdrop.ground = rgb;
    }

    /// #492: the cast's beings to their holders' spots in the set (lifted like the scene's being), a zone's members by
    /// their arrangement; a spot the set lacks (or no set yet) puts them in a row across the scene's spot. #495: each
    /// member's form for this stage (cast.rs) – a change of form sizes the renderer's buffers anew.
    fn place_cast(&mut self) {
        let n = self.cast.len();
        let stage_3d = self.stage_3d();
        let mut changed = false;
        for (k, c) in self.cast.iter_mut().enumerate() {
            let spot = c.spot.as_ref().and_then(|s| self.spots.iter().find(|(name, _)| name == s)).map(|(_, p)| *p);
            let row = Vec3::X * (k as f32 - (n as f32 - 1.0) * 0.5) * CAST_SPACING;
            c.feet = spot.unwrap_or(self.being.anchor - Vec3::Y * self.lift + row) + c.at;
            c.being.anchor = c.feet + Vec3::Y * self.lift;
            let form = cast::form(&c.who, stage_3d, c.chosen, c.look);
            changed |= form != c.form;
            c.form = form;
        }
        if changed {
            self.generation += 1;
        }
    }

    /// each character member's look form from scene-content's answer (cast.rs `form`: under its holder's choice)
    pub fn set_looks(&mut self, content: &crate::push::Content) {
        for c in &mut self.cast {
            c.look = content.look_form(c.who.character());
        }
        self.place_cast();
    }

    /// #495: whether this stage is 3D – its set's 3D is shown or on its way (a set that failed to load is not)
    pub fn stage_3d(&self) -> bool {
        self.set_path.is_some() && !self.set_failed
    }

    /// #495: the cast members standing as cards this frame (into `out`, cleared first; its capacity the cast's – no
    /// allocation): feet on the spot, the height its kind's (cast.rs), the width its picture's aspect, turned to `eye`
    /// around the vertical; a character's picture in the mood its being reports (the nearest it has, else its portrait),
    /// an item's its one picture. Only its own look's inputs move it: `bounce` hops it, `pulse`, `glow` and `hue` light
    /// it – nothing by default (owner 2026-11-12 [rigs]: no default breath)
    pub fn write_cards(&self, eye: Vec3, out: &mut Vec<Card>) {
        out.clear();
        out.extend(self.cast.iter().filter(|c| c.form == Form::Card).map(|c| self.card(c, eye)));
    }

    /// #495: a member's card as `eye` sees it this frame (what `write_cards` draws and #496's pick hits)
    fn card(&self, c: &Cast, eye: Vec3) -> Card {
        let h = &c.being.head;
        let picture = self.portraits.pick_near(&c.pictures, h.near.get(h.mood), h.mood).or_else(|| self.portraits.pick_near(&h.sprites, h.near.get(h.mood), h.mood));
        let aspect = picture.map_or(CARD_ASPECT, |s| self.portraits.slots[s].aspect);
        let height = cast::card_height(&c.who);
        let at = c.moved.map_or(c.feet, |m| m.position);
        let flat = Vec3::new(eye.x - at.x, 0.0, eye.z - at.z);
        let mut right = Vec3::Y.cross(flat.try_normalize().unwrap_or(Vec3::Z));
        let mut up = Vec3::Y;
        let mut size = glam::Vec2::ONE;
        // a module's turn on a card that stands facing the eye: its roll in the card's plane, then its yaw about the vertical
        if let Some(m) = c.moved {
            let (s, co) = m.rotation.z.sin_cos();
            (right, up) = (right * co + up * s, up * co - right * s);
            let yaw = glam::Quat::from_rotation_y(m.rotation.y);
            (right, up) = (yaw * right, yaw * up);
            size = glam::Vec2::new(m.scale.x, m.scale.y);
        }
        let feet = at + Vec3::Y * c.being.bounce * CARD_HOP * height;
        Card { feet, right: right * height * aspect * 0.5 * size.x, up: up * height * size.y, picture, glow: c.being.glow, hue: c.being.hue, pulse: c.being.pulse }
    }

    /// #495: the middle of a stage whose every member is a card (half way up the cards); None while any stands in 3D
    fn cards_middle(&self) -> Option<Vec3> {
        if self.cast.is_empty() || self.cast.iter().any(|c| c.form == Form::Look) {
            return None;
        }
        let sum = self.cast.iter().fold(Vec3::ZERO, |a, c| a + c.feet + Vec3::Y * cast::card_height(&c.who) * 0.5);
        Some(sum / self.cast.len() as f32)
    }

    /// #495: how many cards `write_cards` writes at most
    pub fn cards(&self) -> usize {
        self.cast.iter().filter(|c| c.form == Form::Card).count()
    }

    /// what a deck announces of its being on the wire (`deck.graph`); None unless the space node joins as a deck
    pub fn wire_graph(&self) -> Option<protocol::DeckGraph> {
        (self.join == Join::Deck).then(|| wire::deck_graph(&self.graph, &self.being.look, &self.being.moods))
    }

    /// #442: the engine's far world nodes and event outputs to the link (nothing without one)
    pub fn wire_far(&self, table: crate::far::FarTable) {
        if let Some(l) = &self.link {
            l.far(table);
        }
    }

    /// #517: whether the deck's playing scene listens to story events (scene_play.rs `Playing::listens`): the link asks the
    /// session for them while it does (nothing without a link, or on the conductor)
    pub fn ask_story(&mut self, on: bool) {
        if let Some(l) = self.link.as_ref().filter(|_| self.join == Join::Deck && self.story != on) {
            l.story(on);
            self.story = on;
        }
    }

    /// #517: the address actions and story events that came for the deck's playing scene, oldest first – its host hands
    /// them to the core (scene_play.rs)
    pub fn take_cast(&mut self) -> Vec<In> {
        self.cast_in.drain(..).collect()
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
        #[cfg(feature = "media")]
        self.portraits.play(f64::from(time) + crate::video::shown_ahead(dt));
        self.clock = time;
        self.light = ctl.input(self.node, "light", params::default(crate::graph::Kind::Space, "light"));
        self.post_knobs = post::Knobs::read(&*ctl, self.node);
        // the focus follows the zoom: the whole being far out, its head close up (owner 2026-10-31) – or, with a knob
        // selected, that knob (#224: "zooming then brings you closer to that point"); the camera glides there – held
        // still while a hand is on a knob (a press selects it: the knob would slide away under the finger)
        if self.camera.take_touched() {
            self.dop.director.interrupt();
            self.dop.viewing = dop::Viewing::Free;
        }
        let directed = self.direct(&dop::Clock::of(time, beat));
        if !directed && !self.dop.director.active() && !self.camera.free && !matches!(self.drag, Drag::Knob { .. } | Drag::Hold) {
            self.camera.goal = self.focus_goal(self.camera.distance);
        }
        self.camera.step(dt);
        self.move_by_modules(&dop::Clock::of(time, beat));
        self.follow_held(dt);
        if !directed {
            self.focus = self.settled_focus();
        }
        let at = |m: Option<usize>| match m {
            Some(i) => self.cast.get(i).map(|c| self.member_sphere(c).0),
            None => Some(self.being.focus().0),
        };
        focus::push(&self.focus_outlets, self.camera.eye(), self.focus, &at, ctl);
        // #209: the knobs face the camera; #224: and fold into its view close up
        self.being.eye = Some(self.camera.eye());
        self.being.frame = Some(self.camera.frame(self.view.0 / self.view.1));
        self.being.update(dt, time, &Local { ctl, meter }, beat);
        self.being.push(ctl);
        // #492: each holder's being on its own look's inputs
        for c in &mut self.cast {
            c.being.eye = self.being.eye;
            c.being.frame = self.being.frame;
            c.being.update(dt, time, &Local { ctl, meter }, beat);
            c.being.push(ctl);
        }
        let local = Local { ctl, meter };
        // what the pointer is over: the hand's – the picked member's knobs (#496), else the space's own being's
        if self.drag == Drag::None {
            let reach = if self.fingers.driver().is_some() { TOUCH_REACH } else { 1.0 };
            let hover = self.pointer.and_then(|(x, y)| self.hand_hit(&self.camera.ray(x, y, self.view.0, self.view.1), reach));
            self.hand_mut().hover = hover;
        }
        let dragging = matches!(self.drag, Drag::Knob { .. } | Drag::Hold);
        self.hand_mut().dragging = dragging;
        self.session(time, &local);
        self.congregation(dt, time, beat);
    }

    /// Where the camera's focus goes at `distance` (camera::focus_target): from the whole being's centre far out to its
    /// head close up – to the selected knob instead of the head while one is (#224; where it sits unfolded, so the
    /// knob comes to the middle of the picture as the camera nears it).
    /// the whole being in the view, from where the camera looks now (an editor's `f`)
    pub fn frame_being(&mut self) {
        self.camera.leave();
        let want = self.frame_distance();
        self.camera.want = want;
        self.camera.goal = self.focus_goal(want);
        self.camera.idle = 0.0;
    }

    /// everything on the stage in view: the whole cast, else the being (`frame_being`)
    pub fn frame_all(&mut self) {
        if self.cast.is_empty() {
            return self.frame_being();
        }
        let spheres: Vec<(Vec3, f32)> = self.cast.iter().map(|c| self.member_sphere(c)).collect();
        self.frame_spheres(&spheres);
    }

    /// cast member `i` in view
    pub fn frame_member(&mut self, i: usize) {
        let Some(c) = self.cast.get(i) else { return };
        let sphere = self.member_sphere(c);
        self.frame_spheres(&[sphere]);
    }

    fn frame_spheres(&mut self, spheres: &[(Vec3, f32)]) {
        self.camera.leave();
        let (lo, hi) = spheres.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), &(c, r)| (lo.min(c - r), hi.max(c + r)));
        let target = (lo + hi) * 0.5;
        self.camera.goal = target;
        self.camera.want = self.camera.fit_distance(self.view.0 / self.view.1, target, spheres, FRAME_MARGIN);
        self.camera.idle = 0.0;
    }

    /// The scene's primary camera: the shot the scene opens with; a scene without shots: head-on (yaw 0) at the stage's
    /// centre, level at the camera's eye height (adult unless the scene says), from the space node's distance.
    pub fn scene_camera(&self) -> Shot {
        if let Some(v) = self.dop.camera.opening().and_then(|s| self.shot_view(&s.id)) {
            return shot_of(&v);
        }
        let (_, _, distance) = self.camera.home();
        let centre = self.stage_centre();
        let target = centre + Vec3::Y * self.dop.camera.height;
        let mut shot = shot_of(&dop::View { target, yaw: 0.0, pitch: 0.0, distance, lens: self.dop.camera.lens });
        if self.dop.camera == crate::scene_camera::SceneCamera::default() {
            shot.fov_y = self.camera.free_fov();
        }
        shot
    }

    /// shot `id`'s view, aimed where its `aim` stands now
    pub fn shot_view(&self, id: &str) -> Option<dop::View> {
        let v = self.dop.view(id)?;
        Some(match self.dop.aim_of(id).and_then(|a| self.aim_point(&a)) {
            Some(target) => dop::View { target, ..v },
            None => v,
        })
    }

    /// Where an aim address (rig/address.rs `Aim`) stands now: a holder's or a filler's member at its middle, a set's
    /// spot at an adult's eye above it. None: not on this stage.
    pub fn aim_point(&self, aim: &str) -> Option<Vec3> {
        use crate::rig::address::Aim;
        let member = |hit: &dyn Fn(&Cast) -> bool| self.cast.iter().find(|c| hit(c)).map(|c| self.member_sphere(c).0);
        match Aim::parse(aim).ok()? {
            Aim::Holder(h) => member(&|c| c.holder == h),
            Aim::Who(w) if self.cast.is_empty() => (self.alone_subject().as_deref() == Some(w.as_str())).then(|| self.being.focus().0),
            Aim::Who(w) => member(&|c| c.who.place() == w),
            Aim::Spot(n) => self.spots.iter().find(|(s, _)| *s == n).map(|(_, at)| *at + Vec3::Y * crate::scene_camera::DEFAULT_HEIGHT),
        }
    }

    /// a rig played alone: its character as a subject (`character:<id>`), the space's own being
    fn alone_subject(&self) -> Option<String> {
        let c = &self.being.look.character;
        (!c.is_empty()).then(|| format!("character:{c}"))
    }

    /// what a shot can aim at on this stage: who fills each holder (`character:<id>`, `item:<id>`), then the set's spots
    pub fn aim_choices(&self) -> Vec<String> {
        let who = self.cast.iter().map(|c| c.who.place()).chain(self.alone_subject().filter(|_| self.cast.is_empty()));
        let spots = self.spots.iter().map(|(n, _)| format!("{}:{n}", crate::rig::address::SPOT));
        let mut out: Vec<String> = who.chain(spots).collect();
        out.dedup();
        out
    }

    /// The scene camera's frusta for an editor's own view (a deck's output never draws them): each shot's by its id, the
    /// primary camera's (none) when there are no shots – and whether it is the one the stage looks through or moves to.
    pub fn camera_frusta(&self) -> Vec<(Option<String>, dop::Frustum, bool)> {
        let aspect = self.view.0 / self.view.1.max(1.0);
        let playing = self.dop.clock.as_ref().and_then(|c| self.dop.director.playing(c)).map(|(c, _)| c.shot.clone());
        let active = playing.or_else(|| match &self.dop.viewing {
            dop::Viewing::Shot(id) => Some(id.clone()),
            dop::Viewing::Primary => self.dop.camera.opening().map(|s| s.id.clone()).or_else(|| Some(String::new())),
            dop::Viewing::Free => None,
        });
        if self.dop.camera.shots.is_empty() {
            let s = self.scene_camera();
            let eye = s.target + s.distance * dop::back(s.yaw, s.pitch);
            return vec![(None, dop::Frustum::of(eye, s.target, s.fov_y, s.ortho, aspect), active.as_deref() == Some(""))];
        }
        self.dop.camera.shots.iter().filter_map(|s| Some((Some(s.id.clone()), self.shot_view(&s.id)?.frustum(aspect), active.as_deref() == Some(s.id.as_str())))).collect()
    }

    /// a world segment in window pixels, cut where it passes behind the eye (None: wholly behind)
    pub fn segment_to_pixels(&self, a: Vec3, b: Vec3) -> Option<((f32, f32), (f32, f32))> {
        const W: f32 = 1e-3;
        let m = self.camera.view_proj(self.view.0 / self.view.1);
        let (mut ca, mut cb) = (m * a.extend(1.0), m * b.extend(1.0));
        if ca.w < W && cb.w < W {
            return None;
        }
        if ca.w < W {
            ca = ca.lerp(cb, (W - ca.w) / (cb.w - ca.w));
        } else if cb.w < W {
            cb = cb.lerp(ca, (W - cb.w) / (ca.w - cb.w));
        }
        let px = |c: glam::Vec4| ((c.x / c.w + 1.0) * 0.5 * self.view.0, (1.0 - c.y / c.w) * 0.5 * self.view.1);
        Some((px(ca), px(cb)))
    }

    /// The view's size in pixels and the free view's lens for it (camera.rs `screen_fov`: upright, wider). A shot's or a
    /// scene camera's own lens is not widened: it wins on every screen.
    pub fn fit_screen(&mut self, w: f32, h: f32) {
        self.set_view(w, h);
        let own = match &self.dop.viewing {
            dop::Viewing::Free => false,
            dop::Viewing::Primary => self.dop.camera != crate::scene_camera::SceneCamera::default(),
            dop::Viewing::Shot(_) => true,
        };
        self.camera.set_free_fov(camera::screen_fov(w / h.max(1.0)), !own);
    }

    /// the stage's centre on its floor: the middle of the cast's feet across, else under the scene's being
    pub fn stage_centre(&self) -> Vec3 {
        let floor = self.floor_point();
        if self.cast.is_empty() {
            return Vec3::new(self.being.anchor.x, floor.y, self.being.anchor.z);
        }
        let (lo, hi) = self.cast.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), c| (lo.min(c.feet), hi.max(c.feet)));
        Vec3::new((lo.x + hi.x) * 0.5, floor.y, (lo.z + hi.z) * 0.5)
    }

    /// the view snaps to the scene's camera and holds it until a hand moves it
    pub fn look_through_scene(&mut self) {
        self.dop.director.interrupt();
        let shot = self.scene_camera();
        self.camera.look_through(shot);
        self.dop.viewing = dop::Viewing::Primary;
        self.dop.focus_shot = None;
    }

    /// an eased glide back to the free view the camera left; a move playing stops where it is
    pub fn look_free(&mut self) {
        self.dop.director.interrupt();
        self.camera.go_free();
        self.dop.viewing = dop::Viewing::Free;
        self.dop.focus_shot = None;
    }

    /// what the stage looks through now
    pub fn viewing(&self) -> &dop::Viewing {
        &self.dop.viewing
    }

    /// An editor opens a scene on its camera: the view is there at once and holds it until a hand moves it – the same
    /// view however the stage was looked at before.
    pub fn open_on_scene_camera(&mut self) {
        let home = self.camera.home();
        let (free, auto, fov) = (self.camera.free, self.camera.auto, self.camera.free_fov());
        self.camera = Orbit::new(self.camera.target, home.0, home.1, home.2);
        (self.camera.free, self.camera.auto) = (free, auto);
        self.camera.set_free_fov(fov, true);
        let shot = self.scene_camera();
        let c = &mut self.camera;
        c.max_distance = c.max_distance.max(shot.distance);
        (c.target, c.goal, c.yaw, c.pitch, c.distance, c.want, c.fov_y, c.ortho) = (shot.target, shot.target, shot.yaw, shot.pitch, shot.distance, shot.distance, shot.fov_y, shot.ortho);
        c.look_through(shot);
        self.dop.viewing = dop::Viewing::Primary;
    }

    /// The scene's camera (`scene`: its place, `scene:<id>`), as the played scene carries it. A deck (not an editor's free
    /// view) opens on its first shot.
    pub fn set_camera(&mut self, scene: Option<&str>, camera: Option<crate::scene_camera::SceneCamera>) {
        self.dop = dop::Dop::new(scene.map(String::from), camera.unwrap_or_default());
        if !self.camera.free {
            self.open_on_scene_camera();
        }
        // a camera word that came before its scene (a show row's cell picks the shot of a scene that starts with it)
        if let Some((address, action, beat)) = self.camera_pending.take() {
            if let Some(Ok(said)) = self.dop.word_on(&address, &action, beat) {
                tracing::info!("{address} > {said} (as the scene arrived)");
            }
        }
    }

    /// the view glides to the shot `id` and looks through it until a hand moves it
    pub fn look_through_shot(&mut self, id: &str) {
        let Some(v) = self.shot_view(id) else { return };
        self.dop.director.interrupt();
        self.dop.current = Some(id.to_string());
        self.camera.look_through(shot_of(&v));
        self.dop.viewing = dop::Viewing::Shot(id.to_string());
        self.dop.focus_shot = None;
    }

    /// where the ray through view pixel (`x`, `y`) meets the level `height` in front of the eye (a track point dragged)
    pub fn level_point(&self, x: f32, y: f32, height: f32) -> Option<Vec3> {
        let ray = self.camera.ray(x, y, self.view.0, self.view.1);
        let t = (height - ray.origin.y) / ray.dir.y;
        (ray.dir.y.abs() > 1e-4 && t > 0.0).then(|| ray.origin + ray.dir * t)
    }

    /// how far along the line from `eye` toward `dir` the ray through view pixel (`x`, `y`) passes closest (m; a focus
    /// marker dragged)
    pub fn along_sight(&self, x: f32, y: f32, eye: Vec3, dir: Vec3) -> Option<f32> {
        let ray = self.camera.ray(x, y, self.view.0, self.view.1);
        let d = dir.try_normalize()?;
        let w = eye - ray.origin;
        let (b, e) = (d.dot(ray.dir), ray.dir.dot(w));
        let denom = ray.dir.length_squared() - b * b;
        let s = (b * e - ray.dir.length_squared() * d.dot(w)) / denom;
        (denom.abs() > 1e-6 && s > 0.0).then_some(s)
    }

    /// the subject (`character:<id>`, `item:<id>`) whose middle is nearest view pixel (`x`, `y`), within `reach` pixels
    pub fn subject_at(&self, x: f32, y: f32, reach: f32) -> Option<String> {
        let alone = self.alone_subject().filter(|_| self.cast.is_empty()).map(|w| (self.being.focus().0, w));
        self.cast
            .iter()
            .map(|c| (self.member_sphere(c).0, c.who.place()))
            .chain(alone)
            .filter_map(|(at, who)| {
                let (px, py) = self.to_pixel(at)?;
                let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
                (d <= reach).then_some((d, who))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, w)| w)
    }

    /// An address action for the scene's camera (`scene:nave > shot:wide`, `> move:intro`): whether it was one.
    pub fn camera_word(&mut self, address: &str, action: &str) -> bool {
        self.camera_word_on(address, action, None)
    }

    /// `camera_word`, played from `beat` of the show clock (a show cell's cut on its row's bar or phrase; none: now)
    pub fn camera_word_on(&mut self, address: &str, action: &str, beat: Option<f64>) -> bool {
        match self.dop.word_on(address, action, beat) {
            None if crate::rig::address::CameraWord::parse(action).is_some() && address.starts_with("scene:") => {
                self.camera_pending = Some((address.to_string(), action.to_string(), beat));
                true
            }
            None => false,
            Some(Ok(said)) => {
                tracing::info!("{address} > {said}");
                true
            }
            Some(Err(e)) => {
                tracing::warn!("{address} > {action}: {e} – kept for the next scene");
                self.camera_pending = Some((address.to_string(), action.to_string(), beat));
                true
            }
        }
    }

    /// the moves' view this frame onto the camera; whether one played
    fn direct(&mut self, clock: &dop::Clock) -> bool {
        if !self.dop.director.active() {
            return false;
        }
        let mut now = dop::View::of(&self.camera);
        // a phone holds the camera: a move starts from the phone's view
        if let Some((eye, rot)) = self.held.as_ref().and_then(|f| f.view()).filter(|_| !self.held_moved) {
            now = dop::View::from_eye(eye, eye + rot * Vec3::NEG_Z, now.lens);
        }
        now.lens.focus = Some(self.focus).filter(|f| *f > 0.0);
        let c = &self.dop.camera;
        let named = c.shots.iter().flat_map(|s| [s.aim.clone(), s.focus_on.clone()]).flatten().chain(c.moves.iter().filter_map(|m| m.spec.follow.clone()));
        let at: Vec<(String, Vec3)> = named.filter_map(|a| self.aim_point(&a).map(|p| (a, p))).collect();
        let subject = |aim: &str| at.iter().find(|(a, _)| a == aim).map(|(_, p)| *p);
        match self.dop.step(clock, now, &subject) {
            Some(v) => {
                self.camera.far_back = self.dop.director.blending(clock);
                v.apply(&mut self.camera);
                self.focus = v.focus();
                true
            }
            None => false,
        }
    }

    /// Where the focus is while no move plays (m from the eye): the shot that holds it (dop `focus_holder`) – its subject's
    /// distance from the eye now (a follow focus), else its lens's, else its aim's; free, at the aim.
    fn settled_focus(&self) -> f32 {
        let eye = self.camera.eye();
        let Some(s) = self.dop.focus_holder() else {
            let primary = self.dop.viewing == dop::Viewing::Primary;
            return primary.then_some(self.dop.camera.lens.focus).flatten().unwrap_or(self.camera.distance);
        };
        let on = s.focus_on.as_deref().and_then(|a| self.aim_point(a)).map(|p| eye.distance(p));
        on.or(s.lens.focus).unwrap_or_else(|| self.shot_view(&s.id).map_or(self.camera.distance, |v| eye.distance(v.target)))
    }

    /// Where shot `id` is focused: on its subject where it stands, else its focus distance along its line of sight, else
    /// its aim (the scene view's focus marker).
    pub fn shot_focus_point(&self, id: &str) -> Option<Vec3> {
        let v = self.shot_view(id)?;
        Some(self.dop.focus_on(id).and_then(|a| self.aim_point(&a)).unwrap_or_else(|| v.focus_point()))
    }

    /// the focus this frame as a point: the focus distance along the camera's line of sight
    pub fn focus_point(&self) -> Vec3 {
        let eye = self.camera.eye();
        eye + (self.camera.target - eye).try_normalize().unwrap_or(Vec3::NEG_Z) * self.focus
    }

    /// every subject's outlets now – its address, how sharp it is, its distance from the eye (m): the cast, else the
    /// space's own being (`scene`)
    pub fn focus_values(&self) -> Vec<(String, f32, f32)> {
        let eye = self.camera.eye();
        if self.cast.is_empty() {
            let (s, d) = focus::values(eye, self.focus, self.being.focus().0);
            return vec![(self.alone_subject().unwrap_or_else(|| "scene".into()), s, d)];
        }
        self.cast
            .iter()
            .map(|c| {
                let (s, d) = focus::values(eye, self.focus, self.member_sphere(c).0);
                (c.who.place(), s, d)
            })
            .collect()
    }

    fn focus_goal(&self, distance: f32) -> Vec3 {
        // #495: a stage of cards only – the space's own being has no body to aim by: the cards' middle, so they stand whole
        // in the picture
        if let Some(middle) = self.cards_middle() {
            return middle;
        }
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
            let points = b.points.iter().map(|p| (p.pos, p.extent()));
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
                if !self.camera.looking() && !self.dop.director.active() {
                    self.camera.look_at(aim(&self.being));
                }
                self.set_status = format!("SET {}K TRIS", model.triangles() / 1000);
                self.spots = model.spots.clone();
                self.model = Some(model);
                self.set_failed = false;
                self.place_cast();
                if self.camera.looking() && self.dop.viewing == dop::Viewing::Primary {
                    self.look_through_scene();
                }
            }
            Err(e) => {
                tracing::error!("set: {e:#}");
                self.set_status = format!("SET ERROR {e}");
                // #495: no 3D after all – the cast stands as cards
                self.set_failed = true;
                self.place_cast();
            }
        }
    }

    /// Another being for this deck (the browser's character switcher, #198): its graph and look, floating where the old
    /// one did; the set and the camera stay, the renderer sizes its buffers anew.
    pub fn replace_being(&mut self, graph: &Graph, look: &Look) {
        self.cast = Cast::of(graph, &mut self.portraits);
        self.focus_outlets = focus::outlets(graph);
        self.picked = None;
        let mut being = if self.cast.is_empty() { Being::new(graph, look) } else { Being::part(graph, look, None, &|_| false) };
        being.head.sprites = self.portraits.request(&look.character, &being.moods);
        being.anchor = self.being.anchor;
        self.being = being;
        self.place_cast();
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
                // a story this deck did not ask for (the conductor hears every reader's) is not its scene's
                In::Story(_) if !self.story => {}
                In::Held(h) => self.held(h),
                In::Address { address, action, beat } if self.camera_word_on(&address, &action, beat) => {}
                cast @ (In::Address { .. } | In::Story(_)) => {
                    if self.cast_in.len() == CAST_CAP {
                        self.cast_in.pop_front();
                    }
                    self.cast_in.push_back(cast);
                }
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
        protocol::SceneSync { scene: scene.into(), sets: Some(sets), characters: Some(characters), scenes: None, beings: None, decks: None }
    }

    /// #195, #371: the conductor's push – the one written out (`--sync`), else this scene's own; its `scene` = the scene
    /// file's name (`conductor`, `cathedral` …), else the stage's `name` as a slug (`push::scene_id`)
    pub fn push_for(&self, file: Option<&Path>, name: &str, written: Option<&str>) -> Result<protocol::SceneSync, String> {
        let scene = crate::push::scene_id(file, name);
        written.map_or_else(|| Ok(self.scene_push(&scene)), |spec| crate::push::parse_push(&scene, spec))
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

    /// every being drawn in 3D: this deck's own first (on a stage of several holders, the cast's instead – #492; only the
    /// members standing as their look, #495: a card is no body), then the decks'
    pub fn beings(&self) -> impl Iterator<Item = &Being> {
        let own = self.cast.is_empty().then_some(&self.being);
        let looks = self.cast.iter().filter(|c| c.form == Form::Look).map(|c| &c.being);
        own.into_iter().chain(looks).chain(self.others.iter().map(|r| &r.being))
    }

    /// This frame's labels (label.rs): the conductor's word over each deck's being (the buttons carry no numbers – owner
    /// 2026-10-31) – at most label::LABELS (`out` cleared first; its capacity from the renderer – no allocation).
    pub fn write_labels(&self, out: &mut Vec<Label>) {
        out.clear();
        // #424 (owner 2026-11-08: "whenever i hover knobs, please show their name" – for now, the one exception to the
        // no-labels rule of 2026-10-31): the hovered knob's name beside it – its parameter, never a number
        let hand = self.hand();
        if let Some(i) = hand.named() {
            let b = &hand.buttons[i];
            let mut text = label::Text::default();
            let _ = std::fmt::Write::write_str(&mut text, hand.name(i));
            let c = being::hsv(being::colour_of(b), 0.35, 1.0);
            out.push(Label { pos: b.pos.into(), offset: b.radius * 2.0, text, color: [c[0], c[1], c[2], 0.95], scale: 1.0, line: 0.0, top: true });
        }
        // #195: the conductor's word over each deck's being – its sync for the push (first: the decks' state matters), then
        // its render quality (#220) and its link (#223), a line each
        for r in &self.others {
            let head = &r.being.head;
            let mut line = 0.0;
            let mut put = |out: &mut Vec<Label>, text: label::Text, hue: f32| {
                if out.len() < label::LABELS {
                    let c = being::hsv(hue, 0.55, 1.0);
                    out.push(Label { pos: (head.pos + Vec3::Y * head.radius * 1.4).into(), offset: head.radius, text, color: [c[0], c[1], c[2], 0.9], scale: 0.85, line, top: false });
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
        match (self.drag, self.hand().hover) {
            (Drag::Knob { .. }, _) => Cursor::Grabbing,
            (Drag::Hold, _) => Cursor::Locked,
            (Drag::None, Some(Hit::Button(i))) if !self.hand().buttons[i].turnable => Cursor::Locked,
            (Drag::None, Some(Hit::Button(_))) => Cursor::Grab,
            _ => Cursor::Default,
        }
    }

    /// a world point in window pixels (None behind the eye)
    pub fn to_pixel(&self, p: Vec3) -> Option<(f32, f32)> {
        let clip = self.camera.view_proj(self.view.0 / self.view.1) * p.extend(1.0);
        (clip.w > 1e-4).then(|| ((clip.x / clip.w + 1.0) * 0.5 * self.view.0, (1.0 - clip.y / clip.w) * 0.5 * self.view.1))
    }

    /// the hand's button `node.port` (or `node.shape`), by name – a check run's `--pointer` / `--drag`; #496: the picked
    /// member's knobs only (another member's knob is no button of the hand: None)
    pub fn button(&self, graph: &Graph, name: &str) -> Option<usize> {
        let (id, port) = name.split_once('.')?;
        self.hand().buttons.iter().position(|b| graph.nodes[b.node].id == id && b.name == port)
    }

    /// button `i`'s tether on the screen: where its 0 sits and the way to its 1 (pixels); a tether that points at the
    /// eye (shorter than a tenth of the window) turns into a plain upward drag over 40 % of the window's height
    pub fn drag_axis(&self, i: usize) -> ((f32, f32), (f32, f32)) {
        let (lo, hi) = self.hand().span(i);
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
        // #496: the pick's mark, and a picked card's knobs (the most any card has)
        let picked = self.cast.iter().filter(|c| c.form == Form::Card).map(|c| pick::knob_spheres(&c.being)).max().unwrap_or(0);
        let mark = if self.cast.is_empty() { 0 } else { pick::MARK_BEADS };
        (s + picked + mark + self.routes.len() * being::BUBBLES, v + self.routes.len() * (being::SEGMENTS + 1) * being::SIDES)
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
        // #496: the picked member's mark at its feet, a picked card's knobs (a look's are its body's)
        let card = self.picked.map(|i| &self.cast[i]).filter(|c| c.form == Form::Card);
        if let Some(c) = card {
            c.being.write_knobs(spheres);
        }
        self.write_mark(spheres);
        let solid = spheres.len();
        let r = Route::radius(self.being.look.size);
        for b in self.beings() {
            b.write_ghosts(spheres);
            b.write_tubes(tubes);
        }
        if let Some(c) = card {
            c.being.write_knob_rings(spheres);
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
                self.set_failed = false;
                self.place_cast();
            }
            _ => {
                if let Some(pos) = self.spots.iter().find(|(n, _)| *n == self.spot).map(|(_, p)| *p) {
                    self.being.anchor = pos + Vec3::Y * self.lift;
                    self.camera.look_at(aim(&self.being));
                    self.place_cast();
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
                let b = &self.hand().buttons[button];
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
        self.hand_mut().hover = None;
    }

    /// the primary button down: over an unlinked button = pull it (twice quickly: back to the scene's value), over a
    /// linked one = it shakes and the HUD says what drives it, else turn the camera
    pub fn press(&mut self) {
        // a touch screen's own mouse emulation: the finger already pressed
        if !self.fingers.touching() {
            self.press_reach(1.0);
        }
    }

    fn press_reach(&mut self, reach: f32) {
        let Some((x, y)) = self.pointer else { return };
        let ray = self.camera.ray(x, y, self.view.0, self.view.1);
        // #496: the hand's – the picked member's knobs, else the space's own being's
        let hit = self.hand_hit(&ray, reach);
        self.hand_mut().hover = hit;
        // #209: what a tap here selects – a button, the being (a point, the head), or nothing (the master section); a
        // hand on a button selects it at once (the last touched knob owns the sticks)
        let head = self.hand_head(&ray);
        let chosen = hit.or(head);
        self.pressed = Some(((x, y), chosen));
        if let Some(Hit::Button(i)) = chosen {
            self.select(Some(Hit::Button(i)));
        }
        self.drag = match hit {
            Some(Hit::Button(i)) if !self.hand().buttons[i].turnable => {
                self.hand_mut().shake(i);
                let b = &self.hand().buttons[i];
                let id = &self.graph_id(b.node);
                self.notice = Some((format!("{id}.{} IS LINKED < {}", b.name, b.source), self.clock + 2.5));
                Drag::Hold
            }
            Some(Hit::Button(i)) => {
                let double = self.last_press.is_some_and(|(j, t)| j == i && self.clock - t < DOUBLE_CLICK);
                if double {
                    self.last_press = None;
                    self.hand_mut().reset(i);
                    Drag::Hold
                } else {
                    self.last_press = Some((i, self.clock));
                    self.hand_mut().reset = None;
                    // the hand starts where the value sits on the tether (not where it hit the sphere)
                    let (lo, axis) = self.drag_axis(i);
                    let n = self.hand().buttons[i].norm;
                    let at = (x - (lo.0 + axis.0 * n) + lo.0, y - (lo.1 + axis.1 * n) + lo.1);
                    Drag::Knob { button: i, norm: 0.0, at, axis }
                }
            }
            _ => Drag::Orbit(x, y),
        };
    }

    pub fn release(&mut self) {
        if !self.fingers.touching() {
            self.let_go();
        }
    }

    /// the hand lifts: a press that barely moved is a tap (#209) – it selects what it hit (nothing: deselect); #496: on a
    /// stage with a cast, a tap on none of the hand's parts picks the member under it, or drops the pick (pick.rs)
    fn let_go(&mut self) {
        if let (Some(((x0, y0), chosen)), Some((x, y))) = (self.pressed.take(), self.pointer) {
            if (x - x0).hypot(y - y0) < TAP * self.view.1 {
                match chosen {
                    None if !self.cast.is_empty() => self.tap_stage(x, y),
                    _ => self.select(chosen),
                }
                self.tapped = Some(self.hand().selected);
            }
        }
        self.drag = Drag::None;
    }

    /// #209: select a button, the being (`Hit::Point` – any), or nothing (None: the master section) – the renderer rings
    /// it; the page asks for taps with `take_tap`
    /// #224: a knob let go (the being or nothing selected now) frames the whole being – the camera glides out (or in)
    /// to `frame_distance`, the focus back to the being
    /// #496: the hand's selection – the picked member's (its knobs, or the whole member), the camera stays on the stage
    pub fn select(&mut self, what: Option<Hit>) {
        let hand = self.hand_mut();
        let was_knob = matches!(hand.selected, Some(Hit::Button(_)));
        hand.selected = match what {
            Some(Hit::Point(_)) => Some(Hit::Point(hand.core_point.unwrap_or(0))),
            Some(Hit::Button(i)) if i < hand.buttons.len() => Some(Hit::Button(i)),
            _ => None,
        };
        let knob = matches!(hand.selected, Some(Hit::Button(_)));
        if was_knob && !knob && self.picked.is_none() {
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
        match self.hand().selected {
            Some(Hit::Point(_)) => "being".into(),
            Some(Hit::Button(i)) => {
                let b = &self.hand().buttons[i];
                format!("knob:{}.{}", self.graph_id(b.node), b.name)
            }
            None => String::new(),
        }
    }

    /// The fingers on a touch screen (winit's `Touch` on the deck, the page's pointer events in the browser; `id` tells
    /// them apart): the first one drives the pointer – down = a press with a bigger reach, up = a release; a second one
    /// lets the first go and the two pinch – their distance zooms the camera by its own step (gesture.rs). The screen's
    /// mouse emulation is ignored meanwhile.
    pub fn pointers(&mut self, id: u64, phase: Touch, x: f32, y: f32, ctl: &mut dyn Knobs) {
        match self.fingers.event(id, phase, x, y) {
            Some(Act::Press(x, y)) => {
                self.pointer_moved(x, y, ctl);
                self.press_reach(TOUCH_REACH);
            }
            Some(Act::Drag(x, y)) => self.pointer_moved(x, y, ctl),
            Some(Act::Lift(x, y)) => {
                self.pointer_moved(x, y, ctl);
                self.let_go();
                self.pointer_left();
            }
            Some(Act::Pinch(steps)) => self.camera.zoom(steps),
            None => {}
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
        let hand = self.hand();
        let line = match hand.hover {
            Some(Hit::Button(i)) => {
                let b = &hand.buttons[i];
                let id = &graph.nodes[b.node].id;
                if b.shape {
                    let names = crate::params::LFO_SHAPES;
                    return format!("{id}.shape {} [{}]", names[(b.value as usize).min(names.len() - 1)], names.join(" ")).to_uppercase().chars().take(40).collect();
                }
                match graph.link_into(b.node, b.port) {
                    Some(l) => format!("{id}.{} {} < {}{}", b.name, num(b.value), graph.nodes[l.from.0].id, if hand.dragging { " *" } else { "" }),
                    None => format!("{id}.{} {}{}", b.name, num(b.value), if hand.dragging { " *" } else { "" }),
                }
            }
            Some(Hit::Point(i)) => {
                let n = &graph.nodes[hand.points[i].node];
                format!("NODE {} ({:?})", n.id, n.kind)
            }
            // #496: the picked member, else the set's state
            None => self.pick_line().unwrap_or_else(|| self.set_status.clone()),
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
                let Some(values::Path::Input(node, port)) = values::resolve_loose(&g, &n.input) else { panic!("{}", n.input) };
                let kind = g.nodes[node].kind;
                if kind.domain() != crate::graph::Domain::Audio {
                    continue;
                }
                let name = kind.inputs()[port].name;
                assert!(n.attack >= 0.15, "{}: {} {} attacks in {} s", scene.stage.name, n.mood, n.input, n.attack);
                if name.starts_with("gain") || name == "volume" {
                    let rest = g.nodes[node].f32(name, params::default(kind, name));
                    let to = n.to.unwrap_or(rest + n.by.unwrap_or(0.0));
                    assert!(to >= rest * 0.5 - 1e-6, "{}: {} ducks {} from {rest} to {to}", scene.stage.name, n.mood, n.input);
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
        space.pointers(7, Touch::Started, x, y, &mut ctl);
        assert_eq!(space.being.hover, Some(Hit::Button(i)), "the finger got it");
        assert_eq!(space.cursor(), Cursor::Grabbing);
        space.press(); // the emulated mouse press: ignored
        let (node, before) = (space.being.buttons[i].node, ctl.input(space.being.buttons[i].node, "index", -1.0));
        let (_, axis) = space.drag_axis(i);
        space.pointers(8, Touch::Moved, x + axis.0, y + axis.1, &mut ctl); // a finger that is not down: ignored
        assert_eq!(ctl.input(node, "index", -1.0), before);
        space.pointers(7, Touch::Moved, x + axis.0 * 0.5, y + axis.1 * 0.5, &mut ctl);
        assert!(ctl.input(node, "index", -1.0) > before);
        space.pointers(7, Touch::Ended, x, y, &mut ctl);
        assert_eq!((space.cursor(), space.being.hover), (Cursor::Default, None));
    }

    /// #376: a recorded two-finger sequence – the page's pointer events and winit's touches deliver the same – lets the
    /// first finger go (nothing is pulled or turned) and zooms the camera by the fingers' spread, `ZOOM_STEP` per step;
    /// the finger left over does nothing, and the mouse emulation stays out of it
    #[test]
    fn two_fingers_pinch_the_camera() {
        use crate::gesture::ZOOM_STEP;
        let (_, mut space, mut ctl) = cathedral();
        let (want, distance) = (space.camera.want, space.camera.distance);
        space.pointers(1, Touch::Started, 600.0, 360.0, &mut ctl);
        space.pointers(2, Touch::Started, 700.0, 360.0, &mut ctl); // the first lets go
        assert_eq!(space.drag, Drag::None);
        assert_eq!(space.cursor(), Cursor::Default);
        // the spread grows 100 → 200 in three moves: one doubling = ln 2 / -ln 0.88 steps in
        for (a, b) in [(575.0, 725.0), (550.0, 750.0), (500.0, 700.0)] {
            space.pointers(1, Touch::Moved, a, 360.0, &mut ctl);
            space.pointers(2, Touch::Moved, b, 360.0, &mut ctl);
        }
        let steps = 2.0f32.ln() / -ZOOM_STEP.ln();
        assert!((space.camera.want - want * ZOOM_STEP.powf(steps)).abs() < want * 1e-3, "{} from {want}", space.camera.want);
        assert_eq!(space.camera.distance, distance, "the camera glides there in step()");
        space.press(); // a touch screen's mouse emulation: ignored
        assert_eq!(space.drag, Drag::None);
        space.pointers(2, Touch::Ended, 700.0, 360.0, &mut ctl);
        space.pointers(1, Touch::Moved, 100.0, 360.0, &mut ctl); // alone after a pinch: nothing
        assert_eq!(space.drag, Drag::None);
        let after = space.camera.want;
        space.pointers(1, Touch::Ended, 100.0, 360.0, &mut ctl);
        assert_eq!(space.camera.want, after);
        assert!(space.fingers.driver().is_none() && !space.fingers.touching());
        // a lone finger on empty space still turns the camera afterwards
        space.pointers(3, Touch::Started, 500.0, 360.0, &mut ctl);
        assert!(matches!(space.drag, Drag::Orbit(..)), "{:?}", space.drag);
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

    /// a stage of three holders (#492, #495): Cat Pope lit by her own LFO, a listener whose rig maps nothing, an incense
    /// burner (an item) whose own LFO kicks its pulse; with a 3D set (`set`, its spots handed in as if it had loaded) or
    /// none
    fn stage(set: bool) -> (Graph, Space, Table) {
        stage_chosen(set, ["", ""])
    }

    /// the stage with the left and right holders' chosen forms (a `form = "card"` line on each look, or none)
    fn stage_chosen(set: bool, forms: [&str; 2]) -> (Graph, Space, Table) {
        let text = format!(
            r#"
[[node]]
id = "space"
kind = "space"
{}
[[node]]
id = "left_lfo"
kind = "lfo"
rate = 2.0
depth = 2.0
[[node]]
id = "left_look"
kind = "look"
holder = "left"
who = "character:catpope"
{}
spot = "spot_left"
at = [0.0, 0.0, 0.0]
being = {{ character = "catpope", expose = ["left_lfo.rate"] }}
[[node]]
id = "right_look"
kind = "look"
holder = "right"
who = "character:mute"
{}
spot = "spot_right"
being = {{ character = "mute" }}
[[node]]
id = "burner_lfo"
kind = "lfo"
rate = 3.0
depth = 1.0
[[node]]
id = "burner_look"
kind = "look"
holder = "burner"
who = "item:incense_burner"
[[link]]
from = "left_lfo"
to = "left_look.glow"
[[link]]
from = "burner_lfo"
to = "burner_look.pulse"
"#,
            if set { "set = \"stage.glb\"" } else { "" },
            forms[0],
            forms[1]
        );
        let scene = Scene::parse(&text).unwrap();
        let g = Graph::build(&scene).unwrap();
        let node = g.first(crate::graph::Kind::Space).unwrap();
        let mut space = Space::new(&g, node, &scene.being, Path::new("/nowhere"), Portraits::manual());
        if set {
            // the set "loaded": its spots, no file read (the loader thread's answer is dropped)
            space.set_rx = None;
            let spots = vec![("spot_left".to_string(), Vec3::new(-2.0, 0.0, 0.0)), ("spot_right".to_string(), Vec3::new(2.0, 0.0, 0.0))];
            space.set_arrived(Ok(glb::Model { vertices: Vec::new(), indices: Vec::new(), draws: Vec::new(), images: Vec::new(), spots, min: Vec3::ZERO, max: Vec3::ONE, anatomy: Default::default() }));
        }
        let ctl = Table::new(&g);
        (g, space, ctl)
    }

    /// a character's rig with no track yet stands on its stage as its 2D picture: one card, no body
    #[test]
    fn a_rig_with_no_track_yet_stands_as_its_card() {
        let played = crate::rig::Bundle::empty("character.yellowmouse").play(None, None).unwrap();
        let scene = Scene::parse(&played.scene).unwrap();
        let g = Graph::build(&scene).unwrap();
        let node = g.first(crate::graph::Kind::Space).unwrap();
        let mut space = Space::new(&g, node, &scene.being, Path::new("/nowhere"), Portraits::manual());
        assert_eq!((space.cast.len(), space.cards(), space.beings().count()), (1, 1, 0));
        assert_eq!(space.cast[0].who, Who::Character("yellowmouse".into()));
        let mut ctl = Table::new(&g);
        run(&mut space, &mut ctl, 3, &mut |_| {});
    }

    fn run(space: &mut Space, ctl: &mut Table, frames: usize, each: &mut dyn FnMut(&Space)) {
        for k in 0..frames {
            ctl.step(1.0 / 30.0);
            space.update(1.0 / 30.0, k as f32 / 30.0, ctl, &tempo::Now::default(), &|_, _| 0.0);
            each(space);
        }
    }

    fn spread(v: &[f32]) -> f32 {
        v.iter().cloned().fold(f32::MIN, f32::max) - v.iter().cloned().fold(f32::MAX, f32::min)
    }

    /// #492: a stage of several holders in a 3D set – a being per character's `look` node, its body its own rig's nodes,
    /// at its own spot; only its own look's links move it (the left one's LFO lights the left one, the right one has none
    /// and stays as it is). #495: the item has no 3D – a card even here
    #[test]
    fn a_stage_of_several_draws_a_being_per_look_each_on_its_own_inputs() {
        let (g, mut space, mut ctl) = stage(true);
        let ids = |b: &Being| b.points.iter().map(|p| g.nodes[p.node].id.as_str()).collect::<Vec<_>>();
        assert!(space.stage_3d());
        assert_eq!(space.cast.len(), 3);
        assert_eq!(ids(&space.cast[0].being), ["left_lfo", "left_look"]);
        assert_eq!(ids(&space.cast[1].being), ["right_look"]);
        assert_eq!((space.cast[0].being.look.character.as_str(), space.cast[1].being.look.character.as_str()), ("catpope", "mute"));
        assert_eq!(space.cast.iter().map(|c| c.form).collect::<Vec<_>>(), [Form::Look, Form::Look, Form::Card]);
        assert_eq!((space.beings().count(), space.cards()), (2, 1), "the characters' beings, not the space's own; the item a card");
        assert!(space.being.points.is_empty() && space.being.buttons.is_empty(), "the space's own being has no body to draw or turn");
        assert_eq!(space.cast[0].feet, Vec3::new(-2.0, 0.0, 0.0), "on its spot in the set");
        assert_eq!(space.cast[0].being.anchor, Vec3::new(-2.0, space.lift, 0.0), "the being floats over it");
        let mut glows = (Vec::new(), Vec::new());
        run(&mut space, &mut ctl, 30, &mut |s| {
            glows.0.push(s.cast[0].being.glow);
            glows.1.push(s.cast[1].being.glow);
        });
        assert!(spread(&glows.0) > 0.5, "the left one glows with its own LFO: {:?}", glows.0);
        assert_eq!(spread(&glows.1), 0.0, "the right one has no mapping: still");
        assert_eq!(glows.1[0], params::default(crate::graph::Kind::Look, "glow"));
        let (s, t) = space.capacity();
        let (mut spheres, mut tubes) = (Vec::with_capacity(s), Vec::with_capacity(t));
        space.write(&mut spheres, &mut tubes);
        assert!(!spheres.is_empty() && spheres.len() <= s);
        // the set gone after all: no 3D, every member a card
        let generation = space.generation;
        space.set_arrived(Err(anyhow::anyhow!("gone")));
        assert!(!space.stage_3d() && space.generation > generation, "the renderer sizes its buffers anew");
        assert_eq!((space.beings().count(), space.cards()), (0, 3));
    }

    /// a holder's chosen form wins over the stage: a card on a 3D stage, a look on a stage without a set
    #[test]
    fn a_holders_chosen_form_wins_over_the_stage() {
        let (_g, space, _ctl) = stage_chosen(true, ["form = \"card\"", ""]);
        assert_eq!(space.cast.iter().map(|c| c.form).collect::<Vec<_>>(), [Form::Card, Form::Look, Form::Card]);
        let (_g, space, _ctl) = stage_chosen(false, ["", "form = \"look\""]);
        assert_eq!(space.cast.iter().map(|c| c.form).collect::<Vec<_>>(), [Form::Card, Form::Look, Form::Card]);
        assert_eq!((space.beings().count(), space.cards()), (1, 2));
    }

    /// a character look's own form stands where its holder chose none; the holder's choice still wins
    #[test]
    fn a_looks_own_form_stands_under_the_holders() {
        let content = crate::push::Content::parse(r#"{"characters":{"catpope":{"form":"card"},"mute":{"form":"look"}}}"#).unwrap();
        let (_g, mut space, _ctl) = stage_chosen(true, ["", "form = \"card\""]);
        space.set_looks(&content);
        assert_eq!(space.cast.iter().map(|c| c.form).collect::<Vec<_>>(), [Form::Card, Form::Card, Form::Card], "her look's card on a 3D stage; the holder's card over the look's 3D");
        let (_g, mut space, _ctl) = stage_chosen(false, ["", ""]);
        space.set_looks(&content);
        assert_eq!(space.cast.iter().map(|c| c.form).collect::<Vec<_>>(), [Form::Card, Form::Look, Form::Card], "the look's 3D on a stage without a set");
    }

    /// #495 (owner 2026-11-12: "We fall back to 2d in all cases"): a stage with no set – every member a card, its feet on
    /// its spot, upright, turned to the eye, as tall as its kind and as wide as its picture; the character's picture its
    /// full view in its being's mood, the item's its picture; each card moved only by its own look's links
    #[test]
    fn a_stage_without_3d_stands_its_cast_as_cards() {
        let (_g, mut space, mut ctl) = stage(false);
        assert!(!space.stage_3d());
        assert_eq!(space.cast.iter().map(|c| c.form).collect::<Vec<_>>(), [Form::Card, Form::Card, Form::Card]);
        assert_eq!((space.beings().count(), space.cards()), (0, 3), "no bodies: cards");
        // the pictures the stage asks for: a character's full view per mood, an item's one picture
        let asked = space.portraits.requests();
        let full = |id: &str| asked.iter().filter(|(_, p)| p.id == id && p.view == cast::CARD_VIEW).count();
        assert_eq!(full("catpope"), space.cast[0].being.moods.len());
        let item = asked.iter().find(|(_, p)| p.kind == "items").map(|(s, p)| (*s, p.stem()));
        assert_eq!(item, Some((space.cast[2].pictures[0], "incense_burner".to_string())));
        // her neutral full view arrives (1 : 2), the burner's picture (2 : 1)
        let (pope, _) = asked.iter().find(|(_, p)| p.id == "catpope" && p.view == cast::CARD_VIEW && p.mood == space.cast[0].being.moods[0]).unwrap().clone();
        space.portraits.deliver(pope, Ok(image::RgbaImage::new(50, 100)));
        space.portraits.deliver(item.unwrap().0, Ok(image::RgbaImage::new(80, 40)));
        let mut pulses = (Vec::new(), Vec::new());
        let mut cards = Vec::with_capacity(space.cards());
        run(&mut space, &mut ctl, 30, &mut |s| {
            s.write_cards(s.camera.eye(), &mut cards);
            pulses.0.push(cards[1].pulse);
            pulses.1.push(cards[2].pulse);
        });
        assert!(spread(&pulses.1) > 0.3, "the burner's card moves with its own wire: {:?}", pulses.1);
        assert_eq!(spread(&pulses.0), 0.0, "the listener's card has no mapping: still");
        let eye = space.camera.eye();
        space.write_cards(eye, &mut cards);
        let (her, his, burner) = (cards[0], cards[1], cards[2]);
        assert_eq!(her.picture, Some(pope), "her full view in her being's mood");
        assert_eq!(his.picture, None, "no picture yet: a blank standee");
        assert_eq!((her.up.y, burner.up.y), (1.7, 0.6), "a person's height, a thing's");
        assert!((her.right.length() * 2.0 / her.up.y - 0.5).abs() < 1e-3, "as wide as her picture");
        assert!((burner.right.length() * 2.0 / burner.up.y - 2.0).abs() < 1e-3, "as wide as its picture");
        for c in &cards {
            assert!(c.up.x == 0.0 && c.up.z == 0.0, "upright");
            assert!(c.right.y == 0.0 && c.right.dot(Vec3::new(eye.x - c.feet.x, 0.0, eye.z - c.feet.z)).abs() < 1e-3, "turned to the eye");
            assert!((c.feet.y - (space.being.anchor.y - space.lift)).abs() < 1e-4, "feet on the floor under the scene's spot");
        }
        assert!(her.feet.distance(his.feet) > 1.0, "side by side, not on one spot");
        let middle = (her.feet + his.feet + burner.feet) / 3.0 + Vec3::Y * (1.7 + 1.7 + 0.6) / 6.0;
        assert!(space.focus_goal(space.camera.distance).distance(middle) < 1e-4, "the camera aims at the cards' middle");
        let (s, t) = space.capacity();
        let (mut spheres, mut tubes) = (Vec::with_capacity(s), Vec::with_capacity(t));
        space.write(&mut spheres, &mut tubes);
        assert!(spheres.is_empty(), "no bodies drawn");
    }
}
