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
/// a free camera's limit: just short of the poles, where "up" would be undefined
const POLE: f32 = 1.56;
/// the vertical field of view a camera has unless a shot says otherwise (rad)
pub const FOV_Y: f32 = 55.0 * std::f32::consts::PI / 180.0;
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

/// #224: the camera's view as the knobs need it to stay reachable close up (being.rs `fold`): the eye, its axes, the
/// half-view's tangents (w / h), and the least depth a knob keeps in front of the eye
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub eye: Vec3,
    pub fwd: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub tan_x: f32,
    pub tan_y: f32,
    pub min_depth: f32,
}

/// a camera to look through: where it aims, from which side, how far and how wide
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shot {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov_y: f32,
    /// an orthographic view's height (m); none = perspective
    pub ortho: Option<f32>,
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
    /// an editor's camera (#454): the pivot stays where the hand put it, the pitch goes to the poles
    pub free: bool,
    /// the camera looked through: the view glides to it and stays until a hand moves the view
    pub through: Option<Shot>,
    /// an orthographic view's height (m); none = perspective
    pub ortho: Option<f32>,
    /// a hand moved the view since the last `take_touched` (a playing move stops)
    touched: bool,
    home: (f32, f32, f32),
    /// the field of view to go back to when the view leaves a camera
    free_fov: f32,
    /// the free view the camera left for a shot: `go_free` glides back to it
    free_view: Option<Shot>,
    /// gliding back to the free view: on arrival the camera is free again
    returning: bool,
    /// a hand left a shot: the field of view glides back to the free one
    fov_back: bool,
    /// the view a glide into `through` started from, and how long it has run (s)
    glide_from: Shot,
    glide_t: f32,
}

