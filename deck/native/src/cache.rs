//! Where the stage keeps what it fetched (the library's chapters and files, the sets' 3D, the heads' portraits) and
//! where it fetches from. The fetching itself is the `chapter` module's (library.rs) and the `sync` module's (sync.rs).
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

/// A set's 3D file at `quality` under `sets` (deck-space's scene.rs – the space node reads it too).
pub use deck_space::scene::set_file;

#[cfg(test)]
mod tests {
    #[test]
    fn a_set_file_is_in_its_folder_else_flat() {
        let dir = std::env::temp_dir().join(format!("deck-cache-sets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(super::set_file(&dir, "cathedral", "low"), dir.join("cathedral").join("cathedral.low.glb"), "neither: where the sync puts it");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("cathedral.low.glb"), "glTF").unwrap();
        assert_eq!(super::set_file(&dir, "cathedral", "low"), dir.join("cathedral.low.glb"), "a build folder");
        std::fs::create_dir_all(dir.join("cathedral")).unwrap();
        std::fs::write(dir.join("cathedral").join("cathedral.low.glb"), "glTF").unwrap();
        assert_eq!(super::set_file(&dir, "cathedral", "low"), dir.join("cathedral").join("cathedral.low.glb"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
