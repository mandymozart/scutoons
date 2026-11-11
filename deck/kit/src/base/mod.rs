//! The kit's base (#485): what the editor and the live components both use. A control draws into a `DrawList` (logical
//! pixels, painter's order) and answers where it landed; `screen` paints draw lists through egui (its context, epaint's
//! shapes and text, the input fed from `input`). The token shape is here, the token sets are the modules'.
pub mod draw;
pub mod input;
#[cfg(feature = "pixel")]
pub mod pixel;
pub mod screen;
pub mod text;
pub mod tokens;
