//! deck-stage – the native AV app the deck boots into (CYBERDECKS.md §8, #179). One window (fullscreen; under cage on
//! the deck), a node graph from a TOML scene: video passes on wgpu, an audio graph on PipeWire / cpal, control values
//! per frame. Keys: F1 the HUD · F2 the browser · Ctrl+Q quit · Ctrl+Alt+Del power off (on a deck) · the tempo clock
//! (#181): T tap · + / − BPM · Space start / stop · F3 the next source · K the click. The splash (the logo) covers the start until the
//! chapter's first slice and the music are there, and the browser's start: with DECK_BROWSER set (deck-session does)
//! F2 starts that command on top of the running stage – cage shows the newest window – and the stage pauses and mutes
//! behind it until it closes; without it F2 quits with exit code 10 (deck-session's older loop opens the browser).
//! A scene with a `space` node (#188): the mouse (or a finger) turns the camera or pulls the being's 3D buttons along
//! their tethers (a double click: back to the scene's value), the wheel / ↑ ↓ zoom, ← → turn, R resets the camera; the
//! conductor's S pushes the scene's sync to the decks (#195 – also once at the start; `--sync` names what).
//! A scene that can't be played (a module this build lacks – #185 –, a broken file) never stops the stage: it shows the
//! test pattern with the reason in the HUD. `--dev` reloads the scene file and its WGSL files when they change.
//! A scene with an `outlet` node (#186, the `live` module) joins the relay as `output` of this deck's outlet
//! (deck.toml's `relay` + `outlet`, or `--relay` / `--outlet`; no relay = the hosted wss://live.scutoons.com/live/CATH,
//! #199) and shows its cells; with an outlet set and no `--scene` the stage plays the built-in `scenes/outlet.toml`.
//! A space node that joins (deck / conductor) without a relay joins the hosted one too.
use deck_stage::{audio, cache, chapter, clock, control, graph, hud, modules, render, scene, splash, tempo, watch};

use anyhow::{bail, Context, Result};
use audio::backend::{self, Choice};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
#[cfg(feature = "space")]
use winit::event::{MouseButton, MouseScrollDelta, TouchPhase};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

/// the exit code that asks deck-session for the browser (the conductor page) instead of the stage
const EXIT_BROWSER: i32 = 10;
/// the splash waits this long at most for the chapter and the music (offline without a cache: the scene anyway)
const SPLASH_MAX_S: f32 = 30.0;

/// what other threads tell the event loop
#[derive(Debug)]
enum Event {
    /// the browser (DECK_BROWSER) exited
    BrowserClosed,
}

