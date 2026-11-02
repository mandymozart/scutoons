//! "Save as preset" from the deck's knobs (#270, research/INSTRUMENTS.md §2, §4–6): a performer turns the knobs of a
//! played rig and keeps the result as their own derivative of the preset the track plays – `POST /api/rigs/presets`
//! with the member's deck token (#180; the member grants the deck the `rigs` scope at pairing). Two rules, the same as
//! deck-space's `PresetBook::derive` and the server's: only what the performer changed against the played knobs goes
//! into the request, and only what differs from the parent's folded lineage is kept; the parent's `derive` right for how
//! the deck holds it decides whether it may be saved at all.
//!
//! A deck is often offline (a venue's LAN, no uplink): a save lands in the **outbox** on disk first
//! (`<cache>/rigs/outbox/<n>.json`, written atomically) and the `Saver`'s thread posts it when the API answers – at
//! once, then every RETRY while something waits. A save the API refuses (4xx: the parent is gone, the deck lost its
//! scope) moves to `outbox/refused/` with the answer, so it never blocks the queue; a network failure keeps the file.
//! The HUD shows the board's line (`SAVE …`).
//!
//! The stage plays scenes, not rigs, today (INSTRUMENTS.md §6 "next": the decks read scene-content's rigs): the key
//! hands the saver a `RigPlay` when a rig plays, and says so when none does.
use deck_space::rig::bundle::Bundle;
use deck_space::rig::Hold;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// how long a queued save waits before the next try while the API is away
pub const RETRY: Duration = Duration::from_secs(60);

/// The body of `POST /api/rigs/presets` (server docs/API.md "Rigs").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// the preset the track plays – the derivative's parent
    pub parent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<BTreeMap<String, String>>,
    /// `node.port` → the value: only what changed
    pub values: BTreeMap<String, f32>,
}

/// The rig a deck plays now: its bundle (core, or scene-content's) and how it holds it, the persona and the mood.
#[derive(Debug, Clone)]
pub struct RigPlay {
    pub bundle: Bundle,
    pub hold: Hold,
    pub persona: Option<String>,
    pub mood: Option<String>,
}

/// The request that saves `track`'s knobs: the played preset as the parent, the knobs the performer moved away from
/// their played values (`node.port` as the preset stores it), folded against the parent's lineage by `derive` (its
/// rights rule too). `values`: the knobs' values now, by their played path `<track>_<node>.<port>`.
pub fn derive_request(play: &RigPlay, track: &str, values: &BTreeMap<String, f32>, name: Option<&str>) -> anyhow::Result<Request> {
    let played = play.bundle.play(play.persona.as_deref(), play.mood.as_deref())?;
    let parent = played
        .tracks
        .iter()
        .find(|t| t.track == track)
        .ok_or_else(|| anyhow::anyhow!("no track '{track}' plays"))?
        .preset
        .clone()
        .ok_or_else(|| anyhow::anyhow!("track '{track}' plays no preset – nothing to derive from"))?;
    let prefix = format!("{track}_");
    let changed: BTreeMap<String, f32> = played
        .knobs
        .iter()
        .filter_map(|k| {
            let port = k.path.strip_prefix(&prefix)?;
            let now = *values.get(&k.path)?;
            (now.is_finite() && now != k.value).then(|| (port.to_string(), now))
        })
        .collect();
    let lib = play.bundle.library();
    let preset = lib.presets.derive(&parent, "m:pending", play.hold, &changed)?;
    let name = name.map(str::trim).filter(|n| !n.is_empty()).map(|n| BTreeMap::from([("en".to_string(), n.chars().take(100).collect())]));
    Ok(Request { parent, name, values: preset.values })
}

/// What posting one request came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Posted {
    /// stored: the new preset's id
    Saved(String),
    /// the API said no (4xx): the reason – the request leaves the queue
    Refused(String),
    /// no answer (offline, 5xx): the request stays
    Away(String),
}

/// The queue on disk.
#[derive(Debug, Clone)]
pub struct Outbox {
    pub dir: PathBuf,
}

impl Outbox {
    pub fn new(cache: &Path) -> Outbox {
        Outbox { dir: cache.join("rigs").join("outbox") }
    }

