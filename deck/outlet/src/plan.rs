//! A cell → stage nodes (#186, research/LIVE-LENSES.md §4): what each of a cell's layers becomes on this deck. Pure –
//! the composer (compose.rs) realises the plan. The mapping (LIVE-NIGHT §2.2 step 4):
//!
//! | Layer source | Stage node | Here |
//! |---|---|---|
//! | `scroll` | `chapter` (its slices running) | drawn into the outlet's plane, one slice per hold |
//! | `slice`, `panel` | `chapter` (one slice, a crop) | drawn into the plane, the dialog set in |
//! | `image`, `set` still | an image | drawn into the plane |
//! | `video`, `set` video | `video` (S4, #179 – not built) | the set spot's still stands in; else a HUD reason |
//! | `model3d`, `set` model3d | `space` (#188) | the scene's space node is the outlet's backdrop: its set, its spot |
//! | `text`, `surtitles` | `text` | drawn into the plane (Bangers, like the chapter's caption) |
//! | `colour` | a solid | drawn into the plane |
//! | `camera` | `video-in` (S4 – not built) | a HUD reason |
//!
//! Every layer that needs a module this build lacks, or a node the stage has not got yet, is a `problem` – the HUD
//! shows it, the rest of the cell plays (never a panic, never an exit).
use protocol::{Cell, CellMode, Fit, Layer, LayerBox, LayerSrc, SpotMode};
use std::collections::HashMap;

/// a layer's fade when the cell names none (ms)
pub const DEFAULT_FADE_MS: u32 = 600;
/// a running chapter's cut inside a cell (ms, like the chapter node's)
pub const SCROLL_FADE_MS: u32 = 350;

/// A medium a layer names: a preloaded item by id, or a set's spot in a mode (#193 – resolved by the adapter: deck-native's `media::Resolve`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MediaRef {
    Item(String),
    Set { set: String, spot: Option<String>, mode: SpotMode },
}

/// What one layer becomes (the stage node it maps to and what that node needs).
#[derive(Debug, Clone, PartialEq)]
pub enum LayerNode {
    /// `chapter`: the slices one after the other (scroll)
    Chapter { toon: String, chapter: u32 },
    /// `chapter` on one slice (reading-order index), a panel = a crop of it (0–1 of the slice)
    Slice { toon: String, chapter: u32, slice: usize, crop: Option<Rect> },
    /// a picture: a preloaded image, a set spot's still
    Image { media: MediaRef },
    /// `video` (S4 – not built yet): `still` stands in where the set has one
    Video { media: MediaRef, still: Option<MediaRef>, looped: bool },
    /// `space`: the set (in model3d mode, or a preloaded .glb) and the spot the camera goes to
    Space { set: MediaRef, spot: Option<String> },
    /// `text`: the line in the deck's locale
    Text { text: String, size: f32, colour: [u8; 4] },
    /// `text` following the show: the dialog of the slice `state.slice` names
    Surtitles { toon: String, chapter: u32, locale: String },
    /// a solid colour
    Solid { rgba: [u8; 4] },
    /// `video-in` (S4 – not built yet)
    VideoIn { input: String },
}

impl LayerNode {
    /// the stage module (cargo feature) the node needs, if not the outlet's own
    pub fn module(&self) -> Option<&'static str> {
        match self {
            LayerNode::Space { .. } => Some("space"),
            _ => None,
        }
    }

    /// why this node can't play on any build yet (the stage lacks the node)
    pub fn not_built(&self) -> Option<&'static str> {
        match self {
            LayerNode::Video { .. } => Some("the stage plays no video yet (S4, #179)"),
            LayerNode::VideoIn { .. } => Some("the stage has no video-in yet (S4, #179)"),
            _ => None,
        }
    }
}

