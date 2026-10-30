//! Where this deck's outlet joins (#186): the relay with the session code (`ws://<host>:7700/live/<code>`), the
//! outlet (the venue patch maps it to this deck – LIVE.md B.5.4) and the locale its text layers show. From
//! `/var/lib/deck/deck.toml` (`relay`, `outlet`, `locale` – one key per line, deck-session reads it with sed too); the
//! stage's `--relay`, `--outlet`, `--locale` override. Without a relay and an outlet the deck plays no outlet.
use anyhow::{bail, Result};
use serde::Deserialize;
use std::path::Path;

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

    /// The outlet to play: None when relay or outlet is unset; Err when the relay URL can't be joined.
    pub fn config(&self) -> Option<Result<Config>> {
        let (relay, outlet) = (self.relay.as_ref()?, self.outlet.as_ref()?);
        Some(Config::new(relay, outlet, self.locale.as_deref().unwrap_or("de")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// `ws://host:port` (no path)
    pub base: String,
    /// the session code
    pub code: String,
    pub outlet: String,
    pub locale: String,
}

impl Config {
    /// `relay` = `ws://host:port/live/<code>` (a query is dropped – the deck adds its own)
    pub fn new(relay: &str, outlet: &str, locale: &str) -> Result<Config> {
        let relay = relay.trim();
        if relay.starts_with("wss://") {
            bail!("{relay}: wss needs TLS, which the deck's client lacks – join the venue's local relay (ws://, LIVE.md B.4)");
        }
        let Some(rest) = relay.strip_prefix("ws://") else { bail!("{relay}: the relay is a ws:// URL") };
        let rest = rest.split('?').next().unwrap_or("");
        let Some((host, path)) = rest.split_once('/') else { bail!("{relay}: no session code (ws://host:7700/live/<code>)") };
        let Some(code) = path.strip_prefix("live/").map(|c| c.trim_matches('/')).filter(|c| !c.is_empty() && !c.contains('/')) else {
            bail!("{relay}: no session code (ws://host:7700/live/<code>)")
        };
        let outlet = outlet.trim();
        if outlet.is_empty() || !outlet.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            bail!("outlet '{outlet}': letters, digits, - and _ only");
        }
        Ok(Config { base: format!("ws://{host}"), code: code.into(), outlet: outlet.into(), locale: locale.into() })
    }

    /// what the client connects to (B.2: the role and group from the URL; the hello repeats them)
    pub fn join_url(&self) -> String {
        format!("{}/live/{}?role=output&group={}", self.base, self.code, self.outlet)
    }
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
        assert!(Settings::parse("relay = \"\"\n").config().is_none(), "no relay, no outlet");
        assert!(Settings::parse("broken = [").relay.is_none());
    }

    #[test]
    fn relay_urls_that_cannot_be_joined_say_why() {
        assert!(Config::new("wss://live.scutoons.com/live/A", "left", "de").unwrap_err().to_string().contains("TLS"));
        assert!(Config::new("http://x/live/A", "left", "de").is_err());
        assert!(Config::new("ws://x:7700/", "left", "de").unwrap_err().to_string().contains("session code"));
        assert!(Config::new("ws://x:7700/live/A", "le ft", "de").is_err());
        assert_eq!(Config::new("ws://x:7700/live/A7K2/?role=x", "l", "en").unwrap().code, "A7K2");
    }
}
