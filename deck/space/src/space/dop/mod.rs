//! The DOP: the scene's primary camera playing – its shots as views, its moves on the tempo clock, its trolley track as a
//! path. A move starts from wherever the camera is, runs over beats, bars (the tempo clock) or seconds, eased, and ends
//! exactly on its shot; it may wait for the bar line, chain to the next, and a hand on the view interrupts it where it
//! is – the orbit camera goes on from that view, nothing jumps. A follow stays on its subject until a hand or another move.
pub mod moves;
pub mod track;

use super::camera::Orbit;
use crate::rig::address::{CameraWord, Target};
use crate::scene_camera::{focal_of, Lens, MoveKind, MoveSpec, Projection, SceneCamera, ShotDef, Start};
use crate::tempo;
use glam::Vec3;
use std::collections::VecDeque;
use track::Path;

/// Where the camera is and how it sees: aimed at `target` from `yaw` / `pitch` (rad) at `distance` (m), through `lens`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub lens: Lens,
}

/// the way from the aim to the eye (unit length)
pub(crate) fn back(yaw: f32, pitch: f32) -> Vec3 {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    Vec3::new(cp * sy, sp, cp * cy)
}

fn wrap_angle(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    a - tau * (a / tau).floor()
}

/// an angle as the shorter turn, in −π..π
pub(crate) fn wrap_pi(a: f32) -> f32 {
    let pi = std::f32::consts::PI;
    wrap_angle(a + pi) - pi
}

impl View {
    pub fn of_shot(s: &ShotDef) -> View {
        View { target: Vec3::from_array(s.target), yaw: s.yaw.to_radians(), pitch: s.pitch.to_radians(), distance: s.distance, lens: s.lens }
    }

    /// the orbit camera's view now
    pub fn of(c: &Orbit) -> View {
        let lens = match c.ortho {
            Some(size) => Lens { projection: Projection::Orthographic, size, focal: focal_of(c.fov_y) },
            None => Lens { projection: Projection::Perspective, focal: focal_of(c.fov_y), size: Lens::default().size },
        };
        View { target: c.target, yaw: c.yaw, pitch: c.pitch, distance: c.distance, lens }
    }

    pub fn eye(&self) -> Vec3 {
        self.target + self.distance * back(self.yaw, self.pitch)
    }

    pub fn from_eye(eye: Vec3, target: Vec3, lens: Lens) -> View {
        let d = eye - target;
        let distance = d.length().max(1e-4);
        let dir = d / distance;
        View { target, yaw: dir.x.atan2(dir.z), pitch: dir.y.clamp(-1.0, 1.0).asin(), distance, lens }
    }

    pub(crate) fn wrapped(mut self) -> View {
        self.yaw = wrap_angle(self.yaw);
        self
    }

    /// the shot this view would be stored as (degrees)
    pub fn shot(&self, id: &str) -> ShotDef {
        let r = |v: f32| (v * 1000.0).round() / 1000.0;
        let t = self.target;
        ShotDef {
            id: id.to_string(),
            label: Default::default(),
            target: [r(t.x), r(t.y), r(t.z)],
            yaw: r(wrap_pi(self.yaw).to_degrees()),
            pitch: r(self.pitch.to_degrees().clamp(-90.0, 90.0)),
            distance: r(self.distance.max(crate::scene_camera::DISTANCE_RANGE.0)),
            lens: self.lens,
            aim: None,
        }
    }

    /// the orbit camera looks through this view at once (its glides have nothing left to do)
    pub fn apply(&self, c: &mut Orbit) {
        c.through = None;
        // a blend's eye far back on its way is no reach for the hand
        if !c.far_back {
            c.max_distance = c.max_distance.max(self.distance);
        }
        (c.target, c.goal, c.yaw, c.pitch) = (self.target, self.target, self.yaw, self.pitch);
        (c.distance, c.want) = (self.distance, self.distance);
        c.fov_y = self.lens.fov_y();
        c.ortho = self.lens.ortho().then_some(self.lens.size);
    }

    /// how far apart two views are: the eyes' and the aims' distance (m) and the lens's field (rad)
    pub fn gap(&self, o: &View) -> f32 {
        self.eye().distance(o.eye()) + self.target.distance(o.target) + (self.lens.fov_y() - o.lens.fov_y()).abs() + (self.lens.size - o.lens.size).abs() * f32::from(u8::from(self.lens.ortho() || o.lens.ortho()))
    }
}

