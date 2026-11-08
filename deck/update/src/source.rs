//! Where an update comes from: the source that answered last, deck.toml's `update_url`, a dev PC found by scanning the
//! deck's LAN (port 7790 on every host of its private /24s), the server. Every manifest is verified before it counts.

use crate::manifest::{Manifest, MANIFEST, SIGNATURE};
use crate::sign;
use crate::state::{Channel, Config, State, DEV_PORT, SCAN_EVERY, SERVER};
use anyhow::{bail, Context, Result};
use ed25519_dalek::VerifyingKey;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

/// HTTP as the updater needs it (tests use a fake)
pub trait Fetch {
    /// the body of `url`, at most `limit` bytes
    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>>;
    /// `url` into `dest`, at most `limit` bytes; returns the SHA-256 (hex)
    fn download(&self, url: &str, dest: &Path, limit: u64) -> Result<String>;
}

pub struct Http {
    agent: ureq::Agent,
}

impl Http {
    pub fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(3))
            .timeout_read(Duration::from_secs(30))
            .build();
        Self { agent }
    }

    fn open(&self, url: &str) -> Result<ureq::Response> {
        match self.agent.get(url).call() {
            Ok(r) => Ok(r),
            Err(ureq::Error::Status(code, _)) => bail!("{url}: HTTP {code}"),
            Err(e) => bail!("{url}: {e}"),
        }
    }
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetch for Http {
    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>> {
        let mut body = Vec::new();
        self.open(url)?.into_reader().take(limit + 1).read_to_end(&mut body)?;
        if body.len() as u64 > limit {
            bail!("{url}: larger than {limit} bytes");
        }
        Ok(body)
    }

    fn download(&self, url: &str, dest: &Path, limit: u64) -> Result<String> {
        let mut reader = self.open(url)?.into_reader().take(limit + 1);
        let mut file = std::fs::File::create(dest).with_context(|| dest.display().to_string())?;
        let mut hash = Sha256::new();
        let mut total = 0u64;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > limit {
                bail!("{url}: larger than {limit} bytes");
            }
            hash.update(&buf[..n]);
            file.write_all(&buf[..n])?;
        }
        file.sync_all()?;
        Ok(crate::manifest::hex(&hash.finalize()))
    }
}

/// A verified manifest and where it came from
#[derive(Debug, Clone)]
pub struct Offer {
    pub base: String,
    pub manifest: Manifest,
    pub raw: Vec<u8>,
    pub signature: String,
}

impl Offer {
    pub fn bundle_url(&self) -> String {
        format!("{}/{}", self.base, self.manifest.bundle)
    }
}

/// The manifest at `base`, verified and of the deck's channel
pub fn offer(fetch: &dyn Fetch, base: &str, keys: &[VerifyingKey], channel: Channel) -> Result<Offer> {
    let base = base.trim_end_matches('/').to_string();
    let raw = fetch.get(&format!("{base}/{MANIFEST}"), 1 << 20)?;
    let signature = String::from_utf8(fetch.get(&format!("{base}/{SIGNATURE}"), 1024)?)?.trim().to_string();
    sign::verify(keys, &raw, &signature).with_context(|| format!("{base}/{MANIFEST}"))?;
    let manifest: Manifest = serde_json::from_slice(&raw).with_context(|| format!("{base}/{MANIFEST}"))?;
    manifest.validate()?;
    if manifest.channel != channel.name() {
        bail!("{base} offers the {} channel, the deck follows {}", manifest.channel, channel.name());
    }
    Ok(Offer { base, manifest, raw, signature })
}

/// The sources to ask, in order, without duplicates. `lan` is only called when the dev channel scans: every
/// `SCAN_EVERY`, or at once on a manual run (`eager`) that remembers no dev PC.
pub fn bases(
    config: &Config,
    state: &State,
    now: u64,
    eager: bool,
    lan: &dyn Fn() -> Vec<String>,
) -> (Vec<String>, bool) {
    let mut out: Vec<String> = Vec::new();
    let mut push = |b: String| {
        let b = b.trim_end_matches('/').to_string();
        if !out.contains(&b) {
            out.push(b);
        }
    };
    if let Some(u) = &config.url {
        push(u.clone());
    }
    let mut scanned = false;
    if config.channel == Channel::Dev {
        if let Some(s) = &state.source {
            push(s.clone());
        }
        let remembered = state.source.as_deref().is_some_and(|s| s.starts_with("http://"));
        if (eager && !remembered) || now.saturating_sub(state.scanned_at) >= SCAN_EVERY {
            for b in lan() {
                push(b);
            }
            scanned = true;
        }
    }
    push(format!("{SERVER}/{}", config.channel.name()));
    (out, scanned)
}

