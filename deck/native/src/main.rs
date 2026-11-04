//! deck-native – the native AV app the deck boots into (CYBERDECKS.md §8, #179). One window (fullscreen; under cage on
//! the deck), a node graph from a TOML scene: video passes on wgpu, an audio graph on PipeWire / cpal, control values
//! per frame. Keys: F1 the HUD · F12 the conductor page · Ctrl+Q quit · Ctrl+Alt+Del power off (on a deck) · the tempo clock
//! (#181): T tap · + / − BPM · Space start / stop · F3 the next source · K the click. The splash (the logo) covers the start until the
//! chapter's first slice and the music are there, and the browser's start: with DECK_BROWSER set (deck-session does)
//! F12 starts that command on top of the running stage – cage shows the newest window – and the stage pauses and mutes
//! behind it until it closes; without it F12 quits with exit code 10 (deck-session's older loop opens the browser).
//! A scene with a `space` node (#188): the mouse (or a finger) turns the camera or pulls the being's 3D buttons along
//! their tethers (a double click: back to the scene's value), the wheel / ↑ ↓ zoom, ← → turn, R resets the camera; the
//! conductor's S pushes the scene's sync to the decks (#195 – also once at the start; `--sync` names what).
//! A scene that can't be played (a module this build lacks – #185 –, a broken file) never stops the stage: it shows the
//! test pattern with the reason in the HUD. `--dev` reloads the scene file and its WGSL files when they change.
//! A scene with an `outlet` node (#186, the `live` module) joins the relay as `output` of this deck's outlet
//! (deck.toml's `relay` + `outlet`, or `--relay` / `--outlet`; no relay = the hosted wss://live.scutoons.com/live/CATH,
//! #199) and shows its cells; with an outlet set and no `--scene` the stage plays the built-in `scenes/outlet.toml`.
//! A space node that joins (deck / conductor) without a relay joins the hosted one too.
//! The render quality (#220, deck-space quality.rs, the stage's quality.rs): the preset file (`--quality-file`) – a
//! level (`--quality` over it, else by the GPU) is the ceiling, the governor steps the render size, bloom, FXAA, the
//! lights and the frame rate cap below it under load and back up with headroom; F4 opens the settings page (every
//! preset's knobs, the display mode – saved, reloaded when the file changes), Shift+F4 switches the governor; a deck
//! reports its quality to the session (the conductor shows it).
use deck_native::{audio, cache, chapter, clock, control, graph, hud, modules, render, scene, splash, switcher, tempo, volume, watch};
use deck_space::quality::{self, Governor, Presets};
use deck_native::quality as settings;

use anyhow::{bail, Context, Result};
use audio::backend::{self, Choice};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
#[cfg(feature = "space")]
use winit::event::{MouseButton, MouseScrollDelta, TouchPhase};
use winit::event::StartCause;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

/// the exit code that asks deck-session for the browser (the conductor page) instead of the stage
const EXIT_BROWSER: i32 = 10;
/// the splash waits this long at most for the chapter and the music (offline without a cache: the scene anyway)
const SPLASH_MAX_S: f32 = 30.0;
/// #208: the audio glitch warning – its HUD text's start and how long it flashes after a glitch (s)
const ALERT: &str = "AUDIO!";

/// what other threads tell the event loop
#[derive(Debug)]
enum Event {
    /// the browser (DECK_BROWSER) exited
    BrowserClosed,
    /// #263: a stop signal (SIGTERM / SIGHUP / SIGINT, Ctrl+C on Windows) – the show ends like Ctrl+Q
    Stop,
}

