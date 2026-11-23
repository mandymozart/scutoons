//! scutoons.com for the stage (CYBERDECKS.md §8): the public library API read-only and the CDN files, mirrored into a
//! disk cache so a venue runs offline – `<cache>/api/toons/<toon>/chapters/<n>.json` (refreshed when online, the cached
//! copy when not) and `<cache>/cdn/<toon>/<path>` (CDN paths are immutable – a changed file gets a new name – so a
//! cached file is never fetched again; a module's slot asset is the exception: the library's Replace keeps its path, so
//! `load_fresh` asks the CDN once per load whether the cached copy is still current – a conditional GET on the ETag /
//! Last-Modified kept with it under `<cache>/validators/` – and takes the new bytes when it was replaced; offline the
//! cached copy stays). Which rendition loads – and, offline, the best one the cache holds – is the
//! general-purpose loader's (deck-media `asset::load` over deck-catalog's pick rule); decoding: deck-media.
use crate::audio::{MusicFeed, Track};
use anyhow::{bail, Context, Result};
use deck_catalog::rendition::{Media, Quality};
use deck_media::asset::{self, Files, Got};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub use crate::cache::{API, CDN};

#[derive(Clone)]
pub struct Library {
    pub api: String,
    pub cdn: String,
    pub cache: PathBuf,
}

pub use deck_space::chapter::{Audio, Chapter, Group, Music, MusicTrack, Slice};

