//! A motion slot's module run by the module host (deck-space space/motion.rs `MotionRun`): the stage's clock as the
//! module's frame and tempo, its entity in, its transform out – one call a drawn frame. `opener` is what a deck with a
//! module library installs.
use deck_core::tempo::TempoMap;
use deck_module::{TransformIn, TransformOut};
use deck_module_host::{Backend, Instance, Loaded, Mode, Setup};
use deck_space::space::dop::Clock;
use deck_space::space::motion::{MotionRun, Opener};
use std::sync::Arc;

/// the rate a motion module's frames count at (its calls follow the drawn frames, not the audio)
pub const RATE: f64 = 48_000.0;

/// a motion module's instance on the stage
pub struct Run<B: Backend>(pub Instance<B>);

impl<B: Backend + Send + 'static> MotionRun for Run<B> {
    fn step(&mut self, clock: &Clock, input: &TransformIn) -> Option<TransformOut> {
        let frame = clock.secs * RATE;
        let m = TempoMap { frame, beat: clock.beat, bpm: clock.bpm, rate: RATE, running: clock.running, per_bar: clock.per_bar };
        let i = &mut self.0;
        i.set_tempo(&m);
        i.set_transform(input);
        i.run(frame, 0).then(|| i.transform_out()).flatten()
    }
}

/// An `Opener` over a module library: `find` gives a loaded module by id; it must fit the slot (deck-catalog
/// `slot::fits`), then it is instantiated in a show's mode (a trap restarts it) with its parameters.
pub fn opener<B: Backend + Send + 'static>(find: impl Fn(&str) -> Option<Arc<Loaded<B>>> + Send + Sync + 'static) -> Box<Opener> {
    Box::new(move |id, params, slot| {
        let loaded = find(id).ok_or_else(|| format!("no module '{id}' on this deck"))?;
        deck_catalog::slot::fits(slot.entity, slot.domain.word(), &loaded.desc, params)?;
        let mut inst = Instance::new(&loaded, Setup::new(RATE, 1).mode(Mode::Show)).map_err(|e| format!("{e:#}"))?;
        for (k, v) in params {
            if let Some(i) = loaded.param_index(k) {
                inst.set_param(i, *v);
            }
        }
        Ok(Box::new(Run(inst)) as Box<dyn MotionRun>)
    })
}