/// How a camera shows itself in an editor's 3D (never on a deck's output): its view as a pyramid from the eye to the
/// frame at its aim's depth – orthographic, a box from the eye's plane to the aim's – and the aim, its focal point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum {
    pub eye: Vec3,
    pub aim: Vec3,
    /// the frame at the aim's depth: top left, top right, bottom right, bottom left
    pub far: [Vec3; 4],
    /// orthographic: the same frame at the eye; none = the corner lines meet at the eye
    pub near: Option<[Vec3; 4]>,
}

impl Frustum {
    /// from `eye` toward `aim` with a vertical field of view `fov_y` (rad), or an orthographic view `ortho` m high, on a
    /// screen of `aspect` (w / h)
    pub fn of(eye: Vec3, aim: Vec3, fov_y: f32, ortho: Option<f32>, aspect: f32) -> Frustum {
        let depth = eye.distance(aim).max(1e-3);
        let fwd = (aim - eye) / depth;
        let right = fwd.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
        let up = right.cross(fwd);
        let half = ortho.map_or(depth * (fov_y * 0.5).tan(), |h| h * 0.5);
        let (r, u) = (right * half * aspect.max(1e-3), up * half);
        let frame = |c: Vec3| [c - r + u, c + r + u, c + r - u, c - r - u];
        Frustum { eye, aim, far: frame(aim), near: ortho.map(|_| frame(eye)) }
    }

    /// its lines: the corner lines (from the eye, or the near frame's corners) and the far frame; the near frame too
    pub fn lines(&self) -> Vec<(Vec3, Vec3)> {
        let ring = |f: [Vec3; 4]| (0..4).map(move |i| (f[i], f[(i + 1) % 4]));
        let mut out: Vec<(Vec3, Vec3)> = (0..4).map(|i| (self.near.map_or(self.eye, |n| n[i]), self.far[i])).collect();
        out.extend(ring(self.far));
        if let Some(n) = self.near {
            out.extend(ring(n));
        }
        out
    }
}

impl View {
    pub fn frustum(&self, aspect: f32) -> Frustum {
        Frustum::of(self.eye(), self.target, self.lens.fov_y(), self.lens.ortho().then_some(self.lens.size), aspect)
    }
}

/// the tempo clock and the show's time, as a move reads them
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clock {
    pub secs: f64,
    pub beat: f64,
    pub bpm: f64,
    pub per_bar: u32,
    pub running: bool,
}

impl Clock {
    pub fn of(secs: f32, now: &tempo::Now) -> Clock {
        Clock { secs: f64::from(secs), beat: now.pos.beat, bpm: now.bpm.max(1.0), per_bar: now.per_bar.max(1), running: now.running }
    }

    /// the first bar line at or after `beat`
    pub fn bar_after(&self, beat: f64) -> f64 {
        let per = f64::from(self.per_bar);
        let b = (beat / per).ceil() * per;
        if b - beat > per - 1e-6 {
            b - per
        } else {
            b
        }
    }

    /// the show's time at `beat` (the tempo held)
    fn secs_at(&self, beat: f64) -> f64 {
        self.secs + (beat - self.beat) * 60.0 / self.bpm
    }

    fn beat_at(&self, secs: f64) -> f64 {
        self.beat + (secs - self.secs) * self.bpm / 60.0
    }
}

/// A move to play: how, to which view, named for the log.
#[derive(Debug, Clone, PartialEq)]
pub struct Cue {
    pub spec: MoveSpec,
    pub to: View,
    /// the shot it ends on
    pub shot: String,
    /// what that shot aims at by address (its `aim`): the move ends on it where it stands
    pub aim: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Base {
    Beats,
    Secs,
}

#[derive(Debug, Clone)]
struct Run {
    cue: Cue,
    /// the view it starts on; none until it starts (a move waiting for the bar line starts from where the camera is then)
    from: Option<View>,
    base: Base,
    at: f64,
    len: f64,
}

/// Plays cues: one at a time, the next from where the last ended.
#[derive(Debug, Clone, Default)]
pub struct Director {
    run: Option<Run>,
    queue: VecDeque<Cue>,
    /// a follow that arrived: it keeps its shot on the subject
    hold: Option<Cue>,
    /// where and when the last move ended – the next starts there
    ended: Option<(View, f64)>,
}

/// where an aim address (rig/address.rs `Aim`) stands now: a follow's subject, a shot's aim
pub type Subject<'a> = &'a dyn Fn(&str) -> Option<Vec3>;

impl Director {
    /// `cues` in a chain, in place of anything playing or queued; the first starts from where the camera is
    pub fn play(&mut self, cues: impl IntoIterator<Item = Cue>) {
        self.interrupt();
        self.queue.extend(cues);
    }

