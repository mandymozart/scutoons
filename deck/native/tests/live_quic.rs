//! #438: the deck's real socket thread (live/socket.rs, live/quic.rs) against the real relay and its QUIC door on
//! 127.0.0.1: a deck joins over QUIC and reaches a conductor on the WebSocket; with no door it is on the WebSocket within
//! a moment; when UDP dies mid-show it moves to the WebSocket as the same client. Ports 7000–7099 only.
#![cfg(feature = "live")]

use deck_native::live::socket::{self, Side};
use deck_space::relay::{Client, In, Options, Out};
use protocol::{DeckGraph, DeckValues, Role};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(20);

fn free_port() -> u16 {
    let start = 7000 + (std::process::id() % 50) as u16;
    (start..7100).chain(7000..start).find(|p| TcpListener::bind(("127.0.0.1", *p)).is_ok() && UdpSocket::bind(("127.0.0.1", *p)).is_ok()).expect("a free port in 7000–7099")
}

/// The real relay on its own runtime with the door open; its WebSocket port, the door's address, the certificate's hash.
fn relay() -> (tokio::runtime::Runtime, u16, SocketAddr, String) {
    static N: AtomicU32 = AtomicU32::new(0);
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let identity = wtransport::Identity::self_signed(["localhost", "127.0.0.1"]).unwrap();
    let hash: String = identity.certificate_chain().as_slice()[0].hash().as_ref().iter().map(|b| format!("{b:02x}")).collect();
    let dir = std::env::temp_dir().join(format!("deck-live-quic-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).unwrap();
    let files = relay::tls::TlsFiles { cert: dir.join("cert.pem"), key: dir.join("key.pem") };
    std::fs::write(&files.cert, identity.certificate_chain().as_slice()[0].to_pem()).unwrap();
    std::fs::write(&files.key, identity.private_key().to_secret_pem()).unwrap();
    let std_listener = TcpListener::bind(("127.0.0.1", free_port())).unwrap();
    let port = std_listener.local_addr().unwrap().port();
    std_listener.set_nonblocking(true).unwrap();
    let door: SocketAddr = ([127, 0, 0, 1], port).into();
    let shared = Arc::new(relay::Relay::new());
    {
        let _in = rt.enter();
        let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
        rt.spawn(relay::serve(listener, shared.clone()));
        rt.spawn(relay::doors::quic::door(relay::doors::Doors::new(shared), door, files).unwrap());
    }
    (rt, port, door, hash)
}

/// What a test's client hears; it sends each value it finds in `give` once it is joined.
#[derive(Debug, PartialEq)]
enum Heard {
    Joined { client: String, quic: bool },
    Values(HashMap<String, Vec<f64>>),
}

struct Test {
    tx: Sender<Heard>,
    give: Receiver<DeckValues>,
}

impl Side for Test {
    fn feed(&mut self, c: &mut Client, _now: f64) {
        while let Ok(v) = self.give.try_recv() {
            c.values(v);
        }
    }
    fn hear(&mut self, out: Out, c: &Client) {
        let _ = match out {
            Out::Joined { client, .. } => self.tx.send(Heard::Joined { client, quic: c.on_quic() }),
            Out::Deck(In::Values { values, .. }) => self.tx.send(Heard::Values(values)),
            _ => Ok(()),
        };
    }
}

/// a client on the real socket thread; stops when dropped
struct Running {
    heard: Receiver<Heard>,
    give: Sender<DeckValues>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            t.join().unwrap();
        }
    }
}

fn start(url: &str, role: Role, graph: Option<DeckGraph>) -> Running {
    let mut c = Client::new(Options::new(url, role, Some("decks"))).unwrap();
    if let Some(g) = graph {
        c.set_graph(g);
    }
    let (tx, heard) = mpsc::channel();
    let (give_tx, give) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let halt = stop.clone();
    let thread = std::thread::spawn(move || socket::run(c, &mut Test { tx, give }, &halt, None, POLL));
    Running { heard, give: give_tx, stop, thread: Some(thread) }
}

fn graph() -> DeckGraph {
    serde_json::from_value(serde_json::json!({ "nodes": [{ "id": "lfo", "kind": "lfo" }], "links": [] })).unwrap()
}

