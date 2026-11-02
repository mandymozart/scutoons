//! A deck's outlet against the real relay on 127.0.0.1 (#186), in the style of bridge/tests/timing.rs: the conductor
//! sends `media.preload` + a cell; the deck fetches with its token from a local file server, answers `media.ready`
//! (what is missing, and why), composes the cell (its 3D set → the space target, its picture into the plane); a deck
//! that starts again gets the outlet's snapshot (cell + preload) and shows the cell at once; a dropped connection
//! resumes and gets what was sent meanwhile. Ports 7000–7099 only (the machine's agreed slot); everything started here
//! stops with the test.
#![cfg(feature = "live")]

use deck_native::library::Library;
use deck_native::live::config::Config;
use deck_native::live::frame::{Frame, SpaceTarget};
use deck_native::live::media::{Fixed, HttpFetch, LocalSets, Media};
use deck_native::live::Live;
use image::{Rgba, RgbaImage};
use protocol::{CellChange, Envelope, Hello, MediaItem, MediaPreload, Message, Role, SpotMode};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message as Ws, WebSocket};

/// a free port in the agreed slot (7000–7099), held by the listener
fn listen() -> TcpListener {
    let start = 7000 + (std::process::id() % 50) as u16;
    (start..7100).chain(7000..start).find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok()).expect("a free port in 7000–7099")
}

/// The real relay on its own runtime (stops when the runtime drops).
fn relay() -> (tokio::runtime::Runtime, u16) {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let std_listener = listen();
    let port = std_listener.local_addr().unwrap().port();
    std_listener.set_nonblocking(true).unwrap();
    let listener = {
        let _in = rt.enter();
        tokio::net::TcpListener::from_std(std_listener).unwrap()
    };
    rt.spawn(relay::serve(listener, Arc::new(relay::Relay::new())));
    (rt, port)
}

/// A file server for show media: `/live/...` from `root`, only with `Authorization: Bearer T` (else 401).
struct Files {
    port: u16,
    stop: Arc<AtomicBool>,
    hits: Arc<Mutex<Vec<String>>>,
}

impl Files {
    fn start(root: PathBuf) -> Files {
        let l = listen();
        let port = l.local_addr().unwrap().port();
        l.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let hits = Arc::new(Mutex::new(vec![]));
        let (s, h) = (stop.clone(), hits.clone());
        std::thread::spawn(move || {
            while !s.load(Ordering::Relaxed) {
                match l.accept() {
                    Ok((stream, _)) => serve(stream, &root, &h),
                    Err(_) => std::thread::sleep(Duration::from_millis(10)),
                }
            }
        });
        Files { port, stop, hits }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn serve(mut s: TcpStream, root: &Path, hits: &Mutex<Vec<String>>) {
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
    hits.lock().unwrap().push(path.clone());
    let authorised = head.lines().any(|l| l.eq_ignore_ascii_case("authorization: bearer T"));
    let file = root.join(path.trim_start_matches('/'));
    let (status, body) = if !authorised {
        ("401 Unauthorized", b"{\"error\":\"token\"}".to_vec())
    } else if let Ok(bytes) = std::fs::read(&file) {
        ("200 OK", bytes)
    } else {
        ("404 Not Found", b"{}".to_vec())
    };
    let _ = write!(s, "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n", body.len());
    let _ = s.write_all(&body);
}

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

/// a build without the `space` module plays the cell without its 3D set (a HUD reason) – the rest is the same
const SPACE: bool = cfg!(feature = "space");

/// the sermon's plane: the pulpit's picture in the top left box
fn sermon(f: &Frame) -> bool {
    f.backdrop == SPACE && f.plane.get_pixel(10, 10).0 == [0, 180, 0, 255]
}

/// The conductor: joins with a (dev) token, sends, waits for a message.
struct Conductor(Socket);

impl Conductor {
    fn join(port: u16, code: &str) -> Conductor {
        let (mut ws, _) = tungstenite::connect(format!("ws://127.0.0.1:{port}/live/{code}")).unwrap();
        if let MaybeTlsStream::Plain(s) = ws.get_ref() {
            s.set_read_timeout(Some(Duration::from_millis(100))).unwrap();
        }
        let hello = Hello { code: code.into(), role: Role::Conductor, group: None, nick: None, token: Some("dev".into()), resume: None };
        ws.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Hello(hello))).unwrap())).unwrap();
        Conductor(ws)
    }

    fn send(&mut self, m: Message) {
        self.0.send(Ws::Text(serde_json::to_string(&Envelope::new("", m)).unwrap())).unwrap();
    }

    /// the first message within 10 s that `pick` takes
    fn wait<T>(&mut self, what: &str, mut pick: impl FnMut(&Message) -> Option<T>) -> T {
        let until = Instant::now() + Duration::from_secs(10);
        while Instant::now() < until {
            match self.0.read() {
                Ok(Ws::Text(t)) => {
                    let e: Envelope = serde_json::from_str(&t).unwrap();
                    if let Some(x) = pick(&e.message) {
                        return x;
                    }
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                Err(e) => panic!("{what}: the conductor's socket broke: {e}"),
            }
        }
        panic!("{what}: nothing within 10 s");
    }

    /// presence says `n` clients in group `left`
    fn outlets(&mut self, n: i64) {
        self.wait(&format!("{n} in left"), |m| match m {
            Message::Presence(p) if p.groups.get("left").copied().unwrap_or(0) == n => Some(()),
            _ => None,
        });
    }
}