    /// `cue` after everything playing or queued
    pub fn queue(&mut self, cue: Cue) {
        self.queue.push_back(cue);
    }

    /// a hand on the view: everything stops where it is
    pub fn interrupt(&mut self) {
        (self.run, self.hold, self.ended) = (None, None, None);
        self.queue.clear();
    }

    /// playing, waiting, queued or holding a subject
    pub fn active(&self) -> bool {
        self.run.is_some() || self.hold.is_some() || !self.queue.is_empty()
    }

    /// a move between a perspective and an orthographic lens on its way (moves.rs `blended`)
    pub fn blending(&self, clock: &Clock) -> bool {
        self.run.as_ref().is_some_and(|r| {
            let p = progress(r, clock);
            r.from.is_some_and(|f| f.lens.ortho() != r.cue.to.lens.ortho()) && p > 0.0 && p < 1.0 && r.cue.spec.kind != MoveKind::Cut
        })
    }

    /// the cue playing (or waiting to) and how far it is (0..1)
    pub fn playing(&self, clock: &Clock) -> Option<(&Cue, f32)> {
        self.run.as_ref().map(|r| (&r.cue, progress(r, clock).clamp(0.0, 1.0) as f32))
    }

    fn begin(&mut self, cue: Cue, now: &View, clock: &Clock) {
        let (from, at_secs) = match self.ended.take() {
            Some((v, secs)) => (Some(v), secs),
            None => (None, clock.secs),
        };
        let at_beat = clock.beat_at(at_secs);
        let tempo = matches!(cue.spec.length, crate::scene_camera::Length::Beats(_) | crate::scene_camera::Length::Bars(_));
        let run = if clock.running && (tempo || cue.spec.start == Start::Bar) {
            let at = if cue.spec.start == Start::Bar { clock.bar_after(at_beat) } else { at_beat };
            Run { len: cue.spec.length.beats(clock.per_bar, clock.bpm), cue, from, base: Base::Beats, at }
        } else {
            Run { len: cue.spec.length.seconds(clock.per_bar, clock.bpm), cue, from, base: Base::Secs, at: at_secs }
        };
        self.run = Some(run);
        if self.run.as_ref().is_some_and(|r| r.from.is_none() && progress(r, clock) >= 0.0) {
            if let Some(r) = self.run.as_mut() {
                r.from = Some(*now);
            }
        }
    }

    /// The view at `clock` – None while nothing plays (or a move waits for its bar line): the camera is the hand's.
    pub fn step(&mut self, clock: &Clock, now: View, track: Option<&Path>, subject: Subject) -> Option<View> {
        let ctx = moves::Ctx { track, secs: clock.secs };
        loop {
            if self.run.is_none() {
                match self.queue.pop_front() {
                    Some(cue) => {
                        self.hold = None;
                        self.begin(cue, &now, clock);
                    }
                    None => {
                        self.ended = None;
                        return self.hold.as_ref().map(|c| aimed(c, subject));
                    }
                }
            }
            let run = self.run.as_mut()?;
            let p = progress(run, clock);
            if p < 0.0 {
                return None;
            }
            let from = *run.from.get_or_insert(now);
            let to = aimed(&run.cue, subject);
            if p < 1.0 {
                return Some(moves::pose(&run.cue.spec, &from, &to, p as f32, &ctx));
            }
            let end = match run.base {
                Base::Beats => clock.secs_at(run.at + run.len),
                Base::Secs => run.at + run.len,
            };
            let done = self.run.take().map(|r| r.cue);
            if self.queue.is_empty() {
                self.hold = done.filter(|c| c.spec.kind == MoveKind::Follow);
                return Some(to);
            }
            self.ended = Some((to, end));
        }
    }
}

fn progress(r: &Run, clock: &Clock) -> f64 {
    let now = match r.base {
        Base::Beats => clock.beat,
        Base::Secs => clock.secs,
    };
    if r.len <= 0.0 {
        return if now >= r.at { 1.0 } else { -1.0 };
    }
    (now - r.at) / r.len
}

/// a cue's end view, aimed where its follow's subject or its shot's aim stands now
fn aimed(c: &Cue, subject: Subject) -> View {
    let follow = c.spec.follow.as_deref().filter(|_| c.spec.kind == MoveKind::Follow);
    match follow.or(c.aim.as_deref()).and_then(subject) {
        Some(at) => View { target: at, ..c.to },
        None => c.to,
    }
}

/// What the stage looks through: the free view, the scene's primary camera (where it opens) or one of its shots.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Viewing {
    #[default]
    Free,
    Primary,
    Shot(String),
}