/// The newest verified offer among `bases`, with what went wrong at the others (for the log)
pub fn newest(
    fetch: &dyn Fetch,
    bases: &[String],
    keys: &[VerifyingKey],
    channel: Channel,
) -> (Option<Offer>, Vec<String>) {
    let mut best: Option<Offer> = None;
    let mut errors = Vec::new();
    for b in bases {
        match offer(fetch, b, keys, channel) {
            Ok(o) => {
                if best.as_ref().is_none_or(|x| o.manifest.build > x.manifest.build) {
                    best = Some(o);
                }
            }
            Err(e) => errors.push(format!("{e:#}")),
        }
    }
    (best, errors)
}

/// The deck's private IPv4 addresses from `ip -4 -o addr show scope global`
pub fn parse_ip_addr(text: &str) -> Vec<Ipv4Addr> {
    text.lines()
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            words.position(|w| w == "inet")?;
            words.next()?.split('/').next()?.parse::<Ipv4Addr>().ok()
        })
        .filter(|a| a.is_private())
        .collect()
}

pub fn local_addresses() -> Vec<Ipv4Addr> {
    std::process::Command::new("ip")
        .args(["-4", "-o", "addr", "show", "scope", "global"])
        .output()
        .map(|o| parse_ip_addr(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default()
}

/// Every host of each address's /24 (but the deck itself) that accepts a connection on `port`, as `http://ip:port`.
/// 64 probes at a time; `probe` is a TCP connect with a short timeout on the deck.
pub fn scan(own: &[Ipv4Addr], port: u16, probe: &(dyn Fn(SocketAddr) -> bool + Sync)) -> Vec<String> {
    let mut hosts: Vec<Ipv4Addr> = Vec::new();
    for a in own {
        let [x, y, z, _] = a.octets();
        for h in 1..=254u8 {
            let ip = Ipv4Addr::new(x, y, z, h);
            if !own.contains(&ip) && !hosts.contains(&ip) {
                hosts.push(ip);
            }
        }
    }
    let mut found = Vec::new();
    for chunk in hosts.chunks(64) {
        let hits: Vec<Ipv4Addr> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|ip| s.spawn(move || probe(SocketAddr::from((*ip, port))).then_some(*ip)))
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok().flatten()).collect()
        });
        found.extend(hits.into_iter().map(|ip| format!("http://{ip}:{port}")));
    }
    found
}

