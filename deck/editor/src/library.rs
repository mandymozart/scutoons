//! The library panel (deck-kit `library`, the core's entries from deck-catalog / deck-space `library`) in Build, Show and
//! Module: what each workspace loads into the one browser, the previews' pictures – a still the stage drew of what it
//! played, kept in the cache by key, and an asset's picture fetched once – and the card's way out: dropped where the
//! workspace takes it (Build's stage: a track; Show's cell: its piece), opened in its editor view on a second press.
use crate::show::api::Api;
use deck_catalog::library::shelf::{Context, Shelf, Use};
use deck_catalog::library::{self, Entry, Kind, Preview, Purpose};
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::library::{Browser, Did, Source};
use image::RgbaImage;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

pub const PANEL: &str = "library";
/// the panel's width: three cards
pub const WIDTH: f32 = 3.0 * 132.0 + 2.0 * 8.0;
/// a still's size, twice a card's preview
pub const STILL: (u32, u32) = (264, 168);
/// frames a thing plays before its still is taken
const SETTLE: u64 = 90;
/// the pictures the screen holds at once (the newest; an older one shows its kind again)
const KEPT: usize = 48;

/// the previews' pictures: the cache's stills and fetched pictures by key, each a picture id of the screen
pub struct Pictures {
    dir: PathBuf,
    loaded: HashMap<String, u64>,
    pub images: Vec<(u64, RgbaImage)>,
    asked: HashSet<String>,
    tx: Sender<(String, Option<RgbaImage>)>,
    rx: Receiver<(String, Option<RgbaImage>)>,
    /// the screen has not got the newest pictures yet
    pub changed: bool,
}

fn id_of(key: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut h);
    h.finish()
}

/// a still's or a picture's key in the cache
fn key_of(p: &Preview) -> Option<&str> {
    match p {
        Preview::Still { key } => Some(key),
        Preview::Picture { url } => Some(url),
        _ => None,
    }
}

/// a picture cut to the still's shape (its middle) and made the still's size
pub fn fitted(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let want = STILL.0 as f32 / STILL.1 as f32;
    let (cw, ch) = if w as f32 / h.max(1) as f32 > want { ((h as f32 * want) as u32, h) } else { (w, (w as f32 / want) as u32) };
    let crop = image::imageops::crop_imm(img, (w - cw.min(w)) / 2, (h - ch.min(h)) / 2, cw.max(1), ch.max(1)).to_image();
    image::imageops::resize(&crop, STILL.0, STILL.1, image::imageops::FilterType::Triangle)
}

impl Pictures {
    pub fn new(dir: PathBuf) -> Pictures {
        let (tx, rx) = channel();
        Pictures { dir, loaded: HashMap::new(), images: Vec::new(), asked: HashSet::new(), tx, rx, changed: false }
    }

    fn file(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{:016x}.png", id_of(key)))
    }

    /// the screen's picture of a preview, once it is there
    pub fn get(&self, p: &Preview) -> Option<u64> {
        key_of(p).and_then(|k| self.loaded.get(k).copied())
    }

    /// the screen's picture of `key`, once it is there
    pub fn of(&self, key: &str) -> Option<u64> {
        self.loaded.get(key).copied()
    }

    /// whether the cache has a picture of `key`
    pub fn has(&self, key: &str) -> bool {
        self.loaded.contains_key(key)
    }

    /// The previews of `entries` asked for once each: a still from the cache, a picture from the cache or the net (on a
    /// thread).
    pub fn want(&mut self, entries: &[Entry]) {
        for e in entries {
            if let Some(key) = key_of(&e.preview) {
                self.ask(key, matches!(e.preview, Preview::Picture { .. }));
            }
        }
    }

    /// the stills of `keys` (a node's piece) asked of the cache once each
    pub fn want_stills(&mut self, keys: &[String]) {
        for k in keys {
            self.ask(k, false);
        }
    }

    fn ask(&mut self, key: &str, fetch: bool) {
        if !self.asked.insert(key.to_string()) {
            return;
        }
        {
            let (file, tx, key) = (self.file(key), self.tx.clone(), key.to_string());
            let url = fetch.then(|| key.clone());
            std::thread::spawn(move || {
                let img = image::open(&file).ok().map(|i| i.to_rgba8()).or_else(|| {
                    let url = url?;
                    let bytes = if std::path::Path::new(&url).is_file() { std::fs::read(&url).ok()? } else { Api::bytes_of(&url).ok()? };
                    let img = fitted(&image::load_from_memory(&bytes).ok()?.to_rgba8());
                    if let Some(dir) = file.parent() {
                        std::fs::create_dir_all(dir).ok();
                    }
                    img.save(&file).ok();
                    Some(img)
                });
                let _ = tx.send((key, img));
            });
        }
    }

    /// what the threads brought
    pub fn poll(&mut self) {
        while let Ok((key, img)) = self.rx.try_recv() {
            if let Some(img) = img {
                self.show(key, img);
            }
        }
    }

    fn show(&mut self, key: String, img: RgbaImage) {
        let id = id_of(&key);
        self.images.retain(|(i, _)| *i != id);
        self.images.push((id, img));
        self.loaded.insert(key, id);
        if self.images.len() > KEPT {
            let (gone, _) = self.images.remove(0);
            self.loaded.retain(|_, i| *i != gone);
        }
        self.changed = true;
    }

