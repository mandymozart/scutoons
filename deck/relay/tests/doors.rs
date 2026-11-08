//! #438 (feature `doors`): every transport joins the same session, and each message takes its lane.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use protocol::{Envelope, Message};
use relay::doors::{quic, renet, Doors};
use relay::outbox::Lane;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as Ws;

const WAIT: Duration = Duration::from_secs(10);
const KEY: [u8; 32] = [7; 32];
const RESEND: Duration = Duration::from_millis(40);

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

fn identity() -> wtransport::Identity {
    wtransport::Identity::self_signed(["localhost", "127.0.0.1"]).unwrap()
}

fn der(identity: &wtransport::Identity) -> (quinn::rustls::pki_types::CertificateDer<'static>, quinn::rustls::pki_types::PrivateKeyDer<'static>) {
    let chain = identity.certificate_chain().as_slice()[0].der().to_vec();
    let key = identity.private_key().secret_der().to_vec();
    (chain.into(), quinn::rustls::pki_types::PrivateKeyDer::Pkcs8(key.into()))
}

/// A deck on raw QUIC, a deck on netcode UDP and a deck on WebTransport reach one conductor on WebSocket, in one session.
#[tokio::test(flavor = "multi_thread")]
async fn every_door_joins_the_same_session() {
    let relay = Arc::new(relay::Relay::new());
    let doors = Doors::new(relay.clone());
    let url = relay_ws(relay.clone()).await;
    let (_cond, mut cond_rx) = ws_client(&url, "conductor").await;
    until(&mut cond_rx, welcome).await;

    // quinn
    let identity = identity();
    let (chain, key) = der(&identity);
    let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let quinn_addr = udp.local_addr().unwrap();
    drop(udp);
    tokio::spawn(quic::quinn_door(doors.clone(), quinn_addr, vec![chain.clone()], key).unwrap());
    let (conn, mut send, recv) = quic::quinn_client(quinn_addr, chain).await.unwrap();
    for line in [hello("musician"), GRAPH.to_owned()] {
        send.write_all(format!("{line}\n").as_bytes()).await.unwrap();
    }
    let mut lines = BufReader::new(recv).lines();
    tokio::time::timeout(WAIT, async { assert!(lines.next_line().await.unwrap().unwrap().contains("welcome")) }).await.unwrap();
    conn.send_datagram(values(1, 0.25).into_bytes().into()).unwrap();
    let got = |x: f64| move |e: &Envelope| matches!(&e.message, Message::DeckValues(v) if v.values.get("lfo.rate") == Some(&vec![x])).then_some(());
    until(&mut cond_rx, got(0.25)).await;

    // renet2 (netcode over UDP)
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let renet_addr = socket.local_addr().unwrap();
    tokio::spawn(renet::netcode_door(doors.clone(), socket, vec![], KEY, RESEND));
    let (out, out_rx) = mpsc::unbounded_channel();
    let (in_tx, mut in_rx) = mpsc::unbounded_channel();
    tokio::spawn(renet::client(renet_addr, KEY, 42, RESEND, out_rx, in_tx));
    out.send((Lane::Reliable, hello("musician"))).unwrap();
    out.send((Lane::Reliable, GRAPH.to_owned())).unwrap();
    until(&mut in_rx, welcome).await;
    out.send((Lane::Unreliable, values(1, 0.5))).unwrap();
    until(&mut cond_rx, got(0.5)).await;

    // WebTransport, on the same wire as a browser
    let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let wt_addr = udp.local_addr().unwrap();
    drop(udp);
    let hash = identity.certificate_chain().as_slice()[0].hash();
    tokio::spawn(quic::wt_door(doors.clone(), wt_addr, identity.clone_identity()).unwrap());
    let config = wtransport::ClientConfig::builder().with_bind_default().with_server_certificate_hashes([hash]).build();
    let endpoint = wtransport::Endpoint::client(config).unwrap();
    let wt = endpoint.connect(format!("https://127.0.0.1:{}/live/ROOM?role=musician", wt_addr.port())).await.unwrap();
    let (mut send, recv) = wt.open_bi().await.unwrap().await.unwrap();
    for line in [hello("musician"), GRAPH.to_owned()] {
        send.write_all(format!("{line}\n").as_bytes()).await.unwrap();
    }
    let mut lines = BufReader::new(recv).lines();
    tokio::time::timeout(WAIT, async { assert!(lines.next_line().await.unwrap().unwrap().contains("welcome")) }).await.unwrap();
    wt.send_datagram(values(1, 0.75).into_bytes()).unwrap();
    until(&mut cond_rx, got(0.75)).await;

    let open = relay.with_session("ROOM", |s| s.clients.len()).unwrap();
    assert_eq!(open, 4, "one conductor + three decks, one session");
}

