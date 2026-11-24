//! The editor's floating panels (deck-kit `floats`): each workspace's set over the full-window stage, the size each
//! content asks for this frame (its own when it knows it, else what it drew the frame before), and where the viewer left
//! them – a per-viewer convenience in the editor's cache, one map per workspace. A panel is a view: hidden, a panel, on
//! the stage (deck-ui `stage`: one of the four layouts' cells) or full screen; the panels are painted in their order,
//! the last touched on top.
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::floats::{self, Chrome, Corner, Floats, Metrics, Place, Shown, Spec, View};
use deck_ui::stage::{StageState, ViewState};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

const fn spec(id: &'static str, name: &'static str, icon: &'static str, about: &'static str, place: Place) -> Spec {
    Spec { id, name, icon, about, place, essential: false, inline: false }
}

const TL: Corner = Corner::TopLeft;
const TR: Corner = Corner::TopRight;
const BL: Corner = Corner::BottomLeft;
const BR: Corner = Corner::BottomRight;

// the panels every view has alike, defined once
const LOG: Spec = spec("log", "Log", "entry", "Every message, newest first, and the technical log: filter, find, copy all, save as a file.", Place::at(BR, 0.0, 0.0).closed());
const HISTORY: Spec = spec(crate::history_panel::PANEL, "History", "clock", "The open rig's, scene's, show's or module's saved steps: who, when, what changed; revert one or go back to a point, always asked first.", Place::at(TR, 0.0, 0.0).closed());
const SETTINGS_PANEL: Spec = spec("settings", "Settings", "settings", "Day or night, motion & feedback, sound and the UI size: this machine's, remembered.", Place::at(TR, 0.0, 0.0).closed());

// The order is the order of growth: an earlier panel leaves a later one the room it asks for, so the ones whose content
// scrolls well come first and the ones of a fixed size last.
/// the stage editor's: a Scene tab and a character's Asset tab
pub const BUILD: [Spec; 11] = [
    spec("tree", "Rig", "tree", "The rig, its tracks and their nodes. In Wire a click goes to that level.", Place::at(TL, 0.0, 0.0)),
    spec("library", "Library", "grid", "The toon's things and files by place: modules and instruments drop on the stage as a track; a click steps into a thing, a double click opens it.", Place::at(BL, 0.0, 0.0)),
    spec("inspector", "Inspector", "inspector", "What is selected: its values as scrub numbers; the rig picker and Save.", Place::at(TR, 0.0, 0.0)).essential(),
    spec("audio", "Audio", "activity", "The sound card, its inputs and outputs, the map and the test tones.", Place::at(BL, 740.0, 0.0).folded()),
    spec("info", "Info", "info", "What is under the pointer: its name and what it does.", Place::at(BL, 450.0, 0.0)),
    spec("mixer", "Mixer", "sliders", "One strip per track at the level of the tree shown; inside a track its gains.", Place::at(BR, 0.0, 0.0)),
    spec("dop", "Camera tools", "dop-camera", "The scene's camera: its lens, its shots, the moves between them and the trolley track.", Place::at(TL, 340.0, 0.0)),
    spec("tools", "Tools", "tool", "The stage's tools: the tool in hand, the view, grid & snap, framing – what this view offers.", Place::at(TR, 360.0, 0.0)).inline().essential(),
    HISTORY,
    LOG,
    SETTINGS_PANEL,
];

/// a Show tab's: authoring and running at once – the rows band is the cue matrix (a row's ▶ fires it, a cell's ▶ launches
/// it by hand); Next, Stage now and Session are the optional panels of a running show
pub const SHOW: [Spec; 14] = [
    spec("outline", "Outline", "nodes", "The conductors, the roles this one gives out and the rows of the document shown.", Place::at(TL, 0.0, 0.0)),
    spec("inspector", "Inspector", "inspector", "The row, the role or the cells selected; Load, Flash and Save.", Place::at(TR, 0.0, 0.0)).essential(),
    spec("sheet", "Sheet", "layers", "The sheet open: cues, problems, who is here, the session, the vote, a picker or the flash list.", Place::at(TL, 320.0, 0.0)).essential(),
    spec("nodes", "Spores", "node", "What comes from far, the cells of the row and what the row sends, over the stage.", Place::at(TL, 320.0, 0.0)),
    spec("library", "Library", "grid", "The toon's things and files by place: drag a scene, character, item or place onto a cell; a click steps into a thing, a double click opens it as a tab.", Place::at(BR, 0.0, 0.0)),
    spec("rows", "Rows", "list", "The rows of the show: what fires each one and every column's cell; a row's ▶ fires it, a cell's ▶ launches it.", Place::at(BL, 0.0, 0.0)).essential(),
    spec("session", "Session", "activity", "This conductor, its seats, the linked readers, the vote and the outlets.", Place::at(TL, 0.0, 0.0).closed()),
    spec("now", "Stage now", "eye", "What plays on each outlet now.", Place::at(TL, 320.0, 0.0).closed()),
    spec("info", "Info", "info", "What is under the pointer: its name and what it does.", Place::at(BR, 440.0, 0.0).folded()),
    spec("next", "Next", "play", "The suggestion with GO and Drop, else GO; the seats.", Place::at(TR, 360.0, 0.0).closed()),
    spec("tools", "Tools", "tool", "The stage's tools: the tool in hand, the view, grid & snap, framing – what this view offers.", Place::at(TR, 380.0, 0.0)).inline(),
    HISTORY,
    LOG,
    SETTINGS_PANEL,
];