    /// a still the stage drew of `key`: kept in the cache and shown
    pub fn keep(&mut self, key: &str, img: RgbaImage) {
        let img = fitted(&img);
        std::fs::create_dir_all(&self.dir).ok();
        if let Err(e) = img.save(self.file(key)) {
            tracing::warn!("a still of {key}: {e}");
        }
        self.asked.insert(key.to_string());
        self.show(key.to_string(), img);
    }
}

/// the toon's assets, asked once signed in: their families (deck-catalog `library::shelf`) and one entry per family
#[derive(Default)]
struct Assets {
    shelf: Option<Shelf>,
    list: Vec<Entry>,
    /// every file its own entry (the browser's single levels)
    singles: Vec<Entry>,
    /// where their paths are served from (the answer's `cdn`)
    cdn: String,
    rx: Option<Receiver<Result<Answer, String>>>,
    asked: bool,
}

/// the assets' families and their CDN base
type Answer = (Shelf, String);

/// the assets' answer's CDN base
fn cdn_of(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text).ok().and_then(|v| v["cdn"].as_str().map(str::to_string)).unwrap_or_default()
}

/// `GET /api/assets`' answer as the shelf's families and their CDN base
fn answer(text: &str) -> Result<Answer, String> {
    Ok((Shelf::from_json(text)?, cdn_of(text)))
}

impl Assets {
    fn take(&mut self, (mut shelf, cdn): Answer, context: &Context) {
        shelf.set_context(context.clone());
        self.list = shelf.families(false).iter().map(|f| f.entry_on(&cdn)).collect();
        self.singles = shelf.families(true).iter().map(|f| f.entry_on(&cdn)).collect();
        (self.shelf, self.cdn) = (Some(shelf), cdn);
    }
}

/// A scene's context: what its stage shows (the background's and the layers' media) is used by it.
pub fn scene_context(id: &str, toons: &[String], media: Vec<String>, toon: Option<&str>) -> Context {
    let open = format!("scene:{id}");
    let uses = media.into_iter().map(|path| Use { path, item: open.clone(), part: "stage".into(), at: String::new() }).collect();
    Context { toon: toons.first().cloned().or_else(|| toon.map(str::to_string)), open: Some(open), uses, ..Context::default() }
}

/// A character rig's context: no use index for a rig, its files are the ones in its character's folder.
pub fn rig_context(character: &str, toon: Option<&str>) -> Context {
    let id = character.strip_prefix("character:").unwrap_or(character).split('.').next().unwrap_or(character);
    Context { open: Some(format!("character:{id}")), toon: toon.map(str::to_string), by_home: true, ..Context::default() }
}

/// now in ms since 1970: the shelf's "recently added"
pub fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

/// the library panel's state, one for the editor
pub struct Library {
    pub browser: Browser,
    /// this frame's entries, the workspace's
    pub entries: Vec<Entry>,
    pub purpose: Purpose,
    pub pictures: Pictures,
    assets: Assets,
    /// what plays on the stage and since which frame, until its still is taken
    playing: Option<(String, u64)>,
    /// the rig the "related" filter last turned itself on for (an empty character rig: its modules first)
    related_for: Option<String>,
    /// the open item and what it uses, handed to the shelf
    context: Context,
    /// counts the shelf's and its context's changes (the browser asks its view again)
    stamp: u64,
    /// the API the assets came from (an upload goes there, then they are asked again)
    base: String,
    /// a dropped file on its way: the path it lands at, or why not
    uploading: Option<Receiver<Result<String, String>>>,
    /// the file just uploaded: its card is chosen once the assets come back
    fresh: Option<String>,
}

impl Library {
    pub fn new(cache: PathBuf) -> Library {
        Library { browser: Browser::default(), entries: Vec::new(), purpose: Purpose::Build, pictures: Pictures::new(cache.join("stills")), assets: Assets::default(), playing: None, related_for: None, context: Context::default(), stamp: 0, base: String::new(), uploading: None, fresh: None }
    }

    /// the workspace's entries this frame, only the kinds its purpose loads
    pub fn set(&mut self, purpose: Purpose, mut entries: Vec<Entry>) {
        entries.retain(|e| purpose.kinds().contains(&e.kind));
        if purpose != self.purpose {
            self.browser.filter.kinds.clear();
            self.browser.held = None;
        }
        self.purpose = purpose;
        self.pictures.want(&entries);
        self.entries = entries;
    }

    /// the assets once, on a thread (signed in: the API answers only an editor)
    pub fn ask_assets(&mut self, base: &str) {
        if std::mem::replace(&mut self.assets.asked, true) {
            return;
        }
        self.base = base.to_string();
        let (tx, rx) = channel();
        let api = Api::new(base.to_string());
        std::thread::spawn(move || {
            let _ = tx.send(api.assets().and_then(|t| answer(&t)));
        });
        self.assets.rx = Some(rx);
    }

    /// A check run's catalog folder (`--catalog`): its assets (`assets.json`, `/assets`) and the Module workspace's list
    /// of its modules (`modules.json`).
    pub fn answers(&mut self, dir: &std::path::Path) -> Option<Vec<crate::module::Entry>> {
        if let Ok(text) = std::fs::read_to_string(dir.join("assets.json")) {
            self.assets.asked = true;
            match answer(&text) {
                Ok(a) => {
                    self.assets.take(a, &self.context);
                    self.stamp += 1;
                }
                Err(e) => tracing::warn!("--catalog assets: {e}"),
            }
        }
        let text = std::fs::read_to_string(dir.join("modules.json")).ok()?;
        crate::module::entries(&text).map_err(|e| tracing::warn!("--catalog modules: {e}")).ok()
    }

