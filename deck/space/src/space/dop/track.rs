//! The trolley track as a path: Bézier segments between its points, walked by arc length so a move along it keeps an even
//! speed – or, when its points have arrival times, reaches each point on its time with the way's ease (`knots`, `warp`).
//! Built once when the scene's camera arrives; `at`, `tangent` and `warp` allocate nothing.
use crate::scene_camera::{Ease, Length, Track};
use glam::Vec3;

/// samples per segment for the arc length
const SAMPLES: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    segs: Vec<[Vec3; 4]>,
    /// the length walked at each sample, from 0; `SAMPLES` per segment and the end
    walked: Vec<f32>,
    /// each point's arrival time and the ease of the way there (the track's)
    times: Vec<Option<Length>>,
    eases: Vec<Option<Ease>>,
}

/// A point as a timed fly-through passes it: the share of the move's time (`lin`) and of the track's length (`s`) at it,
/// and the ease of the way there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Knot {
    pub lin: f32,
    pub s: f32,
    pub ease: Ease,
}

/// Where along the track (0..1 of its length) a timed fly-through is at `lin` (0..1 of its time): the knots' way it is on,
/// eased.
pub fn warp(knots: &[Knot], lin: f32) -> f32 {
    let lin = lin.clamp(0.0, 1.0);
    let j = knots.partition_point(|k| k.lin < lin).clamp(1, knots.len().max(2) - 1);
    let (Some(a), Some(b)) = (knots.get(j - 1), knots.get(j)) else { return lin };
    let u = if b.lin > a.lin { (lin - a.lin) / (b.lin - a.lin) } else { 1.0 };
    a.s + (b.s - a.s) * b.ease.at(u)
}

fn v(a: [f32; 3]) -> Vec3 {
    Vec3::from_array(a)
}

/// each segment's four control points: a point's handle, else a smooth curve through its neighbours (Catmull-Rom)
pub fn controls(track: &Track) -> Vec<[Vec3; 4]> {
    let p = &track.points;
    let n = p.len();
    let handle = |i: usize| -> Vec3 {
        if let Some(h) = p[i].handle {
            return v(h);
        }
        let (prev, next) = (v(p[i.saturating_sub(1)].at), v(p[(i + 1).min(n - 1)].at));
        let span = if i == 0 || i == n - 1 { 3.0 } else { 6.0 };
        (next - prev) / span
    };
    (0..n.saturating_sub(1)).map(|i| [v(p[i].at), v(p[i].at) + handle(i), v(p[i + 1].at) - handle(i + 1), v(p[i + 1].at)]).collect()
}

pub fn bezier(c: &[Vec3; 4], t: f32) -> Vec3 {
    let s = 1.0 - t;
    c[0] * (s * s * s) + c[1] * (3.0 * s * s * t) + c[2] * (3.0 * s * t * t) + c[3] * (t * t * t)
}

fn bezier_d(c: &[Vec3; 4], t: f32) -> Vec3 {
    let s = 1.0 - t;
    (c[1] - c[0]) * (3.0 * s * s) + (c[2] - c[1]) * (6.0 * s * t) + (c[3] - c[2]) * (3.0 * t * t)
}

impl Path {
    /// None: fewer than two points
    pub fn of(track: &Track) -> Option<Path> {
        if track.points.len() < 2 {
            return None;
        }
        let segs = controls(track);
        let mut walked = Vec::with_capacity(segs.len() * SAMPLES + 1);
        let mut total = 0.0;
        walked.push(0.0);
        for c in &segs {
            let mut last = c[0];
            for k in 1..=SAMPLES {
                let p = bezier(c, k as f32 / SAMPLES as f32);
                total += p.distance(last);
                last = p;
                walked.push(total);
            }
        }
        Some(Path { segs, walked, times: track.points.iter().map(|p| p.time).collect(), eases: track.points.iter().map(|p| p.ease).collect() })
    }

    pub fn len(&self) -> f32 {
        *self.walked.last().unwrap_or(&0.0)
    }

