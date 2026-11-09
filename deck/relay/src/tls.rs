//! wss for the relay in the venue (#201, LIVE.md B.4): a phone on https://scutoons.com/live/deck may not open
//! `ws://192.168.…`, so a deck's relay answers under a name of ours (`<lan-ip-with-dashes>.lan.scutoons.com`, bunny DNS) with the wildcard
//! certificate. rustls (ring), no OpenSSL. The hosted relay has one too, for `live.scutoons.com`, which it fetches itself (cert.rs, #441).
//!
//! - `relay [addr] --tls-cert FILE --tls-key FILE` (or `RELAY_TLS_CERT`, `RELAY_TLS_KEY`): PEM files, the full chain and
//!   its private key (PKCS#8, PKCS#1 or SEC1 – what acme.sh / Let's Encrypt write).
//! - With a certificate the port speaks both: a connection that starts with a TLS handshake record (0x16) gets TLS, an
//!   HTTP request (a letter) stays plain ws – the decks on the LAN keep `ws://<ip>:7700`, phones use
//!   `wss://<name>:7700`, one port (lib.rs `serve_with`).
//! - Renewal without a restart: every TLS handshake compares the two files' modification time and size with the ones
//!   loaded; changed → loaded again. A pair that does not load (written half, the key not yet the cert's) keeps the old
//!   certificate, logs a warning and is tried again on the next handshake. No timer, no file watcher (the dogma: no
//!   polling) – two `stat`s per handshake.
//! - A certificate also opens the QUIC door (#438), on the same port number over UDP, unless `--no-quic` or
//!   `RELAY_QUIC_ADDR=off` says not to; `--quic ADDR` puts it on another address.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use rustls::crypto::CryptoProvider;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use tokio_rustls::TlsAcceptor;

/// The certificate's two PEM files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsFiles {
    /// the full chain (the server's certificate first)
    pub cert: PathBuf,
    pub key: PathBuf,
}

const USAGE: &str = "relay [addr] [--tls-cert FILE --tls-key FILE] [--quic ADDR | --no-quic]";

/// What `relay` listens on (`USAGE`), the env as the fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listen {
    pub addr: String,
    pub tls: Option<TlsFiles>,
    /// #438: the QUIC door's UDP address (`addr` when there is a certificate, or `--quic` / `RELAY_QUIC_ADDR`); none: no UDP
    /// port is opened at all
    pub quic: Option<String>,
    /// #441: fetch the certificate from here (cert.rs); `tls` then says where it goes
    pub cert_source: Option<crate::cert::Source>,
}

/// #441: where a fetched pair goes when nothing else is named: the folder the hosted image makes for it (Dockerfile)
const FETCHED_CERT: &str = "/certs/fullchain.pem";
const FETCHED_KEY: &str = "/certs/privkey.pem";

impl Listen {
    /// `args` without the program name; `env` = `std::env::var(..).ok()` (a closure, so the tests need no real env).
    /// Empty values count as unset; a cert without a key (or the reverse) is an error, not a silent plain relay.
    pub fn parse(args: impl IntoIterator<Item = String>, env: impl Fn(&str) -> Option<String>) -> Result<Listen, String> {
        let (mut addr, mut cert, mut key, mut quic, mut no_quic) = (None, None, None, None, false);
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--tls-cert" => cert = Some(args.next().ok_or("--tls-cert needs a file")?),
                "--tls-key" => key = Some(args.next().ok_or("--tls-key needs a file")?),
                "--quic" => quic = Some(args.next().ok_or("--quic needs an address")?),
                "--no-quic" => no_quic = true,
                a if a.starts_with("--") => return Err(format!("unknown option {a} ({USAGE})")),
                _ if addr.is_none() => addr = Some(arg),
                _ => return Err(format!("one address only, got {arg} too")),
            }
        }
        let set = |v: Option<String>| v.filter(|s| !s.trim().is_empty());
        let addr = set(addr).or_else(|| set(env("RELAY_ADDR"))).unwrap_or_else(|| "0.0.0.0:7700".to_owned());
        let cert = set(cert).or_else(|| set(env("RELAY_TLS_CERT")));
        let key = set(key).or_else(|| set(env("RELAY_TLS_KEY")));
        let quic = set(quic).or_else(|| set(env("RELAY_QUIC_ADDR")));
        let cert_source = match (set(env("RELAY_CERT_URL")), set(env("RELAY_CERT_KEY"))) {
            (Some(url), Some(key)) => Some(crate::cert::Source { url, key }),
            (None, _) => None,
            (Some(_), None) => return Err("RELAY_CERT_URL without RELAY_CERT_KEY".into()),
        };
        let tls = match (cert, key) {
            (Some(cert), Some(key)) => Some(TlsFiles { cert: cert.into(), key: key.into() }),
            (None, None) if cert_source.is_some() => Some(TlsFiles { cert: FETCHED_CERT.into(), key: FETCHED_KEY.into() }),
            (None, None) => None,
            (Some(_), None) => return Err("a certificate without its key (--tls-key / RELAY_TLS_KEY)".into()),
            (None, Some(_)) => return Err("a key without its certificate (--tls-cert / RELAY_TLS_CERT)".into()),
        };
        let off = no_quic || quic.as_deref() == Some("off");
        if !off && quic.is_some() && tls.is_none() {
            return Err("the QUIC door needs a certificate (--tls-cert and --tls-key)".into());
        }
        let quic = if off { None } else { quic.or_else(|| tls.as_ref().map(|_| addr.clone())) };
        Ok(Listen { addr, tls, quic, cert_source })
    }
}

