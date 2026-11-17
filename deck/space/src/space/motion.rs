//! Motion modules on a stage (research/MODULE-EXTENSIONS.md §3): what a camera's or an item's `motion` slot holds runs
//! once a drawn frame, after the shot, the move or the spot put the entity where it is (its base); what the module
//! writes goes on top of that base – an offset by default (a shake, a bob, a dolly: the moves and the cuts go on under
//! it), or, for a part it sets (`absolute`), in place of the base while it is attached. Only the parts its slot writes
//! (deck-catalog `slot`) are taken. The deck's module library opens the modules (`install`); a deck without one plays
//! the entity's base and says why.
use super::dop::Clock;
use deck_catalog::slot::{self, Attach, Entity, Slot};
use deck_module::motion::part;
use deck_module::{TransformIn, TransformOut};
use glam::{Quat, Vec3};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// One running motion module, whatever runs it (deck-engine: the module host's instance).
pub trait MotionRun: Send {
    /// one call at `clock` (its seconds and its tempo) with `input`; None = it did not run (silenced, restarting): the
    /// entity keeps its base
    fn step(&mut self, clock: &Clock, input: &TransformIn) -> Option<TransformOut>;
}

/// Opens module `id` with `params` for `slot` (`camera.motion`): checked against the slot, instantiated. Err = why not.
pub type Opener = dyn Fn(&str, &BTreeMap<String, f32>, &Slot) -> Result<Box<dyn MotionRun>, String> + Send + Sync;

static OPENER: OnceLock<Box<Opener>> = OnceLock::new();

/// The process's module library for motion slots (a deck with a module runtime installs it once, at start).
pub fn install(opener: Box<Opener>) {
    let _ = OPENER.set(opener);
}

// TODO(#290): the browser deck has no module runtime yet
const NO_RUNTIME: &str = "this deck runs no modules yet";

/// What a motion module moves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// the scene's primary camera
    Camera,
    /// a holder's member on the stage
    Holder(String),
}

impl Target {
    fn entity(&self) -> Entity {
        match self {
            Target::Camera => Entity::Camera,
            Target::Holder(_) => Entity::Item,
        }
    }
}

struct Running {
    target: Target,
    attach: Attach,
    slot: &'static Slot,
    run: Result<Box<dyn MotionRun>, String>,
}

/// The stage's motion modules: one per attached slot.
#[derive(Default)]
pub struct Motions {
    running: Vec<Running>,
}

impl std::fmt::Debug for Motions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.running.iter().map(|r| (&r.target, &r.attach.module))).finish()
    }
}

impl Motions {
    /// The attachments to run: one kept as it runs when its target and attachment are the same, the rest opened anew.
    pub fn set(&mut self, list: Vec<(Target, Attach)>) {
        let mut old = std::mem::take(&mut self.running);
        for (target, attach) in list {
            if let Some(i) = old.iter().position(|r| r.target == target && r.attach == attach) {
                self.running.push(old.swap_remove(i));
                continue;
            }
            let Some(slot) = slot::of(target.entity()).find(|s| s.domain.word() == attach.slot && s.built) else { continue };
            let run = match OPENER.get() {
                Some(open) => open(&attach.module, &attach.params, slot),
                None => Err(NO_RUNTIME.to_string()),
            };
            if let Err(e) = &run {
                tracing::warn!("{}: module '{}' does not run – {e}", slot.name(), attach.module);
            }
            self.running.push(Running { target, attach, slot, run });
        }
    }

    pub fn is_empty(&self) -> bool {
        self.running.is_empty()
    }

    pub fn has(&self, target: &Target) -> bool {
        self.running.iter().any(|r| &r.target == target)
    }

    pub fn has_holder(&self, holder: &str) -> bool {
        self.running.iter().any(|r| matches!(&r.target, Target::Holder(h) if h == holder))
    }

    /// `step` for a holder's member, by its holder's id
    pub fn step_holder(&mut self, holder: &str, clock: &Clock, input: &TransformIn) -> Option<(TransformOut, &'static Slot)> {
        let r = self.running.iter_mut().find(|r| matches!(&r.target, Target::Holder(h) if h == holder))?;
        let slot = r.slot;
        r.run.as_mut().ok()?.step(clock, input).map(|o| (o, slot))
    }

