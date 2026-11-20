//! Show documents through the API (#90 `/api/shows`, role `live`): load the newest draft, save a new draft (the server
//! stamps the version). Each document read or saved is also kept in the cache (`<cache>/shows/<id>.json`), so the editor
//! opens a show offline; the API stays the source (RULES E3). The token: the editor's sign-in (sign_in.rs; `SCUTOONS_TOKEN`
//! overrides it). Drafts of any entity (`draft`, `save_draft`) and of a deck scene (`scene_draft`, `save_scene`): read
//! with their version, saved on it as `base`.
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

/// a module build is at most 4 MB (the API refuses larger ones); a little over it is read so a larger file fails its hash
const MAX_MODULE_BYTES: u64 = 5 << 20;

#[derive(Clone)]
pub struct Api {
    /// `http://127.0.0.1:9000/api`, `https://api.scutoons.com/api`
    pub base: String,
    auth: crate::sign_in::Auth,
    cache: PathBuf,
    /// where what is sent waits until the server has it
    pub outbox: crate::autosave::Outbox,
}

impl Api {
    pub fn new(base: String) -> Api {
        Api { base, auth: crate::sign_in::current(), cache: crate::cache_dir().join("shows"), outbox: crate::autosave::Outbox::current() }
    }

    /// the signed-in member's name (a note's author); empty when nobody is
    pub fn who(&self) -> String {
        self.auth.name().unwrap_or_default()
    }

    /// #537's UI-model test: a test API and its own cache
    #[cfg(test)]
    pub fn with_cache(base: String, cache: PathBuf) -> Api {
        Api { base, auth: crate::sign_in::Auth::fixed(None), cache, outbox: crate::autosave::Outbox::none() }
    }

    /// the same API at another address (a test's server coming back)
    #[cfg(test)]
    pub fn with_base(&self, base: &str) -> Api {
        Api { base: base.into(), ..self.clone() }
    }

    fn agent() -> ureq::Agent {
        ureq::AgentBuilder::new().timeout(Duration::from_secs(4)).build()
    }

    fn auth(&self, r: ureq::Request) -> ureq::Request {
        match self.auth.token() {
            Some(t) => r.set("Authorization", &format!("Bearer {t}")),
            None => r,
        }
    }

    fn keep(&self, id: &str, text: &str) {
        let _ = std::fs::create_dir_all(&self.cache).and_then(|_| std::fs::write(self.cache.join(format!("{id}.json")), text));
    }

    /// The newest draft of show `id` (its document's JSON) and its version – from the API, else the cache (and why the
    /// API did not answer).
    pub fn load(&self, id: &str) -> Result<Loaded, String> {
        let url = format!("{}/shows/{id}?draft=1", self.base);
        let api = self.auth(Self::agent().get(&url)).call().map_err(|e| e.to_string()).and_then(|r| r.into_json::<serde_json::Value>().map_err(|e| e.to_string()));
        match api.map(|v| (v.get("content").cloned(), v.pointer("/version/id").and_then(serde_json::Value::as_u64))) {
            Ok((Some(doc), version)) if doc.is_object() => {
                let text = serde_json::to_string_pretty(&doc).unwrap_or_default();
                self.keep(id, &text);
                Ok(Loaded { text, version, why: None })
            }
            other => {
                let why = match other {
                    Err(e) => e,
                    _ => "no document".into(),
                };
                let text = std::fs::read_to_string(self.cache.join(format!("{id}.json"))).map_err(|_| format!("{url}: {why}"))?;
                Ok(Loaded { text, version: None, why: Some(why) })
            }
        }
    }

    /// #539: every toon's story events (`GET /api/creator/story-events`; the server asks a content editor's token): the
    /// answer's text, or its HTTP status (none: no answer) and what the transport said. deck-show story.rs reads both.
    pub fn story_events(&self) -> Result<String, (Option<u16>, String)> {
        let url = deck_show::story::url(&self.base);
        match self.auth(Self::agent().get(&url)).call() {
            Ok(r) => r.into_string().map_err(|e| (None, format!("{url}: {e}"))),
            Err(ureq::Error::Status(code, _)) => {
                if code == 401 {
                    self.auth.refused();
                }
                Err((Some(code), String::new()))
            }
            Err(e) => Err((None, format!("{url}: {e}"))),
        }
    }

