//! The hosted relay's certificate (#441): `live.scutoons.com` answers wss and QUIC at the relay itself, so the relay needs
//! a certificate, and its image has no shell for a `deck-tls`. It fetches the pair the way a deck does: one PEM text from
//! the API (`GET /api/relay/live-cert` with `X-Relay-Key`, stored there by the workflow lan-cert.yml), at start and every
//! six hours. No certificate at start is an error: the relay stops and the container comes back and tries again. Later a
//! fetch that fails is only logged; a certificate lives 90 days and is renewed monthly.
//!
//! This only writes the two files. `tls::Reloading` picks them up on the next handshake, for wss and QUIC alike.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::tls::{self, TlsFiles};

/// how often a running relay asks for a newer pair
pub const EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const TIMEOUT: Duration = Duration::from_secs(30);
const KEY_HEADER: &str = "X-Relay-Key";

/// Where the certificate comes from (`RELAY_CERT_URL`, `RELAY_CERT_KEY`).
#[derive(Clone, PartialEq, Eq)]
pub struct Source {
    pub url: String,
    pub key: String,
}

/// The key stays out of every log line.
impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Source").field("url", &self.url).finish_non_exhaustive()
    }
}

/// One PEM text, the chain and the key's block in it → (chain, key). None when one of the two is missing.
pub fn split(both: &str) -> Option<(String, String)> {
    let (mut chain, mut key, mut in_key) = (String::new(), String::new(), false);
    for line in both.lines().map(str::trim_end).filter(|l| !l.is_empty()) {
        in_key |= line.starts_with("-----BEGIN") && line.ends_with("PRIVATE KEY-----");
        let to = if in_key { &mut key } else { &mut chain };
        to.push_str(line);
        to.push('\n');
        in_key &= !(line.starts_with("-----END") && line.ends_with("PRIVATE KEY-----"));
    }
    (chain.contains("BEGIN CERTIFICATE") && !key.is_empty()).then_some((chain, key))
}

/// Ask the API. Blocking: `refresh` runs it off the runtime.
pub fn fetch(source: &Source) -> Result<String, String> {
    let answer = ureq::get(&source.url)
        .set(KEY_HEADER, &source.key)
        .timeout(TIMEOUT)
        .call()
        .map_err(|e| e.to_string())?; // ureq's message names the URL
    answer.into_string().map_err(|e| format!("{}: {e}", source.url))
}

/// `name.new` next to `name`
fn beside(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".new");
    PathBuf::from(name)
}

fn write_private(path: &Path, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(content.as_bytes())
}

/// Put a pair in place if it is new and it loads; a broken one leaves the files as they were. `Ok(true)`: replaced.
pub fn install(both: &str, files: &TlsFiles) -> Result<bool, String> {
    let (chain, key) = split(both).ok_or("the answer holds no certificate and key")?;
    let same = |path: &Path, new: &str| std::fs::read_to_string(path).is_ok_and(|old| old == new);
    if same(&files.cert, &chain) && same(&files.key, &key) {
        return Ok(false);
    }
    let new = TlsFiles { cert: beside(&files.cert), key: beside(&files.key) };
    let io = |path: &Path, e: std::io::Error| format!("{}: {e}", path.display());
    write_private(&new.cert, &chain).map_err(|e| io(&new.cert, e))?;
    write_private(&new.key, &key).map_err(|e| io(&new.key, e))?;
    if let Err(e) = tls::load(&new, &rustls::crypto::ring::default_provider()) {
        let _ = std::fs::remove_file(&new.cert);
        let _ = std::fs::remove_file(&new.key);
        return Err(e);
    }
    std::fs::rename(&new.cert, &files.cert).map_err(|e| io(&files.cert, e))?;
    std::fs::rename(&new.key, &files.key).map_err(|e| io(&files.key, e))?;
    Ok(true)
}

/// Fetch and install, off the runtime. `Ok(true)`: there is a new pair.
pub async fn refresh(source: Source, files: TlsFiles) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || install(&fetch(&source)?, &files)).await.map_err(|e| e.to_string())?
}

/// A running relay's renewal.
pub async fn keep_fresh(source: Source, files: TlsFiles) {
    loop {
        tokio::time::sleep(EVERY).await;
        match refresh(source.clone(), files.clone()).await {
            Ok(true) => tracing::info!("new certificate fetched"),
            Ok(false) => tracing::debug!("the certificate is current"),
            Err(e) => tracing::warn!("certificate fetch failed, keeping the one we have: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pem_text_comes_apart_in_chain_and_key() {
        let both = "-----BEGIN CERTIFICATE-----\nAA\n-----END CERTIFICATE-----\n-----BEGIN CERTIFICATE-----\nBB\n-----END CERTIFICATE-----\n\n-----BEGIN EC PRIVATE KEY-----\nKK\n-----END EC PRIVATE KEY-----\n";
        let (chain, key) = split(both).unwrap();
        assert_eq!(chain, "-----BEGIN CERTIFICATE-----\nAA\n-----END CERTIFICATE-----\n-----BEGIN CERTIFICATE-----\nBB\n-----END CERTIFICATE-----\n");
        assert_eq!(key, "-----BEGIN EC PRIVATE KEY-----\nKK\n-----END EC PRIVATE KEY-----\n");
    }

    #[test]
    fn half_an_answer_is_none() {
        assert_eq!(split("-----BEGIN CERTIFICATE-----\nAA\n-----END CERTIFICATE-----\n"), None);
        assert_eq!(split("-----BEGIN PRIVATE KEY-----\nKK\n-----END PRIVATE KEY-----\n"), None);
        assert_eq!(split("{\"error\":\"no_certificate\"}"), None);
    }

    #[test]
    fn the_key_is_not_in_the_debug_output() {
        let s = format!("{:?}", Source { url: "https://a.test/".into(), key: "s3cret".into() });
        assert!(s.contains("a.test") && !s.contains("s3cret"), "{s}");
    }
}