/// A datagram cannot carry anything but `deck.values`, and a settle brings what a lost one would have said.
#[tokio::test(flavor = "multi_thread")]
async fn a_cue_on_the_datagram_lane_is_ignored_and_the_settle_repairs() {
    let relay = Arc::new(relay::Relay::new());
    let doors = Doors::new(relay.clone());
    let identity = identity();
    let (chain, key) = der(&identity);
    let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr = udp.local_addr().unwrap();
    drop(udp);
    tokio::spawn(quic::quinn_door(doors.clone(), addr, vec![chain.clone()], key).unwrap());

    let (conductor, mut csend, crecv) = quic::quinn_client(addr, chain.clone()).await.unwrap();
    csend.write_all(format!("{}\n", hello("conductor")).as_bytes()).await.unwrap();
    let mut clines = BufReader::new(crecv).lines();
    tokio::time::timeout(WAIT, async { assert!(clines.next_line().await.unwrap().unwrap().contains("welcome")) }).await.unwrap();

    let (deck, mut dsend, drecv) = quic::quinn_client(addr, chain).await.unwrap();
    for line in [hello("musician"), GRAPH.to_owned()] {
        dsend.write_all(format!("{line}\n").as_bytes()).await.unwrap();
    }
    let mut dlines = BufReader::new(drecv).lines();
    tokio::time::timeout(WAIT, async { assert!(dlines.next_line().await.unwrap().unwrap().contains("welcome")) }).await.unwrap();

    let cue = r#"{"v":1,"seq":5,"from":"","kind":"cue","p":{"id":"x","actions":[]}}"#;
    conductor.send_datagram(cue.as_bytes().to_vec().into()).unwrap();
    // "lost" on the way to the conductor: the relay folded it, the conductor's datagram buffer never saw it – the reliable
    // lane still brings it within a settle
    deck.send_datagram(values(1, 0.125).into_bytes().into()).unwrap();
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

/// One netcode server can hold a UDP socket and a WebTransport socket at once (the browser side is renet2's wasm client,
/// which only exists for wasm32; here its server socket is opened next to a native client that connects).
#[tokio::test(flavor = "multi_thread")]
async fn renet2_runs_udp_and_webtransport_sockets_on_one_server() {
    use renet2_netcode::{BoxedSocket, WebTransportServer, WebTransportServerConfig};
    let relay = Arc::new(relay::Relay::new());
    let doors = Doors::new(relay.clone());
    let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let wt_addr = udp.local_addr().unwrap();
    drop(udp);
    let (config, _hash) = WebTransportServerConfig::new_selfsigned(wt_addr, 64).unwrap();
    // renet2's constructor blocks on the runtime it is given: never on a runtime thread
    let handle = tokio::runtime::Handle::current();
    let wt = tokio::task::spawn_blocking(move || WebTransportServer::new(config, handle)).await.unwrap().unwrap();
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let addr = socket.local_addr().unwrap();
    tokio::spawn(renet::netcode_door(doors, socket, vec![(BoxedSocket::new(wt), wt_addr)], KEY, RESEND));
    let (out, out_rx) = mpsc::unbounded_channel();
    let (in_tx, mut in_rx) = mpsc::unbounded_channel();
    tokio::spawn(renet::client(addr, KEY, 7, RESEND, out_rx, in_tx));
    out.send((Lane::Reliable, hello("musician"))).unwrap();
    until(&mut in_rx, welcome).await;
    assert_eq!(relay.status().0, 1);
}
