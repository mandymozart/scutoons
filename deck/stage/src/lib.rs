//! deck-stage as a library: the binary (main.rs) is the window + event loop around these; the integration tests
//! (tests/) drive the audio engine directly. Modules are cargo features (#185, modules.rs): the core is always here,
//! `space`, `sync`, `chapter`, `midi`, `osc`, `link`, `pipewire` only when the build asks for them. The space and the
//! scene model live in deck-space (deck/space, #198).
pub mod audio;
pub mod cache;
pub mod chapter;
pub mod clock;
pub mod control;
pub mod fft;
pub mod hud;
pub mod live;
pub mod modules;
#[cfg(feature = "modules")]
pub mod module_host;
pub mod render;
pub mod pairing;
pub mod quality;
pub mod splash;
pub mod stop;
pub mod switcher;
pub mod tempo;
pub mod volume;
pub mod watch;

// the scene model is deck-space's (#198: the space builds its being from it, in the browser too)
pub use deck_space::{graph, params, scene};

#[cfg(feature = "chapter")]
pub mod library;
#[cfg(any(feature = "chapter", feature = "space"))]
pub mod text;
#[cfg(feature = "space")]
pub mod space;
#[cfg(feature = "sync")]
pub mod sync;
/// "save as preset" from the knobs into the member's account, queued offline (#270)
#[cfg(feature = "sync")]
pub mod save;
#[cfg(feature = "outputs")]
pub mod outputs;
#[cfg(feature = "gamepad")]
pub mod pad;
