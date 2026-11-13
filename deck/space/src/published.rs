//! Published scenes on the decks (#246, research/SCENES.md §6 2a / 2b): a deck plays `scene:<id>[@v]` from the API – the
//! one loader every deck drives (the native deck's `--scene` / deck.toml `scene`, the browser deck's `?scene=`). Sans IO:
//! the loader says what to ask (`Step::Ask`, the path after the API's base), the host fetches it and hands back the status
//! and the body (`Load::answer`, status 0 = the server can't be reached); what it plays and what the host keeps is the
//! loader's. The host only moves bytes: GETs and one cache file per scene (`SceneRef::cache_name`).
//!
//! - **The document:** `GET /scenes/<id>` (the published one) or `/scenes/<id>?v=<n>` (`@n`), `{ scene }` (server
//!   ScenesController); format 2 (scene_document.rs).
//! - **What it needs:** one `GET /library/scene-content?locations=…&characters=…&items=…` for its location's set and its
//!   fillers' looks and rigs (the cast of the core's resolve step, and each holder's pinned rig); none when it names none.
//! - **Resolved by the core:** `scene_resolve::resolve` with the answer's rigs (a rig not in it: the deck's core rig, else
//!   the member stands silent – said in the notes) and the location's set (`push::Content::set_of`). A deck plays what is
//!   written: no previews, a placeholder no use fills stays empty.
//! - **The cache, by version:** the document and scene-content's answer as they came, with the document's version
//!   (`Kept`); resolved again at each load. A pinned version the cache has is played without asking; the published one is
//!   asked for, and when its version is the cached one nothing else is fetched; a new version replaces the cached one
//!   (`Loaded::store`). The server can't be reached (or answers 5xx): the cached version plays, said in the notes.
//! - **What does not play** (unknown, not published, broken, offline with no copy): `Step::Fail` with the reason in the
//!   core's words – the host plays the test pattern and shows it.
//! - **Preloaded** (#513): the conductor's `scene.sync` (push.rs) and a flash's check (fetched.rs, deck-show flashed.rs)
//!   drive the same loader, so a scene is in the deck's cache before it plays; `listed` names a cached copy for a deck's
//!   list (F5) by its stage's name.
use crate::rig::bundle::Bundle;
use crate::rig::content::{safe_id, SceneRigs};
use crate::scene::Scene;
use crate::scene_document::{FillerRef, SceneDocument};
use crate::scene_resolve::{core_rigs, no_rigs, no_sets, resolve, Resolved};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// the scene argument's prefix that names a published scene
pub const PREFIX: &str = "scene:";
/// scene-content takes at most this many ids per kind
const MAX_IDS: usize = 32;

/// A published scene by reference: `scene:<id>` (the published version) or `scene:<id>@<n>` (that version).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SceneRef {
    pub id: String,
    pub version: Option<u32>,
}

impl SceneRef {
    /// What `arg` names: None = no published scene (no `scene:`); Err = a `scene:` reference that is broken.
    pub fn parse(arg: &str) -> Option<Result<SceneRef, String>> {
        let rest = arg.strip_prefix(PREFIX)?;
        let (id, version) = match rest.split_once('@') {
            Some((id, v)) => match v.parse::<u32>() {
                Ok(n) => (id, Some(n)),
                Err(_) => return Some(Err(format!("'{arg}': the version after @ is a number (scene:<id>@<n>)"))),
            },
            None => (rest, None),
        };
        if !safe_id(id) || id.contains('.') {
            return Some(Err(format!("'{arg}' names no scene id (scene:<id>[@<n>])")));
        }
        Some(Ok(SceneRef { id: id.to_string(), version }))
    }

    /// the document's path after the API's base
    pub fn path(&self) -> String {
        match self.version {
            Some(v) => format!("/scenes/{}?v={v}", self.id),
            None => format!("/scenes/{}", self.id),
        }
    }

    /// the host's cache file for this scene (one per scene: a new version replaces it)
    pub fn cache_name(&self) -> String {
        format!("{}.json", self.id)
    }
}