impl Library {
    fn agent() -> ureq::Agent {
        ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(5)).timeout(Duration::from_secs(60)).build()
    }

    pub fn chapter_cache_path(&self, toon: &str, n: u32) -> Result<PathBuf> {
        Ok(self.cache.join("api").join("toons").join(safe_segment(toon)?).join("chapters").join(format!("{n}.json")))
    }

    pub fn file_cache_path(&self, toon: &str, path: &str) -> Result<PathBuf> {
        let mut p = self.cache.join("cdn").join(safe_segment(toon)?);
        for seg in path.split('/') {
            p.push(safe_segment(seg)?);
        }
        Ok(p)
    }

    /// the chapter from the API (written to the cache), or the cached copy when the API cannot be reached
    pub fn chapter(&self, toon: &str, n: u32) -> Result<(Chapter, &'static str)> {
        let cached = self.chapter_cache_path(toon, n)?;
        let url = format!("{}/library/toons/{toon}/chapters/{n}", self.api.trim_end_matches('/'));
        let mut req = Self::agent().get(&url);
        // the deck token once a member approved the deck (#180, deck-link's run directory) – public chapters load without
        if let Some(t) = deck_link::TokenSource::from_env().get() {
            req = req.set("Authorization", &format!("Bearer {t}"));
        }
        match req.call() {
            Ok(r) => {
                let text = r.into_string()?;
                let ch: Chapter = serde_json::from_str(&text).with_context(|| format!("{url}: not a chapter"))?;
                write_atomic(&cached, text.as_bytes())?;
                Ok((ch, "online"))
            }
            Err(e) => {
                let text = std::fs::read_to_string(&cached)
                    .with_context(|| format!("{url}: {e} – and no cached copy at {}", cached.display()))?;
                tracing::warn!("library: {url}: {e} – using the cached chapter");
                Ok((serde_json::from_str(&text)?, "cached"))
            }
        }
    }

    /// where a cached CDN file's validators (its ETag, its Last-Modified) are kept
    fn validators_path(&self, toon: &str, path: &str) -> Result<PathBuf> {
        let mut p = self.cache.join("validators").join(safe_segment(toon)?);
        for seg in path.split('/') {
            p.push(safe_segment(seg)?);
        }
        Ok(p)
    }

    /// a CDN file of the toon's folder, downloaded once into the cache (its validators beside it, for `revalidate`)
    pub fn file(&self, toon: &str, path: &str) -> Result<PathBuf> {
        let local = self.file_cache_path(toon, path)?;
        if local.is_file() {
            return Ok(local);
        }
        let url = format!("{}/{toon}/{path}", self.cdn.trim_end_matches('/'));
        let r = Self::agent().get(&url).call().with_context(|| format!("downloading {url}"))?;
        self.keep(toon, path, r)?;
        Ok(local)
    }

    /// a fetched file's bytes into the cache, its validators beside them
    fn keep(&self, toon: &str, path: &str, r: ureq::Response) -> Result<Vec<u8>> {
        let validators = Validators { etag: r.header("ETag").map(str::to_string), modified: r.header("Last-Modified").map(str::to_string) };
        let mut bytes = Vec::new();
        r.into_reader().take(512 << 20).read_to_end(&mut bytes)?;
        write_atomic(&self.file_cache_path(toon, path)?, &bytes)?;
        write_atomic(&self.validators_path(toon, path)?, validators.text().as_bytes())?;
        Ok(bytes)
    }

    /// Whether the cached copy of a CDN file is still the CDN's: a conditional GET on its validators (none kept: a plain
    /// GET, once); a replaced file's new bytes go into the cache. Offline (or any error) the cached copy stays.
    pub fn revalidate(&self, toon: &str, path: &str) -> Fresh {
        let Ok(local) = self.file_cache_path(toon, path) else { return Fresh::Missing };
        if !local.is_file() {
            return Fresh::Missing;
        }
        let kept = self.validators_path(toon, path).ok().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| Validators::parse(&t)).unwrap_or_default();
        let url = format!("{}/{toon}/{path}", self.cdn.trim_end_matches('/'));
        let mut req = Self::agent().get(&url);
        if let Some(e) = &kept.etag {
            req = req.set("If-None-Match", e);
        }
        if let Some(m) = &kept.modified {
            req = req.set("If-Modified-Since", m);
        }
        match req.call() {
            Ok(r) if r.status() == 304 => Fresh::Unchanged,
            Ok(r) => {
                let before = std::fs::read(&local).ok();
                match self.keep(toon, path, r) {
                    Ok(bytes) if before.as_deref() == Some(bytes.as_slice()) => Fresh::Unchanged,
                    Ok(_) => Fresh::Replaced,
                    Err(e) => Fresh::Offline(format!("{e:#}")),
                }
            }
            Err(e) => Fresh::Offline(e.to_string()),
        }
    }

    /// `media`'s file for `q` through the general-purpose loader: paths under `folder` (a toon), or CDN-root paths
    /// `<folder>/<rest>` when `folder` is None
    pub fn load(&self, folder: Option<&str>, media: &Media, q: Quality) -> Result<Got> {
        asset::load(media, q, &Cdn { lib: self, folder, fresh: false }).map_err(|e| anyhow::anyhow!("{}", e.0))
    }

    /// `load`, each cached file it takes first checked against the CDN (`revalidate`): a module's slot assets, which the
    /// library's Replace changes under the same path – once per load (a scene's or rig's build), never per frame
    pub fn load_fresh(&self, folder: Option<&str>, media: &Media, q: Quality) -> Result<Got> {
        asset::load(media, q, &Cdn { lib: self, folder, fresh: true }).map_err(|e| anyhow::anyhow!("{}", e.0))
    }
}

/// What `revalidate` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fresh {
    /// not cached: nothing to check
    Missing,
    /// the cached copy is the CDN's
    Unchanged,
    /// the CDN has other bytes now: they are in the cache
    Replaced,
    /// the CDN could not be asked: the cached copy stays
    Offline(String),
}

/// a cached file's HTTP validators, one per line (`etag: …`, `modified: …`)
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Validators {
    etag: Option<String>,
    modified: Option<String>,
}

impl Validators {
    fn text(&self) -> String {
        let line = |k: &str, v: &Option<String>| v.as_ref().map(|v| format!("{k}: {v}\n")).unwrap_or_default();
        line("etag", &self.etag) + &line("modified", &self.modified)
    }

    fn parse(text: &str) -> Validators {
        let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(k)).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        Validators { etag: get("etag:"), modified: get("modified:") }
    }
}

