//! deck-space (#188, #198): the 3D space and the deck's being, shared by the native stage (deck-stage) and the browser
//! (deck/space-web, wasm32). The core is the scene model the being is built from – the node graph (graph.rs), the
//! parameters' ranges (params.rs), the scene document (scene.rs) – and the tempo's `Now`; the space itself
//! (space/) is the feature `being`, its GPU passes `gpu`, the loaders on threads and from files `native`.
pub mod controls;
pub mod graph;
pub mod params;
pub mod scene;
pub mod tempo;

#[cfg(feature = "being")]
pub mod space;
