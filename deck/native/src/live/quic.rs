//! The relay client's QUIC connection on a thread of its own (#438): the same door a browser opens as WebTransport
//! (`wtransport`). deck-space relay.rs decides when to dial and what goes on which lane; this adapter only moves bytes:
//! one bidirectional stream, datagrams, a current-thread tokio runtime that ends with the connection.
use std::time::Duration;
use tokio::sync::mpsc;
use wtransport::config::IpBindConfig;
use wtransport::tls::Sha256Digest;
use wtransport::{ClientConfig, Endpoint};

/// a silent connection ends after this, as at the relay's door (deck/relay/src/doors/mod.rs `DOOR_IDLE`)
const IDLE: Duration = Duration::from_secs(3);
const KEEP_ALIVE: Duration = Duration::from_secs(1);
/// how long a close may take to reach the relay before the socket goes
const CLOSING: Duration = Duration::from_millis(100);

/// What the connection tells the client's thread.
pub enum Event {
    Opened,
    Stream(Vec<u8>),
    Datagram(Vec<u8>),
    Closed(String),
}

enum Say {
    Stream(String),
    Datagram(String),
}

/// A connection (being) dialled; dropping it closes it.
pub struct Handle {
    tx: mpsc::UnboundedSender<Say>,
}

impl Handle {
    pub fn stream(&self, text: String) {
        let _ = self.tx.send(Say::Stream(text));
    }

    pub fn datagram(&self, text: String) {
        let _ = self.tx.send(Say::Datagram(text));
    }
}

/// Dial `url` (`https://host:port/live/<code>?…`); `hash` pins a certificate (SHA-256, hex) instead of the webpki roots.
pub fn dial(url: String, hash: Option<String>, on: impl Fn(Event) + Send + 'static) -> Handle {
    let (tx, rx) = mpsc::unbounded_channel();
    let thread = std::thread::Builder::new().name("relay-quic".into()).spawn(move || {
        let why = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt.block_on(run(url, hash, rx, &on)).err().unwrap_or_default(),
            Err(e) => e.to_string(),
        };
        on(Event::Closed(why));
    });
    if let Err(e) = thread {
        tracing::warn!("relay: no QUIC thread: {e}");
    }
    Handle { tx }
}

fn pin(hash: &str) -> Result<Sha256Digest, String> {
    let bytes: Vec<u8> = (0..hash.len()).step_by(2).filter_map(|i| hash.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok())).collect();
    <[u8; 32]>::try_from(bytes).map(Sha256Digest::new).map_err(|_| format!("quichash '{hash}': not a SHA-256 in hex"))
}

fn config(hash: Option<&str>) -> Result<ClientConfig, String> {
    // IPv4: bunny's Anycast and a venue's LAN are; a host that has only IPv6 stays on the WebSocket
    let builder = ClientConfig::builder().with_bind_config(IpBindConfig::InAddrAnyV4);
    let builder = match hash {
        Some(h) => builder.with_server_certificate_hashes([pin(h)?]),
        None => {
            let roots = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
            let mut tls = rustls::ClientConfig::builder_with_provider(rustls::crypto::ring::default_provider().into())
                .with_protocol_versions(&[&rustls::version::TLS13])
                .map_err(|e| e.to_string())?
                .with_root_certificates(roots)
                .with_no_client_auth();
            tls.alpn_protocols = vec![b"h3".to_vec()];
            builder.with_custom_tls(tls)
        }
    };
    Ok(builder.max_idle_timeout(Some(IDLE)).map_err(|e| e.to_string())?.keep_alive_interval(Some(KEEP_ALIVE)).build())
}

/// The connection until it ends (Ok: the handle was dropped).
async fn run(url: String, hash: Option<String>, mut rx: mpsc::UnboundedReceiver<Say>, on: &(impl Fn(Event) + Send)) -> Result<(), String> {
    let endpoint = Endpoint::client(config(hash.as_deref())?).map_err(|e| e.to_string())?;
    let conn = endpoint.connect(&url).await.map_err(|e| e.to_string())?;
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?.await.map_err(|e| e.to_string())?;
    on(Event::Opened);
    // the stream on its own task: a stalled stream must not hold up the datagrams
    let (stream, mut lines) = mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        while let Some(text) = lines.recv().await {
            if send.write_all(text.as_bytes()).await.is_err() {
                return;
            }
        }
    });
    let mut chunk = vec![0u8; 16 * 1024];
    loop {
        tokio::select! {
            say = rx.recv() => match say {
                Some(Say::Stream(text)) => { let _ = stream.send(text); }
                // too large or refused: the core only sends what fits, and a lost value is allowed
                Some(Say::Datagram(text)) => { let _ = conn.send_datagram(text.into_bytes()); }
                None => {
                    conn.close(0u32.into(), b"");
                    let _ = tokio::time::timeout(CLOSING, endpoint.wait_idle()).await;
                    return Ok(());
                }
            },
            got = recv.read(&mut chunk) => match got {
                Ok(Some(n)) => on(Event::Stream(chunk[..n].to_vec())),
                Ok(None) => return Err("the stream ended".into()),
                Err(e) => return Err(e.to_string()),
            },
            d = conn.receive_datagram() => match d {
                Ok(d) => on(Event::Datagram(d.payload().to_vec())),
                Err(e) => return Err(e.to_string()),
            },
        }
    }
}
