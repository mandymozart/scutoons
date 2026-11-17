//! Each move as a path from the view it starts on to the shot it ends on: `pose` at progress 0 is the start, at 1 exactly
//! the shot. Paths come in families – around the aim (glide, orbit, spiral, zoom), a straight dolly (push, pull), the head
//! turning where the camera stands (truck, tilt, whip), a curve through a control point (in, crane), the trolley track
//! (fly) – and shake adds unrest that is gone at both ends.
use super::track::Path;
use super::{wrap_pi, View};
use crate::scene_camera::{focal_of, Lens, MoveKind, MoveSpec, Projection, Side, Turn};
use glam::Vec3;
use std::f32::consts::{PI, TAU};

/// what a move may need beside its two views
pub struct Ctx<'a> {
    pub track: Option<&'a Path>,
    /// the show's time (s): shake's unrest
    pub secs: f64,
}

fn lerp(a: f32, b: f32, u: f32) -> f32 {
    a + (b - a) * u
}

fn log_lerp(a: f32, b: f32, u: f32) -> f32 {
    (a.max(1e-4).ln() + (b.max(1e-4).ln() - a.max(1e-4).ln()) * u).exp()
}

fn smooth(u: f32) -> f32 {
    u * u * (3.0 - 2.0 * u)
}

/// the lenses between two: focal length and view height in their logs; the projection changes halfway
pub fn lens(a: &Lens, b: &Lens, u: f32) -> Lens {
    Lens { projection: if u < 0.5 { a.projection } else { b.projection }, focal: log_lerp(a.focal, b.focal, u), size: log_lerp(a.size, b.size, u) }
}

/// around the aim: the aim straight, the angles the shorter way plus `extra` turns, the distance in its log
fn around(f: &View, t: &View, u: f32, extra: f32) -> View {
    View {
        target: f.target.lerp(t.target, u),
        yaw: f.yaw + (wrap_pi(t.yaw - f.yaw) + extra) * u,
        pitch: lerp(f.pitch, t.pitch, u),
        distance: log_lerp(f.distance, t.distance, u),
        lens: lens(&f.lens, &t.lens, u),
    }
}

/// the head turns where the camera stands: the eye straight, the angles the shorter way, the aim in front of it
fn pivot(f: &View, t: &View, u: f32) -> View {
    let eye = f.eye().lerp(t.eye(), u);
    let (yaw, pitch, distance) = (f.yaw + wrap_pi(t.yaw - f.yaw) * u, lerp(f.pitch, t.pitch, u), log_lerp(f.distance, t.distance, u));
    View { target: eye - super::back(yaw, pitch) * distance, yaw, pitch, distance, lens: lens(&f.lens, &t.lens, u) }
}

/// a dolly on a straight rail: eye and aim straight
fn dolly(f: &View, t: &View, u: f32) -> View {
    View::from_eye(f.eye().lerp(t.eye(), u), f.target.lerp(t.target, u), lens(&f.lens, &t.lens, u))
}

/// the eye along a curve through `c1`, `c2`, the aim straight
fn curve(f: &View, t: &View, u: f32, c1: Vec3, c2: Vec3) -> View {
    let eye = super::track::bezier(&[f.eye(), c1, c2, t.eye()], u);
    View::from_eye(eye, f.target.lerp(t.target, u), lens(&f.lens, &t.lens, u))
}

fn dolly_zoom(f: &View, t: &View, u: f32) -> View {
    if f.lens.projection != Projection::Perspective || t.lens.projection != Projection::Perspective {
        return around(f, t, u, 0.0);
    }
    let width = |v: &View| 2.0 * v.distance * (v.lens.fov_y() * 0.5).tan();
    let mut v = around(f, t, u, 0.0);
    let w = log_lerp(width(f), width(t), u);
    v.lens.focal = focal_of(2.0 * (w / (2.0 * v.distance)).atan());
    v
}

fn fly(f: &View, t: &View, u: f32, path: &Path) -> View {
    let eye = path.at(u) + (f.eye() - path.at(0.0)) * (1.0 - u).powi(3) + (t.eye() - path.at(1.0)) * u.powi(3);
    let ahead = |v: &View| (v.target - v.eye()).try_normalize().unwrap_or(Vec3::NEG_Z);
    let (a, b, c) = ((1.0 - u) * (1.0 - u), 2.0 * u * (1.0 - u), u * u);
    let look = (ahead(f) * a + path.tangent(u) * b + ahead(t) * c).try_normalize().unwrap_or_else(|| ahead(t));
    let distance = log_lerp(f.distance, t.distance, u);
    View::from_eye(eye, eye + look * distance, lens(&f.lens, &t.lens, u))
}

