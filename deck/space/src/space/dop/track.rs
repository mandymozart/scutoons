//! The trolley track as a path: Bézier segments between its points, walked by arc length so a move along it keeps an even
//! speed. Built once when the scene's camera arrives; `at` and `tangent` allocate nothing.
use crate::scene_camera::Track;
use glam::Vec3;

/// samples per segment for the arc length
const SAMPLES: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    segs: Vec<[Vec3; 4]>,
    /// the length walked at each sample, from 0; `SAMPLES` per segment and the end
    walked: Vec<f32>,
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
        Some(Path { segs, walked })
    }

    pub fn len(&self) -> f32 {
        *self.walked.last().unwrap_or(&0.0)
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
        Track { points: points.iter().map(|p| TrackPoint { at: *p, handle: None }).collect() }
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
