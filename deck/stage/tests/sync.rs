//! The deck's sync (#195) against a stub of scutoons.com in-process (127.0.0.1, 7000–7099): one server answers
//! `/api/library/scene-content` and serves the CDN files, counting every request. A scene's needs → one API request →
//! the set's 3D and the portraits in the cache; a second run skips what is cached; a changed path is fetched again; a
//! broken file is refused (the old one stays); offline plays from the cache; a character wanted later is asked for then.
#![cfg(feature = "sync")]
use deck_stage::sync::{Config, DeckSync, Needs, Want};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// a free port in the agreed slot (7000–7099), held by the listener
fn listen() -> TcpListener {
    let start = 7000 + (std::process::id() % 50) as u16 + 50;
    (start..7100).chain(7000..start).find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok()).expect("a free port in 7000–7099")
}

/// The stub: `/api/library/scene-content?…` answers `api` (the JSON the test sets), `/cdn/<path>` answers `files`.
struct Stub {
    port: u16,
    stop: Arc<AtomicBool>,
    hits: Arc<Mutex<Vec<String>>>,
    api: Arc<Mutex<String>>,
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Stub {
    fn start() -> Stub {
        let l = listen();
        let port = l.local_addr().unwrap().port();
        l.set_nonblocking(true).unwrap();
        let stub = Stub { port, stop: Default::default(), hits: Default::default(), api: Default::default(), files: Default::default() };
        let (stop, hits, api, files) = (stub.stop.clone(), stub.hits.clone(), stub.api.clone(), stub.files.clone());
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                match l.accept() {
                    Ok((s, _)) => serve(s, &hits, &api, &files),
                    Err(_) => std::thread::sleep(Duration::from_millis(5)),
                }
            }
        });
        stub
    }

    fn config(&self, cache: &Path) -> Config {
        Config {
            api: format!("http://127.0.0.1:{}/api", self.port),
            cdn: format!("http://127.0.0.1:{}/cdn", self.port),
            cache: cache.into(),
            sets: Some(cache.join("sets")),
            portraits: Some(cache.join("portraits")),
            token: Some("T".into()),
        }
    }

    fn hits(&self, prefix: &str) -> Vec<String> {
        self.hits.lock().unwrap().iter().filter(|h| h.starts_with(prefix)).cloned().collect()
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn serve(mut s: TcpStream, hits: &Mutex<Vec<String>>, api: &Mutex<String>, files: &Mutex<HashMap<String, Vec<u8>>>) {
    s.set_nonblocking(false).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut head = Vec::new();
    let mut b = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        let Ok(n) = s.read(&mut b) else { return };
        if n == 0 {
            return;
        }
        head.extend_from_slice(&b[..n]);
    }
    let head = String::from_utf8_lossy(&head).to_string();
    let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
    let bearer = head.lines().any(|l| l.eq_ignore_ascii_case("authorization: bearer t"));
    hits.lock().unwrap().push(format!("{path}{}", if bearer { " +token" } else { "" }));
    let (status, body) = if path.starts_with("/api/library/scene-content") {
        ("200 OK", api.lock().unwrap().clone().into_bytes())
    } else if let Some(file) = path.strip_prefix("/cdn/").and_then(|f| files.lock().unwrap().get(f).cloned()) {
        ("200 OK", file)
    } else {
        ("404 Not Found", b"{}".to_vec())
    };
    let _ = write!(s, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
    let _ = s.write_all(&body);
}

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("deck-sync-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn glb(tag: &str) -> Vec<u8> {
    format!("glTF\x02\0\0\0{tag}").into_bytes()
}

fn png() -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(4, 4, image::Rgba([9, 8, 7, 255])).write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

/// the answer for the cathedral set (its low 3D: `low`) + catpope (neutral, happy), each with its file's real size
/// unless `said` names a path and the size the API says instead
fn answer(files: &HashMap<String, Vec<u8>>, low: &str, neutral: &str, happy: &str, said: Option<(&str, usize)>) -> String {
    let f = |p: &str| format!(r#"{{"path":"{p}","size":{}}}"#, said.filter(|(q, _)| *q == p).map_or_else(|| files[p].len(), |(_, n)| n));
    format!(
        r#"{{"sets":{{"cathedral":{{"name":{{"en":"Cathedral"}},"model":{{"low":{},"medium":{},"high":{}}}}}}},
            "characters":{{"catpope":{{"character":"catpope","persona":"regular","look":"default","portraits":{{"neutral":{},"happy":{}}}}}}},
            "missing":{{"sets":[],"characters":[]}}}}"#,
        f(low),
        f("catpope/map/sets/cathedral.medium.v1.glb"),
        f("catpope/map/sets/cathedral.medium.v1.glb"),
        f(neutral),
        f(happy),
    )
}

