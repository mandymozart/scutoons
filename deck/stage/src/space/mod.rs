//! The 3D space (#188 "Cathedral of decks", phases 1–3): a `space` node draws a glTF room (glb.rs, meshopt.rs) and the
//! deck's being (being.rs – its running graph as a floating body) through an orbit camera (camera.rs) into its texture,
//! with a depth buffer (gpu.rs, shaders/space.wgsl). The pointer picks the being's 3D buttons by ray cast; a drag turns
//! one like a MIDI CC (`Controls::set`), elsewhere it turns the camera. This module holds the CPU side: the room's
//! loader thread, the camera, the being, the pointer; the renderer draws what it says.
pub mod being;
pub mod camera;
pub mod glb;
pub mod gpu;
pub mod meshopt;
pub mod params;

use crate::control::Controls;
use crate::graph::Graph;
use crate::tempo;
use anyhow::Result;
use being::{Being, Hit, Look};
use camera::Orbit;
use glam::Vec3;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

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

impl Space {
    /// The space node `node` of `graph`: its being from the graph + `look`, its room loading on a thread of its own
    /// (from `rooms`, the `--rooms` folder).
    pub fn new(graph: &Graph, node: usize, look: &Look, rooms: &Path) -> Space {
        let n = &graph.nodes[node];
        let mut being = Being::new(graph, look);
        let at = match n.f32s("at").as_slice() {
            [x, y, z] => Some(Vec3::new(*x, *y, *z)),
            _ => None,
        };
        being.anchor = at.unwrap_or(FALLBACK_ANCHOR);
        let (room_rx, room_status) = match room_path(graph, node, rooms) {
            Some(path) => {
                let (tx, rx) = mpsc::channel();
                let max = n.f32("max_texture", 2048.0) as u32;
                let shown = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
                std::thread::Builder::new()
                    .name("room".into())
                    .spawn(move || {
                        let t = std::time::Instant::now();
                        let r = glb::load(&path, max);
                        if let Ok(room) = &r {
                            tracing::info!("room {}: {} triangles, {} images, {:.0} ms", path.display(), room.triangles(), room.images.len(), t.elapsed().as_secs_f32() * 1000.0);
                        }
                        let _ = tx.send(r);
                    })
                    .expect("a thread");
                (Some(rx), format!("LOADING {shown}"))
            }
            None => (None, "NO ROOM".into()),
        };
        let camera = Orbit::new(being.anchor, n.f32("yaw", 20.0).to_radians(), n.f32("pitch", 8.0).to_radians(), n.f32("distance", 2.4));
        Space {
            node,
            being,
            camera,
            room_rx,
            room: None,
            room_status,
            spot: n.str("spot").unwrap_or("spot_aisle").to_string(),
            lift: n.f32("lift", 1.5),
            at,
            light: 1.0,
            pointer: None,
            view: (1.0, 1.0),
            drag: Drag::None,
        }
    }

    /// One frame: the room's arrival, the camera, the being, what the pointer is over.
    pub fn update(&mut self, dt: f32, time: f32, ctl: &Controls, beat: &tempo::Now, meter: &dyn Fn(usize, usize) -> f32) {
        if let Some(rx) = &self.room_rx {
            match rx.try_recv() {
                Ok(Ok(room)) => {
                    let spot = room.spot(&self.spot);
                    if spot.is_none() && self.at.is_none() {
                        tracing::warn!("room: no spot '{}' – the being floats over the room's centre", self.spot);
                    }
                    let base = spot.unwrap_or_else(|| Vec3::new((room.min.x + room.max.x) * 0.5, room.min.y, (room.min.z + room.max.z) * 0.5));
                    self.being.anchor = self.at.unwrap_or(base + Vec3::Y * self.lift);
                    self.camera.target = self.being.anchor;
                    self.room_status = format!("ROOM {}K TRIS", room.triangles() / 1000);
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
        self.light = ctl.input(self.node, "light", params::default(crate::graph::Kind::Space, "light"));
        self.camera.step(dt);
        self.being.update(dt, time, ctl, beat, meter);
        if self.drag == Drag::None {
            self.being.hover = self.pointer.and_then(|(x, y)| self.being.pick(&self.camera.ray(x, y, self.view.0, self.view.1)));
        }
        self.being.dragging = matches!(self.drag, Drag::Knob { .. });
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

    #[test]
    fn a_drag_on_a_button_sets_its_input_and_elsewhere_turns_the_camera() {
        let scene = Scene::parse(CATHEDRAL_SCENE).unwrap();
        let g = Graph::build(&scene).unwrap();
        let node = g.first(crate::graph::Kind::Space).unwrap();
        let mut space = Space::new(&g, node, &scene.being, Path::new("/nowhere"));
        let mut ctl = Controls::new(&g, None);
        space.set_view(1280.0, 720.0);
        let beat = tempo::Now::default();
        space.update(0.016, 0.0, &ctl, &beat, &|_, _| 0.0);
        // aim at an unlinked button: project its centre into the window
        let (i, b) = space.being.buttons.iter().enumerate().find(|(_, b)| !b.linked).unwrap();
        let clip = space.camera.view_proj(1280.0 / 720.0) * b.pos.extend(1.0);
        let (x, y) = ((clip.x / clip.w + 1.0) * 640.0, (1.0 - clip.y / clip.w) * 360.0);
        space.pointer_moved(x, y, &mut ctl);
        space.update(0.0, 0.0, &ctl, &beat, &|_, _| 0.0);
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