fn joined(r: &Running) -> (String, bool) {
    let end = Instant::now() + WAIT;
    while let Ok(h) = r.heard.recv_timeout(end.saturating_duration_since(Instant::now())) {
        if let Heard::Joined { client, quic } = h {
            return (client, quic);
        }
    }
    panic!("never joined");
}

fn value(x: f64) -> DeckValues {
    DeckValues { values: HashMap::from([("lfo.rate".to_string(), vec![x])]), mood: None }
}

fn hears(r: &Running, x: f64) {
    let end = Instant::now() + WAIT;
    while let Ok(h) = r.heard.recv_timeout(end.saturating_duration_since(Instant::now())) {
        if matches!(&h, Heard::Values(v) if v.get("lfo.rate") == Some(&vec![x])) {
            return;
        }
    }
    panic!("{x} never arrived");
}

#[test]
fn a_deck_joins_over_quic_and_reaches_a_conductor_on_the_websocket() {
    let (_rt, port, door, hash) = relay();
    let conductor = start(&format!("ws://127.0.0.1:{port}/live/ROOM"), Role::Conductor, None);
    assert!(!joined(&conductor).1, "no ?quic=: only the WebSocket");
    let deck = start(&format!("ws://127.0.0.1:{port}/live/ROOM?quic={door}&quichash={hash}"), Role::Musician, Some(graph()));
    assert!(joined(&deck).1, "the door answered: QUIC");
    deck.give.send(value(0.25)).unwrap();
    hears(&conductor, 0.25);
}

#[test]
fn with_no_door_the_websocket_takes_over_at_once() {
    let (_rt, port, _door, hash) = relay();
    let closed = UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
    let t = Instant::now();
    let deck = start(&format!("ws://127.0.0.1:{port}/live/ROOM?quic={closed}&quichash={hash}"), Role::Musician, Some(graph()));
    assert!(!joined(&deck).1);
    assert!(t.elapsed() < Duration::from_secs(2), "joined after {:?}", t.elapsed());
}

/// UDP between the deck and the door, until `cut`.
fn udp_path(door: SocketAddr, cut: Arc<AtomicBool>) -> SocketAddr {
    let outer = UdpSocket::bind("127.0.0.1:0").unwrap();
    let at = outer.local_addr().unwrap();
    outer.set_read_timeout(Some(Duration::from_millis(1))).unwrap();
    let inner = UdpSocket::bind("127.0.0.1:0").unwrap();
    inner.connect(door).unwrap();
    inner.set_read_timeout(Some(Duration::from_millis(1))).unwrap();
    std::thread::spawn(move || {
        let (mut buf, mut peer) = (vec![0u8; 2048], None);
        for _ in 0..40_000 {
            if let Ok((n, from)) = outer.recv_from(&mut buf) {
                peer = Some(from);
                if !cut.load(Ordering::Relaxed) {
                    let _ = inner.send(&buf[..n]);
                }
            }
            if let (Ok(n), Some(p)) = (inner.recv(&mut buf), peer) {
                if !cut.load(Ordering::Relaxed) {
                    let _ = outer.send_to(&buf[..n], p);
                }
            }
        }
    });
    at
}

#[test]
fn when_udp_dies_mid_show_the_deck_moves_to_the_websocket_as_the_same_client() {
    let (_rt, port, door, hash) = relay();
    let conductor = start(&format!("ws://127.0.0.1:{port}/live/ROOM"), Role::Conductor, None);
    joined(&conductor);
    let cut = Arc::new(AtomicBool::new(false));
    let path = udp_path(door, cut.clone());
    let deck = start(&format!("ws://127.0.0.1:{port}/live/ROOM?quic={path}&quichash={hash}"), Role::Musician, Some(graph()));
    let (id, quic) = joined(&deck);
    assert!(quic);
    deck.give.send(value(0.5)).unwrap();
    hears(&conductor, 0.5);

    cut.store(true, Ordering::Relaxed);
    let t = Instant::now();
    let (again, quic) = joined(&deck);
    assert!(!quic, "on the WebSocket now");
    assert_eq!(again, id, "the same client");
    assert!(t.elapsed() < Duration::from_secs(5), "after {:?}", t.elapsed());
    deck.give.send(value(0.75)).unwrap();
    hears(&conductor, 0.75);
}