const LOW: &str = "catpope/map/sets/cathedral.low.v1.glb";
const NEUTRAL: &str = "characters/catpope/catpope_portrait-neutral.png";
const HAPPY: &str = "characters/catpope/catpope_portrait-happy.png";

fn scene() -> Needs {
    Needs { sets: vec![("cathedral".into(), "low".into())], characters: vec![("catpope".into(), vec!["neutral".into(), "happy".into(), "dizzy".into()])] }
}

/// wait until the sync has settled every want of `needs`
fn settle(sync: &DeckSync, needs: &Needs) {
    for w in needs.wants() {
        assert!(sync.wait(w.clone(), Duration::from_secs(10)), "{w:?} settled");
    }
    let until = Instant::now() + Duration::from_secs(5);
    while sync.board.lock().unwrap().busy && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn stub_with_files() -> Stub {
    let stub = Stub::start();
    let mut files = HashMap::new();
    files.insert("catpope/map/sets/cathedral.low.v1.glb".to_string(), glb("low"));
    files.insert("catpope/map/sets/cathedral.medium.v1.glb".to_string(), glb("medium"));
    files.insert("characters/catpope/catpope_portrait-neutral.png".to_string(), png());
    files.insert("characters/catpope/catpope_portrait-neutral.v2.png".to_string(), png());
    files.insert("characters/catpope/catpope_portrait-happy.png".to_string(), png());
    *stub.api.lock().unwrap() = answer(&files, LOW, NEUTRAL, HAPPY, None);
    *stub.files.lock().unwrap() = files;
    stub
}

#[test]
fn a_scene_s_needs_land_in_the_cache_once() {
    let stub = stub_with_files();
    let cache = dir("once");
    let sync = DeckSync::start(stub.config(&cache));
    sync.prepare(scene());
    settle(&sync, &scene());
    // one request for the whole scene, with the token; then the CDN files (the mood without a sprite: nothing)
    let api = stub.hits("/api/");
    assert_eq!(api.len(), 1, "{api:?}");
    assert!(api[0].contains("sets=cathedral&characters=catpope") && api[0].ends_with("+token"), "{}", api[0]);
    let mut cdn = stub.hits("/cdn/");
    cdn.sort();
    assert_eq!(
        cdn,
        ["/cdn/catpope/map/sets/cathedral.low.v1.glb", "/cdn/characters/catpope/catpope_portrait-happy.png", "/cdn/characters/catpope/catpope_portrait-neutral.png"],
        "only the wanted quality, no token to the CDN"
    );
    let set = cache.join("sets/cathedral/cathedral.low.glb");
    assert_eq!(std::fs::read(&set).unwrap(), glb("low"));
    assert_eq!(deck_stage::cache::set_file(&cache.join("sets"), "cathedral", "low"), set, "where the space node reads it");
    assert!(cache.join("portraits/catpope/catpope_portrait-neutral.png").is_file());
    assert!(cache.join("sync/index.json").is_file());
    assert_eq!(sync.line(), "SYNC OK 3 NEW 0 CACHED");
    drop(sync);

    // the next start: asked again, nothing fetched
    let sync = DeckSync::start(stub.config(&cache));
    sync.prepare(scene());
    settle(&sync, &scene());
    assert_eq!((stub.hits("/api/").len(), stub.hits("/cdn/").len()), (2, 3), "cached files are skipped");
    assert_eq!(sync.line(), "SYNC OK 0 NEW 3 CACHED");
    drop(sync);

    // a changed file has a new path: only it is fetched again
    let files = stub.files.lock().unwrap().clone();
    *stub.api.lock().unwrap() = answer(&files, LOW, "characters/catpope/catpope_portrait-neutral.v2.png", HAPPY, None);
    let sync = DeckSync::start(stub.config(&cache));
    sync.prepare(scene());
    settle(&sync, &scene());
    assert_eq!(stub.hits("/cdn/").last().unwrap(), "/cdn/characters/catpope/catpope_portrait-neutral.v2.png");
    assert_eq!(stub.hits("/cdn/").len(), 4);
    assert_eq!(sync.line(), "SYNC OK 1 NEW 2 CACHED");
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn a_broken_file_is_refused_and_the_old_one_stays() {
    let stub = stub_with_files();
    let cache = dir("broken");
    let sync = DeckSync::start(stub.config(&cache));
    sync.prepare(scene());
    settle(&sync, &scene());
    drop(sync);
    // v2 of the set arrives truncated (the API's size says more), v2 of a portrait is an error page
    {
        let mut files = stub.files.lock().unwrap();
        files.insert("catpope/map/sets/cathedral.low.v2.glb".into(), glb("lo"));
        files.insert("characters/catpope/catpope_portrait-happy.v2.png".into(), b"<html>502</html>".to_vec());
    }
    let files = stub.files.lock().unwrap().clone();
    let (low2, happy2) = ("catpope/map/sets/cathedral.low.v2.glb", "characters/catpope/catpope_portrait-happy.v2.png");
    *stub.api.lock().unwrap() = answer(&files, low2, NEUTRAL, happy2, Some((low2, glb("low").len())));
    let sync = DeckSync::start(stub.config(&cache));
    sync.prepare(scene());
    settle(&sync, &scene());
    assert!(stub.hits("/cdn/").iter().any(|h| h.ends_with("cathedral.low.v2.glb")), "it was tried");
    assert_eq!(std::fs::read(cache.join("sets/cathedral/cathedral.low.glb")).unwrap(), glb("low"), "the old set stays");
    assert_eq!(std::fs::read(cache.join("portraits/catpope/catpope_portrait-happy.png")).unwrap(), png(), "the old sprite stays");
    assert!(!cache.join("sets/cathedral/cathedral.low.part").exists());
    let line = sync.line();
    assert!(line.starts_with("SYNC ERR"), "{line}");
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn offline_the_stage_plays_from_the_cache() {
    let cache = dir("offline");
    {
        let stub = stub_with_files();
        let sync = DeckSync::start(stub.config(&cache));
        sync.prepare(scene());
        settle(&sync, &scene());
    }
    // nothing listens there any more
    let gone = listen();
    let port = gone.local_addr().unwrap().port();
    drop(gone);
    let cfg = Config {
        api: format!("http://127.0.0.1:{port}/api"),
        cdn: format!("http://127.0.0.1:{port}/cdn"),
        cache: cache.clone(),
        sets: Some(cache.join("sets")),
        portraits: Some(cache.join("portraits")),
        token: None,
    };
    let sync = DeckSync::start(cfg);
    let t = Instant::now();
    sync.prepare(scene());
    settle(&sync, &scene());
    assert!(t.elapsed() < Duration::from_secs(8), "settled at once, not after timeouts: {:?}", t.elapsed());
    assert_eq!(sync.line(), "SYNC OFFLINE - FROM THE CACHE");
    assert!(cache.join("sets/cathedral/cathedral.low.glb").is_file(), "the cache is untouched");
    // a character wanted later doesn't wait either
    assert!(sync.wait(Want::Portrait { character: "yellowmouse".into(), mood: "neutral".into() }, Duration::from_secs(5)));
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn a_character_wanted_later_is_asked_for_then() {
    let stub = stub_with_files();
    let cache = dir("later");
    let sync = DeckSync::start(stub.config(&cache));
    // a deck joins the conductor: its head asks for catpope's neutral sprite; no prepare named it
    assert!(sync.wait(Want::Portrait { character: "catpope".into(), mood: "neutral".into() }, Duration::from_secs(10)));
    let api = stub.hits("/api/");
    assert_eq!(api.len(), 1);
    assert!(api[0].contains("sets=&characters=catpope"), "{}", api[0]);
    assert!(cache.join("portraits/catpope/catpope_portrait-neutral.png").is_file());
    // an unknown set is named an error, the stage keeps going
    *stub.api.lock().unwrap() = r#"{"sets":{},"characters":{},"missing":{"sets":["crypt"],"characters":[]}}"#.into();
    assert!(sync.wait(Want::Set { id: "crypt".into(), quality: "low".into() }, Duration::from_secs(10)));
    let until = Instant::now() + Duration::from_secs(5);
    while sync.board.lock().unwrap().busy && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(sync.line(), "SYNC ERR SET CRYPT IS UNKNOWN TO THE API");
    let _ = std::fs::remove_dir_all(&cache);
}

/// the space side: the set loader's gate fetches before it reads, the heads' Source fetches then decodes
#[cfg(feature = "space")]
#[test]
fn the_space_node_s_loaders_go_through_the_sync() {
    use deck_stage::space::portrait::Source;
    let stub = stub_with_files();
    let cache = dir("space");
    let sync = DeckSync::start(stub.config(&cache));
    let file = deck_stage::cache::set_file(&cache.join("sets"), "cathedral", "low");
    assert!(!file.exists());
    (sync.gate())(&file);
    assert_eq!(std::fs::read(&file).unwrap(), glb("low"), "the gate returned once the file was there");
    let faces = sync.portraits().unwrap();
    assert_eq!(faces.load("catpope", "happy").unwrap().dimensions(), (4, 4));
    assert!(faces.load("catpope", "dizzy").unwrap_err().contains("catpope_portrait-dizzy"), "no sprite: the head falls back");
    let _ = std::fs::remove_dir_all(&cache);
}
