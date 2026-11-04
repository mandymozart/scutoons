//! Module ABI v2 (#323, research/ABI-V2.md): the memory layout derived from the descriptor (`layout`), the frozen
//! headers (`hdr`), the 40-byte event record (`ev`), planar audio and typed views (`io`), the parameter table
//! (`params`), `Ctx` v2, `Block` / `Outputs`, the counting allocator (`alloc`), the runtime the exports call (`rt`)
//! and the native harness (`testing::Rig`).
//!
//! ```ignore
//! use deck_module::v2::{Block, Ctx, Module};
//!
//! #[derive(Module)]
//! #[module(name = "gain", abi = 2)]
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
//! v2 lives beside v1 until the three repository modules move (#343): a module opts in with `#[module(abi = 2)]`; the
//! v1 SDK (the crate root's `abi`, `Block`, `Ctx`, `rt`, `testing`) and the v1 host keep working until then, and go
//! with that ticket (no dual-ABI host: ABI-V2 §7).
pub mod alloc;
pub mod block;
pub mod ctx;
pub mod ev;
pub mod hdr;
pub mod io;
pub mod layout;
pub mod params;
pub mod rt;
pub mod testing;

pub use crate::ctx::Tempo;
pub use crate::math;
pub use crate::smooth::Smooth;
pub use block::{Block, Outputs};
pub use ctx::{Asset, AssetSlot, CameraMotion, Character, Ctx, Face, Motion, Position, Status, Var};
pub use deck_module_macros::Module;
pub use ev::{Ev, Expr, Kind};
pub use hdr::{AssetInfo, AssetKind, InHdr, OutHdr};
pub use io::{InpMut, Planar, PlanarMut};
pub use layout::{Layout, Sensor, Shape, ABI_VERSION, QUANTUM};
pub use params::{Dirty, ParamInfo};

/// where a module runs (the descriptor's `domain`): its one call, `process` or `control`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// `dm_process`, in the audio half of a block
    Audio,
    /// `dm_control`, once per block in the control half (no audio regions)
    Control,
}

/// What `#[derive(Module)]` with `abi = 2` writes: the descriptor, its shape and layout as constants, the parameter
/// table's ranges, and the copy from the table into the struct's fields.
pub trait Describe: Sized {
    const NAME: &'static str;
    /// the descriptor as JSON (`abi: 2`; `host::parse_v2` reads it into deck-catalog's `Descriptor`)
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
