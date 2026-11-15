//! Build's rigs on the server (research/NATIVE-EDITOR-GUI.md §5 "How the two sides share data"): the open toon's rigs
//! from the catalog (`GET /api/catalog/docs/rigs`, and the instruments, presets and modules a rig names), one opened as
//! the core makes a bundle of it (`rig::assign::catalog_bundle`), saved back as a new draft version on the version it
//! was opened at (`PUT /api/entities/rig:<subject>`, a 409 said in words). Publishing stays in the browser
//! (decisions/deck.md 2026-11-09): a save gives the creator's link. The draft file (wires.rs) stays the offline copy and
//! is offered when the server's version differs from the one it was made on. Every ask runs on a thread; the UI polls.
use crate::show::api::Api;
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

/// what a save came to
#[derive(Debug, Clone, PartialEq)]
pub enum Saved {
    Busy,
    Done { version: u64, link: String },
    Refused(String),
}

impl Saved {
    pub fn words(&self) -> String {
        match self {
            Saved::Busy => "saving …".into(),
            Saved::Done { version, link } => format!("saved as draft v{version} – publish it in the creator: {link}"),
            Saved::Refused(why) => why.clone(),
        }
    }
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

/// the offline copy of `subject`
pub fn draft_path(cache: &Path, subject: &str) -> PathBuf {
    cache.join("drafts").join(format!("{subject}.rig.toml"))
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
        None => anyhow::bail!("no core rig {subject} and no copy of it from the server"),
    }
}

fn keep_base(cache: &Path, subject: &str, bundle: &Bundle, version: Option<u64>) {
    let text = serde_json::json!({ "version": version, "bundle": bundle }).to_string();
    let path = base_path(cache, subject);
    let _ = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&path, text));
}

