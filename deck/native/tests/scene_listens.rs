//! A scene that listens, on the native deck, against the real relay on 127.0.0.1 (#517, research/SCENE-EVENTS.md §4 E2a):
//! two decks play the published `scene:stage` from their caches – one with the listener `joinScene → join`, one without.
//! The listening deck's link asks for the readers' story (`deck.story`); a stub reader sends `joinScene` with Cat Pope;
//! the deck's cast side (live/cast.rs) hands it to the core, Cat Pope stands at `left` and the changed scene is written for
//! the switch. The deck that did not ask hears nothing and its stage stays empty. Ports 7150–7199 only; everything stops
//! with the test.
#![cfg(all(feature = "live", feature = "space"))]

use deck_native::live::cast::Side;
use deck_native::space::link::Link;
use deck_native::space::Join;
use deck_space::published::SceneRef;
use deck_space::relay::In;
use deck_space::space::wire::Wire;
use protocol::{Envelope, Hello, Message, Role};
use serde_json::{json, Value};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::Message as Ws;

const STAGE: &str = include_str!("../../../server/tests/fixtures/scenes/templates/stage.json");
const PATIENCE: Duration = Duration::from_secs(30);

fn relay() -> (tokio::runtime::Runtime, u16) {
    let start = 7160 + (std::process::id() % 30) as u16;
    let std_listener = (start..7199).chain(7150..start).find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok()).expect("a free port in 7150–7198");
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let port = std_listener.local_addr().unwrap().port();
    std_listener.set_nonblocking(true).unwrap();
    let listener = {
        let _in = rt.enter();
        tokio::net::TcpListener::from_std(std_listener).unwrap()
    };
    rt.spawn(relay::serve(listener, Arc::new(relay::Relay::new())));
    (rt, port)
}

/// a deck's cache with `scene:stage` published (version 3) and kept – with these listeners, or none
fn cache(name: &str, listener: Option<Value>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("deck-listens-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut doc: Value = serde_json::from_str(STAGE).unwrap();
    doc["version"] = json!(3);
    if let Some(l) = listener {
        doc["listener"] = l;
    }
    let body = json!({ "status": "success", "scene": doc }).to_string();
    let api = |path: &str| if path.starts_with("/scenes/stage") { (200, body.clone()) } else { (404, "{}".to_string()) };
    deck_native::published::load(SceneRef::parse("scene:stage").unwrap().unwrap(), &dir.join("scenes"), api).unwrap();
    dir
}

/// a phone reading the chapter: joins as `reader`, sends its story events
struct Reader(tungstenite::WebSocket<MaybeTlsStream<std::net::TcpStream>>);

impl Reader {
    fn join(port: u16, code: &str) -> Reader {
        let (mut ws, _) = tungstenite::connect(format!("ws://127.0.0.1:{port}/live/{code}")).unwrap();
        if let MaybeTlsStream::Plain(s) = ws.get_ref() {
            s.set_read_timeout(Some(Duration::from_millis(50))).unwrap();
        }
        let hello = Hello { code: code.into(), role: Role::Reader, group: None, nick: None, token: None, device: None, proof: None, resume: None };
        ws.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Hello(hello))).unwrap().into())).unwrap();
        // #527: it drives the session's story (only the driving reader's story goes on)
        ws.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::ReaderDrive(Default::default()))).unwrap().into())).unwrap();
        Reader(ws)
    }

    fn story(&mut self, event: Value) {
        let Value::Object(p) = event else { panic!("an object") };
        self.0.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Story(p))).unwrap().into())).unwrap();
        // what the relay says back (its welcome, pongs) is read and dropped
        while self.0.read().is_ok() {}
    }
}

/// what came for a deck's playing scene so far, and whether a story was among it
fn came(link: &Link) -> (Vec<In>, bool) {
    let mut out = Vec::new();
    while let Some(i) = link.recv() {
        out.push(i);
    }
    let story = out.iter().any(|i| matches!(i, In::Story(_)));
    (out, story)
}

fn cast_of(side: &Side) -> Vec<(String, String)> {
    side.cast().unwrap().playing().resolved().cast.iter().map(|m| (m.holder.clone(), m.place.clone())).collect()
}

#[test]
fn a_listening_deck_s_scene_takes_a_readers_join_and_one_that_did_not_ask_hears_nothing() {
    let (_rt, port) = relay();
    let url = format!("ws://127.0.0.1:{port}/live/LISTEN");
    let (listens, quiet) = (cache("listens", Some(json!([{ "on": "joinScene", "do": "join" }]))), cache("quiet", None));
    let mut sides = [Side::default(), Side::default()];
    sides[0].scene(&listens, Some("scene:stage"));
    sides[1].scene(&quiet, Some("scene:stage"));
    assert!(sides[0].listens() && !sides[1].listens());
    assert!(cast_of(&sides[0]).is_empty() && cast_of(&sides[1]).is_empty(), "the stage plays empty");
    let links = [
        Link::start(&url, Join::Deck, None, Some("listens".into()), None, None, None).unwrap(),
        Link::start(&url, Join::Deck, None, Some("quiet".into()), None, None, None).unwrap(),
    ];
    for (l, s) in links.iter().zip(&sides) {
        l.story(s.listens());
    }
    let mut reader = Reader::join(port, "LISTEN");
    let join = json!({ "type": "joinScene", "id": "ch3-e12", "params": { "characters": [{ "character": "catpope" }] } });
    // the reader sends until the asking deck's ask stands at the relay and the event arrives
    let t0 = Instant::now();
    let file = loop {
        reader.story(join.clone());
        std::thread::sleep(Duration::from_millis(250));
        let (ins, _) = came(&links[0]);
        if let Some(file) = sides[0].take(&listens, ins) {
            break file;
        }
        let (ins, story) = came(&links[1]);
        assert!(!story, "the deck that did not ask got a story");
        assert_eq!(sides[1].take(&quiet, ins), None);
        assert!(t0.elapsed() < PATIENCE, "no joinScene within {PATIENCE:?} – listens: {} · quiet: {}", links[0].status(), links[1].status());
    };
    assert_eq!(cast_of(&sides[0]), [("left".to_string(), "character:catpope".to_string())], "Cat Pope stands at left");
    assert!(Path::new(&file).starts_with(&listens) && std::fs::read_to_string(&file).unwrap().contains("left_look"), "the changed scene is written for the switch");
    // the same event once more: she stands there already – nothing to switch to
    reader.story(join);
    std::thread::sleep(Duration::from_millis(500));
    let (ins, _) = came(&links[0]);
    assert_eq!(sides[0].take(&listens, ins), None);
    // the deck that did not ask: nothing came, nothing changed
    let (ins, story) = came(&links[1]);
    assert!(!story);
    assert_eq!(sides[1].take(&quiet, ins), None);
    assert!(cast_of(&sides[1]).is_empty(), "without a listener: nothing");
    drop(links);
    for d in [listens, quiet] {
        let _ = std::fs::remove_dir_all(d);
    }
}
