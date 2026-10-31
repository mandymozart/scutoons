//! The stub of scutoons.com the sync tests share (#195): one server on 127.0.0.1 (7000–7099) answers
//! `/api/library/scene-content` and serves the CDN files, counting every request.
#![allow(dead_code)]
use deck_stage::sync::{Config, DeckSync, Needs};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// a free port in the agreed slot (7000–7099), held by the listener
pub fn listen() -> TcpListener {
    let start = 7000 + (std::process::id() % 50) as u16 + 50;
    (start..7100).chain(7000..start).find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok()).expect("a free port in 7000–7099")
}

/// The stub: `/api/library/scene-content?…` answers `api` (the JSON the test sets), `/cdn/<path>` answers `files`.
pub struct Stub {
    pub port: u16,
    pub stop: Arc<AtomicBool>,
    pub hits: Arc<Mutex<Vec<String>>>,
    pub api: Arc<Mutex<String>>,
    pub files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Stub {
    pub fn start() -> Stub {
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

    pub fn config(&self, cache: &Path) -> Config {
        Config {
            api: format!("http://127.0.0.1:{}/api", self.port),
            cdn: format!("http://127.0.0.1:{}/cdn", self.port),
            cache: cache.into(),
            sets: Some(cache.join("sets")),
            portraits: Some(cache.join("portraits")),
            token: Some("T".into()),
            quality: "low".into(),
        }
    }

    pub fn hits(&self, prefix: &str) -> Vec<String> {
        self.hits.lock().unwrap().iter().filter(|h| h.starts_with(prefix)).cloned().collect()
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn serve(mut s: TcpStream, hits: &Mutex<Vec<String>>, api: &Mutex<String>, files: &Mutex<HashMap<String, Vec<u8>>>) {
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

pub fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("deck-sync-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

pub fn glb(tag: &str) -> Vec<u8> {
    format!("glTF\x02\0\0\0{tag}").into_bytes()
}

pub fn png() -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(4, 4, image::Rgba([9, 8, 7, 255])).write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

/// the answer for the cathedral set (its low 3D: `low`) + catpope (neutral, happy), each with its file's real size
/// unless `said` names a path and the size the API says instead
pub fn answer(files: &HashMap<String, Vec<u8>>, low: &str, neutral: &str, happy: &str, said: Option<(&str, usize)>) -> String {
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

pub const LOW: &str = "catpope/map/sets/cathedral.low.v1.glb";
pub const NEUTRAL: &str = "characters/catpope/catpope_portrait-neutral.png";
pub const HAPPY: &str = "characters/catpope/catpope_portrait-happy.png";

pub fn scene() -> Needs {
    Needs { sets: vec![("cathedral".into(), "low".into())], characters: vec![("catpope".into(), vec!["neutral".into(), "happy".into(), "dizzy".into()])] }
}

/// wait until the sync has settled every want of `needs`
pub fn settle(sync: &DeckSync, needs: &Needs) {
    for w in needs.wants() {
        assert!(sync.wait(w.clone(), Duration::from_secs(10)), "{w:?} settled");
    }
    let until = Instant::now() + Duration::from_secs(5);
    while sync.board.lock().unwrap().busy && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub fn stub_with_files() -> Stub {
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

