//! The native editor's proof (#444, research/NATIVE-EDITOR-GUI.md §6; owner 2026-11-09: our own UI, not GPUI): one window
//! on deck-native's pieces (#443 `host::window`, `gpu::Gpu`), the stage on the left – catpope's core rig played and drawn
//! as the deck does it (stage.rs) –, a panel on the right: a draggable divider, three tabs, the rig's knobs as
//! scrub-numbers (drag turns them in the engine), one text field (umlauts, dead keys, the IME, selection, copy and
//! paste), The design tokens come from a watched file (tokens.rs). Everything is drawn in
//! one 2D pass (ui.rs). The gates are measured and printed at exit: `--frames N` quits after N frames with the numbers
//! (the UI's build time per frame, the frame interval, the audio glitches); `--screenshot FILE` saves the last frame.
//!
//! The Wire side (#431, wires.rs): the top bar's switch or `w`; the ports around the being's points, a dragged cable is a
//! route in the rig through the core's edit and validate, the rig replays, the draft is a file; ctrl+z takes it back.
//!
//! deck-editor [--rig catpope] [--frames N] [--screenshot FILE] [--tokens FILE] [--volume V] [--wire] [--draft FILE] [--gesture FROM,TO]
mod bar;
mod gpu_time;
mod logo;
mod mixer;
mod inspector;
mod stage;
mod tokens;
mod ui;
mod wires;

use anyhow::{Context, Result};
use deck_kit::controls::{self, Rect};
use deck_kit::info::{self, Help};
use deck_kit::{text, typeahead};
use deck_native::gpu::Gpu;
use deck_native::host;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

struct Args {
    rig: String,
    frames: Option<u64>,
    screenshot: Option<PathBuf>,
    tokens: Option<PathBuf>,
    volume: f32,
    /// the window's logical size (the gate is measured at 1920,1080)
    size: Option<(u32, u32)>,
    /// start on the Wire side
    wire: bool,
    /// the rig's draft file (default: the cache's drafts folder)
    draft: Option<PathBuf>,
    /// a check run's drag, by the pointer's own path: from a port to a port (their addresses)
    gesture: Option<(String, String)>,
    /// a check run's view along an axis: 0 1 2 = +x +y +z, 3 4 5 = -x -y -z
    view: Option<usize>,
    /// a check run's level: enter this track
    enter: Option<String>,
    /// a check run's selection: this graph node's id, or a track's
    select: Option<String>,
    /// a check run's knob pull on the stage, in Play
    pull: bool,
    /// a check run's typeahead: arm this port, open it, type this; pick the first row if asked
    menu: Option<(String, String, bool)>,
}

fn args() -> Result<Args> {
    let mut a = Args { rig: "catpope".into(), frames: None, screenshot: None, tokens: None, volume: 0.5, size: None, wire: true, draft: None, gesture: None, view: None, enter: None, select: None, pull: false, menu: None };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().with_context(|| format!("{k} needs a value"));
        match k.as_str() {
            "--rig" => a.rig = val()?,
            "--frames" => a.frames = Some(val()?.parse()?),
            "--screenshot" => a.screenshot = Some(PathBuf::from(val()?)),
            "--tokens" => a.tokens = Some(PathBuf::from(val()?)),
            "--volume" => a.volume = val()?.parse()?,
            "--wire" => a.wire = true,
            "--view" => a.view = Some(val()?.parse()?),
            "--select" => a.select = Some(val()?),
            "--pull" => {
                a.pull = true;
                a.wire = false;
            }
            "--play" => a.wire = false,
            "--enter" => {
                a.enter = Some(val()?);
                a.wire = true;
            }
            "--menu" => {
                let v = val()?;
                let mut p = v.split(',');
                a.menu = Some((p.next().unwrap_or("").to_string(), p.next().unwrap_or("").to_string(), p.next() == Some("pick")));
                a.wire = true;
            }
            "--draft" => a.draft = Some(PathBuf::from(val()?)),
            "--gesture" => {
                let v = val()?;
                let (from, to) = v.split_once(',').with_context(|| format!("--gesture takes FROM,TO, not {v}"))?;
                a.gesture = Some((from.to_string(), to.to_string()));
                a.wire = true;
            }
            "--size" => {
                let v = val()?;
                let (w, h) = v.split_once([',', 'x']).with_context(|| format!("--size takes W,H, not {v}"))?;
                a.size = Some((w.trim().parse()?, h.trim().parse()?));
            }
            "--help" | "-h" => {
                println!("deck-editor [--rig catpope] [--frames N] [--screenshot FILE] [--tokens FILE] [--size W,H] [--volume V] [--wire] [--draft FILE] [--gesture FROM,TO] [--view 0-5] [--menu PORT,QUERY[,pick]]");
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument {other}"),
        }
    }
    Ok(a)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Rig,
    Text,
}

const TABS: [(Tab, &str); 2] = [(Tab::Rig, "Inspector"), (Tab::Text, "Text")];
/// a scrub-number's travel for the knob's whole range (pixels)
const SCRUB_PX: f32 = 240.0;

/// what the pointer is over, from the last layout
#[derive(Clone, Copy, PartialEq)]
enum Hit {
    Nothing,
    Stage,
    Divider,
    Tab(Tab),
    Knob(usize),
    Field(f32),
    /// a parameter of a node in the inspector
    Param(usize, usize),
    /// the mixer's head, a strip's fader, a strip's mute
    MixerHead,
    MixerTab(usize),
    GainText(mixer::Target),
    /// a mixer strip: selects what it is the level of; moving its fader does not
    StripSelect(mixer::Target),
    Fader(mixer::Target, Rect),
    Mute(usize),
    Balance(usize),
    /// a highest-peak readout: a strip's, or the output's
    Peak(Option<mixer::Target>),
    /// the Play / Wire switch (true = Wire)
    Mode(bool),
    /// a port, by its index in the stage's list
    Port(usize),
    /// a row of the tree on the left
    Row(usize),
    /// the Info view's head, and the view itself
    InfoHead,
    InfoBody,
    /// a step of the way down the tree, 0 = the rig
    Crumb(usize),
    /// a track's body at the rig level
    Body(usize),
    /// the view gizmo: an axis (0 1 2 = +x +y +z, 3 4 5 = the minus sides), or its ball
    Axis(Option<usize>),
}

/// what is being dragged
#[derive(Clone, Copy, PartialEq)]
enum Drag {
    Divider,
    Knob(usize, f32, f32),
    /// a strip's fader: which, its travel's rect
    Fader(mixer::Target, Rect),
    /// a strip's pan: which, the pointer's start, the start's value, now
    Balance(usize, f32, f32, f32),
    /// a node's parameter: node, port, the pointer's start, the start's 0..1
    Param(usize, usize, f32, f32),
    Select,
    /// a new cable from this port
    Cable(usize),
    /// the cable into this input, pulled off: dropped on nothing it is cut, on another input it moves
    Grab(usize),
    /// the right button down, not moved yet: a pan if it moves, the context menu if it does not
    Right(f32, f32),
    /// the stage's own press (Play): its knobs, or its orbit
    Space,
    /// the camera, from this pointer position
    Orbit(f32, f32),
    Pan(f32, f32),
    /// the view gizmo: the axis pressed, whether it has moved, the last position
    Gizmo(Option<usize>, bool, f32, f32),
}

/// a port on the screen this frame (logical pixels)
struct Mark {
    port: usize,
    x: f32,
    y: f32,
}

/// what a typed value goes into
#[derive(Clone, Copy, PartialEq)]
enum Editing {
    Knob(usize),
    Param(usize, usize),
    Gain(mixer::Target),
}

/// a value being typed, in place of its field
struct ValueEdit {
    what: Editing,
    at: Rect,
    field: text::Field,
}

/// the typeahead open on the stage
struct Menu {
    ta: typeahead::Typeahead,
    /// the armed port it connects from; none: every port, a pick arms it
    from: Option<usize>,
}

/// a sentence at the place of a gesture, for a few seconds
struct Notice {
    text: String,
    x: f32,
    y: f32,
    until: Instant,
    bad: bool,
}

/// the panel's state and this frame's hit rectangles
struct Panel {
    width: f32,
    tab: Tab,
    field: text::Field,
    drag: Option<Drag>,
    hits: Vec<(Rect, Hit)>,
    field_rect: Rect,
    advance: f32,
}

struct App {
    args: Args,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    ui: Option<ui::Ui>,
    gpu_time: Option<gpu_time::GpuTime>,
    /// how much of the frame the CPU and the GPU work, smoothed
    cpu_load: f32,
    gpu_load: f32,
    stage: stage::Stage,
    tokens: tokens::Watched,
    panel: Panel,
    mods: ModifiersState,
    cursor: (f32, f32),
    last: Option<Instant>,
    started: Instant,
    frames: u64,
    /// per frame: the UI's build and encode time (ms), the frame interval (ms)
    ui_ms: Vec<f32>,
    frame_ms: Vec<f32>,
    exit: i32,
    clipboard: Option<arboard::Clipboard>,
    token_changes: u32,
    /// the Wire side: the ports show, a drag from one is a cable
    wire: bool,
    marks: Vec<Mark>,
    notice: Option<Notice>,
    /// the rigs before each change: ctrl+z
    undo: Vec<deck_space::rig::Bundle>,
    draft: PathBuf,
    /// the ports the cable in the hand may reach (the core's answer, per port)
    reach: Vec<bool>,
    /// a port clicked (not dragged): the next right click connects from it
    armed: Option<usize>,
    menu: Option<Menu>,
    /// this frame's bodies you can click: (x, y, name, what it selects)
    bodies: Vec<(f32, f32, String, inspector::Sel)>,
    /// the last press on a body: for a double click
    last_body: Option<(usize, Instant)>,
    /// is the stage showing a level (Wire) now
    leveled: bool,
    /// the Info view is open (remembered in `info_file`)
    info_open: bool,
    info_file: PathBuf,
    /// where the stage starts (logical pixels), after the tree
    stage_x: f32,
    /// the tree's rows this frame: the level each one goes to (empty = the rig), what it selects
    rows: Vec<(Vec<String>, inspector::Sel)>,
    /// what the inspector shows
    sel: inspector::Sel,
    mixer_open: bool,
    /// the level the mixer shows; none = the one the stage is at
    mixer_tab: Option<usize>,
    /// a value being typed
    edit: Option<ValueEdit>,
    /// the last press on a field: for a double click
    last_field: Option<(Editing, Instant)>,
}

fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,wgpu_core=warn,wgpu_hal=error,naga=warn".into())).init();
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            tracing::error!("{e:#}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let args = args()?;
    let subject = if args.rig.contains('.') { args.rig.clone() } else { format!("character.{}", args.rig) };
    let draft = args.draft.clone().unwrap_or_else(|| deck_native::cache::default_dir().join("drafts").join(format!("{subject}.rig.toml")));
    let stage = stage::Stage::new(&args.rig, args.volume, Some(&draft))?;
    let info_file = deck_native::cache::default_dir().join("editor-info-closed");
    let tokens = tokens::Watched::new(args.tokens.clone().unwrap_or_else(tokens::default_path));
    let event_loop = EventLoop::new().context("no event loop (no display?)")?;
    let width = tokens.tokens.size.inspector;
    let wire = args.wire;
    let mut app = App {
        panel: Panel { width, tab: Tab::Rig, field: text::Field::with("Grüße an die Bühne – type here"), drag: None, hits: Vec::new(), field_rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, advance: 8.0 },
        args,
        window: None,
        gpu: None,
        ui: None,
        gpu_time: None,
        cpu_load: 0.0,
        gpu_load: 0.0,
        stage,
        tokens,
        mods: ModifiersState::empty(),
        cursor: (0.0, 0.0),
        last: None,
        started: Instant::now(),
        frames: 0,
        ui_ms: Vec::new(),
        frame_ms: Vec::new(),
        exit: 0,
        clipboard: arboard::Clipboard::new().map_err(|e| tracing::warn!("no clipboard: {e}")).ok(),
        token_changes: 0,
        wire,
        marks: Vec::new(),
        notice: None,
        undo: Vec::new(),
        draft,
        reach: Vec::new(),
        armed: None,
        menu: None,
        bodies: Vec::new(),
        last_body: None,
        leveled: false,
        info_open: !info_closed(&info_file),
        info_file,
        stage_x: 0.0,
        rows: Vec::new(),
        sel: inspector::Sel::Rig,
        mixer_open: true,
        mixer_tab: None,
        edit: None,
        last_field: None,
    };
    event_loop.run_app(&mut app).context("the event loop")?;
    let summary = app.summary();
    tracing::info!("{summary}");
    println!("{summary}");
    Ok(app.exit)
}

impl App {
    fn summary(&self) -> String {
        let stat = |v: &[f32]| -> (f32, f32) {
            if v.is_empty() {
                return (0.0, 0.0);
            }
            let mut s = v.to_vec();
            s.sort_by(f32::total_cmp);
            (s.iter().sum::<f32>() / s.len() as f32, s[s.len() * 99 / 100])
        };
        let (ui_avg, ui_p99) = stat(&self.ui_ms);
        let (f_avg, f_p99) = stat(&self.frame_ms);
        let size = self.gpu.as_ref().map(|g| g.size()).unwrap_or((0, 0));
        format!(
            "deck-editor: frames={} size={}x{} controls={} ui_ms avg={ui_avg:.2} p99={ui_p99:.2} frame_ms avg={f_avg:.2} p99={f_p99:.2} glitches={} audio={} tokens_reloads={} adapter={}",
            self.frames,
            size.0,
            size.1,
            self.stage.knobs.len(),
            self.stage.glitches(),
            self.stage.audio,
            self.token_changes,
            self.gpu.as_ref().map(|g| g.adapter.name.clone()).unwrap_or_default()
        )
    }

    fn scale(&self) -> f32 {
        self.window.as_ref().map_or(1.0, |w| w.scale_factor() as f32)
    }

    /// what is under the pointer, from the last frame's layout
    fn hit(&self, x: f32, y: f32) -> Hit {
        if let Some(port) = nearest(&self.marks, x, y) {
            return Hit::Port(port);
        }
        if let Some(i) = self.body_at(x, y) {
            return Hit::Body(i);
        }
        self.panel.hits.iter().find(|(r, _)| r.has(x, y)).map_or(Hit::Nothing, |(_, h)| *h)
    }

    /// what the Info view says about what is under the pointer: the name and what it does
    fn help_now(&self) -> Option<Help> {
        let (x, y) = self.cursor;
        Some(match self.hit(x, y) {
            Hit::Port(i) => {
                let p = self.stage.ports.get(i)?;
                let what = format!("{} {}.", if p.input { "An input," } else { "An output," }, p.ty);
                let how = match (p.input, p.cables.last()) {
                    (true, Some(c)) => format!("Fed by {}. Pull it off and let go to cut it, drop it on another in to move it.", c.from),
                    (true, None) => "Free. Drag an out here to connect it.".to_string(),
                    (false, _) => "Drag to an in to connect it.".to_string(),
                };
                Help::new(p.address.clone(), format!("{what} {how} Click to make it active, then right click to find what it can reach. Ctrl+z takes a change back."))
            }
            Hit::Body(i) => Help::new(self.bodies.get(i)?.2.clone(), "A track, one body from outside: only its outlets show. Double click, or point at it and press tab, to go in."),
            Hit::Crumb(0) => Help::new("rig", "Back to the rig, every track one body."),
            Hit::Row(_) => Help::new("Tree", "The rig, its tracks and their nodes. In Wire a click goes to that level."),
            Hit::Crumb(d) => Help::new(self.stage.path.get(d - 1)?.clone(), "Back to this level. Esc or shift+tab goes one level up."),
            Hit::Mode(false) => Help::new("Play", "Play it as a deck shows it: every node, knobs turn on the stage. W does the same."),
            Hit::Mode(true) => Help::new("Stop", "Back to the wiring: the body holds still, the ports show, one level of the tree at a time. W does the same."),
            Hit::Axis(Some(a)) => Help::new(format!("{}{}", if a < 3 { "+" } else { "-" }, ["X", "Y", "Z"][a % 3]), "Click to look along this axis, drag to orbit."),
            Hit::Axis(None) => Help::new("View", "Drag to orbit, click an axis to look along it."),
            Hit::Stage if self.wire => Help::new("Stage", "Left drag orbits, middle or right drag pans, the wheel zooms, F frames. Right click finds any port."),
            Hit::Stage => Help::new("Stage", "Drag a knob to turn it, anywhere else to orbit. Middle drag pans, the wheel zooms."),
            Hit::Knob(i) => Help::new(self.stage.knobs.get(i)?.label.clone(), "Drag or use the wheel to turn it in the engine."),
            Hit::Divider => Help::new("Panel edge", "Drag to resize the panel."),
            Hit::Tab(_) => Help::new("Tab", "Switches what the panel shows."),
            Hit::InfoHead => Help::new("Info", "What is under the pointer. Click to fold or open it."),
            Hit::Param(n, p) => Help::new(self.stage.param(n, p)?.name, "Drag or use the wheel to set it in the engine."),
            Hit::MixerHead => Help::new("Mixer", "One strip per track. Click to fold or open it."),
            Hit::GainText(_) => Help::new("Level", "In dB. Double click to type a value, enter to set it, esc to leave it."),
            Hit::StripSelect(_) => Help::new("Strip", "Click to show what it belongs to in the inspector and on the stage."),
            Hit::MixerTab(_) => Help::new("Level", "The mixer at this level of the tree: the rig's tracks, or the gains inside a track."),
            Hit::Fader(mixer::Target::Track(i), _) => Help::new(self.stage.strips.get(i)?.track.clone(), "The track's level. Drag it; it goes into the draft when you let go."),
            Hit::Fader(mixer::Target::Inner(i), _) => {
                let s = self.stage.inner.get(i)?;
                Help::new(format!("{}:{}", s.track, s.label), format!("How much of {} goes on. Drag it; it goes into the draft when you let go.", s.source))
            }
            Hit::Balance(i) => Help::new(format!("Pan {}", self.stage.strips.get(i)?.track), "Left and right on a stereo pair. Drag it; it applies when you let go."),
            Hit::Peak(_) => Help::new("Highest peak", "The loudest moment in dB, red once it went over 0. Click to clear it."),
            Hit::Mute(i) => Help::new(format!("Mute {}", self.stage.strips.get(i)?.track), "Takes the track out of the sound."),
            Hit::Field(_) | Hit::InfoBody | Hit::Nothing => return None,
        })
    }

    fn body_at(&self, x: f32, y: f32) -> Option<usize> {
        self.bodies.iter().position(|(bx, by, ..)| (bx - x).hypot(by - y) < 26.0)
    }

    /// down into a track
    fn enter(&mut self, track: String) {
        self.mixer_tab = None;
        self.armed = None;
        self.reach.clear();
        let mut path = self.stage.path.clone();
        path.push(track);
        self.stage.enter(path);
    }

    /// up to the level at this depth of the path (0 = the rig)
    fn up_to(&mut self, depth: usize) {
        self.mixer_tab = None;
        self.armed = None;
        self.reach.clear();
        let mut path = self.stage.path.clone();
        path.truncate(depth);
        self.stage.enter(path);
    }

    fn knob_at(&mut self, i: usize, x: f32) {
        let Some(Drag::Knob(_, x0, v0)) = self.panel.drag else { return };
        let k = &self.stage.knobs[i];
        let v = scrub(v0, x - x0, k.min, k.max, k.log);
        let (node, port, min, max) = (k.node, k.port_index, k.min, k.max);
        self.stage.set(node, port, v, min, max);
    }

