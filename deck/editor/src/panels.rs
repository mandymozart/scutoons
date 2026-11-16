//! The editor's floating panels (deck-kit `floats`): each workspace's set over the full-window stage, the size each
//! content asks for this frame (its own when it knows it, else what it drew the frame before), and where the viewer left
//! them – a per-viewer convenience in the editor's cache, one map per workspace.
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::floats::{self, Corner, Floats, Metrics, Place, Shown, Spec, View};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

const fn spec(id: &'static str, name: &'static str, icon: &'static str, about: &'static str, place: Place) -> Spec {
    Spec { id, name, icon, about, place }
}

const TL: Corner = Corner::TopLeft;
const TR: Corner = Corner::TopRight;
const BL: Corner = Corner::BottomLeft;
const BR: Corner = Corner::BottomRight;

// The order is the order of growth: an earlier panel leaves a later one the room it asks for, so the ones whose content
// scrolls well come first and the ones of a fixed size last.
pub const BUILD: [Spec; 5] = [
    spec("tree", "Rig", "tree", "The rig, its tracks and their nodes. In Wire a click goes to that level.", Place::at(TL, 0.0, 0.0)),
    spec("inspector", "Inspector", "inspector", "What is selected: its values as scrub numbers; the rig picker and Save.", Place::at(TR, 0.0, 0.0)),
    spec("audio", "Audio", "activity", "The sound card, its inputs and outputs, the map and the test tones.", Place::at(BL, 300.0, 0.0).folded()),
    spec("info", "Info", "info", "What is under the pointer: its name and what it does.", Place::at(BL, 0.0, 0.0)),
    spec("mixer", "Mixer", "sliders", "One strip per track at the level of the tree shown; inside a track its gains.", Place::at(BR, 0.0, 0.0)),
];

pub const SHOW: [Spec; 7] = [
    spec("outline", "Outline", "nodes", "The conductors, the roles this one gives out and the rows of the document shown.", Place::at(TL, 0.0, 0.0)),
    spec("inspector", "Inspector", "inspector", "The row, the role or the cells selected; Load, Flash and Save.", Place::at(TR, 0.0, 0.0)),
    spec("sheet", "Sheet", "layers", "The sheet open: cues, problems, who is here, a picker or the flash list.", Place::at(TL, 320.0, 0.0)),
    spec("nodes", "Nodes", "node", "What comes from far, the cells of the row and what the row sends, over the stage.", Place::at(TL, 320.0, 0.0)),
    spec("scenes", "Scenes", "grid", "The toon's published scenes: drag one onto a cell.", Place::at(BR, 0.0, 0.0)),
    spec("rows", "Rows", "list", "The rows of the show: what fires each one and every column's cell.", Place::at(BL, 0.0, 0.0)),
    spec("info", "Info", "info", "What is under the pointer: its name and what it does.", Place::at(BR, 300.0, 0.0).folded()),
];

pub const MODULE: [Spec; 4] = [
    spec("modules", "Modules", "list", "The catalog's modules: a click opens one's working copy.", Place::at(TL, 0.0, 0.0)),
    spec("module", "Module", "code", "The module open: its working copy, Open in editor, Build, what happened last, its parameters.", Place::at(TR, 0.0, 0.0)),
    spec("problems", "Problems", "alert", "The last build's problems: a click opens the code there.", Place::at(BL, 0.0, 0.0)),
    spec("info", "Info", "info", "What is under the pointer: its name and what it does.", Place::at(BR, 0.0, 0.0).folded()),
];

