//! What a deck plays for a show's cell (#467, agents/research/SHOW.md §5): the conductor always sends a **full cell** – a
//! piece and all its values – and the deck compares it with what plays: the same piece glides to the new values, another
//! piece loads and crossfades. Platform-free and sans IO: the cell and the time go in, the adapter gets what to do (load a
//! scene with a crossfade, glide values, fade to black) and reads the gliding values each frame (`values_at`). It plays
//! pieces through what the deck already has – a scene (`Played::scene`), a rig or a character (`Bundle::play`, a core rig
//! first) – and what the deck has beyond its core rigs comes through the adapter's `find` (bytes in, RULES A1).
//!
//! - **Modes** (#89): the conductor resolves `keep` and `values` along the path really taken, so a deck sees `own` with a
//!   piece (play: what the cell holds replaces what was set – a value it no longer names glides back to the piece's own),
//!   `black` (the piece fades out over its transition), or `mirror` (the conductor sends the cell itself: black here, said).
//! - **Values**: a value names a slot of the piece by the one parser (`rig/address.rs`: `node.port`, `<track>:<node>.<port>`);
//!   a bare name is that node's first knob (the scene's `[being] expose` orders them). A string `mood` plays a rig for that
//!   mood. A value that names nothing the piece plays is a reason on the HUD, the rest plays.
//! - **Time**: the cell's layer `enter`, else the piece's `[stage] transition_secs` (#264: a switch crossfades, `cut` and
//!   `blackout` hand over at once).
//! - **Not ready** (a piece or its media the deck does not have, a scene that does not build): a reason for the HUD and the
//!   log; what plays stays. It never blocks GO: the conductor fires on, the next cell is compared again.
use crate::rig::address::{node_port, Address, Leaf, Place};
use crate::rig::{Bundle, Played};
use crate::scene::{Scene, SceneTransition, StageSettings};
use protocol::{Cell, CellMode, Layer, LayerSrc};
use serde::Serialize;
use std::collections::VecDeque;

/// the log keeps this many lines; the oldest go (a show must not grow without bound, RULES A10)
pub const LOG_CAP: usize = 256;
/// two values closer than this do not glide
const SAME: f32 = 1e-6;

/// What the adapter has for a piece the deck carries no core rig of: a scene's TOML, or a rig bundle's TOML.
#[derive(Debug, Clone, PartialEq)]
pub enum Found {
    Scene(String),
    Rig(String),
}

/// One value gliding from what plays to the cell's.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Glide {
    pub path: String,
    pub from: f32,
    pub to: f32,
}

/// What the adapter does for a cell.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "do", rename_all = "lowercase")]
pub enum What {
    /// another piece: load `scene` (the engine's – live-core's – TOML), the one before crossfading out under it over
    /// `ms`, then set `values` at once
    Load { piece: String, scene: String, values: Vec<(String, f32)>, ms: u32 },
    /// the same piece: these values glide over `ms` (`values_at` each frame)
    Glide { piece: String, glides: Vec<Glide>, ms: u32 },
    /// no piece any more: the deck's own silent scene (#425) crossfades in over `ms` – the piece fades out under it
    Black { scene: String, ms: u32 },
    /// the same piece with the same values
    Same,
    /// the piece cannot play here: what plays stays (the problems say why)
    Kept,
}

/// A cell's answer: what to do, and the reasons for the HUD (also in the log).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Answer {
    pub what: What,
    pub problems: Vec<String>,
}

/// The piece on: its place, the mood it plays (a rig), how it plays, the values the cells set (slot → value), its time.
#[derive(Debug, Clone)]
struct On {
    piece: String,
    mood: Option<String>,
    played: Played,
    set: Vec<(String, f32)>,
    ms: u32,
}

/// Values on their way: from `at` (ms) over `ms`.
#[derive(Debug, Clone)]
struct Moving {
    glides: Vec<Glide>,
    at: f64,
    ms: u32,
}

/// A deck's pieces: what plays, what glides, the log.
#[derive(Debug, Clone, Default)]
pub struct Pieces {
    on: Option<On>,
    moving: Option<Moving>,
    log: VecDeque<String>,
}