const HELP: &str = "deck [--scene FILE|cathedral|conductor|outlet] [--audio auto|pipewire|cpal|null] [--volume 0-1] [--cache DIR]
           [--audio-host default|wasapi|asio] [--audio-device NAME] [--audio-buffer FRAMES] [--audio-thread on|off]
           [--audio-list]
           [--sets DIR] [--portraits DIR] [--token T] [--api URL] [--cdn URL] [--windowed] [--frames N]
           [--screenshot FILE] [--pointer X,Y|NODE.PORT] [--drag NODE.PORT[,DN]] [--no-hud] [--hud-bar] [--dev] [--modules]
           [--relay ws[s]://HOST[:PORT]/live/CODE] [--sync set:ID,character:ID]
           [--outlet ID] [--locale de] [--quality auto|low|medium|high|ultra[,fixed]] [--quality-file FILE] [--settings]
           [--scenes DIR] [--switcher] [--switch NAME]
  --scene      a scene TOML (default: the built-in idle scene – fireflies on black, no sound, scenes/idle.toml, #252;
               `madness` = AV madness, scenes/madness.toml); `cathedral` = the built-in
               Cathedral of decks (scenes/cathedral.toml – a deck's being), `conductor` = its conductor in the middle
               (scenes/conductor.toml), `outlet` = the built-in Live outlet (scenes/outlet.toml – the default when an
               outlet is set)
  --sync       the conductor's push (#195): what the decks fetch into their cache – sent at the start and on S
               (default: the scene's set and the characters on stage)
  --relay      the Live relay with the session code (default: deck.toml's `relay`, else the hosted
               wss://live.scutoons.com/live/CATH – when a scene joins or an outlet is set); --outlet this deck's outlet
               (deck.toml's `outlet`); --locale the language of its text layers (deck.toml's `locale`, else de)
  --audio      the backend; auto = PipeWire → cpal → null (no device: the clock and the FFT still run)
  --audio-host  cpal's host (#208): default (WASAPI shared on Windows) · wasapi · asio (a build with the asio feature);
               --audio-device a part of the output device's name (default: the host's default); --audio-buffer the
               device's buffer in frames (128 / 256 / 512 / 1024; clamped to what the device takes). Over deck.toml's
               audio_host / audio_device / audio_buffer, over the scene's [stage] audio_buffer. --audio-list prints the
               hosts and devices, then quits. --audio-thread on (default) renders on a thread of its own into a ring
               ahead of the device, with an adaptive lead; off renders in the device callback (deck.toml audio_thread)
  --cache      the library cache (default /var/lib/deck/cache on a deck, DECK_CACHE, else the user cache folder)
  --sets       a local folder of sets instead of the sync's: <id>/<id>.<quality>.glb or <id>.<quality>.glb (DECK_SETS;
               default <cache>/sets, synced from the API)
  --portraits  a local folder of the beings' heads instead of the sync's: <char>_portrait-<mood>.png|webp
               (DECK_PORTRAITS; default <cache>/portraits, synced from the API)
  --token      the conductor's session token for a space node with join = conductor (default LIVE_TOKEN; a dev relay
               takes any)
  pairing      (#180) while deck-link's run directory says the deck waits for a member, the code + QR in the corner;
               its token goes to the API once approved (DECK_LINK_DIR, default /run/deck-link; DECK_APP; DECK_TOKEN)
  --quality    the render quality's level – the highest the governor steps up to (default auto: by the GPU – software
               low, integrated high, discrete ultra; DECK_QUALITY); ,fixed = no governor. Over the preset file
  --quality-file  the presets (deck/native/README.md, Quality; default /var/lib/deck/quality.toml on a deck,
               DECK_QUALITY_FILE): F4 edits them on the HUD's settings page, Shift+F4 the governor; --settings
               starts with the page open (a check run's screenshot)
  --scenes     the scene switcher's folder of *.toml (#250; default /usr/share/deck/scenes): F5 lists the built-in scenes
               and these, arrows + Enter load one in place and write it into deck.toml's `scene`; --switcher starts with
               the list open, --switch NAME loads that scene after 30 frames (a check run)
  --show-volume  the volume overlay (#253: the volume keys' feedback) stays on, at 60 % (a check run's screenshot)
  exit codes   0 quit (Ctrl+Q) · 10 F12 without DECK_BROWSER · 75 the display was lost (the compositor went away) · 1 failed
  --frames     quit after N frames and print the measured numbers (a check run)
  --screenshot save the last frame of a --frames run as a PNG
  --pointer    put the pointer at window pixel X,Y, or on the being's button NODE.PORT (a check run's hover)
  --drag       a check run's drag: after a second press the button NODE.PORT and pull it DN (0.3) of its range out
               along its tether over half a second, then hold it
  --dev        reload the scene file and its shaders' WGSL files when they change
  --modules    print the modules (cargo features) this build carries and their kinds, then quit";

struct Args {
    scene: Option<PathBuf>,
    audio: Choice,
    /// #208: cpal's host, the device, the buffer (over deck.toml's, over the scene's)
    audio_host: Option<backend::Host>,
    audio_device: Option<String>,
    audio_buffer: Option<u32>,
    audio_thread: Option<bool>,
    volume: f32,
    cache: Option<PathBuf>,
    api: String,
    cdn: String,
    windowed: bool,
    frames: Option<u64>,
    hud: bool,
    /// #209: start with the collapsed stats bar instead of the full HUD (F1 cycles)
    hud_bar: bool,
    /// a local folder of sets (#195: else <cache>/sets, synced)
    sets: Option<PathBuf>,
    /// the heads' sprites (#188 phase 5) and the conductor's session token (phase 4)
    portraits: Option<PathBuf>,
    token: Option<String>,
    screenshot: Option<PathBuf>,
    pointer: Option<Aim>,
    /// a check run's drag on a button: its name, how far (0..1 of its range)
    drag: Option<(String, f32)>,
    dev: bool,
    /// the Live outlet (#186): --relay, --outlet, --locale over deck.toml's keys
    relay: Option<String>,
    outlet: Option<String>,
    locale: Option<String>,
    /// #195: the conductor's push (`set:<id>,character:<id>`)
    sync: Option<String>,
    /// #220: the render quality's level and the governor, the preset file
    quality: quality::Choice,
    quality_file: Option<PathBuf>,
    settings: bool,
    /// #250: the scene switcher's folder, open at the start, and a check run's switch after 30 frames
    scenes: PathBuf,
    switcher: bool,
    switch_to: Option<String>,
    /// #253: a check run's volume overlay, shown all the time
    show_volume: bool,
}

/// a check run's pointer: a pixel, or on a button of the being (it follows the button)
#[derive(Debug, Clone, PartialEq)]
enum Aim {
    At(f32, f32),
    Button(String),
}

fn args() -> Result<Args> {
    let mut a = Args {
        scene: None,
        audio: Choice::Auto,
        audio_host: None,
        audio_device: None,
        audio_buffer: None,
        audio_thread: None,
        volume: 1.0,
        cache: None,
        api: std::env::var("DECK_API").unwrap_or_else(|_| cache::API.into()),
        cdn: std::env::var("DECK_CDN").unwrap_or_else(|_| cache::CDN.into()),
        windowed: false,
        frames: None,
        hud: true,
        hud_bar: false,
        sets: std::env::var_os("DECK_SETS").map(PathBuf::from),
        portraits: std::env::var_os("DECK_PORTRAITS").map(PathBuf::from),
        token: std::env::var("LIVE_TOKEN").ok().filter(|t| !t.is_empty()),
        screenshot: None,
        pointer: None,
        drag: None,
        dev: false,
        relay: None,
        outlet: None,
        locale: None,
        sync: None,
        quality: std::env::var("DECK_QUALITY").ok().and_then(|q| quality::Choice::parse(&q)).unwrap_or_default(),
        quality_file: None,
        settings: false,
        scenes: switcher::SCENES_DIR.into(),
        switcher: false,
        switch_to: None,
        show_volume: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut val = || it.next().with_context(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--scene" => a.scene = Some(val()?.into()),
            "--audio" => {
                let v = val()?;
                a.audio = Choice::parse(&v).with_context(|| format!("--audio {v}: auto, pipewire, cpal or null"))?
            }
            "--audio-host" => {
                let v = val()?;
                a.audio_host = Some(backend::Host::parse(&v).with_context(|| format!("--audio-host {v}: default, wasapi or asio"))?)
            }
            "--audio-device" => a.audio_device = Some(val()?),
            "--audio-thread" => {
                let v = val()?;
                a.audio_thread = Some(match v.as_str() {
                    "on" | "true" | "1" => true,
                    "off" | "false" | "0" => false,
                    _ => bail!("--audio-thread {v}: on or off"),
                })
            }
            "--audio-buffer" => a.audio_buffer = Some(val()?.parse().context("--audio-buffer: frames, e.g. 256")?),
            "--audio-list" => {
                print!("{}", backend::list());
                std::process::exit(0);
            }
            "--volume" => a.volume = val()?.parse::<f32>()?.clamp(0.0, 1.0),
            "--cache" => a.cache = Some(val()?.into()),
            "--api" => a.api = val()?,
            "--cdn" => a.cdn = val()?,
            "--frames" => a.frames = Some(val()?.parse()?),
            "--sets" => a.sets = Some(val()?.into()),
            "--portraits" => a.portraits = Some(val()?.into()),
            "--token" => a.token = Some(val()?),
            "--screenshot" => a.screenshot = Some(val()?.into()),
            "--pointer" => {
                let v = val()?;
                a.pointer = Some(match v.split_once(',') {
                    Some((x, y)) => Aim::At(x.trim().parse()?, y.trim().parse()?),
                    None if v.contains('.') => Aim::Button(v),
                    None => bail!("--pointer {v}: X,Y or NODE.PORT"),
                });
            }
            "--drag" => {
                let v = val()?;
                let (name, dn) = v.split_once(',').unwrap_or((v.as_str(), "0.3"));
                a.drag = Some((name.to_string(), dn.trim().parse().with_context(|| format!("--drag {v}: NODE.PORT[,DN]"))?));
            }
            "--windowed" => a.windowed = true,
            "--no-hud" => a.hud = false,
            "--hud-bar" => a.hud_bar = true,
            "--dev" => a.dev = true,
            "--relay" => a.relay = Some(val()?),
            "--outlet" => a.outlet = Some(val()?),
            "--locale" => a.locale = Some(val()?),
            "--sync" => a.sync = Some(val()?),
            "--quality-file" => a.quality_file = Some(val()?.into()),
            "--settings" => a.settings = true,
            "--scenes" => a.scenes = val()?.into(),
            "--switcher" => a.switcher = true,
            "--show-volume" => a.show_volume = true,
            "--switch" => a.switch_to = Some(val()?),
            "--quality" => {
                let v = val()?;
                a.quality = quality::Choice::parse(&v).with_context(|| format!("--quality {v}: auto, low, medium, high or ultra (,fixed)"))?
            }
            "--modules" => {
                for m in modules::registry() {
                    let what: Vec<&str> = m.kinds().chain(m.sources.iter().copied()).collect();
                    println!("{:<9} {} – {}", m.feature, what.join(" "), m.what);
                }
                std::process::exit(0);
            }
            "-h" | "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            other => bail!("unknown argument '{other}'\n{HELP}"),
        }
    }
    Ok(a)
}

/// what the app measures (printed at exit; the numbers of CYBERDECKS.md §8's dogma)
#[derive(Default)]
struct Stats {
    frames: u64,
    intervals: Vec<f32>,
    work: Vec<f32>,
    first_frame_ms: Option<f32>,
    /// #364: the HUD's numbers – deck-core's monitor (fed each frame, its report refreshed with the HUD)
    feed: audio::glitch::MonitorFeed,
}

/// A loaded scene and everything that plays it: the graph, the audio engine on its device, the control domain, the
/// tempo clock, the loaders, the space. A reload (`--dev`) replaces it whole.
struct Show {
    graph: graph::Graph,
    scene_name: String,
    render_scale: f32,
    controls: control::Controls,
    audio: Option<backend::Running>,
    /// how the audio graph runs: "batch avx2" · "batch" (the summary of a --frames run)
    audio_build: &'static str,
    clock: Option<audio::Clock>,
    music: Vec<Arc<Mutex<String>>>,
    /// the time base's frame-thread half (clock.rs); None without audio (then the wall clock since the start)
    timebase: Option<clock::TimeBase>,
    /// the tempo clock on top of it (tempo/, #181); its I/O opens with `start`
    tempo: tempo::Tempo,
    /// #321: the show clock's side of the relay link, and the link for the clock alone (a scene without a space)
    #[cfg(all(feature = "live", feature = "space"))]
    show_link: Arc<tempo::relay::ShowLink>,
    #[cfg(all(feature = "live", feature = "space"))]
    #[allow(dead_code)]
    clock_link: Option<deck_native::space::link::Link>,
    /// the chapter nodes' loaders, until the renderer takes them
    feeds: Vec<chapter::ChapterFeed>,
    /// the master gain (the browser mutes it; it returns to --volume)
    master: Option<audio::Param>,
    /// the limiter's gain reduction (dB, f32 bits – taken each frame, #188)
    reduction: Option<Arc<std::sync::atomic::AtomicU32>>,
    /// the glitch indicator (#208) and its log (drained by the frame thread)
    glitches: Option<Arc<audio::glitch::Glitches>>,
    glitch_log: Option<rtrb::Consumer<audio::glitch::Event>>,
    /// #236: the hosts' sequencer and dispatcher threads (a scene with a `walk` / `events` node), their drop counters
    #[cfg(feature = "events")]
    events: Option<audio::dispatch::Running>,
    /// the scene's space node (#188): the camera, the being, the pointer
    space: Option<render::Space>,
    /// the Live outlet (#186): its threads, the space targets for the space node; or why it doesn't play
    #[cfg(feature = "live")]
    live: Option<deck_native::live::Live>,
    live_note: Option<String>,
    /// the deck's sync (#195): the scene's sets and portraits from the API into the cache; its HUD line
    #[cfg(feature = "sync")]
    sync: Option<deck_native::sync::DeckSync>,
    /// the audio outputs' peaks (the being's audio hoses)
    #[cfg_attr(not(feature = "space"), allow(dead_code))]
    meters: Option<Arc<audio::Meters>>,
    /// the parameters as outputs (#211): MIDI out, DMX, the strobe – and why they don't run (the HUD)
    #[cfg(feature = "outputs")]
    outputs: Option<deck_native::outputs::Outputs>,
    outputs_note: Option<String>,
    /// a gamepad's sticks and buttons over the space (#228; the feature `gamepad`, over a scene with a space)
    #[cfg(feature = "gamepad")]
    pad: Option<deck_native::pad::Pad>,
}

/// the Live outlet's settings from the CLI (deck.toml's under them – live/config.rs)
#[cfg(feature = "live")]
fn outlet_settings(args: &Args) -> deck_native::live::config::Settings {
    deck_native::live::config::Settings { relay: args.relay.clone(), outlet: args.outlet.clone(), locale: args.locale.clone() }
}

/// a relay and an outlet are set (deck.toml or the CLI): the deck is a Live outlet
fn outlet_configured(args: &Args) -> bool {
    #[cfg(feature = "live")]
    {
        let file = deck_native::live::config::Settings::read(std::path::Path::new(deck_native::live::config::DECK_TOML));
        file.merged(outlet_settings(args)).config().is_some()
    }
    #[cfg(not(feature = "live"))]
    {
        args.relay.is_some() && args.outlet.is_some()
    }
}

/// the quality a pushed set is fetched at (#195): the space node's, else low
#[cfg(feature = "sync")]
fn set_quality(graph: &graph::Graph) -> String {
    #[cfg(feature = "space")]
    if let Some((_, q)) = graph.first(graph::Kind::Space).and_then(|n| deck_native::space::set_need(graph, n)) {
        return q;
    }
    let _ = graph;
    "low".into()
}

/// the conductor's push (#195): `--sync`, else the scene's set and the characters on stage; its `scene` = the scene
/// file's name (`conductor`, `cathedral` …), else the stage's name as a slug
#[cfg(all(feature = "live", feature = "space"))]
fn push_for(args: &Args, name: &str, space: &deck_native::space::Space) -> Result<protocol::SceneSync, String> {
    let scene = scene_id(args.scene.as_deref(), name);
    let scene = scene.as_str();
    match &args.sync {
        Some(spec) => deck_native::space::link::parse_push(scene, spec),
        None => Ok(space.scene_push(scene)),
    }
}

/// a scene's id for the push (#195): the file's stem (or the built-in's name), else the stage's name as a slug
#[cfg_attr(not(all(feature = "live", feature = "space")), allow(dead_code))]
fn scene_id(file: Option<&std::path::Path>, name: &str) -> String {
    if let Some(stem) = file.and_then(|p| p.file_stem()).and_then(|s| s.to_str()) {
        return stem.to_string();
    }
    let slug: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    slug.split('-').filter(|w| !w.is_empty()).collect::<Vec<_>>().join("-")
}

/// #208: how the audio plays – the flags over deck.toml's audio keys (the machine's) over the scene's buffer
fn audio_options(args: &Args, scene: &scene::Scene) -> backend::Options {
    let deck = backend::DeckAudio::read();
    let host = args.audio_host.or_else(|| {
        let h = deck.audio_host.as_deref()?;
        backend::Host::parse(h).or_else(|| {
            tracing::warn!("deck.toml: audio_host = \"{h}\" – default, wasapi or asio; the default instead");
            None
        })
    });
    backend::Options {
        choice: args.audio,
        host: host.unwrap_or_default(),
        device: args.audio_device.clone().or(deck.audio_device),
        buffer: args.audio_buffer.or(deck.audio_buffer).or(scene.stage.audio_buffer),
        thread: args.audio_thread.or(deck.audio_thread).unwrap_or(true),
    }
}

/// the scene the arguments name: a file, `cathedral` / `conductor` / `outlet` / `madness` / `idle` (built in), the outlet
/// when one is set, or the idle scene (#252)
fn read_scene(args: &Args) -> Result<scene::Scene> {
    match &args.scene {
        Some(p) if !p.exists() && p.to_str().and_then(switcher::builtin).is_some() => scene::Scene::parse(p.to_str().and_then(switcher::builtin).unwrap_or_default()),
        Some(p) => scene::Scene::load(p),
        None if outlet_configured(args) => scene::Scene::parse(scene::OUTLET_SCENE),
        None => scene::Scene::parse(scene::IDLE_SCENE),
    }
}

/// #264: the outputs of the scene before, on their way into the next one (or out): `Show::load` takes them over when the
/// new scene runs on the same devices, else they go dark first.
#[cfg(feature = "outputs")]
type Prev = Option<deck_native::outputs::Outputs>;
#[cfg(not(feature = "outputs"))]
type Prev = ();

impl Show {
    /// #263: the end of the show: the outputs dark first (the lamps must not wait for the audio's fade), then the audio
    /// stopped (the PipeWire backend's drop fades it out, #250). The rest goes with the process.
    fn end(&mut self) {
        self.outputs_dark();
        self.audio = None;
    }

    /// the outputs stopped dark (`Outputs`' drop). Touches no window / GPU object, so it is safe on a lost display too.
    fn outputs_dark(&mut self) {
        #[cfg(feature = "outputs")]
        {
            self.outputs = None;
        }
    }

    /// Build the scene's graph (refused when it needs a module this build lacks – modules.rs) and start what plays it.
    fn load(args: &Args, scene: &scene::Scene, #[allow(unused_variables)] prev: &mut Prev) -> Result<Show> {
        modules::check(scene)?;
        let graph = graph::Graph::build(scene)?;
        tracing::info!("scene '{}': {} nodes, {} links (modules: {})", scene.stage.name, graph.nodes.len(), graph.links.len(), modules::summary());
        let tempo_cfg = tempo::config_from_graph(&graph)?;

        // audio first: its clock is the master clock, its handles feed the control domain
        let has_audio = graph.nodes.iter().any(|n| n.kind.domain() == graph::Domain::Audio);
        let mut handles = None;
        let audio = if has_audio {
            let mut factory = |rate: f32, channels: usize| {
                let (engine, h) = audio::Engine::build_for(&graph, rate, args.volume, channels)?;
                tracing::info!("audio graph: {} ({} steps, {} batches, {} device channels)", h.build.label(), engine.shape().0, engine.shape().1, engine.device_channels());
                for note in &h.notes {
                    tracing::info!("audio: {note}");
                }
                handles = Some(h);
                Ok(engine)
            };
            match backend::start(&audio_options(args, scene), &mut factory) {
                Ok(r) => Some(r),
                Err(e) => {
                    tracing::warn!("audio off: {e:#}");
                    handles = None;
                    None
                }
            }
        } else {
            None
        };
        #[cfg(any(feature = "chapter", feature = "space", feature = "sync"))]
        let cache = args.cache.clone().unwrap_or_else(cache::default_dir);
        #[cfg(any(feature = "live", feature = "space"))]
        let sets = args.sets.clone().unwrap_or_else(|| cache.join("sets"));
        // the sync (#195): what the scene needs, from the API into the cache – a kind given as a local folder is not synced
        #[cfg(feature = "sync")]
        let sync = graph.first(graph::Kind::Space).or(graph.first(graph::Kind::Outlet)).map(|_| {
            deck_native::sync::DeckSync::start(deck_native::sync::Config {
                api: args.api.clone(),
                cdn: args.cdn.clone(),
                cache: cache.clone(),
                sets: args.sets.is_none().then(|| cache.join("sets")),
                portraits: args.portraits.is_none().then(|| cache.join("portraits")),
                // #180: the deck token from deck-link's run directory (DECK_TOKEN on a dev deck); the content is public
                token: deck_link::TokenSource::from_env(),
                // #195: a pushed set at the space node's quality
                quality: set_quality(&graph),
            })
        });
        // the Live outlet (#186): its threads start with the scene; the renderer takes its planes
        #[cfg(feature = "live")]
        let (live, live_note) = match graph.first(graph::Kind::Outlet) {
            Some(_) => {
                let lib = deck_native::library::Library { api: args.api.clone(), cdn: args.cdn.clone(), cache: cache.clone() };
                let synced = cfg!(feature = "sync") && args.sets.is_none();
                match deck_native::live::start_outlet(outlet_settings(args), lib, sets.clone(), synced, (1920, 1080)) {
                    Ok(l) => (Some(l), None),
                    Err(e) => {
                        tracing::warn!("live: {e}");
                        (None, Some(e))
                    }
                }
            }
            None => (None, None),
        };
        #[cfg(not(feature = "live"))]
        let live_note: Option<String> = None;
        #[cfg(feature = "chapter")]
        let lib = deck_native::library::Library { api: args.api.clone(), cdn: args.cdn.clone(), cache: cache.clone() };
        // the chapter_music nodes' loaders (their status for the HUD and the splash)
        #[cfg(feature = "chapter")]
        let music: Vec<Arc<Mutex<String>>> = match handles.as_mut() {
            Some(h) if !h.music.is_empty() => {
                tracing::info!("library: {} · cache {}", lib.api, lib.cache.display());
                let spawn = |feed: audio::MusicFeed| {
                    let status = feed.status.clone();
                    deck_native::library::spawn_music_loader(lib.clone(), &graph.nodes[feed.node], feed);
                    status
                };
                h.music.drain(..).map(spawn).collect()
            }
            _ => Vec::new(),
        };
        #[cfg(not(feature = "chapter"))]
        let music = Vec::new();
        #[cfg(feature = "chapter")]
        let feeds: Vec<chapter::ChapterFeed> = (0..graph.nodes.len())
            .filter(|&i| graph.nodes[i].kind == graph::Kind::Chapter)
            .map(|i| chapter::spawn(lib.clone(), &graph.nodes[i], i))
            .collect();
        #[cfg(not(feature = "chapter"))]
        let feeds = Vec::new();
        let master = handles.as_ref().map(|h| h.master.clone());
        let reduction = handles.as_ref().map(|h| h.reduction.clone());
        let glitches = handles.as_ref().map(|h| h.glitches.clone());
        let glitch_log = handles.as_mut().and_then(|h| h.glitch_log.take());
        // #236: the hosts' generators on the sequencer thread, the dispatcher – before the first callback needs them
        #[cfg(feature = "events")]
        let events = match handles.as_mut().map(|h| h.start_events()).transpose() {
            Ok(r) => r.flatten(),
            Err(e) => {
                tracing::warn!("events: no thread ({e}) – the generators stay silent");
                None
            }
        };
        let meters = handles.as_ref().and_then(|h| h.meters.clone());
        // #321: the show clock's side of the relay link (the space's link, or one for the clock alone below)
        #[cfg(all(feature = "live", feature = "space"))]
        let show_link = Arc::new(tempo::relay::ShowLink::default());
        #[cfg(feature = "space")]
        let space = graph.first(graph::Kind::Space).map(|n| {
            let faces = args.portraits.clone().unwrap_or_else(|| cache.join("portraits"));
            tracing::info!("space: sets in {}, portraits in {}", sets.display(), faces.display());
            use deck_native::space::portrait::Portraits;
            // the one request for the scene goes out before the head asks for its first sprite
            #[cfg(feature = "sync")]
            if let Some(y) = &sync {
                y.prepare(deck_native::sync::needs_of(&graph, &scene.being));
            }
            // through the sync (it fetches, then the folder is read), else the local folders as they are
            #[cfg(feature = "sync")]
            let (portraits, gate) = match &sync {
                Some(y) => (y.portraits().map_or_else(|| Portraits::local(&faces), |p| Portraits::new(Box::new(p))), Some(y.gate())),
                None => (Portraits::local(&faces), None),
            };
            #[cfg(not(feature = "sync"))]
            let (portraits, gate) = (Portraits::local(&faces), None);
            #[cfg_attr(not(feature = "live"), allow(unused_mut))]
            let mut s = deck_native::space::Space::with_gate(&graph, n, &scene.being, &sets, portraits, gate);
            // phase 4: with a relay (deck.toml or --relay) the being joins the session as its `join` says
            #[cfg(feature = "live")]
            {
                let relay = outlet_settings(args).relay.or_else(|| deck_native::live::config::Settings::read(std::path::Path::new(deck_native::live::config::DECK_TOML)).relay);
                use deck_native::space::Connect;
                // #195: a deck answers the conductor's pushes through its sync
                #[cfg(feature = "sync")]
                let hook = sync.clone().map(|y| Box::new(y) as Box<dyn deck_native::space::link::Syncs>);
                #[cfg(not(feature = "sync"))]
                let hook = None;
                s.connect(relay.as_deref(), args.token.clone(), Some(scene.stage.name.clone()), hook, Some(show_link.clone()));
                // the conductor pushes the scene's sync once joined (decks that come later find it in their snapshot)
                if s.join == deck_native::space::Join::Conductor {
                    match push_for(args, &scene.stage.name, &s) {
                        Ok(p) => {
                            s.push_sync(p);
                        }
                        Err(e) => tracing::warn!("{e}"),
                    }
                }
            }
            s
        });
        #[cfg(not(feature = "space"))]
        let space = None;
        let audio_build = handles.as_ref().map_or("", |h| h.build.label());
        let clock = handles.as_ref().map(|h| h.clock.clone());
        let timebase = clock.as_ref().map(|c| clock::TimeBase::new(c.rate));
        let controls = control::Controls::new(&graph, handles.as_mut().map(|h| h.core_mut()));
        // the tempo clock: the engine's shared map (or its own without audio)
        let rate = clock.as_ref().map_or(48_000.0, |c| c.rate as f64);
        let shared = handles.as_ref().map(|h| h.tempo.clone()).unwrap_or_default();
        // #321: a clock on the show clock, one that leads, or MIDI clock in with a relay set joins the relay – through the
        // space's link when the scene has one, else a link for the clock alone (no being)
        #[cfg(all(feature = "live", feature = "space"))]
        let clock_link = {
            let linked = space.as_ref().is_some_and(|s| s.join != deck_native::space::Join::Alone);
            let set = outlet_settings(args).relay.or_else(|| deck_native::live::config::Settings::read(std::path::Path::new(deck_native::live::config::DECK_TOML)).relay).filter(|r| !r.trim().is_empty());
            let wants = tempo_cfg.source == tempo::Source::Relay || tempo_cfg.lead || (tempo_cfg.source == tempo::Source::Midi && set.is_some());
            if linked || !wants {
                None
            } else {
                let relay = set.unwrap_or_else(|| deck_native::live::config::HOSTED_RELAY.to_owned());
                let link = deck_native::space::link::Link::start(&relay, deck_native::space::Join::Deck, None, Some(scene.stage.name.clone()), None, None, Some(show_link.clone()));
                link.map_err(|e| tracing::warn!("tempo: no relay link for the show clock: {e}")).ok()
            }
        };
        let tempo = tempo::Tempo::new(tempo_cfg, shared, rate);
        // #211: the scene's [outputs] – a wrong table or a missing device never stops the scene: the HUD says why
        #[cfg(feature = "outputs")]
        let (outputs, outputs_note) = match &scene.outputs {
            Some(table) => {
                let dir = args.scene.as_deref().filter(|p| p.exists()).and_then(|p| p.parent());
                let switch = deck_native::outputs::Switch { transition: scene.stage.transition, secs: scene.stage.transition_secs };
                match deck_native::outputs::Outputs::start_after(table, dir, &graph, tempo.shared(), prev, switch) {
                    Ok(o) => (o, None),
                    Err(e) => {
                        tracing::error!("outputs off: {e:#}");
                        (None, Some(format!("{e:#}")))
                    }
                }
            }
            None => (None, None),
        };
        #[cfg(not(feature = "outputs"))]
        let outputs_note = scene.outputs.as_ref().map(|_| {
            tracing::warn!("this build of the stage does not carry the module 'outputs' – the scene's [outputs] is off");
            "this build does not carry the module 'outputs' (--features outputs)".to_owned()
        });
        // #228: a gamepad drives the being's sticks, over the scene's space (a pad system that can't start only says so)
        #[cfg(feature = "gamepad")]
        let pad = space.as_ref().map(|s| deck_native::pad::Pad::new(&graph, s));
        Ok(Show {
            scene_name: scene.stage.name.clone(),
            render_scale: scene.stage.render_scale,
            graph,
            controls,
            audio,
            audio_build,
            clock,
            music,
            timebase,
            tempo,
            #[cfg(all(feature = "live", feature = "space"))]
            show_link,
            #[cfg(all(feature = "live", feature = "space"))]
            clock_link,
            feeds,
            master,
            reduction,
            glitches,
            glitch_log,
            #[cfg(feature = "events")]
            events,
            space,
            #[cfg(feature = "live")]
            live,
            live_note,
            #[cfg(feature = "sync")]
            sync,
            meters,
            #[cfg(feature = "outputs")]
            outputs,
            outputs_note,
            #[cfg(feature = "gamepad")]
            pad,
        })
    }

    /// The scene the arguments name, or – when it can't be played – the test pattern and the reason.
    fn load_or_explain(args: &Args) -> Result<(Show, Option<String>)> {
        match read_scene(args).and_then(|s| Show::load(args, &s, &mut Prev::default())) {
            Ok(show) => Ok((show, None)),
            Err(e) => {
                tracing::error!("the scene can't be played – the test pattern instead: {e:#}");
                let fallback = scene::Scene::parse(scene::TEST_PATTERN_SCENE)?;
                Ok((Show::load(args, &fallback, &mut Prev::default())?, Some(format!("{e:#}"))))
            }
        }
    }

    /// the shader nodes' WGSL files (what `--dev` watches besides the scene)
    fn shader_files(&self) -> Vec<(usize, PathBuf)> {
        let g = &self.graph;
        (0..g.nodes.len()).filter(|&i| g.nodes[i].kind == graph::Kind::Shader).filter_map(|i| Some((i, g.nodes[i].str("wgsl")?.into()))).collect()
    }

    /// the post stack's WGSL files in this checkout (#204: the space node's `post` passes and their prelude; a deck
    /// without the sources watches nothing)
    #[cfg(feature = "space")]
    fn post_files(&self) -> Vec<(Watched, PathBuf)> {
        use deck_native::space::post::{self, Chain};
        let Some(n) = self.graph.first(graph::Kind::Space) else { return Vec::new() };
        let (chain, _) = Chain::of(&self.graph.nodes[n]);
        if chain.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<(Watched, PathBuf)> = chain.effects().map(|e| (Watched::Post(Some(e)), post::source_path(e.file()))).collect();
        out.push((Watched::Post(None), post::source_path("common.wgsl")));
        out.retain(|(_, p)| p.exists());
        out
    }
}

/// what `--dev` watches: the scene file, a shader node's WGSL file
#[derive(Debug, Clone, Copy, PartialEq)]
enum Watched {
    Scene,
    Shader(usize),
    /// a post pass's WGSL (#204), None = their prelude (every pass reloads)
    #[cfg(feature = "space")]
    Post(Option<deck_native::space::post::Effect>),
}

struct App {
    args: Args,
    started: Instant,
    show: Show,
    /// why the scene isn't playing (or the last reload failed) – the HUD shows it on top
    error: Option<String>,
    /// `--dev`: the files and what each one is
    watch: Option<(watch::Watch, Vec<Watched>)>,
    window: Option<Arc<Window>>,
    renderer: Option<render::Renderer>,
    hud: hud::Hud,
    hud_at: f32,
    last: Option<Instant>,
    /// the start on clock::now_ns's clock (the time base without audio)
    t0_ns: u64,
    /// this frame's view of the time base: the audio position heard at the frame's moment, what is rendered, the line
    moment: clock::Moment,
    stats: Stats,
    mods: ModifiersState,
    exit: i32,
    splash: splash::Fade,
    /// the pairing card (#180): deck-link's state, read every 2 s (`pairing_at`, the frame time); `None` = no card
    pairing: Option<deck_native::pairing::Card>,
    pairing_at: f32,
    /// the browser on top: since when; `paused` once it has the focus (the stage stops drawing)
    browser: Option<Instant>,
    paused: bool,
    proxy: EventLoopProxy<Event>,
    /// the pointer's look shown (#188: a hand over a button)
    #[cfg(feature = "space")]
    cursor: deck_native::space::Cursor,
    /// a check run's --drag: where the press was and the tether's way on the screen
    #[cfg(feature = "space")]
    drag_from: Option<((f32, f32), (f32, f32))>,
    /// #220: the render quality's governor (made with the first renderer – its default comes from the GPU), the GPU, the
    /// preset file and its presets, the settings page (F4), its status line, the display's sizes (largest first)
    governor: Option<Governor>,
    gpu: quality::Gpu,
    qfile: settings::QualityFile,
    presets: Presets,
    page: settings::Page,
    /// #250: the scene switcher (F5)
    switcher: switcher::Switcher,
    /// #250: the fade through black while a picked scene loads
    trans: Option<switcher::Transition>,
    /// #264: the lamps of a scene without outputs, fading to 0 – dropped (dark) when the time is up
    #[cfg(feature = "outputs")]
    dying: Option<(deck_native::outputs::Outputs, Instant)>,
    /// #253: the hardware volume keys (wpctl on the default sink) and their overlay; `vol_forced` = the HUD is off and
    /// the bar is borrowed to show it
    volume: volume::Volume,
    vol_overlay: volume::Overlay,
    vol_forced: bool,
    quality_note: String,
    modes: Vec<(u32, u32)>,
    /// #211: the outputs' MIDI for the conductor this frame (reused)
    #[cfg(feature = "outputs")]
    conductor_midi: Vec<[u8; 3]>,
}

impl App {
    fn time(&self) -> f32 {
        self.moment.seconds() as f32
    }

    /// the time base (clock.rs, CYBERDECKS.md §8 "Two clocks"): the audio position heard at this frame's moment – the
    /// device's block stamps through a DLL, interpolated between blocks, minus the output latency, never backwards (the
    /// X1's SOF card hands over whole periods – counting rendered frames stepped the picture). Before the first block
    /// it stands at 0; no audio: the wall clock since the start.
    fn advance_time(&mut self) {
        let now = clock::now_ns();
        self.moment = match (&self.show.clock, self.show.timebase.as_mut()) {
            (Some(c), Some(tb)) => {
                tb.poll(&c.stamp);
                tb.moment(now, c.rendered()).unwrap_or_else(|| clock::Moment::wall(now, now, c.rate as f64))
            }
            _ => clock::Moment::wall(now, self.t0_ns, 48_000.0),
        };
    }

    /// how many samples the audio callback has rendered past the heard moment (the FFT looks back by it)
    fn lead(&self) -> usize {
        self.moment.lead()
    }

    /// still loading what the splash waits for: a chapter's first slice, a music track
    fn loading(&self) -> bool {
        if self.started.elapsed().as_secs_f32() > SPLASH_MAX_S {
            return false;
        }
        let music = self.show.music.iter().any(|s| {
            let s = s.lock().unwrap();
            s.is_empty() || ["waiting", "loading", "fetching", "decoding"].iter().any(|p| s.starts_with(p))
        });
        music || self.renderer.as_ref().is_some_and(|r| !r.slides_ready())
    }

    /// the files `--dev` watches for this show
    fn watch_files(&self) -> Option<(watch::Watch, Vec<Watched>)> {
        if !self.args.dev {
            return None;
        }
        let mut files = Vec::new();
        let mut what = Vec::new();
        if let Some(p) = self.args.scene.as_ref().filter(|p| p.exists()) {
            files.push(p.clone());
            what.push(Watched::Scene);
        }
        for (node, path) in self.show.shader_files() {
            files.push(path);
            what.push(Watched::Shader(node));
        }
        #[cfg(feature = "space")]
        for (w, path) in self.show.post_files() {
            files.push(path);
            what.push(w);
        }
        let w = watch::Watch::new(files);
        tracing::info!("dev: watching {}", w.paths().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", "));
        Some((w, what))
    }

    /// `--dev`: a changed scene reloads the show, a changed WGSL file its pass
    fn poll_watch(&mut self) {
        let Some((w, what)) = self.watch.as_mut() else { return };
        let changed: Vec<Watched> = w.changed().into_iter().map(|i| what[i]).collect();
        if changed.contains(&Watched::Scene) {
            self.reload();
            return;
        }
        let nodes: Vec<usize> = changed.iter().filter_map(|w| if let Watched::Shader(n) = w { Some(*n) } else { None }).collect();
        if let (false, Some(r)) = (nodes.is_empty(), self.renderer.as_mut()) {
            let errors = r.reload_shaders(&self.show.graph, &nodes);
            self.error = (!errors.is_empty()).then(|| errors.join("; "));
            self.hud_at = -1.0;
        }
        #[cfg(feature = "space")]
        {
            use deck_native::space::post::Effect;
            // the prelude changed: every listed pass; else the passes whose file changed
            let all = what.iter().filter_map(|w| if let Watched::Post(Some(e)) = w { Some(*e) } else { None });
            let effects: Vec<Effect> = if changed.contains(&Watched::Post(None)) {
                all.collect()
            } else {
                changed.iter().filter_map(|w| if let Watched::Post(Some(e)) = w { Some(*e) } else { None }).collect()
            };
            if let (false, Some(r)) = (effects.is_empty(), self.renderer.as_mut()) {
                let errors = r.reload_post(&effects);
                self.error = (!errors.is_empty()).then(|| errors.join("; "));
                self.hud_at = -1.0;
            }
        }
    }

    /// The scene file again (`--dev`): checked first – a scene that doesn't build keeps the running show and says why –,
    /// then the old show stops (its audio device, its sources' ports, its renderer's surface) and the new one starts.
    fn reload(&mut self) {
        self.hud_at = -1.0;
        let scene = match read_scene(&self.args).and_then(|s| modules::check(&s).and_then(|_| graph::Graph::build(&s)).map(|_| s)) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("reload: {e:#} – the running scene stays");
                self.error = Some(format!("{e:#}"));
                return;
            }
        };
        self.renderer = None;
        self.show.audio = None;
        // #264: the lamps of the scene before ride along into the new one (hand-over), or fade out with it
        #[cfg(feature = "outputs")]
        let mut prev: Prev = self.show.outputs.take();
        #[cfg(not(feature = "outputs"))]
        let mut prev: Prev = ();
        let (show, error) = match Show::load(&self.args, &scene, &mut prev) {
            Ok(s) => (s, None),
            Err(e) => match scene::Scene::parse(scene::TEST_PATTERN_SCENE).and_then(|s| Show::load(&self.args, &s, &mut prev)) {
                Ok(s) => (s, Some(format!("{e:#}"))),
                Err(e2) => {
                    tracing::error!("reload: {e:#}; the test pattern: {e2:#}");
                    self.exit = 1;
                    return;
                }
            },
        };
        self.show = show;
        self.show.tempo.start();
        #[cfg(feature = "outputs")]
        if let Some(mut o) = prev {
            // no outputs in the new scene: `fade` lets the lamps down over its time, `cut` / `blackout` end them now
            if scene.stage.transition == scene::SceneTransition::Fade {
                o.fade_out(scene.stage.transition_secs);
                self.dying = Some((o, Instant::now() + std::time::Duration::from_secs_f32(scene.stage.transition_secs + 0.3)));
            }
        }
        self.error = error;
        self.watch = self.watch_files();
        self.last = None;
        if let Some(w) = self.window.clone() {
            self.start_renderer(&w);
        }
        tracing::info!("reloaded '{}'", self.show.scene_name);
    }

    /// the playing scene as the switcher names it (`--scene`'s value; none = the default)
    fn current_scene(&self) -> Option<&str> {
        self.args.scene.as_deref().and_then(|p| p.to_str())
    }

    /// #250: F5 – the list on the scenes folder, the cursor on the playing scene (the settings page makes room)
    fn open_switcher(&mut self) {
        self.page.open = false;
        let current = self.current_scene().map(str::to_owned);
        self.switcher.open(&self.args.scenes, current.as_deref());
        self.switcher.status.clear();
    }

    /// #250: Enter on a scene – loaded in place like a `--dev` reload (the window and the Live link's process stay; a
    /// scene that doesn't build keeps the old one and the list says why), then written into deck.toml for the next boot
    fn switch(&mut self, value: &str) {
        if self.trans.is_some() {
            return;
        }
        // checked first: a scene that doesn't build keeps the old one playing, nothing fades
        let old_scene = self.args.scene.replace(value.into());
        let checked = read_scene(&self.args).and_then(|s| modules::check(&s).and_then(|_| graph::Graph::build(&s)).map(|_| ()));
        self.args.scene = old_scene;
        if let Err(e) = checked {
            self.switcher.status = format!("NOT LOADED: {e:#}");
            return;
        }
        // the picture fades to black and the sound out as long (PipeWire), then `load_switch`
        if let Some(a) = &self.show.audio {
            a.fade_out();
        }
        self.trans = Some(switcher::Transition::new(value));
        self.switcher.status = format!("FADING TO {}", value.rsplit(['/', '\\']).next().unwrap_or(value).to_uppercase());
    }

    /// #250: the black is full – the old show stops, the new one is built, deck.toml remembers it
    fn load_switch(&mut self, value: &str) {
        let (old_scene, old_error) = (self.args.scene.clone(), self.error.take());
        let started = Instant::now();
        self.args.scene = Some(value.into());
        self.reload();
        if let Some(e) = self.error.take() {
            self.args.scene = old_scene;
            self.error = old_error;
            self.switcher.status = format!("NOT LOADED: {e}");
            return;
        }
        self.switcher.status = match switcher::remember(&switcher::deck_toml(), value) {
            Ok(()) => format!("LOADED {} IN {:.1}S", value.rsplit(['/', '\\']).next().unwrap_or(value), started.elapsed().as_secs_f32()),
            Err(e) => format!("LOADED, NOT SAVED: {e}"),
        };
        tracing::info!("switcher: {value} ({})", self.switcher.status);
    }

    /// #220: the governor's rung into the renderer (a resize only when the scale changed); a deck tells the session
    fn apply_quality(&mut self) {
        let Some(g) = &self.governor else { return };
        if let Some(r) = self.renderer.as_mut() {
            r.set_quality(*g.rung(), self.show.render_scale);
        }
        tracing::info!("quality: {} {:.2} (frames {:.1} ms, the level {}, cap {} fps)", g.level().name(), g.scale(), g.frame_ms(), g.ceiling().name(), g.rung().fps);
    }

    /// #220: the presets into the governor (the command line's level / ,fixed over the file's), the display mode
    fn apply_presets(&mut self) {
        let mut eff = self.presets.clone();
        self.args.quality.over(&mut eff);
        if let Some(g) = self.governor.as_mut() {
            g.set_presets(&eff, self.gpu);
        }
        self.apply_quality();
        self.apply_display();
    }

    /// #220: the display mode the presets name – exclusive fullscreen at that size (the highest refresh it has); none,
    /// or one the monitor lacks: the desktop's (borderless). A windowed stage keeps its window. Under a Wayland
    /// compositor (cage on the deck) winit has no exclusive mode: the desktop's then, said in the page's status.
    fn apply_display(&mut self) {
        let Some(w) = self.window.clone().filter(|_| !self.args.windowed) else { return };
        let want = self.presets.display_mode();
        let mode = want.and_then(|(mw, mh)| {
            w.current_monitor()?.video_modes().filter(|m| (m.size().width, m.size().height) == (mw, mh)).max_by_key(|m| m.refresh_rate_millihertz())
        });
        let target = match &mode {
            Some(m) => Fullscreen::Exclusive(m.clone()),
            None => Fullscreen::Borderless(None),
        };
        if want.is_some() && mode.is_none() {
            self.quality_note = format!("NO {} MODE: DESKTOP", self.presets.display.clone().unwrap_or_default().to_uppercase());
        }
        let same = match (w.fullscreen(), &target) {
            (Some(Fullscreen::Exclusive(a)), Fullscreen::Exclusive(b)) => a == *b,
            (Some(Fullscreen::Borderless(_)), Fullscreen::Borderless(_)) => true,
            _ => false,
        };
        if !same {
            tracing::info!("display: {}", mode.as_ref().map_or("the desktop's".into(), |m| format!("{}x{} @ {} Hz", m.size().width, m.size().height, m.refresh_rate_millihertz() / 1000)));
            w.set_fullscreen(Some(target));
        }
    }

    /// #220: a key on the open settings page – applied at once and saved
    fn page_key(&mut self, k: settings::Key) {
        let h = self.window.as_ref().map_or(1080, |w| w.inner_size().height);
        if self.page.key(k, &mut self.presets, &self.modes, h) {
            self.quality_note = match self.qfile.save(&self.presets) {
                Ok(()) => format!("SAVED {}", self.qfile.path.file_name().map(|n| n.to_string_lossy().to_uppercase()).unwrap_or_default()),
                Err(e) => format!("NOT SAVED: {e}"),
            };
            self.apply_presets();
            self.hud_at = -1.0;
        }
    }

    /// #220: the preset file changed from outside (an editor, deck-set): its presets now
    fn watch_quality(&mut self) {
        let now = self.started.elapsed().as_secs_f32();
        if let Some((p, problem)) = self.qfile.changed(now) {
            tracing::info!("quality: {} changed", self.qfile.path.display());
            self.quality_note = problem.map_or_else(|| "RELOADED".into(), |e| e.to_uppercase());
            self.presets = p;
            self.apply_presets();
        }
    }

    /// the renderer for the current show on `window` (the space's view and the pointer with it)
    fn start_renderer(&mut self, window: &Arc<Window>) {
        #[cfg(feature = "space")]
        if let Some(s) = self.show.space.as_mut() {
            let size = window.inner_size();
            s.set_view(size.width as f32, size.height as f32);
            if let Some(Aim::At(x, y)) = self.args.pointer {
                s.pointer_moved(x, y, &mut self.show.controls);
            }
        }
        // the kiosk hides the pointer – unless the scene has a space to point into
        window.set_cursor_visible(self.args.windowed || self.show.space.is_some());
        let feeds = std::mem::take(&mut self.show.feeds);
        match render::Renderer::new(window.clone(), &self.show.graph, self.show.render_scale, feeds, self.trans.is_some()) {
            #[allow(unused_mut)]
            Ok(mut r) => {
                #[cfg(feature = "live")]
                if let Some(l) = self.show.live.as_mut() {
                    if let Some(rx) = l.frames.take() {
                        r.set_outlet(rx);
                    }
                    let size = window.inner_size();
                    let k = self.show.render_scale.clamp(0.25, 1.0);
                    l.resize((size.width as f32 * k) as u32, (size.height as f32 * k) as u32);
                }
                // #220: the level by the GPU (unless chosen), kept over a reload; the renderer at its rung
                let gpu = match r.adapter.device_type {
                    wgpu::DeviceType::Cpu => quality::Gpu::Software,
                    wgpu::DeviceType::IntegratedGpu => quality::Gpu::Integrated,
                    wgpu::DeviceType::DiscreteGpu => quality::Gpu::Discrete,
                    _ => quality::Gpu::Other,
                };
                self.gpu = gpu;
                let mut eff = self.presets.clone();
                self.args.quality.over(&mut eff);
                let g = self.governor.get_or_insert_with(|| Governor::with(&eff, gpu));
                let size = window.inner_size();
                g.set_display(size.width, size.height);
                tracing::info!("quality: {} ({}, the governor {})", g.ceiling().name(), if eff.level.is_some() { "chosen" } else { "by the GPU" }, if g.auto() { "on" } else { "off" });
                r.set_quality(*g.rung(), self.show.render_scale);
                self.renderer = Some(r)
            }
            Err(e) => {
                tracing::error!("render: {e:#}");
                self.exit = 1;
            }
        }
        window.request_redraw();
    }

    /// F12: the conductor page (a locked-down browser window) on top of the stage (DECK_BROWSER), else exit code 10 for deck-session
    fn open_browser(&mut self, el: &ActiveEventLoop) {
        if self.browser.is_some() {
            return;
        }
        let Some(cmd) = std::env::var("DECK_BROWSER").ok().filter(|c| !c.trim().is_empty()) else {
            self.exit = EXIT_BROWSER;
            el.exit();
            return;
        };
        tracing::info!("browser: {cmd}");
        match std::process::Command::new("/bin/sh").arg("-c").arg(&cmd).spawn() {
            Ok(mut child) => {
                let proxy = self.proxy.clone();
                std::thread::spawn(move || {
                    let st = child.wait();
                    tracing::info!("browser closed ({st:?})");
                    let _ = proxy.send_event(Event::BrowserClosed);
                });
                self.browser = Some(Instant::now());
                if let Some(m) = &self.show.master {
                    m.set(0.0);
                }
            }
            Err(e) => tracing::error!("browser: {cmd}: {e}"),
        }
    }

    /// Ctrl+Alt+Del: a clean power-off (logind lets the active local session do it); the logo stays up meanwhile.
    /// Not in a --windowed stage (a desktop).
    fn power_off(&mut self) {
        if self.args.windowed {
            return;
        }
        tracing::info!("power off (Ctrl+Alt+Del)");
        self.browser.get_or_insert_with(Instant::now); // the splash comes up and stays
        if let Some(m) = &self.show.master {
            m.set(0.0);
        }
        if let Err(e) = std::process::Command::new("systemctl").arg("poweroff").spawn() {
            tracing::error!("systemctl poweroff: {e}");
        }
    }

    fn redraw(&mut self, el: &ActiveEventLoop) {
        if self.paused {
            return; // the browser has the screen; the next frame comes when it closes
        }
        self.poll_watch();
        if self.stats.frames >= 30 {
            if let Some(name) = self.args.switch_to.take() {
                self.switch(&name);
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
                return;
            }
        }
        if self.exit != 0 {
            el.exit();
            return;
        }
        // #220: the rung's frame rate cap – an early frame waits (no draw, no spin: the loop sleeps until it is due)
        if let (Some(g), Some(l)) = (&self.governor, self.last) {
            let since = l.elapsed().as_secs_f32() * 1000.0;
            if !g.due(since) {
                let wait = (g.rung().interval_ms() - since - 1.5).max(0.0);
                el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + std::time::Duration::from_secs_f32(wait / 1000.0)));
                return;
            }
        }
        self.watch_quality();
        let now = Instant::now();
        let wall_ms = self.started.elapsed().as_secs_f64() * 1000.0;
        let dt = self.last.map(|l| (now - l).as_secs_f32()).unwrap_or(1.0 / 60.0).min(0.25);
        if let Some(l) = self.last {
            let ms = (now - l).as_secs_f32() * 1000.0;
            push_capped(&mut self.stats.intervals, ms);
            self.hud.push_frame(ms);
            // #220: the governor sees every interval (a stall over 250 ms is a hitch to it) and the audio's glitches
            let wall = self.started.elapsed().as_secs_f64();
            if self.governor.as_mut().is_some_and(|g| g.frame(wall, ms, self.stats.feed.seen)) {
                self.apply_quality();
            }
        }
        self.last = Some(now);
        self.advance_time();
        #[cfg(all(feature = "live", feature = "space"))]
        self.show.show_link.feed(&mut self.show.tempo);
        let beat = self.show.tempo.frame(&self.moment);
        #[cfg(all(feature = "live", feature = "space"))]
        self.show.show_link.after(&mut self.show.tempo);
        self.show.controls.update(dt, self.lead(), &beat);
        #[cfg(feature = "outputs")]
        if self.dying.as_ref().is_some_and(|(_, until)| Instant::now() >= *until) {
            self.dying = None;
        }
        // #211: the values to the outputs' runner; its MIDI for the conductor to the relay link
        #[cfg(feature = "outputs")]
        if let Some(o) = &self.show.outputs {
            o.update(&self.show.controls);
            self.conductor_midi.clear();
            o.conductor_midi(&mut self.conductor_midi);
            #[cfg(feature = "space")]
            if let Some(s) = &self.show.space {
                s.send_midi(&self.conductor_midi);
            }
        }
        let time = self.time();
        // a Live cell's 3D set: the space node's set and spot (#186)
        #[cfg(all(feature = "live", feature = "space"))]
        if let (Some(l), Some(s)) = (self.show.live.as_ref(), self.show.space.as_mut()) {
            while let Ok(t) = l.space.try_recv() {
                s.show(t.set.as_deref(), t.spot.as_deref());
            }
        }
        #[cfg(feature = "space")]
        self.check_hand();
        // #188: the limiter's gain reduction since the last frame (the HUD holds it a moment; the being's red face); #208:
        // the audio glitches (logged, counted, the warning)
        let reduction = self.show.reduction.as_ref().map_or(0.0, |r| f32::from_bits(r.swap(0, std::sync::atomic::Ordering::Relaxed)));
        // #208: each glitch logged (its numbers from the callback's ring); #364: the monitor fed
        if let Some(log) = self.show.glitch_log.as_mut() {
            while let Ok(e) = log.pop() {
                tracing::warn!("{} (at {:.3} s)", e.describe(), e.at_ns.saturating_sub(self.t0_ns) as f64 / 1e9);
            }
        }
        let fps = self.governor.as_ref().map_or(0.0, |g| g.rung().fps as f32);
        self.stats.feed.feed(wall_ms, self.show.glitches.as_deref(), reduction, self.show.clock.as_ref().map_or(48_000.0, |c| c.rate), fps);
        #[cfg(feature = "space")]
        if let Some(s) = self.show.space.as_mut() {
            let meters = self.show.meters.as_deref();
            s.being.strain(dt, reduction);
            s.update(dt, time, &mut self.show.controls, &beat, &|n, p| meters.map_or(0.0, |m| m.level(n, p)));
            // #220: a deck reports its quality to the session (at once on a change, else at most every 2 s)
            if let Some(q) = self.governor.as_mut().and_then(|g| g.report_due(self.started.elapsed().as_secs_f64())) {
                s.report_quality(q.wire());
            }
            // the pointer's look: a hand over a button, a closed one on it, "no" over a linked one
            let c = s.cursor();
            if c != self.cursor {
                self.cursor = c;
                if let Some(w) = &self.window {
                    use deck_native::space::Cursor;
                    use winit::window::CursorIcon;
                    w.set_cursor(match c {
                        Cursor::Default => CursorIcon::Default,
                        Cursor::Grab => CursorIcon::Grab,
                        Cursor::Grabbing => CursorIcon::Grabbing,
                        Cursor::Locked => CursorIcon::NotAllowed,
                    });
                }
            }
        }
        // a scene that can't be played is explained even without the HUD
        if let Some(l) = self.volume.poll() {
            self.vol_overlay.show(time, l);
            self.hud_at = -1.0;
        }
        if self.args.show_volume {
            self.vol_overlay.show(time, volume::Level { volume: 0.6, muted: false });
        }
        let vol = self.vol_overlay.text(time);
        // #253: with the HUD off the bar carries the overlay for its second, then goes away again
        if vol.is_some() && self.hud.mode == hud::Mode::Off {
            self.hud.mode = hud::Mode::Bar;
            self.vol_forced = true;
        } else if vol.is_none() && self.vol_forced {
            self.hud.mode = hud::Mode::Off;
            self.vol_forced = false;
            self.hud_at = -1.0;
        }
        let hud_on = self.args.hud || self.error.is_some() || self.page.open || self.switcher.open || vol.is_some();
        if hud_on && time - self.hud_at >= 0.25 || self.stats.frames == 0 || self.hud_at < 0.0 {
            self.hud_at = time;
            self.stats.feed.report = self.stats.feed.monitor.report(wall_ms);
            // #220: the settings page instead of the HUD's lines, its cursor's line lit
            let (lines, cursor) = if self.switcher.open {
                let (l, c) = self.switcher.lines(self.current_scene());
                (l, Some(c))
            } else if self.page.open {
                let now = self.governor.as_ref().map_or(String::new(), |g| format!("{} {:.2} {:.0}MS", g.level().name().to_uppercase(), g.scale(), g.frame_ms()));
                let (l, c) = self.page.lines(&self.presets, &now, &self.modes, &self.quality_note);
                (l, Some(c))
            } else {
                (self.hud_lines(), None)
            };
            let t = self.show.tempo.now();
            let row = lines.iter().position(|l| l.starts_with("BPM"));
            self.hud.set_beat(row.map(|row| hud::Beat { row, beat: t.pos.beat_in_bar, per_bar: t.per_bar, running: t.running }));
            // #208: the glitch warning flashes at 2 Hz (the lines refresh every 0.25 s)
            let alert = lines.iter().position(|l| l.starts_with(ALERT));
            let lit = (self.started.elapsed().as_secs_f32() * 4.0) as u32 % 2 == 0;
            self.hud.set_alert(match cursor {
                Some(row) => Some(hud::Alert { row, lit: true }),
                None => alert.map(|row| hud::Alert { row, lit }),
            });
            let mut lines = lines;
            let mut items = self.bar_items();
            if let Some(v) = &vol {
                lines.insert(0, v.clone());
                items.insert(0, hud::Item { text: v.clone(), level: hud::Level::Ok });
            }
            self.hud.set_lines(lines);
            self.hud.set_items(items);
        }
        if self.error.is_some() || self.page.open || self.switcher.open {
            self.hud.mode = hud::Mode::Full;
        }
        match self.hud.mode {
            hud::Mode::Full => self.hud.raster(),
            // #209: the collapsed bar at the screen's width
            hud::Mode::Bar => self.hud.raster_bar(self.renderer.as_ref().map_or(hud::W, |r| r.bar_width())),
            hud::Mode::Off => {}
        }
        let cover = self.loading() || self.browser.is_some();
        self.splash.step(cover, dt);
        // #250: a scene change fades through a black veil (no logo): out, the load behind it, in
        let mut splash = (self.splash.alpha, cover);
        let ready = !self.loading();
        if let Some(t) = self.trans.as_mut() {
            let (step, done) = t.step(dt, ready);
            splash = (t.veil, false);
            if done {
                self.trans = None;
            } else if step == switcher::Step::Load {
                let to = t.to.clone();
                self.load_switch(&to);
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
                return;
            }
        }
        let veil = self.trans.is_some();
        let Some(r) = self.renderer.as_mut() else { return };
        r.veil_only = veil;
        // #180: while the deck waits for a member, the code + QR in the corner (deck-link's run directory, a small file)
        if time - self.pairing_at >= 2.0 || time < self.pairing_at {
            self.pairing_at = time;
            let state = deck_link::pairing::read_state(&deck_link::pairing::run_dir());
            let card = deck_native::pairing::Card::of(state.as_ref(), &deck_link::pairing::app_url());
            if card != self.pairing {
                r.set_pairing(card.as_ref().map(|c| c.raster()).as_deref());
                self.pairing = card;
            }
        }
        let shown = (hud_on && self.hud.visible()).then_some(&self.hud);
        if self.args.frames.is_some_and(|n| self.stats.frames + 1 >= n) {
            r.capture = self.args.screenshot.clone();
        }
        // #228: the pad's sticks and buttons, read once per frame (no pad: nothing)
        #[cfg(feature = "gamepad")]
        if let (Some(pad), Some(space)) = (self.show.pad.as_mut(), self.show.space.as_mut()) {
            pad.poll(space, &mut self.show.controls);
        }
        let before = (now.elapsed().as_secs_f32()) * 1000.0;
        match r.frame(&self.show.graph, &self.show.controls, time, dt, shown, splash, self.show.space.as_mut()) {
            // the frame thread's work: controls + HUD before the render, the render after the surface's vsync wait
            Ok(ms) => {
                push_capped(&mut self.stats.work, before + ms);
                self.stats.feed.monitor.frame(wall_ms, before + ms, true);
            }
            Err(e) => {
                tracing::error!("render: {e:#}");
                el.exit();
                self.exit = 1;
                return;
            }
        }
        self.stats.frames += 1;
        if self.stats.first_frame_ms.is_none() {
            let ms = self.started.elapsed().as_secs_f32() * 1000.0;
            self.stats.first_frame_ms = Some(ms);
            tracing::info!("first frame {ms:.0} ms after start");
        }
        if self.args.frames.is_some_and(|n| self.stats.frames >= n) {
            el.exit();
            return;
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// #236: events dropped on the way since the scene started (a scene with hosts)
    fn events_dropped(&self) -> Option<u64> {
        #[cfg(feature = "events")]
        return self.show.events.as_ref().map(|e| e.counters.dropped());
        #[cfg(not(feature = "events"))]
        None
    }

    /// #209: the collapsed bar's items – the monitor's (FPS, CPU, AUDIO, LIM; #364: deck-core's words and judgement),
    /// the quality, the relay, the GPU backend
    fn bar_items(&self) -> Vec<hud::Item> {
        let item = |text: String, warn: bool| hud::Item { text, level: if warn { hud::Level::Warn } else { hud::Level::Ok } };
        let mut out: Vec<hud::Item> = self.stats.feed.report.bar.iter().map(|i| hud::Item { text: i.text(), level: i.level }).collect();
        // #220: the level now and the render scale – amber while the governor holds it below the level chosen
        if let Some(g) = &self.governor {
            out.insert(1, item(format!("Q {} {:.2}{}", g.level().name().to_uppercase(), g.scale(), if g.auto() { "" } else { " FIX" }), g.level() != g.ceiling()));
        }
        // the relay: the space's session (a deck, the conductor), else the Live outlet's
        let mut relay = String::new();
        #[cfg(feature = "space")]
        if let Some(s) = self.show.space.as_ref().filter(|s| s.join != deck_native::space::Join::Alone) {
            relay = s.session_lines().into_iter().next().unwrap_or_default();
        }
        #[cfg(feature = "live")]
        if relay.is_empty() {
            if let Some(l) = &self.show.live {
                relay = l.hud_lines().into_iter().next().unwrap_or_default();
            }
        }
        out.push(item(format!("RELAY {}", if relay.is_empty() { "-".into() } else { relay.replace(['–', '—'], "-").to_uppercase() }), false));
        let gpu = self.renderer.as_ref().map(|r| format!("{:?}", r.adapter.backend).to_uppercase()).unwrap_or_default();
        out.push(item(format!("GPU {gpu}"), false));
        out.push(item("F1".into(), false));
        out
    }

    fn hud_lines(&self) -> Vec<String> {
        let gpu = self.renderer.as_ref().map(|r| format!("{} {:?}", r.adapter.name, r.adapter.backend)).unwrap_or_default();
        // #208: the device's buffer (its largest callback) and what it costs, how far ahead of the ear the audio is
        // rendered (#188); #364: the monitor's lines (frames, the block loads, the lead, the limiter, the glitches)
        let rate = self.show.clock.as_ref().map_or(48_000.0, |c| c.rate);
        let ms = |frames: f32| frames * 1000.0 / rate;
        let device = match (&self.show.audio, &self.show.glitches) {
            (Some(a), Some(g)) => {
                let buf = g.device_frames();
                Some(format!("AUDIO {} {}K BUF {buf} {:.0}MS OUT {:.0}MS", a.name.to_uppercase(), a.rate / 1000, ms(buf as f32), ms(self.lead() as f32)))
            }
            _ => None,
        };
        let report = &self.stats.feed.report;
        let mut lines = Vec::new();
        if let Some(e) = &self.error {
            // font8x8 has no dashes but the ASCII one
            lines.push("SCENE ERROR - THE TEST PATTERN INSTEAD".into());
            lines.extend(hud::wrap(&e.replace(['–', '—'], "-"), hud::COLS).into_iter().take(hud::LINES - 4));
        }
        // the Live outlet (#186) right under the title – its link, media, cell and every reason it can't show a layer
        // come first (the HUD has 11 lines; the key help and the rest follow as far as they fit)
        let mut live: Vec<String> = Vec::new();
        #[cfg(feature = "live")]
        if let Some(l) = &self.show.live {
            live.extend(l.hud_lines());
        }
        if let Some(n) = &self.show.live_note {
            live.push(format!("LIVE: {}", n.replace(['–', '—'], "-").to_uppercase()));
        }
        let live = live.iter().flat_map(|l| hud::wrap(l, hud::COLS));
        lines.push(format!("SCUTOONSDECK STAGE  {}", self.show.scene_name));
        // #208: a glitch flashes the warning right under the title for a few seconds (the frame sets its row red)
        if !report.alert_line.is_empty() {
            lines.push(format!("{ALERT} {}", report.alert_line));
        }
        lines.extend(live);
        lines.extend([
            "F12 WEB CTRL+ALT+ F2 SHELL / DEL OFF".into(),
            "T TAP +/- BPM SPACE RUN F3 SRC K CLICK".into(),
            format!("GPU {gpu}"),
        ]);
        // #220: the quality now, the level chosen, the governor; the keys
        if let Some(g) = &self.governor {
            let render = self.renderer.as_ref().map_or(1.0, |r| r.render_scale());
            lines.push(format!("Q {} {render:.2}X OF {} {} F4 SET", g.level().name(), g.ceiling().name(), if g.auto() { "AUTO" } else { "FIXED" }).to_uppercase());
        }
        lines.extend(device);
        lines.extend(report.lines.iter().cloned());
        // #236: the hosts' events – delivered by the dispatcher, lost on the way (capped, hops, refused, late)
        #[cfg(feature = "events")]
        if let Some(c) = self.show.events.as_ref().map(|e| &e.counters) {
            lines.push(format!("EVENTS IN {} DROP {}", c.routed.load(std::sync::atomic::Ordering::Relaxed), c.dropped()));
        }
        lines.extend([self.tempo_line(), self.source_line()]);
        // #211: what the outputs drive, or why they are off
        #[cfg(feature = "outputs")]
        if let Some(o) = &self.show.outputs {
            lines.push(format!("OUTPUTS {}", o.summary.to_uppercase()));
        }
        if let Some(n) = &self.show.outputs_note {
            lines.extend(hud::wrap(&format!("OUTPUTS OFF: {}", n.replace(['–', '—', '…'], "-").to_uppercase()), hud::COLS).into_iter().take(3));
        }
        // the sync (#195) while it fetches, is offline or failed
        #[cfg(feature = "sync")]
        if let Some(line) = self.show.sync.as_ref().map(|y| y.line()).filter(|l| !l.starts_with("SYNC OK")) {
            lines.push(line);
        }
        // the conductor's push, as this deck answers it (#195)
        #[cfg(feature = "sync")]
        if let Some(line) = self.show.sync.as_ref().and_then(|y| y.push_line()) {
            lines.push(line);
        }
        #[cfg(feature = "space")]
        if let Some(s) = &self.show.space {
            // the mood and why (#188's feelings) first: the HUD shows 11 lines
            lines.push(s.being.mood_line());
            lines.push(s.hud_line(&self.show.graph));
            lines.extend(s.session_lines());
            lines.push("DRAG TURN/KNOB WHEEL ZOOM R HOME".into());
        }
        if let Some([l, b, m, h]) = self.show.controls.fft() {
            lines.push(format!("FFT L {l:.2} B {b:.2} M {m:.2} H {h:.2}"));
        }
        for s in &self.show.music {
            lines.push(format!("MUSIC {}", s.lock().unwrap()));
        }
        for s in self.renderer.as_ref().map(|r| r.slides_status()).unwrap_or_default() {
            lines.push(format!("SLICE {s}"));
        }

        lines.truncate(hud::LINES);
        lines
    }

    /// BPM, bar and beat (the beat lights sit at its end), stopped, the click
    fn tempo_line(&self) -> String {
        let t = self.show.tempo.now();
        format!(
            "BPM {:.1}  BAR {} BEAT {}{}{}",
            t.bpm,
            t.pos.bar + 1,
            t.pos.beat_in_bar + 1,
            if t.running { "" } else { " STOP" },
            if self.show.tempo.click { " CLK" } else { "" }
        )
    }

    /// the clock's master and its state, the MIDI clock out
    fn source_line(&self) -> String {
        let out = self.show.tempo.clock_out().map(|_| "  > MIDI").unwrap_or_default();
        format!("SRC {} {}{out}", self.show.tempo.source.name().to_uppercase(), self.show.tempo.status())
    }

    fn summary(&self) -> String {
        let mut iv = self.stats.intervals.clone();
        iv.sort_by(f32::total_cmp);
        let avg = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
        let p99 = iv.get(((iv.len() as f32) * 0.99) as usize).or(iv.last()).copied().unwrap_or(0.0);
        let adapter = self.renderer.as_ref().map(|r| format!("{} ({:?})", r.adapter.name, r.adapter.backend)).unwrap_or_default();
        let error = self.error.as_ref().map(|e| format!(" scene_error=\"{e}\"")).unwrap_or_default();
        // #208: the glitches by cause, the device's buffer, the render lead at the end
        let glitches = self
            .show
            .glitches
            .as_ref()
            .map(|g| {
                let [late, long, under] = g.counts();
                format!(" glitches={} (late {late} long {long} underrun {under}) buffer={} lead={}", g.count(), g.device_frames(), g.lead())
            })
            .unwrap_or_default();
        // #236: events lost on the way (capped, hops, refused, late)
        let glitches = match self.events_dropped() {
            Some(n) => format!("{glitches} events_dropped={n}"),
            None => glitches,
        };
        format!(
            "deck-native: frames={} avg_fps={:.1} frame_ms avg={:.2} p99={:.2} cpu_ms avg={:.2} first_frame_ms={:.0} adapter={adapter} quality={} audio={}{glitches} modules={}{error}",
            self.stats.frames,
            1000.0 / avg(&iv).max(1e-3),
            avg(&iv),
            p99,
            avg(&self.stats.work),
            self.stats.first_frame_ms.unwrap_or(0.0),
            self.governor.as_ref().map_or("-".into(), |g| format!("{} {:.2} of {}{}", g.level().name(), g.scale(), g.ceiling().name(), if g.auto() { "" } else { " fixed" })),
            self.show.audio.as_ref().map(|a| format!("{} {} Hz ({})", a.name, a.rate, self.show.audio_build)).unwrap_or_else(|| "off".into()),
            modules::summary(),
        )
    }

    /// A check run's hand on the being (`--pointer NODE.PORT`, `--drag`): the pointer on the named button each frame;
    /// a drag presses it after 60 frames and pulls it along its tether over the next 30, then holds.
    #[cfg(feature = "space")]
    fn check_hand(&mut self) {
        let Some(s) = self.show.space.as_mut() else { return };
        let f = self.stats.frames;
        let ctl = &mut self.show.controls;
        if let Some((name, dn)) = &self.args.drag {
            let Some(i) = s.button(&self.show.graph, name) else { return };
            match (f, self.drag_from) {
                (..60, _) => {
                    if let Some((x, y)) = s.to_pixel(s.being.buttons[i].pos) {
                        s.pointer_moved(x, y, ctl);
                    }
                }
                (60.., None) => {
                    s.press();
                    let (_, axis) = s.drag_axis(i);
                    if let Some(at) = s.to_pixel(s.being.buttons[i].pos) {
                        self.drag_from = Some((at, axis));
                    }
                }
                (61.., Some((at, axis))) => {
                    let k = ((f - 60) as f32 / 30.0).min(1.0) * dn;
                    s.pointer_moved(at.0 + axis.0 * k, at.1 + axis.1 * k, ctl);
                }
                _ => {}
            }
        } else if let Some(Aim::Button(name)) = &self.args.pointer {
            if let Some((x, y)) = s.button(&self.show.graph, name).and_then(|i| s.to_pixel(s.being.buttons[i].pos)) {
                s.pointer_moved(x, y, ctl);
            }
        }
    }

    /// the space node's pointer, wheel and keys (#188); false = not the space's event
    #[cfg(feature = "space")]
    fn space_event(&mut self, event: &WindowEvent) -> bool {
        let Some(s) = self.show.space.as_mut() else { return false };
        match event {
            WindowEvent::Resized(size) => s.set_view(size.width as f32, size.height as f32),
            // a check run's --pointer / --drag stays where it was put
            WindowEvent::CursorMoved { .. } | WindowEvent::CursorLeft { .. } | WindowEvent::MouseInput { .. } | WindowEvent::Touch(_)
                if self.args.pointer.is_some() || self.args.drag.is_some() => {}
            // a finger: like the mouse, with a bigger reach (winit's touch events – Windows, Wayland, X11 with XInput2)
            WindowEvent::Touch(t) => {
                use deck_native::space::Touch;
                let phase = match t.phase {
                    TouchPhase::Started => Touch::Started,
                    TouchPhase::Moved => Touch::Moved,
                    TouchPhase::Ended | TouchPhase::Cancelled => Touch::Ended,
                };
                s.touch(t.id, phase, t.location.x as f32, t.location.y as f32, &mut self.show.controls);
            }
            WindowEvent::CursorMoved { position, .. } => s.pointer_moved(position.x as f32, position.y as f32, &mut self.show.controls),
            WindowEvent::CursorLeft { .. } => s.pointer_left(),
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => match state {
                ElementState::Pressed => s.press(),
                ElementState::Released => s.release(),
            },
            WindowEvent::MouseWheel { delta, .. } => s.camera.zoom(match delta {
                MouseScrollDelta::LineDelta(_, y) => *y,
                MouseScrollDelta::PixelDelta(p) => p.y as f32 / 60.0,
            }),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && !event.repeat => {
                match event.logical_key.as_ref() {
                    Key::Character("r") => s.camera.reset(),
                    // #195: the conductor pushes the scene's sync again (decks that joined since get the characters)
                    #[cfg(feature = "live")]
                    Key::Character("s") if s.join == deck_native::space::Join::Conductor => match push_for(&self.args, &self.show.scene_name, s) {
                        Ok(p) => {
                            s.push_sync(p);
                        }
                        Err(e) => tracing::warn!("{e}"),
                    },
                    Key::Named(NamedKey::ArrowLeft) => s.camera.turn(-0.12),
                    Key::Named(NamedKey::ArrowRight) => s.camera.turn(0.12),
                    Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::PageUp) => s.camera.zoom(1.0),
                    Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::PageDown) => s.camera.zoom(-1.0),
                    _ => return false,
                }
            }
            _ => return false,
        }
        true
    }
}