    /// the assets' entries: one per family, or one per file when the browser shows the levels single
    pub fn assets(&self) -> Vec<Entry> {
        if self.browser.single { self.assets.singles.clone() } else { self.assets.list.clone() }
    }

    /// where the assets' paths are served from (empty until they are known)
    pub fn cdn(&self) -> &str {
        &self.assets.cdn
    }

    /// what the threads brought; a word for the status line when an upload ended
    pub fn poll(&mut self) -> Option<String> {
        self.pictures.poll();
        if let Some(Ok(r)) = self.assets.rx.as_ref().map(Receiver::try_recv) {
            match r {
                Ok(a) => {
                    self.assets.take(a, &self.context);
                    self.stamp += 1;
                    self.choose_fresh();
                }
                Err(e) => tracing::info!("the library's assets: {e}"),
            }
            self.assets.rx = None;
        }
        let done = self.uploading.as_ref().and_then(|rx| rx.try_recv().ok())?;
        self.uploading = None;
        Some(match done {
            Ok(path) => {
                self.fresh = Some(path.clone());
                self.assets.asked = false;
                let base = self.base.clone();
                self.ask_assets(&base);
                format!("{path} uploaded")
            }
            Err(e) => format!("the upload: {e}"),
        })
    }

    /// Where a file dropped on the editor lands: the core's upload path for the open item (its folder, else the toon's).
    pub fn upload_target(&self, file_name: &str) -> Option<String> {
        self.assets.shelf.as_ref()?.upload_path(file_name, None)
    }

    /// a file dropped on the editor: uploaded to the open item's folder on a thread; the path it goes to, or why not
    pub fn upload(&mut self, file: &std::path::Path) -> Result<String, String> {
        if self.uploading.is_some() {
            return Err("one upload at a time: the last one is still on its way".into());
        }
        let name = file.file_name().and_then(|n| n.to_str()).ok_or("the file has no name")?;
        let path = self.upload_target(name).ok_or("no folder for it: open a scene of a toon or a character's rig first")?;
        let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
        let (tx, rx) = channel();
        let (api, to) = (Api::new(self.base.clone()), path.clone());
        std::thread::spawn(move || {
            let _ = tx.send(api.upload_asset(&to, &bytes));
        });
        self.uploading = Some(rx);
        Ok(path)
    }

    /// the file just uploaded, chosen once its family is on the shelf
    fn choose_fresh(&mut self) {
        let Some(path) = self.fresh.take() else { return };
        if let Some(f) = self.assets.shelf.as_ref().and_then(|s| s.family_in(&path, self.browser.single)) {
            self.browser.chosen = vec![f.main.clone()];
        }
    }

    /// What the stage plays now (`module:walk`, `character:catpope`); true = take its still after this frame.
    pub fn playing(&mut self, key: Option<String>, frame: u64) -> bool {
        match (key, &self.playing) {
            (None, _) => self.playing = None,
            (Some(k), Some((p, _))) if *p == k => {}
            (Some(k), _) => self.playing = Some((k, frame)),
        }
        matches!(&self.playing, Some((k, since)) if frame >= since + SETTLE && !self.pictures.has(k))
    }

    /// the still the stage drew of what plays
    pub fn still(&mut self, img: RgbaImage) {
        if let Some((key, _)) = self.playing.clone() {
            self.pictures.keep(&key, img);
        }
    }

    /// the panel's content in `a`; `action` = the workspace's one button
    pub fn draw(&mut self, d: &mut DrawList, t: &Tokens, a: Rect, pointer: (f32, f32), action: Option<&str>) {
        let pictures = &self.pictures;
        let src = Source { entries: &self.entries, shelf: self.assets.shelf.as_ref(), stamp: self.stamp, now_ms: now_ms() };
        self.browser.draw(d, t, (a, pointer), &src, &|p| pictures.get(p), action);
    }

    /// An empty character rig turns "related" on once, with the modules: what is made for it first (owner 2026-11-16
    /// [rigs]); any other rig leaves the filter as the viewer set it.
    pub fn follow_rig(&mut self, subject: &str, character: Option<&str>, empty: bool) {
        if self.related_for.as_deref() == Some(subject) {
            return;
        }
        self.related_for = Some(subject.to_string());
        if let (Some(_), true) = (character, empty) {
            self.browser.filter.related = Some(String::new());
            self.browser.filter.kinds = vec![Kind::Module];
        }
    }

    /// the open item and what it uses: the shelf takes it when it changed
    pub fn set_context(&mut self, context: Context) {
        if context == self.context {
            return;
        }
        if let Some(s) = self.assets.shelf.as_mut() {
            s.set_context(context.clone());
        }
        self.context = context;
        self.stamp += 1;
    }

    /// the family of a file's entry
    pub fn family(&self, e: &Entry) -> Option<&deck_catalog::library::family::Family> {
        self.assets.shelf.as_ref().filter(|_| e.kind.is_file()).and_then(|s| s.family_in(&e.address, self.browser.single))
    }