/// The dev PCs on the deck's LAN
pub fn lan() -> Vec<String> {
    scan(&local_addresses(), DEV_PORT, &|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::manifest::sha256_hex;
    use ed25519_dalek::SigningKey;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// URLs → bodies; records what was asked
    #[derive(Default)]
    pub struct FakeFetch {
        pub bodies: Mutex<HashMap<String, Vec<u8>>>,
        pub asked: Mutex<Vec<String>>,
    }

    impl FakeFetch {
        pub fn put(&self, url: &str, body: Vec<u8>) {
            self.bodies.lock().unwrap().insert(url.to_string(), body);
        }
        /// a signed manifest (+ a fake tarball) at `base`
        pub fn serve(&self, base: &str, key: &SigningKey, m: &Manifest, bundle: &[u8]) {
            let raw = serde_json::to_vec(m).unwrap();
            self.put(&format!("{base}/{MANIFEST}"), raw.clone());
            self.put(&format!("{base}/{SIGNATURE}"), sign::sign(key, &raw).into_bytes());
            self.put(&format!("{base}/{}", m.bundle), bundle.to_vec());
        }
    }

    impl Fetch for FakeFetch {
        fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>> {
            self.asked.lock().unwrap().push(url.to_string());
            let body = self.bodies.lock().unwrap().get(url).cloned();
            match body {
                Some(b) if b.len() as u64 <= limit => Ok(b),
                Some(_) => bail!("{url}: too large"),
                None => bail!("{url}: unreachable"),
            }
        }
        fn download(&self, url: &str, dest: &Path, limit: u64) -> Result<String> {
            let b = self.get(url, limit)?;
            std::fs::write(dest, &b)?;
            Ok(sha256_hex(&b))
        }
    }

    pub fn manifest(build: u64, channel: &str) -> Manifest {
        let mut m = crate::manifest::tests::manifest();
        m.build = build;
        m.channel = channel.into();
        m
    }

    #[test]
    fn the_newest_verified_offer_wins() {
        let key = SigningKey::from_bytes(&[1; 32]);
        let stranger = SigningKey::from_bytes(&[2; 32]);
        let f = FakeFetch::default();
        f.serve("http://a:7790", &key, &manifest(5, "dev"), b"a");
        f.serve("http://b:7790", &key, &manifest(9, "dev"), b"b");
        f.serve("http://evil:7790", &stranger, &manifest(99, "dev"), b"x");
        f.serve("http://other:7790", &key, &manifest(50, "stable"), b"s");
        let bases: Vec<String> =
            ["http://a:7790", "http://evil:7790", "http://b:7790", "http://other:7790", "http://gone:7790"]
                .map(String::from)
                .to_vec();
        let (best, errors) = newest(&f, &bases, &[key.verifying_key()], Channel::Dev);
        let best = best.unwrap();
        assert_eq!((best.base.as_str(), best.manifest.build), ("http://b:7790", 9));
        assert_eq!(best.bundle_url(), "http://b:7790/bundle.tar.gz");
        assert_eq!(errors.len(), 3, "{errors:?}");
    }

    #[test]
    fn the_sources_in_order() {
        let cfg = Config { channel: Channel::Dev, url: Some("http://fixed:7790/".into()) };
        let lan = || vec!["http://10.0.0.7:7790".to_string(), "http://fixed:7790".to_string()];
        // never scanned → scan
        let (b, scanned) = bases(&cfg, &State::default(), 1000, false, &lan);
        assert!(scanned);
        assert_eq!(b, vec!["http://fixed:7790", "http://10.0.0.7:7790", format!("{SERVER}/dev").as_str()]);
        // a remembered dev PC, scanned a minute ago → no scan, also not on a manual run
        let st = State { source: Some("http://10.0.0.7:7790".into()), scanned_at: 940, ..State::default() };
        let (b, scanned) = bases(&Config { url: None, ..cfg.clone() }, &st, 1000, true, &lan);
        assert!(!scanned);
        assert_eq!(b, vec!["http://10.0.0.7:7790", format!("{SERVER}/dev").as_str()]);
        // nothing remembered, scanned a minute ago: the timer waits, a manual run scans
        let none = State { scanned_at: 940, ..State::default() };
        assert!(!bases(&cfg, &none, 1000, false, &lan).1);
        assert!(bases(&cfg, &none, 1000, true, &lan).1);
        // stable: never the LAN
        let (b, scanned) = bases(&Config { channel: Channel::Stable, url: None }, &st, 1000, true, &lan);
        assert!(!scanned);
        assert_eq!(b, vec![format!("{SERVER}/stable")]);
    }

    #[test]
    fn the_lan_scan() {
        let text = "2: wlan0    inet 192.168.1.116/24 brd 192.168.1.255 scope global dynamic wlan0\n\
                    3: wwan0    inet 100.64.3.2/10 scope global wwan0\n\
                    4: eth0    inet 10.42.0.1/24 scope global eth0\n";
        let own = parse_ip_addr(text);
        assert_eq!(own, vec![Ipv4Addr::new(192, 168, 1, 116), Ipv4Addr::new(10, 42, 0, 1)]);
        let probe = |a: SocketAddr| a.ip() == Ipv4Addr::new(192, 168, 1, 50) || a.ip() == Ipv4Addr::new(192, 168, 1, 116);
        assert_eq!(scan(&own, 7790, &probe), vec!["http://192.168.1.50:7790"], "never the deck itself");
    }
}
