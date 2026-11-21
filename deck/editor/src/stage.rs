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
use deck_space::edit_play::{self, LiveLayer, Monitor, Outlet};
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
    /// the eyedropper's pixel (the stage's pixels): asked before a frame, copied after its draw, read after its submit
    pixel: Option<(u32, u32)>,
    pixel_buf: Option<wgpu::Buffer>,
    /// the library's still of what plays: asked before a frame, the whole stage copied after its draw (rows padded)
    still: bool,
    still_buf: Option<(wgpu::Buffer, u32, (u32, u32))>,
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
    /// what the rig is to far addresses: who it holds, its far world nodes (deck-space holders.rs)
    pub reach: deck_space::holders::Reach,
    /// the rig before any draft: what the draft is the difference to
    pub base: Bundle,
    /// the ports a cable can end at
    pub ports: Vec<Port>,
    /// which of them the level shown holds
    pub visible: Vec<bool>,
    /// the level shown: the tracks entered, top down (#457; a path, so it can go deep: #458)
    pub path: Vec<String>,
    /// the parameter groups opened on the stage: (node, group)
    opened: Vec<(usize, String)>,
    core_node: usize,
    /// the graph's node ids: a being's point is its node
    pub node_ids: Vec<String>,
    pub kinds: Vec<Kind>,
    /// the mixer's strips, and what goes out
    pub strips: Vec<Strip>,
    /// the gains inside the tracks
    pub inner: Vec<Inner>,
    /// the audio sends between tracks (from, to): what the solo rule follows
    sends: Vec<(String, String)>,
    master: Option<(usize, usize)>,
    pub master_peak: [f32; 2],
    pub master_max: f32,
    volume: f32,
    /// the outlet groups the rig plays in, `main` first
    pub groups: Vec<String>,
    /// the tempo clock's last frame: its BPM and whether it runs
    pub beat: Option<(f64, bool)>,
    /// where that tempo comes from, as of the last frame (the toolbar's INT / EXT)
    pub pill: Option<deck_core::tempo::Pill>,
    /// the clock's beats a bar
    pub per_bar: u32,
    /// a scene of Show's plays in the rig's place (`play_scene`): its id
    pub scene: Option<String>,
    played_scene: Option<Scene>,
    /// a panic holds: no sound device, a black picture
    pub dark: bool,
    /// the background of what plays, and the show's over it
    own_background: Option<deck_space::Background>,
    over: Option<deck_space::Background>,
    /// why a track is silent: its module is not loaded (published.rs), by track id
    pub silent: std::collections::BTreeMap<String, String>,
    /// Play: the item runs as a deck from the start; Edit: it is worked on, live (deck-space edit_play.rs)
    pub playing: bool,
    /// what outside inputs set over the hand's values: shown on the knobs, never saved
    pub layer: LiveLayer,
    /// the mixer's solos and the armed track, for the session
    pub monitor: Monitor,
    /// Edit plays on the device's cue (not a live show's preview)
    pub on_cue: bool,
    /// the source the item's clock names (its tempo node)
    clock_source: tempo::Source,
    /// the far world nodes' rings (deck-space far.rs), the MIDI world nodes' way in
    far: Option<deck_space::far::FarTable>,
    midi: Option<deck_events::adapters::midi::Midi>,
    /// a show is live (#630): what plays here is a preview on the cue (deck-engine cue.rs), its clock the show's
    pub live: bool,
    /// the device's cue: the preview's or a pre-listened cell's way onto it, and the cue mix
    cue: Option<deck_engine::cue::Cue>,
    pub cue_mix: f32,
    /// a show cell pre-listened on the cue: what it is, and the engine's frame side
    pub prelisten: Option<Prelisten>,
}

/// A show cell played on the cue before its row fires: the second, small engine's frame side.
pub struct Prelisten {
    pub label: String,
    _handles: audio::Handles,
    _events: Option<audio::dispatch::Running>,
}

/// a track's strip on the mixer
pub struct Strip {
    pub track: String,
    pub node: usize,
    pub port: usize,
    pub name: String,
    pub owned: bool,
    from: (usize, usize),
    /// the held peak, left and right
    pub peak: [f32; 2],
    /// the highest peak since it was last cleared
    pub max: f32,
}

/// a gain inside a track, as the mixer shows it
pub struct Inner {
    pub track: String,
    pub label: String,
    /// the node whose sound it sets
    pub source: String,
    pub node: usize,
    pub port: usize,
    pub name: String,
    /// `node.port` in the track's documents
    pub path: String,
    /// a cable feeds it: no hand on it
    pub linked: bool,
    max_value: f32,
    from: (usize, usize),
    /// the source sits before this gain (a mixer's input): the level reads after it
    before_gain: bool,
    pub peak: [f32; 2],
    pub max: f32,
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
    groups: Vec<String>,
    reach: deck_space::holders::Reach,
}

