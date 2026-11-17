//! The library panel (deck-kit `library`, the core's entries from deck-catalog / deck-space `library`) in Build, Show and
//! Module: what each workspace loads into the one browser, the previews' pictures – a still the stage drew of what it
//! played, kept in the cache by key, and an asset's picture fetched once – and the card's way out: dropped where the
//! workspace takes it (Build's stage: a track; Show's cell: its piece), opened in its editor view on a second press.
use crate::show::api::Api;
use deck_catalog::library::{self, Entry, Kind, Preview, Purpose};
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::library::{Browser, Did};
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

    /// whether the cache has a picture of `key`
    pub fn has(&self, key: &str) -> bool {
        self.loaded.contains_key(key)
    }

    /// The previews of `entries` asked for once each: a still from the cache, a picture from the cache or the net (on a
    /// thread).
    pub fn want(&mut self, entries: &[Entry]) {
        for e in entries {
            let Some(key) = key_of(&e.preview) else { continue };
            if !self.asked.insert(key.to_string()) {
                continue;
            }
            let (file, tx, key) = (self.file(key), self.tx.clone(), key.to_string());
            let url = matches!(e.preview, Preview::Picture { .. }).then(|| key.clone());
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

/// the toon's assets, asked once signed in
#[derive(Default)]
struct Assets {
    list: Vec<Entry>,
    rx: Option<Receiver<Result<Vec<Entry>, String>>>,
    asked: bool,
}

/// the library panel's state, one for the editor
pub struct Library {
    pub browser: Browser,
    /// this frame's entries, the workspace's
    pub entries: Vec<Entry>,
    pub purpose: Purpose,
    pub pictures: Pictures,
    assets: Assets,
    /// the sentence where a drop was refused
    pub refused: Option<(f32, f32, String)>,
    /// what plays on the stage and since which frame, until its still is taken
    playing: Option<(String, u64)>,
    /// the rig the "related" filter last turned itself on for (an empty character rig: its modules first)
    related_for: Option<String>,
}

impl Library {
    pub fn new(cache: PathBuf) -> Library {
        Library { browser: Browser::default(), entries: Vec::new(), purpose: Purpose::Build, pictures: Pictures::new(cache.join("stills")), assets: Assets::default(), refused: None, playing: None, related_for: None }
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
        let (tx, rx) = channel();
        let api = Api::new(base.to_string());
        std::thread::spawn(move || {
            let _ = tx.send(api.assets().and_then(|t| library::assets(&t)));
        });
        self.assets.rx = Some(rx);
    }

    /// A check run's catalog folder (`--catalog`): its assets (`assets.json`, `/assets`) and the Module workspace's list
    /// of its modules (`modules.json`).
    pub fn answers(&mut self, dir: &std::path::Path) -> Option<Vec<crate::module::Entry>> {
        if let Ok(text) = std::fs::read_to_string(dir.join("assets.json")) {
            self.assets.asked = true;
            match library::assets(&text) {
                Ok(list) => self.assets.list = list,
                Err(e) => tracing::warn!("--catalog assets: {e}"),
            }
        }
        let text = std::fs::read_to_string(dir.join("modules.json")).ok()?;
        crate::module::entries(&text).map_err(|e| tracing::warn!("--catalog modules: {e}")).ok()
    }

    pub fn assets(&self) -> Vec<Entry> {
        self.assets.list.clone()
    }

    pub fn poll(&mut self) {
        self.pictures.poll();
        if let Some(Ok(r)) = self.assets.rx.as_ref().map(Receiver::try_recv) {
            match r {
                Ok(list) => self.assets.list = list,
                Err(e) => tracing::info!("the library's assets: {e}"),
            }
            self.assets.rx = None;
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
        self.browser.draw(d, t, (a, pointer), &self.entries, &|p| pictures.get(p), action);
    }

    /// over everything: the sentence where a card was refused (the card in the hand is the editor's one `Carry`)
    pub fn over(&self, d: &mut DrawList, t: &Tokens, max_x: f32) {
        if let Some((x, y, why)) = &self.refused {
            deck_kit::editor::carry::refused(d, t, (*x, *y), max_x, why);
        }
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

    /// the entry dragged now, if this workspace takes its kind
    pub fn carrying(&self) -> Option<&Entry> {
        self.browser.dragging().and_then(|i| self.entries.get(i)).filter(|e| self.purpose.takes(e.kind).is_ok())
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

/// a check run's drag: the card's address, where it is let go (none: the stage's middle), kept in the hand
pub type ScriptDrag = (String, Option<(f32, f32)>, bool);

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

/// The library on the editor: its entries each frame, a card's way out, the find field's keys, a check run's drag.
impl crate::App {
    pub(crate) fn follow_library(&mut self) {
        use crate::show::Workspace;
        self.library.poll();
        let purpose = match self.workspace {
            Workspace::Build => Purpose::Build,
            Workspace::Module => Purpose::Module,
            Workspace::Show => Purpose::Show,
            Workspace::Perform => {
                self.show.carrying = false;
                return;
            }
        };
        if purpose == Purpose::Build && self.auth.token().is_some() {
            self.library.ask_assets(&self.args.api);
        }
        let entries = match purpose {
            Purpose::Build => of_build(self.rigs.module_docs(), &crate::adding::instruments(&self.stage.bundle, self.rigs.instruments()), self.book.book.as_deref(), self.library.assets()),
            Purpose::Show => self.show.library_entries(),
            Purpose::Module => of_modules(&self.module.list),
        };
        self.library.set(purpose, entries);
        let rig = (self.scene.is_none()).then(|| self.stage.bundle.rig.subject.clone());
        let character = rig.as_deref().and_then(crate::toon::character).map(String::from);
        self.library.browser.character = if purpose == Purpose::Build { character.clone() } else { None };
        if let (Purpose::Build, Some(subject)) = (purpose, rig.as_deref()) {
            self.library.follow_rig(subject, character.as_deref(), crate::empty::is_empty(&self.stage.bundle));
        }
        self.show.carrying = purpose == Purpose::Show && self.library.carrying().is_some();
        if self.workspace == Workspace::Module {
            if let Some(j) = self.library_open.as_ref().and_then(|id| self.module.list.iter().position(|m| m.id == *id)) {
                self.library_open = None;
                let x = self.cursor.0;
                self.module.press(crate::module::Hit::Open(j), x, &self.stage);
            }
        }
        let key = match self.workspace {
            Workspace::Build => playing_key(rig.as_deref(), None),
            Workspace::Module => playing_key(None, self.module.open.as_ref().filter(|_| self.module.heard).map(|o| o.id.as_str())),
            _ => None,
        };
        if self.library.playing(key, self.frames) {
            self.stage.ask_still();
        }
    }

    fn library_go(&mut self, w: crate::show::Workspace) {
        if self.workspace == crate::show::Workspace::Module && w != crate::show::Workspace::Module {
            self.module.leave(&mut self.stage);
        }
        self.workspace = w;
    }

    pub(crate) fn library_did(&mut self, did: Did) {
        match did {
            Did::Taken => {}
            Did::Action => self.show.press(crate::show::Hit::Compose(crate::show::new_scene::ComposeHit::Open)),
            Did::Click(i) if self.workspace == crate::show::Workspace::Module => self.library_open_entry(i),
            Did::Click(_) => {}
            Did::Open(i) => self.library_open_entry(i),
            Did::Drop { entry, at } => {
                self.carry = None;
                self.library_drop(entry, at)
            }
        }
    }

    /// the pointer moved: a card dragged far enough goes into the one hand
    pub(crate) fn library_drag_to(&mut self, x: f32, y: f32) {
        self.library.browser.drag_to(x, y);
        if self.carry.is_none() {
            if let Some(e) = self.library.browser.dragging().and_then(|i| self.library.entries.get(i)) {
                self.carry = Some(deck_kit::editor::carry::Carry::new(&e.address, &e.label));
            }
        }
    }

    /// an entry opened in its editor view: a module in Module, a scene or a character's rig in Build
    fn library_open_entry(&mut self, i: usize) {
        let Some(e) = self.library.entries.get(i).cloned() else { return };
        let (x, y) = self.cursor;
        match e.kind {
            Kind::Module => {
                self.library_open = e.address.strip_prefix("module:").map(String::from);
                self.library_go(crate::show::Workspace::Module);
            }
            Kind::Scene => self.show.open_scene(&e.address),
            Kind::Character => {
                self.leave_view();
                self.library_go(crate::show::Workspace::Build);
                if let Err(err) = self.rigs.open(&e.address.replacen(':', ".", 1)) {
                    self.say(err, x, y, true);
                }
            }
            _ => self.say(format!("{} has no editor view here", e.label), x, y, false),
        }
    }

    /// a card let go at `at`: Build's stage takes a module or an instrument as a track, Show's cell a piece
    fn library_drop(&mut self, entry: usize, at: (f32, f32)) {
        let Some(e) = self.library.entries.get(entry).cloned() else { return };
        if self.library.browser.hit(at.0, at.1).is_some() {
            return;
        }
        if let Err(why) = self.library.purpose.takes(e.kind) {
            self.library.refused = Some((at.0, at.1, why));
            return;
        }
        match self.workspace {
            crate::show::Workspace::Show => {
                self.show.drop_piece(&e.address, at.0, at.1);
            }
            crate::show::Workspace::Build if !self.panels().floats.covers(at.0, at.1) => {
                let choice = match e.kind {
                    Kind::Module => self.rigs.modules().0.into_iter().find(|(_, d)| format!("module:{}", d.name) == e.address).map(|m| crate::adding::Choice::Module(Box::new(m))),
                    _ => crate::adding::instruments(&self.stage.bundle, self.rigs.instruments()).into_iter().find(|i| format!("instrument:{}", i.id) == e.address).map(|i| crate::adding::Choice::Track(Box::new(i))),
                };
                match choice {
                    Some(c) => self.added(c, at.0, at.1),
                    None => self.say(format!("{} is not built yet: it has no track to play", e.address), at.0, at.1, true),
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

    /// a check run's `--library-find`, `--library-list` and `--library-drag`: the card pressed, carried, let go
    pub(crate) fn script_library(&mut self) {
        const START: u64 = 150;
        if self.frames == 1 {
            if let Some(q) = self.args.library_find.clone() {
                self.library.browser.filter.query = q;
            }
            if self.args.library_list {
                self.library.browser.mode = deck_kit::editor::library::Mode::List;
            }
        }
        let Some((address, at, hold)) = self.args.library_drag.clone() else { return };
        let (x, y) = at.unwrap_or_else(|| {
            let (w, h) = self.window_size();
            (w * 0.5, h * 0.45)
        });
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
            f if f > START && f <= START + 10 => {
                let (x0, y0) = self.cursor;
                let k = 1.0 / (START + 11 - f) as f32;
                self.pointer(x0 + (x - x0) * k, y0 + (y - y0) * k);
            }
            f if f == START + 12 && !hold => self.release(),
            _ => {}
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
        for _ in 0..200 {
            again.poll();
            if again.get(&p).is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(again.get(&p).is_some(), "the next start reads it from the cache");
        assert!(!lib.playing(Some("character:catpope".into()), 500), "something else plays: it settles first");
        std::fs::remove_dir_all(&dir).ok();
    }
}
