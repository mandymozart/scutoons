//! Show media for the outlet (#186, LIVE.md B.5.6): `media.preload` items fetched with the deck token, cached on
//! DECKDATA (`<cache>/live/`), answered with `media.ready`; set references resolved through `Resolve` (#193).
//!
//! - **Sets** (#193: a use names set + spot + mode, never a file): `Resolve` turns one into a `Source`. The set data
//!   model is not built yet, so `LocalSets` – today's resolver – looks on the deck first (`cache::set_file` for 3D, the
//!   files the space node reads and the sync (sync.rs, #195) fetches; `<sets>/<set>/<spot>.<mode>.<ext>` for stills and
//!   videos) and otherwise uses the item's `url`.
//! - **Fetching** (`Fetch`): `HttpFetch` (ureq, `Authorization: Bearer <deck token>`, streamed into a `.part` file,
//!   then renamed); tests use the file system or a local server. CDN paths are immutable (a changed file gets a new
//!   name), so a cached URL is never fetched again.
//! - **The token** (`Token`): `Linked` = what deck-link's daemon keeps in /run/deck-link (#180: approved → a ~1 h token, renewed; pending
//!   → the code, which the HUD shows as the reason a medium is missing).
use super::plan::MediaRef;
use protocol::{MediaItem, MediaReady, SpotMode};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

/// Where a medium comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Url(String),
    File(PathBuf),
}

/// set + spot + mode → where it is (#193). Err = why it can't be found (the HUD's reason).
pub trait Resolve: Send + Sync {
    fn spot(&self, set: &str, spot: Option<&str>, mode: SpotMode) -> Result<Source, String>;
}

/// Today's resolver: the deck's own files (`--sets`, default `<cache>/sets`).
pub struct LocalSets {
    /// a set's 3D: `cache::set_file` (`<sets>/<set>/<set>.<quality>.glb`); stills and videos per spot:
    /// `<sets>/<set>/<spot>.<still|video>.<ext>` (`default` = no spot)
    pub sets: PathBuf,
    /// the 3D level (low = the X1's)
    pub quality: String,
    /// the sync fetches a set's 3D when the space node loads it (its gate): a missing one resolves to where it lands
    pub synced: bool,
}

const STILL_EXT: &[&str] = &["webp", "png"];
const VIDEO_EXT: &[&str] = &["webm", "mp4"];

impl Resolve for LocalSets {
    fn spot(&self, set: &str, spot: Option<&str>, mode: SpotMode) -> Result<Source, String> {
        let safe = |s: &str| !s.is_empty() && !s.contains(['/', '\\', ':']) && s != "..";
        if !safe(set) || !spot.map_or(true, safe) {
            return Err(format!("set '{set}' spot '{}': not a plain name", spot.unwrap_or("")));
        }
        let spot_name = spot.unwrap_or("default");
        let tried: Vec<PathBuf> = match mode {
            SpotMode::Model3d => {
                let file = crate::cache::set_file(&self.sets, set, &self.quality);
                if self.synced {
                    return Ok(Source::File(file));
                }
                vec![file]
            }
            SpotMode::Still => STILL_EXT.iter().map(|e| self.sets.join(set).join(format!("{spot_name}.still.{e}"))).collect(),
            SpotMode::Video => VIDEO_EXT.iter().map(|e| self.sets.join(set).join(format!("{spot_name}.video.{e}"))).collect(),
        };
        tried.iter().find(|p| p.is_file()).map(|p| Source::File(p.clone())).ok_or_else(|| {
            tracing::debug!("live: set {set} {spot_name} ({}): looked for {}", mode.as_str(), tried[0].display());
            format!("set {set} {spot_name} ({}) is not on this deck – no set data yet (#193)", mode.as_str())
        })
    }
}

/// The deck token for show media (#180). Ok(None) = no token (public files still load); Err = why there is none.
pub trait Token: Send + Sync {
    fn bearer(&self) -> Result<Option<String>, String>;
}

/// no token (a deck before pairing; tests)
pub struct NoToken;