impl std::fmt::Display for SceneRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.version {
            Some(v) => write!(f, "{PREFIX}{}@{v}", self.id),
            None => write!(f, "{PREFIX}{}", self.id),
        }
    }
}

/// What the host keeps of a scene (its cache file): the document and scene-content's answer as they came.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Kept {
    scene: String,
    version: u32,
    document: Value,
    #[serde(default)]
    content: Value,
}

/// #513: what a host's cache file holds, for a deck's list of its scenes (the native deck's F5): the scene at the kept
/// version (`scene:<id>@<v>` – it plays without asking) and its stage's name (the id when it names none). None: no copy
/// of a scene.
pub fn listed(cached: &str) -> Option<(SceneRef, String)> {
    let k = serde_json::from_str::<Kept>(cached).ok().filter(|k| safe_id(&k.scene))?;
    let name = k.document.get("stage").and_then(|s| s.get("name")).and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty());
    let name = name.unwrap_or(&k.scene).to_string();
    Some((SceneRef { id: k.scene, version: Some(k.version) }, name))
}

/// What the loader wants next.
#[derive(Debug)]
pub enum Step {
    /// GET this path after the API's base, then `Load::answer`
    Ask(String),
    /// the scene plays
    Play(Box<Loaded>),
    /// it does not play: why, in the core's words
    Fail(String),
}

/// A published scene ready to play.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub scene: Scene,
    /// the document's version
    pub version: u32,
    /// what plays less than written, and where it came from (the cache while offline)
    pub notes: Vec<String>,
    /// it came from the cache (nothing new was fetched)
    pub cached: bool,
    /// the host writes this to its cache file (`SceneRef::cache_name`), replacing what was there; None = keep the file
    pub store: Option<String>,
}

