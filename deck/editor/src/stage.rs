//! The deck's stage inside the editor (#444): a core rig played by deck-space's `Deck`, its graph built into the deck's
//! audio engine through deck-native's backend (deck.toml's device, like the deck), its being drawn by deck-space's
//! `SpaceGpu` into a texture of the viewport's size – the pieces the deck plays and draws with, nothing of the stage
//! rewritten here. The knobs are the rig's (`Played::knobs`), turned through the engine's `Controls` like a 3D button.
use crate::wires::{self, Port};
use anyhow::{bail, Context, Result};
use deck_native::audio::{self, backend};
use deck_native::clock::{self, Moment};
use deck_native::control::Controls;
use deck_native::space::portrait::Portraits;
use deck_native::space::Space;
use deck_native::tempo::{self, Tempo};
use deck_space::deck::Deck;
use deck_space::graph::{Graph, Kind, PortType};
use deck_space::params;
use deck_space::quality::Rung;
use deck_space::rig::address::node_port;
use deck_space::rig::Bundle;
use deck_space::scene::Scene;
use deck_space::space::gpu::SpaceGpu;
use std::time::Instant;

/// the stage's texture format (the renderer's offscreen format)
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// one rig knob as the panel turns it
pub struct Knob {
    pub label: String,
    /// the rig's name for it: a replay carries its value over by this
    pub path: String,
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
    /// the rig as it plays (#431: the documents a gesture changes)
    pub bundle: Bundle,
    /// the rig before any draft: what the draft is the difference to
    pub base: Bundle,
    /// the ports a cable can end at
    pub ports: Vec<Port>,
    /// which of them the level shown holds
    pub visible: Vec<bool>,
    /// the level shown: the tracks entered, top down (#457; a path, so it can go deep: #458)
    pub path: Vec<String>,
    core_node: usize,
    /// the graph's node ids: a being's point is its node
    pub node_ids: Vec<String>,
    kinds: Vec<Kind>,
    /// the mixer's strips, and what goes out
    pub strips: Vec<Strip>,
    master: Option<(usize, usize)>,
    pub master_peak: f32,
    volume: f32,
}

/// a track's strip on the mixer
pub struct Strip {
    pub track: String,
    pub node: usize,
    pub port: usize,
    pub name: String,
    pub owned: bool,
    from: (usize, usize),
    /// the held peak
    pub peak: f32,
}

/// a control input of a node, as the inspector shows it
pub struct Param {
    pub name: &'static str,
    pub port: usize,
    pub spec: params::Spec,
    pub value: f32,
    pub linked: Option<String>,
}

/// a rig ready to sound: everything that can refuse it, before the running engine is stopped
struct Ready {
    bundle: Bundle,
    knobs: Vec<deck_space::rig::bundle::Knob>,
    strips: Vec<deck_space::rig::bundle::Strip>,
    sum: Option<String>,
    scene: Scene,
    graph: Graph,
    tempo: tempo::Config,
    space_node: usize,
    ports: Vec<Port>,
}

fn prepare(bundle: Bundle) -> Result<Ready> {
    let mut deck = Deck::new();
    let turn = deck.play(Some(bundle.clone()), "")?;
    if !turn.rig {
        bail!("rig {}: it does not play on this build ({})", bundle.rig.subject, turn.note);
    }
    let scene = Scene::parse(&turn.played.scene)?;
    let graph = Graph::build(&scene)?;
    let tempo = tempo::config_from_graph(&graph)?;
    let space_node = graph.first(Kind::Space).context("the rig's scene has a space node")?;
    let ports = wires::ports(&turn.played)?;
    Ok(Ready { bundle, knobs: turn.played.knobs.clone(), strips: turn.played.strips(), sum: turn.played.sum(), scene, graph, tempo, space_node, ports })
}

/// the deck's audio options from deck.toml (`DECK_TOML`): host, device, buffer
fn options() -> backend::Options {
    let deck = backend::DeckAudio::read();
    let host = deck.audio_host.as_deref().and_then(backend::Host::parse).unwrap_or_default();
    backend::Options { choice: backend::Choice::Auto, host, device: deck.audio_device, buffer: deck.audio_buffer, thread: deck.audio_thread.unwrap_or(true) }
}

impl Stage {
    /// `rig` = a core rig's character (`catpope`) or subject, with the `draft` file's rig over it if there is one; it plays at `volume`
    pub fn new(rig: &str, volume: f32, draft: Option<&std::path::Path>) -> Result<Stage> {
        let subject = if rig.contains('.') { rig.to_string() } else { format!("character.{rig}") };
        let base = Bundle::core(&subject).with_context(|| format!("no core rig {subject}"))??;
        let bundle = draft.map_or_else(|| base.clone(), |path| wires::load_draft(path, &base));
        let mut stage = Stage::start(prepare(bundle)?, volume);
        stage.base = base;
        Ok(stage)
    }

    /// a changed rig takes the stage: refused before anything stops; the sound device is let go before the new engine
    /// opens it; the camera and the turned knobs stay
    pub fn replay(&mut self, bundle: Bundle) -> Result<()> {
        let ready = prepare(bundle)?;
        let knobs: Vec<(String, f32)> = self.knobs.iter().map(|k| (k.path.clone(), self.value(k))).collect();
        let camera = self.space.camera;
        self._events = None;
        self._audio = None;
        self.handles = None;
        let mut next = Stage::start(ready, self.volume);
        next.base = self.base.clone();
        next.enter(self.path.clone());
        next.space.camera = camera;
        next.space.being.hold(self.space.being.held(), true);
        for (path, value) in knobs {
            if let Some(k) = next.knobs.iter().find(|k| k.path == path) {
                let (node, port, min, max) = (k.node, k.port_index, k.min, k.max);
                next.set(node, port, value, min, max);
            }
        }
        if let Some((tex, view, size)) = self.texture.take() {
            next.space.set_view(size.0 as f32, size.1 as f32);
            next.texture = Some((tex, view, size));
        }
        *self = next;
        Ok(())
    }

