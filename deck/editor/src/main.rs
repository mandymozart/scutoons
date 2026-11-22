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
mod dop;
mod empty;
mod frame;
mod gpu_time;
mod grid;
mod history;
mod history_panel;
mod home;
mod idle;
mod library;
mod live_tabs;
mod log;
mod logo;
mod media;
mod media_draw;
mod media_tab;
mod mixer;
mod module;
mod inspector;
mod latency;
mod panels;
mod play;
mod posters;
mod published;
mod routing;
mod stage;
mod tokens;
mod ui;
mod rig_presets;
mod rigs;
mod saving;
mod scene;
mod scene_parts;
mod scene_prompt;
mod scene_stage;
mod settings;
mod show;
mod slots;
mod status;
mod sign_in;
mod tab_view;
mod tabs;
mod take;
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
    /// `--rig` was given: its tab opens (else the last tabs or Home; a check run without item flags starts on Home)
    rig_given: bool,
    /// the tabs a check run opens (`--open scene:nave,character:catpope`; `home`: none), the one in front (`--tab`, an
    /// address or an index), the most tabs at once
    tabs: Option<Vec<String>>,
    front: Option<String>,
    tab_cap: Option<usize>,
    /// a check run's tab list open at the start
    tab_list: bool,
    /// where the tabs are remembered: a check run's own file (`--tabs-file`, a warm start tried by hand), else none in a
    /// check run and the cache's otherwise
    tabs_file: Option<PathBuf>,
    /// a check run's inputs from outside (play.rs `Input::parse`), at this frame; its Stop at that frame
    inputs: Vec<String>,
    input_at: Option<u64>,
    stop_at: Option<u64>,
    /// a check run's solos and arm (`--solo TRACK`, `--arm TRACK`), pressed at frame 12
    monitor: Vec<(bool, String)>,
    /// a check run's press on Home's New (scene | show | module) at its start
    home_new: Option<String>,
    /// fullscreen on the current monitor (a deck's kiosk), else a 1280×720 window
    fullscreen: bool,
    /// the inspector's tab at the start: inspector | text; audio opens the Audio panel
    tab: Tab,
    audio: bool,
    frames: Option<u64>,
    screenshot: Option<PathBuf>,
    /// a check run types this on the toolbar's BPM (its screenshots)
    tempo_type: Option<String>,
    /// a check run turns the metronome on (its screenshots)
    click: bool,
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
    /// parameter groups opened on the level entered, comma-separated in turn (`o1/o2`; `SRC,SRC/OSC 2`: an area, then its page)
    open: Option<String>,
    /// the selected node's inspector opened on this page of its sections (`SRC/OSC 2`)
    page: Option<String>,
    /// a check run's pointer, logical pixels: what it hovers
    pointer: Option<(f32, f32)>,
    /// #609: the hand-off this editor came with (its scene goes back to the stage), a check run's F9 at frame N
    handoff: deck_native::handoff::Handoff,
    f9_at: Option<u64>,
    /// `--asset NODE[,play]`: the scene's video node opened in its Asset tab (its use), played at once with `,play`
    asset: Option<(String, bool)>,
    /// `--asset-edits JSON`: the open medium's asset edits as if the registry had answered them (deck-space asset_edits)
    asset_edits: Option<String>,
    /// `--units KIND=UNIT,…`: the unit each kind of value is read in (crop=px, focal=deg), as if chosen
    units: Vec<(String, deck_core::units::Unit)>,
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
    /// a check run's scene member (scene.rs `script_scene`): selected, a knob turned, opened as its rig if asked; or its
    /// parts joined by `+` (scene_parts.rs `script_parts`: slot:, video:, stack, layer:DEPTH, fill:N=PATH, background:, camera, orbit:DX,DY)
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
    /// a check run's Go live, pressed on each of these shows' tabs in turn (live_tabs.rs); then the lock
    go_live: Vec<String>,
    lock: bool,
    /// a check run shows the save pill in this state: saving, offline or conflict (its screenshots)
    save_state: Option<String>,
    /// a check run's tools pill folded at the start
    tools_folded: bool,
    /// a check run's panels closed in every workspace, and the main menu open
    closed: Vec<String>,
    /// the Log panel open in every workspace
    log: bool,
    /// the History panel open in Build and Show; a check run's ask on its step (`revert:N` | `back:N`, `:confirm` stores it)
    history: bool,
    history_ask: Option<history_panel::ScriptAsk>,
    main_menu: bool,
    /// a check run's colour picker on the shown background: open | pick | drop:X,Y | take:X,Y (colour.rs `script_colour`)
    colour: Option<String>,
    /// a check run's library: a card of this address dragged to X,Y and let go there (`hold`: kept in the hand)
    library_drag: Option<library::ScriptDrag>,
    /// a check run's library in the list mode, filtered by a query
    library_find: Option<String>,
    library_list: bool,
    /// a check run's Library shows the levels single
    library_single: bool,
    /// a check run's Library starts with these cards chosen (addresses)
    library_choose: Vec<String>,
    library_related: bool,
    /// a check run's view: panned by (dx, dy) pixels at the start, then looking through the scene's camera if asked
    pan: Option<(f32, f32)>,
    camera: bool,
    /// a check run's scene camera (dop.rs `script_dop`): shots, a track, a move previewed
    dop: Option<String>,
    /// a check run's scene document (JSON) opened in Build
    scene_file: Option<PathBuf>,
    /// a check run's rig preset steps (rig_presets.rs `script_presets`)
    rig_preset: Option<String>,
    /// a check run's node view: its direction, a zoom once it is framed
    nodes: Option<deck_kit::base::graph::Direction>,
    node_zoom: Option<f32>,
}

