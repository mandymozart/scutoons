//! Panning laws over a speaker layout (plan 2.6, #318): equal-power between two speakers, VBAP (Pulkki 1997: the gains of the
//! speaker pair – or, when the array has height, the triple – that encloses the direction, power-normalised) and DBAP
//! (Lossius 2009: distance-based, any array). The speakers are data (layout.rs). Directions are azimuth degrees clockwise
//! from the front, elevation up. Pure maths on libm and IEEE sqrt: the same bits on every target.
//!
//! Everything is built once (`Panner::new`: the triples' inverses are the one allocation) and `gains` writes into a slice –
//! the render only calls `gains`.
use super::layout::{Layout, Speaker};
use anyhow::{bail, Result};
use std::f32::consts::{FRAC_PI_4, PI};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Law {
    EqualPower,
    Vbap,
    Dbap,
}

impl Law {
    pub fn parse(s: &str) -> Option<Law> {
        Some(match s {
            "equal_power" => Law::EqualPower,
            "vbap" => Law::Vbap,
            "dbap" => Law::Dbap,
            _ => return None,
        })
    }
}

/// unit vector of a direction: x right, y front, z up
fn dir(az: f32, el: f32) -> [f32; 3] {
    let (a, e) = (az * PI / 180.0, el * PI / 180.0);
    let ce = libm::cosf(e);
    [libm::sinf(a) * ce, libm::cosf(a) * ce, libm::sinf(e)]
}

/// a triple of speakers and the inverse of their unit-vector matrix (row-major)
struct Tri {
    idx: [usize; 3],
    inv: [f32; 9],
}

pub struct Panner {
    law: Law,
    /// the speakers' directions (unit vectors) and positions (direction × distance)
    units: Vec<[f32; 3]>,
    spots: Vec<[f32; 3]>,
    lfe: Vec<bool>,
    /// azimuths of the non-LFE speakers, sorted, with their channel (VBAP in the plane, equal power)
    ring: Vec<(f32, usize)>,
    /// VBAP over an array with height
    tris: Vec<Tri>,
    /// DBAP: the exponent of the distance (rolloff dB / 6.02) and the blur squared
    a: f32,
    blur2: f32,
}

fn det3(m: &[f32; 9]) -> f32 {
    m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6]) + m[2] * (m[3] * m[7] - m[4] * m[6])
}

/// the inverse of a 3 × 3 (row-major); None when singular
fn inv3(m: &[f32; 9]) -> Option<[f32; 9]> {
    let d = det3(m);
    if d.abs() < 1e-3 {
        return None;
    }
    let c = |a: usize, b: usize, c: usize, d: usize| m[a] * m[d] - m[b] * m[c];
    Some([
        c(4, 5, 7, 8) / d,
        -c(1, 2, 7, 8) / d,
        c(1, 2, 4, 5) / d,
        -c(3, 5, 6, 8) / d,
        c(0, 2, 6, 8) / d,
        -c(0, 2, 3, 5) / d,
        c(3, 4, 6, 7) / d,
        -c(0, 1, 6, 7) / d,
        c(0, 1, 3, 4) / d,
    ])
}

