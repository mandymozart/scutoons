//! The relay client's sockets on a thread (#366): deck-space relay.rs decides – the join, hello, keyframe, resume,
//! backoff, pings, which socket (#438) and the status; this adapter only moves bytes: a blocking tungstenite socket
//! (`config::dial` on its own thread, then a read timeout of `poll` so the loop gets its turns), the QUIC connection
//! (quic.rs) and the client's clock in ms of `now_ns` (a ping's `t0` is its µs).
//! The deck's link (space/link.rs) and the outlet's client (client.rs) are its `Side`s.
use super::{config, quic};
use deck_space::relay::{Client, Out};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message as Ws, WebSocket};

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

/// the longest nap while there is no socket (the app's outbox keeps folding)
const NAP: Duration = Duration::from_millis(50);
/// on QUIC the thread only waits for its channel: the app's outbox gets a turn this often (a WebSocket's `poll` is 20 ms
/// of latency on every value sent; QUIC is here to take that away)
const QUIC_TURN: Duration = Duration::from_millis(4);

/// the client's clock: ms of `now_ns` (the show clock's base)
pub fn now_ms() -> f64 {
    crate::clock::now_ns() as f64 / 1e6
}

/// What the app does around its client.
pub trait Side {
    /// each turn: hand the client what the app has (its outbox, the clock's asks)
    fn feed(&mut self, c: &mut Client, now: f64);
    /// what the client says – everything but the socket's own actions
    fn hear(&mut self, out: Out, c: &Client);
}

/// what the helper threads report; the number is the dial's, so a late word from a socket given up counts for nothing
enum Ev {
    Ws(u64, Result<Socket, String>),
    Quic(u64, quic::Event),
}

struct Links {
    ws: Option<Socket>,
    /// a WebSocket is being dialled
    dialing: bool,
    ws_dial: u64,
    quic: Option<quic::Handle>,
    quic_dial: u64,
    tx: Sender<Ev>,
    rx: Receiver<Ev>,
    poll: Duration,
}

/// Run `c` until `stop` (`rejoin`: drop the socket now and join again – a network change, tests).
pub fn run(mut c: Client, side: &mut impl Side, stop: &AtomicBool, rejoin: Option<&AtomicBool>, poll: Duration) {
    let (tx, rx) = mpsc::channel();
    let mut l = Links { ws: None, dialing: false, ws_dial: 0, quic: None, quic_dial: 0, tx, rx, poll };
    c.start(now_ms());
    while !stop.load(Ordering::Relaxed) {
        let now = now_ms();
        if rejoin.is_some_and(|r| r.swap(false, Ordering::Relaxed)) {
            c.rejoin();
        }
        side.feed(&mut c, now);
        c.tick(now);
        act(&mut c, &mut l, side);
        if let Some(sock) = l.ws.as_mut() {
            match sock.read() {
                Ok(Ws::Text(t)) => c.text(now_ms(), &t),
                Ok(Ws::Close(_)) => lost(&mut c, &mut l.ws, "the relay closed it"),
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if config::timed_out(&e) => {}
                Err(e) => lost(&mut c, &mut l.ws, &e.to_string()),
            }
            hear(&mut c, &mut l, None);
        } else {
            let until = Duration::from_secs_f64(c.next_wake().map_or(f64::INFINITY, |at| ((at - now) / 1000.0).max(0.0)).min(NAP.as_secs_f64()));
            let wait = if c.on_quic() { QUIC_TURN.min(until) } else { until };
            hear(&mut c, &mut l, Some(wait));
        }
        act(&mut c, &mut l, side);
    }
    c.stop();
    act(&mut c, &mut l, side);
}

/// what the helpers said; `wait`: for the first word at most this long
fn hear(c: &mut Client, l: &mut Links, wait: Option<Duration>) {
    let mut next = match wait {
        Some(w) => l.rx.recv_timeout(w),
        None => l.rx.try_recv().map_err(|_| RecvTimeoutError::Timeout),
    };
    while let Ok(ev) = next {
        let now = now_ms();
        match ev {
            Ev::Ws(n, r) if n == l.ws_dial && l.dialing => {
                l.dialing = false;
                match r {
                    Ok(s) => {
                        l.ws = Some(s);
                        c.opened(now);
                    }
                    Err(e) => c.closed(now, &e),
                }
            }
            Ev::Quic(n, e) if n == l.quic_dial => match e {
                quic::Event::Opened => c.quic_opened(now),
                quic::Event::Stream(b) => c.quic_stream(now, &b),
                quic::Event::Datagram(b) => c.quic_datagram(now, &b),
                quic::Event::Closed(why) => {
                    l.quic = None;
                    tracing::info!("relay {}: QUIC ended ({why})", c.code());
                    c.quic_closed(now, &why);
                }
            },
            _ => {}
        }
        next = l.rx.try_recv().map_err(|_| RecvTimeoutError::Timeout);
    }
}

fn lost(c: &mut Client, ws: &mut Option<Socket>, why: &str) {
    if let Some(mut s) = ws.take() {
        let _ = s.close(None);
        let _ = s.flush();
    }
    tracing::info!("relay {}: the connection ended ({why})", c.code());
    c.closed(now_ms(), why);
}

/// do what the client asks until it asks nothing more
fn act(c: &mut Client, l: &mut Links, side: &mut impl Side) {
    loop {
        let outs = c.take();
        if outs.is_empty() {
            return;
        }
        for o in outs {
            match o {
                Out::Dial(url) => {
                    l.ws_dial += 1;
                    l.dialing = true;
                    let (n, tx, poll) = (l.ws_dial, l.tx.clone(), l.poll);
                    std::thread::spawn(move || {
                        let _ = tx.send(Ev::Ws(n, config::dial(&url, poll)));
                    });
                }
                Out::Send(text) => {
                    if let Some(Err(e)) = l.ws.as_mut().map(|s| s.send(Ws::Text(text)).map_err(|e| e.to_string())) {
                        lost(c, &mut l.ws, &e);
                    }
                }
                Out::Close {} => {
                    l.dialing = false;
                    lost(c, &mut l.ws, "");
                }
                Out::DialQuic { url, hash } => {
                    l.quic_dial += 1;
                    let (n, tx) = (l.quic_dial, l.tx.clone());
                    l.quic = Some(quic::dial(url, hash, move |e| {
                        let _ = tx.send(Ev::Quic(n, e));
                    }));
                }
                Out::Stream(text) => {
                    if let Some(q) = &l.quic {
                        q.stream(text);
                    }
                }
                Out::Datagram(text) => {
                    if let Some(q) = &l.quic {
                        q.datagram(text);
                    }
                }
                Out::CloseQuic {} => {
                    l.quic = None;
                    l.quic_dial += 1;
                    c.quic_closed(now_ms(), "");
                }
                other => side.hear(other, c),
            }
        }
    }
}
