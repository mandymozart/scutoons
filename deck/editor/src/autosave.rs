//! When the editor saves itself (one rule for every document it edits): a burst of edits is one save, sent once nothing
//! changed for `IDLE`, or at once on leaving the view, the workspace or the editor, or on ctrl+s. No answer: the local
//! copy keeps it and it goes again every `RETRY`. Someone else's newer version: yours stays local and nothing more is
//! sent until the newest is opened. The save runs on a thread; the toolbar's pill says where it stands.
use crate::show::api::Refusal;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

pub const IDLE: Duration = Duration::from_secs(2);
pub const RETRY: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    /// the server has what is open (or nothing was ever saved: `version` is None)
    Clean,
    Saving,
    Offline,
    /// someone saved `theirs` over the version yours was made on
    Conflict(Option<u64>),
    Refused(String),
}

/// what a save carries back, and the server's answer
pub type Answer<T> = (T, Result<u64, Refusal>);

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

    #[cfg(test)]
    pub fn pending(&self) -> bool {
        self.edited.is_some()
    }

    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    fn may_send(&self) -> bool {
        self.job.is_none() && self.edited.is_some() && !matches!(self.state, State::Conflict(_) | State::Refused(_))
    }

    /// the burst is over (or, offline, the next try is)
    pub fn due(&self, now: Instant) -> bool {
        let Some((_, last)) = self.edited.filter(|_| self.may_send()) else { return false };
        match self.state {
            State::Offline => self.tried.is_none_or(|t| now.duration_since(t) >= RETRY),
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