    fn pointer(&mut self, x: f32, y: f32) {
        self.cursor = (x, y);
        if let Some(m) = self.menu.as_mut() {
            m.ta.hover(x, y);
        }
        match self.panel.drag {
            Some(Drag::Divider) => {
                let w = self.gpu.as_ref().map_or(1280.0, |g| g.size().0 as f32);
                self.panel.width = (w - x).clamp(160.0, w * 0.6);
            }
            Some(Drag::Knob(i, _, _)) => self.knob_at(i, x),
            Some(Drag::Param(node, port, x0, f0)) => {
                if let Some(p) = self.stage.param(node, port) {
                    let v = p.spec.value((f0 + (x - x0) / SCRUB_PX).clamp(0.0, 1.0));
                    self.stage.set(node, port, v, p.spec.min, p.spec.max);
                }
            }
            Some(Drag::Select) => {
                let col = ((x - self.panel.field_rect.x) / self.panel.advance).round().max(0.0) as usize;
                let to = self.panel.field.byte_at_char(col);
                self.panel.field.go(to, true);
            }
            Some(Drag::Cable(_) | Drag::Grab(_)) => {}
            Some(Drag::Space) => {
                let s = self.scale();
                let top = self.tokens.tokens.size.top;
                self.stage.pointer((x - self.stage_x) * s, (y - top) * s);
            }
            Some(Drag::Fader(i, r)) => self.fader_to(i, r, y),
            Some(Drag::Balance(i, x0, v0, _)) => self.panel.drag = Some(Drag::Balance(i, x0, v0, (v0 + (x - x0) / 40.0).clamp(-1.0, 1.0))),
            Some(Drag::Right(lx, ly)) => {
                if (x - lx).abs() + (y - ly).abs() > 3.0 {
                    self.panel.drag = Some(Drag::Pan(x, y));
                }
            }
            Some(Drag::Orbit(lx, ly)) => {
                self.stage.space.camera.drag(x - lx, y - ly);
                self.panel.drag = Some(Drag::Orbit(x, y));
            }
            Some(Drag::Pan(lx, ly)) => {
                let (s, top) = (self.scale(), self.tokens.tokens.size.top);
                let h = self.gpu.as_ref().map_or(720.0, |g| g.size().1 as f32 / s) - top;
                self.stage.space.camera.pan(x - lx, y - ly, h);
                self.panel.drag = Some(Drag::Pan(x, y));
            }
            Some(Drag::Gizmo(axis, moved, lx, ly)) => {
                if moved || (x - lx).abs() + (y - ly).abs() > 3.0 {
                    self.stage.space.camera.drag(x - lx, y - ly);
                    self.panel.drag = Some(Drag::Gizmo(axis, true, x, y));
                }
            }
            None => {
                if let Hit::Stage = self.hit(x, y) {
                    let s = self.scale();
                    let top = self.tokens.tokens.size.top;
                    self.stage.pointer((x - self.stage_x) * s, (y - top) * s);
                }
            }
        }
    }

    fn press(&mut self) {
        let (x, y) = self.cursor;
        // a click elsewhere finishes the typing
        if let Some(e) = &self.edit {
            if e.at.has(x, y) {
                return;
            }
            self.commit_edit();
        }
        if let Some(m) = &self.menu {
            match m.ta.id_at(x, y) {
                Some(id) => self.pick(id),
                None => self.menu = None,
            }
            return;
        }
        match self.hit(x, y) {
            Hit::Divider => self.panel.drag = Some(Drag::Divider),
            Hit::Tab(t) => {
                self.panel.tab = t;
                self.focus(false);
            }
            Hit::Knob(i) => {
                if self.double(Editing::Knob(i)) {
                    let v = self.stage.value(&self.stage.knobs[i]);
                    return self.begin_edit(Editing::Knob(i), Hit::Knob(i), number(v));
                }
                let k = &self.stage.knobs[i];
                let v = self.stage.value(k);
                self.panel.drag = Some(Drag::Knob(i, x, v));
                self.focus(false);
            }
            Hit::Param(n, p) => {
                if self.double(Editing::Param(n, p)) {
                    if let Some(q) = self.stage.param(n, p) {
                        self.begin_edit(Editing::Param(n, p), Hit::Param(n, p), number(q.value));
                    }
                    return;
                }
                if let Some(q) = self.stage.param(n, p) {
                    self.panel.drag = Some(Drag::Param(n, p, x, q.spec.norm(q.value)));
                }
            }
            Hit::Field(_) => {
                let col = ((x - self.panel.field_rect.x) / self.panel.advance).round().max(0.0) as usize;
                let to = self.panel.field.byte_at_char(col);
                self.panel.field.go(to, self.mods.shift_key());
                self.panel.drag = Some(Drag::Select);
                self.focus(true);
            }
            Hit::Mode(on) => self.wire = on,
            Hit::Port(i) => {
                let p = &self.stage.ports[i];
                if p.input && !p.cables.is_empty() {
                    match wires::pulled(&self.stage.bundle, p) {
                        Ok(_) => {
                            let reach = wires::reachable_moved(&self.stage.bundle, &self.stage.ports, p);
                            self.reach = self.at_level(reach);
                            self.panel.drag = Some(Drag::Grab(i));
                        }
                        Err(sentence) => self.say(sentence, x, y, true),
                    }
                } else {
                    let reach = wires::reachable(&self.stage.bundle, &self.stage.ports, p);
                    self.reach = self.at_level(reach);
                    self.panel.drag = Some(Drag::Cable(i));
                }
            }
            Hit::Axis(a) => self.panel.drag = Some(Drag::Gizmo(a, false, x, y)),
            Hit::Crumb(depth) => self.up_to(depth),
            Hit::MixerHead => self.mixer_open = !self.mixer_open,
            Hit::MixerTab(depth) => self.mixer_tab = Some(depth),
            Hit::GainText(target) => {
                if self.double(Editing::Gain(target)) {
                    let gain = match target {
                        mixer::Target::Track(i) => self.stage.strips.get(i).map(|s| self.stage.strip_gain(s)),
                        mixer::Target::Inner(i) => self.stage.inner.get(i).map(|s| self.stage.inner_gain(s)),
                    }
                    .unwrap_or(1.0);
                    self.begin_edit(Editing::Gain(target), Hit::GainText(target), format!("{:.1}", deck_core::meters::level_db(gain)));
                }
            }
            Hit::StripSelect(mixer::Target::Track(i)) => {
                if let Some(s) = self.stage.strips.get(i) {
                    self.sel = inspector::Sel::Track(s.track.clone());
                }
            }
            Hit::StripSelect(mixer::Target::Inner(i)) => {
                if let Some(s) = self.stage.inner.get(i) {
                    self.sel = inspector::Sel::Node(self.stage.inner_source(s));
                }
            }
            Hit::Peak(Some(mixer::Target::Track(i))) => {
                if let Some(s) = self.stage.strips.get_mut(i) {
                    s.max = 0.0;
                }
            }
            Hit::Peak(Some(mixer::Target::Inner(i))) => {
                if let Some(s) = self.stage.inner.get_mut(i) {
                    s.max = 0.0;
                }
            }
            Hit::Peak(None) => self.stage.master_max = 0.0,
            Hit::Fader(i, r) => {
                self.panel.drag = Some(Drag::Fader(i, r));
                self.fader_to(i, r, y);
            }
            Hit::Balance(i) => {
                if let Some(s) = self.stage.strips.get(i) {
                    let v = self.stage.bundle.rig.tracks.iter().find(|t| t.id == s.track).map_or(0.0, |t| t.pan);
                    self.panel.drag = Some(Drag::Balance(i, x, v, v));
                }
            }
            Hit::Mute(i) => {
                if let Some(s) = self.stage.strips.get(i) {
                    let track = s.track.clone();
                    let muted = self.stage.bundle.rig.tracks.iter().find(|t| t.id == track).is_some_and(|t| t.mute);
                    let done = wires::set_mute(&self.stage.bundle, &track, !muted);
                    self.change(done, format!("{track}: {}", if muted { "on" } else { "muted" }), x, y);
                }
            }
            Hit::Row(i) => {
                if let Some((path, sel)) = self.rows.get(i).cloned() {
                    self.sel = sel;
                    if self.wire {
                        self.mixer_tab = None;
                        self.armed = None;
                        self.reach.clear();
                        self.stage.enter(path);
                    }
                }
            }
            Hit::InfoHead => {
                self.info_open = !self.info_open;
                remember_info(&self.info_file, self.info_open);
            }
            Hit::InfoBody => {}
            Hit::Body(i) => {
                let double = self.last_body.is_some_and(|(j, at)| j == i && at.elapsed().as_millis() < 400);
                self.last_body = Some((i, Instant::now()));
                self.sel = self.bodies[i].3.clone();
                if double && matches!(self.sel, inspector::Sel::Track(_)) {
                    let track = self.bodies[i].2.clone();
                    self.enter(track);
                } else {
                    self.panel.drag = Some(Drag::Orbit(x, y));
                }
            }
            Hit::Stage => {
                self.focus(false);
                self.armed = None;
                // Play: the deck's own hand – a knob under the pointer turns, anywhere else it orbits
                if self.wire {
                    self.panel.drag = Some(Drag::Orbit(x, y));
                } else {
                    self.stage.space.press();
                    self.panel.drag = Some(Drag::Space);
                    self.sel_from_stage();
                }
            }
            Hit::Nothing => self.focus(false),
        }
    }

    /// what a press on the stage selected (Play: the deck's own hand) becomes the selection
    fn sel_from_stage(&mut self) {
        use deck_space::space::being::Hit as Picked;
        let being = &self.stage.space.being;
        let node = match being.selected {
            Some(Picked::Button(i)) => being.buttons.get(i).map(|b| b.node),
            Some(Picked::Point(i)) => being.points.get(i).map(|p| p.node),
            None => None,
        };
        if let Some(n) = node {
            self.sel = if self.stage.ports.iter().any(|p| Some(&p.node) == self.stage.node_ids.get(n)) { inspector::Sel::Node(n) } else { inspector::Sel::Rig };
        }
    }