fn prepare(bundle: Bundle) -> Result<Ready> {
    let mut deck = Deck::new();
    let turn = deck.play(Some(bundle.clone()), "")?;
    if !turn.rig {
        bail!("rig {}: it does not play on this build ({})", bundle.rig.subject, turn.note);
    }
    // a module whose build is not loaded yet leaves its node out: the rest of the rig sounds meanwhile
    let library = deck_native::module_host::Library::global();
    let scene = Scene::parse(&turn.played.scene)?.modules_kept(|kind| library.get(kind.trim_start_matches(deck_space::modreg::PREFIX)).is_some()).with_metronome();
    let graph = Graph::build(&scene)?;
    let tempo = tempo::config_from_graph(&graph)?;
    let space_node = graph.first(Kind::Space).context("the rig's scene has a space node")?;
    let ports = wires::ports(&turn.played)?;
    let groups = deck_engine::outlet_groups(&graph);
    let reach = deck_space::rig::address::Place::of_subject(&bundle.rig.subject).map(|p| deck_space::holders::reach_of(&p, &turn.played)).unwrap_or_default();
    Ok(Ready { reach, bundle, knobs: turn.played.knobs.clone(), strips: turn.played.strips(), sum: turn.played.sum(), scene, graph, tempo, space_node, ports, groups })
}

/// the scene a rig bundle plays as (a pre-listened cell's piece, #630)
pub fn bundle_scene(bundle: Bundle) -> Result<Scene> {
    prepare(bundle).map(|r| r.scene)
}

impl Stage {
    /// `plays` on the stage at `volume`, the difference to the rig `base` (rigs.rs `base`)
    pub fn new(base: Bundle, plays: Bundle, volume: f32) -> Result<Stage> {
        let mut stage = Stage::start(prepare(plays)?, volume, false, deck_engine::cue::MIX, false);
        stage.base = base;
        Ok(stage)
    }

    /// a changed rig takes the stage: refused before anything stops; the sound device is let go before the new engine
    /// opens it; the camera and the turned knobs stay
    pub fn replay(&mut self, bundle: Bundle) -> Result<()> {
        self.take(prepare(bundle)?, true)
    }

    /// A scene the core resolved (Show's new scene) plays in the rig's place, the rig kept for `replay`: its cast sounds
    /// and stands at its spots; no knobs, no cables of a rig.
    pub fn play_scene(&mut self, id: &str, scene: Scene) -> Result<()> {
        let scene = scene.with_metronome();
        let graph = Graph::build(&scene)?;
        let tempo = tempo::config_from_graph(&graph)?;
        let space_node = graph.first(Kind::Space).context("the scene has a space node")?;
        let groups = deck_engine::outlet_groups(&graph);
        let played = scene.clone();
        let ready = Ready { bundle: self.bundle.clone(), knobs: Vec::new(), strips: Vec::new(), sum: None, scene, graph, tempo, space_node, ports: Vec::new(), groups, reach: Default::default() };
        self.path.clear();
        let same = self.scene.as_deref() == Some(id);
        self.take(ready, same)?;
        self.space.dop.scene = Some(format!("scene:{id}"));
        if !same {
            self.space.open_on_scene_camera();
        }
        self.scene = Some(id.to_string());
        self.played_scene = Some(played);
        Ok(())
    }

    /// The show's background over what plays: the parent wins.
    pub fn wear(&mut self, show: Option<&deck_space::Background>) {
        if self.over.as_ref() == show {
            return;
        }
        self.over = show.cloned();
        self.space.set_background(deck_catalog::background::resolve(show, self.own_background.as_ref(), None, None).cloned());
    }

    /// Home: the sound device is let go until the next replay (the picture is the idle scene's, idle.rs)
    pub fn rest(&mut self) {
        self._events = None;
        self._audio = None;
        self.handles = None;
    }

    /// the master gain what plays here is built with
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// let go by `rest` (not by a panic): a replay brings the sound back
    pub fn resting(&self) -> bool {
        self._audio.is_none() && !self.dark
    }

    /// Black and silent until `resume`: the sound device is let go, the picture goes black.
    pub fn panic(&mut self) {
        self.prelisten = None;
        self._events = None;
        self._audio = None;
        self.handles = None;
        self.dark = true;
    }

    /// What played before the panic (the rig or Show's scene), from its start.
    pub fn resume(&mut self) -> Result<()> {
        self.dark = false;
        match (self.scene.clone(), self.played_scene.clone()) {
            (Some(id), Some(scene)) => self.play_scene(&id, scene),
            _ => self.replay(self.bundle.clone()),
        }
    }

