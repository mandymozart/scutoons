//! The hosted relay fetches its certificate from the API (#441, src/cert.rs): a fake API on a local port, real files, and a
//! real wss handshake that has to show the renewed certificate.

use std::io::{Read, Write};
use std::net::TcpListener as StdListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use relay::cert::{self, Source};
use relay::tls::TlsFiles;
use rustls::pki_types::{CertificateDer, ServerName};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsConnector;

const KEY: &str = "the-relay-key";

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("relay-cert-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn files(d: &std::path::Path) -> TlsFiles {
    TlsFiles { cert: d.join("fullchain.pem"), key: d.join("privkey.pem") }
}

/// (chain + key as the API sends them, the certificate)
fn pair(name: &str) -> (String, CertificateDer<'static>) {
    let c = rcgen::generate_simple_self_signed(vec![name.to_owned()]).unwrap();
    (format!("{}\n{}", c.cert.pem(), c.key_pair.serialize_pem()), c.cert.der().clone())
}

/// A fake API: answers every request with what `body` holds now, 403 for the wrong key.
fn api(body: Arc<Mutex<String>>) -> Source {
    let listener = StdListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/api/relay/live-cert", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let (mut seen, mut chunk) = (Vec::new(), [0u8; 1024]);
            while !seen.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut chunk) {
                    Ok(n) if n > 0 => seen.extend_from_slice(&chunk[..n]),
                    _ => break,
                }
            }
            let asked = String::from_utf8_lossy(&seen).to_lowercase();
            let (status, text) = if asked.contains(&format!("x-relay-key: {KEY}")) {
                ("200 OK", body.lock().unwrap().clone())
            } else {
                ("403 Forbidden", "{\"error\":\"forbidden\"}".to_owned())
            };
            let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
        }
    });
    Source { url, key: KEY.to_owned() }
}

async fn presented(port: u16, name: &str, root: &CertificateDer<'static>) -> std::io::Result<()> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(root.clone()).unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let tcp = TcpStream::connect(("127.0.0.1", port)).await?;
    TlsConnector::from(Arc::new(config)).connect(ServerName::try_from(name.to_owned()).unwrap(), tcp).await.map(|_| ())
}

#[tokio::test]
async fn the_relay_fetches_its_certificate_and_a_renewed_one_is_used_without_a_restart() {
    let d = dir("renew");
    let files = files(&d);
    let (first, first_der) = pair("live.test");
    let body = Arc::new(Mutex::new(first));
    let source = api(body.clone());

    assert!(cert::refresh(source.clone(), files.clone()).await.unwrap(), "no files yet: installed");
    assert!(!cert::refresh(source.clone(), files.clone()).await.unwrap(), "the same pair: nothing to do");

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let tls = relay::tls::acceptor(files.clone()).unwrap();
    tokio::spawn(relay::serve_with(listener, Arc::new(relay::Relay::new()), Some(tls)));
    presented(port, "live.test", &first_der).await.unwrap();

    let (second, second_der) = pair("live.test");
    *body.lock().unwrap() = second;
    tokio::time::sleep(Duration::from_millis(20)).await; // a new modification time on coarse clocks
    assert!(cert::refresh(source, files).await.unwrap());
    presented(port, "live.test", &second_der).await.expect("the renewed certificate on the next handshake");
    assert!(presented(port, "live.test", &first_der).await.is_err(), "and not the old one");
}

#[tokio::test]
async fn a_wrong_key_gets_no_certificate() {
    let d = dir("key");
    let files = files(&d);
    let (pem, _) = pair("live.test");
    let mut source = api(Arc::new(Mutex::new(pem)));
    source.key = "wrong".to_owned();
    let e = cert::refresh(source, files.clone()).await.unwrap_err();
    assert!(e.contains("403"), "{e}");
    assert!(!e.contains("wrong"), "the key stays out of the message: {e}");
    assert!(!files.cert.exists());
}

#[tokio::test]
async fn a_broken_pair_leaves_the_good_one_alone() {
    let d = dir("broken");
    let files = files(&d);
    let (good, _) = pair("live.test");
    assert!(cert::install(&good, &files).unwrap());
    let before = std::fs::read_to_string(&files.cert).unwrap();

    let (other, _) = pair("other.test");
    let (chain, _) = cert::split(&good).unwrap();
    let (_, foreign_key) = cert::split(&other).unwrap();
    assert!(cert::install(&format!("{chain}{foreign_key}"), &files).is_err());

    assert_eq!(std::fs::read_to_string(&files.cert).unwrap(), before);
    assert!(relay::tls::load(&files, &rustls::crypto::ring::default_provider()).is_ok());
    assert!(!d.join("fullchain.pem.new").exists() && !d.join("privkey.pem.new").exists(), "nothing left behind");
}
