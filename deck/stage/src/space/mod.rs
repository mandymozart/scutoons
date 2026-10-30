//! The 3D space (#188 "Cathedral of decks"): a `space` node draws a glTF room (glb.rs, meshopt.rs) and the deck's
//! being (being.rs – its running graph as a floating body, its head the character's portrait – portrait.rs) through an
//! orbit camera (camera.rs) into its texture, with a depth buffer (gpu.rs, shaders/space.wgsl). The pointer picks the
//! being's 3D buttons by ray cast; a drag turns one like a MIDI CC (`Controls::set`), elsewhere it turns the camera.
//!
//! Phase 4 – the session (the space node's `join`, with a relay set): a **deck** streams its being to the relay
//! (stream.rs, link.rs); the **conductor**'s deck draws every deck's being in a circle around its own, bigger one in
//! the middle of the nave, and the hoses between them along the session's routes (decks.rs). Each deck shows only its
//! own being. This module holds the CPU side; the renderer draws what it says.
pub mod being;
pub mod camera;
pub mod decks;
pub mod feel;
pub mod glb;
pub mod gpu;
#[cfg(feature = "live")]
pub mod link;
pub mod meshopt;
pub mod portrait;
pub mod stream;
pub mod values;

use crate::control::Controls;
use crate::graph::Graph;
use crate::params;
use crate::tempo;
use anyhow::Result;
use being::{Being, Hit, Look, Sphere, TubeVertex};
use camera::Orbit;
use decks::{Remote, Route};
use glam::Vec3;
use portrait::Portraits;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use values::Local;

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

/// where the being floats when the room has no such spot (or there is no room)
const FALLBACK_ANCHOR: Vec3 = Vec3::new(0.0, 1.4, 0.0);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    None,
    /// turning the camera from the last pointer position
    Orbit(f32, f32),
    /// turning button `i`: its 0..1 and the pointer's y when it started
    Knob { button: usize, norm: f32, y: f32 },
}

pub struct Space {
    /// the space node
    pub node: usize,
    pub being: Being,
    pub camera: Orbit,
    /// the room, from its loader until the renderer takes it
    room_rx: Option<mpsc::Receiver<Result<glb::Room>>>,
    room: Option<glb::Room>,
    /// the room file shown (or loading) and its spots (kept after the renderer took the room – a Live cell moves the
    /// camera to one, #186)
    room_path: Option<PathBuf>,
    spots: Vec<(String, Vec3)>,
    max_texture: u32,
    /// the room's nodes left out (the space node's `hide`: their meshes – e.g. the cathedral's incense burner)
    hide: Vec<String>,
    /// what the HUD says about the room ("cathedral.low.glb 114788 tris", "loading …", the error)
    pub room_status: String,
    spot: String,
    lift: f32,
    /// an explicit place in the room (metres) instead of spot + lift
    at: Option<Vec3>,
    /// the room's light (the space node's `light`) this frame
    pub light: f32,
    pointer: Option<(f32, f32)>,
    view: (f32, f32),
    drag: Drag,
    /// this deck's graph (the stream reads it; a remote's comes from the wire)
    #[cfg_attr(not(feature = "live"), allow(dead_code))]
    graph: Graph,
    /// the heads' sprites
    pub portraits: Portraits,
    pub join: Join,
    /// a deck: what it streams of its being
    #[cfg_attr(not(feature = "live"), allow(dead_code))]
    stream: Option<stream::Stream>,
    #[cfg(feature = "live")]
    link: Option<link::Link>,
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
}

/// the room file a space node names: `room` with an extension = a path (relative to `rooms`), else
/// `<rooms>/<room>.<quality>.glb` (quality low · medium · high, default low – the X1's UHD 620)
pub fn room_path(graph: &Graph, node: usize, rooms: &Path) -> Option<PathBuf> {
    let n = &graph.nodes[node];
    let room = n.str("room")?;
    let p = Path::new(room);
    Some(if p.extension().is_some() {
        rooms.join(p)
    } else {
        rooms.join(format!("{room}.{}.glb", n.str("quality").unwrap_or("low")))
    })
}