    /// the stage plays `ready`; `keep`: the view and a playing move stay (else the new scene's own camera)
    fn take(&mut self, ready: Ready, keep: bool) -> Result<()> {
        let knobs: Vec<(String, f32)> = self.knobs.iter().map(|k| (k.path.clone(), self.value(k))).collect();
        let camera = self.space.camera;
        let (director, current, viewing) = (self.space.dop.director.clone(), self.space.dop.current.clone(), self.space.dop.viewing.clone());
        self._events = None;
        self._audio = None;
        self.handles = None;
        self.prelisten = None;
        let mut next = Stage::start(ready, self.volume, self.live, self.cue_mix, self.playing);
        next.base = self.base.clone();
        next.opened = std::mem::take(&mut self.opened);
        next.enter(self.path.clone());
        if keep {
            next.started = self.started;
            next.space.camera = camera;
            (next.space.dop.director, next.space.dop.current, next.space.dop.viewing) = (director, current, viewing);
        }
        next.space.being.hold(self.space.being.held(), true);
        for (path, value) in knobs {
            if let Some(k) = next.knobs.iter().find(|k| k.path == path) {
                let (node, port, min, max) = (k.node, k.port_index, k.min, k.max);
                next.put(node, port, value, min, max);
            }
        }
        if let Some((tex, view, size)) = self.texture.take() {
            next.space.set_view(size.0 as f32, size.1 as f32);
            next.texture = Some((tex, view, size));
        }
        let dark = self.dark;
        next.layer = std::mem::take(&mut self.layer);
        next.monitor = std::mem::take(&mut self.monitor);
        *self = next;
        self.follow_clock();
        let tracks: Vec<String> = self.bundle.rig.tracks.iter().map(|t| t.id.clone()).collect();
        self.monitor.keep_only(&tracks.iter().map(String::as_str).collect::<Vec<_>>());
        self.apply_monitor();
        // a rig changed during a panic stays silent until the resume
        if dark {
            self.panic();
        }
        Ok(())
    }

    /// the rig again on the machine's audio settings as deck.toml has them now (a test tone stops)
    pub fn restart_audio(&mut self) -> Result<()> {
        self.replay(self.bundle.clone())
    }

    /// what the device watch says: the saved device away (silent, waiting for it) or back
    pub fn audio_note(&self) -> Option<deck_native::audio::watch::Note> {
        self._audio.as_ref().and_then(backend::Running::note)
    }

    /// a short UI tone at `hz` on top of what plays (the cue, else main; deck-core `ui_tone`); nothing while the audio is off
    pub fn ui_tone(&self, hz: f32, volume: f32) {
        if let Some(h) = &self.handles {
            h.tones.play(hz, volume);
        }
    }

    /// the rig's sound stops and a test tone plays on these settings until `restart_audio`
    pub fn tone(&mut self, o: &backend::Options, outlets: &[deck_engine::OutletMap], tone: &deck_engine::Tone) -> Result<()> {
        self._events = None;
        self._audio = None;
        self.handles = None;
        let run = audio::Engine::tone(o, outlets, tone)?;
        self.audio = format!("tone {} {} Hz", run.name, run.rate);
        self._audio = Some(run);
        Ok(())
    }

