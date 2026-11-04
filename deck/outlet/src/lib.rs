//! A Live outlet's logic (#186, #378; research/LIVE-LENSES.md): what a deck shows for the cells the conductor sends to its
//! outlet – platform-free, so the native deck and a browser outlet play the same code.
//!
//! - `plan`: a cell → its layers as stage nodes, and what this deck can't play (the HUD's problems)
//! - `compose`: a plan → one plane at the outlet's size (the flat layers painted back → front; the 3D set named for the
//!   space node) – what it reads comes through a `Shelf`
//! - `frame`: the composed plane and the crossfade the renderer's outlet pass reads per frame (no allocation)
//! - `text`: a slice's dialog set the way the reader places it, the caption, an outlet's text layer (the reader's fonts)
//! - `route`: what a relay message means for the outlet (the composer's input, the preloader's job)
//! - `recompose`: when the composer composes again (a new cell, black, the slice under surtitles, new media, a resize, the
//!   scroll's hold) – a state machine on a clock the adapter passes in (#391)
//! - `preload`: the preloader's retry schedule and its HUD line (#391)
//! - `status`: the HUD's lines for the outlet (#391)
//!
//! The adapter (deck-native's live/) keeps the threads, the relay socket, the HTTP fetches, the disk cache and decoding.
pub mod compose;
pub mod frame;
pub mod plan;
pub mod preload;
pub mod recompose;
pub mod route;
pub mod status;
pub mod text;