    /// Save `doc` as a new draft on `base` (none known: on whatever the server has); the version the server gave it.
    /// Kept in the cache either way.
    pub fn save(&self, id: &str, doc: &serde_json::Value, base: Option<u64>) -> Result<u64, Refusal> {
        self.keep(id, &serde_json::to_string_pretty(doc).unwrap_or_default());
        let url = format!("{}/shows/{id}", self.base);
        let mut body = serde_json::json!({ "content": doc, "message": "deck editor" });
        if let Some(b) = base {
            body["base"] = b.into();
        }
        let r = match self.auth(Self::agent().put(&url)).send_json(body) {
            Ok(r) => r,
            Err(ureq::Error::Status(code, r)) => return Err(self.refusal(&format!("show:{id}"), code, &r.into_json().unwrap_or_default())),
            Err(e) => return Err(Refusal::Offline(format!("{url}: {e} – kept in the cache"))),
        };
        let v: serde_json::Value = r.into_json().map_err(|e| Refusal::Other(format!("{url}: {e}")))?;
        v.pointer("/version/id").and_then(serde_json::Value::as_u64).ok_or_else(|| Refusal::Other(format!("{url}: the answer has no version")))
    }
}

/// a show read: its document's JSON, its version (none from the cache), why the API did not answer
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub text: String,
    pub version: Option<u64>,
    pub why: Option<String>,
}

impl Api {
    /// the catalog's documents of `kind` (`GET /api/catalog/docs/rigs` …): the answer's text, else what went wrong in words
    pub fn catalog(&self, kind: &str) -> Result<String, String> {
        let url = format!("{}/catalog/docs/{kind}", self.base);
        match self.auth(Self::agent().get(&url)).call() {
            Ok(r) => r.into_string().map_err(|e| format!("{url}: {e}")),
            Err(ureq::Error::Status(code, r)) => Err(self.refused_words(kind, code, &r.into_json().unwrap_or_default())),
            Err(e) => Err(format!("{url}: {e}")),
        }
    }

    /// The newest draft of entity `key` (`rig:character.catpope` …) and its version – the `base` of the next save;
    /// None = never saved.
    pub fn draft(&self, key: &str) -> Result<Option<Draft>, String> {
        let url = format!("{}/entities/{key}?draft=1", self.base);
        match self.auth(Self::agent().get(&url)).call() {
            Ok(r) => {
                let version = r.header("X-Content-Version").and_then(|v| v.parse().ok());
                let content = r.into_json().map_err(|e| format!("{url}: {e}"))?;
                Ok(Some(Draft { content, version }))
            }
            Err(ureq::Error::Status(404, _)) => Ok(None),
            Err(ureq::Error::Status(code, r)) => Err(self.refused_words(key, code, &r.into_json().unwrap_or_default())),
            Err(e) => Err(format!("{url}: {e}")),
        }
    }

    /// Save `content` as a new draft of `key` on `base` (the version it was read at; None = the first) → the new version.
    /// Someone else's newer save is a 409, said in words: nothing is overwritten.
    pub fn save_draft(&self, key: &str, content: &serde_json::Value, base: Option<u64>, message: &str) -> Result<u64, Refusal> {
        self.put(&format!("entities/{key}"), key, content, base, message)
    }

    /// A deck scene's draft goes through `/api/scenes/<id>` (the server stamps its `version`), the same way.
    pub fn save_scene(&self, id: &str, content: &serde_json::Value, base: Option<u64>, message: &str) -> Result<u64, Refusal> {
        self.put(&format!("scenes/{id}"), &format!("scene:{id}"), content, base, message)
    }

    /// The newest draft of deck scene `id` and its version; None = never saved.
    pub fn scene_draft(&self, id: &str) -> Result<Option<Draft>, String> {
        let url = format!("{}/scenes/{id}?draft=1", self.base);
        match self.auth(Self::agent().get(&url)).call() {
            Ok(r) => {
                let v: serde_json::Value = r.into_json().map_err(|e| format!("{url}: {e}"))?;
                Ok(Some(Draft { content: v["content"].clone(), version: v.pointer("/version/id").and_then(|x| x.as_u64()) }))
            }
            Err(ureq::Error::Status(404, _)) => Ok(None),
            Err(ureq::Error::Status(code, r)) => Err(self.refused_words(&format!("scene:{id}"), code, &r.into_json().unwrap_or_default())),
            Err(e) => Err(format!("{url}: {e}")),
        }
    }

