//! The deck's sync (#195, the `sync` module): what the scene needs – the space node's set (its 3D at the scene's
//! quality) and the beings' characters (their portrait sprites) – comes from scutoons.com into the cache on DECKDATA,
//! not baked into the image (owner 2026-10-30: "The API should deliver this"). One worker thread does all the network:
//!
//! - **Asking:** at a scene's load (`prepare`) one request, `GET <api>/library/scene-content?sets=…&characters=…`
//!   (server/src/Domain/Library/SceneContent.php) – each set's 3D per quality, each character's portraits per mood, CDN
//!   paths (immutable: a changed file gets a new name, so the path is the version) with their sizes. A character or set
//!   that turns up later (a deck joining the conductor, a Live cell's set) is asked for when it is wanted.
//! - **Fetching:** only what is missing or changed: `<cache>/sync/index.json` remembers which CDN path each local file
//!   came from; same path + same size = cached, skipped. A download goes to `.part`, is checked (the answer's size, the
//!   file's magic: glTF / WebP / PNG) and only then renamed into place – a broken file is refused, the old one stays.
//! - **Where:** `<sets>/<id>/<id>.<quality>.glb` (cache::set_file) and `<portraits>/<char>/<char>_portrait-<mood>.<ext>`
//!   (portrait::LocalDir's layout) – the loaders read them as before. `--sets` / `--portraits` (or their env) point at a
//!   local folder instead: then that kind is not synced (local development).
//! - **Waiting, never on the frame:** the set loader thread passes its file through `gate` and the portraits thread
//!   through `Portraits` (a portrait::Source next to LocalDir) – each blocks its own thread until the sync has settled
//!   that file (fetched, cached, failed, or offline), then reads the local file.
//! - **Offline:** the request fails → the board says so, every file settles at once and the stage plays from the cache;
//!   the API is asked again after 30 s when something new is wanted, and at the next scene load.
//! - **The deck token (#180):** `token` is sent to the API as `Authorization: Bearer` when there is one (deck-link's
//!   daemon keeps it in /run/deck-link once a member approved the deck; DECK_TOKEN on a dev deck) – the content is
//!   public, so nothing waits for it. Never to the CDN.
//!
//! - **The conductor's push (#195):** a `scene.sync` (link.rs hands it in: `push`) is an extra need – its sets at this
//!   deck's quality (`Config::quality`), its characters with every portrait the API lists – through the same request,
//!   cache and skipping. The worker keeps the answer, `deck.synced` (`synced`: ready / missing per item `set:<id>` /
//!   `character:<id>`, files done / total, bytes), new after every settled file; the link sends it to the conductor.
//!   Honest: an item is ready only when its files are in the cache (offline too).
//!
//! The HUD shows `DeckSync::line` (progress, offline, the last error) and `push_line` (the push's state).
use protocol::{DeckSynced, SceneSync, SyncState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// how long a set's loader waits for its file (a few MB on a slow venue network; the frame never waits)
pub const SET_WAIT: Duration = Duration::from_secs(300);
/// how long the portraits thread waits for one sprite
pub const PORTRAIT_WAIT: Duration = Duration::from_secs(60);
/// after a failed request, the API is asked again no sooner than this
const RETRY: Duration = Duration::from_secs(30);
/// the largest file the sync takes
const MAX_FILE: u64 = 256 << 20;

pub struct Config {
    pub api: String,
    pub cdn: String,
    /// the cache (the index lives in `<cache>/sync/`)
    pub cache: PathBuf,
    /// where sets' 3D files land (None: `--sets` names a local folder – not synced)
    pub sets: Option<PathBuf>,
    /// where portraits land (None: `--portraits` names a local folder – not synced)
    pub portraits: Option<PathBuf>,
    /// the deck token for the API (#180): deck-link's run directory (or DECK_TOKEN), read at each request; public
    /// content without one
    pub token: deck_link::TokenSource,
    /// #195: the quality a pushed set's 3D is fetched at (the scene's space node's, else `low`)
    pub quality: String,
}

/// one file the stage wants
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Want {
    Set { id: String, quality: String },
    Portrait { character: String, mood: String },
}

/// what a scene needs: its sets (id, quality) and its characters with their moods
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Needs {
    pub sets: Vec<(String, String)>,
    pub characters: Vec<(String, Vec<String>)>,
}

impl Needs {
    pub fn wants(&self) -> Vec<Want> {
        let sets = self.sets.iter().map(|(id, quality)| Want::Set { id: id.clone(), quality: quality.clone() });
        let faces = self.characters.iter().flat_map(|(c, moods)| moods.iter().map(|m| Want::Portrait { character: c.clone(), mood: m.clone() }));
        sets.chain(faces).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty() && self.characters.is_empty()
    }
}

/// The API's answer (SceneContent.php).
#[derive(Debug, Default, Deserialize)]
pub struct Content {
    #[serde(default)]
    pub sets: HashMap<String, SetContent>,
    #[serde(default)]
    pub characters: HashMap<String, CharacterContent>,
    #[serde(default)]
    pub missing: Missing,
}

#[derive(Debug, Default, Deserialize)]
pub struct SetContent {
    /// quality → its 3D file
    #[serde(default)]
    pub model: HashMap<String, File>,
}

#[derive(Debug, Default, Deserialize)]
pub struct CharacterContent {
    /// mood → its portrait
    #[serde(default)]
    pub portraits: HashMap<String, File>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct File {
    /// from the CDN root
    pub path: String,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Missing {
    #[serde(default)]
    pub sets: Vec<String>,
    #[serde(default)]
    pub characters: Vec<String>,
}

/// What the HUD shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Board {
    /// asking the API or fetching
    pub busy: bool,
    /// files settled / still queued
    pub done: usize,
    pub queued: usize,
    /// downloaded / already in the cache
    pub fetched: usize,
    pub cached: usize,
    pub bytes: u64,
    /// the API can't be reached: the stage plays from the cache
    pub offline: bool,
    pub error: Option<String>,
}

impl Board {
    /// ≤ 40 characters, upper case
    pub fn line(&self) -> String {
        let s = if self.busy {
            format!("SYNC {}/{} {:.1} MB", self.done, self.done + self.queued, self.bytes as f64 / 1_048_576.0)
        } else if self.offline {
            "SYNC OFFLINE - FROM THE CACHE".into()
        } else if let Some(e) = &self.error {
            format!("SYNC ERR {e}")
        } else {
            format!("SYNC OK {} NEW {} CACHED", self.fetched, self.cached)
        };
        s.to_uppercase().chars().take(40).collect()
    }
}

enum Msg {
    Prepare(Needs),
    Want(Want, Sender<()>),
    Push(SceneSync),
}

/// #195: the answer to the conductor's last push, numbered (the link sends each new one)
#[derive(Default)]
struct Answer {
    n: u64,
    last: Option<DeckSynced>,
}

/// The handle (clone it freely; the worker ends with the last one).
#[derive(Clone)]
pub struct DeckSync {
    tx: Sender<Msg>,
    pub board: Arc<Mutex<Board>>,
    answer: Arc<Mutex<Answer>>,
    sets: Option<PathBuf>,
    #[cfg_attr(not(feature = "space"), allow(dead_code))]
    portraits: Option<PathBuf>,
}

impl DeckSync {
    /// the worker thread
    pub fn start(cfg: Config) -> DeckSync {
        let (tx, rx) = mpsc::channel();
        let board = Arc::new(Mutex::new(Board::default()));
        let answer = Arc::new(Mutex::new(Answer::default()));
        let handle = DeckSync { tx, board: board.clone(), answer: answer.clone(), sets: cfg.sets.clone(), portraits: cfg.portraits.clone() };
        let index = Index::load(&cfg.cache);
        let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(5)).timeout_read(Duration::from_secs(30)).build();
        let worker = Worker {
            cfg,
            agent,
            board,
            answer,
            push: None,
            failed: HashMap::new(),
            pushed_bytes: 0,
            content: Content::default(),
            known: HashSet::new(),
            retry_at: None,
            index,
            settled: HashSet::new(),
            queue: VecDeque::new(),
            waiters: HashMap::new(),
        };
        std::thread::Builder::new().name("sync".into()).spawn(move || worker.run(rx)).expect("a thread");
        handle
    }

    /// a scene loads: ask for everything it needs (one request) and fetch it
    pub fn prepare(&self, needs: Needs) {
        if !needs.is_empty() {
            let _ = self.tx.send(Msg::Prepare(needs));
        }
    }

    /// #195: the conductor's push – fetch what it names (skipping what is cached) and keep the answer
    pub fn push(&self, sync: SceneSync) {
        let _ = self.tx.send(Msg::Push(sync));
    }

    /// #195: the answer to the last push, if there is one newer than `seen` (its number, the answer)
    pub fn synced(&self, seen: u64) -> Option<(u64, DeckSynced)> {
        let a = self.answer.lock().ok()?;
        a.last.clone().filter(|_| a.n > seen).map(|l| (a.n, l))
    }

    /// #195: the HUD's line about the push (None: nothing pushed)
    pub fn push_line(&self) -> Option<String> {
        let a = self.answer.lock().ok()?;
        let r = a.last.as_ref()?;
        let state = match r.state {
            SyncState::Syncing => format!("{}/{}", r.done.unwrap_or(0), r.total.unwrap_or(0)),
            SyncState::Ready => "READY".into(),
            SyncState::Missing => format!("MISSING {}", r.missing.len()),
            SyncState::Offline => "OFFLINE".into(),
        };
        // the scene last: the HUD cuts at 40 characters
        Some(format!("PUSH {state} {}", r.scene).to_uppercase().chars().take(40).collect())
    }

    /// Block this (worker) thread until `want` is settled, at most `timeout`; false = it wasn't (or no sync runs).
    pub fn wait(&self, want: Want, timeout: Duration) -> bool {
        let (tx, rx) = mpsc::channel();
        if self.tx.send(Msg::Want(want, tx)).is_err() {
            return false;
        }
        rx.recv_timeout(timeout).is_ok()
    }

    /// the HUD's line
    pub fn line(&self) -> String {
        self.board.lock().map(|b| b.line()).unwrap_or_default()
    }

    /// the set a file of the sync's sets folder is (`<sets>/<id>/<id>.<quality>.glb`); None = not the sync's
    pub fn set_want(&self, path: &Path) -> Option<Want> {
        let sets = self.sets.as_ref()?;
        let dir = path.parent()?;
        let id = dir.file_name()?.to_str()?;
        let (stem, quality) = path.file_name()?.to_str()?.strip_suffix(".glb")?.rsplit_once('.')?;
        (dir.parent()? == sets.as_path() && stem == id).then(|| Want::Set { id: id.into(), quality: quality.into() })
    }

    /// the set loader's gate (space::Gate): a synced set's file waits for the sync
    #[cfg(feature = "space")]
    pub fn gate(&self) -> crate::space::Gate {
        let sync = self.clone();
        Arc::new(move |path: &Path| {
            if let Some(w) = sync.set_want(path) {
                sync.wait(w, SET_WAIT);
            }
        })
    }

    /// the heads' sprites through the sync (None: `--portraits` names a local folder – read it as it is)
    #[cfg(feature = "space")]
    pub fn portraits(&self) -> Option<Portraits> {
        Some(Portraits { local: crate::space::portrait::LocalDir(self.portraits.clone()?), sync: self.clone() })
    }
}

/// the link's side of the sync (#195): a deck's link hands it the conductor's pushes and sends its answers
#[cfg(all(feature = "live", feature = "space"))]
impl crate::space::link::Syncs for DeckSync {
    fn push(&self, sync: SceneSync) {
        DeckSync::push(self, sync);
    }

    fn synced(&self, seen: u64) -> Option<(u64, DeckSynced)> {
        DeckSync::synced(self, seen)
    }
}

/// What a scene with a space node needs, before its space is built (so the one request names it all): every space
/// node's set (by id) and the being's character with the being's whole palette – `look.moods`, then the moods its
/// feelings and nudges name (Feel::new's order).
#[cfg(feature = "space")]
pub fn needs_of(graph: &crate::graph::Graph, look: &crate::space::being::Look) -> Needs {
    let sets = (0..graph.nodes.len()).filter(|&i| graph.nodes[i].kind == crate::graph::Kind::Space).filter_map(|i| crate::space::set_need(graph, i)).collect();
    if look.character.is_empty() {
        return Needs { sets, characters: Vec::new() };
    }
    let mut moods = look.moods.clone();
    for m in look.feel.iter().map(|r| &r.mood).chain(look.react.iter().map(|n| &n.mood)) {
        if !moods.contains(m) {
            moods.push(m.clone());
        }
    }
    Needs { sets, characters: vec![(look.character.clone(), moods)] }
}

/// The portraits through the sync (a portrait::Source next to LocalDir): wait for the sprite, then read the folder.
#[cfg(feature = "space")]
pub struct Portraits {
    local: crate::space::portrait::LocalDir,
    sync: DeckSync,
}

#[cfg(feature = "space")]
impl crate::space::portrait::Source for Portraits {
    fn load(&self, character: &str, mood: &str) -> Result<image::RgbaImage, String> {
        self.sync.wait(Want::Portrait { character: character.into(), mood: mood.into() }, PORTRAIT_WAIT);
        self.local.load(character, mood)
    }
}

/// which CDN file each local file came from: `<cache>/sync/index.json`
#[derive(Default, Serialize, Deserialize)]
struct Index {
    #[serde(skip)]
    at: PathBuf,
    files: BTreeMap<String, File>,
}

impl Index {
    fn load(cache: &Path) -> Index {
        let at = cache.join("sync").join("index.json");
        let mut index: Index = std::fs::read_to_string(&at).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        index.at = at;
        index
    }

    fn save(&self) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        write_atomic(&self.at, text.as_bytes())
    }
}

