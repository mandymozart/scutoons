//! `dolly` – a starter motion module for a camera's slot (`camera.motion`): the camera pushes in toward its aim and back
//! out, once every `beats` on the tempo clock, on top of whatever shot or move holds it. `zoom` opens the lens as the
//! camera nears (the dolly zoom: the aim keeps its size, the room behind it stretches).
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p dolly-module`.
use deck_module::{math, Ctx, Module, Outputs};

#[derive(Module)]
#[module(name = "dolly", slot = "camera.motion", doc = "the camera pushes in toward its aim and back, on the tempo clock")]
pub struct Dolly {
    #[param(range = 0.0..4.0, default = 0.6, unit = "m", doc = "how far the camera pushes in")]
    depth: f32,
    #[param(range = 1.0..64.0, default = 8.0, unit = "beats", doc = "one push in and back out")]
    beats: f32,
    #[param(range = 0.0..1.0, default = 0.0, doc = "the dolly zoom: the lens opens as the camera nears")]
    zoom: f32,
}

/// the beat now; while the tempo stands, the beat its seconds would make (the motion never freezes)
pub fn beat(ctx: &Ctx) -> f64 {
    let t = ctx.tempo();
    if t.running {
        ctx.beats().0
    } else {
        ctx.frame() / ctx.rate().max(1.0) * t.bpm / 60.0
    }
}

impl Module for Dolly {
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        let (Some(t), Some(w)) = (ctx.transform(), out.transform()) else { return };
        let cycle = beat(ctx) / f64::from(self.beats.max(1.0));
        let phase = (cycle - math::libm::floor(cycle)) as f32;
        let u = 0.5 - 0.5 * math::cos(core::f32::consts::TAU * phase);
        let dir = [t.target[0] - t.position[0], t.target[1] - t.position[1], t.target[2] - t.position[2]];
        let dist = math::sqrt(dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]);
        if dist < 1e-4 {
            return;
        }
        let step = (self.depth * u).min(dist * 0.9);
        w.offset_position(dir.map(|v| v / dist * step));
        if self.zoom > 0.0 && t.projection == 0 {
            w.offset_focal(-t.focal * step / dist * self.zoom);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_module::motion::{part, KIND_CAMERA};
    use deck_module::testing::Rig;
    use deck_module::{Describe, TransformIn};

    fn camera() -> TransformIn {
        TransformIn { kind: KIND_CAMERA, position: [0.0, 1.6, 4.0], target: [0.0, 1.6, 0.0], focal: 30.0, ..TransformIn::ZERO }
    }

    /// the offset along the view at beat `b` (120 bpm: half a second a beat)
    fn at(rig: &mut Rig<Dolly>, b: f64) -> deck_module::TransformOut {
        rig.set_transform(&camera());
        rig.run(b * 0.5 * 48_000.0, 0);
        rig.transform_out().unwrap()
    }

    #[test]
    fn it_pushes_in_toward_the_aim_and_back_over_its_beats() {
        let mut rig = Rig::<Dolly>::new(48_000.0, 1);
        let start = at(&mut rig, 0.0);
        assert!(start.wrote(part::POSITION) && !start.sets(part::POSITION), "an offset on top of the shot");
        assert!(start.position[2].abs() < 1e-6);
        let mid = at(&mut rig, 4.0);
        assert!((mid.position[2] + 0.6).abs() < 1e-5 && mid.position[0].abs() < 1e-6, "{:?}", mid.position);
        assert!(at(&mut rig, 8.0).position[2].abs() < 1e-5, "back out after 8 beats");
        assert!(!mid.wrote(part::FOCAL), "no zoom by default");
        rig.set_param(Dolly::ZOOM, 1.0);
        let zoomed = at(&mut rig, 4.0);
        assert!((zoomed.focal + 30.0 * 0.6 / 4.0).abs() < 1e-4, "{}", zoomed.focal);
    }

    #[test]
    fn its_descriptor_names_the_cameras_motion_slot() {
        let d = deck_module::host::parse(Dolly::DESCRIPTOR).unwrap();
        assert_eq!(d.slot.as_deref(), Some("camera.motion"));
        assert!(Dolly::SHAPE.motion && d.ports.is_empty());
        assert_eq!(deck_module::Shape::of(&d).unwrap(), Dolly::SHAPE);
    }
}
