//! The tempo map and its hand-over across the stage's threads: all deck-core's `tempo` module (#181, #185, #308) -
//! kept as a path for the stage's own code.
pub use deck_core::tempo::{clamp_bpm, fit, position, BeatReader, Position, Quantize, Shared, Tap, TempoMap, MAX_BPM, MIN_BPM};