/// #195: the conductor's push being worked on: its items (`set:<id>`, `character:<id>`) and the files each needs, and
/// why an item can't be had at all (unknown to the API, offline without a cached copy)
struct Push {
    scene: String,
    items: Vec<(String, Vec<Want>)>,
    why: HashMap<String, String>,
}

struct Worker {
    cfg: Config,
    agent: ureq::Agent,
    board: Arc<Mutex<Board>>,
    answer: Arc<Mutex<Answer>>,
    push: Option<Push>,
    /// why a file failed (until it is wanted again)
    failed: HashMap<Want, String>,
    /// bytes downloaded since the last push
    pushed_bytes: u64,
    content: Content,
    /// set / character ids the API has answered (or named missing) since the last prepare
    known: HashSet<(bool, String)>,
    /// offline: not before this
    retry_at: Option<Instant>,
    index: Index,
    settled: HashSet<Want>,
    queue: VecDeque<Want>,
    waiters: HashMap<Want, Vec<Sender<()>>>,
}

impl Worker {
    fn run(mut self, rx: Receiver<Msg>) {
        loop {
            let first = if self.queue.is_empty() {
                match rx.recv() {
                    Ok(m) => Some(m),
                    Err(_) => break,
                }
            } else {
                None
            };
            for m in first.into_iter().chain(std::iter::from_fn(|| rx.try_recv().ok())) {
                self.handle(m);
            }
            if let Some(w) = self.queue.pop_front() {
                self.publish(|b| b.busy = true);
                match self.fetch(&w) {
                    Ok(()) => {
                        self.failed.remove(&w);
                    }
                    Err(e) => {
                        tracing::warn!("sync: {e}");
                        self.failed.insert(w.clone(), e.clone());
                        self.publish(|b| b.error = Some(e));
                    }
                }
                self.settled.insert(w.clone());
                for s in self.waiters.remove(&w).unwrap_or_default() {
                    let _ = s.send(());
                }
                let queued = self.queue.len();
                self.publish(|b| {
                    b.done += 1;
                    b.queued = queued;
                    b.busy = queued > 0;
                });
                self.answer(false);
            }
        }
    }

