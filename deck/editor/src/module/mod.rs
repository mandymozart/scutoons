//! The Module workspace (research/NATIVE-EDITOR-GUI.md §5–§6 step 6): a `module:<id>` of the catalog opened into a local
//! working copy (copy.rs), its code in the external editor (code.rs), built here by deck/build's own compiler (`local`),
//! the problems listed (problems.rs), the module heard alone on the stage, its Parameters from the descriptor as the kit's
//! scrub numbers, saved as a draft. A factory module is read-only on the server: its save is a derived module of your own
//! (`origin`). The compile service stays the only producer of a published module; publishing is the browser's.
pub mod code;
pub mod copy;
pub mod draw;
pub mod problems;

use crate::show::api::Api;
use crate::stage::Stage;
use deck_catalog::{Curve, Descriptor, ParamDesc};
use deck_kit::editor::controls::Rect;
use deck_space::rig::{assign, Bundle};
use scutoons_build::files::Files;
use scutoons_build::local::{Local, Outcome};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// the track and node the audition rig plays the module in (`assign::audition`)
const NODE: &str = "module_module";
/// pixels of drag from one end of a parameter to the other
const SCRUB_PX: f32 = 240.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    Open(usize),
    Code,
    Build,
    Save,
    Param(usize),
    Problem(usize),
}

/// one module of the catalog
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: String,
    pub factory: bool,
    pub draft: bool,
    pub latest: Option<u64>,
    pub content: Value,
}

/// the module open: its document as read, the version a save names, its working copy
#[derive(Debug, Clone)]
pub struct Open {
    pub id: String,
    pub factory: bool,
    pub content: Value,
    pub version: Option<u64>,
    pub dir: PathBuf,
    /// a factory module's save: the id of the derived module
    pub save_as: String,
}

impl Open {
    pub fn key(&self) -> String {
        format!("module:{}", self.id)
    }
}

/// `GET /api/catalog/docs/modules`'s answer → the list, removed ones left out
pub fn entries(text: &str) -> Result<Vec<Entry>, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("the modules' answer: {e}"))?;
    let list = v["entries"].as_array().ok_or("the modules' answer has no entries")?;
    let mut out: Vec<Entry> = list
        .iter()
        .filter(|e| e["removed"] != Value::Bool(true) && e["content"].is_object())
        .filter_map(|e| {
            let id = e["id"].as_str()?.to_string();
            let factory = e["factory"] == Value::Bool(true) || e["content"]["factory"] == Value::Bool(true);
            Some(Entry { id, factory, draft: e["draft"] == Value::Bool(true), latest: e["latest"].as_u64(), content: e["content"].clone() })
        })
        .collect();
    out.sort_by(|a, b| (!a.factory, &a.id).cmp(&(!b.factory, &b.id)));
    Ok(out)
}

/// a document's sources (`files`)
pub fn files_of(content: &Value) -> Files {
    content["files"].as_object().map(|o| o.iter().filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string()))).collect()).unwrap_or_default()
}

/// what a save writes: a factory module's sources become a derived module of `save_as` (`origin` = the factory one), an
/// own module's document takes the new sources; descriptor and wasm only ever come from the compile service's build
pub fn draft(open: &Open, files: &Files) -> (String, Value) {
    if open.factory {
        let id = open.save_as.clone();
        (format!("module:{id}"), json!({ "id": id, "name": id, "origin": open.id, "files": files }))
    } else {
        let mut c = open.content.clone();
        if let Some(o) = c.as_object_mut() {
            o.remove("descriptor");
            o.remove("wasm");
            o.insert("id".into(), json!(open.id));
            o.insert("files".into(), json!(files));
        }
        (open.key(), c)
    }
}

/// a factory module's derived id: `my_<id>` within the id's 32 characters
/// how a module sets its stage background: in its code, read by the build into the descriptor
pub const BACKGROUND_NOTE: &str = "read-only: set in the code, #[module(background_colour = \"#rrggbb\")] or background_image / background_video; Build reads it";

