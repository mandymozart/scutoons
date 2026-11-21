//! A show row's camera cut over the real relay on 127.0.0.1 (#582 "relay end to end"): the show conductor (deck-show's
//! `Subtree`, as the editor runs it) fires its rows and its envelopes go to the relay as the conductor's socket; the
//! deck – its Space on its own relay link, and its outlet of the cell's column `left` beside it, as a deck's main
//! starts them – takes the camera word `scene:stage > shot:<id>` and looks through the shot: the orthographic one
//! orthographic, the next row's one with its own lens. A word that comes before the scene's camera is kept until it
//! arrives. Ports 7200–7249 only; everything stops with the test.
//!
//! No sleep stands in for a result (#639): each wait is for a STATE (a deck joined, the camera looking through the shot)
//! with a deadline that is paid only when the test fails; the stage steps on a virtual clock (a frame a step).
#![cfg(all(feature = "live", feature = "space"))]

use deck_native::control::Controls;
use deck_native::graph::{Graph, Kind};
use deck_native::scene::{Scene, CATHEDRAL_SCENE};
use deck_native::space::portrait::Portraits;
use deck_native::space::{Connect, Space};
use deck_native::tempo;
use deck_show::subtree::{ClockChoice, Subtree};
use deck_space::scene_camera::SceneCamera;
use deck_space::space::dop::Viewing;
use protocol::{Envelope, Hello, Message, Role};
use serde_json::json;
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::Message as Ws;

const SHOW: &str = include_str!("../../show/fixtures/camera-shot.show.json");
/// a deadline for something to HAPPEN, paid only when the test fails (#382)
const PATIENCE: Duration = Duration::from_secs(60);

fn relay() -> (tokio::runtime::Runtime, u16) {
    let start = 7200 + (std::process::id() % 25) as u16;
    let std_listener = (start..7250).chain(7200..start).find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok()).expect("a free port in 7200–7249");
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

fn camera() -> SceneCamera {
    serde_json::from_value(json!({
        "lens": { "focal": 30.0 },
        "shot": [
            { "id": "wide", "target": [0.0, 1.0, 0.0], "yaw": 20.0, "pitch": 8.0, "distance": 6.0, "lens": { "focal": 50.0 } },
            { "id": "iso", "target": [0.0, 0.5, 0.0], "yaw": 45.0, "pitch": 35.0, "distance": 12.0, "lens": { "projection": "orthographic", "size": 5.0 } }
        ]
    }))
    .unwrap()
}

/// a deck's stage: its graph, its control domain, its space on the relay link; the frame steps on a virtual clock
struct Stage {
    ctl: Controls,
    space: Space,
    t: f32,
}

impl Stage {
    fn new(relay: &str, column: &str) -> Stage {
        let scene = deck_native::modules::carried_part(&Scene::parse(CATHEDRAL_SCENE).unwrap());
        let graph = Graph::build(&scene).unwrap();
        let node = graph.first(Kind::Space).unwrap();
        let nowhere = Path::new("/nowhere");
        let mut space = Space::new(&graph, node, &scene.being, nowhere, Portraits::local(nowhere));
        space.set_view(1280.0, 720.0);
        space.column = Some(column.into());
        space.connect(Some(relay), Some("dev".into()), Some("camera-deck".into()), None, None, None);
        let ctl = Controls::new(&graph, None);
        Stage { ctl, space, t: 0.0 }
    }

    fn step(&mut self) {
        let beat = tempo::Now { bpm: 84.0, pos: tempo::position(self.t as f64 * 1.4, 4), per_bar: 4, running: true };
        self.ctl.update(1.0 / 60.0, 0, &beat);
        self.space.update(1.0 / 60.0, self.t, &mut self.ctl, &beat, &|_, _| 0.3);
        self.t += 1.0 / 60.0;
    }