/// modification time + size of both files – what "changed on disk" means
type Stamp = Option<(SystemTime, u64, SystemTime, u64)>;

fn stamp(files: &TlsFiles) -> Stamp {
    let (c, k) = (std::fs::metadata(&files.cert).ok()?, std::fs::metadata(&files.key).ok()?);
    Some((c.modified().ok()?, c.len(), k.modified().ok()?, k.len()))
}

/// Read the pair: the chain, the key, and the check that the key is the certificate's.
pub fn load(files: &TlsFiles, provider: &CryptoProvider) -> Result<CertifiedKey, String> {
    let cert = files.cert.display();
    let chain = CertificateDer::pem_file_iter(&files.cert)
        .map_err(|e| format!("{cert}: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{cert}: {e}"))?;
    if chain.is_empty() {
        return Err(format!("{cert}: no certificate in it"));
    }
    let key = PrivateKeyDer::from_pem_file(&files.key).map_err(|e| format!("{}: {e}", files.key.display()))?;
    CertifiedKey::from_der(chain, key, provider).map_err(|e| format!("{cert} + {}: {e}", files.key.display()))
}

/// The certificate rustls hands out, loaded again when its files change (module doc).
#[derive(Debug)]
pub struct Reloading {
    files: TlsFiles,
    provider: Arc<CryptoProvider>,
    loaded: Mutex<(Stamp, Arc<CertifiedKey>)>,
}

impl Reloading {
    pub fn new(files: TlsFiles, provider: Arc<CryptoProvider>) -> Result<Reloading, String> {
        let at = stamp(&files);
        let key = load(&files, &provider)?;
        Ok(Reloading { files, provider, loaded: Mutex::new((at, Arc::new(key))) })
    }

    /// The certificate for the next handshake – the files again if they changed since.
    pub fn current(&self) -> Arc<CertifiedKey> {
        let mut loaded = self.loaded.lock().unwrap();
        let now = stamp(&self.files);
        if now.is_some() && now != loaded.0 {
            match load(&self.files, &self.provider) {
                Ok(key) => {
                    *loaded = (now, Arc::new(key));
                    tracing::info!(cert = %self.files.cert.display(), "certificate reloaded");
                }
                Err(e) => tracing::warn!("certificate changed but does not load – keeping the old one: {e}"),
            }
        }
        loaded.1.clone()
    }
}

impl ResolvesServerCert for Reloading {
    fn resolve(&self, _hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(self.current())
    }
}

