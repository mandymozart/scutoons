//! The deck's stage inside the editor (#444): a core rig played by deck-space's `Deck`, its graph built into the deck's
//! audio engine through deck-native's backend (deck.toml's device, like the deck), its being drawn by deck-space's
//! `SpaceGpu` into a texture of the viewport's size – the pieces the deck plays and draws with, nothing of the stage
//! rewritten here. The knobs are the rig's (`Played::knobs`), turned through the engine's `Controls` like a 3D button.
use anyhow::{bail, Context, Result};
use deck_native::audio::{self, backend};
use deck_native::clock::{self, Moment};
use deck_native::control::Controls;
use deck_native::space::portrait::Portraits;
use deck_native::space::Space;
use deck_native::tempo::{self, Tempo};
use deck_space::deck::Deck;
use deck_space::graph::{Graph, Kind};
use deck_space::quality::Rung;
use deck_space::rig::address::node_port;
use deck_space::scene::Scene;
use deck_space::space::gpu::SpaceGpu;
use std::time::Instant;

/// the stage's texture format (the renderer's offscreen format)
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// one rig knob as the panel turns it
pub struct Knob {
    pub label: String,
    pub node: usize,
    pub port: String,
    pub port_index: usize,
    pub min: f32,
    pub max: f32,
    pub log: bool,
}

pub struct Stage {
    pub controls: Controls,
    pub space: Space,
    gpu: Option<SpaceGpu>,
    texture: Option<(wgpu::Texture, wgpu::TextureView, (u32, u32))>,
    tempo: Tempo,
    handles: Option<audio::Handles>,
    _audio: Option<backend::Running>,
    _events: Option<audio::dispatch::Running>,
    origin_ns: u64,
    rate: f64,
    started: Instant,
    pub knobs: Vec<Knob>,
    /// the audio backend's name and rate, for the status line
    pub audio: String,
}

/// the deck's audio options from deck.toml (`DECK_TOML`): host, device, buffer
fn options() -> backend::Options {
    let deck = backend::DeckAudio::read();
    let host = deck.audio_host.as_deref().and_then(backend::Host::parse).unwrap_or_default();
    backend::Options { choice: backend::Choice::Auto, host, device: deck.audio_device, buffer: deck.audio_buffer, thread: deck.audio_thread.unwrap_or(true) }
}

impl Stage {
    /// `rig` = a core rig's character (`catpope`) or subject; the scene plays at `volume`
    pub fn new(rig: &str, volume: f32) -> Result<Stage> {
        let mut deck = Deck::new();
        let turn = deck.play_core(rig, "");
        if !turn.rig {
            bail!("rig {rig}: it does not play on this build ({})", if turn.note.is_empty() { "no core rig of that name" } else { turn.note.as_str() });
        }
        let scene = Scene::parse(&turn.played.scene)?;
        let graph = Graph::build(&scene)?;
        let tempo_cfg = tempo::config_from_graph(&graph)?;
        let mut handles = None;
        let mut audio_name = "off".to_string();
        let audio = {
            let mut factory = |rate: f32, channels: usize| {
                let (engine, h) = audio::Engine::build_for(&graph, rate, volume, channels)?;
                tracing::info!("audio graph: {} ({} device channels)", h.build.label(), engine.device_channels());
                handles = Some(h);
                Ok(engine)
            };
            match backend::start(&options(), &mut factory) {
                Ok(r) => {
                    audio_name = format!("{} {} Hz", r.name, r.rate);
                    Some(r)
                }
                Err(e) => {
                    tracing::warn!("audio off: {e:#}");
                    handles = None;
                    None
                }
            }
        };
        let events = handles.as_mut().and_then(|h| h.start_events().map_err(|e| tracing::warn!("events: no thread ({e})")).ok().flatten());
        let controls = Controls::new(&graph, handles.as_mut().map(|h| h.core_mut()));
        let rate = handles.as_ref().map_or(48_000.0, |h| h.clock.rate as f64);
        let shared = handles.as_ref().map(|h| h.tempo.clone()).unwrap_or_default();
        let mut tempo = Tempo::new(tempo_cfg, shared, rate);
        tempo.start();
        let node = graph.first(Kind::Space).context("the rig's scene has a space node")?;
        let cache = deck_native::cache::default_dir();
        let sets = cache.join("sets");
        let mut space = Space::with_gate(&graph, node, &scene.being, &sets, Portraits::local(&cache.join("portraits")), None);
        // the cathedral around the being, as the deck's scene shows it (the cache's set at low quality; nothing without the file)
        space.show(Some(&deck_space::scene::set_file(&sets, "cathedral", "low")), None);
        let mut knobs = Vec::new();
        for k in &turn.played.knobs {
            let (id, port) = node_port(&k.path);
            let Some(node) = graph.nodes.iter().position(|n| n.id == id) else { continue };
            let Some(port_index) = graph.nodes[node].kind.input(port) else { continue };
            let label = k.label.get("en").cloned().unwrap_or_else(|| k.path.clone());
            knobs.push(Knob { label, node, port: port.to_string(), port_index, min: k.min, max: k.max, log: k.log });
        }
        Ok(Stage { controls, space, gpu: None, texture: None, tempo, handles, _audio: audio, _events: events, origin_ns: clock::now_ns(), rate, started: Instant::now(), knobs, audio: audio_name })
    }

    pub fn glitches(&self) -> u64 {
        self.handles.as_ref().map_or(0, |h| h.glitches.count())
    }

    /// the knob's value as the engine has it
    pub fn value(&self, k: &Knob) -> f32 {
        self.controls.input(k.node, &k.port, k.min)
    }

    /// a knob's input `port` of `node` to `value` inside its range (a drag, the wheel)
    pub fn set(&mut self, node: usize, port: usize, value: f32, min: f32, max: f32) {
        self.controls.set_base(node, port, value.clamp(min.min(max), max.max(min)));
    }

    /// the pointer over the stage's viewport, in its pixels
    pub fn pointer(&mut self, x: f32, y: f32) {
        self.space.pointer_moved(x, y, &mut self.controls);
    }

    /// the stage's texture view this frame, drawn at `size`: the clock, the controls, the being, the GPU
    pub fn frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, size: (u32, u32), dt: f32) -> (&wgpu::TextureView, bool) {
        let size = (size.0.max(1), size.1.max(1));
        let mut fresh = false;
        if self.texture.as_ref().is_none_or(|t| t.2 != size) {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("stage"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&Default::default());
            self.texture = Some((tex, view, size));
            self.space.set_view(size.0 as f32, size.1 as f32);
            if let Some(g) = self.gpu.as_mut() {
                g.resize(device, size);
            }
            fresh = true;
        }
        let moment = Moment::wall(clock::now_ns(), self.origin_ns, self.rate);
        let now = self.tempo.frame(&moment);
        self.controls.update(dt, moment.lead(), &now);
        let time = self.started.elapsed().as_secs_f32();
        let meters = self.handles.as_ref().and_then(|h| h.meters.clone());
        self.space.update(dt, time, &mut self.controls, &now, &|n, p| meters.as_ref().map_or(0.0, |m| m.level(n, p)));
        let space = &mut self.space;
        let gpu = self.gpu.get_or_insert_with(|| {
            let mut g = SpaceGpu::new(device, queue, space, FORMAT, size);
            g.set_quality(Rung::default());
            g
        });
        gpu.sync(device, queue, space);
        let (_, view, _) = self.texture.as_ref().expect("made above");
        gpu.draw(queue, enc, view, size, space, time);
        (view, fresh)
    }

    pub fn glitch_text(&self) -> String {
        format!("glitches {}", self.glitches())
    }
}