impl Panner {
    /// `rolloff` dB per doubling of the distance and `blur` are DBAP's
    pub fn new(layout: &Layout, law: Law, rolloff: f32, blur: f32) -> Result<Panner> {
        let live: Vec<usize> = (0..layout.channels()).filter(|&i| !layout.speakers[i].lfe).collect();
        match law {
            Law::EqualPower if live.len() != 2 => bail!("equal power pans between two speakers; layout '{}' has {}", layout.name, live.len()),
            _ if live.is_empty() => bail!("layout '{}' has no speakers to pan to", layout.name),
            _ => {}
        }
        let units: Vec<[f32; 3]> = layout.speakers.iter().map(|s: &Speaker| dir(s.az, s.el)).collect();
        let spots = layout.speakers.iter().zip(&units).map(|(s, u)| [u[0] * s.dist, u[1] * s.dist, u[2] * s.dist]).collect();
        let mut ring: Vec<(f32, usize)> = live.iter().map(|&i| (layout.speakers[i].az.rem_euclid(360.0), i)).collect();
        ring.sort_by(|a, b| a.0.total_cmp(&b.0));
        let height = live.iter().any(|&i| layout.speakers[i].el.abs() > 1e-3);
        let mut tris = Vec::new();
        if law == Law::Vbap && height {
            for (x, &i) in live.iter().enumerate() {
                for (y, &j) in live.iter().enumerate().skip(x + 1) {
                    for &k in live.iter().skip(y + 1) {
                        let (a, b, c) = (units[i], units[j], units[k]);
                        if let Some(inv) = inv3(&[a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]]) {
                            tris.push(Tri { idx: [i, j, k], inv });
                        }
                    }
                }
            }
            if tris.is_empty() && live.len() > 2 {
                bail!("layout '{}': VBAP finds no speaker triple (all speakers on one line?)", layout.name);
            }
        }
        Ok(Panner {
            law,
            units,
            spots,
            lfe: layout.speakers.iter().map(|s| s.lfe).collect(),
            ring,
            tris,
            a: rolloff / 6.020_6,
            blur2: blur * blur,
        })
    }

    pub fn channels(&self) -> usize {
        self.units.len()
    }

    /// The gains of every speaker for a source at (`az`, `el`, `dist`): `out` has one slot per speaker. Never allocates.
    pub fn gains(&self, az: f32, el: f32, dist: f32, out: &mut [f32]) {
        out.fill(0.0);
        match self.law {
            Law::EqualPower => self.equal_power(az, out),
            Law::Vbap if self.tris.is_empty() => self.vbap_plane(az, out),
            Law::Vbap => self.vbap_triples(az, el, out),
            Law::Dbap => self.dbap(az, el, dist, out),
        }
    }

    /// between the two speakers: the source at the first one is all first, at the second all second, the middle half and
    /// half – cos θ, sin θ with θ from 0 to a quarter turn, so the powers always add to one. The speakers are taken in the
    /// order a clockwise turn of less than a half circle runs from the first to the second.
    fn equal_power(&self, az: f32, out: &mut [f32]) {
        let ((a, i), (b, j)) = (self.ring[0], self.ring[1]);
        let d = (b - a + 540.0).rem_euclid(360.0) - 180.0;
        let ((a, first), (d, second)) = if d >= 0.0 { ((a, i), (d, j)) } else { ((b, j), (-d, i)) };
        let (mid, half) = (a + 0.5 * d, 0.5 * d);
        let rel = (az - mid + 180.0).rem_euclid(360.0) - 180.0;
        let x = if half > 0.0 { (rel / half).clamp(-1.0, 1.0) } else { 0.0 };
        let theta = (x + 1.0) * FRAC_PI_4;
        out[first] = libm::cosf(theta);
        out[second] = libm::sinf(theta);
    }

    /// 2-D VBAP on the ring of speakers (elevation ignored)
    fn vbap_plane(&self, az: f32, out: &mut [f32]) {
        let n = self.ring.len();
        if n == 1 {
            out[self.ring[0].1] = 1.0;
            return;
        }
        let a = az.rem_euclid(360.0);
        // the adjacent pair (i, i+1 around the circle) the direction lies between
        for i in 0..n {
            let (a0, c0) = self.ring[i];
            let (a1, c1) = self.ring[(i + 1) % n];
            let (a1, p) = if a1 <= a0 { (a1 + 360.0, if a < a0 { a + 360.0 } else { a }) } else { (a1, a) };
            if p < a0 || p > a1 {
                continue;
            }
            if a1 - a0 >= 180.0 {
                // the pair spans a half turn or more: its vectors do not enclose the direction – the nearer speaker plays
                out[if p - a0 <= a1 - p { c0 } else { c1 }] = 1.0;
                return;
            }
            let (l0, l1) = (self.units[c0], self.units[c1]);
            let pv = dir(a, 0.0);
            // p = g0 l0 + g1 l1 in the plane: invert the 2 × 2 of the speakers' (x, y)
            let det = l0[0] * l1[1] - l1[0] * l0[1];
            let (g0, g1) = ((pv[0] * l1[1] - l1[0] * pv[1]) / det, (l0[0] * pv[1] - pv[0] * l0[1]) / det);
            let norm = (g0 * g0 + g1 * g1).sqrt();
            out[c0] = g0 / norm;
            out[c1] = g1 / norm;
            return;
        }
    }

    /// 3-D VBAP: of all speaker triples those whose gains are all non-negative enclose the direction; the tightest (the
    /// smallest sum of gains – it is 1 when the source stands on a speaker, and grows with the size of the triangle) plays.
    /// Outside every triple (below the array's lowest speakers) the best triple's negative gains are cut.
    fn vbap_triples(&self, az: f32, el: f32, out: &mut [f32]) {
        let p = dir(az, el);
        let mut best: Option<(bool, f32, usize, [f32; 3])> = None;
        for (t, tri) in self.tris.iter().enumerate() {
            let m = &tri.inv;
            // g = p · inverse (p is a row vector)
            let g = [p[0] * m[0] + p[1] * m[3] + p[2] * m[6], p[0] * m[1] + p[1] * m[4] + p[2] * m[7], p[0] * m[2] + p[1] * m[5] + p[2] * m[8]];
            let low = g[0].min(g[1]).min(g[2]);
            let enclosing = low >= -1e-5;
            // an enclosing triple ranks by the smallest sum, the others by the largest minimum
            let score = if enclosing { g[0] + g[1] + g[2] } else { -low };
            let better = match best {
                None => true,
                Some((e, s, _, _)) => (enclosing && !e) || (enclosing == e && score < s),
            };
            if better {
                best = Some((enclosing, score, t, g));
            }
        }
        let Some((_, _, t, g)) = best else { return };
        let g = g.map(|x| x.max(0.0));
        let norm = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
        if norm > 0.0 {
            for (k, &c) in self.tris[t].idx.iter().enumerate() {
                out[c] = g[k] / norm;
            }
        }
    }

    /// DBAP: every speaker plays 1 / d^a, d the (blurred) distance to the source, normalised to unit power
    fn dbap(&self, az: f32, el: f32, dist: f32, out: &mut [f32]) {
        let u = dir(az, el);
        let src = [u[0] * dist, u[1] * dist, u[2] * dist];
        let mut sum = 0.0f32;
        for (i, s) in self.spots.iter().enumerate() {
            if self.lfe[i] {
                continue;
            }
            let (dx, dy, dz) = (src[0] - s[0], src[1] - s[1], src[2] - s[2]);
            let d = (dx * dx + dy * dy + dz * dz + self.blur2).sqrt();
            let w = 1.0 / libm::powf(d.max(1e-6), self.a);
            out[i] = w;
            sum += w * w;
        }
        let k = 1.0 / sum.sqrt();
        for g in out.iter_mut() {
            *g *= k;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::layout::builtin;
    use super::*;

    fn gains(p: &Panner, az: f32, el: f32) -> Vec<f32> {
        let mut g = vec![0.0; p.channels()];
        p.gains(az, el, 1.0, &mut g);
        g
    }

    fn power(g: &[f32]) -> f32 {
        g.iter().map(|x| x * x).sum()
    }

    /// equal power: the middle is 1/√2 each, a speaker's own direction is that speaker alone, and the powers add to one
    /// everywhere between and beyond
    #[test]
    fn equal_power_keeps_the_power() {
        let p = Panner::new(&builtin("stereo").unwrap(), Law::EqualPower, 6.0, 0.1).unwrap();
        let mid = gains(&p, 0.0, 0.0);
        assert!((mid[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6 && (mid[1] - mid[0]).abs() < 1e-6, "{mid:?}");
        assert_eq!(gains(&p, -30.0, 0.0)[1], 0.0);
        assert!((gains(&p, -30.0, 0.0)[0] - 1.0).abs() < 1e-7);
        let right = gains(&p, 30.0, 0.0);
        assert!(right[0].abs() < 1e-6 && (right[1] - 1.0).abs() < 1e-7, "{right:?}");
        assert_eq!(gains(&p, 170.0, 0.0), gains(&p, 30.0, 0.0), "behind: the nearer side's speaker");
        for az in -40..=40 {
            assert!((power(&gains(&p, az as f32 * 1.5, 0.0)) - 1.0).abs() < 1e-6, "az {az}");
        }
    }

    /// VBAP over four speakers (45° 135° 225° 315°): a speaker's own direction is that speaker alone; between two the pair
    /// shares by Pulkki's tangent law, powers add to one, the other two are silent
    #[test]
    fn vbap_over_four_speakers() {
        let ring = Layout::new("ring", &[
            super::super::layout::Speaker { az: 45.0, el: 0.0, dist: 1.0, lfe: false },
            super::super::layout::Speaker { az: 135.0, el: 0.0, dist: 1.0, lfe: false },
            super::super::layout::Speaker { az: 225.0, el: 0.0, dist: 1.0, lfe: false },
            super::super::layout::Speaker { az: 315.0, el: 0.0, dist: 1.0, lfe: false },
        ]);
        let p = Panner::new(&ring, Law::Vbap, 6.0, 0.1).unwrap();
        let at = gains(&p, 135.0, 0.0);
        assert!((at[1] - 1.0).abs() < 1e-6 && at[0].abs() < 1e-6 && at[2].abs() < 1e-6 && at[3] == 0.0, "{at:?}");
        let mid = gains(&p, 90.0, 0.0);
        assert!((mid[0] - mid[1]).abs() < 1e-6 && (mid[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6, "{mid:?}");
        assert_eq!((mid[2], mid[3]), (0.0, 0.0));
        // the tangent law of a symmetric pair (speakers 45° and 135°, the middle 90°): at 67.5°, 22.5° off the middle,
        let q = gains(&p, 67.5, 0.0);
        // (g_a − g_b) / (g_a + g_b) = tan θ / tan θ0 with θ the offset from the pair's middle and θ0 = 45° the half spread
        let (a, b) = (q[0], q[1]);
        let theta = (67.5f32 - 90.0).to_radians();
        let lhs = (a - b) / (a + b);
        let rhs = -theta.tan() / 45.0f32.to_radians().tan();
        assert!((lhs - rhs).abs() < 1e-5, "{lhs} vs {rhs}");
        // every direction: power one, at most two speakers play, none negative
        for deg in 0..360 {
            let g = gains(&p, deg as f32, 0.0);
            assert!((power(&g) - 1.0).abs() < 1e-5, "az {deg}: {g:?}");
            assert!(g.iter().all(|x| *x >= -1e-6) && g.iter().filter(|x| **x > 1e-6).count() <= 2, "az {deg}: {g:?}");
        }
    }

    /// 5.1: the LFE slot stays silent, the centre speaker is hit exactly from the front
    #[test]
    fn the_lfe_gets_nothing() {
        let l = builtin("5.1").unwrap();
        for law in [Law::Vbap, Law::Dbap] {
            let p = Panner::new(&l, law, 6.0, 0.1).unwrap();
            for az in [-170.0, -60.0, 0.0, 20.0, 100.0] {
                let g = gains(&p, az, 0.0);
                assert_eq!(g[3], 0.0, "{law:?} az {az}");
                assert!((power(&g) - 1.0).abs() < 1e-5, "{law:?} az {az}: {g:?}");
            }
        }
        let c = gains(&Panner::new(&l, Law::Vbap, 6.0, 0.1).unwrap(), 0.0, 0.0);
        assert!((c[2] - 1.0).abs() < 1e-6, "{c:?}");
    }

    /// DBAP: the nearest speaker plays most, the powers add to one, a source far out in one direction leans the same way
    #[test]
    fn dbap_leans_to_the_near_speakers() {
        let p = Panner::new(&builtin("quad").unwrap(), Law::Dbap, 6.0, 0.1).unwrap();
        let g = gains(&p, 45.0, 0.0);
        assert!(g[1] > g[0] && g[1] > g[2] && g[1] > g[3], "{g:?}");
        assert!((power(&g) - 1.0).abs() < 1e-5);
        let mid = gains(&p, 0.0, 0.0);
        assert!((mid[0] - mid[1]).abs() < 1e-6 && mid[0] > mid[2], "{mid:?}");
    }

    /// an array with height: a source straight up between four ceiling speakers is shared by the triple/pair around it
    #[test]
    fn vbap_with_height_uses_triples() {
        let sp = |az, el| super::super::layout::Speaker { az, el, dist: 1.0, lfe: false };
        let l = Layout::new("dome", &[sp(-45.0, 0.0), sp(45.0, 0.0), sp(135.0, 0.0), sp(-135.0, 0.0), sp(0.0, 60.0), sp(180.0, 60.0)]);
        let p = Panner::new(&l, Law::Vbap, 6.0, 0.1).unwrap();
        assert!(!p.tris.is_empty());
        let on = gains(&p, 0.0, 60.0);
        assert!((on[4] - 1.0).abs() < 1e-5, "{on:?}");
        for (az, el) in [(10.0, 20.0), (100.0, 40.0), (-170.0, 10.0), (0.0, 0.0)] {
            let g = gains(&p, az, el);
            assert!((power(&g) - 1.0).abs() < 1e-4 && g.iter().all(|x| *x >= 0.0), "({az}, {el}): {g:?}");
            assert!(g.iter().filter(|x| **x > 1e-5).count() <= 3);
        }
    }

    #[test]
    fn equal_power_needs_two_speakers() {
        assert!(Panner::new(&builtin("quad").unwrap(), Law::EqualPower, 6.0, 0.1).is_err());
    }
}