/// a room file → its loader thread; the receiver and the HUD's "LOADING …"
fn load_room(path: PathBuf, max: u32, hide: Vec<String>) -> (Option<mpsc::Receiver<Result<glb::Room>>>, String) {
    let (tx, rx) = mpsc::channel();
    let shown = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    std::thread::Builder::new()
        .name("room".into())
        .spawn(move || {
            let t = std::time::Instant::now();
            let r = glb::load(&path, max, &hide);
            if let Ok(room) = &r {
                tracing::info!("room {}: {} triangles, {} images, {:.0} ms", path.display(), room.triangles(), room.images.len(), t.elapsed().as_secs_f32() * 1000.0);
            }
            let _ = tx.send(r);
        })
        .expect("a thread");
    (Some(rx), format!("LOADING {shown}"))
}

impl Space {
    /// The space node `node` of `graph`: its being from the graph + `look`, its room loading on a thread of its own
    /// (from `rooms`, the `--rooms` folder), its head's sprites from `portraits`.
    pub fn new(graph: &Graph, node: usize, look: &Look, rooms: &Path, mut portraits: Portraits) -> Space {
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
        let path = room_path(graph, node, rooms);
        let (room_rx, room_status) = match &path {
            Some(path) => load_room(path.clone(), max_texture, hide.clone()),
            None => (None, "NO ROOM".into()),
        };
        let camera = Orbit::new(aim(&being), n.f32("yaw", 20.0).to_radians(), n.f32("pitch", 8.0).to_radians(), n.f32("distance", 2.4));
        Space {
            node,
            being,
            camera,
            room_rx,
            room: None,
            room_path: path,
            spots: Vec::new(),
            max_texture,
            hide,
            room_status,
            spot: n.str("spot").unwrap_or("spot_aisle").to_string(),
            lift: n.f32("lift", 1.5),
            at,
            light: 1.0,
            pointer: None,
            view: (1.0, 1.0),
            drag: Drag::None,
            graph: graph.clone(),
            portraits,
            join,
            stream: None,
            #[cfg(feature = "live")]
            link: None,
            note: String::new(),
            others: Vec::new(),
            routes: Vec::new(),
            route_ids: Vec::new(),
            me: None,
            ring: n.f32("ring", RING),
            generation: 0,
            places: Vec::new(),
        }
    }