/// the piece's own transition (ms): `fade` its `transition_secs`, `cut` / `blackout` at once
fn stage_ms(stage: &StageSettings) -> u32 {
    match stage.transition {
        SceneTransition::Fade => (stage.transition_secs.clamp(0.0, 60.0) * 1000.0).round() as u32,
        _ => 0,
    }
}

/// the piece's layer of a cell (the first; the other layers are the outlet's)
fn piece_layer(cell: &Cell) -> Option<(&Layer, &protocol::PieceSrc)> {
    cell.layers.iter().flatten().find_map(|l| match &l.src {
        LayerSrc::Piece(p) => Some((l, p)),
        _ => None,
    })
}

impl Pieces {
    pub fn new() -> Pieces {
        Pieces::default()
    }

    /// the piece on (`scene:nave`), if one plays
    pub fn piece(&self) -> Option<&str> {
        self.on.as_ref().map(|o| o.piece.as_str())
    }

    /// how the piece on plays (its knobs, its engine scene)
    pub fn played(&self) -> Option<&Played> {
        self.on.as_ref().map(|o| &o.played)
    }

    /// the log, oldest first
    pub fn log(&self) -> impl Iterator<Item = &str> {
        self.log.iter().map(String::as_str)
    }

    fn note(&mut self, line: String) {
        if self.log.len() == LOG_CAP {
            self.log.pop_front();
        }
        self.log.push_back(line);
    }

    /// how far a glide is at `now` (0..1); None when nothing glides
    pub fn gliding(&self, now: f64) -> Option<f32> {
        self.moving.as_ref().map(|m| if m.ms == 0 { 1.0 } else { ((now - m.at) / f64::from(m.ms)).clamp(0.0, 1.0) as f32 })
    }

    /// The gliding values at `now` (ms, the adapter's clock): the adapter sets them each frame. The frame that reaches the
    /// end gets the targets and ends the glide; none when nothing glides.
    pub fn values_at(&mut self, now: f64) -> Vec<(String, f32)> {
        let Some(t) = self.gliding(now) else { return Vec::new() };
        let m = self.moving.as_ref().map(|m| m.glides.iter().map(|g| (g.path.clone(), g.from + (g.to - g.from) * t)).collect()).unwrap_or_default();
        if t >= 1.0 {
            self.moving = None;
        }
        m
    }

    /// what `path` plays at `now`: gliding, else what a cell set, else the piece's own
    fn current(&self, path: &str, now: f64) -> Option<f32> {
        let t = self.gliding(now).unwrap_or(1.0);
        if let Some(g) = self.moving.as_ref().and_then(|m| m.glides.iter().find(|g| g.path == path)) {
            return Some(g.from + (g.to - g.from) * t);
        }
        let on = self.on.as_ref()?;
        on.set.iter().find(|(p, _)| p == path).map(|(_, v)| *v).or_else(|| on.played.value(path))
    }

    /// A cell for this deck's outlet at `now` (ms). `find` is asked for a piece the deck carries no core rig of.
    pub fn cell(&mut self, cell: &Cell, now: f64, find: impl FnOnce(&Place) -> Option<Found>) -> Answer {
        let mut problems = Vec::new();
        let what = self.decide(cell, now, find, &mut problems);
        let line = match &what {
            What::Load { piece, values, ms, .. } => format!("load {piece} ({} values), crossfade {ms} ms", values.len()),
            What::Glide { piece, glides, ms } => format!("glide {piece}: {} values over {ms} ms", glides.len()),
            What::Black { ms, .. } => format!("black over {ms} ms"),
            What::Same => "the same piece and values".into(),
            What::Kept => format!("kept {}", self.piece().unwrap_or("nothing")),
        };
        self.note(line);
        for p in &problems {
            self.note(p.clone());
        }
        Answer { what, problems }
    }

    fn black(&mut self) -> What {
        match self.on.take() {
            Some(on) => {
                self.moving = None;
                let scene = Played::silent().map(|p| p.engine.scene).unwrap_or_default();
                What::Black { scene, ms: on.ms }
            }
            None => What::Same,
        }
    }

