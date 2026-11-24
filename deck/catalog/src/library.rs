//! The library (owner 2026-11-16 [design]): one list of what can be found, dragged and opened – things and files, each
//! with the preview its content asks for, and the one rule of what a query, a kind and "related" leave. The editor's
//! browser (deck-kit `library`) only draws it.
//!
//! What the list holds is the server's answer (`served`: GET /api/library, its kinds, names and pictures – one Library for
//! the creator and the deck). The toon's files: `family` – the files of one asset as one family with its versions and
//! frames –, `shelf` – what a slot takes, the selection, where an upload lands, and the creator's view – and its JSON for
//! the wasm bridge.
use crate::{Descriptor, Dir, Domain, PortType};
use serde::Serialize;
use serde_json::Value;

pub mod family;
pub mod open;
pub mod served;
pub mod shelf;
#[cfg(test)]
mod asset_tests;

/// what an entry is: a thing (the server's ThingType) or a file (its FileKind), in the Library's order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Toon,
    Character,
    Scene,
    Show,
    Place,
    Item,
    Module,
    Instrument,
    Rig,
    /// the storyboard's providers
    Generator,
    /// a Kanon text (one per text and language)
    Kanon,
    /// a person named in credits
    Person,
    Image,
    Audio,
    Video,
    Model,
    /// #817: a shared animation – a skeleton and clips, nothing drawn (the API's kind `animation`)
    Animation,
    File,
}

impl Kind {
    pub const ALL: [Kind; 18] = [
        Kind::Toon,
        Kind::Character,
        Kind::Scene,
        Kind::Show,
        Kind::Place,
        Kind::Item,
        Kind::Module,
        Kind::Instrument,
        Kind::Rig,
        Kind::Generator,
        Kind::Kanon,
        Kind::Person,
        Kind::Image,
        Kind::Audio,
        Kind::Video,
        Kind::Model,
        Kind::Animation,
        Kind::File,
    ];

    /// its facet's word
    pub fn word(self) -> &'static str {
        match self {
            Kind::Toon => "toons",
            Kind::Character => "characters",
            Kind::Scene => "scenes",
            Kind::Show => "shows",
            Kind::Place => "places",
            Kind::Item => "items",
            Kind::Module => "modules",
            Kind::Instrument => "instruments",
            Kind::Rig => "rigs",
            Kind::Generator => "generators",
            Kind::Kanon => "Kanon texts",
            Kind::Person => "people",
            Kind::Image => "images",
            Kind::Audio => "audio",
            Kind::Video => "video",
            Kind::Model => "3d models",
            Kind::Animation => "animations",
            Kind::File => "files",
        }
    }

    /// its id on the wire (the server's ThingType or FileKind)
    pub fn id(self) -> &'static str {
        match self {
            Kind::Toon => "toon",
            Kind::Character => "character",
            Kind::Scene => "scene",
            Kind::Show => "show",
            Kind::Place => "place",
            Kind::Item => "item",
            Kind::Module => "module",
            Kind::Instrument => "instrument",
            Kind::Rig => "rig",
            Kind::Generator => "generator",
            Kind::Kanon => "kanon",
            Kind::Person => "person",
            Kind::Image => "image",
            Kind::Audio => "audio",
            Kind::Video => "video",
            Kind::Model => "model",
            Kind::Animation => "animation",
            Kind::File => "file",
        }
    }

    /// a toon's file (what the shelf's families hold), not a thing
    pub fn is_file(self) -> bool {
        matches!(self, Kind::Image | Kind::Audio | Kind::Video | Kind::Model | Kind::Animation | Kind::File)
    }

    pub fn of_id(id: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.id() == id)
    }

    /// the kit's icon for it
    pub fn icon(self) -> &'static str {
        match self {
            Kind::Toon => "book",
            Kind::Character => "users",
            Kind::Scene => "layers",
            Kind::Show => "play",
            Kind::Place => "globe",
            Kind::Item => "package",
            Kind::Module => "code",
            Kind::Instrument => "sliders",
            Kind::Rig => "tree",
            Kind::Generator => "bolt",
            Kind::Kanon => "type",
            Kind::Person => "user",
            Kind::Image => "image",
            Kind::Audio => "activity",
            Kind::Video => "video",
            Kind::Model => "model",
            Kind::Animation => "activity",
            Kind::File => "entry",
        }
    }
}

