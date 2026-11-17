//! The native editor's proof (#444, research/NATIVE-EDITOR-GUI.md §6; owner 2026-11-09: our own UI, not GPUI): one window
//! on deck-native's pieces (#443 `host::window`, `gpu::Gpu`), the stage on the left – catpope's core rig played and drawn
//! as the deck does it (stage.rs) –, a panel on the right: a draggable divider, three tabs, the rig's knobs as
//! scrub-numbers (drag turns them in the engine), one text field (umlauts, dead keys, the IME, selection, copy and
//! paste), The design tokens come from a watched file (tokens.rs). Everything is drawn in
//! one 2D pass (ui.rs). The gates are measured and printed at exit: `--frames N` quits after N frames with the numbers
//! (the UI's build time per frame, the frame interval, the audio glitches); `--screenshot FILE` saves the last frame.
//!
//! The Wire side (#431, wires.rs): the top bar's switch or `w`; the ports around the being's points, a dragged cable is a
//! route in the rig through the core's edit and validate, the rig replays, the draft is its outbox entry; ctrl+z takes it back.
//!
//! deck-editor [--rig catpope] [--frames N] [--screenshot FILE] [--tokens FILE] [--volume V] [--wire] [--draft FILE] [--gesture FROM,TO]
mod adding;
mod autosave;
mod bar;
mod book;
mod colour;
mod empty;
mod gpu_time;
mod grid;
mod library;
mod logo;
mod mixer;
mod module;
mod inspector;
mod panels;
mod published;
mod routing;
mod stage;
mod tokens;
mod ui;
mod rigs;
mod saving;
mod scene;
mod scene_parts;
mod settings;
mod show;
mod sign_in;
mod tools;
mod toon;
mod wires;

use anyhow::{Context, Result};
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::info::{self, Help};
use deck_kit::base::text;
use deck_kit::editor::typeahead;
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

static CACHE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// the editor's cache: the one for the API it talks to (set at the start), a test's own folder in tests
pub fn cache_dir() -> PathBuf {
    if cfg!(test) {
        return deck_native::cache::test_dir();
    }
    CACHE.get_or_init(deck_native::cache::default_dir).clone()
}

struct Args {
    rig: String,
    /// fullscreen on the current monitor (a deck's kiosk), else a 1280×720 window
    fullscreen: bool,
    /// the inspector's tab at the start: inspector | text; audio opens the Audio panel
    tab: Tab,
    audio: bool,
    frames: Option<u64>,
    screenshot: Option<PathBuf>,
    tokens: Option<PathBuf>,
    volume: f32,
    /// the window's logical size (the gate is measured at 1920,1080)
    size: Option<(u32, u32)>,
    /// start on the Wire side
    wire: bool,
    /// a rig draft file to play and keep the rig in, instead of the outbox
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
    /// #91: the workspace it opens on, the show (a file, or an id of /api/shows), the relay its conductor joins, the API,
    /// and a check run's presses (a reader's event, a row, a cell, GO)
    workspace: show::Workspace,
    show_file: Option<PathBuf>,
    show_id: Option<String>,
    relay: Option<String>,
    api: String,
    script: show::Script,
    /// #477: the toon whose address book the typeahead offers (the editor knows no toon of its own), or a book file instead
    /// of the API; a check run's address typeahead on the selected far world node: type this, pick the first row if asked
    toon: Option<String>,
    book: Option<PathBuf>,
    far: Option<(String, bool)>,
    /// #539: a check run's story events (the API's answer in a file) in place of `GET /api/creator/story-events`
    story: Option<PathBuf>,
    /// the Module workspace: a module to open, and a check run's build and save
    module: module::Script,
    /// #568: a check run's rigs (`<id>.toml` bundles) for the cells' pieces the deck carries no core rig of
    rigs: Option<PathBuf>,
    /// a check run's catalog: the answers of `/catalog/docs/<kind>` as `<kind>.json` in a folder
    catalog: Option<PathBuf>,
    /// a check run's Add typeahead: type this, pick the first row if asked
    add: Option<(String, bool)>,
    /// a check run's scene member (scene.rs `script_scene`): selected, a knob turned, opened as its rig if asked
    member: Option<(String, bool)>,
    /// a demo show opened in Show (show/demos.rs)
    demo: Option<String>,
    /// a check run's grid & snap on every view (grid.rs `script`)
    grid: Option<String>,
    /// the look and the UI size for this run (else the remembered ones)
    theme: Option<tokens::Theme>,
    ui_size: Option<f32>,
    /// the settings panel open at the start
    settings: bool,
    /// a check run's panic pill, pressed at the start
    panic: bool,
    /// a check run shows the save pill in this state: saving, offline or conflict (its screenshots)
    save_state: Option<String>,
    /// a check run's tools pill folded at the start
    tools_folded: bool,
    /// a check run's panels closed in every workspace, and the main menu open
    closed: Vec<String>,
    main_menu: bool,
    /// a check run's colour picker on the shown background: open | pick | drop:X,Y | take:X,Y (colour.rs `script_colour`)
    colour: Option<String>,
    /// a check run's library: a card of this address dragged to X,Y and let go there (`hold`: kept in the hand)
    library_drag: Option<library::ScriptDrag>,
    /// a check run's library in the list mode, filtered by a query
    library_find: Option<String>,
    library_list: bool,
}

fn args() -> Result<Args> {
    let mut a = Args { module: module::Script::default(), rig: "catpope".into(), fullscreen: false, frames: None, screenshot: None, tokens: None, volume: 0.5, size: None, wire: true, draft: None, gesture: None, view: None, enter: None, select: None, pull: false, menu: None, tab: Tab::Rig, audio: false, workspace: show::Workspace::Build, show_file: None, show_id: None, relay: None, api: std::env::var("DECK_API").unwrap_or_else(|_| deck_native::cache::API.into()), script: show::Script::default(), toon: None, book: None, far: None, story: None, rigs: None, catalog: None, add: None, member: None, demo: None, grid: None, theme: None, ui_size: None, settings: false, panic: false, save_state: None, tools_folded: false, closed: Vec::new(), main_menu: false, colour: None, library_drag: None, library_find: None, library_list: false };
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
            "--fullscreen" => a.fullscreen = true,
            "--panic" => a.panic = true,
            "--colour" => a.colour = Some(val()?),
            "--library-drag" => {
                let v = val()?;
                let p: Vec<&str> = v.split(',').collect();
                let hold = p.last() == Some(&"hold");
                let at = match (p.get(1), p.get(2)) {
                    (Some(x), Some(y)) if *x != "hold" => Some((x.parse()?, y.parse()?)),
                    _ => None,
                };
                a.library_drag = Some((p[0].to_string(), at, hold));
            }
            "--library-find" => a.library_find = Some(val()?),
            "--library-list" => a.library_list = true,
            "--save-state" => a.save_state = Some(val()?),
            "--tools-folded" => a.tools_folded = true,
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
            "--tab" => match val()? {
                v if v.eq_ignore_ascii_case("audio") => a.audio = true,
                v => a.tab = TABS.iter().find(|(_, n)| n.eq_ignore_ascii_case(&v)).map(|(t, _)| *t).context("--tab inspector | text | audio")?,
            },
            "--workspace" => a.workspace = show::Workspace::parse(&val()?).context("--workspace build | module | show | perform")?,
            "--show" => match val()? {
                v if v.ends_with(".json") => a.show_file = Some(PathBuf::from(v)),
                v => a.show_id = Some(v),
            },
            "--relay" => a.relay = Some(val()?),
            "--api" => a.api = val()?,
            "--toon" => a.toon = Some(val()?),
            "--book" => a.book = Some(PathBuf::from(val()?)),
            "--story-events" => a.story = Some(PathBuf::from(val()?)),
            "--module" => {
                a.module.open = Some(val()?);
                a.workspace = show::Workspace::Module;
            }
            "--module-build" => a.module.build = true,
            "--module-save" => a.module.save = true,
            "--module-save-as" => a.module.save_as = Some(val()?),
            "--rigs" => a.rigs = Some(PathBuf::from(val()?)),
            "--catalog" => a.catalog = Some(PathBuf::from(val()?)),
            "--demo" => {
                a.demo = Some(val()?);
                a.workspace = show::Workspace::Show;
            }
            "--add" => {
                let v = val()?;
                a.add = Some(v.strip_suffix(",pick").map_or((v.clone(), false), |q| (q.to_string(), true)));
            }
            "--member" => {
                let v = val()?;
                a.member = Some(v.strip_suffix(",inside").map_or((v.clone(), false), |h| (h.to_string(), true)));
            }
            "--grid" => a.grid = Some(val()?),
            "--theme" => a.theme = Some(if val()? == "night" { tokens::Theme::Night } else { tokens::Theme::Day }),
            "--ui-size" => a.ui_size = Some(val()?.parse()?),
            "--settings" => a.settings = true,
            "--close" => a.closed.extend(val()?.split(',').map(str::trim).filter(|n| !n.is_empty()).map(String::from)),
            "--main-menu" => a.main_menu = true,
            "--open-build" => a.script.open_build = true,
            "--to-cell" => a.script.to_cell = true,
            "--new-scene" => a.script.new_scene = Some(show::NewScene::parse(&val()?).context("--new-scene ID,NAME,FILLER+FILLER[,play|build]")?),
            "--far" => {
                let v = val()?;
                a.far = Some(match v.strip_suffix(",pick") {
                    Some(q) => (q.to_string(), true),
                    None => (v, false),
                });
            }
            "--event" => a.script.event = Some(val()?),
            "--fire" => a.script.fire = Some(val()?),
            "--launch" => a.script.launch = val()?.split_once(',').map(|(r, c)| (r.to_string(), c.to_string())),
            "--stop" => a.script.stop = Some(val()?),
            "--go" => a.script.go = true,
            "--flash" => a.script.flash = true,
            "--flash-filter" => a.script.flash_filter = Some(val()?),
            "--flash-ask" => a.script.flash_ask = true,
            "--flash-confirm" => a.script.flash_confirm = true,
            "--flash-force" => a.script.flash_force = true,
            "--seat" => a.script.seat = Some(val()?),
            "--row" => a.script.row = Some(val()?),
            "--sheet" => a.script.sheet = Some(show::Sheet::parse(&val()?).context("--sheet cues | problems | flash | who | session | vote | event | scene | demos | new-show")?),
            "--role" => a.script.role = Some(val()?),
            "--who-move" => a.script.who_move = val()?.split_once('=').map(|(c, r)| (c.to_string(), r.to_string())),
            "--who-menu" => a.script.who_menu = Some(val()?),
            "--linked" => a.script.linked = val()?.split(',').map(str::trim).filter(|n| !n.is_empty()).map(String::from).collect(),
            "--reader-place" => {
                let v = val()?;
                let mut p = v.splitn(3, ',');
                let toon = p.next().unwrap_or_default().trim().to_string();
                let chapter = p.next().map(|c| c.trim().parse()).transpose().context("--reader-place TOON,CHAPTER[,LABEL]")?;
                a.script.reader_place = Some(deck_ui::session::Place { toon, chapter, label: p.next().unwrap_or_default().trim().to_string() });
            }
            "--vote-tally" => a.script.vote_tally = Some(val()?),
            "--problem" => a.script.problem = Some(val()?.parse().context("--problem N (0-based)")?),
            "--author" => {
                let v = val()?;
                let mut p = v.splitn(3, ',').map(|s| s.trim().to_string());
                let (row, column) = (p.next().unwrap_or_default(), p.next().context("--author ROW,COLUMN[,PIECE]")?);
                a.script.author = Some((row, column, p.next().filter(|s| !s.is_empty())));
            }
            "--in-place" => a.script.in_place = Some(val()?),
            "--cell" => a.script.cell = val()?.split_once(',').map(|(r, c)| (r.trim().to_string(), c.trim().to_string())),
            "--question" => {
                let v = val()?;
                let (query, pick) = v.strip_suffix(",pick").map_or((v.as_str(), false), |q| (q, true));
                a.script.question = Some((query.trim().to_string(), pick));
            }
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
                println!("deck-editor [--rig catpope] [--fullscreen] [--frames N] [--screenshot FILE] [--tokens FILE] [--size W,H] [--volume V] [--wire] [--draft FILE] [--gesture FROM,TO] [--view 0-5] [--menu PORT,QUERY[,pick]] [--toon TOON] [--book FILE] [--select NODE --far QUERY[,pick]] [--demo NAME] [--panic] [--colour open|pick|drop:X,Y|take:X,Y] [--save-state saving|offline|conflict] [--settings] [--close ID,ID] [--main-menu] [--module ID [--module-build] [--module-save] [--module-save-as ID]]");
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument {other}"),
        }
    }
    a.script.character.clone_from(&a.rig);
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
    /// a panel's ground: it keeps the press from the stage
    Ground,
    /// the main menu's hamburger (its lines: `menu_lines`)
    Menu,
    /// the toolbar's panic pill
    Panic,
    /// the toolbar's day / night switch
    Look,
    /// the toolbar's tempo pill, its account icon, its meters
    Tempo,
    Account,
    Meters,
    Tab(Tab),
    Knob(usize),
    Field(f32),
    /// a parameter of a node in the inspector
    Param(usize, usize),
    /// a far world node's address in the inspector (#477)
    Address(usize),
    /// the mixer's level tab, a strip's fader, a strip's mute
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
    /// a row of the rig's tree
    Row(usize),
    /// a step of the way down the tree, 0 = the rig (in Show: of the conductors' path)
    Crumb(usize),
    /// a track's body at the rig level
    Body(usize),
    /// the view gizmo: an axis (0 1 2 = +x +y +z, 3 4 5 = the minus sides), or its ball
    Axis(Option<usize>),
    /// #91: a workspace pill in the top bar
    Workspace(show::Workspace),
    /// Build's rig row: the rig picker
    RigPick,
    /// Build's Add, the inspector's Remove
    Add,
    Remove,
    Background,
    /// a background's swatch: the colour picker
    BackgroundColour,
    /// the Audio panel's settings, map and tones
    Routing(routing::Field),
    /// #568: Build linked to a show cell – its knobs into the cell, back to Show
    ToCell,
    BackToShow,
    /// the scene's inspector (scene.rs): a member's knob, its ×, "Open as rig"
    SceneKnob(usize),
    SceneClear(usize),
    SceneInto,
    /// a slot's or a video node's field (scene_parts.rs)
    SceneField(scene::Field),
    /// a way to start the rig of a character without one (empty.rs)
    Way(empty::Way),
}

