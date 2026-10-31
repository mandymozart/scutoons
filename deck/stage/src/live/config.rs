//! Where this deck's outlet joins (#186): the relay with the session code (`ws://<host>:7700/live/<code>`), the
//! outlet (the venue patch maps it to this deck – LIVE.md B.5.4) and the locale its text layers show. From
//! `/var/lib/deck/deck.toml` (`relay`, `outlet`, `locale` – one key per line, deck-session reads it with sed too); the
//! stage's `--relay`, `--outlet`, `--locale` override. Without an outlet the deck plays no outlet; without a relay it
//! joins the hosted one (`HOSTED_RELAY`, #199). `dial` opens the socket for both clients (the outlet's, link.rs).
use anyhow::{bail, Result};
use serde::Deserialize;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::WebSocket;

/// The hosted relay and the session a deck joins when nothing names one (#199, LIVE.md B.4: bunny Magic Containers
/// `scutoons-live`): a scene that joins (space `join` = deck / conductor) or an outlet without `relay` in deck.toml or
/// `--relay`. The code is the Cathedral's for now.
pub const HOSTED_RELAY: &str = "wss://live.scutoons.com/live/CATH";

/// The deck's settings file (deck/os/rootfs/usr/share/deck/deck.default.toml, copied at first boot).
pub const DECK_TOML: &str = "/var/lib/deck/deck.toml";

/// The outlet keys of deck.toml (the file holds others: kiosk, scene, conductor …)
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub relay: Option<String>,
    #[serde(default)]
    pub outlet: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
}

impl Settings {
    /// deck.toml's text (unknown keys are the session's; a broken file = no settings, said in the log)
    pub fn parse(text: &str) -> Settings {
        toml::from_str(text).unwrap_or_else(|e| {
            tracing::warn!("deck.toml: {e}");
            Settings::default()
        })
    }

    /// `path` when it exists, else nothing set
    pub fn read(path: &Path) -> Settings {
        std::fs::read_to_string(path).map(|t| Settings::parse(&t)).unwrap_or_default()
    }

    /// `over`'s keys win (the CLI over deck.toml); empty strings count as unset
    pub fn merged(self, over: Settings) -> Settings {
        let pick = |a: Option<String>, b: Option<String>| b.filter(|s| !s.is_empty()).or(a.filter(|s| !s.is_empty()));
        Settings { relay: pick(self.relay, over.relay), outlet: pick(self.outlet, over.outlet), locale: pick(self.locale, over.locale) }
    }

