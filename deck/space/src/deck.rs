//! A deck's state without a stage, a window or a sound device (#367, #368): what a platform's deck holds around the
//! engine and the 3D – its mood (mood.rs `Ladder`). The browser deck has it as space-web's `DeckState` (no GPU: it works
//! when the 3D can't), the headless host drives it directly. The page passes its inputs each frame and draws the answers.
use crate::mood::Ladder;

/// The deck: its mood ladder and the mood it last showed.
#[derive(Debug, Clone, Default)]
pub struct Deck {
    pub ladder: Ladder,
    /// the mood last handed out by `step` ("" before the first frame)
    shown: String,
}

impl Deck {
    pub fn new() -> Deck {
        Deck::default()
    }

    /// One frame: the sound's level (0..1), the clock's tempo and whether it runs, a hand on the deck (0..1), `dt` s →
    /// the mood when it is another than the last frame's (the ladder's turn, a pinned mood, another character's ladder).
    pub fn step(&mut self, level: f32, bpm: f32, running: bool, touch: f32, dt: f32) -> Option<&str> {
        let mood = self.ladder.step(level, bpm, running, touch, dt);
        if mood == self.shown {
            return None;
        }
        self.shown = mood.to_string();
        Some(&self.shown)
    }

    /// the mood now (the ladder's or the pinned one)
    pub fn mood(&self) -> &str {
        self.ladder.mood()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mood::MOODS;

    #[test]
    fn a_step_says_the_mood_only_when_it_turned() {
        let mut d = Deck::new();
        d.ladder.set_portraits(&MOODS.map(String::from));
        assert_eq!(d.step(0.0, 0.0, false, 0.0, 1.0 / 60.0), Some("neutral"), "the first frame says it");
        assert_eq!(d.step(0.0, 0.0, false, 0.0, 1.0 / 60.0), None);
        d.ladder.pin(Some("happy"));
        assert_eq!(d.step(0.0, 0.0, false, 0.0, 1.0 / 60.0), Some("happy"));
        d.ladder.pin(None);
        assert_eq!(d.step(0.0, 0.0, false, 0.0, 1.0 / 60.0), Some("neutral"));
    }
}
