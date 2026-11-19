//! The native deck's audio: DEVICE I/O around deck-engine (#317, research/ENGINE-REFACTOR-PLAN.md P4). The engine – the
//! graph, the batches, every audio kind, the control domain – is the universal core's crate; what stays here is what only
//! a native deck has:
//! - `backend` – the device (PipeWire, cpal, ASIO, or a timer thread without a device) and its callback
//! - `watch` – the saved device, reopened by itself when it comes back; `pw_watch` – its port for a PipeWire sink
//! - `ring` – the render thread, the lock-free ring to the callback and the adaptive lead (deck-core's `lead`)
//! - `glitch` – the glitch indicator that measures every callback (#208)
//! - `priority` – the audio threads' scheduling class
//! - `sequencer` / `dispatch` – the threads the engine's event windows and routing run on (feature `events`)
//! - `modules` – the wasmtime module library as the engine's `ModuleSource` (feature `modules`)
//!
//! `Engine` and `Handles` here are deck-engine's with the glitch monitor beside them: the device callback owns the
//! monitor, the frame thread reads its counters (`Handles::glitches`). Everything else dereferences to the core's.
pub mod backend;
pub mod deck_toml;
#[cfg(feature = "events")]
pub mod dispatch;
pub mod glitch;
#[cfg(feature = "modules")]
pub mod modules;
pub mod priority;
pub mod pw_watch;
pub mod ring;
#[cfg(feature = "events")]
pub mod sequencer;
pub mod watch;

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

/// What the build asks of the native platform: the device (its channel count and deck.toml's `[[audio_outlet]]` map are the deck
/// configuration, plan 2.6b, #428: nothing mapped = main on the first channels) and the module library, when the build has modules.
fn seams(device_channels: usize, outlets: &[deck_engine::OutletMap]) -> deck_engine::Seams {
    let deck = outlets.iter().fold(deck_engine::DeckConfig::device(device_channels), |d, o| d.with_outlet(&o.group, &o.channels));
    #[allow(unused_mut)] // the module runtime is a field of a feature
    let mut seams = deck_engine::Seams::with_deck(deck);
    #[cfg(feature = "modules")]
    {
        seams.modules = Some(Arc::new(modules::Global));
    }
    seams
}

/// The groups of `outlets` with a channel beyond a device of `device` channels (#627: deck.toml's map made for a bigger
/// interface). Those channels are silent – the group plays on the ones it has, every other group as mapped.
fn out_of_range(outlets: &[deck_engine::OutletMap], device: usize) -> Vec<&str> {
    outlets.iter().filter(|o| o.channels.iter().any(|&c| c < 1 || c > device)).map(|o| o.group.as_str()).collect()
}

/// `main` mapped to no channel (`channels = []`, what the Audio tab writes when every main cell is off): the main mix is
/// silent on purpose – the one map that silences everything a plain scene plays (#627 was this, not the cue on 3/4)
fn main_unmapped(outlets: &[deck_engine::OutletMap]) -> bool {
    outlets.iter().any(|o| o.group == "main" && o.channels.is_empty())
}

