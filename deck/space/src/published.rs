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
//! - **Cast while it plays** (#517): `Cast` is the cached scene as the core's playing scene (scene_play.rs) – the cast
//!   words of an `address` and the story events its listeners hear change who stands on it; the host loads what it resolves to.
//! - **A link of its own** (#520): `CastLink` – a deck without a space link (the browser deck) joins the session for its
//!   published scene as a deck of the session, asks for the readers' story while the scene listens and hands what comes
//!   to the `Cast`; the host drives the socket and loads the scene the cast changed to.
use crate::relay::{Client, In, Options, Out, DECKS};
use crate::rig::bundle::Bundle;
use crate::rig::content::{safe_id, SceneRigs};
use crate::scene::Scene;
use crate::scene_document::{FillerRef, SceneDocument};
use crate::scene_play::{Playing, Recast};
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

/// #517: a published scene playing on a deck that takes cast words and story events while it plays: the core's `Playing`
/// (scene_play.rs) of the host's cache file – the document and scene-content's answer as they came – with that answer's
/// rigs (else the deck's core rigs) and its location's set, as `Load` resolves them. When a word or an event changed the
/// cast, the host loads `playing().resolved()` in place of the scene before.
#[derive(Debug, Clone)]
pub struct Cast {
    playing: Playing,
    rigs: SceneRigs,
    sets: crate::push::Content,
}

impl Cast {
    /// the scene a host's cache file keeps (`SceneRef::cache_name`), playing as written: no use fills its placeholders
    pub fn from_cache(cached: &str) -> Result<Cast, String> {
        let k: Kept = serde_json::from_str(cached).map_err(|e| format!("the cached copy: {e}"))?;
        let doc = SceneDocument::from_value(k.document).map_err(|e| format!("scene:{}: the cached copy: {e:#}", k.scene))?;
        let rigs: SceneRigs = serde_json::from_value(k.content.clone()).unwrap_or_default();
        let sets: crate::push::Content = serde_json::from_value(k.content).unwrap_or_default();
        let rig = |subject: &str, version: Option<&str>| rig_of(&rigs, subject).or_else(|| core_rigs(subject, version));
        let set = |location: &str| sets.set_of(location);
        let playing = Playing::start(doc, BTreeMap::new(), &rig, &set).map_err(|problems| format!("scene:{}: {}", k.scene, problems.join("; ")))?;
        Ok(Cast { playing, rigs, sets })
    }

    /// what plays now, its log, whether it listens
    pub fn playing(&self) -> &Playing {
        &self.playing
    }

    /// what plays now as TOML (what a deck that loads a scene from text plays)
    pub fn text(&self) -> Result<String, String> {
        toml::to_string(&self.playing.resolved().scene).map_err(|e| format!("the resolved scene: {e}"))
    }

    /// an address action (the relay's `address`) on it
    pub fn word(&mut self, address: &str, action: &str) -> Recast {
        let (rigs, sets) = (&self.rigs, &self.sets);
        let rig = |subject: &str, version: Option<&str>| rig_of(rigs, subject).or_else(|| core_rigs(subject, version));
        let set = |location: &str| sets.set_of(location);
        self.playing.word(address, action, &rig, &set)
    }

    /// a story event (the VNEvent's JSON) against its listeners
    pub fn story(&mut self, event: &Value) -> Vec<Recast> {
        let (rigs, sets) = (&self.rigs, &self.sets);
        let rig = |subject: &str, version: Option<&str>| rig_of(rigs, subject).or_else(|| core_rigs(subject, version));
        let set = |location: &str| sets.set_of(location);
        self.playing.story(event, &rig, &set)
    }

    /// #520: what came for it through a link (`In::Address`: a cast word, `In::Story`: a story event), as a host hands it
    /// on; anything else is not the scene's
    pub fn came(&mut self, came: In) -> Vec<Recast> {
        match came {
            In::Address { address, action } => vec![self.word(&address, &action)],
            In::Story(event) => self.story(&event.into()),
            _ => Vec::new(),
        }
    }
}

/// #520: a published scene playing on a deck that has no space link (the browser deck): the deck joins the session for it
/// as a deck of the session (a musician of the decks, as the native deck's link joins) and plays the scene's `Cast`. Sans
/// IO: the host's socket drives the relay client (`relay`, `relay_at`, `take`), the host says which scene plays (`play`:
/// the loader's kept copy, `Fetched::kept`) and loads the scene the cast changed to (`changed`). While the scene listens
/// the client asks the session for the readers' story (`deck.story`); what comes for it goes to the cast as it came.
pub struct CastLink {
    relay: Client,
    cast: Option<Cast>,
    /// the host's clock at its last call (ms): the story's ask is renewed on it
    now: f64,
    /// the cast changed since the host last loaded it
    changed: bool,
    /// what the cast's words did, for the host's log, until it reads them
    said: Vec<String>,
}

