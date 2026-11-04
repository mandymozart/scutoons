//! deck-module – the SDK for rig modules (#280, research/RIG-EDITING.md §4). A module is a crate that derives
//! `Module` for a struct and implements the `Module` trait:
//!
//! ```ignore
//! use deck_module::{Block, Ctx, Module};
//!
//! #[derive(Module)]
//! #[module(name = "gain")]
//! #[port(input, audio, name = "in")]
//! #[port(output, audio, name = "out")]
//! struct Gain {
//!     #[param(range = 0.0..2.0, default = 1.0)]
//!     amount: f32,
//! }
//!
//! impl Module for Gain {
//!     fn process(&mut self, block: &mut Block, _: &Ctx) {
//!         let n = block.frames();
//!         let (il, ir) = block.audio_in(0);
//!         let (mut l, mut r) = ([0.0; 1024], [0.0; 1024]); // no allocation in process
//!         for i in 0..n { l[i] = il[i] * self.amount; r[i] = ir[i] * self.amount; }
//!         let (ol, or) = block.audio_out(0);
//!         ol.copy_from_slice(&l[..n]);
//!         or.copy_from_slice(&r[..n]);
//!     }
//! }
//! ```
//!
//! The derive writes the **descriptor** (the parameters with range and default, the ports with their types, the ABI
//! version) as JSON – in a `deck.module` custom section of the wasm, so a host reads it without running the module –
//! and, on wasm32, the ABI's exports (README.md). The host feeds `process` (audio blocks, no allocation) and `control`
//! (control rate); both see a read-only `Ctx` (tempo, character, variables, seed) and write only through the declared
//! outputs. No `build.rs`, no macro of your own: the SDK's derive is the only one.
//!
//! The same code runs natively in a test (`testing::Rig`), which is how a module's output is proven against a built-in.
extern crate self as deck_module;

pub mod abi;
pub mod block;
pub mod ctx;
pub mod cv;
pub mod rt;
pub mod testing;
/// ABI v2 (#340): opt in with `#[module(abi = 2)]`; replaces the v1 items above with #343
pub mod v2;

#[cfg(feature = "host")]
pub mod host;
/// deterministic maths (libm): the same bits native and wasm (ABI-V2 §6)
pub mod math;
/// a one-pole step per block (ABI-V2 §2: the module smooths)
pub mod smooth;

pub use abi::ABI_VERSION;
pub use block::{Block, Event, Outputs};
pub use ctx::{Character, Ctx, Tempo};
pub use deck_module_macros::Module;

/// What the derive writes: the module's name and descriptor, its parameters by index (declaration order).
pub trait Describe: Sized {
    const NAME: &'static str;
    /// the descriptor as JSON (`host::parse` reads it into deck-catalog's `Descriptor`)
    const DESCRIPTOR: &'static str;
    /// the number of `#[param]` fields
    const PARAMS: usize;
    /// a module with every parameter at its default (other fields `Default`)
    fn build() -> Self;
    /// set parameter `index` – clamped to its range; a non-finite value or an unknown index is ignored
    fn set_param(&mut self, index: usize, value: f32);
    fn param(&self, index: usize) -> f32;
}

/// The code a creator writes. All three methods have an empty default.
pub trait Module: Describe + 'static {
    /// once, after `build`: the seed, the rate and the first context are there
    fn init(&mut self, _ctx: &Ctx) {}
    /// one audio block (≤ `abi::MAX_BLOCK` frames): fill the audio outputs, emit events, set control outputs. Runs on
    /// the audio thread: no allocation, no locks.
    fn process(&mut self, _block: &mut Block, _ctx: &Ctx) {}
    /// one control tick (the frame thread, ~60 Hz): set control outputs, emit events
    fn control(&mut self, _ctx: &Ctx, _out: &mut Outputs) {}
}

/// A line to the host's console under the editor – rate-limited by the host, per module. `log!("step {}", k)`.
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => { $crate::rt::log(::core::format_args!($($arg)*)) };
}