/// what the map leaves silent on this device, in the log once per map and device – an engine is built per scene, the
/// warning is not repeated
fn say_map(outlets: &[deck_engine::OutletMap], device: usize) {
    static SAID: std::sync::Mutex<Option<(Vec<deck_engine::OutletMap>, usize)>> = std::sync::Mutex::new(None);
    let groups = out_of_range(outlets, device);
    let main = main_unmapped(outlets);
    let mut said = SAID.lock().unwrap_or_else(|e| e.into_inner());
    if (groups.is_empty() && !main) || said.as_ref().is_some_and(|(m, d)| m == outlets && *d == device) {
        return;
    }
    *said = Some((outlets.to_vec(), device));
    if !groups.is_empty() {
        tracing::warn!("audio: deck.toml maps {} beyond the device's {device} channels – silent there, the rest plays", groups.iter().map(|g| format!("'{g}'")).collect::<Vec<_>>().join(", "));
    }
    if main {
        tracing::warn!("audio: deck.toml maps 'main' to no channel (channels = []) – the main mix is silent; remove the entry for channels 1..");
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
        Engine::build_mapped(graph, rate, volume, build, seq, channels, &backend::DeckAudio::read().audio_outlet)
    }

    /// `build_device` with this channel map instead of deck.toml's: a test tone on a map not saved yet
    fn build_mapped(graph: &Graph, rate: f32, volume: f32, build: Build, seq: Sequencing, channels: usize, outlets: &[deck_engine::OutletMap]) -> Result<(Engine, Handles)> {
        say_map(outlets, channels);
        let (monitor, glitches, glitch_log) = glitch::Monitor::new(rate);
        #[allow(unused_mut)] // the restart lanes are a field of a feature
        let (core, mut handles) = deck_engine::Engine::build_seams(graph, rate, volume, build, seq, &seams(channels, outlets))?;
        // #342: a show's module restarts are rebuilt on a loader thread (it ends with the engine)
        #[cfg(feature = "modules")]
        if let Err(e) = modules::spawn_loader(std::mem::take(&mut handles.restarts)) {
            tracing::warn!("modules: no loader thread ({e}) – a trapped module stays silent");
        }
        Ok((Engine { core, monitor: Some(monitor) }, Handles { core: handles, glitches, glitch_log: Some(glitch_log) }))
    }

    /// A test tone on `o`'s device with this channel map: plays until the returned backend is dropped.
    pub fn tone(o: &backend::Options, outlets: &[deck_engine::OutletMap], tone: &deck_engine::Tone) -> Result<backend::Running> {
        let graph = Graph::build(&deck_space::scene::Scene::parse(&tone.scene())?)?;
        let mut factory = |rate: f32, channels: usize| Ok(Engine::build_mapped(&graph, rate, 1.0, detect_build(), Sequencing::InBlock, channels, outlets)?.0);
        backend::start(o, &mut factory)
    }

    /// The glitch monitor for the thread that runs the device callback (#208) – once; a second call gets a fresh one
    /// whose counts nobody reads.
    pub fn take_monitor(&mut self) -> glitch::Monitor {
        self.monitor.take().unwrap_or_else(|| glitch::Monitor::new(self.core.rate()).0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test_tone_plays_on_a_map_not_saved_yet() {
        let null = backend::Options { choice: backend::Choice::Null, ..backend::Options::default() };
        let map = [deck_engine::OutletMap { group: "cue".into(), channels: vec![2] }];
        for tone in [deck_engine::Tone::Channel(2), deck_engine::Tone::Group("cue".into())] {
            let run = Engine::tone(&null, &map, &tone).unwrap();
            assert_eq!(run.name, "null");
        }
    }

    /// #627: a map made for a 20-channel interface (main 1/2, cue 3/4, stem 5/6) on a 2-channel output – the groups beyond the
    /// device are silent on their own, main still plays. The map is passed in, never the machine's deck.toml.
    #[test]
    fn a_group_mapped_beyond_the_device_is_silent_on_its_own() {
        let map = |g: &str, c: &[usize]| deck_engine::OutletMap { group: g.into(), channels: c.to_vec() };
        let rack = [map("main", &[1, 2]), map("cue", &[3, 4]), map("stem", &[5, 6])];
        let scene = "[[node]]\nid=\"a\"\nkind=\"osc\"\nfreq=440\ngain=0.5\n\
                     [[node]]\nid=\"b\"\nkind=\"osc\"\nfreq=660\ngain=0.5\n\
                     [[node]]\nid=\"m\"\nkind=\"out\"\ngroup=\"main\"\n\
                     [[node]]\nid=\"c\"\nkind=\"out\"\ngroup=\"cue\"\n\
                     [[link]]\nfrom=\"a\"\nto=\"m.in\"\n[[link]]\nfrom=\"b\"\nto=\"c.in\"\n";
        let graph = Graph::build(&deck_space::scene::Scene::parse(scene).unwrap()).unwrap();
        let peaks = |channels: usize| {
            let (mut e, h) = Engine::build_mapped(&graph, 48_000.0, 1.0, detect_build(), Sequencing::InBlock, channels, &rack).unwrap();
            let mut out = vec![0.0f32; 4800 * channels];
            e.render(&mut out, channels, |x| x);
            let mut p = vec![0.0f32; channels];
            for f in out.chunks_exact(channels) {
                for (a, &x) in p.iter_mut().zip(f) {
                    *a = a.max(x.abs());
                }
            }
            (p, h.notes.clone())
        };
        let (two, notes) = peaks(2);
        assert!(two.iter().all(|&p| p > 0.05), "main plays on the 2-channel output: {two:?}");
        assert!(notes.iter().any(|n| n.contains("'cue'") && n.contains("silent")), "the cue is named silent: {notes:?}");
        let (six, _) = peaks(6);
        assert!(six[..4].iter().all(|&p| p > 0.05) && six[4..] == [0.0, 0.0], "on six channels main and cue both play, stem has no outlet: {six:?}");
        assert_eq!(out_of_range(&rack, 2), ["cue", "stem"]);
        assert!(out_of_range(&rack, 6).is_empty());
        assert!(!main_unmapped(&rack));
    }

    /// #627's real cause on the owner's PC: deck.toml had `main` with `channels = []` (the Audio tab's "main off") – main is
    /// silent then on any device, by the map, while cue and stem still play where the device has them
    #[test]
    fn main_mapped_to_no_channel_is_silent_and_said() {
        let map = |g: &str, c: &[usize]| deck_engine::OutletMap { group: g.into(), channels: c.to_vec() };
        let off = [map("cue", &[3, 4]), map("main", &[])];
        assert!(main_unmapped(&off));
        let graph = Graph::build(&deck_space::scene::Scene::parse(&deck_engine::Tone::Group("main".into()).scene()).unwrap()).unwrap();
        let (mut e, h) = Engine::build_mapped(&graph, 48_000.0, 1.0, detect_build(), Sequencing::InBlock, 2, &off).unwrap();
        let mut out = vec![0.0f32; 4800 * 2];
        e.render(&mut out, 2, |x| x);
        assert!(out.iter().all(|x| *x == 0.0), "main off: silent");
        assert!(h.notes.iter().any(|n| n.contains("'main'") && n.contains("silent")), "{:?}", h.notes);
    }
}