/// The TLS side of `serve_with`: the certificate checked once now (a broken pair stops the start), then reloaded as
/// it changes.
pub fn acceptor(files: TlsFiles) -> Result<TlsAcceptor, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let certs = Arc::new(Reloading::new(files, provider.clone())?);
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_cert_resolver(certs);
    // a WebSocket over TLS is HTTP/1.1 (browsers offer h2 too – never for a ws upgrade on this server)
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// The same certificate for the QUIC door: TLS 1.3 only, HTTP/3's ALPN (what WebTransport runs on), reloaded the same way.
pub fn quic_config(files: TlsFiles) -> Result<rustls::ServerConfig, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let certs = Arc::new(Reloading::new(files, provider.clone())?);
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_cert_resolver(certs);
    config.alpn_protocols = vec![b"h3".to_vec()];
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| (*v).to_owned())
    }

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn plain_by_default() {
        assert_eq!(Listen::parse(args(&[]), env(&[])).unwrap(), Listen { addr: "0.0.0.0:7700".into(), tls: None, quic: None, cert_source: None });
        assert_eq!(Listen::parse(args(&["127.0.0.1:7071"]), env(&[("RELAY_ADDR", "x")])).unwrap().addr, "127.0.0.1:7071");
        assert_eq!(Listen::parse(args(&[]), env(&[("RELAY_ADDR", "0.0.0.0:7072"), ("RELAY_TLS_CERT", "")])).unwrap().tls, None);
    }

    #[test]
    fn a_certificate_from_the_args_or_the_env() {
        let tls = Some(TlsFiles { cert: "/c.pem".into(), key: "/k.pem".into() });
        let l = Listen::parse(args(&["0.0.0.0:7700", "--tls-cert", "/c.pem", "--tls-key", "/k.pem"]), env(&[])).unwrap();
        assert_eq!(l.tls, tls);
        let l = Listen::parse(args(&["--tls-key", "/k.pem"]), env(&[("RELAY_TLS_CERT", "/c.pem")])).unwrap();
        assert_eq!((l.addr.as_str(), l.tls), ("0.0.0.0:7700", tls));
    }

    #[test]
    fn a_certificate_opens_the_quic_door_unless_told_otherwise() {
        let pair = ["--tls-cert", "/c.pem", "--tls-key", "/k.pem"];
        let with = |more: &[&str], e: &'static [(&'static str, &'static str)]| {
            let mut a = args(&pair);
            a.extend(args(more));
            Listen::parse(a, env(e)).unwrap().quic
        };
        assert_eq!(with(&[], &[]).as_deref(), Some("0.0.0.0:7700"), "the relay's own port number, over UDP");
        assert_eq!(with(&["1.2.3.4:9"], &[]).as_deref(), Some("1.2.3.4:9"));
        assert_eq!(with(&["--quic", "0.0.0.0:443"], &[]).as_deref(), Some("0.0.0.0:443"));
        assert_eq!(with(&[], &[("RELAY_QUIC_ADDR", "0.0.0.0:443")]).as_deref(), Some("0.0.0.0:443"));
        assert_eq!(with(&["--no-quic"], &[]), None);
        assert_eq!(with(&[], &[("RELAY_QUIC_ADDR", "off")]), None);
        assert_eq!(Listen::parse(args(&[]), env(&[("RELAY_QUIC_ADDR", "off")])).unwrap().quic, None, "off needs no certificate");
        assert!(Listen::parse(args(&["--quic", "0.0.0.0:7700"]), env(&[])).unwrap_err().contains("needs a certificate"));
    }

    #[test]
    fn a_certificate_url_means_tls_and_the_door_with_the_files_in_the_image() {
        let l = Listen::parse(args(&[]), env(&[("RELAY_CERT_URL", "https://a.test/c"), ("RELAY_CERT_KEY", "k")])).unwrap();
        assert_eq!(l.cert_source, Some(crate::cert::Source { url: "https://a.test/c".into(), key: "k".into() }));
        assert_eq!(l.tls, Some(TlsFiles { cert: "/certs/fullchain.pem".into(), key: "/certs/privkey.pem".into() }));
        assert_eq!(l.quic.as_deref(), Some("0.0.0.0:7700"));
        assert!(Listen::parse(args(&[]), env(&[("RELAY_CERT_URL", "https://a.test/c")])).unwrap_err().contains("RELAY_CERT_KEY"));
    }

    #[test]
    fn half_a_pair_or_a_typo_is_an_error() {
        assert!(Listen::parse(args(&["--tls-cert", "/c.pem"]), env(&[])).unwrap_err().contains("without its key"));
        assert!(Listen::parse(args(&[]), env(&[("RELAY_TLS_KEY", "/k.pem")])).unwrap_err().contains("without its certificate"));
        assert!(Listen::parse(args(&["--tls-cert"]), env(&[])).is_err());
        assert!(Listen::parse(args(&["--tls"]), env(&[])).unwrap_err().contains("unknown option"));
        assert!(Listen::parse(args(&["a:1", "b:2"]), env(&[])).is_err());
    }
}
