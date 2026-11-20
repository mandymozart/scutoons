//! The History (after OSCT's #312) of the document the front tab edits: a rig (`rig:<subject>`, the entity API), a deck
//! scene or a show (their own routes; the server stamps their `version`). Its saved steps with what each changed in
//! words, revert one step or go back to a point (always asked first), and this session's undo / redo of the editor's own
//! saved steps. Every rule – a step, a change in words, a revert, whom a back-to undoes – is deck-core history.rs; the
//! server stores each revert as a new draft. Every ask runs on a thread; the UI polls.
use crate::autosave;
use crate::show::api::{Api, Refusal, Route, Version};
use deck_core::history::{self as core, BackTo, Description, Revert, Saved, Step};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex, PoisonError};

/// the newest steps described (each needs two stored documents)
const DESCRIBED: usize = 12;

/// what the History asks the server
pub trait Remote: Send + Sync + 'static {
    fn versions(&self, r: &Route) -> Result<Vec<Version>, String>;
    fn doc(&self, r: &Route, id: u64) -> Result<Value, String>;
    /// the signed-in member as a version's author names them
    fn me(&self) -> Result<String, String>;
    fn revert(&self, r: &Route, version: u64, content: &Value, base: u64) -> Result<u64, Refusal>;
    fn revert_to(&self, r: &Route, version: u64, base: u64) -> Result<u64, Refusal>;
}

impl Remote for Api {
    fn versions(&self, r: &Route) -> Result<Vec<Version>, String> {
        Api::versions(self, r)
    }
    fn doc(&self, r: &Route, id: u64) -> Result<Value, String> {
        self.version_doc(r, id)
    }
    fn me(&self) -> Result<String, String> {
        Api::me(self)
    }
    fn revert(&self, r: &Route, version: u64, content: &Value, base: u64) -> Result<u64, Refusal> {
        Api::revert(self, r, version, content, base)
    }
    fn revert_to(&self, r: &Route, version: u64, base: u64) -> Result<u64, Refusal> {
        Api::revert_to(self, r, version, base)
    }
}

/// stored documents by version (versions never change), the server's stamp taken out
#[derive(Clone, Default)]
struct Docs(Arc<Mutex<HashMap<u64, Value>>>);

impl Docs {
    fn get(&self, remote: &dyn Remote, r: &Route, id: u64) -> Result<Value, String> {
        if let Some(v) = self.0.lock().unwrap_or_else(PoisonError::into_inner).get(&id) {
            return Ok(v.clone());
        }
        let mut v = remote.doc(r, id)?;
        if let (true, Some(o)) = (r.stamped(), v.as_object_mut()) {
            o.remove("version");
        }
        self.0.lock().unwrap_or_else(PoisonError::into_inner).insert(id, v.clone());
        Ok(v)
    }
}

fn saved(versions: &[Version]) -> Vec<Saved> {
    versions.iter().map(|v| Saved { id: v.id as i64, author: v.author.clone(), at_ms: v.at_ms, message: v.message.clone() }).collect()
}

/// a step of this session's own: one of its versions was stored by this editor
fn own(versions: &[Version], s: &Step) -> bool {
    versions.iter().any(|v| (s.first..=s.last).contains(&(v.id as i64)) && autosave::mine(v.id))
}

/// one step as the panel shows it
#[derive(Debug, Clone)]
pub struct Row {
    pub step: Step,
    /// what it changed; None: the first version, or older than the steps described
    pub described: Option<Result<Description, String>>,
    pub mine: bool,
    pub published: bool,
}

/// a document's steps, listed on the version the editor stood on
#[derive(Debug, Clone)]
pub struct Listed {
    pub route: Route,
    pub base: Option<u64>,
    pub rows: Vec<Row>,
}

fn list(remote: &dyn Remote, docs: &Docs, r: &Route, base: Option<u64>) -> Result<Listed, String> {
    let versions = remote.versions(r)?;
    let rows = core::steps(&saved(&versions))
        .into_iter()
        .enumerate()
        .map(|(i, step)| {
            let described = step.before.filter(|_| i < DESCRIBED).map(|b| describe(remote, docs, r, b as u64, step.last as u64));
            Row { mine: own(&versions, &step), published: versions.iter().any(|v| v.id as i64 == step.last && v.published), described, step }
        })
        .collect();
    Ok(Listed { route: r.clone(), base, rows })
}