impl Token for NoToken {
    fn bearer(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
}

/// a fixed token (tests, a dev deck: `DECK_TOKEN`)
pub struct Fixed(pub String);

impl Token for Fixed {
    fn bearer(&self) -> Result<Option<String>, String> {
        Ok(Some(self.0.clone()))
    }
}

/// The deck's token as deck-link's daemon keeps it (`/run/deck-link`, #180) or `DECK_TOKEN`: paired → its token; waiting
/// for a member → the code to pair with, as the reason; no daemon → no token.
pub struct Linked(pub deck_link::TokenSource);

impl Token for Linked {
    fn bearer(&self) -> Result<Option<String>, String> {
        match (self.0.get(), self.0.pending_code()) {
            (Some(t), _) => Ok(Some(t)),
            (None, Some(code)) => Err(format!("the deck is not paired – code {}", deck_link::format_code(&code))),
            (None, None) => Ok(None),
        }
    }
}

/// A GET into a file. Err = the reason, short (the HUD's and `media.ready`'s `why`).
pub trait Fetch: Send + Sync {
    fn get(&self, url: &str, bearer: Option<&str>, to: &Path) -> Result<(), String>;
}

pub struct HttpFetch {
    agent: ureq::Agent,
}

impl Default for HttpFetch {
    fn default() -> Self {
        HttpFetch { agent: ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(5)).timeout_read(Duration::from_secs(30)).build() }
    }
}

impl Fetch for HttpFetch {
    fn get(&self, url: &str, bearer: Option<&str>, to: &Path) -> Result<(), String> {
        let mut req = self.agent.get(url);
        if let Some(t) = bearer {
            req = req.set("Authorization", &format!("Bearer {t}"));
        }
        let r = req.call().map_err(|e| match e {
            ureq::Error::Status(code, _) => format!("HTTP {code}"),
            ureq::Error::Transport(t) => format!("offline ({})", t.kind()),
        })?;
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let part = to.with_extension("part");
        let mut file = std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
        // show media are big (high-res video, stems) – streamed, capped at 4 GiB
        std::io::copy(&mut r.into_reader().take(4 << 30), &mut file).map_err(|e| format!("download broke off ({e})"))?;
        drop(file);
        std::fs::rename(&part, to).map_err(|e| format!("{}: {e}", to.display()))
    }
}

/// FNV-1a over the URL: the cache file's prefix (no dependency for a hash that only names files)
fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// The outlet's media: where each preloaded id is, the cache, the resolver, the fetcher, the token.
pub struct Media {
    pub cache: PathBuf,
    pub resolve: Box<dyn Resolve>,
    pub fetch: Box<dyn Fetch>,
    pub token: Box<dyn Token>,
    /// id → the local file (filled by `preload`, read by the composer)
    pub local: Mutex<HashMap<String, PathBuf>>,
}

impl Media {
    /// `<cache>/live/<hash>-<file name>` (the name kept readable for a look on DECKDATA)
    pub fn cache_path(&self, url: &str) -> PathBuf {
        let name: String = url.rsplit('/').next().unwrap_or("").split(['?', '#']).next().unwrap_or("").chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')).take(64).collect();
        self.cache.join("live").join(format!("{:016x}-{name}", fnv(url)))
    }

    /// A source → a local file: a file as it is, a URL from the cache or fetched (with the token) into it.
    pub fn localize(&self, source: &Source) -> Result<PathBuf, String> {
        match source {
            Source::File(p) if p.is_file() => Ok(p.clone()),
            Source::File(p) => Err(format!("{} is gone", p.display())),
            Source::Url(url) => {
                let to = self.cache_path(url);
                if to.is_file() {
                    return Ok(to);
                }
                let (bearer, unpaired) = match self.token.bearer() {
                    Ok(t) => (t, None),
                    Err(why) => (None, Some(why)),
                };
                self.fetch.get(url, bearer.as_deref(), &to).map(|_| to).map_err(|e| match (e.as_str(), unpaired) {
                    ("HTTP 401" | "HTTP 403", Some(why)) => format!("{e} – {why}"),
                    _ => e,
                })
            }
        }
    }

