//! A scene's primary camera (the DOP panel's document): its source, its lens, its shots, its trolley track and its named
//! moves. Stored on the scene document (`camera`), carried into the played scene, checked here; the move engine that plays
//! it is space/dop.
//!
//! - **Lens:** perspective with a focal length in mm (the presets, or a typed value) on a full-frame sensor, or orthographic
//!   with a view height in metres (isometric views).
//! - **Shot:** where the camera stands – aim, yaw and pitch in degrees, distance in metres, its lens. Picked from outside by
//!   address: `scene:<id> > shot:<shot>` cuts to it (rig/address.rs `CameraWord`).
//! - **Track:** the trolley track a fly-through runs along: a start and an end point, points between them, each with an
//!   optional Bézier handle (none = a smooth curve through its neighbours).
//! - **Move:** a kind, the shot it ends on, a length in beats, bars or seconds, an ease, whether it waits for the bar line,
//!   the move it chains to; `scene:<id> > move:<move>` plays it.
use crate::rig::address::word;
use crate::scene_document::Label;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// the focal presets (mm), the lens control's segments
pub const FOCAL_PRESETS: [f32; 6] = [17.0, 30.0, 50.0, 70.0, 100.0, 150.0];
/// a typed focal length's range (mm): fisheye to a long tele
pub const FOCAL_RANGE: (f32, f32) = (4.0, 1200.0);
/// an orthographic view's height range (m)
pub const SIZE_RANGE: (f32, f32) = (0.1, 500.0);
/// the sensor a focal length is read on: full frame, 24 mm high
pub const SENSOR_H_MM: f32 = 24.0;
/// a shot's distance range (m)
pub const DISTANCE_RANGE: (f32, f32) = (0.05, 500.0);
/// the most rounds an orbit or a spiral turns
pub const MAX_ROUNDS: f32 = 16.0;

/// The eye heights a camera stands at (m above the stage's floor; the cast's cards stand ~1.7 m): each looks level at
/// its aim, the bird down from high above.
pub const EYE_HEIGHTS: [(&str, f32); 7] = [("frog", 0.1), ("cat", 0.3), ("kid", 1.1), ("adult", 1.6), ("giant", 3.0), ("giraffe", 5.0), ("bird", 9.0)];
/// the bird's look down (degrees)
pub const BIRD_PITCH: f32 = 50.0;
/// the default primary camera's eye height: adult
pub const DEFAULT_HEIGHT: f32 = 1.6;
/// a typed eye height's range (m)
pub const HEIGHT_RANGE: (f32, f32) = (0.02, 100.0);