impl CastLink {
    /// Join `relay` (ws:// or wss://…/live/<code>) as a deck of its session, named `nick` on `device`. None: no relay is
    /// named – the scene plays alone; Err: a relay that is no relay.
    pub fn join(relay: &str, nick: &str, device: &str) -> Option<Result<CastLink, String>> {
        let relay = relay.trim();
        if relay.is_empty() {
            return None;
        }
        let mut o = Options::new(relay, protocol::Role::Musician, Some(DECKS));
        o.nick = Some(nick.to_string()).filter(|n| !n.trim().is_empty());
        o.device = Some(device.to_string()).filter(|d| !d.is_empty());
        Some(Client::new(o).map(|relay| CastLink { relay, cast: None, now: 0.0, changed: false, said: Vec::new() }))
    }

    /// the relay client, for a call of the host's socket at its clock `now` (ms)
    pub fn relay_at(&mut self, now: f64) -> &mut Client {
        self.now = now;
        &mut self.relay
    }

    /// the relay client, for a call without the clock
    pub fn relay(&mut self) -> &mut Client {
        &mut self.relay
    }

    pub fn client(&self) -> &Client {
        &self.relay
    }

    /// The deck plays the published scene of this kept copy (the loader's, `Fetched::kept`); None: another scene, or no
    /// copy – no cast. Err: why the scene takes no cast words or story (it plays on as it is).
    pub fn play(&mut self, kept: Option<&str>) -> Result<(), String> {
        self.changed = false;
        let (cast, said) = match kept.map(Cast::from_cache).transpose() {
            Ok(cast) => (cast, Ok(())),
            Err(e) => (None, Err(e)),
        };
        self.cast = cast;
        self.relay.set_story(self.now, self.listens());
        said
    }

    /// the playing scene's cast, if it has one
    pub fn cast(&self) -> Option<&Cast> {
        self.cast.as_ref()
    }

    /// whether the playing scene listens to story events (the client asks for them while it does)
    pub fn listens(&self) -> bool {
        self.cast.as_ref().is_some_and(|c| c.playing().listens())
    }

    /// What the client asks of the host's socket and tells it (relay.rs `Out`); what came for the scene went to its cast
    /// first, and the session's other news is not the scene's.
    pub fn take(&mut self) -> Vec<Out> {
        let mut out = Vec::new();
        for o in self.relay.take() {
            match o {
                Out::Deck(came @ (In::Address { .. } | In::Story(_))) => self.came(came),
                Out::Deck(_) => {}
                o => out.push(o),
            }
        }
        out
    }

    fn came(&mut self, came: In) {
        let Some(cast) = self.cast.as_mut() else { return };
        for r in cast.came(came).into_iter().filter(|r| *r != Recast::NotMine) {
            self.said.push(format!("scene: {r:?} – {}", cast.playing().log().last().unwrap_or_default()));
            self.changed |= matches!(r, Recast::Changed { .. });
        }
    }

    /// the scene the cast changed to (TOML), once – the host loads it in place of the one before; Err: why it can't
    pub fn changed(&mut self) -> Option<Result<String, String>> {
        if !std::mem::take(&mut self.changed) {
            return None;
        }
        self.cast.as_ref().map(Cast::text)
    }