    /// the middle button pans (owner 2026-11-09), with shift it orbits; the right button: a drag pans, a click is the context menu – at
    /// once while a port is held down (#455)
    fn press_view(&mut self, button: MouseButton) {
        let (x, y) = self.cursor;
        if self.menu.take().is_some() {
            return;
        }
        if let (MouseButton::Right, Some(Drag::Cable(a) | Drag::Grab(a))) = (button, self.panel.drag) {
            self.panel.drag = None;
            self.open_menu(Some(a), x, y);
            return;
        }
        if matches!(self.hit(x, y), Hit::Stage | Hit::Port(_)) {
            self.panel.drag = Some(match button {
                MouseButton::Right => Drag::Right(x, y),
                _ if self.mods.shift_key() => Drag::Orbit(x, y),
                _ => Drag::Pan(x, y),
            });
        }
    }

    /// is this press the second of a double click on this field
    fn double(&mut self, what: Editing) -> bool {
        let twice = self.last_field.is_some_and(|(w, at)| w == what && at.elapsed().as_millis() < 400);
        self.last_field = if twice { None } else { Some((what, Instant::now())) };
        twice
    }

    /// type into the field of this hit, its value selected
    fn begin_edit(&mut self, what: Editing, hit: Hit, value: String) {
        let Some(at) = self.panel.hits.iter().find(|(_, h)| *h == hit).map(|(r, _)| *r) else { return };
        let mut field = text::Field::with(&value);
        field.select_all();
        field.focused = true;
        self.panel.drag = None;
        self.edit = Some(ValueEdit { what, at, field });
    }

    /// what was typed goes into the engine and the draft; what does not read as a number leaves everything as it was
    fn commit_edit(&mut self) {
        let Some(e) = self.edit.take() else { return };
        let text = e.field.text.clone();
        match e.what {
            Editing::Knob(i) => {
                let Some(k) = self.stage.knobs.get(i) else { return };
                let (node, port, name, min, max) = (k.node, k.port_index, k.port.clone(), k.min, k.max);
                if let Some(v) = typed_number(&text) {
                    self.stage.set(node, port, v, min, max);
                    self.persist(node, &name);
                }
            }
            Editing::Param(n, p) => {
                let Some(q) = self.stage.param(n, p) else { return };
                if let Some(v) = typed_number(&text) {
                    self.stage.set(n, p, v, q.spec.min, q.spec.max);
                    self.persist(n, q.name);
                }
            }
            Editing::Gain(target) => {
                let Some(gain) = mixer::typed_gain(&text) else { return };
                match target {
                    mixer::Target::Track(i) => {
                        if let Some(s) = self.stage.strips.get(i) {
                            let (node, port) = (s.node, s.port);
                            self.stage.set(node, port, gain, 0.0, mixer::GAIN_MAX);
                        }
                    }
                    mixer::Target::Inner(i) => self.stage.inner_set(i, gain),
                }
                self.store_fader(target);
            }
        }
    }

    /// a node's input as the engine has it now goes into the draft (no replay: the engine has it already)
    fn persist(&mut self, node: usize, name: &str) {
        let Some(id) = self.stage.node_ids.get(node) else { return };
        let Some(p) = self.stage.ports.iter().find(|q| q.input && q.node == *id && q.name == name) else { return };
        let Some((_, local)) = p.address.split_once(':') else { return };
        let done = wires::set_value(&self.stage.bundle, &p.track, local, self.stage.controls.input(node, name, 0.0));
        self.store(done);
    }

    /// a changed document the engine already plays: into the rig, the undo and the draft
    fn store(&mut self, done: Result<deck_space::rig::Bundle, String>) {
        let (x, y) = self.cursor;
        match done {
            Ok(next) => {
                self.undo.push(self.stage.bundle.clone());
                self.stage.bundle = next;
                if let Err(e) = wires::save_draft(&self.draft, &self.stage.base, &self.stage.bundle) {
                    self.say(format!("{e:#}"), x, y, true);
                }
            }
            Err(sentence) => self.say(sentence, x, y, true),
        }
    }

    /// a fader's value goes into the document that holds it
    fn store_fader(&mut self, target: mixer::Target) {
        let done = match target {
            mixer::Target::Track(i) => {
                let Some(s) = self.stage.strips.get(i) else { return };
                wires::set_gain(&self.stage.bundle, &s.track, self.stage.strip_gain(s))
            }
            mixer::Target::Inner(i) => {
                let Some(s) = self.stage.inner.get(i) else { return };
                wires::set_value(&self.stage.bundle, &s.track, &s.path, self.stage.inner_gain(s))
            }
        };
        self.store(done);
    }

    /// a key while a value is typed
    fn edit_key(&mut self, k: &KeyEvent) {
        if k.state != ElementState::Pressed {
            return;
        }
        let ctrl = self.mods.control_key() || self.mods.super_key();
        let shift = self.mods.shift_key();
        match k.logical_key.as_ref() {
            Key::Named(NamedKey::Escape) => self.edit = None,
            Key::Named(NamedKey::Enter) => self.commit_edit(),
            _ => {
                let Some(f) = self.edit.as_mut().map(|e| &mut e.field) else { return };
                match k.logical_key.as_ref() {
                    Key::Named(NamedKey::Backspace) => f.backspace(),
                    Key::Named(NamedKey::Delete) => f.delete(),
                    Key::Named(NamedKey::ArrowLeft) => f.left(ctrl, shift),
                    Key::Named(NamedKey::ArrowRight) => f.right(ctrl, shift),
                    Key::Named(NamedKey::Home) => f.home(shift),
                    Key::Named(NamedKey::End) => f.end(shift),
                    Key::Character(c) if ctrl && c.eq_ignore_ascii_case("a") => f.select_all(),
                    _ => {
                        if let Some(t) = k.text.as_ref().filter(|t| !ctrl && !t.chars().any(char::is_control)) {
                            f.insert(t);
                        }
                    }
                }
            }
        }
    }

    /// a fader under the pointer: the engine at once
    fn fader_to(&mut self, target: mixer::Target, r: Rect, y: f32) {
        let f = (1.0 - (y - r.y) / r.h).clamp(0.0, 1.0);
        let gain = mixer::position_gain(f);
        match target {
            mixer::Target::Track(i) => {
                if let Some(s) = self.stage.strips.get(i) {
                    let (node, port) = (s.node, s.port);
                    self.stage.set(node, port, gain, 0.0, mixer::GAIN_MAX);
                }
            }
            mixer::Target::Inner(i) => self.stage.inner_set(i, gain),
        }
    }

    /// `port` is active: what it may reach lights up
    fn arm(&mut self, port: usize) {
        self.armed = Some(port);
        let reach = wires::reachable(&self.stage.bundle, &self.stage.ports, &self.stage.ports[port]);
        self.reach = self.at_level(reach);
    }

    /// only what the level shows
    fn at_level(&self, reach: Vec<bool>) -> Vec<bool> {
        reach.into_iter().zip(&self.stage.visible).map(|(r, v)| r && *v).collect()
    }

    /// the typeahead: what the armed port may reach (the core's answer), or every port to find one
    fn open_menu(&mut self, from: Option<usize>, x: f32, y: f32) {
        if !self.wire {
            return;
        }
        if let Some(a) = from {
            self.arm(a);
        }
        let items = self.stage.ports.iter().enumerate().filter(|(i, _)| if from.is_none() { self.stage.visible[*i] } else { self.reach.get(*i).copied().unwrap_or(false) });
        let items = items.map(|(id, p)| typeahead::Item { id, label: p.address.clone(), detail: format!("{} {}", if p.input { "in" } else { "out" }, p.ty) }).collect();
        self.menu = Some(Menu { ta: typeahead::Typeahead::new(items, x, y), from });
    }

    /// a row chosen: connected from the armed port, or the port found is armed
    fn pick(&mut self, id: usize) {
        let Some(menu) = self.menu.take() else { return };
        let Some(a) = menu.from else { return self.arm(id) };
        let (x, y) = (menu.ta.x, menu.ta.y);
        let (Some(pa), Some(pb)) = (self.stage.ports.get(a), self.stage.ports.get(id)) else { return };
        let done = wires::connect(&self.stage.bundle, pa, pb);
        let text = format!("{} → {}", pa.address, pb.address);
        self.armed = None;
        self.change(done, text, x, y);
    }

    /// a key while the typeahead is open
    fn menu_key(&mut self, k: &KeyEvent) {
        if k.state != ElementState::Pressed {
            return;
        }
        let Some(m) = self.menu.as_mut() else { return };
        match k.logical_key.as_ref() {
            Key::Named(NamedKey::Escape) => self.menu = None,
            Key::Named(NamedKey::Enter) => {
                if let Some(id) = m.ta.chosen() {
                    self.pick(id);
                }
            }
            Key::Named(NamedKey::ArrowUp) => m.ta.step(-1),
            Key::Named(NamedKey::ArrowDown) => m.ta.step(1),
            Key::Named(NamedKey::ArrowLeft) => m.ta.left(),
            Key::Named(NamedKey::ArrowRight) => m.ta.right(),
            Key::Named(NamedKey::Backspace) => m.ta.backspace(),
            Key::Named(NamedKey::Delete) => m.ta.delete(),
            _ => {
                if let Some(t) = k.text.as_ref().filter(|t| !t.chars().any(char::is_control)) {
                    m.ta.insert(t);
                }
            }
        }
    }