pub const MODULE: [Spec; 9] = [
    spec("library", "Library", "grid", "The catalog's modules with their previews: a click opens one's working copy.", Place::at(TL, 0.0, 0.0)),
    spec("drives", "Drives", "activity", "What the module drives while it plays: each control output as a lamp, its value and its trace.", Place::at(BL, 0.0, 200.0).closed()),
    spec("module", "Module", "code", "The module open: its working copy, Open in editor, Build, its parameters.", Place::at(TR, 0.0, 0.0)).essential(),
    spec("problems", "Problems", "alert", "The last build's problems: a click opens the code there.", Place::at(BL, 0.0, 0.0)),
    spec("info", "Info", "info", "What is under the pointer: its name and what it does.", Place::at(BR, 0.0, 0.0).folded()),
    spec("tools", "Tools", "tool", "The stage's tools: the tool in hand, the view, grid & snap, framing – what this view offers.", Place::at(TR, 420.0, 0.0)).inline(),
    HISTORY,
    LOG,
    SETTINGS_PANEL,
];

/// A tab kind's Play preset before the viewer changed it: the panels that do nothing while playing close, the ones
/// that play open where they are put.
pub struct PlaySet {
    pub kind: &'static str,
    pub closes: &'static [&'static str],
    pub opens: &'static [(&'static str, Place)],
}

/// a Scene or Asset tab playing: the tree's edits, the Library's drops and History's steps are dropped on Stop; only a
/// scene has a camera to work
pub const BUILD_PLAY: [PlaySet; 2] = [
    PlaySet {
        kind: "scene",
        closes: &["tree", crate::library::PANEL, crate::history_panel::PANEL],
        opens: &[("dop", Place::at(TL, 0.0, 0.0)), ("mixer", Place::at(BL, 0.0, 0.0)), ("info", Place::at(BR, 0.0, 0.0))],
    },
    PlaySet { kind: "asset", closes: &["tree", crate::library::PANEL, crate::history_panel::PANEL, "dop"], opens: &[("mixer", Place::at(BL, 0.0, 0.0)), ("info", Place::at(BR, 0.0, 0.0))] },
];

/// a module playing: what it drives shows, the catalog and History rest
pub const MODULE_PLAY: [PlaySet; 1] = [PlaySet { kind: "module", closes: &[crate::library::PANEL, crate::history_panel::PANEL], opens: &[("drives", Place::at(BL, 0.0, 200.0))] }];

/// Play's places over Edit's `edit`: the preset's panels closed and opened, the rest where Edit has them
pub fn play_preset(edit: &BTreeMap<String, Place>, set: &PlaySet) -> BTreeMap<String, Place> {
    let mut p = edit.clone();
    for id in set.closes {
        if let Some(place) = p.get_mut(*id) {
            place.closed = true;
        }
    }
    for (id, place) in set.opens {
        p.insert((*id).to_string(), *place);
    }
    p
}

/// a running show's panels in a Show tab: closed until the show goes live (or the panel list opens one)
pub const OPTIONAL: [&str; 3] = ["session", "now", "next"];

/// Home's: only what every view has
pub const HOME: [Spec; 2] = [LOG, SETTINGS_PANEL];

/// a content height before it was ever drawn: as tall as the room lets it be, the next frame its own
const UNMEASURED: f32 = 10_000.0;
/// the library before it was drawn: a row of cards; it grows to its own height the next frame
const LIBRARY_START: f32 = 240.0;

/// a panel's content being drawn this frame (from `open` to `close`)
pub struct Open {
    pub id: &'static str,
    /// where to draw: the visible width, the content's own height, moved up by the scroll
    pub area: Rect,
    view: View,
    /// where its ground began, and its content
    start: usize,
    prims: usize,
    hits: usize,
}

