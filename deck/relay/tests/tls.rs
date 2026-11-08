//! wss in the venue (#201, src/tls.rs): the relay with a self-signed certificate on a free port – a client that trusts
//! it joins over wss (welcome, presence, the keep-alive ping), a deck on the same port keeps plain ws, `/status`
//! answers over https, and new certificate files are used from the next handshake on (a mismatched pair is not).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use protocol::{Envelope, Hello, Message, Role};
use relay::tls::TlsFiles;
use rustls::pki_types::{CertificateDer, ServerName};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::Message as Ws;
use tokio_tungstenite::WebSocketStream;

/// a fresh folder for one test's PEM files
fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("relay-tls-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A self-signed certificate for `names`, written as fullchain.pem + privkey.pem (what deck-tls fetches); its DER for
/// the client's root store.
fn certificate(dir: &Path, names: &[&str]) -> (TlsFiles, CertificateDer<'static>) {
    let rcgen::CertifiedKey { cert, key_pair } = rcgen::generate_simple_self_signed(names.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>()).unwrap();
    let files = TlsFiles { cert: dir.join("fullchain.pem"), key: dir.join("privkey.pem") };
    std::fs::write(&files.cert, cert.pem()).unwrap();
    std::fs::write(&files.key, key_pair.serialize_pem()).unwrap();
    (files, cert.der().clone())
}

async fn start(files: TlsFiles) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let tls = relay::tls::acceptor(files).unwrap();
    tokio::spawn(relay::serve_with(listener, Arc::new(relay::Relay::new()), Some(tls)));
    port
}

/// TLS to 127.0.0.1:`port` as `name`, trusting only `root`
async fn tls_to(port: u16, name: &str, root: &CertificateDer<'static>) -> std::io::Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(root.clone()).unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let tcp = TcpStream::connect(("127.0.0.1", port)).await?;
    TlsConnector::from(Arc::new(config)).connect(ServerName::try_from(name.to_owned()).unwrap(), tcp).await
}

/// The next text message (the keep-alive pings skipped).
async fn next<S: AsyncRead + AsyncWrite + Unpin>(ws: &mut WebSocketStream<S>) -> Envelope {
    loop {
        let frame = tokio::time::timeout(Duration::from_secs(3), ws.next()).await.expect("a message").unwrap().unwrap();
        if let Ws::Text(text) = frame {
            return serde_json::from_str(&text).unwrap();
        }
    }
}

async fn hello<S: AsyncRead + AsyncWrite + Unpin>(ws: &mut WebSocketStream<S>, role: Role, token: Option<&str>) {
    let hello = Hello { code: "LAN".into(), role, group: None, nick: None, token: token.map(str::to_owned), resume: None };
    ws.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Hello(hello))).unwrap().into())).await.unwrap();
}