    /// what the cast's words did since the host last asked, for its log
    pub fn said(&mut self) -> Vec<String> {
        std::mem::take(&mut self.said)
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

    /// the template `scene:stage` as the loader keeps it (no sources: it names none) – with these listeners, or none
    fn stage_kept(listener: Option<Value>) -> String {
        let mut doc: Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/templates/stage.json"))).unwrap();
        if let Some(l) = listener {
            doc["listener"] = l;
        }
        let (mut load, _) = Load::start(SceneRef::parse("scene:stage").unwrap().unwrap(), None);
        play(load.answer(200, &published(&doc.to_string(), 3))).store.expect("the stage is kept")
    }

    /// a relay's envelope as the socket brings it
    fn envelope(seq: u64, from: &str, kind: &str, p: Value) -> String {
        serde_json::json!({ "v": 1, "seq": seq, "from": from, "kind": kind, "p": p }).to_string()
    }

    /// a deck's link, joined as `client` (its socket opened, the relay's welcome) – the kinds it sent
    fn joined(link: &mut CastLink, client: &str) -> Vec<String> {
        link.relay_at(0.0).start(0.0);
        link.relay_at(1.0).opened(1.0);
        link.relay_at(2.0).text(2.0, &envelope(1, "relay", "welcome", serde_json::json!({ "client": client, "snapshot": {}, "now": 0, "key": "k" })));
        sent(link)
    }

    /// the kinds of what the link sent since
    fn sent(link: &mut CastLink) -> Vec<String> {
        link.take()
            .into_iter()
            .filter_map(|o| match o {
                Out::Send(text) => serde_json::from_str::<Value>(&text).ok()?.get("kind")?.as_str().map(String::from),
                _ => None,
            })
            .collect()
    }

    fn standing(link: &CastLink) -> Vec<(String, String)> {
        link.cast().unwrap().playing().resolved().cast.iter().map(|m| (m.holder.clone(), m.place.clone())).collect()
    }

    /// #520 (plan E2c): the browser deck's link – a deck of the session for its published scene. A listening `scene:stage`
    /// asks for the story; a reader's `joinScene` with Cat Pope puts her at `left` and the changed scene is the host's to
    /// load; a scene without a listener asks nothing and a story changes nothing; the cast word `join:character:catpope`
    /// does the same as the event.
    #[test]
    fn a_link_of_its_own_takes_the_story_and_cast_words_into_the_playing_scene() {
        let join = serde_json::json!({ "type": "joinScene", "id": "ch3-e12", "params": { "characters": [{ "character": "catpope" }] } });
        let mut listens = CastLink::join("ws://127.0.0.1:7070/live/LISTEN", "listens", "browser").unwrap().unwrap();
        listens.play(Some(&stage_kept(Some(serde_json::json!([{ "on": "joinScene", "do": "join" }]))))).unwrap();
        assert!(listens.listens() && standing(&listens).is_empty(), "the stage plays empty");
        assert!(joined(&mut listens, "c1").contains(&"deck.story".to_string()), "it asks for the story once joined");
        listens.relay_at(3.0).text(3.0, &envelope(2, "r1", "story", join.clone()));
        let sent = sent(&mut listens);
        assert!(sent.is_empty(), "nothing goes out: {sent:?}");
        let scene = listens.changed().expect("the cast changed").unwrap();
        assert_eq!(standing(&listens), [("left".to_string(), "character:catpope".to_string())], "Cat Pope stands at left");
        assert!(scene.contains("left_look"), "the scene to load has her look at left");
        assert_eq!(listens.changed(), None, "once");
        assert!(listens.said()[0].starts_with("scene: Changed"));
        // the same event again: she stands there – nothing to load
        listens.relay_at(4.0).text(4.0, &envelope(3, "r1", "story", join.clone()));
        listens.take();
        assert_eq!(listens.changed(), None);

        // no listener: no ask, a story changes nothing
        let mut quiet = CastLink::join("ws://127.0.0.1:7070/live/LISTEN", "quiet", "browser").unwrap().unwrap();
        quiet.play(Some(&stage_kept(None))).unwrap();
        assert!(!quiet.listens());
        assert!(!joined(&mut quiet, "c2").contains(&"deck.story".to_string()), "a scene that does not listen asks nothing");
        quiet.relay_at(3.0).text(3.0, &envelope(2, "r1", "story", join));
        quiet.take();
        assert_eq!(quiet.changed(), None);
        assert!(standing(&quiet).is_empty(), "without a listener: nothing");

        // the cast word, as the conductor's `address` brings it: the same, and another scene's word is not its
        quiet.relay_at(5.0).text(5.0, &envelope(3, "cond", "address", serde_json::json!({ "id": "a1", "address": "scene:nave", "action": "join:character:catpope" })));
        quiet.take();
        assert_eq!(quiet.changed(), None, "another scene's word");
        quiet.relay_at(6.0).text(6.0, &envelope(4, "cond", "address", serde_json::json!({ "id": "a2", "address": "scene:stage", "action": "join:character:catpope" })));
        quiet.take();
        assert!(quiet.changed().is_some_and(|s| s.is_ok_and(|s| s.contains("left_look"))));
        assert_eq!(standing(&quiet), [("left".to_string(), "character:catpope".to_string())]);
        // another scene plays: no cast, the ask ends
        listens.play(None).unwrap();
        assert!(!listens.listens() && listens.cast().is_none());
    }

    #[test]
    fn a_link_joins_only_a_named_relay() {
        assert!(CastLink::join(" ", "x", "browser").is_none(), "no relay: the scene plays alone");
        assert!(matches!(CastLink::join("http://nowhere", "x", "browser"), Some(Err(_))));
        let mut link = CastLink::join("ws://127.0.0.1:7070/live/X", "", "browser").unwrap().unwrap();
        assert!(link.play(Some("not a kept copy")).is_err() && link.cast().is_none());
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
