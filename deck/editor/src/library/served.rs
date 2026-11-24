//! The Library the server answers (`GET /api/library`, deck-catalog `library::served`), read with the editor's token on a
//! thread and kept on disk by its query: asked again with the version held (`If-None-Match` → 304, nothing sent), and
//! offline or refused the last answer stays with a word that it is the last one. A thing stepped into is read the same way.
use deck_catalog::library::served::{self, Inside, Place};
use deck_space::library::served as answer;
use deck_catalog::library::Entry;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// a held answer is asked again this often (seconds): a 304 costs the server nothing
const AGAIN: f64 = 60.0;
/// a list read at most this many pages (a cursor that never ends stops here)
const MAX_PAGES: usize = 20;

/// what a read came to: the pages (or the step) as text, and the word when it is the last answer kept, not a new one
#[derive(Debug, Clone, PartialEq)]
pub struct Read {
    pub pages: Vec<String>,
    pub stale: Option<String>,
}

/// a list or a step asked: its query, when, its answer on the way
struct Asking {
    query: String,
    at: f64,
    rx: Option<Receiver<Result<Read, String>>>,
}

/// the server's Library as the editor holds it
pub struct Served {
    dir: PathBuf,
    pub place: Option<Place>,
    list: Option<Asking>,
    /// the entries of the place shown and the word when they are the last ones kept
    pub entries: Option<(Vec<Entry>, Option<String>)>,
    step: Option<Asking>,
    stepped: Option<String>,
    /// the thing stepped into, once read
    pub inside: Option<(Inside, Option<String>)>,
    /// why the last read failed with nothing kept (said once)
    pub failed: Option<String>,
}

impl Served {
    pub fn new(dir: PathBuf) -> Served {
        Served { dir, place: None, list: None, entries: None, step: None, stepped: None, inside: None, failed: None }
    }

    fn file(&self, query: &str) -> PathBuf {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        query.hash(&mut h);
        self.dir.join(format!("{:016x}.json", h.finish()))
    }

    /// The place's list for `toon`: asked when the query changed or a minute passed (`now` in seconds), from the cache
    /// at once when there is none yet. Signed out, only the cache.
    pub fn ask(&mut self, base: &str, token: Option<String>, toon: Option<&str>, open: Option<&str>, now: f64) {
        let place = self.place.clone().filter(|p| Place::offered(open, toon).contains(p)).unwrap_or_else(|| Place::start(open, toon));
        self.place = Some(place.clone());
        let query = place.query(toon, crate::LOCALE);
        let same = self.list.as_ref().is_some_and(|a| a.query == query);
        if same && self.list.as_ref().is_some_and(|a| a.rx.is_some() || now - a.at < AGAIN) {
            return;
        }
        if !same {
            self.entries = kept(&self.file(&query)).map(|r| (list_entries(&r.pages), Some("the Library as last read".to_string())));
        }
        self.list = Some(Asking { rx: token.map(|t| read(base, &t, &query, &self.file(&query), true)), query, at: now });
    }

    /// step into `address` (none: out again)
    pub fn step_into(&mut self, base: &str, token: Option<String>, address: Option<&str>, now: f64) {
        self.stepped = address.map(str::to_string);
        let Some(address) = address else {
            (self.step, self.inside) = (None, None);
            return;
        };
        let query = served::step_query(address, crate::LOCALE);
        self.inside = kept(&self.file(&query)).and_then(|r| step_of(&r.pages, Some("the Library as last read".into())));
        self.step = Some(Asking { rx: token.map(|t| read(base, &t, &query, &self.file(&query), false)), query, at: now });
    }

    /// the address stepped into (asked or read)
    pub fn at(&self) -> Option<&str> {
        self.stepped.as_deref()
    }

    /// what the threads brought: true when the list or the step changed
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Some(r) = take(&mut self.list) {
            match r {
                Ok(r) => {
                    let fresh = (list_entries(&r.pages), r.stale);
                    changed |= self.entries.as_ref() != Some(&fresh);
                    self.entries = Some(fresh);
                }
                Err(why) => self.failed = Some(why),
            }
        }
        if let Some(r) = take(&mut self.step) {
            match r.map(|r| step_of(&r.pages, r.stale)) {
                Ok(Some(i)) => {
                    changed |= self.inside.as_ref() != Some(&i);
                    self.inside = Some(i);
                }
                Ok(None) => {}
                Err(why) => self.failed = Some(why),
            }
        }
        changed
    }
}

/// the answer of an asking that came back (it is asked again later)
fn take(a: &mut Option<Asking>) -> Option<Result<Read, String>> {
    let asking = a.as_mut()?;
    let r = asking.rx.as_ref()?.try_recv().ok()?;
    asking.rx = None;
    Some(r)
}

fn list_entries(pages: &[String]) -> Vec<Entry> {
    let lists: Vec<_> = pages.iter().filter_map(|p| answer::list(p).map_err(|e| tracing::warn!("{e}")).ok()).collect();
    answer::entries(&lists)
}

fn step_of(pages: &[String], stale: Option<String>) -> Option<(Inside, Option<String>)> {
    let i = answer::inside(pages.first()?).map_err(|e| tracing::warn!("{e}")).ok()?;
    Some((i, stale))
}

