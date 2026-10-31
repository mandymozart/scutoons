//! #223: one client's send queue, drained by its writer task (lib.rs). The latest value wins:
//!
//! - **ordered** messages (cues, `scene.sync`, `deck.graph`, `deck.synced`, snapshots, welcome, votes, errors …) stay
//!   complete and in order;
//! - **state-like** ones – each a snapshot of something current, so only the newest matters: `deck.values` (per sending
//!   deck; they carry only what changed, so a newer one merges into the waiting one key by key), `presence`,
//!   `vote.tally`, `state` – replace the one of their kind still waiting. A client that stalled gets the newest when it
//!   drains, not a backlog of stale values.
//!
//! Nothing is reordered across an ordered message: a state-like message merges only into one queued after the last
//! ordered message, else it queues anew. A client whose ordered backlog grows past `MAX_QUEUED` / `MAX_BYTES`, or whose
//! oldest ordered message waited `MAX_WAIT`, is dropped – it reconnects and resyncs from its snapshot (B.6).
//!
//! A message is serialized once (`Frame`, shared by every client it goes to); the writer copies its text into the
//! WebSocket frame (tungstenite 0.24 takes an owned String) – a coalesced-away message is never copied.
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use protocol::{Envelope, Message};
use tokio::sync::Notify;

/// ordered messages waiting for one client before it is dropped (a resume's replay is ≤ 500, session.rs)
pub const MAX_QUEUED: usize = 2048;
/// … or their bytes
pub const MAX_BYTES: usize = 8 << 20;
/// … or the age of the oldest one
pub const MAX_WAIT: Duration = Duration::from_secs(5);

/// What a state-like message replaces: the one of the same key still waiting.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Latest {
    /// a deck's `deck.values`, by its client id
    Values(Arc<str>),
    Presence,
    Tally,
    State,
}

/// One message as it goes out – serialized once, shared by every client it reaches.
#[derive(Debug)]
pub struct Frame {
    pub text: String,
    key: Option<Latest>,
    /// `deck.values`: the envelope, to merge a newer one into
    values: Option<Envelope>,
}

impl Frame {
    /// Serialize `e` (its `seq` and `from` already stamped) and classify it.
    pub fn of(e: Envelope) -> Arc<Frame> {
        let text = serde_json::to_string(&e).expect("envelope serializes");
        let key = match &e.message {
            Message::DeckValues(_) => Some(Latest::Values(Arc::from(e.from.as_str()))),
            Message::Presence(_) => Some(Latest::Presence),
            Message::VoteTally(_) => Some(Latest::Tally),
            Message::State(_) => Some(Latest::State),
            _ => None,
        };
        let values = matches!(e.message, Message::DeckValues(_)).then_some(e);
        Arc::new(Frame { text, key, values })
    }

    /// An already serialized ordered message (the replay log).
    pub fn ordered(text: String) -> Arc<Frame> {
        Arc::new(Frame { text, key: None, values: None })
    }
}

/// a state-like message waiting: the newest frame of its key, or – after merges – the merged values
#[derive(Debug)]
struct Slot {
    key: Latest,
    frame: Arc<Frame>,
    merged: Option<Box<Envelope>>,
}

impl Slot {
    fn absorb(&mut self, newer: Arc<Frame>) {
        // values merge (the first merge copies the waiting envelope once); anything else is replaced
        self.merged = match (newer.values.as_ref(), self.merged.take().or_else(|| self.frame.values.clone().map(Box::new))) {
            (Some(new), Some(mut base)) => {
                merge(&mut base, new);
                Some(base)
            }
            _ => None,
        };
        self.frame = newer;
    }

    fn text(self) -> String {
        match self.merged {
            Some(e) => serde_json::to_string(&e).expect("envelope serializes"),
            None => self.frame.text.clone(),
        }
    }
}

/// `newer`'s values over `base`'s, key by key; its seq, `to` and mood (when it has one)
fn merge(base: &mut Envelope, newer: &Envelope) {
    base.seq = newer.seq;
    base.to.clone_from(&newer.to);
    if let (Message::DeckValues(b), Message::DeckValues(n)) = (&mut base.message, &newer.message) {
        for (k, v) in &n.values {
            b.values.insert(k.clone(), v.clone());
        }
        if n.mood.is_some() {
            b.mood.clone_from(&n.mood);
        }
    }
}

#[derive(Debug)]
enum Entry {
    Ordered(Arc<Frame>, Instant),
    Latest(Slot),
}

#[derive(Debug, Default)]
struct Queue {
    entries: VecDeque<Entry>,
    /// entries taken so far – an entry's absolute index is `taken + its position`
    taken: u64,
    /// the state-like entries queued after the last ordered one, by key (absolute index)
    open: HashMap<Latest, u64>,
    ordered: usize,
    bytes: usize,
    /// dropped (too far behind) or the writer is gone: pushes fail
    closed: bool,
    /// state-like messages replaced before they went out
    skipped: u64,
}