    /// frames until `done`, the network getting a moment's real time between them
    fn until(&mut self, what: &str, mut done: impl FnMut(&Space) -> bool) {
        let end = Instant::now() + PATIENCE;
        while Instant::now() < end {
            self.step();
            if done(&self.space) {
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panic!("timed out: {what} (viewing {:?}, ortho {:?}, link: {})", self.space.viewing(), self.space.camera.ortho, self.space.session_lines().join(" | "));
    }
}

type Socket = tungstenite::WebSocket<MaybeTlsStream<std::net::TcpStream>>;

fn join(port: u16, code: &str, role: Role, group: Option<&str>, token: Option<&str>) -> Socket {
    let (mut ws, _) = tungstenite::connect(format!("ws://127.0.0.1:{port}/live/{code}")).unwrap();
    if let MaybeTlsStream::Plain(s) = ws.get_ref() {
        s.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
    }
    let hello = Hello { code: code.into(), role, group: group.map(Into::into), nick: None, token: token.map(Into::into), device: Some("native".into()), proof: None, resume: None };
    ws.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Hello(hello))).unwrap().into())).unwrap();
    ws
}

fn send(ws: &mut Socket, envelopes: Vec<Envelope>) {
    for e in envelopes {
        ws.send(Ws::Text(serde_json::to_string(&e).unwrap().into())).unwrap();
    }
}

/// what a socket has been told so far (its welcome, presence, the words that came), read without waiting
fn heard(ws: &mut Socket) -> Vec<Message> {
    let mut out = Vec::new();
    loop {
        match ws.read() {
            Ok(Ws::Text(t)) => out.push(serde_json::from_str::<Envelope>(&t).unwrap().message),
            Ok(_) => {}
            Err(_) => return out,
        }
    }
}

#[test]
fn a_shows_camera_cuts_reach_the_deck_over_the_relay() {
    let (_rt, port) = relay();
    let url = format!("ws://127.0.0.1:{port}/live/CAM");
    let mut stage = Stage::new(&url, "left");
    // the deck of the other column plays the same scene and is not the cell's
    let mut other = Stage::new(&url, "right");
    other.space.set_camera(None, Some(camera()));
    stage.space.set_camera(None, Some(camera()));
    // the deck's outlet of the cell's column, as a deck's main starts it beside the space's link
    let mut outlet = join(port, "CAM", Role::Output, Some("left"), None);
    let mut conductor = join(port, "CAM", Role::Conductor, None, Some("dev"));
    let mut show = Subtree::new(deck_show::document::parse(SHOW).unwrap(), ClockChoice::Parent);

    // the conductor sees the deck: only then does a word have somewhere to go
    let end = Instant::now() + PATIENCE;
    while !heard(&mut conductor).iter().any(|m| matches!(m, Message::Presence(p) if p.decks.as_ref().is_some_and(|d| !d.is_empty()))) {
        stage.step();
        other.step();
        assert!(Instant::now() < end, "the deck never joined: {}", stage.space.session_lines().join(" | "));
        std::thread::sleep(Duration::from_millis(5));
    }

    // the opening row picks no shot: the scene's own camera, no word
    send(&mut conductor, show.fire("opening", 0));
    // the overhead row's cell picks the orthographic shot
    send(&mut conductor, show.fire("overhead", 1_000));
    stage.until("the deck looks through shot iso, orthographically", |s| matches!(s.viewing(), Viewing::Shot(i) if i == "iso") && s.camera.ortho.is_some_and(|h| (h - 5.0).abs() < 0.05));

    // a shot change mid-show: the next row's cell, the perspective shot with its own lens (50 mm = 27°)
    send(&mut conductor, show.fire("closer", 2_000));
    stage.until("the deck cuts to shot wide with its 50 mm lens", |s| {
        matches!(s.viewing(), Viewing::Shot(i) if i == "wide") && s.camera.ortho.is_none() && (s.camera.fov_y.to_degrees() - 27.0).abs() < 0.2
    });

    // the right column's deck heard the same session and its camera stayed where it was
    for _ in 0..30 {
        other.step();
    }
    assert!(matches!(other.space.viewing(), Viewing::Primary | Viewing::Shot(_)) && !matches!(other.space.viewing(), Viewing::Shot(i) if i == "iso" || i == "wide"), "{:?}", other.space.viewing());

    // the outlet of column `left` was sent the cells and the words alike; the deck did not need it for the camera
    assert!(heard(&mut outlet).iter().any(|m| matches!(m, Message::Cell(_))), "the outlet hears its cells");
}
