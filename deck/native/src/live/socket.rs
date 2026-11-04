//! The relay client's socket on a thread (#366): deck-space relay.rs decides – the join, hello, keyframe, resume,
//! backoff, pings and the status; this adapter only moves bytes: a blocking tungstenite socket (`config::dial`, a read
//! timeout of `poll` so the loop gets its turns) and the client's clock in ms of `now_ns` (a ping's `t0` is its µs).
//! The deck's link (space/link.rs) and the outlet's client (client.rs) are its `Side`s.
use super::config;
use deck_space::relay::{Client, Out};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message as Ws, WebSocket};

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

/// the longest nap while there is no socket (the app's outbox keeps folding)
const NAP: Duration = Duration::from_millis(50);

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

/// Run `c` until `stop` (`rejoin`: drop the socket now and join again – a network change, tests).
pub fn run(mut c: Client, side: &mut impl Side, stop: &AtomicBool, rejoin: Option<&AtomicBool>, poll: Duration) {
    let mut ws: Option<Socket> = None;
    c.start(now_ms());
    while !stop.load(Ordering::Relaxed) {
        let now = now_ms();
        if rejoin.is_some_and(|r| r.swap(false, Ordering::Relaxed)) {
            c.rejoin();
        }
        side.feed(&mut c, now);
        c.tick(now);
        act(&mut c, &mut ws, side, poll);
        let Some(sock) = ws.as_mut() else {
            let wait = c.next_wake().map_or(NAP, |at| Duration::from_secs_f64(((at - now) / 1000.0).clamp(0.0, NAP.as_secs_f64())));
            std::thread::sleep(wait);
            continue;
        };
        match sock.read() {
            Ok(Ws::Text(t)) => c.text(now_ms(), &t),
            Ok(Ws::Close(_)) => lost(&mut c, &mut ws, "the relay closed it"),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if config::timed_out(&e) => {}
            Err(e) => lost(&mut c, &mut ws, &e.to_string()),
        }
        act(&mut c, &mut ws, side, poll);
    }
    c.stop();
    act(&mut c, &mut ws, side, poll);
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
fn act(c: &mut Client, ws: &mut Option<Socket>, side: &mut impl Side, poll: Duration) {
    loop {
        let outs = c.take();
        if outs.is_empty() {
            return;
        }
        for o in outs {
            match o {
                Out::Dial(url) => match config::dial(&url, poll) {
                    Ok(s) => {
                        *ws = Some(s);
                        c.opened(now_ms());
                    }
                    Err(e) => c.closed(now_ms(), &e),
                },
                Out::Send(text) => {
                    if let Some(Err(e)) = ws.as_mut().map(|s| s.send(Ws::Text(text)).map_err(|e| e.to_string())) {
                        lost(c, ws, &e);
                    }
                }
                Out::Close {} => lost(c, ws, ""),
                other => side.hear(other, c),
            }
        }
    }
}