    /// what `target`'s module wrote this call and its slot; None = none attached or it did not run
    pub fn step(&mut self, target: &Target, clock: &Clock, input: &TransformIn) -> Option<(TransformOut, &'static Slot)> {
        let r = self.running.iter_mut().find(|r| &r.target == target)?;
        let slot = r.slot;
        r.run.as_mut().ok()?.step(clock, input).map(|o| (o, slot))
    }

    /// why an attached module does not run, by target
    pub fn trouble(&self, target: &Target) -> Option<&str> {
        self.running.iter().find(|r| &r.target == target).and_then(|r| r.run.as_ref().err()).map(String::as_str)
    }
}

/// a part the slot lets the module write, as it wrote it: (set instead of offset)
fn took(out: &TransformOut, slot: &Slot, bit: u32) -> Option<bool> {
    let word = part::WORDS.iter().find(|(_, b)| *b == bit)?.0;
    (out.wrote(bit) && slot.writes(word)).then(|| out.sets(bit))
}

fn finite(v: [f32; 3]) -> Option<Vec3> {
    v.iter().all(|x| x.is_finite()).then(|| Vec3::from_array(v))
}

/// An entity on the stage: where it stands (m), its turn (rad, x y z), its scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Pose {
    pub fn at(position: Vec3) -> Pose {
        Pose { position, rotation: Vec3::ZERO, scale: Vec3::ONE }
    }

    /// the turn as a rotation (x, then y, then z)
    pub fn quat(&self) -> Quat {
        Quat::from_euler(glam::EulerRot::ZYX, self.rotation.z, self.rotation.y, self.rotation.x)
    }
}

/// most a module may scale an entity, either way
const SCALE: (f32, f32) = (0.01, 100.0);

/// `base` with what the module wrote on top
pub fn entity(base: Pose, out: &TransformOut, slot: &Slot) -> Pose {
    let mut p = base;
    if let (Some(set), Some(v)) = (took(out, slot, part::POSITION), finite(out.position)) {
        p.position = if set { v } else { base.position + v };
    }
    if let (Some(set), Some(v)) = (took(out, slot, part::ROTATION), finite(out.rotation)) {
        p.rotation = if set { v } else { base.rotation + v };
    }
    if let (Some(set), Some(v)) = (took(out, slot, part::SCALE), finite(out.scale)) {
        p.scale = (if set { v } else { base.scale * v }).clamp(Vec3::splat(SCALE.0), Vec3::splat(SCALE.1));
    }
    p
}

/// A camera as a motion module sees and moves it: its eye and aim (m), its focal length (mm), an orthographic view's
/// height (m).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lensed {
    pub eye: Vec3,
    pub target: Vec3,
    pub focal: f32,
    pub ortho: Option<f32>,
}