/// unrest in −1..1, smooth, the same at the same time
fn unrest(secs: f64, seed: f64) -> f32 {
    let s = secs + seed * 17.3;
    ((s * TAU as f64 * 2.1).sin() + 0.6 * (s * TAU as f64 * 3.7 + 1.3).sin() + 0.3 * (s * TAU as f64 * 7.9 + 2.9).sin()) as f32 / 1.9
}

/// The view `lin` (0..1, linear in time) of the way from `f` to `t`; at 1 exactly `t`.
pub fn pose(spec: &MoveSpec, f: &View, t: &View, lin: f32, ctx: &Ctx) -> View {
    if lin >= 1.0 || spec.kind == MoveKind::Cut {
        return *t;
    }
    let lin = lin.max(0.0);
    let u = spec.ease.at(lin);
    let turns = |rounds: f32| rounds * TAU * if spec.turn == Turn::Left { 1.0 } else { -1.0 };
    let v = match spec.kind {
        MoveKind::Cut => *t,
        MoveKind::Glide | MoveKind::Follow => around(f, t, u, 0.0),
        MoveKind::Orbit => around(f, t, u, turns(spec.rounds)),
        MoveKind::Spiral => {
            let mut v = around(f, t, u, turns(spec.rounds));
            v.distance = log_lerp(f.distance, t.distance, u * u);
            v
        }
        MoveKind::Zoom => {
            let mut v = around(f, t, smooth((lin * 2.0).min(1.0)), 0.0);
            v.lens = lens(&f.lens, &t.lens, u);
            v
        }
        MoveKind::DollyZoom => dolly_zoom(f, t, u),
        MoveKind::Push | MoveKind::Pull => dolly(f, t, u),
        MoveKind::Truck | MoveKind::Tilt | MoveKind::Whip => pivot(f, t, u),
        MoveKind::In => {
            let fwd = (t.target - t.eye()).try_normalize().unwrap_or(Vec3::NEG_Z);
            let right = fwd.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
            let reach = t.distance.max(1.0) * 1.5;
            let side = if spec.side == Side::Left { -1.0 } else { 1.0 };
            let c = t.eye() + right * side * reach - fwd * reach * 0.5;
            curve(f, t, u, f.eye().lerp(c, 0.66), c)
        }
        MoveKind::CraneDown | MoveKind::CraneUp => {
            let (fe, te) = (f.eye(), t.eye());
            let y = if spec.kind == MoveKind::CraneDown { fe.y.max(te.y) + spec.height } else { fe.y.min(te.y) - spec.height };
            curve(f, t, u, Vec3::new(fe.x, y, fe.z), Vec3::new(te.x, y, te.z))
        }
        MoveKind::Fly => match ctx.track.filter(|p| !p.is_empty()) {
            Some(p) => fly(f, t, u, p),
            None => around(f, t, u, 0.0),
        },
        MoveKind::Shake => {
            let mut v = around(f, t, u, 0.0);
            let env = (lin * PI).sin().sqrt();
            let a = spec.amount.to_radians() * env;
            v.yaw += a * unrest(ctx.secs, 0.0);
            v.pitch += a * 0.7 * unrest(ctx.secs, 1.0);
            v
        }
    };
    blended(v, f, t, u).wrapped()
}

/// Between a perspective and an orthographic end the lens blends as a dolly zoom (camera.rs `projection_blend`): the eye
/// on the way backs off along its line of sight while the lens narrows, the aim and the angles stay the move's.
fn blended(mut v: View, f: &View, t: &View, u: f32) -> View {
    let end = |w: &View| (w.lens.fov_y(), w.lens.ortho().then_some(w.lens.size), w.distance);
    if let Some((fov, distance)) = crate::space::camera::projection_blend(end(f), end(t), u) {
        v.lens = Lens { projection: Projection::Perspective, focal: focal_of(fov), size: v.lens.size };
        v.distance = distance;
    }
    v
}