    /// whether a point has its own arrival time
    pub fn timed(&self) -> bool {
        self.times.iter().any(Option::is_some)
    }

    /// how long a fly-through along it takes: the end's time, else the move's `length`
    pub fn length(&self, length: Length) -> Length {
        self.times.last().copied().flatten().unwrap_or(length)
    }

    /// point `i`'s share of the track's length
    fn s_of(&self, i: usize) -> f32 {
        let len = self.len();
        if len <= 0.0 {
            return i as f32 / self.segs.len().max(1) as f32;
        }
        self.walked[(i * SAMPLES).min(self.walked.len() - 1)] / len
    }

    /// The timed fly-through's knots for a move of `length` on a clock of `per_bar` and `bpm`: every point at its time, a
    /// point without one by the track's length between its timed neighbours. None: no point has a time, or no time passes.
    pub fn knots(&self, length: Length, per_bar: u32, bpm: f64) -> Option<Vec<Knot>> {
        if !self.timed() {
            return None;
        }
        let n = self.times.len();
        let secs = |l: Length| l.seconds(per_bar, bpm) as f32;
        let mut t: Vec<Option<f32>> = self.times.iter().map(|x| x.map(secs)).collect();
        t[0] = Some(0.0);
        t[n - 1] = Some(secs(self.length(length)));
        let s: Vec<f32> = (0..n).map(|i| self.s_of(i)).collect();
        let mut a = 0;
        for b in 1..n {
            let Some(tb) = t[b] else { continue };
            let ta = t[a].unwrap_or(0.0).min(tb);
            for i in a + 1..b {
                let share = if s[b] > s[a] { (s[i] - s[a]) / (s[b] - s[a]) } else { (i - a) as f32 / (b - a) as f32 };
                t[i] = Some(ta + (tb - ta) * share);
            }
            t[b] = Some(tb.max(ta));
            a = b;
        }
        let total = t[n - 1].unwrap_or(0.0);
        if total <= 0.0 {
            return None;
        }
        Some((0..n).map(|i| Knot { lin: t[i].unwrap_or(0.0) / total, s: s[i], ease: self.eases[i].unwrap_or(Ease::Linear) }).collect())
    }

    pub fn is_empty(&self) -> bool {
        self.len() <= 0.0
    }

    /// the segment and its parameter at `s` (0..1 of the length)
    fn locate(&self, s: f32) -> (usize, f32) {
        let goal = s.clamp(0.0, 1.0) * self.len();
        let i = self.walked.partition_point(|w| *w < goal).clamp(1, self.walked.len() - 1);
        let (a, b) = (self.walked[i - 1], self.walked[i]);
        let f = if b > a { (goal - a) / (b - a) } else { 0.0 };
        let sample = (i - 1) as f32 + f;
        let seg = ((sample / SAMPLES as f32) as usize).min(self.segs.len() - 1);
        (seg, (sample - (seg * SAMPLES) as f32) / SAMPLES as f32)
    }

    /// the point at `s` (0 the start, 1 the end, by length)
    pub fn at(&self, s: f32) -> Vec3 {
        if s <= 0.0 {
            return self.segs[0][0];
        }
        if s >= 1.0 {
            return self.segs[self.segs.len() - 1][3];
        }
        let (seg, t) = self.locate(s);
        bezier(&self.segs[seg], t)
    }

