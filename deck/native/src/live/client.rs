//! The outlet's relay client (#186, LIVE.md B.6): deck-space's relay client (relay.rs, #366 – the join as `output` of the
//! outlet, the keepalive, `resume { client, seq }` on a lost connection: inside the relay's replay buffer it gets what it
//! missed, else the snapshot) on the socket thread (socket.rs). Where a message goes – the cells, preloads and the show's
//! slice of this outlet – is deck-outlet's route.rs; here they go to the worker threads (mod.rs), what they answer
//! (`media.ready`) out.
use super::config::Config;
use super::socket::{self, Side};
use super::Status;
use deck_outlet::route::{route, Input, Routed};
use deck_space::relay::{Client as Relay, Options, Out};
use deck_ui::live::{Do, Session};
use protocol::{Cell, MediaPreload, Message, Role};
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
    /// #91: every cell → the deck's piece side (the frame thread: piece.rs)
    pub pieces: Sender<Cell>,
    /// #91: a flash → the deck's flash side (flash.rs); #471: and the client id the welcome gave
    pub flash: Sender<super::flash::In>,
    /// what the deck says (media.ready, flash.report)
    pub outbox: Receiver<Message>,
    pub status: Arc<Mutex<Status>>,
    pub stop: Arc<AtomicBool>,
    /// drop the connection now and join again with resume (tests; a network change)
    pub rejoin: Arc<AtomicBool>,
    /// #478: what the Live page asks (a role, a name) and what it shows (the relay client's session)
    pub page: Receiver<Do>,
    pub session: Arc<Mutex<Session>>,
}

impl Client {
    /// the thread's body: the relay client until `stop`
    pub fn run(mut self) {
        // #478: an empty outlet is no role (deck.toml's `role = ""`)
        let group = Some(self.cfg.outlet.as_str()).filter(|g| !g.is_empty());
        let mut o = Options::new(&format!("{}/live/{}{}", self.cfg.base, self.cfg.code, self.cfg.door), Role::Output, group);
        o.nick = self.nick.take();
        o.device = Some("native".into());
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
}

impl Side for Client {
    fn feed(&mut self, c: &mut Relay, _now: f64) {
        // #478: the Live page's choice – to the relay, the name kept in deck.toml (the role when the relay moved it)
        while let Ok(d) = self.page.try_recv() {
            match d {
                Do::Take(role) => {
                    c.take_role(role.as_deref());
                }
                Do::Name(name) => {
                    c.set_name(&name);
                    keep("nick", &name);
                }
            }
        }
        *self.session.lock().unwrap() = c.live();
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
        for r in route(e.message, &self.cfg.outlet) {
            match r {
                Routed::Joined(client) => {
                    tracing::info!("live: output {client} of outlet '{}' (session {})", self.cfg.outlet, self.cfg.code);
                    let _ = self.flash.send(super::flash::In::Joined(client));
                }
                Routed::Compose(i) => {
                    let _ = self.compose.send(i);
                }
                Routed::Preload(m) => {
                    let _ = self.preload.send(m);
                }
                Routed::Piece(c) => {
                    let _ = self.pieces.send(c);
                }
                Routed::Flash(f) => {
                    let _ = self.flash.send(super::flash::In::Flash(f));
                }
                Routed::Refused { code, message } => {
                    tracing::warn!("live: the relay says {code}: {message}");
                    self.status.lock().unwrap().problems_relay = Some(format!("RELAY {code}: {message}"));
                }
            }
        }
        *self.session.lock().unwrap() = c.live();
    }
}

/// #478: the Live page's choice in deck.toml (config.rs `Choice`)
fn keep(key: &str, value: &str) {
    if let Err(e) = super::config::Choice::keep(&super::config::deck_toml(), key, value) {
        tracing::warn!("live: {key} not kept in deck.toml: {e}");
    }
}
