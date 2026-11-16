//! The deck's answer to the conductor's push (#195, #371): one sans-IO state machine that every deck drives – the native
//! deck's sync worker (deck-native sync.rs: its threads, cache and HTTP), the browser deck's fetch loop (space-web
//! `DeckPush` under live-deck sync.ts) and the headless host. The core decides; an adapter only moves bytes.
//!
//! - **Items** (`items_of`): each published scene the push names (`scene:<id>[@<v>]`, #513) as written, then `set:<id>`
//!   for each set, then `character:<id>`.
//! - **Published scenes first** (#513): each is loaded by the core's loader (published.rs; the native deck drives it
//!   with its cache, `Run` without one) – its item is ready when it loaded, missing with the loader's reason when not –
//!   and its sets and characters join the push's (`with_scenes`, `scene_needs`), so its media come as any set's and
//!   character's. Nothing plays: a push only fills the cache.
//! - **The request** (`content_url`): one `GET <api>/library/scene-content?sets=…&characters=…` (server
//!   SceneContent.php); its answer (`Content`, merged with what the deck already knew) is the plan's input.
//! - **The plan** (`Push::plan`): a set's 3D at the deck's quality (`low` when it names none), a character's every
//!   portrait (by mood, in order); or why an item can't be had – unknown to the API, no 3D at that quality, or the API
//!   wasn't reached and the deck's cache (`Cache`) has no copy.
//! - **Progress** (`Push::settle`, `next_fetch`): each file settles once – fetched whole (its size and type checked:
//!   `check`), already in the cache, failed (an HTTP status, a refused file) or offline.
//! - **The answer** (`Push::answer`, `deck.synced`): ready / missing per item with the why, files done / total, bytes
//!   fetched; `syncing` while a file is pending, then `ready`, `offline` (the network was gone) or `missing`.
//! - **The line** (`push_line`): `PUSH 2/5 CATHEDRAL`, `PUSH READY …`, `PUSH MISSING 1 …`, `PUSH OFFLINE …` (≤ 40).
//!
//! `Run` is the flow of a deck that asks once and fetches file by file (the browser, the headless host); `script` runs a
//! scripted push through it – the answers every adapter's test compares against.
use crate::graph::{Graph, Kind};
use crate::published::{Load, SceneRef, Step};
use crate::scene::Scene;
use protocol::{DeckSynced, SceneSync, SyncState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

/// the 3D quality a set is fetched at when the deck names none
pub const DEFAULT_QUALITY: &str = "low";
/// the largest file a deck takes
pub const MAX_FILE: u64 = 256 << 20;
/// the bytes of a file's start `check` reads (the longest magic: WebP's 12)
pub const HEAD: usize = 12;
/// the HUD cuts its lines here
const LINE_MAX: usize = 40;

/// one file a deck's stage wants
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Want {
    Set { id: String, quality: String },
    Portrait { character: String, mood: String },
    /// a file named by its CDN path (a stage background's image): immutable, so the path is its version
    Media { path: String },
}

/// The API's answer (SceneContent.php): each set's media per quality, each location's set (#512), each character's
/// portraits per mood, the ids it doesn't know.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Content {
    #[serde(default)]
    pub sets: BTreeMap<String, SetContent>,
    /// a place id → the set it names (`?locations=`): stored configuration, what `scene_resolve::resolve` asks through `set_of`
    #[serde(default)]
    pub locations: BTreeMap<String, LocationContent>,
    #[serde(default)]
    pub characters: BTreeMap<String, CharacterContent>,
    #[serde(default)]
    pub missing: Missing,
}

/// A set's media per rung of the render ladder (research/SCENES.md §9.4), each quality → its file.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct SetContent {
    /// rung 1: quality → its 3D file (`.glb`)
    #[serde(default)]
    pub model: BTreeMap<String, File>,
    /// rung 2: quality → its video loop
    #[serde(default)]
    pub video: BTreeMap<String, File>,
    /// rung 3: quality → its still (the video's poster)
    #[serde(default)]
    pub still: BTreeMap<String, File>,
}

/// A location as the answer names it: its set (none = it names none, the neutral staging plays).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct LocationContent {
    #[serde(default)]
    pub set: Option<String>,
}

/// A set's file per rung at one quality (`Content::media`); a rung the set lacks is none.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SetMedia<'a> {
    pub model: Option<&'a File>,
    pub video: Option<&'a File>,
    pub still: Option<&'a File>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct CharacterContent {
    /// mood → its portrait
    #[serde(default)]
    pub portraits: BTreeMap<String, File>,
}

/// a file on the CDN (immutable: a changed file gets a new path, so the path is the version)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct File {
    /// from the CDN root
    pub path: String,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Missing {
    #[serde(default)]
    pub sets: Vec<String>,
    /// places with no published document (#512)
    #[serde(default)]
    pub locations: Vec<String>,
    #[serde(default)]
    pub characters: Vec<String>,
}

impl Content {
    /// the API's answer text
    pub fn parse(text: &str) -> Result<Content, String> {
        serde_json::from_str(text).map_err(|e| format!("not an answer: {e}"))
    }

    /// a later answer over what the deck knew
    pub fn merge(&mut self, c: Content) {
        self.sets.extend(c.sets);
        self.locations.extend(c.locations);
        self.characters.extend(c.characters);
        self.missing.sets.extend(c.missing.sets);
        self.missing.locations.extend(c.missing.locations);
        self.missing.characters.extend(c.missing.characters);
    }

    /// the set a location names (`cathedral` for the place `cathedral`); none = it names none, or the answer lacks it –
    /// `scene_resolve::resolve`'s `Sets` (the lookup is the core's; the server only stores and serves the field)
    pub fn set_of(&self, location: &str) -> Option<String> {
        self.locations.get(location).and_then(|l| l.set.clone())
    }

    /// a set's file per rung at `quality` (rung 1 the `.glb`, 2 the video loop, 3 the still); none = the answer lacks the set
    pub fn media(&self, set: &str, quality: &str) -> Option<SetMedia<'_>> {
        self.sets.get(set).map(|s| SetMedia { model: s.model.get(quality), video: s.video.get(quality), still: s.still.get(quality) })
    }

    /// What the API named for `want`: its file; None – not answered (the cache's copy counts), or a mood without a
    /// sprite (the head shows its nearest); or why it can't be had.
    pub fn file(&self, want: &Want) -> Result<Option<&File>, String> {
        match want {
            Want::Set { id, quality } => match self.sets.get(id) {
                Some(s) => s.model.get(quality).map(Some).ok_or_else(|| format!("set {id} has no 3D at {quality}")),
                None if self.missing.sets.contains(id) => Err(format!("set {id} is unknown to the API")),
                None => Ok(None),
            },
            Want::Portrait { character, mood } => match self.characters.get(character) {
                Some(c) => Ok(c.portraits.get(mood)),
                None if self.missing.characters.contains(character) => Err(format!("character {character} is unknown to the API")),
                None => Ok(None),
            },
            Want::Media { .. } => Ok(None),
        }
    }
}

