//! deck-outputs (#211): every parameter is an output too – the deck drives an external FX unit over MIDI and the
//! owner's strobe over DMX, on the deck's clock (DECISIONS: "as much as possible in the Rust engine incl. hardware
//! communication"; "performance and stability is everything").
//!
//! - `shape`: the mapping math – range, curve, invert, output range, smoothing, 7 / 14 / 8 / 16 bit.
//! - `fixture`: fixture profiles (TOML: channel names → numbers), the generic strobe built in.
//! - `strobe`: the clock-synced strobe – flashes on the tempo grid, a parameter steps the divisions from a neutral 1/4.
//! - `config`: the scene's `[outputs]` → a checked `Plan` (errors name where and what).
//! - `engine`: the plan + the deck's values + the clock → timed MIDI / DMX messages (DMX smoothed at 40 Hz, flashes
//!   sent ahead by the lead). No ports, no threads, no allocation per step.
//! - `host` (feature): the runner that plays the engine's messages through `adapters-core` adapters (the stage's MIDI
//!   out port, the USB-DMX widget) on its own thread.
pub mod config;
pub mod engine;
pub mod fixture;
pub mod shape;
pub mod strobe;
#[cfg(feature = "host")]
pub mod host;

pub use config::{parse, MidiPort, Plan};
pub use engine::{Engine, Out, Timed};
pub use strobe::Clock;
