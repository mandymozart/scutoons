//! The native deck's audio: DEVICE I/O around deck-engine (#317, research/ENGINE-REFACTOR-PLAN.md P4). The engine – the
//! graph, the batches, every audio kind, the control domain – is the universal core's crate; what stays here is what only
//! a native deck has:
//! - `backend` – the device (PipeWire, cpal, ASIO, or a timer thread without a device) and its callback
//! - `ring` / `lead` – the render thread, the lock-free ring to the callback and the adaptive lead
//! - `glitch` – the glitch indicator that measures every callback (#208)
//! - `priority` – the audio threads' scheduling class
//! - `sequencer` / `dispatch` – the threads the engine's event windows and routing run on (feature `events`)
//! - `modules` – the wasmtime module library as the engine's `ModuleSource` (feature `modules`)
//!
//! `Engine` and `Handles` here are deck-engine's with the glitch monitor beside them: the device callback owns the
//! monitor, the frame thread reads its counters (`Handles::glitches`). Everything else dereferences to the core's.
pub mod backend;
#[cfg(feature = "events")]
pub mod dispatch;
pub mod glitch;
pub mod lead;
#[cfg(feature = "modules")]
pub mod modules;
pub mod priority;
pub mod ring;
#[cfg(feature = "events")]
pub mod sequencer;

pub use deck_engine::{default_for, Block, Buf, Build, Clock, Meters, MusicFeed, Param, Sequencing, Track, MAX_BLOCK, QUANTUM};

use anyhow::Result;
use deck_space::graph::Graph;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// `Build::detect()` with `DECK_SIMD=baseline` keeping the baseline loops (to compare AVX2 with the baseline by ear / bench)
pub fn detect_build() -> Build {
    let baseline = std::env::var("DECK_SIMD").is_ok_and(|v| v == "baseline");
    let b = Build::detect();
    Build { avx2: b.avx2 && !baseline, ..b }
}

/// the sequencer's thread, unless `DECK_EVENTS=block` (the generators then run inside the audio block)
pub fn detect_sequencing() -> Sequencing {
    match std::env::var("DECK_EVENTS") {
        Ok(v) if v == "block" => Sequencing::InBlock,
        _ => Sequencing::Thread,
    }
}

/// What the build asks of the native platform: the device (its channel count is the deck configuration, plan 2.6b: main on
/// the first channels, nothing else configured) and the module library, when the build has modules.
fn seams(device_channels: usize) -> deck_engine::Seams {
    deck_engine::Seams {
        deck: deck_engine::DeckConfig::device(device_channels),
        #[cfg(feature = "modules")]
        modules: Some(Arc::new(modules::Global)),
        ..Default::default()
    }
}

/// The engine and the glitch monitor of its device callback (#208).
pub struct Engine {
    core: deck_engine::Engine,
    monitor: Option<glitch::Monitor>,
}

/// What the frame thread keeps of a built engine: deck-engine's handles plus the glitch counters and their log.
pub struct Handles {
    core: deck_engine::Handles,
    /// the glitch indicator (#208): counts per cause, the newest, the load, the device's buffer, the render lead
    pub glitches: Arc<glitch::Glitches>,
    /// each glitch's numbers for the log – the frame thread drains it (never the audio thread)
    pub glitch_log: Option<rtrb::Consumer<glitch::Event>>,
}

impl Deref for Engine {
    type Target = deck_engine::Engine;
    fn deref(&self) -> &deck_engine::Engine {
        &self.core
    }
}

impl DerefMut for Engine {
    fn deref_mut(&mut self) -> &mut deck_engine::Engine {
        &mut self.core
    }
}

impl Deref for Handles {
    type Target = deck_engine::Handles;
    fn deref(&self) -> &deck_engine::Handles {
        &self.core
    }
}

impl DerefMut for Handles {
    fn deref_mut(&mut self) -> &mut deck_engine::Handles {
        &mut self.core
    }
}

impl Handles {
    /// the engine's own handles (the control domain takes the params and taps out of them)
    pub fn core_mut(&mut self) -> &mut deck_engine::Handles {
        &mut self.core
    }

    /// Start the threads a scene with hosts needs (the sequencer, the dispatcher). None without hosts.
    #[cfg(feature = "events")]
    pub fn start_events(&mut self) -> std::io::Result<Option<dispatch::Running>> {
        dispatch::start(&mut self.core.events, self.glitches.clone())
    }
}

impl Engine {
    /// Build the graph's audio nodes for `rate` on the batch runtime (`detect_build`), the hosts' generators on the
    /// sequencer thread (`detect_sequencing` – start it with `handles.start_events()`). `volume` is the master gain on
    /// top of the audio_out node's own.
    pub fn build(graph: &Graph, rate: f32, volume: f32) -> Result<(Engine, Handles)> {
        Engine::build_seq(graph, rate, volume, detect_build(), detect_sequencing())
    }

    /// `build` for a device with `channels` channels (what its backend reports; `build` is a stereo device)
    pub fn build_for(graph: &Graph, rate: f32, volume: f32, channels: usize) -> Result<(Engine, Handles)> {
        Engine::build_device(graph, rate, volume, detect_build(), detect_sequencing(), channels)
    }

    /// `build` with this `build`, the hosts' generators inside the block (no thread to start – the tests)
    pub fn build_with(graph: &Graph, rate: f32, volume: f32, build: Build) -> Result<(Engine, Handles)> {
        Engine::build_seq(graph, rate, volume, build, Sequencing::InBlock)
    }

    pub fn build_seq(graph: &Graph, rate: f32, volume: f32, build: Build, seq: Sequencing) -> Result<(Engine, Handles)> {
        Engine::build_device(graph, rate, volume, build, seq, 2)
    }

    fn build_device(graph: &Graph, rate: f32, volume: f32, build: Build, seq: Sequencing, channels: usize) -> Result<(Engine, Handles)> {
        let (monitor, glitches, glitch_log) = glitch::Monitor::new(rate);
        let (core, handles) = deck_engine::Engine::build_seams(graph, rate, volume, build, seq, &seams(channels))?;
        Ok((Engine { core, monitor: Some(monitor) }, Handles { core: handles, glitches, glitch_log: Some(glitch_log) }))
    }

    /// The glitch monitor for the thread that runs the device callback (#208) – once; a second call gets a fresh one
    /// whose counts nobody reads.
    pub fn take_monitor(&mut self) -> glitch::Monitor {
        self.monitor.take().unwrap_or_else(|| glitch::Monitor::new(self.core.rate()).0)
    }
}