    /// the button let go: a cable dropped on a port is a gesture on the rig
    fn release(&mut self) {
        let a = match self.panel.drag.take() {
            Some(Drag::Cable(a)) => a,
            Some(Drag::Grab(a)) => return self.drop_grabbed(a),
            Some(Drag::Space) => return self.stage.space.release(),
            Some(Drag::Balance(i, _, v0, v)) => {
                if v != v0 {
                    if let Some(track) = self.stage.strips.get(i).map(|s| s.track.clone()) {
                        let (x, y) = self.cursor;
                        let done = wires::set_pan(&self.stage.bundle, &track, v);
                        self.change(done, format!("{track}: pan {}", number(v)), x, y);
                    }
                }
                return;
            }
            // the fader let go: the gain goes into the rig, no replay: the engine has it already
            Some(Drag::Fader(target, _)) => return self.store_fader(target),
            // a knob or a parameter let go: its value goes into the draft
            Some(Drag::Knob(i, ..)) => {
                if let Some(k) = self.stage.knobs.get(i) {
                    let (node, name) = (k.node, k.port.clone());
                    self.persist(node, &name);
                }
                return;
            }
            Some(Drag::Param(n, p, ..)) => {
                if let Some(q) = self.stage.param(n, p) {
                    self.persist(n, q.name);
                }
                return;
            }
            Some(Drag::Right(x, y)) => {
                let on = match self.hit(x, y) {
                    Hit::Port(i) => Some(i),
                    _ => self.armed,
                };
                return self.open_menu(on, x, y);
            }
            Some(Drag::Gizmo(Some(axis), false, ..)) => {
                self.stage.space.camera.view_along(axis % 3, axis < 3);
                return;
            }
            _ => return,
        };
        let (x, y) = self.cursor;
        let Hit::Port(b) = self.hit(x, y) else { return };
        if a == b {
            return self.arm(a);
        }
        let (Some(pa), Some(pb)) = (self.stage.ports.get(a), self.stage.ports.get(b)) else { return };
        let done = wires::connect(&self.stage.bundle, pa, pb);
        let text = format!("{} → {}", pa.address, pb.address);
        self.armed = None;
        self.change(done, text, x, y);
    }

    /// the pulled cable let go: on another input it moves, on nothing it is cut, back on its port nothing happened
    fn drop_grabbed(&mut self, a: usize) {
        let (x, y) = self.cursor;
        let Some(port) = self.stage.ports.get(a) else { return };
        let (done, text) = match self.hit(x, y) {
            Hit::Port(b) if b == a => return self.arm(a),
            Hit::Port(b) if self.stage.ports.get(b).is_some_and(|q| q.input) => {
                let to = &self.stage.ports[b];
                (wires::moved(&self.stage.bundle, &self.stage.ports, port, to), format!("moved to {}", to.address))
            }
            Hit::Port(_) => return,
            _ => (wires::cut(&self.stage.bundle, port), format!("cut: {}", port.address)),
        };
        self.change(done, text, x, y);
    }

    /// a gesture's outcome: on the stage, the sentence at the place either way
    fn change(&mut self, done: Result<deck_space::rig::Bundle, String>, text: String, x: f32, y: f32) {
        match done {
            Ok(next) => {
                let before = self.stage.bundle.clone();
                if self.take(next, x, y) {
                    self.undo.push(before);
                    self.say(text, x, y, false);
                }
            }
            Err(sentence) => self.say(sentence, x, y, true),
        }
    }

    /// a changed rig on the stage and in the draft; false and the sentence when it does not play
    fn take(&mut self, next: deck_space::rig::Bundle, x: f32, y: f32) -> bool {
        if let Err(e) = self.stage.replay(next) {
            self.say(format!("{e:#}"), x, y, true);
            return false;
        }
        if let Err(e) = wires::save_draft(&self.draft, &self.stage.base, &self.stage.bundle) {
            self.say(format!("{e:#}"), x, y, true);
        }
        true
    }

    fn say(&mut self, text: String, x: f32, y: f32, bad: bool) {
        tracing::info!("wire: {}{text}", if bad { "refused: " } else { "" });
        self.notice = Some(Notice { text, x, y, until: Instant::now() + std::time::Duration::from_secs(6), bad });
    }

    fn take_back(&mut self) {
        let Some(next) = self.undo.pop() else { return };
        let (x, y) = self.cursor;
        self.take(next, x, y);
    }

    /// a check run's drag at fixed frames: the pointer's own path, through hit test, press, move and release
    fn script(&mut self) {
        const START: u64 = 60;
        if self.args.pull {
            let top = self.tokens.tokens.size.top;
            let s = self.scale();
            let values = |app: &App| app.stage.knobs.iter().map(|k| app.stage.value(k)).collect::<Vec<f32>>();
            match self.frames {
                f if f == START => {
                    let b = self.stage.space.being.buttons.iter().find(|b| !b.hidden && b.turnable).map(|b| b.pos);
                    if let Some((px, py)) = b.and_then(|p| self.stage.space.to_pixel(p)) {
                        let (x, y) = (px / s, py / s + top);
                        self.pointer(x, y);
                        tracing::info!("wire: pull at {x:.0},{y:.0} before {:?}", values(self));
                        self.press();
                    }
                }
                f if f == START + 10 => {
                    let (x, y) = self.cursor;
                    self.pointer(x + 40.0, y - 40.0);
                }
                f if f == START + 20 => {
                    self.release();
                    tracing::info!("wire: pull after {:?}", values(self));
                }
                _ => {}
            }
            return;
        }
        if let Some((port, query, pick)) = self.args.menu.clone() {
            let i = self.stage.ports.iter().position(|p| p.address == port);
            let at = i.and_then(|i| self.marks.iter().find(|m| m.port == i)).map(|m| (m.x, m.y));
            match self.frames {
                f if f == START => {
                    if let Some((x, y)) = at {
                        self.pointer(x, y);
                        self.press();
                        self.release();
                    } else {
                        tracing::error!("--menu: no port {port} on the stage");
                    }
                }
                f if f == START + 5 => {
                    self.press_view(MouseButton::Right);
                    self.release();
                }
                f if f == START + 10 => {
                    if let Some(m) = self.menu.as_mut() {
                        m.ta.insert(&query);
                    }
                }
                f if f == START + 20 && pick => {
                    if let Some(id) = self.menu.as_ref().and_then(|m| m.ta.chosen()) {
                        self.pick(id);
                    }
                }
                _ => {}
            }
            return;
        }
        let Some((from, to)) = self.args.gesture.clone() else { return };
        let at = |app: &App, address: &str| {
            let i = app.stage.ports.iter().position(|p| p.address == address);
            i.and_then(|i| app.marks.iter().find(|m| m.port == i)).map(|m| (m.x, m.y))
        };
        match self.frames {
            f if f == START => {
                if let Some((x, y)) = at(self, &from) {
                    self.pointer(x, y);
                    self.press();
                } else {
                    tracing::error!("--gesture: no port {from} on the stage");
                }
            }
            f if f == START + 10 && to == "-" => self.pointer(30.0, self.tokens.tokens.size.top + 60.0),
            f if f == START + 10 => {
                if let Some((x, y)) = at(self, &to) {
                    self.pointer(x, y);
                } else {
                    tracing::error!("--gesture: no port {to} on the stage");
                }
            }
            f if f == START + 20 => self.release(),
            _ => {}
        }
    }

    fn focus(&mut self, on: bool) {
        if self.panel.field.focused != on {
            self.panel.field.focused = on;
            if let Some(w) = &self.window {
                w.set_ime_allowed(on);
            }
        }
    }

    /// a key while the field has the focus
    fn key(&mut self, k: &KeyEvent) {
        if k.state != ElementState::Pressed {
            return;
        }
        let (ctrl, shift) = (self.mods.control_key() || self.mods.super_key(), self.mods.shift_key());
        let f = &mut self.panel.field;
        match k.logical_key.as_ref() {
            Key::Named(NamedKey::Backspace) => f.backspace(),
            Key::Named(NamedKey::Delete) => f.delete(),
            Key::Named(NamedKey::ArrowLeft) => f.left(ctrl, shift),
            Key::Named(NamedKey::ArrowRight) => f.right(ctrl, shift),
            Key::Named(NamedKey::Home) => f.home(shift),
            Key::Named(NamedKey::End) => f.end(shift),
            Key::Named(NamedKey::Escape) => self.focus(false),
            Key::Character(c) if ctrl => match c.to_ascii_lowercase().as_str() {
                "a" => f.select_all(),
                "c" | "x" => {
                    let s = f.selected().to_string();
                    if !s.is_empty() {
                        if let Some(cb) = self.clipboard.as_mut() {
                            let _ = cb.set_text(s);
                        }
                        if c.eq_ignore_ascii_case("x") {
                            f.backspace();
                        }
                    }
                }
                "v" => {
                    if let Some(t) = self.clipboard.as_mut().and_then(|cb| cb.get_text().ok()) {
                        f.insert(t.lines().next().unwrap_or(""));
                    }
                }
                _ => {}
            },
            _ => {
                // the text the key produces: umlauts and dead-key compositions arrive here, composed
                if !ctrl && !self.mods.alt_key() {
                    if let Some(t) = k.text.as_ref().filter(|t| !t.chars().any(char::is_control)) {
                        f.insert(t);
                    }
                }
            }
        }
    }