    fn decide(&mut self, cell: &Cell, now: f64, find: impl FnOnce(&Place) -> Option<Found>, problems: &mut Vec<String>) -> What {
        match cell.mode {
            CellMode::Black => return self.black(),
            CellMode::Mirror => {
                problems.push(format!("mirror of '{}': the conductor sends the cell itself – black here", cell.of.as_deref().unwrap_or("?")));
                return self.black();
            }
            CellMode::Own => {}
        }
        // a full cell without a piece: no piece plays
        let Some((layer, src)) = piece_layer(cell) else { return self.black() };
        let place = match Place::parse(&src.piece) {
            Ok(p) => p,
            Err(e) => {
                problems.push(format!("{}: {e}", src.piece));
                return What::Kept;
            }
        };
        let piece = place.to_string();
        let values = src.values.clone().unwrap_or_default();
        let mut mood = None;
        let mut numbers = Vec::new();
        for (k, v) in &values {
            match (k.as_str(), v) {
                ("mood", serde_json::Value::String(m)) => mood = Some(m.clone()),
                (_, v) if v.as_f64().is_some() => numbers.push((k.clone(), v.as_f64().unwrap_or_default() as f32)),
                _ => problems.push(format!("{piece}: the value '{k}' is no number")),
            }
        }
        let enter = layer.enter.as_ref().map(|t| t.ms.min(60_000) as u32);
        // the same piece (and mood): its values glide
        if let Some(on) = self.on.as_ref().filter(|o| o.piece == piece && o.mood == mood) {
            let played = on.played.clone();
            let ms = enter.unwrap_or(on.ms);
            let targets = resolve(&played, &piece, &numbers, problems);
            return self.glide(piece, played, mood, targets, ms, now);
        }
        let played = match load(&place, mood.as_deref(), find, problems) {
            Ok(p) => p,
            Err(e) => {
                problems.push(e);
                return What::Kept;
            }
        };
        if mood.is_some() && place.kind == "scene" {
            problems.push(format!("{piece}: a scene plays no mood – only a rig does"));
        }
        let ms = enter.unwrap_or_else(|| Scene::parse(&played.scene).map(|s| stage_ms(&s.stage)).unwrap_or(0));
        let targets = resolve(&played, &piece, &numbers, problems);
        // the same rig for another mood: the replay's values glide unless its shape changed (then it loads again)
        if let Some(on) = self.on.as_ref().filter(|o| o.piece == piece) {
            let d = on.played.diff(&played);
            if !d.reload {
                let mut all = d.values;
                all.retain(|(p, _)| played.target(p).is_some() && !targets.iter().any(|(q, _)| q == p));
                all.extend(targets);
                let ms = enter.unwrap_or(on.ms);
                return self.glide(piece, played, mood, all, ms, now);
            }
        }
        self.moving = None;
        let scene = played.engine.scene.clone();
        self.on = Some(On { piece: piece.clone(), mood, played, set: targets.clone(), ms });
        What::Load { piece, scene, values: targets, ms }
    }

    /// the same piece to `targets`: every value that moves glides from where it is now; a value the last cell set and
    /// this one does not glides back to the piece's own (play replaces values)
    fn glide(&mut self, piece: String, played: Played, mood: Option<String>, targets: Vec<(String, f32)>, ms: u32, now: f64) -> What {
        // what the cell sets, what the last one set, what is still on its way back
        let mut paths: Vec<String> = targets.iter().map(|(p, _)| p.clone()).collect();
        let old = self.on.iter().flat_map(|o| o.set.iter().map(|(p, _)| p.clone()));
        let moving = self.moving.iter().flat_map(|m| m.glides.iter().map(|g| g.path.clone()));
        for p in old.chain(moving).collect::<Vec<_>>() {
            if !paths.contains(&p) {
                paths.push(p);
            }
        }
        let mut glides = Vec::new();
        for path in paths {
            let to = targets.iter().find(|(p, _)| *p == path).map(|(_, v)| *v).or_else(|| played.value(&path));
            let (Some(to), Some(from)) = (to, self.current(&path, now)) else { continue };
            if (to - from).abs() > SAME {
                glides.push(Glide { path, from, to });
            }
        }
        self.on = Some(On { piece: piece.clone(), mood, played, set: targets, ms });
        if glides.is_empty() {
            self.moving = None;
            return What::Same;
        }
        self.moving = Some(Moving { glides: glides.clone(), at: now, ms });
        What::Glide { piece, glides, ms }
    }
}