/// `base` with what the module wrote on top: its eye and aim moved or set, its focal length and projection inside the
/// lens's ranges
pub fn camera(base: Lensed, out: &TransformOut, slot: &Slot) -> Lensed {
    use crate::scene_camera::{FOCAL_RANGE, SIZE_RANGE};
    let mut c = base;
    if let (Some(set), Some(v)) = (took(out, slot, part::POSITION), finite(out.position)) {
        c.eye = if set { v } else { base.eye + v };
    }
    if let (Some(set), Some(v)) = (took(out, slot, part::TARGET), finite(out.target)) {
        c.target = if set { v } else { base.target + v };
    }
    if c.eye.distance(c.target) < 1e-3 {
        c.eye = base.eye;
        c.target = base.target;
    }
    if let Some(set) = took(out, slot, part::FOCAL).filter(|_| out.focal.is_finite()) {
        c.focal = (if set { out.focal } else { base.focal + out.focal }).clamp(FOCAL_RANGE.0, FOCAL_RANGE.1);
    }
    if took(out, slot, part::PROJECTION).is_some() {
        c.ortho = (out.projection == 1).then(|| if out.size.is_finite() { out.size.clamp(SIZE_RANGE.0, SIZE_RANGE.1) } else { SIZE_RANGE.0 });
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(name: &str) -> &'static Slot {
        slot::find(name).unwrap()
    }

    #[test]
    fn an_offset_goes_on_top_of_the_base_and_a_set_part_replaces_it() {
        let base = Pose { position: Vec3::new(1.0, 0.0, 2.0), rotation: Vec3::new(0.0, 0.5, 0.0), scale: Vec3::splat(2.0) };
        let mut out = TransformOut::ZERO;
        assert_eq!(entity(base, &out, slot("item.motion")), base, "nothing written: the base");
        out.offset_position([0.0, 0.1, 0.0]);
        out.offset_rotation([0.0, 0.0, 0.2]);
        out.scale_by([1.0, 1.5, 1.0]);
        let p = entity(base, &out, slot("item.motion"));
        assert_eq!((p.position, p.rotation, p.scale), (Vec3::new(1.0, 0.1, 2.0), Vec3::new(0.0, 0.5, 0.2), Vec3::new(2.0, 3.0, 2.0)));
        out.set_position([5.0, 5.0, 5.0]);
        out.scale_by([0.0, f32::NAN, 1.0]);
        let p = entity(base, &out, slot("item.motion"));
        assert_eq!(p.position, Vec3::splat(5.0), "driven while attached");
        assert_eq!(p.scale, base.scale, "a non-finite write is left out");
    }

    #[test]
    fn a_camera_takes_only_what_its_slot_writes_inside_the_lens_ranges() {
        let base = Lensed { eye: Vec3::new(0.0, 1.6, 4.0), target: Vec3::new(0.0, 1.6, 0.0), focal: 30.0, ortho: None };
        let mut out = TransformOut::ZERO;
        out.offset_position([0.0, 0.0, -1.0]);
        out.offset_focal(-29.0);
        out.offset_target([0.2, 0.0, 0.0]);
        let c = camera(base, &out, slot("camera.motion"));
        assert_eq!((c.eye, c.target, c.focal), (Vec3::new(0.0, 1.6, 3.0), Vec3::new(0.2, 1.6, 0.0), 4.0), "the focal clamped to the lens's range");
        out.set_projection(Some(6.0));
        assert_eq!(camera(base, &out, slot("camera.motion")).ortho, Some(6.0));
        out.set_position([0.0, 1.6, 0.0]);
        out.set_target([0.0, 1.6, 0.0]);
        let c = camera(base, &out, slot("camera.motion"));
        assert_eq!((c.eye, c.target), (base.eye, base.target), "an eye on its aim sees nothing: the base");

        let mut rot = TransformOut::ZERO;
        rot.offset_rotation([0.0, 1.0, 0.0]);
        rot.offset_focal(10.0);
        let p = entity(Pose::at(Vec3::ZERO), &rot, slot("item.motion"));
        assert_eq!(p.rotation, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(camera(base, &rot, slot("camera.motion")).focal, 40.0);
        let mut scale = TransformOut::ZERO;
        scale.scale_by([3.0; 3]);
        assert_eq!(camera(base, &scale, slot("camera.motion")), base, "a camera has no scale");
    }

    struct Fixed(TransformOut);

    impl MotionRun for Fixed {
        fn step(&mut self, _: &Clock, _: &TransformIn) -> Option<TransformOut> {
            Some(self.0)
        }
    }

    #[test]
    fn attachments_are_kept_while_the_same_and_say_why_one_does_not_run() {
        let mut m = Motions::default();
        m.set(vec![(Target::Camera, Attach::new("motion", "dolly")), (Target::Holder("left".into()), Attach::new("picture", "x"))]);
        assert!(m.has(&Target::Camera) && !m.has(&Target::Holder("left".into())), "a slot not built is not run");
        assert!(m.trouble(&Target::Camera).is_some(), "no library installed in this test");
        m.running[0].run = Ok(Box::new(Fixed(TransformOut::ZERO)));
        m.set(vec![(Target::Camera, Attach::new("motion", "dolly"))]);
        let clock = Clock { secs: 0.0, beat: 0.0, bpm: 120.0, per_bar: 4, running: true };
        assert!(m.step(&Target::Camera, &clock, &TransformIn::ZERO).is_some(), "kept running");
        m.set(Vec::new());
        assert!(m.is_empty());
    }
}
