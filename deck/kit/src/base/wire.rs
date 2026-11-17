//! A link between two ports in the 2D views, hanging like the 3D views' hoses: a cubic curve that sags under its ends,
//! deeper the farther apart they are, sampled into a polyline, with beads spaced evenly along it that run with the signal.

/// how far under its ends a wire hangs at least, in `unit`s (the UI's rem)
pub const SAG_MIN: f32 = 0.35;
/// how much deeper it hangs per pixel between its ends
pub const SAG_PER_PX: f32 = 0.1;
/// the polyline's pieces
pub const SAMPLES: usize = 32;
/// the beads' spacing and how far they run per second, in `unit`s
pub const BEAD_GAP: f32 = 2.4;
pub const BEAD_SPEED: f32 = 2.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wire {
    /// the curve's control points: the two ends and the two pulled down between them
    pub c: [(f32, f32); 4],
}

impl Wire {
    /// The wire from `a` to `b` (y grows down); between level ends it hangs `SAG_MIN` × `unit` + `SAG_PER_PX` × their
    /// distance under them at its middle.
    pub fn hang(a: (f32, f32), b: (f32, f32), unit: f32) -> Wire {
        let depth = SAG_MIN * unit + SAG_PER_PX * (b.0 - a.0).hypot(b.1 - a.1);
        // a cubic's middle sits at ¾ of its inner points' offset
        let pull = depth / 0.75;
        // pulled down from the lower end: ends at two heights still dip under both before they rise into the lower
        let low = a.1.max(b.1) + pull;
        Wire { c: [a, (a.0 + (b.0 - a.0) / 3.0, low), (b.0 + (a.0 - b.0) / 3.0, low), b] }
    }

    /// The wire from an outlet at the foot of a node down to an inlet on top of another (a top → bottom view): it leaves
    /// straight down and comes straight down into `b`, by half their height apart, at least the sag's least.
    pub fn drop(a: (f32, f32), b: (f32, f32), unit: f32) -> Wire {
        let pull = ((b.1 - a.1).abs() * 0.5).max(SAG_MIN * unit * 4.0);
        Wire { c: [a, (a.0, a.1 + pull), (b.0, b.1 - pull), b] }
    }

    /// the point at `t` (0 = from, 1 = to)
    pub fn at(&self, t: f32) -> (f32, f32) {
        let [p0, p1, p2, p3] = self.c;
        let s = 1.0 - t;
        let (k0, k1, k2, k3) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
        (k0 * p0.0 + k1 * p1.0 + k2 * p2.0 + k3 * p3.0, k0 * p0.1 + k1 * p1.1 + k2 * p2.1 + k3 * p3.1)
    }

    /// the curve as `SAMPLES` straight pieces
    pub fn points(&self) -> Vec<(f32, f32)> {
        (0..=SAMPLES).map(|i| self.at(i as f32 / SAMPLES as f32)).collect()
    }

    /// its length along the polyline
    pub fn length(&self) -> f32 {
        self.points().windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum()
    }

    /// The beads, `gap` apart along the wire, moved `travelled` along it from `from` toward `to` (they wrap: a bead
    /// leaving at the end comes in at the start).
    pub fn beads(&self, gap: f32, travelled: f32) -> Vec<(f32, f32)> {
        let pts = self.points();
        let mut out = Vec::new();
        if gap <= 0.0 {
            return out;
        }
        let mut s = travelled.rem_euclid(gap);
        let mut walked = 0.0;
        for w in pts.windows(2) {
            let piece = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
            while s <= walked + piece && piece > 0.0 {
                let f = (s - walked) / piece;
                out.push((w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f));
                s += gap;
            }
            walked += piece;
        }
        out
    }

    /// the beads after the signal has run `secs`
    pub fn flowing(&self, unit: f32, secs: f32) -> Vec<(f32, f32)> {
        self.beads(BEAD_GAP * unit, BEAD_SPEED * unit * secs)
    }

    /// how far its lowest point hangs under its lower end
    pub fn depth(&self) -> f32 {
        let low = self.points().into_iter().map(|p| p.1).fold(f32::MIN, f32::max);
        low - self.c[0].1.max(self.c[3].1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wire_ends_at_its_ports_and_hangs_under_them() {
        let w = Wire::hang((0.0, 100.0), (300.0, 100.0), 16.0);
        assert_eq!(w.at(0.0), (0.0, 100.0));
        assert_eq!(w.at(1.0), (300.0, 100.0));
        let mid = w.at(0.5);
        assert!((mid.0 - 150.0).abs() < 1e-3, "level ends: the low point in the middle");
        assert!((w.depth() - (SAG_MIN * 16.0 + SAG_PER_PX * 300.0)).abs() < 0.5, "{}", w.depth());
    }

    #[test]
    fn the_sag_grows_with_the_distance() {
        let near = Wire::hang((0.0, 0.0), (100.0, 0.0), 16.0).depth();
        let far = Wire::hang((0.0, 0.0), (400.0, 0.0), 16.0).depth();
        assert!(far > near + 25.0, "{near} {far}");
        let close = Wire::hang((0.0, 0.0), (0.0, 0.0), 16.0).depth();
        assert!(close > 0.0, "even ends that meet hang a little: {close}");
    }

    #[test]
    fn a_wire_between_ends_at_two_heights_still_dips_under_the_lower() {
        let w = Wire::hang((0.0, 0.0), (40.0, 200.0), 16.0);
        assert!(w.depth() > 0.0);
        assert!(w.points().iter().all(|p| p.0 >= -1e-3 && p.0 <= 40.0 + 1e-3), "it hangs, it does not swing out sideways");
    }

    #[test]
    fn a_dropped_wire_leaves_and_enters_straight_down() {
        let w = Wire::drop((0.0, 0.0), (120.0, 200.0), 16.0);
        assert_eq!((w.at(0.0), w.at(1.0)), ((0.0, 0.0), (120.0, 200.0)));
        let (start, end) = (w.at(0.02), w.at(0.98));
        assert!(start.0.abs() < 1.0 && start.1 > 0.0, "{start:?}");
        assert!((end.0 - 120.0).abs() < 1.0 && end.1 < 200.0, "{end:?}");
        let back = Wire::drop((0.0, 200.0), (0.0, 0.0), 16.0);
        assert!(back.points().iter().any(|p| p.1 > 200.0) && back.points().iter().any(|p| p.1 < 0.0), "a wire back up loops round both ends");
    }

    #[test]
    fn beads_sit_evenly_along_it_and_run_with_the_signal() {
        let w = Wire::hang((0.0, 0.0), (300.0, 0.0), 16.0);
        let len = w.length();
        let beads = w.beads(30.0, 0.0);
        assert_eq!(beads.len(), (len / 30.0) as usize + 1);
        assert_eq!(beads[0], (0.0, 0.0), "at rest a bead sits on the from port");
        let gaps: Vec<f32> = beads.windows(2).map(|b| (b[1].0 - b[0].0).hypot(b[1].1 - b[0].1)).collect();
        assert!(gaps.iter().all(|g| *g > 25.0 && *g <= 30.0 + 1e-3), "{gaps:?}");
        // moved on: the first bead has left the port toward `to`, and a whole gap later they stand where they were
        let moved = w.beads(30.0, 10.0);
        assert!(moved[0].0 > 0.0 && moved[0].0 < beads[1].0);
        let lap = w.beads(30.0, 30.0);
        assert!((lap[0].0 - beads[0].0).abs() < 1e-3 && lap.len() == beads.len());
        assert!(w.beads(0.0, 5.0).is_empty());
    }
}