/// A rig opened from the server and the offline copy beside it: what plays (the copy when it was made on this very
/// version: its unsaved changes are back), and the copy to offer when it was made on another
pub fn with_copy(path: &Path, subject: &str, server: &Bundle, at: Option<u64>) -> (Bundle, Option<Local>) {
    let Some(made_on) = wires::draft_version(path) else { return (server.clone(), None) };
    let copy = wires::load_draft(path, server);
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
fn save(api: &Api, key: &str, rig: &Value, base: Option<u64>, others: &[(String, Value, Option<u64>)]) -> (Saved, Vec<(String, u64)>) {
    let mut got = Vec::new();
    for (k, content, on) in others {
        match api.save_draft(k, content, *on, MESSAGE) {
            Ok(v) => got.push((k.clone(), v)),
            Err(e) => return (Saved::Refused(format!("{e} – the local copy keeps it")), got),
        }
    }
    let saved = match api.save_draft(key, rig, base, MESSAGE) {
        Ok(version) => Saved::Done { version, link: creator_link(&api.base, key) },
        Err(e) => Saved::Refused(format!("{e} – the local copy keeps it")),
    };
    (saved, got)
}

enum Listing {
    Asking(Receiver<Result<Catalog, String>>),
    Ready(Catalog),
    Failed(String),
}

/// a rig read from the server: its subject, its bundle, its version
type Read = Result<(String, Bundle, Option<u64>), String>;
/// a save's outcome and the versions the changed documents got
type Done = (Saved, Vec<(String, u64)>);

/// what happened since the last poll, for the stage and the notice
pub enum Event {
    /// a rig opened: its subject, the bundle it is the difference to, what plays
    Opened(String, Box<(Bundle, Bundle)>),
    /// saved: what plays is the server's now; the creator's link
    Saved(String),
    Say(String, bool),
}

/// Build's rig on the server: the list, the rig open and its version, the save
pub struct Rigs {
    api: String,
    cache: PathBuf,
    pub subject: String,
    /// the server version open: the base of the next save (None: never saved there)
    pub version: Option<u64>,
    listing: Listing,
    /// signed in when the list was asked: asked again when that changes
    asked_signed: bool,
    told: bool,
    opening: Option<Receiver<Read>>,
    saving: Option<Receiver<Done>>,
    pub saved: Option<Saved>,
    /// the offline copy offered
    pub local: Option<Local>,
    /// what the picker shows now, by its ids
    pub choices: Vec<Choice>,
}

impl Rigs {
    pub fn new(api: &str, cache: &Path, subject: &str, version: Option<u64>, signed: bool) -> Rigs {
        let mut r = Rigs { api: api.to_string(), cache: cache.to_path_buf(), subject: subject.to_string(), version, listing: Listing::Failed(String::new()), asked_signed: signed, told: false, opening: None, saving: None, saved: None, local: None, choices: Vec::new() };
        r.list(signed);
        r
    }

    /// no asking at all (a check run)
    pub fn offline(cache: &Path, subject: &str, version: Option<u64>, why: &str) -> Rigs {
        Rigs { api: String::new(), cache: cache.to_path_buf(), subject: subject.to_string(), version, listing: Listing::Failed(why.to_string()), asked_signed: false, told: true, opening: None, saving: None, saved: None, local: None, choices: Vec::new() }
    }

    fn list(&mut self, signed: bool) {
        let (tx, rx) = channel();
        let api = self.api.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Catalog::fetch(&Api::new(api)));
        });
        (self.listing, self.asked_signed) = (Listing::Asking(rx), signed);
    }

    pub fn draft(&self) -> PathBuf {
        draft_path(&self.cache, &self.subject)
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
            if let Some(e) = c.rig(&self.subject).filter(|e| compare(self.version, e.latest).is_some()) {
                let local = Local { subject: self.subject.clone(), made_on: self.version, server: e.latest };
                out.push(Event::Say(format!("{} – the rig picker opens the server's", local.words()), false));
            }
        }
        if let Some(Ok(answer)) = self.opening.as_ref().map(Receiver::try_recv) {
            self.opening = None;
            match answer {
                Ok((subject, server, at)) => {
                    keep_base(&self.cache, &subject, &server, at);
                    let (plays, local) = with_copy(&draft_path(&self.cache, &subject), &subject, &server, at);
                    let back = local.is_none() && plays != server;
                    (self.subject, self.version, self.local, self.saved) = (subject.clone(), at, local.clone(), None);
                    out.push(Event::Opened(subject.clone(), Box::new((server, plays))));
                    let words = match (local, back) {
                        (Some(l), _) => format!("{} – the rig picker offers it", l.words()),
                        (None, true) => format!("{subject} {}: the local copy's unsaved changes are back", version(at)),
                        (None, false) => format!("{subject} {} from the server", version(at)),
                    };
                    out.push(Event::Say(words, false));
                }
                Err(why) => out.push(Event::Say(why, true)),
            }
        }
        if let Some(Ok((saved, versions))) = self.saving.as_ref().map(Receiver::try_recv) {
            self.saving = None;
            if let Listing::Ready(c) = &mut self.listing {
                c.versions.extend(versions);
            }
            out.push(Event::Say(saved.words(), !matches!(saved, Saved::Done { .. })));
            if let Saved::Done { version, link } = &saved {
                self.version = Some(*version);
                out.push(Event::Saved(link.clone()));
            }
            self.saved = Some(saved);
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
            let key = format!("rig:{subject}");
            let read = Api::new(api).draft(&key).and_then(|d| match d {
                Some(d) => Ok((d.content, d.version)),
                None => catalog.rig(&subject).map(|e| (e.content.clone(), e.latest)).ok_or_else(|| format!("{key}: not on the server")),
            });
            let _ = tx.send(read.and_then(|(content, at)| Ok((subject, catalog.bundle(content)?, at))));
        });
        self.opening = Some(rx);
        Ok(())
    }

    /// the offline copy over `server`: what plays, saved on the server's version (picking it makes it the next one)
    pub fn take_local(&mut self, server: &Bundle) -> Option<Bundle> {
        let l = self.local.take()?;
        self.version = l.server;
        Some(wires::load_draft(&self.draft(), server))
    }

    /// `now` saved as the rig's next draft on the version open, the documents changed against `base` before it
    pub fn save(&mut self, base: &Bundle, now: &Bundle) {
        if self.saving.is_some() {
            return;
        }
        let versions = match &self.listing {
            Listing::Ready(c) => c.versions.clone(),
            _ => BTreeMap::new(),
        };
        let others = changed(base, now, &versions);
        let (api, key, at) = (self.api.clone(), format!("rig:{}", now.rig.subject), self.version);
        let rig = serde_json::to_value(&now.rig).unwrap_or_default();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(save(&Api::new(api), &key, &rig, at, &others));
        });
        (self.saving, self.saved) = (Some(rx), Some(Saved::Busy));
    }

    /// the rig row's line: the subject, its version, what the last save came to
    pub fn line(&self) -> String {
        let at = format!("{} {}", self.subject, version(self.version));
        match &self.saved {
            Some(s) => format!("{at} · {}", s.words()),
            None => at,
        }
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

    /// the panel model: saving, saved with the creator's link, refused in words
    #[test]
    fn the_save_states_are_said() {
        assert_eq!(Saved::Busy.words(), "saving …");
        let done = Saved::Done { version: 15, link: creator_link("https://api.scutoons.com/api", "rig:character.catpope") };
        assert_eq!(done.words(), "saved as draft v15 – publish it in the creator: https://creator.scutoons.com/edit/rig:character.catpope");
        assert_eq!(creator_link("http://127.0.0.1:9000/api", "rig:x"), "http://127.0.0.1:10000/edit/rig:x");
        let mut r = offline();
        assert_eq!(r.line(), "character.catpope v12");
        r.saved = Some(Saved::Refused("rig:character.catpope: someone saved a newer version – nothing was overwritten".into()));
        assert_eq!(r.line(), "character.catpope v12 · rig:character.catpope: someone saved a newer version – nothing was overwritten");
    }

    /// no API (nothing listens on port 9): the save is refused, says where it tried and that the local copy keeps it
    #[test]
    fn an_offline_save_keeps_the_local_copy() {
        let (saved, _) = save(&Api::new("http://127.0.0.1:9/api".into()), "rig:character.catpope", &serde_json::json!({}), Some(12), &[]);
        let Saved::Refused(why) = saved else { panic!("{saved:?}") };
        assert!(why.starts_with("http://127.0.0.1:9/api/entities/rig:character.catpope") && why.ends_with("the local copy keeps it"), "{why}");
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
        let first = r.version;
        // another gain each run: an unchanged save keeps its version
        let next = wires::set_gain(&plays, "drone", 0.2 + (std::process::id() % 60) as f32 / 100.0).expect("set");
        let next = wires::set_value(&next, "drone", "drone.gain", 0.33).expect("a preset's value");
        r.save(&base, &next);
        let seen = wait(&mut r, &|r| r.saving.is_none());
        assert!(matches!(r.saved, Some(Saved::Done { .. })), "{:?}", r.saved);
        assert!(seen.iter().any(|e| matches!(e, Event::Saved(link) if link.ends_with("/edit/rig:character.catpope"))));
        let saved = r.version.expect("a version");
        assert_ne!(Some(saved), first);
        std::fs::remove_file(r.draft()).ok();
        r.open("character.catpope").expect("listed");
        let (_, again) = opened(wait(&mut r, &|r| r.opening.is_none()));
        assert_eq!((r.version, &again.rig), (Some(saved), &next.rig), "the saved draft comes back");
        r.version = first;
        r.save(&base, &next);
        wait(&mut r, &|r| r.saving.is_none());
        let Some(Saved::Refused(why)) = &r.saved else { panic!("{:?}", r.saved) };
        assert!(why.contains("someone saved a newer version"), "{why}");
        std::fs::remove_dir_all(&cache).ok();
    }

    #[test]
    fn opened_beside_its_copy() {
        let b = Bundle::core("character.catpope").expect("a core rig").expect("it reads");
        let next = wires::set_gain(&b, "drone", 0.5).expect("set");
        let file = std::env::temp_dir().join(format!("deck-editor-rigs-{}", std::process::id())).join("rig.toml");
        assert_eq!(with_copy(&file, "character.catpope", &b, Some(14)), (b.clone(), None), "no copy");
        wires::save_draft(&file, &b, &next, Some(14)).expect("written");
        assert_eq!(with_copy(&file, "character.catpope", &b, Some(14)).0.rig, next.rig, "made on this version: its changes are back");
        let (plays, local) = with_copy(&file, "character.catpope", &b, Some(16));
        assert_eq!((plays.rig, local.map(|l| l.made_on)), (b.rig, Some(Some(14))), "made on an older one: offered");
        std::fs::remove_file(&file).ok();
    }
}
