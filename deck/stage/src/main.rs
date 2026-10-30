//! deck-stage – the native AV app the deck boots into (CYBERDECKS.md §8, #179). One window (fullscreen; under cage on
//! the deck), a node graph from a TOML scene: video passes on wgpu, an audio graph on PipeWire / cpal, control values
//! per frame. Keys: F1 the HUD · F2 the browser · Ctrl+Q quit · Ctrl+Alt+Del power off (on a deck) · the tempo clock
//! (#181): T tap · + / − BPM · Space start / stop · F3 the next source · K the click. The splash (the logo) covers the start until the
//! chapter's first slice and the music are there, and the browser's start: with DECK_BROWSER set (deck-session does)
//! F2 starts that command on top of the running stage – cage shows the newest window – and the stage pauses and mutes
//! behind it until it closes; without it F2 quits with exit code 10 (deck-session's older loop opens the browser).
use deck_stage::{audio, chapter, clock, control, graph, hud, library, render, scene, splash, tempo};

use anyhow::{bail, Context, Result};
use audio::backend::{self, Choice};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

/// the exit code that asks deck-session for the browser (the conductor page) instead of the stage
const EXIT_BROWSER: i32 = 10;
/// the HUD's tempo line (the beat lights are drawn on it)
const TEMPO_ROW: usize = 6;
/// the splash waits this long at most for the chapter and the music (offline without a cache: the scene anyway)
const SPLASH_MAX_S: f32 = 30.0;

/// what other threads tell the event loop
#[derive(Debug)]
enum Event {
    /// the browser (DECK_BROWSER) exited
    BrowserClosed,
}

const HELP: &str = "deck-stage [--scene FILE] [--audio auto|pipewire|cpal|null] [--volume 0-1] [--cache DIR]
           [--api URL] [--cdn URL] [--windowed] [--frames N] [--no-hud]
  --scene    a scene TOML (default: the built-in scene, deck/stage/scenes/default.toml)
  --audio    the backend; auto = PipeWire → cpal → null (no device: the clock and the FFT still run)
  --cache    the library cache (default /var/lib/deck/cache on a deck, DECK_CACHE, else the user cache folder)
  --frames   quit after N frames and print the measured numbers (a check run)";

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
}