/// one workspace's panels
pub struct Panels {
    pub floats: Floats,
    measured: HashMap<&'static str, f32>,
    /// the panels that said their height this frame; the others draw without an end and are measured
    known: Vec<&'static str>,
    grew: bool,
    /// the settings panel's content size (laid out with the workspace's own unless it is closed)
    pub settings: Option<(f32, f32)>,
    /// the tools pill's content size (tools.rs)
    pub tools: Option<(f32, f32)>,
    /// the workspace's Play presets, one per tab kind that plays
    play: &'static [PlaySet],
    /// Play shows for this tab kind; Edit's places kept aside until Stop
    playing: Option<(String, BTreeMap<String, Place>)>,
    /// Play's places per tab kind as the viewer left them
    plays: BTreeMap<String, BTreeMap<String, Place>>,
    /// the views on the stage and the one full screen (the editor's session holds them per workspace)
    pub stage: StageState,
    pub full: Option<String>,
    /// this frame's panels as drawn: each one's range of the draw list
    drawn: RefCell<Vec<(&'static str, usize, usize)>>,
}

/// the settings have a main menu line of their own, so the menu's panel list leaves them out
pub const SETTINGS: &str = "settings";

impl Panels {
    pub fn new(specs: &[Spec]) -> Panels {
        Panels { floats: Floats::new(specs), measured: HashMap::from([(crate::library::PANEL, LIBRARY_START)]), known: Vec::new(), grew: false, settings: None, tools: None, play: &[], playing: None, plays: BTreeMap::new(), stage: StageState::default(), full: None, drawn: RefCell::new(Vec::new()) }
    }

    /// a workspace that plays: Play shows its tab kind's own preset, remembered per tab kind
    pub fn with_play(specs: &[Spec], sets: &'static [PlaySet]) -> Panels {
        Panels { play: sets, ..Panels::new(specs) }
    }

    /// the tab kind whose Play preset shows, if one does
    pub fn play_kind(&self) -> Option<&str> {
        self.playing.as_ref().map(|(k, _)| k.as_str())
    }

    /// The panels as the mode wants them: Play for tab kind `kind` (its places as the viewer left them, else the preset
    /// over Edit's), or Edit (none, or a kind without Play) – Play's places kept for its kind, Edit's back. True when
    /// they changed.
    pub fn show_mode(&mut self, kind: Option<&str>) -> bool {
        let set = kind.and_then(|k| self.play.iter().find(|s| s.kind == k));
        if self.play_kind() == set.map(|s| s.kind) {
            return false;
        }
        if let Some((was, edit)) = self.playing.take() {
            self.plays.insert(was, self.floats.places());
            self.floats.restore(&edit);
        }
        if let Some(set) = set {
            let edit = self.floats.places();
            let play = self.plays.get(set.kind).cloned().unwrap_or_else(|| play_preset(&edit, set));
            self.floats.restore(&play);
            self.playing = Some((set.kind.to_string(), edit));
        }
        true
    }

    /// the places to keep under `name`: Edit's (also while Play shows), Play's per tab kind under `name.play.<kind>`
    pub fn remember(&self, name: &str, all: &mut Remembered) {
        let now = self.floats.places();
        let kind = self.play_kind();
        all.insert(name.to_string(), self.playing.as_ref().map_or_else(|| now.clone(), |(_, edit)| edit.clone()));
        for (k, p) in self.plays.iter().filter(|(k, _)| Some(k.as_str()) != kind) {
            all.insert(format!("{name}.play.{k}"), p.clone());
        }
        if let Some(k) = kind {
            all.insert(format!("{name}.play.{k}"), now);
        }
    }

    /// the places kept under `name`: Edit's shown, Play's per tab kind for when it plays
    pub fn recall(&mut self, name: &str, all: &Remembered) {
        if let Some(p) = all.get(name) {
            self.floats.restore(p);
        }
        let prefix = format!("{name}.play.");
        self.plays = all.iter().filter_map(|(k, p)| Some((k.strip_prefix(prefix.as_str())?.to_string(), p.clone()))).collect();
    }

    /// the panels that close, and whether each is open
    #[cfg(test)]
    pub fn listed(&self) -> Vec<(&'static str, &'static str, bool)> {
        self.floats.closable().into_iter().filter(|(s, _)| s.id != SETTINGS).map(|(s, open)| (s.id, s.name, open)).collect()
    }

    /// an open panel closes, a closed one comes back open
    #[cfg(test)]
    pub fn flip(&mut self, id: &str) {
        if self.floats.closed(id) {
            self.floats.reopen(id);
        } else {
            self.floats.close(id);
        }
    }

    /// the room for panels in a window `size` under a toolbar ending at `top`
    pub fn room(size: (f32, f32), top: f32, gap: f32) -> Rect {
        Rect { x: gap, y: top + gap, w: (size.0 - 2.0 * gap).max(1.0), h: (size.1 - top - 2.0 * gap).max(1.0) }
    }

    /// a panel's state as a view
    pub fn view_state(&self, id: &str) -> ViewState {
        if self.full.as_deref() == Some(id) {
            ViewState::Full
        } else if self.stage.has(id) {
            ViewState::Stage
        } else if self.floats.closed(id) {
            ViewState::Hidden
        } else {
            ViewState::Panel
        }
    }

