//! deck-module – the SDK for rig modules (#280, module ABI v2 #323, research/ABI-V2.md). A module is a crate that
//! derives `Module` for a struct and implements the `Module` trait:
//!
//! ```ignore
//! use deck_module::{Block, Ctx, Module};
//!
//! #[derive(Module)]
//! #[module(name = "gain")]
//! #[port(input, audio, name = "in", channels = "any")]
//! #[port(output, audio, name = "out", channels = "any")]
//! struct Gain {
//!     #[param(range = 0.0..2.0, default = 1.0, unit = "x", doc = "the gain")]
//!     amount: f32,
//! }
//!
//! impl Module for Gain {
//!     fn process(&mut self, block: &mut Block, _: &Ctx) {
//!         let inp = block.audio_in(0);
//!         let mut out = block.audio_out(0);
//!         for c in 0..out.ch() {
//!             for (o, i) in out.chan(c).iter_mut().zip(inp.chan(c)) { *o = i * self.amount; }
//!         }
//!     }
//! }
//! ```
//!
//! The derive writes the **descriptor** (deck-catalog's one `Descriptor` as JSON, `abi: 2`) into a `deck.module`
//! custom section of the wasm – a host reads it without running the module – and the module's **memory layout**,
//! derived from it (`layout`: one `inp` block the host writes and one `out` block it reads per call), as constants,
//! with the exports on wasm32. Inside: the frozen headers (`hdr`), the 40-byte event record (`ev`), planar audio and
//! typed views (`io`), the parameter table (`params`), `Ctx`, `Block` / `Outputs`, the counting allocator (`alloc`),
//! the runtime the exports call (`rt`) and the native harness (`testing::Rig`) – the same code a module's tests run.
//! No `build.rs`, no macro of your own: the SDK's derive is the only one. deck/module/README.md is the guide.
extern crate self as deck_module;

pub mod alloc;
pub mod block;
pub mod ctx;
pub mod cv;
pub mod ev;
pub mod hdr;
pub mod io;
pub mod layout;
pub mod params;
pub mod rt;
pub mod testing;

#[cfg(feature = "host")]
pub mod host;
/// deterministic maths (libm): the same bits native and wasm (ABI-V2 §6)
pub mod math;
/// a one-pole step per block (ABI-V2 §2: the module smooths)
pub mod smooth;

pub use block::{Block, Outputs};
pub use ctx::{Asset, AssetSlot, CameraMotion, Character, Ctx, Face, Motion, Position, Status, Tempo, Var};
pub use deck_module_macros::Module;
pub use ev::{Ev, Expr, Kind};
pub use hdr::{AssetInfo, AssetKind, InHdr, OutHdr};
pub use io::{InpMut, Planar, PlanarMut};
pub use layout::{Layout, Sensor, Shape, ABI_VERSION, QUANTUM};
pub use params::{Dirty, ParamInfo};
pub use smooth::Smooth;

/// where a module runs (the descriptor's `domain`): its one call, `process` or `control`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// `dm_process`, in the audio half of a block
    Audio,
    /// `dm_control`, once per block in the control half (no audio regions)
    Control,
}

/// What `#[derive(Module)]` writes: the descriptor, its shape and layout as constants, the parameter table's ranges,
/// and the copy from the table into the struct's fields.
pub trait Describe: Sized {
    const NAME: &'static str;
    /// the descriptor as JSON (`abi: 2`; `host::parse` reads it into deck-catalog's `Descriptor`)
    const DESCRIPTOR: &'static str;
    const DOMAIN: Domain;
    /// the numbers of the descriptor that place bytes – what `Shape::of` computes from the parsed descriptor
    const SHAPE: Shape;
    /// the module's memory layout, a constant (`dm_layout()` exports its hash)
    const LAYOUT: Layout = Layout::of(&Self::SHAPE);
    /// the parameters' ranges and defaults, by index
    const PARAMS: &'static [ParamInfo];
    /// the factory preset's values by parameter index (empty: none or all defaults)
    const PRESET: &'static [(usize, f32)];
    /// a module with every parameter at its default (other fields `Default`)
    fn build() -> Self;
    /// copy the table's `dirty` entries into the fields (the host has clamped them; a non-finite value is skipped)
    fn load_params(&mut self, table: &[f32], dirty: Dirty);
    /// parameter `index` as the fields hold it
    fn param(&self, index: usize) -> f32;
}

/// The code a creator writes; every method has an empty default. `process` runs for `domain = "audio"`, `control`
/// for `domain = "control"` – the other is never called.
pub trait Module: Describe + 'static {
    /// once, after `build` and the parameters: the seed, the rate, the first context and the assets are there
    fn init(&mut self, _ctx: &Ctx) {}
    /// before a call in which parameters changed (their fields already hold the new values)
    fn params_changed(&mut self, _dirty: Dirty) {}
    /// one audio block (≤ `QUANTUM` frames). Runs on the audio thread: no allocation (counted, strict = a trap), no locks.
    fn process(&mut self, _block: &mut Block, _ctx: &Ctx) {}
    /// one control block: control outputs (cables or `path` targets), events. The inputs are on `ctx`.
    fn control(&mut self, _ctx: &Ctx, _out: &mut Outputs) {}
}

/// A line to the host's console under the editor – rate-limited by the host, per module. `log!("step {}", k)`.
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => { $crate::rt::log(::core::format_args!($($arg)*)) };
}
