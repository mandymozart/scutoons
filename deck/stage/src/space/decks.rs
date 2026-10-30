//! The decks around the conductor (#188 phase 4): every deck that streams its being over the relay becomes a `Remote`
//! – its graph rebuilt from the wire, a being made the same way as this deck's own (values.rs `Mirror` instead of the
//! control domain), its head with its character's portrait. They float in a circle around the conductor's being in
//! the middle of the nave (owner 2026-10-30: "not at the pulpit but in the middle of the nave"), evenly spaced by
//! client id; one that joins drifts into its place, the others make room. Hoses between beings follow the session's
//! routes (presence: who sends values to whom), sagging between their cores; bubbles run along while values come.
//!
//! Joining / leaving allocates (a new being, new GPU buffers – `Space::generation`); a frame does not.
use super::being::{bezier, hsv, kind, tube, Being, Bubble, Look, Sphere, TubeVertex, BUBBLES};
use super::values::Mirror;
use crate::graph::Graph;
use crate::tempo;
use glam::Vec3;

pub struct Remote {
    pub client: String,
    pub graph: Graph,
    pub being: Being,
    pub mirror: Mirror,
    /// where it floats in the circle (the being eases toward it)
    pub place: Vec3,
    /// values per second lately (the hoses out of it bubble with it)
    pub rate: f32,
    count: f32,
}

impl Remote {
    pub fn new(client: String, graph: Graph, look: &Look) -> Remote {
        let being = Being::new(&graph, look);
        let mirror = Mirror::new(&graph);
        Remote { client, graph, being, mirror, place: Vec3::ZERO, rate: 0.0, count: 0.0 }
    }

    /// a `deck.values` message: its values into the mirror, its mood onto the head
    pub fn apply(&mut self, values: &std::collections::HashMap<String, Vec<f64>>, mood: Option<&str>) {
        let mut v = [0.0f32; 16];
        for (path, x) in values {
            let n = x.len().min(v.len());
            for (d, s) in v.iter_mut().zip(x.iter()) {
                *d = *s as f32;
            }
            self.mirror.apply(&self.graph, path, &v[..n]);
        }
        if let Some(m) = mood {
            self.being.head.fixed = self.being.look.moods.iter().position(|x| x == m);
        }
        self.count += 1.0;
    }

    pub fn step(&mut self, dt: f32, time: f32, beat: &tempo::Now) {
        self.being.anchor += (self.place - self.being.anchor) * (dt * 0.8).min(1.0);
        // the rate over the last ~second
        self.rate += (self.count / dt.max(1e-3) - self.rate) * (dt * 1.5).min(1.0);
        self.count = 0.0;
        self.being.update(dt, time, &self.mirror, beat);
    }
}

/// The places on the circle: `n` beings around `center` at `radius`, the first toward `facing` (radians, the camera's
/// start) – so the view starts on a deck, with the conductor behind it.
pub fn circle(center: Vec3, radius: f32, n: usize, facing: f32, out: &mut [Vec3]) {
    for (i, p) in out.iter_mut().enumerate().take(n) {
        let a = facing + std::f32::consts::TAU * i as f32 / n.max(1) as f32;
        *p = center + Vec3::new(a.cos(), 0.0, a.sin()) * radius;
    }
}

/// a hose between two beings: `from` / `to` = None for this deck's own being, else a remote's index
#[derive(Debug, Clone)]
pub struct Route {
    pub from: Option<usize>,
    pub to: Option<usize>,
    pub ctrl: [Vec3; 4],
    pub activity: f32,
    pub bubbles: [Bubble; BUBBLES],
    acc: f32,
}

impl Route {
    pub fn new(from: Option<usize>, to: Option<usize>) -> Route {
        Route { from, to, ctrl: [Vec3::ZERO; 4], activity: 0.0, bubbles: [Bubble::default(); BUBBLES], acc: 0.0 }
    }

    /// the curve between the two cores (sagging like a cable between them) and the bubbles: `rate` = the sender's
    /// values per second
    pub fn step(&mut self, dt: f32, a: Vec3, b: Vec3, rate: f32) {
        let len = (b - a).length();
        let sag = Vec3::Y * (-0.45 * len - 0.3);
        self.ctrl = [a, a + (b - a) * 0.25 + sag, b + (a - b) * 0.25 + sag, b];
        let rate = (rate * 0.25).min(10.0);
        self.activity += ((rate / 6.0).min(1.0) - self.activity) * (dt * 3.0).min(1.0);
        for x in self.bubbles.iter_mut().filter(|x| x.size > 0.0) {
            x.t += x.speed * dt;
            if x.t >= 1.0 {
                x.size = 0.0;
            }
        }
        self.acc = (self.acc + rate * dt).min(2.0);
        while self.acc >= 1.0 {
            self.acc -= 1.0;
            if let Some(free) = self.bubbles.iter_mut().find(|x| x.size == 0.0) {
                *free = Bubble { t: 0.0, speed: 0.22, size: 0.7 };
            }
        }
    }

    /// radius of the hose
    pub fn radius(size: f32) -> f32 {
        0.045 * size.max(0.3)
    }

    pub fn write_ghosts(&self, hue: f32, glow: f32, r: f32, spheres: &mut Vec<Sphere>) {
        let c = hsv(hue + 0.1, 0.3, 1.0);
        for x in self.bubbles.iter().filter(|x| x.size > 0.0) {
            let pos = bezier(&self.ctrl, x.t);
            let fade = (x.t * 10.0).min(1.0) * ((1.0 - x.t) * 10.0).min(1.0);
            spheres.push(Sphere { pos: pos.into(), radius: r * 1.4 * fade.max(0.2), color: [c[0], c[1], c[2], 0.9], extra: [kind::BUBBLE, glow, 0.0, x.size] });
        }
    }

    pub fn write_tube(&self, r: f32, tubes: &mut Vec<TubeVertex>) {
        tube(tubes, &self.ctrl, r, r, self.activity, 0.1, 3.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_circle_is_even_and_starts_where_the_camera_looks() {
        let mut p = [Vec3::ZERO; 4];
        circle(Vec3::new(1.0, 2.0, 3.0), 2.0, 4, 0.0, &mut p);
        assert!((p[0] - Vec3::new(3.0, 2.0, 3.0)).length() < 1e-5);
        for i in 0..4 {
            assert!(((p[i] - Vec3::new(1.0, 2.0, 3.0)).length() - 2.0).abs() < 1e-5);
            assert!(((p[i] - p[(i + 1) % 4]).length() - 8f32.sqrt()).abs() < 1e-4, "even");
        }
    }

    #[test]
    fn a_route_sags_and_bubbles_while_values_come() {
        let mut r = Route::new(Some(0), None);
        let (a, b) = (Vec3::new(-2.0, 1.0, 0.0), Vec3::new(2.0, 1.0, 0.0));
        for _ in 0..120 {
            r.step(1.0 / 60.0, a, b, 30.0);
        }
        assert!(bezier(&r.ctrl, 0.5).y < 1.0 - 0.5, "it sags");
        assert!(r.bubbles.iter().any(|x| x.size > 0.0) && r.activity > 0.5);
        for _ in 0..600 {
            r.step(1.0 / 60.0, a, b, 0.0);
        }
        assert!(r.bubbles.iter().all(|x| x.size == 0.0) && r.activity < 0.01, "quiet when nothing comes");
    }
}