    /// a panel put in a state: hidden closes it (an essential one only folds), the others open it; on the stage it takes
    /// a free cell (or the layout grows), full screen it takes the whole
    pub fn set_view(&mut self, id: &str, state: ViewState) {
        if state == ViewState::Hidden {
            if !self.floats.close(id) {
                self.floats.set_collapsed(id, true);
            }
        } else {
            self.floats.reopen(id);
        }
        self.stage = if state == ViewState::Stage { self.stage.add(id) } else { self.stage.without(id) };
        if state == ViewState::Full {
            self.full = Some(id.to_string());
        } else if self.full.as_deref() == Some(id) {
            self.full = None;
        }
    }

    /// every panel as a view: its id, name, icon and state (the ☰ Views)
    pub fn views(&self) -> Vec<(&'static str, &'static str, &'static str, ViewState)> {
        self.floats.all().into_iter().filter(|s| s.id != SETTINGS && !s.inline).map(|s| (s.id, s.name, s.icon, self.view_state(s.id))).collect()
    }

    /// the panel dragged off the stage this frame, if one was: the stage forgets it
    pub fn take_left_stage(&mut self) -> Option<&'static str> {
        let id = self.floats.left_stage()?;
        self.stage = self.stage.without(id);
        if self.full.as_deref() == Some(id) {
            self.full = None;
        }
        Some(id)
    }