impl Serialize for Kind {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.id())
    }
}

/// a node of a wiring preview: its place in 0..1 both ways and what it carries
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Dot {
    pub x: f32,
    pub y: f32,
    pub audio: bool,
}

/// what an entry's card shows, chosen by its content
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Preview {
    /// a still of what it draws, rendered once by the stage and kept under `key` (until then its kind)
    Still { key: String },
    /// its internal wiring: nodes in columns by depth, tubes between them (indices into `dots`)
    Graph { dots: Vec<Dot>, tubes: Vec<(usize, usize)> },
    /// code only: its parameters' names and its ports in and out
    Code { params: Vec<String>, ins: usize, outs: usize },
    /// a picture at this address
    Picture { url: String },
    /// nothing to show but its kind
    Kind,
}

/// one thing in the library
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// what a drop or an open is told (`module:walk`, `scene:nave`, `character:catpope`, an asset's path)
    pub address: String,
    pub label: String,
    /// one short line under the label
    pub detail: String,
    pub kind: Kind,
    pub preview: Preview,
    /// the characters it is made for (a module's `characters`: `catpope`, a persona `yellowmouse.kid`): "related" for those
    pub made_for: Vec<String>,
}

/// what the browser shows of a list: a query (every word in the label, the address or the detail – the typeahead's
/// rule), the kinds picked (none = all), and only what is made for a character
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Filter {
    pub query: String,
    pub kinds: Vec<Kind>,
    pub related: Option<String>,
}

/// where the browser stands decides what it loads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// a rig's tracks: modules and instruments to drop in, characters to open
    Build,
    /// a show's cells: scenes, characters and items as pieces
    Show,
    /// modules to open
    Module,
    /// a scene's slots in Build: the book's characters and items to fill them
    Scene,
}

impl Purpose {
    pub fn kinds(self) -> &'static [Kind] {
        match self {
            Purpose::Build => &[Kind::Module, Kind::Instrument, Kind::Character, Kind::Image, Kind::Audio, Kind::Video, Kind::Model, Kind::Animation],
            Purpose::Show => &[Kind::Scene, Kind::Character, Kind::Item, Kind::Place],
            Purpose::Module => &[Kind::Module],
            Purpose::Scene => &[Kind::Character, Kind::Item, Kind::Image, Kind::Module],
        }
    }

    /// where a drop lands in this workspace (none: entries are only opened here)
    pub fn view(self) -> Option<crate::drop::View> {
        match self {
            Purpose::Build => Some(crate::drop::View::Rig),
            Purpose::Show => Some(crate::drop::View::ShowRow),
            Purpose::Scene => Some(crate::drop::View::Scene),
            Purpose::Module => None,
        }
    }

    /// whether an entry of `kind` dropped here lands (where exactly is the target's to say), else the sentence why not:
    /// the drop plan's answer
    pub fn takes(self, kind: Kind) -> Result<(), String> {
        match self.view() {
            Some(view) => crate::drop::lands(view, kind).map(|_| ()),
            None => Err(format!("{} cannot be dropped here: a double click opens a module", kind.word())),
        }
    }
}

impl Purpose {
    /// what a toon's file dropped here must be (none: files are only opened here)
    pub fn fit(self) -> Option<shelf::Fit> {
        match self {
            Purpose::Scene => Some(shelf::Fit { kinds: vec![Kind::Image], square: false }),
            Purpose::Build | Purpose::Show | Purpose::Module => None,
        }
    }

    /// whether a family dropped here lands: its kind by `takes`, then the slot's fit, else the sentence why not
    pub fn takes_family(self, f: &family::Family) -> Result<(), String> {
        self.takes(f.kind)?;
        self.fit().map_or(Ok(()), |fit| fit.takes(f).map_err(|m| m.sentence()))
    }
}