/// a piece → how it plays: a scene from the adapter; a character or an item from the deck's core rigs, else the adapter;
/// a rig from the adapter. Err = why it cannot play here.
fn load(place: &Place, mood: Option<&str>, find: impl FnOnce(&Place) -> Option<Found>, problems: &mut Vec<String>) -> Result<Played, String> {
    let piece = place.to_string();
    let core = matches!(place.kind.as_str(), "character" | "item").then(|| Bundle::core(&format!("{}.{}", place.kind, place.id))).flatten();
    let found = match core {
        Some(b) => Some(b.map_err(|e| format!("{piece}: {e:#}"))?),
        None => None,
    };
    let bundle = match (found, place.kind.as_str()) {
        (Some(b), _) => b,
        (None, "scene" | "character" | "item" | "rig") => match find(place) {
            Some(Found::Scene(text)) => return Played::scene(&text).map_err(|e| format!("{piece}: {e:#}")),
            Some(Found::Rig(text)) => toml::from_str::<Bundle>(&text).map_err(|e| format!("{piece}: {e}"))?,
            None => return Err(format!("{piece} is not on this deck")),
        },
        (None, kind) => return Err(format!("{piece}: a piece of the kind '{kind}' does not play on a deck")),
    };
    match bundle.play(None, mood) {
        Ok(p) => Ok(p),
        Err(e) if mood.is_some() => {
            problems.push(format!("{piece}: the mood '{}' – {e:#}; it plays without", mood.unwrap_or_default()));
            bundle.play(None, None).map_err(|e| format!("{piece}: {e:#}"))
        }
        Err(e) => Err(format!("{piece}: {e:#}")),
    }
}

/// the node kinds a deck's outlet scene keeps when a piece plays beside it (the picture's way out, the clock)
const OUTLET_KINDS: [&str; 3] = ["outlet", "screen", "tempo"];