/// A box in 0–1 of the outlet (x, y = the top left corner).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const FULL: Rect = Rect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };

    /// a protocol box (%) → 0–1, clamped to the outlet; None when it is empty
    pub fn from_box(b: &LayerBox) -> Option<Rect> {
        let x = (b.x as f32 / 100.0).clamp(0.0, 1.0);
        let y = (b.y as f32 / 100.0).clamp(0.0, 1.0);
        let w = (b.w as f32 / 100.0).clamp(0.0, 1.0 - x);
        let h = (b.h as f32 / 100.0).clamp(0.0, 1.0 - y);
        (w > 0.0 && h > 0.0).then_some(Rect { x, y, w, h })
    }
}

/// One planned layer.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub id: String,
    pub node: LayerNode,
    pub rect: Rect,
    pub fit: Fit,
    pub opacity: f32,
    pub enter_ms: u32,
    pub exit_ms: u32,
}

/// A cell as this deck plays it: its layers back → front, and what it can't play (with the layer's id).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Plan {
    pub layers: Vec<Planned>,
    pub problems: Vec<String>,
    /// #91: the cell holds a show's piece – the deck plays it (its space is the outlet's backdrop, like a 3D set)
    pub piece: bool,
}

impl Plan {
    /// the space layer (the outlet's backdrop) – the first one; a second is a problem (one space per scene)
    pub fn space(&self) -> Option<&Planned> {
        self.layers.iter().find(|l| matches!(l.node, LayerNode::Space { .. }))
    }

    /// the cell's fade in / out: the longest of its layers' (a cell with no layers fades out the old one)
    pub fn fades(&self) -> (u32, u32) {
        let enter = self.layers.iter().map(|l| l.enter_ms).max().unwrap_or(DEFAULT_FADE_MS);
        let exit = self.layers.iter().map(|l| l.exit_ms).max().unwrap_or(DEFAULT_FADE_MS);
        (enter, exit)
    }

    pub fn scrolls(&self) -> bool {
        self.layers.iter().any(|l| matches!(l.node, LayerNode::Chapter { .. }))
    }

    pub fn follows_the_slice(&self) -> bool {
        self.layers.iter().any(|l| matches!(l.node, LayerNode::Surtitles { .. }))
    }
}

/// The text of a locale map in `locale`, else English, else German, else the first key in order.
pub fn pick_locale<'a>(text: &'a HashMap<String, String>, locale: &str) -> Option<&'a str> {
    [locale, "en", "de"].iter().find_map(|l| text.get(*l)).or_else(|| text.iter().min_by(|a, b| a.0.cmp(b.0)).map(|(_, v)| v)).map(String::as_str)
}

/// `#rgb`, `#rrggbb`, `#rrggbbaa` (the toon's colours)
pub fn parse_colour(s: &str) -> Option<[u8; 4]> {
    let h = s.trim().strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        3 => {
            let d = |i: usize| u8::from_str_radix(h.get(i..i + 1)?, 16).ok().map(|v| v * 17);
            Some([d(0)?, d(1)?, d(2)?, 255])
        }
        6 => Some([byte(0)?, byte(2)?, byte(4)?, 255]),
        8 => Some([byte(0)?, byte(2)?, byte(4)?, byte(6)?]),
        _ => None,
    }
}

