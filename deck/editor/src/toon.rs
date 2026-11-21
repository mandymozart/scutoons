//! Which toons' address book the editor reads: the one of what is open – in Build the rig's (its character's `own_toon`
//! in the library's registry, `GET /api/library/characters`), in Show and Perform the show's toons (#708: several, the
//! home first); `--toon` overrides both. The registry is asked once, on a thread, and kept in the cache for the next offline start.
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// the toons to read: `forced` (`--toon`), else the rig's in Build, else the show's (none: an empty list)
pub fn wanted(forced: Option<&str>, build: bool, rig: Option<&str>, show: &[String]) -> Vec<String> {
    let list: Vec<&str> = match (forced, build) {
        (Some(f), _) => vec![f],
        (None, true) => rig.into_iter().collect(),
        (None, false) => show.iter().map(String::as_str).collect(),
    };
    list.into_iter().map(str::trim).filter(|t| !t.is_empty()).map(str::to_string).collect()
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
        let show = ["feilchen".to_string(), "catpope".to_string()];
        assert_eq!(wanted(None, true, Some("catpope"), &show), ["catpope"], "Build: the rig's");
        assert_eq!(wanted(None, false, Some("catpope"), &show), ["feilchen", "catpope"], "Show / Perform: the show's toons, the home first");
        assert_eq!(wanted(Some("other"), false, Some("catpope"), &show), ["other"], "--toon overrides");
        assert!(wanted(None, false, Some("catpope"), &[]).is_empty(), "a show without its toon: none, not the rig's");
        assert!(wanted(None, false, None, &[]).is_empty(), "none at all");
        assert!(wanted(None, true, None, &show).is_empty());
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