    /// Join the session at `relay` (`ws://host:port/live/<code>`) as the space node's `join` says – a deck streams its
    /// being, the conductor gathers the decks'. `token`: the conductor's session token (a dev relay takes any).
    #[cfg(feature = "live")]
    pub fn connect(&mut self, relay: Option<&str>, token: Option<String>, nick: Option<String>) {
        let join = self.join;
        if join == Join::Alone {
            return;
        }
        let Some(relay) = relay else {
            self.note = "NO RELAY SET (--relay)".into();
            return;
        };
        let graph = (join == Join::Deck).then(|| link::deck_graph(&self.graph, &self.being.look, &self.being.moods));
        match link::Link::start(relay, join, token, nick, graph) {
            Ok(l) => {
                if join == Join::Deck {
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

    /// One frame: the room's arrival, the camera, the being (its feelings' nudges into `ctl`), what the pointer is
    /// over; a deck's stream, the conductor's congregation.
    pub fn update(&mut self, dt: f32, time: f32, ctl: &mut Controls, beat: &tempo::Now, meter: &dyn Fn(usize, usize) -> f32) {
        if let Some(rx) = &self.room_rx {
            match rx.try_recv() {
                Ok(Ok(room)) => {
                    let spot = room.spot(&self.spot);
                    if spot.is_none() && self.at.is_none() {
                        tracing::warn!("room: no spot '{}' – the being floats over the room's centre", self.spot);
                    }
                    let base = spot.unwrap_or_else(|| Vec3::new((room.min.x + room.max.x) * 0.5, room.min.y, (room.min.z + room.max.z) * 0.5));
                    self.being.anchor = self.at.unwrap_or(base + Vec3::Y * self.lift);
                    self.camera.target = aim(&self.being);
                    self.room_status = format!("ROOM {}K TRIS", room.triangles() / 1000);
                    self.spots = room.spots.clone();
                    self.room = Some(room);
                    self.room_rx = None;
                }
                Ok(Err(e)) => {
                    tracing::error!("room: {e:#}");
                    self.room_status = format!("ROOM ERROR {e}");
                    self.room_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.room_rx = None,
            }
        }
        self.portraits.poll();
        self.light = ctl.input(self.node, "light", params::default(crate::graph::Kind::Space, "light"));
        self.camera.step(dt);
        self.being.update(dt, time, &Local { ctl, meter }, beat);
        self.being.push(ctl);
        let local = Local { ctl, meter };
        if self.drag == Drag::None {
            self.being.hover = self.pointer.and_then(|(x, y)| self.being.pick(&self.camera.ray(x, y, self.view.0, self.view.1)));
        }
        self.being.dragging = matches!(self.drag, Drag::Knob { .. });
        #[cfg(feature = "live")]
        self.session(time, &local);
        self.congregation(dt, time, beat);
    }

    /// a deck: its changes to the relay (≤ 30 / s); the conductor: the decks' beings, their values, presence
    #[cfg(feature = "live")]
    fn session(&mut self, time: f32, local: &Local) {
        let Some(l) = &self.link else { return };
        if let Some(s) = self.stream.as_mut() {
            if let Some(c) = s.sample(time, &self.graph, local, &self.being) {
                let values = c.values.into_iter().map(|(k, v)| (k, v.into_iter().map(f64::from).collect())).collect();
                l.send(protocol::DeckValues { values, mood: c.mood });
            }
        }
        while let Some(ev) = self.link.as_ref().and_then(|l| l.rx.try_recv().ok()) {
            match ev {
                link::In::Me(id) => self.me = Some(id),
                link::In::Deck { client, graph, values, mood } => match link::from_wire(&graph) {
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
                link::In::Values { client, values, mood } => {
                    if let Some(r) = self.others.iter_mut().find(|r| r.client == client) {
                        r.apply(&values, mood.as_deref());
                    }
                }
                link::In::Presence { decks, routes } => {
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

    /// the decks' places: a circle around this being's anchor (it moves with the room's spot), the first between two
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
        for r in &mut self.others {
            r.step(dt, time, beat);
        }
        let center = |i: Option<usize>| i.map_or(self.being.center, |i| self.others[i].being.center);
        for k in 0..self.routes.len() {
            let (a, b) = (center(self.routes[k].from), center(self.routes[k].to));
            let rate = self.routes[k].from.map_or(0.0, |i| self.others[i].rate);
            self.routes[k].step(dt, a, b, rate);
        }
    }

    /// every being: this deck's own first, then the decks'
    pub fn beings(&self) -> impl Iterator<Item = &Being> {
        std::iter::once(&self.being).chain(self.others.iter().map(|r| &r.being))
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
        out.push(format!("FACE {}", self.portraits.status(&self.being.look.character)));
        out.iter().map(|l| l.replace(['–', '—'], "-").to_uppercase().chars().take(40).collect()).collect()
    }

    /// A Live cell's 3D set (#186, #193: set + spot): another room file loads on its thread (the old one stays until
    /// it is there), and the being and the camera go to `spot` (in the new room once it has arrived). A spot the room
    /// lacks keeps the place – the room's status says so.
    pub fn show(&mut self, room: Option<&Path>, spot: Option<&str>) {
        if let Some(spot) = spot {
            self.spot = spot.to_owned();
            self.at = None;
        }
        match room {
            Some(p) if self.room_path.as_deref() != Some(p) => {
                (self.room_rx, self.room_status) = load_room(p.to_path_buf(), self.max_texture, self.hide.clone());
                self.room_path = Some(p.to_path_buf());
            }
            _ => {
                if let Some(pos) = self.spots.iter().find(|(n, _)| *n == self.spot).map(|(_, p)| *p) {
                    self.being.anchor = pos + Vec3::Y * self.lift;
                    self.camera.target = aim(&self.being);
                } else if spot.is_some() && self.room_rx.is_none() {
                    tracing::warn!("room: no spot '{}' – the being stays", self.spot);
                }
            }
        }
    }

    /// the room, once – the renderer uploads it
    pub fn take_room(&mut self) -> Option<glb::Room> {
        self.room.take()
    }

    /// the window's size in pixels (the pointer's frame)
    pub fn set_view(&mut self, w: f32, h: f32) {
        self.view = (w.max(1.0), h.max(1.0));
    }

    pub fn pointer_moved(&mut self, x: f32, y: f32, ctl: &mut Controls) {
        match self.drag {
            Drag::Orbit(lx, ly) => {
                self.camera.drag(x - lx, y - ly);
                self.drag = Drag::Orbit(x, y);
            }
            Drag::Knob { button, norm, y: y0 } => {
                let b = &self.being.buttons[button];
                // the window's height ≈ 2.5 full turns: fine enough, quick enough
                let n = (norm + (y0 - y) / (self.view.1 * 0.4)).clamp(0.0, 1.0);
                if b.shape {
                    ctl.set_lfo_shape(b.node, b.spec.value(n).round() as usize);
                } else {
                    ctl.set(b.node, b.port, b.spec.value(n));
                }
                self.camera.idle = 0.0;
            }
            Drag::None => {}
        }
        self.pointer = Some((x, y));
    }

    pub fn pointer_left(&mut self) {
        self.pointer = None;
        self.being.hover = None;
    }

    /// the primary button down: over an unlinked button = turn it, else turn the camera
    pub fn press(&mut self) {
        let Some((x, y)) = self.pointer else { return };
        let hit = self.being.pick(&self.camera.ray(x, y, self.view.0, self.view.1));
        self.being.hover = hit;
        self.drag = match hit {
            Some(Hit::Button(i)) if !self.being.buttons[i].linked => Drag::Knob { button: i, norm: self.being.buttons[i].norm, y },
            _ => Drag::Orbit(x, y),
        };
    }

    pub fn release(&mut self) {
        self.drag = Drag::None;
    }

    /// The HUD's line: the hovered button (its node, name, value, range – or where its hose comes from), the hovered
    /// node, else the room's state. ≤ 40 characters.
    pub fn hud_line(&self, graph: &Graph) -> String {
        let line = match self.being.hover {
            Some(Hit::Button(i)) => {
                let b = &self.being.buttons[i];
                let id = &graph.nodes[b.node].id;
                if b.shape {
                    let names = crate::control::LFO_SHAPES;
                    return format!("{id}.shape {} [{}]", names[(b.value as usize).min(names.len() - 1)], names.join(" ")).to_uppercase().chars().take(40).collect();
                }
                match graph.link_into(b.node, b.port) {
                    Some(l) => format!("{id}.{} {} < {}", b.name, num(b.value), graph.nodes[l.from.0].id),
                    None => format!("{id}.{} {} [{}-{}]{}", b.name, num(b.value), num(b.spec.min), num(b.spec.max), if self.being.dragging { " *" } else { "" }),
                }
            }
            Some(Hit::Point(i)) => {
                let n = &graph.nodes[self.being.points[i].node];
                format!("NODE {} ({:?})", n.id, n.kind)
            }
            None => self.room_status.clone(),
        };
        line.to_uppercase().chars().take(40).collect()
    }
}

/// a value in ≤ 6 characters
fn num(v: f32) -> String {
    match v.abs() {
        a if a >= 1000.0 => format!("{v:.0}"),
        a if a >= 10.0 => format!("{v:.1}"),
        _ => format!("{v:.2}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Scene, CATHEDRAL_SCENE};

    #[test]
    fn room_paths_follow_the_quality() {
        let g = Graph::build(&Scene::parse("[[node]]\nid=\"s\"\nkind=\"space\"\nroom=\"cathedral\"\nquality=\"medium\"\n").unwrap()).unwrap();
        assert_eq!(room_path(&g, 0, Path::new("/r")), Some(Path::new("/r").join("cathedral.medium.glb")));
        let g = Graph::build(&Scene::parse("[[node]]\nid=\"s\"\nkind=\"space\"\nroom=\"x/plain.glb\"\n").unwrap()).unwrap();
        assert_eq!(room_path(&g, 0, Path::new("/r")), Some(Path::new("/r").join("x/plain.glb")));
        let g = Graph::build(&Scene::parse("[[node]]\nid=\"s\"\nkind=\"space\"\n").unwrap()).unwrap();
        assert_eq!(room_path(&g, 0, Path::new("/r")), None);
    }

    /// the owner (2026-10-30): the whole mood set, every mood reachable – each has a rule and a nudge in both bundled
    /// scenes, all of them resolve; the conductor's rules read a deck's graph too
    #[test]
    fn every_mood_has_a_rule_and_a_nudge() {
        use crate::scene::CONDUCTOR_SCENE;
        let cathedral = crate::modules::carried_part(&Scene::parse(CATHEDRAL_SCENE).unwrap());
        let deck = Graph::build(&cathedral).unwrap();
        for text in [CATHEDRAL_SCENE, CONDUCTOR_SCENE] {
            let scene = crate::modules::carried_part(&Scene::parse(text).unwrap());
            let g = Graph::build(&scene).unwrap();
            let b = Being::new(&g, &scene.being);
            assert!(b.feel.problems.is_empty(), "{}: {:?}", scene.stage.name, b.feel.problems);
            for (mood, _) in portrait::NEAR {
                assert!(scene.being.feel.iter().any(|r| r.mood == mood), "{}: no rule for {mood}", scene.stage.name);
                assert!(scene.being.react.iter().any(|n| n.mood == mood), "{}: no nudge for {mood}", scene.stage.name);
            }
            assert_eq!(b.moods.len(), portrait::NEAR.len(), "the palette is the whole set");
        }
        let conductor = crate::modules::carried_part(&Scene::parse(CONDUCTOR_SCENE).unwrap());
        let look = being::Look { feel: conductor.being.feel.clone(), ..cathedral.being.clone() };
        let mirrored = Being::new(&deck, &look);
        assert!(mirrored.feel.problems.is_empty(), "the conductor's rules on a deck's graph: {:?}", mirrored.feel.problems);
    }

    #[test]
    fn a_drag_on_a_button_sets_its_input_and_elsewhere_turns_the_camera() {
        let scene = crate::modules::carried_part(&Scene::parse(CATHEDRAL_SCENE).unwrap());
        let g = Graph::build(&scene).unwrap();
        let node = g.first(crate::graph::Kind::Space).unwrap();
        let mut space = Space::new(&g, node, &scene.being, Path::new("/nowhere"), Portraits::local(Path::new("/nowhere")));
        let mut ctl = Controls::new(&g, None);
        space.set_view(1280.0, 720.0);
        let beat = tempo::Now::default();
        space.update(0.016, 0.0, &mut ctl, &beat, &|_, _| 0.0);
        // aim at an unlinked button: project its centre into the window
        let (i, b) = space.being.buttons.iter().enumerate().find(|(_, b)| !b.linked).unwrap();
        let clip = space.camera.view_proj(1280.0 / 720.0) * b.pos.extend(1.0);
        let (x, y) = ((clip.x / clip.w + 1.0) * 640.0, (1.0 - clip.y / clip.w) * 360.0);
        space.pointer_moved(x, y, &mut ctl);
        space.update(0.0, 0.0, &mut ctl, &beat, &|_, _| 0.0);
        let Some(Hit::Button(hit)) = space.being.hover else { panic!("the button under the pointer: {:?}", space.being.hover) };
        let (node, port, before) = (space.being.buttons[hit].node, space.being.buttons[hit].port, space.being.buttons[hit].norm);
        let _ = i;
        space.press();
        space.pointer_moved(x, y - 72.0, &mut ctl); // up a tenth of the window = +0.25
        let name = g.nodes[node].kind.inputs()[port].name;
        let spec = params::spec(g.nodes[node].kind, name);
        let want = spec.value((before + 0.25).min(1.0));
        assert!((ctl.input(node, name, -1.0) - want).abs() < 1e-3 * want.abs().max(1.0), "{name}");
        assert!(space.hud_line(&g).contains(&name.to_uppercase()));
        space.release();
        // an empty spot: the camera turns
        let yaw = space.camera.yaw;
        space.pointer_moved(5.0, 5.0, &mut ctl);
        space.press();
        space.pointer_moved(105.0, 5.0, &mut ctl);
        assert!(space.camera.yaw != yaw);
        space.release();
        assert!(space.hud_line(&g).starts_with("ROOM ERROR") || space.hud_line(&g).starts_with("LOADING"));
    }
}