/// the stats keep the newest ~5 minutes at 60 fps (a kiosk runs for days)
fn push_capped(v: &mut Vec<f32>, x: f32) {
    if v.len() >= 18_000 {
        v.drain(..9_000);
    }
    v.push(x);
}

impl ApplicationHandler<Event> for App {
    fn user_event(&mut self, el: &ActiveEventLoop, event: Event) {
        match event {
            Event::BrowserClosed => {
                self.browser = None;
                self.paused = false;
                self.last = None; // the pause is not a frame interval
                if let Some(m) = &self.show.master {
                    m.set(self.args.volume);
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            Event::Stop => {
                tracing::info!("stop signal – ending the show");
                el.exit();
            }
        }
    }

    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let mut attrs = Window::default_attributes().with_title("SCUTOONSDECK stage");
        if !self.args.windowed {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            attrs = attrs.with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
        }
        let window = match el.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!("no window: {e}");
                self.exit = 1;
                el.exit();
                return;
            }
        };
        // #220: the display's sizes (the settings page; the largest is the hardware's maximum)
        self.modes = window.current_monitor().map(|m| settings::sizes(m.video_modes().map(|v| (v.size().width, v.size().height)).collect())).unwrap_or_default();
        self.start_renderer(&window);
        if self.exit != 0 {
            el.exit();
            return;
        }
        self.window = Some(window);
        self.apply_display();
    }

    /// #220: the frame rate cap's wait is over – the next frame
    fn new_events(&mut self, el: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            el.set_control_flow(ControlFlow::Wait);
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        // #250: the open scene list takes the keys
        if let WindowEvent::KeyboardInput { event: k, .. } = &event {
            if self.switcher.open && k.state == ElementState::Pressed && !matches!(k.logical_key.as_ref(), Key::Named(NamedKey::F1 | NamedKey::F12 | NamedKey::F4 | NamedKey::F5 | NamedKey::AudioVolumeUp | NamedKey::AudioVolumeDown | NamedKey::AudioVolumeMute)) {
                let key = match k.logical_key.as_ref() {
                    Key::Named(NamedKey::ArrowUp) => Some(switcher::Key::Up),
                    Key::Named(NamedKey::ArrowDown) => Some(switcher::Key::Down),
                    Key::Named(NamedKey::Enter) => Some(switcher::Key::Enter),
                    Key::Named(NamedKey::Escape) => {
                        self.switcher.open = false;
                        None
                    }
                    _ => None,
                };
                self.hud_at = -1.0;
                if let Some(entry) = key.and_then(|key| self.switcher.key(key)) {
                    self.switch(&entry.value);
                }
                return;
            }
        }
        // #220: the open settings page takes the keys (the arrows, + / −, Space would turn the camera and the tempo)
        if let WindowEvent::KeyboardInput { event: k, .. } = &event {
            if self.page.open && k.state == ElementState::Pressed && !matches!(k.logical_key.as_ref(), Key::Named(NamedKey::F1 | NamedKey::F12 | NamedKey::F4 | NamedKey::AudioVolumeUp | NamedKey::AudioVolumeDown | NamedKey::AudioVolumeMute)) {
                let key = match k.logical_key.as_ref() {
                    Key::Named(NamedKey::ArrowUp) => Some(settings::Key::Up),
                    Key::Named(NamedKey::ArrowDown) => Some(settings::Key::Down),
                    Key::Named(NamedKey::ArrowLeft) => Some(settings::Key::Left),
                    Key::Named(NamedKey::ArrowRight) => Some(settings::Key::Right),
                    Key::Named(NamedKey::Enter | NamedKey::Space) => Some(settings::Key::Toggle),
                    Key::Character("+" | "=") => Some(settings::Key::More),
                    Key::Character("-") => Some(settings::Key::Less),
                    Key::Character("p" | "P") => Some(settings::Key::Pixels),
                    Key::Character("r" | "R") => Some(settings::Key::Reset),
                    Key::Named(NamedKey::Escape) => {
                        self.page.open = false;
                        None
                    }
                    _ => None,
                };
                if let Some(key) = key {
                    self.page_key(key);
                }
                return;
            }
        }
        #[cfg(feature = "space")]
        if !matches!(event, WindowEvent::Resized(_)) && self.space_event(&event) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => {
                if let Some(r) = self.renderer.as_mut() {
                    r.resize(s.width, s.height);
                }
                if let Some(g) = self.governor.as_mut() {
                    g.set_display(s.width, s.height);
                }
                #[cfg(feature = "live")]
                if let Some(l) = &self.show.live {
                    let k = self.show.render_scale.clamp(0.25, 1.0);
                    l.resize((s.width as f32 * k) as u32, (s.height as f32 * k) as u32);
                }
                #[cfg(feature = "space")]
                self.space_event(&event);
            }
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            // the browser's window took the screen: stop drawing behind it (hidden, a frame could block on vsync)
            WindowEvent::Focused(false) if self.browser.is_some() => self.paused = true,
            // #253: the hardware volume keys – the default sink's volume, wpctl off the frame thread; held keys repeat
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && volume_key(&event.logical_key).is_some() => {
                if let Some(k) = volume_key(&event.logical_key) {
                    if !event.repeat {
                        tracing::info!("volume key {k:?}");
                    }
                    self.volume.press(k);
                }
            }
            // a key the keymap has no name for: logged once, so a deck's odd volume keys can be found
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && !event.repeat && matches!(event.logical_key, Key::Unidentified(_)) => {
                tracing::info!("unidentified key {:?}", event.physical_key);
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && !event.repeat => {
                let tempo = &self.show.tempo;
                match event.logical_key.as_ref() {
                    // #209: the full HUD → the collapsed stats bar → nothing
                    Key::Named(NamedKey::F1) => {
                        self.vol_forced = false;
                        self.hud.mode = self.hud.mode.next();
                    }
                    Key::Named(NamedKey::F12) => self.open_browser(el),
                    Key::Named(NamedKey::Escape) if self.args.windowed => el.exit(),
                    Key::Named(NamedKey::Delete) if self.mods.control_key() && self.mods.alt_key() => self.power_off(),
                    Key::Character("q") if self.mods.control_key() => el.exit(),
                    Key::Named(NamedKey::F3) => tempo.key(tempo::Key::NextSource),
                    // #220: F4 the settings page, Shift+F4 the governor
                    Key::Named(NamedKey::F4) if self.mods.shift_key() => {
                        if let Some(g) = self.governor.as_mut() {
                            g.set_auto(!g.auto());
                        }
                        self.presets.auto = self.governor.as_ref().is_some_and(|g| g.auto());
                        let _ = self.qfile.save(&self.presets);
                        self.apply_quality();
                    }
                    Key::Named(NamedKey::F5) => {
                        if self.switcher.open {
                            self.switcher.open = false;
                        } else {
                            self.open_switcher();
                        }
                        self.hud_at = -1.0;
                    }
                    Key::Named(NamedKey::F4) => {
                        self.switcher.open = false;
                        self.page.open = !self.page.open;
                        self.hud_at = -1.0;
                    }
                    Key::Named(NamedKey::Space) => tempo.key(tempo::Key::StartStop),
                    Key::Character("t") => tempo.key(tempo::Key::Tap),
                    Key::Character("+" | "=") => tempo.key(tempo::Key::Faster),
                    Key::Character("-") => tempo.key(tempo::Key::Slower),
                    Key::Character("k") => tempo.key(tempo::Key::Click),
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => self.redraw(el),
            _ => {}
        }
    }
}

/// #253: the key as a volume key
fn volume_key(k: &Key) -> Option<volume::Key> {
    match k {
        Key::Named(NamedKey::AudioVolumeUp) => Some(volume::Key::Up),
        Key::Named(NamedKey::AudioVolumeDown) => Some(volume::Key::Down),
        Key::Named(NamedKey::AudioVolumeMute) => Some(volume::Key::Mute),
        _ => None,
    }
}

/// #257: the exit code of a stage whose display connection died (the compositor went away) – deck-session ends on it
/// (the kiosk restarts) instead of opening a browser on the dead display
const EXIT_DISPLAY_LOST: i32 = 75;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,wgpu_core=warn,wgpu_hal=error,naga=warn".into()))
        .init();
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            tracing::error!("{e:#}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let started = Instant::now();
    let args = args()?;
    let (mut show, error) = Show::load_or_explain(&args)?;
    show.tempo.start();
    let rate = show.clock.as_ref().map_or(48_000.0, |c| c.rate as f64);

    let event_loop = EventLoop::<Event>::with_user_event().build().context("no event loop (no display?)")?;
    let proxy = event_loop.create_proxy();
    // #263: a stop signal (the kiosk's stop, `deck-set --restart`, Ctrl+C) ends the show the normal way: the outputs dark,
    // the audio faded out. A handler that can't be installed never stops the show.
    let stop_proxy = proxy.clone();
    if let Err(e) = deck_native::stop::install(move || {
        let _ = stop_proxy.send_event(Event::Stop);
    }) {
        tracing::warn!("{e:#} – a stop signal ends the process as it is");
    }
    // #220: the preset file
    let args_settings = args.settings;
    let mut qfile = settings::QualityFile::new(args.quality_file.clone().unwrap_or_else(settings::default_path));
    let (presets, problem) = qfile.load();
    if let Some(e) = &problem {
        tracing::warn!("quality: {e} – the defaults");
    }
    let quality_note = problem.map_or_else(|| format!("FILE {}", qfile.path.file_name().map(|n| n.to_string_lossy().to_uppercase()).unwrap_or_default()), |e| e.to_uppercase());
    let mut app = App {
        hud: hud::Hud::with_mode(if args.hud_bar { hud::Mode::Bar } else { hud::Mode::Full }),
        args,
        started,
        show,
        error,
        watch: None,
        window: None,
        renderer: None,
        hud_at: -1.0,
        volume: volume::Volume::default(),
        vol_overlay: volume::Overlay::default(),
        vol_forced: false,
        last: None,
        t0_ns: clock::now_ns(),
        moment: clock::Moment::wall(clock::now_ns(), clock::now_ns(), rate),
        stats: Stats::default(),
        mods: ModifiersState::empty(),
        exit: 0,
        splash: splash::Fade { alpha: 1.0 },
        pairing: None,
        pairing_at: -10.0,
        browser: None,
        paused: false,
        proxy,
        #[cfg(feature = "space")]
        cursor: deck_native::space::Cursor::Default,
        #[cfg(feature = "space")]
        drag_from: None,
        governor: None,
        gpu: quality::Gpu::Other,
        qfile,
        presets,
        page: { let mut p = settings::Page::default(); p.open = args_settings; p },
        switcher: switcher::Switcher::default(),
        trans: None,
        #[cfg(feature = "outputs")]
        dying: None,
        quality_note,
        modes: Vec::new(),
        #[cfg(feature = "outputs")]
        conductor_midi: Vec::new(),
    };
    app.watch = app.watch_files();
    if app.args.switcher {
        app.open_switcher();
    }
    // #257: the display connection died under us (the compositor went away): leave at once, and drop nothing –
    // the window / surface / EGL objects talk to a dead socket and segfault
    if let Err(e) = event_loop.run_app(&mut app) {
        tracing::error!("display lost: {e}");
        // the compositor going away is the kiosk's stop (#256) as often as a crash: the lamps still go dark
        #[cfg(feature = "outputs")]
        {
            app.dying = None;
        }
        app.show.outputs_dark();
        std::process::exit(EXIT_DISPLAY_LOST);
    }
    let summary = app.summary();
    tracing::info!("{summary}");
    println!("{summary}");
    // a check run (--frames) that couldn't play its scene fails; the kiosk never exits for it
    let code = if app.exit == 0 && app.error.is_some() && app.args.frames.is_some() { 2 } else { app.exit };
    // #263: the show ends the way a scene ends – its outputs dark (a USB-DMX widget repeats the last frame for ever), its
    // audio faded out – whatever stopped it (Ctrl+Q, a stop signal, --frames). No teardown of the window / GPU objects:
    // the process ends right after (main calls exit)
    #[cfg(feature = "outputs")]
    {
        app.dying = None; // #264: lamps still fading out go dark too
    }
    app.show.end();
    std::mem::forget(app);
    Ok(code)
}

#[cfg(test)]
mod tests {
    /// #195: the push names the scene by its file, else by the stage's name as a slug
    #[test]
    fn a_scene_s_id_for_the_push() {
        assert_eq!(super::scene_id(Some(std::path::Path::new("conductor")), "x"), "conductor");
        assert_eq!(super::scene_id(Some(std::path::Path::new("/s/scenes/sermon.toml")), "x"), "sermon");
        assert_eq!(super::scene_id(None, "Cathedral of decks - conductor"), "cathedral-of-decks-conductor");
    }
}
