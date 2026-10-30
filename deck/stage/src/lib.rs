//! deck-stage as a library: the binary (main.rs) is the window + event loop around these; the integration tests
//! (tests/) drive the audio engine directly. Modules are cargo features (#185, modules.rs): the core is always here,
//! `space`, `sync`, `chapter`, `midi`, `osc`, `link`, `pipewire` only when the build asks for them.
pub mod audio;
pub mod cache;
pub mod chapter;
pub mod clock;
pub mod control;
pub mod fft;
pub mod graph;
pub mod hud;
pub mod live;
pub mod modules;
pub mod params;
pub mod render;
pub mod scene;
pub mod splash;
pub mod tempo;
pub mod watch;

#[cfg(feature = "chapter")]
pub mod library;
#[cfg(feature = "chapter")]
pub mod text;
#[cfg(feature = "space")]
pub mod space;
#[cfg(feature = "sync")]
pub mod sync;