    /// the panels as their order paints them, the last touched on top (after every panel of the frame is drawn)
    pub fn restack(&self, d: &mut DrawList) {
        let mut drawn = self.drawn.borrow_mut();
        let order = self.floats.paint_order();
        let mut parts: Vec<(&'static str, usize, usize)> = order.iter().filter_map(|id| drawn.iter().find(|(d, ..)| d == id).copied()).collect();
        parts.retain(|(_, a, b)| b > a);
        let ranges: Vec<(usize, usize)> = parts.iter().map(|p| (p.1, p.2)).collect();
        let before = d.prims().len();
        d.restack(&ranges);
        let mut sorted = ranges.clone();
        sorted.sort_unstable();
        // restack moved the parts in paint order to the start of their span (unless they already lay so): their places now
        if sorted != ranges && !sorted.windows(2).any(|w| w[0].1 > w[1].0) && d.prims().len() == before {
            let mut at = sorted.first().map_or(0, |r| r.0);
            for (id, a, b) in parts {
                if let Some(e) = drawn.iter_mut().find(|e| e.0 == id) {
                    *e = (id, at, at + (b - a));
                }
                at += b - a;
            }
        }
    }

    /// where panel `id` lies in the draw list this frame (after `restack`)
    pub fn drawn_range(&self, id: &str) -> Option<(usize, usize)> {
        self.drawn.borrow().iter().find(|e| e.0 == id).map(|e| (e.1, e.2))
    }

    /// the box a panel taken full fills: the window less the kit's margin, the bar's place too (RULES H15)
    pub fn full_box(size: (f32, f32), margin: f32) -> Rect {
        Rect { x: margin, y: margin, w: (size.0 - 2.0 * margin).max(1.0), h: (size.1 - 2.0 * margin).max(1.0) }
    }

    /// this frame's layout: the panels shown, each with its content's width and – when it knows it – height (else the
    /// height it drew last frame); places nothing may cover
    pub fn layout(&mut self, d: &DrawList, t: &Tokens, (size, top): ((f32, f32), f32), fixed: &[Rect], shown: &[(&'static str, f32, Option<f32>)]) {
        let m = Metrics::of(t, d);
        self.floats.measure_names(|name| d.sans_width(t.size.font, name));
        let extra = self.settings.map(|(w, h)| (SETTINGS, w, Some(h))).into_iter().chain(self.tools.map(|(w, h)| ("tools", w, Some(h)))).chain([(crate::log::PANEL, crate::log::WIDTH, None), (HISTORY.id, crate::history_panel::WIDTH, None)]);
        let shown: Vec<(&'static str, f32, Option<f32>)> = shown.iter().copied().chain(extra).collect();
        self.known = shown.iter().filter(|(_, _, h)| h.is_some()).map(|(id, ..)| *id).collect();
        let shown: Vec<Shown> = shown.iter().map(|&(id, w, h)| Shown { id, content: (w, h.unwrap_or_else(|| self.measured.get(id).copied().unwrap_or(UNMEASURED))) }).collect();
        let whole = size;
        // the status bar keeps the bottom edge
        let size = (size.0, size.1 - deck_kit::editor::status::reserve(t));
        let mut room = Self::room(size, top, m.gap);
        self.drawn.borrow_mut().clear();
        // a view taken full is a box on the overlay level, alone; the views on the stage fill their cells, a gap apart
        let full = self.full.as_deref().and_then(|f| shown.iter().find(|s| s.id == f)).map(|s| s.id);
        let (cells, shown): (Vec<(&str, Rect)>, Vec<Shown>) = match full {
            Some(f) => {
                room = Self::full_box(whole, m.gap);
                (vec![(f, room)], shown.into_iter().filter(|s| s.id == f).collect())
            }
            None => {
                let half = m.gap * 0.5;
                let cells = self.stage.slot_rects(room).into_iter().filter_map(|(_, v, r)| Some((shown.iter().find(|s| Some(s.id) == v)?.id, Rect { x: r.x + half, y: r.y + half, w: r.w - m.gap, h: r.h - m.gap }))).collect();
                (cells, shown)
            }
        };
        let cells: Vec<(&str, Rect)> = cells.into_iter().map(|(id, r)| (id, Rect { x: r.x.max(room.x), y: r.y.max(room.y), w: r.w.min(room.w), h: r.h.min(room.h) })).collect();
        self.floats.set_stage(&cells);
        self.floats.layout(room, fixed, &shown, m);
    }

    /// a panel's ground and chrome; open, the content's place (`hits` = how many places were pressed before it): a
    /// content measured draws without an end, so a list shows all of itself and the next frame knows its height
    pub fn open(&self, d: &mut DrawList, t: &Tokens, id: &'static str, pointer: (f32, f32), hits: usize) -> Option<Open> {
        let view = self.floats.view(id)?;
        let spec = self.floats.spec(id)?;
        let start = d.prims().len();
        // a panel under another shows no pointer on its chrome
        let pointer = if self.floats.covered(id, pointer.0, pointer.1) { (f32::NAN, f32::NAN) } else { pointer };
        floats::begin(d, t, spec, &view, pointer);
        if view.collapsed {
            self.drawn.borrow_mut().push((id, start, d.prims().len()));
            return None;
        }
        let mut area = self.floats.content(id)?;
        if !self.known.contains(&id) {
            area.h = UNMEASURED;
        }
        Some(Open { id, area, view, start, prims: d.prims().len(), hits })
    }

    /// whether a measured content came out taller than its panel was laid out for (once asked, false again): what
    /// was drawn after it is laid out again, so a row just added shows and answers in the same frame
    pub fn take_grew(&mut self) -> bool {
        std::mem::take(&mut self.grew)
    }

    /// the content drawn: its height is kept for the next frame, its places cut to what shows
    pub fn close<H>(&mut self, d: &mut DrawList, o: Open, hits: &mut Vec<(Rect, H)>) {
        if let Some(e) = d.extent(o.prims) {
            let h = (e.y + e.h - o.area.y).max(0.0) + 1.0;
            if !self.known.contains(&o.id) && h > self.measured.get(o.id).copied().unwrap_or(UNMEASURED) + 0.5 && o.view.over <= 0.0 {
                self.grew = true;
            }
            self.measured.insert(o.id, h);
        }
        floats::clip_hits(&o.view, hits, o.hits);
        floats::clip_hits_under(&self.floats.above(o.id), hits, o.hits);
        floats::end(d, &o.view);
        self.drawn.borrow_mut().push((o.id, o.start, d.prims().len()));
    }

    /// the panels' chrome (handles, chevrons) and grounds, so the stage under them takes no press
    pub fn grounds(&self, ids: &[&'static str]) -> Vec<Rect> {
        ids.iter().filter_map(|id| self.floats.view(id)).map(|v| v.whole).collect()
    }
}

/// the pointer over the panels: a grab on a handle or a panel in the hand, an edge's arrows on its grip or while it resizes
pub fn cursor(f: &Floats, (x, y): (f32, f32)) -> winit::window::CursorIcon {
    use winit::window::CursorIcon as C;
    let edge = f.resizing().or_else(|| if f.dragging() { None } else { f.chrome_at(x, y).and_then(|(_, c)| if let Chrome::Edge(e) = c { Some(e) } else { None }) });
    match edge.map(|e| (e.x, e.y)) {
        Some((0, _)) => C::NsResize,
        Some((_, 0)) => C::EwResize,
        Some((x, y)) if x == y => C::NwseResize,
        Some(_) => C::NeswResize,
        None if f.dragging() || matches!(f.chrome_at(x, y), Some((_, Chrome::Handle))) => C::Grab,
        None => C::Default,
    }
}

/// every workspace's places, as the cache keeps them
pub type Remembered = BTreeMap<String, BTreeMap<String, Place>>;

pub fn load(file: &Path) -> Remembered {
    std::fs::read_to_string(file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save(file: &Path, all: &Remembered) {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    if let Ok(s) = serde_json::to_string_pretty(all) {
        std::fs::write(file, s).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspaces() -> [(&'static str, &'static [Spec]); 4] {
        [("build", &BUILD), ("module", &MODULE), ("show", &SHOW), ("live", &SHOW)]
    }

    /// what each workspace's panels ask for on a typical frame (the sheet and its twin are never shown together)
    fn typical(name: &str) -> Vec<(&'static str, f32, Option<f32>)> {
        match name {
            "build" => vec![("tree", 220.0, Some(200.0)), ("library", 412.0, Some(300.0)), ("inspector", 280.0, Some(900.0)), ("audio", 300.0, Some(500.0)), ("info", 260.0, Some(100.0)), ("mixer", 560.0, Some(112.0))],
            "module" => vec![("library", 412.0, Some(400.0)), ("module", 360.0, Some(700.0)), ("problems", 700.0, Some(200.0)), ("info", 260.0, Some(100.0))],
            "show" => vec![("outline", 280.0, Some(500.0)), ("inspector", 320.0, Some(1200.0)), ("nodes", 760.0, Some(360.0)), ("library", 412.0, Some(220.0)), ("rows", 900.0, Some(160.0)), ("info", 260.0, Some(100.0))],
            // a running show with its optional panels open (the node view gives them its room)
            _ => vec![("outline", 280.0, Some(300.0)), ("inspector", 320.0, Some(600.0)), ("rows", 1000.0, Some(200.0)), ("session", 220.0, Some(400.0)), ("now", 700.0, Some(90.0)), ("info", 260.0, Some(100.0)), ("next", 320.0, Some(300.0))],
        }
    }

    #[test]
    fn every_panel_has_an_icon_of_the_kit_and_an_id_of_its_own() {
        for (name, specs) in workspaces() {
            for s in specs {
                assert!(deck_kit::editor::icons::has(s.icon), "{name}: {} has no icon {}", s.id, s.icon);
                assert!(!s.about.is_empty(), "{name}: {} says what it is", s.id);
            }
            let mut ids: Vec<&str> = specs.iter().map(|s| s.id).collect();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), specs.len(), "{name}: ids are unique");
            assert!(specs.iter().any(|s| s.id == "info"), "{name}: the info is one of its panels");
        }
    }

    #[test]
    fn each_workspace_lays_out_its_own_panels_apart_on_every_window_size_and_the_log_on_top() {
        let (t, d) = (Tokens::default(), DrawList::new(0.6));
        for (name, specs) in workspaces() {
            for size in [(1920.0, 1080.0), (1600.0, 900.0), (1280.0, 720.0), (1024.0, 640.0)] {
                let mut p = Panels::new(specs);
                p.tools = Some((8.0 * t.size.row, t.size.row));
                p.floats.reopen(crate::log::PANEL);
                for id in OPTIONAL.iter().filter(|_| name == "live") {
                    p.floats.reopen(id);
                }
                let mut shown = typical(name);
                p.layout(&d, &t, (size, 52.0), &[], &shown);
                shown.extend([("tools", 0.0, None), (crate::log::PANEL, 0.0, None)]);
                let room = Panels::room(size, 52.0, t.size.gap);
                let rects: Vec<Rect> = shown.iter().map(|(id, ..)| p.floats.view(id).unwrap().whole).collect();
                for (i, a) in rects.iter().enumerate() {
                    assert!(a.x >= room.x - 0.01 && a.y >= room.y - 0.01 && a.x + a.w <= room.x + room.w + 0.01 && a.y + a.h <= room.y + room.h + 0.01, "{name} {size:?}: {} inside", shown[i].0);
                    // the Log and a show's optional panels open where they were put, and a window narrower than the defaults were made for lays them over
                    // each other: inside the walls is all
                    let later = |id: &str| id == crate::log::PANEL || OPTIONAL.contains(&id);
                    if later(shown[i].0) || size.0 < 1280.0 {
                        continue;
                    }
                    for (j, b) in rects.iter().enumerate().skip(i + 1).filter(|(j, _)| !later(shown[*j].0)) {
                        let g = t.size.gap - 0.01;
                        let apart = a.x + a.w + g <= b.x || b.x + b.w + g <= a.x || a.y + a.h + g <= b.y || b.y + b.h + g <= a.y;
                        assert!(apart, "{name} {size:?}: {} {a:?} and {} {b:?}", shown[i].0, shown[j].0);
                    }
                }
            }
        }
    }

    #[test]
    fn a_content_takes_the_height_it_drew_the_frame_before() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let mut p = Panels::new(&BUILD);
        let shown = [("tree", 220.0, None)];
        p.layout(&d, &t, ((1600.0, 900.0), 52.0), &[], &shown);
        assert!(p.floats.view("tree").unwrap().whole.h > 700.0, "unmeasured: all the room");
        let mut hits: Vec<(Rect, ())> = Vec::new();
        let o = p.open(&mut d, &t, "tree", (0.0, 0.0), 0).unwrap();
        d.rect(o.area.x, o.area.y, 10.0, 120.0, [1.0; 4]);
        p.close(&mut d, o, &mut hits);
        p.layout(&d, &t, ((1600.0, 900.0), 52.0), &[], &shown);
        let v = p.floats.view("tree").unwrap();
        assert!((v.content.h - 121.0).abs() < 1e-3, "the rows it drew");
        assert_eq!(v.over, 0.0);
    }

    #[test]
    fn each_workspace_keeps_its_essentials_and_lists_the_rest() {
        let essentials = |specs: &[Spec]| specs.iter().filter(|s| s.essential).map(|s| s.id).collect::<Vec<_>>();
        assert_eq!(essentials(&BUILD), ["inspector", "tools"]);
        assert_eq!(essentials(&MODULE), ["module"]);
        assert_eq!(essentials(&SHOW), ["inspector", "sheet", "rows"]);
        for (name, specs) in workspaces() {
            let p = Panels::new(specs);
            let listed = p.listed();
            assert!(listed.iter().all(|(id, ..)| !specs.iter().any(|s| s.id == *id && s.essential)), "{name}: no essential is listed");
            assert!(listed.iter().all(|(id, ..)| *id != SETTINGS), "{name}: the settings have their own line");
            assert_eq!(listed.len(), specs.len() - essentials(specs).len() - 1, "{name}: every other panel is");
            assert!(p.floats.closed(SETTINGS), "{name}: the settings start closed");
        }
    }

    #[test]
    fn a_panel_closed_from_the_list_is_not_laid_out_and_comes_back() {
        let (t, d) = (Tokens::default(), DrawList::new(0.6));
        for (name, specs) in workspaces() {
            let mut p = Panels::new(specs);
            let shown = typical(name);
            for (id, _, open) in p.listed() {
                if id == crate::log::PANEL || id == HISTORY.id {
                    assert!(!open, "{name}: {id} starts closed");
                    continue;
                }
                if (matches!(name, "show" | "live") && OPTIONAL.contains(&id)) || (name == "module" && id == "drives") {
                    assert!(!open, "{name}: {id} is a running show's or Play's, it starts closed");
                    p.flip(id);
                }
                assert!(p.listed().iter().any(|(i, _, open)| *i == id && *open), "{name}: {id} open");
                p.flip(id);
                p.layout(&d, &t, ((1600.0, 900.0), 52.0), &[], &shown);
                assert!(p.floats.view(id).is_none(), "{name}: {id} closed");
                assert!(p.listed().iter().any(|(i, _, open)| *i == id && !open));
                p.flip(id);
                p.layout(&d, &t, ((1600.0, 900.0), 52.0), &[], &shown);
                assert_eq!(p.floats.view(id).is_some(), shown.iter().any(|(s, ..)| *s == id), "{name}: {id} back");
            }
            for s in specs.iter().filter(|s| s.essential) {
                p.flip(s.id);
                assert!(!p.floats.closed(s.id), "{name}: {} cannot close", s.id);
            }
        }
    }

    #[test]
    fn the_tools_pill_is_one_line_moves_by_its_handle_and_folds_to_its_wrench() {
        let (t, d) = (Tokens::default(), DrawList::new(0.6));
        let mut p = Panels::new(&BUILD);
        p.tools = Some((8.0 * t.size.row, t.size.row));
        let shown = typical("build");
        p.layout(&d, &t, ((1600.0, 900.0), 52.0), &[], &shown);
        let before = p.floats.view("tools").unwrap();
        assert!(before.whole.h < 2.0 * t.size.row, "one line: {before:?}");
        assert!(before.handle.y == before.whole.y && before.content.x >= before.handle.x + before.handle.w - 0.01, "the grip beside the tools");
        let (hx, hy) = (before.handle.x + before.handle.w * 0.5, before.handle.y + before.handle.h * 0.5);
        assert!(p.floats.press(hx, hy) && p.floats.dragging());
        p.floats.drag_to(hx, hy + 200.0);
        p.floats.release();
        p.layout(&d, &t, ((1600.0, 900.0), 52.0), &[], &shown);
        let moved = p.floats.view("tools").unwrap();
        assert!(moved.whole.y > before.whole.y + 10.0, "{before:?} → {moved:?}");
        let f = moved.fold;
        assert!(p.floats.press(f.x + f.w * 0.5, f.y + f.h * 0.5));
        p.floats.release();
        p.layout(&d, &t, ((1600.0, 900.0), 52.0), &[], &shown);
        let pill = p.floats.view("tools").unwrap();
        assert!(pill.collapsed && pill.whole.w == pill.whole.h, "folded: the wrench alone");
        assert!(p.floats.places()["tools"].collapsed, "kept folded");
        assert!(!p.floats.close("tools"), "essential in Build");
        assert!(p.floats.press(pill.whole.x + 2.0, pill.whole.y + 2.0));
        p.floats.release();
        assert!(!p.floats.collapsed("tools"), "a click on the wrench opens it");
    }

    #[test]
    fn play_shows_its_own_preset_and_stop_brings_edit_back() {
        let mut p = Panels::with_play(&BUILD, &BUILD_PLAY);
        p.floats.toggle("mixer");
        let edit = p.floats.places();
        assert!(p.show_mode(Some("scene")), "Play: the panels change");
        assert_eq!(p.play_kind(), Some("scene"));
        for id in BUILD_PLAY[0].closes {
            assert!(p.floats.closed(id), "{id} rests in Play");
        }
        assert!(!p.floats.closed("dop") && !p.floats.collapsed("mixer") && !p.floats.closed("info"), "what plays is out");
        assert_eq!(p.floats.places()["mixer"].corner, BL);
        assert_eq!(p.floats.places()["dop"].corner, TL);
        assert!(!p.show_mode(Some("scene")), "already Play: nothing changes");
        p.floats.reopen("tree");
        assert!(p.show_mode(None), "Stop");
        assert_eq!(p.floats.places(), edit, "Edit's layout as it was");
        assert!(p.show_mode(Some("scene")));
        assert!(!p.floats.closed("tree"), "the viewer's own Play is remembered for scenes");
        p.show_mode(None);
        p.show_mode(Some("asset"));
        assert!(p.floats.closed("tree"), "an asset tab has a Play of its own");
        assert!(p.floats.closed("dop") && !p.floats.closed("mixer"), "no camera to work in an asset");
        p.show_mode(None);
        assert_eq!(p.floats.places(), edit);
    }

    #[test]
    fn a_module_plays_with_its_drives_out() {
        let mut p = Panels::with_play(&MODULE, &MODULE_PLAY);
        assert!(p.floats.closed("drives"), "Edit: nothing drives yet");
        p.show_mode(Some("module"));
        assert!(!p.floats.closed("drives") && p.floats.closed(crate::library::PANEL));
        p.show_mode(None);
        assert!(p.floats.closed("drives") && !p.floats.closed(crate::library::PANEL));
        for (specs, sets) in [(&BUILD[..], &BUILD_PLAY[..]), (&MODULE[..], &MODULE_PLAY[..])] {
            for s in sets {
                for id in s.closes.iter().chain(s.opens.iter().map(|(id, _)| id)) {
                    assert!(specs.iter().any(|x| x.id == *id && !x.essential), "{}: {id} is a panel of its workspace that closes", s.kind);
                }
            }
        }
        let mut home = Panels::new(&HOME);
        assert!(!home.show_mode(Some("x")) && home.play_kind().is_none(), "a workspace without Play stays");
        assert!(!p.show_mode(Some("scene")) && p.play_kind().is_none(), "a kind without Play is Edit");
    }

    #[test]
    fn edit_and_play_places_are_kept_apart_in_the_cache() {
        let mut p = Panels::with_play(&BUILD, &BUILD_PLAY);
        p.show_mode(Some("asset"));
        p.floats.reopen(crate::library::PANEL);
        p.show_mode(None);
        p.show_mode(Some("scene"));
        p.floats.toggle("mixer");
        let mut all = Remembered::new();
        p.remember("build", &mut all);
        assert!(!all["build"]["tree"].closed, "Edit's places, also while Play shows");
        assert!(all["build.play.scene"]["mixer"].collapsed && all["build.play.scene"]["tree"].closed);
        assert!(!all["build.play.asset"][crate::library::PANEL].closed);
        let mut q = Panels::with_play(&BUILD, &BUILD_PLAY);
        q.recall("build", &all);
        assert!(!q.floats.closed("tree"), "Edit shows at the start");
        q.show_mode(Some("scene"));
        assert!(q.floats.collapsed("mixer"), "Play as the viewer left it");
        q.show_mode(Some("asset"));
        assert!(!q.floats.closed(crate::library::PANEL));
    }

    #[test]
    fn places_come_back_from_the_cache_file() {
        let file = std::env::temp_dir().join(format!("deck-editor-panels-{}.json", std::process::id()));
        let mut all = Remembered::new();
        let mut p = Panels::new(&SHOW);
        p.floats.toggle("rows");
        p.flip("library");
        all.insert("show".into(), p.floats.places());
        save(&file, &all);
        let back = load(&file);
        std::fs::remove_file(&file).ok();
        assert_eq!(back, all);
        let mut q = Panels::new(&SHOW);
        q.floats.restore(&back["show"]);
        assert!(q.floats.collapsed("rows"));
        assert!(q.floats.closed("library"));
        assert!(load(Path::new("no/such/file.json")).is_empty(), "nothing kept: the defaults");
    }
}
