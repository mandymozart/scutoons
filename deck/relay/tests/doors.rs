//! #438: both doors join the same session, and each message takes its lane.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use protocol::{Cue, DeckValues, Envelope, Message, Role};
use relay::doors::{quic, Doors};
use relay::outbox::{Frame, Outbox, MAX_WAIT};
use relay::tls::TlsFiles;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as Ws;

const WAIT: Duration = Duration::from_secs(10);

fn hello(role: &str) -> String {
    format!(r#"{{"v":1,"seq":0,"from":"","kind":"hello","p":{{"code":"ROOM","role":"{role}","group":"decks","token":"t"}}}}"#)
}

const GRAPH: &str = r#"{"v":1,"seq":0,"from":"","kind":"deck.graph","p":{"nodes":[{"id":"lfo","kind":"lfo"}],"links":[]}}"#;

fn values(seq: u64, x: f64) -> String {
    format!(r#"{{"v":1,"seq":{seq},"from":"","kind":"deck.values","p":{{"values":{{"lfo.rate":[{x}]}}}}}}"#)
}

/// The first message `rx` gets that `want` accepts.
async fn until<T>(rx: &mut mpsc::UnboundedReceiver<String>, want: impl Fn(&Envelope) -> Option<T>) -> T {
    tokio::time::timeout(WAIT, async {
        loop {
            let text = rx.recv().await.expect("the connection ended");
            if let Some(t) = serde_json::from_str::<Envelope>(&text).ok().as_ref().and_then(&want) {
                return t;
            }
        }
    })
    .await
    .expect("timed out")
}

fn welcome(e: &Envelope) -> Option<()> {
    matches!(e.message, Message::Welcome(_)).then_some(())
}

async fn relay_ws(relay: Arc<relay::Relay>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(relay::serve(listener, relay));
    format!("ws://{addr}/live/ROOM?role=conductor")
}

/// a WebSocket conductor: what it receives; the sender for what it sends
async fn ws_client(url: &str, role: &str) -> (mpsc::UnboundedSender<String>, mpsc::UnboundedReceiver<String>) {
    let (ws, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    let (mut tx, mut rx) = ws.split();
    tx.send(Ws::Text(hello(role).into())).await.unwrap();
    let (out, mut out_rx) = mpsc::unbounded_channel::<String>();
    let (in_tx, in_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(text) = out_rx.recv().await {
            if tx.send(Ws::Text(text.into())).await.is_err() {
                return;
            }
        }
    });
    tokio::spawn(async move {
        while let Some(Ok(frame)) = rx.next().await {
            if let Ws::Text(t) = frame {
                if in_tx.send(t.to_string()).is_err() {
                    return;
                }
            }
        }
    });
    (out, in_rx)
}

/// the door on a free UDP port under a fresh certificate; its address and the hash a client pins it by
fn open_door(doors: Arc<Doors>) -> (SocketAddr, wtransport::tls::Sha256Digest) {
    static N: AtomicU32 = AtomicU32::new(0);
    let identity = wtransport::Identity::self_signed(["localhost", "127.0.0.1"]).unwrap();
    let hash = identity.certificate_chain().as_slice()[0].hash();
    let dir = std::env::temp_dir().join(format!("relay-doors-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).unwrap();
    let files = TlsFiles { cert: dir.join("cert.pem"), key: dir.join("key.pem") };
    std::fs::write(&files.cert, identity.certificate_chain().as_slice()[0].to_pem()).unwrap();
    std::fs::write(&files.key, identity.private_key().to_secret_pem()).unwrap();
    let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr = udp.local_addr().unwrap();
    drop(udp);
    tokio::spawn(quic::door(doors, addr, files).unwrap());
    (addr, hash)
}

type Lines = tokio::io::Lines<BufReader<wtransport::RecvStream>>;

/// a client through the door, welcomed; `after`: what it says right behind its hello
async fn through_door(addr: SocketAddr, hash: wtransport::tls::Sha256Digest, role: &str, after: &[&str]) -> (wtransport::Endpoint<wtransport::endpoint::endpoint_side::Client>, wtransport::Connection, wtransport::SendStream, Lines) {
    let config = wtransport::ClientConfig::builder().with_bind_default().with_server_certificate_hashes([hash]).build();
    let endpoint = wtransport::Endpoint::client(config).unwrap();
    let conn = endpoint.connect(format!("https://127.0.0.1:{}/live/ROOM?role={role}", addr.port())).await.unwrap();
    let (mut send, recv) = conn.open_bi().await.unwrap().await.unwrap();
    send.write_all(format!("{}\n", hello(role)).as_bytes()).await.unwrap();
    for line in after {
        send.write_all(format!("{line}\n").as_bytes()).await.unwrap();
    }
    let mut lines = BufReader::new(recv).lines();
    tokio::time::timeout(WAIT, async { assert!(lines.next_line().await.unwrap().unwrap().contains("welcome")) }).await.unwrap();
    (endpoint, conn, send, lines)
}

/// A deck through the QUIC door reaches a conductor on WebSocket, in one session.
#[tokio::test(flavor = "multi_thread")]
async fn both_doors_join_the_same_session() {
    let relay = Arc::new(relay::Relay::new());
    let url = relay_ws(relay.clone()).await;
    let (_cond, mut cond_rx) = ws_client(&url, "conductor").await;
    until(&mut cond_rx, welcome).await;
    let (addr, hash) = open_door(Doors::new(relay.clone()));
    let (_endpoint, deck, _send, _lines) = through_door(addr, hash, "musician", &[GRAPH]).await;
    deck.send_datagram(values(1, 0.75).into_bytes()).unwrap();
    until(&mut cond_rx, |e: &Envelope| matches!(&e.message, Message::DeckValues(v) if v.values.get("lfo.rate") == Some(&vec![0.75])).then_some(())).await;
    assert_eq!(relay.with_session("ROOM", |s| s.clients.len()).unwrap(), 2, "one conductor, one deck, one session");
}

/// A datagram cannot carry anything but `deck.values`, and a settle brings what a lost one would have said.
#[tokio::test(flavor = "multi_thread")]
async fn a_cue_on_the_datagram_lane_is_ignored_and_the_settle_repairs() {
    let relay = Arc::new(relay::Relay::new());
    let (addr, hash) = open_door(Doors::new(relay));
    let (_e1, conductor, _csend, mut clines) = through_door(addr, hash.clone(), "conductor", &[]).await;
    let (_e2, deck, _dsend, _dlines) = through_door(addr, hash, "musician", &[GRAPH]).await;

    let cue = r#"{"v":1,"seq":5,"from":"","kind":"cue","p":{"id":"x","actions":[]}}"#;
    conductor.send_datagram(cue.as_bytes()).unwrap();
    // "lost" on the way to the conductor: the reliable lane still brings it within a settle
    deck.send_datagram(values(1, 0.125).into_bytes()).unwrap();
    let settled = tokio::time::timeout(WAIT, async {
        loop {
            let line = clines.next_line().await.unwrap().unwrap();
            assert!(!line.contains(r#""kind":"cue""#), "a cue arrived over the datagram lane");
            if line.contains("deck.values") && line.contains("0.125") {
                return line;
            }
        }
    })
    .await
    .unwrap();
    let e: Envelope = serde_json::from_str(&settled).unwrap();
    assert!(e.to.is_some(), "the settle is addressed to the conductor");
    assert!(e.seq > 0, "and carries a seq to compare with newer datagrams");
}

fn outbox_of(relay: &relay::Relay, role: Role) -> Arc<Outbox> {
    relay.with_session("ROOM", |s| s.clients.values().find(|c| c.role == role).unwrap().outbox.clone()).unwrap()
}

/// a message too big to fit what a stream that is not read will take
fn big() -> Arc<Frame> {
    Frame::ordered("x".repeat(1 << 20).into())
}

fn cue() -> Arc<Frame> {
    Frame::of(Envelope::new("relay", Message::Cue(Cue { id: "late".into(), actions: vec![] })))
}

fn deck_values(x: f64) -> Arc<Frame> {
    Frame::of(Envelope::new("d", Message::DeckValues(DeckValues { values: [("lfo.rate".to_string(), vec![x])].into(), mood: None })))
}

/// A client that stopped reading is let go: the connection ends, it reconnects and resyncs.
#[tokio::test(flavor = "multi_thread")]
async fn a_client_too_far_behind_is_disconnected() {
    let relay = Arc::new(relay::Relay::new());
    let (addr, hash) = open_door(Doors::new(relay.clone()));
    let (_e, conn, _send, _unread) = through_door(addr, hash, "conductor", &[]).await;
    let outbox = outbox_of(&relay, Role::Conductor);
    for _ in 0..16 {
        if !outbox.push(big(), Instant::now()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::timeout(WAIT, conn.closed()).await.expect("still connected, and deaf");
    tokio::time::timeout(WAIT, async {
        while relay.with_session("ROOM", |s| s.clients.len()).unwrap() > 0 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("still in the session");
}

/// What a stalled stream cannot take stays in the outbox, where its limits and "the latest value wins" apply.
#[tokio::test(flavor = "multi_thread")]
async fn a_stalled_stream_leaves_its_backlog_in_the_outbox() {
    let relay = Arc::new(relay::Relay::new());
    let (addr, hash) = open_door(Doors::new(relay.clone()));
    let (_e, _conn, _send, _unread) = through_door(addr, hash, "conductor", &[]).await;
    let outbox = outbox_of(&relay, Role::Conductor);
    for _ in 0..6 {
        assert!(outbox.push(big(), Instant::now()));
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;

    let t = Instant::now();
    let skipped = outbox.skipped();
    assert!(outbox.push(deck_values(0.1), t));
    assert!(outbox.push(deck_values(0.2), t));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(outbox.skipped(), skipped + 1, "the newer value replaced the waiting one");
    assert!(outbox.push(cue(), t));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!outbox.push(cue(), t + MAX_WAIT), "a backlog older than MAX_WAIT drops the client");
}

/// One settle task per session, however quickly its members come and go.
#[tokio::test]
async fn a_quick_return_does_not_start_a_second_settle() {
    let doors = Doors::new(Arc::new(relay::Relay::new()));
    let admit = || doors.admit(&hello("conductor"), relay::Join::default()).ok().expect("admitted");
    let first = admit();
    first.leave();
    let _second = admit();
    assert_eq!(tokio::runtime::Handle::current().metrics().num_alive_tasks(), 1);
}

/// A connection that never opens its stream is let go, keep-alives or not.
#[tokio::test(flavor = "multi_thread")]
async fn a_connection_without_a_stream_is_let_go() {
    let relay = Arc::new(relay::Relay::new());
    let (addr, hash) = open_door(Doors::new(relay));
    let config = wtransport::ClientConfig::builder().with_bind_default().with_server_certificate_hashes([hash]).keep_alive_interval(Some(Duration::from_secs(1))).build();
    let endpoint = wtransport::Endpoint::client(config).unwrap();
    let conn = endpoint.connect(format!("https://127.0.0.1:{}/live/ROOM?role=conductor", addr.port())).await.unwrap();
    tokio::time::timeout(WAIT, conn.closed()).await.expect("held open without a hello");
}

/// A settle delayed on the stream does not take back what a newer datagram already said, and still brings the rest.
#[tokio::test(flavor = "multi_thread")]
async fn a_late_settle_does_not_overwrite_a_newer_datagram() {
    let relay = Arc::new(relay::Relay::new());
    let url = relay_ws(relay.clone()).await;
    let (_cond, mut cond_rx) = ws_client(&url, "conductor").await;
    until(&mut cond_rx, welcome).await;
    let (addr, hash) = open_door(Doors::new(relay));
    let (_e, deck, mut send, _lines) = through_door(addr, hash, "musician", &[GRAPH]).await;
    deck.send_datagram(values(5, 0.9).into_bytes()).unwrap();
    until(&mut cond_rx, |e: &Envelope| matches!(&e.message, Message::DeckValues(v) if v.values.get("lfo.rate") == Some(&vec![0.9])).then_some(())).await;

    let settle = r#"{"v":1,"seq":3,"from":"","kind":"deck.values","p":{"values":{"lfo.rate":[0.1],"lfo.depth":[0.2]}}}"#;
    send.write_all(format!("{settle}\n").as_bytes()).await.unwrap();
    let rate = until(&mut cond_rx, |e: &Envelope| match &e.message {
        Message::DeckValues(v) if v.values.contains_key("lfo.depth") => Some(v.values.get("lfo.rate").cloned()),
        _ => None,
    })
    .await;
    assert_eq!(rate, None, "the settle's older lfo.rate is left out");
}
