//! `deck-core` – the small core of the stage, with modules compiled in only when a build asks for them (#182).
//!
//! - **The core** (always): ports and kinds (`module`), the registry that knows which modules this build carries, the
//!   control graph (`graph`) that checks a scene and runs its control domain once per frame.
//! - **Modules** (cargo features): `control` (LFOs, math), `synth` (audio blocks), `analysis` (the FFT: bands, the
//!   stage's analyzer), `tempo` (the tempo map, quantize, tap – #181 / #185). The stage's
//!   GPU nodes (shader, chapter, overlay, 3D, text) stay in deck/stage – they need wgpu – but register the same way.
//! - **Scene-driven builds**: every kind names its module; `stage::features` lists the deck-stage features a scene needs
//!   (#185 – the stage's own modules: space, chapter, midi, osc, link, pipewire), `features_for` this crate's modules for
//!   a browser worker's wasm; `deck-scene-features <scene.toml>…` prints them.
//!
//! Platform-free on purpose: the same code runs in deck/stage (native) and in a Web Worker (wasm32, deck/lab).

pub mod batch;
pub mod graph;
pub mod lanes;
pub mod math;
pub mod module;
pub mod stage;

#[cfg(feature = "analysis")]
pub mod analysis;
#[cfg(feature = "control")]
pub mod control;
#[cfg(feature = "synth")]
pub mod synth;
#[cfg(feature = "tempo")]
pub mod tempo;

pub use graph::{ControlGraph, SceneDef};
pub use module::{features_for, Block, KindInfo, Module, Params, Port, PortType, Registry};
