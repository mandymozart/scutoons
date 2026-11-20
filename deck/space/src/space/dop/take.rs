//! A take (LIVE-CAMERAS.md §7): a phone's pose stream recorded as a camera move. The `Recorder` follows the phone the way
//! a deck does (live_cameras.rs `Follow`: anchored on the held camera's view, Recenter re-anchors, the short ease against
//! the sensor's jitter is the smoothing) and keeps every view; `take` thins them to keys – a key stays where leaving it out
//! would move the eye or the look more than a hair from the straight way between its neighbours in time – and times them
//! in beats (the tempo clock running: the take stays on the music) or seconds. Played back, `at` walks the keys on their
//! times (a smooth curve through the eyes, the look turned the shorter way).
use super::{back, wrap_pi, View};
use crate::live_cameras::Follow;
use crate::scene_camera::{Length, Take, TakeKey, TAKE_MAX_KEYS};
use glam::{Quat, Vec3};

/// how far the eye may stray from the way between two keys (m) before a key stays
const EYE_TOL: f32 = 0.01;
/// how far the look may turn from it (rad, ~0.5°)
const LOOK_TOL: f32 = 0.0087;
/// the longest take kept (s); poses after it are dropped
pub const MAX_SECS: f32 = 600.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Sample {
    secs: f32,
    eye: Vec3,
    rot: Quat,
}

/// A take being recorded from the phone holding `camera`.
#[derive(Debug, Clone)]
pub struct Recorder {
    camera: String,
    /// the held camera's view when the take began: the first pose is it, the lens and the aim's distance are its
    anchor: View,
    follow: Follow,
    /// the first pose's and the last pose's time (the phone's clock, ms)
    t0: Option<f64>,
    last: Option<f64>,
    samples: Vec<Sample>,
    /// poses are kept as samples (a live follow that is not taking yet only moves the view)
    keeping: bool,
    /// phone metres → scene metres
    scale: f32,
}

/// a kept pose as a page draws it: seconds from the take's start, the eye, the camera's rotation (x y z w)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub secs: f32,
    pub eye: Vec3,
    pub rot: Quat,
}

/// A page's camera (position, rotation x y z w; looks down -Z) as the view a take starts on. None: malformed.
pub fn anchor_of(position: &[f64], quaternion: &[f64]) -> Option<View> {
    if position.len() != 3 || quaternion.len() != 4 || !position.iter().chain(quaternion).all(|v| v.is_finite()) {
        return None;
    }
    let eye = Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32);
    let rot = Quat::from_xyzw(quaternion[0] as f32, quaternion[1] as f32, quaternion[2] as f32, quaternion[3] as f32).normalize();
    Some(View::from_eye(eye, eye + rot * Vec3::NEG_Z, crate::scene_camera::Lens::focal(50.0)))
}

/// a pose for a page: position (3), rotation (4), the eye rounded to 0.1 mm and the rotation to 1e-5
pub fn flat(eye: Vec3, rot: Quat) -> [f64; 7] {
    let r = |v: f32, k: f64| (v as f64 * k).round() / k;
    [r(eye.x, 1e4), r(eye.y, 1e4), r(eye.z, 1e4), r(rot.x, 1e5), r(rot.y, 1e5), r(rot.z, 1e5), r(rot.w, 1e5)]
}

/// the eye's look as a shot's yaw and pitch (rad): the eye stands that way from its aim
fn angles(rot: Quat) -> (f32, f32) {
    let b = (rot * Vec3::Z).normalize_or_zero();
    (b.x.atan2(b.z), b.y.clamp(-1.0, 1.0).asin())
}

impl Recorder {
    /// a take of the phone holding `camera`, starting on `anchor` (that camera's view now)
    pub fn new(camera: &str, anchor: View) -> Recorder {
        Recorder { camera: camera.to_string(), anchor, follow: Follow::default(), t0: None, last: None, samples: Vec::new(), keeping: true, scale: 1.0 }
    }

    /// A phone's follow that takes nothing until `keep(true)`: the view it holds moves with the phone (a page's live view), and
    /// `scale` scene metres go for a phone metre.
    pub fn live(camera: &str, anchor: View, scale: f32) -> Recorder {
        Recorder { keeping: false, scale: if scale.is_finite() && scale > 0.0 { scale } else { 1.0 }, ..Recorder::new(camera, anchor) }
    }

    /// Start (true: a new take from the next pose, what was kept goes) or end (false) the keeping of poses.
    pub fn keep(&mut self, on: bool) {
        if on && !self.keeping {
            (self.samples, self.t0) = (Vec::new(), None);
        }
        self.keeping = on;
    }

    pub fn keeping(&self) -> bool {
        self.keeping
    }

