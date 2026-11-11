//! The kit's base (#485): what the editor and the live components both use. A control draws into a `DrawList` (logical
//! pixels, painter's order) and answers where it landed; `screen` paints draw lists as epaint's shapes, the mono face
//! set on the decks' own atlas (`glyphs`) or by egui's context (the editor, feature `egui-text`). The token shape is here, the token sets are the modules'.
pub mod draw;
pub mod glyphs;
pub mod input;
#[cfg(feature = "pixel")]
pub mod pixel;
pub mod screen;
pub mod text;
pub mod tokens;