    /// #195: the push's answer now – kept (numbered) when it changed, or always (`fresh`: a new push wants an answer
    /// even when it is the old one)
    fn answer(&self, fresh: bool) {
        let Some(p) = &self.push else { return };
        let (mut ready, mut missing, mut why) = (Vec::new(), Vec::new(), HashMap::new());
        let mut pending = false;
        let (mut total, mut done) = (0u64, 0u64);
        for (item, wants) in &p.items {
            total += wants.len() as u64;
            done += wants.iter().filter(|w| self.settled.contains(w)).count() as u64;
            let failed = wants.iter().find_map(|w| self.failed.get(w));
            if let Some(reason) = p.why.get(item).or(failed) {
                missing.push(item.clone());
                why.insert(item.clone(), reason.clone());
            } else if !wants.iter().all(|w| self.settled.contains(w)) {
                pending = true;
            } else if wants.iter().all(|w| self.present(w)) {
                ready.push(item.clone());
            } else {
                missing.push(item.clone());
                why.insert(item.clone(), "not in the cache".into());
            }
        }
        let offline = self.board.lock().map(|b| b.offline).unwrap_or(false);
        let state = match () {
            _ if pending => SyncState::Syncing,
            _ if missing.is_empty() => SyncState::Ready,
            _ if offline => SyncState::Offline,
            _ => SyncState::Missing,
        };
        let r = DeckSynced {
            scene: p.scene.clone(),
            state,
            done: Some(done),
            total: Some(total),
            bytes: Some(self.pushed_bytes),
            ready,
            missing,
            why: (!why.is_empty()).then_some(why),
        };
        if let Ok(mut a) = self.answer.lock() {
            if fresh || a.last.as_ref() != Some(&r) {
                a.n += 1;
                a.last = Some(r);
            }
        }
    }

