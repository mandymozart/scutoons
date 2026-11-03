//! The outlet's relay client (#186, LIVE.md B.6): one thread with a blocking WebSocket (tungstenite) – joins as
//! `output` of the outlet (`hello { role: output, group: <outlet> }`), hands the outlet's cells, preloads and the
//! show's slice to the worker threads (mod.rs), sends what they answer (`media.ready`), pings every 20 s (B.6's
//! keepalive), and on a lost connection joins again with `resume { client, seq }`: inside the relay's replay buffer it
//! gets what it missed, else the snapshot – the outlet's current cell and its last preload, shown at once.
use super::config::Config;
use super::{Input, Status};
use protocol::{Envelope, Hello, Message, MediaPreload, Ping, Resume, Role, Snapshot};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message as Ws, WebSocket};

/// B.6: ping every 20 s (also the keepalive)
const PING_EVERY: Duration = Duration::from_secs(20);
/// how long a read waits before the outbox and the flags get a turn
const POLL: Duration = Duration::from_millis(50);
/// after a dropped connection, before joining again (the relay frees the client id for the resume)
const REJOIN_PAUSE: Duration = Duration::from_millis(250);
/// reconnect backoff (s)
const BACKOFF: [u64; 5] = [1, 2, 4, 8, 10];

pub struct Client {
    pub cfg: Config,
    pub nick: Option<String>,
    /// cells, the show's slice, panic → the composer
    pub compose: Sender<Input>,
    /// preloads → the preloader
    pub preload: Sender<MediaPreload>,
    /// what the deck says (media.ready)
    pub outbox: Receiver<Message>,
    pub status: Arc<Mutex<Status>>,
    pub stop: Arc<AtomicBool>,
    /// drop the connection now and join again with resume (tests; a network change)
    pub rejoin: Arc<AtomicBool>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

impl Client {
    fn set_link(&self, s: String) {
        self.status.lock().unwrap().link = s;
    }

    /// the thread's body: join, serve, back off, join again – until `stop`
    pub fn run(self) {
        let mut resume: Option<Resume> = None;
        let mut tries = 0usize;
        while !self.stop.load(Ordering::Relaxed) {
            match self.connect() {
                Ok(mut ws) => {
                    tries = 0;
                    let why = self.serve(&mut ws, &mut resume);
                    let _ = ws.close(None);
                    let _ = ws.flush();
                    if self.stop.load(Ordering::Relaxed) {
                        break;
                    }
                    tracing::info!("live: the relay connection ended ({why}) – joining again");
                    self.set_link(format!("OFFLINE {why}"));
                    // a moment for the relay to see the old connection go – then the resume takes the old id
                    std::thread::sleep(REJOIN_PAUSE);
                }
                Err(e) => {
                    let wait = BACKOFF[tries.min(BACKOFF.len() - 1)];
                    tries += 1;
                    tracing::debug!("live: {e} – again in {wait} s");
                    self.set_link(format!("NO RELAY {e} - AGAIN IN {wait} S"));
                    let until = Instant::now() + Duration::from_secs(wait);
                    while Instant::now() < until && !self.stop.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
            }
        }
    }

    fn connect(&self) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, String> {
        super::config::dial(&self.cfg.join_url(), POLL)
    }

    fn send(ws: &mut WebSocket<MaybeTlsStream<TcpStream>>, m: Message) -> Result<(), String> {
        let text = serde_json::to_string(&Envelope::new("", m)).expect("an envelope serializes");
        ws.send(Ws::Text(text)).map_err(|e| e.to_string())
    }

    /// One connection: hello, then messages until it breaks; `resume` = where to resume next time
    fn serve(&self, ws: &mut WebSocket<MaybeTlsStream<TcpStream>>, resume: &mut Option<Resume>) -> String {
        let hello = Hello {
            code: self.cfg.code.clone(),
            role: Role::Output,
            group: Some(self.cfg.outlet.clone()),
            nick: self.nick.clone(),
            token: None,
            resume: resume.clone(),
        };
        if let Err(e) = Self::send(ws, Message::Hello(hello)) {
            return e;
        }
        let mut pinged = Instant::now();
        let mut seq: i64 = resume.as_ref().map_or(0, |r| r.seq);
        loop {
            if self.stop.load(Ordering::Relaxed) {
                return "stopping".into();
            }
            if self.rejoin.swap(false, Ordering::Relaxed) {
                return "asked to join again".into();
            }
            while let Ok(m) = self.outbox.try_recv() {
                if let Err(e) = Self::send(ws, m) {
                    return e;
                }
            }
            if pinged.elapsed() >= PING_EVERY {
                pinged = Instant::now();
                if let Err(e) = Self::send(ws, Message::Ping(Ping { t0: now_ms() })) {
                    return e;
                }
            }
            let text = match ws.read() {
                Ok(Ws::Text(t)) => t,
                Ok(Ws::Close(_)) => return "the relay closed it".into(),
                Ok(_) => continue,
                Err(tungstenite::Error::Io(e)) if crate::live::config::timed_out(&e) => continue,
                Err(e) => return e.to_string(),
            };
            let e = match serde_json::from_str::<Envelope>(&text) {
                Ok(e) => e,
                Err(err) => {
                    tracing::warn!("live: a message the deck doesn't read: {err}");
                    continue;
                }
            };
            seq = seq.max(e.seq as i64);
            match e.message {
                Message::Welcome(w) => {
                    tracing::info!("live: output {} of outlet '{}' (session {})", w.client, self.cfg.outlet, self.cfg.code);
                    self.set_link(format!("OUTPUT {} OF {} @ {}", w.client, self.cfg.outlet, self.cfg.code));
                    *resume = Some(Resume { client: w.client.clone(), seq });
                    self.apply(w.snapshot);
                }
                Message::Snapshot(s) => self.apply(s),
                Message::Cell(c) if c.outlet == self.cfg.outlet => {
                    let _ = self.compose.send(Input::Cell(c.cell));
                }
                Message::MediaPreload(m) if m.outlet == self.cfg.outlet => {
                    let _ = self.preload.send(m);
                }
                Message::State(s) => {
                    let _ = self.compose.send(Input::Slice(s.slice.max(0) as usize));
                }
                Message::Panic(p) if p.black == Some(true) => {
                    let _ = self.compose.send(Input::Black);
                }
                Message::Error(err) => {
                    tracing::warn!("live: the relay says {}: {}", err.code, err.message);
                    self.status.lock().unwrap().problems_relay = Some(format!("RELAY {}: {}", err.code, err.message));
                }
                _ => {}
            }
            if let Some(r) = resume.as_mut() {
                r.seq = seq;
            }
        }
    }

    /// a snapshot (join, reconnect outside the replay buffer, an assign): the outlet's cell, its preload, the slice
    fn apply(&self, s: Snapshot) {
        if let Some(m) = s.media.and_then(|mut m| m.remove(&self.cfg.outlet)) {
            let _ = self.preload.send(m);
        }
        if let Some(st) = s.state {
            let _ = self.compose.send(Input::Slice(st.slice.max(0) as usize));
        }
        if let Some(c) = s.cells.and_then(|mut c| c.remove(&self.cfg.outlet)) {
            let _ = self.compose.send(Input::Cell(c));
        }
    }
}