/// The scene's camera on a stage: its document, its track as a path, the director, the shot taken last.
#[derive(Debug, Clone, Default)]
pub struct Dop {
    pub camera: SceneCamera,
    /// the scene it belongs to (`scene:<id>`: the place a camera word must pick up); none = any
    pub scene: Option<String>,
    path: Option<Path>,
    pub director: Director,
    /// the shot the camera went to last
    pub current: Option<String>,
    /// the clock at the last frame
    pub clock: Option<Clock>,
    pub viewing: Viewing,
}

impl Dop {
    pub fn new(scene: Option<String>, camera: SceneCamera) -> Dop {
        let path = camera.track.as_ref().and_then(Path::of);
        Dop { camera, scene, path, ..Dop::default() }
    }

    /// the camera changed in an editor: what plays keeps playing
    pub fn edited(&mut self, camera: SceneCamera) {
        self.path = camera.track.as_ref().and_then(Path::of);
        self.camera = camera;
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_ref()
    }

    pub fn view(&self, shot: &str) -> Option<View> {
        self.camera.shot(shot).map(View::of_shot)
    }

    /// the shot `id` by a cut
    pub fn cut(&self, id: &str) -> Result<Cue, String> {
        let to = self.view(id).ok_or_else(|| format!("no shot '{id}'"))?;
        Ok(Cue { spec: MoveSpec::of(MoveKind::Cut), to, shot: id.to_string(), aim: self.aim_of(id) })
    }

    /// `spec` to the shot `to`
    pub fn cue(&self, spec: MoveSpec, to: &str) -> Result<Cue, String> {
        let view = self.view(to).ok_or_else(|| format!("no shot '{to}'"))?;
        Ok(Cue { spec, to: view, shot: to.to_string(), aim: self.aim_of(to) })
    }

    /// what shot `id` aims at by address
    pub fn aim_of(&self, id: &str) -> Option<String> {
        self.camera.shot(id).and_then(|s| s.aim.clone())
    }


    /// the named move `id` and its chain
    pub fn chain(&self, id: &str) -> Result<Vec<Cue>, String> {
        let chain = self.camera.chain(id);
        if chain.is_empty() {
            return Err(format!("no move '{id}'"));
        }
        chain.into_iter().map(|m| self.cue(m.spec.clone(), &m.to)).collect()
    }

    /// cues to play now, in place of what plays
    pub fn play(&mut self, cues: Vec<Cue>) {
        if let Some(c) = cues.last() {
            self.current = Some(c.shot.clone());
            self.viewing = Viewing::Shot(c.shot.clone());
        }
        self.director.play(cues);
    }

    /// An address action on this camera (`scene:nave`, `shot:wide` / `move:intro`): None = not a camera word of this
    /// scene; else what it did, or why not.
    pub fn word(&mut self, address: &str, action: &str) -> Option<Result<String, String>> {
        let word = CameraWord::parse(action)?;
        let mine = match &self.scene {
            Some(place) => Target::parse(address).is_ok_and(|t| t.picks_up(place)),
            None => Target::parse(address).is_ok_and(|t| t.kind == "scene"),
        };
        if !mine {
            return None;
        }
        let word = match word {
            Ok(w) => w,
            Err(e) => return Some(Err(e)),
        };
        let cues = match &word {
            CameraWord::Shot(id) => self.cut(id).map(|c| vec![c]),
            CameraWord::Move(id) => self.chain(id),
        };
        Some(cues.map(|c| {
            self.play(c);
            format!("camera: {action}")
        }))
    }

    /// the view at `clock` while a move plays (None: the camera is the hand's)
    pub fn step(&mut self, clock: &Clock, now: View, subject: Subject) -> Option<View> {
        self.clock = Some(*clock);
        self.director.step(clock, now, self.path.as_ref(), subject)
    }
}

#[cfg(test)]
mod tests;
