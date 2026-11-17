//! When the editor saves itself (one rule for every document it edits): a burst of edits is one save, sent once nothing
//! changed for `IDLE`, or at once on leaving the view, the workspace or the editor, or on ctrl+s. No answer: the local
//! copy keeps it and it goes again every `RETRY`. Someone else's newer version: yours stays local and nothing more is
//! sent until the newest is opened. The save runs on a thread; the toolbar's pill says where it stands.
//! What goes out is first written to the outbox on disk and leaves it after a 2xx, so a restart sends it again.
use crate::show::api::Refusal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub const IDLE: Duration = Duration::from_secs(2);
pub const RETRY: Duration = Duration::from_secs(10);

/// a change not on the server yet: what it is, the server's version it was made on, what goes up
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub kind: String,
    pub id: String,
    pub base: Option<u64>,
    pub body: Value,
    /// the server had a newer version than `base`: kept, never sent over it
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub conflict: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theirs: Option<u64>,
}

impl Entry {
    pub fn new(kind: &str, id: &str, base: Option<u64>, body: Value) -> Entry {
        Entry { kind: kind.into(), id: id.into(), base, body, conflict: false, theirs: None }
    }
}

/// `<cache>/outbox/<kind>-<id>.json`, one file per document; none in a check run
#[derive(Debug, Clone, Default)]
pub struct Outbox {
    dir: Option<PathBuf>,
}

static CURRENT: OnceLock<Outbox> = OnceLock::new();

impl Outbox {
    pub fn at(cache: &Path) -> Outbox {
        Outbox { dir: Some(cache.join("outbox")) }
    }

    pub fn none() -> Outbox {
        Outbox::default()
    }

    /// the editor's outbox, for every `Api` it makes
    pub fn open(cache: &Path) -> Outbox {
        CURRENT.get_or_init(|| Outbox::at(cache)).clone()
    }

    pub fn current() -> Outbox {
        CURRENT.get().cloned().unwrap_or_default()
    }

    fn file(&self, kind: &str, id: &str) -> Option<PathBuf> {
        let name: String = format!("{kind}-{id}").chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' }).collect();
        Some(self.dir.as_ref()?.join(format!("{name}.json")))
    }

    /// written whole or not at all (a temporary file renamed over it)
    pub fn put(&self, e: &Entry) {
        let Some(file) = self.file(&e.kind, &e.id) else { return };
        let tmp = file.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(e).unwrap_or_default();
        let done = file.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&tmp, text)).and_then(|_| std::fs::rename(&tmp, &file));
        if let Err(err) = done {
            tracing::warn!("outbox {}: {err}", file.display());
        }
    }

    pub fn get(&self, kind: &str, id: &str) -> Option<Entry> {
        let text = std::fs::read_to_string(self.file(kind, id)?).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn remove(&self, kind: &str, id: &str) {
        if let Some(f) = self.file(kind, id) {
            let _ = std::fs::remove_file(f);
        }
    }

    pub fn all(&self) -> Vec<Entry> {
        let Some(dir) = &self.dir else { return Vec::new() };
        let mut out: Vec<Entry> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|f| f.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|f| serde_json::from_str(&std::fs::read_to_string(f.path()).ok()?).ok())
            .collect();
        out.sort_by(|a, b| (&a.kind, &a.id).cmp(&(&b.kind, &b.id)));
        out
    }

    /// the server's answer to `sent`: a 2xx takes it off (a newer change written since stays, on the new version);
    /// a conflict is marked and kept
    pub fn answered(&self, sent: &Entry, answer: &Result<u64, Refusal>) {
        let Some(now) = self.get(&sent.kind, &sent.id) else { return };
        match answer {
            Ok(_) if now.body == sent.body => self.remove(&sent.kind, &sent.id),
            Ok(v) => self.put(&Entry { base: Some(*v), ..now }),
            Err(Refusal::Conflict { theirs, .. }) if now.body == sent.body => self.put(&Entry { conflict: true, theirs: *theirs, ..now }),
            Err(_) => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    /// the server has what is open (or nothing was ever saved: `version` is None)
    Clean,
    Saving,
    /// from the outbox, not sent yet since the start
    Kept,
    Offline,
    /// someone saved `theirs` over the version yours was made on
    Conflict(Option<u64>),
    Refused(String),
}

/// a save on its way: what it carries back, and the server's answer
type Job<T> = Receiver<(T, Result<u64, Refusal>)>;

pub struct Autosave<T = ()> {
    /// the server's version of what is open: the base of the next save
    pub version: Option<u64>,
    pub state: State,
    /// the last look's fingerprint
    seen: Option<u64>,
    /// the first edit not on its way yet, and the last one
    edited: Option<(Instant, Instant)>,
    sent: Option<(Instant, Instant)>,
    tried: Option<Instant>,
    job: Option<Job<T>>,
}

impl<T> Default for Autosave<T> {
    fn default() -> Self {
        Autosave { version: None, state: State::Clean, seen: None, edited: None, sent: None, tried: None, job: None }
    }
}

/// what is on screen, as a number that changes when it does
pub fn fingerprint(v: &impl serde::Serialize) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    serde_json::to_vec(v).unwrap_or_default().hash(&mut h);
    h.finish()
}