    /// the view the follow holds now (eye, rotation); none before the first pose
    pub fn held(&self) -> Option<(Vec3, Quat)> {
        self.follow.view()
    }

    /// the live camera it records (`scene:<id>` or `scene:<id> > shot:<id>`)
    pub fn camera(&self) -> &str {
        &self.camera
    }

    /// A pose of the phone at `t` (its clock, ms): position (3), rotation (x y z w). False: late, malformed or past the
    /// longest take – dropped.
    pub fn pose(&mut self, t: f64, p: &[f64], q: &[f64]) -> bool {
        if self.keeping && self.secs_at(t) > MAX_SECS {
            return false;
        }
        let eye = self.anchor.eye();
        let anchor = (eye, crate::live_cameras::look_rotation(eye, self.anchor.target));
        let scaled: Vec<f64> = p.iter().map(|v| v * self.scale as f64).collect();
        if !self.follow.pose(t, &scaled, q, anchor) {
            return false;
        }
        let dt = self.last.map_or(0.0, |l| ((t - l) / 1000.0) as f32);
        self.last = Some(t);
        let Some((eye, rot)) = self.follow.step(dt) else { return false };
        if self.keeping {
            self.t0.get_or_insert(t);
            self.samples.push(Sample { secs: self.secs_at(t), eye, rot });
        }
        true
    }

    fn secs_at(&self, t: f64) -> f32 {
        self.t0.map_or(0.0, |t0| ((t - t0) / 1000.0) as f32)
    }

    /// the phone's Recenter: its next pose is the view now
    pub fn recenter(&mut self) {
        self.follow.recenter();
    }

    /// how long it runs so far (s) and how many poses it kept
    pub fn secs(&self) -> f32 {
        self.samples.last().map_or(0.0, |s| s.secs)
    }

    pub fn poses(&self) -> usize {
        self.samples.len()
    }

    /// the view the take is on now (the anchor's lens, its aim as far ahead as the anchor's); none before the first pose
    pub fn view(&self) -> Option<View> {
        let s = self.samples.last()?;
        let (yaw, pitch) = angles(s.rot);
        let d = self.anchor.distance;
        Some(View { target: s.eye - back(yaw, pitch) * d, yaw, pitch, distance: d, lens: self.anchor.lens })
    }

    /// the indices of the samples that stay (see `thin`), at most TAKE_MAX_KEYS; none: too short to be a move
    fn kept(&self) -> Option<Vec<usize>> {
        let s = &self.samples;
        if s.len() < 2 || self.secs() <= 0.0 {
            return None;
        }
        let mut tol = 1.0;
        loop {
            let kept = thin(s, tol);
            if kept.len() <= TAKE_MAX_KEYS {
                return Some(kept);
            }
            tol *= 2.0;
        }
    }

    /// The take's keys as a page that draws its own camera takes them: the thinned poses, their time in seconds from the
    /// first one, the eye and the rotation as kept (the phone's roll too). None: too short to be a move.
    pub fn keys(&self) -> Option<Vec<Key>> {
        let kept = self.kept()?;
        Some(kept.into_iter().map(|i| Key { secs: self.samples[i].secs, eye: self.samples[i].eye, rot: self.samples[i].rot }).collect())
    }

    /// The take: its keys timed in beats at `bpm` (the tempo clock ran), else in seconds, and the view it ends on (the
    /// anchor's lens, its aim as far ahead as the anchor's). None: too short to be a move.
    pub fn take(&self, bpm: Option<f64>) -> Option<(Take, View)> {
        let s = &self.samples;
        let kept = self.kept()?;
        let r = |v: f32, k: f32| (v * k).round() / k;
        let time = |secs: f32| match bpm {
            Some(bpm) => Length::Beats(r(secs * (bpm / 60.0) as f32, 1000.0)),
            None => Length::Seconds(r(secs, 1000.0)),
        };
        let mut keys: Vec<TakeKey> = Vec::with_capacity(kept.len());
        for i in kept {
            let (yaw, pitch) = angles(s[i].rot);
            let key = TakeKey { at: s[i].eye.to_array().map(|v| r(v, 1000.0)), yaw: r(wrap_pi(yaw).to_degrees(), 100.0), pitch: r(pitch.to_degrees(), 100.0), time: time(s[i].secs) };
            // two keys rounded onto one moment: the later stays
            match keys.last_mut() {
                Some(k) if k.time.value() >= key.time.value() => *k = TakeKey { time: k.time, ..key },
                _ => keys.push(key),
            }
        }
        if keys.len() < 2 {
            return None;
        }
        let take = Take { keys };
        let end = key_view(take.keys.last()?, self.anchor.distance, self.anchor);
        Some((take, end))
    }
}