/// The plan of `cell` for this deck (its `locale`; `carried` = whether this build carries a module).
pub fn plan(cell: &Cell, locale: &str, carried: impl Fn(&str) -> bool) -> Plan {
    let mut out = Plan::default();
    match cell.mode {
        CellMode::Black => return out,
        CellMode::Mirror => {
            let of = cell.of.as_deref().unwrap_or("?");
            out.problems.push(format!("mirror of '{of}': the conductor sends the cell itself – black here"));
            return out;
        }
        CellMode::Own => {}
    }
    for layer in cell.layers.iter().flatten() {
        if matches!(layer.src, LayerSrc::Piece(_)) {
            out.piece = true;
            continue;
        }
        let Some(node) = node_of(layer, locale, &mut out.problems) else { continue };
        if let Some(m) = node.module().filter(|m| !carried(m)) {
            out.problems.push(format!("{}: this build does not carry the module '{m}' (--features {m})", layer.id));
            continue;
        }
        if matches!(node, LayerNode::Space { .. }) && out.space().is_some() {
            out.problems.push(format!("{}: one 3D set per cell – the first one plays", layer.id));
            continue;
        }
        if let Some(why) = node.not_built() {
            let still = matches!(&node, LayerNode::Video { still: Some(_), .. });
            out.problems.push(format!("{}: {why}{}", layer.id, if still { " – its still instead" } else { "" }));
        }
        let rect = layer.r#box.as_ref().map_or(Some(Rect::FULL), Rect::from_box);
        let Some(rect) = rect else {
            out.problems.push(format!("{}: an empty box", layer.id));
            continue;
        };
        out.layers.push(Planned {
            id: layer.id.clone(),
            node,
            rect,
            fit: layer.fit.unwrap_or(Fit::Cover),
            opacity: layer.opacity.map_or(1.0, |o| o.clamp(0.0, 1.0) as f32),
            enter_ms: layer.enter.as_ref().map_or(DEFAULT_FADE_MS, |t| t.ms.min(60_000) as u32),
            exit_ms: layer.exit.as_ref().map_or(DEFAULT_FADE_MS, |t| t.ms.min(60_000) as u32),
        });
    }
    out
}

