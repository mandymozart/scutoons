//! The outlet's relay client (#186, LIVE.md B.6): deck-space's relay client (relay.rs, #366 – the join as `output` of the
//! outlet, the keepalive, `resume { client, seq }` on a lost connection: inside the relay's replay buffer it gets what it
//! missed, else the snapshot) on the socket thread (socket.rs). What is the outlet's: the cells, preloads and the
//! show's slice to the worker threads (mod.rs), what they answer (`media.ready`) out.
use super::config::Config;
use super::socket::{self, Side};
use super::{Input, Status};
use deck_space::relay::{Client as Relay, Options, Out};
use protocol::{MediaPreload, Message, Role, Snapshot};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// how long a read waits before the outbox and the flags get a turn
const POLL: Duration = Duration::from_millis(50);

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

impl Client {
    /// the thread's body: the relay client until `stop`
    pub fn run(mut self) {
        let mut o = Options::new(&format!("{}/live/{}", self.cfg.base, self.cfg.code), Role::Output, Some(&self.cfg.outlet));
        o.nick = self.nick.take();
        o.resume = true;
        o.messages = true;
        let relay = match Relay::new(o) {
            Ok(r) => r,
            Err(e) => {
                self.status.lock().unwrap().link = e;
                return;
            }
        };
        let (stop, rejoin) = (self.stop.clone(), self.rejoin.clone());
        socket::run(relay, &mut self, &stop, Some(&rejoin), POLL);
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

impl Side for Client {
    fn feed(&mut self, c: &mut Relay, _now: f64) {
        // media.ready waits for an open socket
        while c.open() {
            let Ok(m) = self.outbox.try_recv() else { break };
            c.send_message(m);
        }
    }

    fn hear(&mut self, out: Out, c: &Relay) {
        let e = match out {
            Out::Status(_) => {
                self.status.lock().unwrap().link = c.line();
                return;
            }
            Out::Message(e) => e,
            _ => return,
        };
        match e.message {
            Message::Welcome(w) => {
                tracing::info!("live: output {} of outlet '{}' (session {})", w.client, self.cfg.outlet, self.cfg.code);
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
    }
}