    fn start(ready: Ready, volume: f32, live: bool, cue_mix: f32, playing: bool) -> Stage {
        let Ready { bundle, knobs: played_knobs, strips: played_strips, sum, scene, graph, tempo: tempo_cfg, space_node: node, ports, groups, reach } = ready;
        let tempo_cfg = if live { tempo_cfg.preview() } else { tempo_cfg };
        let mut handles = None;
        let mut cue = None;
        let mut audio_name = "off".to_string();
        let mut on_cue = false;
        let audio = {
            let mut factory = |rate: f32, channels: usize| {
                let (engine, h, c) = if live {
                    preview(&graph, rate, volume, channels)?
                } else {
                    let (engine, h, c) = own(&graph, rate, volume, channels)?;
                    if edit_play::outlet(playing, false, c.heard()) == Outlet::Cue {
                        drop((engine, h, c));
                        on_cue = true;
                        preview(&graph, rate, volume, channels)?
                    } else {
                        (engine, h, c)
                    }
                };
                let to = if live { ", a preview on the cue" } else if on_cue { ", Edit on the cue" } else { "" };
                tracing::info!("audio graph: {} ({} device channels{to})", h.build.label(), engine.device_channels());
                c.set_mix(cue_mix);
                handles = Some(h);
                cue = Some(c);
                Ok(engine)
            };
            match backend::start(&backend::Options::from_deck(&backend::DeckAudio::read()), &mut factory) {
                Ok(r) => {
                    audio_name = format!("{} {} Hz", r.name, r.rate);
                    Some(r)
                }
                Err(e) => {
                    tracing::warn!("audio off: {e:#}");
                    handles = None;
                    cue = None;
                    None
                }
            }
        };
        let far = handles.as_mut().and_then(|h| deck_native::space::far_table(h, &bundle.rig.subject));
        let midi = handles.as_mut().map(|h| deck_events::adapters::midi::Midi::new(&mut h.core_mut().events.names.clone(), Some("midi:editor"), 1));
        let events = handles.as_mut().and_then(|h| h.start_events().map_err(|e| tracing::warn!("events: no thread ({e})")).ok().flatten());
        let controls = Controls::new(&graph, handles.as_mut().map(|h| h.core_mut()));
        let rate = handles.as_ref().map_or(48_000.0, |h| h.clock.rate as f64);
        let shared = handles.as_ref().map(|h| h.tempo.clone()).unwrap_or_default();
        let clock_source = tempo_cfg.source;
        let mut tempo = Tempo::new(tempo_cfg, shared, rate);
        tempo.start();
        if live && clock_source != tempo::Source::Internal {
            tempo.set_source(tempo::Source::Internal);
        }
        let cache = crate::cache_dir();
        // the rig editor's stage is empty ([rigs] 2026-11-08): no set around the being
        let mut space = Space::with_gate(&graph, node, &scene.being, &cache.join("sets"), Portraits::local(&cache.join("portraits")), None);
        let own_background = scene.stage.background.clone();
        space.set_background(own_background.clone());
        space.set_layers(&scene.stage.layers);
        // an editor's view: no breath, the camera stays where the hand puts it (owner 2026-11-09)
        space.being.look.breath = 0.0;
        space.camera.free = true;
        space.camera.auto = false;
        space.set_camera(None, scene.camera.clone());
        space.set_motions(&scene);
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
                Some(Strip { track: s.track.clone(), node, port, name, owned: s.owned, from: (fnode, fport), peak: [0.0; 2], max: 0.0 })
            })
            .collect();
        let track_of = |n: usize| ports.iter().find(|p| p.node == graph.nodes[n].id).map(|p| p.track.clone());
        let mut sends: Vec<(String, String)> = Vec::new();
        for l in graph.links.iter().filter(|l| l.ty == PortType::Audio) {
            if let (Some(a), Some(b)) = (track_of(l.from.0), track_of(l.to.0)) {
                if a != b && !sends.contains(&(a.clone(), b.clone())) {
                    sends.push((a, b));
                }
            }
        }
        // the gains inside each track: a node's `gain` (its output's level), a mixer's `gainK` (what feeds its `inK`)
        let mut inner = Vec::new();
        for (n, node) in graph.nodes.iter().enumerate() {
            let Some(port_of) = ports.iter().find(|p| p.node == node.id) else { continue };
            let track = port_of.track.clone();
            let local = node.id.strip_prefix(&format!("{track}_")).unwrap_or(&node.id).to_string();
            for (port, input) in node.kind.inputs().iter().enumerate().filter(|(_, p)| p.ty == PortType::Control && p.name.starts_with("gain")) {
                let linked = graph.links.iter().any(|l| l.to == (n, port));
                let source = match input.name.strip_prefix("gain").filter(|k| !k.is_empty()) {
                    Some(k) => node.kind.input(&format!("in{k}")).and_then(|i| graph.links.iter().find(|l| l.to == (n, i))).map(|l| l.from),
                    None => (!node.kind.outputs().is_empty()).then_some((n, 0)),
                };
                let Some(from) = source else { continue };
                // its internal address, what the documents call it
                let label = format!("{local}.{}", input.name);
                let source_name = graph.nodes[from.0].id.strip_prefix(&format!("{track}_")).unwrap_or(&graph.nodes[from.0].id).to_string();
                let spec = params::spec(node.kind, input.name);
                inner.push(Inner { track: track.clone(), label, source: source_name, node: n, port, name: input.name.to_string(), path: format!("{local}.{}", input.name), linked, max_value: spec.max, from, before_gain: input.name != "gain", peak: [0.0; 2], max: 0.0 });
            }
        }
        // one strip per source: a source that feeds an internal mixer is that mixer's input; its own `gain` is a sound
        // parameter (the inspector's), not a second fader on the same signal
        let summed: Vec<(usize, usize)> = inner.iter().filter(|s: &&Inner| s.before_gain).map(|s| s.from).collect();
        inner.retain(|s: &Inner| s.before_gain || !summed.contains(&s.from));
        let master = sum.as_deref().and_then(|s| slot(s, true)).map(|(n, p, _)| (n, p));
        let node_ids = graph.nodes.iter().map(|n| n.id.clone()).collect();
        let kinds = graph.nodes.iter().map(|n| n.kind).collect();
        Stage { controls, space, gpu: None, texture: None, pixel: None, pixel_buf: None, still: false, still_buf: None, tempo, handles, _audio: audio, _events: events, origin_ns: clock::now_ns(), rate, started: Instant::now(), knobs, audio: audio_name, base: bundle.clone(), bundle, visible: vec![true; ports.len()], ports, node_ids, kinds, strips, inner, sends, master, master_peak: [0.0; 2], master_max: 0.0, volume, path: Vec::new(), opened: Vec::new(), core_node: node, groups, reach, scene: None, played_scene: None, dark: false, silent: Default::default(), beat: None, pill: None, per_bar: deck_core::units::PER_BAR, own_background, over: None, playing, layer: LiveLayer::default(), monitor: Monitor::default(), on_cue, clock_source, far, midi, live, cue, cue_mix, prelisten: None }
    }

    /// A show went live or ended (#630): what plays here becomes a preview on the cue, or plays on its outlets again –
    /// rebuilt from what plays (the rig or Show's scene), the camera and the knobs kept. A panic holds on: the next resume
    /// builds it so.
    pub fn set_live(&mut self, live: bool) -> Result<()> {
        if self.live == live {
            return Ok(());
        }
        self.live = live;
        if self.dark || self._audio.is_none() {
            return Ok(());
        }
        self.resume()
    }

    /// the clock locked to a live show's (#630), or let go
    pub fn hold(&mut self, held: Option<deck_core::tempo::Held>) {
        if held.is_some() || self.tempo.held() {
            self.tempo.hold(held);
        }
    }

    /// the master BPM typed on the toolbar's pill (the core's rules: a follower is left for it, Link takes it)
    pub fn set_bpm(&mut self, bpm: f64) {
        // the item's own source gives way with the follower: Edit and Play go on with the typed tempo
        if self.tempo.source.typed() != self.tempo.source {
            self.clock_source = self.tempo.source.typed();
        }
        self.tempo.set_bpm(bpm);
    }

    /// the source picked on the toolbar's pill: the item's own now, until another is picked
    pub fn pick_clock(&mut self, s: tempo::Source) {
        self.clock_source = s;
        self.tempo.pick(s);
    }

    /// the metronome on or off (the pill's click button)
    pub fn set_click(&mut self, on: bool) {
        self.tempo.set_click(on);
    }

    /// a show's kept tempo takes the clock: its BPM, then its source, the item's own until another is picked
    pub fn take_saved(&mut self, t: &deck_show::Tempo) {
        self.clock_source = t.source;
        self.tempo.send(t.event());
    }

    /// the sources the pill cycles through here: the show clock is the deck's, a stage has none of its own
    pub fn picks() -> Vec<tempo::Source> {
        Tempo::offered().into_iter().filter(|s| *s != tempo::Source::Relay).collect()
    }

    /// whether the clock is the live show's
    pub fn held(&self) -> bool {
        self.tempo.held()
    }

    /// the cue mix: 0 = the show, 1 = the preview
    pub fn set_cue_mix(&mut self, mix: f32) {
        self.cue_mix = mix.clamp(0.0, 1.0);
        if let Some(c) = &self.cue {
            c.set_mix(self.cue_mix);
        }
    }

    /// whether the device has a cue of its own (None: no sound device)
    pub fn cue_heard(&self) -> Option<bool> {
        self.cue.as_ref().map(deck_engine::cue::Cue::heard)
    }

    /// A show cell's scene on the cue, a second engine beside what plays here (its knobs as the cell sets them), until
    /// `stop_prelisten`; refused while a panic holds or without a sound device.
    pub fn prelisten(&mut self, label: &str, scene: &Scene, set: &[(String, f32)]) -> Result<()> {
        if self.dark {
            bail!("a panic holds: resume first");
        }
        let graph = Graph::build(scene)?;
        let rate = self.handles.as_ref().map(|h| h.clock.rate).context("no sound device")?;
        let (engine, mut h) = audio::Engine::build_folded(&graph, rate, self.volume)?;
        let mut controls = Controls::new(&graph, Some(h.core_mut()));
        for (path, v) in set {
            let (id, port) = node_port(path);
            let Some(n) = graph.nodes.iter().position(|x| x.id == id) else { continue };
            let Some(i) = graph.nodes[n].kind.input(port) else { continue };
            let spec = params::spec(graph.nodes[n].kind, graph.nodes[n].kind.inputs()[i].name);
            controls.set_base(n, i, v.clamp(spec.min.min(spec.max), spec.max.max(spec.min)));
        }
        let events = h.start_events().map_err(|e| tracing::warn!("pre-listen events: no thread ({e})")).ok().flatten();
        let cue = self.cue.as_mut().context("no sound device")?;
        if !cue.play(Some(engine.into_core())) {
            bail!("the cue did not take it");
        }
        self.prelisten = Some(Prelisten { label: label.to_string(), _handles: h, _events: events });
        Ok(())
    }

    /// the pre-listened cell off the cue: what played there before comes back (a live stage's own preview is rebuilt)
    pub fn stop_prelisten(&mut self) -> Result<()> {
        if self.prelisten.take().is_none() {
            return Ok(());
        }
        if self.live && !self.dark {
            return self.resume();
        }
        if let Some(c) = self.cue.as_mut() {
            c.play(None);
        }
        Ok(())
    }

    /// Back to Edit (or Play): the clock stays the item's own; built again when Edit sounds elsewhere than Play (the cue)
    pub fn follow(&mut self, playing: bool) {
        self.playing = playing;
        self.follow_clock();
        let cue = self.cue_heard().unwrap_or(false);
        let wants = !self.live && edit_play::outlet(playing, false, cue) == Outlet::Cue;
        if wants != self.on_cue && !self.dark && self._audio.is_some() {
            if let Err(e) = self.resume() {
                tracing::warn!("edit / play: {e:#}");
            }
        }
    }

    /// in Edit and in Play the clock follows the item's own source (a live show holds it)
    fn follow_clock(&mut self) {
        if self.clock_source != tempo::Source::Internal && !self.live {
            self.tempo.set_source(self.clock_source);
        }
    }

    /// Play: the item built again from the start, as a deck would run it – states reset, the timeline at 0, on the deck's
    /// outputs; the knobs as the hand left them. Nothing while a panic holds.
    pub fn play_from_start(&mut self) -> Result<()> {
        self.playing = true;
        if self.dark {
            return Ok(());
        }
        self.resume()?;
        self.started = Instant::now();
        Ok(())
    }

    /// The live layer dropped: each input it held back to the hand's value. False: it held nothing.
    pub fn drop_layer(&mut self) -> bool {
        let restore = edit_play::stop(&mut self.layer).restore;
        let any = !restore.is_empty();
        for (slot, v) in restore {
            if let Some((node, port, min, max)) = self.slot(&slot) {
                self.put(node, port, v, min, max);
            }
        }
        any
    }

    /// the live layer over a knob: its hand's value and the live one
    pub fn live(&self, k: &Knob) -> Option<&edit_play::Held> {
        self.layer.held(&k.path)
    }

    /// what an input saves: the hand's value, never the live layer's
    pub fn saved(&self, node: usize, name: &str) -> f32 {
        let engine = self.controls.input(node, name, 0.0);
        self.node_ids.get(node).map_or(engine, |id| self.layer.saved(&format!("{id}.{name}"), engine))
    }

    /// a track soloed or not any more (for the session); whether it is now
    pub fn solo(&mut self, track: &str) -> bool {
        let on = self.monitor.solo(track);
        self.apply_monitor();
        on
    }

    /// a track armed or disarmed: it takes the hand, its monitoring on; whether it is now
    pub fn arm(&mut self, track: &str) -> bool {
        let on = self.monitor.arm(track);
        self.apply_monitor();
        on
    }

    /// the strips as the monitor hears them: a strip not heard is silent, a heard one at its stored gain
    fn apply_monitor(&mut self) {
        let set: Vec<(usize, usize, f32)> = self
            .strips
            .iter()
            .filter(|s| !s.owned)
            .map(|s| {
                let stored = self.bundle.rig.tracks.iter().find(|t| t.id == s.track).map_or(1.0, |t| t.gain);
                (s.node, s.port, if self.monitor.heard(&s.track, &self.sends) { stored } else { 0.0 })
            })
            .collect();
        for (node, port, gain) in set {
            self.controls.set_base(node, port, gain);
        }
    }

    /// the clock's source now and the item's own (`midi`, `internal` …)
    pub fn clock(&self) -> (&'static str, &'static str) {
        (self.tempo.source.name(), self.clock_source.name())
    }

    /// far events into the far world nodes listening to their address; how many went in
    pub fn take_far(&mut self, events: &[protocol::DeckEvent]) -> usize {
        self.far.as_mut().map_or(0, |f| f.take(events))
    }

    /// a MIDI message into the rig's MIDI world nodes (through the dispatcher, as the deck's MIDI adapter does)
    pub fn take_midi(&mut self, bytes: &[u8]) -> bool {
        let (Some(m), Some(run)) = (self.midi.as_mut(), self._events.as_ref()) else { return false };
        let mut out = Vec::new();
        let fed = m.feed(bytes, self.started.elapsed().as_secs_f64(), &mut out);
        for e in out {
            run.inlet.send(e);
        }
        fed
    }

    /// A value from outside on a control input named by its compiled slot (`drone_osc.freq`), inside its range: it goes
    /// into the live layer over the hand's value. False: no such input.
    pub fn set_slot(&mut self, slot: &str, value: f32) -> bool {
        let Some((node, port, min, max)) = self.slot(slot) else { return false };
        let value = value.clamp(min.min(max), max.max(min));
        let now = self.controls.input_at(node, port).unwrap_or(value);
        self.layer.outside(slot, now, value);
        self.put(node, port, value, min, max);
        true
    }

    /// a compiled slot's node, input and range
    fn slot(&self, slot: &str) -> Option<(usize, usize, f32, f32)> {
        let (id, port) = node_port(slot);
        let node = self.node_ids.iter().position(|n| n == id)?;
        let kind = *self.kinds.get(node)?;
        let index = kind.input(port)?;
        let spec = params::spec(kind, kind.inputs()[index].name);
        Some((node, index, spec.min, spec.max))
    }

    /// every rig knob's value as the engine has it, by its path
    pub fn knob_values(&self) -> Vec<(String, f32)> {
        self.knobs.iter().map(|k| (k.path.clone(), self.value(k))).collect()
    }

    /// knobs back to these values (by path)
    pub fn set_knob_values(&mut self, values: &[(String, f32)]) {
        for (path, v) in values {
            if let Some((node, port, min, max)) = self.knobs.iter().find(|k| k.path == *path).map(|k| (k.node, k.port_index, k.min, k.max)) {
                self.put(node, port, *v, min, max);
            }
        }
    }

    /// show this level of the tree in Wire; empty = the rig
    pub fn enter(&mut self, path: Vec<String>) {
        let (mut level, visible) = wires::level(&self.bundle, &self.ports, &self.node_ids, self.core_node, path.last().map(String::as_str));
        level.opened = self.opened.clone();
        self.space.being.show_level(Some(&level));
        self.visible = visible;
        self.path = path;
    }

    /// a parameter group on the stage opened, or closed again
    pub fn toggle_group(&mut self, node: usize, group: &str) {
        match self.opened.iter().position(|(n, g)| *n == node && g == group) {
            Some(at) => _ = self.opened.remove(at),
            None => self.opened.push((node, group.to_string())),
        }
        self.enter(self.path.clone());
    }

    /// Play: every node, as the deck shows it
    pub fn show_all(&mut self) {
        self.space.being.show_level(None);
        self.visible = vec![true; self.ports.len()];
    }

    /// the node whose sound an inner gain sets
    pub fn inner_source(&self, s: &Inner) -> usize {
        s.from.0
    }

    /// an inner gain as the engine has it
    pub fn inner_gain(&self, s: &Inner) -> f32 {
        self.controls.input(s.node, &s.name, 1.0)
    }

    /// an inner gain set in the engine
    pub fn inner_set(&mut self, i: usize, value: f32) {
        if let Some(s) = self.inner.get(i) {
            let (node, port, max) = (s.node, s.port, s.max_value);
            self.set(node, port, value, 0.0, max);
        }
    }

    /// a strip's gain as the engine has it
    pub fn strip_gain(&self, s: &Strip) -> f32 {
        self.controls.input(s.node, &s.name, 1.0)
    }

    /// the sound's rate, if there is sound
    pub fn khz(&self) -> Option<f64> {
        self.handles.as_ref().map(|_| self.rate / 1000.0)
    }

    /// a graph node that is a far world node (#432): its track, its id in the instrument, its address (#477: the inspector's
    /// address row, set from the address book)
    pub fn far(&self, node: usize) -> Option<(String, String, deck_space::rig::address::Address)> {
        let id = self.node_ids.get(node)?;
        self.bundle.rig.far_world_nodes(&self.bundle.library()).into_iter().find(|(track, n, _)| *id == format!("{track}_{n}"))
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

    /// The hand: a knob's input `port` of `node` to `value` inside its range (a drag, the wheel, typed); it takes the input
    /// back from the live layer.
    pub fn set(&mut self, node: usize, port: usize, value: f32, min: f32, max: f32) {
        if let (Some(id), Some(kind)) = (self.node_ids.get(node), self.kinds.get(node)) {
            if let Some(p) = kind.inputs().get(port) {
                self.layer.hand(&format!("{id}.{}", p.name));
            }
        }
        self.put(node, port, value, min, max);
    }

    /// an input to `value` inside its range, whoever's it is
    fn put(&mut self, node: usize, port: usize, value: f32, min: f32, max: f32) {
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
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
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
        self.beat = Some((now.bpm, now.running));
        self.pill = Some(self.tempo.pill(moment.now_ns));
        self.per_bar = now.per_bar;
        self.controls.update(dt, moment.lead(), &now);
        let time = self.started.elapsed().as_secs_f32();
        let meters = self.handles.as_ref().and_then(|h| h.meters.clone());
        let fall = (-dt / deck_core::meters::FALL_SECONDS).exp();
        // left and right; a mono output is the same on both
        let level = |(n, p): (usize, usize)| -> [f32; 2] {
            meters.as_ref().map_or([0.0; 2], |m| {
                let left = m.channel_level(n, p, 0);
                [left, if m.width(n, p) > 1 { m.channel_level(n, p, 1) } else { left }]
            })
        };
        for s in &mut self.strips {
            // after the fader, as a channel's meter reads
            let g = self.controls.input(s.node, &s.name, 1.0);
            let [l, r] = level(s.from);
            s.peak = [(l * g).max(s.peak[0] * fall), (r * g).max(s.peak[1] * fall)];
            s.max = s.max.max((l * g).max(r * g));
        }
        for s in &mut self.inner {
            let after = if s.before_gain { self.controls.input(s.node, &s.name, 1.0) } else { 1.0 };
            let [l, r] = level(s.from);
            s.peak = [(l * after).max(s.peak[0] * fall), (r * after).max(s.peak[1] * fall)];
            s.max = s.max.max((l * after).max(r * after));
        }
        let [l, r] = self.master.map_or([0.0; 2], level);
        self.master_peak = [l.max(self.master_peak[0] * fall), r.max(self.master_peak[1] * fall)];
        self.master_max = self.master_max.max(l.max(r));
        self.space.update(dt, time, &mut self.controls, &now, &|n, p| meters.as_ref().map_or(0.0, |m| m.level(n, p)));
        let space = &mut self.space;
        let gpu = self.gpu.get_or_insert_with(|| {
            let mut g = SpaceGpu::new(device, queue, space, FORMAT, size);
            g.set_quality(Rung::default());
            g
        });
        gpu.sync(device, queue, space);
        let (_, view, _) = self.texture.as_ref().expect("made above");
        if self.dark {
            enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("stage dark"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
        } else {
            gpu.draw(queue, enc, view, size, space, time);
        }
        if let (Some((x, y)), Some((tex, _, _))) = (self.pixel.filter(|&(x, y)| x < size.0 && y < size.1), self.texture.as_ref()) {
            let buf = self.pixel_buf.get_or_insert_with(|| device.create_buffer(&wgpu::BufferDescriptor { label: Some("eyedropper"), size: 256, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
            enc.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d { x, y, z: 0 }, aspect: wgpu::TextureAspect::All },
                wgpu::TexelCopyBufferInfo { buffer: buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(256), rows_per_image: None } },
                wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            );
        } else {
            self.pixel = None;
        }
        if std::mem::take(&mut self.still) {
            if let Some((tex, _, _)) = self.texture.as_ref() {
                let stride = (size.0 * 4).div_ceil(256) * 256;
                let buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("still"), size: u64::from(stride) * u64::from(size.1), usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
                enc.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                    wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(stride), rows_per_image: None } },
                    wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                );
                self.still_buf = Some((buf, stride, size));
            }
        }
        let (_, view, _) = self.texture.as_ref().expect("made above");
        (view, fresh)
    }

    /// the library asks for a still of the stage in the next frame
    pub fn ask_still(&mut self) {
        self.still = true;
    }

    /// The still asked for, as the stage drew it: after the frame's submit.
    pub fn read_still(&mut self, device: &wgpu::Device) -> Option<image::RgbaImage> {
        let (buf, stride, (w, h)) = self.still_buf.take()?;
        let slice = buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
        rx.recv().ok()?.ok()?;
        let data = slice.get_mapped_range().ok()?;
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        for row in 0..h {
            px.extend(data[(row * stride) as usize..(row * stride + w * 4) as usize].chunks_exact(4).flat_map(|p| [p[0], p[1], p[2], 255]));
        }
        drop(data);
        buf.unmap();
        image::RgbaImage::from_raw(w, h, px)
    }

    /// the eyedropper asks for the stage's pixel at (x, y) in the next frame; none: it stops
    pub fn ask_pixel(&mut self, at: Option<(u32, u32)>) {
        self.pixel = at;
    }

    /// The pixel asked for, as the stage drew it (lit and tone-mapped, no panels over it): after the frame's submit.
    pub fn read_pixel(&mut self, device: &wgpu::Device) -> Option<[u8; 3]> {
        self.pixel?;
        let buf = self.pixel_buf.as_ref()?;
        let slice = buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
        rx.recv().ok()?.ok()?;
        let px = {
            let data = slice.get_mapped_range().ok()?;
            [data[0], data[1], data[2]]
        };
        buf.unmap();
        Some(px)
    }

}

