//! #438: both doors join the same session, and each message takes its lane.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use protocol::{Envelope, Message};
use relay::doors::{quic, Doors};
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
    tx.send(Ws::Text(hello(role))).await.unwrap();
    let (out, mut out_rx) = mpsc::unbounded_channel::<String>();
    let (in_tx, in_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(text) = out_rx.recv().await {
            if tx.send(Ws::Text(text)).await.is_err() {
                return;
            }
        }
    });
    tokio::spawn(async move {
        while let Some(Ok(frame)) = rx.next().await {
            if let Ws::Text(t) = frame {
                if in_tx.send(t).is_err() {
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
    conductor.send_datagram(cue.as_bytes().to_vec()).unwrap();
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