fn args() -> Result<Args> {
    let mut a = Args { rig_given: false, tabs: None, front: None, tab_cap: None, tab_list: false, tabs_file: None, inputs: Vec::new(), input_at: None, stop_at: None, monitor: Vec::new(), home_new: None, module: module::Script::default(), rig: "catpope".into(), fullscreen: false, frames: None, screenshot: None, tempo_type: None, click: false, tokens: None, volume: 0.5, size: None, wire: true, draft: None, gesture: None, view: None, enter: None, select: None, pull: false, menu: None, tab: Tab::Rig, audio: false, workspace: show::Workspace::Stage, show_file: None, show_id: None, relay: None, api: std::env::var("DECK_API").unwrap_or_else(|_| deck_native::cache::API.into()), script: show::Script::default(), toon: None, book: None, far: None, story: None, rigs: None, catalog: None, add: None, member: None, demo: None, grid: None, theme: None, ui_size: None, settings: false, panic: false, go_live: Vec::new(), lock: false, save_state: None, tools_folded: false, closed: Vec::new(), log: false, history: false, history_ask: None, main_menu: false, colour: None, library_drag: None, library_find: None, library_list: false, library_single: false, library_choose: Vec::new(), library_related: false, pan: None, camera: false, dop: None, scene_file: None, rig_preset: None, nodes: None, node_zoom: None, open: None, page: None, pointer: None, handoff: Default::default(), f9_at: None, asset: None, asset_edits: None, units: Vec::new() };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().with_context(|| format!("{k} needs a value"));
        match k.as_str() {
            "--rig" => {
                a.rig = val()?;
                a.rig_given = true;
            }
            "--tab-cap" => a.tab_cap = Some(val()?.parse()?),
            "--tab-list" => a.tab_list = true,
            "--tabs-file" => a.tabs_file = Some(PathBuf::from(val()?)),
            "--input" => a.inputs.push(val()?),
            "--input-at" => a.input_at = Some(val()?.parse()?),
            "--stop-at" => a.stop_at = Some(val()?.parse()?),
            "--solo" => a.monitor.push((false, val()?)),
            "--arm" => a.monitor.push((true, val()?)),
            "--home-new" => a.home_new = Some(val()?),
            "--frames" => a.frames = Some(val()?.parse()?),
            "--screenshot" => a.screenshot = Some(PathBuf::from(val()?)),
            "--tokens" => a.tokens = Some(PathBuf::from(val()?)),
            "--volume" => a.volume = val()?.parse()?,
            "--wire" => a.wire = true,
            "--fullscreen" => a.fullscreen = true,
            "--panic" => a.panic = true,
            "--tempo-type" => a.tempo_type = Some(val()?),
            "--click" => a.click = true,
            "--go-live" => a.go_live = val()?.split(',').map(str::to_string).collect(),
            "--lock" => a.lock = true,
            "--colour" => a.colour = Some(val()?),
            "--library-drag" => {
                let v = val()?;
                let p: Vec<&str> = v.split(',').collect();
                let hold = p.len() > 1 && p.last() == Some(&"hold");
                let to = library::DragTo::parse(&p[1..p.len() - usize::from(hold)])?;
                a.library_drag = Some((p[0].to_string(), to, hold));
            }
            "--library-find" => a.library_find = Some(val()?),
            "--library-list" => a.library_list = true,
            "--library-single" => a.library_single = true,
            "--library-choose" => a.library_choose = val()?.split(',').map(str::to_string).collect(),
            "--library-related" => a.library_related = true,
            "--save-state" => a.save_state = Some(val()?),
            "--tools-folded" => a.tools_folded = true,
            "--view" => a.view = Some(val()?.parse()?),
            "--select" => a.select = Some(val()?),
            "--pull" => {
                a.pull = true;
                a.wire = false;
            }
            "--play" => a.wire = false,
            // items to open as tabs, else (no address in it) parameter groups on the level entered
            "--open" => {
                let v = val()?;
                let items: Vec<String> = v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect();
                if !items.is_empty() && items.iter().all(|i| i == "home" || tabs::Kind::of(i).is_some()) {
                    a.tabs = Some(items);
                } else {
                    a.open = Some(v);
                }
            }
            "--page" => a.page = Some(val()?),
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
            // the inspector's tab, the Audio panel, else the editor's tab in front (an address or an index)
            "--tab" => match val()? {
                v if v.eq_ignore_ascii_case("audio") => a.audio = true,
                v => match TABS.iter().find(|(_, n)| n.eq_ignore_ascii_case(&v)) {
                    Some((t, _)) => a.tab = *t,
                    None => a.front = Some(v),
                },
            },
            "--workspace" => a.workspace = show::Workspace::parse(&val()?).context("--workspace scene | asset | module | show | home")?,
            "--show" => match val()? {
                v if v.ends_with(".json") => a.show_file = Some(PathBuf::from(v)),
                v => a.show_id = Some(v),
            },
            "--relay" => a.relay = Some(val()?),
            "--handoff" => a.handoff = deck_native::handoff::Handoff::take(std::path::Path::new(&val()?)),
            "--f9" => a.f9_at = Some(val()?.parse()?),
            "--asset" => {
                let v = val()?;
                a.asset = Some(v.strip_suffix(",play").map_or((v.clone(), false), |n| (n.to_string(), true)));
            }
            "--asset-edits" => a.asset_edits = Some(val()?),
            "--units" => {
                for pair in val()?.split(',') {
                    let (kind, id) = pair.split_once('=').context("--units KIND=UNIT,…")?;
                    a.units.push((kind.trim().to_string(), deck_core::units::Unit::of_id(id.trim()).context("--units: a unit id (s, frames, px, percent, deg, mm, db, gain …)")?));
                }
            }
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
            "--pan" => {
                let v = val()?;
                let (x, y) = v.split_once(',').context("--pan DX,DY")?;
                a.pan = Some((x.trim().parse()?, y.trim().parse()?));
            }
            "--camera" => a.camera = true,
            "--nodes" => a.nodes = Some(if val()? == "tb" { deck_kit::base::graph::Direction::Tb } else { deck_kit::base::graph::Direction::Lr }),
            "--node-zoom" => a.node_zoom = Some(val()?.parse()?),
            "--dop" => a.dop = Some(val()?),
            "--scene-file" => a.scene_file = Some(PathBuf::from(val()?)),
            "--rig-preset" => a.rig_preset = Some(val()?),
            "--log" => a.log = true,
            "--history" => a.history = true,
            "--history-ask" => {
                a.history = true;
                a.history_ask = Some(history_panel::ScriptAsk::parse(&val()?).context("--history-ask revert:N | back:N[:confirm]")?);
            }
            "--log-info" => {}
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
            "--camera-pick" => a.script.camera_pick = Some(val()?),
            "--cameras" => a.script.cameras = val()?.split(',').map(str::trim).filter(|n| !n.is_empty()).map(|c| c.split_once('=').map_or((c.to_string(), None), |(id, who)| (id.to_string(), Some(who.to_string())))).collect(),
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
            "--node-select" => a.script.node_select = val()?.split('+').map(String::from).collect(),
            "--node-frame" => a.script.node_frame = true,
            "--node-box" => {
                let v: Vec<f32> = val()?.split(',').map(|x| x.trim().parse()).collect::<Result<_, _>>()?;
                a.script.node_box = Some(v.try_into().map_err(|_| anyhow::anyhow!("--node-box X0,Y0,X1,Y1 (fractions of the canvas)"))?);
            }
            "--node-pin" => a.script.node_pin = Some(val()?.parse()?),
            "--node-wire" => a.script.node_wire = Some(val()?),
            "--band-drag" => {
                let v = val()?;
                let (edge, by) = v.rsplit_once(',').context("--band-drag EDGE,PIXELS (column:ID, row:ID or heads)")?;
                a.script.band_drag = Some((edge.to_string(), by.trim().parse()?));
            }
            "--known-shots" => a.script.known_shots = val()?.split_once('=').map(|(s, ids)| (s.to_string(), ids.split('+').map(String::from).collect())),
            "--wiring" => a.script.wiring = Some(val()?),
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
            "--pointer" => {
                let v = val()?;
                let (x, y) = v.split_once(',').with_context(|| format!("--pointer takes X,Y, not {v}"))?;
                a.pointer = Some((x.trim().parse()?, y.trim().parse()?));
            }
            "--size" => {
                let v = val()?;
                let (w, h) = v.split_once([',', 'x']).with_context(|| format!("--size takes W,H, not {v}"))?;
                a.size = Some((w.trim().parse()?, h.trim().parse()?));
            }
            "--help" | "-h" => {
                println!("deck-editor [--open ITEM,ITEM|home] [--tab ITEM|N] [--tab-cap N] [--tab-list] [--tabs-file FILE] [--input ADDRESS=V|ADDRESS>NAME[:K=V;…]|midi:B,B,B [--input-at N]] [--play [--stop-at N]] [--home-new scene[=NAME]|show|module] [--rig catpope] [--fullscreen] [--frames N] [--screenshot FILE] [--tempo-type TEXT] [--click] [--tokens FILE] [--size W,H] [--pointer X,Y] [--volume V] [--wire] [--draft FILE] [--gesture FROM,TO] [--view 0-5] [--menu PORT,QUERY[,pick]] [--toon TOON] [--book FILE] [--select NODE --far QUERY[,pick]] [--demo NAME] [--panic] [--colour open|pick|drop:X,Y|take:X,Y] [--save-state saving|offline|conflict] [--settings] [--close ID,ID] [--main-menu] [--module ID [--module-build] [--module-save] [--module-save-as ID]] [--scene-file FILE [--dop modules]] [--rig-preset save-as:NAME|dirty:NAME|member:NAME] [--handoff FILE] [--f9 N]");
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument {other}"),
        }
    }
    a.script.character.clone_from(&a.rig);
    if !a.inputs.is_empty() && a.input_at.is_none() {
        a.input_at = Some(40);
    }
    // #609: the hand-off wins over the flags – they are how the editor was started, it is where the stage left off
    if let Some(r) = a.handoff.relay.clone() {
        a.relay = Some(r);
    }
    match a.handoff.show.clone() {
        Some(v) if v.ends_with(".json") => (a.show_file, a.show_id) = (Some(PathBuf::from(v)), None),
        Some(v) => (a.show_file, a.show_id) = (None, Some(v)),
        None => {}
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
    /// a panel's ground: it keeps the press from the stage
    Ground,
    /// the main menu's hamburger (its lines: `menu_lines`)
    Menu,
    /// the toolbar's panic pill
    Panic,
    /// the toolbar's tempo pill, its account icon, its meters
    Tempo,
    /// inside the tempo pill: the BPM (a press types on it), the source's word (a press picks the next source)
    TempoBpm,
    TempoSource,
    /// the metronome's button in the tempo pill
    TempoClick,
    Account,
    Meters,
    Tab(Tab),
    Knob(usize),
    Field(f32),
    /// a parameter of a node in the inspector
    Param(usize, usize),
    /// a far world node's address in the inspector (#477)
    Address(usize),
    /// an area tab of a node's sections (node, area)
    Area(usize, usize),
    /// the mixer's level tab, a strip's fader, a strip's mute
    MixerTab(usize),
    GainText(mixer::Target),
    /// a mixer strip: selects what it is the level of; moving its fader does not
    StripSelect(mixer::Target),
    Fader(mixer::Target, controls::Fader),
    Mute(usize),
    Solo(usize),
    /// the selected track armed or disarmed
    Arm,
    Balance(usize),
    /// a highest-peak readout: a strip's, or the output's
    Peak(Option<mixer::Target>),
    /// the Edit / Play switch: a press flips it
    Switch,
    /// a port, by its index in the stage's list
    Port(usize),
    /// a node's parameter group on its ring, by its index in this frame's list
    Cluster(usize),
    /// a row of the rig's tree
    Row(usize),
    /// a step of the way down the tree, 0 = the rig (in Show: of the conductors' path)
    Crumb(usize),
    /// a track's body at the rig level
    Body(usize),
    /// the view gizmo: an axis (0 1 2 = +x +y +z, 3 4 5 = the minus sides), or its ball
    Axis(Option<usize>),
    /// a Show tab's live switch: Go live, End, Lock (live_tabs.rs)
    Live(live_tabs::Press),
    /// the toolbar's Home, a tab (its close), the tab list; a press on Home's page
    Home,
    TabFront(usize),
    TabClose(usize),
    TabList,
    HomePress(home::Press),
    /// Build's rig row: the rig picker
    RigPick,
    /// Build's Add, the inspector's Remove
    Add,
    Remove,
    Background,
    /// a background's swatch: the colour picker
    BackgroundColour,
    /// the rig's presets (rig_presets.rs): load one, save the working state as one, save the loaded one
    PresetLoad(usize),
    PresetSaveAs,
    PresetSave,
    /// Play: what it changed becomes the working state (not dropped on Stop)
    Keep,
    /// a scene member's rig preset (its index in the member's presets; `usize::MAX` = none)
    ScenePreset(usize),
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
    /// the DOP panel's field (dop.rs)
    Dop(dop::Field),
    /// a way to start the rig of a character without one (empty.rs)
    Way(empty::Way),
}

/// what is being dragged
#[derive(Clone, Copy, PartialEq)]
enum Drag {
    Knob(usize, f32, f32),
    /// a strip's fader: which, the hand on it
    Fader(mixer::Target, controls::FaderGrip),
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
    /// a number of the DOP panel: which, the pointer's start, the start's value
    Dop(dop::Num, f32, f32),
    /// a point of the trolley track on the stage (scene_stage.rs), or its handle
    TrackPoint(usize, bool),
    /// a shot's focus marker on the stage
    Focus(usize),
    /// a parallax layer's handle: which, the pointer's start, its depth then
    LayerDepth(usize, f32, f32),
}

/// a port on the screen this frame (logical pixels)
#[derive(Clone, Copy)]
struct Mark {
    port: usize,
    x: f32,
    y: f32,
}

impl Mark {
    fn distance(&self, (x, y): (f32, f32)) -> f32 {
        (self.x - x).hypot(self.y - y)
    }
}

/// what a typed value goes into
#[derive(Clone, Copy, PartialEq)]
enum Editing {
    Knob(usize),
    Param(usize, usize),
    Gain(mixer::Target),
    Background,
    SceneBackground,
    /// a new rig preset's name
    PresetName,
    Focal,
    Height,
    /// a layer's picture
    SceneLayer(usize),
    /// the Log panel's find
    LogFind,
    /// the master BPM on the toolbar's pill
    Tempo,
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
    cpu: Option<deck_native::cpu::CpuMeter>,
    gpu_busy: deck_core::load::Busy,
    gpu_load: deck_core::load::Meter,
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
    latency: latency::Probe,
    /// the window's own frame (Windows, windowed): the toolbar is its title bar
    own_frame: Option<deck_kit::editor::window::Side>,
    frame: frame::Own,
    exit: i32,
    /// #609: F9 or the main menu's Quit to stage – the next frame hands over
    to_deck: bool,
    clipboard: Option<arboard::Clipboard>,
    token_changes: u32,
    /// Edit (the Wire side: the ports show, a drag from one is a cable); off: Play (play.rs)
    wire: bool,
    /// the front tab's Play: what Edit left, what reached it from outside
    play: play::Play,
    marks: Vec<Mark>,
    /// the messages: the status bar's newest and counts, the Log panel's lines
    log: log::Log,
    /// the Log panel's places and the status bar's that open it, this frame
    log_hits: Vec<(Rect, log::Press)>,
    opens_log: Vec<Rect>,
    /// the front document's saved steps (history.rs) and its panel's places this frame
    history: history::History,
    history_hits: Vec<(Rect, history_panel::Press)>,
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
    /// this frame's parameter groups you can click: (x, y, node, group, how many)
    clusters: Vec<(f32, f32, usize, &'static str, usize)>,
    /// the last press on a body: for a double click
    last_body: Option<(usize, Instant)>,
    /// the last press on a cast member: a double one frames it
    last_member: Option<(usize, Instant)>,
    /// is the stage showing a level (Wire) now
    leveled: bool,
    /// Build's floating panels; where the viewer left every workspace's (none in a check run: it starts from the defaults)
    build: panels::Panels,
    panels_file: Option<PathBuf>,
    panels_moved: bool,
    /// the main menu is open; its lines this frame
    main_menu: bool,
    menu_lines: Vec<(Rect, bar::Line)>,
    /// the items open (tabs.rs, tab_view.rs), where a later start finds them, the tab list open and its lines
    tabs: tabs::Tabs<tab_view::Kept>,
    tabs_file: Option<PathBuf>,
    tab_list: bool,
    tab_lines: Vec<(Rect, bar::ListLine)>,
    /// Home's panels: only what every view has
    home_panels: panels::Panels,
    /// Home's stage: the deck's idle scene
    idle: Option<idle::Idle>,
    /// the tree's rows this frame: the level each one goes to (empty = the rig), what it selects
    rows: Vec<(Vec<String>, inspector::Sel)>,
    /// what the inspector shows
    sel: inspector::Sel,
    /// the inspector's page of a node's sections
    paging: inspector::Paging,
    /// the level the mixer shows; none = the one the stage is at
    mixer_tab: Option<usize>,
    /// a value being typed
    edit: Option<ValueEdit>,
    /// the last press on a field: for a double click
    last_field: Option<(Editing, Instant)>,
    /// #91: what the front tab draws with; the show's conductor and what a Show tab shows
    workspace: show::Workspace,
    show: show::Host,
    /// #477: the toon's address book, of the toon open now (toon.rs) – the toon and whether signed in when it was asked
    book: book::Loader,
    book_key: (Vec<String>, bool),
    /// the shown show and the tempo it keeps, as last taken by the clock
    show_tempo: Option<(String, Option<deck_show::Tempo>)>,
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
    /// a phone's take being recorded from Build's DOP panel (take.rs)
    take: Option<deck_space::space::dop::take::Recorder>,
    /// the video nodes' cards: their live frames (posters.rs)
    posters: posters::Posters,
    /// the medium in front, in its Asset tab (media.rs)
    media: media::Media,
    /// the scene tree's last press, for a double one
    last_row: Option<(deck_space::scene_edit::Part, Instant)>,
    /// Build's grid & snap (grid.rs)
    grid: grid::GridSnap,
    colour: colour::Colour,
    /// Home's New scene: its name and id asked first (scene_prompt.rs)
    scene_prompt: Option<scene_prompt::ScenePrompt>,
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
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,wgpu_core=warn,wgpu_hal=error,naga=warn".into());
    // the technical log goes to stderr and into the Log panel (info there only with --log-info)
    let info = std::env::args().any(|a| a == "--log-info");
    tracing_subscriber::registry().with(tracing_subscriber::fmt::layer().with_filter(filter)).with(log::Tech::new(info)).init();
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
    // `DECK_MODULES` before the rig: a draft naming `module:<name>` finds its kind
    deck_native::module_host::autoload();
    deck_space::space::motion::install(deck_engine::motion::opener(|id| {
        deck_native::module_host::autoload();
        deck_native::module_host::Library::global().get(id).map(|l| l.host())
    }));
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
    let mut build = panels::Panels::with_play(&panels::BUILD, &panels::BUILD_PLAY);
    let settings_file = args.frames.is_none().then(|| settings::file(&cache));
    let app_colour = colour::Colour::load(args.frames.is_none().then(|| colour::file(&cache)));
    let mut app_settings = settings_file.as_deref().map(settings::load).unwrap_or_default();
    app_settings.look.theme = args.theme.unwrap_or(app_settings.look.theme);
    app_settings.look.base = args.ui_size.unwrap_or(app_settings.look.base);
    for (kind, unit) in &args.units {
        app_settings.units.set(kind, *unit);
    }
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
    app_show.nodes.direction = args.nodes.unwrap_or(app_settings.node_flow);
    app_show.nodes.zoom_once = args.node_zoom;
    app_show.knobs.rigs.clone_from(&args.rigs);
    if args.frames.is_none() {
        app_show.keep_views_in(cache_dir().join("editor-show-views.json"));
    }
    if let Some(f) = &args.story {
        app_show.story_from_file(f);
    }
    if let Some(name) = &args.demo {
        app_show.open_demo(name).map_err(anyhow::Error::msg)?;
    }
    let forced: Vec<String> = args.toon.iter().cloned().collect();
    let app_book = book::Loader::new(&args.api, &forced, args.book.as_deref(), auth.token(), &cache);
    let book_key = (forced, auth.token().is_some());
    let mut app_module = module::Module::new(&args.api, &cache, args.module.clone());
    app_module.autosaves = args.frames.is_none();
    let mut app_library = library::Library::new(cache.clone());
    if let Some(dir) = &args.catalog {
        if let Some(list) = app_library.answers(dir) {
            app_module.list = list;
        }
    }
    let app_published = published::Published::new(&args.api, &cache);
    build.recall("build", &remembered);
    app_module.panels.recall("module", &remembered);
    if let Some(p) = remembered.get("show") {
        app_show.show_panels.floats.restore(p);
    }
    if args.audio {
        build.floats.reopen("audio");
    }
    if args.tools_folded {
        for p in [&mut build.floats, &mut app_module.panels.floats, &mut app_show.show_panels.floats] {
            p.set_collapsed("tools", true);
        }
    }
    if args.settings {
        for p in [&mut build, &mut app_module.panels, &mut app_show.show_panels] {
            p.floats.reopen(panels::SETTINGS);
        }
    }
    if args.log {
        for p in [&mut build, &mut app_module.panels, &mut app_show.show_panels] {
            p.floats.reopen(log::PANEL);
        }
    }
    if args.history {
        for p in [&mut build, &mut app_module.panels, &mut app_show.show_panels] {
            p.floats.reopen(history_panel::PANEL);
        }
    }
    for id in &args.closed {
        for p in [&mut build, &mut app_module.panels, &mut app_show.show_panels] {
            p.floats.close(id);
        }
    }
    let main_menu = args.main_menu;
    let app_tabs = tabs::Tabs::new(args.tab_cap.unwrap_or_else(|| tabs::cap_for(tabs::memory_mib(), std::env::var("DECK_EDITOR_TABS").ok().as_deref())));
    let tabs_file = args.tabs_file.clone().or_else(|| args.frames.is_none().then(|| tabs::file(&cache)));
    let tab_list = args.tab_list;
    let mut home_panels = panels::Panels::new(&panels::HOME);
    if args.log {
        home_panels.floats.reopen(log::PANEL);
    }
    if args.settings {
        home_panels.floats.reopen(panels::SETTINGS);
    }
    let own_frame = frame::own(args.fullscreen);
    let pointer = args.pointer;
    // a check run asks no server for presets
    let app_routing = routing::Page::new(deck_native::audio::backend::DeckAudio::read());
    let app_routing = if args.frames.is_none() { app_routing.with_api(&args.api) } else { app_routing };
    let app_history = history::History::new(std::sync::Arc::new(show::api::Api::new(args.api.clone())));
    let mut app = App {
        panel: Panel { tab: args.tab, field: text::Field::with("Grüße an die Bühne – type here"), drag: None, hits: Vec::new(), field_rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, advance: 8.0 },
        args,
        window: None,
        gpu: None,
        ui: None,
        gpu_time: None,
        cpu: deck_native::cpu::CpuMeter::spawn(),
        gpu_busy: Default::default(),
        gpu_load: Default::default(),
        stage,
        tokens,
        settings: app_settings,
        settings_file,
        settings_hits: Vec::new(),
        mods: ModifiersState::empty(),
        cursor: pointer.unwrap_or_default(),
        last: None,
        started: Instant::now(),
        posters: Default::default(),
        media: Default::default(),
        frames: 0,
        ui_ms: Vec::new(),
        frame_ms: Vec::new(),
        latency: latency::Probe::from_env(),
        own_frame,
        frame: frame::Own::default(),
        exit: 0,
        to_deck: false,
        clipboard: arboard::Clipboard::new().map_err(|e| tracing::warn!("no clipboard: {e}")).ok(),
        token_changes: 0,
        wire,
        play: play::Play::default(),
        marks: Vec::new(),
        log: log::Log::default(),
        log_hits: Vec::new(),
        opens_log: Vec::new(),
        history: app_history,
        history_hits: Vec::new(),
        undo: Vec::new(),
        draft,
        reach: Vec::new(),
        armed: None,
        menu: None,
        bodies: Vec::new(),
        clusters: Vec::new(),
        last_body: None,
        last_member: None,
        leveled: false,
        build,
        panels_file,
        panels_moved: false,
        main_menu,
        menu_lines: Vec::new(),
        tabs: app_tabs,
        tabs_file,
        tab_list,
        tab_lines: Vec::new(),
        home_panels,
        idle: None,
        rows: Vec::new(),
        sel: inspector::Sel::Rig,
        paging: inspector::Paging::default(),
        mixer_tab: None,
        edit: None,
        last_field: None,
        workspace: app_workspace,
        saved_in: app_workspace,
        autosaves: app_autosaves,
        show: app_show,
        book: app_book,
        book_key,
        show_tempo: None,
        rig_toon,
        auth,
        rigs,
        carry: None,
        routing: app_routing,
        module: app_module,
        published: app_published,
        scene: None,
        take: None,
        last_row: None,
        grid: grid::GridSnap::load(app_grid_file),
        colour: app_colour,
        scene_prompt: None,
        tools: tools::Tools::default(),
        library: app_library,
        library_open: None,
    };
    event_loop.run_app(&mut app).context("the event loop")?;
    app.flush_all();
    let summary = app.summary();
    tracing::info!("{summary}");
    println!("{summary}");
    if let Some(l) = app.latency.summary() {
        println!("{l}");
    }
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
        if let Some(k) = self.clusters.iter().position(|(cx, cy, ..)| (cx - x).hypot(cy - y) < 12.0) {
            return Hit::Cluster(k);
        }
        if let Some(i) = self.body_at(x, y) {
            return Hit::Body(i);
        }
        self.panel.hits.iter().find(|(r, _)| r.has(x, y)).map_or(Hit::Nothing, |(_, h)| *h)
    }

    /// the panels of the workspace shown
    fn panels(&self) -> &panels::Panels {
        match self.workspace {
            show::Workspace::Stage => &self.build,
            show::Workspace::Module => &self.module.panels,
            show::Workspace::Show => &self.show.show_panels,
            show::Workspace::Home | show::Workspace::Media => &self.home_panels,
        }
    }

    fn panels_mut(&mut self) -> &mut panels::Panels {
        match self.workspace {
            show::Workspace::Stage => &mut self.build,
            show::Workspace::Module => &mut self.module.panels,
            show::Workspace::Show => &mut self.show.show_panels,
            show::Workspace::Home | show::Workspace::Media => &mut self.home_panels,
        }
    }

    /// where the viewer left the panels, kept when one was moved or folded
    fn remember_panels(&mut self) {
        let Some(file) = &self.panels_file else { return };
        let mut all = panels::Remembered::new();
        self.build.remember("build", &mut all);
        self.module.panels.remember("module", &mut all);
        all.insert("show".into(), self.show.show_panels.floats.places());
        all.insert("home".into(), self.home_panels.floats.places());
        panels::save(file, &all);
    }

    /// whether a main menu line can be done here, and if not why
    fn menu_why_not(&self, a: bar::Action) -> Option<&'static str> {
        use bar::Action as A;
        match a {
            A::Open if self.workspace == show::Workspace::Module => Some(module::draw::PICK),
            A::Audio if self.workspace != show::Workspace::Stage && !self.tabs.list.iter().any(|t| matches!(t.kind, tabs::Kind::Scene | tabs::Kind::Character)) => Some("The Audio panel is a Scene's or a character's: open one."),
            A::Open | A::Audio | A::Settings | A::New => None,
            A::Publish => Some("The editor opens no browser yet: publish the saved draft in the creator."),
            A::Account => Some("The editor opens no browser yet: the account is on scutoons.com."),
            A::Quit if deck_native::handoff::sibling(deck_native::handoff::App::Deck).is_some() => None,
            A::Quit => Some("No stage next to this editor (deck is not installed here)."),
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
        if let Some(why) = tools::later(p) {
            return self.note(why.to_string());
        }
        match p {
            tools::Press::Tool(tool) => self.tools.tool = tool,
            tools::Press::View(two_d) => self.show.press(show::Hit::View(two_d)),
            tools::Press::Grid => self.grid.toggle(),
            tools::Press::Frame => self.stage.space.frame_all(),
            tools::Press::Look(l) => self.look_to(l),
            tools::Press::Menu(_) => {}
        }
    }

    /// what the stage looks through now, as the camera switcher shows it
    fn looking(&self) -> tools::Look {
        let space = &self.stage.space;
        match space.viewing() {
            deck_space::space::dop::Viewing::Free => tools::Look::Free,
            deck_space::space::dop::Viewing::Primary => tools::Look::Primary,
            deck_space::space::dop::Viewing::Shot(id) => space.dop.camera.shots.iter().position(|s| s.id == *id).map_or(tools::Look::Primary, tools::Look::Shot),
        }
    }

    /// the camera switcher: an eased glide to the free view, the primary camera or a shot; previous / next walk them
    fn look_to(&mut self, l: tools::Look) {
        let n = self.stage.space.dop.camera.shots.len();
        let l = match l {
            tools::Look::Prev | tools::Look::Next => self.looking().step(n, l == tools::Look::Next),
            l => l,
        };
        let space = &mut self.stage.space;
        match l {
            tools::Look::Free | tools::Look::Prev | tools::Look::Next => space.look_free(),
            tools::Look::Primary => space.look_through_scene(),
            tools::Look::Shot(i) => {
                if let Some(id) = space.dop.camera.shots.get(i).map(|s| s.id.clone()) {
                    space.look_through_shot(&id);
                    if let Some(v) = self.scene.as_mut() {
                        v.dop.shot = Some(i);
                    }
                }
            }
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
            (A::Open, show::Workspace::Stage | show::Workspace::Home) => self.open_rigs(x, y),
            (A::Open, _) => self.show.press(show::Hit::Load),
            (A::New, show::Workspace::Show) => self.show.press(show::Hit::Sheet(show::Sheet::NewShow)),
            // New is Home's: a scene, a module, a show
            (A::New, _) => self.home(),
            (A::Audio, _) => {
                if self.workspace != show::Workspace::Stage {
                    if let Some(i) = self.tabs.list.iter().position(|t| matches!(t.kind, tabs::Kind::Scene | tabs::Kind::Character)) {
                        self.activate(i);
                    }
                }
                self.build.floats.reopen("audio");
                self.panels_moved = true;
            }
            (A::Settings, _) => self.open_settings(),
            // the next frame leaves, as F9 does
            (A::Quit, _) => self.to_deck = true,
            _ => {}
        }
    }

    /// another look or a step of the UI size (0: the size stays, unless the theme is the same: back to 100 %)
    fn look(&mut self, theme: tokens::Theme, step: i32) {
        let now = self.settings.look;
        let next = if theme != now.theme { tokens::Look { theme, ..now } } else { now.sized(step) };
        self.change_settings(|s| s.look = next);
    }

    /// the node view's switches, its clock, its focus tone and the stills of the pieces its nodes play (settings.rs,
    /// library.rs); a check run's pointer stays where it was put
    fn follow_nodes(&mut self) {
        let m = self.settings.motion;
        if self.args.script.node_wire.is_none() || self.show.nodes.hand.is_none() {
            self.show.cursor = self.args.pointer.unwrap_or(self.cursor);
        }
        let n = &mut self.show.nodes;
        n.secs = self.started.elapsed().as_secs_f32();
        n.motion = show::nodes::Motion { activity: m.allows(m.signal_flow), reveal: m.allows(m.hover_reveal), flash: m.allows(m.focus_flash), previews: m.allows(m.live_previews) };
        if let (Some(hz), Some(volume)) = (n.take_tone(), self.settings.sound.focus()) {
            self.stage.ui_tone(hz, volume);
        }
        n.pictures.clear();
        if n.motion.previews {
            self.library.pictures.want_stills(&n.wanted);
            for k in n.wanted.clone() {
                if let Some(id) = self.library.pictures.of(&k) {
                    n.pictures.insert(k.clone(), id);
                }
            }
        }
    }

    /// the settings changed: kept, and the look applied at once
    fn change_settings(&mut self, f: impl FnOnce(&mut settings::Settings)) {
        let was = self.settings.clone();
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
                deck_kit::editor::floats::Chrome::Edge(_) => "Drag to resize it; it stops 1rem before a wall or another panel, and keeps its size.",
                deck_kit::editor::floats::Chrome::Scroll => "Click the track to jump there; drag the thumb to scroll.",
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
            Hit::Menu => Help::new("Menu", "Open, publish, day or night, the settings, the audio and the account; this view's panels; back to the deck's stage."),
            Hit::TempoClick => Help::new(
                if self.stage.pill.is_some_and(|p| p.click) { "Click on" } else { "Click off" },
                "The metronome: a tick on every beat, higher on the bar's first, on its own bus (the click outlet group, on the main out until the Audio tab routes it). Click to turn it on or off.",
            ),
            Hit::Tempo | Hit::TempoBpm | Hit::TempoSource => Help::new(format!("{} BPM", self.stage.beat.map_or_else(|| "–".to_string(), |(b, _)| format!("{b:.1}"))), match (self.stage.beat, self.stage.pill) {
                _ if self.stage.held() => "LIVE: the live show's clock, locked – what plays here follows it.",
                _ if self.stage.live => "LIVE: locked while the show is live; it follows the show's clock once the relay sends it.",
                (_, Some(p)) if p.external && !p.locked => "EXT: waiting for the tempo source. Click the BPM to type a tempo and take the clock back; click EXT for the next source.",
                (_, Some(p)) if p.external => "EXT: the tempo comes from outside the deck (MIDI clock in, Link). Click the BPM to type a tempo and take the clock back; click EXT for the next source.",
                (Some((_, true)), _) => "INT: the deck's own tempo clock runs. Click the BPM to type a tempo; click INT to follow MIDI clock in or Link.",
                (Some((_, false)), _) => "INT: the deck's own tempo clock, standing. Click the BPM to type a tempo.",
                (None, _) => "No tempo clock plays here.",
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
            Hit::Meters => Help::new("Load", bar::meters_words(self.cpu.as_ref().and_then(|c| c.reading()), self.gpu_time.as_ref().map(|_| self.gpu_load.reading().unwrap_or_default()), self.stage.khz().map(|k| k as f32), self.fps())),
            Hit::Panic if self.show.sub.panicked() => Help::new("Resume", "The panic holds. Press to bring every deck back to the cells the show holds now."),
            Hit::Panic => Help::new("Panic", "Black and silence on every deck below this conductor and on this stage, until you resume here. Shift+Esc too."),
            Hit::Ground => return None,
            Hit::Live(live_tabs::Press::Go) => Help::new("Go live", "This show joins the relay: its decks, the readers and the vote. Only one show is live; another live show is asked to end first."),
            Hit::Live(live_tabs::Press::End) => Help::new("End", "The show leaves the relay, its decks and readers with it. The tab stays open; the lock comes off."),
            Hit::Live(live_tabs::Press::Lock) => Help::new(if self.show.locked { "Unlock" } else { "Lock" }, "Against edits during a live night: launching, GO, the session and the vote stay; every edit of the show is refused until unlocked."),
            Hit::Home => Help::new("Home", "Recent, the library, New and the deck's state. The tabs stay open behind it."),
            Hit::TabFront(i) if self.tabs.front == Some(i) => Help::new(self.tabs.list.get(i)?.name.clone(), "In front: it runs on the stage. Click to go back to its top."),
            Hit::TabFront(i) => {
                let tab = self.tabs.list.get(i)?;
                Help::new(tab.name.clone(), format!("A {} in a {} tab, kept as you left it. Click to bring it to the front.", tab.kind.word(), tab.kind.view().word()))
            }
            Hit::TabClose(i) => Help::new(format!("Close {}", self.tabs.list.get(i)?.name), "What it changed is saved; it goes to Home's Recent."),
            Hit::TabList => Help::new("Tabs", "Every tab open, and Close others, Close all (all closed: Home)."),
            Hit::HomePress(home::Press::Docs) => Help::new("Help", "Copies the link to the creator's docs."),
            Hit::HomePress(home::Press::New(_)) => Help::new("New", "A new scene (in a show's composer), a module (from one of the catalog's) or a show, in a tab of its own."),
            Hit::HomePress(_) => Help::new("Open", "Opens it in a tab, or brings its tab to the front."),
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
            Hit::Cluster(k) => {
                let &(_, _, _, name, count) = self.clusters.get(k)?;
                Help::new(format!("{name} · {count}"), "A group of the node's parameters. Click to open it in place, again to close it. Its wired ports always show.")
            }
            Hit::Body(i) => Help::new(self.bodies.get(i)?.2.clone(), "A track, one body from outside: only its outlets show. Double click, or point at it and press tab, to go in."),
            Hit::Crumb(0) => Help::new("rig", "Back to the rig, every track one body."),
            Hit::Row(_) => Help::new("Tree", "The rig, its tracks and their nodes. In Wire a click goes to that level."),
            Hit::Crumb(d) => Help::new(self.stage.path.get(d - 1)?.clone(), "Back to this level. Esc or shift+tab goes one level up."),
            Hit::Switch if self.playing() => Help::new("Edit · Play", "Playing: it runs as a deck would, the panels that play are out and Edit's rest. A click goes back to Edit, its panels as you left them; what you changed while playing is dropped unless you Keep it."),
            Hit::Switch => Help::new("Edit · Play", "Editing: the body holds still, the ports show, inputs from outside only light their ports. A click plays it as a deck would, with Play's own panels – what you change there is remembered for this kind of tab."),
            Hit::Axis(Some(a)) => Help::new(format!("{}{}", if a < 3 { "+" } else { "-" }, ["X", "Y", "Z"][a % 3]), "Click to look along this axis, drag to orbit."),
            Hit::Axis(None) => Help::new("View", "Drag to orbit, click an axis to look along it."),
            Hit::Stage if self.wire => Help::new("Stage", "Left drag orbits; middle or right drag, Shift + drag or two fingers pan; the wheel zooms, F frames. Right click finds any port."),
            Hit::Stage => Help::new("Stage", "Drag a knob to turn it, anywhere else to orbit. Middle or right drag, Shift + drag or two fingers pan; the wheel zooms, F frames all, a double click frames a member."),
            Hit::Knob(i) => Help::new(self.stage.knobs.get(i)?.label.clone(), "Drag or use the wheel to turn it in the engine."),
            Hit::Tab(_) => Help::new("Tab", "Switches what the inspector shows."),
            Hit::Routing(f) => {
                let (title, text) = self.routing.help(f);
                Help::new(title, text)
            }
            Hit::Param(n, p) => Help::new(self.stage.param(n, p)?.name, "Drag or use the wheel to set it in the engine."),
            Hit::Area(..) => Help::new("Area", "One button per area, the Analog Four's way: a press opens its first page, a press again its next page. Twin pages put one idea on the same knob."),
            Hit::Address(_) => Help::new("Address", "Click: the toon's address book. A pick becomes the holder this far world node listens to."),
            Hit::GainText(_) => Help::new("Level", "In dB. Double click to type a value, enter to set it, esc to leave it."),
            Hit::StripSelect(_) => Help::new("Strip", "Click to show what it belongs to in the inspector and on the stage."),
            Hit::MixerTab(_) => Help::new("Level", "The mixer at this level of the tree: the rig's tracks, or the gains inside a track."),
            Hit::Fader(mixer::Target::Track(i), _) => Help::new(self.stage.strips.get(i)?.track.clone(), "The track's level. Press anywhere on it or drag it; it goes into the draft when you let go."),
            Hit::Fader(mixer::Target::Inner(i), _) => {
                let s = self.stage.inner.get(i)?;
                Help::new(format!("{}:{}", s.track, s.label), format!("How much of {} goes on. Press anywhere on it or drag it; it goes into the draft when you let go.", s.source))
            }
            Hit::Balance(i) => Help::new(format!("Pan {}", self.stage.strips.get(i)?.track), "Left and right on a stereo pair. Drag it; it applies when you let go."),
            Hit::Peak(_) => Help::new("Highest peak", "The loudest moment in dB, red once it went over 0. Click to clear it."),
            Hit::RigPick => Help::new("Rig", "Click: the toon's rigs on the server; a pick opens it on the stage. A local copy that differs from the server's is offered."),
            Hit::Add => Help::new("Add", "Inside a track: a node, a far world node with its holder from the address book. At the rig: a track playing an instrument of the catalog. Ctrl+z takes it back."),
            Hit::BackgroundColour => Help::new("Colour", "Choose the background's colour: the palette, your recent colours, saturation and brightness over the hue, or the eyedropper – a click takes the colour under the pointer on the stage, Esc cancels."),
            Hit::Background => Help::new("Background", "The stage's background while this rig plays: a colour (#rrggbb) or a media path (an image laid over the stage). Empty = its modules', else black; a scene or a show over it wins."),
            Hit::PresetLoad(_) => Help::new("Load preset", "Puts every knob where this preset has it: the rig's working state, which saves itself as the rig's draft. A dot says a knob moved since. Ctrl+z takes it back."),
            Hit::PresetSaveAs => Help::new("Save as preset", "Saves every knob of the rig as a new preset of yours, under the name you type. A draft – publish it in the creator."),
            Hit::PresetSave => Help::new("Save preset", "Saves the knobs into the loaded preset. A factory preset is never changed: this makes a preset of yours that names it as its origin."),
            Hit::Keep => Help::new("Keep", "What you changed while playing becomes the working state: Stop keeps it and it saves itself. Values from outside are not kept."),
            Hit::ScenePreset(_) => Help::new("Rig preset", "The member plays its rig on this preset; the scene's values are single knobs over it. None = the rig's own values."),
            Hit::Remove => Help::new("Remove", "Takes the selected node or track away, its cables with it (delete does the same). Ctrl+z takes it back."),
            Hit::Mute(i) => Help::new(format!("Mute {}", self.stage.strips.get(i)?.track), "Takes the track out of the sound."),
            Hit::Solo(i) => Help::new(format!("Solo {}", self.stage.strips.get(i)?.track), "Only the soloed tracks are heard (and an armed one). For this session; not saved with the rig."),
            Hit::Arm => Help::new("Arm", "This track takes your hand in Edit, and its monitoring is on: you hear it through another's solo. One at a time; not saved."),
            Hit::ToCell => Help::new(show::build::TO_CELL, "Writes the knobs as this tab has them now into the show cell this rig was opened from; the show checks their ranges."),
            Hit::BackToShow => Help::new(show::build::BACK, "The show's tab, the linked cell selected."),
            Hit::SceneKnob(_) => Help::new("Value in the scene", "Drag to set this member's knob in the scene; the stage plays it when you let go; the scene saves itself as a draft."),
            Hit::SceneClear(_) => Help::new("Its rig's value", "Takes the scene's value off: the member plays its rig's own."),
            Hit::SceneInto => Help::new("Open as rig", "One level down: the member's rig alone on the stage, the scene's values on its knobs. Esc or the scene's line takes them back into the scene."),
            Hit::Way(empty::Way::Modules) => Help::new("Start from modules", "Add at the rig: a track playing a module or an instrument of the catalog; the rig saves itself as its first draft."),
            Hit::Way(empty::Way::NewModule) => Help::new("New module", "A module of the character's own as a draft, opened in a tab of its own: write it, build it, hear it, then add it here as a track."),
            Hit::SceneField(scene::Field::Background) => Help::new("Background", "The stage's background in this scene: a colour (#rrggbb), an image's media path laid over the stage, or a video's (it does not play yet). Empty = its rigs', else black; a show over it wins."),
            Hit::Dop(dop::Field::Focal) => Help::new("Focal length", "Type a focal length in mm for the extreme cases; the presets are 17, 30, 50, 70, 100 and 150."),
            Hit::Dop(dop::Field::Preview) => Help::new("Preview", "Plays the move from where the camera is now to the selected shot on this stage; a hand on the view stops it."),
            Hit::Dop(dop::Field::Fold(_)) => Help::new("Camera tools", "Click a section to open or close it; Alt-click opens or closes all of them. Each tab keeps its own."),
            Hit::Dop(_) => Help::new("Camera tools", "The scene's camera: its lens, its shots (a show row picks one by scene:<id> > shot:<id>), a move to preview or keep (scene:<id> > move:<id>), the trolley track."),
            Hit::SceneField(scene::Field::AddStack | scene::Field::AddLayer | scene::Field::Picture | scene::Field::Anchor) => Help::new("Parallax layers", "Pictures standing in the 3D at their depths, fitted to the scene camera's view there: a far matte, layers behind and around the 3D, strips at the edges in front. Drop a picture from the library on a layer's handle; drag the handle up or down to move it in depth – it is fitted anew."),
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
        if self.media.drag.is_some() {
            return self.media_drag_to(x);
        }
        if self.grid.scrub_to(x) {
            return;
        }
        match self.panel.drag {
            Some(Drag::Member) if self.tools.tool == tools::Tool::Move => self.grid_drag(x, y),
            Some(Drag::Member) => {}
            Some(Drag::Knob(i, _, _)) => self.knob_at(i, x),
            Some(Drag::SceneKnob(i, x0, v0)) => self.scene_knob_to(i, x0, v0, x),
            Some(Drag::Dop(n, x0, v0)) => self.dop_scrub_to(n, x0, v0, x),
            Some(Drag::TrackPoint(i, handle)) => self.track_point_to(i, handle, x, y),
            Some(Drag::Focus(i)) => self.focus_to(i, x, y),
            Some(Drag::LayerDepth(l, y0, d0)) => self.layer_depth_to(l, y0, d0, y),
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
            Some(Drag::Fader(i, grip)) => self.fader_to(i, grip.at(y)),
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
                let h = self.view_h();
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

    /// the window's own frame takes a press on its edge, a window button or the toolbar's empty run; an open menu,
    /// a typed field or the colour taker has the press first
    fn frame_press(&mut self) -> bool {
        if self.scene_prompt.is_some() || self.edit.is_some() || self.menu.is_some() || self.main_menu || self.colour.taking() {
            return false;
        }
        let Some(w) = self.window.clone() else { return false };
        self.frame.press(&w, self.cursor, self.started.elapsed().as_secs_f64())
    }

    fn press(&mut self) {
        let (x, y) = self.cursor;
        if let Some(p) = self.scene_prompt.as_mut() {
            let outcome = p.press(x, y);
            return self.scene_prompt_done(outcome);
        }
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
        // the tab list takes the press while it is open, as the main menu does
        if self.tab_list {
            self.tab_list = false;
            match self.tab_lines.iter().find(|(r, _)| r.has(x, y)).map(|(_, l)| *l) {
                Some(bar::ListLine::Tab(i)) => self.activate(i),
                Some(bar::ListLine::CloseOthers) => self.close_others(),
                Some(bar::ListLine::CloseAll) => self.close_all(),
                None => {}
            }
            return;
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
        if self.press_history(x, y) || self.press_status(x, y) {
            return;
        }
        // a panel's handle takes it in the hand, its chevron folds it, its close closes it
        if self.panels_mut().floats.press(x, y) {
            self.panels_moved = true;
            return;
        }
        if self.press_log(x, y) {
            return;
        }
        if self.workspace == show::Workspace::Show && self.show.nodes.space && self.show.nodes_grab(x, y) {
            return;
        }
        self.library.browser.keys = (self.mods.shift_key(), self.mods.control_key() || self.mods.super_key());
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
        if self.workspace == show::Workspace::Media {
            if self.media_press(x, y) {
                return;
            }
        } else if self.workspace == show::Workspace::Module {
            if let Some(module::Hit::Unit(kind, unit)) = self.module.hit(x, y) {
                return self.change_settings(|s| s.units.set(kind, unit));
            }
            if let Some(h) = self.module.hit(x, y) {
                return self.module.press(h, x, &self.stage);
            }
            if self.panels().floats.covers(x, y) {
                return;
            }
        } else if self.workspace == show::Workspace::Home {
            if self.panels().floats.covers(x, y) {
                return;
            }
        } else if self.workspace != show::Workspace::Stage {
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
            if let Some(u) = self.grid.take_unit() {
                self.change_settings(|s| s.units.set(dop::DISTANCE_UNIT, u));
            }
            return;
        }
        match self.hit(x, y) {
            Hit::Live(p) => self.live_press(p),
            Hit::Home => self.home(),
            Hit::TabFront(i) if self.tabs.front == Some(i) => self.tab_top(),
            Hit::TabFront(i) => self.activate(i),
            Hit::TabClose(i) => self.close_tab(i),
            Hit::TabList => self.tab_list = true,
            Hit::HomePress(p) => self.home_press(p),
            Hit::Menu => self.main_menu = true,
            Hit::Panic => self.panic_pill(),
            Hit::Account => {
                let line = self.auth.line(&deck_link::pairing::app_url());
                let said = match self.auth.name() {
                    Some(name) => format!("signed in as the editor {name:?}"),
                    None if line.is_empty() => "signed in".to_string(),
                    None => line,
                };
                if self.auth.state() == sign_in::Signed::Out {
                    self.refuse(said, x, y);
                } else {
                    self.note(said);
                }
            }
            Hit::Tempo | Hit::Meters => {}
            Hit::TempoBpm => self.tempo_edit(x, y),
            Hit::TempoSource => self.tempo_pick(x, y),
            Hit::TempoClick => self.tempo_click(),
            Hit::Ground => {}
            Hit::Crumb(i) if self.workspace != show::Workspace::Stage => self.show.press(show::Hit::Crumb(i)),
            Hit::Routing(f) => routing::act(&mut self.routing, &mut self.stage, &self.args.api, f),
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
            Hit::Area(n, a) => {
                let pages = self.stage.kinds.get(n).map_or(&[][..], |k| k.pages());
                let count = inspector::areas(pages).get(a).map_or(1, |(_, list)| list.len());
                self.paging = self.paging.press(n, a, count);
            }
            Hit::Field(_) => {
                let col = ((x - self.panel.field_rect.x) / self.panel.advance).round().max(0.0) as usize;
                let to = self.panel.field.byte_at_char(col);
                self.panel.field.go(to, self.mods.shift_key());
                self.panel.drag = Some(Drag::Select);
                self.focus(true);
            }
            Hit::Switch => self.set_play(!self.playing()),
            Hit::Port(i) => {
                let p = &self.stage.ports[i];
                if p.input && !p.cables.is_empty() {
                    match wires::pulled(&self.stage.bundle, p) {
                        Ok(_) => {
                            let reach = wires::reachable_moved(&self.stage.bundle, &self.stage.ports, p);
                            self.reach = self.at_level(reach);
                            self.panel.drag = Some(Drag::Grab(i));
                        }
                        Err(sentence) => self.refuse(sentence, x, y),
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
            Hit::Fader(i, f) => {
                let (grip, jumped) = f.press(y);
                self.panel.drag = Some(Drag::Fader(i, grip));
                if let Some(to) = jumped {
                    self.fader_to(i, to);
                }
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
            Hit::Solo(i) => {
                if let Some(track) = self.stage.strips.get(i).map(|s| s.track.clone()) {
                    let on = self.stage.solo(&track);
                    self.note(format!("{track}: {}", if on { "solo" } else { "solo off" }));
                }
            }
            Hit::Arm => {
                if let inspector::Sel::Track(track) = self.sel.clone() {
                    let on = self.stage.arm(&track);
                    self.note(format!("{track}: {}", if on { "armed, monitored" } else { "disarmed" }));
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
            Hit::Cluster(k) => {
                if let Some(&(_, _, node, group, _)) = self.clusters.get(k) {
                    self.stage.toggle_group(node, group);
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
                if self.scene_mark_press(x, y) {
                    return;
                }
                let (sx, sy) = self.stage_pixel(x, y);
                let member = self.stage.space.pick_at(sx, sy);
                let double = member.is_some() && self.last_member.is_some_and(|(m, at)| Some(m) == member && at.elapsed().as_millis() < 400);
                self.last_member = member.map(|m| (m, Instant::now()));
                if let (true, Some(m)) = (double, member) {
                    self.stage.space.frame_member(m);
                    return;
                }
                // Play: the deck's own hand – a knob under the pointer turns, anywhere else it orbits; Shift pans off the cast
                if self.mods.shift_key() && member.is_none() {
                    self.panel.drag = Some(Drag::Pan(x, y));
                } else if self.grid_grab(x, y) {
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
            Hit::Dop(dop::Field::Scrub(n)) => {
                if let Some(v) = self.scene.as_ref().map(|v| dop::value(v, n)) {
                    self.panel.drag = Some(Drag::Dop(n, x, v));
                }
            }
            Hit::Dop(f) => self.dop_field(f, x, y),
            Hit::Add => self.open_add(x, y),
            Hit::Way(w) => self.start_rig(w, x, y),
            Hit::Remove => self.remove_selected(),
            Hit::Background => {
                let now = self.stage.bundle.rig.background.as_ref().map_or("", |b| b.text()).to_string();
                self.begin_edit(Editing::Background, Hit::Background, now);
            }
            Hit::BackgroundColour => self.colour_open(colour::Target::Rig),
            Hit::PresetLoad(i) => self.preset_load(i),
            Hit::PresetSaveAs => {
                let name = format!("preset {}", self.rigs.presets(&self.stage.bundle.rig.subject).len() + 1);
                self.begin_edit(Editing::PresetName, Hit::PresetSaveAs, name);
            }
            Hit::PresetSave => self.preset_save(),
            Hit::Keep => self.keep_play(),
            Hit::ScenePreset(i) => self.scene_preset(i),
            Hit::ToCell => self.knobs_to_cell(),
            Hit::BackToShow => self.go_show(),
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
        if self.workspace == show::Workspace::Show && button == MouseButton::Middle && self.show.nodes_grab(x, y) {
            return;
        }
        if self.workspace == show::Workspace::Show && button == MouseButton::Right && self.mods.control_key() && self.show.nodes_cut(x, y) {
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

    /// the stage's height in logical pixels
    fn view_h(&self) -> f32 {
        self.gpu.as_ref().map_or(720.0, |g| g.size().1 as f32 / self.scale())
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
            Editing::PresetName => self.preset_save_as(&text),
            Editing::Focal => self.dop_focal(&text),
            Editing::Height => self.dop_height(&text),
            Editing::SceneLayer(l) => self.scene_layer(l, &text),
            Editing::LogFind => self.log.find = text,
            Editing::Tempo => self.tempo_typed(&text),
        }
    }

    /// the master BPM: typed in place on the pill (a live show's clock holds it back)
    fn tempo_edit(&mut self, x: f32, y: f32) {
        if self.stage.held() || self.stage.live {
            return self.refuse("the live show's clock sets the tempo while it is live".into(), x, y);
        }
        let Some((bpm, _)) = self.stage.beat else { return self.refuse("no tempo clock plays here".into(), x, y) };
        self.begin_edit(Editing::Tempo, Hit::TempoBpm, deck_core::units::number(bpm, deck_core::units::Unit::Bpm));
    }

    /// what was typed on the pill, in BPM, into the deck's clock
    fn tempo_typed(&mut self, text: &str) {
        let (x, y) = self.cursor;
        use deck_core::units::{convert, parse, Refs, Unit};
        let typed = parse(text, Unit::Bpm).and_then(|(v, u)| convert(v, u, Unit::Bpm, &Refs::default()));
        let Some(bpm) = typed.filter(|v| *v > 0.0) else { return self.refuse(format!("'{text}': a tempo in BPM"), x, y) };
        let held = deck_core::tempo::clamp_bpm(bpm);
        let source = self.stage.pill.map_or(deck_core::tempo::Source::Internal, |p| p.source).typed();
        self.stage.set_bpm(held);
        self.keep_tempo(held, source);
        let said = if (held - bpm).abs() > 1e-9 { format!("tempo: {held:.0} BPM ({bpm:.0} is out of {:.0}-{:.0})", deck_core::tempo::MIN_BPM, deck_core::tempo::MAX_BPM) } else { format!("tempo: {held:.1} BPM") };
        self.note(said);
    }

    /// the pill's source word: the next pick - own clock, MIDI clock in, Link (the show clock is the deck's)
    fn tempo_pick(&mut self, x: f32, y: f32) {
        if self.stage.held() || self.stage.live {
            return self.refuse("the live show's clock sets the tempo while it is live".into(), x, y);
        }
        let now = self.stage.pill.map_or(deck_core::tempo::Source::Internal, |p| p.source);
        let next = now.next_pick(&stage::Stage::picks());
        self.stage.pick_clock(next);
        if let Some((bpm, _)) = self.stage.beat {
            self.keep_tempo(bpm, next);
        }
        self.note(match next {
            deck_core::tempo::Source::Internal => "tempo: the deck's own clock".to_string(),
            s => format!("tempo: follows {} - it sets the BPM; type one to take it back", pick_name(s)),
        });
    }

    /// the metronome on or off (the pill's button)
    fn tempo_click(&mut self) {
        let on = !self.stage.pill.is_some_and(|p| p.click);
        self.stage.set_click(on);
        self.note(if on { "click on: the metronome on its own bus" } else { "click off" }.to_string());
    }

    /// in the Show workspace the pill's tempo is the show's: kept in its document, saved with it
    fn keep_tempo(&mut self, bpm: f64, source: deck_core::tempo::Source) {
        if self.workspace == show::Workspace::Show {
            let to = deck_show::Tempo::of(bpm, source);
            self.show.keep_tempo(to);
            self.show_tempo = Some(self.show.tempo());
        }
    }

    /// a show opened in the Show workspace sets the clock to the tempo it keeps (once per document and tempo; a live
    /// show's clock holds it back)
    fn take_show_tempo(&mut self) {
        if self.workspace != show::Workspace::Show || self.stage.held() || self.stage.live {
            return;
        }
        let kept = self.show.tempo();
        if self.show_tempo.as_ref() == Some(&kept) {
            return;
        }
        if let Some(t) = &kept.1 {
            self.stage.take_saved(t);
        }
        self.show_tempo = Some(kept);
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
            Err(e) => self.refuse(e, x, y),
        }
    }

    fn scene_background(&mut self, text: &str) {
        let (x, y) = self.cursor;
        let Some(view) = self.scene.as_mut() else { return };
        match view.edit.set_background(text) {
            Ok(said) => {
                self.replay_scene();
                self.note(said);
            }
            Err(e) => self.refuse(e, x, y),
        }
    }

    /// a node's input as the engine has it now goes into the draft (no replay: the engine has it already)
    fn persist(&mut self, node: usize, name: &str) {
        let Some(id) = self.stage.node_ids.get(node) else { return };
        let Some(p) = self.stage.ports.iter().find(|q| q.input && q.node == *id && q.name == name) else { return };
        let Some((_, local)) = p.address.split_once(':') else { return };
        let done = wires::set_value(&self.stage.bundle, &p.track, local, self.stage.saved(node, name));
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
            Err(sentence) => self.refuse(sentence, x, y),
        }
    }

    /// the rig as it plays into its copy; outside a scene it saves itself (inside, the scene takes its knobs)
    fn drafted(&mut self, x: f32, y: f32) {
        // Play's changes are dropped on Stop: no copy, no save
        if self.playing() {
            return;
        }
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
                    self.refuse(format!("{e:#}"), x, y);
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

    /// a fader at `f` of its track: the engine at once
    fn fader_to(&mut self, target: mixer::Target, f: f32) {
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
        let row = |i: usize| book.row(i, LOCALE).map(|r| typeahead::Item { id: i, label: r.chain, detail: format!("{}{}{}", r.kind.as_str(), if r.draft { " · draft" } else { "" }, r.toon.map(|t| format!(" · {t}")).unwrap_or_default()) });
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
        let build = self.workspace == show::Workspace::Stage;
        let toons = toon::wanted(self.args.toon.as_deref(), build, self.rig_toon.toon.as_deref(), &self.show.shown().toons);
        let token = self.auth.token();
        let key = (toons, token.is_some());
        if key != self.book_key {
            self.book = book::Loader::new(&self.args.api, &key.0, None, token, &cache_dir());
            self.book_key = key;
        }
    }

    /// Build's rig on the server (rigs.rs): what its threads answered goes onto the stage and into the messages
    fn follow_rigs(&mut self) {
        let (x, y) = self.cursor;
        for e in self.rigs.poll(self.auth.token().is_some()) {
            match e {
                rigs::Event::Opened(subject, opened) => {
                    let (base, plays) = *opened;
                    let (key, name) = tab_view::rig_key(&subject);
                    self.adopt(&key, &name);
                    self.workspace = show::Workspace::Stage;
                    self.drop_scene();
                    self.stage.path.clear();
                    if let Err(err) = self.stage.replay(plays) {
                        self.refuse(format!("{err:#}"), x, y);
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
                rigs::Event::Say(text, true) => self.refuse(text, x, y),
                rigs::Event::Say(text, false) => self.note(text),
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
        if let show::build::ToBuild::Scene(o) = asked {
            return self.open_scene(*o);
        }
        match asked {
            show::build::ToBuild::Scene(_) => {}
            show::build::ToBuild::Play(o, set) => {
                let subject = o.bundle.rig.subject.clone();
                let (key, name) = tab_view::rig_key(&subject);
                self.adopt(&key, &name);
                self.workspace = show::Workspace::Stage;
                self.drop_scene();
                self.stage.path.clear();
                if let Err(err) = self.stage.replay(o.bundle.clone()) {
                    return self.refuse(format!("{err:#}"), x, y);
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
            }
            show::build::ToBuild::Server(subject) => {
                if let Err(e) = self.rigs.open(&subject) {
                    self.refuse(format!("{subject}: {e}"), x, y);
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
                self.refuse(format!("scene:{id}: {err:#}"), x, y);
            }
        }
        if self.workspace == show::Workspace::Stage && self.stage.scene.is_some() && self.scene.is_none() {
            if let Err(err) = self.stage.replay(self.stage.bundle.clone()) {
                self.refuse(format!("{err:#}"), x, y);
            }
            self.stage.scene = None;
        }
        if self.workspace == show::Workspace::Stage && self.scene.as_ref().is_some_and(|v| v.inside.is_none() && !v.shown) {
            self.replay_scene();
        }
    }

    /// #568: "values → cell" – the knobs as the stage has them, into the linked cell (show/build.rs)
    fn knobs_to_cell(&mut self) {
        let positions: Vec<(String, f32)> = self.stage.knobs.iter().map(|k| (k.path.clone(), self.stage.value(k))).collect();
        let done = self.show.values_to_cell(&positions);
        if !done {
            self.show.bad = true;
        }
    }

    /// the rig picker: the toon's rigs on the server, the local copy when it differs; why there are none inside it
    fn open_rigs(&mut self, x: f32, y: f32) {
        let registry = std::fs::read_to_string(toon::kept(&cache_dir())).unwrap_or_default();
        let (items, note) = self.rigs.items(self.book_key.0.first().map(String::as_str), &registry, &self.auth.line(&deck_link::pairing::app_url()));
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
                        self.refuse(e, x, y);
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
            Some(Drag::SceneKnob(..) | Drag::LayerDepth(..)) => return self.scene_settled(),
            Some(Drag::Dop(n, ..)) => return self.dop_settled(n),
            Some(Drag::TrackPoint(..)) => return self.track_settled(),
            Some(Drag::Focus(i)) => return self.focus_settled(i),
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
                    self.note(text);
                }
            }
            Err(sentence) => self.refuse(sentence, x, y),
        }
    }

    /// a changed rig on the stage and in the draft; false and the sentence when it does not play
    fn take(&mut self, next: deck_space::rig::Bundle, x: f32, y: f32) -> bool {
        if let Err(e) = self.stage.replay(next) {
            self.refuse(format!("{e:#}"), x, y);
            return false;
        }
        self.drafted(x, y);
        true
    }

    /// #609: F9 – hands the session, the show and the stage's scene to the deck and quits; no deck next to this editor (or
    /// a failed start) stays and says why in the status bar. True = the editor is leaving.
    fn hand_off(&mut self, el: &ActiveEventLoop) -> bool {
        use deck_native::handoff::{self, App, Handoff};
        let show = self.args.show_file.as_ref().map(|p| p.display().to_string()).or_else(|| self.args.show_id.clone());
        let h = Handoff { relay: self.args.relay.clone(), show, ..self.args.handoff.clone() };
        match handoff::go(App::Editor, h, handoff::own_args(std::env::args().skip(1)), &deck_native::switcher::deck_toml()) {
            Ok(code) => {
                self.exit = code;
                el.exit();
                true
            }
            Err(r) => {
                log::post(deck_ui::messages::Level::Warn, "editor", r.sentence());
                false
            }
        }
    }

    /// what the editor did or found, to the status bar and the Log
    fn note(&mut self, text: String) {
        log::post(deck_ui::messages::Level::Info, "editor", text);
    }

    /// a press or a drop refused at (x, y): a short ring there, the sentence to the status bar and the Log
    fn refuse(&mut self, text: String, x: f32, y: f32) {
        self.log.refuse(text, (x, y));
    }

    fn take_back(&mut self) {
        let Some(next) = self.undo.pop() else { return };
        let (x, y) = self.cursor;
        self.take(next, x, y);
    }

    /// a check run's drag at fixed frames: the pointer's own path, through hit test, press, move and release
    fn script(&mut self) {
        self.script_history();
        self.script_colour();
        self.script_library();
        self.show.script(self.frames, &self.args.script);
        if self.frames == 70 && self.args.script.to_cell {
            self.knobs_to_cell();
        }
        self.script_scene();
        self.script_presets();
        self.script_dop();
        self.script_slots();
        self.script_media();
        self.media_saved();
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
            self.media.panic();
        } else if let Err(err) = self.stage.resume() {
            let (x, y) = self.cursor;
            self.refuse(format!("{err:#}"), x, y);
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
        self.latency.frame();
        if self.frames == 0 {
            self.start_tabs();
        }
        let dir = self.show.nodes.direction;
        if dir != self.settings.node_flow {
            self.change_settings(|s| s.node_flow = dir);
        }
        self.follow_toon();
        self.follow_rigs();
        self.follow_history();
        self.follow_modules();
        self.script_play();
        self.autosave();
        // Home lists the catalog's modules too (a check run's come from --catalog)
        if self.workspace == show::Workspace::Module || (self.workspace == show::Workspace::Home && self.autosaves) {
            self.module.enter();
        }
        self.module.poll(&mut self.stage);
        if let (Some(link), Some(cb)) = (self.module.link.take(), self.clipboard.as_mut()) {
            let _ = cb.set_text(link);
        }
        self.follow_cell();
        self.follow_scene();
        self.follow_tabs();
        self.take_frame();
        if let Some(f) = self.show.demo_book.take() {
            self.book = book::Loader::new(&self.args.api, &[], Some(&f), None, &cache_dir());
            self.args.book = Some(f);
        }
        self.book.poll();
        // #537: the Show workspace picks a cell's piece from the book's scenes
        self.show.book.clone_from(&self.book.book);
        self.show.book_why = self.book_why();
        if let Some(view) = self.scene.as_ref() {
            let camera = view.edit.camera();
            let shots = camera.shots.iter().map(|s| (s.id.clone(), dop::shot_words(s))).collect();
            self.show.cameras.insert(view.edit.key(), shots);
            self.show.offer_cameras(view.edit.key(), deck_space::live_cameras::offered(&view.edit.key(), &camera));
        }
        self.follow_library();
        if std::mem::take(&mut self.panels_moved) {
            self.remember_panels();
        }
        if self.frames == 0 {
            self.script_save_state();
            self.script_live();
        }
        self.follow_live();
        self.follow_cue();
        if self.frames == 0 && self.args.panic {
            self.panic_pill();
        }
        if self.frames == 2 && self.args.click {
            self.tempo_click();
        }
        if self.frames == 4 {
            if let Some(typed) = self.args.tempo_type.clone() {
                self.tempo_edit(0.0, 0.0);
                if let Some(e) = self.edit.as_mut() {
                    e.field = text::Field::with(&typed);
                    e.field.focused = true;
                }
            }
        }
        self.take_show_tempo();
        self.play_panels();
        let help = self.help_now();
        let preset_line = self.preset_line();
        self.show.help.clone_from(&help);
        let enabled: Vec<(bar::Action, bool)> = bar::MENU.iter().map(|(a, ..)| (*a, self.menu_why_not(*a).is_none())).collect();
        let band = (self.workspace == show::Workspace::Show).then(|| self.show.band_cursor(self.cursor.0, self.cursor.1)).flatten();
        let grab = frame::cursor(&self.frame.frame, self.cursor).or(band).unwrap_or_else(|| panels::cursor(&self.panels().floats, self.cursor));
        self.take_messages();
        let words = self.state_words();
        let account = self.auth.state();
        let (path, crumb_base) = self.tab_path();
        let pills = self.tab_pills();
        let home_page = (self.workspace == show::Workspace::Home).then(|| {
            let (recent, library, deck) = self.home_lines();
            let why = library.is_empty().then(|| self.book_why().unwrap_or_else(|| "the library comes with a toon: sign in, or open a character of one".into()));
            (recent, library, deck, why)
        });
        let fps = self.fps();
        let look = self.looking();
        self.follow_nodes();
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
            if let Some((dx, dy)) = self.args.pan {
                let h = gpu.size().1 as f32 / window.scale_factor() as f32;
                self.stage.space.camera.drag(dx * 0.4, dy * 0.2);
                self.stage.space.camera.pan(dx, dy, h);
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
        // a module loads after the first frames: its groups show then
        // a node of a module selects once its module has loaded (the first frame may not have it)
        if let Some(id) = self.args.select.clone() {
            if let Some(n) = self.stage.node_ids.iter().position(|x| *x == id) {
                self.sel = inspector::Sel::Node(n);
                self.args.select = None;
            }
        }
        if let (Some(key), inspector::Sel::Node(n)) = (self.args.page.clone(), &self.sel) {
            let n = *n;
            if let Some(p) = self.stage.kinds.get(n).and_then(|k| inspector::Paging::at(n, k.pages(), &key)) {
                self.paging = p;
                self.args.page = None;
            }
        }
        // `--open AREA,AREA/PAGE`: one at a time, each once its cluster shows (a page shows inside its open area)
        if let Some(open) = self.args.open.clone() {
            let (group, rest) = open.split_once(',').map_or((open.as_str(), None), |(g, r)| (g, Some(r.to_string())));
            let nodes: Vec<usize> = self.stage.space.being.clusters().iter().filter(|c| c.name == group).map(|c| c.node).collect();
            if !nodes.is_empty() {
                self.args.open = rest;
            }
            for n in nodes {
                self.stage.toggle_group(n, group);
            }
        }
        if self.frames == 20 && self.args.camera {
            self.stage.space.look_through_scene();
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
        self.tools.shots = self.stage.space.dop.camera.shots.iter().map(|s| s.id.clone()).collect();
        let shots = &self.tools.shots;
        for (p, ws) in [(&mut self.build, show::Workspace::Stage), (&mut self.module.panels, show::Workspace::Module), (&mut self.show.show_panels, show::Workspace::Show), (&mut self.home_panels, show::Workspace::Home)] {
            p.settings = open;
            p.tools = Some(tools::size(t, ws, shots));
        }
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        if let Some(g) = self.gpu_time.as_mut() {
            g.begin(&mut enc);
        }
        // the stage at its viewport's size in pixels
        let [r, g, b, _] = t.color.stage;
        self.stage.space.set_ground([r, g, b]);
        let show_bg = matches!(self.workspace, show::Workspace::Show).then(|| self.show.sub.conductor().show().background.clone()).flatten();
        self.stage.wear(show_bg.as_ref());
        self.stage.ask_pixel(self.colour.dropper.map(|_| ((self.cursor.0 * scale) as u32, (self.cursor.1 * scale) as u32)));
        let tempo_now = self.stage.controls.tempo;
        let stage_px = ((stage_w * scale) as u32, (stage_h * scale) as u32);
        let (stage_view, fresh) = self.stage.frame(&gpu.device, &gpu.queue, &mut enc, stage_px, dt);
        if self.workspace == show::Workspace::Home {
            if self.idle.is_none() {
                self.idle = idle::Idle::new(&gpu.device, &gpu.queue, stage_px).map_err(|e| tracing::warn!("the idle scene: {e:#}")).ok();
            }
            if let Some(i) = self.idle.as_mut() {
                i.frame(&gpu.device, &gpu.queue, &mut enc, (stage_view, stage_px), dt, &tempo_now);
            }
        }
        if fresh {
            ui.set_stage(&gpu.device, stage_view);
        }
        // the UI: everything in logical pixels, the pass scales
        let t0 = Instant::now();
        ui.begin();
        // a medium is shown on its own: no stage behind it
        if self.workspace != show::Workspace::Media {
            ui.stage(stage_x, top, stage_w, stage_h);
        }
        self.show.frame();
        self.panel.hits.clear();
        self.marks.clear();
        self.clusters.clear();
        let (pad, font, row) = (t.size.pad, t.size.font, t.size.row);
        // the toolbar: the tabs, the front one's path inside it (the state words are the status bar's)
        let build = self.workspace == show::Workspace::Stage;
        // a Scene or Asset tab's Edit · Play: the stage's, or a medium's
        let edits = build || (self.workspace == show::Workspace::Media && self.media.open.as_ref().is_some_and(|o| o.length().is_some())) || (self.workspace == show::Workspace::Module && self.module.open.is_some());
        let playing = match self.workspace {
            show::Workspace::Media => self.media.open.as_ref().is_some_and(|o| o.playing),
            show::Workspace::Module => self.module.heard,
            _ => self.play.playing(),
        };
        let tab_pills: Vec<bar::TabPill> = pills.iter().map(|(name, icon, dirty, mark)| bar::TabPill { icon, name, dirty: *dirty, mark: *mark }).collect();
        let shows = matches!(self.workspace, show::Workspace::Show);
        // a Scene or Asset tab's Edit · Play; a Show tab's Go live, or End and Lock
        let (live_words, live_on, live_presses) = live_tabs::switch(self.show.live(), self.show.locked);
        // Play on the stage: Keep takes what it changed into the working state
        let keeps = build && playing;
        let slot = if shows { show::author::view_width(ui, t) } else { 0.0 };
        // the signing-in icon breathes, unless motion is reduced
        let pulse = if self.settings.motion.allows(true) { 0.5 + 0.5 * (self.started.elapsed().as_secs_f32() * std::f32::consts::TAU / 1.6).cos() } else { 1.0 };
        let spec = bar::Bar {
            tabs: &tab_pills,
            front: self.tabs.front,
            path: &path,
            modes: if edits { Some(bar::Modes::Switch { playing, keep: keeps }) } else { shows.then_some(bar::Modes::Pills(live_words, live_on)) },
            play: None,
            beat: self.stage.beat,
            meter: self.stage.pill.map(|p| (p.beat, p.click)),
            clock: match self.stage.pill {
                _ if self.stage.live || self.stage.held() => bar::Clock::Live,
                Some(p) if p.external => bar::Clock::Ext { locked: p.locked },
                _ => bar::Clock::Int,
            },
            account,
            pulse,
            pointer: self.cursor,
            cpu: self.cpu.as_ref().and_then(|c| c.reading()),
            gpu: self.gpu_load.reading(),
            khz: self.stage.khz().map(|k| k as f32),
            fps,
            slot,
            panicked: self.show.sub.panicked(),
            window: self.own_frame.map(|s| (s, window.is_maximized())),
        };
        let placed = bar::draw(ui, t, wf, &spec);
        if self.own_frame.is_some() {
            self.frame.frame = deck_kit::editor::window::Frame { size: (wf, hf), maximised: window.is_maximized(), band: bar_bottom, buttons: placed.window.clone(), solid: placed.solid.clone() };
        }
        self.panel.hits.push((placed.menu, Hit::Menu));
        self.panel.hits.push((placed.panic, Hit::Panic));
        self.panel.hits.push((placed.account, Hit::Account));
        self.panel.hits.push((placed.tempo, Hit::Tempo));
        self.panel.hits.push((placed.bpm, Hit::TempoBpm));
        self.panel.hits.push((placed.source, Hit::TempoSource));
        if placed.click.w > 0.0 {
            self.panel.hits.push((placed.click, Hit::TempoClick));
        }
        if let Some(r) = placed.meters {
            self.panel.hits.push((r, Hit::Meters));
        }
        self.panel.hits.push((placed.home, Hit::Home));
        let mode_hits: Vec<Hit> = if edits { [Hit::Switch, Hit::Keep].into_iter().take(1 + usize::from(keeps)).collect() } else { live_presses.iter().map(|p| Hit::Live(*p)).collect() };
        for (r, h) in placed.modes.iter().zip(mode_hits) {
            self.panel.hits.push((*r, h));
        }
        // a step of the path goes back to it (Show's last one is the row: it goes nowhere)
        for (i, r) in placed.path.iter().enumerate() {
            if build || i + 1 < path.len() {
                self.panel.hits.push((*r, Hit::Crumb(i + crumb_base)));
            }
        }
        // a tab's close before the tab under it
        for &(i, r, close) in &placed.tabs {
            self.panel.hits.push((close, Hit::TabClose(i)));
            self.panel.hits.push((r, Hit::TabFront(i)));
        }
        if let Some(r) = placed.list {
            self.panel.hits.push((r, Hit::TabList));
        }
        self.show.view_slot = placed.slot;
        'build: {
        if !build {
            self.show.hits.clear();
            self.show.flow = self.settings.motion.allows(self.settings.motion.signal_flow).then(|| self.started.elapsed().as_secs_f32());
            match self.workspace {
                show::Workspace::Show => show::author::draw(ui, t, &mut self.show, wf, hf, self.cursor),
                show::Workspace::Module => module::draw::draw(ui, t, &mut self.module, (&self.stage, &self.settings.units), (wf, hf), self.cursor, help.as_ref()),
                show::Workspace::Home => {
                    self.home_panels.layout(ui, t, ((wf, hf), bar_bottom), &[], &[]);
                    if let Some((recent, library, deck, why)) = &home_page {
                        let page = home::Home { recent, library, deck, library_why: why.as_deref() };
                        for (r, p) in home::draw(ui, t, (wf, hf), bar_bottom, &page, self.cursor) {
                            self.panel.hits.push((r, Hit::HomePress(p)));
                        }
                    }
                }
                show::Workspace::Media => {
                    self.home_panels.layout(ui, t, ((wf, hf), bar_bottom), &[], &[]);
                    let shown = self.media.frame(&gpu.device, &gpu.queue, dt).map(|(view, fresh)| {
                        ui.set_card(&gpu.device, media::CARD, view, fresh);
                    });
                    if let (Some(r), Some(())) = (media_draw::draw(ui, t, &mut self.media, (wf, hf), bar_bottom, &media_draw::Reading { prefs: &self.settings.units, tempo: self.stage.beat.map(|(b, _)| (b, self.stage.per_bar)) }), shown) {
                        ui.card(media::CARD, r.x, r.y, r.w, r.h);
                    }
                }
                show::Workspace::Stage => {}
            }
            break 'build;
        }
        self.grid.follow(self.scene.as_ref(), &self.stage, self.args.grid.as_deref());
        self.grid.draw_stage(ui, t, &self.stage.space, Rect { x: stage_x, y: top, w: stage_w, h: stage_h }, scale);
        if let Some(view) = self.scene.as_mut() {
            // the `video` nodes with a file: their live frames on their cards
            let doc = &view.edit.doc;
            let nodes: Vec<(String, String, deck_core::trim::Trim)> = view.edit.videos().into_iter().filter(|&v| doc.nodes[v].kind == "video").filter_map(|v| Some((doc.nodes[v].id.clone(), view.edit.video_src(v)?.to_string(), view.edit.video_trim(v)))).collect();
            let index = |id: &str| doc.nodes.iter().position(|n| n.id == id);
            let mut posters = Vec::new();
            for (id, f) in self.posters.frame(&gpu.device, &gpu.queue, &nodes) {
                let Some(v) = index(&id) else { continue };
                posters.push((v, match f {
                    posters::Frame::Live(tex, fresh) => {
                        ui.set_card(&gpu.device, &id, tex, fresh);
                        None
                    }
                    posters::Frame::Not(why) => Some(why.to_string()),
                }));
            }
            ui.keep_cards(&nodes.iter().map(|(id, _, _)| id.as_str()).collect::<Vec<_>>());
            view.posters = posters;
            let carry = self.carry.as_ref().and_then(|c| c.at);
            scene_stage::draw(ui, t, view, &self.stage.space, Rect { x: stage_x, y: top, w: stage_w, h: stage_h }, scale, carry);
            for &(r, v) in &view.poster_rects {
                ui.card(&view.edit.doc.nodes[v].id, r.x, r.y, r.w, r.h);
            }
        } else {
            ui.keep_cards(&[]);
        }
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
        let dop_h = self.scene.as_ref().map_or(t.size.row * 2.0, |v| dop::height(t, v));
        let shown = [("tree", t.size.tree, None), (library::PANEL, library::WIDTH, None), ("inspector", t.size.inspector, None), ("audio", 320.0, None), ("info", 260.0, Some(info_h)), ("mixer", mixer_w, Some(mixer_h)), ("dop", dop::WIDTH, Some(dop_h))];
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
            let mut lit_nodes: Vec<String> = Vec::new();
            for (index, q) in self.stage.ports.iter().enumerate().filter(|(i, _)| self.stage.visible[*i]) {
                let Some(node) = self.stage.node_ids.iter().position(|n| *n == q.node) else { continue };
                let Some(world) = being.port_world(node, &q.name, q.input) else { continue };
                let Some((sx, sy)) = self.stage.space.to_pixel(world) else { continue };
                let (mx, my) = (sx / scale + stage_x, sy / scale + top);
                if mx > stage_x + stage_w || mx < stage_x || my < top || my > top + stage_h || covered(&self.build.floats, mx, my) {
                    continue;
                }
                self.marks.push(Mark { port: index, x: mx, y: my });
                // an input from outside reached it: lit, with what came (Edit takes none in)
                if let Some(a) = self.play.lit(&q.address).filter(|_| q.input).or_else(|| self.play.lit(&q.node).filter(|_| !lit_nodes.contains(&q.node))) {
                    lit_nodes.push(q.node.clone());
                    let fade = 1.0 - a.at.elapsed().as_secs_f32() / play::LIT.as_secs_f32();
                    let c = t.color.accent;
                    ui.ring(mx, my, 9.0 + 4.0 * (1.0 - fade), [c[0], c[1], c[2], c[3] * fade.max(0.2)]);
                    controls::chip(ui, t, (mx + 12.0, my - 26.0), wf, &a.words, false);
                }
            }
            for c in being.clusters() {
                let Some((sx, sy)) = self.stage.space.to_pixel(being.cluster_world(c)) else { continue };
                let (cx, cy) = (sx / scale + stage_x, sy / scale + top);
                if cx > stage_x + stage_w || cx < stage_x || cy < top || cy > top + stage_h || covered(&self.build.floats, cx, cy) {
                    continue;
                }
                self.clusters.push((cx, cy, c.node, c.name, c.count));
                if c.open {
                    ui.ring(cx, cy, 8.0, t.color.bright);
                } else {
                    ui.disc(cx, cy, 8.0, t.color.bright);
                }
                let small = font * 0.75;
                let label = format!("{} {}", c.label(), c.count);
                let w = ui.advance(small) * label.chars().count() as f32;
                let left = self.stage.space.to_pixel(being.points[c.point].pos).is_some_and(|(px, _)| sx < px);
                ui.text(if left { cx - w - 12.0 } else { cx + 12.0 }, cy - small * 0.6, small, &label, t.color.bright);
            }
            let under = nearest(&self.marks, self.cursor.0, self.cursor.1);
            let small = font * 0.75;
            let mut labels: Vec<Rect> = Vec::new();
            let mut by_cursor: Vec<usize> = (0..self.marks.len()).collect();
            by_cursor.sort_by(|&a, &b| self.marks[a].distance(self.cursor).total_cmp(&self.marks[b].distance(self.cursor)));
            for k in by_cursor {
                let Mark { port: index, x: mx, y: my } = self.marks[k];
                let q = &self.stage.ports[index];
                // a dense ring seen from afar: the dots shrink rather than touch
                let gap = self.marks.iter().enumerate().filter(|(j, _)| *j != k).map(|(_, m)| m.distance((mx, my))).fold(f32::MAX, f32::min);
                let dot = (gap * 0.4).clamp(2.0, 6.0);
                let near = self.marks[k].distance(self.cursor) < 110.0;
                let hover = under == Some(index);
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
                    ui.ring(mx, my, dot, c);
                } else {
                    ui.disc(mx, my, dot, c);
                }
                // the labels nearest the pointer first, none over another
                if near {
                    let w = ui.advance(small) * q.name.chars().count() as f32;
                    let lx = if q.input { mx - w - 10.0 } else { mx + 10.0 };
                    let r = Rect { x: lx, y: my - small * 0.6, w, h: small * 1.2 };
                    if !labels.iter().any(|o| o.x < r.x + r.w && r.x < o.x + o.w && o.y < r.y + r.h && r.y < o.y + o.h) {
                        ui.text(lx, my - small * 0.6, small, &q.name, t.color.bright);
                        labels.push(r);
                    }
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
                let flow = self.settings.motion.allows(self.settings.motion.signal_flow).then(|| self.started.elapsed().as_secs_f32());
                show::band::wire(ui, t, (from, to), (t.size.rem(1.0), flow), c);
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
            let label = match self.stage.space.viewing() {
                _ if !self.stage.space.camera.looking() => None,
                deck_space::space::dop::Viewing::Shot(id) => Some(format!("SHOT {}", id.to_uppercase())),
                _ => Some("PRIMARY CAMERA".to_string()),
            };
            if let Some(label) = label {
                let small = font * 0.75;
                let w = ui.advance(small) * label.chars().count() as f32;
                ui.text(gx - w * 0.5, gy + R + 12.0 + small * 0.4, small, &label, t.color.accent);
            }
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
            self.build.close(ui, o, &mut self.panel.hits);
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
            if !self.play.playing() {
                for (r, h) in controls::pills(ui, t, pick.x + pick.w + pad, y, row - 4.0, &["Add"], usize::MAX).into_iter().zip([Hit::Add]) {
                    self.panel.hits.push((r, h));
                }
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
                        for (r, field) in scene::inspect(ui, t, area, view, (hot, !self.play.playing())) {
                            let hit = match field {
                                scene::Field::Knob(i) => Hit::SceneKnob(i),
                                scene::Field::Clear(i) => Hit::SceneClear(i),
                                scene::Field::Into => Hit::SceneInto,
                                scene::Field::Preset(i) => Hit::ScenePreset(i),
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
                        let line = &preset_line;
                        for (r, field) in inspector::draw(ui, t, area, (&self.stage, line), &self.sel, (self.paging, !self.play.playing()), hot) {
                            let hit = match field {
                                inspector::Field::Knob(i) => Hit::Knob(i),
                                inspector::Field::Param(n, p) => Hit::Param(n, p),
                                inspector::Field::Address(n) => Hit::Address(n),
                                inspector::Field::Area(n, a) => Hit::Area(n, a),
                                inspector::Field::Remove => Hit::Remove,
                                inspector::Field::Arm => Hit::Arm,
                                inspector::Field::Background => Hit::Background,
                                inspector::Field::BackgroundColour => Hit::BackgroundColour,
                                inspector::Field::PresetLoad(i) => Hit::PresetLoad(i),
                                inspector::Field::PresetSaveAs => Hit::PresetSaveAs,
                                inspector::Field::PresetSave => Hit::PresetSave,
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
            self.build.close(ui, o, &mut self.panel.hits);
        }
        // the Audio panel: the settings, the map and the tones
        if let Some(o) = self.build.open(ui, t, "audio", self.cursor, self.panel.hits.len()) {
            for (r, f) in routing::draw(ui, t, o.area, &mut self.routing, &self.stage, self.cursor) {
                self.panel.hits.push((r, Hit::Routing(f)));
            }
            self.build.close(ui, o, &mut self.panel.hits);
        }
        if let Some(o) = self.build.open(ui, t, "info", self.cursor, self.panel.hits.len()) {
            info::body(ui, t, (o.area.x, o.area.y, o.area.w), help.as_ref());
            self.build.close(ui, o, &mut self.panel.hits);
        }
        if let Some(o) = self.build.open(ui, t, "mixer", self.cursor, self.panel.hits.len()) {
            for (r, part) in mixer::draw(ui, t, o.area, &self.stage, &view) {
                let hit = match part {
                    mixer::Part::Tab(depth) => Hit::MixerTab(depth),
                    mixer::Part::Select(target) => Hit::StripSelect(target),
                    mixer::Part::GainText(target) => Hit::GainText(target),
                    mixer::Part::Fader(i, f) => Hit::Fader(i, f),
                    mixer::Part::Mute(i) => Hit::Mute(i),
                    mixer::Part::Solo(i) => Hit::Solo(i),
                    mixer::Part::Pan(i) => Hit::Balance(i),
                    mixer::Part::Peak(p) => Hit::Peak(p),
                };
                self.panel.hits.push((r, hit));
            }
            self.build.close(ui, o, &mut self.panel.hits);
        }
        if let Some(v) = self.scene.as_mut() {
            v.carry = self.carry.as_ref().and_then(|c| c.at);
            v.dop.floor = self.stage.space.floor_point().y;
            v.dop.aims = self.stage.space.aim_choices();
        }
        if let Some(o) = self.build.open(ui, t, "dop", self.cursor, self.panel.hits.len()) {
            match self.scene.as_ref().filter(|v| v.inside.is_none()) {
                Some(view) => {
                    let dop = &self.stage.space.dop;
                    let playing = dop.clock.and_then(|c| dop.director.playing(&c)).map(|(c, p)| (c.spec.kind.word().to_string(), c.shot.clone(), p));
                    let hot = match self.panel.drag {
                        Some(Drag::Dop(n, ..)) => Some(n),
                        _ => None,
                    };
                    let us = dop::Units::of(&self.settings.units);
                    for (r, f) in dop::draw(ui, t, o.area, view, playing, us, hot) {
                        self.panel.hits.push((r, Hit::Dop(f)));
                    }
                }
                None => {
                    ui.text(o.area.x, o.area.y + 4.0, t.size.small, "open a scene: its camera, shots and moves", t.color.muted);
                }
            }
            self.build.close(ui, o, &mut self.panel.hits);
        }
        // a panel's ground keeps the press from the stage under it
        for r in self.build.grounds(&panels::BUILD.map(|s| s.id)) {
            self.panel.hits.push((r, Hit::Ground));
        }
        // over the panels: a cable in the hand, where it would land
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
        // over everything else: what is being typed, the typeahead
        if let Some(e) = &self.edit {
            draw_edit(ui, t, e, font);
        }
        self.grid.popover(ui, t, (wf, hf), dop::Units::of(&self.settings.units).distance);
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
                show::Workspace::Stage | show::Workspace::Module => {
                    let panels = if build { &mut self.build } else { &mut self.module.panels };
                    match panels.open(ui, t, library::PANEL, cursor, 0) {
                        Some(o) => {
                            self.library.draw(ui, t, o.area, cursor, None);
                            panels.close(ui, o, &mut self.library.browser.hits);
                        }
                        None => self.library.browser.hits.clear(),
                    }
                }
                show::Workspace::Home | show::Workspace::Media => self.library.browser.hits.clear(),
            }
        }
        // the card in the hand, over every workspace's panels
        if let Some(c) = &self.carry {
            c.draw(ui, t, wf);
        }
        // the tools pill, over the workspace's panels
        {
            let cursor = self.cursor;
            let ws = self.workspace;
            let state = tools::State { tool: self.tools.tool, two_d: self.show.two_d, grid: self.grid.settings().grid, look };
            let panels = match ws {
                show::Workspace::Stage => &mut self.build,
                show::Workspace::Module => &mut self.module.panels,
                show::Workspace::Show => &mut self.show.show_panels,
                show::Workspace::Home | show::Workspace::Media => &mut self.home_panels,
            };
            let mut places = Vec::new();
            if let Some(o) = panels.open(ui, t, "tools", cursor, 0) {
                places = self.tools.draw(ui, t, o.area, ws, &state, cursor);
                panels.close(ui, o, &mut places);
            }
            self.tools.shown(places);
            self.grid.anchor(if build { self.tools.grid_at } else { None });
        }
        // the settings, over the workspace's panels
        self.settings_hits.clear();
        {
            let cursor = self.cursor;
            let s = self.settings.clone();
            let ws = match self.workspace {
                show::Workspace::Stage => &mut self.build,
                show::Workspace::Module => &mut self.module.panels,
                show::Workspace::Show => &mut self.show.show_panels,
                show::Workspace::Home | show::Workspace::Media => &mut self.home_panels,
            };
            if let Some(o) = ws.open(ui, t, panels::SETTINGS, cursor, 0) {
                let mut places = settings::draw(ui, t, o.area, &s);
                ws.close(ui, o, &mut places);
                self.settings_hits = places;
            }
        }
        // the Log panel and the status bar, in every workspace
        {
            let typing = self.edit.as_ref().filter(|e| e.what == Editing::LogFind).map(|e| e.field.text.clone());
            let ws = match self.workspace {
                show::Workspace::Stage => &mut self.build,
                show::Workspace::Module => &mut self.module.panels,
                show::Workspace::Show => &mut self.show.show_panels,
                show::Workspace::Home | show::Workspace::Media => &mut self.home_panels,
            };
            (self.log_hits, self.opens_log) = status::draw(ui, t, (wf, hf), &self.log, ws, (typing.as_deref(), &words), self.cursor);
        }
        self.tools.draw_menu(ui, t, (wf, hf), self.cursor);
        let value = self.colour.target().and_then(|target| colour::value(target, &self.stage, self.scene.as_ref(), &self.show));
        self.colour.draw(ui, t, (wf, hf), value, self.cursor);
        self.log.draw_ring(ui, t);
        {
            let ws = match self.workspace {
                show::Workspace::Stage => &mut self.build,
                show::Workspace::Module => &mut self.module.panels,
                show::Workspace::Show => &mut self.show.show_panels,
                show::Workspace::Home | show::Workspace::Media => &mut self.home_panels,
            };
            self.history_hits = history_panel::draw_all(ui, t, (wf, hf), ws, &self.history, self.cursor);
        }
        if let Some(p) = self.scene_prompt.as_mut() {
            p.draw(ui, t, (wf, hf));
        }
        // the main menu over everything, the workspace's panels under its lines
        self.menu_lines.clear();
        if self.main_menu {
            let listed = match self.workspace {
                show::Workspace::Stage => &self.build,
                show::Workspace::Module => &self.module.panels,
                show::Workspace::Show => &self.show.show_panels,
                show::Workspace::Home | show::Workspace::Media => &self.home_panels,
            }
            .listed();
            self.menu_lines = bar::menu(ui, t, placed.menu, &|a| enabled.iter().any(|(b, on)| *b == a && *on), &listed, self.cursor);
        }
        self.tab_lines.clear();
        if self.tab_list {
            if let Some(at) = placed.list {
                self.tab_lines = bar::tab_list(ui, t, at, &tab_pills, self.tabs.front, self.cursor);
            }
        }
        // the toolbar's BPM is typed on in every workspace; the build stage draws the typing in its own pass
        if let (false, Some(e)) = (build, self.edit.as_ref().filter(|e| e.what == Editing::Tempo)) {
            draw_edit(ui, t, e, font);
        }
        window.set_cursor(grab);
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
        gpu.queue.submit([enc.finish()]);
        gpu.queue.present(frame);
        self.latency.presented();
        if let Some(ms) = self.gpu_time.as_mut().and_then(|g| g.after_submit(&gpu.device)) {
            let frame_s = f64::from(self.frame_ms.last().copied().unwrap_or(16.7).max(1.0)) / 1000.0;
            if let Some((share, span)) = self.gpu_busy.add(f64::from(ms) / 1000.0, frame_s, 1) {
                self.gpu_load.push(share, span);
            }
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
        // the main menu's Quit to stage, a check run's F9: as F9 – here, before the next frame is prepared (a prepared
        // frame dropped unpresented panics in wgpu)
        if std::mem::take(&mut self.to_deck) || self.args.f9_at.is_some_and(|n| self.frames >= n) {
            self.args.f9_at = None;
            if self.hand_off(el) {
                return;
            }
        }
        // the wait for the display comes now, so the pointer events that arrive meanwhile reach the next frame
        if let Some(g) = self.gpu.as_mut() {
            let asked = Instant::now();
            if let Err(e) = g.prepare() {
                tracing::error!("render: {e:#}");
            }
            self.latency.acquired(asked, g.config.desired_maximum_frame_latency as usize);
        }
        window.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let mut attrs = host::attributes("SCUTOONS editor", !self.args.fullscreen);
        if self.own_frame.is_some() {
            attrs = frame::undecorated(attrs);
        }
        let window = match el.create_window(attrs).map(Arc::new) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("no window: {e}");
                self.exit = 1;
                el.exit();
                return;
            }
        };
        match Gpu::with_features(window.clone(), gpu_time::FEATURES).and_then(|gpu| ui::Ui::new(&gpu.device, &gpu.queue, gpu.format(), self.tokens.tokens.size.font).map(|ui| (gpu, ui))) {
            Ok((mut gpu, ui)) => {
                gpu.set_frame_latency(1);
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
            WindowEvent::DroppedFile(path) => self.library_file(&path),
            WindowEvent::Resized(s) => {
                if let Some(g) = self.gpu.as_mut() {
                    g.resize(s.width, s.height);
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {}
            WindowEvent::ModifiersChanged(m) => {
                self.mods = m.state();
                self.show.nodes.shift = self.mods.shift_key();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.latency.event(self.panels().floats.dragging());
                let s = self.scale();
                self.pointer(position.x as f32 / s, position.y as f32 / s);
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                if !self.frame_press() {
                    self.press();
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: button @ (MouseButton::Middle | MouseButton::Right), .. } => self.press_view(button),
            WindowEvent::MouseInput { state: ElementState::Released, .. } => {
                if let Some(w) = self.window.clone() {
                    if self.frame.release(&w, el, self.cursor) {
                        return;
                    }
                }
                self.show.release();
                self.module.release();
                self.media_release();
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
                let fingers = matches!(delta, MouseScrollDelta::PixelDelta(_));
                if self.workspace == show::Workspace::Show && self.show.nodes_wheel(self.cursor, (across, steps), fingers, self.mods.control_key()) {
                    return;
                }
                if self.workspace != show::Workspace::Stage && self.show.band_wheel(self.cursor.0, self.cursor.1, band) {
                    return;
                }
                if self.workspace != show::Workspace::Stage && self.show.wheel(self.cursor.0, self.cursor.1, steps) {
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
                    // a touchpad's two fingers pan, its pinch (Ctrl) and a wheel's notches zoom
                    Hit::Stage | Hit::Port(_) | Hit::Axis(_) => match delta {
                        MouseScrollDelta::PixelDelta(p) if !self.mods.control_key() => {
                            let h = self.view_h();
                            self.stage.space.camera.pan(p.x as f32 / self.scale(), p.y as f32 / self.scale(), h);
                        }
                        _ => self.stage.space.camera.zoom(steps),
                    },
                    _ => {}
                }
            }
            WindowEvent::PinchGesture { delta, .. } if matches!(self.hit(self.cursor.0, self.cursor.1), Hit::Stage | Hit::Port(_) | Hit::Axis(_)) => {
                let steps = (1.0 + delta as f32).max(0.1).ln() / -deck_space::gesture::ZOOM_STEP.ln();
                self.stage.space.camera.zoom(steps);
            }
            // Shift+Esc panics from anywhere (Esc alone leaves the editor); only the pill resumes
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) && self.mods.shift_key() =>
            {
                if !self.show.sub.panicked() {
                    self.panic_pill();
                }
            }
            WindowEvent::KeyboardInput { event, .. } if self.history.asking.is_some() => self.history_prompt_key(&event),
            WindowEvent::KeyboardInput { event, .. } if self.scene_prompt.is_some() => self.scene_prompt_key(&event),
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
            WindowEvent::KeyboardInput { event, .. } if self.workspace == show::Workspace::Show && event.logical_key == Key::Named(NamedKey::Space) && !self.panel.field.focused && self.show.nodes_space(event.state == ElementState::Pressed) => {}
            // #471: the flash list's typed filter has the keys while it is focused
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && self.workspace == show::Workspace::Show && self.show.key(&event.logical_key, self.mods.control_key() || self.mods.super_key()) => {}
            // #609: F9 – the stage instead of the editor, on this session and show (deck-native handoff.rs)
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::F9) => {
                self.to_deck = true;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) && !self.panel.field.focused {
                    if self.wire && !self.stage.path.is_empty() {
                        self.up_to(self.stage.path.len() - 1);
                    } else if self.scene.as_ref().is_some_and(|v| v.inside.is_some()) {
                        self.scene_up();
                    } else if self.looking() != tools::Look::Free || self.stage.space.dop.director.active() {
                        self.look_to(tools::Look::Free);
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
                } else if event.state == ElementState::Pressed && matches!(&event.logical_key, Key::Character(c) if c.eq_ignore_ascii_case("z")) && self.mods.control_key() {
                    self.history_key(self.mods.shift_key());
                } else if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Delete) && self.workspace == show::Workspace::Stage {
                    self.remove_selected();
                } else if let Some(tool) = (event.state == ElementState::Pressed && self.workspace == show::Workspace::Stage && !self.mods.control_key()).then(|| if let Key::Character(c) = &event.logical_key { tools::tool_key(c) } else { None }).flatten() {
                    self.tools.tool = tool;
                } else if let Some(l) = (event.state == ElementState::Pressed && !self.mods.control_key()).then(|| tools::look_key(&event.logical_key, self.stage.space.dop.camera.shots.len())).flatten() {
                    self.look_to(l);
                } else if event.state == ElementState::Pressed && event.logical_key == Key::Character("f".into()) {
                    self.stage.space.frame_all();
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

/// the value being typed, over its field
fn draw_edit(ui: &mut ui::Ui, t: &deck_kit::base::tokens::Tokens, e: &ValueEdit, px: f32) {
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

/// a tempo source in words
fn pick_name(s: deck_core::tempo::Source) -> &'static str {
    match s {
        deck_core::tempo::Source::Midi => "the MIDI clock in",
        deck_core::tempo::Source::Link => "Ableton Link",
        deck_core::tempo::Source::Relay => "the show clock",
        deck_core::tempo::Source::Osc => "OSC",
        deck_core::tempo::Source::Internal => "the deck's own clock",
    }
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
