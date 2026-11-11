//! The kit's `editor` components (#456, #485; owner 2026-11-11: "Many editor gui wont be needed in live"): the native
//! editor's controls in its look (the OSCT editor's dark values, the base's `Tokens::default()`, overridden live by the
//! editor's `tokens.toml`). Behind the `editor` feature: the decks never build them. Drawn through the editor's own GPU
//! adapter (deck-editor `ui.rs`) until the editor moves onto the base's egui screen (#485, slice 2).
pub mod controls;
pub mod icons;
pub mod info;
pub mod typeahead;
