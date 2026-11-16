//! Which toon's address book the editor reads: the one of what is open – in Build the rig's (its character's `own_toon`
//! in the library's registry, `GET /api/library/characters`), in Show and Perform the Session sheet's, else the rig's;
//! `--toon` overrides both. The registry is asked once, on a thread, and kept in the cache for the next offline start.
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// the toon to read: `forced` (`--toon`), else the rig's in Build, else the Session sheet's and without one the rig's
pub fn wanted(forced: Option<&str>, build: bool, rig: Option<&str>, place: Option<&str>) -> Option<String> {
    let place = place.filter(|p| !p.trim().is_empty());
    let t = forced.or(if build { rig } else { place.or(rig) })?.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// the character a rig subject names (`character.catpope` → `catpope`); items and others have none
pub fn character(subject: &str) -> Option<&str> {
    subject.strip_prefix("character.").filter(|c| !c.is_empty())
}

/// the character's own toon in the library's registry (`{ characters: [{ id, own_toon? }] }`)
pub fn own_toon(registry: &str, character: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(registry).ok()?;
    let list = v["characters"].as_array().or_else(|| v["content"]["characters"].as_array())?;
    let c = list.iter().find(|c| c["id"].as_str() == Some(character))?;
    c["own_toon"].as_str().filter(|t| !t.is_empty()).map(str::to_string)
}

/// The rig's toon: asked on a thread, there once it answered.
pub struct RigToon {
    rx: Option<Receiver<Option<String>>>,
    pub toon: Option<String>,
}

impl RigToon {
    /// none asked (`--toon`, a check run)
    pub fn none() -> RigToon {
        RigToon { rx: None, toon: None }
    }

    pub fn new(api: &str, subject: &str, cache: &Path) -> RigToon {
        let Some(character) = character(subject).map(str::to_string) else { return RigToon::none() };
        let (tx, rx) = channel();
        let (url, kept) = (format!("{api}/library/characters"), kept(cache));
        std::thread::spawn(move || {
            let fetched = ureq::AgentBuilder::new().timeout(Duration::from_secs(4)).build().get(&url).call().ok().and_then(|r| r.into_string().ok());
            if let Some(text) = &fetched {
                let _ = kept.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&kept, text));
            }
            let text = fetched.or_else(|| std::fs::read_to_string(&kept).ok()).unwrap_or_default();
            let _ = tx.send(own_toon(&text, &character));
        });
        RigToon { rx: Some(rx), toon: None }
    }

    pub fn poll(&mut self) {
        if let Some(Ok(toon)) = self.rx.as_ref().map(Receiver::try_recv) {
            (self.toon, self.rx) = (toon, None);
        }
    }
}

/// the registry as last asked (rigs.rs reads it to list the toon's rigs)
pub fn kept(cache: &Path) -> PathBuf {
    cache.join("library").join("characters.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_toon_follows_what_is_open() {
        assert_eq!(wanted(None, true, Some("catpope"), Some("feilchen")).as_deref(), Some("catpope"), "Build: the rig's");
        assert_eq!(wanted(None, false, Some("catpope"), Some("feilchen")).as_deref(), Some("feilchen"), "Show / Perform: the Session sheet's");
        assert_eq!(wanted(Some("other"), false, Some("catpope"), Some("feilchen")).as_deref(), Some("other"), "--toon overrides");
        assert_eq!(wanted(None, false, Some("catpope"), Some(" ")).as_deref(), Some("catpope"), "no toon picked in the Session sheet: the rig's");
        assert_eq!(wanted(None, false, None, None), None, "none at all");
        assert_eq!(wanted(None, true, None, Some("feilchen")), None);
    }

    #[test]
    fn a_rigs_toon_is_its_characters_own() {
        let registry = r#"{ "characters": [{ "id": "nun" }, { "id": "catpope", "own_toon": "catpope" }] }"#;
        assert_eq!(character("character.catpope"), Some("catpope"));
        assert_eq!(character("item.incense_burner"), None);
        assert_eq!(own_toon(registry, "catpope").as_deref(), Some("catpope"));
        assert_eq!(own_toon(registry, "nun"), None);
        assert_eq!(own_toon("not json", "catpope"), None);
    }
}