pub const PERFORM: [Spec; 7] = [
    spec("rail", "Session", "nodes", "This conductor, its seats, the relay, the linked readers, the vote and the outlets.", Place::at(TL, 0.0, 0.0)),
    spec("side", "Next", "play", "The suggestion with GO and Drop, else GO; the seats.", Place::at(TR, 0.0, 0.0)),
    spec("sheet", "Show runner", "layers", "The show runner's sheet open: the session for linked readers, or the vote.", Place::at(TL, 300.0, 0.0)),
    spec("now", "Stage now", "eye", "What plays on each outlet now.", Place::at(TL, 300.0, 0.0)),
    spec("log", "Log", "entry", "What happened, newest first.", Place::at(BR, 0.0, 0.0).folded()),
    spec("matrix", "Cues", "list", "Rows × outlets: the row's play fires it, a cell's play launches it by hand.", Place::at(BL, 0.0, 0.0)),
    spec("info", "Info", "info", "What is under the pointer: its name and what it does.", Place::at(BR, 120.0, 0.0).folded()),
];

/// a content height before it was ever drawn: as tall as the room lets it be, the next frame its own
const UNMEASURED: f32 = 10_000.0;

/// a panel's content being drawn this frame (from `open` to `close`)
pub struct Open {
    pub id: &'static str,
    /// where to draw: the visible width, the content's own height, moved up by the scroll
    pub area: Rect,
    view: View,
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
}

impl Panels {
    pub fn new(specs: &[Spec]) -> Panels {
        Panels { floats: Floats::new(specs), measured: HashMap::new(), known: Vec::new(), grew: false }
    }

    /// the room for panels in a window `size` under a toolbar ending at `top`
    pub fn room(size: (f32, f32), top: f32, gap: f32) -> Rect {
        Rect { x: gap, y: top + gap, w: (size.0 - 2.0 * gap).max(1.0), h: (size.1 - top - 2.0 * gap).max(1.0) }
    }

