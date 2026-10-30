//! Where the stage keeps what it fetched (the library's chapters and files, the space's rooms) and where it fetches
//! from. The fetching itself is the `chapter` module's (library.rs).
use std::path::{Path, PathBuf};

pub const API: &str = "https://api.scutoons.com/api";
pub const CDN: &str = "https://cdn.scutoons.com";

/// the cache directory: `--cache` / DECK_CACHE, else the deck's DECKDATA (/var/lib/deck/cache) when it exists, else
/// the user's cache folder
pub fn default_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("DECK_CACHE") {
        return p.into();
    }
    let deck = Path::new("/var/lib/deck");
    if deck.is_dir() {
        return deck.join("cache");
    }
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("scutoons").join("deck-cache")
}