fn args() -> Result<Args> {
    let mut a = Args {
        scene: None,
        audio: Choice::Auto,
        volume: 1.0,
        cache: None,
        api: std::env::var("DECK_API").unwrap_or_else(|_| library::API.into()),
        cdn: std::env::var("DECK_CDN").unwrap_or_else(|_| library::CDN.into()),
        windowed: false,
        frames: None,
        hud: true,
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
            "--windowed" => a.windowed = true,
            "--no-hud" => a.hud = false,
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
}

struct App {
    args: Args,
    started: Instant,
    graph: graph::Graph,
    scene_name: String,
    render_scale: f32,
    controls: control::Controls,
    audio: Option<backend::Running>,
    clock: Option<audio::Clock>,
    music: Vec<Arc<Mutex<String>>>,
    window: Option<Arc<Window>>,
    renderer: Option<render::Renderer>,
    hud: hud::Hud,
    hud_at: f32,
    last: Option<Instant>,
    /// the start on clock::now_ns's clock (the time base without audio)
    t0_ns: u64,
    /// the time base's frame-thread half (clock.rs); None without audio (then the wall clock since the start)
    timebase: Option<clock::TimeBase>,
    /// this frame's view of the time base: the audio position heard at the frame's moment, what is rendered, the line
    moment: clock::Moment,
    /// the tempo clock on top of it (tempo/, #181)
    tempo: tempo::Tempo,
    stats: Stats,
    mods: ModifiersState,
    exit: i32,
    /// the chapter nodes' loaders, until the renderer takes them
    feeds: Vec<chapter::ChapterFeed>,
    splash: splash::Fade,
    /// the master gain (the browser mutes it; it returns to --volume)
    master: Option<audio::Param>,
    /// the browser on top: since when; `paused` once it has the focus (the stage stops drawing)
    browser: Option<Instant>,
    paused: bool,
    proxy: EventLoopProxy<Event>,
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
        self.moment = match (&self.clock, self.timebase.as_mut()) {
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
        let music = self.music.iter().any(|s| {
            let s = s.lock().unwrap();
            s.is_empty() || ["waiting", "loading", "fetching", "decoding"].iter().any(|p| s.starts_with(p))
        });
        music || self.renderer.as_ref().is_some_and(|r| !r.slides_ready())
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
                if let Some(m) = &self.master {
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
        if let Some(m) = &self.master {
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
        let now = Instant::now();
        let dt = self.last.map(|l| (now - l).as_secs_f32()).unwrap_or(1.0 / 60.0).min(0.25);
        if let Some(l) = self.last {
            let ms = (now - l).as_secs_f32() * 1000.0;
            push_capped(&mut self.stats.intervals, ms);
            self.hud.push_frame(ms);
        }
        self.last = Some(now);
        self.advance_time();
        let beat = self.tempo.frame(&self.moment);
        self.controls.update(&self.graph, dt, self.lead(), &beat);
        let time = self.time();
        self.stats.window.0 += 1;
        self.stats.window.1 += dt;
        if self.stats.window.1 >= 1.0 {
            self.stats.fps = self.stats.window.0 as f32 / self.stats.window.1;
            self.stats.window = (0, 0.0);
        }
        if self.args.hud && time - self.hud_at >= 0.25 || self.stats.frames == 0 {
            self.hud_at = time;
            self.hud.set_lines(self.hud_lines());
            let t = self.tempo.now();
            self.hud.set_beat(Some(hud::Beat { row: TEMPO_ROW, beat: t.pos.beat_in_bar, per_bar: t.per_bar, running: t.running }));
        }
        if self.hud.visible {
            self.hud.raster();
        }
        let cover = self.loading() || self.browser.is_some();
        self.splash.step(cover, dt);
        let splash = (self.splash.alpha, cover);
        let Some(r) = self.renderer.as_mut() else { return };
        let shown = (self.args.hud && self.hud.visible).then_some(&self.hud);
        let before = (now.elapsed().as_secs_f32()) * 1000.0;
        match r.frame(&self.graph, &self.controls, time, dt, shown, splash) {
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
        let audio = match &self.audio {
            Some(a) => format!("{} {} HZ", a.name, a.rate),
            None => "off".into(),
        };
        let mut lines = vec![
            format!("SCUTOONSDECK STAGE  {}", self.scene_name),
            "F2 WEB  CTRL+ALT+ F2 SHELL / DEL OFF".into(),
            "T TAP +/- BPM SPACE RUN F3 SRC K CLICK".into(),
            format!("FPS {:.1}  FRAME {:.1} MS  CPU {:.2} MS", self.stats.fps, last(&self.stats.intervals), last(&self.stats.work)),
            format!("GPU {gpu}"),
            format!("AUDIO {audio}  T {:.1} S  AHEAD {:.0} MS", self.time(), self.lead() as f32 * 1000.0 / self.clock.as_ref().map_or(48_000.0, |c| c.rate)),
            self.tempo_line(),
            self.source_line(),
        ];
        debug_assert!(lines[TEMPO_ROW].starts_with("BPM"), "the beat lights' row");
        if let Some([l, b, m, h]) = self.controls.fft() {
            lines.push(format!("FFT L {l:.2} B {b:.2} M {m:.2} H {h:.2}"));
        }
        for s in &self.music {
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
        let t = self.tempo.now();
        format!(
            "BPM {:.1}  BAR {} BEAT {}{}{}",
            t.bpm,
            t.pos.bar + 1,
            t.pos.beat_in_bar + 1,
            if t.running { "" } else { " STOP" },
            if self.tempo.click { " CLK" } else { "" }
        )
    }

    /// the clock's master and its state, the MIDI clock out
    fn source_line(&self) -> String {
        let out = self.tempo.clock_out().map(|_| "  > MIDI").unwrap_or_default();
        format!("SRC {} {}{out}", self.tempo.source.name().to_uppercase(), self.tempo.status())
    }

    fn summary(&self) -> String {
        let mut iv = self.stats.intervals.clone();
        iv.sort_by(f32::total_cmp);
        let avg = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
        let p99 = iv.get(((iv.len() as f32) * 0.99) as usize).or(iv.last()).copied().unwrap_or(0.0);
        let adapter = self.renderer.as_ref().map(|r| format!("{} ({:?})", r.adapter.name, r.adapter.backend)).unwrap_or_default();
        format!(
            "deck-stage: frames={} avg_fps={:.1} frame_ms avg={:.2} p99={:.2} cpu_ms avg={:.2} first_frame_ms={:.0} adapter={adapter} audio={}",
            self.stats.frames,
            1000.0 / avg(&iv).max(1e-3),
            avg(&iv),
            p99,
            avg(&self.stats.work),
            self.stats.first_frame_ms.unwrap_or(0.0),
            self.audio.as_ref().map(|a| format!("{} {} Hz", a.name, a.rate)).unwrap_or_else(|| "off".into()),
        )
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
                if let Some(m) = &self.master {
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
        if !self.args.windowed {
            window.set_cursor_visible(false);
        }
        match render::Renderer::new(window.clone(), &self.graph, self.render_scale, std::mem::take(&mut self.feeds)) {
            Ok(r) => self.renderer = Some(r),
            Err(e) => {
                tracing::error!("render: {e:#}");
                self.exit = 1;
                el.exit();
                return;
            }
        }
        window.request_redraw();
        self.window = Some(window);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => {
                if let Some(r) = self.renderer.as_mut() {
                    r.resize(s.width, s.height);
                }
            }
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            // the browser's window took the screen: stop drawing behind it (hidden, a frame could block on vsync)
            WindowEvent::Focused(false) if self.browser.is_some() => self.paused = true,
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && !event.repeat => {
                match event.logical_key.as_ref() {
                    Key::Named(NamedKey::F1) => self.hud.visible = !self.hud.visible,
                    Key::Named(NamedKey::F2) => self.open_browser(el),
                    Key::Named(NamedKey::Escape) if self.args.windowed => el.exit(),
                    Key::Named(NamedKey::Delete) if self.mods.control_key() && self.mods.alt_key() => self.power_off(),
                    Key::Character("q") if self.mods.control_key() => el.exit(),
                    Key::Named(NamedKey::F3) => self.tempo.key(tempo::Key::NextSource),
                    Key::Named(NamedKey::Space) => self.tempo.key(tempo::Key::StartStop),
                    Key::Character("t") => self.tempo.key(tempo::Key::Tap),
                    Key::Character("+" | "=") => self.tempo.key(tempo::Key::Faster),
                    Key::Character("-") => self.tempo.key(tempo::Key::Slower),
                    Key::Character("k") => self.tempo.key(tempo::Key::Click),
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
    let scene = match &args.scene {
        Some(p) => scene::Scene::load(p)?,
        None => scene::Scene::parse(scene::DEFAULT_SCENE)?,
    };
    let graph = graph::Graph::build(&scene)?;
    tracing::info!("scene '{}': {} nodes, {} links", scene.stage.name, graph.nodes.len(), graph.links.len());

    // audio first: its clock is the master clock, its handles feed the control domain
    let has_audio = graph.nodes.iter().any(|n| n.kind.domain() == graph::Domain::Audio);
    let mut handles = None;
    let audio = if has_audio {
        let mut factory = |rate: f32| {
            let (engine, h) = audio::Engine::build(&graph, rate, args.volume)?;
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
    let lib = library::Library {
        api: args.api.clone(),
        cdn: args.cdn.clone(),
        cache: args.cache.clone().unwrap_or_else(library::Library::default_cache),
    };
    let mut music = Vec::new();
    if let Some(h) = handles.as_mut() {
        tracing::info!("library: {} · cache {}", lib.api, lib.cache.display());
        for feed in h.music.drain(..) {
            music.push(feed.status.clone());
            library::spawn_music_loader(lib.clone(), &graph.nodes[feed.node], feed);
        }
    }
    let feeds: Vec<chapter::ChapterFeed> = (0..graph.nodes.len())
        .filter(|&i| graph.nodes[i].kind == graph::Kind::Chapter)
        .map(|i| chapter::spawn(lib.clone(), &graph.nodes[i], i))
        .collect();
    let master = handles.as_ref().map(|h| h.master.clone());
    let clock = handles.as_ref().map(|h| h.clock.clone());
    let timebase = clock.as_ref().map(|c| clock::TimeBase::new(c.rate));
    let controls = control::Controls::new(&graph, handles.as_mut());
    // the tempo clock: the engine's shared map (or its own without audio), the scene's source opened
    let rate = clock.as_ref().map_or(48_000.0, |c| c.rate as f64);
    let shared = handles.as_ref().map(|h| h.tempo.clone()).unwrap_or_default();
    let mut tempo = tempo::Tempo::new(tempo::Config::from_graph(&graph)?, shared, rate);
    tempo.start();

    let event_loop = EventLoop::<Event>::with_user_event().build().context("no event loop (no display?)")?;
    let proxy = event_loop.create_proxy();
    let mut app = App {
        hud: hud::Hud::default(),
        args,
        started,
        graph,
        scene_name: scene.stage.name.clone(),
        render_scale: scene.stage.render_scale,
        controls,
        audio,
        clock,
        music,
        window: None,
        renderer: None,
        hud_at: -1.0,
        last: None,
        t0_ns: clock::now_ns(),
        timebase,
        moment: clock::Moment::wall(clock::now_ns(), clock::now_ns(), rate),
        tempo,
        stats: Stats::default(),
        mods: ModifiersState::empty(),
        exit: 0,
        feeds,
        splash: splash::Fade { alpha: 1.0 },
        master,
        browser: None,
        paused: false,
        proxy,
    };
    event_loop.run_app(&mut app)?;
    let summary = app.summary();
    tracing::info!("{summary}");
    println!("{summary}");
    Ok(app.exit)
}
