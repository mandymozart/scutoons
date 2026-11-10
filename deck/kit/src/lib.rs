//! The component library (#456; owner 2026-11-09: "the editor component library. we have to start this now"): one set of
//! controls for the editor, the deck's own UI (#386) and module interfaces. No GPU and no window here (`check:deck-deps`):
//! a control draws into a `DrawList`, an adapter turns the list into draws – the editor's on the GPU, and `raster` here on
//! the CPU for the deck's own UI (#481: one panel for the native deck and the browser deck).
pub mod controls;
pub mod draw;
pub mod icons;
pub mod info;
pub mod panel;
#[cfg(feature = "raster")]
pub mod raster;
pub mod text;
pub mod tokens;
pub mod typeahead;