    /// Queues a request: a new file, written whole (a cut-off leaves nothing half)
    pub fn queue(&self, r: &Request) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(&self.dir)?;
        let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        let mut n = 0u32;
        let path = loop {
            let p = self.dir.join(format!("{ms:013}-{n:02}.json"));
            if !p.exists() {
                break p;
            }
            n += 1;
        };
        let tmp = path.with_extension("json.part");
        std::fs::write(&tmp, serde_json::to_vec_pretty(r).map_err(std::io::Error::other)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(path)
    }

    /// what waits, oldest first
    pub fn pending(&self) -> Vec<PathBuf> {
        let Ok(dir) = std::fs::read_dir(&self.dir) else { return Vec::new() };
        let mut out: Vec<PathBuf> = dir.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect();
        out.sort();
        out
    }

    /// Posts what waits, oldest first, until the API is away: saved files go, refused ones move to `refused/` with the
    /// answer next to them. Returns what happened, in order.
    pub fn flush(&self, mut post: impl FnMut(&Request) -> Posted) -> Vec<Posted> {
        let mut out = Vec::new();
        for path in self.pending() {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let Ok(r) = serde_json::from_str::<Request>(&text) else {
                self.refuse(&path, "not a request");
                out.push(Posted::Refused("not a request".into()));
                continue;
            };
            let p = post(&r);
            match &p {
                Posted::Saved(_) => {
                    let _ = std::fs::remove_file(&path);
                }
                Posted::Refused(why) => self.refuse(&path, why),
                Posted::Away(_) => {
                    out.push(p);
                    break;
                }
            }
            out.push(p);
        }
        out
    }

    fn refuse(&self, path: &Path, why: &str) {
        let dir = self.dir.join("refused");
        if std::fs::create_dir_all(&dir).is_ok() {
            if let Some(name) = path.file_name() {
                let _ = std::fs::rename(path, dir.join(name));
                let _ = std::fs::write(dir.join(name).with_extension("why.txt"), why);
                return;
            }
        }
        let _ = std::fs::remove_file(path);
    }
}

/// `POST <api>/rigs/presets` with the deck token (none: the API answers 401 – refused, the member pairs it first).
pub fn post_api(agent: &ureq::Agent, api: &str, token: Option<&str>, r: &Request) -> Posted {
    let url = format!("{}/rigs/presets", api.trim_end_matches('/'));
    let mut req = agent.post(&url).set("Content-Type", "application/json");
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {t}"));
    }
    let body = match serde_json::to_string(r) {
        Ok(b) => b,
        Err(e) => return Posted::Refused(format!("not JSON: {e}")),
    };
    match req.send_string(&body) {
        Ok(resp) => match resp.into_string().ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
            Some(v) => Posted::Saved(v["preset"]["id"].as_str().unwrap_or("?").to_string()),
            None => Posted::Saved("?".into()),
        },
        Err(ureq::Error::Status(code, resp)) if (400..500).contains(&code) => {
            let text = resp.into_string().unwrap_or_default();
            let why = serde_json::from_str::<serde_json::Value>(&text).ok().and_then(|v| v["message"].as_str().map(str::to_string)).unwrap_or(text);
            Posted::Refused(format!("HTTP {code}: {why}"))
        }
        Err(ureq::Error::Status(code, _)) => Posted::Away(format!("HTTP {code}")),
        Err(ureq::Error::Transport(t)) => Posted::Away(t.to_string()),
    }
}

/// What the HUD shows of the saver.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Board {
    /// requests waiting in the outbox
    pub queued: usize,
    /// the last answer: SAVED <id tail>, REFUSED …, OFFLINE, or what the key said
    pub note: String,
}

impl Board {
    /// ≤ 39 characters (the HUD's columns), upper case
    pub fn line(&self) -> String {
        let s = match (self.queued, self.note.is_empty()) {
            (0, true) => return String::new(),
            (0, false) => format!("SAVE {}", self.note),
            (n, true) => format!("SAVE QUEUED {n}"),
            (n, false) => format!("SAVE QUEUED {n} {}", self.note),
        };
        s.replace(['–', '—'], "-").to_uppercase().chars().take(39).collect()
    }
}

enum Msg {
    Queue(Request),
    Note(String),
    Flush,
}

