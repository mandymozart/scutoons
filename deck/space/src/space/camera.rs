//! The orbit camera (#188): it orbits the being – a mouse drag turns it, the wheel / keys move it in and out, R resets it. Right-handed, Y up, wgpu's
//! depth range 0..1 (glam's `perspective_rh`). `ray` turns a pixel into a world ray for picking. It stands still
//! unless touched (owner 2026-10-31: "The turning camera isn't needed. It's enough that I can orbit."); a scene may
//! turn it slowly by itself once idle (`auto`, the space node's `auto_turn = true`).
use glam::{Mat4, Vec3, Vec4Swizzles};

/// seconds without input before the auto-orbit starts (it eases in over the next two)
pub const IDLE_S: f32 = 4.0;
/// the auto-orbit's speed (rad / s – a turn in ~2 minutes)
pub const AUTO_RAD_S: f32 = 0.05;
const PITCH_MIN: f32 = -1.2;
const PITCH_MAX: f32 = 1.4;
/// the zoom glides to where the wheel / pinch sent it (owner 2026-10-31: "a bit of damping … it should ease in and
/// smooth out. Flowing", #224): the time constant of the distance's ease, s – ~95 % there after 3 of them
pub const ZOOM_EASE_S: f32 = 0.12;
/// #224: the focus glides the same way – to a tapped knob, back to the being: the time constant of the target's ease, s
pub const FOCUS_EASE_S: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    /// unit length
    pub dir: Vec3,
}

impl Ray {
    /// the nearest t ≥ 0 where the ray meets the sphere, if it does
    pub fn sphere(&self, center: Vec3, radius: f32) -> Option<f32> {
        let oc = self.origin - center;
        let b = oc.dot(self.dir);
        let c = oc.length_squared() - radius * radius;
        let disc = b * b - c;
        if disc < 0.0 {
            return None;
        }
        let s = disc.sqrt();
        let t = if -b - s >= 0.0 { -b - s } else { -b + s };
        (t >= 0.0).then_some(t)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orbit {
    pub target: Vec3,
    /// where the focus glides to (`target` follows it in `step`, #224)
    pub goal: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    /// where the zoom glides to (`distance` follows it in `step`)
    pub want: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    /// vertical field of view (rad)
    pub fov_y: f32,
    /// seconds since the last input
    pub idle: f32,
    /// turn slowly by itself once idle (the space node's `auto_turn`; off by default)
    pub auto: bool,
    home: (f32, f32, f32),
}

impl Orbit {
    pub fn new(target: Vec3, yaw: f32, pitch: f32, distance: f32) -> Orbit {
        Orbit {
            target,
            goal: target,
            yaw,
            pitch,
            distance,
            want: distance,
            min_distance: 0.4,
            max_distance: 14.0,
            fov_y: 55f32.to_radians(),
            idle: IDLE_S,
            auto: false,
            home: (yaw, pitch, distance),
        }
    }

    /// the start view (yaw, pitch, distance) – R goes back to it
    pub fn home(&self) -> (f32, f32, f32) {
        self.home
    }

    pub fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.target + self.distance * Vec3::new(cp * sy, sp, cp * cy)
    }

    pub fn view(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye(), self.target, Vec3::Y)
    }