    /// every deck scene with its newest draft (`GET /api/scenes`, a content editor's answer): the answer's text
    pub fn scenes(&self) -> Result<String, String> {
        let url = format!("{}/scenes", self.base);
        match self.auth(Self::agent().get(&url)).call() {
            Ok(r) => r.into_string().map_err(|e| format!("{url}: {e}")),
            Err(ureq::Error::Status(code, r)) => Err(self.refused_words("scenes", code, &r.into_json().unwrap_or_default())),
            Err(e) => Err(format!("{url}: {e}")),
        }
    }

    fn put(&self, path: &str, key: &str, content: &serde_json::Value, base: Option<u64>, message: &str) -> Result<u64, Refusal> {
        let url = format!("{}/{path}", self.base);
        let body = serde_json::json!({ "content": content, "base": base, "message": message });
        match self.auth(Self::agent().put(&url)).send_json(body) {
            Ok(r) => {
                let v: serde_json::Value = r.into_json().map_err(|e| Refusal::Other(format!("{url}: {e}")))?;
                v.pointer("/version/id").and_then(|x| x.as_u64()).ok_or_else(|| Refusal::Other(format!("{url}: the answer has no version")))
            }
            Err(ureq::Error::Status(code, r)) => Err(self.refusal(key, code, &r.into_json().unwrap_or_default())),
            Err(e) => Err(Refusal::Offline(format!("{url}: {e} – not saved"))),
        }
    }

    /// A catalog module as the API answers it (`GET /api/modules/<id>`: its document, its build's hash and url)
    pub fn module(&self, id: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/{}", self.base, deck_space::rig::build::path(id));
        match self.auth(Self::agent().get(&url)).call() {
            Ok(r) => r.into_json().map_err(|e| format!("{url}: {e}")),
            Err(ureq::Error::Status(404, _)) => Err(format!("module:{id} is not in the catalog")),
            Err(ureq::Error::Status(401, _)) => {
                self.auth.refused();
                Err(format!("module:{id}: signed out – approve the editor on the account page to hear it"))
            }
            Err(ureq::Error::Status(code, r)) => Err(refused_words(&format!("module:{id}"), code, &r.into_json().unwrap_or_default())),
            Err(e) => Err(format!("{url}: {e}")),
        }
    }

    /// the asset library's first page (`GET /api/assets`, an editor's): the answer's text
    pub fn assets(&self) -> Result<String, String> {
        let url = format!("{}/assets?pageSize=200", self.base);
        match self.auth(Self::agent().get(&url)).call() {
            Ok(r) => r.into_string().map_err(|e| format!("{url}: {e}")),
            Err(ureq::Error::Status(code, r)) => Err(self.refused_words("assets", code, &r.into_json().unwrap_or_default())),
            Err(e) => Err(format!("{url}: {e}")),
        }
    }

    /// the asset a file belongs to (`GET /api/assets/lookup`, an editor's): its detail – `asset` (with its `edits`) and
    /// `versions` (each file's time map); none when the registry does not know the file
    pub fn asset_of(&self, path: &str) -> Result<Option<serde_json::Value>, String> {
        let url = format!("{}/assets/lookup", self.base);
        match self.auth(Self::agent().get(&url)).query("path", path).call() {
            Ok(r) => r.into_json::<serde_json::Value>().map(|v| Some(v["asset"].clone()).filter(|a| a.is_object())).map_err(|e| format!("{url}: {e}")),
            Err(ureq::Error::Status(code, r)) => Err(self.refused_words("the asset", code, &r.into_json().unwrap_or_default())),
            Err(e) => Err(format!("{url}: {e}")),
        }
    }