/// what is being dragged
#[derive(Clone, Copy, PartialEq)]
enum Drag {
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
    /// a scene member in the hand (grid.rs)
    Member,
    /// the camera, from this pointer position
    Orbit(f32, f32),
    Pan(f32, f32),
    /// the view gizmo: the axis pressed, whether it has moved, the last position
    Gizmo(Option<usize>, bool, f32, f32),
    /// a knob of the scene's member: which, the pointer's start, the start's value
    SceneKnob(usize, f32, f32),
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
    Background,
    SceneBackground,
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
    /// #477: the address book's entries for a far world node's address (track, node, its address): a pick is its holder
    far: Option<(String, String, deck_space::rig::address::Address)>,
    /// the rig picker: a pick opens that rig (rigs.rs)
    rigs: bool,
    /// Build's Add: what each row adds (adding.rs)
    add: Vec<adding::Choice>,
    /// the address book for a new far world node in this track
    far_new: Option<String>,
    /// the scene's slots and video nodes (scene_parts.rs)
    scene: Option<scene_parts::Pick>,
}

/// a sentence at the place of a gesture, for a few seconds
struct Notice {
    text: String,
    x: f32,
    y: f32,
    until: Instant,
    bad: bool,
}

/// the inspector's state and this frame's hit rectangles
struct Panel {
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
    /// this machine's settings, where they are kept (none in a check run), their panel's places
    settings: settings::Settings,
    settings_file: Option<PathBuf>,
    settings_hits: Vec<(Rect, settings::Press)>,
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
    draft: rigs::Copy,
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
    /// Build's floating panels; where the viewer left every workspace's (none in a check run: it starts from the defaults)
    build: panels::Panels,
    panels_file: Option<PathBuf>,
    panels_moved: bool,
    /// the main menu is open; its lines this frame
    main_menu: bool,
    menu_lines: Vec<(Rect, bar::Line)>,
    /// the tree's rows this frame: the level each one goes to (empty = the rig), what it selects
    rows: Vec<(Vec<String>, inspector::Sel)>,
    /// what the inspector shows
    sel: inspector::Sel,
    /// the level the mixer shows; none = the one the stage is at
    mixer_tab: Option<usize>,
    /// a value being typed
    edit: Option<ValueEdit>,
    /// the last press on a field: for a double click
    last_field: Option<(Editing, Instant)>,
    /// #91: Build, Show or Perform; the show's conductor and what the show workspaces show
    workspace: show::Workspace,
    show: show::Host,
    /// #477: the toon's address book, of the toon open now (toon.rs) – the toon and whether signed in when it was asked
    book: book::Loader,
    book_key: (Option<String>, bool),
    rig_toon: toon::RigToon,
    auth: sign_in::Auth,
    rigs: rigs::Rigs,
    /// the one hand (deck-kit `carry`): a library card on its way into a view
    carry: Option<deck_kit::editor::carry::Carry>,
    routing: routing::Page,
    module: module::Module,
    /// the catalog modules Build's rig plays, fetched and loaded
    published: published::Published,
    /// Build edits this scene (scene.rs); none = a rig
    scene: Option<scene::SceneView>,
    /// the scene tree's last press, for a double one
    last_row: Option<(deck_space::scene_edit::Part, Instant)>,
    /// Build's grid & snap (grid.rs)
    grid: grid::GridSnap,
    colour: colour::Colour,
    /// the workspace the last frame saved in: leaving one saves what it edits
    saved_in: show::Workspace,
    /// a check run saves nothing by itself (it never writes to the server)
    autosaves: bool,
    /// the tools pill (tools.rs)
    tools: tools::Tools,
    /// the library panel (library.rs) and a module to open once the Module workspace lists it
    library: library::Library,
    library_open: Option<String>,
}

