//! deck-space (#188, #198): the 3D space and the deck's being, shared by the native stage (deck-native) and the browser
//! (deck/space-web, wasm32). The core is the scene model the being is built from – the node graph (graph.rs), the
//! parameters' ranges (params.rs), the scene document (scene.rs) – and the tempo's `Now`; the space itself
//! (space/) is the feature `being`, its GPU passes and the scene renderer every deck draws with (stage.rs) `gpu`, the loaders on threads and from files `native`. rig/ = the
//! instruments, presets and character rigs a being's sound is made of (#191, research/INSTRUMENTS.md).
pub use deck_catalog::{background, Background};
pub mod cast;
pub mod context;
pub mod controls;
pub mod deck;
pub mod devices;
pub mod far;
pub mod fetched;
pub mod gamepad;
pub mod gesture;
pub mod holders;
pub mod linked;
pub mod graph;
pub mod library;
pub mod world;
pub mod modreg;
pub mod mood;
pub mod params;
pub mod piece;
pub mod published;
pub mod problem;
pub mod push;
pub mod quality;
pub mod relay;
pub mod resolver;
pub mod rig;
pub mod scene;
pub mod scene_camera;
pub mod scene_document;
pub mod scene_edit;
pub mod scene_slots;
pub mod scene_video;
pub mod scene_play;
pub mod scene_resolve;
pub mod snap;
pub mod tempo;
pub mod veil;

#[cfg(feature = "being")]
pub mod space;

/// #549: the audience start – a deck that is only there to vote (a phone), with its own relay client and its proof
#[cfg(feature = "gpu")]
pub mod audience;
#[cfg(feature = "gpu")]
pub mod build_label;
#[cfg(feature = "gpu")]
pub mod chapter;
#[cfg(feature = "gpu")]
pub mod hud;
#[cfg(feature = "gpu")]
pub mod pairing;
/// #531: the deck editor's Session panel as deck-ui's plain data (the readers, the core's link, its QR)
#[cfg(feature = "gpu")]
pub mod session_panel;
#[cfg(feature = "gpu")]
pub mod splash;
#[cfg(feature = "gpu")]
pub mod stage;

/// #423: this build's version (`0.1.0.r3093.g8eccc753[.dirty]`, `dev` outside git), every deck draws it top right: the
/// update bundle's version string, from the env DECK_BUILD_VERSION or git at build time (build.rs)
pub const BUILD: &str = env!("DECK_BUILD");