/// how long a glide into a shot, or back to the free view, takes (s): eased in and out
pub const GLIDE_S: f32 = 1.0;

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
            fov_y: FOV_Y,
            idle: IDLE_S,
            auto: false,
            free: false,
            through: None,
            ortho: None,
            touched: false,
            home: (yaw, pitch, distance),
            free_fov: FOV_Y,
            free_view: None,
            returning: false,
            fov_back: false,
            glide_from: Shot { target, yaw, pitch, distance, fov_y: FOV_Y, ortho: None },
            glide_t: 0.0,
        }
    }

    /// whether a hand moved the view since the last ask
    pub fn take_touched(&mut self) -> bool {
        std::mem::take(&mut self.touched)
    }

    /// look through `shot`: the view glides there (`step`) and holds it
    pub fn look_through(&mut self, shot: Shot) {
        if self.through.is_none() && !self.returning {
            self.free_fov = if self.fov_back { self.free_fov } else { self.fov_y };
            self.free_view = Some(self.as_shot(self.free_fov));
        }
        (self.returning, self.fov_back) = (false, false);
        self.glide_from = Shot { target: self.target, yaw: self.yaw, pitch: self.pitch, distance: self.distance, fov_y: self.fov_y, ortho: None };
        self.glide_t = 0.0;
        self.max_distance = self.max_distance.max(shot.distance);
        self.ortho = shot.ortho;
        self.through = Some(shot);
        self.goal = shot.target;
        self.want = shot.distance.clamp(self.min_distance, self.max_distance);
        self.idle = 0.0;
    }

    /// this view as a shot, with the field of view `fov_y`
    fn as_shot(&self, fov_y: f32) -> Shot {
        Shot { target: self.goal, yaw: self.yaw, pitch: self.pitch, distance: self.want, fov_y, ortho: None }
    }

    /// the eased glide back to the free view the camera left for a shot; already free: nothing
    pub fn go_free(&mut self) {
        match self.free_view.take() {
            Some(free) if self.through.is_some() => {
                self.look_through(free);
                self.returning = true;
            }
            _ => {}
        }
    }

    /// whether the camera looks through a shot (or glides to one) – not on its way back to the free view
    pub fn looking(&self) -> bool {
        self.through.is_some() && !self.returning
    }

    /// a hand takes the view where it is: no jump; the field of view glides back to the free one
    pub fn leave(&mut self) {
        self.touched = true;
        self.returning = false;
        if self.through.take().is_some() {
            self.fov_back = self.ortho.is_none();
        }
    }

    /// whether the view has arrived at the camera it looks through
    pub fn arrived(&self) -> bool {
        self.through.is_some() && self.glide_t >= GLIDE_S
    }

    /// The distance from which every sphere fits the view of `aspect` around `target`, from this side; the farthest the
    /// camera goes if even that is too close.
    pub fn fit_distance(&self, aspect: f32, target: Vec3, spheres: &[(Vec3, f32)], margin: f32) -> f32 {
        let fits = |d: f32| {
            let mut c = *self;
            (c.target, c.distance) = (target, d);
            c.fits(aspect, spheres.iter().copied(), margin)
        };
        let (mut lo, mut hi) = (self.min_distance, self.max_distance);
        if fits(lo) {
            return lo;
        }
        if !fits(hi) {
            return hi;
        }
        for _ in 0..24 {
            let mid = (lo * hi).sqrt();
            if fits(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        hi
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
        match self.ortho {
            Some(h) => {
                let (hh, hw) = (h * 0.5, h * 0.5 * aspect.max(1e-3));
                Mat4::orthographic_rh(-hw, hw, -hh, hh, 0.05, 200.0 + self.distance)
            }
            None => Mat4::perspective_rh(self.fov_y, aspect.max(1e-3), 0.05, 200.0),
        }
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
        if let Some(s) = self.through {
            // a glide eased in and out: it starts and ends at rest, so neither the shot nor a hand meets a jump
            self.glide_t += dt.max(0.0);
            let x = (self.glide_t / GLIDE_S).min(1.0);
            let u = x * x * x * (x * (x * 6.0 - 15.0) + 10.0);
            let f = self.glide_from;
            let to = s.distance.clamp(self.min_distance, self.max_distance);
            self.target = f.target.lerp(s.target, u);
            self.goal = self.target;
            self.yaw = wrap_angle(f.yaw + wrap_pi(s.yaw - f.yaw) * u);
            self.pitch = f.pitch + (s.pitch - f.pitch) * u;
            self.fov_y = f.fov_y + (s.fov_y - f.fov_y) * u;
            self.distance = (f.distance.max(1e-4).ln() + (to.ln() - f.distance.max(1e-4).ln()) * u).exp();
            if x >= 1.0 {
                (self.target, self.goal, self.yaw, self.pitch, self.fov_y, self.distance) = (s.target, s.target, s.yaw, s.pitch, s.fov_y, to);
            }
            self.want = self.distance;
        } else if self.fov_back {
            let k = 1.0 - (-dt.max(0.0) / FOCUS_EASE_S).exp();
            self.fov_y += (self.free_fov - self.fov_y) * k;
            if (self.fov_y - self.free_fov).abs() < 1e-5 {
                (self.fov_y, self.fov_back) = (self.free_fov, false);
            }
        }
        // in the log of the distance, so a step in is as smooth close up as far out
        let k = 1.0 - (-dt.max(0.0) / ZOOM_EASE_S).exp();
        let d = (self.distance.ln() + (self.want.ln() - self.distance.ln()) * k).exp();
        self.distance = if (d / self.want - 1.0).abs() < 1e-4 { self.want } else { d };
        // the focus: a straight glide toward the goal, a share of the way each frame – never past it (#224)
        let k = 1.0 - (-dt.max(0.0) / FOCUS_EASE_S).exp();
        let t = self.target + (self.goal - self.target) * k;
        self.target = if t.distance(self.goal) < 1e-5 { self.goal } else { t };
        if self.returning && self.arrived() {
            (self.through, self.returning) = (None, false);
        }
        self.idle += dt;
        if !self.auto {
            return;
        }
        let ease = ((self.idle - IDLE_S) / 2.0).clamp(0.0, 1.0);
        self.yaw = wrap_angle(self.yaw + AUTO_RAD_S * ease * dt);
    }

    /// a drag by (dx, dy) pixels: right turns the camera to the left around the target, up looks from above
    pub fn drag(&mut self, dx: f32, dy: f32) {
        self.leave();
        self.yaw = wrap_angle(self.yaw - dx * 0.006);
        let (lo, hi) = if self.free { (-POLE, POLE) } else { (PITCH_MIN, PITCH_MAX) };
        self.pitch = (self.pitch + dy * 0.006).clamp(lo, hi);
        self.idle = 0.0;
    }

    /// the camera's right, up and back in the world
    fn basis(&self) -> [Vec3; 3] {
        let v = self.view();
        [Vec3::new(v.x_axis.x, v.y_axis.x, v.z_axis.x), Vec3::new(v.x_axis.y, v.y_axis.y, v.z_axis.y), Vec3::new(v.x_axis.z, v.y_axis.z, v.z_axis.z)]
    }

    /// a drag by (dx, dy) pixels of a view `view_h` pixels high moves the view sideways, the pivot with it: the point at
    /// the pivot's depth stays under the pointer
    pub fn pan(&mut self, dx: f32, dy: f32, view_h: f32) {
        self.leave();
        let [right, up, _] = self.basis();
        let height = self.ortho.unwrap_or_else(|| 2.0 * self.distance * (self.fov_y * 0.5).tan());
        let per_pixel = height / view_h.max(1.0);
        let by = (up * dy - right * dx) * per_pixel;
        self.target += by;
        self.goal += by;
        self.idle = 0.0;
    }

    /// look along a world axis (0 x, 1 y, 2 z) from its plus side, or from its minus side
    pub fn view_along(&mut self, axis: usize, plus: bool) {
        self.leave();
        use std::f32::consts::{FRAC_PI_2, PI};
        let s = if plus { 1.0 } else { -1.0 };
        (self.yaw, self.pitch) = match axis {
            0 => (s * FRAC_PI_2, 0.0),
            1 => (0.0, s * POLE),
            _ => (if plus { 0.0 } else { PI }, 0.0),
        };
        self.idle = 0.0;
    }

    /// the world's x, y and z on the screen: each axis as (right, down, toward the eye), of length 1 in all three
    pub fn axes(&self) -> [(f32, f32, f32); 3] {
        let [right, up, back] = self.basis();
        [Vec3::X, Vec3::Y, Vec3::Z].map(|a| (a.dot(right), -a.dot(up), a.dot(back)))
    }

    /// `steps` > 0 moves in (a wheel notch = 1, 12 % each); the camera glides there in `step`
    pub fn zoom(&mut self, steps: f32) {
        self.leave();
        self.want = (self.want * crate::gesture::ZOOM_STEP.powf(steps)).clamp(self.min_distance, self.max_distance);
        if let Some(h) = self.ortho.as_mut() {
            *h = (*h * crate::gesture::ZOOM_STEP.powf(steps)).clamp(crate::scene_camera::SIZE_RANGE.0, crate::scene_camera::SIZE_RANGE.1);
        }
        self.idle = 0.0;
    }

    /// turn by `rad` (the arrow keys)
    pub fn turn(&mut self, rad: f32) {
        self.leave();
        self.yaw = wrap_angle(self.yaw + rad);
        self.idle = 0.0;
    }

    pub fn reset(&mut self) {
        self.leave();
        (self.yaw, self.pitch, self.distance) = self.home;
        self.want = self.distance;
        self.idle = 0.0;
    }

    /// #224: this view's frame for a w / h of `aspect`
    pub fn frame(&self, aspect: f32) -> Frame {
        let eye = self.eye();
        let fwd = (self.target - eye).try_normalize().unwrap_or(Vec3::NEG_Z);
        let right = fwd.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
        let tan_y = (self.fov_y * 0.5).tan();
        Frame { eye, fwd, right, up: right.cross(fwd), tan_x: tan_y * aspect.max(1e-3), tan_y, min_depth: self.distance * 0.6 }
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

/// an angle as the shorter turn, in −π..π
fn wrap_pi(a: f32) -> f32 {
    let pi = std::f32::consts::PI;
    wrap_angle(a + pi) - pi
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pan_moves_the_pivot_sideways_and_keeps_the_distance() {
        let mut c = Orbit::new(Vec3::ZERO, 0.0, 0.0, 4.0);
        c.pan(100.0, 0.0, 800.0);
        assert!(c.target.x < 0.0, "dragged right, the world follows the hand: {:?}", c.target);
        assert!(c.target.y.abs() < 1e-5 && c.target.z.abs() < 1e-5);
        assert_eq!(c.goal, c.target);
        assert_eq!(c.distance, 4.0);
    }

    #[test]
    fn looking_through_a_camera_glides_there_holds_and_a_hand_leaves_it() {
        let mut c = Orbit::new(Vec3::ZERO, 0.2, 0.1, 4.0);
        c.free = true;
        let shot = Shot { target: Vec3::new(1.0, 0.5, -1.0), yaw: -0.4, pitch: 0.3, distance: 2.5, fov_y: 40f32.to_radians(), ortho: None };
        c.look_through(shot);
        assert!(!c.arrived(), "no jump");
        c.step(1.0 / 60.0);
        assert!(c.yaw < 0.2 || c.yaw > 6.0, "the shorter way round: {}", c.yaw);
        for _ in 0..240 {
            c.step(1.0 / 60.0);
        }
        assert!(c.arrived(), "{c:?}");
        // a pan while looking through leaves the camera, the free field of view glides back
        c.pan(10.0, 0.0, 800.0);
        assert_eq!(c.through, None);
        assert!((c.fov_y - shot.fov_y).abs() < 1e-4, "no jump of the lens");
        for _ in 0..240 {
            c.step(1.0 / 60.0);
        }
        assert_eq!(c.fov_y, FOV_Y);
        c.look_through(shot);
        c.zoom(1.0);
        assert_eq!(c.through, None, "a zoom leaves it too");
        c.look_through(shot);
        c.drag(1.0, 0.0);
        assert_eq!(c.through, None, "and an orbit");
    }

    /// the largest change of the eye (m), the angles (rad) and the lens (rad) from one frame to the next
    fn worst_step(frames: &[Orbit]) -> (f32, f32, f32) {
        frames.windows(2).fold((0.0f32, 0.0f32, 0.0f32), |(e, a, f), w| {
            let angle = wrap_pi(w[1].yaw - w[0].yaw).abs() + (w[1].pitch - w[0].pitch).abs();
            (e.max(w[0].eye().distance(w[1].eye())), a.max(angle), f.max((w[1].fov_y - w[0].fov_y).abs()))
        })
    }

    /// into a shot and back to the free view, each an eased glide; a hand mid-glide takes the view where it is – no
    /// frame jumps in position, angle or lens either way
    #[test]
    fn into_a_shot_and_back_to_free_glide_without_a_jump_even_when_grabbed() {
        let start = |c: &mut Orbit| {
            *c = Orbit::new(Vec3::new(0.0, 1.0, 0.0), 0.3, 0.1, 4.0);
            c.free = true;
        };
        let shot = Shot { target: Vec3::new(1.5, 0.8, -1.0), yaw: 2.6, pitch: 0.5, distance: 1.8, fov_y: 20f32.to_radians(), ortho: None };
        let mut c = Orbit::new(Vec3::ZERO, 0.0, 0.0, 1.0);
        start(&mut c);
        let free = c;
        let run = |c: &mut Orbit, n: usize, frames: &mut Vec<Orbit>| {
            for _ in 0..n {
                c.step(1.0 / 60.0);
                frames.push(*c);
            }
        };
        // in, all the way, then back to free, all the way
        let mut frames = vec![c];
        c.look_through(shot);
        run(&mut c, 240, &mut frames);
        assert!(c.arrived() && c.looking());
        c.go_free();
        run(&mut c, 240, &mut frames);
        assert!(!c.looking() && c.through.is_none(), "free again once back");
        assert!(c.eye().distance(free.eye()) < 1e-3 && (c.fov_y - free.fov_y).abs() < 1e-4, "back where it was: {c:?} vs {free:?}");
        let (e, a, f) = worst_step(&frames);
        assert!(e < 0.4 && a < 0.4 && f < 0.05, "no jump: {e} m, {a} rad, {f} rad");
        assert!(frames.len() > 400 && e > 0.0);
        // grabbed mid-glide, in and on the way back: the frame of the grab is the frame before
        for back in [false, true] {
            start(&mut c);
            let mut frames = vec![c];
            c.look_through(shot);
            run(&mut c, if back { 240 } else { 10 }, &mut frames);
            if back {
                c.go_free();
                run(&mut c, 10, &mut frames);
            }
            let before = c;
            c.drag(0.0, 0.0);
            assert!(c.through.is_none() && !c.looking());
            assert!(c.eye().distance(before.eye()) < 1e-6 && c.fov_y == before.fov_y, "the grab itself moves nothing");
            run(&mut c, 120, &mut frames);
            let (e, a, f) = worst_step(&frames);
            assert!(e < 0.4 && a < 0.4 && f < 0.05, "back {back}: {e} m, {a} rad, {f} rad");
        }
    }

    #[test]
    fn the_fit_distance_grows_with_what_must_fit() {
        let c = Orbit::new(Vec3::ZERO, 0.0, 0.0, 4.0);
        let near = c.fit_distance(1.6, Vec3::ZERO, &[(Vec3::ZERO, 0.3)], 0.1);
        let far = c.fit_distance(1.6, Vec3::ZERO, &[(Vec3::new(-1.0, 0.0, 0.0), 0.3), (Vec3::new(1.0, 0.0, 0.0), 0.3)], 0.1);
        assert!(near < far, "{near} {far}");
        let mut at = c;
        at.distance = far;
        assert!(at.fits(1.6, [(Vec3::new(1.0, 0.0, 0.0), 0.3)], 0.1));
    }

    #[test]
    fn looking_along_an_axis_puts_the_eye_on_it() {
        let mut c = Orbit::new(Vec3::ZERO, 0.3, 0.2, 4.0);
        c.free = true;
        for (axis, plus, want) in [(0, true, Vec3::X), (0, false, -Vec3::X), (2, true, Vec3::Z), (2, false, -Vec3::Z), (1, true, Vec3::Y), (1, false, -Vec3::Y)] {
            c.view_along(axis, plus);
            assert!(c.eye().normalize().distance(want) < 0.02, "{axis} {plus}: {:?}", c.eye());
        }
    }

    #[test]
    fn the_axes_on_the_screen_follow_the_view() {
        let mut c = Orbit::new(Vec3::ZERO, 0.0, 0.0, 4.0);
        c.view_along(2, true);
        let [x, y, z] = c.axes();
        assert!((x.0 - 1.0).abs() < 1e-4, "x points right: {x:?}");
        assert!((y.1 + 1.0).abs() < 1e-4, "y points up the screen: {y:?}");
        assert!((z.2 - 1.0).abs() < 1e-4, "z points at the eye: {z:?}");
    }

    #[test]
    fn only_a_free_camera_reaches_the_poles() {
        let mut c = Orbit::new(Vec3::ZERO, 0.0, 0.0, 4.0);
        c.drag(0.0, 10_000.0);
        assert_eq!(c.pitch, PITCH_MAX);
        c.free = true;
        c.drag(0.0, 10_000.0);
        assert_eq!(c.pitch, POLE);
    }

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