/// The module's background row, read-only (the descriptor is the build's): what its descriptor sets, how to set it.
pub fn background_row(descriptor: Option<&Descriptor>) -> crate::inspector::BackgroundRow {
    let own = descriptor.and_then(|d| d.background.as_ref());
    let mut row = crate::inspector::background_row(own, "black, unless its rig, scene or show sets one");
    row.note = Some(row.note.map_or_else(|| BACKGROUND_NOTE.to_string(), |n| format!("{n} · {BACKGROUND_NOTE}")));
    row
}

pub fn derived_id(id: &str) -> String {
    format!("my_{id}").chars().take(32).collect()
}

/// where a value sits along a parameter's travel (0..1): log for a log curve, the index for choices
pub fn fraction(p: &ParamDesc, v: f32) -> f32 {
    if !p.choices.is_empty() {
        return v / (p.choices.len().max(2) - 1) as f32;
    }
    if p.max <= p.min {
        return 0.0;
    }
    match p.curve {
        Curve::Log if p.min > 0.0 => (v.max(p.min) / p.min).ln() / (p.max / p.min).ln(),
        _ => (v - p.min) / (p.max - p.min),
    }
    .clamp(0.0, 1.0)
}

/// the value at `f` along a parameter's travel
pub fn value_at(p: &ParamDesc, f: f32) -> f32 {
    let f = f.clamp(0.0, 1.0);
    if !p.choices.is_empty() {
        return (f * (p.choices.len().max(1) - 1) as f32).round();
    }
    match p.curve {
        Curve::Log if p.min > 0.0 => p.min * (p.max / p.min).powf(f),
        _ => p.min + f * (p.max - p.min),
    }
}

/// a parameter's value in words
pub fn shown(p: &ParamDesc, v: f32) -> String {
    if let Some(c) = p.choices.get(v.round().max(0.0) as usize).filter(|_| !p.choices.is_empty()) {
        return c.to_string();
    }
    let n = if v.abs() >= 100.0 { format!("{v:.0}") } else if v.abs() >= 10.0 { format!("{v:.1}") } else { format!("{v:.2}") };
    if p.unit.is_empty() { n } else { format!("{n} {}", p.unit) }
}

pub struct Module {
    api: String,
    root: PathBuf,
    local: Arc<OnceLock<Result<Local, String>>>,
    pub editor: code::Editor,
    pub list: Vec<Entry>,
    pub open: Option<Open>,
    /// the Parameters shown: the last build's descriptor, else the document's
    pub descriptor: Option<Descriptor>,
    /// the built module plays on the stage
    pub heard: bool,
    pub problems: Vec<problems::Row>,
    pub status: String,
    pub bad: bool,
    pub hits: Vec<(Rect, Hit)>,
    /// the workspace's floating panels
    pub panels: crate::panels::Panels,
    drag: Option<(usize, f32, f32)>,
    listing: Option<Receiver<Result<Vec<Entry>, String>>>,
    building: Option<Receiver<(Outcome, Duration)>>,
    saving: Option<Receiver<Saved>>,
    /// a new module's first draft on its way: its id once it is on the server
    creating: Option<Receiver<Result<String, String>>>,
    /// the rig the stage played before the first audition: back on leaving
    restore: Option<Bundle>,
    asked: bool,
    /// a save's creator link, for the clipboard
    pub link: Option<String>,
    /// a check run's wishes: open this module, build once it is open, save once it is heard
    pub script: Script,
}

#[derive(Debug, Clone, Default)]
pub struct Script {
    pub open: Option<String>,
    pub build: bool,
    pub save: bool,
    pub save_as: Option<String>,
}

/// a save's answer: the key, the document written, its new version
type Saved = Result<(String, Value, u64), String>;

/// the deck workspace whose SDK a local build bakes: `DECK_SOURCE`, else this checkout's
fn deck_source() -> PathBuf {
    std::env::var_os("DECK_SOURCE").map(PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).parent().map(Path::to_path_buf).unwrap_or_default())
}

impl Module {
    pub fn new(api: &str, cache: &Path, script: Script) -> Module {
        Module {
            api: api.to_string(),
            root: cache.join("modules"),
            local: Arc::new(OnceLock::new()),
            editor: code::Editor::from_env(),
            list: Vec::new(),
            open: None,
            descriptor: None,
            heard: false,
            problems: Vec::new(),
            status: String::new(),
            bad: false,
            hits: Vec::new(),
            panels: crate::panels::Panels::new(&crate::panels::MODULE),
            drag: None,
            listing: None,
            building: None,
            saving: None,
            creating: None,
            restore: None,
            asked: false,
            link: None,
            script,
        }
    }