/// The saver: the outbox and the thread that posts it.
pub struct Saver {
    tx: Sender<Msg>,
    board: Arc<Mutex<Board>>,
    pub outbox: Outbox,
}

impl Saver {
    /// Starts the thread: it flushes what the outbox holds at once, then every RETRY while something waits.
    pub fn start(cache: &Path, api: String, token: deck_link::TokenSource) -> Saver {
        let outbox = Outbox::new(cache);
        let board = Arc::new(Mutex::new(Board { queued: outbox.pending().len(), ..Board::default() }));
        let (tx, rx) = channel();
        let worker = Worker { outbox: outbox.clone(), api, token, board: board.clone(), agent: ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build() };
        std::thread::Builder::new().name("deck-save".into()).spawn(move || worker.run(rx)).expect("a thread");
        Saver { tx, board, outbox }
    }

    /// Queues a save (on disk) and posts it when the API answers
    pub fn save(&self, r: Request) {
        let _ = self.tx.send(Msg::Queue(r));
    }

    /// what the key has to say when nothing can be saved (no rig, no track)
    pub fn note(&self, text: impl Into<String>) {
        let _ = self.tx.send(Msg::Note(text.into()));
    }

    pub fn flush(&self) {
        let _ = self.tx.send(Msg::Flush);
    }

    pub fn board(&self) -> Board {
        self.board.lock().map(|b| b.clone()).unwrap_or_default()
    }

    /// the HUD's line ("" when there is nothing to say)
    pub fn line(&self) -> String {
        self.board().line()
    }
}

struct Worker {
    outbox: Outbox,
    api: String,
    token: deck_link::TokenSource,
    board: Arc<Mutex<Board>>,
    agent: ureq::Agent,
}