/// how the deck's last request to the API went
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Api {
    #[default]
    Answered,
    /// the network is gone
    Offline,
    /// an HTTP status, or an answer that isn't one
    Failed(String),
}

impl Api {
    /// the API's answer by its HTTP status (0: the network is gone) and body; `Ok` = its content
    pub fn reply(status: u16, body: &str) -> Result<Content, Api> {
        match status {
            0 => Err(Api::Offline),
            200..=299 => Content::parse(body).map_err(Api::Failed),
            code => Err(Api::Failed(format!("API HTTP {code}"))),
        }
    }
}

/// What the deck's own cache holds (the native deck's files on DECKDATA; the browser holds nothing: `NoCache`).
pub trait Cache {
    /// the file is there to play (a kind the deck doesn't sync – a local folder – always is)
    fn has(&self, want: &Want) -> bool;
    /// the moods of `character` the cache holds (what a deck that can't reach the API can still count on)
    fn moods(&self, character: &str) -> Vec<String>;
}

/// a deck without a cache of its own (the browser fetches through the HTTP cache)
pub struct NoCache;

impl Cache for NoCache {
    fn has(&self, _: &Want) -> bool {
        false
    }

    fn moods(&self, _: &str) -> Vec<String> {
        Vec::new()
    }
}

/// how a file settled
#[derive(Debug, Clone, PartialEq)]
pub enum Got {
    /// downloaded whole and checked: its bytes
    Fetched(u64),
    /// the cache has it, nothing fetched
    Cached,
    /// nothing came and the cache has no copy
    Absent,
    /// the CDN answered this status
    Http(u16),
    /// the network is gone; `cached`: the cache has a copy to play
    Offline { cached: bool },
    /// another failure (a refused download, a write)
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    Pending,
    /// handed to the adapter (`next_fetch`)
    Fetching,
    Ok,
    Failed(String),
}

/// one file of an item
#[derive(Debug, Clone)]
struct Slot {
    want: Want,
    /// None: the API didn't name it – the cache's copy or nothing
    file: Option<File>,
    state: State,
}

impl Slot {
    fn settled(&self) -> bool {
        matches!(self.state, State::Ok | State::Failed(_))
    }
}

#[derive(Debug, Clone)]
struct Item {
    name: String,
    /// why it can't be had at all
    why: Option<String>,
    slots: Vec<Slot>,
}

/// One push being answered.
#[derive(Debug, Clone)]
pub struct Push {
    scene: String,
    items: Vec<Item>,
    /// the network was gone (the request or a file)
    offline: bool,
    bytes: u64,
}

/// the items a push names, in its order: each published scene as written, `set:<id>`, then `character:<id>`
pub fn items_of(s: &SceneSync) -> Vec<String> {
    let scenes = s.scenes.iter().flatten().cloned();
    let sets = s.sets.iter().flatten().map(|id| format!("set:{id}"));
    scenes.chain(sets).chain(s.characters.iter().flatten().map(|id| format!("character:{id}"))).collect()
}

/// #513: what became of a published scene a push names (its item, the reference as written): the scene the core's
/// loader resolved, or why it is not here – in the loader's words
pub type Scened = (String, Result<Scene, String>);

/// #513: what a published scene needs on a deck besides its document: each space node's set (by id; a file path is no
/// set) and every character on its stage – the being's, then each look's (several holders, #492)
pub fn scene_needs(scene: &Scene) -> (Vec<String>, Vec<String>) {
    let str_of = |v: Option<&toml::Value>| v.and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(str::to_string);
    let sets = scene.nodes.iter().filter(|n| n.kind == "space").filter_map(|n| str_of(n.params.get("set"))).filter(|s| Path::new(s).extension().is_none());
    let looks = scene.nodes.iter().filter(|n| n.kind == "look").filter_map(|n| str_of(n.params.get("being").and_then(|b| b.get("character"))));
    let characters = Some(scene.being_character()).filter(|c| !c.is_empty()).into_iter().chain(looks);
    (once(sets), once(characters))
}