impl<T: Send + 'static> Autosave<T> {
    pub fn new(version: Option<u64>) -> Self {
        Autosave { version, ..Self::default() }
    }

    /// an outbox entry: sent at once on its base, a conflict only named
    pub fn kept(e: &Entry) -> Self {
        let now = Instant::now();
        let state = if e.conflict { State::Conflict(e.theirs) } else { State::Kept };
        Autosave { version: e.base, state, edited: Some((now, now)), ..Self::default() }
    }

    /// a document opened at `version`: nothing of it is edited yet (the last one's save still lands on the server, its
    /// answer is not this one's)
    pub fn open(&mut self, version: Option<u64>) {
        *self = Autosave::new(version);
    }

    pub fn edit(&mut self, now: Instant) {
        self.edited = Some((self.edited.map_or(now, |(first, _)| first), now));
        if matches!(self.state, State::Refused(_)) {
            self.state = State::Clean;
        }
    }

    /// an edit when the fingerprint differs from the last look (the first look only takes it in)
    pub fn look(&mut self, fingerprint: u64, now: Instant) {
        if self.seen.is_some_and(|s| s != fingerprint) {
            self.edit(now);
        }
        self.seen = Some(fingerprint);
    }

    pub fn pending(&self) -> bool {
        self.edited.is_some()
    }

    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// nothing left to send, nothing on its way
    pub fn settled(&self) -> bool {
        self.state == State::Clean && !self.pending() && !self.busy()
    }

    fn may_send(&self) -> bool {
        self.job.is_none() && self.edited.is_some() && !matches!(self.state, State::Conflict(_) | State::Refused(_))
    }

    /// the burst is over (or, offline, the next try is)
    pub fn due(&self, now: Instant) -> bool {
        let Some((_, last)) = self.edited.filter(|_| self.may_send()) else { return false };
        match self.state {
            State::Offline | State::Kept => self.tried.is_none_or(|t| now.duration_since(t) >= RETRY),
            _ => now.duration_since(last) >= IDLE,
        }
    }

    /// leaving the view, the workspace or the editor: what is edited goes now
    pub fn leave(&self) -> bool {
        self.may_send()
    }

    /// ctrl+s: now, a refused save too
    pub fn now(&self) -> bool {
        self.job.is_none() && self.edited.is_some() && !matches!(self.state, State::Conflict(_))
    }

    /// `save` on a thread, on `self.version`; its answer comes through `poll`
    pub fn send(&mut self, now: Instant, save: impl FnOnce(Option<u64>) -> (T, Result<u64, Refusal>) + Send + 'static) {
        let (tx, rx) = channel();
        let base = self.version;
        std::thread::spawn(move || {
            let _ = tx.send(save(base));
        });
        (self.job, self.sent, self.edited, self.tried, self.state) = (Some(rx), self.edited, None, Some(now), State::Saving);
    }

    /// `body` as `kind`-`id` into the outbox, then `put` on a thread on `self.version`; the answer settles the entry
    pub fn send_kept(&mut self, now: Instant, outbox: &Outbox, kind: &str, id: &str, body: Value, put: impl FnOnce(&Value, Option<u64>) -> (T, Result<u64, Refusal>) + Send + 'static) {
        let entry = Entry::new(kind, id, self.version, body);
        outbox.put(&entry);
        let outbox = outbox.clone();
        self.send(now, move |base| {
            let (t, answer) = put(&entry.body, base);
            outbox.answered(&entry, &answer);
            (t, answer)
        });
    }

    /// a save that could not even start (the core's problems): said, sent again after the next edit
    pub fn refuse(&mut self, why: String) {
        self.state = State::Refused(why);
    }

    /// the answer, once it is here
    pub fn poll(&mut self) -> Option<(T, Result<u64, Refusal>)> {
        let answer = self.job.as_ref()?.try_recv().ok()?;
        self.answered(&answer.1);
        Some(answer)
    }

    /// up to `until` for the answer (quitting)
    pub fn wait(&mut self, until: Instant) -> Option<(T, Result<u64, Refusal>)> {
        let answer = self.job.as_ref()?.recv_timeout(until.saturating_duration_since(Instant::now())).ok()?;
        self.answered(&answer.1);
        Some(answer)
    }

    fn answered(&mut self, r: &Result<u64, Refusal>) {
        self.job = None;
        let sent = self.sent.take();
        if let Ok(v) = r {
            (self.version, self.state) = (Some(*v), State::Clean);
            return;
        }
        // what went is still to save, before what came since
        self.edited = match (sent, self.edited) {
            (Some((first, _)), Some((_, last))) => Some((first, last)),
            (s, e) => e.or(s),
        };
        self.state = match r {
            Err(Refusal::Offline(_)) => State::Offline,
            Err(Refusal::Conflict { theirs, .. }) => State::Conflict(*theirs),
            Err(Refusal::Other(w)) => State::Refused(w.clone()),
            Ok(_) => unreachable!(),
        };
    }

    /// the toolbar pill's words
    pub fn pill(&self) -> String {
        let yours = self.version.map_or_else(|| "a new one".to_string(), |v| format!("v{v}"));
        match &self.state {
            State::Conflict(theirs) => format!("conflict – {} on the server, yours (on {yours}) kept locally", theirs.map_or_else(|| "a newer version".to_string(), |v| format!("v{v}"))),
            State::Offline => "offline – kept locally".into(),
            State::Kept => "unsent – kept locally".into(),
            State::Refused(why) => format!("not saved – {why}"),
            State::Saving => "saving…".into(),
            State::Clean if self.edited.is_some() => "saving…".into(),
            State::Clean => self.version.map_or_else(|| "not saved yet".into(), |v| format!("saved · v{v}")),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// A fake API on a free port of 127.0.0.1: each PUT is answered by the next status of `answers` (200 with the next
    /// version, 409 naming `theirs`), every other request with a 404; the PUTs' paths and bodies are kept.
    pub struct Fake {
        pub base: String,
        pub puts: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
    }

    impl Fake {
        pub fn start(answers: &[u16]) -> Fake {
            let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
            let base = format!("http://{}/api", listener.local_addr().expect("its address"));
            let puts: Arc<Mutex<Vec<(String, serde_json::Value)>>> = Arc::default();
            let answers = Arc::new(Mutex::new(answers.to_vec()));
            let (p, a) = (puts.clone(), answers.clone());
            std::thread::spawn(move || {
                for stream in listener.incoming().flatten() {
                    let mut r = BufReader::new(stream);
                    let mut first = String::new();
                    if r.read_line(&mut first).is_err() {
                        continue;
                    }
                    let mut length = 0usize;
                    loop {
                        let mut line = String::new();
                        if r.read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
                            break;
                        }
                        if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                            length = v.trim().parse().unwrap_or(0);
                        }
                    }
                    let mut body = vec![0; length];
                    let _ = r.read_exact(&mut body);
                    let mut parts = first.split_whitespace();
                    let (method, path) = (parts.next().unwrap_or_default().to_string(), parts.next().unwrap_or_default().to_string());
                    let (code, answer) = if method == "PUT" {
                        let mut puts = p.lock().expect("puts");
                        puts.push((path, serde_json::from_slice(&body).unwrap_or_default()));
                        let n = puts.len() as u64;
                        let next = {
                            let mut a = a.lock().expect("answers");
                            if a.is_empty() { 200 } else { a.remove(0) }
                        };
                        match next {
                            409 => (409, serde_json::json!({ "error": "conflict", "latest": { "id": 900 + n } }).to_string()),
                            _ => (200, serde_json::json!({ "version": { "id": 100 + n, "version": 100 + n } }).to_string()),
                        }
                    } else {
                        (404, "{}".to_string())
                    };
                    let mut s = r.into_inner();
                    let _ = write!(s, "HTTP/1.1 {code} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}", answer.len());
                }
            });
            Fake { base, puts }
        }

        pub fn count(&self) -> usize {
            self.puts.lock().expect("puts").len()
        }
    }

    /// poll until the answer is here
    pub fn settle<T: Send + 'static>(a: &mut Autosave<T>) -> Option<(T, Result<u64, Refusal>)> {
        a.wait(Instant::now() + Duration::from_secs(5))
    }

    #[test]
    fn a_burst_of_edits_is_one_save_after_the_idle_time() {
        let t0 = Instant::now();
        let mut a: Autosave = Autosave::new(Some(828));
        assert_eq!(a.pill(), "saved · v828");
        a.look(1, t0);
        assert!(!a.pending(), "the first look only takes it in");
        for i in 0..5 {
            a.look(2 + i, t0 + Duration::from_millis(300 * i));
        }
        assert_eq!(a.pill(), "saving…");
        assert!(!a.due(t0 + Duration::from_millis(1200 + 1900)), "the burst is not over");
        let end = t0 + Duration::from_millis(1200 + 2000);
        assert!(a.due(end));
        let mut sent = 0;
        while a.due(end) {
            sent += 1;
            a.send(end, |base| ((), Ok(base.unwrap_or_default() + 1)));
        }
        assert_eq!(sent, 1);
        assert!(settle(&mut a).is_some_and(|(_, r)| r.is_ok()));
        assert_eq!((a.pill(), a.pending()), ("saved · v829".to_string(), false));
    }

    #[test]
    fn offline_keeps_it_and_tries_again_later() {
        let t0 = Instant::now();
        let mut a: Autosave = Autosave::new(Some(3));
        a.edit(t0);
        a.send(t0 + IDLE, |_| ((), Err(Refusal::Offline("no answer".into()))));
        settle(&mut a);
        assert_eq!(a.pill(), "offline – kept locally");
        assert!(a.pending() && !a.due(t0 + IDLE + Duration::from_secs(1)));
        assert!(a.leave(), "leaving tries at once");
        assert!(a.due(t0 + IDLE + RETRY));
        a.send(t0 + IDLE + RETRY, |base| ((), Ok(base.unwrap_or_default() + 1)));
        settle(&mut a);
        assert_eq!(a.pill(), "saved · v4");
    }

    #[test]
    fn a_conflict_keeps_yours_and_names_both_versions() {
        let t0 = Instant::now();
        let mut a: Autosave = Autosave::new(Some(828));
        a.edit(t0);
        a.send(t0 + IDLE, |_| ((), Err(Refusal::Conflict { theirs: Some(830), words: String::new() })));
        settle(&mut a);
        assert_eq!(a.pill(), "conflict – v830 on the server, yours (on v828) kept locally");
        a.edit(t0 + RETRY);
        assert!(!a.due(t0 + RETRY * 2) && !a.leave() && !a.now(), "nothing more is sent");
        a.open(Some(830));
        assert_eq!((a.pill(), a.pending()), ("saved · v830".to_string(), false), "the newest opened");
    }

    #[test]
    fn a_refused_save_waits_for_the_next_edit_and_ctrl_s_sends_it() {
        let t0 = Instant::now();
        let mut a: Autosave = Autosave::new(None);
        assert_eq!(a.pill(), "not saved yet");
        a.edit(t0);
        a.refuse("the scene has no cast".into());
        assert_eq!(a.pill(), "not saved – the scene has no cast");
        assert!(!a.due(t0 + RETRY) && !a.leave() && a.now());
        a.edit(t0 + RETRY);
        assert!(a.due(t0 + RETRY + IDLE));
    }

    #[test]
    fn edits_while_a_save_is_on_its_way_go_next() {
        let t0 = Instant::now();
        let mut a: Autosave = Autosave::new(Some(1));
        a.edit(t0);
        let (tx, rx) = channel::<()>();
        a.send(t0 + IDLE, move |_| {
            let _ = rx.recv();
            ((), Ok(2))
        });
        a.edit(t0 + IDLE + Duration::from_millis(10));
        assert!(!a.due(t0 + IDLE * 3), "one at a time");
        tx.send(()).expect("go");
        settle(&mut a);
        assert_eq!(a.version, Some(2));
        assert!(a.due(t0 + IDLE * 3), "the later edit is next");
    }

    fn outbox(name: &str) -> (std::path::PathBuf, Outbox) {
        let dir = std::env::temp_dir().join(format!("deck-editor-autosave-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let ob = Outbox::at(&dir);
        (dir, ob)
    }

    /// written whole (no temporary file left), a 2xx takes it off, a newer change written meanwhile stays on the new
    /// version, a conflict is marked and kept, an offline answer leaves it
    #[test]
    fn the_outbox_keeps_what_the_server_has_not_got() {
        let (dir, ob) = outbox("rule");
        let e = Entry::new("scene", "duet:1", Some(3), serde_json::json!({ "id": "duet" }));
        ob.put(&e);
        assert_eq!(std::fs::read_dir(dir.join("outbox")).unwrap().count(), 1, "one file, no temporary one");
        assert_eq!(ob.get("scene", "duet:1"), Some(e.clone()));
        ob.answered(&e, &Err(Refusal::Offline("no answer".into())));
        assert_eq!(ob.all(), std::slice::from_ref(&e));
        ob.answered(&e, &Ok(4));
        assert!(ob.all().is_empty());
        ob.put(&e);
        let newer = Entry::new("scene", "duet:1", Some(3), serde_json::json!({ "id": "duet", "n": 2 }));
        ob.put(&newer);
        ob.answered(&e, &Ok(4));
        assert_eq!(ob.get("scene", "duet:1").map(|x| (x.base, x.body)), Some((Some(4), newer.body.clone())));
        let on4 = Entry { base: Some(4), ..newer };
        ob.answered(&on4, &Err(Refusal::Conflict { theirs: Some(7), words: String::new() }));
        let kept = ob.get("scene", "duet:1").expect("kept");
        assert_eq!((kept.conflict, kept.theirs), (true, Some(7)));
        let a: Autosave = Autosave::kept(&kept);
        assert_eq!(a.pill(), "conflict – v7 on the server, yours (on v4) kept locally");
        assert!(!a.due(Instant::now() + RETRY) && !a.leave(), "never sent over theirs");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// no outbox (a check run): nothing is written, nothing is read
    #[test]
    fn no_outbox_reads_and_writes_nothing() {
        let ob = Outbox::none();
        let e = Entry::new("show", "s", None, serde_json::json!({}));
        ob.put(&e);
        assert_eq!((ob.get("show", "s"), ob.all().len()), (None, 0));
        assert!(Outbox::current().all().is_empty(), "no editor opened one in the tests");
    }

    /// a kept entry goes at once; the pill says it is unsent until it does
    #[test]
    fn a_kept_entry_is_sent_at_once() {
        let (dir, ob) = outbox("send");
        let e = Entry::new("show", "s", Some(5), serde_json::json!({ "id": "s" }));
        let mut a: Autosave<()> = Autosave::kept(&e);
        assert_eq!(a.pill(), "unsent – kept locally");
        assert!(a.due(Instant::now()));
        a.send_kept(Instant::now(), &ob, "show", "s", e.body.clone(), |body, base| {
            assert_eq!((body["id"].as_str(), base), (Some("s"), Some(5)));
            ((), Ok(6))
        });
        settle(&mut a);
        assert!(a.settled() && ob.all().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_fake_api_answers_puts_in_order() {
        let fake = Fake::start(&[409]);
        let api = crate::show::api::Api::with_cache(fake.base.clone(), std::env::temp_dir().join(format!("deck-editor-fake-{}", std::process::id())));
        assert!(matches!(api.save_scene("duet", &serde_json::json!({}), Some(3), "t"), Err(Refusal::Conflict { theirs: Some(901), .. })));
        assert_eq!(api.save_draft("rig:x", &serde_json::json!({}), Some(3), "t"), Ok(102));
        assert_eq!(fake.count(), 2);
        assert_eq!(fake.puts.lock().unwrap()[0].1["base"], 3);
    }
}