#[tokio::test]
async fn a_phone_joins_over_wss_and_a_deck_keeps_ws_on_the_same_port() {
    let d = dir("join");
    let (files, root) = certificate(&d, &["x1.lan.scutoons.test"]);
    let port = start(files).await;

    let tls = tls_to(port, "x1.lan.scutoons.test", &root).await.expect("the certificate is trusted for its name");
    let (mut phone, _) = tokio_tungstenite::client_async(format!("wss://x1.lan.scutoons.test:{port}/live/LAN?role=conductor"), tls).await.unwrap();
    hello(&mut phone, Role::Conductor, Some("dev-token")).await;
    let welcome = next(&mut phone).await;
    assert!(matches!(welcome.message, Message::Welcome(_)), "welcome over wss: {welcome:?}");
    assert!(matches!(next(&mut phone).await.message, Message::Presence(_)), "presence after the own join");

    // a deck on the LAN: plain ws, same port
    let (mut deck, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/live/LAN?role=output&group=nave")).await.unwrap();
    hello(&mut deck, Role::Output, None).await;
    assert!(matches!(next(&mut deck).await.message, Message::Welcome(_)), "welcome over ws");
    let Message::Presence(p) = next(&mut phone).await.message else { panic!("presence for the deck's join") };
    assert_eq!(p.groups.get("nave"), Some(&1), "the phone sees the deck: {p:?}");
    std::fs::remove_dir_all(d).ok();
}

#[tokio::test]
async fn the_keep_alive_ping_reaches_a_wss_client() {
    let d = dir("ping");
    let (files, root) = certificate(&d, &["localhost"]);
    let port = start(files).await;
    let tls = tls_to(port, "localhost", &root).await.unwrap();
    let (mut ws, _) = tokio_tungstenite::client_async(format!("wss://localhost:{port}/live/LAN"), tls).await.unwrap();
    hello(&mut ws, Role::Output, None).await;
    let deadline = tokio::time::Instant::now() + relay::PING_EVERY + Duration::from_secs(2);
    loop {
        let frame = tokio::time::timeout_at(deadline, ws.next()).await.expect("a ping within PING_EVERY").unwrap().unwrap();
        if matches!(frame, Ws::Ping(_)) {
            break;
        }
    }
    std::fs::remove_dir_all(d).ok();
}

async fn status<S: AsyncRead + AsyncWrite + Unpin>(mut s: S) -> String {
    s.write_all(b"GET /status HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").await.unwrap();
    let mut body = String::new();
    // a TLS peer may close without close_notify after the response – what was read counts
    let _ = tokio::time::timeout(Duration::from_secs(3), s.read_to_string(&mut body)).await.unwrap();
    body
}

#[tokio::test]
async fn status_over_https_and_http() {
    let d = dir("status");
    let (files, root) = certificate(&d, &["localhost"]);
    let port = start(files).await;
    let https = status(tls_to(port, "localhost", &root).await.unwrap()).await;
    assert!(https.starts_with("HTTP/1.1 200 OK") && https.ends_with(r#"{"sessions":0,"live":0}"#), "{https}");
    let http = status(TcpStream::connect(("127.0.0.1", port)).await.unwrap()).await;
    assert!(http.ends_with(r#"{"sessions":0,"live":0}"#), "{http}");
    std::fs::remove_dir_all(d).ok();
}

#[tokio::test]
async fn a_renewed_certificate_is_used_without_a_restart() {
    let d = dir("renew");
    let (files, old) = certificate(&d, &["localhost"]);
    let port = start(files.clone()).await;
    assert!(tls_to(port, "localhost", &old).await.is_ok());

    // half a renewal (a new chain, the old key still there): the pair does not match – the old certificate stays
    let other = dir("renew-other");
    let (new_files, new) = certificate(&other, &["localhost", "x1.lan.scutoons.test"]);
    std::fs::write(&files.cert, std::fs::read(&new_files.cert).unwrap()).unwrap();
    assert!(tls_to(port, "localhost", &old).await.is_ok(), "a mismatched pair keeps the old certificate");

    // the key follows: the next handshake has the new certificate
    std::fs::write(&files.key, std::fs::read(&new_files.key).unwrap()).unwrap();
    assert!(tls_to(port, "x1.lan.scutoons.test", &new).await.is_ok(), "the renewed certificate, no restart");
    assert!(tls_to(port, "localhost", &old).await.is_err(), "the old one is gone");
    std::fs::remove_dir_all(d).ok();
    std::fs::remove_dir_all(other).ok();
}

#[test]
fn a_broken_pair_stops_the_start() {
    let d = dir("broken");
    let (files, _) = certificate(&d, &["localhost"]);
    let (other, _) = certificate(&dir("broken-other"), &["localhost"]);
    let mismatched = TlsFiles { cert: files.cert.clone(), key: other.key.clone() };
    assert!(relay::tls::acceptor(mismatched).is_err(), "the key is not the certificate's");
    let missing = TlsFiles { cert: d.join("nope.pem"), key: files.key.clone() };
    assert!(relay::tls::acceptor(missing).err().unwrap().contains("nope.pem"));
    std::fs::remove_dir_all(d).ok();
    std::fs::remove_dir_all(dir("broken-other")).ok();
}