/// a key as a view: the eye, the aim `distance` ahead, `like`'s lens
fn key_view(k: &TakeKey, distance: f32, like: View) -> View {
    let (yaw, pitch) = (k.yaw.to_radians(), k.pitch.to_radians());
    let eye = Vec3::from_array(k.at);
    View { target: eye - back(yaw, pitch) * distance, yaw, pitch, distance, lens: like.lens }
}

/// How far sample `m` is off the straight way between `a` and `b` in time, in tolerances (above 1: it must stay).
fn off(s: &[Sample], a: usize, b: usize, m: usize, tol: f32) -> f32 {
    let span = s[b].secs - s[a].secs;
    let u = if span > 0.0 { (s[m].secs - s[a].secs) / span } else { 0.5 };
    let eye = s[a].eye.lerp(s[b].eye, u);
    let rot = s[a].rot.slerp(s[b].rot, u);
    let eye_off = s[m].eye.distance(eye) / (EYE_TOL * tol);
    let look_off = s[m].rot.angle_between(rot) / (LOOK_TOL * tol);
    eye_off.max(look_off)
}

/// The samples to keep (their indices, in order): the ends, and every one that leaving out would cost more than the
/// tolerance (Ramer–Douglas–Peucker over time).
fn thin(s: &[Sample], tol: f32) -> Vec<usize> {
    let mut keep = vec![false; s.len()];
    keep[0] = true;
    keep[s.len() - 1] = true;
    let mut todo = vec![(0, s.len() - 1)];
    while let Some((a, b)) = todo.pop() {
        if b <= a + 1 {
            continue;
        }
        let (m, worst) = (a + 1..b).map(|m| (m, off(s, a, b, m, tol))).fold((a, 0.0), |w, x| if x.1 > w.1 { x } else { w });
        if worst > 1.0 {
            keep[m] = true;
            todo.push((a, m));
            todo.push((m, b));
        }
    }
    (0..s.len()).filter(|i| keep[*i]).collect()
}