/// The scene a deck with an outlet plays for a piece (#91, the native deck's live outlet): the piece's whole scene
/// (`Played::scene`, its sound and its space) with the deck scene's `outlet` and `screen` kept, so the outlet goes on
/// drawing its cells over the piece's space; the deck's space and tempo stand in when the piece has none. `values` (a
/// Load's) are set as the nodes' params where the slot is `node.port`; the rest come with the next frame's glide.
/// TOML text in and out: every deck reads its scene the same way.
pub fn beside(piece: &str, deck: &str, values: &[(String, f32)]) -> Result<String, String> {
    let table = |t: &str, what: &str| t.parse::<toml::Table>().map_err(|e| format!("{what}: {e}"));
    let (mut out, deck) = (table(piece, "the piece's scene")?, table(deck, "the deck's scene")?);
    let nodes = |t: &toml::Table| t.get("node").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let kind = |n: &toml::Value| n.get("kind").and_then(|k| k.as_str()).unwrap_or_default().to_string();
    let id = |n: &toml::Value| n.get("id").and_then(|k| k.as_str()).unwrap_or_default().to_string();
    let mut mine: Vec<toml::Value> = nodes(&out).into_iter().filter(|n| !matches!(kind(n).as_str(), "outlet" | "screen")).collect();
    let dropped: Vec<String> = nodes(&out).iter().filter(|n| matches!(kind(n).as_str(), "outlet" | "screen")).map(id).collect();
    let has = |nodes: &[toml::Value], k: &str| nodes.iter().any(|n| kind(n) == k);
    let theirs = nodes(&deck);
    // a rig's space comes bare (`Played::scene`): it plays in the deck's room – every param of the deck's space it does not set
    if let (Some(own), Some(space)) = (theirs.iter().find(|n| kind(n) == "space").and_then(|n| n.as_table()), mine.iter_mut().find(|n| kind(n) == "space").and_then(|n| n.as_table_mut())) {
        for (k, v) in own {
            if k != "id" {
                space.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
    }
    for n in &theirs {
        let k = kind(n);
        let keep = OUTLET_KINDS.contains(&k.as_str()) && !(k == "tempo" && has(&mine, "tempo"));
        if keep || (k == "space" && !has(&mine, "space")) {
            mine.push(n.clone());
        }
    }
    let Some(outlet) = theirs.iter().find(|n| kind(n) == "outlet").map(id) else { return Err("the deck's scene has no outlet".into()) };
    let screen = theirs.iter().find(|n| kind(n) == "screen").map(id);
    let space = mine.iter().find(|n| kind(n) == "space").map(id);
    // the piece's own links, but none into what it lost; then the outlet's: space → outlet → screen
    let ends = |l: &toml::Value, k: &str| l.get(k).and_then(|v| v.as_str()).map(|s| s.split('.').next().unwrap_or_default().to_string()).unwrap_or_default();
    let mut links: Vec<toml::Value> = out.get("link").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    links.retain(|l| !dropped.contains(&ends(l, "from")) && !dropped.contains(&ends(l, "to")));
    let link = |from: &str, to: String| {
        let mut t = toml::Table::new();
        t.insert("from".into(), from.into());
        t.insert("to".into(), to.into());
        toml::Value::Table(t)
    };
    if let Some(s) = &space {
        links.retain(|l| !(ends(l, "from") == *s && ends(l, "to") == outlet));
        links.push(link(s, format!("{outlet}.in")));
    }
    if let Some(sc) = &screen {
        links.push(link(&outlet, format!("{sc}.in")));
    }
    for (path, v) in values {
        let Some((node, port)) = path.rsplit_once('.') else { continue };
        if let Some(t) = mine.iter_mut().find(|n| id(n) == node).and_then(|n| n.as_table_mut()) {
            t.insert(port.into(), toml::Value::Float(f64::from(*v)));
        }
    }
    out.insert("node".into(), toml::Value::Array(mine));
    out.insert("link".into(), toml::Value::Array(links));
    toml::to_string(&out).map_err(|e| format!("the scene beside the outlet: {e}"))
}

/// the cell's numbers → the slots of the piece they set (the one parser; a bare name is its node's first knob)
fn resolve(played: &Played, piece: &str, numbers: &[(String, f32)], problems: &mut Vec<String>) -> Vec<(String, f32)> {
    let mut out = Vec::new();
    for (key, v) in numbers {
        let slot = match Address::leaf(key) {
            Ok(Leaf::Port { track: None, node, .. }) if !key.contains('.') => played.knobs.iter().find(|k| node_port(&k.path).0 == node).map(|k| k.path.clone()),
            Ok(leaf) => leaf.slot(),
            Err(e) => {
                problems.push(format!("{piece}: the value '{key}': {e}"));
                continue;
            }
        };
        match slot.filter(|s| played.target(s).is_some()) {
            Some(s) => out.push((s, *v)),
            None => problems.push(format!("{piece}: the value '{key}' names nothing it plays")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a stand-in for a show's nave: a tone (`light`) and a noise (`dim`) into a mixer; a bare name turns the node's gain
    pub const NAVE: &str = r#"
[stage]
name = "nave"
transition_secs = 0.5

[being]
expose = ["light.gain", "dim.gain"]

[[node]]
id = "light"
kind = "osc"
freq = 220.0
gain = 0.8

[[node]]
id = "dim"
kind = "noise"
gain = 0.3

[[node]]
id = "mix"
kind = "mixer"

[[node]]
id = "out"
kind = "audio_out"

[[link]]
from = "light"
to = "mix.in1"

[[link]]
from = "dim"
to = "mix.in2"

[[link]]
from = "mix"
to = "out.in"
"#;

    fn cell(json: &str) -> Cell {
        serde_json::from_str(json).unwrap()
    }

    fn pc(p: &str, values: &str) -> Cell {
        cell(&format!(r#"{{"mode":"own","layers":[{{"id":"piece","src":{{"kind":"piece","p":{{"piece":"{p}","values":{values}}}}}}}]}}"#))
    }

    fn nave(p: &Place) -> Option<Found> {
        (p.to_string() == "scene:nave").then(|| Found::Scene(NAVE.into()))
    }

    #[test]
    fn a_piece_loads_then_glides_on_the_same_piece() {
        let mut d = Pieces::new();
        let a = d.cell(&pc("scene:nave", r#"{"light":0.2}"#), 0.0, nave);
        let What::Load { piece, values, ms, scene } = &a.what else { panic!("{a:?}") };
        assert_eq!((piece.as_str(), ms), ("scene:nave", &500), "the scene's transition");
        assert_eq!(values, &[("light.gain".to_string(), 0.2)]);
        assert!(scene.contains("noise") && a.problems.is_empty(), "{a:?}");
        // the conductor's merged cell: light stays, dim glides from the scene's own 0.3
        let b = d.cell(&pc("scene:nave", r#"{"light":0.2,"dim":0.5}"#), 1000.0, |_| None);
        assert_eq!(b.what, What::Glide { piece: "scene:nave".into(), glides: vec![Glide { path: "dim.gain".into(), from: 0.3, to: 0.5 }], ms: 500 });
        let mid = d.values_at(1250.0);
        assert_eq!(mid.len(), 1);
        assert!((mid[0].1 - 0.4).abs() < 1e-6, "{mid:?}");
        assert_eq!(d.values_at(1600.0), vec![("dim.gain".to_string(), 0.5)], "the end: the target, once");
        assert!(d.values_at(1700.0).is_empty() && d.gliding(1700.0).is_none());
        assert_eq!(d.cell(&pc("scene:nave", r#"{"light":0.2,"dim":0.5}"#), 2000.0, |_| None).what, What::Same);
        // play replaces values: dim no longer named glides back to the scene's own
        let c = d.cell(&pc("scene:nave", r#"{"light":0.2}"#), 3000.0, |_| None);
        let What::Glide { glides, .. } = &c.what else { panic!("{c:?}") };
        assert_eq!(glides, &[Glide { path: "dim.gain".into(), from: 0.5, to: 0.3 }]);
        // a new cell mid-glide starts from where the value is
        let e = d.cell(&pc("scene:nave", r#"{"light":0.6}"#), 3250.0, |_| None);
        let What::Glide { glides, .. } = &e.what else { panic!("{e:?}") };
        assert!(glides.iter().any(|g| g.path == "dim.gain" && (g.from - 0.4).abs() < 1e-6 && g.to == 0.3), "{glides:?}");
        assert!(d.log().any(|l| l.starts_with("glide scene:nave")));
    }

    #[test]
    fn another_piece_crossfades_and_a_core_rig_needs_no_adapter() {
        let mut d = Pieces::new();
        d.cell(&pc("scene:nave", "{}"), 0.0, nave);
        let a = d.cell(&pc("character:catpope", r#"{"mood":"laughing"}"#), 100.0, |_| None);
        let What::Load { piece, ms, .. } = &a.what else { panic!("{a:?}") };
        assert_eq!(piece, "character:catpope");
        assert!(*ms > 0, "the rig's stage transition");
        assert_eq!(d.piece(), Some("character:catpope"));
        assert!(a.problems.is_empty(), "{a:?}");
        // the cell's own enter wins
        let b = cell(r#"{"mode":"own","layers":[{"id":"piece","enter":{"ms":1200},"src":{"kind":"piece","p":{"piece":"scene:nave"}}}]}"#);
        assert!(matches!(d.cell(&b, 200.0, nave).what, What::Load { ms: 1200, .. }));
    }

    #[test]
    fn not_ready_keeps_what_plays_and_says_why() {
        let mut d = Pieces::new();
        d.cell(&pc("scene:nave", "{}"), 0.0, nave);
        let a = d.cell(&pc("character:yellowmouse", r#"{"mood":"shy"}"#), 10.0, |_| None);
        assert_eq!(a.what, What::Kept);
        assert_eq!(a.problems, ["character:yellowmouse is not on this deck"]);
        assert_eq!(d.piece(), Some("scene:nave"), "what plays stays");
        let broken = d.cell(&pc("scene:broken", "{}"), 20.0, |_| Some(Found::Scene("[[node".into())));
        assert_eq!(broken.what, What::Kept);
        assert!(broken.problems[0].starts_with("scene:broken: "), "{broken:?}");
        let odd = d.cell(&pc("toon:catpope", "{}"), 30.0, |_| None);
        assert_eq!(odd.problems, ["toon:catpope: a piece of the kind 'toon' does not play on a deck"]);
        // values naming nothing: said, the rest plays
        let v = d.cell(&pc("scene:nave", r#"{"fog":1,"light.freq":330,"dim":"loud"}"#), 40.0, nave);
        assert!(matches!(&v.what, What::Glide { glides, .. } if glides.len() == 1 && glides[0].path == "light.freq"), "{v:?}");
        assert_eq!(v.problems, ["scene:nave: the value 'dim' is no number", "scene:nave: the value 'fog' names nothing it plays"]);
        assert!(d.log().any(|l| l == "character:yellowmouse is not on this deck"));
    }

    #[test]
    fn a_piece_plays_beside_the_decks_outlet() {
        let deck = crate::scene::OUTLET_SCENE;
        let text = beside(NAVE, deck, &[("light.gain".into(), 0.2)]).unwrap();
        let s = Scene::parse(&text).unwrap();
        let kinds: Vec<&str> = s.nodes.iter().map(|n| n.kind.as_str()).collect();
        assert_eq!(kinds, ["osc", "noise", "mixer", "audio_out", "tempo", "space", "outlet", "screen"], "the piece, then the outlet's");
        assert_eq!(s.stage.name, "nave", "the piece's stage");
        let light = s.nodes.iter().find(|n| n.id == "light").unwrap();
        assert_eq!(light.params.get("gain").and_then(|v| v.as_float()), Some(f64::from(0.2f32)), "a Load's value, set");
        let links: Vec<(&str, &str)> = s.links.iter().map(|l| (l.from.as_str(), l.to.as_str())).collect();
        assert!(links.contains(&("space", "outlet.in")) && links.contains(&("outlet", "screen.in")) && links.contains(&("mix", "out.in")), "{links:?}");
        assert!(crate::graph::Graph::build(&s).is_ok(), "it builds");
        // a rig's bare space plays in the deck's room
        let rig = "[stage]\nname = \"r\"\n[[node]]\nid = \"space\"\nkind = \"space\"\nspot = \"own\"\n";
        let s = Scene::parse(&beside(rig, deck, &[]).unwrap()).unwrap();
        let space = s.nodes.iter().find(|n| n.kind == "space").unwrap();
        assert_eq!((space.params.get("set").and_then(|v| v.as_str()), space.params.get("spot").and_then(|v| v.as_str())), (Some("cathedral"), Some("own")));
        assert_eq!(s.nodes.iter().filter(|n| n.kind == "space").count(), 1);
        assert_eq!(beside(NAVE, NAVE, &[]), Err("the deck's scene has no outlet".into()));
    }

    #[test]
    fn black_mirror_and_a_cell_without_a_piece() {
        let mut d = Pieces::new();
        assert_eq!(d.cell(&cell(r#"{"mode":"black"}"#), 0.0, nave).what, What::Same, "nothing plays: nothing to black");
        d.cell(&pc("scene:nave", "{}"), 0.0, nave);
        let b = d.cell(&cell(r#"{"mode":"black"}"#), 10.0, nave).what;
        assert!(matches!(&b, What::Black { ms: 500, scene } if !scene.is_empty()), "the silent scene fades in: {b:?}");
        assert_eq!(d.piece(), None);
        d.cell(&pc("scene:nave", "{}"), 20.0, nave);
        let m = d.cell(&cell(r#"{"mode":"mirror","of":"left"}"#), 30.0, nave);
        assert!(matches!(m.what, What::Black { ms: 500, .. }) && m.problems[0].starts_with("mirror of 'left'"), "{m:?}");
        d.cell(&pc("scene:nave", "{}"), 40.0, nave);
        let flat = cell(r##"{"mode":"own","layers":[{"id":"w","src":{"kind":"colour","p":{"colour":"#000"}}}]}"##);
        assert!(matches!(d.cell(&flat, 50.0, nave).what, What::Black { ms: 500, .. }), "a full cell without a piece: none plays");
        let mut log = LOG_CAP + 10;
        while log > 0 {
            d.cell(&cell(r#"{"mode":"black"}"#), 60.0, nave);
            log -= 1;
        }
        assert_eq!(d.log().count(), LOG_CAP, "the log is bounded");
    }
}
