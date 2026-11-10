//! deck-native as a library: the binary (main.rs) is the window + event loop around these; the integration tests
//! (tests/) drive the audio engine directly. Modules are cargo features (#185, modules.rs): the core is always here,
//! `space`, `sync`, `chapter`, `midi`, `osc`, `link`, `pipewire` only when the build asks for them. The space and the
//! scene model live in deck-space (deck/space, #198).
pub mod audio;
pub mod cache;
pub mod chapter;
pub mod clock;
pub mod gpu;
pub mod host;
pub mod live;
pub mod modules;
#[cfg(feature = "modules")]
pub mod module_host;
pub mod render;
pub mod quality;
pub mod rig;
pub mod stop;
pub mod switcher;
pub mod tempo;
pub mod volume;
pub mod watch;

/// #423: this build's version (`0.1.0.r3093.g8eccc753[.dirty]`, `dev` outside git): the update bundle's version string,
/// from the env DECK_BUILD_VERSION or git at build time (deck-space's build.rs); drawn top right (build_label.rs) and logged at start
pub const BUILD: &str = deck_space::BUILD;

// the engine's control domain and FFT half (#317: deck-engine, the universal core)
pub use deck_engine::{control, fft};

// the scene model is deck-space's (#198: the space builds its being from it, in the browser too)
pub use deck_space::{build_label, graph, hud, pairing, params, scene, splash};

#[cfg(feature = "chapter")]
pub mod library;
#[cfg(feature = "space")]
pub mod space;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "outputs")]
pub mod outputs;
#[cfg(feature = "gamepad")]
pub mod pad;