    pub fn busy(&self) -> bool {
        self.listing.is_some() || self.building.is_some() || self.saving.is_some() || self.creating.is_some() || (!self.bad && (self.script.build || (self.script.save && self.heard)))
    }

    fn say(&mut self, text: impl Into<String>, bad: bool) {
        self.status = text.into();
        self.bad = bad;
    }

    /// the workspace is shown: the catalog's modules are asked for once
    pub fn enter(&mut self) {
        if self.asked {
            return;
        }
        self.asked = true;
        self.relist();
        self.say("asking the catalog for its modules", false);
    }

    fn relist(&mut self) {
        let (tx, rx) = channel();
        let api = self.api.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Api::new(api).catalog("modules").and_then(|t| entries(&t)));
        });
        self.listing = Some(rx);
    }

    /// a new module (`key`, its first draft `content`) saved as a draft unless it is there already, then opened
    pub fn create(&mut self, key: &str, content: Value) {
        if self.creating.is_some() {
            return;
        }
        let (tx, rx) = channel();
        let (api, key) = (self.api.clone(), key.to_string());
        std::thread::spawn(move || {
            let api = Api::new(api);
            let made = api.draft(&key).and_then(|there| match there {
                Some(_) => Ok(()),
                None => api.save_draft(&key, &content, None, "deck editor").map(|_| ()),
            });
            let _ = tx.send(made.map(|()| key.trim_start_matches("module:").to_string()));
        });
        self.creating = Some(rx);
        self.asked = true;
        self.say("saving the new module's first draft …", false);
    }

    /// the workspace is left: the rig the stage played before comes back
    pub fn leave(&mut self, stage: &mut Stage) {
        if let Some(b) = self.restore.take() {
            if let Err(e) = stage.replay(b) {
                tracing::warn!("the rig does not come back after the audition: {e:#}");
            }
        }
        self.heard = false;
    }

    pub fn hit(&self, x: f32, y: f32) -> Option<Hit> {
        self.hits.iter().find(|(r, _)| r.has(x, y)).map(|(_, h)| *h)
    }

    pub fn press(&mut self, hit: Hit, x: f32, stage: &Stage) {
        match hit {
            Hit::Open(i) => self.open_entry(i),
            Hit::Code => self.code(None),
            Hit::Build => self.build(),
            Hit::Save => self.save(),
            Hit::Problem(i) => self.code(Some(i)),
            Hit::Param(i) => {
                if let Some(p) = self.descriptor.as_ref().and_then(|d| d.params.get(i)) {
                    self.drag = Some((i, x, fraction(p, self.value(i, stage))));
                }
            }
        }
    }

    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub fn drag_to(&mut self, x: f32, stage: &mut Stage) {
        let Some((i, x0, f0)) = self.drag else { return };
        self.turn(i, f0 + (x - x0) / SCRUB_PX, stage);
    }

    pub fn wheel(&mut self, i: usize, steps: f32, stage: &mut Stage) {
        if let Some(p) = self.descriptor.as_ref().and_then(|d| d.params.get(i)) {
            let f = fraction(p, self.value(i, stage));
            self.turn(i, f + steps * 12.0 / SCRUB_PX, stage);
        }
    }

    pub fn release(&mut self) {
        self.drag = None;
    }

    /// the module node on the stage and a parameter's port on it, while the module is heard
    fn port(&self, i: usize, stage: &Stage) -> Option<(usize, usize)> {
        if !self.heard {
            return None;
        }
        let name = &self.descriptor.as_ref()?.params.get(i)?.name;
        let node = stage.node_ids.iter().position(|n| n == NODE)?;
        Some((node, stage.params(node).into_iter().find(|p| p.name == name.as_ref())?.port))
    }

    /// a parameter's value: the stage's while the module is heard, else its default
    pub fn value(&self, i: usize, stage: &Stage) -> f32 {
        let Some(p) = self.descriptor.as_ref().and_then(|d| d.params.get(i)) else { return 0.0 };
        match self.port(i, stage).and_then(|(n, port)| stage.param(n, port)) {
            Some(q) => q.value,
            None => p.default.unwrap_or(p.min),
        }
    }

    fn turn(&mut self, i: usize, f: f32, stage: &mut Stage) {
        let Some(p) = self.descriptor.as_ref().and_then(|d| d.params.get(i)).cloned() else { return };
        if let Some((node, port)) = self.port(i, stage) {
            let (min, max) = if p.choices.is_empty() { (p.min, p.max) } else { (0.0, (p.choices.len().max(1) - 1) as f32) };
            stage.set(node, port, value_at(&p, f), min, max);
        } else {
            self.say("build to hear it: the parameters turn the module on the stage", false);
        }
    }

    fn open_entry(&mut self, i: usize) {
        let Some(e) = self.list.get(i).cloned() else { return };
        let key = format!("module:{}", e.id);
        let dir = copy::folder(&self.root, &key);
        let local = match self.local() {
            Ok(l) => l,
            Err(e) => return self.say(e, true),
        };
        let files = files_of(&e.content);
        if files.is_empty() {
            return self.say(format!("{key} has no sources"), true);
        }
        let how = copy::checkout(&dir, &key, e.latest, &files, &local.manifest(), &scutoons_build::determinism::clippy_toml());
        let save_as = self.script.save_as.clone().unwrap_or_else(|| derived_id(&e.id));
        self.descriptor = e.content.get("descriptor").and_then(|d| serde_json::from_value(d.clone()).ok());
        self.problems.clear();
        self.open = Some(Open { id: e.id.clone(), factory: e.factory, content: e.content, version: e.latest, dir: dir.clone(), save_as });
        match how {
            Ok(copy::Checkout::Fresh) => self.say(format!("{key} in {}", dir.display()), false),
            Ok(copy::Checkout::Kept) => self.say(format!("{key}: the working copy's own changes are kept ({})", dir.display()), false),
            Err(err) => self.say(format!("{key}: the working copy could not be written: {err}"), true),
        }
    }

    /// the local builder, made once (the SDK baked into the cache)
    fn local(&self) -> Result<&Local, String> {
        let root = self.root.join(".build");
        self.local.get_or_init(|| Local::new(&deck_source(), &root).map_err(|e| format!("the local build cannot start: {e}"))).as_ref().map_err(Clone::clone)
    }

    fn code(&mut self, problem: Option<usize>) {
        let Some(open) = &self.open else { return self.say("open a module first", true) };
        let (file, line, column) = match problem.and_then(|i| self.problems.get(i)) {
            Some(p) if !p.file.is_empty() => (p.file.clone(), p.line, p.column),
            _ => (scutoons_build::files::MAIN_FILE.to_string(), 1, 1),
        };
        let path = open.dir.join("src").join(file);
        match self.editor.open(&open.dir, &path, line, column) {
            Ok(()) => self.say(format!("opened {}:{line}", path.display()), false),
            Err(e) => self.say(e, true),
        }
    }

    pub fn build(&mut self) {
        let Some(open) = &self.open else { return self.say("open a module first", true) };
        if self.building.is_some() {
            return;
        }
        let files = match copy::sources(&open.dir) {
            Ok(f) => f,
            Err(e) => return self.say(format!("the working copy does not read: {e}"), true),
        };
        let (tx, rx) = channel();
        let local = self.local.clone();
        let root = self.root.join(".build");
        std::thread::spawn(move || {
            let started = Instant::now();
            let outcome = match local.get_or_init(|| Local::new(&deck_source(), &root).map_err(|e| format!("the local build cannot start: {e}"))) {
                Ok(l) => l.build(&files),
                Err(e) => Err(vec![scutoons_build::diag::Diag::general(e.clone())]),
            };
            let _ = tx.send((outcome, started.elapsed()));
        });
        self.building = Some(rx);
        self.say("building …", false);
    }

    pub fn save(&mut self) {
        let Some(open) = self.open.clone() else { return self.say("open a module first", true) };
        if self.saving.is_some() {
            return;
        }
        let files = match copy::sources(&open.dir) {
            Ok(f) => f,
            Err(e) => return self.say(format!("the working copy does not read: {e}"), true),
        };
        let (key, content) = draft(&open, &files);
        let (tx, rx) = channel();
        let api = self.api.clone();
        std::thread::spawn(move || {
            let api = Api::new(api);
            let base = if open.factory { api.draft(&key).map(|d| d.and_then(|d| d.version)) } else { Ok(open.version) };
            let saved = base.and_then(|b| api.save_draft(&key, &content, b, "deck editor"));
            let _ = tx.send(saved.map(|v| (key, content, v)));
        });
        self.saving = Some(rx);
        self.say("saving the draft …", false);
    }

    /// what the threads answered; a good build is heard at once
    pub fn poll(&mut self, stage: &mut Stage) {
        if let Some(rx) = &self.listing {
            if let Ok(r) = rx.try_recv() {
                self.listing = None;
                match r {
                    Ok(list) => {
                        if self.open.is_none() {
                            self.say(format!("{} modules in the catalog", list.len()), false);
                        }
                        self.list = list;
                        if let Some(id) = self.script.open.take() {
                            match self.list.iter().position(|e| e.id == id) {
                                Some(i) => self.open_entry(i),
                                None => self.say(format!("module:{id} is not in the catalog"), true),
                            }
                        }
                    }
                    Err(e) => self.say(e, true),
                }
            }
        }
        if let Some(rx) = &self.creating {
            if let Ok(r) = rx.try_recv() {
                self.creating = None;
                match r {
                    Ok(id) => {
                        self.script.open = Some(id);
                        self.relist();
                    }
                    Err(e) => self.say(e, true),
                }
            }
        }
        if let Some(rx) = &self.building {
            if let Ok((outcome, took)) = rx.try_recv() {
                self.building = None;
                self.built(outcome, took, stage);
            }
        }
        if let Some(rx) = &self.saving {
            if let Ok(r) = rx.try_recv() {
                self.saving = None;
                self.saved(r);
            }
        }
        if self.script.build && self.open.is_some() && self.building.is_none() {
            self.script.build = false;
            self.build();
        }
        if self.script.save && self.heard && self.saving.is_none() {
            self.script.save = false;
            self.save();
        }
    }

    fn built(&mut self, outcome: Outcome, took: Duration, stage: &mut Stage) {
        match outcome {
            Ok(built) => {
                self.problems = problems::rows(&built.warnings);
                let heard = self.audition(&built.wasm, stage);
                let words = format!("built in {:.1} s · {} · {}", took.as_secs_f32(), problems::summary(&self.problems), &built.hash[..12]);
                match heard {
                    Ok(()) => self.say(format!("{words} · heard on the stage"), false),
                    Err(e) => self.say(format!("{words} · not heard: {e}"), true),
                }
            }
            Err(diags) => {
                self.problems = problems::rows(&diags);
                self.say(format!("the build failed after {:.1} s · {}", took.as_secs_f32(), problems::summary(&self.problems)), true);
            }
        }
    }

    /// the built module alone on the stage, in the subject's being (`assign::audition`)
    fn audition(&mut self, wasm: &[u8], stage: &mut Stage) -> Result<(), String> {
        let loaded = deck_native::module_host::Library::global().load(wasm).map_err(|e| format!("{e:#}"))?;
        let descriptor: Descriptor = loaded.desc.clone();
        let subject = self.restore.as_ref().unwrap_or(&stage.bundle).rig.subject.clone();
        let bundle = assign::audition(&subject, &descriptor).map_err(|e| format!("{e:#}"))?;
        let before = stage.bundle.clone();
        stage.replay(bundle).map_err(|e| format!("{e:#}"))?;
        if self.restore.is_none() {
            self.restore = Some(before);
        }
        self.descriptor = Some(descriptor);
        self.heard = true;
        Ok(())
    }

    fn saved(&mut self, r: Saved) {
        let Some(open) = self.open.as_mut() else { return };
        match r {
            Ok((key, content, version)) => {
                if open.factory {
                    // the derived module is the one open now: its own folder, saved on its own versions
                    let id = open.save_as.clone();
                    let dir = copy::folder(&self.root, &key);
                    if !dir.exists() && std::fs::rename(&open.dir, &dir).is_err() {
                        tracing::warn!("the working copy stays in {}", open.dir.display());
                    } else {
                        open.dir = dir;
                    }
                    (open.id, open.factory) = (id, false);
                }
                open.content = content;
                open.version = Some(version);
                let _ = copy::saved(&open.dir, &key, version);
                let link = crate::rigs::creator_link(&self.api, &key);
                self.say(format!("saved {key} as a draft (version {version}) – publish it in the browser: {link}"), false);
                self.link = Some(link);
                self.relist();
            }
            Err(e) => self.say(e, true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(json: Value) -> ParamDesc {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn the_catalog_lists_factory_modules_first() {
        let text = json!({ "entries": [
            { "id": "mine", "latest": 9, "draft": true, "content": { "id": "mine", "files": { "lib.rs": "" } } },
            { "id": "walk", "latest": 3, "content": { "id": "walk", "factory": true, "files": { "lib.rs": "// walk" } } },
            { "id": "gone", "removed": true, "content": { "id": "gone" } },
        ] })
        .to_string();
        let list = entries(&text).unwrap();
        assert_eq!(list.iter().map(|e| (e.id.as_str(), e.factory, e.draft, e.latest)).collect::<Vec<_>>(), [("walk", true, false, Some(3)), ("mine", false, true, Some(9))]);
        assert_eq!(files_of(&list[0].content)["lib.rs"], "// walk");
        assert!(entries("{}").is_err());
    }

    #[test]
    fn a_factory_module_saves_as_a_derived_one_and_an_own_one_drops_the_old_build() {
        let files = Files::from([("lib.rs".to_string(), "// changed".to_string())]);
        let walk = Open { id: "walk".into(), factory: true, content: json!({ "id": "walk", "factory": true, "files": {}, "descriptor": {}, "wasm": "x" }), version: Some(3), dir: PathBuf::new(), save_as: derived_id("walk") };
        let (key, doc) = draft(&walk, &files);
        assert_eq!(key, "module:my_walk");
        assert_eq!(doc, json!({ "id": "my_walk", "name": "my_walk", "origin": "walk", "files": { "lib.rs": "// changed" } }));
        let mine = Open { id: "my_walk".into(), factory: false, content: json!({ "id": "my_walk", "origin": "walk", "files": {}, "descriptor": {}, "wasm": "h" }), ..walk };
        let (key, doc) = draft(&mine, &files);
        assert_eq!(key, "module:my_walk");
        assert_eq!(doc, json!({ "id": "my_walk", "origin": "walk", "files": { "lib.rs": "// changed" } }));
        assert_eq!(derived_id(&"x".repeat(40)).len(), 32);
    }

    #[test]
    fn the_modules_background_is_the_descriptors_read_only() {
        let none = background_row(None);
        assert!(none.value.starts_with("none · black") && none.note.as_deref() == Some(BACKGROUND_NOTE));
        let mut d = deck_catalog::builtin("osc").unwrap().clone();
        d.background = Some(deck_space::Background::colour("#ff8000"));
        let row = background_row(Some(&d));
        assert_eq!((row.value.as_str(), row.swatch), ("#ff8000", Some([1.0, 128.0 / 255.0, 0.0])));
        d.background = deck_space::Background::typed("loops/sky.mp4").unwrap();
        let note = background_row(Some(&d)).note.unwrap();
        assert!(note.starts_with(crate::inspector::VIDEO_NOTE) && note.ends_with(BACKGROUND_NOTE), "{note}");
    }

    #[test]
    fn parameters_travel_on_their_curve() {
        let lin = param(json!({ "name": "glide", "min": 0.0, "max": 1.0, "default": 0.5 }));
        assert_eq!((fraction(&lin, 0.25), value_at(&lin, 0.75)), (0.25, 0.75));
        let log = param(json!({ "name": "base", "min": 20.0, "max": 2000.0, "curve": "log", "unit": "Hz" }));
        assert!((fraction(&log, 200.0) - 0.5).abs() < 1e-4);
        assert!((value_at(&log, 0.5) - 200.0).abs() < 1e-2);
        assert_eq!(shown(&log, 147.0), "147 Hz");
        let wave = param(json!({ "name": "wave", "choices": ["saw", "pulse", "sine"], "default": 0 }));
        assert_eq!((fraction(&wave, 1.0), value_at(&wave, 0.9), shown(&wave, 2.0)), (0.5, 2.0, "sine".to_string()));
    }
}