    /// The outlet to play: None when the outlet is unset; the hosted relay when the relay is (#199); Err when the
    /// relay URL can't be joined. Empty strings count as unset.
    pub fn config(&self) -> Option<Result<Config>> {
        fn set(v: &Option<String>) -> Option<&str> {
            v.as_deref().filter(|s| !s.trim().is_empty())
        }
        let outlet = set(&self.outlet)?;
        Some(Config::new(set(&self.relay).unwrap_or(HOSTED_RELAY), outlet, set(&self.locale).unwrap_or("de")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// `ws://host:port` or `wss://host[:port]` (no path)
    pub base: String,
    /// the session code
    pub code: String,
    pub outlet: String,
    pub locale: String,
}

impl Config {
    /// `relay` = `ws://host:port/live/<code>`, or `wss://…` (TLS – the hosted relay, #199); a query is dropped – the
    /// deck adds its own
    pub fn new(relay: &str, outlet: &str, locale: &str) -> Result<Config> {
        let relay = relay.trim();
        let (scheme, rest) = match (relay.strip_prefix("wss://"), relay.strip_prefix("ws://")) {
            (Some(rest), _) => ("wss", rest),
            (None, Some(rest)) => ("ws", rest),
            (None, None) => bail!("{relay}: the relay is a ws:// or wss:// URL"),
        };
        let rest = rest.split('?').next().unwrap_or("");
        let Some((host, path)) = rest.split_once('/') else { bail!("{relay}: no session code (ws://host:7700/live/<code>)") };
        let Some(code) = path.strip_prefix("live/").map(|c| c.trim_matches('/')).filter(|c| !c.is_empty() && !c.contains('/')) else {
            bail!("{relay}: no session code (ws://host:7700/live/<code>)")
        };
        let outlet = outlet.trim();
        if outlet.is_empty() || !outlet.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            bail!("outlet '{outlet}': letters, digits, - and _ only");
        }
        Ok(Config { base: format!("{scheme}://{host}"), code: code.into(), outlet: outlet.into(), locale: locale.into() })
    }

    /// what the client connects to (B.2: the role and group from the URL; the hello repeats them)
    pub fn join_url(&self) -> String {
        format!("{}/live/{}?role=output&group={}", self.base, self.code, self.outlet)
    }
}

/// `host:port` of a `ws://` / `wss://` URL – the scheme's port (80 / 443) when it names none
fn authority(url: &str) -> std::result::Result<String, String> {
    let (tls, rest) = match (url.strip_prefix("wss://"), url.strip_prefix("ws://")) {
        (Some(rest), _) => (true, rest),
        (None, Some(rest)) => (false, rest),
        (None, None) => return Err(format!("{url}: not a ws:// or wss:// URL")),
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() {
        return Err(format!("{url}: no host"));
    }
    // `[::1]` has colons but no port; `host:7700` and `[::1]:7700` have one
    let has_port = host
        .rsplit_once(':')
        .is_some_and(|(h, p)| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) && (!h.starts_with('[') || h.ends_with(']')));
    Ok(if has_port { host.to_owned() } else { format!("{host}:{}", if tls { 443 } else { 80 }) })
}

/// Open the WebSocket at `url` – `ws://`, or `wss://` with TLS (rustls + the webpki roots, no OpenSSL – #199): the
/// name's addresses in turn (a TCP connect within 3 s each; #201: `wss://<lan-ip-with-dashes>.lan.scutoons.com:7700`), no Nagle, the
/// handshake (at most 10 s per read on a host that stays silent), then a read timeout of `poll` on the socket under it,
/// so the client's loop gets its turns.
pub fn dial(url: &str, poll: Duration) -> std::result::Result<WebSocket<MaybeTlsStream<TcpStream>>, String> {
    let host = authority(url)?;
    // every address the name has, in the resolver's order (a name with AAAA + A, `localhost` = ::1 and 127.0.0.1)
    let addrs: Vec<_> = host.to_socket_addrs().map_err(|e| format!("{host}: {e}"))?.collect();
    let mut tcp = Err(format!("{host}: no address"));
    for addr in &addrs {
        tcp = TcpStream::connect_timeout(addr, Duration::from_secs(3)).map_err(|e| format!("{host}: {e}"));
        if tcp.is_ok() {
            break;
        }
    }
    let tcp = tcp?;
    tcp.set_nodelay(true).ok();
    tcp.set_read_timeout(Some(Duration::from_secs(10))).ok();
    let (ws, _) = tungstenite::client_tls(url, tcp).map_err(|e| format!("handshake: {e}"))?;
    let sock = match ws.get_ref() {
        MaybeTlsStream::Plain(s) => s,
        MaybeTlsStream::Rustls(s) => &s.sock,
        _ => return Err("an unexpected TLS stream".into()),
    };
    sock.set_read_timeout(Some(poll)).map_err(|e| e.to_string())?;
    Ok(ws)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_toml_and_the_cli() {
        let file = Settings::parse("kiosk = \"stage\"\nscene = \"\"\nrelay = \"ws://127.0.0.1:7700/live/DECK\"\noutlet = \"left\"\n");
        assert_eq!(file.relay.as_deref(), Some("ws://127.0.0.1:7700/live/DECK"));
        let cli = Settings { outlet: Some("nave".into()), locale: Some(String::new()), ..Default::default() };
        let s = file.merged(cli);
        let c = s.config().unwrap().unwrap();
        assert_eq!((c.base.as_str(), c.code.as_str(), c.outlet.as_str(), c.locale.as_str()), ("ws://127.0.0.1:7700", "DECK", "nave", "de"));
        assert_eq!(c.join_url(), "ws://127.0.0.1:7700/live/DECK?role=output&group=nave");
        assert!(Settings::parse("relay = \"\"\n").config().is_none(), "no outlet: no outlet node");
        let hosted = Settings::parse("relay = \"\"\noutlet = \"left\"\n").config().unwrap().unwrap();
        assert_eq!((hosted.base.as_str(), hosted.code.as_str()), ("wss://live.scutoons.com", "CATH"), "no relay: the hosted one");
        assert!(Settings::parse("broken = [").relay.is_none());
    }

    #[test]
    fn relay_urls_that_cannot_be_joined_say_why() {
        let tls = Config::new("wss://live.scutoons.com/live/A", "left", "de").unwrap();
        assert_eq!(tls.join_url(), "wss://live.scutoons.com/live/A?role=output&group=left");
        assert!(Config::new("http://x/live/A", "left", "de").is_err());
        assert!(Config::new("ws://x:7700/", "left", "de").unwrap_err().to_string().contains("session code"));
        assert!(Config::new("ws://x:7700/live/A", "le ft", "de").is_err());
        assert_eq!(Config::new("ws://x:7700/live/A7K2/?role=x", "l", "en").unwrap().code, "A7K2");
        // #201: a venue's relay by its name – the deck's LAN IP with dashes, answered by bunny DNS
        let venue = Config::new("wss://192-168-1-116.lan.scutoons.com:7700/live/CATH", "left", "de").unwrap();
        assert_eq!((venue.base.as_str(), venue.code.as_str()), ("wss://192-168-1-116.lan.scutoons.com:7700", "CATH"));
        assert_eq!(authority(&venue.join_url()).unwrap(), "192-168-1-116.lan.scutoons.com:7700");
    }

    #[test]
    fn the_port_of_a_relay_url() {
        assert_eq!(authority("wss://live.scutoons.com/live/CATH").unwrap(), "live.scutoons.com:443");
        assert_eq!(authority("ws://10.42.0.1:7700/live/CATH?role=x").unwrap(), "10.42.0.1:7700");
        assert_eq!(authority("ws://deck.local/live/A").unwrap(), "deck.local:80");
        assert_eq!(authority("wss://[::1]/live/A").unwrap(), "[::1]:443");
        assert_eq!(authority("ws://[::1]:7071/live/A").unwrap(), "[::1]:7071");
        assert!(authority("http://x/live/A").is_err());
        assert!(authority("ws:///live/A").is_err());
    }

    /// what a client sends first: a stand-in server on 127.0.0.1 keeps the first bytes and closes
    fn first_bytes(scheme: &str) -> (Vec<u8>, String) {
        first_bytes_at(scheme, "127.0.0.1")
    }

    /// the same for a host name: the stand-in listens on the address the name resolves to first (`dial` resolves it)
    fn first_bytes_at(scheme: &str, host: &str) -> (Vec<u8>, String) {
        let addr = (host, 0).to_socket_addrs().unwrap().next().expect("the name resolves");
        let server = std::net::TcpListener::bind(addr).unwrap();
        let port = server.local_addr().unwrap().port();
        let seen = std::thread::spawn(move || {
            use std::io::Read;
            let (mut s, _) = server.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut buf = vec![0u8; 4096];
            let mut n = s.read(&mut buf).unwrap();
            // the rest of a ClientHello that came in more than one segment
            s.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
            while n < buf.len() {
                match s.read(&mut buf[n..]) {
                    Ok(0) | Err(_) => break,
                    Ok(m) => n += m,
                }
            }
            buf.truncate(n);
            buf
        });
        let Err(err) = dial(&format!("{scheme}://{host}:{port}/live/CATH"), Duration::from_millis(50)) else { panic!("the stand-in closes") };
        (seen.join().unwrap(), err)
    }

    #[test]
    fn a_relay_by_name_resolves_connects_and_names_itself() {
        // #201: a venue's relay is `wss://<lan-ip-with-dashes>.lan.scutoons.com:7700` – the name resolved, the TLS hello carries it
        // (SNI), so the relay's certificate is checked against that name; `localhost` stands in for the LAN name
        let (tls, err) = first_bytes_at("wss", "localhost");
        assert!(tls.first() == Some(&0x16), "a TLS ClientHello");
        assert!(tls.windows(9).any(|w| w == b"localhost"), "the name in the hello (SNI)");
        assert!(err.starts_with("handshake"), "{err}");
        let (plain, _) = first_bytes_at("ws", "localhost");
        let head = String::from_utf8_lossy(&plain);
        assert!(head.starts_with("GET /live/CATH HTTP/1.1") && head.to_ascii_lowercase().contains("host: localhost:"), "{head}");
        assert!(dial("wss://no-such-deck.invalid:7700/live/CATH", Duration::from_millis(50)).unwrap_err().contains("no-such-deck.invalid"));
    }

    #[test]
    fn wss_speaks_tls_and_ws_plain_http() {
        // wss: a TLS record of type handshake (0x16, version 3.x) – the ClientHello of rustls with the webpki roots
        let (tls, err) = first_bytes("wss");
        assert!(tls.len() > 5 && tls[0] == 0x16 && tls[1] == 0x03, "a TLS ClientHello, got {:?}", &tls[..tls.len().min(8)]);
        assert!(err.starts_with("handshake"), "{err}");
        // ws: the plain HTTP upgrade
        let (plain, _) = first_bytes("ws");
        assert!(plain.starts_with(b"GET /live/CATH HTTP/1.1"), "{}", String::from_utf8_lossy(&plain));
    }
}