impl Entry {
    /// a file's path (`image:cat/a.webp` → `cat/a.webp`); a thing: its address
    pub fn path(&self) -> &str {
        match self.address.split_once(':') {
            Some((kind, path)) if self.kind.is_file() && kind == self.kind.id() => path,
            _ => &self.address,
        }
    }

    /// made for `character` (`catpope`, a persona `catpope.biker`; a reference `character:catpope` too) by the
    /// descriptor's rule
    pub fn related_to(&self, character: &str) -> bool {
        crate::made_for(&self.made_for, character.strip_prefix("character:").unwrap_or(character))
    }

    fn found_by(&self, query: &str) -> bool {
        let hay = format!("{} {} {}", self.label, self.address, self.detail).to_lowercase();
        query.split_whitespace().all(|w| hay.contains(&w.to_lowercase()))
    }
}

/// the entries `f` leaves, in their order
pub fn find(entries: &[Entry], f: &Filter) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| f.kinds.is_empty() || f.kinds.contains(&e.kind))
        .filter(|(_, e)| f.related.as_deref().is_none_or(|c| e.related_to(c)))
        .filter(|(_, e)| e.found_by(&f.query))
        .map(|(i, _)| i)
        .collect()
}

/// the kinds there are, in the order of `Kind::ALL`, with how many of each
pub fn facets(entries: &[Entry]) -> Vec<(Kind, usize)> {
    Kind::ALL.iter().map(|&k| (k, entries.iter().filter(|e| e.kind == k).count())).filter(|(_, n)| *n > 0).collect()
}

/// A module's preview by what it is: one that draws (a picture out, the video domain, an image slot) a still of the
/// stage; one whose document carries a wiring (`graph`: nodes and links) that wiring; else its code's face.
pub fn module_preview(id: &str, d: &Descriptor, content: &Value) -> Preview {
    let draws = d.domain == Domain::Video || d.ports.iter().any(|p| p.ty == PortType::Video && p.dir == Dir::Out) || d.assets.iter().any(|a| a.kind == crate::AssetKind::Image);
    if draws {
        return Preview::Still { key: format!("module:{id}") };
    }
    if let Some(g) = wiring_of(&content["graph"]) {
        return g;
    }
    Preview::Code { params: d.params.iter().map(|p| p.name.to_string()).collect(), ins: d.inputs().count(), outs: d.outputs().count() }
}

/// a `{ nodes: [{ id, kind }], links: [{ from, to }] }` document as a wiring preview
fn wiring_of(v: &Value) -> Option<Preview> {
    let nodes: Vec<(String, bool)> = v["nodes"].as_array()?.iter().filter_map(|n| Some((n["id"].as_str()?.to_string(), n["domain"].as_str() == Some("audio")))).collect();
    let links: Vec<(String, String)> = v["links"].as_array().map(|l| l.iter().filter_map(|x| Some((x["from"].as_str()?.to_string(), x["to"].as_str()?.to_string()))).collect()).unwrap_or_default();
    (!nodes.is_empty()).then(|| graph(&nodes, &links))
}

