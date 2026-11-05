//! The live context's probe (#409, T2 of #314; agents/research/RIG-MODULE-GROUPS.md §4): a control-domain module – the
//! shape of a code rig's policy – that writes what `ctx.character()` says, every block:
//! - `index` = the mood's place in its own `moods` + 1 (A1; 0 = a mood it does not list),
//! - `energy` = the being's energy, `touch` = the hand on the deck,
//! - `sad` = 1 while the mood's string is "sad" (a policy that matches strings, the design note's example), else 0.
//!
//! A test module (deck-native tests/modules.rs builds it for wasm32 and plays it in the engine), not a factory module.
#![allow(clippy::duplicated_attributes)]
use deck_module::{Ctx, Module, Outputs};

#[derive(Module)]
#[module(name = "context_probe", domain = "control", doc = "writes the live context it reads", moods = "thinking|sad|happy|laughing|surprised")]
#[port(output, control, name = "index")]
#[port(output, control, name = "energy")]
#[port(output, control, name = "touch")]
#[port(output, control, name = "sad")]
pub struct ContextProbe {}

impl Module for ContextProbe {
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        let c = ctx.character();
        out.set(Self::INDEX, c.mood_index as f32);
        out.set(Self::ENERGY, c.energy);
        out.set(Self::TOUCH, c.touch);
        out.set(Self::SAD, if c.mood == "sad" { 1.0 } else { 0.0 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_module::testing::Rig;
    use deck_module::Describe;

    /// the SDK's strict rig: what the header says is what the outputs say, without an allocation
    #[test]
    fn it_writes_the_context_it_reads() {
        let mut rig = Rig::<ContextProbe>::new(48_000.0, 1);
        rig.set_character("catpope", "", "sad", 2);
        rig.set_senses(0.5, 0.25, 0.0);
        rig.run(0.0, 128);
        let outs: Vec<(f32, bool)> = (0..4).map(|i| rig.control_out(i)).collect();
        assert_eq!(outs, [(2.0, true), (0.5, true), (0.25, true), (1.0, true)]);
        let d = deck_module::host::parse(ContextProbe::DESCRIPTOR).unwrap();
        assert_eq!(d.moods.len(), 5);
    }
}