/// the language of the book's labels: the editor's own words are English (not translated yet)
const LOCALE: &str = "en";

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
    let _ = CACHE.set(deck_native::cache::for_api(&args.api));
    let subject = if args.rig.contains('.') { args.rig.clone() } else { format!("character.{}", args.rig) };
    let cache = cache_dir();
    // a check run never reads or writes the outbox
    let outbox = if args.frames.is_none() { autosave::Outbox::open(&cache) } else { autosave::Outbox::none() };
    if args.frames.is_none() {
        rigs::migrate(&cache, &outbox);
    }
    let (base, at) = rigs::base(&cache, &subject)?;
    let (draft, plays, made_on) = match &args.draft {
        Some(f) => (rigs::Copy::File(f.clone()), wires::load_draft(f, &base), wires::draft_version(f)),
        None if args.frames.is_some() => (rigs::Copy::Nothing, base.clone(), None),
        None => {
            let (plays, made_on) = rigs::kept(&outbox, &subject, &base);
            (rigs::Copy::Outbox, plays, made_on)
        }
    };
    let version = made_on.unwrap_or(at);
    let stage = stage::Stage::new(base, plays, args.volume)?;
    let panels_file = args.frames.is_none().then(|| cache_dir().join("editor-panels.json"));
    let remembered = panels_file.as_deref().map(panels::load).unwrap_or_default();
    let mut build = panels::Panels::new(&panels::BUILD);
    let settings_file = args.frames.is_none().then(|| settings::file(&cache));
    let app_colour = colour::Colour::load(args.frames.is_none().then(|| colour::file(&cache)));
    let mut app_settings = settings_file.as_deref().map(settings::load).unwrap_or_default();
    app_settings.look.theme = args.theme.unwrap_or(app_settings.look.theme);
    app_settings.look.base = args.ui_size.unwrap_or(app_settings.look.base);
    let tokens = tokens::Watched::new(args.tokens.clone().unwrap_or_else(tokens::default_path), app_settings.look);
    let event_loop = EventLoop::new().context("no event loop (no display?)")?;
    let wire = args.wire;
    let app_workspace = args.workspace;
    let app_autosaves = args.frames.is_none();
    // a check run keeps no grid settings
    let app_grid_file = args.frames.is_none().then(|| grid::cache_file(&cache));
    // a check run (--frames) never asks for a sign-in: SCUTOONS_TOKEN or the kept token only
    let auth = sign_in::Auth::start(&args.api, &cache, args.frames.is_none());
    let mut rigs = if args.frames.is_none() || args.script.new_scene.is_some() { rigs::Rigs::new(&args.api, &cache, &subject, version, auth.token().is_some()) } else { rigs::Rigs::offline(&cache, &subject, version, "a check run asks no server") };
    if let Some(dir) = &args.catalog {
        rigs.read_catalog(dir).map_err(|e| anyhow::anyhow!("--catalog {}: {e}", dir.display()))?;
    }
    // the kept changes the server has not got yet go up by themselves
    if stage.bundle != stage.base {
        rigs.auto.edit(Instant::now());
    }
    let rig_toon = if args.toon.is_none() && args.book.is_none() { toon::RigToon::new(&args.api, &subject, &cache) } else { toon::RigToon::none() };
    let mut app_show = show::Host::new(args.show_file.clone(), args.show_id.clone(), args.relay.clone(), args.api.clone());
    app_show.toon.clone_from(&args.toon);
    app_show.knobs.rigs.clone_from(&args.rigs);
    if let Some(f) = &args.story {
        app_show.story_from_file(f);
    }
    if let Some(name) = &args.demo {
        app_show.open_demo(name).map_err(anyhow::Error::msg)?;
    }
    let app_book = book::Loader::new(&args.api, args.toon.as_deref(), args.book.as_deref(), auth.token(), &cache);
    let book_key = (args.toon.clone(), auth.token().is_some());
    let mut app_module = module::Module::new(&args.api, &cache, args.module.clone());
    app_module.autosaves = args.frames.is_none();
    let mut app_library = library::Library::new(cache.clone());
    if let Some(dir) = &args.catalog {
        if let Some(list) = app_library.answers(dir) {
            app_module.list = list;
        }
    }
    let app_published = published::Published::new(&args.api, &cache);
    if let Some(p) = remembered.get("build") {
        build.floats.restore(p);
    }
    if let Some(p) = remembered.get("module") {
        app_module.panels.floats.restore(p);
    }
    if let Some(p) = remembered.get("show") {
        app_show.show_panels.floats.restore(p);
    }
    if let Some(p) = remembered.get("perform") {
        app_show.perform_panels.floats.restore(p);
    }
    if args.audio {
        build.floats.reopen("audio");
    }
    if args.tools_folded {
        for p in [&mut build.floats, &mut app_module.panels.floats, &mut app_show.show_panels.floats, &mut app_show.perform_panels.floats] {
            p.set_collapsed("tools", true);
        }
    }
    if args.settings {
        for p in [&mut build, &mut app_module.panels, &mut app_show.show_panels, &mut app_show.perform_panels] {
            p.floats.reopen(panels::SETTINGS);
        }
    }
    for id in &args.closed {
        for p in [&mut build, &mut app_module.panels, &mut app_show.show_panels, &mut app_show.perform_panels] {
            p.floats.close(id);
        }
    }
    let main_menu = args.main_menu;
    let mut app = App {
        panel: Panel { tab: args.tab, field: text::Field::with("Grüße an die Bühne – type here"), drag: None, hits: Vec::new(), field_rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, advance: 8.0 },
        args,
        window: None,
        gpu: None,
        ui: None,
        gpu_time: None,
        cpu_load: 0.0,
        gpu_load: 0.0,
        stage,
        tokens,
        settings: app_settings,
        settings_file,
        settings_hits: Vec::new(),
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
        build,
        panels_file,
        panels_moved: false,
        main_menu,
        menu_lines: Vec::new(),
        rows: Vec::new(),
        sel: inspector::Sel::Rig,
        mixer_tab: None,
        edit: None,
        last_field: None,
        workspace: app_workspace,
        saved_in: app_workspace,
        autosaves: app_autosaves,
        show: app_show,
        book: app_book,
        book_key,
        rig_toon,
        auth,
        rigs,
        carry: None,
        routing: routing::Page::new(deck_native::audio::backend::DeckAudio::read()),
        module: app_module,
        published: app_published,
        scene: None,
        last_row: None,
        grid: grid::GridSnap::load(app_grid_file),
        colour: app_colour,
        tools: tools::Tools::default(),
        library: app_library,
        library_open: None,
    };
    event_loop.run_app(&mut app).context("the event loop")?;
    app.flush_all();
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
            "deck-editor: frames={} size={}x{} controls={} ui_ms avg={ui_avg:.2} p99={ui_p99:.2} frame_ms avg={f_avg:.2} p99={f_p99:.2} glitches={} audio={} peaks={} tokens_reloads={} adapter={}",
            self.frames,
            size.0,
            size.1,
            self.stage.knobs.len(),
            self.stage.glitches(),
            self.stage.audio,
            self.stage.strips.iter().map(|s| format!("{}:{:.3}", s.track, s.max)).collect::<Vec<_>>().join(","),
            self.token_changes,
            self.gpu.as_ref().map(|g| g.adapter.name.clone()).unwrap_or_default()
        )
    }

    fn scale(&self) -> f32 {
        self.window.as_ref().map_or(1.0, |w| w.scale_factor() as f32)
    }

    /// the window's size in logical pixels
    fn window_size(&self) -> (f32, f32) {
        let s = self.scale();
        self.window.as_ref().map_or((1280.0, 720.0), |w| (w.inner_size().width as f32 / s, w.inner_size().height as f32 / s))
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

    /// the panels of the workspace shown
    fn panels(&self) -> &panels::Panels {
        match self.workspace {
            show::Workspace::Build => &self.build,
            show::Workspace::Module => &self.module.panels,
            show::Workspace::Show => &self.show.show_panels,
            show::Workspace::Perform => &self.show.perform_panels,
        }
    }

    fn panels_mut(&mut self) -> &mut panels::Panels {
        match self.workspace {
            show::Workspace::Build => &mut self.build,
            show::Workspace::Module => &mut self.module.panels,
            show::Workspace::Show => &mut self.show.show_panels,
            show::Workspace::Perform => &mut self.show.perform_panels,
        }
    }

    /// where the viewer left the panels, kept when one was moved or folded
    fn remember_panels(&mut self) {
        let Some(file) = &self.panels_file else { return };
        let mut all = panels::Remembered::new();
        all.insert("build".into(), self.build.floats.places());
        all.insert("module".into(), self.module.panels.floats.places());
        all.insert("show".into(), self.show.show_panels.floats.places());
        all.insert("perform".into(), self.show.perform_panels.floats.places());
        panels::save(file, &all);
    }

    /// whether a main menu line can be done here, and if not why
    fn menu_why_not(&self, a: bar::Action) -> Option<&'static str> {
        use bar::Action as A;
        match a {
            A::Open if self.workspace == show::Workspace::Perform => Some("Perform runs the show open: open another in Show or Build."),
            A::Open if self.workspace == show::Workspace::Module => Some(module::draw::PICK),
            A::Open | A::Audio | A::DayNight | A::Settings => None,
            A::New if self.workspace == show::Workspace::Show => None,
            A::New => Some("A new show starts in Show; a new rig in the creator."),
            A::Publish => Some("The editor opens no browser yet: publish the saved draft in the creator."),
            A::Account => Some("The editor opens no browser yet: the account is on scutoons.com."),
            A::Quit if std::env::var_os("DECK_SWAP").is_some() => None,
            A::Quit => Some("Only when the editor was opened from the deck (F9 goes back to its stage)."),
        }
    }

    /// the frames a second over the last half second or so
    fn fps(&self) -> f32 {
        let recent = &self.frame_ms[self.frame_ms.len().saturating_sub(30)..];
        let mean = if recent.is_empty() { 16.7 } else { recent.iter().sum::<f32>() / recent.len() as f32 };
        1000.0 / mean.max(0.1)
    }

    /// a press on the tools pill, or an option its menu or a key picked
    fn tool_press(&mut self, p: tools::Press) {
        let (x, y) = self.cursor;
        if let Some(why) = tools::later(p) {
            return self.say(why.to_string(), x, y, false);
        }
        match p {
            tools::Press::Tool(tool) => self.tools.tool = tool,
            tools::Press::View(two_d) => self.show.press(show::Hit::View(two_d)),
            tools::Press::Grid => self.grid.toggle(),
            tools::Press::Frame => self.stage.space.frame_being(),
            tools::Press::Look(_) | tools::Press::Menu(_) => {}
        }
    }

    /// a main menu line pressed
    fn menu_act(&mut self, a: bar::Action) {
        use bar::Action as A;
        self.main_menu = false;
        if self.menu_why_not(a).is_some() {
            return;
        }
        let (x, y) = self.cursor;
        match (a, self.workspace) {
            (A::Open, show::Workspace::Build) => self.open_rigs(x, y),
            (A::Open, _) => self.show.press(show::Hit::Load),
            (A::New, _) => self.show.press(show::Hit::Sheet(show::Sheet::NewShow)),
            (A::Audio, _) => {
                self.workspace = show::Workspace::Build;
                self.build.floats.reopen("audio");
                self.panels_moved = true;
            }
            (A::DayNight, _) => self.look(self.tokens.look.theme.other(), 0),
            (A::Settings, _) => self.open_settings(),
            // the next frame leaves, as F9 does
            (A::Quit, _) => self.exit = 11,
            _ => {}
        }
    }

    /// another look or a step of the UI size (0: the size stays, unless the theme is the same: back to 100 %)
    fn look(&mut self, theme: tokens::Theme, step: i32) {
        let now = self.settings.look;
        let next = if theme != now.theme { tokens::Look { theme, ..now } } else { now.sized(step) };
        self.change_settings(|s| s.look = next);
    }

    /// the settings changed: kept, and the look applied at once
    fn change_settings(&mut self, f: impl FnOnce(&mut settings::Settings)) {
        let was = self.settings;
        f(&mut self.settings);
        if self.settings == was {
            return;
        }
        if let Some(file) = &self.settings_file {
            settings::save(file, &self.settings);
        }
        if self.tokens.set(self.settings.look) {
            self.token_changes += 1;
            self.panels_moved = true;
        }
    }

    /// the settings as a floating panel of the workspace shown
    fn open_settings(&mut self) {
        self.panels_mut().floats.reopen(panels::SETTINGS);
        self.panels_moved = true;
    }

    /// a line of the main menu's panel list: the panel closes, or comes back open
    fn menu_panel(&mut self, id: &str) {
        self.main_menu = false;
        self.panels_mut().flip(id);
        self.panels_moved = true;
    }

    /// what the Info view says about what is under the pointer: the name and what it does
    fn help_now(&self) -> Option<Help> {
        let (x, y) = self.cursor;
        if let Some((spec, chrome)) = self.panels().floats.chrome_at(x, y) {
            let how = match chrome {
                deck_kit::editor::floats::Chrome::Handle if spec.inline && self.panels().floats.collapsed(spec.id) => "Click to open it; drag to move it.",
                deck_kit::editor::floats::Chrome::Handle => "Drag the handle to move it; it stops 1rem before a wall or another panel.",
                deck_kit::editor::floats::Chrome::Fold => "Click to fold it to its pill or open it again.",
                deck_kit::editor::floats::Chrome::Close => "Click to close it; the main menu's Panels bring it back.",
            };
            return Some(Help::new(spec.name, format!("{} {how}", spec.about)));
        }
        if let Some((name, text)) = self.tools.help_at(x, y) {
            return Some(Help::new(name, text));
        }
        if self.main_menu {
            match self.menu_lines.iter().find(|(r, _)| r.has(x, y)).map(|(_, l)| *l) {
                Some(bar::Line::Do(a)) => {
                    let label = bar::MENU.iter().find(|(m, ..)| *m == a).map_or("", |(_, l, _)| *l);
                    return Some(Help::new(label, self.menu_why_not(a).unwrap_or("Click to do it.")));
                }
                Some(bar::Line::Panel(id)) => {
                    let spec = self.panels().floats.spec(id)?;
                    let how = if self.panels().floats.closed(id) { "Closed: click to bring it back." } else { "Open: click to close it." };
                    return Some(Help::new(spec.name, format!("{} {how}", spec.about)));
                }
                None => {}
            }
        }
        Some(match self.hit(x, y) {
            Hit::Menu => Help::new("Menu", "Open, publish, day or night, the settings, the audio and the account; the workspace's panels; back to the deck's stage."),
            Hit::Tempo => Help::new(format!("{} BPM", self.stage.beat.map_or_else(|| "–".to_string(), |(b, _)| format!("{b:.1}"))), match self.stage.beat {
                Some((_, true)) => "INT: the deck's own tempo clock runs; it sets the tempo of what plays here.",
                Some((_, false)) => "INT: the deck's own tempo clock, standing.",
                None => "No tempo clock plays here.",
            }),
            Hit::Account => {
                let line = self.auth.line(&deck_link::pairing::app_url());
                match (self.auth.state(), self.auth.name()) {
                    (sign_in::Signed::In, Some(name)) => Help::new("Signed in", format!("As the editor {name:?}: it reads and saves drafts; publishing is in the creator.")),
                    (sign_in::Signed::In, None) => Help::new("Signed in", "With a token of its own: it reads and saves drafts; publishing is in the creator."),
                    (sign_in::Signed::Signing, _) => Help::new("Signing in", "Asking the server for this editor's code …"),
                    (sign_in::Signed::Out, _) => Help::new("Not signed in", format!("{line}. Click to see it again.")),
                }
            }
            Hit::Meters => Help::new("Load", bar::meters_words(self.cpu_load, self.gpu_time.as_ref().map(|_| self.gpu_load), self.stage.khz().map(|k| k as f32), self.fps())),
            Hit::Look => Help::new("Day / night", "Click to switch between the light and the dark look; ctrl + and ctrl − change the UI size, ctrl 0 sets it back."),
            Hit::Panic if self.show.sub.panicked() => Help::new("Resume", "The panic holds. Press to bring every deck back to the cells the show holds now."),
            Hit::Panic => Help::new("Panic", "Black and silence on every deck below this conductor and on this stage, until you resume here. Shift+Esc too."),
            Hit::Ground => return None,
            Hit::Workspace(_) => Help::new("Workspaces", "Build: rigs and scenes. Module: a module's code in your editor, built here, heard on the stage, saved as a draft. Show: rows, cells, seats, saved through /api/shows. Perform: the matrix, GO, the log."),
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
            Hit::Mode(false) => Help::new("Play", "Play it as a deck shows it: every node, knobs turn on the stage."),
            Hit::Mode(true) => Help::new("Stop", "Back to the wiring: the body holds still, the ports show, one level of the tree at a time."),
            Hit::Axis(Some(a)) => Help::new(format!("{}{}", if a < 3 { "+" } else { "-" }, ["X", "Y", "Z"][a % 3]), "Click to look along this axis, drag to orbit."),
            Hit::Axis(None) => Help::new("View", "Drag to orbit, click an axis to look along it."),
            Hit::Stage if self.wire => Help::new("Stage", "Left drag orbits, middle or right drag pans, the wheel zooms, F frames. Right click finds any port."),
            Hit::Stage => Help::new("Stage", "Drag a knob to turn it, anywhere else to orbit. Middle drag pans, the wheel zooms."),
            Hit::Knob(i) => Help::new(self.stage.knobs.get(i)?.label.clone(), "Drag or use the wheel to turn it in the engine."),
            Hit::Tab(_) => Help::new("Tab", "Switches what the inspector shows."),
            Hit::Routing(f) => {
                let (title, text) = self.routing.help(f);
                Help::new(title, text)
            }
            Hit::Param(n, p) => Help::new(self.stage.param(n, p)?.name, "Drag or use the wheel to set it in the engine."),
            Hit::Address(_) => Help::new("Address", "Click: the toon's address book. A pick becomes the holder this far world node listens to."),
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
            Hit::RigPick => Help::new("Rig", "Click: the toon's rigs on the server; a pick opens it on the stage. A local copy that differs from the server's is offered."),
            Hit::Add => Help::new("Add", "Inside a track: a node, a far world node with its holder from the address book. At the rig: a track playing an instrument of the catalog. Ctrl+z takes it back."),
            Hit::BackgroundColour => Help::new("Colour", "Choose the background's colour: the palette, your recent colours, saturation and brightness over the hue, or the eyedropper – a click takes the colour under the pointer on the stage, Esc cancels."),
            Hit::Background => Help::new("Background", "The stage's background while this rig plays: a colour (#rrggbb) or a media path (an image laid over the stage). Empty = its modules', else black; a scene or a show over it wins."),
            Hit::Remove => Help::new("Remove", "Takes the selected node or track away, its cables with it (delete does the same). Ctrl+z takes it back."),
            Hit::Mute(i) => Help::new(format!("Mute {}", self.stage.strips.get(i)?.track), "Takes the track out of the sound."),
            Hit::ToCell => Help::new(show::build::TO_CELL, "Writes the knobs as Build has them now into the show cell this rig was opened from; the show checks their ranges."),
            Hit::BackToShow => Help::new(show::build::BACK, "The Show workspace, the linked cell selected."),
            Hit::SceneKnob(_) => Help::new("Value in the scene", "Drag to set this member's knob in the scene; the stage plays it when you let go; the scene saves itself as a draft."),
            Hit::SceneClear(_) => Help::new("Its rig's value", "Takes the scene's value off: the member plays its rig's own."),
            Hit::SceneInto => Help::new("Open as rig", "One level down: the member's rig alone on the stage, the scene's values on its knobs. Esc or the scene's line takes them back into the scene."),
            Hit::Way(empty::Way::Modules) => Help::new("Start from modules", "Add at the rig: a track playing a module or an instrument of the catalog; the rig saves itself as its first draft."),
            Hit::Way(empty::Way::NewModule) => Help::new("New module", "A module of the character's own as a draft, opened in the Module workspace: write it, build it, hear it, then add it here as a track."),
            Hit::SceneField(scene::Field::Background) => Help::new("Background", "The stage's background in this scene: a colour (#rrggbb), an image's media path laid over the stage, or a video's (it does not play yet). Empty = its rigs', else black; a show over it wins."),
            Hit::SceneField(_) => Help::new("Slot or video", "A slot's filter (every chip on holds), its preview from the toon's book; a video node's place and knobs. The core refuses what does not fit, in its words."),
                        Hit::Field(_) | Hit::Nothing => return None,
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
        if self.colour.dragging() {
            let d = self.colour.drag_to(x, y);
            return self.colour_do(d);
        }
        if let Some(m) = self.menu.as_mut() {
            m.ta.hover(x, y);
        }
        if self.panels_mut().floats.drag_to(x, y) {
            return;
        }
        self.show.pointer(x, y);
        self.library_drag_to(x, y);
        if let Some(c) = self.carry.as_mut() {
            c.to(x, y);
        }
        if self.module.dragging() {
            return self.module.drag_to(x, &mut self.stage);
        }
        if self.grid.scrub_to(x) {
            return;
        }
        match self.panel.drag {
            Some(Drag::Member) if self.tools.tool == tools::Tool::Move => self.grid_drag(x, y),
            Some(Drag::Member) => {}
            Some(Drag::Knob(i, _, _)) => self.knob_at(i, x),
            Some(Drag::SceneKnob(i, x0, v0)) => self.scene_knob_to(i, x0, v0, x),
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
                self.stage.pointer(x * s, y * s);
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
                let s = self.scale();
                let h = self.gpu.as_ref().map_or(720.0, |g| g.size().1 as f32 / s);
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
                    self.stage.pointer(x * s, y * s);
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
        if self.colour.taking() {
            let d = self.colour.press(x, y);
            return self.colour_do(d);
        }
        // the main menu takes the press while it is open: a line does what it says, anywhere else closes it
        if self.main_menu {
            match self.menu_lines.iter().find(|(r, _)| r.has(x, y)).map(|(_, l)| *l) {
                Some(bar::Line::Do(a)) if self.menu_why_not(a).is_some() => {}
                Some(bar::Line::Do(a)) => self.menu_act(a),
                Some(bar::Line::Panel(id)) => self.menu_panel(id),
                None => self.main_menu = false,
            }
            return;
        }
        // a panel's handle takes it in the hand, its chevron folds it, its close closes it
        if self.panels_mut().floats.press(x, y) {
            self.panels_moved = true;
            return;
        }
        self.library.refused = None;
        match self.library.browser.press(x, y, library::now()) {
            Some(did) => return self.library_did(did),
            None => self.library.browser.stop_typing(),
        }
        if let Some((r, p)) = self.settings_hits.iter().find(|(r, _)| r.has(x, y)).copied() {
            let p = settings::at(&r, p, x);
            return self.change_settings(|s| s.press(p));
        }
        if let Some(p) = self.tools.press(x, y) {
            return self.tool_press(p);
        }
        if self.workspace == show::Workspace::Module {
            if let Some(h) = self.module.hit(x, y) {
                return self.module.press(h, x, &self.stage);
            }
            if self.panels().floats.covers(x, y) {
                return;
            }
        } else if self.workspace != show::Workspace::Build {
            // #551: the right click's typeahead on the selected cells takes the press while it is open
            if self.show.context_press(x, y) {
                return;
            }
            if let Some(h) = self.show.hit(x, y) {
                if h == show::Hit::BackgroundColour {
                    return self.colour_open(colour::Target::Show);
                }
                // #551: shift and a press add a cell to the row's selection
                return self.show.press(if self.mods.shift_key() || self.mods.control_key() { h.with_shift() } else { h });
            }
            if self.panels().floats.covers(x, y) {
                return;
            }
        } else if self.grid.press(x, y) {
            return;
        }
        match self.hit(x, y) {
            Hit::Workspace(w) => {
                if self.workspace == show::Workspace::Module && w != show::Workspace::Module {
                    self.module.leave(&mut self.stage);
                }
                self.workspace = w;
            }
            Hit::Menu => self.main_menu = true,
            Hit::Look => self.look(self.tokens.look.theme.other(), 0),
            Hit::Panic => self.panic_pill(),
            Hit::Account => {
                let line = self.auth.line(&deck_link::pairing::app_url());
                let said = match self.auth.name() {
                    Some(name) => format!("signed in as the editor {name:?}"),
                    None if line.is_empty() => "signed in".to_string(),
                    None => line,
                };
                self.say(said, x, y, self.auth.state() == sign_in::Signed::Out);
            }
            Hit::Tempo | Hit::Meters => {}
            Hit::Ground => {}
            Hit::Crumb(i) if self.workspace != show::Workspace::Build => self.show.press(show::Hit::Crumb(i)),
            Hit::Routing(f) => routing::act(&mut self.routing, &mut self.stage, f),
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
            Hit::Address(n) => self.open_book(n, x, y),
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
            Hit::Row(i) if self.scene.as_ref().is_some_and(|v| i < v.rows.len()) => {
                let part = self.scene.as_ref().map(|v| v.rows[i]).expect("checked");
                self.scene_row(part);
            }
            Hit::Row(i) => {
                let i = i - self.scene.as_ref().map_or(0, |v| v.rows.len());
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
                if self.grid_grab(x, y) {
                    self.panel.drag = Some(Drag::Member);
                } else if self.wire {
                    self.panel.drag = Some(Drag::Orbit(x, y));
                } else {
                    self.stage.space.press();
                    self.panel.drag = Some(Drag::Space);
                    self.sel_from_stage();
                }
            }
            Hit::RigPick => self.open_rigs(x, y),
            Hit::Add if self.scene.as_ref().is_some_and(|v| v.inside.is_none()) => self.scene_add(x, y),
            Hit::SceneField(f) => self.scene_field(f, x, y),
            Hit::Add => self.open_add(x, y),
            Hit::Way(w) => self.start_rig(w, x, y),
            Hit::Remove => self.remove_selected(),
            Hit::Background => {
                let now = self.stage.bundle.rig.background.as_ref().map_or("", |b| b.text()).to_string();
                self.begin_edit(Editing::Background, Hit::Background, now);
            }
            Hit::BackgroundColour => self.colour_open(colour::Target::Rig),
            Hit::ToCell => self.knobs_to_cell(),
            Hit::BackToShow => self.workspace = show::Workspace::Show,
            Hit::SceneKnob(i) => {
                if let Some(v) = self.scene_knob(i) {
                    self.panel.drag = Some(Drag::SceneKnob(i, x, v));
                }
            }
            Hit::SceneClear(i) => self.scene_clear(i),
            Hit::SceneInto => {
                if let Some(m) = self.scene.as_ref().and_then(|v| v.member()) {
                    self.scene_into(m);
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
        // #551: Show's right click on a cell of the rows band is the same typeahead, on the selected cells of a row
        // (show/ask.rs); elsewhere the stage pans as in Build
        if self.workspace == show::Workspace::Show && button == MouseButton::Right && self.show.context(x, y) {
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
            Editing::Background => self.rig_background(&text),
            Editing::SceneBackground => self.scene_background(&text),
        }
    }

    /// the rig's background as typed or picked: into the rig, the undo and the draft
    fn rig_background(&mut self, text: &str) {
        let (x, y) = self.cursor;
        match deck_space::Background::typed(text) {
            Ok(b) => {
                let mut next = self.stage.bundle.clone();
                next.rig.background = b;
                let before = self.stage.bundle.clone();
                if self.take(next, x, y) {
                    self.undo.push(before);
                }
            }
            Err(e) => self.say(e, x, y, true),
        }
    }

    fn scene_background(&mut self, text: &str) {
        let (x, y) = self.cursor;
        let Some(view) = self.scene.as_mut() else { return };
        match view.edit.set_background(text) {
            Ok(said) => {
                self.replay_scene();
                self.say(said, x, y, false);
            }
            Err(e) => self.say(e, x, y, true),
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
                self.drafted(x, y);
            }
            Err(sentence) => self.say(sentence, x, y, true),
        }
    }

    /// the rig as it plays into its copy; outside a scene it saves itself (inside, the scene takes its knobs)
    fn drafted(&mut self, x: f32, y: f32) {
        self.keep_copy(x, y);
        if self.scene.is_none() {
            self.rigs.auto.edit(Instant::now());
        }
    }

    fn keep_copy(&mut self, x: f32, y: f32) {
        let (base, now) = (&self.stage.base, &self.stage.bundle);
        match &self.draft {
            rigs::Copy::Outbox => self.rigs.keep(base, now),
            rigs::Copy::File(f) => {
                if let Err(e) = wires::save_draft(f, base, now, self.rigs.auto.version) {
                    self.say(format!("{e:#}"), x, y, true);
                }
            }
            rigs::Copy::Nothing => {}
        }
    }

    /// the scene leaves Build: what it has not sent waits in Show's `unsent`
    pub(crate) fn drop_scene(&mut self) {
        if let Some(v) = self.scene.take() {
            self.show.keep_unsent(v.edit.doc, v.auto);
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
        self.menu = Some(Menu { ta: typeahead::Typeahead::new(items, x, y), from, far: None, rigs: false, add: Vec::new(), far_new: None, scene: None });
    }

    /// #477: the typeahead over the toon's address book for a far world node's address – the kinds a holder may be, found
    /// by the book's own rule (deck-space rig/book.rs); a pick makes the entry the address's holder
    fn open_book(&mut self, node: usize, x: f32, y: f32) {
        let Some(far) = self.stage.far(node) else { return };
        let ta = self.holders(x, y);
        self.menu = Some(Menu { ta, from: None, far: Some(far), rigs: false, add: Vec::new(), far_new: None, scene: None });
    }

    /// the address book's holders a far world node may listen to; without a book it says why inside
    fn holders(&mut self, x: f32, y: f32) -> typeahead::Typeahead {
        self.book.poll();
        let why = self.book_why();
        let Some(book) = self.book.book.clone() else {
            return typeahead::Typeahead::new(Vec::new(), x, y).with_note(why.unwrap_or_default());
        };
        let kinds = deck_space::world::FAR_HOLDERS;
        let row = |i: usize| book.row(i, LOCALE).map(|r| typeahead::Item { id: i, label: r.chain, detail: format!("{}{}", r.kind.as_str(), if r.draft { " · draft" } else { "" }) });
        let items = book.find("", &kinds, LOCALE).into_iter().filter_map(row).collect();
        let found = book.clone();
        let ta = typeahead::Typeahead::found_by(items, x, y, move |q| found.find(q, &kinds, LOCALE));
        match why {
            Some(w) => ta.with_note(w),
            None => ta,
        }
    }

    /// why the book is not fresh, in words (book.rs `Why`)
    fn book_why(&self) -> Option<String> {
        self.book.why.as_ref().map(|w| w.text(self.auth.code().as_deref(), std::time::SystemTime::now()))
    }

    /// the book follows the toon open now (toon.rs) and the sign-in: a new loader when either changed
    fn follow_toon(&mut self) {
        if self.args.book.is_some() {
            return;
        }
        self.rig_toon.poll();
        let build = self.workspace == show::Workspace::Build;
        let toon = toon::wanted(self.args.toon.as_deref(), build, self.rig_toon.toon.as_deref(), self.show.show_toon());
        let token = self.auth.token();
        let key = (toon, token.is_some());
        if key != self.book_key {
            self.book = book::Loader::new(&self.args.api, key.0.as_deref(), None, token, &cache_dir());
            self.book_key = key;
        }
    }

    /// Build's rig on the server (rigs.rs): what its threads answered goes onto the stage and into the notice
    fn follow_rigs(&mut self) {
        let (x, y) = self.cursor;
        for e in self.rigs.poll(self.auth.token().is_some()) {
            match e {
                rigs::Event::Opened(subject, opened) => {
                    let (base, plays) = *opened;
                    self.drop_scene();
                    self.stage.path.clear();
                    if let Err(err) = self.stage.replay(plays) {
                        self.say(format!("{err:#}"), x, y, true);
                        continue;
                    }
                    self.stage.base = base;
                    (self.undo, self.sel, self.armed, self.draft) = (Vec::new(), inspector::Sel::Rig, None, rigs::Copy::Outbox);
                    if self.args.toon.is_none() && self.args.book.is_none() {
                        self.rig_toon = toon::RigToon::new(&self.args.api, &subject, &cache_dir());
                    }
                }
                rigs::Event::Saved(sent) => {
                    self.stage.base = *sent;
                    match &self.draft {
                        rigs::Copy::Outbox => self.rigs.saved(&self.stage.base, &self.stage.bundle),
                        _ => self.keep_copy(x, y),
                    }
                }
                rigs::Event::Say(text, bad) => self.say(text, x, y, bad),
            }
        }
    }

    /// #568: what Show asked Build to play (show/build.rs): a rig at hand takes the stage, the server's is opened
    fn follow_cell(&mut self) {
        if self.show.to_build.is_some() {
            self.leave_view();
        }
        let Some(asked) = self.show.to_build.take() else { return };
        let (x, y) = self.cursor;
        self.workspace = show::Workspace::Build;
        if let show::build::ToBuild::Scene(o) = asked {
            return self.open_scene(*o);
        }
        self.drop_scene();
        match asked {
            show::build::ToBuild::Scene(_) => {}
            show::build::ToBuild::Play(o, set) => {
                let subject = o.bundle.rig.subject.clone();
                self.stage.path.clear();
                if let Err(err) = self.stage.replay(o.bundle.clone()) {
                    return self.say(format!("{err:#}"), x, y, true);
                }
                self.stage.base = o.bundle;
                for (path, v) in set {
                    if let Some(k) = self.stage.knobs.iter().find(|k| k.path == path) {
                        let (node, port, min, max) = (k.node, k.port_index, k.min, k.max);
                        self.stage.set(node, port, v, min, max);
                    }
                }
                (self.rigs.subject, self.rigs.auto.version) = (subject.clone(), None);
                (self.undo, self.sel, self.armed) = (Vec::new(), inspector::Sel::Rig, None);
                self.draft = rigs::Copy::Outbox;
                self.leveled = !self.wire;
                let line = self.show.status.clone();
                self.say(line, x, y, false);
            }
            show::build::ToBuild::Server(subject) => {
                if let Err(e) = self.rigs.open(&subject) {
                    self.say(format!("{subject}: {e}"), x, y, true);
                }
            }
        }
    }

    /// Show's scene sheet: the toon's rigs to cast from, a scene it asks the stage to play; back in Build the rig plays again
    fn follow_scene(&mut self) {
        if self.show.compose.catalog.is_none() {
            self.show.compose.catalog = self.rigs.catalog().map(|c| std::sync::Arc::new(c.clone()));
        }
        let (x, y) = self.cursor;
        if let Some(scene) = self.show.to_stage.take() {
            if let Some(v) = self.scene.as_mut() {
                v.shown = false;
            }
            let id = self.show.compose.id.clone();
            if let Err(err) = self.stage.play_scene(&id, scene) {
                self.say(format!("scene:{id}: {err:#}"), x, y, true);
            }
        }
        if self.workspace == show::Workspace::Build && self.stage.scene.is_some() && self.scene.is_none() {
            if let Err(err) = self.stage.replay(self.stage.bundle.clone()) {
                self.say(format!("{err:#}"), x, y, true);
            }
            self.stage.scene = None;
        }
        if self.workspace == show::Workspace::Build && self.scene.as_ref().is_some_and(|v| v.inside.is_none() && !v.shown) {
            self.replay_scene();
        }
    }

    /// #568: "values → cell" – the knobs as the stage has them, into the linked cell (show/build.rs)
    fn knobs_to_cell(&mut self) {
        let positions: Vec<(String, f32)> = self.stage.knobs.iter().map(|k| (k.path.clone(), self.stage.value(k))).collect();
        let done = self.show.values_to_cell(&positions);
        let (x, y) = self.cursor;
        let line = self.show.status.clone();
        self.say(line, x, y, !done);
    }

    /// the rig picker: the toon's rigs on the server, the local copy when it differs; why there are none inside it
    fn open_rigs(&mut self, x: f32, y: f32) {
        let registry = std::fs::read_to_string(toon::kept(&cache_dir())).unwrap_or_default();
        let (items, note) = self.rigs.items(self.book_key.0.as_deref(), &registry, &self.auth.line(&deck_link::pairing::app_url()));
        let mut ta = typeahead::Typeahead::new(items, x, y);
        if let Some(n) = note {
            ta = ta.with_note(n);
        }
        self.menu = Some(Menu { ta, from: None, far: None, rigs: true, add: Vec::new(), far_new: None, scene: None });
    }

    /// a row chosen: connected from the armed port, or the port found is armed
    fn pick(&mut self, id: usize) {
        let Some(menu) = self.menu.take() else { return };
        let (x, y) = (menu.ta.x, menu.ta.y);
        if let Some(p) = menu.scene {
            return self.scene_pick(p, id, x, y);
        }
        if menu.rigs {
            match self.rigs.choices.get(id).cloned() {
                Some(rigs::Choice::Server(subject)) => {
                    self.leave_view();
                    if let Err(e) = self.rigs.open(&subject) {
                        self.say(e, x, y, true);
                    }
                }
                Some(rigs::Choice::Local) => {
                    let local = self.rigs.take_local(&self.stage.base).ok_or_else(|| "no local copy".to_string());
                    self.change(local, "the local copy, saved on the server's version".into(), x, y);
                }
                None => {}
            }
            return;
        }
        if let Some(c) = menu.add.get(id).cloned() {
            return self.added(c, x, y);
        }
        if let Some(track) = menu.far_new {
            return self.far_added(&track, id, x, y);
        }
        if let Some((track, node, address)) = menu.far {
            // the entry's address (the book offers only entries a receiver exists for: rig/book.rs `find`)
            let Some(entry) = self.book.book.as_ref().and_then(|b| b.entry(id)).and_then(|e| e.address.clone()) else { return };
            let next = deck_space::rig::address::Place::parse(&entry).map(|p| address.with_holder(p).to_string());
            let done = next.and_then(|a| self.stage.bundle.address_set(&track, &node, &a));
            return self.change(done, format!("{node}: {entry}"), x, y);
        }
        let Some(a) = menu.from else { return self.arm(id) };
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
        if self.colour.dragging() {
            let d = self.colour.release();
            return self.colour_do(d);
        }
        if self.panels_mut().floats.release() {
            self.panels_moved = true;
            return;
        }
        // a library card lands where the workspace takes it
        if let Some(did) = self.library.browser.release(self.cursor.0, self.cursor.1) {
            return self.library_did(did);
        }
        if self.grid.scrub_end() {
            return;
        }
        let a = match self.panel.drag.take() {
            Some(Drag::Member) => return self.grid_drop(),
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
            Some(Drag::SceneKnob(..)) => return self.scene_settled(),
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
        self.drafted(x, y);
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
        self.script_colour();
        self.script_library();
        self.show.script(self.frames, &self.args.script);
        if self.frames == 70 && self.args.script.to_cell {
            self.knobs_to_cell();
        }
        self.script_scene();
        const START: u64 = 60;
        if self.args.pull {
            let s = self.scale();
            let values = |app: &App| app.stage.knobs.iter().map(|k| app.stage.value(k)).collect::<Vec<f32>>();
            match self.frames {
                f if f == START => {
                    let b = self.stage.space.being.buttons.iter().find(|b| !b.hidden && b.turnable).map(|b| b.pos);
                    if let Some((px, py)) = b.and_then(|p| self.stage.space.to_pixel(p)) {
                        let (x, y) = (px / s, py / s);
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
        if self.args.add.is_some() {
            return self.script_add();
        }
        if let Some((query, pick)) = self.args.far.clone() {
            match self.frames {
                f if f == START => match self.sel {
                    inspector::Sel::Node(n) if self.stage.far(n).is_some() => {
                        let at = self.panel.hits.iter().find(|(_, h)| *h == Hit::Address(n)).map(|(r, _)| (r.x, r.y + r.h));
                        let (x, y) = at.unwrap_or(self.cursor);
                        self.open_book(n, x, y);
                    }
                    _ => tracing::error!("--far: --select a far world node first"),
                },
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

    /// The panic pill: the show core's panic or resume, and this stage with it.
    fn panic_pill(&mut self) {
        self.show.panic_pill();
        if self.show.sub.panicked() {
            self.stage.panic();
        } else if let Err(err) = self.stage.resume() {
            let (x, y) = self.cursor;
            self.say(format!("{err:#}"), x, y, true);
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
        self.follow_toon();
        self.follow_rigs();
        self.follow_modules();
        self.autosave();
        if self.workspace == show::Workspace::Module {
            self.module.enter();
        }
        self.module.poll(&mut self.stage);
        if let (Some(link), Some(cb)) = (self.module.link.take(), self.clipboard.as_mut()) {
            let _ = cb.set_text(link);
        }
        self.follow_cell();
        self.follow_scene();
        if let Some(f) = self.show.demo_book.take() {
            self.book = book::Loader::new(&self.args.api, None, Some(&f), None, &cache_dir());
            self.args.book = Some(f);
        }
        self.book.poll();
        // #537: the Show workspace picks a cell's piece from the book's scenes
        self.show.book.clone_from(&self.book.book);
        self.show.book_why = self.book_why();
        self.follow_library();
        // the main menu's Quit to stage: as F9
        if self.exit == 11 {
            el.exit();
            return;
        }
        if std::mem::take(&mut self.panels_moved) {
            self.remember_panels();
        }
        if self.frames == 0 {
            self.script_save_state();
        }
        if self.frames == 0 && self.args.panic {
            self.panic_pill();
        }
        let help = self.help_now();
        self.show.help.clone_from(&help);
        let enabled: Vec<(bar::Action, bool)> = bar::MENU.iter().map(|(a, ..)| (*a, self.menu_why_not(*a).is_none())).collect();
        let grab = self.panels().floats.dragging() || matches!(self.panels().floats.chrome_at(self.cursor.0, self.cursor.1), Some((_, deck_kit::editor::floats::Chrome::Handle)));
        let pill = self.save_pill();
        let account = self.auth.state();
        let fps = self.fps();
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
        // the stage fills the window; the toolbar and the panels float over it
        let (stage_x, top, stage_w, stage_h) = (0.0, 0.0, wf, hf);
        let bar_bottom = bar::bottom(t);
        let open = Some((t.size.rem(22.0), settings::height(t)));
        for (p, ws) in [(&mut self.build, show::Workspace::Build), (&mut self.module.panels, show::Workspace::Module), (&mut self.show.show_panels, show::Workspace::Show), (&mut self.show.perform_panels, show::Workspace::Perform)] {
            p.settings = open;
            p.tools = Some(tools::size(t, ws));
        }
        let work_start = Instant::now();
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        if let Some(g) = self.gpu_time.as_mut() {
            g.begin(&mut enc);
        }
        // the stage at its viewport's size in pixels
        let [r, g, b, _] = t.color.stage;
        self.stage.space.set_ground([r, g, b]);
        let show_bg = matches!(self.workspace, show::Workspace::Show | show::Workspace::Perform).then(|| self.show.sub.conductor().show().background.clone()).flatten();
        self.stage.wear(show_bg.as_ref());
        self.stage.ask_pixel(self.colour.dropper.map(|_| ((self.cursor.0 * scale) as u32, (self.cursor.1 * scale) as u32)));
        let (stage_view, fresh) = self.stage.frame(&gpu.device, &gpu.queue, &mut enc, ((stage_w * scale) as u32, (stage_h * scale) as u32), dt);
        if fresh {
            ui.set_stage(&gpu.device, stage_view);
        }
        // the UI: everything in logical pixels, the pass scales
        let t0 = Instant::now();
        ui.begin();
        ui.stage(stage_x, top, stage_w, stage_h);
        self.show.frame();
        self.panel.hits.clear();
        self.marks.clear();
        let (pad, font, row) = (t.size.pad, t.size.font, t.size.row);
        // the toolbar: the path of what is open and a word of status (Build: the numbers of the gates, a broken tokens
        // file first: the pill cuts from the end)
        let build = self.workspace == show::Workspace::Build;
        let (path, status) = if build {
            let glitches = self.stage.glitches();
            let status = format!(
                "{}{}",
                self.tokens.problem.as_ref().map(|p| format!("{p} – ")).unwrap_or_default(),
                if glitches > 0 { format!("{glitches} glitches") } else { String::new() },
            );
            let status = if self.undo.is_empty() { status } else { format!("{} change{}{}{status}", self.undo.len(), if self.undo.len() == 1 { "" } else { "s" }, if status.is_empty() { "" } else { " · " }) };
            let open = self.scene.as_ref().map_or_else(|| self.stage.bundle.rig.subject.clone(), |v| v.crumb());
            let path: Vec<String> = std::iter::once(open).chain(self.stage.path.iter().cloned()).collect();
            let join = |a: String| if status.is_empty() { a } else { format!("{a} · {status}") };
            let status = match (self.show.build_line(), &self.scene) {
                (Some(l), _) => join(l),
                (None, Some(v)) => join(v.title()),
                (None, None) => status.clone(),
            };
            (path, status)
        } else if self.workspace == show::Workspace::Module {
            (vec![self.module.open.as_ref().map_or_else(|| "module".to_string(), module::Open::key)], self.stage.audio.clone())
        } else if self.workspace == show::Workspace::Show {
            let row = self.show.shown().rows.get(self.show.row).map(|r| format!("{} {}", self.show.row + 1, r.id)).unwrap_or_default();
            (self.show.crumbs().into_iter().chain(std::iter::once(row).filter(|r| !r.is_empty())).collect(), self.show.link_line())
        } else {
            (vec![self.show.sub.conductor().show().id.clone()], self.show.link_line())
        };
        let status = match pill {
            Some(p) if status.is_empty() => p,
            Some(p) => format!("{p} · {status}"),
            None => status,
        };
        let names = show::WORKSPACES.map(|(_, n)| n);
        // reduce motion stills the animated logo
        let logo_frame = if self.settings.motion.allows(true) { ((self.started.elapsed().as_secs_f32() / logo::SECONDS).fract() * logo::FRAMES as f32) as usize } else { 0 };
        let slot = if matches!(self.workspace, show::Workspace::Build | show::Workspace::Module) { 0.0 } else { show::author::view_width(ui, t, &self.show, self.workspace) };
        // the signing-in icon breathes, unless motion is reduced
        let pulse = if self.settings.motion.allows(true) { 0.5 + 0.5 * (self.started.elapsed().as_secs_f32() * std::f32::consts::TAU / 1.6).cos() } else { 1.0 };
        let spec = bar::Bar {
            workspaces: &names,
            workspace: self.workspace.index(),
            path: &path,
            status: &status,
            play: build.then_some(self.wire),
            beat: self.stage.beat,
            clock: bar::Clock::Int,
            account,
            pulse,
            logo_frame,
            pointer: self.cursor,
            cpu: self.cpu_load,
            gpu: self.gpu_time.as_ref().map(|_| self.gpu_load),
            khz: self.stage.khz().map(|k| k as f32),
            fps,
            slot,
            night: self.tokens.look.theme == tokens::Theme::Night,
            panicked: self.show.sub.panicked(),
        };
        let placed = bar::draw(ui, t, wf, &spec);
        self.panel.hits.push((placed.menu, Hit::Menu));
        self.panel.hits.push((placed.panic, Hit::Panic));
        self.panel.hits.push((placed.day, Hit::Look));
        self.panel.hits.push((placed.account, Hit::Account));
        self.panel.hits.push((placed.tempo, Hit::Tempo));
        if let Some(r) = placed.meters {
            self.panel.hits.push((r, Hit::Meters));
        }
        for (r, (w, _)) in placed.workspaces.iter().zip(show::WORKSPACES) {
            self.panel.hits.push((*r, Hit::Workspace(w)));
        }
        if let Some(r) = placed.play {
            self.panel.hits.push((r, Hit::Mode(!self.wire)));
        }
        // a step of the path goes back to it (Show's last one is the row: it goes nowhere)
        for (i, r) in placed.path.iter().enumerate() {
            if build || i + 1 < path.len() {
                self.panel.hits.push((*r, Hit::Crumb(i)));
            }
        }
        self.show.view_slot = placed.slot;
        'build: {
        if !build {
            self.show.hits.clear();
            match self.workspace {
                show::Workspace::Show => show::author::draw(ui, t, &mut self.show, wf, hf, self.cursor),
                show::Workspace::Module => module::draw::draw(ui, t, &mut self.module, &self.stage, (wf, hf), self.cursor, help.as_ref()),
                _ => show::perform::draw(ui, t, &mut self.show, wf, hf, self.cursor),
            }
            break 'build;
        }
        self.grid.follow(self.scene.as_ref(), &self.stage, self.args.grid.as_deref());
        self.grid.draw_stage(ui, t, &self.stage.space, Rect { x: stage_x, y: top, w: stage_w, h: stage_h }, scale);
        // Build's panels and what each asks for this frame; the view gizmo is a place no panel covers
        let gizmo = Rect { x: wf * 0.5 - 40.0, y: bar_bottom + 16.0, w: 80.0, h: 80.0 };
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
        let (mixer_w, mixer_h) = mixer::size(t, &self.stage, &view);
        let info_h = t.size.small * 1.4 * 6.0;
        let shown = [("tree", t.size.tree, None), (library::PANEL, library::WIDTH, None), ("inspector", t.size.inspector, None), ("audio", 320.0, None), ("info", 260.0, Some(info_h)), ("mixer", mixer_w, Some(mixer_h))];
        self.build.layout(ui, t, ((wf, hf), bar_bottom), &[gizmo], &shown);
        let covered = |floats: &deck_kit::editor::floats::Floats, x: f32, y: f32| y < bar_bottom || floats.covers(x, y);
        // the Wire side: the ports around the being's points, the core's list on this frame's places (not under a panel)
        if self.wire {
            let being = &self.stage.space.being;
            self.bodies.clear();
            if self.stage.path.is_empty() {
                for track in &self.stage.bundle.rig.tracks {
                    let node = self.stage.ports.iter().filter(|p| p.track == track.id).find_map(|p| self.stage.node_ids.iter().position(|n| *n == p.node).filter(|n| being.point_world(*n).is_some()));
                    let place = node.and_then(|n| being.point_world(n)).and_then(|w| self.stage.space.to_pixel(w));
                    if let Some((sx, sy)) = place {
                        let (bx, by) = (sx / scale + stage_x, sy / scale + top);
                        if covered(&self.build.floats, bx, by) {
                            continue;
                        }
                        self.bodies.push((bx, by, track.id.clone(), inspector::Sel::Track(track.id.clone())));
                        ui.text(bx - 16.0, by + 18.0, font, &track.id, t.color.bright);
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
                        if covered(&self.build.floats, bx, by) {
                            continue;
                        }
                        let name = p.address.split_once(':').and_then(|(_, r)| r.split_once('.')).map_or("", |(n, _)| n).to_string();
                        ui.text(bx - 16.0, by + 18.0, font, &name, t.color.bright);
                        self.bodies.push((bx, by, name, inspector::Sel::Node(n)));
                    }
                }
            }
            for (index, q) in self.stage.ports.iter().enumerate().filter(|(i, _)| self.stage.visible[*i]) {
                let Some(node) = self.stage.node_ids.iter().position(|n| *n == q.node) else { continue };
                let Some(world) = being.port_world(node, &q.name, q.input) else { continue };
                let Some((sx, sy)) = self.stage.space.to_pixel(world) else { continue };
                let (mx, my) = (sx / scale + stage_x, sy / scale + top);
                if mx > stage_x + stage_w || mx < stage_x || my < top || my > top + stage_h || covered(&self.build.floats, mx, my) {
                    continue;
                }
                self.marks.push(Mark { port: index, x: mx, y: my });
                let near = (self.cursor.0 - mx).hypot(self.cursor.1 - my) < 110.0;
                let hover = (self.cursor.0 - mx).hypot(self.cursor.1 - my) < 9.0;
                let c = if q.input { t.color.bright } else { t.color.accent };
                let dragging = self.menu.is_some() || self.armed.is_some() || matches!(self.panel.drag, Some(Drag::Cable(_) | Drag::Grab(_)));
                // what the typeahead still shows, else what the cable in the hand may reach
                let open = match &self.menu {
                    Some(m) if m.far.is_none() => m.ta.shows(index),
                    Some(_) => false,
                    None => self.reach.get(index).copied().unwrap_or(false),
                };
                if self.armed == Some(index) {
                    ui.ring(mx, my, 13.0, t.color.bright);
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
                    ui.text(lx, my - small * 0.6, small, &q.name, t.color.bright);
                }
            }
        }
        // #568: every far world node with a tube off the stage's right edge, towards its holder: the outlet of the linked
        // cell's row it plays on (deck-show far.rs), else its address
        {
            let linked = self.show.build_wires(&self.stage.reach);
            let being = &self.stage.space.being;
            let mut k = 0.0;
            for (n, id) in self.stage.node_ids.iter().enumerate() {
                let Some((_, node, address)) = self.stage.far(n) else { continue };
                let track = id.strip_suffix(&format!("_{node}")).unwrap_or(id);
                let on = being.point_world(n).or_else(|| self.stage.ports.iter().filter(|p| p.track == track).find_map(|p| self.stage.node_ids.iter().position(|x| *x == p.node).and_then(|m| being.point_world(m))));
                let Some((sx, sy)) = on.and_then(|w| self.stage.space.to_pixel(w)) else { continue };
                let from = (sx / scale + stage_x, sy / scale + top);
                let to = (stage_x + stage_w - 12.0, top + stage_h * 0.35 + k * row * 1.6);
                k += 1.0;
                let (words, held) = match linked.iter().find(|l| l.node == *id) {
                    Some(l) => (l.words(), matches!(l.holder, deck_show::far::Holder::Outlet(_) | deck_show::far::Holder::Here)),
                    None => (format!("{id} ← off stage · {address}"), false),
                };
                let c = if held { t.color.accent } else { t.color.muted };
                ui.cable(from, to, c);
                ui.disc(to.0, to.1, 5.0, c);
                let w = ui.advance(font * 0.85) * words.chars().count() as f32 + 12.0;
                controls::chip(ui, t, ((to.0 - w).max(stage_x + pad), to.1 + 8.0), stage_x + stage_w, &words, false);
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
                    if !covered(&self.build.floats, bx, by) {
                        ui.ring(bx, by, 20.0, t.color.bright);
                    }
                }
            }
        }
        // the view gizmo in its place: its ball orbits, an axis looks along itself
        {
            const R: f32 = 24.0;
            let (gx, gy) = (gizmo.x + gizmo.w * 0.5, gizmo.y + gizmo.h * 0.5);
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
        // the rig's tree: the rig, its tracks, the nodes of the one entered
        if let Some(o) = self.build.open(ui, t, "tree", self.cursor, self.panel.hits.len()) {
            // a scene's lines (scene.rs), then the rig, its tracks, the nodes of the one entered
            let mut rows = self.scene.as_mut().map(|v| v.tree_rows()).unwrap_or_default();
            let at_scene = self.scene.as_ref().is_some_and(|v| v.inside.is_none());
            let down = if self.scene.is_some() { 2 } else { 0 };
            rows.push(controls::TreeRow { depth: down, label: "rig".into(), detail: String::new(), icon: "nodes", open: Some(true), selected: self.sel == inspector::Sel::Rig });
            self.rows.clear();
            self.rows.push((Vec::new(), inspector::Sel::Rig));
            if at_scene {
                rows.pop();
                self.rows.clear();
            }
            for track in self.stage.bundle.rig.tracks.iter().filter(|_| !at_scene) {
                let inside = self.stage.path.first() == Some(&track.id);
                rows.push(controls::TreeRow { depth: down + 1, label: track.id.clone(), detail: if self.stage.silent.contains_key(&track.id) { format!("{} · silent", track.instrument) } else { track.instrument.clone() }, icon: "folder", open: Some(inside), selected: self.sel == inspector::Sel::Track(track.id.clone()) });
                self.rows.push((vec![track.id.clone()], inspector::Sel::Track(track.id.clone())));
                if inside {
                    let mut seen: Vec<&str> = Vec::new();
                    for p in self.stage.ports.iter().filter(|p| p.track == track.id) {
                        let name = p.address.split_once(':').and_then(|(_, r)| r.split_once('.')).map_or("", |(n, _)| n);
                        if !seen.contains(&name) {
                            seen.push(name);
                            let node = self.stage.node_ids.iter().position(|id| *id == p.node).unwrap_or(0);
                            rows.push(controls::TreeRow { depth: down + 2, label: name.to_string(), detail: String::new(), icon: "sliders", open: None, selected: self.sel == inspector::Sel::Node(node) });
                            self.rows.push((vec![track.id.clone()], inspector::Sel::Node(node)));
                        }
                    }
                }
            }
            for (i, r) in controls::tree(ui, t, o.area.x, o.area.y, o.area.w, &rows, self.cursor).into_iter().enumerate() {
                self.panel.hits.push((r, Hit::Row(i)));
            }
            self.build.close(ui, t, o, &mut self.panel.hits);
        }
        // the inspector: the rig open, its version and what the last save came to; the picker and Save (rigs.rs); the tabs
        if let Some(o) = self.build.open(ui, t, "inspector", self.cursor, self.panel.hits.len()) {
            let a = o.area;
            let mut y = a.y;
            let save = ui.advance(t.size.small) * 7.0 + 4.0 * pad;
            let pick = Rect { x: a.x, y, w: a.w - pad - save, h: row - 4.0 };
            ui.round_rect(pick.x, pick.y, pick.w, pick.h, t.size.radius, t.color.control);
            let small = font * 0.85;
            let line = self.scene.as_ref().map_or_else(|| self.rigs.line(), |v| v.line());
            let line = controls::fit(&line, ((pick.w - 12.0) / ui.advance(small)) as usize);
            ui.text(pick.x + 6.0, pick.y + (pick.h - small * 1.25) * 0.5, small, &line, t.color.text);
            self.panel.hits.push((pick, Hit::RigPick));
            for (r, h) in controls::pills(ui, t, pick.x + pick.w + pad, y, row - 4.0, &["Add"], usize::MAX).into_iter().zip([Hit::Add]) {
                self.panel.hits.push((r, h));
            }
            y += row + pad / 2.0;
            if self.show.build.is_none() && self.scene.is_some() {
                let w = ui.advance(small) * show::build::BACK.chars().count() as f32 + 3.0 * pad;
                for (r, h) in controls::pills(ui, t, a.x + a.w - w + pad, y, row - 4.0, &[show::build::BACK], usize::MAX).into_iter().zip([Hit::BackToShow]) {
                    self.panel.hits.push((r, h));
                }
                y += row + pad / 2.0;
            }
            if let Some(line) = self.show.build_line() {
                let pills = [show::build::TO_CELL, show::build::BACK];
                let pw: f32 = pills.iter().map(|p| ui.advance(small) * p.chars().count() as f32 + 2.0 * pad + pad).sum();
                let fitted = controls::fit(&line, ((a.w - pad - pw) / ui.advance(small)).max(4.0) as usize);
                ui.text(a.x, y + (row - 4.0 - small * 1.25) * 0.5, small, &fitted, t.color.accent);
                for (r, h) in controls::pills(ui, t, a.x + a.w - pw + pad, y, row - 4.0, &pills, usize::MAX).into_iter().zip([Hit::ToCell, Hit::BackToShow]) {
                    self.panel.hits.push((r, h));
                }
                y += row + pad / 2.0;
            }
            let names = TABS.map(|(_, name)| name);
            let active = TABS.iter().position(|(tab, _)| *tab == self.panel.tab).unwrap_or(0);
            for (r, (tab, _)) in controls::pills(ui, t, a.x, y, row - 4.0, &names, active).into_iter().zip(TABS) {
                self.panel.hits.push((r, Hit::Tab(tab)));
            }
            y += row + pad / 2.0;
            match self.panel.tab {
                Tab::Rig => {
                    let area = Rect { x: a.x, y, w: a.w, h: a.y + a.h - y };
                    let hot = match self.panel.drag {
                        Some(Drag::Knob(i, ..)) => Some(inspector::Field::Knob(i)),
                        Some(Drag::Param(n, p, ..)) => Some(inspector::Field::Param(n, p)),
                        _ => None,
                    };
                    if let Some(view) = self.scene.as_ref().filter(|v| v.inside.is_none()) {
                        let hot = match self.panel.drag {
                            Some(Drag::SceneKnob(i, ..)) => Some(scene::Field::Knob(i)),
                            _ => None,
                        };
                        for (r, field) in scene::inspect(ui, t, area, view, hot) {
                            let hit = match field {
                                scene::Field::Knob(i) => Hit::SceneKnob(i),
                                scene::Field::Clear(i) => Hit::SceneClear(i),
                                scene::Field::Into => Hit::SceneInto,
                                f => Hit::SceneField(f),
                            };
                            self.panel.hits.push((r, hit));
                        }
                    } else if empty::is_empty(&self.stage.bundle) {
                        let mut end = area.y;
                        for (r, way) in empty::draw(ui, t, area, &self.stage.bundle) {
                            end = end.max(r.y + r.h);
                            self.panel.hits.push((r, Hit::Way(way)));
                        }
                    } else {
                        for (r, field) in inspector::draw(ui, t, area, &self.stage, &self.sel, hot) {
                            let hit = match field {
                                inspector::Field::Knob(i) => Hit::Knob(i),
                                inspector::Field::Param(n, p) => Hit::Param(n, p),
                                inspector::Field::Address(n) => Hit::Address(n),
                                inspector::Field::Remove => Hit::Remove,
                                inspector::Field::Background => Hit::Background,
                                inspector::Field::BackgroundColour => Hit::BackgroundColour,
                            };
                            self.panel.hits.push((r, hit));
                        }
                    }
                }
                Tab::Text => {
                    let cols = (a.w / ui.advance(font * 0.85)) as usize;
                    ui.text(a.x, y, font * 0.85, &controls::fit("umlauts, dead keys, IME · shift+arrows · ctrl+a/c/x/v", cols), t.color.muted);
                    y += row * 0.8;
                    let (fx, fw) = (a.x, a.w);
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
                    ui.text(a.x, y, font * 0.85, &format!("{} chars · cursor {} · selected {:?}", self.panel.field.text.chars().count(), cursor_col, self.panel.field.selected()), t.color.muted);
                }
            }
            self.build.close(ui, t, o, &mut self.panel.hits);
        }
        // the Audio panel: the settings, the map and the tones
        if let Some(o) = self.build.open(ui, t, "audio", self.cursor, self.panel.hits.len()) {
            for (r, f) in routing::draw(ui, t, o.area, &mut self.routing, &self.stage, self.cursor) {
                self.panel.hits.push((r, Hit::Routing(f)));
            }
            self.build.close(ui, t, o, &mut self.panel.hits);
        }
        if let Some(o) = self.build.open(ui, t, "info", self.cursor, self.panel.hits.len()) {
            info::body(ui, t, (o.area.x, o.area.y, o.area.w), help.as_ref());
            self.build.close(ui, t, o, &mut self.panel.hits);
        }
        if let Some(o) = self.build.open(ui, t, "mixer", self.cursor, self.panel.hits.len()) {
            for (r, part) in mixer::draw(ui, t, o.area, &self.stage, &view) {
                let hit = match part {
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
            self.build.close(ui, t, o, &mut self.panel.hits);
        }
        // a panel's ground keeps the press from the stage under it
        for r in self.build.grounds(&panels::BUILD.map(|s| s.id)) {
            self.panel.hits.push((r, Hit::Ground));
        }
        // over the panels: a cable in the hand, where it would land, a notice
        if self.wire {
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
                    controls::chip(ui, t, (self.cursor.0 + 14.0, self.cursor.1 + 14.0), wf, &format!("{} · {}", q.address, q.ty), false);
                }
            }
        }
        if self.notice.as_ref().is_some_and(|n| Instant::now() > n.until) {
            self.notice = None;
        }
        if let Some(n) = &self.notice {
            controls::chip(ui, t, (n.x + 14.0, n.y - 34.0), wf, &n.text, n.bad);
        }
        if let Some(c) = &self.carry {
            c.draw(ui, t, wf);
        }
        // over everything else: what is being typed, the typeahead
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
        self.grid.popover(ui, t, (wf, hf));
        if let Some(m) = self.menu.as_mut() {
            m.ta.draw(ui, t, (wf, hf));
        }
        self.panel.hits.push((Rect { x: stage_x, y: top, w: stage_w, h: stage_h }, Hit::Stage));
        }
        // the library over the workspace's panels
        {
            let cursor = self.cursor;
            match self.workspace {
                show::Workspace::Show => show::scenes::panel(ui, t, &mut self.show, &mut self.library, cursor),
                show::Workspace::Build | show::Workspace::Module => {
                    let panels = if build { &mut self.build } else { &mut self.module.panels };
                    match panels.open(ui, t, library::PANEL, cursor, 0) {
                        Some(o) => {
                            self.library.draw(ui, t, o.area, cursor, None);
                            panels.close(ui, t, o, &mut self.library.browser.hits);
                        }
                        None => self.library.browser.hits.clear(),
                    }
                }
                show::Workspace::Perform => self.library.browser.hits.clear(),
            }
        }
        // the tools pill, over the workspace's panels
        {
            let cursor = self.cursor;
            let ws = self.workspace;
            let state = tools::State { tool: self.tools.tool, two_d: self.show.two_d, grid: self.grid.settings().grid };
            let panels = match ws {
                show::Workspace::Build => &mut self.build,
                show::Workspace::Module => &mut self.module.panels,
                show::Workspace::Show => &mut self.show.show_panels,
                show::Workspace::Perform => &mut self.show.perform_panels,
            };
            let mut places = Vec::new();
            if let Some(o) = panels.open(ui, t, "tools", cursor, 0) {
                places = self.tools.draw(ui, t, o.area, ws, &state, cursor);
                panels.close(ui, t, o, &mut places);
            }
            self.tools.shown(places);
            self.grid.anchor(if build { self.tools.grid_at } else { None });
        }
        // the settings, over the workspace's panels
        self.settings_hits.clear();
        {
            let cursor = self.cursor;
            let s = self.settings;
            let ws = match self.workspace {
                show::Workspace::Build => &mut self.build,
                show::Workspace::Module => &mut self.module.panels,
                show::Workspace::Show => &mut self.show.show_panels,
                show::Workspace::Perform => &mut self.show.perform_panels,
            };
            if let Some(o) = ws.open(ui, t, panels::SETTINGS, cursor, 0) {
                let mut places = settings::draw(ui, t, o.area, &s);
                ws.close(ui, t, o, &mut places);
                self.settings_hits = places;
            }
        }
        self.tools.draw_menu(ui, t, (wf, hf), self.cursor);
        let value = self.colour.target().and_then(|target| colour::value(target, &self.stage, self.scene.as_ref(), &self.show));
        self.colour.draw(ui, t, (wf, hf), value, self.cursor);
        self.library.over(ui, t, wf);
        // the main menu over everything, the workspace's panels under its lines
        self.menu_lines.clear();
        if self.main_menu {
            let listed = match self.workspace {
                show::Workspace::Build => &self.build,
                show::Workspace::Module => &self.module.panels,
                show::Workspace::Show => &self.show.show_panels,
                show::Workspace::Perform => &self.show.perform_panels,
            }
            .listed();
            self.menu_lines = bar::menu(ui, t, placed.menu, &|a| enabled.iter().any(|(b, on)| *b == a && *on), &self.tokens.look, &listed, self.cursor);
        }
        window.set_cursor(if grab { winit::window::CursorIcon::Grab } else { winit::window::CursorIcon::Default });
        // the pass: logical pixels → the surface's, through the screen uniform
        let view = frame.texture.create_view(&Default::default());
        if std::mem::take(&mut self.library.pictures.changed) {
            ui.set_thumbs(&self.library.pictures.images);
        }
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
        gpu.queue.present(frame);
        let budget = self.frame_ms.last().copied().unwrap_or(16.7).max(1.0);
        let ease = |load: f32, now: f32| load + (now - load) * 0.08;
        self.cpu_load = ease(self.cpu_load, cpu_ms / budget);
        if let Some(g) = self.gpu_time.as_mut() {
            g.after_submit(&gpu.device);
            self.gpu_load = ease(self.gpu_load, g.last_ms / budget);
        }
        if self.colour.dropper.is_some() {
            self.colour.under = self.stage.read_pixel(&gpu.device);
        }
        if let Some(img) = self.stage.read_still(&gpu.device) {
            self.library.still(img);
        }
        if let (Some(path), Some((buf, stride))) = (shot, readback) {
            if let Err(e) = save(&gpu.device, &buf, stride, (w, h), gpu.format(), &path) {
                tracing::error!("screenshot: {e:#}");
            } else {
                tracing::info!("screenshot: {}", path.display());
            }
        }
        self.script();
        // a check run waits for the module's build and save before it counts on
        if self.frames == 0 || !self.module.busy() {
            self.frames += 1;
        }
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
        let window = match host::window(el, "SCUTOONS editor", !self.args.fullscreen) {
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
            WindowEvent::MouseInput { state: ElementState::Released, .. } => {
                self.show.release();
                self.module.release();
                self.release()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                // the rows band scrolls both ways (shift or a sideways wheel: across), a cell's knob in Show, then a panel
                // taller than its place scrolls
                let across = match delta {
                    MouseScrollDelta::LineDelta(x, _) => x,
                    MouseScrollDelta::PixelDelta(p) => p.x as f32 / 40.0,
                };
                let band = if self.mods.shift_key() { (steps, 0.0) } else { (across, steps) };
                if self.workspace != show::Workspace::Build && self.show.band_wheel(self.cursor.0, self.cursor.1, band) {
                    return;
                }
                if self.workspace != show::Workspace::Build && self.show.wheel(self.cursor.0, self.cursor.1, steps) {
                    return;
                }
                if let (show::Workspace::Module, Some(module::Hit::Param(i))) = (self.workspace, self.module.hit(self.cursor.0, self.cursor.1)) {
                    return self.module.wheel(i, steps, &mut self.stage);
                }
                let hit = self.hit(self.cursor.0, self.cursor.1);
                let (cx, cy) = self.cursor;
                if !matches!(hit, Hit::Knob(_) | Hit::Param(..)) && self.panels_mut().floats.wheel(cx, cy, steps) {
                    return;
                }
                match hit {
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
            // Shift+Esc panics from anywhere (Esc alone leaves the editor); only the pill resumes
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) && self.mods.shift_key() =>
            {
                if !self.show.sub.panicked() {
                    self.panic_pill();
                }
            }
            WindowEvent::KeyboardInput { event, .. } if self.edit.is_some() => self.edit_key(&event),
            WindowEvent::KeyboardInput { event, .. } if self.menu.is_some() => self.menu_key(&event),
            WindowEvent::KeyboardInput { event, .. } if self.library.browser.typing.is_some() => self.library_key(&event),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) && self.colour.taking() => {
                let d = self.colour.release();
                self.colour_do(d);
                self.colour.escape();
            }
            // ctrl+s: save now, in every workspace (everything saves itself anyway)
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && event.logical_key == Key::Character("s".into()) && self.mods.control_key() && !self.panel.field.focused && self.show.typing.is_none() =>
            {
                self.save_now();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && self.tools.menu_open() => {
                let mut chosen = None;
                if self.tools.key(&event.logical_key, &mut chosen) {
                    if let Some(p) = chosen {
                        self.tool_press(p);
                    }
                }
            }
            // #471: the flash list's typed filter has the keys while it is focused
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && matches!(self.workspace, show::Workspace::Show | show::Workspace::Perform) && self.show.key(&event.logical_key, self.mods.control_key() || self.mods.super_key(), self.workspace == show::Workspace::Show) => {}
            // #560: F9 – back to the stage (deck-session swaps; elsewhere F9 does nothing): exit code 11
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::F9) && std::env::var_os("DECK_SWAP").is_some() =>
            {
                self.exit = 11;
                el.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) && !self.panel.field.focused {
                    if self.wire && !self.stage.path.is_empty() {
                        self.up_to(self.stage.path.len() - 1);
                    } else if self.scene.as_ref().is_some_and(|v| v.inside.is_some()) {
                        self.scene_up();
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
                } else if let Some(step) = (event.state == ElementState::Pressed && self.mods.control_key()).then(|| size_key(&event.logical_key)).flatten() {
                    let theme = self.tokens.look.theme;
                    self.look(theme, step);
                } else if event.state == ElementState::Pressed && event.logical_key == Key::Character("z".into()) && self.mods.control_key() {
                    self.take_back();
                } else if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Delete) && self.workspace == show::Workspace::Build {
                    self.remove_selected();
                } else if let Some(tool) = (event.state == ElementState::Pressed && self.workspace == show::Workspace::Build && !self.mods.control_key()).then(|| if let Key::Character(c) = &event.logical_key { tools::tool_key(c) } else { None }).flatten() {
                    self.tools.tool = tool;
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
    device.poll(wgpu::PollType::wait_indefinitely()).context("waiting for the screenshot")?;
    rx.recv().context("the map's answer")?.context("mapping the screenshot")?;
    let data = slice.get_mapped_range().context("reading the screenshot")?;
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

/// ctrl + / − / 0: a step of the UI size, as a browser's zoom
fn size_key(key: &Key) -> Option<i32> {
    match key {
        Key::Character(c) if c == "+" || c == "=" => Some(1),
        Key::Character(c) if c == "-" => Some(-1),
        Key::Character(c) if c == "0" => Some(0),
        _ => None,
    }
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