    /// Where a preload item comes from: its url, or its set reference resolved (a local set file wins over the url).
    pub fn source(&self, item: &MediaItem) -> Result<Source, String> {
        if let Some(set) = &item.set {
            let mode = item.mode.unwrap_or(SpotMode::Still);
            match (self.resolve.spot(set, item.spot.as_deref(), mode), &item.url) {
                (Ok(s), _) => return Ok(s),
                (Err(_), Some(url)) => return Ok(Source::Url(url.clone())),
                (Err(why), None) => return Err(why),
            }
        }
        item.url.clone().map(Source::Url).ok_or_else(|| format!("{}: neither a url nor a set", item.id))
    }

    /// A layer's medium → a local file: a preloaded id (Err while it is still missing), or a set reference resolved
    /// now (a set's 3D, a still that was not in the preload).
    pub fn file_of(&self, r: &MediaRef) -> Result<PathBuf, String> {
        match r {
            MediaRef::Item(id) => self.local.lock().unwrap().get(id).cloned().ok_or_else(|| format!("'{id}' is not preloaded (yet)")),
            MediaRef::Set { set, spot, mode } => self.localize(&self.resolve.spot(set, spot.as_deref(), *mode)?),
        }
    }

    /// One round of a preload: every item not local yet is fetched; the answer for the conductor. `items` in order.
    pub fn preload(&self, outlet: &str, items: &[MediaItem]) -> MediaReady {
        let mut ready = vec![];
        let mut missing = vec![];
        let mut why = HashMap::new();
        for item in items {
            let known = self.local.lock().unwrap().get(&item.id).filter(|p| p.is_file()).cloned();
            let got = match known {
                Some(p) => Ok(p),
                None => self.source(item).and_then(|s| self.localize(&s)),
            };
            match got {
                Ok(p) => {
                    self.local.lock().unwrap().insert(item.id.clone(), p);
                    ready.push(item.id.clone());
                }
                Err(e) => {
                    tracing::warn!("live: media {} missing: {e}", item.id);
                    missing.push(item.id.clone());
                    why.insert(item.id.clone(), e);
                }
            }
        }
        MediaReady { outlet: outlet.into(), ready, missing, why: (!why.is_empty()).then_some(why) }
    }
}