impl Loaded {
    /// the scene as TOML (what a deck that builds from text plays: the browser)
    pub fn text(&self) -> Result<String, String> {
        toml::to_string(&self.scene).map_err(|e| format!("the resolved scene: {e}"))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Waits {
    Document,
    Content,
    Done,
}

/// One published scene being loaded.
#[derive(Debug, Clone)]
pub struct Load {
    r: SceneRef,
    cached: Option<Kept>,
    /// the fetched document, while its scene-content is asked for
    doc: Option<(Value, SceneDocument)>,
    waits: Waits,
}

impl Load {
    /// Start loading `r` with what the host's cache file holds (None: nothing, or no cache).
    pub fn start(r: SceneRef, cached: Option<&str>) -> (Load, Step) {
        // a cache file that does not read (or names another scene) is as none
        let cached = cached.and_then(|t| serde_json::from_str::<Kept>(t).ok()).filter(|k| k.scene == r.id);
        let load = Load { r, cached, doc: None, waits: Waits::Document };
        let step = match (&load.cached, load.r.version) {
            (Some(k), Some(v)) if k.version == v => load.cached_copy(Vec::new()),
            _ => Step::Ask(load.r.path()),
        };
        (load, step)
    }

    /// the reference being loaded
    pub fn reference(&self) -> &SceneRef {
        &self.r
    }

    /// The answer to the last `Step::Ask`: its HTTP status (0 = the server can't be reached) and body.
    pub fn answer(&mut self, status: u16, body: &str) -> Step {
        match std::mem::replace(&mut self.waits, Waits::Done) {
            Waits::Document => self.document(status, body),
            Waits::Content => self.content(status, body),
            Waits::Done => Step::Fail(format!("{}: an answer nobody asked for", self.r)),
        }
    }

    fn document(&mut self, status: u16, body: &str) -> Step {
        let r = &self.r;
        match status {
            200..=299 => {}
            0 | 500..=599 => return self.unreachable(if status == 0 { "the API can't be reached".into() } else { format!("the API answered HTTP {status}") }),
            404 if r.version.is_some() => return Step::Fail(format!("{r}: no such version is published")),
            404 => return Step::Fail(format!("{r} is not published")),
            code => return Step::Fail(format!("{r}: the API answered HTTP {code}")),
        }
        let value = match serde_json::from_str::<Value>(body).ok().and_then(|mut v| v.get_mut("scene").map(Value::take)) {
            Some(v) => v,
            None => return Step::Fail(format!("{r}: the API's answer holds no scene")),
        };
        let doc = match SceneDocument::from_value(value.clone()) {
            Ok(d) => d,
            Err(e) => return Step::Fail(format!("{r}: {e:#}")),
        };
        if doc.id != r.id {
            return Step::Fail(format!("{r}: the API answered scene:{}", doc.id));
        }
        let Some(version) = doc.version else { return Step::Fail(format!("{r}: the published document carries no version")) };
        // unchanged: what the cache has plays, nothing else is fetched
        if self.cached.as_ref().is_some_and(|k| k.version == version) {
            return self.cached_copy(Vec::new());
        }
        // what it needs, from the core's resolve step without rigs or sets (its validity: a broken scene says so now)
        let first = match resolve(&doc, &BTreeMap::new(), false, &no_rigs, &no_sets) {
            Ok(first) => first,
            Err(problems) => return Step::Fail(format!("{r}: {}", problems.join("; "))),
        };
        match content_path(&doc, &first) {
            Some(path) => {
                self.doc = Some((value, doc));
                self.waits = Waits::Content;
                Step::Ask(path)
            }
            None => self.played(value, &doc, Value::Null, Vec::new()),
        }
    }

    fn content(&mut self, status: u16, body: &str) -> Step {
        let Some((value, doc)) = self.doc.take() else { return Step::Fail(format!("{}: no document", self.r)) };
        let answer = (200..300).contains(&status).then(|| serde_json::from_str::<Value>(body).ok()).flatten();
        match answer {
            Some(content) => self.played(value, &doc, content, Vec::new()),
            None => {
                let why = match status {
                    0 => "the API can't be reached".to_string(),
                    200..=299 => "its answer is not JSON".to_string(),
                    code => format!("the API answered HTTP {code}"),
                };
                // a copy plays as it was; with none, the new version plays without its sources (not kept)
                if self.cached.is_some() && self.r.version.is_none() {
                    return self.cached_copy(vec![format!("scene-content: {why}")]);
                }
                let note = format!("scene-content: {why} – its rigs and set are not at hand");
                match play(&self.r, &doc, &Value::Null) {
                    Ok(resolved) => loaded(resolved, doc.version.unwrap_or_default(), vec![note], false, None),
                    Err(e) => Step::Fail(e),
                }
            }
        }
    }

    /// the server can't be reached: the cached version plays (the pinned one only when it is that)
    fn unreachable(&self, why: String) -> Step {
        let r = &self.r;
        match &self.cached {
            Some(k) if r.version.is_none_or(|v| v == k.version) => self.cached_copy(vec![format!("{why}: the cached version {} plays", k.version)]),
            _ => Step::Fail(format!("{r}: {why} and this deck has no copy")),
        }
    }

    /// the cached version, resolved again
    fn cached_copy(&self, notes: Vec<String>) -> Step {
        let Some(k) = &self.cached else { return Step::Fail(format!("{}: no copy", self.r)) };
        match SceneDocument::from_value(k.document.clone()).map_err(|e| format!("{}: the cached copy: {e:#}", self.r)).and_then(|doc| play(&self.r, &doc, &k.content)) {
            Ok(resolved) => loaded(resolved, k.version, notes, true, None),
            Err(e) => Step::Fail(e),
        }
    }

    /// a fetched version resolved, and kept when it plays
    fn played(&self, document: Value, doc: &SceneDocument, content: Value, notes: Vec<String>) -> Step {
        let version = doc.version.unwrap_or_default();
        match play(&self.r, doc, &content) {
            Ok(resolved) => {
                let kept = Kept { scene: self.r.id.clone(), version, document, content };
                loaded(resolved, version, notes, false, serde_json::to_string(&kept).ok())
            }
            Err(e) => Step::Fail(e),
        }
    }
}

fn loaded(resolved: Resolved, version: u32, mut notes: Vec<String>, cached: bool, store: Option<String>) -> Step {
    notes.extend(resolved.notes);
    Step::Play(Box::new(Loaded { scene: resolved.scene, version, notes, cached, store }))
}

/// The scene `doc` resolved with scene-content's answer `content` (Null: none): its rigs (else the deck's core rigs) and
/// its location's set.
fn play(r: &SceneRef, doc: &SceneDocument, content: &Value) -> Result<Resolved, String> {
    let rigs: SceneRigs = serde_json::from_value(content.clone()).unwrap_or_default();
    let sets: crate::push::Content = serde_json::from_value(content.clone()).unwrap_or_default();
    let rig = |subject: &str, version: Option<&str>| rig_of(&rigs, subject).or_else(|| core_rigs(subject, version));
    let set = |location: &str| sets.set_of(location);
    resolve(doc, &BTreeMap::new(), false, &rig, &set).map_err(|problems| format!("{r}: {}", problems.join("; ")))
}

/// The rig `subject` (`character.catpope`, `character.catpope.pope`, `item.incense_burner`) from scene-content's answer:
/// the entry filed as it, else the asked character's or item's (a persona's rig is its character's when it has none).
fn rig_of(answer: &SceneRigs, subject: &str) -> Option<Bundle> {
    let entry = answer.rigs().map(|(_, _, e)| e).find(|e| e.id == subject).or_else(|| {
        let (kind, id) = asked(subject)?;
        if kind == "character" { answer.character(&id) } else { answer.item(&id) }
    })?;
    answer.bundle(entry).ok()
}

/// a rig subject → the kind and id scene-content is asked with (`character.catpope.pope` → `catpope-pope`)
fn asked(subject: &str) -> Option<(&str, String)> {
    let (kind, rest) = subject.split_once('.')?;
    match kind {
        "character" => Some((kind, rest.replacen('.', "-", 1))),
        "item" => Some((kind, rest.to_string())),
        _ => None,
    }
}

/// scene-content's path for what `doc` needs (its location, its cast and each holder's pinned rig); None = nothing.
fn content_path(doc: &SceneDocument, first: &Resolved) -> Option<String> {
    let (mut characters, mut items) = (Vec::<String>::new(), Vec::<String>::new());
    let mut add = |kind: &str, id: String| {
        let list = if kind == "character" { &mut characters } else { &mut items };
        if safe_id(&id) && !list.contains(&id) && list.len() < MAX_IDS {
            list.push(id);
        }
    };
    for m in &first.cast {
        if let Some((kind, id)) = FillerRef::parse(&m.filler).ok().and_then(|f| f.rig_subject()).as_deref().and_then(asked) {
            add(kind, id);
        }
    }
    for pin in doc.holders.iter().filter_map(|h| h.rig.as_ref()?.subject()) {
        if let Some((kind, id)) = asked(pin) {
            add(kind, id);
        }
    }
    let location = first.location.as_deref().map(|l| l.strip_prefix("location:").unwrap_or(l)).filter(|l| safe_id(l)).unwrap_or("");
    if characters.is_empty() && items.is_empty() && location.is_empty() {
        return None;
    }
    Some(format!("/library/scene-content?sets=&locations={location}&characters={}&items={}", characters.join(","), items.join(",")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAVE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/nave.json"));
    /// scene-content as the server writes it (SceneContentRigsTest): catpope's rig, the incense burner's
    const RIGS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/rigs/scene-content.json"));
    /// scene-content with `?locations=cathedral` (#512)
    const LOCATION: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/scene-content-location.json"));

    /// the API's answer for a document at `version`
    fn published(doc: &str, version: u32) -> String {
        let mut v: Value = serde_json::from_str(doc).unwrap();
        v["version"] = version.into();
        serde_json::json!({ "status": "success", "scene": v }).to_string()
    }

    /// a scene at `location:cathedral` with Cat Pope (her rig) – a test scene of the brief
    fn at_cathedral() -> String {
        serde_json::json!({ "format": 2, "id": "vigil", "toons": ["catpope"], "location": "location:cathedral",
            "holder": [{ "id": "pope", "ref": "character:catpope" }],
            "stage": { "name": "Vigil" }, "node": [{ "id": "space", "kind": "space" }] })
        .to_string()
    }

    fn play(step: Step) -> Loaded {
        match step {
            Step::Play(l) => *l,
            other => panic!("not played: {other:?}"),
        }
    }

    fn ask(step: &Step) -> &str {
        match step {
            Step::Ask(p) => p,
            other => panic!("not an ask: {other:?}"),
        }
    }

    #[test]
    fn a_reference_names_a_published_scene_and_its_version() {
        assert_eq!(SceneRef::parse("scene:nave"), Some(Ok(SceneRef { id: "nave".into(), version: None })));
        assert_eq!(SceneRef::parse("scene:nave@3"), Some(Ok(SceneRef { id: "nave".into(), version: Some(3) })));
        assert_eq!(SceneRef::parse("idle"), None);
        assert_eq!(SceneRef::parse("rig:catpope"), None);
        for broken in ["scene:", "scene:../x", "scene:nave@x", "scene:a.b"] {
            assert!(matches!(SceneRef::parse(broken), Some(Err(_))), "{broken}");
        }
        let r = SceneRef::parse("scene:nave@3").unwrap().unwrap();
        assert_eq!((r.path(), r.cache_name(), r.to_string()), ("/scenes/nave?v=3".into(), "nave.json".into(), "scene:nave@3".into()));
    }

    #[test]
    fn nave_is_fetched_with_its_rig_resolved_and_kept() {
        let r = SceneRef::parse("scene:nave").unwrap().unwrap();
        let (mut load, step) = Load::start(r, None);
        assert_eq!(ask(&step), "/scenes/nave");
        let step = load.answer(200, &published(NAVE, 4));
        assert_eq!(ask(&step), "/library/scene-content?sets=&locations=&characters=catpope&items=");
        let l = play(load.answer(200, RIGS));
        assert_eq!((l.version, l.cached), (4, false));
        assert!(l.store.is_some(), "a fetched version is kept");
        // the rig played alone at the scene's own spot: its being is the space's, the set the document's
        assert_eq!(l.scene.being_character(), "catpope");
        let space = l.scene.nodes.iter().find(|n| n.kind == "space").unwrap();
        assert_eq!(space.params.get("set").and_then(|v| v.as_str()), Some("cathedral"));
        // the browser builds it from its text: the same graph (a knob's hover label is no scene key)
        let text = Scene::parse(&l.text().unwrap()).unwrap();
        assert_eq!((&text.nodes, &text.links, text.being_character()), (&l.scene.nodes, &l.scene.links, l.scene.being_character()));
    }

    #[test]
    fn a_location_s_set_comes_from_scene_content() {
        let r = SceneRef::parse("scene:vigil").unwrap().unwrap();
        let (mut load, _) = Load::start(r, None);
        let step = load.answer(200, &published(&at_cathedral(), 1));
        assert_eq!(ask(&step), "/library/scene-content?sets=&locations=cathedral&characters=catpope&items=");
        let l = play(load.answer(200, LOCATION));
        let space = l.scene.nodes.iter().find(|n| n.kind == "space").unwrap();
        assert_eq!(space.params.get("set").and_then(|v| v.as_str()), Some("cathedral"));
    }

    #[test]
    fn an_unchanged_version_is_not_fetched_again_and_a_new_one_replaces_it() {
        let r = SceneRef::parse("scene:nave").unwrap().unwrap();
        let (mut load, _) = Load::start(r.clone(), None);
        load.answer(200, &published(NAVE, 4));
        let kept = play(load.answer(200, RIGS)).store.unwrap();
        // the same version: only the document is asked, the copy plays, the file stays
        let (mut load, step) = Load::start(r.clone(), Some(&kept));
        assert_eq!(ask(&step), "/scenes/nave");
        let l = play(load.answer(200, &published(NAVE, 4)));
        assert_eq!((l.version, l.cached, l.store.is_none()), (4, true, true));
        // a pinned version the cache has: nothing is asked at all
        let (_, step) = Load::start(SceneRef { version: Some(4), ..r.clone() }, Some(&kept));
        assert_eq!(play(step).version, 4);
        // a new version: its sources are fetched and it replaces the copy
        let (mut load, _) = Load::start(r, Some(&kept));
        assert!(matches!(load.answer(200, &published(NAVE, 5)), Step::Ask(_)));
        let l = play(load.answer(200, RIGS));
        assert_eq!((l.version, l.cached), (5, false));
        assert!(l.store.unwrap().contains("\"version\":5"));
    }

    /// #513: a cache file names its scene at the kept version and the stage's name, for F5
    #[test]
    fn a_kept_copy_is_listed_by_its_stage_name() {
        let r = SceneRef::parse("scene:nave").unwrap().unwrap();
        let (mut load, _) = Load::start(r, None);
        load.answer(200, &published(NAVE, 4));
        let kept = play(load.answer(200, RIGS)).store.unwrap();
        let (r, name) = listed(&kept).unwrap();
        assert_eq!((r.to_string(), name.as_str()), ("scene:nave@4".to_string(), "Nave"));
        // the kept version plays without asking
        assert!(matches!(Load::start(r, Some(&kept)).1, Step::Play(_)));
        let (mut load, _) = Load::start(SceneRef::parse("scene:vigil").unwrap().unwrap(), None);
        let doc = at_cathedral().replace("\"name\":\"Vigil\"", "\"name\":\" \"");
        load.answer(200, &published(&doc, 1));
        assert_eq!(listed(&play(load.answer(200, LOCATION)).store.unwrap()).unwrap().1, "vigil", "no name: the id");
        assert!(listed("not json").is_none() && listed("{}").is_none());
    }

    #[test]
    fn offline_the_cached_version_plays() {
        let r = SceneRef::parse("scene:nave").unwrap().unwrap();
        let (mut load, _) = Load::start(r.clone(), None);
        load.answer(200, &published(NAVE, 4));
        let kept = play(load.answer(200, RIGS)).store.unwrap();
        for status in [0, 503] {
            let (mut load, _) = Load::start(r.clone(), Some(&kept));
            let l = play(load.answer(status, ""));
            assert!(l.cached && l.notes[0].contains("the cached version 4 plays"), "{:?}", l.notes);
            assert_eq!(l.scene.being_character(), "catpope", "its rig from the copy's answer");
        }
        // no copy, or not the pinned version: it says so
        let (mut load, _) = Load::start(r.clone(), None);
        let Step::Fail(why) = load.answer(0, "") else { panic!("no copy") };
        assert_eq!(why, "scene:nave: the API can't be reached and this deck has no copy");
        let (mut load, step) = Load::start(SceneRef { version: Some(3), ..r }, Some(&kept));
        assert_eq!(ask(&step), "/scenes/nave?v=3");
        assert!(matches!(load.answer(0, ""), Step::Fail(_)));
    }

    #[test]
    fn an_unknown_or_broken_scene_says_why() {
        let r = SceneRef::parse("scene:crypt").unwrap().unwrap();
        let (mut load, _) = Load::start(r.clone(), None);
        let Step::Fail(why) = load.answer(404, r#"{"status":"error"}"#) else { panic!() };
        assert_eq!(why, "scene:crypt is not published");
        // a holder whose filler its placeholder does not admit: the core's words
        let broken = serde_json::json!({ "status": "success", "scene": { "format": 2, "id": "crypt", "version": 1,
            "holder": [{ "id": "a", "ref": "character:catpope" }, { "id": "b", "ref": "character:catpope" }],
            "node": [{ "id": "space", "kind": "space" }] } });
        let (mut load, _) = Load::start(r.clone(), None);
        let Step::Fail(why) = load.answer(200, &broken.to_string()) else { panic!() };
        assert!(why.starts_with("scene:crypt: ") && why.contains("stands on this stage already"), "{why}");
        // another scene's document, or none
        let (mut load, _) = Load::start(r, None);
        assert!(matches!(load.answer(200, &published(NAVE, 1)), Step::Fail(w) if w.contains("answered scene:nave")));
    }
}