    /// is a settled file there to play (a kind that isn't synced – a local folder – always is)?
    fn present(&self, w: &Want) -> bool {
        match w {
            Want::Set { id, quality } => self.cfg.sets.as_ref().map_or(true, |d| d.join(id).join(format!("{id}.{quality}.glb")).is_file()),
            Want::Portrait { character, mood } => self.cfg.portraits.as_ref().map_or(true, |d| {
                let stem = d.join(character).join(format!("{character}_portrait-{mood}"));
                ["webp", "png"].iter().any(|ext| stem.with_extension(ext).is_file())
            }),
        }
    }

    /// the moods of `character` the cache holds (offline: what a push can still count on)
    fn cached_moods(&self, character: &str) -> Vec<String> {
        let Some(dir) = &self.cfg.portraits else { return Vec::new() };
        let prefix = format!("{character}_portrait-");
        let mut moods: Vec<String> = std::fs::read_dir(dir.join(character))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                let (stem, ext) = name.rsplit_once('.')?;
                matches!(ext, "webp" | "png").then(|| stem.strip_prefix(&prefix).map(str::to_owned)).flatten()
            })
            .collect();
        moods.sort();
        moods.dedup();
        moods
    }

    /// #195: a push becomes wants – asked for afresh (a changed file is noticed), each file checked again
    fn take_push(&mut self, sync: SceneSync) {
        let sets = sync.sets.unwrap_or_default();
        let chars = sync.characters.unwrap_or_default();
        tracing::info!("sync: the conductor pushes '{}' – sets {sets:?}, characters {chars:?}", sync.scene);
        for id in &sets {
            self.known.remove(&(true, id.clone()));
        }
        for id in &chars {
            self.known.remove(&(false, id.clone()));
        }
        self.retry_at = None;
        self.pushed_bytes = 0;
        if self.queue.is_empty() {
            self.publish(|b| *b = Board { busy: true, ..Board::default() });
        }
        let s: Vec<&str> = sets.iter().map(String::as_str).collect();
        let c: Vec<&str> = chars.iter().map(String::as_str).collect();
        self.ask(&s, &c);
        let mut push = Push { scene: sync.scene, items: Vec::new(), why: HashMap::new() };
        for id in sets {
            push.items.push((format!("set:{id}"), vec![Want::Set { id, quality: self.cfg.quality.clone() }]));
        }
        for c in chars {
            let item = format!("character:{c}");
            let moods: Vec<String> = match self.content.characters.get(&c) {
                Some(cc) => {
                    let mut m: Vec<String> = cc.portraits.keys().cloned().collect();
                    m.sort();
                    m
                }
                None if self.content.missing.characters.contains(&c) => {
                    push.why.insert(item.clone(), format!("character {c} is unknown to the API"));
                    Vec::new()
                }
                None => {
                    // offline (or the API failed): what the cache has of it
                    let m = self.cached_moods(&c);
                    if m.is_empty() {
                        push.why.insert(item.clone(), "offline - not in the cache".into());
                    }
                    m
                }
            };
            push.items.push((item, moods.into_iter().map(|mood| Want::Portrait { character: c.clone(), mood }).collect()));
        }
        for w in push.items.iter().flat_map(|(_, ws)| ws.iter()) {
            self.settled.remove(w);
            self.failed.remove(w);
            if !self.queue.contains(w) {
                self.queue.push_back(w.clone());
            }
        }
        self.push = Some(push);
        self.answer(true);
    }

    fn publish(&self, f: impl FnOnce(&mut Board)) {
        if let Ok(mut b) = self.board.lock() {
            f(&mut b);
        }
    }

    fn handle(&mut self, m: Msg) {
        match m {
            Msg::Prepare(needs) => {
                // a scene load asks afresh (a changed file is noticed here) and checks every file again
                self.known.clear();
                self.retry_at = None;
                self.publish(|b| *b = Board { busy: true, ..Board::default() });
                let sets: Vec<&str> = needs.sets.iter().map(|(id, _)| id.as_str()).collect();
                let chars: Vec<&str> = needs.characters.iter().map(|(c, _)| c.as_str()).collect();
                self.ask(&sets, &chars);
                for w in needs.wants() {
                    self.settled.remove(&w);
                    if !self.queue.contains(&w) {
                        self.queue.push_back(w);
                    }
                }
            }
            Msg::Push(sync) => self.take_push(sync),
            Msg::Want(w, reply) => {
                if self.settled.contains(&w) {
                    let _ = reply.send(());
                    return;
                }
                self.waiters.entry(w.clone()).or_default().push(reply);
                // a loader is waiting: before the rest of the queue
                self.queue.retain(|q| *q != w);
                self.queue.push_front(w);
            }
        }
        let queued = self.queue.len();
        self.publish(|b| {
            b.queued = queued;
            b.busy = queued > 0;
        });
    }

    /// GET scene-content for these ids; merged into `content`. Offline → the board says so, retried after RETRY.
    fn ask(&mut self, sets: &[&str], characters: &[&str]) {
        if sets.is_empty() && characters.is_empty() || self.retry_at.is_some_and(|t| Instant::now() < t) {
            return;
        }
        let url = format!("{}/library/scene-content?sets={}&characters={}", self.cfg.api.trim_end_matches('/'), sets.join(","), characters.join(","));
        let mut req = self.agent.get(&url);
        if let Some(t) = self.cfg.token.get() {
            req = req.set("Authorization", &format!("Bearer {t}"));
        }
        let answer = match req.call() {
            Ok(r) => r.into_string().map_err(|e| e.to_string()).and_then(|t| serde_json::from_str::<Content>(&t).map_err(|e| format!("not an answer: {e}"))),
            Err(ureq::Error::Status(code, _)) => Err(format!("API HTTP {code}")),
            Err(ureq::Error::Transport(t)) => {
                tracing::warn!("sync: {url}: {t} – playing from the cache");
                self.retry_at = Some(Instant::now() + RETRY);
                self.publish(|b| b.offline = true);
                return;
            }
        };
        match answer {
            Ok(c) => {
                tracing::info!("sync: {} sets, {} characters ({} missing)", c.sets.len(), c.characters.len(), c.missing.sets.len() + c.missing.characters.len());
                for id in c.missing.sets.iter().chain(c.sets.keys()) {
                    self.known.insert((true, id.clone()));
                }
                for id in c.missing.characters.iter().chain(c.characters.keys()) {
                    self.known.insert((false, id.clone()));
                }
                self.content.sets.extend(c.sets);
                self.content.characters.extend(c.characters);
                self.content.missing.sets.extend(c.missing.sets);
                self.content.missing.characters.extend(c.missing.characters);
                self.publish(|b| b.offline = false);
            }
            Err(e) => {
                tracing::warn!("sync: {url}: {e}");
                self.retry_at = Some(Instant::now() + RETRY);
                self.publish(|b| b.error = Some(e));
            }
        }
    }

    /// one wanted file: asked for if the API hasn't said, then fetched unless the cache has it. Ok = settled quietly
    /// (fetched, cached, nothing to fetch, or offline with the cache).
    fn fetch(&mut self, w: &Want) -> Result<(), String> {
        let (is_set, id) = match w {
            Want::Set { id, .. } => (true, id.as_str()),
            Want::Portrait { character, .. } => (false, character.as_str()),
        };
        if !self.known.contains(&(is_set, id.to_string())) {
            let id = id.to_string();
            if is_set { self.ask(&[&id], &[]) } else { self.ask(&[], &[&id]) }
        }
        let (local, file) = match w {
            Want::Set { id, quality } => {
                let Some(dir) = &self.cfg.sets else { return Ok(()) };
                let local = dir.join(id).join(format!("{id}.{quality}.glb"));
                match self.content.sets.get(id) {
                    Some(s) => match s.model.get(quality) {
                        Some(f) => (local, f.clone()),
                        None => return Err(format!("set {id} has no 3D at {quality}")),
                    },
                    None if self.content.missing.sets.contains(id) => return Err(format!("set {id} is unknown to the API")),
                    None => return Ok(()), // offline: the cache
                }
            }
            Want::Portrait { character, mood } => {
                let Some(dir) = &self.cfg.portraits else { return Ok(()) };
                match self.content.characters.get(character) {
                    Some(c) => match c.portraits.get(mood) {
                        Some(f) => {
                            let ext = Path::new(&f.path).extension().and_then(|e| e.to_str()).unwrap_or("webp");
                            (dir.join(character).join(format!("{character}_portrait-{mood}.{ext}")), f.clone())
                        }
                        None => return Ok(()), // no sprite for this mood: the head shows its nearest
                    },
                    None if self.content.missing.characters.contains(character) => return Err(format!("character {character} is unknown to the API")),
                    None => return Ok(()),
                }
            }
        };
        self.download(&local, &file)
    }

    /// `file` from the CDN into `local` – unless the index says `local` is that file and its size agrees
    fn download(&mut self, local: &Path, file: &File) -> Result<(), String> {
        let key = local.to_string_lossy().to_string();
        let len = std::fs::metadata(local).ok().map(|m| m.len());
        if len.is_some() && self.index.files.get(&key).is_some_and(|f| f.path == file.path) && file.size.map_or(true, |s| Some(s) == len) {
            self.publish(|b| b.cached += 1);
            return Ok(());
        }
        let url = format!("{}/{}", self.cfg.cdn.trim_end_matches('/'), file.path.trim_start_matches('/'));
        let bytes = match self.agent.get(&url).call() {
            Ok(r) => {
                let mut bytes = Vec::new();
                r.into_reader().take(MAX_FILE + 1).read_to_end(&mut bytes).map_err(|e| format!("{url}: {e}"))?;
                bytes
            }
            Err(ureq::Error::Status(code, _)) => return Err(format!("{}: HTTP {code}", file.path)),
            Err(ureq::Error::Transport(t)) => {
                tracing::warn!("sync: {url}: {t}");
                self.publish(|b| b.offline = true);
                return Ok(()); // the cached copy, if any, plays
            }
        };
        check(&file.path, file.size, &bytes)?;
        write_atomic(local, &bytes)?;
        self.index.files.insert(key, file.clone());
        self.index.save()?;
        let n = bytes.len() as u64;
        tracing::info!("sync: {} → {} ({n} bytes)", file.path, local.display());
        self.pushed_bytes += n;
        self.publish(|b| {
            b.fetched += 1;
            b.bytes += n;
        });
        Ok(())
    }
}