    fn redraw(&mut self, el: &ActiveEventLoop) {
        let help = self.help_now();
        let (Some(window), Some(gpu), Some(ui)) = (self.window.clone(), self.gpu.as_mut(), self.ui.as_mut()) else { return };
        self.stage.space.being.hold(self.wire, false);
        // Wire shows a level of the tree, Play every node
        if self.wire != self.leveled {
            self.leveled = self.wire;
            if self.wire {
                let path = self.stage.path.clone();
                self.stage.enter(path);
            } else {
                self.stage.show_all();
                self.bodies.clear();
            }
        }
        if self.frames == 0 {
            if let Some(a) = self.args.view {
                self.stage.space.camera.view_along(a % 3, a < 3);
            }
            if let Some(track) = self.args.enter.clone() {
                self.stage.enter(vec![track]);
            }
            if let Some(id) = self.args.select.clone() {
                self.sel = match self.stage.node_ids.iter().position(|n| *n == id) {
                    Some(n) => inspector::Sel::Node(n),
                    None => inspector::Sel::Track(id),
                };
            }
        }
        if self.tokens.poll() {
            self.token_changes += 1;
        }
        let now = Instant::now();
        let dt = self.last.map(|l| (now - l).as_secs_f32()).unwrap_or(1.0 / 60.0).min(0.25);
        if let Some(l) = self.last {
            self.frame_ms.push((now - l).as_secs_f32() * 1000.0);
        }
        self.last = Some(now);
        let Some(frame) = gpu.acquire().unwrap_or_else(|e| {
            tracing::error!("render: {e:#}");
            self.exit = 1;
            el.exit();
            None
        }) else {
            window.request_redraw();
            return;
        };
        let t = &self.tokens.tokens;
        let scale = window.scale_factor() as f32;
        let (w, h) = gpu.size();
        let (wf, hf) = (w as f32 / scale, h as f32 / scale);
        let panel_w = self.panel.width.min(wf - 300.0).max(160.0);
        let divider = t.size.edge;
        let tree_w = t.size.tree.min(wf * 0.4);
        self.stage_x = tree_w;
        let stage_x = tree_w;
        let stage_w = (wf - tree_w - panel_w - divider).max(1.0);
        // the regions start under the top bar (bar.rs: the title, the status, the version)
        let top = t.size.top;
        let body = (hf - top).max(1.0);
        let band = if self.mixer_open { mixer::OPEN } else { mixer::FOLDED };
        let stage_h = (body - band).max(1.0);
        let work_start = Instant::now();
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        if let Some(g) = self.gpu_time.as_mut() {
            g.begin(&mut enc);
        }
        // the stage at its viewport's size in pixels
        let (stage_view, fresh) = self.stage.frame(&gpu.device, &gpu.queue, &mut enc, ((stage_w * scale) as u32, (stage_h * scale) as u32), dt);
        if fresh {
            ui.set_stage(&gpu.device, stage_view);
        }
        // the panel: everything in logical pixels, the pass scales
        let t0 = Instant::now();
        ui.begin();
        ui.stage(stage_x, top, stage_w, stage_h);
        let px = panel_w;
        let x0 = wf - panel_w;
        ui.rect(x0, top, px, body, t.color.surface);
        ui.rect(0.0, top, tree_w, body, t.color.surface);
        ui.rect(tree_w - 1.0, top, 1.0, body, t.color.line);
        self.panel.hits.clear();
        self.marks.clear();
        let edge = Rect { x: x0 - divider - 4.0, y: top, w: divider + 8.0, h: body };
        let on_edge = edge.has(self.cursor.0, self.cursor.1) || matches!(self.panel.drag, Some(Drag::Divider));
        controls::edge(ui, t, x0 - divider, top, body, on_edge);
        window.set_cursor(if on_edge { winit::window::CursorIcon::ColResize } else { winit::window::CursorIcon::Default });
        self.panel.hits.push((edge, Hit::Divider));
        let (pad, font, row) = (t.size.pad, t.size.font, t.size.row);
        // the tree on the left: the rig, its tracks, the nodes of the one entered
        let mut rows = vec![controls::TreeRow { depth: 0, label: "rig".into(), detail: String::new(), icon: "nodes", open: Some(true), selected: self.sel == inspector::Sel::Rig }];
        self.rows.clear();
        self.rows.push((Vec::new(), inspector::Sel::Rig));
        for track in &self.stage.bundle.rig.tracks {
            let inside = self.stage.path.first() == Some(&track.id);
            rows.push(controls::TreeRow { depth: 1, label: track.id.clone(), detail: track.instrument.clone(), icon: "folder", open: Some(inside), selected: self.sel == inspector::Sel::Track(track.id.clone()) });
            self.rows.push((vec![track.id.clone()], inspector::Sel::Track(track.id.clone())));
            if inside {
                let mut seen: Vec<&str> = Vec::new();
                for p in self.stage.ports.iter().filter(|p| p.track == track.id) {
                    let name = p.address.split_once(':').and_then(|(_, r)| r.split_once('.')).map_or("", |(n, _)| n);
                    if !seen.contains(&name) {
                        seen.push(name);
                        let node = self.stage.node_ids.iter().position(|id| *id == p.node).unwrap_or(0);
                        rows.push(controls::TreeRow { depth: 2, label: name.to_string(), detail: String::new(), icon: "sliders", open: None, selected: self.sel == inspector::Sel::Node(node) });
                        self.rows.push((vec![track.id.clone()], inspector::Sel::Node(node)));
                    }
                }
            }
        }
        for (i, r) in controls::tree(ui, t, 0.0, top + pad * 0.5, tree_w - 1.0, &rows, self.cursor).into_iter().enumerate() {
            self.panel.hits.push((r, Hit::Row(i)));
        }
        let mut y = top + pad;
        // the tabs
        let names = TABS.map(|(_, name)| name);
        let active = TABS.iter().position(|(tab, _)| *tab == self.panel.tab).unwrap_or(0);
        for (r, (tab, _)) in controls::pills(ui, t, x0 + pad, y, row - 4.0, &names, active).into_iter().zip(TABS) {
            self.panel.hits.push((r, Hit::Tab(tab)));
        }
        y += row + pad / 2.0;
        match self.panel.tab {
            Tab::Rig => {
                let area = Rect { x: x0 + pad, y, w: px - 2.0 * pad, h: hf - y - pad };
                let hot = match self.panel.drag {
                    Some(Drag::Knob(i, ..)) => Some(inspector::Field::Knob(i)),
                    Some(Drag::Param(n, p, ..)) => Some(inspector::Field::Param(n, p)),
                    _ => None,
                };
                for (r, field) in inspector::draw(ui, t, area, &self.stage, &self.sel, hot) {
                    let hit = match field {
                        inspector::Field::Knob(i) => Hit::Knob(i),
                        inspector::Field::Param(n, p) => Hit::Param(n, p),
                    };
                    self.panel.hits.push((r, hit));
                }
            }
            Tab::Text => {
                ui.text(x0 + pad, y, font * 0.85, "umlauts, dead keys, IME · shift+arrows · ctrl+a/c/x/v", t.color.muted);
                y += row * 0.8;
                let fx = x0 + pad;
                let fw = px - 2.0 * pad;
                let focused = self.panel.field.focused;
                ui.rect(fx, y, fw, row, t.color.control);
                if focused {
                    ui.rect(fx, y + row - 2.0, fw, 2.0, t.color.accent);
                }
                let adv = ui.advance(font);
                self.panel.advance = adv;
                self.panel.field_rect = Rect { x: fx + 6.0, y, w: fw - 6.0, h: row };
                let (shown, cursor_col) = self.panel.field.shown();
                if let Some((a, b)) = self.panel.field.selection() {
                    let (ca, cb) = (self.panel.field.chars_before(a) as f32, self.panel.field.chars_before(b) as f32);
                    ui.rect(fx + 6.0 + ca * adv, y + 4.0, (cb - ca) * adv, row - 8.0, t.color.selection);
                }
                ui.text(fx + 6.0, y + 6.0, font, &shown, t.color.text);
                if let Some((p, _)) = &self.panel.field.preedit {
                    let c0 = self.panel.field.chars_before(self.panel.field.cursor) as f32;
                    ui.rect(fx + 6.0 + c0 * adv, y + row - 5.0, p.chars().count() as f32 * adv, 1.5, t.color.accent);
                }
                if focused && (self.started.elapsed().as_millis() / 500) % 2 == 0 {
                    ui.rect(fx + 6.0 + cursor_col as f32 * adv, y + 5.0, 1.5, row - 10.0, t.color.text);
                }
                self.panel.hits.push((Rect { x: fx, y, w: fw, h: row }, Hit::Field(fx + 6.0)));
                if focused {
                    let cx = (fx + 6.0 + cursor_col as f32 * adv) * scale;
                    window.set_ime_cursor_area(winit::dpi::PhysicalPosition::new(cx as i32, (y * scale) as i32), winit::dpi::PhysicalSize::new(2u32, (row * scale) as u32));
                }
                y += row + pad;
                ui.text(x0 + pad, y, font * 0.85, &format!("{} chars · cursor {} · selected {:?}", self.panel.field.text.chars().count(), cursor_col, self.panel.field.selected()), t.color.muted);
            }
        }
        // the status: the numbers of the gates, live (a broken tokens file first: the bar cuts from the end)
        let recent = &self.frame_ms[self.frame_ms.len().saturating_sub(30)..];
        let mean = if recent.is_empty() { 16.7 } else { recent.iter().sum::<f32>() / recent.len() as f32 };
        let glitches = self.stage.glitches();
        let status = format!(
            "{}{:.0} fps{}{}",
            self.tokens.problem.as_ref().map(|p| format!("{p} – ")).unwrap_or_default(),
            1000.0 / mean.max(0.1),
            self.stage.khz().map(|k| format!(" · {k:.0} kHz")).unwrap_or_default(),
            if glitches > 0 { format!(" · {glitches} glitches") } else { String::new() },
        );
        let status = if self.undo.is_empty() { status } else { format!("{} change{} · {status}", self.undo.len(), if self.undo.len() == 1 { "" } else { "s" }) };
        let logo_frame = ((self.started.elapsed().as_secs_f32() / logo::SECONDS).fract() * logo::FRAMES as f32) as usize;
        let title = self.stage.bundle.rig.subject.as_str();
        let button = bar::draw(ui, t, wf, &bar::Bar { title, status: &status, wire: self.wire, logo_frame, pointer: self.cursor, cpu: self.cpu_load, gpu: self.gpu_time.as_ref().map(|_| self.gpu_load) });
        self.panel.hits.push((button, Hit::Mode(!self.wire)));
        // the Wire side: the ports around the being's points, the core's list on this frame's places
        if self.wire {
            let being = &self.stage.space.being;
            self.bodies.clear();
            if self.stage.path.is_empty() {
                for track in &self.stage.bundle.rig.tracks {
                    let node = self.stage.ports.iter().filter(|p| p.track == track.id).find_map(|p| self.stage.node_ids.iter().position(|n| *n == p.node).filter(|n| being.point_world(*n).is_some()));
                    let place = node.and_then(|n| being.point_world(n)).and_then(|w| self.stage.space.to_pixel(w));
                    if let Some((sx, sy)) = place {
                        let (bx, by) = (sx / scale + stage_x, sy / scale + top);
                        self.bodies.push((bx, by, track.id.clone(), inspector::Sel::Track(track.id.clone())));
                        ui.text(bx - 16.0, by + 18.0, font, &track.id, t.color.text);
                    }
                }
            } else {
                // inside a track: its nodes
                let mut seen: Vec<usize> = Vec::new();
                for p in self.stage.ports.iter().filter(|p| self.stage.path.last() == Some(&p.track)) {
                    let Some(n) = self.stage.node_ids.iter().position(|id| *id == p.node) else { continue };
                    if seen.contains(&n) {
                        continue;
                    }
                    seen.push(n);
                    if let Some((sx, sy)) = being.point_world(n).and_then(|w| self.stage.space.to_pixel(w)) {
                        let (bx, by) = (sx / scale + stage_x, sy / scale + top);
                        let name = p.address.split_once(':').and_then(|(_, r)| r.split_once('.')).map_or("", |(n, _)| n).to_string();
                        ui.text(bx - 16.0, by + 18.0, font, &name, t.color.text);
                        self.bodies.push((bx, by, name, inspector::Sel::Node(n)));
                    }
                }
            }
            for (index, q) in self.stage.ports.iter().enumerate().filter(|(i, _)| self.stage.visible[*i]) {
                let Some(node) = self.stage.node_ids.iter().position(|n| *n == q.node) else { continue };
                let Some(world) = being.port_world(node, &q.name, q.input) else { continue };
                let Some((sx, sy)) = self.stage.space.to_pixel(world) else { continue };
                let (mx, my) = (sx / scale + stage_x, sy / scale + top);
                if mx > stage_x + stage_w || mx < stage_x || my < top || my > top + stage_h {
                    continue;
                }
                self.marks.push(Mark { port: index, x: mx, y: my });
                let near = (self.cursor.0 - mx).hypot(self.cursor.1 - my) < 110.0;
                let hover = (self.cursor.0 - mx).hypot(self.cursor.1 - my) < 9.0;
                let c = if q.input { t.color.text } else { t.color.accent };
                let dragging = self.menu.is_some() || self.armed.is_some() || matches!(self.panel.drag, Some(Drag::Cable(_) | Drag::Grab(_)));
                // what the typeahead still shows, else what the cable in the hand may reach
                let open = match &self.menu {
                    Some(m) => m.ta.shows(index),
                    None => self.reach.get(index).copied().unwrap_or(false),
                };
                if self.armed == Some(index) {
                    ui.ring(mx, my, 13.0, t.color.text);
                }
                let c = if dragging && !open { [c[0], c[1], c[2], 0.22] } else { c };
                if dragging && open {
                    ui.ring(mx, my, 10.0, t.color.accent);
                }
                if hover {
                    ui.ring(mx, my, 11.0, t.color.accent);
                }
                if q.input && q.cables.is_empty() {
                    ui.ring(mx, my, 6.0, c);
                } else {
                    ui.disc(mx, my, 6.0, c);
                }
                if near {
                    let small = font * 0.75;
                    let w = ui.advance(small) * q.name.chars().count() as f32;
                    let lx = if q.input { mx - w - 10.0 } else { mx + 10.0 };
                    ui.text(lx, my - small * 0.6, small, &q.name, t.color.text);
                }
            }
            // where we are in the tree, at any depth
            let mut crumbs = vec!["rig".to_string()];
            crumbs.extend(self.stage.path.iter().cloned());
            for (depth, r) in controls::breadcrumb(ui, t, stage_x + pad, top + pad, &crumbs).into_iter().enumerate() {
                self.panel.hits.push((r, Hit::Crumb(depth)));
            }
            if let Some(Drag::Cable(a)) = self.panel.drag {
                if let Some(m) = self.marks.iter().find(|m| m.port == a) {
                    ui.cable((m.x, m.y), self.cursor, t.color.accent);
                }
            }
            if let Some(Drag::Grab(a)) = self.panel.drag {
                let from = self.stage.ports.get(a).and_then(|p| p.cables.last()).map(|c| c.from.clone());
                let source = from.and_then(|f| self.stage.ports.iter().position(|p| p.address == f));
                if let Some(m) = source.and_then(|s| self.marks.iter().find(|m| m.port == s)) {
                    ui.cable((m.x, m.y), self.cursor, t.color.accent);
                }
            }
            if matches!(self.panel.drag, Some(Drag::Cable(_) | Drag::Grab(_))) {
                if let Some(q) = nearest(&self.marks, self.cursor.0, self.cursor.1).and_then(|b| self.stage.ports.get(b)) {
                    controls::chip(ui, t, (self.cursor.0 + 14.0, self.cursor.1 + 14.0), stage_x + stage_w, &format!("{} · {}", q.address, q.ty), false);
                }
            }
        }
        // what is selected shows on the stage
        {
            let being = &self.stage.space.being;
            let nodes: Vec<usize> = match &self.sel {
                inspector::Sel::Rig => Vec::new(),
                inspector::Sel::Node(n) => vec![*n],
                inspector::Sel::Track(id) => {
                    let mut all: Vec<usize> = self.stage.ports.iter().filter(|p| p.track == *id).filter_map(|p| self.stage.node_ids.iter().position(|n| *n == p.node)).collect();
                    all.dedup();
                    all
                }
            };
            for n in nodes {
                if let Some((sx, sy)) = being.point_world(n).and_then(|w| self.stage.space.to_pixel(w)) {
                    let (bx, by) = (sx / scale + stage_x, sy / scale + top);
                    if by < top + stage_h {
                        ui.ring(bx, by, 20.0, t.color.text);
                    }
                }
            }
        }
        // the view gizmo, top right of the stage: its ball orbits, an axis looks along itself
        {
            const R: f32 = 24.0;
            let (gx, gy) = (stage_x + stage_w - R - 20.0, top + R + 20.0);
            let axes = self.stage.space.camera.axes();
            ui.disc(gx, gy, R + 12.0, [t.color.surface_head[0], t.color.surface_head[1], t.color.surface_head[2], 0.55]);
            // back to front to draw; the front ones are hit first
            let mut ends: Vec<(f32, usize)> = (0..6).map(|a| (if a < 3 { axes[a].2 } else { -axes[a % 3].2 }, a)).collect();
            ends.sort_by(|a, b| a.0.total_cmp(&b.0));
            let place = |a: usize| {
                let s = if a < 3 { 1.0 } else { -1.0 };
                (gx + s * axes[a % 3].0 * R, gy + s * axes[a % 3].1 * R)
            };
            for &(_, a) in &ends {
                let (x, y) = place(a);
                let c = t.color.axis[a % 3];
                if a < 3 {
                    ui.line((gx, gy), (x, y), 0.9, c);
                    ui.disc(x, y, 8.0, c);
                    let small = font * 0.75;
                    let half = ui.advance(small) * 0.5;
                    ui.text(x - half, y - small * 0.62, small, ["X", "Y", "Z"][a], t.color.surface);
                } else {
                    ui.ring(x, y, 6.5, c);
                }
            }
            for &(_, a) in ends.iter().rev() {
                let (x, y) = place(a);
                self.panel.hits.push((Rect { x: x - 9.0, y: y - 9.0, w: 18.0, h: 18.0 }, Hit::Axis(Some(a))));
            }
            self.panel.hits.push((Rect { x: gx - R - 12.0, y: gy - R - 12.0, w: 2.0 * (R + 12.0), h: 2.0 * (R + 12.0) }, Hit::Axis(None)));
        }
        let landed = info::view(ui, t, 0.0, hf, tree_w - 1.0, self.info_open, help.as_ref());
        self.panel.hits.push((landed.head, Hit::InfoHead));
        self.panel.hits.push((landed.whole, Hit::InfoBody));
        if let Some(e) = &self.edit {
            let px = font;
            let adv = ui.advance(px);
            let tx = e.at.x + 6.0;
            ui.round_rect(e.at.x, e.at.y, e.at.w.max(52.0), e.at.h, t.size.radius, t.color.control);
            ui.outline(e.at.x, e.at.y, e.at.w.max(52.0), e.at.h, (t.size.radius, 1.0), t.color.accent);
            if let Some((a, b)) = e.field.selection() {
                let (ca, cb) = (e.field.chars_before(a) as f32, e.field.chars_before(b) as f32);
                ui.rect(tx + ca * adv, e.at.y + 2.0, (cb - ca) * adv, e.at.h - 4.0, t.color.selection);
            }
            let (shown, column) = e.field.shown();
            ui.text(tx, e.at.y + (e.at.h - px * 1.25) * 0.5, px, &shown, t.color.text);
            ui.rect(tx + column as f32 * adv, e.at.y + 3.0, 1.5, e.at.h - 6.0, t.color.text);
        }
        if let Some(m) = self.menu.as_mut() {
            m.ta.draw(ui, t, (wf, hf));
        }
        if self.notice.as_ref().is_some_and(|n| Instant::now() > n.until) {
            self.notice = None;
        }
        if let Some(n) = &self.notice {
            controls::chip(ui, t, (n.x + 14.0, n.y - 34.0), stage_x + stage_w, &n.text, n.bad);
        }
        let panning = match self.panel.drag {
            Some(Drag::Balance(i, _, _, v)) => Some((i, v)),
            _ => None,
        };
        let hot = match self.panel.drag {
            Some(Drag::Fader(target, _)) => Some(target),
            _ => None,
        };
        let mut tabs = vec!["rig".to_string()];
        if self.wire {
            tabs.extend(self.stage.path.iter().cloned());
        }
        let tab = self.mixer_tab.unwrap_or(tabs.len() - 1).min(tabs.len() - 1);
        let selected = match &self.sel {
            inspector::Sel::Track(id) if tab == 0 => self.stage.strips.iter().position(|s| s.track == *id).map(mixer::Target::Track),
            inspector::Sel::Node(n) if tab > 0 => self.stage.inner.iter().position(|s| self.stage.inner_source(s) == *n).map(mixer::Target::Inner),
            _ => None,
        };
        let view = mixer::View { tabs, tab, selected, hot, panning };
        for (r, part) in mixer::draw(ui, t, Rect { x: stage_x, y: top + stage_h, w: stage_w, h: band }, self.mixer_open, &self.stage, &view) {
            let hit = match part {
                mixer::Part::Head => Hit::MixerHead,
                mixer::Part::Tab(depth) => Hit::MixerTab(depth),
                mixer::Part::Select(target) => Hit::StripSelect(target),
                mixer::Part::GainText(target) => Hit::GainText(target),
                mixer::Part::Fader(i, f) => Hit::Fader(i, f),
                mixer::Part::Mute(i) => Hit::Mute(i),
                mixer::Part::Pan(i) => Hit::Balance(i),
                mixer::Part::Peak(p) => Hit::Peak(p),
            };
            self.panel.hits.push((r, hit));
        }
        self.panel.hits.push((Rect { x: stage_x, y: top, w: stage_w, h: stage_h }, Hit::Stage));
        // the pass: logical pixels → the surface's, through the screen uniform
        let view = frame.texture.create_view(&Default::default());
        ui.flush(&gpu.device, &gpu.queue, &mut enc, &view, (wf, hf), t.color.stage);
        self.ui_ms.push(t0.elapsed().as_secs_f32() * 1000.0);
        let shot = self.args.screenshot.clone().filter(|_| self.args.frames.is_some_and(|n| self.frames + 1 >= n) && gpu.can_capture);
        let readback = shot.as_ref().map(|_| copy_out(&gpu.device, &mut enc, &frame.texture, (w, h)));
        if let Some(g) = self.gpu_time.as_mut() {
            g.end(&mut enc);
        }
        // the CPU's part ends here: present waits for the display
        let cpu_ms = work_start.elapsed().as_secs_f32() * 1000.0;
        gpu.queue.submit([enc.finish()]);
        frame.present();
        let budget = self.frame_ms.last().copied().unwrap_or(16.7).max(1.0);
        let ease = |load: f32, now: f32| load + (now - load) * 0.08;
        self.cpu_load = ease(self.cpu_load, cpu_ms / budget);
        if let Some(g) = self.gpu_time.as_mut() {
            g.after_submit(&gpu.device);
            self.gpu_load = ease(self.gpu_load, g.last_ms / budget);
        }
        if let (Some(path), Some((buf, stride))) = (shot, readback) {
            if let Err(e) = save(&gpu.device, &buf, stride, (w, h), gpu.format(), &path) {
                tracing::error!("screenshot: {e:#}");
            } else {
                tracing::info!("screenshot: {}", path.display());
            }
        }
        self.script();
        self.frames += 1;
        if self.args.frames.is_some_and(|n| self.frames >= n) {
            el.exit();
            return;
        }
        window.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = match host::window(el, "SCUTOONS editor – proof", true) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("no window: {e}");
                self.exit = 1;
                el.exit();
                return;
            }
        };
        match Gpu::with_features(window.clone(), gpu_time::FEATURES).and_then(|gpu| ui::Ui::new(&gpu.device, &gpu.queue, gpu.format(), self.tokens.tokens.size.font).map(|ui| (gpu, ui))) {
            Ok((gpu, ui)) => {
                self.gpu_time = gpu_time::GpuTime::new(&gpu.device, &gpu.queue);
                self.gpu = Some(gpu);
                self.ui = Some(ui);
            }
            Err(e) => {
                tracing::error!("render: {e:#}");
                self.exit = 1;
                el.exit();
                return;
            }
        }
        window.set_cursor_visible(true);
        if let Some((w, h)) = self.args.size {
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(w, h));
        }
        self.window = Some(window.clone());
        window.request_redraw();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => {
                if let Some(g) = self.gpu.as_mut() {
                    g.resize(s.width, s.height);
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {}
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            WindowEvent::CursorMoved { position, .. } => {
                let s = self.scale();
                self.pointer(position.x as f32 / s, position.y as f32 / s);
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => self.press(),
            WindowEvent::MouseInput { state: ElementState::Pressed, button: button @ (MouseButton::Middle | MouseButton::Right), .. } => self.press_view(button),
            WindowEvent::MouseInput { state: ElementState::Released, .. } => self.release(),
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                match self.hit(self.cursor.0, self.cursor.1) {
                    Hit::Knob(i) => {
                        let k = &self.stage.knobs[i];
                        let v = self.stage.value(k);
                        let v = scrub(v, steps * 12.0, k.min, k.max, k.log);
                        let (node, port, min, max) = (k.node, k.port_index, k.min, k.max);
                        self.stage.set(node, port, v, min, max);
                    }
                    Hit::Param(n, port) => {
                        if let Some(p) = self.stage.param(n, port) {
                            let v = p.spec.value((p.spec.norm(p.value) + steps * 12.0 / SCRUB_PX).clamp(0.0, 1.0));
                            self.stage.set(n, port, v, p.spec.min, p.spec.max);
                        }
                    }
                    Hit::Stage | Hit::Port(_) | Hit::Axis(_) => self.stage.space.camera.zoom(steps),
                    _ => {}
                }
            }
            WindowEvent::KeyboardInput { event, .. } if self.edit.is_some() => self.edit_key(&event),
            WindowEvent::KeyboardInput { event, .. } if self.menu.is_some() => self.menu_key(&event),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) && !self.panel.field.focused {
                    if self.wire && !self.stage.path.is_empty() {
                        self.up_to(self.stage.path.len() - 1);
                    } else {
                        el.exit();
                    }
                } else if self.wire && event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Tab) && !self.panel.field.focused {
                    if self.mods.shift_key() {
                        self.up_to(self.stage.path.len().saturating_sub(1));
                    } else if let Some(i) = self.body_at(self.cursor.0, self.cursor.1).filter(|&i| matches!(self.bodies[i].3, inspector::Sel::Track(_))) {
                        let track = self.bodies[i].2.clone();
                        self.enter(track);
                    }
                } else if self.panel.field.focused {
                    self.key(&event);
                } else if event.state == ElementState::Pressed && event.logical_key == Key::Character("z".into()) && self.mods.control_key() {
                    self.take_back();
                } else if event.state == ElementState::Pressed && event.logical_key == Key::Character("w".into()) {
                    self.wire = !self.wire;
                } else if event.state == ElementState::Pressed && event.logical_key == Key::Character("f".into()) {
                    self.stage.space.frame_being();
                } else if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Tab) {
                    let i = TABS.iter().position(|(t, _)| *t == self.panel.tab).unwrap_or(0);
                    self.panel.tab = TABS[(i + 1) % TABS.len()].0;
                }
            }
            WindowEvent::Ime(ime) => match ime {
                Ime::Preedit(s, cursor) => self.panel.field.preedit(s, cursor),
                Ime::Commit(s) => self.panel.field.insert(&s),
                Ime::Enabled | Ime::Disabled => {}
            },
            WindowEvent::RedrawRequested => self.redraw(el),
            _ => {}
        }
    }
}

