//! `bob` – a starter motion module for an item's slot (`item.motion`): the item floats up and down on the tempo clock,
//! tilting a little as it rises and stretching at the top, on top of where its spot puts it.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p bob-module`.
use deck_module::{math, Ctx, Module, Outputs};

#[derive(Module)]
#[module(name = "bob", slot = "item.motion", doc = "the item floats up and down on the tempo clock")]
pub struct Bob {
    #[param(range = 0.0..1.0, default = 0.08, unit = "m", doc = "how high it floats")]
    height: f32,
    #[param(range = 0.25..16.0, default = 2.0, unit = "beats", doc = "one rise and fall")]
    beats: f32,
    #[param(range = 0.0..30.0, default = 6.0, unit = "°", doc = "how far it tilts")]
    tilt: f32,
    #[param(range = 0.0..0.3, default = 0.04, doc = "how much it stretches at the top")]
    stretch: f32,
}

/// the beat now; while the tempo stands, the beat its seconds would make (the motion never freezes)
fn beat(ctx: &Ctx) -> f64 {
    let t = ctx.tempo();
    if t.running {
        ctx.beats().0
    } else {
        ctx.frame() / ctx.rate().max(1.0) * t.bpm / 60.0
    }
}

impl Module for Bob {
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        let Some(w) = out.transform() else { return };
        let cycle = beat(ctx) / f64::from(self.beats.max(0.25));
        let phase = (cycle - math::libm::floor(cycle)) as f32;
        let a = core::f32::consts::TAU * phase;
        let up = 0.5 - 0.5 * math::cos(a);
        w.offset_position([0.0, self.height * up, 0.0]);
        w.offset_rotation([0.0, 0.0, self.tilt.to_radians() * math::sin(a)]);
        let s = 1.0 + self.stretch * up;
        w.scale_by([1.0 / math::sqrt(s), s, 1.0 / math::sqrt(s)]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_module::motion::{part, KIND_ENTITY};
    use deck_module::testing::Rig;
    use deck_module::{Describe, TransformIn};

    fn at(rig: &mut Rig<Bob>, b: f64) -> deck_module::TransformOut {
        rig.set_transform(&TransformIn { kind: KIND_ENTITY, position: [1.0, 0.0, 0.0], ..TransformIn::ZERO });
        rig.run(b * 0.5 * 48_000.0, 0);
        rig.transform_out().unwrap()
    }

    #[test]
    fn it_floats_up_and_down_over_its_beats() {
        let mut rig = Rig::<Bob>::new(48_000.0, 1);
        let low = at(&mut rig, 0.0);
        assert_eq!(low.written, part::POSITION | part::ROTATION | part::SCALE);
        assert_eq!(low.absolute, 0, "offsets on top of its spot");
        assert!(low.position[1].abs() < 1e-6);
        let top = at(&mut rig, 1.0);
        assert!((top.position[1] - 0.08).abs() < 1e-6, "{:?}", top.position);
        assert!((top.scale[1] - 1.04).abs() < 1e-6 && top.scale[0] < 1.0, "it stretches at the top");
        let rising = at(&mut rig, 0.5);
        assert!((rising.rotation[2] - 6f32.to_radians()).abs() < 1e-5, "{:?}", rising.rotation);
        assert!(at(&mut rig, 2.0).position[1].abs() < 1e-5, "down again after 2 beats");
    }

    #[test]
    fn its_descriptor_names_the_items_motion_slot() {
        let d = deck_module::host::parse(Bob::DESCRIPTOR).unwrap();
        assert_eq!(d.slot.as_deref(), Some("item.motion"));
        assert_eq!(deck_module::Shape::of(&d).unwrap(), Bob::SHAPE);
    }
}
