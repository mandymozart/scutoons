//! Where things live on a deck, the settings it reads from deck.toml and what it remembers between runs.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// The dev PC's `deck-update serve` (`npm run deck:update`)
pub const DEV_PORT: u16 = 7790;
/// The server's bundles, one folder per channel (bunny, like the CDN's media)
pub const SERVER: &str = "https://cdn.scutoons.com/deck/update";
/// The stable channel asks the server this often (the dev channel on every timer run)
pub const STABLE_EVERY: u64 = 6 * 3600;
/// The timer scans the LAN at most this often (a manual run that remembers no dev PC scans at once)
pub const SCAN_EVERY: u64 = 5 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Off,
    Dev,
    Stable,
}

impl Channel {
    pub fn name(self) -> &'static str {
        match self {
            Channel::Off => "off",
            Channel::Dev => "dev",
            Channel::Stable => "stable",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub channel: Channel,
    /// a fixed source (`http://192.168.1.50:7790`) – asked first
    pub url: Option<String>,
}

impl Config {
    /// `update_channel` (`dev` until the first stable release exists – #237) and `update_url` from deck.toml
    pub fn parse(text: &str) -> Config {
        let table: toml::Table = text.parse().unwrap_or_default();
        let get = |k: &str| table.get(k).and_then(|v| v.as_str()).map(str::trim).unwrap_or("").to_string();
        let channel = match get("update_channel").as_str() {
            "off" => Channel::Off,
            "stable" => Channel::Stable,
            _ => Channel::Dev,
        };
        let url = Some(get("update_url")).filter(|u| !u.is_empty());
        Config { channel, url }
    }
}

/// The paths, all under `root` (`/` on a deck, a temp dir in tests)
#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    pub fn deck_toml(&self) -> PathBuf {
        self.root.join("var/lib/deck/deck.toml")
    }
    /// downloads, the staged bundle the boot re-applies, the state
    pub fn dir(&self) -> PathBuf {
        self.root.join("var/lib/deck/update")
    }
    pub fn state(&self) -> PathBuf {
        self.dir().join("state.json")
    }
    /// the last applied bundle (manifest, signature, tarball): a live stick's root resets at every boot, so
    /// `deck-update boot` puts it back
    pub fn current(&self) -> PathBuf {
        self.dir().join("current")
    }
    /// the image's keys, then the deck's own (`deck-update trust`)
    pub fn keys(&self) -> [PathBuf; 2] {
        [self.root.join("usr/share/deck/update-keys"), self.root.join("var/lib/deck/update-keys")]
    }
    pub fn own_keys(&self) -> PathBuf {
        self.root.join("var/lib/deck/update-keys")
    }
    /// what the running root holds: written by the image build and by every update
    pub fn stamp(&self) -> PathBuf {
        self.root.join("usr/share/deck/build.json")
    }
}

/// `usr/share/deck/build.json`
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stamp {
    pub version: String,
    pub build: u64,
}

impl Stamp {
    pub fn read(paths: &Paths) -> Stamp {
        fs::read(paths.stamp()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct State {
    /// the source that answered last (asked first next time)
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub checked_at: u64,
    #[serde(default)]
    pub scanned_at: u64,
    /// builds that were rolled back – never tried again by the timer
    #[serde(default)]
    pub bad_builds: Vec<u64>,
    /// what the last run did, for `deck-update status`
    #[serde(default)]
    pub last: String,
}

impl State {
    pub fn load(paths: &Paths) -> State {
        fs::read(paths.state()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self, paths: &Paths) -> Result<()> {
        write_atomic(&paths.state(), &serde_json::to_vec_pretty(self)?)
    }
}

/// A temp file next to `path`, then a rename – a power cut leaves the old file or the new one
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("deck-new");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_from_deck_toml() {
        let c = Config::parse("kiosk = \"stage\"\nupdate_channel = \"stable\"\nupdate_url = \" http://10.0.0.5:7790 \"\n");
        assert_eq!(c, Config { channel: Channel::Stable, url: Some("http://10.0.0.5:7790".into()) });
        assert_eq!(Config::parse("").channel, Channel::Dev, "dev until a stable release exists");
        assert_eq!(Config::parse("update_channel = \"off\"").channel, Channel::Off);
        assert_eq!(Config::parse("update_url = \"\"").url, None);
        assert_eq!(Config::parse("not toml [").channel, Channel::Dev);
    }

    #[test]
    fn state_and_stamp_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        assert_eq!(State::load(&paths), State::default());
        let s = State { source: Some("http://x:7790".into()), checked_at: 5, bad_builds: vec![3], ..State::default() };
        s.save(&paths).unwrap();
        assert_eq!(State::load(&paths), s);
        assert_eq!(Stamp::read(&paths), Stamp::default());
    }
}
