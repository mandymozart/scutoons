//! Module ABI v2 (#323, research/ABI-V2.md): the memory layout derived from the descriptor (`layout`), the frozen
//! headers (`hdr`), the 40-byte event record (`ev`).
//!
//! v2 lives beside v1 until the three repository modules move (#343): a module opts in with `#[module(abi = 2)]`; the
//! v1 SDK (the crate root's `abi`, `Block`, `Ctx`, `rt`, `testing`) and the v1 host keep working until then, and
//! go with that ticket (no dual-ABI host: A9, ABI-V2 §7).
pub mod ev;
pub mod hdr;
pub mod layout;

pub use ev::{Ev, Expr, Kind};
pub use hdr::{AssetInfo, AssetKind, InHdr, OutHdr};
pub use layout::{Layout, Sensor, Shape, ABI_VERSION, QUANTUM};