    /// this frame's layout: the panels shown, each with its content's width and – when it knows it – height (else the
    /// height it drew last frame); places nothing may cover
    pub fn layout(&mut self, d: &DrawList, t: &Tokens, (size, top): ((f32, f32), f32), fixed: &[Rect], shown: &[(&'static str, f32, Option<f32>)]) {
        let m = Metrics::of(t, d);
        self.known = shown.iter().filter(|(_, _, h)| h.is_some()).map(|(id, ..)| *id).collect();
        let shown: Vec<Shown> = shown.iter().map(|&(id, w, h)| Shown { id, content: (w, h.unwrap_or_else(|| self.measured.get(id).copied().unwrap_or(UNMEASURED))) }).collect();
        self.floats.layout(Self::room(size, top, m.gap), fixed, &shown, m);
    }

    /// a panel's ground and chrome; open, the content's place (`hits` = how many places were pressed before it): a
    /// content measured draws without an end, so a list shows all of itself and the next frame knows its height
    pub fn open(&self, d: &mut DrawList, t: &Tokens, id: &'static str, pointer: (f32, f32), hits: usize) -> Option<Open> {
        let view = self.floats.view(id)?;
        let spec = self.floats.spec(id)?;
        floats::begin(d, t, spec, &view, pointer);
        if view.collapsed {
            return None;
        }
        let mut area = self.floats.content(id)?;
        if !self.known.contains(&id) {
            area.h = UNMEASURED;
        }
        Some(Open { id, area, view, prims: d.prims().len(), hits })
    }

    /// whether a measured content came out taller than its panel was laid out for (once asked, false again): what
    /// was drawn after it is laid out again, so a row just added shows and answers in the same frame
    pub fn take_grew(&mut self) -> bool {
        std::mem::take(&mut self.grew)
    }

    /// the content drawn: its height is kept for the next frame, its places cut to what shows
    pub fn close<H>(&mut self, d: &mut DrawList, t: &Tokens, o: Open, hits: &mut Vec<(Rect, H)>) {
        if let Some(e) = d.extent(o.prims) {
            let h = (e.y + e.h - o.area.y).max(0.0) + 1.0;
            if !self.known.contains(&o.id) && h > self.measured.get(o.id).copied().unwrap_or(UNMEASURED) + 0.5 && o.view.over <= 0.0 {
                self.grew = true;
            }
            self.measured.insert(o.id, h);
        }
        floats::clip_hits(&o.view, hits, o.hits);
        floats::end(d, t, &o.view);
    }

    /// the panels' chrome (handles, chevrons) and grounds, so the stage under them takes no press
    pub fn grounds(&self, ids: &[&'static str]) -> Vec<Rect> {
        ids.iter().filter_map(|id| self.floats.view(id)).map(|v| v.whole).collect()
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
        [("build", &BUILD), ("module", &MODULE), ("show", &SHOW), ("perform", &PERFORM)]
    }

    /// what each workspace's panels ask for on a typical frame (the sheet and its twin are never shown together)
    fn typical(name: &str) -> Vec<(&'static str, f32, Option<f32>)> {
        match name {
            "build" => vec![("tree", 220.0, Some(200.0)), ("inspector", 280.0, Some(900.0)), ("audio", 300.0, Some(500.0)), ("info", 260.0, Some(100.0)), ("mixer", 560.0, Some(112.0))],
            "module" => vec![("modules", 220.0, Some(400.0)), ("module", 360.0, Some(700.0)), ("problems", 700.0, Some(200.0)), ("info", 260.0, Some(100.0))],
            "show" => vec![("outline", 280.0, Some(500.0)), ("inspector", 320.0, Some(1200.0)), ("nodes", 760.0, Some(360.0)), ("scenes", 240.0, Some(220.0)), ("rows", 900.0, Some(160.0)), ("info", 260.0, Some(100.0))],
            _ => vec![("rail", 220.0, Some(400.0)), ("side", 300.0, Some(300.0)), ("now", 700.0, Some(90.0)), ("log", 520.0, Some(160.0)), ("matrix", 1000.0, Some(200.0)), ("info", 260.0, Some(100.0))],
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
    fn each_workspace_lays_out_apart_on_every_window_size() {
        let (t, d) = (Tokens::default(), DrawList::new(0.6));
        for (name, specs) in workspaces() {
            for size in [(1920.0, 1080.0), (1600.0, 900.0), (1280.0, 720.0), (1024.0, 640.0)] {
                let mut p = Panels::new(specs);
                let shown = typical(name);
                p.layout(&d, &t, (size, 52.0), &[], &shown);
                let room = Panels::room(size, 52.0, 16.0);
                let rects: Vec<Rect> = shown.iter().map(|(id, ..)| p.floats.view(id).unwrap().whole).collect();
                for (i, a) in rects.iter().enumerate() {
                    assert!(a.x >= room.x - 0.01 && a.y >= room.y - 0.01 && a.x + a.w <= room.x + room.w + 0.01 && a.y + a.h <= room.y + room.h + 0.01, "{name} {size:?}: {} inside", shown[i].0);
                    for (j, b) in rects.iter().enumerate().skip(i + 1) {
                        let apart = a.x + a.w + 15.99 <= b.x || b.x + b.w + 15.99 <= a.x || a.y + a.h + 15.99 <= b.y || b.y + b.h + 15.99 <= a.y;
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
        p.close(&mut d, &t, o, &mut hits);
        p.layout(&d, &t, ((1600.0, 900.0), 52.0), &[], &shown);
        let v = p.floats.view("tree").unwrap();
        assert_eq!(v.content.h, 121.0, "the rows it drew");
        assert_eq!(v.over, 0.0);
    }

    #[test]
    fn places_come_back_from_the_cache_file() {
        let file = std::env::temp_dir().join(format!("deck-editor-panels-{}.json", std::process::id()));
        let mut all = Remembered::new();
        let mut p = Panels::new(&SHOW);
        p.floats.toggle("rows");
        all.insert("show".into(), p.floats.places());
        save(&file, &all);
        let back = load(&file);
        std::fs::remove_file(&file).ok();
        assert_eq!(back, all);
        let mut q = Panels::new(&SHOW);
        q.floats.restore(&back["show"]);
        assert!(q.floats.collapsed("rows"));
        assert!(load(Path::new("no/such/file.json")).is_empty(), "nothing kept: the defaults");
    }
}
