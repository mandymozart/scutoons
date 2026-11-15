//! The conductor's link to its relay (#91): deck-space's relay client (relay.rs: the join, resume, keepalive, which
//! socket) on deck-native's socket thread (live/socket.rs), joined as `conductor`. It only moves envelopes: what the show
//! core (deck-show's `Subtree`) sends goes out whole (its moment and its `to`), every envelope that comes is handed to the
//! frame thread, which gives it to the core. #531: the relay client's view of the linked readers (deck-space linked.rs
//! `Readers`) is copied out when it changes, for the Session sheet.
use deck_native::live::socket::{self, Side};
use deck_space::linked::Readers;
use deck_space::relay::{Client, Options, Out};
use protocol::{Envelope, Role};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const POLL: Duration = Duration::from_millis(20);

pub struct Link {
    /// what came from the relay, for the core
    pub inbox: Receiver<Envelope>,
    outbox: Sender<Envelope>,
    /// the relay client's line (`connected as c3`, `offline: …`)
    pub line: Arc<Mutex<String>>,
    /// #531: the session's code, and the linked readers as the relay client last heard them
    pub code: String,
    /// #530: the relay it joined – the reader's link names it when it is not the hosted one
    pub relay: String,
    pub readers: Arc<Mutex<Readers>>,
    stop: Arc<AtomicBool>,
}

struct Conductor {
    outbox: Receiver<Envelope>,
    inbox: Sender<Envelope>,
    line: Arc<Mutex<String>>,
    readers: Arc<Mutex<Readers>>,
}

impl Side for Conductor {
    fn feed(&mut self, c: &mut Client, _now: f64) {
        while c.open() {
            let Ok(e) = self.outbox.try_recv() else { break };
            c.send_whole(e);
        }
    }

    fn hear(&mut self, out: Out, c: &Client) {
        match out {
            Out::Status(_) => *self.line.lock().unwrap() = c.line(),
            Out::Message(e) => {
                let _ = self.inbox.send(e);
            }
            _ => {}
        }
        let mut readers = self.readers.lock().unwrap();
        if *readers != *c.readers() {
            readers.clone_from(c.readers());
        }
    }
}

impl Link {
    /// Join `relay` (`ws://host:port/live/<code>`) as its conductor; `token`: the session's (a dev relay takes any).
    pub fn start(relay: &str, token: Option<String>) -> Result<Link, String> {
        let mut o = Options::new(relay, Role::Conductor, None);
        o.token = token.or_else(|| Some("dev".into()));
        o.nick = Some("deck-editor".into());
        o.resume = true;
        o.messages = true;
        let client = Client::new(o)?;
        let code = client.code().to_string();
        let (outbox, out_rx) = mpsc::channel();
        let (in_tx, inbox) = mpsc::channel();
        let line = Arc::new(Mutex::new(format!("joining {relay}")));
        let stop = Arc::new(AtomicBool::new(false));
        let readers = Arc::new(Mutex::new(Readers::default()));
        let mut side = Conductor { outbox: out_rx, inbox: in_tx, line: line.clone(), readers: readers.clone() };
        let flag = stop.clone();
        std::thread::Builder::new().name("show-link".into()).spawn(move || socket::run(client, &mut side, &flag, None, POLL)).map_err(|e| e.to_string())?;
        Ok(Link { inbox, outbox, line, code, relay: relay.to_string(), readers, stop })
    }

    /// #551: a link without a socket, for the UI-model tests – what the core sends comes out of the first, what the second
    /// takes arrives as if the relay said it
    #[cfg(test)]
    pub fn loopback() -> (Link, Receiver<Envelope>, Sender<Envelope>) {
        let (outbox, sent) = mpsc::channel();
        let (relay, inbox) = mpsc::channel();
        let link = Link { inbox, outbox, line: Arc::new(Mutex::new("loopback".into())), code: "test".into(), relay: "loopback".into(), readers: Arc::default(), stop: Arc::default() };
        (link, sent, relay)
    }

    pub fn send(&self, out: Vec<Envelope>) {
        for e in out {
            let _ = self.outbox.send(e);
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
