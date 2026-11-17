//! Build's rigs on the server (research/NATIVE-EDITOR-GUI.md §5 "How the two sides share data"): the open toon's rigs
//! from the catalog (`GET /api/catalog/docs/rigs`, and the instruments, presets and modules a rig names), one opened as
//! the core makes a bundle of it (`rig::assign::catalog_bundle`), saving itself (autosave.rs) as a new draft version on
//! the version it was opened at (`PUT /api/entities/rig:<subject>`, a 409 said in words). Publishing stays in the
//! browser (the creator). What the server has not got is the rig's outbox entry (autosave.rs), its difference written at
//! every edit: sent when the rig opens on the version it was made on, offered when the server's differs. Every ask runs
//! on a thread; the UI polls.
use crate::autosave::{self, Autosave, Outbox};
use crate::show::api::{Api, Refusal};
use crate::{toon, wires};
use deck_kit::editor::typeahead::Item;
use deck_space::rig::{assign, Bundle, Rig};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

/// what a save names itself in the history
const MESSAGE: &str = "deck editor";

/// one rig of the catalog's list
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub subject: String,
    /// its newest version; None: in the catalog without a version of its own
    pub latest: Option<u64>,
    /// the newest version is not published yet
    pub draft: bool,
    /// the toons it belongs to (`*`: none named)
    pub toons: Vec<String>,
    pub content: Value,
}

/// the catalog as a rig needs it: the rigs, the documents they name, each document's newest version
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub rigs: Vec<Entry>,
    pub instruments: Vec<Value>,
    pub presets: Vec<Value>,
    pub modules: Vec<Value>,
    /// `instrument:<id>`, `preset:<id>` → its newest version: the base a save of a changed one names
    pub versions: BTreeMap<String, u64>,
}

fn entries(text: &str) -> Result<Vec<Value>, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("the catalog's answer: {e}"))?;
    let list = v["entries"].as_array().ok_or("the catalog's answer has no entries")?;
    Ok(list.iter().filter(|e| e["removed"] != Value::Bool(true) && e["content"].is_object()).cloned().collect())
}

impl Catalog {
    /// the four answers of `/catalog/docs/<kind>` (rigs, instruments, presets, modules)
    pub fn parse(rigs: &str, instruments: &str, presets: &str, modules: &str) -> Result<Catalog, String> {
        let mut c = Catalog::default();
        for e in entries(rigs)? {
            let Some(subject) = e["id"].as_str() else { continue };
            let toons = e["toons"].as_array().map(|t| t.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
            c.rigs.push(Entry { subject: subject.to_string(), latest: e["latest"].as_u64(), draft: e["draft"] == Value::Bool(true), toons, content: e["content"].clone() });
        }
        for (kind, text, into) in [("instrument", instruments, &mut c.instruments), ("preset", presets, &mut c.presets), ("module", modules, &mut c.modules)] {
            for e in entries(text)? {
                if let (Some(id), Some(v)) = (e["id"].as_str(), e["latest"].as_u64()) {
                    c.versions.insert(format!("{kind}:{id}"), v);
                }
                into.push(e["content"].clone());
            }
        }
        c.rigs.sort_by(|a, b| a.subject.cmp(&b.subject));
        Ok(c)
    }

    pub fn fetch(api: &Api) -> Result<Catalog, String> {
        Catalog::parse(&api.catalog("rigs")?, &api.catalog("instruments")?, &api.catalog("presets")?, &api.catalog("modules")?)
    }

    pub fn rig(&self, subject: &str) -> Option<&Entry> {
        self.rigs.iter().find(|e| e.subject == subject)
    }

    /// the rig's document with what it names from the catalog, as the core bundles it
    pub fn bundle(&self, content: Value) -> Result<Bundle, String> {
        let rig: Rig = serde_json::from_value(content).map_err(|e| format!("the rig does not read: {e}"))?;
        assign::catalog_bundle(rig, &self.instruments, &self.presets, &self.modules).map_err(|e| format!("{e:#}"))
    }
}

/// is `e` one of `toon`'s rigs: its toon names it, or – naming none – its character's own toon is it (an item's rig is
/// every toon's); no toon open: every rig
pub fn of_toon(e: &Entry, toon: Option<&str>, registry: &str) -> bool {
    let Some(t) = toon else { return true };
    if e.toons.iter().any(|x| x == t) {
        return true;
    }
    e.toons.iter().all(|x| x == "*") && toon::character(&e.subject).is_none_or(|c| toon::own_toon(registry, c).as_deref() == Some(t))
}

/// how the offline copy stands to the server: "older" / "newer"; None: made on the server's version
pub fn compare(made_on: Option<u64>, server: Option<u64>) -> Option<&'static str> {
    match (made_on, server) {
        (a, b) if a == b => None,
        (Some(a), Some(b)) => Some(if a < b { "older" } else { "newer" }),
        (None, _) => Some("older"),
        (Some(_), None) => Some("newer"),
    }
}

fn version(v: Option<u64>) -> String {
    v.map_or_else(|| "no version".into(), |v| format!("v{v}"))
}

/// the offline copy of a rig, when it differs from the server's
#[derive(Debug, Clone, PartialEq)]
pub struct Local {
    pub subject: String,
    pub made_on: Option<u64>,
    pub server: Option<u64>,
}