/// which preset an eye height is, if one
pub fn height_preset(h: f32) -> Option<usize> {
    EYE_HEIGHTS.iter().position(|(_, p)| (p - h).abs() < 1e-3)
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Projection {
    #[default]
    Perspective,
    /// parallel rays: isometric views; the lens's `size` is the view's height
    Orthographic,
}

/// What steers the camera.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// its shots and moves
    #[default]
    Scene,
    /// a phone in a hand – no behaviour yet. TODO(#605)
    Handheld,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lens {
    #[serde(default)]
    pub projection: Projection,
    /// mm on a full-frame sensor (perspective)
    #[serde(default = "default_focal")]
    pub focal: f32,
    /// the view's height in metres (orthographic)
    #[serde(default = "default_size")]
    pub size: f32,
}

fn default_focal() -> f32 {
    30.0
}

fn default_size() -> f32 {
    4.0
}

impl Default for Lens {
    fn default() -> Lens {
        Lens { projection: Projection::Perspective, focal: default_focal(), size: default_size() }
    }
}

impl Lens {
    pub fn focal(mm: f32) -> Lens {
        Lens { focal: mm, ..Lens::default() }
    }

    pub fn orthographic(size: f32) -> Lens {
        Lens { projection: Projection::Orthographic, size, ..Lens::default() }
    }

    pub fn ortho(&self) -> bool {
        self.projection == Projection::Orthographic
    }

    /// the vertical field of view of the focal length (rad)
    pub fn fov_y(&self) -> f32 {
        fov_of(self.focal)
    }

    /// which preset the focal length is, if one
    pub fn preset(&self) -> Option<usize> {
        FOCAL_PRESETS.iter().position(|p| (p - self.focal).abs() < 1e-3)
    }

    pub fn problems(&self, who: &str, out: &mut Vec<String>) {
        if !(FOCAL_RANGE.0..=FOCAL_RANGE.1).contains(&self.focal) {
            out.push(format!("{who}: the focal length is {}–{} mm", FOCAL_RANGE.0, FOCAL_RANGE.1));
        }
        if !(SIZE_RANGE.0..=SIZE_RANGE.1).contains(&self.size) {
            out.push(format!("{who}: an orthographic view is {}–{} m high", SIZE_RANGE.0, SIZE_RANGE.1));
        }
    }
}

/// the vertical field of view of a focal length in mm (rad)
pub fn fov_of(focal: f32) -> f32 {
    2.0 * (SENSOR_H_MM / (2.0 * focal.max(1e-3))).atan()
}

/// the focal length in mm of a vertical field of view (rad)
pub fn focal_of(fov_y: f32) -> f32 {
    SENSOR_H_MM / (2.0 * (fov_y * 0.5).tan().max(1e-6))
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShotDef {
    /// a word: `scene:<id> > shot:<id>` picks it
    pub id: String,
    #[serde(default, skip_serializing_if = "Label::is_empty")]
    pub label: Label,
    /// where it aims (m, the set's space)
    pub target: [f32; 3],
    /// degrees around the aim: 0 looks along −z from +z
    pub yaw: f32,
    /// degrees above the aim
    pub pitch: f32,
    /// metres from the aim
    pub distance: f32,
    #[serde(default)]
    pub lens: Lens,
}

impl ShotDef {
    /// the eye (m)
    pub fn eye(&self) -> [f32; 3] {
        let (sy, cy) = self.yaw.to_radians().sin_cos();
        let (sp, cp) = self.pitch.to_radians().sin_cos();
        let [x, y, z] = self.target;
        [x + self.distance * cp * sy, y + self.distance * sp, z + self.distance * cp * cy]
    }

    /// The shot with its eye `height` m above a floor at `floor`, its side kept: level at its aim (the aim rises or sinks
    /// with the eye, the distance stays) – the bird keeps its aim and looks down at it from that height.
    pub fn at_height(&self, height: f32, floor: f32) -> ShotDef {
        let eye_y = floor + height;
        if height_preset(height) == Some(EYE_HEIGHTS.len() - 1) {
            let above = (eye_y - self.target[1]).max(DISTANCE_RANGE.0);
            return ShotDef { pitch: BIRD_PITCH, distance: (above / BIRD_PITCH.to_radians().sin()).min(DISTANCE_RANGE.1), ..self.clone() };
        }
        ShotDef { target: [self.target[0], eye_y, self.target[2]], pitch: 0.0, ..self.clone() }
    }

    /// its eye's height above a floor at `floor`
    pub fn height(&self, floor: f32) -> f32 {
        self.eye()[1] - floor
    }
}

/// A point of the trolley track: where it is, and its Bézier handle (the way out of it; the way in is its mirror).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackPoint {
    pub at: [f32; 3],
    /// metres from `at`; none = a smooth curve through the neighbours
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<[f32; 3]>,
}

/// The trolley track: its first point is the start, its last the end.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Track {
    #[serde(rename = "point")]
    pub points: Vec<TrackPoint>,
}