/// is the Info view folded (a file in the cache)
fn info_closed(file: &std::path::Path) -> bool {
    file.is_file()
}

fn remember_info(file: &std::path::Path, open: bool) {
    if open {
        std::fs::remove_file(file).ok();
    } else {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        std::fs::write(file, "").ok();
    }
}

/// a number as someone types it: a point or a German comma, a typographic minus
pub(crate) fn typed_number(text: &str) -> Option<f32> {
    text.trim().replace(',', ".").replace('−', "-").parse().ok().filter(|v: &f32| v.is_finite())
}

/// the port under (x, y): the nearest one, the being's points lie close together
fn nearest(marks: &[Mark], x: f32, y: f32) -> Option<usize> {
    const REACH: f32 = 12.0;
    marks.iter().map(|m| (m.port, (m.x - x).hypot(m.y - y))).filter(|(_, d)| *d < REACH).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(p, _)| p)
}

/// a knob's value under the pointer's travel `dx` from `v0` (the whole range over SCRUB_PX; a log knob in its exponent)
fn scrub(v0: f32, dx: f32, min: f32, max: f32, log: bool) -> f32 {
    let f0 = fraction(v0, min, max, log);
    let f = (f0 + dx / SCRUB_PX).clamp(0.0, 1.0);
    if log && min > 0.0 && max > 0.0 {
        min * (max / min).powf(f)
    } else {
        min + (max - min) * f
    }
}