/// each id once, in its first place
fn once(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// #513: a push's item as a published scene, or why it names none
fn scene_ref(item: &str) -> Result<SceneRef, String> {
    SceneRef::parse(item).unwrap_or_else(|| Err(format!("'{item}' names no published scene (scene:<id>[@<n>])")))
}

/// #513: the push's published scenes, each loaded by `load` (a deck with a worker of its own: the core's loader driven with
/// its copy – the native deck's published.rs), and the push with their sets and characters (`with_scenes`)
pub fn preload(sync: &SceneSync, mut load: impl FnMut(SceneRef) -> Result<Scene, String>) -> (SceneSync, Vec<Scened>) {
    let scenes: Vec<Scened> = sync.scenes.iter().flatten().map(|item| (item.clone(), scene_ref(item).and_then(&mut load))).collect();
    (with_scenes(sync, &scenes), scenes)
}

/// #513: the push with its published scenes' sets and characters added (each once, the push's own first) – what the deck
/// asks scene-content for and plans
pub fn with_scenes(sync: &SceneSync, scenes: &[Scened]) -> SceneSync {
    let needs: Vec<(Vec<String>, Vec<String>)> = scenes.iter().filter_map(|(_, got)| got.as_ref().ok()).map(scene_needs).collect();
    if needs.is_empty() {
        return sync.clone();
    }
    let sets = once(sync.sets.iter().flatten().cloned().chain(needs.iter().flat_map(|(s, _)| s.iter().cloned())));
    let characters = once(sync.characters.iter().flatten().cloned().chain(needs.iter().flat_map(|(_, c)| c.iter().cloned())));
    SceneSync { sets: Some(sets), characters: Some(characters), ..sync.clone() }
}

impl Push {
    /// The push's files at the deck's `quality`, from what the deck knows of the API's answers (`content`), how its last
    /// request went (`api`) and its `cache`. A file the API didn't name settles now: the cache's copy, or missing.
    /// `scenes` (#513): its published scenes as the loader left them – each an item without files of its own (its media
    /// are in `sync` already: `with_scenes`).
    pub fn plan(sync: &SceneSync, scenes: &[Scened], quality: &str, content: &Content, api: &Api, cache: &dyn Cache) -> Push {
        let quality = if quality.is_empty() { DEFAULT_QUALITY } else { quality };
        let unreached = match api {
            Api::Answered => "not in the API's answer".to_string(),
            Api::Offline => "offline - not in the cache".to_string(),
            Api::Failed(e) => format!("{e} - not in the cache"),
        };
        let slot = |want: Want, file: Option<&File>| {
            let state = match file {
                Some(_) => State::Pending,
                None if cache.has(&want) => State::Ok,
                None => State::Failed(unreached.clone()),
            };
            Slot { want, file: file.cloned(), state }
        };
        let item = |name: String, slots: Vec<Slot>| Item { name, why: None, slots };
        let why = |name: String, why: String| Item { name, why: Some(why), slots: Vec::new() };
        // a published scene: here (loaded) or not, with the loader's reason; its media are the set and character items
        let mut items: Vec<Item> = scenes.iter().map(|(name, got)| Item { name: name.clone(), why: got.as_ref().err().cloned(), slots: Vec::new() }).collect();
        for id in sync.sets.iter().flatten() {
            let name = format!("set:{id}");
            let want = Want::Set { id: id.clone(), quality: quality.to_string() };
            items.push(match content.file(&want) {
                Ok(f) => item(name, vec![slot(want, f)]),
                Err(e) => why(name, e),
            });
        }
        for c in sync.characters.iter().flatten() {
            let name = format!("character:{c}");
            let face = |mood: &str| Want::Portrait { character: c.clone(), mood: mood.to_string() };
            items.push(match (content.characters.get(c), content.file(&face(""))) {
                (Some(cc), _) => item(name, cc.portraits.iter().map(|(mood, f)| slot(face(mood), Some(f))).collect()),
                (None, Err(e)) => why(name, e),
                (None, Ok(_)) => match cache.moods(c) {
                    moods if moods.is_empty() => why(name, unreached.clone()),
                    moods => item(name, moods.iter().map(|m| slot(face(m), None)).collect()),
                },
            });
        }
        Push { scene: sync.scene.clone(), items, offline: *api == Api::Offline, bytes: 0 }
    }

    /// every file the push needs (the native worker queues them all: its loaders wait on the same files)
    pub fn wants(&self) -> impl Iterator<Item = &Want> {
        self.items.iter().flat_map(|i| i.slots.iter().map(|s| &s.want))
    }

    fn slots(&self) -> impl Iterator<Item = &Slot> {
        self.items.iter().flat_map(|i| i.slots.iter())
    }

    /// the next file to fetch – its number and CDN path – in the items' order; None: nothing left to fetch
    pub fn next_fetch(&mut self) -> Option<(usize, String)> {
        let slots = self.items.iter_mut().flat_map(|i| i.slots.iter_mut());
        let (n, s) = slots.enumerate().find(|(_, s)| s.state == State::Pending && s.file.is_some())?;
        s.state = State::Fetching;
        Some((n, s.file.as_ref()?.path.clone()))
    }

    /// The file `want` settled. False: not this push's, or settled before (a file settles once).
    pub fn settle(&mut self, want: &Want, got: Got) -> bool {
        let n = self.slots().position(|s| &s.want == want && !s.settled());
        n.is_some_and(|n| self.settle_at(n, got))
    }

    /// file `n` (`next_fetch`'s number) settled
    pub fn settle_at(&mut self, n: usize, got: Got) -> bool {
        let Some(s) = self.items.iter_mut().flat_map(|i| i.slots.iter_mut()).nth(n).filter(|s| !s.settled()) else { return false };
        let path = s.file.as_ref().map_or("", |f| f.path.as_str());
        s.state = match got {
            Got::Fetched(n) => {
                self.bytes += n;
                State::Ok
            }
            Got::Cached => State::Ok,
            Got::Absent => State::Failed("not in the cache".into()),
            Got::Http(code) => State::Failed(http_failure(path, code)),
            Got::Offline { cached } => {
                self.offline = true;
                if cached { State::Ok } else { State::Failed("offline - not in the cache".into()) }
            }
            Got::Failed(e) => State::Failed(e),
        };
        true
    }

    /// File `n` came with this HTTP `status` (0: the network is gone), `len` bytes starting with `head`: checked, then
    /// settled.
    pub fn received(&mut self, n: usize, status: u16, len: u64, head: &[u8]) -> bool {
        let Some(file) = self.slots().nth(n).and_then(|s| s.file.clone()) else { return false };
        let got = match status {
            0 => Got::Offline { cached: false },
            200..=299 => match check(&file.path, file.size, len, head) {
                Ok(()) => Got::Fetched(len),
                Err(e) => Got::Failed(e),
            },
            code => Got::Http(code),
        };
        self.settle_at(n, got)
    }

    /// The answer now (`deck.synced`).
    pub fn answer(&self) -> DeckSynced {
        let (mut ready, mut missing, mut why) = (Vec::new(), Vec::new(), BTreeMap::new());
        let (mut done, mut total, mut pending) = (0u64, 0u64, false);
        for i in &self.items {
            total += i.slots.len() as u64;
            done += i.slots.iter().filter(|s| s.settled()).count() as u64;
            let failed = i.slots.iter().find_map(|s| match &s.state {
                State::Failed(e) => Some(e),
                _ => None,
            });
            if let Some(reason) = i.why.as_ref().or(failed) {
                missing.push(i.name.clone());
                why.insert(i.name.clone(), reason.clone());
            } else if i.slots.iter().all(Slot::settled) {
                ready.push(i.name.clone());
            } else {
                pending = true;
            }
        }
        let state = match () {
            _ if pending => SyncState::Syncing,
            _ if missing.is_empty() => SyncState::Ready,
            _ if self.offline => SyncState::Offline,
            _ => SyncState::Missing,
        };
        DeckSynced {
            scene: self.scene.clone(),
            state,
            done: Some(done),
            total: Some(total),
            bytes: Some(self.bytes),
            ready,
            missing,
            why: (!why.is_empty()).then(|| why.into_iter().collect()),
        }
    }
}

/// the status line of a push's answer: `PUSH 2/5 CATHEDRAL` · `PUSH READY CATHEDRAL` · `PUSH MISSING 1 …` ·
/// `PUSH OFFLINE …` – the scene last (the HUD cuts at 40 characters)
pub fn push_line(r: &DeckSynced) -> String {
    let state = match r.state {
        SyncState::Syncing => format!("{}/{}", r.done.unwrap_or(0), r.total.unwrap_or(0)),
        SyncState::Ready => "READY".into(),
        SyncState::Missing => format!("MISSING {}", r.missing.len()),
        SyncState::Offline => "OFFLINE".into(),
    };
    format!("PUSH {state} {}", r.scene).to_uppercase().chars().take(LINE_MAX).collect()
}

/// a file the CDN answered with a status that isn't 2xx
pub fn http_failure(path: &str, code: u16) -> String {
    format!("{path}: HTTP {code}")
}

/// A download is what the answer said: no larger than `MAX_FILE`, its `size`, and its type's magic in `head` (its first
/// `HEAD` bytes; a broken or truncated file, an error page, is refused).
pub fn check(path: &str, size: Option<u64>, len: u64, head: &[u8]) -> Result<(), String> {
    if len > MAX_FILE {
        return Err(format!("{path}: larger than {} MB", MAX_FILE >> 20));
    }
    if let Some(s) = size.filter(|&s| s != len) {
        return Err(format!("{path}: {len} bytes, the API said {s} – refused"));
    }
    let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let ok = match ext.as_str() {
        "glb" => head.starts_with(b"glTF"),
        "webp" => head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP",
        "png" => head.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => head.starts_with(b"\xFF\xD8\xFF"),
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{path}: not a .{ext} file – refused"))
    }
}

/// `id` as a URL's query value (RFC 3986's unreserved characters stay)
fn encode(id: &str) -> String {
    id.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// the one request for these sets and characters
pub fn content_url(api: &str, sets: &[&str], characters: &[&str]) -> String {
    let list = |ids: &[&str]| ids.iter().map(|id| encode(id)).collect::<Vec<_>>().join(",");
    format!("{}/library/scene-content?sets={}&characters={}", api.trim_end_matches('/'), list(sets), list(characters))
}

/// a file's CDN URL
pub fn cdn_url(cdn: &str, path: &str) -> String {
    format!("{}/{}", cdn.trim_end_matches('/'), path.trim_start_matches('/'))
}

/// where a media path lives below a deck's folder: the CDN path itself; None = it would leave the folder
pub fn media_file(dir: &Path, path: &str) -> Option<std::path::PathBuf> {
    let rel = Path::new(path.trim_start_matches('/'));
    let inside = rel.components().all(|c| matches!(c, std::path::Component::Normal(_)));
    (inside && rel.components().next().is_some()).then(|| dir.join(rel))
}

/// the space node's set id and quality (None: no set, or a file path) – what a deck fetches for its own scene
pub fn set_need(graph: &Graph, node: usize) -> Option<(String, String)> {
    let set = graph.nodes[node].str("set")?;
    let quality = graph.nodes[node].str("quality").unwrap_or(DEFAULT_QUALITY);
    Path::new(set).extension().is_none().then(|| (set.to_string(), quality.to_string()))
}

/// the quality a pushed set is fetched at: the scene's space node's, else `DEFAULT_QUALITY`
pub fn set_quality(graph: &Graph) -> String {
    graph.first(Kind::Space).and_then(|n| set_need(graph, n)).map_or_else(|| DEFAULT_QUALITY.to_string(), |(_, q)| q)
}

/// a scene's id for a push: the scene file's stem (or the built-in's name), else the stage's name as a slug
pub fn scene_id(file: Option<&Path>, name: &str) -> String {
    if let Some(stem) = file.and_then(|p| p.file_stem()).and_then(|s| s.to_str()) {
        return stem.to_string();
    }
    let slug: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    slug.split('-').filter(|w| !w.is_empty()).collect::<Vec<_>>().join("-")
}

/// A push written out (`--sync set:<id>,character:<id>,being:<id>,scene:<id>[@<v>]`).
pub fn parse_push(scene: &str, spec: &str) -> Result<SceneSync, String> {
    let (mut sets, mut characters, mut beings, mut scenes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        match part.split_once(':') {
            Some(("set", id)) if !id.is_empty() => sets.push(id.to_string()),
            Some(("character", id)) if !id.is_empty() => characters.push(id.to_string()),
            Some(("being", id)) if !id.is_empty() => beings.push(id.to_string()),
            Some(("scene", _)) => scenes.push(SceneRef::parse(part).and_then(Result::ok).ok_or_else(|| format!("--sync {part}: scene:<id>[@<n>]"))?.to_string()),
            _ => return Err(format!("--sync {part}: set:<id>, character:<id>, scene:<id> or being:<id>")),
        }
    }
    let scenes = (!scenes.is_empty()).then_some(scenes);
    Ok(SceneSync { scene: scene.into(), sets: Some(sets), characters: Some(characters), scenes, beings: (!beings.is_empty()).then_some(beings), decks: None })
}

/// A push answered request by request, then file by file – the flow of a deck without a worker of its own (the browser,
/// the headless host): `new` gives the first URL to ask, `asked` takes its reply and gives the next (each published
/// scene's loader asks first, #513; then the one scene-content request) until none is left, then `next_fetch` /
/// `received` per file. It keeps no copy of a scene (the browser's HTTP cache does).
#[derive(Debug, Clone)]
pub struct Run {
    sync: SceneSync,
    api: String,
    cdn: String,
    quality: String,
    /// #513: the published scenes still to load, the one loading (its item, its loader), and what became of each
    todo: VecDeque<String>,
    loading: Option<(String, Load)>,
    scenes: Vec<Scened>,
    /// the push with its scenes' sets and characters, once they are loaded
    merged: Option<SceneSync>,
    push: Option<Push>,
}

impl Run {
    /// a push to answer; the URL to ask first
    pub fn new(api: &str, cdn: &str, quality: &str, sync: SceneSync) -> (Run, String) {
        let todo = sync.scenes.iter().flatten().cloned().collect();
        let mut run = Run { api: api.into(), cdn: cdn.into(), quality: quality.into(), todo, loading: None, scenes: Vec::new(), merged: None, push: None, sync };
        let url = run.next(None);
        (run, url)
    }

    /// The reply to the last URL: its HTTP status (0: the network is gone) and body. The next URL to ask; None: the
    /// push is planned, the files follow (`next_fetch`).
    pub fn asked(&mut self, status: u16, body: &str) -> Option<String> {
        if let Some((_, load)) = &mut self.loading {
            let step = load.answer(status, body);
            return Some(self.next(Some(step)));
        }
        let merged = self.merged.as_ref()?;
        if self.push.is_some() {
            return None;
        }
        let (content, api) = match Api::reply(status, body) {
            Ok(c) => (c, Api::Answered),
            Err(api) => (Content::default(), api),
        };
        self.push = Some(Push::plan(merged, &self.scenes, &self.quality, &content, &api, &NoCache));
        None
    }

    /// the loader's step for the scene loading (none: start the next) – the URL it asks, or, every scene done, the one
    /// scene-content request for the push with their sets and characters
    fn next(&mut self, mut step: Option<Step>) -> String {
        let api = self.api.trim_end_matches('/').to_string();
        loop {
            let done = match step.take() {
                Some(Step::Ask(path)) => return format!("{api}{path}"),
                Some(Step::Play(l)) => Some(Ok(l.scene)),
                Some(Step::Fail(why)) => Some(Err(why)),
                None => None,
            };
            if let (Some(got), Some((item, _))) = (done, self.loading.take()) {
                self.scenes.push((item, got));
            }
            let Some(item) = self.todo.pop_front() else { break };
            match scene_ref(&item) {
                Ok(r) => {
                    let (load, first) = Load::start(r, None);
                    self.loading = Some((item, load));
                    step = Some(first);
                }
                Err(why) => self.scenes.push((item, Err(why))),
            }
        }
        let merged = with_scenes(&self.sync, &self.scenes);
        let sets: Vec<&str> = merged.sets.iter().flatten().map(String::as_str).collect();
        let characters: Vec<&str> = merged.characters.iter().flatten().map(String::as_str).collect();
        let url = content_url(&api, &sets, &characters);
        self.merged = Some(merged);
        url
    }

    /// the next file to fetch: its number and URL
    pub fn next_fetch(&mut self) -> Option<(usize, String)> {
        let (n, path) = self.push.as_mut()?.next_fetch()?;
        Some((n, cdn_url(&self.cdn, &path)))
    }

    /// file `n` came: its HTTP status (0: the network is gone), its length and first bytes
    pub fn received(&mut self, n: usize, status: u16, len: u64, head: &[u8]) -> bool {
        self.push.as_mut().is_some_and(|p| p.received(n, status, len, head))
    }

    pub fn answer(&self) -> DeckSynced {
        match &self.push {
            Some(p) => p.answer(),
            // not asked yet
            None => DeckSynced { scene: self.sync.scene.clone(), state: SyncState::Syncing, done: Some(0), total: Some(0), bytes: Some(0), ready: Vec::new(), missing: Vec::new(), why: None },
        }
    }

    pub fn line(&self) -> String {
        push_line(&self.answer())
    }
}

/// A scripted push: what the API and the CDN reply.
#[derive(Debug, Clone, Deserialize)]
pub struct Script {
    pub push: SceneSync,
    #[serde(default)]
    pub quality: String,
    pub api: Reply,
    /// #513: the API's reply by its path after the base (a published scene's document, its scene-content); a path not
    /// here: `api`
    #[serde(default)]
    pub answers: BTreeMap<String, Reply>,
    /// CDN path → its reply (a path not here: 404)
    #[serde(default)]
    pub files: BTreeMap<String, Reply>,
}

/// a scripted reply: a status (0: the network is gone), a body (the API's) or a file's length and first bytes
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Reply {
    #[serde(default)]
    pub status: Option<u16>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    #[serde(default)]
    pub bytes: u64,
    /// the file's first bytes as text (`glTF`, `RIFF....WEBP`)
    #[serde(default)]
    pub head: String,
}

/// one call an adapter makes on a `Run`: its name, its arguments, what the core answered
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Call {
    pub name: &'static str,
    pub args: serde_json::Value,
    pub answer: serde_json::Value,
}

/// A scripted push through a `Run` in the order every adapter calls it: `new` (the first URL), `asked` with each URL's
/// reply until it names none, the answer and line; then per file `next_fetch`, `received`, the answer and line; the last
/// `next_fetch` answers null.
pub fn script(s: &Script, api: &str, cdn: &str) -> Vec<Call> {
    use serde_json::{json, Value};
    let answer = |r: &Run| serde_json::to_value(r.answer()).unwrap_or(Value::Null);
    let mut calls = Vec::new();
    let (mut run, url) = Run::new(api, cdn, &s.quality, s.push.clone());
    calls.push(Call { name: "start", args: json!([s.push, s.quality]), answer: json!(url) });
    let mut next = Some(url);
    // a published scene's loader asks a few times; the push's request once (a loop that keeps asking is cut)
    for _ in 0..64 {
        let Some(url) = next.take() else { break };
        let path = url.strip_prefix(api.trim_end_matches('/')).unwrap_or(&url);
        let reply = s.answers.get(path).unwrap_or(&s.api);
        let status = reply.status.unwrap_or(200);
        // as every adapter hands it in: no body unless 2xx
        let body = if status / 100 == 2 { reply.body.as_ref().map_or_else(String::new, Value::to_string) } else { String::new() };
        next = run.asked(status, &body);
        calls.push(Call { name: "asked", args: json!([status, body]), answer: json!(next) });
    }
    let say = |calls: &mut Vec<Call>, run: &Run| {
        calls.push(Call { name: "answer", args: json!([]), answer: answer(run) });
        calls.push(Call { name: "line", args: json!([]), answer: json!(run.line()) });
    };
    say(&mut calls, &run);
    while let Some((n, url)) = run.next_fetch() {
        calls.push(Call { name: "next_fetch", args: json!([]), answer: json!([n, url]) });
        let path = url.strip_prefix(&cdn_url(cdn, "")).unwrap_or(&url);
        let r = s.files.get(path).cloned().unwrap_or(Reply { status: Some(404), ..Reply::default() });
        let status = r.status.unwrap_or(200);
        let len = if status / 100 == 2 { r.bytes } else { 0 };
        let head: Vec<u8> = r.head.bytes().take(HEAD.min(len as usize)).collect();
        run.received(n, status, len, &head);
        calls.push(Call { name: "received", args: json!([n, status, len, head]), answer: Value::Null });
        say(&mut calls, &run);
    }
    calls.push(Call { name: "next_fetch", args: json!([]), answer: Value::Null });
    calls
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const API: &str = "https://api.test/api";
    const CDN: &str = "https://cdn.test/";

    fn push(characters: &[&str]) -> SceneSync {
        SceneSync { scene: "cathedral".into(), sets: Some(vec!["cathedral".into()]), characters: Some(characters.iter().map(|c| c.to_string()).collect()), scenes: None, beings: None, decks: None }
    }

    /// scene-content for the cathedral and catpope; ghost is unknown
    fn content() -> serde_json::Value {
        json!({
            "sets": { "cathedral": { "model": { "low": { "path": "catpope/map/sets/cathedral.low.v1.glb", "size": 4 }, "high": { "path": "catpope/map/sets/cathedral.high.v1.glb", "size": 9 } } } },
            "characters": { "catpope": { "portraits": { "neutral": { "path": "characters/catpope/n.webp", "size": 12 }, "happy": { "path": "characters/catpope/h.webp" } } } },
            "missing": { "sets": [], "characters": ["ghost"] }
        })
    }

    const WEBP: &str = "RIFF....WEBP";

    fn files(glb: u64) -> serde_json::Value {
        json!({
            "catpope/map/sets/cathedral.low.v1.glb": { "bytes": glb, "head": "glTF" },
            "characters/catpope/n.webp": { "bytes": 12, "head": WEBP },
            "characters/catpope/h.webp": { "bytes": 30, "head": WEBP }
        })
    }

    /// #513: a push naming only published scenes
    fn scenes(refs: &[&str]) -> SceneSync {
        SceneSync { scene: "nave".into(), sets: None, characters: None, scenes: Some(refs.iter().map(|r| r.to_string()).collect()), beings: None, decks: None }
    }

    /// #513: the API's answers for `scene:nave` at version 4 – its document, then its scene-content (Cat Pope's rig)
    fn nave_answers() -> serde_json::Value {
        let mut doc: serde_json::Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/nave.json"))).unwrap();
        doc["version"] = 4.into();
        let rigs: serde_json::Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/rigs/scene-content.json"))).unwrap();
        json!({
            "/scenes/nave": { "body": { "status": "success", "scene": doc } },
            "/library/scene-content?sets=&locations=&characters=catpope&items=": { "body": rigs },
            "/scenes/crypt": { "status": 404 }
        })
    }

    /// The scripted pushes the browser deck's test replays (shared/live-deck/test/fixtures/sync.json): every file
    /// whole; unknown to the API and a short file; the deck's own quality; no network; the API failing; #513: a published
    /// scene with its set and cast, and one that is not published.
    pub fn scripts() -> Vec<(&'static str, Script)> {
        let s = |push: SceneSync, quality: &str, api: serde_json::Value, files: serde_json::Value| {
            let answers = if push.scenes.is_some() { nave_answers() } else { json!({}) };
            serde_json::from_value::<Script>(json!({ "push": push, "quality": quality, "api": api, "files": files, "answers": answers })).unwrap()
        };
        vec![
            ("ready", s(push(&["catpope"]), "", json!({ "body": content() }), files(4))),
            ("missing", s(push(&["catpope", "ghost"]), "", json!({ "body": content() }), files(3))),
            ("high", s(push(&[]), "high", json!({ "body": content() }), files(4))),
            ("offline", s(push(&["catpope"]), "", json!({ "status": 0 }), files(4))),
            ("api-error", s(push(&["catpope"]), "", json!({ "status": 503 }), files(4))),
            ("cdn-gone", s(push(&["catpope"]), "", json!({ "body": content() }), json!({ "catpope/map/sets/cathedral.low.v1.glb": { "status": 0 } }))),
            ("scene", s(scenes(&["scene:nave"]), "", json!({ "body": content() }), files(4))),
            ("scene-unknown", s(scenes(&["scene:crypt", "scene:nave"]), "", json!({ "body": content() }), files(4))),
        ]
    }

    fn answers(calls: &[Call]) -> Vec<(DeckSynced, String)> {
        let mut out = Vec::new();
        for w in calls.windows(2) {
            if w[0].name == "answer" {
                out.push((serde_json::from_value(w[0].answer.clone()).unwrap(), w[1].answer.as_str().unwrap().to_string()));
            }
        }
        out
    }

    /// #512: the server's answer for `?locations=cathedral,dom,nowhere` (its SceneContentTest writes the file): the place
    /// `cathedral` names the set `cathedral`, which answers every rung; `dom` names none; `nowhere` has no place
    #[test]
    fn a_location_names_its_set_and_the_set_every_rung() {
        let c = Content::parse(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/scene-content-location.json"))).unwrap();
        assert_eq!(c.set_of("cathedral").as_deref(), Some("cathedral"));
        assert_eq!((c.set_of("dom"), c.set_of("nowhere")), (None, None));
        assert_eq!(c.missing.locations, ["nowhere"]);
        let path = |f: Option<&File>| f.map(|f| f.path.clone());
        let low = c.media("cathedral", "low").unwrap();
        assert_eq!(path(low.model).as_deref(), Some("catpope/map/sets/cathedral.low.v2.glb"));
        assert_eq!(path(low.video).as_deref(), Some("catpope/map/sets/cathedral.loop.v1.mp4"));
        assert_eq!(path(low.still).as_deref(), Some("catpope/map/sets/cathedral.still.v1.webp"));
        assert_eq!(path(c.media("cathedral", "high").unwrap().video).as_deref(), Some("catpope/map/sets/cathedral.loop.4k.v1.mp4"));
        assert!(c.media("cave", "low").is_none());
    }

    #[test]
    fn items_sets_then_characters() {
        assert_eq!(items_of(&push(&["catpope", "ghost"])), ["set:cathedral", "character:catpope", "character:ghost"]);
        assert!(items_of(&SceneSync { scene: "x".into(), sets: None, characters: None, scenes: None, beings: None, decks: None }).is_empty());
        assert_eq!(items_of(&SceneSync { scenes: Some(vec!["scene:nave@3".into()]), ..push(&[]) }), ["scene:nave@3", "set:cathedral"]);
    }

    /// #513: `scene:nave` – its document and scene-content first (the core's loader), then the push's one request with its
    /// set and its cast; the scene is an item of its own, its media the set's and character's
    #[test]
    fn a_published_scene_is_loaded_first_and_its_set_and_cast_join_the_push() {
        let calls = script(&scripts()[6].1, API, CDN);
        assert_eq!(calls[0].answer, "https://api.test/api/scenes/nave");
        let asked: Vec<&serde_json::Value> = calls.iter().filter(|c| c.name == "asked").map(|c| &c.answer).collect();
        assert_eq!(asked, [&json!("https://api.test/api/library/scene-content?sets=&locations=&characters=catpope&items="), &json!("https://api.test/api/library/scene-content?sets=cathedral&characters=catpope"), &serde_json::Value::Null]);
        let (last, line) = answers(&calls).pop().unwrap();
        assert_eq!((last.state, last.ready, line), (SyncState::Ready, vec!["scene:nave".to_string(), "set:cathedral".into(), "character:catpope".into()], "PUSH READY NAVE".to_string()));
        assert_eq!(last.total, Some(3), "the scene has no files of its own");
        // one that is not published: missing with the loader's words; the other still comes
        let (last, _) = answers(&script(&scripts()[7].1, API, CDN)).pop().unwrap();
        assert_eq!((last.state, last.missing.clone()), (SyncState::Missing, vec!["scene:crypt".to_string()]));
        assert_eq!(last.why.unwrap()["scene:crypt"], "scene:crypt is not published");
        assert_eq!(last.ready, ["scene:nave", "set:cathedral", "character:catpope"]);
    }

    #[test]
    fn a_worker_preloads_with_its_own_loader() {
        let scene = Scene::parse("[being]\ncharacter = \"catpope\"\n[[node]]\nid = \"space\"\nkind = \"space\"\nset = \"cathedral\"\n").unwrap();
        let mut asked = Vec::new();
        let sync = SceneSync { scenes: Some(vec!["scene:nave@2".into(), "scene:crypt".into(), "set:x".into()]), ..push(&[]) };
        let (merged, scenes) = preload(&sync, |r| {
            asked.push(r.to_string());
            if r.id == "nave" { Ok(scene.clone()) } else { Err(format!("{r} is not published")) }
        });
        assert_eq!(asked, ["scene:nave@2", "scene:crypt"]);
        assert_eq!(merged.characters.unwrap(), ["catpope"]);
        assert_eq!(scenes.iter().map(|(i, g)| (i.as_str(), g.as_ref().err().map(String::as_str))).collect::<Vec<_>>(), [("scene:nave@2", None), ("scene:crypt", Some("scene:crypt is not published")), ("set:x", Some("'set:x' names no published scene (scene:<id>[@<n>])"))]);
    }

    #[test]
    fn a_scene_s_needs_are_its_sets_and_everyone_on_its_stage() {
        let scene = Scene::parse("[being]\ncharacter = \"catpope\"\n[[node]]\nid = \"space\"\nkind = \"space\"\nset = \"cathedral\"\n[[node]]\nid = \"a.look\"\nkind = \"look\"\nbeing = { character = \"yellowmouse\" }\n[[node]]\nid = \"b.look\"\nkind = \"look\"\nbeing = { character = \"catpope\" }\n[[node]]\nid = \"s2\"\nkind = \"space\"\nset = \"x.glb\"\n").unwrap();
        assert_eq!(scene_needs(&scene), (vec!["cathedral".to_string()], vec!["catpope".to_string(), "yellowmouse".to_string()]));
        let sync = with_scenes(&push(&["yellowmouse", "ghost"]), &[("scene:a".into(), Ok(scene)), ("scene:b".into(), Err("no".into()))]);
        assert_eq!((sync.sets.unwrap(), sync.characters.unwrap()), (vec!["cathedral".to_string()], vec!["yellowmouse".to_string(), "ghost".into(), "catpope".into()]));
        // a scene that failed adds nothing
        assert_eq!(with_scenes(&push(&[]), &[("scene:b".into(), Err("no".into()))]), push(&[]));
    }

    #[test]
    fn a_push_one_request_every_file_progress_then_ready() {
        let calls = script(&scripts()[0].1, API, CDN);
        assert_eq!(calls[0].answer, "https://api.test/api/library/scene-content?sets=cathedral&characters=catpope");
        let fetched: Vec<&str> = calls.iter().filter(|c| c.name == "next_fetch").filter_map(|c| c.answer[1].as_str()).collect();
        assert_eq!(
            fetched,
            ["https://cdn.test/catpope/map/sets/cathedral.low.v1.glb", "https://cdn.test/characters/catpope/h.webp", "https://cdn.test/characters/catpope/n.webp"],
            "the low 3D, then the portraits by mood"
        );
        let a = answers(&calls);
        let steps: Vec<String> = a.iter().map(|(r, _)| format!("{} {}/{}", r.state.as_str(), r.done.unwrap(), r.total.unwrap())).collect();
        assert_eq!(steps, ["syncing 0/3", "syncing 1/3", "syncing 2/3", "ready 3/3"]);
        let (last, line) = a.last().unwrap();
        assert_eq!((last.bytes, last.ready.clone(), last.missing.len(), last.why.is_none()), (Some(46), vec!["set:cathedral".to_string(), "character:catpope".to_string()], 0, true));
        assert_eq!(line, "PUSH READY CATHEDRAL");
        assert_eq!(a[1].1, "PUSH 1/3 CATHEDRAL");
    }

    #[test]
    fn missing_unknown_a_short_file_another_quality_offline() {
        let (r, line) = answers(&script(&scripts()[1].1, API, CDN)).pop().unwrap();
        assert_eq!((r.state, r.ready.clone(), r.missing.clone()), (SyncState::Missing, vec!["character:catpope".to_string()], vec!["set:cathedral".to_string(), "character:ghost".to_string()]));
        let why = r.why.unwrap();
        assert!(why["set:cathedral"].contains("3 bytes, the API said 4"), "{why:?}");
        assert!(why["character:ghost"].contains("unknown to the API"));
        assert_eq!(line, "PUSH MISSING 2 CATHEDRAL");

        let (high, _) = answers(&script(&scripts()[2].1, API, CDN)).pop().unwrap();
        assert!(high.why.unwrap()["set:cathedral"].contains("cathedral.high.v1.glb: HTTP 404"), "the deck's own quality");

        let (off, line) = answers(&script(&scripts()[3].1, API, CDN)).pop().unwrap();
        assert_eq!((off.state, off.missing.clone()), (SyncState::Offline, vec!["set:cathedral".to_string(), "character:catpope".to_string()]));
        assert_eq!(off.why.unwrap()["set:cathedral"], "offline - not in the cache");
        assert_eq!(line, "PUSH OFFLINE CATHEDRAL");

        let (err, _) = answers(&script(&scripts()[4].1, API, CDN)).pop().unwrap();
        assert_eq!(err.state, SyncState::Missing, "the API failing is no offline");
        assert_eq!(err.why.unwrap()["character:catpope"], "API HTTP 503 - not in the cache");

        let (gone, _) = answers(&script(&scripts()[5].1, API, CDN)).pop().unwrap();
        assert_eq!(gone.state, SyncState::Offline, "the network went during the files");
    }

    /// what a cache holds: a deck that can't reach the API still counts on it
    #[test]
    fn the_cache_answers_for_what_the_api_did_not_name() {
        struct Has;
        impl Cache for Has {
            fn has(&self, _: &Want) -> bool {
                true
            }
            fn moods(&self, c: &str) -> Vec<String> {
                if c == "catpope" { vec!["neutral".into()] } else { Vec::new() }
            }
        }
        let p = Push::plan(&push(&["catpope", "ghost"]), &[], "", &Content::default(), &Api::Offline, &Has);
        let r = p.answer();
        assert_eq!((r.state, r.ready.clone(), r.missing.clone()), (SyncState::Offline, vec!["set:cathedral".to_string(), "character:catpope".to_string()], vec!["character:ghost".to_string()]));
        assert_eq!((r.done, r.total), (Some(2), Some(2)));
    }

    #[test]
    fn a_file_settles_once_by_its_want() {
        let content = Content::parse(&content().to_string()).unwrap();
        let mut p = Push::plan(&push(&[]), &[], "", &content, &Api::Answered, &NoCache);
        let w = Want::Set { id: "cathedral".into(), quality: "low".into() };
        assert_eq!(p.wants().collect::<Vec<_>>(), [&w]);
        assert!(p.settle(&w, Got::Fetched(4)));
        assert!(!p.settle(&w, Got::Http(500)), "settled before");
        assert!(!p.settle(&Want::Set { id: "x".into(), quality: "low".into() }, Got::Cached), "not this push's");
        assert_eq!((p.answer().state, p.answer().bytes), (SyncState::Ready, Some(4)));
        assert_eq!(p.next_fetch(), None);
    }

    #[test]
    fn a_download_must_match_its_size_and_type() {
        assert!(check("a.glb", Some(8), 8, b"glTF\x02\0\0\0").is_ok());
        assert!(check("a.glb", Some(9), 8, b"glTF\x02\0\0\0").unwrap_err().contains("the API said 9"));
        assert!(check("a.glb", None, 10, b"<html>oops").unwrap_err().contains("not a .glb"));
        assert!(check("a.webp", None, 16, b"RIFF\0\0\0\0WEBPVP8 ").is_ok());
        assert!(check("a.png", None, 12, b"RIFF\0\0\0\0WEBP").is_err());
        assert!(check("a.bin", None, MAX_FILE + 1, b"").unwrap_err().contains("larger than 256 MB"));
    }

    #[test]
    fn the_line_cuts_at_forty() {
        let r = DeckSynced { scene: "a very long scene name for the hud line".into(), state: SyncState::Syncing, done: Some(2), total: Some(5), bytes: None, ready: vec![], missing: vec![], why: None };
        assert_eq!(push_line(&r), "PUSH 2/5 A VERY LONG SCENE NAME FOR THE ");
    }

    #[test]
    fn urls_scene_ids_and_a_written_push() {
        assert_eq!(content_url("https://a.test/api/", &["cathedral", "a b"], &[]), "https://a.test/api/library/scene-content?sets=cathedral,a%20b&characters=");
        assert_eq!(cdn_url("https://cdn.test/", "/x/y.glb"), "https://cdn.test/x/y.glb");
        let d = Path::new("/c/portraits");
        assert_eq!(media_file(d, "/backgrounds/wide.webp"), Some(d.join("backgrounds").join("wide.webp")));
        assert_eq!(media_file(d, "../etc/x.png"), None, "never above the folder");
        assert_eq!(media_file(d, "a/./b.png"), Some(d.join("a").join("b.png")), "a `.` is dropped by the path");
        assert_eq!(media_file(d, ""), None);
        assert!(check("b.jpg", None, 4, b"\xFF\xD8\xFF\xE0").is_ok() && check("b.jpg", None, 4, b"<htm").is_err());
        assert_eq!(scene_id(Some(Path::new("conductor")), "x"), "conductor");
        assert_eq!(scene_id(Some(Path::new("/s/scenes/sermon.toml")), "x"), "sermon");
        assert_eq!(scene_id(None, "Cathedral of decks - conductor"), "cathedral-of-decks-conductor");
        let p = parse_push("conductor", "set:cathedral, character:catpope,character:yellowmouse").unwrap();
        assert_eq!((p.scene.as_str(), p.sets.unwrap(), p.characters.unwrap()), ("conductor", vec!["cathedral".to_string()], vec!["catpope".to_string(), "yellowmouse".to_string()]));
        assert!(p.beings.is_none() && p.decks.is_none());
        assert_eq!(parse_push("x", "being:ghost").unwrap().beings.unwrap(), ["ghost"]);
        assert!(parse_push("x", "room:cathedral").unwrap_err().contains("set:<id>"));
        assert_eq!(parse_push("x", "scene:nave, scene:crypt@2").unwrap().scenes.unwrap(), ["scene:nave", "scene:crypt@2"]);
        assert!(parse_push("x", "scene:nave@x").unwrap_err().contains("scene:<id>[@<n>]"));
        assert!(parse_push("x", "set:cathedral").unwrap().scenes.is_none());
        assert!(parse_push("x", "set:").is_err());
    }

    /// shared/live-deck/test/fixtures/sync.json: the scripted pushes and the calls the browser deck's adapter makes on
    /// space-web's `DeckPush` (this `Run`) with the core's answers – its test replays them against sync.ts.
    #[test]
    fn push_fixture_for_the_browser_deck_is_current() {
        let fixture: Vec<serde_json::Value> = scripts()
            .into_iter()
            .map(|(name, s)| {
                let files: BTreeMap<&String, serde_json::Value> = s.files.iter().map(|(p, r)| (p, json!({ "status": r.status, "bytes": r.bytes, "head": r.head }))).collect();
                // #513: the API's replies by path (a published scene's document, its scene-content) as the text the core
                // was handed (JSON.stringify would write 4.0 as 4)
                let answers: BTreeMap<&String, serde_json::Value> =
                    s.answers.iter().map(|(p, r)| (p, json!({ "status": r.status, "text": r.body.as_ref().map(serde_json::Value::to_string) }))).collect();
                json!({ "name": name, "push": s.push, "quality": s.quality, "api": { "status": s.api.status, "body": s.api.body }, "answers": answers, "files": files, "calls": script(&s, API, CDN) })
            })
            .collect();
        let text = serde_json::to_string_pretty(&json!({ "api": API, "cdn": CDN, "scripts": fixture })).unwrap() + "\n";
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/live-deck/test/fixtures/sync.json");
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(path, &text).unwrap();
        }
        let on_disk = std::fs::read_to_string(path).unwrap_or_default().replace('\r', "");
        assert!(on_disk == text, "{path} is stale: UPDATE_FIXTURES=1 cargo test -p deck-space push_fixture");
    }
}