impl Local {
    pub fn words(&self) -> String {
        let how = compare(self.made_on, self.server).unwrap_or("the same as");
        let than = if how == "the same as" { "" } else { " than" };
        format!("the local copy of {} (made on {}) is {how}{than} the server's {}", self.subject, version(self.made_on), version(self.server))
    }
}

/// what the rig picker may open
#[derive(Debug, Clone, PartialEq)]
pub enum Choice {
    Server(String),
    Local,
}

/// a draft saved as `version`, and where it is published
#[cfg(test)]
pub fn saved_words(version: u64, link: &str) -> String {
    format!("saved as draft v{version} – publish it in the creator: {link}")
}

/// the creator's page of `key`: `DECK_CREATOR`, else the API's host as the creator's (`api.` → `creator.`, the dev
/// API's port → the dev creator's, shared/dev-ports.json)
pub fn creator_link(api: &str, key: &str) -> String {
    let base = std::env::var("DECK_CREATOR").ok().filter(|c| !c.trim().is_empty()).unwrap_or_else(|| {
        let root = api.trim_end_matches('/').trim_end_matches("/api");
        root.replacen("//api.", "//creator.", 1).replacen(":9000", ":10000", 1)
    });
    format!("{}/edit/{key}", base.trim_end_matches('/'))
}

/// where Build's rig keeps what the server has not got: its outbox entry, the file `--draft` names, or nowhere (a check
/// run, a scene's member opened as its rig)
#[derive(Debug, Clone, PartialEq)]
pub enum Copy {
    Outbox,
    File(PathBuf),
    Nothing,
}

const KIND: &str = "rig";

/// the draft files of before the outbox (`<cache>/drafts/<subject>.rig.toml`), each read once into it and removed
pub fn migrate(cache: &Path, outbox: &Outbox) {
    let Ok(files) = std::fs::read_dir(cache.join("drafts")) else { return };
    for f in files.flatten().map(|f| f.path()) {
        let Some(subject) = f.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".rig.toml")) else { continue };
        let Some((version, body)) = std::fs::read_to_string(&f).ok().and_then(|t| wires::draft_text(&t)) else { continue };
        if outbox.get(KIND, subject).is_none() {
            outbox.put(&autosave::Entry::new(KIND, subject, version, body));
        }
        if outbox.get(KIND, subject).is_some() {
            let _ = std::fs::remove_file(&f);
        }
    }
}

/// what plays at the start: the outbox's copy of `subject` over `base`, and the version it was made on
pub fn kept(outbox: &Outbox, subject: &str, base: &Bundle) -> (Bundle, Option<Option<u64>>) {
    match outbox.get(KIND, subject) {
        Some(e) => (wires::over(&e.body, base), Some(e.base)),
        None => (base.clone(), None),
    }
}

fn base_path(cache: &Path, subject: &str) -> PathBuf {
    cache.join("drafts").join(format!("{subject}.base.json"))
}

/// what the offline copy is the difference to: the server's bundle as last opened, else the deck's core rig
pub fn base(cache: &Path, subject: &str) -> anyhow::Result<(Bundle, Option<u64>)> {
    let kept = std::fs::read_to_string(base_path(cache, subject)).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok());
    if let Some(b) = kept.as_ref().and_then(|k| serde_json::from_value::<Bundle>(k["bundle"].clone()).ok()) {
        return Ok((b, kept.and_then(|k| k["version"].as_u64())));
    }
    match Bundle::core(subject) {
        Some(b) => Ok((b?, None)),
        None => Ok((Bundle::empty(subject), None)),
    }
}

/// the characters of `toon` in the library's registry (every one with no toon open) that have no rig in the catalog
pub fn without_rig(c: &Catalog, registry: &str, toon: Option<&str>) -> Vec<String> {
    let v: Value = serde_json::from_str(registry).unwrap_or_default();
    let list = v["characters"].as_array().or_else(|| v["content"]["characters"].as_array()).cloned().unwrap_or_default();
    let mut out: Vec<String> = list
        .iter()
        .filter(|ch| toon.is_none_or(|t| ch["own_toon"].as_str() == Some(t)))
        .filter_map(|ch| ch["id"].as_str())
        .map(|id| format!("character.{id}"))
        .filter(|s| c.rig(s).is_none())
        .collect();
    out.sort();
    out
}

/// what opens for `subject`: its draft, else the catalog's rig, else – a character without one – its empty rig
fn read(catalog: &Catalog, subject: &str, draft: Option<crate::show::api::Draft>) -> Read {
    let (content, at) = match draft {
        Some(d) => (d.content, d.version),
        None => match catalog.rig(subject) {
            Some(e) => (e.content.clone(), e.latest),
            None if toon::character(subject).is_some() => return Ok((subject.to_string(), Bundle::empty(subject), None)),
            None => return Err(format!("rig:{subject}: not on the server")),
        },
    };
    Ok((subject.to_string(), catalog.bundle(content)?, at))
}

fn keep_base(cache: &Path, subject: &str, bundle: &Bundle, version: Option<u64>) {
    let text = serde_json::json!({ "version": version, "bundle": bundle }).to_string();
    let path = base_path(cache, subject);
    let _ = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&path, text));
}

/// A rig opened from the server and its outbox copy beside it: what plays (the copy when it was made on this very
/// version: its unsaved changes are back), and the copy to offer when it was made on another
pub fn with_copy(outbox: &Outbox, subject: &str, server: &Bundle, at: Option<u64>) -> (Bundle, Option<Local>) {
    let Some(e) = outbox.get(KIND, subject) else { return (server.clone(), None) };
    let (made_on, copy) = (e.base, wires::over(&e.body, server));
    if made_on == at {
        return (copy, None);
    }
    let same = copy.rig == server.rig && copy.instruments == server.instruments && copy.presets == server.presets;
    (server.clone(), (!same).then(|| Local { subject: subject.to_string(), made_on, server: at }))
}