/// the next plane that satisfies `ok` within 10 s
fn frame(rx: &Receiver<Frame>, what: &str, ok: impl Fn(&Frame) -> bool) -> Frame {
    let until = Instant::now() + Duration::from_secs(10);
    while let Some(left) = until.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(f) if ok(&f) => return f,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    panic!("{what}: no such plane within 10 s");
}

fn space(rx: &Receiver<SpaceTarget>) -> SpaceTarget {
    rx.recv_timeout(Duration::from_secs(10)).expect("a space target")
}

struct Deck {
    root: PathBuf,
    port: u16,
}

impl Deck {
    /// a deck's outlet `left` of session `code`: its cache and sets under `root`, the media server's token T
    fn start(&self, code: &str) -> Live {
        let cfg = Config::new(&format!("ws://127.0.0.1:{}/live/{code}", self.port), "left", "de").unwrap();
        let cache = self.root.join("cache");
        let lib = Library { api: "offline://api".into(), cdn: "offline://cdn".into(), cache: cache.clone() };
        let media = Media {
            cache,
            resolve: Box::new(LocalSets { sets: self.root.join("sets"), quality: "low".into(), synced: false }),
            fetch: Box::new(HttpFetch::default()),
            token: Box::new(Fixed("T".into())),
            local: Mutex::new(HashMap::new()),
        };
        Live::start(cfg, lib, media, (160, 90), Some("test-deck".into()))
    }
}

fn temp_root() -> PathBuf {
    let d = std::env::temp_dir().join(format!("deck-live-relay-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("www/live/catpope")).unwrap();
    std::fs::create_dir_all(d.join("sets/cathedral")).unwrap();
    RgbaImage::from_pixel(32, 18, Rgba([0, 180, 0, 255])).save(d.join("www/live/catpope/pulpit.png")).unwrap();
    std::fs::write(d.join("sets/cathedral/cathedral.low.glb"), "a set").unwrap();
    d
}

fn sermon_cell() -> protocol::Cell {
    serde_json::from_str(
        r##"{ "mode": "own", "layers": [
            { "id": "set", "src": { "kind": "set", "p": { "set": "cathedral", "spot": "nave-centre", "mode": "model3d" } }, "enter": { "ms": 1200 } },
            { "id": "pulpit", "src": { "kind": "image", "p": { "media": "pulpit" } }, "box": { "x": 0, "y": 0, "w": 50, "h": 50 } },
            { "id": "line", "src": { "kind": "text", "p": { "text": { "de": "Liebe Gemeinde" } } }, "box": { "x": 0, "y": 70, "w": 100, "h": 30 } }
        ] }"##,
    )
    .unwrap()
}