/// a download is what the answer said: its size, and its type's magic (a broken or truncated file is refused)
pub fn check(path: &str, size: Option<u64>, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() as u64 > MAX_FILE {
        return Err(format!("{path}: larger than {} MB", MAX_FILE >> 20));
    }
    if let Some(s) = size {
        if bytes.len() as u64 != s {
            return Err(format!("{path}: {} bytes, the API said {s} – refused", bytes.len()));
        }
    }
    let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let ok = match ext.as_str() {
        "glb" => bytes.starts_with(b"glTF"),
        "webp" => bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
        "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{path}: not a .{ext} file – refused"))
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_become_wants_sets_first() {
        let n = Needs { sets: vec![("cathedral".into(), "low".into())], characters: vec![("plf".into(), vec!["neutral".into(), "smug".into()])] };
        assert_eq!(
            n.wants(),
            [
                Want::Set { id: "cathedral".into(), quality: "low".into() },
                Want::Portrait { character: "plf".into(), mood: "neutral".into() },
                Want::Portrait { character: "plf".into(), mood: "smug".into() },
            ]
        );
    }

    #[test]
    fn a_download_must_match_its_size_and_type() {
        assert!(check("a.glb", Some(8), b"glTF\x02\0\0\0").is_ok());
        assert!(check("a.glb", Some(9), b"glTF\x02\0\0\0").unwrap_err().contains("the API said 9"));
        assert!(check("a.glb", None, b"<html>oops").unwrap_err().contains("not a .glb"));
        assert!(check("a.webp", None, b"RIFF\0\0\0\0WEBPVP8 ").is_ok());
        assert!(check("a.png", None, b"RIFF\0\0\0\0WEBP").is_err());
    }

    #[test]
    fn only_the_sync_s_own_set_files_are_gated() {
        let (tx, _rx) = mpsc::channel();
        let s = DeckSync { tx, board: Default::default(), answer: Default::default(), sets: Some(PathBuf::from("/c/sets")), portraits: None };
        let p = Path::new("/c/sets").join("cathedral").join("cathedral.low.glb");
        assert_eq!(s.set_want(&p), Some(Want::Set { id: "cathedral".into(), quality: "low".into() }));
        assert_eq!(s.set_want(&Path::new("/c/sets").join("cathedral.low.glb")), None, "a flat build output");
        assert_eq!(s.set_want(Path::new("/elsewhere/cathedral/cathedral.low.glb")), None);
        assert_eq!(s.set_want(&Path::new("/c/sets").join("x").join("cathedral.low.glb")), None);
    }

    /// the one request names the being's whole palette – what its head will ask for
    #[cfg(feature = "space")]
    #[test]
    fn a_scene_s_needs_are_its_set_and_the_being_s_palette() {
        use crate::scene::{Scene, CONDUCTOR_SCENE};
        let scene = crate::modules::carried_part(&Scene::parse(CONDUCTOR_SCENE).unwrap());
        let g = crate::graph::Graph::build(&scene).unwrap();
        let needs = needs_of(&g, &scene.being);
        assert_eq!(needs.sets, [("cathedral".to_string(), "low".to_string())]);
        let being = crate::space::being::Being::new(&g, &scene.being);
        assert_eq!(needs.characters, [("plf".to_string(), being.moods.clone())]);
    }

    #[test]
    fn the_board_speaks_for_the_hud() {
        assert_eq!(Board { busy: true, done: 2, queued: 3, bytes: 3 << 20, ..Board::default() }.line(), "SYNC 2/5 3.0 MB");
        assert_eq!(Board { offline: true, ..Board::default() }.line(), "SYNC OFFLINE - FROM THE CACHE");
        assert_eq!(Board { fetched: 1, cached: 4, ..Board::default() }.line(), "SYNC OK 1 NEW 4 CACHED");
        assert!(Board { error: Some("set x is unknown".into()), ..Board::default() }.line().starts_with("SYNC ERR SET X"));
    }
}
