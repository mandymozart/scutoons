//! catpope's mood switching as a code rig's policy (#410, T3 of #314; agents/research/RIG-MODULE-GROUPS.md §3, with A1:
//! the mood by its place in the module's own `moods`, not by its string). A control-domain module over catpope's tracks:
//! - `voice_preset` → `voice:@preset`: the voice's preset, an index into its choices (the deck replays the rig with it);
//! - `voice_index` → `voice:voice.index` (set): the voice's FM index – the policy owns it, a preset's value there is dropped;
//! - `drone_gain` → `drone:drone.gain` (set): the drone's level.
//!
//! The table is catpope.toml's moods: thinking / sad → `low`, happy / laughing / surprised → `bright`, laughing and
//! surprised open the index, sad and surprised move the drone. `heat` > 0 opens the index further with the being's energy.
//! A test module (deck-native tests/modules.rs builds it for wasm32 and plays catpope with it), not a factory module.
#![allow(clippy::duplicated_attributes)]
use deck_module::{Ctx, Module, Outputs};

#[derive(Module)]
#[module(name = "catpope_rig", domain = "control", doc = "catpope: the voice's preset and FM index, the drone's level, by mood", moods = "thinking|sad|happy|laughing|surprised")]
#[port(output, control, name = "voice_preset", path = "voice:@preset", choices = "cathedral_voice.base|cathedral_voice.low|cathedral_voice.bright")]
#[port(output, control, name = "voice_index", path = "voice:voice.index", mode = "set")]
#[port(output, control, name = "drone_gain", path = "drone:drone.gain", mode = "set")]
pub struct CatpopeRig {
    #[param(range = 0.0..1.0, default = 0.0, doc = "how far the being's energy opens the FM index (0 = the table alone)")]
    pub heat: f32,
    /// the row last written (None before the first block)
    row: Option<usize>,
}

/// mood_index → (voice preset choice, FM index, drone gain): row 0 = neutral and every mood the list does not name, then
/// the moods in the order of `moods`
pub const MOODS: [(usize, f32, f32); 6] = [(0, 2.0, 0.1), (1, 1.0, 0.1), (1, 1.0, 0.14), (2, 2.0, 0.1), (2, 4.0, 0.1), (2, 6.0, 0.05)];

impl Module for CatpopeRig {
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        let c = ctx.character();
        let row = (c.mood_index as usize).min(MOODS.len() - 1);
        let (preset, index, drone) = MOODS[row];
        if self.row != Some(row) {
            // select on a change only: an output holds its last value
            self.row = Some(row);
            out.set(Self::VOICE_PRESET, preset as f32);
            out.set(Self::DRONE_GAIN, drone);
        }
        // the policy owns voice.index: the table's value, opened by energy when heat > 0 (libm: the same bits everywhere)
        out.set(Self::VOICE_INDEX, index * deck_module::math::exp2(self.heat * c.energy));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_module::testing::Rig;
    use deck_module::Describe;

    /// the SDK's strict rig: each mood's row, the preset written only when it changes, no allocation; the descriptor
    /// says its moods and its three targets
    #[test]
    fn it_writes_the_table_of_the_mood_it_reads() {
        let mut rig = Rig::<CatpopeRig>::new(48_000.0, 1);
        let moods = ["", "thinking", "sad", "happy", "laughing", "surprised"];
        for (i, mood) in moods.iter().enumerate() {
            rig.set_character("catpope", "", mood, i as u32);
            rig.run(i as f64 * 256.0, 128);
            let (preset, index, drone) = MOODS[i];
            assert_eq!([rig.control_out(0), rig.control_out(1), rig.control_out(2)], [(preset as f32, true), (index, true), (drone, true)], "{mood}");
            rig.run(i as f64 * 256.0 + 128.0, 128);
            assert!(!rig.control_out(0).1, "{mood}: the preset is written on a change only");
        }
        let d = deck_module::host::parse(CatpopeRig::DESCRIPTOR).unwrap();
        assert_eq!(d.moods.len(), 5);
        assert_eq!(d.ports.iter().filter(|p| !p.path.is_empty()).count(), 3);
    }
}