/// the documents of `now` that differ from `base` beside the rig (a value set, a cable inside a track), each with its
/// key and the version it is saved on
pub fn changed(base: &Bundle, now: &Bundle, versions: &BTreeMap<String, u64>) -> Vec<(String, Value, Option<u64>)> {
    let mut out = Vec::new();
    for i in now.instruments.iter().filter(|i| !base.instruments.contains(i)) {
        let key = format!("instrument:{}", i.id);
        out.push((key.clone(), serde_json::to_value(i).unwrap_or_default(), versions.get(&key).copied()));
    }
    for p in now.presets.iter().filter(|p| !base.presets.contains(p)) {
        let key = format!("preset:{}", p.id);
        out.push((key.clone(), serde_json::to_value(p).unwrap_or_default(), versions.get(&key).copied()));
    }
    out
}

/// the changed documents first, then the rig on `base`: the first refusal stops it; the versions the documents got
fn save(api: &Api, key: &str, rig: &Value, base: Option<u64>, others: &[(String, Value, Option<u64>)]) -> (Result<u64, Refusal>, Vec<(String, u64)>) {
    let mut got = Vec::new();
    for (k, content, on) in others {
        match api.save_draft(k, content, *on, MESSAGE) {
            Ok(v) => got.push((k.clone(), v)),
            Err(e) => return (Err(e), got),
        }
    }
    (api.save_draft(key, rig, base, MESSAGE), got)
}

enum Listing {
    Asking(Receiver<Result<Catalog, String>>),
    Ready(Catalog),
    Failed(String),
}

/// a rig read from the server: its subject, its bundle, its version
type Read = Result<(String, Bundle, Option<u64>), String>;

/// what happened since the last poll, for the stage and the notice
pub enum Event {
    /// a rig opened: its subject, the bundle it is the difference to, what plays
    Opened(String, Box<(Bundle, Bundle)>),
    /// saved: the bundle sent is the server's now
    Saved(Box<Bundle>),
    Say(String, bool),
}

/// Build's rig on the server: the list, the rig open and its version, the save
pub struct Rigs {
    api: String,
    cache: PathBuf,
    pub subject: String,
    /// when it saves itself; its version is the server's open, the base of the next save (None: never saved there).
    /// A save carries back the versions the changed documents got and the bundle sent.
    pub auto: Autosave<(Vec<(String, u64)>, Bundle)>,
    listing: Listing,
    /// signed in when the list was asked: asked again when that changes
    asked_signed: bool,
    told: bool,
    opening: Option<Receiver<Read>>,
    /// the offline copy offered
    pub local: Option<Local>,
    /// what the picker shows now, by its ids
    pub choices: Vec<Choice>,
    pub outbox: Outbox,
}

impl Rigs {
    pub fn new(api: &str, cache: &Path, subject: &str, version: Option<u64>, signed: bool) -> Rigs {
        let mut r = Rigs { api: api.to_string(), outbox: Outbox::current(), ..Rigs::offline(cache, subject, version, "") };
        (r.told, r.listing) = (false, Listing::Failed(String::new()));
        r.list(signed);
        r
    }

    /// no asking at all (a check run): nothing saves itself
    pub fn offline(cache: &Path, subject: &str, version: Option<u64>, why: &str) -> Rigs {
        Rigs { api: String::new(), cache: cache.to_path_buf(), subject: subject.to_string(), auto: Autosave::new(version), listing: Listing::Failed(why.to_string()), asked_signed: false, told: true, opening: None, local: None, choices: Vec::new(), outbox: Outbox::none() }
    }