/// What a writer's `take` found.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Took {
    /// how long the oldest ordered message taken had waited
    pub waited: Duration,
}

/// One client's send queue; the session pushes, the writer takes.
#[derive(Default)]
pub struct Outbox {
    q: Mutex<Queue>,
    /// something was pushed
    wake: Notify,
    /// the client fell too far behind (its connection ends)
    kill: Notify,
}

impl fmt::Debug for Outbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let q = self.lock();
        f.debug_struct("Outbox").field("queued", &q.entries.len()).field("closed", &q.closed).finish()
    }
}

impl Outbox {
    pub fn new() -> Arc<Outbox> {
        Arc::new(Outbox::default())
    }

    fn lock(&self) -> MutexGuard<'_, Queue> {
        self.q.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Queue `frame` for this client at `now`. False when the client is gone or was just dropped for falling behind.
    pub fn push(&self, frame: Arc<Frame>, now: Instant) -> bool {
        let mut q = self.lock();
        if q.closed {
            return false;
        }
        match frame.key.clone() {
            None => {
                q.bytes += frame.text.len();
                q.ordered += 1;
                q.entries.push_back(Entry::Ordered(frame, now));
                q.open.clear();
                let oldest = q.entries.iter().find_map(|e| match e {
                    Entry::Ordered(_, t) => Some(*t),
                    Entry::Latest(_) => None,
                });
                let too_old = oldest.is_some_and(|t| now.saturating_duration_since(t) >= MAX_WAIT);
                if q.ordered > MAX_QUEUED || q.bytes > MAX_BYTES || too_old {
                    tracing::info!(queued = q.ordered, bytes = q.bytes, too_old, "a client fell too far behind – dropped");
                    *q = Queue { closed: true, skipped: q.skipped, ..Queue::default() };
                    drop(q);
                    self.kill.notify_one();
                    return false;
                }
            }
            Some(key) => {
                let at = q.open.get(&key).map(|&abs| (abs - q.taken) as usize);
                match at.and_then(|i| match q.entries.get_mut(i) {
                    Some(Entry::Latest(slot)) => Some(slot),
                    _ => None,
                }) {
                    Some(slot) => {
                        slot.absorb(frame);
                        q.skipped += 1;
                    }
                    None => {
                        let abs = q.taken + q.entries.len() as u64;
                        q.entries.push_back(Entry::Latest(Slot { key: key.clone(), frame, merged: None }));
                        q.open.insert(key, abs);
                    }
                }
            }
        }
        drop(q);
        self.wake.notify_one();
        true
    }

    /// Move everything waiting into `out` (texts, in order) at `now`.
    pub fn take(&self, now: Instant, out: &mut Vec<String>) -> Took {
        let mut q = self.lock();
        let mut took = Took::default();
        while let Some(entry) = q.entries.pop_front() {
            let abs = q.taken;
            q.taken += 1;
            match entry {
                Entry::Ordered(frame, t) => {
                    q.ordered -= 1;
                    q.bytes -= frame.text.len();
                    took.waited = took.waited.max(now.saturating_duration_since(t));
                    out.push(frame.text.clone());
                }
                Entry::Latest(slot) => {
                    if q.open.get(&slot.key) == Some(&abs) {
                        q.open.remove(&slot.key);
                    }
                    out.push(slot.text());
                }
            }
        }
        took
    }

    /// Wait until something is pushed (or was, since the last wait).
    pub async fn pushed(&self) {
        self.wake.notified().await
    }

    /// Wait until this client is dropped for falling behind.
    pub async fn dropped(&self) {
        self.kill.notified().await
    }

    /// The writer is gone: later pushes fail (a session still holding the client sees it as unreachable).
    pub fn close(&self) {
        self.lock().closed = true;
    }

    pub fn is_closed(&self) -> bool {
        self.lock().closed
    }

    /// state-like messages replaced before they went out, so far
    pub fn skipped(&self) -> u64 {
        self.lock().skipped
    }