    pub fn proj(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, aspect.max(1e-3), 0.05, 200.0)
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.proj(aspect) * self.view()
    }

    /// aim at `p` at once (a new spot, a set's arrival – no glide across the room)
    pub fn look_at(&mut self, p: Vec3) {
        (self.target, self.goal) = (p, p);
    }

    /// the zoom's and the focus' glides, and the auto-orbit once idle (eased in) when `auto`
    pub fn step(&mut self, dt: f32) {
        // in the log of the distance, so a step in is as smooth close up as far out
        let k = 1.0 - (-dt.max(0.0) / ZOOM_EASE_S).exp();
        let d = (self.distance.ln() + (self.want.ln() - self.distance.ln()) * k).exp();
        self.distance = if (d / self.want - 1.0).abs() < 1e-4 { self.want } else { d };
        // the focus: a straight glide toward the goal, a share of the way each frame – never past it (#224)
        let k = 1.0 - (-dt.max(0.0) / FOCUS_EASE_S).exp();
        let t = self.target + (self.goal - self.target) * k;
        self.target = if t.distance(self.goal) < 1e-5 { self.goal } else { t };
        self.idle += dt;
        if !self.auto {
            return;
        }
        let ease = ((self.idle - IDLE_S) / 2.0).clamp(0.0, 1.0);
        self.yaw = wrap_angle(self.yaw + AUTO_RAD_S * ease * dt);
    }

    /// a drag by (dx, dy) pixels: right turns the camera to the left around the target, up looks from above
    pub fn drag(&mut self, dx: f32, dy: f32) {
        self.yaw = wrap_angle(self.yaw - dx * 0.006);
        self.pitch = (self.pitch + dy * 0.006).clamp(PITCH_MIN, PITCH_MAX);
        self.idle = 0.0;
    }

    /// `steps` > 0 moves in (a wheel notch = 1, 12 % each); the camera glides there in `step`
    pub fn zoom(&mut self, steps: f32) {
        self.want = (self.want * 0.88f32.powf(steps)).clamp(self.min_distance, self.max_distance);
        self.idle = 0.0;
    }

    /// turn by `rad` (the arrow keys)
    pub fn turn(&mut self, rad: f32) {
        self.yaw = wrap_angle(self.yaw + rad);
        self.idle = 0.0;
    }

    pub fn reset(&mut self) {
        (self.yaw, self.pitch, self.distance) = self.home;
        self.want = self.distance;
        self.idle = 0.0;
    }

    /// #224: every sphere (centre, radius) inside the view of `aspect` (w / h), kept `margin` (a share of the half
    /// width / height) off the edges and in front of the near plane
    pub fn fits(&self, aspect: f32, spheres: impl IntoIterator<Item = (Vec3, f32)>, margin: f32) -> bool {
        let view = self.view();
        let ty = (self.fov_y * 0.5).tan() * (1.0 - margin);
        let tx = ty * aspect.max(1e-3);
        spheres.into_iter().all(|(c, r)| {
            let q = view.transform_point3(c);
            let z = -q.z;
            // the sphere's edge against each side plane, conservatively (r along the plane's normal ≤ r)
            z > r + 0.05 && q.x.abs() + r * (1.0 + tx * tx).sqrt() <= z * tx && q.y.abs() + r * (1.0 + ty * ty).sqrt() <= z * ty
        })
    }

    /// the world ray through pixel (x, y) of a w × h view (y down, like the window's cursor)
    pub fn ray(&self, x: f32, y: f32, w: f32, h: f32) -> Ray {
        let inv = self.view_proj(w / h).inverse();
        let ndc = (2.0 * x / w - 1.0, 1.0 - 2.0 * y / h);
        let near = inv * glam::Vec4::new(ndc.0, ndc.1, 0.0, 1.0);
        let far = inv * glam::Vec4::new(ndc.0, ndc.1, 1.0, 1.0);
        let (near, far) = (near.xyz() / near.w, far.xyz() / far.w);
        Ray { origin: near, dir: (far - near).normalize() }
    }
}

/// The photographer's focus (owner 2026-10-31: "the focus point depends on the zoom"): from `far` out the camera aims at
/// the whole being's centre (`body`), from `near` in at its head's, between them it wanders up smoothly – a smoothstep
/// over the distance's log, so each zoom step moves it alike.
pub fn focus_target(body: Vec3, head: Vec3, distance: f32, near: f32, far: f32) -> Vec3 {
    let span = (far.max(near * 1.001) / near.max(1e-3)).ln();
    let t = ((far.max(1e-3) / distance.max(1e-3)).ln() / span).clamp(0.0, 1.0);
    body.lerp(head, t * t * (3.0 - 2.0 * t))
}