    /// whether this workspace takes `e`: a file by its family (its kind, then the fit), anything else by its kind
    pub fn takes(&self, e: &Entry) -> Result<(), String> {
        match self.family(e) {
            Some(f) => self.purpose.takes_family(f),
            None => self.purpose.takes(e.kind),
        }
    }

    /// the entry dragged now, and whether this workspace takes it (else the core's sentence why not)
    pub fn carried(&self) -> Option<(&Entry, Result<(), String>)> {
        self.browser.dragging().and_then(|i| self.entries.get(i)).map(|e| (e, self.takes(e)))
    }
}

/// Module: the catalog's modules as the Module workspace listed them
pub fn of_modules(list: &[crate::module::Entry]) -> Vec<Entry> {
    list.iter()
        .filter_map(|m| library::module(&serde_json::json!({ "id": m.id, "factory": m.factory, "draft": m.draft, "content": m.content })))
        .collect()
}

/// Show: the scene list's scenes (the book's, then the drafts) and the book's characters and items
pub fn of_show(scenes: Vec<deck_show::rows::PieceView>, book: Option<&deck_space::rig::book::Book>) -> Vec<Entry> {
    let mut out: Vec<Entry> = scenes
        .into_iter()
        .map(|p| Entry { preview: Preview::Still { key: p.address.clone() }, address: p.address, label: p.label, detail: p.detail, kind: Kind::Scene, made_for: Vec::new() })
        .collect();
    if let Some(b) = book {
        out.extend(deck_space::library::from_book(b, crate::LOCALE).into_iter().filter(|e| e.kind != Kind::Scene));
    }
    out
}

/// where a check run lets its card go
#[derive(Debug, Clone, PartialEq)]
pub enum DragTo {
    /// the stage's middle
    Middle,
    At(f32, f32),
    /// Show's node view: `node:<id>` a node, `canvas` a free spot beside the nodes
    Nodes(String),
}

impl DragTo {
    /// `X,Y`, `node:<id>`, `canvas`, or nothing: the parts of `--library-drag` after the address (without `hold`)
    pub fn parse(parts: &[&str]) -> anyhow::Result<DragTo> {
        Ok(match parts {
            [] => DragTo::Middle,
            [to] if *to == "canvas" || to.starts_with("node:") => DragTo::Nodes(to.to_string()),
            [x, y] => DragTo::At(x.parse()?, y.parse()?),
            _ => anyhow::bail!("--library-drag ADDRESS[,X,Y | ,node:ID | ,canvas][,hold], not {}", parts.join(",")),
        })
    }
}

/// a check run's drag: the card's address, where it is let go, kept in the hand
pub type ScriptDrag = (String, DragTo, bool);

/// several cards dropped at once land this far apart, down and to the right
const DROP_STEP: f32 = 28.0;

/// seconds since the editor started: the browser's clock for a second press
pub fn now() -> f64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64()
}

/// what the stage plays that has a still: Build's character rig, Module's module once heard
pub fn playing_key(build: Option<&str>, module: Option<&str>) -> Option<String> {
    match (build, module) {
        (Some(subject), _) => subject.strip_prefix("character.").map(|c| format!("character:{}", c.split('.').next().unwrap_or(c))),
        (_, Some(id)) => Some(format!("module:{id}")),
        _ => None,
    }
}

/// Build: the catalog's module documents and instruments to drop on the rig, the book's characters to open, the assets
pub fn of_build(modules: &[serde_json::Value], instruments: &[deck_space::rig::Instrument], book: Option<&deck_space::rig::book::Book>, assets: Vec<Entry>) -> Vec<Entry> {
    let mut out: Vec<Entry> = modules.iter().filter_map(|c| library::module(&serde_json::json!({ "id": c["id"], "content": c }))).collect();
    out.extend(deck_space::library::from_instruments(instruments));
    if let Some(b) = book {
        out.extend(deck_space::library::from_book(b, crate::LOCALE).into_iter().filter(|e| e.kind == Kind::Character));
    }
    out.extend(assets);
    out
}

/// Build at a scene: the book's characters and items to fill its slots (without a book, what the deck's core rigs stand
/// for), and the pictures among the assets to fill its parallax layers
pub fn of_scene(modules: &[serde_json::Value], book: Option<&deck_space::rig::book::Book>, assets: Vec<Entry>) -> Vec<Entry> {
    let mut out: Vec<Entry> = match book {
        Some(b) => deck_space::library::from_book(b, crate::LOCALE).into_iter().filter(|e| matches!(e.kind, Kind::Character | Kind::Item)).collect(),
        None => deck_space::rig::bundle::CORE
            .iter()
            .filter_map(|(s, _)| deck_space::scene_document::FillerRef::of_rig(s))
            .map(|f| {
                let kind = if f.starts_with("item:") { Kind::Item } else { Kind::Character };
                let label = f.split_once(':').map_or(f.as_str(), |(_, id)| id).to_string();
                Entry { address: f.clone(), label, detail: f, kind, preview: Preview::Kind, made_for: Vec::new() }
            })
            .collect(),
    };
    let slotted = modules.iter().filter(|c| c["descriptor"]["slot"].is_string());
    out.extend(slotted.filter_map(|c| library::module(&serde_json::json!({ "id": c["id"], "content": c }))));
    out.extend(assets.into_iter().filter(|e| e.kind == Kind::Image));
    out
}