#[test]
fn preload_cells_ready_and_the_snapshot_on_a_restart() {
    let root = temp_root();
    let (_rt, port) = relay();
    let files = Files::start(root.join("www"));
    let deck = Deck { root: root.clone(), port };
    let mut cond = Conductor::join(port, "T186");
    cond.wait("welcome", |m| matches!(m, Message::Welcome(_)).then_some(()));

    let mut live = deck.start("T186");
    cond.outlets(1);

    // doors: the conductor preloads the outlet – a show medium, a set's 3D variant, one that isn't there
    let item = |id: &str, url: Option<String>, set: Option<(&str, SpotMode)>| MediaItem {
        id: id.into(),
        url,
        set: set.map(|s| s.0.into()),
        spot: set.map(|_| "nave-centre".into()),
        mode: set.map(|s| s.1),
        bytes: None,
    };
    let items = vec![
        item("pulpit", Some(files.url("/live/catpope/pulpit.png")), None),
        item("nave", None, Some(("cathedral", SpotMode::Model3d))),
        item("organ", Some(files.url("/live/catpope/organ.webm")), None),
    ];
    cond.send(Message::MediaPreload(MediaPreload { outlet: "left".into(), items: items.clone() }));
    let ready = cond.wait("media.ready", |m| if let Message::MediaReady(r) = m { Some(r.clone()) } else { None });
    assert_eq!(ready.outlet, "left");
    assert_eq!(ready.ready, ["pulpit", "nave"]);
    assert_eq!(ready.missing, ["organ"]);
    assert_eq!(ready.why.unwrap()["organ"], "HTTP 404");
    assert!(files.hits.lock().unwrap().contains(&"/live/catpope/pulpit.png".to_string()), "fetched with the token");

    // the sermon: the cell → the space target (the cathedral at the nave's centre) + the plane (the pulpit's picture)
    let frames = live.frames.take().unwrap();
    cond.send(Message::Cell(CellChange { outlet: "left".into(), cell: sermon_cell() }));
    let f = frame(&frames, "the sermon", sermon);
    assert_eq!((f.plane.width(), f.plane.height(), f.enter_ms), (160, 90, if SPACE { 1200 } else { 600 }), "the longest enter: the set's");
    assert_eq!(f.plane.get_pixel(120, 30).0[3], 0, "the space shows through");
    if SPACE {
        assert_eq!(space(&live.space), SpaceTarget { set: Some(root.join("sets/cathedral/cathedral.low.glb")), spot: Some("nave-centre".into()) });
    } else {
        assert!(live.hud_lines().iter().any(|l| l.contains("MODULE 'SPACE'")), "{:?}", live.hud_lines());
    }
    assert!(live.hud_lines()[0].starts_with("LIVE LEFT  OUTPUT "), "{:?}", live.hud_lines());

    // a dropped connection: the deck joins again with resume (its client id kept) and gets what was sent meanwhile
    let joined = live.hud_lines()[0].clone();
    live.rejoin();
    cond.outlets(0);
    let black: protocol::Cell = serde_json::from_str(r#"{ "mode": "black" }"#).unwrap();
    cond.send(Message::Cell(CellChange { outlet: "left".into(), cell: black }));
    cond.outlets(1);
    frame(&frames, "the black cell sent while the deck was away", |f| !f.backdrop && f.plane.pixels().all(|p| p.0[3] == 0));
    assert_eq!(live.hud_lines()[0], joined, "resumed as the same client");
    cond.send(Message::Cell(CellChange { outlet: "left".into(), cell: sermon_cell() }));
    frame(&frames, "the sermon again", sermon);

    // the deck starts again (a reboot): no resume – the snapshot has the outlet's cell and its preload
    drop(live);
    drop(frames);
    cond.outlets(0);
    let mut again = deck.start("T186");
    let frames = again.frames.take().unwrap();
    // (the cell may be composed a moment before the snapshot's preload has found the cached files: then once more)
    frame(&frames, "the snapshot's cell with the cached picture", sermon);
    let ready = cond.wait("media.ready after the restart", |m| if let Message::MediaReady(r) = m { Some(r.clone()) } else { None });
    assert_eq!((ready.ready.len(), ready.missing.clone()), (2, vec!["organ".to_string()]), "the snapshot's preload, from the cache");
    if SPACE {
        assert_eq!(space(&again.space).spot.as_deref(), Some("nave-centre"));
    }
    drop(again);
    drop(files);
    let _ = std::fs::remove_dir_all(&root);
}