impl Worker {
    fn run(self, rx: Receiver<Msg>) {
        self.flush();
        loop {
            let waiting = !self.outbox.pending().is_empty();
            let msg = if waiting { rx.recv_timeout(RETRY) } else { rx.recv().map_err(|_| RecvTimeoutError::Disconnected) };
            match msg {
                Ok(Msg::Queue(r)) => {
                    match self.outbox.queue(&r) {
                        Ok(_) => self.set(|b| b.note = "QUEUED".into()),
                        Err(e) => self.set(|b| b.note = format!("NOT QUEUED: {e}")),
                    }
                    self.flush();
                }
                Ok(Msg::Note(text)) => self.set(|b| b.note = text),
                Ok(Msg::Flush) | Err(RecvTimeoutError::Timeout) => self.flush(),
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn flush(&self) {
        if self.outbox.pending().is_empty() {
            self.set(|b| b.queued = 0);
            return;
        }
        let token = self.token.get();
        let results = self.outbox.flush(|r| post_api(&self.agent, &self.api, token.as_deref(), r));
        let note = match results.last() {
            Some(Posted::Saved(id)) => format!("SAVED {}", &id[id.len().saturating_sub(6)..]),
            Some(Posted::Refused(why)) => format!("REFUSED {why}"),
            Some(Posted::Away(why)) => {
                tracing::warn!("save: the API is away ({why}) – {} waiting", self.outbox.pending().len());
                "OFFLINE".into()
            }
            None => String::new(),
        };
        for r in &results {
            if let Posted::Refused(why) = r {
                tracing::warn!("save: refused: {why}");
            }
        }
        self.set(|b| {
            b.queued = self.outbox.pending().len();
            b.note = note;
        });
    }

    fn set(&self, f: impl FnOnce(&mut Board)) {
        if let Ok(mut b) = self.board.lock() {
            f(&mut b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core() -> RigPlay {
        RigPlay { bundle: Bundle::core("character.catpope").unwrap().unwrap(), hold: Hold::Core, persona: None, mood: None }
    }

    fn played_values(play: &RigPlay) -> BTreeMap<String, f32> {
        play.bundle.play(None, None).unwrap().knobs.into_iter().map(|k| (k.path, k.value)).collect()
    }

    #[test]
    fn a_request_keeps_only_what_the_performer_changed() {
        let play = core();
        let mut v = played_values(&play);
        v.insert("voice_voice.index".into(), 4.0);
        v.insert("drone_drone.gain".into(), 0.2); // another track
        v.insert("voice_voice.ratio".into(), 1.5); // untouched: the played value
        let r = derive_request(&play, "voice", &v, Some("  Vespers ")).unwrap();
        assert_eq!(r.parent, "cathedral_voice.base");
        assert_eq!(r.values, BTreeMap::from([("voice.index".to_string(), 4.0)]));
        assert_eq!(r.name, Some(BTreeMap::from([("en".to_string(), "Vespers".to_string())])));
        let d = derive_request(&play, "drone", &v, None).unwrap();
        assert_eq!((d.parent.as_str(), d.values.len(), d.name), ("drone.base", 1, None));
        assert!(derive_request(&play, "organ", &v, None).unwrap_err().to_string().contains("no track"));
    }

    #[test]
    fn a_change_back_to_the_parents_folded_value_is_no_change() {
        // the bright mood plays cathedral_voice.bright (ratio 3.0 over base's 1.5); the performer turns it to 1.5 –
        // against the played value that is a change, against the parent's fold it is one too (bright = 3.0): kept
        let mut play = core();
        play.mood = Some("happy".into());
        let played = play.bundle.play(None, Some("happy")).unwrap();
        let bright = played.tracks.iter().find(|t| t.track == "voice").unwrap().preset.clone().unwrap();
        let mut v: BTreeMap<String, f32> = played.knobs.into_iter().map(|k| (k.path, k.value)).collect();
        v.insert("voice_voice.ratio".into(), 1.5);
        let r = derive_request(&play, "voice", &v, None).unwrap();
        assert_eq!(r.parent, bright);
        assert_eq!(r.values.get("voice.ratio"), Some(&1.5));
        // a lent rig may not be derived from (Rights::default: lent = play + tweak)
        play.hold = Hold::Lent;
        assert!(derive_request(&play, "voice", &v, None).unwrap_err().to_string().contains("Lent"));
    }

    #[test]
    fn the_outbox_queues_flushes_and_keeps_what_the_api_did_not_take() {
        let dir = std::env::temp_dir().join(format!("deck-save-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let outbox = Outbox::new(&dir);
        let r = |n: f32| Request { parent: "cathedral_voice.base".into(), name: None, values: BTreeMap::from([("voice.index".to_string(), n)]) };
        outbox.queue(&r(1.0)).unwrap();
        outbox.queue(&r(2.0)).unwrap();
        outbox.queue(&r(3.0)).unwrap();
        assert_eq!(outbox.pending().len(), 3);
        // offline: nothing goes, the first try stops the pass
        let mut tries = 0;
        let out = outbox.flush(|_| {
            tries += 1;
            Posted::Away("fetch failed".into())
        });
        assert_eq!((tries, out.len(), outbox.pending().len()), (1, 1, 3));
        // online: the first is saved, the second refused (moved aside with the reason), the third saved
        let mut seen = Vec::new();
        let out = outbox.flush(|r| {
            seen.push(r.values["voice.index"]);
            if r.values["voice.index"] == 2.0 { Posted::Refused("HTTP 422: nope".into()) } else { Posted::Saved("m:01X".into()) }
        });
        assert_eq!(seen, vec![1.0, 2.0, 3.0], "oldest first");
        assert_eq!(out, vec![Posted::Saved("m:01X".into()), Posted::Refused("HTTP 422: nope".into()), Posted::Saved("m:01X".into())]);
        assert_eq!(outbox.pending().len(), 0);
        let refused: Vec<_> = std::fs::read_dir(dir.join("rigs/outbox/refused")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert_eq!(refused.len(), 2, "the request and its why.txt");
        assert!(refused.iter().any(|n| n.ends_with(".why.txt")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_board_line_fits_the_hud() {
        assert_eq!(Board::default().line(), "");
        assert_eq!(Board { queued: 2, note: String::new() }.line(), "SAVE QUEUED 2");
        assert_eq!(Board { queued: 0, note: "SAVED 9G5FAV".into() }.line(), "SAVE SAVED 9G5FAV");
        assert!(Board { queued: 1, note: "REFUSED HTTP 403: a deck only reads – changes are made in the app".into() }.line().len() <= 39);
    }
}