impl Track {
    /// a straight track from `start` to `end`
    pub fn between(start: [f32; 3], end: [f32; 3]) -> Track {
        Track { points: vec![TrackPoint { at: start, handle: None }, TrackPoint { at: end, handle: None }] }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveKind {
    /// at once
    Cut,
    /// an eased glide around the aim
    Glide,
    /// travels in from a side onto the subject (`side`)
    In,
    /// along the trolley track
    Fly,
    /// from high above down onto the shot (`height`)
    CraneDown,
    /// from below up onto the shot (`height`)
    CraneUp,
    /// zooms in while circling the aim (`rounds`, `turn`)
    Spiral,
    /// dollies straight in
    Push,
    /// dollies straight out
    Pull,
    /// circles the aim at a steady speed (`rounds`, `turn`)
    Orbit,
    /// slides sideways, turning as it goes
    Truck,
    /// the head turns where the camera stands
    Tilt,
    /// between focal lengths
    Zoom,
    /// distance and focal length against each other: the subject keeps its size, the room stretches
    DollyZoom,
    /// a fast swing of the head
    Whip,
    /// keeps the shot on a moving subject (`follow`) and stays on it
    Follow,
    /// a handheld unrest on the way (`amount`)
    Shake,
    // TODO(#606): rack focus, once the stage has a depth of field – a lens's focus distance beside `focal`
}

impl MoveKind {
    pub const ALL: [MoveKind; 17] = [
        MoveKind::Cut,
        MoveKind::Glide,
        MoveKind::In,
        MoveKind::Fly,
        MoveKind::CraneDown,
        MoveKind::CraneUp,
        MoveKind::Spiral,
        MoveKind::Push,
        MoveKind::Pull,
        MoveKind::Orbit,
        MoveKind::Truck,
        MoveKind::Tilt,
        MoveKind::Zoom,
        MoveKind::DollyZoom,
        MoveKind::Whip,
        MoveKind::Follow,
        MoveKind::Shake,
    ];

    pub fn word(self) -> &'static str {
        match self {
            MoveKind::Cut => "cut",
            MoveKind::Glide => "glide",
            MoveKind::In => "in",
            MoveKind::Fly => "fly",
            MoveKind::CraneDown => "crane down",
            MoveKind::CraneUp => "crane up",
            MoveKind::Spiral => "spiral",
            MoveKind::Push => "push in",
            MoveKind::Pull => "pull out",
            MoveKind::Orbit => "orbit",
            MoveKind::Truck => "truck",
            MoveKind::Tilt => "tilt",
            MoveKind::Zoom => "zoom",
            MoveKind::DollyZoom => "dolly zoom",
            MoveKind::Whip => "whip pan",
            MoveKind::Follow => "follow",
            MoveKind::Shake => "shake",
        }
    }

    /// the length a new move of this kind takes
    pub fn length(self) -> Length {
        match self {
            MoveKind::Cut => Length::Seconds(0.0),
            MoveKind::Whip => Length::Beats(1.0),
            MoveKind::Orbit | MoveKind::Spiral | MoveKind::Fly => Length::Bars(4.0),
            _ => Length::Bars(2.0),
        }
    }

    pub fn ease(self) -> Ease {
        match self {
            MoveKind::Whip => Ease::Whip,
            MoveKind::Orbit => Ease::Linear,
            _ => Ease::InOut,
        }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Length {
    /// on the tempo clock
    Beats(f32),
    Bars(f32),
    Seconds(f32),
}

impl Length {
    pub fn value(self) -> f32 {
        match self {
            Length::Beats(v) | Length::Bars(v) | Length::Seconds(v) => v,
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Length::Beats(_) => "beats",
            Length::Bars(_) => "bars",
            Length::Seconds(_) => "s",
        }
    }

    pub fn with(self, v: f32) -> Length {
        match self {
            Length::Beats(_) => Length::Beats(v),
            Length::Bars(_) => Length::Bars(v),
            Length::Seconds(_) => Length::Seconds(v),
        }
    }

    /// in beats on a clock of `per_bar` beats a bar and `bpm`
    pub fn beats(self, per_bar: u32, bpm: f64) -> f64 {
        match self {
            Length::Beats(b) => f64::from(b),
            Length::Bars(b) => f64::from(b) * f64::from(per_bar.max(1)),
            Length::Seconds(s) => f64::from(s) * bpm / 60.0,
        }
    }

    /// in seconds at `bpm`
    pub fn seconds(self, per_bar: u32, bpm: f64) -> f64 {
        match self {
            Length::Seconds(s) => f64::from(s),
            l => l.beats(per_bar, bpm) * 60.0 / bpm.max(1.0),
        }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ease {
    Linear,
    In,
    Out,
    #[default]
    InOut,
    /// slow at both ends, a snap in the middle
    Whip,
}

impl Ease {
    pub const ALL: [Ease; 5] = [Ease::Linear, Ease::In, Ease::Out, Ease::InOut, Ease::Whip];

    pub fn word(self) -> &'static str {
        match self {
            Ease::Linear => "linear",
            Ease::In => "in",
            Ease::Out => "out",
            Ease::InOut => "in-out",
            Ease::Whip => "whip",
        }
    }

    /// 0..1 → 0..1, 0 and 1 kept exactly
    pub fn at(self, u: f32) -> f32 {
        let u = u.clamp(0.0, 1.0);
        match self {
            Ease::Linear => u,
            Ease::In => u * u * u,
            Ease::Out => 1.0 - (1.0 - u).powi(3),
            Ease::InOut => u * u * u * (u * (u * 6.0 - 15.0) + 10.0),
            Ease::Whip => {
                let k = 9.0;
                let s = |x: f32| 1.0 / (1.0 + (-k * (x - 0.5)).exp());
                ((s(u) - s(0.0)) / (s(1.0) - s(0.0))).clamp(0.0, 1.0)
            }
        }
    }
}

/// When a move starts.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Start {
    #[default]
    Now,
    /// on the next bar line of the tempo clock
    Bar,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Left,
    Right,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Turn {
    /// counter-clockwise seen from above
    #[default]
    Left,
    Right,
}

/// A move as it plays: its kind and settings, its length and ease, when it starts (the shot it ends on stands beside it).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveSpec {
    pub kind: MoveKind,
    pub length: Length,
    #[serde(default)]
    pub ease: Ease,
    #[serde(default)]
    pub start: Start,
    /// orbit, spiral
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub rounds: f32,
    #[serde(default, skip_serializing_if = "is_default")]
    pub turn: Turn,
    /// in
    #[serde(default, skip_serializing_if = "is_default")]
    pub side: Side,
    /// crane: metres above (or below) the higher (lower) end
    #[serde(default = "crane_height", skip_serializing_if = "is_crane_height")]
    pub height: f32,
    /// shake: degrees of unrest
    #[serde(default = "shake_amount", skip_serializing_if = "is_shake_amount")]
    pub amount: f32,
    /// follow: the holder it keeps in the shot
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow: Option<String>,
}

fn one() -> f32 {
    1.0
}
fn is_one(v: &f32) -> bool {
    *v == 1.0
}
fn crane_height() -> f32 {
    1.5
}
fn is_crane_height(v: &f32) -> bool {
    *v == crane_height()
}
fn shake_amount() -> f32 {
    1.5
}
fn is_shake_amount(v: &f32) -> bool {
    *v == shake_amount()
}
fn default_height() -> f32 {
    DEFAULT_HEIGHT
}
fn is_default_height(v: &f32) -> bool {
    *v == DEFAULT_HEIGHT
}
fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

impl MoveSpec {
    /// a move of `kind` with its usual length and ease
    pub fn of(kind: MoveKind) -> MoveSpec {
        MoveSpec { kind, length: kind.length(), ease: kind.ease(), start: Start::Now, rounds: 1.0, turn: Turn::Left, side: Side::Left, height: crane_height(), amount: shake_amount(), follow: None }
    }

    pub fn problems(&self, who: &str, out: &mut Vec<String>) {
        let v = self.length.value();
        if !v.is_finite() || v < 0.0 || (v == 0.0 && self.kind != MoveKind::Cut) {
            out.push(format!("{who}: its length is more than 0"));
        }
        if !(0.0..=MAX_ROUNDS).contains(&self.rounds) {
            out.push(format!("{who}: rounds are 0–{MAX_ROUNDS}"));
        }
        if !self.height.is_finite() || self.height < 0.0 {
            out.push(format!("{who}: the crane's height is 0 or more"));
        }
        if !(0.0..=45.0).contains(&self.amount) {
            out.push(format!("{who}: a shake is 0–45°"));
        }
        if self.follow.is_some() != (self.kind == MoveKind::Follow) {
            out.push(format!("{who}: `follow` names the holder of a follow, and only of a follow"));
        }
    }
}

/// A named move of the scene: `scene:<id> > move:<id>` plays it.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveDef {
    pub id: String,
    /// the shot it ends on
    pub to: String,
    #[serde(flatten)]
    pub spec: MoveSpec,
    /// the move that follows when it ends (a chain)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub then: Option<String>,
}

/// The scene's primary camera.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneCamera {
    #[serde(default, skip_serializing_if = "is_default")]
    pub source: Source,
    /// the lens before a shot is taken
    #[serde(default)]
    pub lens: Lens,
    /// the eye height (m above the floor) of the primary camera while the scene has no shots: head-on at the stage's
    /// centre, level
    #[serde(default = "default_height", skip_serializing_if = "is_default_height")]
    pub height: f32,
    /// its shots; the first opens the scene
    #[serde(default, rename = "shot", skip_serializing_if = "Vec::is_empty")]
    pub shots: Vec<ShotDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<Track>,
    #[serde(default, rename = "move", skip_serializing_if = "Vec::is_empty")]
    pub moves: Vec<MoveDef>,
}

impl Default for SceneCamera {
    fn default() -> SceneCamera {
        SceneCamera { source: Source::Scene, lens: Lens::default(), height: DEFAULT_HEIGHT, shots: Vec::new(), track: None, moves: Vec::new() }
    }
}

impl SceneCamera {
    pub fn shot(&self, id: &str) -> Option<&ShotDef> {
        self.shots.iter().find(|s| s.id == id)
    }