    /// A check run's catalog from `dir`: `rigs.json`, `instruments.json`, `presets.json`, `modules.json`, each the API's
    /// answer to `/catalog/docs/<kind>` (a missing one = none of that kind).
    pub fn read_catalog(&mut self, dir: &Path) -> Result<(), String> {
        let read = |kind: &str| std::fs::read_to_string(dir.join(format!("{kind}.json"))).unwrap_or_else(|_| r#"{ "entries": [] }"#.to_string());
        self.listing = Listing::Ready(Catalog::parse(&read("rigs"), &read("instruments"), &read("presets"), &read("modules"))?);
        Ok(())
    }

    /// a check run's rigs ask nothing, so they save nothing
    pub fn saves(&self) -> bool {
        !self.api.is_empty()
    }

    fn list(&mut self, signed: bool) {
        let (tx, rx) = channel();
        let api = self.api.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Catalog::fetch(&Api::new(api)));
        });
        (self.listing, self.asked_signed) = (Listing::Asking(rx), signed);
    }

    /// the rig as it plays, written as its outbox entry on the version open
    pub fn keep(&self, base: &Bundle, now: &Bundle) {
        self.outbox.put(&autosave::Entry::new(KIND, &self.subject, self.auto.version, wires::difference(base, now)));
    }

    /// `sent` is the server's at the version open now: kept as the base of what opens next; the entry goes unless
    /// something changed since
    pub fn saved(&self, sent: &Bundle, now: &Bundle) {
        keep_base(&self.cache, &self.subject, sent, self.auto.version);
        if now == sent {
            self.outbox.remove(KIND, &self.subject);
        } else {
            self.keep(sent, now);
        }
    }

    /// the catalog once it is here (Show's new scene picks its cast from its rigs)
    pub fn catalog(&self) -> Option<&Catalog> {
        match &self.listing {
            Listing::Ready(c) => Some(c),
            _ => None,
        }
    }

    /// the threads' answers; the list asked again when the sign-in changed
    pub fn poll(&mut self, signed: bool) -> Vec<Event> {
        let mut out = Vec::new();
        if !self.api.is_empty() && signed != self.asked_signed && !matches!(self.listing, Listing::Asking(_)) {
            self.list(signed);
        }
        if let Listing::Asking(rx) = &self.listing {
            if let Ok(answer) = rx.try_recv() {
                self.listing = match answer {
                    Ok(c) => Listing::Ready(c),
                    Err(why) => Listing::Failed(why),
                };
            }
        }
        if let (false, Listing::Ready(c)) = (self.told, &self.listing) {
            self.told = true;
            if let Some(e) = c.rig(&self.subject).filter(|e| compare(self.auto.version, e.latest).is_some()) {
                let local = Local { subject: self.subject.clone(), made_on: self.auto.version, server: e.latest };
                out.push(Event::Say(format!("{} – the rig picker opens the server's", local.words()), false));
            }
        }
        if let Some(Ok(answer)) = self.opening.as_ref().map(Receiver::try_recv) {
            self.opening = None;
            match answer {
                Ok((subject, server, at)) => {
                    keep_base(&self.cache, &subject, &server, at);
                    let (plays, local) = with_copy(&self.outbox, &subject, &server, at);
                    let back = local.is_none() && plays != server;
                    let empty = at.is_none() && plays.rig.tracks.is_empty();
                    (self.subject, self.local) = (subject.clone(), local.clone());
                    self.auto.open(at);
                    if local.is_none() && back {
                        self.auto.edit(std::time::Instant::now());
                    }
                    out.push(Event::Opened(subject.clone(), Box::new((server, plays))));
                    let words = match (local, back) {
                        (None, false) if empty =>format!("{subject} has no rig yet – start from modules or write a new module"),
                        (Some(l), _) => format!("{} – the rig picker offers it", l.words()),
                        (None, true) => format!("{subject} {}: the local copy's unsaved changes are back", version(at)),
                        (None, false) => format!("{subject} {} from the server", version(at)),
                    };
                    out.push(Event::Say(words, false));
                }
                Err(why) => out.push(Event::Say(why, true)),
            }
        }
        if let Some(((versions, sent), answer)) = self.auto.poll() {
            if let Listing::Ready(c) = &mut self.listing {
                c.versions.extend(versions);
            }
            match answer {
                Ok(_) => out.push(Event::Saved(Box::new(sent))),
                // the pill says it goes again; a refusal or a conflict is said once
                Err(Refusal::Offline(_)) => {}
                Err(e) => out.push(Event::Say(e.to_string(), true)),
            }
        }
        out
    }

    /// the picker's rows for `toon` (`registry`: the library's characters) and what it says inside: why there is no
    /// list (`sign_in`: the sign-in's line, "" when signed in), or that it is still asked
    pub fn items(&mut self, toon: Option<&str>, registry: &str, sign_in: &str) -> (Vec<Item>, Option<String>) {
        self.choices.clear();
        let mut items = Vec::new();
        if let Some(l) = &self.local {
            self.choices.push(Choice::Local);
            items.push(Item { id: 0, label: format!("local copy of {}", l.subject), detail: compare(l.made_on, l.server).map(|h| format!("{h} than the server's")).unwrap_or_default() });
        }
        let note = match &self.listing {
            Listing::Asking(_) => Some("asking the server for the rigs …".to_string()),
            Listing::Failed(why) if !sign_in.is_empty() => Some(format!("no rigs from the server: {sign_in}{}", if why.is_empty() { String::new() } else { format!(" ({why})") })),
            Listing::Failed(why) => Some(format!("no rigs from the server: {why}")),
            Listing::Ready(c) => {
                for e in c.rigs.iter().filter(|e| of_toon(e, toon, registry)) {
                    let open = if e.subject == self.subject { " · open" } else { "" };
                    let state = if e.latest.is_none() { "in the catalog".to_string() } else { format!("{}{}", version(e.latest), if e.draft { " · draft" } else { "" }) };
                    items.push(Item { id: self.choices.len(), label: e.subject.clone(), detail: format!("{state}{open}") });
                    self.choices.push(Choice::Server(e.subject.clone()));
                }
                for subject in without_rig(c, registry, toon) {
                    let open = if subject == self.subject { " · open" } else { "" };
                    items.push(Item { id: self.choices.len(), label: subject.clone(), detail: format!("no rig yet{open}") });
                    self.choices.push(Choice::Server(subject));
                }
                (items.is_empty()).then(|| format!("no rigs for {}", toon.unwrap_or("this toon")))
            }
        };
        (items, note)
    }

    /// open `subject` from the server, on a thread (its answer comes through `poll`)
    pub fn open(&mut self, subject: &str) -> Result<(), String> {
        let Listing::Ready(c) = &self.listing else { return Err("no rigs from the server yet".into()) };
        let (catalog, api, subject) = (c.clone(), self.api.clone(), subject.to_string());
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(Api::new(api).draft(&format!("rig:{subject}")).and_then(|d| read(&catalog, &subject, d)));
        });
        self.opening = Some(rx);
        Ok(())
    }

    /// the catalog's instruments that read (none until the list came)
    pub fn instruments(&self) -> Vec<deck_space::rig::Instrument> {
        match &self.listing {
            Listing::Ready(c) => c.instruments.iter().filter_map(|v| serde_json::from_value(v.clone()).ok()).collect(),
            _ => Vec::new(),
        }
    }

    /// the catalog's module documents as listed (the library's source)
    pub fn module_docs(&self) -> &[Value] {
        match &self.listing {
            Listing::Ready(c) => &c.modules,
            _ => &[],
        }
    }

    /// the catalog's modules as tracks (the core wraps each, `assign::module_track`), and why the others are not
    pub fn modules(&self) -> (Vec<(deck_space::rig::Instrument, deck_catalog::Descriptor)>, Vec<String>) {
        let Listing::Ready(c) = &self.listing else { return (Vec::new(), Vec::new()) };
        let (mut ok, mut not) = (Vec::new(), Vec::new());
        for m in &c.modules {
            match assign::module_track(m) {
                Ok(t) => ok.push(t),
                Err(e) => not.push(format!("{e:#}")),
            }
        }
        ok.sort_by(|a, b| a.0.id.cmp(&b.0.id));
        (ok, not)
    }

    /// the catalog's rig of a holder (`character:bells`), as the core makes a bundle of it
    pub fn bundle_of(&self, place: &deck_space::rig::address::Place) -> Option<Bundle> {
        let Listing::Ready(c) = &self.listing else { return None };
        c.bundle(c.rig(&format!("{}.{}", place.kind, place.id))?.content.clone()).ok()
    }

    /// the offline copy over `server`: what plays, saved on the server's version (picking it makes it the next one)
    pub fn take_local(&mut self, server: &Bundle) -> Option<Bundle> {
        let l = self.local.take()?;
        self.auto.version = l.server;
        Some(kept(&self.outbox, &self.subject, server).0)
    }

    /// `now` saved as the rig's next draft on the version open, the documents changed against `base` before it
    pub fn save(&mut self, base: &Bundle, now: &Bundle, at: std::time::Instant) {
        if self.auto.busy() {
            return;
        }
        let versions = match &self.listing {
            Listing::Ready(c) => c.versions.clone(),
            _ => BTreeMap::new(),
        };
        // an instrument a new track brought in as the catalog has it is no change of its own
        let catalog = self.instruments();
        let mut others = changed(base, now, &versions);
        others.retain(|(_, v, _)| !catalog.iter().any(|c| serde_json::from_value::<deck_space::rig::Instrument>(v.clone()).is_ok_and(|i| &i == c)));
        let (api, key, sent) = (self.api.clone(), format!("rig:{}", now.rig.subject), now.clone());
        let rig = serde_json::to_value(&now.rig).unwrap_or_default();
        self.auto.send(at, move |on| {
            let (answer, got) = save(&Api::new(api), &key, &rig, on, &others);
            ((got, sent), answer)
        });
    }

    /// the rig row's line: the subject and its version (where its save stands: the toolbar's pill)
    pub fn line(&self) -> String {
        format!("{} {}", self.subject, version(self.auto.version))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RIGS: &str = r#"{ "entries": [
        { "id": "character.catpope", "latest": 14, "draft": true, "toons": ["*"], "content": { "subject": "character.catpope" } },
        { "id": "character.nun", "latest": 3, "draft": false, "toons": ["*"], "content": { "subject": "character.nun" } },
        { "id": "item.incense_burner", "latest": null, "draft": false, "toons": ["*"], "content": { "subject": "item.incense_burner" } },
        { "id": "character.gone", "latest": 9, "removed": true, "toons": ["*"], "content": {} }
    ] }"#;
    const DOCS: &str = r#"{ "entries": [{ "id": "drone", "latest": 7, "content": { "id": "drone" } }] }"#;
    const REGISTRY: &str = r#"{ "characters": [{ "id": "catpope", "own_toon": "catpope" }, { "id": "nun", "own_toon": "feilchen" }] }"#;

    fn offline() -> Rigs {
        Rigs::offline(&std::env::temp_dir(), "character.catpope", Some(12), "")
    }

    fn ready() -> Rigs {
        let mut r = offline();
        r.listing = Listing::Ready(Catalog::parse(RIGS, DOCS, DOCS, r#"{ "entries": [] }"#).expect("it reads"));
        r
    }

    /// the panel model: the toon's rigs with their versions, the open one marked, a removed one gone
    #[test]
    fn the_picker_lists_the_toons_rigs() {
        let mut r = ready();
        let (items, note) = r.items(Some("catpope"), REGISTRY, "");
        let rows: Vec<(&str, &str)> = items.iter().map(|i| (i.label.as_str(), i.detail.as_str())).collect();
        assert_eq!(rows, [("character.catpope", "v14 · draft · open"), ("item.incense_burner", "in the catalog")]);
        assert_eq!(note, None);
        assert_eq!(r.choices, [Choice::Server("character.catpope".into()), Choice::Server("item.incense_burner".into())]);
        assert_eq!(r.items(None, REGISTRY, "").0.len(), 3, "no toon open: every rig");
        let (none, note) = r.items(Some("plf"), REGISTRY, "");
        assert_eq!((none.len(), note.as_deref()), (1, None), "an item's rig is every toon's");
    }

    /// the panel model: the toon's characters without a rig come after its rigs, and one opens as its empty rig
    #[test]
    fn a_character_without_a_rig_is_listed_and_opens_empty() {
        let registry = r#"{ "characters": [{ "id": "catpope", "own_toon": "catpope" }, { "id": "crow-fox", "own_toon": "catpope" }, { "id": "nun", "own_toon": "feilchen" }, { "id": "boy", "own_toon": "catpope" }] }"#;
        let mut r = ready();
        let (items, _) = r.items(Some("catpope"), registry, "");
        let rows: Vec<(&str, &str)> = items.iter().map(|i| (i.label.as_str(), i.detail.as_str())).collect();
        assert_eq!(rows, [("character.catpope", "v14 · draft · open"), ("item.incense_burner", "in the catalog"), ("character.boy", "no rig yet"), ("character.crow-fox", "no rig yet")]);
        assert_eq!(r.choices[3], Choice::Server("character.crow-fox".into()));
        let Listing::Ready(c) = &r.listing else { unreachable!() };
        let (subject, bundle, at) = read(c, "character.boy", None).expect("its empty rig");
        assert_eq!((subject.as_str(), bundle, at), ("character.boy", Bundle::empty("character.boy"), None));
        assert_eq!(read(c, "item.lamp", None).unwrap_err(), "rig:item.lamp: not on the server");
        let draft = crate::show::api::Draft { content: serde_json::json!({ "subject": "character.boy", "track": [] }), version: Some(1) };
        assert_eq!(read(c, "character.boy", Some(draft)).expect("its draft").2, Some(1), "a saved draft opens, not the empty rig");
        assert!(base(&std::env::temp_dir().join("deck-editor-no-base"), "character.boy").expect("no error").0.rig.tracks.is_empty(), "no core rig: empty");
    }

    #[test]
    fn signed_out_or_offline_the_picker_says_why_inside() {
        let mut r = Rigs::offline(&std::env::temp_dir(), "character.catpope", None, "catalog: HTTP 401");
        let (items, note) = r.items(Some("catpope"), REGISTRY, "sign in: approve code K7Q-42M at https://scutoons.com/account/decks");
        assert!(items.is_empty());
        assert_eq!(note.as_deref(), Some("no rigs from the server: sign in: approve code K7Q-42M at https://scutoons.com/account/decks (catalog: HTTP 401)"));
        let mut r = Rigs::offline(&std::env::temp_dir(), "character.catpope", None, "http://127.0.0.1:9/api/catalog/docs/rigs: connection refused");
        assert_eq!(r.items(None, "", "").1.as_deref(), Some("no rigs from the server: http://127.0.0.1:9/api/catalog/docs/rigs: connection refused"));
        assert!(r.open("character.catpope").is_err(), "nothing to open without the list");
    }

    #[test]
    fn the_offline_copy_says_older_or_newer() {
        assert_eq!(compare(Some(12), Some(14)), Some("older"));
        assert_eq!(compare(Some(15), Some(14)), Some("newer"));
        assert_eq!(compare(None, Some(14)), Some("older"));
        assert_eq!(compare(Some(14), Some(14)), None);
        let l = Local { subject: "character.catpope".into(), made_on: Some(12), server: Some(14) };
        assert_eq!(l.words(), "the local copy of character.catpope (made on v12) is older than the server's v14");
        let mut r = ready();
        r.local = Some(Local { made_on: Some(15), ..l });
        let (items, _) = r.items(Some("catpope"), REGISTRY, "");
        assert_eq!((items[0].label.as_str(), items[0].detail.as_str()), ("local copy of character.catpope", "newer than the server's"));
        assert_eq!(r.choices[0], Choice::Local);
    }

    #[test]
    fn a_save_gives_the_creators_link() {
        let link = creator_link("https://api.scutoons.com/api", "rig:character.catpope");
        assert_eq!(saved_words(15, &link), "saved as draft v15 – publish it in the creator: https://creator.scutoons.com/edit/rig:character.catpope");
        assert_eq!(creator_link("http://127.0.0.1:9000/api", "rig:x"), "http://127.0.0.1:10000/edit/rig:x");
        assert_eq!(offline().line(), "character.catpope v12");
    }

    /// no API (nothing listens on port 9): the save is offline and says where it tried
    #[test]
    fn an_offline_save_keeps_the_local_copy() {
        let (saved, _) = save(&Api::new("http://127.0.0.1:9/api".into()), "rig:character.catpope", &serde_json::json!({}), Some(12), &[]);
        let Err(Refusal::Offline(why)) = saved else { panic!("{saved:?}") };
        assert!(why.starts_with("http://127.0.0.1:9/api/entities/rig:character.catpope"), "{why}");
    }

    fn answer(r: &mut Rigs) -> Vec<Event> {
        for _ in 0..400 {
            let seen = r.poll(false);
            if !r.auto.busy() {
                return seen;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("no answer");
    }

    /// the panel model on a fake API: a burst of edits is one PUT on the version open, its answer the next base and
    /// the bundle sent the new difference; a conflict keeps yours and names both versions; offline goes again later
    #[test]
    fn the_rig_saves_itself_once_per_burst() {
        use crate::autosave::{tests::Fake, IDLE, RETRY};
        use std::time::{Duration, Instant};
        let fake = Fake::start(&[200, 409]);
        let mut r = Rigs::offline(&std::env::temp_dir(), "character.catpope", Some(12), "");
        r.api = fake.base.clone();
        let b = Bundle::core("character.catpope").expect("a core rig").expect("it reads");
        let t0 = Instant::now();
        let mut now = b.clone();
        for i in 0..4 {
            now = wires::set_gain(&now, "drone", 0.1 + i as f32 * 0.1).expect("set");
            r.auto.edit(t0 + Duration::from_millis(400 * i));
            assert!(!r.auto.due(t0 + Duration::from_millis(400 * i + 100)));
        }
        let end = t0 + Duration::from_millis(1200) + IDLE;
        assert!(r.auto.due(end));
        r.save(&b, &now, end);
        r.save(&b, &now, end);
        let seen = answer(&mut r);
        assert_eq!(fake.count(), 1, "one PUT for the burst");
        let (path, body) = fake.puts.lock().unwrap()[0].clone();
        assert_eq!((path.as_str(), &body["base"]), ("/api/entities/rig:character.catpope", &serde_json::json!(12)));
        assert!(seen.iter().any(|e| matches!(e, Event::Saved(sent) if sent.rig == now.rig)));
        assert_eq!((r.auto.version, r.auto.pill()), (Some(101), "saved · v101".to_string()));
        r.auto.edit(end);
        r.save(&b, &now, end + IDLE);
        let seen = answer(&mut r);
        assert_eq!(fake.puts.lock().unwrap()[1].1["base"], 101);
        assert!(seen.iter().any(|e| matches!(e, Event::Say(w, true) if w.contains("someone saved a newer version (v902)"))));
        assert_eq!(r.auto.pill(), "conflict – v902 on the server, yours (on v101) kept locally");
        r.auto.open(Some(101));
        r.api = "http://127.0.0.1:9/api".into();
        r.auto.edit(end);
        r.save(&b, &now, end + IDLE);
        assert!(answer(&mut r).is_empty(), "offline is not said in a notice");
        assert_eq!(r.auto.pill(), "offline – kept locally");
        assert!(!r.auto.due(end + IDLE + Duration::from_secs(1)) && r.auto.due(end + IDLE + RETRY));
    }

    #[test]
    fn a_changed_preset_is_saved_beside_the_rig_on_its_version() {
        let b = Bundle::core("character.catpope").expect("a core rig").expect("it reads");
        let next = wires::set_value(&b, "drone", "drone.gain", 0.33).expect("set");
        let id = next.presets.iter().find(|p| !b.presets.contains(p)).expect("one changed").id.clone();
        let versions = BTreeMap::from([(format!("preset:{id}"), 7)]);
        let c = changed(&b, &next, &versions);
        assert_eq!(c.iter().map(|(k, _, v)| (k.as_str(), *v)).collect::<Vec<_>>(), [(format!("preset:{id}").as_str(), Some(7))]);
        assert!(changed(&b, &b, &versions).is_empty());
    }

    /// Against a running API (`DECK_TEST_API`, the test user's token in `SCUTOONS_TOKEN`; skipped without): the list,
    /// catpope opened, a gain saved as the next draft, opened again at that version, a save on the old version refused.
    #[test]
    fn against_a_server_open_and_save() {
        let Ok(api) = std::env::var("DECK_TEST_API") else { return };
        let cache = std::env::temp_dir().join(format!("deck-editor-rigs-live-{}", std::process::id()));
        let mut r = Rigs::new(&api, &cache, "character.catpope", None, true);
        let wait = |r: &mut Rigs, done: &dyn Fn(&Rigs) -> bool| {
            let mut seen = Vec::new();
            for _ in 0..400 {
                seen.extend(r.poll(true));
                if done(r) {
                    return seen;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            panic!("no answer from {api}");
        };
        wait(&mut r, &|r| matches!(r.listing, Listing::Ready(_) | Listing::Failed(_)));
        let (items, note) = r.items(None, "", "");
        assert_eq!(note, None, "{:?}", r.items(None, "", "").1);
        assert!(items.iter().any(|i| i.label == "character.catpope"), "{items:?}");
        let opened = |seen: Vec<Event>| seen.into_iter().find_map(|e| if let Event::Opened(_, b) = e { Some(*b) } else { None }).expect("opened");
        r.open("character.catpope").expect("listed");
        let (base, plays) = opened(wait(&mut r, &|r| r.opening.is_none()));
        let first = r.auto.version;
        // another gain each run: an unchanged save keeps its version
        let next = wires::set_gain(&plays, "drone", 0.2 + (std::process::id() % 60) as f32 / 100.0).expect("set");
        let next = wires::set_value(&next, "drone", "drone.gain", 0.33).expect("a preset's value");
        r.save(&base, &next, std::time::Instant::now());
        let seen = wait(&mut r, &|r| !r.auto.busy());
        assert_eq!(r.auto.state, crate::autosave::State::Clean);
        assert!(seen.iter().any(|e| matches!(e, Event::Saved(_))));
        let saved = r.auto.version.expect("a version");
        assert_ne!(Some(saved), first);
        r.outbox.remove(KIND, "character.catpope");
        r.open("character.catpope").expect("listed");
        let (_, again) = opened(wait(&mut r, &|r| r.opening.is_none()));
        assert_eq!((r.auto.version, &again.rig), (Some(saved), &next.rig), "the saved draft comes back");
        r.auto.version = first;
        r.save(&base, &next, std::time::Instant::now());
        wait(&mut r, &|r| !r.auto.busy());
        assert!(matches!(r.auto.state, crate::autosave::State::Conflict(Some(_))), "{:?}", r.auto.state);
        std::fs::remove_dir_all(&cache).ok();
    }

    /// Against a running API (as above): a character without a rig opens empty, a catalog instrument becomes its first
    /// track, Save makes `rig:<subject>` a draft that opens again; its new module's first draft is taken by the server.
    #[test]
    fn against_a_server_a_rig_for_a_character_without_one() {
        let Ok(api) = std::env::var("DECK_TEST_API") else { return };
        let character = format!("norig{}", std::process::id());
        let subject = format!("character.{character}");
        let cache = std::env::temp_dir().join(format!("deck-editor-norig-live-{}", std::process::id()));
        let mut r = Rigs::new(&api, &cache, &subject, None, true);
        let wait = |r: &mut Rigs, done: &dyn Fn(&Rigs) -> bool| {
            let mut seen = Vec::new();
            for _ in 0..400 {
                seen.extend(r.poll(true));
                if done(r) {
                    return seen;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            panic!("no answer from {api}");
        };
        wait(&mut r, &|r| matches!(r.listing, Listing::Ready(_) | Listing::Failed(_)));
        r.open(&subject).expect("listed");
        let seen = wait(&mut r, &|r| r.opening.is_none());
        assert!(seen.iter().any(|e| matches!(e, Event::Say(w, false) if w.ends_with("has no rig yet – start from modules or write a new module"))));
        let (base, plays) = seen.into_iter().find_map(|e| if let Event::Opened(_, b) = e { Some(*b) } else { None }).expect("opened");
        assert!(plays.rig.tracks.is_empty() && r.auto.version.is_none());
        let instruments = crate::adding::instruments(&plays, r.instruments());
        let first = instruments.first().cloned().expect("the catalog has instruments");
        let (next, _) = crate::adding::apply(&plays, &crate::adding::Choice::Track(Box::new(first.clone()))).expect("a track");
        r.save(&base, &next, std::time::Instant::now());
        wait(&mut r, &|r| !r.auto.busy());
        assert_eq!(r.auto.state, crate::autosave::State::Clean);
        let saved = r.auto.version.expect("its first draft");
        r.open(&subject).expect("listed");
        let seen = wait(&mut r, &|r| r.opening.is_none());
        let (_, again) = seen.into_iter().find_map(|e| if let Event::Opened(_, b) = e { Some(*b) } else { None }).expect("opened");
        assert_eq!((r.auto.version, again.rig.tracks.iter().map(|t| t.instrument.as_str()).collect::<Vec<_>>()), (Some(saved), vec![first.id.as_str()]));
        let (key, doc) = crate::empty::new_module(&character);
        Api::new(api.clone()).save_draft(&key, &doc, None, MESSAGE).expect("the server takes the new module's first draft");
        std::fs::remove_dir_all(&cache).ok();
    }

    fn outbox(name: &str) -> (PathBuf, Outbox) {
        let dir = std::env::temp_dir().join(format!("deck-editor-rigs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let ob = Outbox::at(&dir);
        (dir, ob)
    }

    #[test]
    fn opened_beside_its_copy() {
        let b = Bundle::core("character.catpope").expect("a core rig").expect("it reads");
        let next = wires::set_gain(&b, "drone", 0.5).expect("set");
        let (dir, ob) = outbox("copy");
        assert_eq!(with_copy(&ob, "character.catpope", &b, Some(14)), (b.clone(), None), "no copy");
        let mut r = Rigs::offline(&dir, "character.catpope", Some(14), "");
        r.outbox = ob.clone();
        r.keep(&b, &next);
        assert_eq!(with_copy(&ob, "character.catpope", &b, Some(14)).0.rig, next.rig, "made on this version: its changes are back");
        let (plays, local) = with_copy(&ob, "character.catpope", &b, Some(16));
        assert_eq!((plays.rig, local.map(|l| l.made_on)), (b.rig.clone(), Some(Some(14))), "made on an older one: offered");
        r.auto.version = Some(15);
        r.saved(&next, &next);
        assert!(ob.all().is_empty(), "saved as it plays: off the outbox");
        assert_eq!(base(&dir, "character.catpope").expect("kept").1, Some(15), "what opens next is the saved one");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// a draft file of before the outbox becomes the rig's entry once, and is gone; an entry already there wins
    #[test]
    fn a_legacy_draft_file_is_moved_into_the_outbox_once() {
        let b = Bundle::core("character.catpope").expect("a core rig").expect("it reads");
        let next = wires::set_gain(&b, "drone", 0.5).expect("set");
        let (dir, ob) = outbox("legacy");
        let file = dir.join("drafts").join("character.catpope.rig.toml");
        wires::save_draft(&file, &b, &next, Some(14)).expect("written");
        migrate(&dir, &ob);
        assert!(!file.exists(), "read once and moved");
        let e = ob.get(KIND, "character.catpope").expect("its entry");
        assert_eq!((e.kind.as_str(), e.base), ("rig", Some(14)));
        let (plays, made_on) = kept(&ob, "character.catpope", &b);
        assert_eq!((plays.rig, made_on), (next.rig.clone(), Some(Some(14))), "the same rig plays");
        let other = wires::set_gain(&b, "drone", 0.9).expect("set");
        wires::save_draft(&file, &b, &other, Some(3)).expect("written");
        migrate(&dir, &ob);
        assert_eq!((file.exists(), ob.get(KIND, "character.catpope").map(|e| e.base)), (false, Some(Some(14))), "the outbox's own stays");
        std::fs::remove_dir_all(&dir).ok();
    }
}
