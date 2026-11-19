//! The DOP: the scene's primary camera playing – its shots as views, its moves on the tempo clock, its trolley track as a
//! path. A move starts from wherever the camera is, runs over beats, bars (the tempo clock) or seconds, eased, and ends
//! exactly on its shot; it may wait for the bar line, chain to the next, and a hand on the view interrupts it where it
//! is – the orbit camera goes on from that view, nothing jumps. A follow stays on its subject until a hand or another move;
//! a move that loops or ping-pongs runs its rounds until another move, a cut or a hand (moves in beats or bars on the bar
//! line); a fly-through along a timed track reaches each point on its time. Every view a move gives carries its focus
//! distance: a shot's subject (`focus_on`) measured from the eye each frame – a follow focus – else its lens's, else its aim's.
pub mod moves;
pub mod track;

use super::camera::Orbit;
use crate::rig::address::{CameraWord, Target};
use crate::scene_camera::{focal_of, Lens, MoveKind, MoveSpec, Projection, Repeat, SceneCamera, ShotDef, Start, FOCUS_RANGE};
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
            Some(size) => Lens { projection: Projection::Orthographic, size, focal: focal_of(c.fov_y), focus: None },
            None => Lens { projection: Projection::Perspective, focal: focal_of(c.fov_y), size: Lens::default().size, focus: None },
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
            focus_on: None,
            phones: None,
        }
    }

    /// its focus distance (m): the lens's, else at the aim
    pub fn focus(&self) -> f32 {
        self.lens.focus.unwrap_or(self.distance)
    }

    /// the point it is focused on: its focus distance along its line of sight
    pub fn focus_point(&self) -> Vec3 {
        let eye = self.eye();
        eye + (self.target - eye).try_normalize().unwrap_or(Vec3::NEG_Z) * self.focus()
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
    /// what that shot keeps in focus by address (its `focus_on`)
    pub focus_on: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Base {
    Beats,
    Secs,
}

impl Base {
    fn now(self, clock: &Clock) -> f64 {
        match self {
            Base::Beats => clock.beat,
            Base::Secs => clock.secs,
        }
    }
}

#[derive(Debug, Clone)]
struct Run {
    cue: Cue,
    /// the view it starts on; none until it starts (a move waiting for the bar line starts from where the camera is then)
    from: Option<View>,
    base: Base,
    at: f64,
    len: f64,
    /// a fly-through along a timed track: its points' knots
    knots: Option<Vec<track::Knot>>,
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
    /// the beat the next cue starts on (a show cell's camera word on its row's bar or phrase)
    on_beat: Option<f64>,
}

/// where an aim address (rig/address.rs `Aim`) stands now: a follow's subject, a shot's aim
pub type Subject<'a> = &'a dyn Fn(&str) -> Option<Vec3>;

impl Director {
    /// `cues` in a chain, in place of anything playing or queued; the first starts from where the camera is
    pub fn play(&mut self, cues: impl IntoIterator<Item = Cue>) {
        self.interrupt();
        self.queue.extend(cues);
    }

    /// `cues` as `play`, the first starting on `beat` of the show clock (none: as it would); what plays goes on until then
    pub fn play_on(&mut self, cues: impl IntoIterator<Item = Cue>, beat: Option<f64>) {
        let Some(beat) = beat else { return self.play(cues) };
        self.queue.clear();
        self.queue.extend(cues);
        self.on_beat = Some(beat);
    }

    /// `cue` after everything playing or queued
    pub fn queue(&mut self, cue: Cue) {
        self.queue.push_back(cue);
    }

    /// a hand on the view: everything stops where it is
    pub fn interrupt(&mut self) {
        (self.run, self.hold, self.ended, self.on_beat) = (None, None, None, None);
        self.queue.clear();
    }

    /// a move plays now (not one waiting for its beat, nor a follow holding its subject after it)
    pub fn moving(&self) -> bool {
        self.run.is_some()
    }

    /// playing, waiting, queued or holding a subject
    pub fn active(&self) -> bool {
        self.run.is_some() || self.hold.is_some() || !self.queue.is_empty()
    }

    /// a move between a perspective and an orthographic lens on its way (moves.rs `blended`)
    pub fn blending(&self, clock: &Clock) -> bool {
        self.run.as_ref().is_some_and(|r| {
            let (_, _, p) = round(r, clock);
            r.from.is_some_and(|f| f.lens.ortho() != r.cue.to.lens.ortho()) && p > 0.0 && p < 1.0 && r.cue.spec.kind != MoveKind::Cut
        })
    }

    /// the cue playing (or waiting to) and how far it is (0..1; a repeating move: in its round)
    pub fn playing(&self, clock: &Clock) -> Option<(&Cue, f32)> {
        self.run.as_ref().map(|r| (&r.cue, round(r, clock).2.clamp(0.0, 1.0) as f32))
    }

    fn begin(&mut self, cue: Cue, now: &View, clock: &Clock, track: Option<&Path>) {
        let (from, at_secs) = match self.ended.take() {
            Some((v, secs)) => (Some(v), secs),
            None => (None, clock.secs),
        };
        let on_beat = self.on_beat.take().filter(|_| clock.running);
        let at_beat = on_beat.unwrap_or_else(|| clock.beat_at(at_secs));
        let timed = track.filter(|p| cue.spec.kind == MoveKind::Fly && p.timed());
        let length = timed.map_or(cue.spec.length, |p| p.length(cue.spec.length));
        let knots = timed.and_then(|p| p.knots(cue.spec.length, clock.per_bar, clock.bpm));
        let tempo = matches!(length, crate::scene_camera::Length::Beats(_) | crate::scene_camera::Length::Bars(_));
        let run = if clock.running && (tempo || cue.spec.start == Start::Bar || on_beat.is_some()) {
            let at = if cue.spec.start == Start::Bar { clock.bar_after(at_beat) } else { at_beat };
            Run { len: length.beats(clock.per_bar, clock.bpm), cue, from, base: Base::Beats, at, knots }
        } else {
            Run { len: length.seconds(clock.per_bar, clock.bpm), cue, from, base: Base::Secs, at: at_secs, knots }
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
        // a camera word on a beat: what plays goes until then
        if self.on_beat.is_some_and(|b| !clock.running || clock.beat >= b) && !self.queue.is_empty() {
            (self.run, self.ended) = (None, None);
        }
        loop {
            if self.run.is_none() {
                match self.queue.pop_front() {
                    Some(cue) => {
                        self.hold = None;
                        self.begin(cue, &now, clock, track);
                    }
                    None => {
                        self.ended = None;
                        return self.hold.as_ref().map(|c| {
                            let v = aimed(c, subject);
                            focused(c, &v, v, subject)
                        });
                    }
                }
            }
            let run = self.run.as_mut()?;
            if progress(run, clock) < 0.0 {
                return None;
            }
            let from = *run.from.get_or_insert(now);
            let to = focused(&run.cue, &from, aimed(&run.cue, subject), subject);
            let ctx = moves::Ctx { track, secs: clock.secs, knots: run.knots.as_deref() };
            let (k, start, p) = round(run, clock);
            // ping-pong: every second round runs back
            let back = run.cue.spec.repeat == Repeat::PingPong && k % 2 == 1;
            let to_end = if back { from } else { to };
            if p < 1.0 {
                let lin = if back { 1.0 - p } else { p };
                return Some(moves::pose(&run.cue.spec, &from, &to, lin as f32, &ctx));
            }
            // a repeat waits for its next round, unless a move is queued after it
            if run.cue.spec.repeat != Repeat::Once && self.queue.is_empty() {
                return Some(to_end);
            }
            let end = match run.base {
                Base::Beats => clock.secs_at(start + run.len),
                Base::Secs => start + run.len,
            };
            let done = self.run.take().map(|r| r.cue);
            if self.queue.is_empty() {
                self.hold = done.filter(|c| c.spec.kind == MoveKind::Follow);
                return Some(to_end);
            }
            self.ended = Some((to_end, end));
        }
    }
}

fn progress(r: &Run, clock: &Clock) -> f64 {
    let now = r.base.now(clock);
    if r.len <= 0.0 {
        return if now >= r.at { 1.0 } else { -1.0 };
    }
    (now - r.at) / r.len
}

/// Which round a run plays (0 the first), where that round started, and how far it is in it (0..1; past 1 while a repeat
/// waits for its next round). A move in beats or bars starts each repeat on a bar line: a round that ends between two waits
/// for the next.
fn round(r: &Run, clock: &Clock) -> (u64, f64, f64) {
    let p = progress(r, clock);
    if r.cue.spec.repeat == Repeat::Once || r.len <= 0.0 || p < 1.0 {
        return (0, r.at, p);
    }
    let now = r.base.now(clock);
    let end = r.at + r.len;
    let (next, period) = match r.base {
        Base::Beats => {
            let per = f64::from(clock.per_bar.max(1));
            (clock.bar_after(end - 1e-9), (r.len / per - 1e-9).ceil().max(1.0) * per)
        }
        Base::Secs => (end, r.len),
    };
    if now < next {
        return (0, r.at, p);
    }
    let k = ((now - next) / period).floor();
    let start = next + k * period;
    (1 + k as u64, start, (now - start) / r.len)
}

/// a cue's end view, aimed where its follow's subject or its shot's aim stands now
fn aimed(c: &Cue, subject: Subject) -> View {
    let follow = c.spec.follow.as_deref().filter(|_| c.spec.kind == MoveKind::Follow);
    match follow.or(c.aim.as_deref()).and_then(subject) {
        Some(at) => View { target: at, ..c.to },
        None => c.to,
    }
}

/// A cue's end view with its focus distance: its subject's distance from the eye now, else its lens's (none: at its aim)
/// – a rack focus keeps the frame it starts on (`from`) and measures from that eye, its aim's distance included.
fn focused(c: &Cue, from: &View, to: View, subject: Subject) -> View {
    let rack = c.spec.kind == MoveKind::Rack;
    let base = if rack { *from } else { to };
    let eye = base.eye();
    let on = c.focus_on.as_deref().and_then(subject).map(|p| eye.distance(p)).or(to.lens.focus);
    let focus = if rack { Some(on.unwrap_or_else(|| eye.distance(to.target))) } else { on };
    View { lens: Lens { focus: focus.map(|f| f.clamp(FOCUS_RANGE.0, FOCUS_RANGE.1)), ..base.lens }, ..base }
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
    /// the shot whose focus holds once nothing plays, when it is not the one looked through (a rack focus ended)
    pub focus_shot: Option<String>,
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
        Ok(Cue { spec: MoveSpec::of(MoveKind::Cut), to, shot: id.to_string(), aim: self.aim_of(id), focus_on: self.focus_on(id) })
    }

    /// `spec` to the shot `to`
    pub fn cue(&self, spec: MoveSpec, to: &str) -> Result<Cue, String> {
        let view = self.view(to).ok_or_else(|| format!("no shot '{to}'"))?;
        Ok(Cue { spec, to: view, shot: to.to_string(), aim: self.aim_of(to), focus_on: self.focus_on(to) })
    }

    /// what shot `id` aims at by address
    pub fn aim_of(&self, id: &str) -> Option<String> {
        self.camera.shot(id).and_then(|s| s.aim.clone())
    }

    /// what shot `id` keeps in focus by address
    pub fn focus_on(&self, id: &str) -> Option<String> {
        self.camera.shot(id).and_then(|s| s.focus_on.clone())
    }

    /// the shot whose focus holds when nothing plays: a rack's, else the one looked through (the primary camera: its first)
    pub fn focus_holder(&self) -> Option<&ShotDef> {
        let id = match (&self.focus_shot, &self.viewing) {
            (_, Viewing::Free) => return None,
            (Some(id), _) | (None, Viewing::Shot(id)) => id.as_str(),
            (None, Viewing::Primary) => return self.camera.opening(),
        };
        self.camera.shot(id)
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
        self.play_on(cues, None);
    }

    /// cues to play from `beat` of the show clock (none: now), in place of what plays then
    pub fn play_on(&mut self, cues: Vec<Cue>, beat: Option<f64>) {
        if let Some(c) = cues.last() {
            if cues.iter().all(|c| c.spec.kind == MoveKind::Rack) {
                // the frame stays: only the focus is the shot's
                self.focus_shot = Some(c.shot.clone());
            } else {
                self.current = Some(c.shot.clone());
                self.viewing = Viewing::Shot(c.shot.clone());
                self.focus_shot = (c.spec.kind == MoveKind::Rack).then(|| c.shot.clone());
            }
        }
        self.director.play_on(cues, beat);
    }

    /// An address action on this camera (`scene:nave`, `shot:wide` / `move:intro`): None = not a camera word of this
    /// scene; else what it did, or why not.
    pub fn word(&mut self, address: &str, action: &str) -> Option<Result<String, String>> {
        self.word_on(address, action, None)
    }

    /// `word`, played from `beat` of the show clock (a show cell's camera word on its row's bar or phrase)
    pub fn word_on(&mut self, address: &str, action: &str, beat: Option<f64>) -> Option<Result<String, String>> {
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
            self.play_on(c, beat);
            match beat {
                Some(b) => format!("camera: {action} on beat {b}"),
                None => format!("camera: {action}"),
            }
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