/// The library's cache and CDN as deck-media's `Files`.
struct Cdn<'a> {
    lib: &'a Library,
    folder: Option<&'a str>,
    /// check a cached file against the CDN first (`Library::revalidate`)
    fresh: bool,
}

impl Cdn<'_> {
    fn split<'p>(&'p self, path: &'p str) -> Result<(&'p str, &'p str), String> {
        match self.folder {
            Some(f) => Ok((f, path)),
            None => path.split_once('/').ok_or_else(|| format!("'{path}' is no CDN path")),
        }
    }
}

impl Files for Cdn<'_> {
    fn cached(&self, path: &str) -> Option<Vec<u8>> {
        let (folder, rest) = self.split(path).ok()?;
        if self.fresh {
            if let Fresh::Offline(why) = self.lib.revalidate(folder, rest) {
                tracing::info!("library: {path} could not be checked ({why}) – the cached copy");
            }
        }
        std::fs::read(self.lib.file_cache_path(folder, rest).ok()?).ok()
    }

    fn fetch(&self, path: &str) -> Result<Vec<u8>, String> {
        let (folder, rest) = self.split(path)?;
        let file = self.lib.file(folder, rest).map_err(|e| format!("{e:#}"))?;
        std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))
    }
}

/// one path segment from the API / a scene: no separators, no `..`, nothing empty
fn safe_segment(s: &str) -> Result<&str> {
    if s.is_empty() || s == "." || s == ".." || s.contains(['/', '\\', ':']) {
        bail!("'{s}' is not a safe path segment");
    }
    Ok(s)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

static QUALITY: std::sync::Mutex<Quality> = std::sync::Mutex::new(Quality::Medium);

/// the deck's media quality (its render ceiling, `--quality` then the governor's; medium until it is known): the level
/// a library file loads at unless a node names its own
pub fn quality() -> Quality {
    *QUALITY.lock().unwrap_or_else(|e| e.into_inner())
}

/// the quality the next loads take
pub fn set_quality(q: Quality) {
    *QUALITY.lock().unwrap_or_else(|e| e.into_inner()) = q;
}

/// decode a whole file (deck-media's decode) into a stereo track at its own rate; mono is doubled, more channels keep L + R
pub fn decode(path: &Path) -> Result<Track> {
    let bytes = std::fs::read(path).with_context(|| format!("opening {}", path.display()))?;
    decode_bytes(bytes, &path.display().to_string())
}

/// … from the file's bytes (`name` for the error)
pub fn decode_bytes(bytes: Vec<u8>, name: &str) -> Result<Track> {
    let p = deck_media::asset::planar(bytes).map_err(|e| anyhow::anyhow!("{name}: {}", e.0))?;
    let l = p.channels[0].clone();
    let r = p.channels.get(1).cloned().unwrap_or_else(|| l.clone());
    Ok(Track { rate: p.rate, l, r })
}

/// A chapter_music node's loader (a plain thread, off the frame and audio threads): chapter → the track's file for the
/// quality (the node's, else the deck's) through the general-purpose loader (offline: the best rendition the cache
/// holds) → decoded track → the music block. The HUD reads `status`.
pub fn spawn_music_loader(lib: Library, node: &crate::graph::Node, feed: MusicFeed) {
    let toon = node.str("toon").unwrap_or("catpope").to_string();
    let n = node.f32("chapter", 1.0) as u32;
    let track = node.str("track").map(str::to_string);
    let quality = node.str("quality").and_then(Quality::parse).unwrap_or_else(quality);
    let MusicFeed { mut tracks, status, .. } = feed;
    let label = format!("{toon} {n}");
    let set = move |s: String| {
        tracing::info!("music {label}: {s}");
        *status.lock().unwrap() = s;
    };
    std::thread::spawn(move || {
        set("loading the chapter".into());
        let t0 = std::time::Instant::now();
        let r = (|| -> Result<String> {
            let (ch, source) = lib.chapter(&toon, n)?;
            let Some(t) = ch.music(track.as_deref()) else { bail!("the chapter has no music") };
            let id = t.id.clone();
            set(format!("fetching '{id}'"));
            let got = lib.load(Some(&toon), &t.media(), quality)?;
            if got.fallback {
                tracing::warn!("music {toon} {n}: '{id}' at {} could not be had – {} plays", quality.name(), got.path);
            }
            set(format!("decoding '{id}'"));
            let t = decode_bytes(got.bytes, &got.path)?;
            let secs = t.seconds();
            if tracks.push(Arc::new(t)).is_err() {
                bail!("the music block is not taking tracks");
            }
            Ok(format!("'{id}' {secs:.0} s ({source}, {:.1} s to load) – {}", t0.elapsed().as_secs_f32(), ch.title))
        })();
        set(match r {
            Ok(s) => s,
            Err(e) => format!("failed: {e:#}"),
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHAPTER: &str = r#"{"title":"Leugnen","audio":{"music":{"defaultTrack":"calmness","tracks":[
        {"id":"tension","path":"s1/c2/t.128.mp3"},
        {"id":"calmness","path":"s1/c1/c.128.mp3","variants":{"low":"s1/c1/c.96.mp3","high":"s1/c1/c.320.mp3"}}]}},
        "groups":[{"slices":[{"image":"s1/c1/a.webp","imageVariants":{"medium":"s1/c1/a.2x.webp"},
          "dialog":[{"rect":{"x":1,"y":2,"width":50},"text":"Leugnen","font":"lettering"}]},{"width":3}]}]}"#;

    #[test]
    fn the_music_path_follows_track_default_and_quality() {
        let ch: Chapter = serde_json::from_str(CHAPTER).unwrap();
        let at = |id: Option<&str>, q: &str| ch.music_path(id, q).map(|(i, p)| (i.to_string(), p));
        assert_eq!(at(None, "high"), Some(("calmness".into(), "s1/c1/c.320.mp3".into())));
        assert_eq!(at(None, "medium"), Some(("calmness".into(), "s1/c1/c.96.mp3".into())), "no medium variant: the nearest lower");
        assert_eq!(at(Some("tension"), "low"), Some(("tension".into(), "s1/c2/t.128.mp3".into())), "no variants: the path");
        let slices: Vec<&Slice> = ch.slices().collect();
        assert_eq!(slices.len(), 1, "a slice without an image is skipped");
        let image = |q: &str| slices[0].image_path(q).unwrap();
        assert_eq!((image("low"), image("medium"), image("high")), ("s1/c1/a.webp".into(), "s1/c1/a.2x.webp".into(), "s1/c1/a.2x.webp".into()), "high missing: the nearest lower");
        assert_eq!(slices[0].dialog[0].text(), "Leugnen");
        let none: Chapter = serde_json::from_str(r#"{"title":"x"}"#).unwrap();
        assert_eq!(none.music_path(None, "low"), None);
    }

    #[test]
    fn cache_paths_mirror_the_cdn_and_refuse_escapes() {
        let lib = Library { api: API.into(), cdn: CDN.into(), cache: PathBuf::from("c") };
        assert_eq!(lib.file_cache_path("catpope", "s1/c1/a.mp3").unwrap(), Path::new("c/cdn/catpope/s1/c1/a.mp3"));
        assert!(lib.file_cache_path("catpope", "s1/../../etc/passwd").is_err());
        assert!(lib.chapter_cache_path("../x", 1).is_err());
    }

    #[test]
    fn the_cached_chapter_is_used_offline() {
        let dir = std::env::temp_dir().join(format!("deck-native-test-{}", std::process::id()));
        let lib = Library { api: "http://127.0.0.1:9".into(), cdn: CDN.into(), cache: dir.clone() };
        write_atomic(&lib.chapter_cache_path("catpope", 1).unwrap(), CHAPTER.as_bytes()).unwrap();
        let (ch, source) = lib.chapter("catpope", 1).unwrap();
        assert_eq!((ch.title.as_str(), source), ("Leugnen", "cached"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn offline_a_track_loads_the_best_rendition_the_cache_holds() {
        let dir = std::env::temp_dir().join(format!("w782-music-offline-{}", std::process::id()));
        let lib = Library { api: "http://127.0.0.1:9".into(), cdn: "http://127.0.0.1:9".into(), cache: dir.clone() };
        write_atomic(&lib.file_cache_path("catpope", "s1/c1/c.128.mp3").unwrap(), b"128").unwrap();
        write_atomic(&lib.file_cache_path("catpope", "s1/c1/c.96.mp3").unwrap(), b"96").unwrap();
        let ch: Chapter = serde_json::from_str(CHAPTER).unwrap();
        let m = ch.music(None).unwrap().media();
        let at = |q| lib.load(Some("catpope"), &m, q).map(|g| (g.path, g.fallback));
        assert_eq!(at(Quality::Low).unwrap(), ("s1/c1/c.96.mp3".into(), false));
        assert_eq!(at(Quality::High).unwrap(), ("s1/c1/c.96.mp3".into(), true), "320 neither cached nor reachable: the rule's next held file");
        std::fs::remove_file(lib.file_cache_path("catpope", "s1/c1/c.96.mp3").unwrap()).unwrap();
        assert_eq!(at(Quality::Low).unwrap(), ("s1/c1/c.128.mp3".into(), true), "a bigger file only when no lower is held");
        let root = lib.load(None, &Media::new("catpope/s1/c1/c.128.mp3"), Quality::High).unwrap();
        assert_eq!(root.bytes, b"128", "CDN-root paths name their folder");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// a CDN's answer: status, headers, body
    type Answer = (u16, Vec<(&'static str, &'static str)>, &'static [u8]);

    /// a CDN on 127.0.0.1 answering `answers` in turn (status, headers, body), one connection each; what it was asked
    fn cdn(answers: Vec<Answer>) -> (String, Arc<std::sync::Mutex<Vec<String>>>) {
        use std::io::Write;
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", server.local_addr().unwrap());
        let asked = Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = asked.clone();
        std::thread::spawn(move || {
            for (status, headers, body) in answers {
                let Ok((mut s, _)) = server.accept() else { return };
                let mut req = Vec::new();
                let mut b = [0u8; 1];
                while !req.ends_with(b"\r\n\r\n") && s.read(&mut b).unwrap_or(0) == 1 {
                    req.push(b[0]);
                }
                log.lock().unwrap().push(String::from_utf8_lossy(&req).to_lowercase());
                let mut head = format!("HTTP/1.1 {status} X\r\nConnection: close\r\nContent-Length: {}\r\n", if status == 304 { 0 } else { body.len() });
                for (k, v) in headers {
                    head += &format!("{k}: {v}\r\n");
                }
                let _ = s.write_all(head.as_bytes());
                let _ = s.write_all(b"\r\n");
                if status != 304 {
                    let _ = s.write_all(body);
                }
            }
        });
        (base, asked)
    }

    fn fresh_lib(name: &str, cdn: &str) -> Library {
        let dir = std::env::temp_dir().join(format!("w805-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Library { api: "http://127.0.0.1:9".into(), cdn: cdn.into(), cache: dir }
    }

    const PUFF: &str = "scutoons/3d/cloud-puff.glb";

    /// a slot asset kept: the next load asks with its ETag and Last-Modified, the CDN's 304 sends no bytes
    #[test]
    fn an_unchanged_slot_asset_is_not_downloaded_again() {
        let (base, asked) = cdn(vec![(200, vec![("ETag", "\"v1\""), ("Last-Modified", "Mon, 23 Nov 2026 10:00:00 GMT")], b"bubble"), (304, vec![], b"")]);
        let lib = fresh_lib("unchanged", &base);
        assert_eq!(lib.load_fresh(None, &Media::new(PUFF), Quality::Medium).unwrap().bytes, b"bubble", "the first load downloads");
        assert_eq!(lib.load_fresh(None, &Media::new(PUFF), Quality::Medium).unwrap().bytes, b"bubble");
        let asked = asked.lock().unwrap();
        assert_eq!(asked.len(), 2, "one check per load");
        assert!(asked[1].contains("if-none-match: \"v1\"") && asked[1].contains("if-modified-since: mon, 23 nov 2026 10:00:00 gmt"), "{}", asked[1]);
        let _ = std::fs::remove_dir_all(&lib.cache);
    }

    /// replaced in the library (same path, new bytes, a new ETag): the next load takes the new bytes and keeps the new ETag
    #[test]
    fn a_replaced_slot_asset_arrives_on_the_next_load() {
        let (base, asked) = cdn(vec![(200, vec![("ETag", "\"v1\"")], b"bubble"), (200, vec![("ETag", "\"v2\"")], b"owner's cloud"), (304, vec![], b"")]);
        let lib = fresh_lib("replaced", &base);
        let load = || lib.load_fresh(None, &Media::new(PUFF), Quality::Medium).unwrap().bytes;
        assert_eq!(load(), b"bubble");
        assert_eq!(lib.revalidate("scutoons", "3d/cloud-puff.glb"), Fresh::Replaced);
        assert_eq!(std::fs::read(lib.file_cache_path("scutoons", "3d/cloud-puff.glb").unwrap()).unwrap(), b"owner's cloud");
        assert_eq!(load(), b"owner's cloud", "the cache holds the new file");
        assert!(asked.lock().unwrap()[2].contains("if-none-match: \"v2\""), "the new ETag kept");
        let _ = std::fs::remove_dir_all(&lib.cache);
    }

    /// offline the cached copy plays; a plain `load` (immutable CDN paths) never asks
    #[test]
    fn offline_the_cached_slot_asset_stays() {
        let lib = fresh_lib("offline", "http://127.0.0.1:9");
        write_atomic(&lib.file_cache_path("scutoons", "3d/cloud-puff.glb").unwrap(), b"bubble").unwrap();
        write_atomic(&lib.validators_path("scutoons", "3d/cloud-puff.glb").unwrap(), b"etag: \"v1\"\n").unwrap();
        assert!(matches!(lib.revalidate("scutoons", "3d/cloud-puff.glb"), Fresh::Offline(_)));
        let got = lib.load_fresh(None, &Media::new(PUFF), Quality::Medium).unwrap();
        assert_eq!((got.bytes.as_slice(), got.fallback), (&b"bubble"[..], false));
        assert_eq!(lib.revalidate("scutoons", "3d/none.glb"), Fresh::Missing);
        assert_eq!(Validators::parse("etag: \"v1\"\n"), Validators { etag: Some("\"v1\"".into()), modified: None });
        let _ = std::fs::remove_dir_all(&lib.cache);
    }

    #[test]
    fn a_wav_decodes_to_stereo() {
        let dir = std::env::temp_dir().join(format!("deck-native-wav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.wav");
        std::fs::write(&path, wav_mono_16(22_050, &[0, 16_384, -16_384, 0])).unwrap();
        let t = decode(&path).unwrap();
        assert_eq!(t.rate, 22_050.0);
        assert_eq!(t.l.len(), 4);
        assert_eq!(t.l, t.r);
        assert!((t.l[1] - 0.5).abs() < 1e-3);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn wav_mono_16(rate: u32, samples: &[i16]) -> Vec<u8> {
        let data = samples.len() as u32 * 2;
        let mut v = Vec::new();
        v.extend(b"RIFF");
        v.extend((36 + data).to_le_bytes());
        v.extend(b"WAVEfmt ");
        v.extend(16u32.to_le_bytes());
        v.extend(1u16.to_le_bytes()); // PCM
        v.extend(1u16.to_le_bytes()); // mono
        v.extend(rate.to_le_bytes());
        v.extend((rate * 2).to_le_bytes());
        v.extend(2u16.to_le_bytes());
        v.extend(16u16.to_le_bytes());
        v.extend(b"data");
        v.extend(data.to_le_bytes());
        for s in samples {
            v.extend(s.to_le_bytes());
        }
        v
    }
}