fn fraction(v: f32, min: f32, max: f32, log: bool) -> f32 {
    if log && min > 0.0 && max > 0.0 && v > 0.0 {
        ((v / min).ln() / (max / min).ln()).clamp(0.0, 1.0)
    } else if (max - min).abs() > f32::EPSILON {
        ((v - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn number(v: f32) -> String {
    if v.abs() >= 100.0 {
        format!("{v:.0}")
    } else if v.abs() >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.3}")
    }
}

/// the surface's texture into a buffer, for `save` (rows padded to 256 bytes)
fn copy_out(device: &wgpu::Device, enc: &mut wgpu::CommandEncoder, tex: &wgpu::Texture, (w, h): (u32, u32)) -> (wgpu::Buffer, u32) {
    let stride = (w * 4).div_ceil(256) * 256;
    let buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("screenshot"), size: u64::from(stride) * u64::from(h), usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(stride), rows_per_image: None } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    (buf, stride)
}

fn save(device: &wgpu::Device, buf: &wgpu::Buffer, stride: u32, (w, h): (u32, u32), format: wgpu::TextureFormat, path: &std::path::Path) -> Result<()> {
    let slice = buf.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::Wait).context("waiting for the screenshot")?;
    rx.recv().context("the map's answer")?.context("mapping the screenshot")?;
    let data = slice.get_mapped_range();
    let bgra = matches!(format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for row in 0..h {
        let line = &data[(row * stride) as usize..(row * stride + w * 4) as usize];
        for p in line.chunks_exact(4) {
            if bgra {
                px.extend_from_slice(&[p[2], p[1], p[0], 255]);
            } else {
                px.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
    }
    drop(data);
    buf.unmap();
    image::RgbaImage::from_raw(w, h, px).context("the screenshot's pixels")?.save(path).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod typed {
    use super::typed_number;

    #[test]
    fn a_number_reads_as_people_write_it() {
        assert_eq!(typed_number(" 1.5 "), Some(1.5));
        assert_eq!(typed_number("1,5"), Some(1.5));
        assert_eq!(typed_number("−3"), Some(-3.0));
        assert_eq!(typed_number("abc"), None);
        assert_eq!(typed_number("inf"), None);
    }
}