fn node_of(layer: &Layer, locale: &str, problems: &mut Vec<String>) -> Option<LayerNode> {
    let chapter = |n: u64| n.min(u32::MAX as u64) as u32;
    Some(match &layer.src {
        LayerSrc::Scroll(c) => LayerNode::Chapter { toon: c.toon.clone(), chapter: chapter(c.chapter) },
        LayerSrc::Slice(s) => LayerNode::Slice { toon: s.toon.clone(), chapter: chapter(s.chapter), slice: s.slice as usize, crop: None },
        LayerSrc::Panel(p) => {
            let Some(crop) = Rect::from_box(&p.crop) else {
                problems.push(format!("{}: an empty crop", layer.id));
                return None;
            };
            LayerNode::Slice { toon: p.toon.clone(), chapter: chapter(p.chapter), slice: p.slice as usize, crop: Some(crop) }
        }
        LayerSrc::Surtitles(s) => LayerNode::Surtitles { toon: s.toon.clone(), chapter: chapter(s.chapter), locale: s.locale.clone() },
        LayerSrc::Image(m) => LayerNode::Image { media: MediaRef::Item(m.media.clone()) },
        LayerSrc::Video(m) => LayerNode::Video { media: MediaRef::Item(m.media.clone()), still: None, looped: layer.r#loop.unwrap_or(false) },
        LayerSrc::Model3d(m) => LayerNode::Space { set: MediaRef::Item(m.media.clone()), spot: None },
        LayerSrc::Set(s) => {
            let at = |mode| MediaRef::Set { set: s.set.clone(), spot: s.spot.clone(), mode };
            match s.mode {
                SpotMode::Still => LayerNode::Image { media: at(SpotMode::Still) },
                SpotMode::Video => LayerNode::Video { media: at(SpotMode::Video), still: Some(at(SpotMode::Still)), looped: layer.r#loop.unwrap_or(true) },
                SpotMode::Model3d => LayerNode::Space { set: at(SpotMode::Model3d), spot: s.spot.clone() },
            }
        }
        LayerSrc::Text(t) => {
            let Some(text) = pick_locale(&t.text, locale) else {
                problems.push(format!("{}: a text without any language", layer.id));
                return None;
            };
            let colour = t.colour.as_deref().and_then(parse_colour).unwrap_or([255; 4]);
            LayerNode::Text { text: text.to_owned(), size: t.size.map_or(6.0, |s| s.clamp(1.0, 50.0) as f32), colour }
        }
        LayerSrc::Colour(c) => match parse_colour(&c.colour) {
            Some(rgba) => LayerNode::Solid { rgba },
            None => {
                problems.push(format!("{}: '{}' is no colour", layer.id, c.colour));
                return None;
            }
        },
        LayerSrc::Camera(c) => LayerNode::VideoIn { input: c.input.clone() },
        // a show's cell (#89): its piece is the deck's sound and space, not the plane's – the outlet hands the cell to the
        // deck's piece side (route.rs `Routed::Piece` → deck-space piece.rs, #91); its reasons are that side's
        LayerSrc::Piece(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(json: &str) -> Cell {
        serde_json::from_str(json).unwrap()
    }

    /// the sermon (research/LIVE-LENSES.md §5, step 3): the cathedral as the space at the nave's centre, a line of the
    /// sermon, a panel of the comic, the choir's video, a colour wash
    const SERMON: &str = r##"{ "mode": "own", "layers": [
        { "id": "set", "src": { "kind": "set", "p": { "set": "cathedral", "spot": "nave-centre", "mode": "model3d" } }, "enter": { "ms": 1200 } },
        { "id": "line", "src": { "kind": "text", "p": { "text": { "de": "Liebe Gemeinde", "en": "Dear congregation" }, "size": 7.5 } },
          "box": { "x": 10, "y": 75, "w": 80, "h": 15 }, "exit": { "ms": 300 } },
        { "id": "panel", "src": { "kind": "panel", "p": { "toon": "catpope", "chapter": 2, "slice": 41, "crop": { "x": 0, "y": 20, "w": 100, "h": 40 } } },
          "box": { "x": 60, "y": 5, "w": 35, "h": 40 }, "fit": "contain", "opacity": 0.8 },
        { "id": "choir", "src": { "kind": "set", "p": { "set": "cathedral", "spot": "choir", "mode": "video" } } },
        { "id": "wash", "src": { "kind": "colour", "p": { "colour": "#1a0b2e80" } } }
    ] }"##;

    #[test]
    fn a_shows_piece_is_not_the_planes() {
        let p = plan(&cell(r#"{ "mode": "own", "layers": [{ "id": "piece", "src": { "kind": "piece", "p": { "piece": "scene:nave", "values": { "light": 0.2 } } } }] }"#), "de", |_| true);
        assert!(p.layers.is_empty() && p.problems.is_empty() && p.piece, "the piece side plays it (route.rs): {p:?}");
    }

    #[test]
    fn the_sermon_maps_to_space_text_chapter_video_and_a_solid() {
        let p = plan(&cell(SERMON), "de", |_| true);
        let kinds: Vec<&str> = p
            .layers
            .iter()
            .map(|l| match l.node {
                LayerNode::Space { .. } => "space",
                LayerNode::Text { .. } => "text",
                LayerNode::Slice { .. } => "chapter",
                LayerNode::Video { .. } => "video",
                LayerNode::Solid { .. } => "solid",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, ["space", "text", "chapter", "video", "solid"]);
        let space = p.space().unwrap();
        assert_eq!(
            space.node,
            LayerNode::Space { set: MediaRef::Set { set: "cathedral".into(), spot: Some("nave-centre".into()), mode: SpotMode::Model3d }, spot: Some("nave-centre".into()) }
        );
        assert!(matches!(&p.layers[1].node, LayerNode::Text { text, size, colour } if text == "Liebe Gemeinde" && *size == 7.5 && *colour == [255; 4]));
        assert_eq!(p.layers[1].rect, Rect { x: 0.1, y: 0.75, w: 0.8, h: 0.15 });
        let LayerNode::Slice { slice, crop: Some(crop), .. } = &p.layers[2].node else { panic!() };
        assert_eq!((*slice, crop.y, crop.h), (41, 0.2, 0.4));
        assert_eq!((p.layers[2].fit, p.layers[2].opacity), (Fit::Contain, 0.8));
        let LayerNode::Video { still: Some(MediaRef::Set { mode: SpotMode::Still, spot, .. }), looped: true, .. } = &p.layers[3].node else { panic!() };
        assert_eq!(spot.as_deref(), Some("choir"));
        assert_eq!(p.layers[4].node, LayerNode::Solid { rgba: [0x1a, 0x0b, 0x2e, 0x80] });
        assert_eq!(p.fades(), (1200, DEFAULT_FADE_MS), "the longest enter / exit");
        assert_eq!(p.problems, ["choir: the stage plays no video yet (S4, #179) – its still instead"]);
        // English deck
        let en = plan(&cell(SERMON), "en", |_| true);
        assert!(matches!(&en.layers[1].node, LayerNode::Text { text, .. } if text == "Dear congregation"));
    }

    #[test]
    fn a_missing_module_or_node_is_a_problem_not_a_panic() {
        let p = plan(&cell(SERMON), "de", |m| m != "space");
        assert!(p.space().is_none());
        assert_eq!(p.problems[0], "set: this build does not carry the module 'space' (--features space)");
        assert_eq!(p.layers.len(), 4, "the rest of the cell plays");
        let cam = cell(r##"{ "mode": "own", "layers": [ { "id": "cam", "src": { "kind": "camera", "p": { "input": "usb-0" } } },
            { "id": "bad", "src": { "kind": "colour", "p": { "colour": "teal" } } },
            { "id": "tiny", "src": { "kind": "colour", "p": { "colour": "#fff" } }, "box": { "x": 100, "y": 0, "w": 10, "h": 10 } } ] }"##);
        let p = plan(&cam, "de", |_| true);
        assert_eq!(p.problems, ["cam: the stage has no video-in yet (S4, #179)", "bad: 'teal' is no colour", "tiny: an empty box"]);
    }

    #[test]
    fn black_mirror_scroll_and_surtitles() {
        assert_eq!(plan(&cell(r#"{"mode":"black"}"#), "de", |_| true), Plan::default());
        let m = plan(&cell(r#"{"mode":"mirror","of":"left"}"#), "de", |_| true);
        assert!(m.layers.is_empty() && m.problems[0].starts_with("mirror of 'left'"));
        let s = plan(
            &cell(r#"{"mode":"own","layers":[{"id":"s","src":{"kind":"scroll","p":{"toon":"catpope","chapter":1}}},
                {"id":"t","src":{"kind":"surtitles","p":{"toon":"catpope","chapter":1,"locale":"en"}}}]}"#),
            "de",
            |_| true,
        );
        assert!(s.scrolls() && s.follows_the_slice());
        assert_eq!(s.fades(), (DEFAULT_FADE_MS, DEFAULT_FADE_MS));
        assert_eq!(plan(&cell(r#"{"mode":"own","layers":[]}"#), "de", |_| true).fades(), (DEFAULT_FADE_MS, DEFAULT_FADE_MS));
    }

    /// the runnable sermon (deck-native's examples/sermon.json, LIVE-LENSES.md §5): every step plans; only the choir's video waits for S4
    #[test]
    fn the_sermon_script_plans() {
        let script: serde_json::Value = serde_json::from_str(include_str!("../../native/examples/sermon.json")).unwrap();
        let mut problems = vec![];
        for step in script["cells"].as_array().unwrap() {
            let c: Cell = serde_json::from_value(step["cell"].clone()).unwrap();
            let p = plan(&c, "de", |_| true);
            assert!(!p.layers.is_empty());
            problems.extend(p.problems);
        }
        assert_eq!(problems, ["choir: the stage plays no video yet (S4, #179)"]);
    }

    #[test]
    fn locales_and_colours() {
        let m: HashMap<String, String> = [("fr", "Bonjour"), ("de", "Hallo")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        assert_eq!(pick_locale(&m, "it"), Some("Hallo"), "no it, no en → de");
        assert_eq!(pick_locale(&m, "fr"), Some("Bonjour"));
        let only: HashMap<String, String> = [("ja".to_string(), "こんにちは".to_string())].into();
        assert_eq!(pick_locale(&only, "de"), Some("こんにちは"));
        assert_eq!(parse_colour("#0f8"), Some([0, 255, 136, 255]));
        assert_eq!(parse_colour("#00ff8840"), Some([0, 255, 136, 64]));
        assert_eq!(parse_colour("00ff88"), None);
    }
}