    /// The asset's edit list onto it (`PUT /api/assets/{id}` `edits`: deck-space `asset_edits::to_json`, null = none)
    pub fn save_asset_edits(&self, id: &str, edits: &serde_json::Value) -> Result<(), String> {
        let url = format!("{}/assets/{id}", self.base);
        match self.auth(Self::agent().put(&url)).send_json(serde_json::json!({ "edits": edits })) {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(code, r)) => Err(self.refused_words("the asset's edits", code, &r.into_json().unwrap_or_default())),
            Err(e) => Err(format!("{url}: {e}")),
        }
    }

    /// The file at `url` (a module's build on the CDN, up to a few MB)
    pub fn bytes(&self, url: &str) -> Result<Vec<u8>, String> {
        Self::bytes_of(url)
    }

    pub fn bytes_of(url: &str) -> Result<Vec<u8>, String> {
        let r = ureq::AgentBuilder::new().timeout(Duration::from_secs(30)).build().get(url).call().map_err(|e| format!("{url}: {e}"))?;
        let mut out = Vec::new();
        r.into_reader().take(MAX_MODULE_BYTES).read_to_end(&mut out).map_err(|e| format!("{url}: {e}"))?;
        Ok(out)
    }

    fn refused_words(&self, key: &str, code: u16, body: &serde_json::Value) -> String {
        if code == 401 {
            self.auth.refused();
        }
        refused_words(key, code, body)
    }

    fn refusal(&self, key: &str, code: u16, body: &serde_json::Value) -> Refusal {
        let words = self.refused_words(key, code, body);
        match code {
            409 => Refusal::Conflict { theirs: body.pointer("/latest/id").and_then(serde_json::Value::as_u64), words },
            _ => Refusal::Other(words),
        }
    }
}

/// why a save did not land: no answer (kept locally, sent again later), someone else's newer version, or a refusal
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    Offline(String),
    Conflict { theirs: Option<u64>, words: String },
    Other(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Offline(w) | Refusal::Other(w) | Refusal::Conflict { words: w, .. } => f.write_str(w),
        }
    }
}

/// a document's newest draft: its JSON and the version a save names as `base`
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub content: serde_json::Value,
    pub version: Option<u64>,
}

/// what an API refusal means for a draft of `key`, in words
pub fn refused_words(key: &str, code: u16, body: &serde_json::Value) -> String {
    let said = body["message"].as_str().unwrap_or("");
    match code {
        409 => {
            let latest = &body["latest"];
            let who = latest["author_email"].as_str().filter(|w| !w.is_empty()).map(|w| format!(" by {w}")).unwrap_or_default();
            let when = latest["created_at"].as_str().map(|w| format!(" at {w}")).unwrap_or_default();
            let theirs = latest["id"].as_u64().map(|v| format!(" (v{v})")).unwrap_or_default();
            format!("{key}: someone saved a newer version{theirs}{who}{when} – nothing was overwritten; yours is kept locally")
        }
        401 => format!("{key}: signed out – approve the editor on the account page, then save again"),
        403 => format!("{key}: this account may not do that{}", if said.is_empty() { String::new() } else { format!(" – {said}") }),
        _ => format!("{key}: HTTP {code}{}", if said.is_empty() { String::new() } else { format!(" – {said}") }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_draft_is_said_in_words() {
        let conflict = serde_json::json!({ "error": "conflict", "latest": { "author_email": "ana@scutoons.test", "created_at": "2026-11-15T10:00:00Z" } });
        assert_eq!(refused_words("rig:character.catpope", 409, &conflict), "rig:character.catpope: someone saved a newer version by ana@scutoons.test at 2026-11-15T10:00:00Z – nothing was overwritten; yours is kept locally");
        assert!(refused_words("k", 409, &serde_json::json!({ "latest": { "id": 830 } })).starts_with("k: someone saved a newer version (v830) – "));
        assert!(refused_words("k", 409, &serde_json::json!({})).starts_with("k: someone saved a newer version – "));
        assert!(refused_words("k", 401, &serde_json::json!({})).contains("signed out"));
        assert_eq!(refused_words("k", 403, &serde_json::json!({ "message": "This needs the creator role." })), "k: this account may not do that – This needs the creator role.");
    }

    /// no API (nothing listens on port 9): a draft can be neither read nor saved, and says where it tried
    #[test]
    fn offline_drafts_say_so() {
        let api = Api::with_cache("http://127.0.0.1:9/api".into(), std::env::temp_dir().join("deck-editor-api-test"));
        assert!(api.draft("moods").unwrap_err().starts_with("http://127.0.0.1:9/api/entities/moods?draft=1"));
        assert!(matches!(api.save_draft("moods", &serde_json::json!({}), Some(3), "deck editor"), Err(Refusal::Offline(w)) if w.ends_with("not saved")));
        assert!(api.save_scene("duet", &serde_json::json!({}), None, "deck editor").unwrap_err().to_string().starts_with("http://127.0.0.1:9/api/scenes/duet: "));
        assert!(matches!(api.save("show", &serde_json::json!({}), None), Err(Refusal::Offline(_))));
        assert!(api.scene_draft("duet").unwrap_err().starts_with("http://127.0.0.1:9/api/scenes/duet?draft=1"));
    }
}