/// The library on the editor: its entries each frame, a card's way out, the find field's keys, a check run's drag.
impl crate::App {
    pub(crate) fn follow_library(&mut self) {
        use crate::show::Workspace;
        if let Some(word) = self.library.poll() {
            self.note(word);
        }
        let purpose = match self.workspace {
            Workspace::Stage if self.scene.as_ref().is_some_and(|v| v.inside.is_none()) => Purpose::Scene,
            Workspace::Stage => Purpose::Build,
            Workspace::Module => Purpose::Module,
            Workspace::Show => Purpose::Show,
            Workspace::Home | Workspace::Media => {
                self.show.carried = None;
                return;
            }
        };
        if matches!(purpose, Purpose::Build | Purpose::Scene) && self.auth.token().is_some() {
            self.library.ask_assets(&self.args.api);
        }
        let entries = match purpose {
            Purpose::Build => of_build(self.rigs.module_docs(), &crate::adding::instruments(&self.stage.bundle, self.rigs.instruments()), self.book.book.as_deref(), self.library.assets()),
            Purpose::Scene => of_scene(self.rigs.module_docs(), self.book.book.as_deref(), self.library.assets()),
            Purpose::Show => self.show.library_entries(),
            Purpose::Module => of_modules(&self.module.list),
        };
        self.library.set(purpose, entries);
        let rig = (self.scene.is_none()).then(|| self.stage.bundle.rig.subject.clone());
        let character = rig.as_deref().and_then(crate::toon::character).map(String::from);
        let toon = self.book.book.as_deref().map(|b| b.toon().to_string()).filter(|t| !t.is_empty());
        let context = match purpose {
            Purpose::Scene => self.scene.as_ref().map(|v| scene_context(&v.edit.doc.id, &v.edit.doc.toons, v.edit.doc.stage.media(), toon.as_deref())),
            Purpose::Build => character.as_deref().map(|c| rig_context(c, toon.as_deref())),
            Purpose::Show | Purpose::Module => None,
        };
        self.library.set_context(context.unwrap_or(Context { toon, ..Context::default() }));
        self.library.browser.character = if purpose == Purpose::Build { character.clone() } else { None };
        if let (Purpose::Build, Some(subject)) = (purpose, rig.as_deref()) {
            self.library.follow_rig(subject, character.as_deref(), crate::empty::is_empty(&self.stage.bundle));
        }
        self.show.carried = self.library.carried().filter(|_| purpose == Purpose::Show).map(|(e, takes)| takes.map(|()| e.address.clone()));
        if self.workspace == Workspace::Module {
            if let Some(j) = self.library_open.as_ref().and_then(|id| self.module.list.iter().position(|m| m.id == *id)) {
                self.library_open = None;
                let x = self.cursor.0;
                self.module.press(crate::module::Hit::Open(j), x, &self.stage);
            }
        }
        let key = match self.workspace {
            Workspace::Stage => playing_key(rig.as_deref(), None),
            Workspace::Module => playing_key(None, self.module.open.as_ref().filter(|_| self.module.heard).map(|o| o.id.as_str())),
            // a Show scene on the stage: its still is the node's preview
            Workspace::Show => self.stage.scene.as_ref().map(|id| format!("scene:{id}")),
            Workspace::Home | Workspace::Media => None,
        };
        if self.library.playing(key, self.frames) {
            self.stage.ask_still();
        }
    }

    pub(crate) fn library_did(&mut self, did: Did) {
        match did {
            // no Library of the editor carries a button of its owner's
            Did::Taken | Did::Action => {}
            Did::Click(i) if self.workspace == crate::show::Workspace::Module => self.library_open_entry(i),
            Did::Click(_) => {}
            Did::Open(i) => self.library_open_entry(i),
            Did::Drop { entry, at } => {
                self.carry = None;
                self.library_drop(entry, at)
            }
        }
    }

    /// A file dropped on the window: where the Library lists the toon's files (Build, a scene), uploaded to the open
    /// item's folder.
    pub(crate) fn library_file(&mut self, file: &std::path::Path) {
        if !matches!(self.library.purpose, Purpose::Build | Purpose::Scene) || !matches!(self.workspace, crate::show::Workspace::Stage) {
            self.note("files are added where the Library lists the toon's files: Build or a scene".into());
            return;
        }
        if self.auth.token().is_none() {
            self.note("sign in to add files".into());
            return;
        }
        let word = match self.library.upload(file) {
            Ok(path) => format!("uploading to {path}"),
            Err(e) => e,
        };
        self.note(word);
    }

    /// the pointer moved: a card dragged far enough goes into the one hand
    pub(crate) fn library_drag_to(&mut self, x: f32, y: f32) {
        self.library.browser.drag_to(x, y);
        if self.carry.is_none() {
            if let Some(e) = self.library.browser.dragging().and_then(|i| self.library.entries.get(i)) {
                let more = self.library.browser.carried().len().saturating_sub(1);
                let label = if more > 0 { format!("{} + {more}", e.label) } else { e.label.clone() };
                self.carry = Some(deck_kit::editor::carry::Carry::new(&e.address, &label));
            }
        }
    }

    /// an entry opened in its tab where the Library's open rule says, else its sentence
    fn library_open_entry(&mut self, i: usize) {
        let Some(e) = self.library.entries.get(i).cloned() else { return };
        match deck_catalog::library::open::open_entry(e.kind, &e.address, deck_catalog::library::open::Editor::Deck) {
            Ok(o) => self.open_item(&o.address, &e.label),
            Err(why) => self.note(format!("{}: {why}", e.label)),
        }
    }

