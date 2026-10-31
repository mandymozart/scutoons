//! The tempo as the space reads it: the clock at this frame's heard moment (deck-stage's tempo clock makes it, a
//! browser page its own). The map, positions and quantize are deck-core's `tempo` module.
pub use deck_core::tempo::{position, Position};

/// The clock at this frame's heard moment.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Now {
    pub bpm: f64,
    pub pos: Position,
    pub per_bar: u32,
    pub running: bool,
}
