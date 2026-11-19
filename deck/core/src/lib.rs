//! `deck-core` – the platform-free core of the stage (#182): the foundations every deck shares, native and wasm32.
//!
//! - **Always:** `math` (sines with the same bits on every target), `lanes` (the `F8` vector ops), `dsp` (the oscillator,
//!   noise and filter units), `smooth` (control values glide into audio – no zipper, #188), `limiter` (the output's soft
//!   cap), `monitor` (the stats every deck shows, #364), `lead` (the adaptive output lead, #365), `meters` (the level
//!   meters' ballistics and dB scale), `load` (the CPU / GPU meters' sampling, smoothing and peak), `devices` (a device message's words, its lamp's activity – #373) and `stage` (the
//!   deck-native modules a scene needs, the carried part of a scene), `ui_tone` (the UI's short tones on top of the output).
//! - **Modules** (cargo features): `analysis` (the FFT: bands, the analyzer), `tempo` (the tempo map, quantize, tap, the
//!   clock controller – #181 / #185).
//! - **Scene-driven builds**: every kind names its module; `stage::scene_features` lists the deck-native features a scene
//!   needs (#185 – space, chapter, midi, osc, link, pipewire); `deck-scene-features <scene.toml>…` prints them.
//!
//! The evaluators live in deck-engine (the audio graph, the control half); this crate holds no graph of its own (the
//! patcher reference `ControlGraph`, `BatchGraph` and the registry of blocks were retired in #324).
//!
//! Platform-free on purpose: the same code runs in deck/native (native) and in deck-engine's wasm32 builds.

pub mod devices;
pub mod dsp;
pub mod fit;
pub mod lanes;
pub mod lead;
pub mod limiter;
pub mod load;
pub mod math;
pub mod meters;
pub mod monitor;
pub mod smooth;
pub mod stage;
pub mod ui_tone;

#[cfg(feature = "analysis")]
pub mod analysis;
#[cfg(feature = "tempo")]
pub mod tempo;