fn describe(remote: &dyn Remote, docs: &Docs, r: &Route, before: u64, after: u64) -> Result<Description, String> {
    Ok(core::describe(&docs.get(remote, r, before)?, &docs.get(remote, r, after)?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    Revert,
    BackTo,
    Undo,
    Redo,
}

/// a revert or a back-to planned on the version the editor stands on, waiting for its prompt
#[derive(Debug, Clone)]
pub struct Plan {
    pub ask: Ask,
    pub route: Route,
    /// the step's last version (a revert), the point (a back-to)
    pub version: u64,
    pub base: u64,
    pub revert: Option<Revert>,
    pub back: Option<BackTo>,
    /// what it changes on the document
    pub described: Description,
    /// the signed-in member is known (`GET /me`), so others' saves can be told from their own (a back-to's names)
    pub knows_me: bool,
}

impl Plan {
    /// an undo or a redo goes without a prompt unless places changed since stay as they are
    pub fn asks(&self) -> bool {
        matches!(self.ask, Ask::Revert | Ask::BackTo) || self.revert.as_ref().is_some_and(|r| !r.kept.is_empty())
    }
}

fn plan_revert(remote: &dyn Remote, docs: &Docs, r: &Route, ask: Ask, (before, after): (u64, u64), base: u64) -> Result<Plan, String> {
    let (a, b, now) = (docs.get(remote, r, before)?, docs.get(remote, r, after)?, docs.get(remote, r, base)?);
    let revert = core::revert(&a, &b, &now);
    Ok(Plan { ask, route: r.clone(), version: after, base, described: core::describe(&now, &revert.doc), revert: Some(revert), back: None, knows_me: true })
}

fn plan_back(remote: &dyn Remote, docs: &Docs, r: &Route, point: u64, base: u64) -> Result<Plan, String> {
    let versions = remote.versions(r)?;
    // not signed in or no answer: whose saves are whose is not known, the prompt says so
    let me = remote.me().unwrap_or_default();
    let back = core::back_to(&saved(&versions), point as i64, &me);
    let described = core::describe(&docs.get(remote, r, base)?, &docs.get(remote, r, point)?);
    Ok(Plan { ask: Ask::BackTo, route: r.clone(), version: point, base, revert: None, back: Some(back), described, knows_me: !me.is_empty() })
}

/// the newest own step of this session not undone yet, planned as its revert (None: there is none)
fn plan_undo(remote: &dyn Remote, docs: &Docs, r: &Route, base: u64, skip: &BTreeSet<u64>) -> Result<Option<Plan>, String> {
    let versions = remote.versions(r)?;
    let steps = core::steps(&saved(&versions));
    let Some(s) = steps.iter().find(|s| s.before.is_some() && own(&versions, s) && !skip.contains(&(s.last as u64))) else { return Ok(None) };
    plan_revert(remote, docs, r, Ask::Undo, (s.before.unwrap_or_default() as u64, s.last as u64), base).map(Some)
}

fn apply(remote: &dyn Remote, p: &Plan) -> Result<u64, Refusal> {
    let id = match (&p.revert, p.ask) {
        (_, Ask::BackTo) => remote.revert_to(&p.route, p.version, p.base)?,
        (Some(rv), _) => remote.revert(&p.route, p.version, &rv.doc, p.base)?,
        (None, _) => return Err(Refusal::Other("nothing planned".into())),
    };
    autosave::remember(id);
    Ok(id)
}

/// an undo of this session's, to redo: the version it stood on and the one it stored
#[derive(Debug, Clone, PartialEq)]
pub struct Redo {
    pub route: Route,
    pub before: u64,
    pub after: u64,
}

enum Answer {
    Listed(Result<Listed, String>),
    Planned(Ask, Result<Option<Plan>, String>),
    Applied(Plan, Result<u64, Refusal>),
}

/// what the editor does after a poll
#[derive(Debug, Clone, PartialEq)]
pub enum Done {
    /// a revert stored as version `u64`: the editor opens the document's newest again
    Stored(Route, u64, String),
    Say(String, bool),
}

/// the History of the front document: its list, the ask on its way, the prompt, this session's undo / redo
pub struct History {
    remote: Arc<dyn Remote>,
    docs: Docs,
    job: Option<Receiver<Answer>>,
    pub listed: Option<Listed>,
    /// why there is no list
    pub why: Option<String>,
    /// the prompt open: confirm stores it, Back drops it
    pub asking: Option<Plan>,
    undone: BTreeSet<u64>,
    made: BTreeSet<u64>,
    pub redos: Vec<Redo>,
    /// the list is asked again at the next `show`
    stale: bool,
    /// the front document and the version the editor stands on, this frame
    pub doc: Option<(Route, Option<u64>)>,
}

impl History {
    pub fn new(remote: Arc<dyn Remote>) -> History {
        History { remote, docs: Docs::default(), job: None, listed: None, why: None, asking: None, undone: BTreeSet::new(), made: BTreeSet::new(), redos: Vec::new(), stale: false, doc: None }
    }

    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    fn run(&mut self, work: impl FnOnce(&dyn Remote, &Docs) -> Answer + Send + 'static) {
        let (tx, rx) = channel();
        let (remote, docs) = (self.remote.clone(), self.docs.clone());
        std::thread::spawn(move || {
            let _ = tx.send(work(&*remote, &docs));
        });
        self.job = Some(rx);
    }

    /// each frame the panel is open: the document shown and the version the editor stands on; a new one is listed
    pub fn show(&mut self, route: Option<&Route>, base: Option<u64>) {
        let Some(r) = route else {
            (self.listed, self.why) = (None, None);
            return;
        };
        let same = self.listed.as_ref().is_some_and(|l| &l.route == r && l.base == base);
        if self.busy() || (same && !self.stale) {
            return;
        }
        self.stale = false;
        let r = r.clone();
        self.run(move |remote, docs| Answer::Listed(list(remote, docs, &r, base)));
    }

    /// a step's revert, planned for the prompt
    pub fn ask_revert(&mut self, r: &Route, step: &Step, base: u64) {
        let Some(before) = step.before.filter(|_| !self.busy()) else { return };
        let (r, after) = (r.clone(), step.last as u64);
        self.run(move |remote, docs| Answer::Planned(Ask::Revert, plan_revert(remote, docs, &r, Ask::Revert, (before as u64, after), base).map(Some)));
    }

    /// back to version `point`, planned for the prompt
    pub fn ask_back(&mut self, r: &Route, point: u64, base: u64) {
        if self.busy() || point == base {
            return;
        }
        let r = r.clone();
        self.run(move |remote, docs| Answer::Planned(Ask::BackTo, plan_back(remote, docs, &r, point, base).map(Some)));
    }

    /// ctrl+z once nothing is unsaved: the newest own step of this session reverted
    pub fn undo(&mut self, r: &Route, base: u64) {
        if self.busy() {
            return;
        }
        let (r, skip) = (r.clone(), self.undone.union(&self.made).copied().collect::<BTreeSet<u64>>());
        self.run(move |remote, docs| Answer::Planned(Ask::Undo, plan_undo(remote, docs, &r, base, &skip)));
    }

    /// ctrl+shift+z: the last undo of this session reverted, while nothing was saved since
    pub fn redo(&mut self, r: &Route, base: u64) -> Option<Done> {
        if self.busy() {
            return None;
        }
        let Some(last) = self.redos.last().filter(|l| &l.route == r).cloned() else { return Some(Done::Say("nothing to redo".into(), false)) };
        if last.after != base {
            self.redos.clear();
            return Some(Done::Say("nothing to redo – saved since the undo".into(), false));
        }
        let r = r.clone();
        self.run(move |remote, docs| Answer::Planned(Ask::Redo, plan_revert(remote, docs, &r, Ask::Redo, (last.before, last.after), base).map(Some)));
        None
    }

    /// the prompt's confirm: stored on a thread
    pub fn confirm(&mut self) {
        let Some(p) = self.asking.take() else { return };
        self.store(p);
    }

    /// the prompt's Back: nothing stored
    pub fn cancel(&mut self) {
        if let Some(p) = self.asking.take().filter(|p| p.ask == Ask::Undo) {
            self.undone.remove(&p.version);
        }
    }

    fn store(&mut self, p: Plan) {
        self.run(move |remote, _| {
            let answer = apply(remote, &p);
            Answer::Applied(p, answer)
        });
    }

    /// the thread's answer, once it is here
    pub fn poll(&mut self) -> Vec<Done> {
        let Some(answer) = self.job.as_ref().and_then(|rx| rx.try_recv().ok()) else { return Vec::new() };
        self.job = None;
        match answer {
            Answer::Listed(Ok(l)) => (self.listed, self.why) = (Some(l), None),
            Answer::Listed(Err(why)) => (self.listed, self.why) = (None, Some(why)),
            Answer::Planned(ask, Err(why)) => return vec![Done::Say(format!("{}: {why}", word(ask)), true)],
            Answer::Planned(Ask::Undo, Ok(None)) => return vec![Done::Say("nothing of yours to undo in this session".into(), false)],
            Answer::Planned(_, Ok(None)) => {}
            Answer::Planned(ask, Ok(Some(p))) => {
                if ask == Ask::Undo {
                    self.undone.insert(p.version);
                }
                let nothing = p.revert.as_ref().is_some_and(|r| r.applied == 0);
                if nothing && ask != Ask::Revert {
                    return vec![Done::Say(format!("{}: everything it changed was changed again since – nothing to take back", word(ask)), false)];
                }
                if p.asks() {
                    self.asking = Some(p);
                } else {
                    self.store(p);
                }
            }
            Answer::Applied(p, Ok(id)) => {
                match p.ask {
                    Ask::Undo => {
                        self.made.insert(id);
                        self.redos.push(Redo { route: p.route.clone(), before: p.base, after: id });
                    }
                    Ask::Redo => {
                        self.redos.pop();
                    }
                    Ask::Revert | Ask::BackTo => {}
                }
                self.stale = true;
                let words = match p.ask {
                    Ask::BackTo => format!("{}: back to v{} as v{id}", p.route.key(), p.version),
                    ask => format!("{}: {} v{} as v{id}", p.route.key(), word(ask), p.version),
                };
                return vec![Done::Stored(p.route, id, words)];
            }
            Answer::Applied(p, Err(e)) => {
                if p.ask == Ask::Undo {
                    self.undone.remove(&p.version);
                }
                self.stale = true;
                return vec![Done::Say(format!("{} not stored: {e}", word(p.ask)), true)];
            }
        }
        Vec::new()
    }

    /// up to `secs` for the answer (the tests, a check run)
    #[cfg(test)]
    pub fn settle(&mut self) -> Vec<Done> {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while self.busy() && std::time::Instant::now() < until {
            let d = self.poll();
            if !d.is_empty() || !self.busy() {
                return d;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        Vec::new()
    }
}

pub fn word(ask: Ask) -> &'static str {
    match ask {
        Ask::Revert => "revert",
        Ask::BackTo => "back to",
        Ask::Undo => "undo",
        Ask::Redo => "redo",
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::json;

    /// a stored version: its id, author, time, note and document
    type Stored = (u64, String, i64, Option<String>, Value);

    /// the server's entity store in memory: every save a new version, a revert on a stale base refused
    #[derive(Default)]
    pub struct Store {
        next: Mutex<u64>,
        versions: Mutex<Vec<Stored>>,
    }

    impl Store {
        pub fn starting(at: u64) -> Arc<Store> {
            Arc::new(Store { next: Mutex::new(at), versions: Mutex::new(Vec::new()) })
        }

        /// a save by `who` at `ms`; `mine`: by this editor
        pub fn save(&self, who: &str, ms: i64, doc: Value, mine: bool) -> u64 {
            let id = self.push(who, ms, None, doc);
            if mine {
                autosave::remember(id);
            }
            id
        }

        fn push(&self, who: &str, ms: i64, message: Option<String>, doc: Value) -> u64 {
            let mut n = self.next.lock().unwrap();
            *n += 1;
            self.versions.lock().unwrap().push((*n, who.into(), ms, message, doc));
            *n
        }

        fn latest(&self) -> u64 {
            self.versions.lock().unwrap().last().map_or(0, |v| v.0)
        }

        pub fn newest(&self) -> (Value, Option<String>) {
            let v = self.versions.lock().unwrap();
            let last = v.last().unwrap();
            (last.4.clone(), last.3.clone())
        }

        fn stored(&self, base: u64, message: String, doc: Value) -> Result<u64, Refusal> {
            if base != self.latest() {
                return Err(Refusal::Conflict { theirs: Some(self.latest()), words: "someone saved a newer version".into() });
            }
            let ms = self.versions.lock().unwrap().last().map_or(0, |v| v.2) + 60_000;
            Ok(self.push("me@test", ms, Some(message), doc))
        }
    }

    impl Remote for Store {
        fn versions(&self, _: &Route) -> Result<Vec<Version>, String> {
            let v = self.versions.lock().unwrap();
            Ok(v.iter().rev().map(|(id, who, ms, message, _)| Version { id: *id, author: Some(who.clone()), at_ms: *ms, message: message.clone(), published: false }).collect())
        }
        fn me(&self) -> Result<String, String> {
            Ok("me@test".into())
        }
        fn doc(&self, _: &Route, id: u64) -> Result<Value, String> {
            self.versions.lock().unwrap().iter().find(|v| v.0 == id).map(|v| v.4.clone()).ok_or_else(|| "unknown version".into())
        }
        fn revert(&self, _: &Route, version: u64, content: &Value, base: u64) -> Result<u64, Refusal> {
            self.stored(base, format!("revert #{version}"), content.clone())
        }
        fn revert_to(&self, r: &Route, version: u64, base: u64) -> Result<u64, Refusal> {
            let doc = self.doc(r, version).map_err(Refusal::Other)?;
            self.stored(base, format!("back to #{version}"), doc)
        }
    }

    fn rig() -> Route {
        Route::Entity("rig:character.catpope".into())
    }

    /// three steps: someone else's first version, two of mine a minute apart
    fn three(at: u64) -> (Arc<Store>, History, [u64; 3]) {
        let s = Store::starting(at);
        let a = s.save("other@test", 0, json!({"gain": 1, "name": "Cat"}), false);
        let b = s.save("me@test", 60_000, json!({"gain": 2, "name": "Cat"}), true);
        let c = s.save("me@test", 120_000, json!({"gain": 2, "name": "Pope"}), true);
        let h = History::new(s.clone());
        (s, h, [a, b, c])
    }

    /// Against a running API (`DECK_TEST_API`, `SCUTOONS_TOKEN` of a content editor; rigs.rs's live tests): three saves of
    /// catpope's rig by this editor, then undo, redo, a revert and a back-to, each a new draft on the server.
    #[test]
    fn against_a_server_undo_redo_revert_and_back_to() {
        let Ok(base_url) = std::env::var("DECK_TEST_API") else { return };
        let cache = std::env::temp_dir().join(format!("deck-editor-history-live-{}", std::process::id()));
        let mut r = crate::rigs::Rigs::new(&base_url, &cache, "character.catpope", None, true);
        let wait = |r: &mut crate::rigs::Rigs, opening: bool| {
            for _ in 0..400 {
                let opened = r.poll(true).into_iter().find_map(|e| if let crate::rigs::Event::Opened(_, b) = e { Some(*b) } else { None });
                if opened.is_some() || (!opening && r.catalog().is_some() && !r.auto.busy()) {
                    return opened;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            panic!("no answer");
        };
        wait(&mut r, false);
        r.open("character.catpope").expect("listed");
        let (base, plays) = wait(&mut r, true).expect("opened");
        let gain = |g: f32| crate::wires::set_gain(&plays, "drone", g).expect("set");
        let g = (std::process::id() % 20) as f32 / 100.0;
        let (zero, one, two) = (gain(0.05 + g), gain(0.31 + g), gain(0.61 + g));
        let mut ids = Vec::new();
        for b in [&zero, &one, &two] {
            r.save(&base, b, std::time::Instant::now());
            wait(&mut r, false);
            ids.push(r.auto.version.expect("saved"));
            std::thread::sleep(std::time::Duration::from_millis(11_000));
        }
        let api = Api::new(base_url.clone());
        let route = Route::Entity("rig:character.catpope".into());
        let newest = || api.draft("rig:character.catpope").unwrap().unwrap();
        let rig_of = |b: &deck_space::rig::Bundle| serde_json::to_value(&b.rig).unwrap();
        let mut h = History::new(Arc::new(api.clone()));
        let stored = |h: &mut History| match h.settle().first() {
            Some(Done::Stored(_, id, _)) => *id,
            other => panic!("{other:?}"),
        };

        h.undo(&route, newest().version.unwrap());
        stored(&mut h);
        assert_eq!(newest().content["tracks"], rig_of(&one)["tracks"], "undo: my newest step taken back");
        assert_eq!(h.redo(&route, newest().version.unwrap()), None);
        stored(&mut h);
        assert_eq!(newest().content["tracks"], rig_of(&two)["tracks"], "redo");

        h.show(Some(&route), newest().version);
        h.settle();
        let rows = h.listed.clone().expect("listed").rows;
        let mine = rows.iter().find(|x| x.step.last as u64 == ids[1]).expect("my second save");
        assert!(mine.mine);
        let mine = mine.step.clone();
        h.ask_revert(&route, &mine, newest().version.unwrap());
        h.settle();
        assert!(h.asking.is_some(), "a revert always asks");
        h.confirm();
        stored(&mut h);
        h.ask_back(&route, mine.last as u64, newest().version.unwrap());
        h.settle();
        assert!(h.asking.as_ref().and_then(|p| p.back.as_ref()).is_some_and(|b| b.later >= 3));
        h.confirm();
        stored(&mut h);
        assert_eq!(newest().content["tracks"], rig_of(&one)["tracks"], "back to my second save");
        r.outbox.remove("rig", "character.catpope");
        std::fs::remove_dir_all(&cache).ok();
    }

    #[test]
    fn the_steps_are_listed_with_what_each_changed() {
        let (s, mut h, [_, b, c]) = three(10_000);
        s.save("me@test", 125_000, json!({"gain": 3, "name": "Pope"}), true);
        h.show(Some(&rig()), Some(c + 1));
        h.settle();
        let l = h.listed.as_ref().expect("listed");
        assert_eq!(l.rows.len(), 3, "my two saves 5 s apart are one step");
        assert_eq!((l.rows[0].step.first, l.rows[0].step.last, l.rows[0].mine), (c as i64, c as i64 + 1, true));
        let d = l.rows[0].described.clone().unwrap().unwrap();
        let lines: Vec<_> = d.lines.iter().map(|l| (l.place.as_str(), l.before.as_str(), l.after.as_str())).collect();
        assert_eq!(lines, vec![("gain", "2", "3"), ("name", "“Cat”", "“Pope”")]);
        assert_eq!(l.rows[1].step.last, b as i64);
        assert!(l.rows[2].described.is_none() && !l.rows[2].mine, "the first version: nothing before it");
    }

    #[test]
    fn a_revert_asks_first_and_keeps_what_changed_since() {
        let (s, mut h, [_, b, c]) = three(20_000);
        let step = core::steps(&saved(&s.versions(&rig()).unwrap())).into_iter().find(|x| x.last == b as i64).unwrap();
        h.ask_revert(&rig(), &step, c);
        assert!(h.settle().is_empty());
        let p = h.asking.clone().expect("always asked");
        assert_eq!((p.ask, p.version, p.base), (Ask::Revert, b, c));
        assert_eq!(p.revert.as_ref().unwrap().doc, json!({"gain": 1, "name": "Pope"}), "only the step's place goes back");
        h.cancel();
        assert_eq!(s.latest(), c, "Back stores nothing");

        h.ask_revert(&rig(), &step, c);
        h.settle();
        h.confirm();
        let done = h.settle();
        assert!(matches!(&done[..], [Done::Stored(_, id, _)] if *id == c + 1), "{done:?}");
        assert_eq!(s.newest(), (json!({"gain": 1, "name": "Pope"}), Some(format!("revert #{b}"))));
    }

    #[test]
    fn back_to_asks_naming_the_later_saves_and_other_people() {
        let (s, mut h, [a, b, c]) = three(30_000);
        let d = s.save("nun@test", 300_000, json!({"gain": 9, "name": "Pope"}), false);
        h.ask_back(&rig(), b, d);
        h.settle();
        let p = h.asking.clone().expect("always asked");
        assert_eq!(p.back, Some(BackTo { later: 2, others: vec!["nun@test".into()] }));
        assert_eq!(p.described.lines.len(), 2);
        h.confirm();
        h.settle();
        assert_eq!(s.newest(), (json!({"gain": 2, "name": "Cat"}), Some(format!("back to #{b}"))));
        h.ask_back(&rig(), a, d);
        h.settle();
        h.confirm();
        let done = h.settle();
        assert!(matches!(&done[..], [Done::Say(w, true)] if w.contains("newer")), "a stale base is refused: {done:?}");
        let _ = c;
    }

    #[test]
    fn the_signed_in_member_is_known_before_this_editor_saved() {
        let s = Store::starting(60_000);
        let a = s.save("me@test", 0, json!({"gain": 1}), false);
        let b = s.save("nun@test", 600_000, json!({"gain": 2}), false);
        let mut h = History::new(s.clone());
        h.ask_back(&rig(), a, b);
        h.settle();
        let p = h.asking.clone().expect("asked");
        assert!(p.knows_me, "the member comes from the API, not from this editor's saves");
        assert_eq!(p.back, Some(BackTo { later: 1, others: vec!["nun@test".into()] }));
    }

    #[test]
    fn undo_takes_back_my_newest_step_and_redo_brings_it_again() {
        let (s, mut h, [_, _, c]) = three(40_000);
        h.undo(&rig(), c);
        let done = h.settle();
        assert!(h.asking.is_none(), "nothing kept: no prompt");
        let Some(Done::Stored(_, u1, _)) = done.first().cloned() else { panic!("{done:?}") };
        assert_eq!(s.newest().0, json!({"gain": 2, "name": "Cat"}));

        h.undo(&rig(), u1);
        let Some(Done::Stored(_, u2, _)) = h.settle().first().cloned() else { panic!("the step before") };
        assert_eq!(s.newest().0, json!({"gain": 1, "name": "Cat"}), "then my step before it, never the undo itself");
        h.undo(&rig(), u2);
        assert!(matches!(&h.settle()[..], [Done::Say(w, false)] if w.contains("nothing")), "someone else's first version is not mine to undo");

        assert_eq!(h.redo(&rig(), u2), None);
        assert!(matches!(h.settle().first(), Some(Done::Stored(..))), "redo");
        assert_eq!(s.newest().0, json!({"gain": 2, "name": "Cat"}));
        assert_eq!(h.redos.len(), 1);
        let other = s.save("nun@test", 900_000, json!({"gain": 5, "name": "Cat"}), false);
        assert!(matches!(h.redo(&rig(), other), Some(Done::Say(w, false)) if w.contains("saved since")));
        assert!(h.redos.is_empty());
    }

    #[test]
    fn an_undo_whose_place_changed_since_asks() {
        let s = Store::starting(50_000);
        s.save("other@test", 0, json!({"gain": 1, "name": "Cat"}), false);
        let b = s.save("me@test", 60_000, json!({"gain": 2, "name": "Pope"}), true);
        let d = s.save("nun@test", 500_000, json!({"gain": 2, "name": "Nun"}), false);
        let mut h = History::new(s.clone());
        h.undo(&rig(), d);
        h.settle();
        let p = h.asking.clone().expect("a place changed since: asked");
        let r = p.revert.clone().unwrap();
        assert_eq!((p.version, r.applied, r.kept, r.doc), (b, 1, vec!["name".to_string()], json!({"gain": 1, "name": "Nun"})));
        h.cancel();
        h.undo(&rig(), d);
        h.settle();
        assert_eq!(h.asking.as_ref().map(|p| p.version), Some(b), "Back gives the step back to the next undo");
    }

    #[test]
    fn a_stamped_documents_version_is_no_change() {
        let s = Store::starting(60_000);
        let a = s.save("me@test", 0, json!({"id": "nave", "version": 1, "name": "Nave"}), true);
        let b = s.save("me@test", 60_000, json!({"id": "nave", "version": 2, "name": "The nave"}), true);
        let mut h = History::new(s.clone());
        let scene = Route::Scene("nave".into());
        h.show(Some(&scene), Some(b));
        h.settle();
        let d = h.listed.as_ref().unwrap().rows[0].described.clone().unwrap().unwrap();
        assert_eq!(d.lines.iter().map(|l| l.place.as_str()).collect::<Vec<_>>(), vec!["name"]);
        h.undo(&scene, b);
        h.settle();
        h.settle();
        assert_eq!(s.newest().0, json!({"id": "nave", "name": "Nave"}), "the server stamps it anew");
        let _ = a;
    }
}