    fn start(ready: Ready, volume: f32) -> Stage {
        let Ready { bundle, knobs: played_knobs, strips: played_strips, sum, scene, graph, tempo: tempo_cfg, space_node: node, ports } = ready;
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
        let cache = deck_native::cache::default_dir();
        // the rig editor's stage is empty ([rigs] 2026-11-08): no set around the being
        let mut space = Space::with_gate(&graph, node, &scene.being, &cache.join("sets"), Portraits::local(&cache.join("portraits")), None);
        // an editor's view: no breath, the camera stays where the hand puts it (owner 2026-11-09)
        space.being.look.breath = 0.0;
        space.camera.free = true;
        space.camera.auto = false;
        let mut knobs = Vec::new();
        for k in &played_knobs {
            let (id, port) = node_port(&k.path);
            let Some(node) = graph.nodes.iter().position(|n| n.id == id) else { continue };
            let Some(port_index) = graph.nodes[node].kind.input(port) else { continue };
            let label = k.label.get("en").cloned().unwrap_or_else(|| k.path.clone());
            knobs.push(Knob { label, path: k.path.clone(), node, port: port.to_string(), port_index, min: k.min, max: k.max, log: k.log });
        }
        let slot = |s: &str, out: bool| -> Option<(usize, usize, String)> {
            let (id, port) = node_port(s);
            let n = graph.nodes.iter().position(|x| x.id == id)?;
            let k = graph.nodes[n].kind;
            let i = if out { k.output(port) } else { k.input(port) }?;
            Some((n, i, port.to_string()))
        };
        let strips = played_strips
            .iter()
            .filter_map(|s| {
                let ((node, port, name), (fnode, fport, _)) = (slot(&s.gain, false)?, slot(&s.from, true)?);
                Some(Strip { track: s.track.clone(), node, port, name, owned: s.owned, from: (fnode, fport), peak: 0.0 })
            })
            .collect();
        let master = sum.as_deref().and_then(|s| slot(s, true)).map(|(n, p, _)| (n, p));
        let node_ids = graph.nodes.iter().map(|n| n.id.clone()).collect();
        let kinds = graph.nodes.iter().map(|n| n.kind).collect();
        Stage { controls, space, gpu: None, texture: None, tempo, handles, _audio: audio, _events: events, origin_ns: clock::now_ns(), rate, started: Instant::now(), knobs, audio: audio_name, base: bundle.clone(), bundle, visible: vec![true; ports.len()], ports, node_ids, kinds, strips, master, master_peak: 0.0, volume, path: Vec::new(), core_node: node }
    }

    /// show this level of the tree in Wire; empty = the rig
    pub fn enter(&mut self, path: Vec<String>) {
        let (level, visible) = wires::level(&self.bundle, &self.ports, &self.node_ids, self.core_node, path.last().map(String::as_str));
        self.space.being.show_level(Some(&level));
        self.visible = visible;
        self.path = path;
    }

    /// Play: every node, as the deck shows it
    pub fn show_all(&mut self) {
        self.space.being.show_level(None);
        self.visible = vec![true; self.ports.len()];
    }

    /// a strip's gain as the engine has it
    pub fn strip_gain(&self, s: &Strip) -> f32 {
        self.controls.input(s.node, &s.name, 1.0)
    }

    /// the sound's rate, if there is sound
    pub fn khz(&self) -> Option<f64> {
        self.handles.as_ref().map(|_| self.rate / 1000.0)
    }

    /// the kind's name of a graph node
    pub fn kind_name(&self, node: usize) -> &'static str {
        self.kinds.get(node).map_or("", |k| k.name())
    }

    /// what can be set on a node: its control inputs, with their ranges and values; a linked one says from where
    pub fn params(&self, node: usize) -> Vec<Param> {
        let (Some(&kind), Some(id)) = (self.kinds.get(node), self.node_ids.get(node)) else { return Vec::new() };
        let controls = kind.inputs().iter().enumerate().filter(|(_, p)| p.ty == PortType::Control);
        controls
            .map(|(port, p)| {
                let spec = params::spec(kind, p.name);
                let linked = self.ports.iter().find(|q| q.input && q.node == *id && q.name == p.name).and_then(|q| q.cables.last()).map(|c| c.from.clone());
                Param { name: p.name, port, spec, value: self.controls.input(node, p.name, spec.default), linked }
            })
            .collect()
    }

    /// one input of a node
    pub fn param(&self, node: usize, port: usize) -> Option<Param> {
        self.params(node).into_iter().find(|p| p.port == port)
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
        let fall = (-dt / deck_core::meters::FALL_SECONDS).exp();
        let level = |(n, p): (usize, usize)| meters.as_ref().map_or(0.0, |m| m.level(n, p));
        for s in &mut self.strips {
            s.peak = level(s.from).max(s.peak * fall);
        }
        self.master_peak = self.master.map_or(0.0, level).max(self.master_peak * fall);
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

}