/// Where a take is at `lin` (0..1 of its time): the eye on a smooth curve through the keys' eyes (Catmull-Rom), the look's
/// yaw and pitch (rad) the shorter way between its keys. None: fewer than two keys, or no time passes.
pub fn at(take: &Take, lin: f32) -> Option<(Vec3, f32, f32)> {
    let k = &take.keys;
    let total = k.last()?.time.value();
    if k.len() < 2 || total <= 0.0 {
        return None;
    }
    let goal = lin.clamp(0.0, 1.0) * total;
    let j = k.partition_point(|x| x.time.value() < goal).clamp(1, k.len() - 1);
    let (a, b) = (&k[j - 1], &k[j]);
    let span = b.time.value() - a.time.value();
    let u = if span > 0.0 { ((goal - a.time.value()) / span).clamp(0.0, 1.0) } else { 1.0 };
    let p = |i: usize| Vec3::from_array(k[i.min(k.len() - 1)].at);
    let (p0, p1, p2, p3) = (p(j.saturating_sub(2)), p(j - 1), p(j), p(j + 1));
    let (u2, u3) = (u * u, u * u * u);
    let eye = 0.5 * ((2.0 * p1) + (p2 - p0) * u + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u2 + (3.0 * p1 - p0 - 3.0 * p2 + p3) * u3);
    let yaw = a.yaw.to_radians() + wrap_pi((b.yaw - a.yaw).to_radians()) * u;
    let pitch = a.pitch.to_radians() + (b.pitch - a.pitch).to_radians() * u;
    Some((eye, yaw, pitch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_camera::Lens;

    fn anchor() -> View {
        View::from_eye(Vec3::new(0.0, 1.6, 4.0), Vec3::new(0.0, 1.6, 0.0), Lens::focal(50.0))
    }

    /// a phone walking 2 m to its right over 4 s at 60 poses a second, turning a quarter to the left half way, its hand shaking a little
    fn walk(r: &mut Recorder) {
        for i in 0..=240 {
            let s = i as f64 / 60.0;
            let x = (s / 4.0 * 2.0).min(2.0);
            let jitter = 0.002 * ((i * 7919) % 13) as f64 / 13.0;
            let turn = if s < 2.0 { 0.0 } else { ((s - 2.0) / 2.0).min(1.0) * std::f64::consts::FRAC_PI_4 };
            let (sh, ch) = (turn / 2.0).sin_cos();
            assert!(r.pose(1000.0 + s * 1000.0, &[x, jitter, 0.0], &[0.0, sh, 0.0, ch]));
        }
    }

    #[test]
    fn a_take_starts_on_the_held_camera_and_keeps_few_keys() {
        let mut r = Recorder::new("scene:duet > shot:wide", anchor());
        walk(&mut r);
        assert_eq!(r.poses(), 241);
        assert!((r.secs() - 4.0).abs() < 1e-3);
        let (take, end) = r.take(None).expect("a take");
        let k = &take.keys;
        assert!(k.len() >= 3 && k.len() < 60, "thinned and smoothed: {} keys", k.len());
        assert_eq!(k[0].time, Length::Seconds(0.0));
        assert_eq!(k.last().unwrap().time, Length::Seconds(4.0));
        assert!(Vec3::from_array(k[0].at).distance(anchor().eye()) < 0.01, "the first key is the held camera's eye");
        assert!((k[0].yaw - 0.0).abs() < 0.5 && (k[0].pitch).abs() < 0.5, "looking along -Z: {:?}", k[0]);
        let last = k.last().unwrap();
        assert!((last.at[0] - 2.0).abs() < 0.05, "walked 2 m right: {:?}", last.at);
        assert!((last.yaw - 45.0).abs() < 2.0, "turned a quarter left: {}", last.yaw);
        let mut problems = Vec::new();
        let spec = crate::scene_camera::MoveSpec { take: Some(take.clone()), length: take.length().unwrap(), ..crate::scene_camera::MoveSpec::of(crate::scene_camera::MoveKind::Take) };
        spec.problems("move take1", &mut problems);
        assert!(problems.is_empty(), "{problems:?}");
        assert!(end.eye().distance(Vec3::from_array(last.at)) < 1e-3 && (end.distance - 4.0).abs() < 1e-4, "ends on its last key, aimed as far ahead as the anchor");
        // every pose is near the keys' way
        for t in [0.5, 1.5, 2.5, 3.5] {
            let (eye, _, _) = at(&take, t / 4.0).unwrap();
            assert!((eye.x - t / 2.0).abs() < 0.08, "{t}: {eye:?}");
        }
    }

    #[test]
    fn a_live_follow_keeps_nothing_until_it_is_asked_and_scales_the_phone() {
        let anchor = anchor_of(&[0.0, 1.6, 4.0], &[0.0, 0.0, 0.0, 1.0]).expect("an anchor");
        let mut r = Recorder::live("shot", anchor, 2.0);
        assert!(r.pose(0.0, &[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0, 1.0]));
        assert!(r.pose(1000.0, &[1.0, 0.0, 0.0], &[0.0, 0.0, 0.0, 1.0]));
        assert_eq!((r.poses(), r.keys()), (0, None), "nothing kept while it only follows");
        r.keep(true);
        for i in 0..=60 {
            assert!(r.pose(2000.0 + i as f64 * 50.0, &[1.0 + i as f64 / 60.0, 0.0, 0.0], &[0.0, 0.0, 0.0, 1.0]));
        }
        let keys = r.keys().expect("a take");
        assert_eq!(keys.first().unwrap().secs, 0.0);
        assert!((keys.last().unwrap().secs - 3.0).abs() < 1e-3);
        let (held, _) = r.held().unwrap();
        assert!((held.x - 4.0).abs() < 0.2, "2 phone metres at scale 2: {held:?}");
        assert!(keys.last().unwrap().eye.distance(held) < 0.3);
        assert!(anchor_of(&[0.0, f64::NAN, 0.0], &[0.0, 0.0, 0.0, 1.0]).is_none());
    }

    #[test]
    fn a_take_on_the_clock_is_timed_in_beats_and_a_recenter_reanchors() {
        let mut r = Recorder::new("scene:duet", anchor());
        walk(&mut r);
        let (take, _) = r.take(Some(120.0)).unwrap();
        assert_eq!(take.length(), Some(Length::Beats(8.0)), "4 s at 120 bpm");
        let mut r = Recorder::new("scene:duet", anchor());
        assert!(r.pose(0.0, &[5.0, 0.0, 5.0], &[0.0, 0.0, 0.0, 1.0]));
        assert!(!r.pose(0.0, &[5.0, 0.0, 5.0], &[0.0, 0.0, 0.0, 1.0]), "a late pose is dropped");
        assert!(!r.pose(10.0, &[f64::NAN, 0.0, 0.0], &[0.0, 0.0, 0.0, 1.0]), "a malformed one too");
        assert!(r.take(None).is_none(), "one pose is no move");
        assert!(r.pose(500.0, &[6.0, 0.0, 5.0], &[0.0, 0.0, 0.0, 1.0]));
        r.recenter();
        assert!(r.pose(1000.0, &[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0, 1.0]), "a jump of the phone after a recenter");
        let (take, _) = r.take(None).unwrap();
        let end = Vec3::from_array(take.keys.last().unwrap().at);
        assert!(end.distance(Vec3::new(1.0, 1.6, 4.0)) < 0.3, "goes on from where the view was, not where the phone jumped: {end:?}");
    }
}
