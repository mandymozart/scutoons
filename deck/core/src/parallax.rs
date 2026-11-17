//! Flat pictures that move like far things (deck-catalog `Layer`, a `Background` with a horizon): as the camera turns
//! round its pivot, a picture `depth` metres behind the pivot follows `depth / (distance + depth)` of the turn – the
//! matte all of it, a layer at the pivot none, a layer in front against it; as the pivot pans, a layer shifts by the pan
//! over its distance from the eye. A horizon row sits where the 3D's horizon would be at that share of the pitch.
//! Screen units: NDC, x right, y up, -1…1.

use crate::fit::cover_scale;

/// the camera as the pictures see it
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub yaw: f32,
    /// above the pivot looking down > 0
    pub pitch: f32,
    /// the view the pictures were placed for (the camera's home): a fixed picture keeps it
    pub home_yaw: f32,
    pub home_pitch: f32,
    /// eye to pivot, metres
    pub distance: f32,
    /// the pivot's place along the camera's right and up, metres
    pub pan: (f32, f32),
    /// the vertical field of view, rad
    pub fov_y: f32,
    /// width / height
    pub aspect: f32,
}

/// where a picture lies: its middle across, its top, its half sizes; across it repeats mirrored
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
    pub x: f32,
    pub half_w: f32,
    pub top: f32,
    pub half_h: f32,
}

/// the nearest a front layer may come to the eye, a share of the eye's distance to the pivot
const NEAREST: f32 = 0.2;
/// the steepest a picture's horizon is followed, rad
const STEEP: f32 = 1.4;

/// eye to a layer `depth` metres behind the pivot
fn reach(depth: f32, distance: f32) -> f32 {
    (distance + depth).max(distance * NEAREST).max(1e-3)
}

/// the share of the camera's turn a layer follows; none = the matte, all of it
pub fn follows(depth: Option<f32>, distance: f32) -> f32 {
    depth.map_or(1.0, |d| d / reach(d, distance))
}

/// A layer `depth` metres behind the pivot (none = the matte), its picture `aspect` wide, `horizon` its horizon row.
pub fn layer(aspect: f32, depth: Option<f32>, horizon: Option<f32>, view: &View) -> Quad {
    let pan = depth.map_or(0.0, |d| 1.0 / reach(d, view.distance));
    place(aspect, horizon, follows(depth, view.distance), pan, view)
}

/// The stage's background picture: `parallax` of the turn followed, no pan (it stands for the far distance).
pub fn background(aspect: f32, horizon: Option<f32>, parallax: f32, view: &View) -> Quad {
    place(aspect, horizon, parallax, 0.0, view)
}

fn place(aspect: f32, horizon: Option<f32>, follows: f32, pan: f32, view: &View) -> Quad {
    let a = view.aspect.max(1e-3);
    let k = cover_scale((aspect, 1.0), (a, 1.0));
    let (half_w, half_h) = (aspect * k / a, k);
    let tan_y = (view.fov_y * 0.5).tan().max(1e-3);
    let tan_x = tan_y * a;
    let turn = wrap(view.yaw - view.home_yaw) * follows;
    let pitch = (view.home_pitch + (view.pitch - view.home_pitch) * follows).clamp(-STEEP, STEEP);
    let x = turn / tan_x - view.pan.0 * pan / tan_x;
    let lift = -view.pan.1 * pan / tan_y;
    let top = match horizon {
        Some(h) => pitch.tan() / tan_y + 2.0 * half_h * h,
        None => (pitch.tan() - view.home_pitch.clamp(-STEEP, STEEP).tan()) / tan_y + half_h,
    };
    Quad { x, half_w, top: top + lift, half_h }
}

/// an angle into -π…π
fn wrap(a: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    (a + PI).rem_euclid(TAU) - PI
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> View {
        View { yaw: 0.0, pitch: 0.2, home_yaw: 0.0, home_pitch: 0.2, distance: 4.0, pan: (0.0, 0.0), fov_y: 55f32.to_radians(), aspect: 16.0 / 9.0 }
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn at_home_a_picture_covers_centred() {
        let q = layer(2.0, Some(6.0), None, &home());
        assert!(near(q.x, 0.0) && near(q.half_h, 1.0) && near(q.top, 1.0), "{q:?}");
        assert!(near(q.half_w, 2.0 / (16.0 / 9.0)), "too wide: the height fits, {q:?}");
        let fixed = background(1.0, None, 0.0, &View { yaw: 1.0, pitch: 0.6, pan: (3.0, 1.0), ..home() });
        assert!(near(fixed.x, 0.0) && near(fixed.top, fixed.half_h), "a fixed background never moves, {fixed:?}");
    }

    #[test]
    fn a_turn_moves_the_far_more_than_the_near() {
        let turned = View { yaw: 0.1, ..home() };
        let x = |depth| layer(1.0, depth, None, &turned).x;
        let (matte, hills, trees, pivot, fog) = (x(None), x(Some(20.0)), x(Some(2.0)), x(Some(0.0)), x(Some(-1.0)));
        assert!(matte > hills && hills > trees && trees > pivot && near(pivot, 0.0) && fog < 0.0, "{matte} {hills} {trees} {pivot} {fog}");
        assert!(near(matte, 0.1 / ((55f32.to_radians() * 0.5).tan() * 16.0 / 9.0)), "the matte follows the whole turn");
        assert!(near(follows(Some(4.0), 4.0), 0.5), "as far behind the pivot as the eye is before it: half");
    }

    #[test]
    fn a_pan_moves_the_near_more_than_the_far() {
        let panned = View { pan: (1.0, 0.0), ..home() };
        let x = |depth| layer(1.0, depth, None, &panned).x;
        assert!(near(x(None), 0.0), "the matte stays");
        assert!(x(Some(10.0)) < 0.0 && x(Some(1.0)) < x(Some(10.0)) && x(Some(-2.0)) < x(Some(1.0)), "the pivot went right: the near go left the most");
        assert!(near(background(1.0, None, 1.0, &panned).x, 0.0), "the background stands for the far distance");
        let up = View { pan: (0.0, 1.0), ..home() };
        assert!(layer(1.0, Some(1.0), None, &up).top < layer(1.0, Some(1.0), None, &home()).top, "the pivot went up: the layer down");
    }

    #[test]
    fn the_horizon_row_sits_on_the_3ds_horizon() {
        let tan_y = (55f32.to_radians() * 0.5).tan();
        for pitch in [0.0, 0.3, -0.2] {
            let v = View { pitch, ..home() };
            let q = layer(16.0 / 9.0, None, Some(0.4), &v);
            let row = q.top - 2.0 * q.half_h * 0.4;
            assert!(near(row, pitch.tan() / tan_y), "pitch {pitch}: the row at {row}");
        }
        let down = View { pitch: 0.5, ..home() };
        let near_layer = layer(1.0, Some(1.0), Some(0.5), &down);
        let matte = layer(1.0, None, Some(0.5), &down);
        assert!(matte.top > near_layer.top, "looking down, the matte's horizon rises more than a near layer's");
    }

    #[test]
    fn a_full_circle_is_no_turn() {
        let round = View { yaw: std::f32::consts::TAU - 0.05, ..home() };
        let back = View { yaw: -0.05, ..home() };
        assert!(near(layer(1.0, None, None, &round).x, layer(1.0, None, None, &back).x));
        let close = layer(1.0, Some(-100.0), None, &View { yaw: 0.1, ..home() });
        assert!(close.x.is_finite() && close.x < 0.0, "a layer past the eye stays near it");
    }
}