fn wrap_angle(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    a - tau * (a / tau).floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn the_eye_sits_on_the_orbit() {
        let c = Orbit::new(Vec3::new(1.0, 2.0, 3.0), 0.0, 0.0, 5.0);
        assert!(close(c.eye(), Vec3::new(1.0, 2.0, 8.0)), "yaw 0 looks along −z from +z");
        let c = Orbit::new(Vec3::ZERO, std::f32::consts::FRAC_PI_2, 0.0, 2.0);
        assert!(close(c.eye(), Vec3::new(2.0, 0.0, 0.0)));
        let c = Orbit::new(Vec3::ZERO, 0.3, 0.7, 3.0);
        assert!((c.eye().length() - 3.0).abs() < 1e-4);
    }

    #[test]
    fn the_centre_ray_hits_the_target() {
        let c = Orbit::new(Vec3::new(4.0, 0.0, 3.0), 1.1, 0.4, 2.5);
        let r = c.ray(640.0, 360.0, 1280.0, 720.0);
        let t = r.sphere(c.target, 0.01).expect("the target is hit");
        assert!((t - (2.5 - 0.05 - 0.01)).abs() < 0.02, "near plane to the target: {t}");
        // a pixel to the right looks right of the target
        let r = c.ray(1200.0, 360.0, 1280.0, 720.0);
        assert!(r.sphere(c.target, 0.3).is_none());
    }

    #[test]
    fn a_projected_point_comes_back_as_a_ray() {
        let c = Orbit::new(Vec3::ZERO, 0.5, 0.2, 3.0);
        let p = Vec3::new(0.3, -0.2, 0.1);
        let clip = c.view_proj(16.0 / 9.0) * p.extend(1.0);
        let ndc = clip.xyz() / clip.w;
        let (x, y) = ((ndc.x + 1.0) * 0.5 * 1600.0, (1.0 - ndc.y) * 0.5 * 900.0);
        assert!((0.0..=1.0).contains(&ndc.z), "wgpu's depth range");
        assert!(c.ray(x, y, 1600.0, 900.0).sphere(p, 1e-3).is_some());
    }

    #[test]
    fn zoom_and_pitch_are_clamped_and_reset_returns_home() {
        let mut c = Orbit::new(Vec3::ZERO, 0.2, 0.1, 2.0);
        c.zoom(100.0);
        assert_eq!(c.want, c.min_distance);
        c.step(2.0);
        assert_eq!(c.distance, c.min_distance);
        c.zoom(-100.0);
        c.step(2.0);
        assert_eq!(c.distance, c.max_distance);
        c.drag(0.0, 10_000.0);
        assert_eq!(c.pitch, PITCH_MAX);
        c.reset();
        assert_eq!((c.yaw, c.pitch, c.distance, c.want), (0.2, 0.1, 2.0, 2.0));
    }

    /// #224: a wheel notch doesn't jump – the distance glides there, a bit at each frame, and arrives
    #[test]
    fn the_zoom_glides() {
        let mut c = Orbit::new(Vec3::ZERO, 0.0, 0.0, 4.0);
        c.zoom(3.0);
        let goal = c.want;
        assert_eq!(c.distance, 4.0, "no jump at the notch");
        let mut last = c.distance;
        let mut frames = 0;
        while c.distance != goal {
            c.step(1.0 / 60.0);
            assert!(c.distance < last && c.distance >= goal, "in, never past: {} → {}", last, c.distance);
            last = c.distance;
            frames += 1;
            assert!(frames < 120, "it arrives within two seconds");
        }
        assert!(frames > 6, "eased, not instant: {frames} frames");
    }

    /// #224: the focus glides to a new goal along a straight line, nearer each frame, never past it, and arrives
    #[test]
    fn the_focus_glides_and_never_overshoots() {
        let mut c = Orbit::new(Vec3::ZERO, 0.0, 0.0, 3.0);
        c.goal = Vec3::new(0.6, -0.4, 0.2);
        let mut last = c.target.distance(c.goal);
        let mut frames = 0;
        while c.target != c.goal {
            c.step(1.0 / 60.0);
            let d = c.target.distance(c.goal);
            assert!(d < last, "nearer each frame: {last} → {d}");
            // on the straight line from the start: never off to a side, never beyond
            let along = c.target.dot(c.goal.normalize()) / c.goal.length();
            assert!((0.0..=1.0).contains(&along) && c.target.cross(c.goal).length() < 1e-5, "on the way: {:?}", c.target);
            last = d;
            frames += 1;
            assert!(frames < 180, "it arrives within three seconds");
        }
        assert!(frames > 8, "eased, not a jump: {frames} frames");
        // a long frame (a stall) lands on the goal, not past it
        c.goal = Vec3::new(-2.0, 0.0, 0.0);
        c.step(10.0);
        assert_eq!(c.target, c.goal);
        c.look_at(Vec3::ONE);
        assert_eq!((c.target, c.goal), (Vec3::ONE, Vec3::ONE), "a spot's change: at once");
    }

    #[test]
    fn a_sphere_fits_until_it_meets_the_view_edge() {
        let c = Orbit::new(Vec3::ZERO, 0.0, 0.0, 3.0);
        assert!(c.fits(1.0, [(Vec3::ZERO, 0.5)], 0.0));
        assert!(!c.fits(1.0, [(Vec3::ZERO, 2.0)], 0.0), "bigger than the view");
        // the edge at depth 3: tan(27.5°) · 3 ≈ 1.56 – a small sphere just inside, then just outside
        let edge = (c.fov_y * 0.5).tan() * 3.0;
        assert!(c.fits(1.0, [(Vec3::new(0.0, edge - 0.15, 0.0), 0.1)], 0.0));
        assert!(!c.fits(1.0, [(Vec3::new(0.0, edge - 0.05, 0.0), 0.1)], 0.0));
        assert!(c.fits(2.0, [(Vec3::new(edge * 1.5, 0.0, 0.0), 0.1)], 0.0), "wider on a wide view");
        assert!(!c.fits(1.0, [(Vec3::new(0.0, 0.0, 3.5), 0.1)], 0.0), "behind the eye");
        assert!(!c.fits(1.0, [(Vec3::new(0.0, edge - 0.15, 0.0), 0.1)], 0.2), "the margin");
    }

    #[test]
    fn it_stands_still_unless_a_scene_asks_it_to_turn() {
        let mut c = Orbit::new(Vec3::ZERO, 0.0, 0.0, 2.0);
        for _ in 0..600 {
            c.step(1.0 / 60.0);
        }
        assert_eq!(c.yaw, 0.0, "by default it never turns by itself");
        // auto_turn: only when idle, eased in
        c.auto = true;
        c.drag(1.0, 0.0);
        let yaw = c.yaw;
        c.step(1.0);
        assert_eq!(c.yaw, yaw, "touched a second ago");
        for _ in 0..600 {
            c.step(1.0 / 60.0);
        }
        assert!(c.yaw != yaw, "ten seconds later it turns");
        let before = c.yaw;
        c.step(1.0);
        assert!((wrap_angle(c.yaw - before) - AUTO_RAD_S).abs() < 1e-4, "at full speed");
    }
}
