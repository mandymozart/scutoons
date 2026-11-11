//! The component library (#456, #485; RULES H8): one internal Rust UI library on egui's primitives, shared by the crates
//! that draw 2D UI – the native editor, the deck's own UI (#386, #481) and module interfaces. egui's painting, text and
//! fonts and input carry it; egui's widgets and theme never show. No GPU and no window here (`check:deck-deps`): a frame
//! comes out as egui's meshes (`base::screen`), an adapter draws them (deck-space's stage through egui-wgpu).
//!
//! Two component modules over one shared base (owner 2026-11-11 [ui-kit]), each behind its cargo feature so a build
//! carries only what it uses:
//! - `base`: what both use – the draw list, the input, the text field's model, the token shape, the pixel face, the
//!   screen on egui.
//! - `editor` (feature `editor`): the native editor's components and look (the controls, the typeahead, the info strip,
//!   the icons). Not in the decks' build.
//! - `live` (feature `live`): the audience-facing deck UI and its own token sets – the HUD's arcade look the deck's menu
//!   is built from (`hud.rs`), the reader's dark panel (`panel.rs`).
pub mod base;
/// the egui the base stands on (its meshes and textures reach the adapter that draws them)
pub use egui;
#[cfg(feature = "editor")]
pub mod editor;
#[cfg(feature = "live")]
pub mod live;