/// A wiring as a picture: each node in the column of its depth (one more than the deepest node feeding it; a cycle
/// stops counting), spread down its column; a link names its ends `node.port` or `node`.
pub fn graph(nodes: &[(String, bool)], links: &[(String, String)]) -> Preview {
    let at = |end: &str| nodes.iter().position(|(id, _)| id == end.split('.').next().unwrap_or(end));
    let tubes: Vec<(usize, usize)> = links.iter().filter_map(|(a, b)| Some((at(a)?, at(b)?))).filter(|(a, b)| a != b).collect();
    let mut depth = vec![0usize; nodes.len()];
    for _ in 0..nodes.len() {
        let mut moved = false;
        for &(a, b) in &tubes {
            if depth[b] < depth[a] + 1 && depth[a] + 1 < nodes.len() {
                depth[b] = depth[a] + 1;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    let columns = depth.iter().max().map_or(1, |m| m + 1);
    let mut row = vec![0usize; columns];
    let mut place = vec![(0usize, 0usize); nodes.len()];
    for (i, &c) in depth.iter().enumerate() {
        place[i] = (c, row[c]);
        row[c] += 1;
    }
    let dots = nodes
        .iter()
        .zip(&place)
        .map(|((_, audio), &(c, r))| Dot { x: (c as f32 + 0.5) / columns as f32, y: (r as f32 + 0.5) / row[c].max(1) as f32, audio: *audio })
        .collect();
    Preview::Graph { dots, tubes }
}

/// `GET /api/catalog/docs/modules`'s answer: the modules not removed, each with its preview
pub fn modules(text: &str) -> Result<Vec<Entry>, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("the modules' answer: {e}"))?;
    let list = v["entries"].as_array().ok_or("the modules' answer has no entries")?;
    Ok(list.iter().filter(|e| e["removed"] != Value::Bool(true) && e["content"].is_object()).filter_map(module).collect())
}

/// one catalog entry of a module
pub fn module(e: &Value) -> Option<Entry> {
    let id = e["id"].as_str()?;
    let content = &e["content"];
    let d: Option<Descriptor> = serde_json::from_value(content["descriptor"].clone()).ok();
    let factory = e["factory"] == Value::Bool(true) || content["factory"] == Value::Bool(true);
    let state = if factory { "factory" } else if e["draft"] == Value::Bool(true) { "draft" } else { "" };
    let (preview, detail) = match &d {
        Some(d) => {
            let p = module_preview(id, d, content);
            let what = match &p {
                Preview::Still { .. } => "draws",
                Preview::Graph { .. } => "wired",
                _ => if d.outputs().any(|p| p.ty == PortType::Audio) { "sound" } else { "events" },
            };
            (p, [state, what].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" · "))
        }
        None => (Preview::Kind, [state, "not built"].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" · ")),
    };
    let made_for = d.as_ref().map(|d| d.characters.iter().map(|c| c.to_string()).collect()).unwrap_or_default();
    Some(Entry { address: format!("module:{id}"), label: id.to_string(), detail, kind: Kind::Module, preview, made_for })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn descriptor(v: Value) -> Descriptor {
        serde_json::from_value(v).unwrap()
    }

    fn entry(address: &str, kind: Kind, made_for: &[&str]) -> Entry {
        Entry { address: address.into(), label: address.split(':').nth(1).unwrap_or(address).into(), detail: String::new(), kind, preview: Preview::Kind, made_for: made_for.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn a_module_is_previewed_by_what_it_is() {
        let code = descriptor(json!({ "name": "walk", "domain": "control", "ports": [{ "name": "out", "dir": "out", "type": "control" }], "params": [{ "name": "rate", "min": 0.0, "max": 1.0, "default": 0.5 }, { "name": "depth", "min": 0.0, "max": 1.0, "default": 0.5 }] }));
        assert_eq!(module_preview("walk", &code, &json!({})), Preview::Code { params: vec!["rate".into(), "depth".into()], ins: 0, outs: 1 });
        let draws = descriptor(json!({ "name": "torso", "domain": "video", "ports": [{ "name": "out", "dir": "out", "type": "video" }] }));
        assert_eq!(module_preview("torso", &draws, &json!({})), Preview::Still { key: "module:torso".into() });
        let wired = json!({ "graph": { "nodes": [{ "id": "osc", "domain": "audio" }, { "id": "lfo" }, { "id": "out", "domain": "audio" }], "links": [{ "from": "lfo.out", "to": "osc.pitch" }, { "from": "osc", "to": "out.in" }] } });
        let Preview::Graph { dots, tubes } = module_preview("pad", &code, &wired) else { panic!("a wiring") };
        assert_eq!(tubes, [(1, 0), (0, 2)]);
        assert!(dots[1].x < dots[0].x && dots[0].x < dots[2].x, "a column per depth: {dots:?}");
        assert!(dots[0].audio && !dots[1].audio);
    }

    #[test]
    fn a_cycle_does_not_spin_the_columns() {
        let nodes = [("a".to_string(), false), ("b".to_string(), false)];
        let Preview::Graph { dots, .. } = graph(&nodes, &[("a".into(), "b".into()), ("b".into(), "a".into())]) else { panic!() };
        assert!(dots.iter().all(|d| (0.0..=1.0).contains(&d.x) && (0.0..=1.0).contains(&d.y)));
    }

    #[test]
    fn the_filter_takes_words_kinds_and_related() {
        let list = [entry("module:torso", Kind::Module, &["catpope"]), entry("module:wing", Kind::Module, &["yellow_mouse.kid"]), entry("scene:nave", Kind::Scene, &[]), entry("module:walk", Kind::Module, &[])];
        assert_eq!(find(&list, &Filter::default()), [0, 1, 2, 3]);
        assert_eq!(find(&list, &Filter { query: "MODULE w".into(), ..Filter::default() }), [1, 3], "every word, any case");
        assert_eq!(find(&list, &Filter { kinds: vec![Kind::Scene], ..Filter::default() }), [2]);
        assert_eq!(find(&list, &Filter { related: Some("character:catpope".into()), ..Filter::default() }), [0]);
        assert_eq!(find(&list, &Filter { related: Some("catpope.biker".into()), ..Filter::default() }), [0], "a persona of a character it names");
        assert_eq!(find(&list, &Filter { related: Some("yellow_mouse.kid".into()), ..Filter::default() }), [1], "the persona it names");
        assert_eq!(find(&list, &Filter { related: Some("yellow_mouse".into()), ..Filter::default() }), Vec::<usize>::new(), "not the whole character");
        assert_eq!(facets(&list), [(Kind::Scene, 1), (Kind::Module, 3)], "the Library's order: things by type");
    }

    #[test]
    fn a_drop_lands_where_its_kind_is_played_or_says_why_not() {
        assert_eq!(Purpose::Build.takes(Kind::Module), Ok(()));
        assert_eq!(Purpose::Show.takes(Kind::Scene), Ok(()));
        assert_eq!(Purpose::Build.takes(Kind::Scene), Err("scenes cannot be dropped on a rig: a show's row plays it, a rig plays modules and instruments".into()));
        assert_eq!(Purpose::Build.takes(Kind::Character), Err("characters on a rig become a world node: not built yet".into()), "planned, not landing yet");
        assert_eq!(Purpose::Show.takes(Kind::Place), Ok(()), "a place is a piece");
        assert_eq!(Purpose::Show.takes(Kind::Module), Err("modules cannot be dropped in a Show row: a row holds pieces: scenes, characters, items and places".into()));
        assert_eq!(Purpose::Module.takes(Kind::Module), Err("modules cannot be dropped here: a double click opens a module".into()));
        assert!(Purpose::Module.takes(Kind::Module).is_err());
        assert_eq!(Purpose::Scene.takes(Kind::Item), Ok(()));
        assert_eq!(Purpose::Scene.takes(Kind::Module), Ok(()), "a module lands on an entity's Modules slot");
        assert!(Purpose::Scene.takes(Kind::Instrument).is_err());
        assert_eq!(Purpose::Scene.takes(Kind::Image), Ok(()), "a picture fills a parallax layer");
    }

    #[test]
    fn the_catalogs_modules_become_entries() {
        let modules_answer = json!({ "entries": [
            { "id": "walk", "factory": true, "content": { "descriptor": { "name": "walk", "ports": [{ "name": "out", "dir": "out", "type": "audio" }] } } },
            { "id": "gone", "removed": true, "content": {} },
            { "id": "torso", "draft": true, "content": { "descriptor": { "name": "torso", "domain": "video", "characters": ["catpope"], "ports": [{ "name": "out", "dir": "out", "type": "video" }] } } },
        ] });
        let m = modules(&modules_answer.to_string()).unwrap();
        assert_eq!(m.iter().map(|e| (e.address.as_str(), e.detail.as_str())).collect::<Vec<_>>(), [("module:walk", "factory · sound"), ("module:torso", "draft · draws")]);
        assert!(m[1].related_to("catpope"));
    }
}