/// the stage's own engine on the machine's outlets, and the device's cue beside it
fn own(graph: &Graph, rate: f32, volume: f32, channels: usize) -> Result<(audio::Engine, audio::Handles, deck_engine::cue::Cue)> {
    let (engine, mut h) = audio::Engine::build_for(graph, rate, volume, channels)?;
    let cue = h.core_mut().cue.take().context("the engine's cue")?;
    Ok((engine, h, cue))
}

/// A show is live (#630): the device's outlets are the show's – this machine plays nothing on them (an empty engine holds
/// the device) – and the stage's graph is a preview folded onto the cue beside it; its handles are the stage's.
fn preview(graph: &Graph, rate: f32, volume: f32, channels: usize) -> Result<(audio::Engine, audio::Handles, deck_engine::cue::Cue)> {
    let (main, mut mh) = audio::Engine::build_for(&Graph::build(&Scene::parse("")?)?, rate, volume, channels)?;
    let (folded, mut h) = audio::Engine::build_folded(graph, rate, volume)?;
    let mut cue = mh.core_mut().cue.take().context("the engine's cue")?;
    cue.play(Some(folded.into_core()));
    // the device's callback measures both: its counters are the stage's
    h.glitches = mh.glitches.clone();
    h.glitch_log = mh.glitch_log.take();
    Ok((main, h, cue))
}
