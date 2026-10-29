//! `deck-core` – the small core of the stage, with modules compiled in only when a build asks for them (#182).
//!
//! - **The core** (always): ports and kinds (`module`), the registry that knows which modules this build carries, the
//!   control graph (`graph`) that checks a scene and runs its control domain once per frame.
//! - **Modules** (cargo features): `control` (LFOs, math), `synth` (audio blocks), `analysis` (FFT bands). The stage's
//!   GPU nodes (shader, chapter, overlay, 3D, text) stay in deck/stage – they need wgpu – but register the same way.
//! - **Scene-driven builds**: every kind names its module; `features_for(scene)` lists the features a scene needs, so a
//!   deck build (or a browser worker's wasm) carries exactly those (`deck-scene-features <scene.toml>`).
//!
//! Platform-free on purpose: the same code runs in deck/stage (native) and in a Web Worker (wasm32, deck/lab).

pub mod batch;
pub mod graph;
pub mod math;
pub mod module;

#[cfg(feature = "analysis")]
pub mod analysis;
#[cfg(feature = "control")]
pub mod control;
#[cfg(feature = "synth")]
pub mod synth;

pub use graph::{ControlGraph, SceneDef};
pub use module::{features_for, Block, KindInfo, Module, Params, Port, PortType, Registry};