    /// Everything waiting, as texts (tests, and the session's own unit tests).
    pub fn drain(&self) -> Vec<String> {
        let mut out = vec![];
        self.take(Instant::now(), &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{Cue, DeckValues, Presence};

    fn values(from: &str, seq: u64, pairs: &[(&str, f64)], mood: Option<&str>) -> Arc<Frame> {
        let values = pairs.iter().map(|(k, v)| (k.to_string(), vec![*v])).collect();
        let mut e = Envelope::new(from, Message::DeckValues(DeckValues { values, mood: mood.map(str::to_owned) }));
        e.seq = seq;
        Frame::of(e)
    }

    fn cue(id: &str) -> Arc<Frame> {
        Frame::of(Envelope::new("cond", Message::Cue(Cue { id: id.into(), actions: vec![] })))
    }

    fn parse(texts: Vec<String>) -> Vec<Envelope> {
        texts.iter().map(|t| serde_json::from_str(t).unwrap()).collect()
    }

    /// a stalled client gets the newest values of each deck (merged key by key), and every cue, in order
    #[test]
    fn a_stalled_client_gets_the_newest_values_and_every_cue() {
        let ob = Outbox::new();
        let t = Instant::now();
        for i in 0..30 {
            ob.push(values("d1", i, &[("lfo.rate", i as f64)], None), t);
            ob.push(values("d2", 100 + i, &[("space.hue", i as f64 / 100.0)], (i == 3).then_some("happy")), t);
        }
        ob.push(values("d1", 40, &[("space.hue", 0.5)], None), t);
        let got = parse(ob.drain());
        assert_eq!(got.len(), 2, "one message per deck: {got:?}");
        let Message::DeckValues(d1) = &got[0].message else { panic!() };
        assert_eq!((got[0].from.as_str(), got[0].seq), ("d1", 40), "the newest seq");
        assert_eq!((d1.values["lfo.rate"].clone(), d1.values["space.hue"].clone()), (vec![29.0], vec![0.5]), "merged key by key");
        let Message::DeckValues(d2) = &got[1].message else { panic!() };
        assert_eq!((d2.values["space.hue"].clone(), d2.mood.as_deref()), (vec![0.29], Some("happy")), "the last mood stays");
        assert_eq!(ob.skipped(), 59);

        // cues between values: all of them, in order, and no value jumps over a cue
        for i in 0..10 {
            ob.push(values("d1", 200 + i, &[("lfo.rate", i as f64)], None), t);
            ob.push(cue(&format!("c{i}")), t);
            ob.push(values("d1", 300 + i, &[("lfo.rate", 10.0 + i as f64)], None), t);
            ob.push(Frame::of(Envelope::new("relay", Message::Presence(Presence { groups: HashMap::new(), ready: vec![], decks: None, routes: None }))), t);
        }
        let got = parse(ob.drain());
        let kinds: Vec<&str> = got.iter().map(|e| e.message.kind()).collect();
        let cues: Vec<String> = got.iter().filter_map(|e| if let Message::Cue(c) = &e.message { Some(c.id.clone()) } else { None }).collect();
        assert_eq!(cues, (0..10).map(|i| format!("c{i}")).collect::<Vec<_>>());
        assert_eq!(kinds[0], "deck.values");
        assert_eq!(&kinds[1..4], ["cue", "deck.values", "presence"], "after a cue: the values and presence sent after it, merged");
        assert_eq!(got.len(), 1 + 10 * 3, "{kinds:?}");
        let last = got.last().unwrap();
        assert_eq!(last.message.kind(), "presence");
        let Message::DeckValues(v) = &got[got.len() - 2].message else { panic!() };
        assert_eq!(v.values["lfo.rate"], vec![19.0]);
    }

    /// a client that drains as fast as it gets messages gets every one of them
    #[test]
    fn a_fast_client_gets_everything() {
        let ob = Outbox::new();
        let t = Instant::now();
        let mut n = 0;
        for i in 0..20 {
            ob.push(values("d1", i, &[("x", i as f64)], None), t);
            n += ob.drain().len();
        }
        assert_eq!((n, ob.skipped()), (20, 0));
    }

    #[test]
    fn a_client_too_far_behind_is_dropped() {
        let ob = Outbox::new();
        let t = Instant::now();
        for i in 0..MAX_QUEUED {
            assert!(ob.push(cue(&format!("c{i}")), t));
        }
        assert!(!ob.push(cue("one too many"), t), "dropped");
        assert!(ob.is_closed() && ob.drain().is_empty(), "the backlog is freed");
        assert!(!ob.push(values("d1", 1, &[("x", 1.0)], None), t), "and stays closed");

        // by age: the oldest ordered message waited MAX_WAIT
        let ob = Outbox::new();
        assert!(ob.push(cue("old"), t));
        assert!(ob.push(cue("4.9 s later"), t + MAX_WAIT - Duration::from_millis(100)));
        assert!(!ob.push(cue("5 s later"), t + MAX_WAIT));
        // state-like messages never drop a client – they don't pile up
        let ob = Outbox::new();
        for i in 0..10 * MAX_QUEUED as u64 {
            assert!(ob.push(values("d1", i, &[("x", 1.0)], None), t + Duration::from_secs(60)));
        }
        assert_eq!(ob.drain().len(), 1);
    }

    #[tokio::test]
    async fn a_drop_and_a_push_wake_their_waiters() {
        let ob = Outbox::new();
        ob.push(cue("c"), Instant::now());
        tokio::time::timeout(Duration::from_secs(1), ob.pushed()).await.expect("a push wakes the writer");
        let t = Instant::now();
        ob.push(cue("old"), t);
        ob.push(cue("late"), t + MAX_WAIT);
        tokio::time::timeout(Duration::from_secs(1), ob.dropped()).await.expect("a drop ends the connection");
    }
}