/// The retries of a preload with something missing (s): quick at first, then every 30 s.
pub const RETRY_S: [u64; 5] = [2, 5, 10, 20, 30];

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// a fetcher over a folder: `url` = `mock://<name>` → `<root>/<name>`; `secret` needs that bearer (else 401)
    pub struct FolderFetch {
        pub root: PathBuf,
        pub secret: Option<String>,
        pub calls: Mutex<Vec<String>>,
    }

    impl Fetch for FolderFetch {
        fn get(&self, url: &str, bearer: Option<&str>, to: &Path) -> Result<(), String> {
            self.calls.lock().unwrap().push(url.to_string());
            if self.secret.is_some() && bearer != self.secret.as_deref() {
                return Err("HTTP 401".into());
            }
            let from = self.root.join(url.strip_prefix("mock://").ok_or("offline (no route)")?);
            let bytes = std::fs::read(&from).map_err(|_| "HTTP 404".to_string())?;
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::write(to, bytes).map_err(|e| e.to_string())
        }
    }

    pub fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("deck-live-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn item(id: &str, url: Option<&str>, set: Option<(&str, &str, SpotMode)>) -> MediaItem {
        MediaItem {
            id: id.into(),
            url: url.map(Into::into),
            set: set.map(|s| s.0.into()),
            spot: set.map(|s| s.1.into()),
            mode: set.map(|s| s.2),
            bytes: None,
        }
    }

    fn media(root: &Path, token: Box<dyn Token>) -> Media {
        Media {
            cache: root.join("cache"),
            resolve: Box::new(LocalSets { sets: root.join("sets"), quality: "low".into(), synced: false }),
            fetch: Box::new(FolderFetch { root: root.join("cdn"), secret: Some("T".into()), calls: Mutex::new(vec![]) }),
            token,
            local: Mutex::new(HashMap::new()),
        }
    }

    #[test]
    fn preload_fetches_with_the_token_caches_and_answers_ready() {
        let root = temp_dir("preload");
        for (p, b) in [("cdn/live/choir.webm", "choir"), ("sets/cathedral/cathedral.low.glb", "glb"), ("sets/cathedral/pulpit.still.png", "png")] {
            std::fs::create_dir_all(root.join(p).parent().unwrap()).unwrap();
            std::fs::write(root.join(p), b).unwrap();
        }
        let m = media(&root, Box::new(Fixed("T".into())));
        let items = [
            item("choir", Some("mock://live/choir.webm"), None),
            item("nave", None, Some(("cathedral", "nave-centre", SpotMode::Model3d))),
            item("pulpit", None, Some(("cathedral", "pulpit", SpotMode::Still))),
            item("organ", Some("mock://live/organ.webm"), None),
            item("crypt", None, Some(("cathedral", "crypt", SpotMode::Still))),
        ];
        let r = m.preload("left", &items);
        assert_eq!(r.ready, ["choir", "nave", "pulpit"]);
        assert_eq!(r.missing, ["organ", "crypt"]);
        let why = r.why.unwrap();
        assert_eq!(why["organ"], "HTTP 404");
        assert_eq!(why["crypt"], "set cathedral crypt (still) is not on this deck – no set data yet (#193)");
        let choir = m.file_of(&MediaRef::Item("choir".into())).unwrap();
        assert!(choir.starts_with(root.join("cache").join("live")) && choir.to_string_lossy().ends_with("-choir.webm"));
        assert_eq!(std::fs::read_to_string(&choir).unwrap(), "choir");
        assert_eq!(m.file_of(&MediaRef::Item("nave".into())).unwrap(), root.join("sets/cathedral/cathedral.low.glb"));
        assert!(m.file_of(&MediaRef::Item("organ".into())).unwrap_err().contains("not preloaded"));

        // the organ arrives on the CDN: the next round (a retry) fetches only what is missing
        std::fs::write(root.join("cdn/live/organ.webm"), "organ").unwrap();
        let r = m.preload("left", &items[..4]);
        assert_eq!((r.ready.len(), r.missing.len(), r.why), (4, 0, None));
        // a new Media on the same cache (a deck restart) finds the files without the network
        let again = media(&root, Box::new(NoToken));
        assert_eq!(again.preload("left", &items[..4]).missing, Vec::<String>::new());
    }

    #[test]
    fn without_a_token_the_reason_is_the_pairing_code() {
        let root = temp_dir("unpaired");
        std::fs::create_dir_all(root.join("cdn")).unwrap();
        std::fs::write(root.join("cdn/x.webm"), "x").unwrap();
        // deck-link's daemon waits for a member (#180): its run directory holds the code, no token
        let run = root.join("run");
        deck_link::pairing::write(&run, &deck_link::Shown::Pending { code: "K7Q42M".into() }, None).unwrap();
        let m = media(&root, Box::new(Linked(deck_link::TokenSource::Link(run.clone()))));
        let r = m.preload("left", &[item("x", Some("mock://x.webm"), None)]);
        assert_eq!(r.why.unwrap()["x"], "HTTP 401 – the deck is not paired – code K7Q-42M");
        // approved: the token from the run directory
        deck_link::pairing::write(&run, &deck_link::Shown::Ready { name: "X1".into() }, Some("T")).unwrap();
        let m = media(&root, Box::new(Linked(deck_link::TokenSource::Link(run))));
        assert_eq!(m.preload("left", &[item("x", Some("mock://x.webm"), None)]).missing, Vec::<String>::new());
    }

    #[test]
    fn a_set_with_a_url_falls_back_to_it_and_names_are_plain() {
        let root = temp_dir("fallback");
        std::fs::create_dir_all(root.join("cdn/sets")).unwrap();
        std::fs::write(root.join("cdn/sets/cathedral-choir.webm"), "v").unwrap();
        let m = media(&root, Box::new(Fixed("T".into())));
        let i = item("choir", Some("mock://sets/cathedral-choir.webm"), Some(("cathedral", "choir", SpotMode::Video)));
        assert_eq!(m.source(&i).unwrap(), Source::Url("mock://sets/cathedral-choir.webm".into()));
        assert!(m.resolve.spot("../etc", None, SpotMode::Still).unwrap_err().contains("plain name"));
        assert_eq!(m.preload("o", &[i]).ready, ["choir"]);
    }
}