    /// Cards let go at `at` (several when chosen cards were dragged): what this workspace takes lands, one after the
    /// other a little apart (Show: one cell down each); what it does not take is said once.
    fn library_drop(&mut self, entry: usize, at: (f32, f32)) {
        if self.library.browser.hit(at.0, at.1).is_some() {
            return;
        }
        let mut list: Vec<Entry> = self.library.browser.dropped().iter().filter_map(|&i| self.library.entries.get(i).cloned()).collect();
        if list.is_empty() {
            list.extend(self.library.entries.get(entry).cloned());
        }
        let (fit, refused): (Vec<Entry>, Vec<Entry>) = list.into_iter().partition(|e| self.library.takes(e).is_ok());
        if let Some(why) = refused.first().and_then(|e| self.library.takes(e).err()) {
            let why = if fit.is_empty() && refused.len() == 1 { why } else { format!("{} of {} not dropped: {why}", refused.len(), refused.len() + fit.len()) };
            self.refuse(why, at.0, at.1);
        }
        if self.workspace == crate::show::Workspace::Show {
            let pieces: Vec<String> = fit.iter().map(|e| e.address.clone()).collect();
            self.show.drop_pieces(&pieces, at.0, at.1);
            return;
        }
        for (k, e) in fit.iter().enumerate() {
            let step = DROP_STEP * k as f32;
            self.library_drop_one(e, (at.0 + step, at.1 + step));
        }
    }

    /// one card that fits let go at `at`: Build's stage takes a module or an instrument as a track, a scene its slots'
    /// and layers' pieces
    fn library_drop_one(&mut self, e: &Entry, at: (f32, f32)) {
        match self.workspace {
            crate::show::Workspace::Stage if self.library.purpose == Purpose::Scene && e.kind == Kind::Module => self.module_drop(&e.address, at.0, at.1),
            crate::show::Workspace::Stage if self.library.purpose == Purpose::Scene => {
                if !self.panels().floats.covers(at.0, at.1) {
                    self.scene_drop(&e.address, e.kind, at.0, at.1);
                }
            }
            crate::show::Workspace::Stage if !self.panels().floats.covers(at.0, at.1) => {
                let choice = match e.kind {
                    Kind::Module => self.rigs.modules().0.into_iter().find(|(_, d)| format!("module:{}", d.name) == e.address).map(|m| crate::adding::Choice::Module(Box::new(m))),
                    _ => crate::adding::instruments(&self.stage.bundle, self.rigs.instruments()).into_iter().find(|i| format!("instrument:{}", i.id) == e.address).map(|i| crate::adding::Choice::Track(Box::new(i))),
                };
                match choice {
                    Some(c) => self.added(c, at.0, at.1),
                    None => self.refuse(format!("{} is not built yet: it has no track to play", e.address), at.0, at.1),
                }
            }
            _ => {}
        }
    }