const HELP: &str = "deck-stage [--scene FILE|cathedral|conductor|outlet] [--audio auto|pipewire|cpal|null] [--volume 0-1] [--cache DIR]
           [--sets DIR] [--portraits DIR] [--token T] [--api URL] [--cdn URL] [--windowed] [--frames N]
           [--screenshot FILE] [--pointer X,Y|NODE.PORT] [--drag NODE.PORT[,DN]] [--no-hud] [--dev] [--modules]
           [--relay ws[s]://HOST[:PORT]/live/CODE] [--sync set:ID,character:ID]
           [--outlet ID] [--locale de]
  --scene      a scene TOML (default: the built-in scene, deck/stage/scenes/default.toml); `cathedral` = the built-in
               Cathedral of decks (scenes/cathedral.toml – a deck's being), `conductor` = its conductor in the middle
               (scenes/conductor.toml), `outlet` = the built-in Live outlet (scenes/outlet.toml – the default when an
               outlet is set)
  --sync       the conductor's push (#195): what the decks fetch into their cache – sent at the start and on S
               (default: the scene's set and the characters on stage)
  --relay      the Live relay with the session code (default: deck.toml's `relay`, else the hosted
               wss://live.scutoons.com/live/CATH – when a scene joins or an outlet is set); --outlet this deck's outlet
               (deck.toml's `outlet`); --locale the language of its text layers (deck.toml's `locale`, else de)
  --audio      the backend; auto = PipeWire → cpal → null (no device: the clock and the FFT still run)
  --cache      the library cache (default /var/lib/deck/cache on a deck, DECK_CACHE, else the user cache folder)
  --sets       a local folder of sets instead of the sync's: <id>/<id>.<quality>.glb or <id>.<quality>.glb (DECK_SETS;
               default <cache>/sets, synced from the API)
  --portraits  a local folder of the beings' heads instead of the sync's: <char>_portrait-<mood>.png|webp
               (DECK_PORTRAITS; default <cache>/portraits, synced from the API)
  --token      the conductor's session token for a space node with join = conductor (default LIVE_TOKEN; a dev relay
               takes any)
  pairing      (#180) while deck-link's run directory says the deck waits for a member, the code + QR in the corner;
               its token goes to the API once approved (DECK_LINK_DIR, default /run/deck-link; DECK_APP; DECK_TOKEN)
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
    volume: f32,
    cache: Option<PathBuf>,
    api: String,
    cdn: String,
    windowed: bool,
    frames: Option<u64>,
    hud: bool,
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
        volume: 1.0,
        cache: None,
        api: std::env::var("DECK_API").unwrap_or_else(|_| cache::API.into()),
        cdn: std::env::var("DECK_CDN").unwrap_or_else(|_| cache::CDN.into()),
        windowed: false,
        frames: None,
        hud: true,
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
            "--dev" => a.dev = true,
            "--relay" => a.relay = Some(val()?),
            "--outlet" => a.outlet = Some(val()?),
            "--locale" => a.locale = Some(val()?),
            "--sync" => a.sync = Some(val()?),
            "--modules" => {
                for m in modules::registry() {
                    let what: Vec<&str> = m.kinds.iter().chain(m.sources).copied().collect();
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
    window: (u32, f32),
    fps: f32,
    /// the limiter's gain reduction the HUD shows (dB, held a moment), the underruns per minute and when last counted
    limit: f32,
    xruns: hud::PerMinute,
    xrun_rate: u64,
    xrun_at: f32,
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
    /// the chapter nodes' loaders, until the renderer takes them
    feeds: Vec<chapter::ChapterFeed>,
    /// the master gain (the browser mutes it; it returns to --volume)
    master: Option<audio::Param>,
    /// the limiter's gain reduction (dB, f32 bits – taken each frame) and the device's underruns (#188)
    reduction: Option<Arc<std::sync::atomic::AtomicU32>>,
    underruns: Option<Arc<std::sync::atomic::AtomicU64>>,
    /// the scene's space node (#188): the camera, the being, the pointer
    space: Option<render::Space>,
    /// the Live outlet (#186): its threads, the space targets for the space node; or why it doesn't play
    #[cfg(feature = "live")]
    live: Option<deck_stage::live::Live>,
    live_note: Option<String>,
    /// the deck's sync (#195): the scene's sets and portraits from the API into the cache; its HUD line
    #[cfg(feature = "sync")]
    sync: Option<deck_stage::sync::DeckSync>,
    /// the audio outputs' peaks (the being's audio hoses)
    #[cfg_attr(not(feature = "space"), allow(dead_code))]
    meters: Option<Arc<audio::Meters>>,
}

/// the Live outlet's settings from the CLI (deck.toml's under them – live/config.rs)
#[cfg(feature = "live")]
fn outlet_settings(args: &Args) -> deck_stage::live::config::Settings {
    deck_stage::live::config::Settings { relay: args.relay.clone(), outlet: args.outlet.clone(), locale: args.locale.clone() }
}

/// a relay and an outlet are set (deck.toml or the CLI): the deck is a Live outlet
fn outlet_configured(args: &Args) -> bool {
    #[cfg(feature = "live")]
    {
        let file = deck_stage::live::config::Settings::read(std::path::Path::new(deck_stage::live::config::DECK_TOML));
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
    if let Some((_, q)) = graph.first(graph::Kind::Space).and_then(|n| deck_stage::space::set_need(graph, n)) {
        return q;
    }
    let _ = graph;
    "low".into()
}

/// the conductor's push (#195): `--sync`, else the scene's set and the characters on stage; its `scene` = the scene
/// file's name (`conductor`, `cathedral` …), else the stage's name as a slug
#[cfg(all(feature = "live", feature = "space"))]
fn push_for(args: &Args, name: &str, space: &deck_stage::space::Space) -> Result<protocol::SceneSync, String> {
    let scene = scene_id(args.scene.as_deref(), name);
    let scene = scene.as_str();
    match &args.sync {
        Some(spec) => deck_stage::space::link::parse_push(scene, spec),
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

/// the scene the arguments name: a file, `cathedral` / `outlet` (built in), the outlet when one is set, or the default
fn read_scene(args: &Args) -> Result<scene::Scene> {
    match &args.scene {
        Some(p) if p.as_os_str() == "cathedral" && !p.exists() => scene::Scene::parse(scene::CATHEDRAL_SCENE),
        Some(p) if p.as_os_str() == "conductor" && !p.exists() => scene::Scene::parse(scene::CONDUCTOR_SCENE),
        Some(p) if p.as_os_str() == "outlet" && !p.exists() => scene::Scene::parse(scene::OUTLET_SCENE),
        Some(p) => scene::Scene::load(p),
        None if outlet_configured(args) => scene::Scene::parse(scene::OUTLET_SCENE),
        None => scene::Scene::parse(scene::DEFAULT_SCENE),
    }
}

impl Show {
    /// Build the scene's graph (refused when it needs a module this build lacks – modules.rs) and start what plays it.
    fn load(args: &Args, scene: &scene::Scene) -> Result<Show> {
        modules::check(scene)?;
        let graph = graph::Graph::build(scene)?;
        tracing::info!("scene '{}': {} nodes, {} links (modules: {})", scene.stage.name, graph.nodes.len(), graph.links.len(), modules::summary());
        let tempo_cfg = tempo::Config::from_graph(&graph)?;

        // audio first: its clock is the master clock, its handles feed the control domain
        let has_audio = graph.nodes.iter().any(|n| n.kind.domain() == graph::Domain::Audio);
        let mut handles = None;
        let audio = if has_audio {
            let mut factory = |rate: f32| {
                let (engine, h) = audio::Engine::build(&graph, rate, args.volume)?;
                tracing::info!("audio graph: {} ({} steps, {} batches)", h.build.label(), engine.shape().0, engine.shape().1);
                handles = Some(h);
                Ok(engine)
            };
            match backend::start(args.audio, &mut factory) {
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
            deck_stage::sync::DeckSync::start(deck_stage::sync::Config {
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
                let lib = deck_stage::library::Library { api: args.api.clone(), cdn: args.cdn.clone(), cache: cache.clone() };
                let synced = cfg!(feature = "sync") && args.sets.is_none();
                match deck_stage::live::start_outlet(outlet_settings(args), lib, sets.clone(), synced, (1920, 1080)) {
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
        let lib = deck_stage::library::Library { api: args.api.clone(), cdn: args.cdn.clone(), cache: cache.clone() };
        // the chapter_music nodes' loaders (their status for the HUD and the splash)
        #[cfg(feature = "chapter")]
        let music: Vec<Arc<Mutex<String>>> = match handles.as_mut() {
            Some(h) if !h.music.is_empty() => {
                tracing::info!("library: {} · cache {}", lib.api, lib.cache.display());
                let spawn = |feed: audio::MusicFeed| {
                    let status = feed.status.clone();
                    deck_stage::library::spawn_music_loader(lib.clone(), &graph.nodes[feed.node], feed);
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
        let underruns = handles.as_ref().map(|h| h.underruns.clone());
        let meters = handles.as_ref().and_then(|h| h.meters.clone());
        #[cfg(feature = "space")]
        let space = graph.first(graph::Kind::Space).map(|n| {
            let faces = args.portraits.clone().unwrap_or_else(|| cache.join("portraits"));
            tracing::info!("space: sets in {}, portraits in {}", sets.display(), faces.display());
            use deck_stage::space::portrait::Portraits;
            // the one request for the scene goes out before the head asks for its first sprite
            #[cfg(feature = "sync")]
            if let Some(y) = &sync {
                y.prepare(deck_stage::sync::needs_of(&graph, &scene.being));
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
            let mut s = deck_stage::space::Space::with_gate(&graph, n, &scene.being, &sets, portraits, gate);
            // phase 4: with a relay (deck.toml or --relay) the being joins the session as its `join` says
            #[cfg(feature = "live")]
            {
                let relay = outlet_settings(args).relay.or_else(|| deck_stage::live::config::Settings::read(std::path::Path::new(deck_stage::live::config::DECK_TOML)).relay);
                use deck_stage::space::Connect;
                // #195: a deck answers the conductor's pushes through its sync
                #[cfg(feature = "sync")]
                let hook = sync.clone().map(|y| Box::new(y) as Box<dyn deck_stage::space::link::Syncs>);
                #[cfg(not(feature = "sync"))]
                let hook = None;
                s.connect(relay.as_deref(), args.token.clone(), Some(scene.stage.name.clone()), hook);
                // the conductor pushes the scene's sync once joined (decks that come later find it in their snapshot)
                if s.join == deck_stage::space::Join::Conductor {
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
        let controls = control::Controls::new(&graph, handles.as_mut());
        // the tempo clock: the engine's shared map (or its own without audio)
        let rate = clock.as_ref().map_or(48_000.0, |c| c.rate as f64);
        let shared = handles.as_ref().map(|h| h.tempo.clone()).unwrap_or_default();
        let tempo = tempo::Tempo::new(tempo_cfg, shared, rate);
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
            feeds,
            master,
            reduction,
            underruns,
            space,
            #[cfg(feature = "live")]
            live,
            live_note,
            #[cfg(feature = "sync")]
            sync,
            meters,
        })
    }

    /// The scene the arguments name, or – when it can't be played – the test pattern and the reason.
    fn load_or_explain(args: &Args) -> Result<(Show, Option<String>)> {
        match read_scene(args).and_then(|s| Show::load(args, &s)) {
            Ok(show) => Ok((show, None)),
            Err(e) => {
                tracing::error!("the scene can't be played – the test pattern instead: {e:#}");
                let fallback = scene::Scene::parse(scene::TEST_PATTERN_SCENE)?;
                Ok((Show::load(args, &fallback)?, Some(format!("{e:#}"))))
            }
        }
    }

    /// the shader nodes' WGSL files (what `--dev` watches besides the scene)
    fn shader_files(&self) -> Vec<(usize, PathBuf)> {
        let g = &self.graph;
        (0..g.nodes.len()).filter(|&i| g.nodes[i].kind == graph::Kind::Shader).filter_map(|i| Some((i, g.nodes[i].str("wgsl")?.into()))).collect()
    }
}

/// what `--dev` watches: the scene file, a shader node's WGSL file
#[derive(Debug, Clone, Copy, PartialEq)]
enum Watched {
    Scene,
    Shader(usize),
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
    pairing: Option<deck_stage::pairing::Card>,
    pairing_at: f32,
    /// the browser on top: since when; `paused` once it has the focus (the stage stops drawing)
    browser: Option<Instant>,
    paused: bool,
    proxy: EventLoopProxy<Event>,
    /// the pointer's look shown (#188: a hand over a button)
    #[cfg(feature = "space")]
    cursor: deck_stage::space::Cursor,
    /// a check run's --drag: where the press was and the tether's way on the screen
    #[cfg(feature = "space")]
    drag_from: Option<((f32, f32), (f32, f32))>,
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
        let (show, error) = match Show::load(&self.args, &scene) {
            Ok(s) => (s, None),
            Err(e) => match scene::Scene::parse(scene::TEST_PATTERN_SCENE).and_then(|s| Show::load(&self.args, &s)) {
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
        self.error = error;
        self.watch = self.watch_files();
        self.last = None;
        if let Some(w) = self.window.clone() {
            self.start_renderer(&w);
        }
        tracing::info!("reloaded '{}'", self.show.scene_name);
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
        match render::Renderer::new(window.clone(), &self.show.graph, self.show.render_scale, feeds) {
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
                self.renderer = Some(r)
            }
            Err(e) => {
                tracing::error!("render: {e:#}");
                self.exit = 1;
            }
        }
        window.request_redraw();
    }

    /// F2: the browser on top of the stage (DECK_BROWSER), else exit code 10 for deck-session
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
        if self.exit != 0 {
            el.exit();
            return;
        }
        let now = Instant::now();
        let dt = self.last.map(|l| (now - l).as_secs_f32()).unwrap_or(1.0 / 60.0).min(0.25);
        if let Some(l) = self.last {
            let ms = (now - l).as_secs_f32() * 1000.0;
            push_capped(&mut self.stats.intervals, ms);
            self.hud.push_frame(ms);
        }
        self.last = Some(now);
        self.advance_time();
        let beat = self.show.tempo.frame(&self.moment);
        self.show.controls.update(dt, self.lead(), &beat);
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
        // #188: the limiter's gain reduction since the last frame (the HUD holds it a moment; the being's red face), the
        // device's underruns per minute (counted once a second)
        let reduction = self.show.reduction.as_ref().map_or(0.0, |r| f32::from_bits(r.swap(0, std::sync::atomic::Ordering::Relaxed)));
        self.stats.limit = reduction.max(self.stats.limit - dt * 6.0);
        if time - self.stats.xrun_at >= 1.0 || time < self.stats.xrun_at {
            self.stats.xrun_at = time;
            let n = self.show.underruns.as_ref().map_or(0, |u| u.load(std::sync::atomic::Ordering::Relaxed));
            self.stats.xrun_rate = self.stats.xruns.push(time, n);
        }
        #[cfg(feature = "space")]
        if let Some(s) = self.show.space.as_mut() {
            let meters = self.show.meters.as_deref();
            s.being.strain(dt, reduction);
            s.update(dt, time, &mut self.show.controls, &beat, &|n, p| meters.map_or(0.0, |m| m.level(n, p)));
            // the pointer's look: a hand over a button, a closed one on it, "no" over a linked one
            let c = s.cursor();
            if c != self.cursor {
                self.cursor = c;
                if let Some(w) = &self.window {
                    use deck_stage::space::Cursor;
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
        self.stats.window.0 += 1;
        self.stats.window.1 += dt;
        if self.stats.window.1 >= 1.0 {
            self.stats.fps = self.stats.window.0 as f32 / self.stats.window.1;
            self.stats.window = (0, 0.0);
        }
        // a scene that can't be played is explained even without the HUD
        let hud_on = self.args.hud || self.error.is_some();
        if hud_on && time - self.hud_at >= 0.25 || self.stats.frames == 0 || self.hud_at < 0.0 {
            self.hud_at = time;
            let lines = self.hud_lines();
            let t = self.show.tempo.now();
            let row = lines.iter().position(|l| l.starts_with("BPM"));
            self.hud.set_beat(row.map(|row| hud::Beat { row, beat: t.pos.beat_in_bar, per_bar: t.per_bar, running: t.running }));
            self.hud.set_lines(lines);
        }
        if self.error.is_some() {
            self.hud.visible = true;
        }
        if self.hud.visible {
            self.hud.raster();
        }
        let cover = self.loading() || self.browser.is_some();
        self.splash.step(cover, dt);
        let splash = (self.splash.alpha, cover);
        let Some(r) = self.renderer.as_mut() else { return };
        // #180: while the deck waits for a member, the code + QR in the corner (deck-link's run directory, a small file)
        if time - self.pairing_at >= 2.0 || time < self.pairing_at {
            self.pairing_at = time;
            let state = deck_link::pairing::read_state(&deck_link::pairing::run_dir());
            let card = deck_stage::pairing::Card::of(state.as_ref(), &deck_link::pairing::app_url());
            if card != self.pairing {
                r.set_pairing(card.as_ref().map(|c| c.raster()).as_deref());
                self.pairing = card;
            }
        }
        let shown = (hud_on && self.hud.visible).then_some(&self.hud);
        if self.args.frames.is_some_and(|n| self.stats.frames + 1 >= n) {
            r.capture = self.args.screenshot.clone();
        }
        let before = (now.elapsed().as_secs_f32()) * 1000.0;
        match r.frame(&self.show.graph, &self.show.controls, time, dt, shown, splash, self.show.space.as_mut()) {
            // the frame thread's work: controls + HUD before the render, the render after the surface's vsync wait
            Ok(ms) => push_capped(&mut self.stats.work, before + ms),
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

    fn hud_lines(&self) -> Vec<String> {
        let last = |v: &[f32]| v.iter().rev().take(60).sum::<f32>() / v.len().clamp(1, 60) as f32;
        let gpu = self.renderer.as_ref().map(|r| format!("{} {:?}", r.adapter.name, r.adapter.backend)).unwrap_or_default();
        // #188: how far ahead the audio is rendered, underruns in the last minute, the limiter's gain reduction (dB)
        let ahead = self.lead() as f32 * 1000.0 / self.show.clock.as_ref().map_or(48_000.0, |c| c.rate);
        let audio = match &self.show.audio {
            Some(a) => format!("{} {}K {ahead:.0}MS XRUN {}/M LIM {:.0}", a.name, a.rate / 1000, self.stats.xrun_rate, self.stats.limit),
            None => "off".into(),
        };
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
        lines.extend(live);
        lines.extend([
            "F2 WEB  CTRL+ALT+ F2 SHELL / DEL OFF".into(),
            "T TAP +/- BPM SPACE RUN F3 SRC K CLICK".into(),
            format!("FPS {:.1}  FRAME {:.1} MS  CPU {:.2} MS", self.stats.fps, last(&self.stats.intervals), last(&self.stats.work)),
            format!("GPU {gpu}"),
            format!("AUDIO {audio}"),
            self.tempo_line(),
            self.source_line(),
        ]);
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
        format!(
            "deck-stage: frames={} avg_fps={:.1} frame_ms avg={:.2} p99={:.2} cpu_ms avg={:.2} first_frame_ms={:.0} adapter={adapter} audio={} modules={}{error}",
            self.stats.frames,
            1000.0 / avg(&iv).max(1e-3),
            avg(&iv),
            p99,
            avg(&self.stats.work),
            self.stats.first_frame_ms.unwrap_or(0.0),
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
                use deck_stage::space::Touch;
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
                    Key::Character("s") if s.join == deck_stage::space::Join::Conductor => match push_for(&self.args, &self.show.scene_name, s) {
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
    fn user_event(&mut self, _: &ActiveEventLoop, event: Event) {
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
        self.start_renderer(&window);
        if self.exit != 0 {
            el.exit();
            return;
        }
        self.window = Some(window);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
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
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && !event.repeat => {
                let tempo = &self.show.tempo;
                match event.logical_key.as_ref() {
                    Key::Named(NamedKey::F1) => self.hud.visible = !self.hud.visible,
                    Key::Named(NamedKey::F2) => self.open_browser(el),
                    Key::Named(NamedKey::Escape) if self.args.windowed => el.exit(),
                    Key::Named(NamedKey::Delete) if self.mods.control_key() && self.mods.alt_key() => self.power_off(),
                    Key::Character("q") if self.mods.control_key() => el.exit(),
                    Key::Named(NamedKey::F3) => tempo.key(tempo::Key::NextSource),
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
    let mut app = App {
        hud: hud::Hud::default(),
        args,
        started,
        show,
        error,
        watch: None,
        window: None,
        renderer: None,
        hud_at: -1.0,
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
        cursor: deck_stage::space::Cursor::Default,
        #[cfg(feature = "space")]
        drag_from: None,
    };
    app.watch = app.watch_files();
    event_loop.run_app(&mut app)?;
    let summary = app.summary();
    tracing::info!("{summary}");
    println!("{summary}");
    // a check run (--frames) that couldn't play its scene fails; the kiosk never exits for it
    Ok(if app.exit == 0 && app.error.is_some() && app.args.frames.is_some() { 2 } else { app.exit })
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