    /// the way the track runs at `s` (unit length; a degenerate spot: towards the end)
    pub fn tangent(&self, s: f32) -> Vec3 {
        let (seg, t) = self.locate(s);
        bezier_d(&self.segs[seg], t).try_normalize().unwrap_or_else(|| (self.at(1.0) - self.at(0.0)).try_normalize().unwrap_or(Vec3::NEG_Z))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_camera::TrackPoint;

    fn track(points: &[[f32; 3]]) -> Track {
        Track { points: points.iter().map(|p| TrackPoint::at(*p)).collect() }
    }

    #[test]
    fn timed_points_are_reached_on_their_time_and_untimed_ones_by_length() {
        let mut t = track(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [6.0, 0.0, 0.0]]);
        assert!(Path::of(&t).unwrap().knots(Length::Bars(4.0), 4, 120.0).is_none(), "no times: an even walk");
        t.points[1].time = Some(Length::Bars(3.0));
        t.points[3].ease = Some(Ease::InOut);
        let p = Path::of(&t).unwrap();
        let k = p.knots(Length::Bars(4.0), 4, 120.0).unwrap();
        // the end has no time: the move's 4 bars; point 3 between point 2 (bar 3) and the end by length: 1 of 5 m
        let lins: Vec<f32> = k.iter().map(|k| k.lin).collect();
        assert!((lins[1] - 0.75).abs() < 1e-4 && (lins[2] - 0.8).abs() < 1e-3 && lins[3] == 1.0, "{lins:?}");
        for (i, x) in [0.0, 1.0, 2.0, 6.0].iter().enumerate() {
            assert!((p.at(warp(&k, k[i].lin)).x - x).abs() < 0.02, "point {i} on its time");
        }
        assert!((warp(&k, 0.375) - k[1].s * 0.5).abs() < 1e-4, "an even speed on the way to point 2 (no ease)");
        let mid = (k[2].lin + 1.0) * 0.5;
        assert!((warp(&k, mid) - (k[2].s + 1.0) * 0.5).abs() < 1e-3, "in-out: halfway in time is halfway there");
        assert!(warp(&k, k[2].lin + 0.01) - k[2].s < (1.0 - k[2].s) * 0.01, "in-out: slow out of point 3");
    }

    #[test]
    fn a_straight_track_is_walked_evenly() {
        let p = Path::of(&Track::between([0.0, 1.0, 0.0], [4.0, 1.0, 0.0])).unwrap();
        assert!((p.len() - 4.0).abs() < 1e-3);
        for i in 0..=10 {
            let s = i as f32 / 10.0;
            assert!((p.at(s).x - 4.0 * s).abs() < 1e-3, "{s}: {:?}", p.at(s));
        }
        assert!(p.tangent(0.5).distance(Vec3::X) < 1e-4);
    }

    #[test]
    fn a_curved_track_runs_through_its_points_from_start_to_end() {
        let pts = [[0.0, 1.0, 0.0], [2.0, 1.0, -2.0], [4.0, 2.0, 0.0], [6.0, 1.0, 2.0]];
        let p = Path::of(&track(&pts)).unwrap();
        assert_eq!(p.at(0.0), Vec3::from_array(pts[0]));
        assert_eq!(p.at(1.0), Vec3::from_array(pts[3]));
        for mid in &pts[1..3] {
            let near = (0..=1000).map(|i| p.at(i as f32 / 1000.0).distance(Vec3::from_array(*mid))).fold(f32::MAX, f32::min);
            assert!(near < 0.02, "passes {mid:?}: {near}");
        }
        // even speed: equal steps of s cover nearly equal lengths
        let steps: Vec<f32> = (0..20).map(|i| p.at((i + 1) as f32 / 20.0).distance(p.at(i as f32 / 20.0))).collect();
        let (lo, hi) = steps.iter().fold((f32::MAX, 0.0f32), |(lo, hi), s| (lo.min(*s), hi.max(*s)));
        assert!(hi / lo < 1.05, "{lo} … {hi}");
    }

    #[test]
    fn a_handle_bends_the_track() {
        let mut t = Track::between([0.0, 0.0, 0.0], [4.0, 0.0, 0.0]);
        t.points[0].handle = Some([0.0, 0.0, -3.0]);
        let p = Path::of(&t).unwrap();
        assert!(p.at(0.3).z < -0.5, "pulled towards the handle: {:?}", p.at(0.3));
        assert!(p.tangent(0.0).z < -0.9, "leaves along the handle");
        assert!(Path::of(&track(&[[0.0; 3]])).is_none(), "one point is no track");
    }
}