    pub(crate) fn library_key(&mut self, k: &winit::event::KeyEvent) {
        use winit::event::ElementState;
        use winit::keyboard::{Key, NamedKey};
        if k.state != ElementState::Pressed {
            return;
        }
        let ctrl = self.mods.control_key() || self.mods.super_key();
        let shift = self.mods.shift_key();
        let Some(f) = self.library.browser.typing.as_mut() else { return };
        match k.logical_key.as_ref() {
            Key::Named(NamedKey::Escape | NamedKey::Enter) => self.library.browser.stop_typing(),
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

    /// a check run's `--library-find`, `--library-list`, `--library-related` and `--library-drag`: the card pressed, carried, let go
    pub(crate) fn script_library(&mut self) {
        const START: u64 = 150;
        if self.frames == 1 {
            if let Some(q) = self.args.library_find.clone() {
                self.library.browser.filter.query = q;
            }
            self.library.browser.chosen = self.args.library_choose.clone();
            if self.args.library_list {
                self.library.browser.mode = deck_kit::editor::library::Mode::List;
            }
            self.library.browser.single |= self.args.library_single;
        }
        if self.frames == 2 && self.args.library_related {
            self.library.browser.filter.related = Some(String::new());
        }
        let Some((address, to, hold)) = self.args.library_drag.clone() else { return };
        if self.frames < START {
            return;
        }
        let middle = || {
            let (w, h) = self.window_size();
            (w * 0.5, h * 0.45)
        };
        let (x, y) = match &to {
            DragTo::Middle => middle(),
            DragTo::At(x, y) => (*x, *y),
            DragTo::Nodes(target) => self.show.node_drop_spot(target).unwrap_or_else(|| {
                if self.frames == START {
                    tracing::error!("--library-drag: no {target} in the node view of this row");
                }
                middle()
            }),
        };
        match self.frames {
            f if f == START => {
                let card = self.library.entries.iter().position(|e| e.address == address).and_then(|i| self.library.browser.hits.iter().find(|(_, h)| *h == deck_kit::editor::library::Hit::Card(i)).map(|(r, _)| *r));
                match card {
                    Some(r) => {
                        self.cursor = (r.x + r.w * 0.5, r.y + r.h * 0.5);
                        self.press();
                    }
                    None => tracing::warn!("--library-drag: no card of {address} in {:?}", self.library.entries.iter().map(|e| &e.address).collect::<Vec<_>>()),
                }
            }
            // a held card follows its target (the node view frames itself again when a preview arrives)
            f if f > START && (hold || f <= START + 10) => {
                let (x0, y0) = self.cursor;
                let k = 1.0 / (START + 11).saturating_sub(f).max(1) as f32;
                self.pointer(x0 + (x - x0) * k, y0 + (y - y0) * k);
            }
            f if f == START + 12 => self.release(),
            _ => {}
        }
        if self.frames == START + 14 {
            let row = self.show.shown().rows.get(self.show.row);
            let cells: Vec<String> = row.map(|r| r.cells.iter().map(|(c, cell)| format!("{c}: {}", deck_show::view::cell_text(cell))).collect()).unwrap_or_default();
            tracing::info!("library-drag {address}: row {} [{}]", row.map_or("-", |r| r.id.as_str()), cells.join(", "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(id: &str) -> crate::module::Entry {
        crate::module::Entry { id: id.into(), factory: true, draft: false, latest: None, content: serde_json::json!({ "descriptor": { "name": id, "ports": [{ "name": "out", "dir": "out", "type": "audio" }] } }) }
    }

    #[test]
    fn each_workspace_loads_its_kinds_into_the_one_browser() {
        let mut lib = Library::new(std::env::temp_dir().join(format!("deck-editor-library-{}", std::process::id())));
        let modules = of_modules(&[module("walk"), module("analog")]);
        let scene = Entry { address: "scene:nave".into(), label: "Nave".into(), detail: String::new(), kind: Kind::Scene, preview: Preview::Kind, made_for: Vec::new() };
        let mut all = modules.clone();
        all.push(scene);
        lib.set(Purpose::Module, all.clone());
        assert_eq!(lib.entries.iter().map(|e| e.address.as_str()).collect::<Vec<_>>(), ["module:walk", "module:analog"], "Module: only modules");
        lib.set(Purpose::Show, all);
        assert_eq!(lib.entries.iter().map(|e| e.address.as_str()).collect::<Vec<_>>(), ["scene:nave"]);
    }

    /// #578's list beside an empty rig is the library's "related" filter on the modules: on once for an empty character
    /// rig, by the descriptor's rule (a persona of a character it names counts)
    #[test]
    fn an_empty_character_rig_shows_the_modules_made_for_it() {
        let made = |id: &str, characters: &[&str]| crate::module::Entry { id: id.into(), factory: false, draft: false, latest: None, content: serde_json::json!({ "descriptor": { "name": id, "characters": characters, "ports": [{ "name": "out", "dir": "out", "type": "audio" }] } }) };
        let mut lib = Library::new(std::env::temp_dir().join(format!("deck-editor-related-{}", std::process::id())));
        lib.set(Purpose::Build, of_modules(&[made("wing", &["yellowmouse"]), made("torso", &["catpope"]), made("drone", &[])]));
        lib.browser.character = Some("yellowmouse.kid".into());
        lib.follow_rig("character.yellowmouse.kid", Some("yellowmouse.kid"), true);
        let shown: Vec<&str> = lib.browser.shown(&lib.entries).into_iter().map(|i| lib.entries[i].address.as_str()).collect();
        assert_eq!(shown, ["module:wing"]);
        lib.browser.filter.related = None;
        lib.follow_rig("character.yellowmouse.kid", Some("yellowmouse.kid"), true);
        assert_eq!(lib.browser.shown(&lib.entries).len(), 3, "the viewer's choice stays for the same rig");
        lib.follow_rig("character.catpope", Some("catpope"), false);
        assert_eq!(lib.browser.filter.related, None, "a rig with tracks: as the viewer set it");
    }

    #[test]
    fn the_assets_answer_is_one_entry_per_family_on_the_cdn() {
        let mut lib = Library::new(std::env::temp_dir().join(format!("deck-editor-families-{}", std::process::id())));
        let text = serde_json::json!({ "cdn": "https://cdn.example", "items": [
            { "id": "a1", "kind": "image", "renditions": [{ "path": "cat/s1/c1/hills.webp" }, { "path": "cat/s1/c1/hills.2x.webp" }, { "path": "cat/s1/c1/hills.3x.webp" }] },
            { "id": "a2", "kind": "audio", "path": "cat/bells.128.mp3" },
        ] })
        .to_string();
        lib.assets.take(answer(&text).unwrap(), &Context::default());
        let list = lib.assets();
        assert_eq!(list.iter().map(|e| e.address.as_str()).collect::<Vec<_>>(), ["cat/bells.128.mp3", "cat/s1/c1/hills.webp"], "three files of hills are one entry");
        assert_eq!(list[1].preview, Preview::Picture { url: "https://cdn.example/cat/s1/c1/hills.webp".into() });
        lib.set(Purpose::Scene, of_scene(&[], None, list));
        assert!(lib.entries.iter().any(|e| e.address == "cat/s1/c1/hills.webp") && !lib.entries.iter().any(|e| e.kind == Kind::Audio), "a scene takes the pictures");
        let files = lib.assets();
        assert_eq!(lib.takes(&files[1]), Ok(()), "a picture fits a scene's layer");
        assert!(lib.takes(&files[0]).unwrap_err().starts_with("audio cannot be dropped on a scene"), "the family's refusal says why");
        lib.browser.single = true;
        let singles = lib.assets();
        assert_eq!(singles.len(), 4, "single levels: every file its own entry");
        let two = singles.iter().find(|e| e.address == "cat/s1/c1/hills.2x.webp").expect("the 2x on its own");
        assert_eq!(lib.family(two).map(|f| f.versions.len()), Some(1), "its family is the file alone");
        assert_eq!(lib.takes(two), Ok(()));
    }

    #[test]
    fn a_dropped_file_lands_in_the_open_items_folder() {
        let mut lib = Library::new(std::env::temp_dir().join(format!("deck-editor-upload-{}", std::process::id())));
        let text = serde_json::json!({ "items": [{ "id": "a1", "kind": "image", "path": "cat/s1/c2/sky.webp" }] }).to_string();
        assert_eq!(lib.upload_target("x.png"), None, "no assets yet: nowhere");
        lib.assets.take(answer(&text).unwrap(), &Context::default());
        assert_eq!(lib.upload_target("Sun Set.PNG"), None, "nothing open and no toon: the person picks a folder");
        lib.set_context(rig_context("yellowmouse.kid", Some("cat")));
        assert_eq!(lib.upload_target("Sun Set.PNG").as_deref(), Some("characters/yellowmouse/sun-set.png"), "a rig: its character's folder");
        lib.set_context(scene_context("nave", &[], Vec::new(), Some("cat")));
        assert_eq!(lib.upload_target("bell.mp3").as_deref(), Some("cat/bell.mp3"), "a scene: its toon's folder");
        assert!(lib.upload(std::path::Path::new("Z:/nowhere/bell.mp3")).unwrap_err().contains("nowhere"), "an unreadable file says so");
        let fresh = serde_json::json!({ "items": [{ "id": "a1", "kind": "image", "path": "cat/s1/c2/sky.webp" }, { "id": "a2", "kind": "image", "renditions": [{ "path": "cat/sun.webp" }, { "path": "cat/sun.2x.webp" }] }] }).to_string();
        lib.fresh = Some("cat/sun.2x.webp".into());
        lib.assets.take(answer(&fresh).unwrap(), &lib.context.clone());
        lib.choose_fresh();
        assert_eq!(lib.browser.chosen, ["cat/sun.webp"], "the new file's card is chosen");
    }

    #[test]
    fn a_scene_hands_its_stage_media_and_a_rig_its_characters_folder_to_the_shelf() {
        let c = scene_context("nave", &[], vec!["cat/s1/c2/sky.webp".into()], Some("cat"));
        assert_eq!((c.open.as_deref(), c.toon.as_deref(), c.uses.len(), c.uses[0].item.as_str()), (Some("scene:nave"), Some("cat"), 1, "scene:nave"));
        assert_eq!(scene_context("nave", &["dog".into()], Vec::new(), Some("cat")).toon.as_deref(), Some("dog"), "the scene's own toon first");
        let r = rig_context("yellowmouse.kid", None);
        assert_eq!((r.open.as_deref(), r.by_home), (Some("character:yellowmouse"), true));
        let mut lib = Library::new(std::env::temp_dir().join(format!("deck-editor-context-{}", std::process::id())));
        let text = serde_json::json!({ "items": [{ "id": "a1", "kind": "image", "path": "cat/s1/c2/sky.webp" }] }).to_string();
        lib.set_context(c.clone());
        let before = lib.stamp;
        lib.set_context(c);
        assert_eq!(lib.stamp, before, "the same context: the view stays");
        lib.assets.take(answer(&text).unwrap(), &lib.context.clone());
        let shelf = lib.assets.shelf.as_ref().unwrap();
        assert_eq!(shelf.view(&deck_catalog::library::shelf::Query::default()).order, ["cat/s1/c2/sky"], "in use by the scene");
    }

    #[test]
    fn a_still_is_taken_once_what_plays_settled_and_kept_in_the_cache() {
        let dir = std::env::temp_dir().join(format!("deck-editor-stills-{}", std::process::id()));
        let mut lib = Library::new(dir.clone());
        assert!(!lib.playing(Some("module:walk".into()), 10));
        assert!(!lib.playing(Some("module:walk".into()), 10 + SETTLE - 1));
        assert!(lib.playing(Some("module:walk".into()), 10 + SETTLE), "settled: take it");
        lib.still(RgbaImage::from_pixel(800, 400, image::Rgba([200, 10, 10, 255])));
        assert!(!lib.playing(Some("module:walk".into()), 10 + SETTLE + 1), "taken once");
        let p = Preview::Still { key: "module:walk".into() };
        assert!(lib.pictures.get(&p).is_some());
        assert_eq!(lib.pictures.images[0].1.dimensions(), STILL);
        let mut again = Pictures::new(dir.join("stills"));
        again.want(&[Entry { address: "module:walk".into(), label: String::new(), detail: String::new(), kind: Kind::Module, preview: p.clone(), made_for: Vec::new() }]);
        // the reading thread's one answer, however long a loaded machine takes (only a hang runs into the minute)
        let (key, img) = again.rx.recv_timeout(std::time::Duration::from_secs(60)).expect("the cache read answers");
        assert!(img.is_some(), "the next start reads it from the cache");
        again.show(key, img.unwrap());
        assert!(again.get(&p).is_some());
        assert!(!lib.playing(Some("character:catpope".into()), 500), "something else plays: it settles first");
        std::fs::remove_dir_all(&dir).ok();
    }
}