    pub fn find_move(&self, id: &str) -> Option<&MoveDef> {
        self.moves.iter().find(|m| m.id == id)
    }

    /// the shot the scene opens with
    pub fn opening(&self) -> Option<&ShotDef> {
        self.shots.first()
    }

    /// a free shot id: `shot1`, `shot2` …
    pub fn free_id(&self, stem: &str) -> String {
        (1..).map(|i| format!("{stem}{i}")).find(|id| self.shot(id).is_none() && self.find_move(id).is_none()).unwrap_or_default()
    }

    /// The move `id` and the ones it chains to, in order; a chain that comes back on itself ends before the repeat.
    pub fn chain(&self, id: &str) -> Vec<&MoveDef> {
        let mut out: Vec<&MoveDef> = Vec::new();
        let mut next = Some(id);
        while let Some(m) = next.and_then(|id| self.find_move(id)) {
            if out.iter().any(|o| o.id == m.id) {
                break;
            }
            out.push(m);
            next = m.then.as_deref();
        }
        out
    }

    /// the core's checks; `holders` are the scene's holder ids (what a follow may name)
    pub fn problems(&self, holders: &[&str], out: &mut Vec<String>) {
        self.lens.problems("camera lens", out);
        if !(HEIGHT_RANGE.0..=HEIGHT_RANGE.1).contains(&self.height) {
            out.push(format!("camera: the eye height is {}–{} m", HEIGHT_RANGE.0, HEIGHT_RANGE.1));
        }
        let mut ids = BTreeSet::new();
        for s in &self.shots {
            let who = format!("shot {}", s.id);
            if let Err(e) = word(&s.id, "a shot's id") {
                out.push(format!("camera: {e}"));
            }
            if !ids.insert(s.id.as_str()) {
                out.push(format!("{who}: the id is taken twice"));
            }
            if s.target.iter().chain([&s.yaw, &s.pitch]).any(|v| !v.is_finite()) {
                out.push(format!("{who}: its aim and angles are numbers"));
            }
            if !(-90.0..=90.0).contains(&s.pitch) {
                out.push(format!("{who}: the pitch is −90–90°"));
            }
            if !(DISTANCE_RANGE.0..=DISTANCE_RANGE.1).contains(&s.distance) {
                out.push(format!("{who}: the distance is {}–{} m", DISTANCE_RANGE.0, DISTANCE_RANGE.1));
            }
            s.lens.problems(&who, out);
        }
        if let Some(t) = &self.track {
            if t.points.len() < 2 {
                out.push("camera track: a start and an end point at least".into());
            }
            if t.points.iter().any(|p| p.at.iter().chain(p.handle.iter().flatten()).any(|v| !v.is_finite())) {
                out.push("camera track: its points and handles are numbers".into());
            }
        }
        for m in &self.moves {
            let who = format!("move {}", m.id);
            if let Err(e) = word(&m.id, "a move's id") {
                out.push(format!("camera: {e}"));
            }
            if !ids.insert(m.id.as_str()) {
                out.push(format!("{who}: the id is taken twice (shots and moves share their ids)"));
            }
            if self.shot(&m.to).is_none() {
                out.push(format!("{who}: it ends on '{}', which is no shot of the camera", m.to));
            }
            if m.spec.kind == MoveKind::Fly && self.track.is_none() {
                out.push(format!("{who}: a fly-through runs along the camera's track – the scene has none"));
            }
            if let Some(h) = m.spec.follow.as_deref().filter(|h| !holders.contains(h)) {
                out.push(format!("{who}: it follows '{h}', which is no holder of the scene"));
            }
            if let Some(then) = m.then.as_deref().filter(|t| self.find_move(t).is_none()) {
                out.push(format!("{who}: it chains to '{then}', which is no move of the camera"));
            }
            m.spec.problems(&who, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(id: &str) -> ShotDef {
        ShotDef { id: id.into(), label: Label::new(), target: [0.0, 1.0, 0.0], yaw: 20.0, pitch: 8.0, distance: 3.0, lens: Lens::default() }
    }

    #[test]
    fn the_focal_presets_are_full_frame_fields_of_view() {
        assert!((fov_of(50.0).to_degrees() - 26.99).abs() < 0.05, "a 50 mm on full frame: 27° high");
        for f in FOCAL_PRESETS {
            assert!((focal_of(fov_of(f)) - f).abs() < 1e-2);
        }
        assert!(fov_of(17.0) > fov_of(150.0));
        assert_eq!(Lens::focal(70.0).preset(), Some(3));
        assert_eq!(Lens::focal(85.0).preset(), None, "a typed value is no preset");
    }

    #[test]
    fn eye_heights_look_level_and_the_bird_looks_down() {
        let s = shot("wide");
        for (name, h) in EYE_HEIGHTS {
            let at = s.at_height(h, 0.5);
            assert!((at.height(0.5) - h).abs() < 1e-3, "{name}: {}", at.height(0.5));
            assert_eq!(height_preset(h).map(|i| EYE_HEIGHTS[i].0), Some(name));
            if name == "bird" {
                assert_eq!((at.pitch, at.target), (BIRD_PITCH, s.target), "the bird keeps its aim and looks down at it");
            } else {
                assert_eq!((at.pitch, at.yaw, at.distance), (0.0, s.yaw, s.distance), "{name}: level, from the same side");
            }
        }
        assert_eq!(height_preset(2.2), None, "a typed height is no preset");
        assert_eq!(SceneCamera::default().height, DEFAULT_HEIGHT);
        assert_eq!(serde_json::to_string(&SceneCamera::default()).unwrap(), "{\"lens\":{\"projection\":\"perspective\",\"focal\":30.0,\"size\":4.0}}");
    }

    #[test]
    fn eases_keep_their_ends_and_rise() {
        for e in Ease::ALL {
            assert_eq!(e.at(0.0), 0.0, "{e:?}");
            assert!((e.at(1.0) - 1.0).abs() < 1e-6, "{e:?}");
            let mut last = 0.0;
            for i in 1..=100 {
                let v = e.at(i as f32 / 100.0);
                assert!(v >= last - 1e-6, "{e:?} rises");
                last = v;
            }
        }
    }

    #[test]
    fn lengths_in_beats_and_seconds() {
        assert_eq!(Length::Bars(2.0).beats(4, 120.0), 8.0);
        assert_eq!(Length::Bars(2.0).seconds(4, 120.0), 4.0);
        assert_eq!(Length::Seconds(3.0).beats(4, 120.0), 6.0);
    }

    #[test]
    fn a_camera_is_checked_and_round_trips() {
        let mut c = SceneCamera { shots: vec![shot("wide"), shot("close")], ..SceneCamera::default() };
        c.moves.push(MoveDef { id: "intro".into(), to: "close".into(), spec: MoveSpec::of(MoveKind::Spiral), then: Some("back".into()) });
        c.moves.push(MoveDef { id: "back".into(), to: "wide".into(), spec: MoveSpec::of(MoveKind::Glide), then: Some("intro".into()) });
        let mut out = Vec::new();
        c.problems(&[], &mut out);
        assert!(out.is_empty(), "{out:?}");
        assert_eq!(c.chain("intro").iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["intro", "back"], "a loop ends before the repeat");
        let text = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<SceneCamera>(&text).unwrap(), c);
        assert!(text.contains("\"length\":{\"bars\":4.0}"), "{text}");

        c.shots.push(shot("wide"));
        c.moves.push(MoveDef { id: "fly".into(), to: "nowhere".into(), spec: MoveSpec::of(MoveKind::Fly), then: None });
        let mut f = MoveSpec::of(MoveKind::Follow);
        f.follow = Some("ghost".into());
        c.moves.push(MoveDef { id: "f".into(), to: "wide".into(), spec: f, then: None });
        c.shots[0].lens.focal = 2.0;
        let mut out = Vec::new();
        c.problems(&["pope"], &mut out);
        let all = out.join(" | ");
        for want in ["taken twice", "no shot of the camera", "the scene has none", "no holder of the scene", "focal length"] {
            assert!(all.contains(want), "{want}: {all}");
        }
    }
}
