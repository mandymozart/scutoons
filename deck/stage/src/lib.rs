//! deck-stage as a library: the binary (main.rs) is the window + event loop around these; the integration tests
//! (tests/) drive the audio engine directly.
pub mod audio;
pub mod control;
pub mod fft;
pub mod graph;
pub mod hud;
pub mod library;
pub mod render;
pub mod scene;
