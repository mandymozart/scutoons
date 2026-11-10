//! Show documents through the API (#90 `/api/shows`, role `live`): load the newest draft, save a new draft (the server
//! stamps the version). Each document read or saved is also kept in the cache (`<cache>/shows/<id>.json`), so the editor
//! opens a show offline; the API stays the source (RULES E3). The token: `SCUTOONS_TOKEN` (a member with the role `live`).
use std::path::PathBuf;
use std::time::Duration;

pub struct Api {
    /// `http://127.0.0.1:9000/api`, `https://api.scutoons.com/api`
    pub base: String,
    token: Option<String>,
    cache: PathBuf,
}

impl Api {
    pub fn new(base: String) -> Api {
        Api { base, token: std::env::var("SCUTOONS_TOKEN").ok().filter(|t| !t.is_empty()), cache: deck_native::cache::default_dir().join("shows") }
    }

    fn agent() -> ureq::Agent {
        ureq::AgentBuilder::new().timeout(Duration::from_secs(4)).build()
    }

    fn auth(&self, r: ureq::Request) -> ureq::Request {
        match &self.token {
            Some(t) => r.set("Authorization", &format!("Bearer {t}")),
            None => r,
        }
    }

    fn keep(&self, id: &str, text: &str) {
        let _ = std::fs::create_dir_all(&self.cache).and_then(|_| std::fs::write(self.cache.join(format!("{id}.json")), text));
    }

    /// The newest draft of show `id` (its document's JSON) – from the API, else the cache (and why the API did not answer).
    pub fn load(&self, id: &str) -> Result<(String, Option<String>), String> {
        let url = format!("{}/shows/{id}?draft=1", self.base);
        let api = self.auth(Self::agent().get(&url)).call().map_err(|e| e.to_string()).and_then(|r| r.into_json::<serde_json::Value>().map_err(|e| e.to_string()));
        match api.map(|v| v.get("content").cloned()) {
            Ok(Some(doc)) if doc.is_object() => {
                let text = serde_json::to_string_pretty(&doc).unwrap_or_default();
                self.keep(id, &text);
                Ok((text, None))
            }
            other => {
                let why = match other {
                    Err(e) => e,
                    _ => "no document".into(),
                };
                let text = std::fs::read_to_string(self.cache.join(format!("{id}.json"))).map_err(|_| format!("{url}: {why}"))?;
                Ok((text, Some(why)))
            }
        }
    }

    /// Save `doc` as a new draft; the version the server gave it. Kept in the cache either way.
    pub fn save(&self, id: &str, doc: &serde_json::Value) -> Result<u64, String> {
        self.keep(id, &serde_json::to_string_pretty(doc).unwrap_or_default());
        let url = format!("{}/shows/{id}", self.base);
        let body = serde_json::json!({ "content": doc, "message": "deck editor" });
        let r = self.auth(Self::agent().put(&url)).send_json(body).map_err(|e| format!("{url}: {e} – kept in the cache"))?;
        let v: serde_json::Value = r.into_json().map_err(|e| e.to_string())?;
        Ok(v.pointer("/version/version").or_else(|| v.get("version")).and_then(|x| x.as_u64()).unwrap_or_default())
    }
}