/// the answer kept on disk for a query: its version and its pages
fn kept(file: &Path) -> Option<Read> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
    let pages = v["pages"].as_array()?.iter().filter_map(|p| p.as_str().map(str::to_string)).collect();
    Some(Read { pages, stale: None })
}

fn keep(file: &Path, etag: &str, pages: &[String]) {
    let text = serde_json::json!({ "etag": etag, "pages": pages }).to_string();
    if let Err(e) = file.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(file, text)) {
        tracing::warn!("the Library's cache {}: {e}", file.display());
    }
}

fn etag_of(file: &Path) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
    v["etag"].as_str().map(str::to_string)
}

/// One read on a thread: the first page with the version held; 304 → the pages kept; else every page (a list follows
/// its cursor) and kept. Offline or refused: the pages kept with the word why, else the error.
fn read(base: &str, token: &str, query: &str, file: &Path, paged: bool) -> Receiver<Result<Read, String>> {
    let (tx, rx) = channel();
    let (base, token, query, file) = (base.trim_end_matches('/').to_string(), token.to_string(), query.to_string(), file.to_path_buf());
    std::thread::spawn(move || {
        let _ = tx.send(fetch(&base, &token, &query, &file, paged));
    });
    rx
}

fn fetch(base: &str, token: &str, query: &str, file: &Path, paged: bool) -> Result<Read, String> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build();
    let get = |q: &str| agent.get(&format!("{base}/library?{q}")).set("Authorization", &format!("Bearer {token}"));
    let held = etag_of(file).filter(|_| kept(file).is_some());
    let first = match held.as_deref() {
        Some(etag) => get(query).set("If-None-Match", etag).call(),
        None => get(query).call(),
    };
    let fallback = |why: String| kept(file).map(|r| Read { stale: Some(format!("{why} – the Library as last read")), ..r }).ok_or(why);
    let first = match first {
        Ok(r) if r.status() == 304 => return kept(file).ok_or_else(|| "the Library's cache is gone".to_string()),
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => return fallback(crate::show::api::refused_words("the Library", code, &r.into_json().unwrap_or_default())),
        Err(e) => {
            tracing::info!("the Library: {e}");
            return fallback("offline".into());
        }
    };
    let etag = first.header("ETag").unwrap_or_default().to_string();
    let mut pages = vec![first.into_string().map_err(|e| format!("the Library: {e}"))?];
    while paged && pages.len() < MAX_PAGES {
        let list = answer::list(pages.last().expect("a page"))?;
        let Some(next) = served::next_query(query, list.next.as_deref()) else { break };
        let page = get(&next).call().map_err(|e| format!("the Library's next page: {e}"))?;
        pages.push(page.into_string().map_err(|e| format!("the Library: {e}"))?);
    }
    keep(file, &etag, &pages);
    Ok(Read { pages, stale: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(version: &str, next: Option<&str>, address: &str) -> String {
        serde_json::json!({ "library": 1, "version": version, "toon": "catpope", "place": "toon", "locale": "en", "total": 2, "next": next, "counts": {},
            "entries": [{ "address": address, "type": "character", "name": address, "picture": null, "toons": [], "usedIn": 0, "updated": null }] })
        .to_string()
    }

    /// a server on a thread: answers each request with the next of `answers` (status, ETag, body) and hands back what it was asked
    fn server(answers: Vec<(u16, &'static str, String)>) -> (String, Receiver<String>) {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/api", listener.local_addr().unwrap());
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            for (status, etag, body) in answers {
                let Ok((mut s, _)) = listener.accept() else { return };
                let mut asked = String::new();
                let mut reader = BufReader::new(s.try_clone().unwrap());
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    asked.push_str(&line);
                }
                let _ = tx.send(asked);
                let _ = write!(s, "HTTP/1.1 {status} X\r\nETag: {etag}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        (base, rx)
    }

    #[test]
    fn a_list_is_read_page_by_page_kept_and_asked_again_with_its_version() {
        let dir = std::env::temp_dir().join(format!("deck-editor-served-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let (base, asked) = server(vec![(200, "\"v1\"", page("v1", Some("c1"), "character:a")), (200, "\"v1\"", page("v1", None, "character:b")), (304, "\"v1\"", String::new())]);
        let file = dir.join("x.json");
        let r = fetch(&base, "scd_t", "toon=catpope&place=toon", &file, true).unwrap();
        assert_eq!((r.pages.len(), r.stale), (2, None), "the cursor followed");
        assert!(asked.recv().unwrap().contains("Bearer scd_t"));
        assert!(asked.recv().unwrap().contains("cursor=c1"));
        let again = fetch(&base, "scd_t", "toon=catpope&place=toon", &file, true).unwrap();
        assert!(asked.recv().unwrap().to_lowercase().contains("if-none-match: \"v1\""), "the version held");
        assert_eq!(list_entries(&again.pages).iter().map(|e| e.address.as_str()).collect::<Vec<_>>(), ["character:a", "character:b"], "304: the pages kept");
        let offline = fetch("http://127.0.0.1:9/api", "scd_t", "toon=catpope&place=toon", &file, true).unwrap();
        assert!(offline.stale.is_some_and(|w| w.starts_with("offline") && w.ends_with("the Library as last read")));
        assert!(fetch("http://127.0.0.1:9/api", "scd_t", "place=all", &dir.join("none.json"), true).is_err(), "offline and nothing kept: why");
        std::fs::remove_dir_all(&dir).ok();
    }
}
