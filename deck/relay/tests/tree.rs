//! The acceptance of #90, headless: a parent and a child conductor on separate relays, each with its own decks and readers, over
//! real WebSockets. Rows, cells, events and the clock pass down; values, readiness and events up. If the parent goes away the
//! child keeps playing its last row.

use std::sync::Arc;
use std::time::{Duration, Instant};

use deck_show::address::{receive, Receiver};
use deck_show::{flash, parse, Assignment, ClockChoice, Lock, Logged, Pinned, Subtree, Word};
use futures_util::{SinkExt, StreamExt};
use protocol::{DeckGraph, DeckValues, Envelope, Flash, Hello, Message, Role, Tempo};
use relay::tree::{keep_linked, Parent};
use relay::{now_ms, Relay};
use serde_json::json;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio_tungstenite::tungstenite::Message as Ws;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

const TOP: &str = r#"{
  "id": "top", "version": 1,
  "outlets": [{ "id": "left" }],
  "seats": [{ "id": "foyer", "show": { "id": "foyer-loop", "version": 2 }, "sits": "deck:foyer", "word": "follow", "lockable": true,
              "uses": { "rows": ["idle", "storm"], "events": ["door"], "outlets": ["wall"] } }],
  "rows": [
    { "id": "open", "fire": { "event": "start", "how": "auto" }, "cells": { "left": { "mode": "play", "piece": "scene:nave" }, "foyer": { "mode": "go", "row": "idle" } } },
    { "id": "rain", "fire": { "event": "ch.storm", "how": "auto" }, "cells": { "foyer": { "mode": "go", "row": "storm" } } },
    { "id": "doorbell", "fire": { "event": "door", "how": "auto" }, "cells": { "left": { "mode": "black" } } }
  ]
}"#;

const FOYER: &str = r#"{
  "id": "foyer-loop", "version": 2,
  "outlets": [{ "id": "wall" }],
  "rows": [
    { "id": "idle", "cells": { "wall": { "mode": "play", "piece": "scene:fireflies" } } },
    { "id": "storm", "cells": { "wall": { "mode": "play", "piece": "scene:storm" } } },
    { "id": "backstage", "fire": { "event": "door", "how": "hand" }, "cells": { "wall": { "mode": "black" } } }
  ],
  "face": { "rows": ["idle", "storm"], "events": ["door"], "outlets": ["wall"] }
}"#;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct Peer {
    ws: Socket,
    id: String,
    /// a conductor's whole mind; none = a deck or a phone
    sub: Option<Subtree>,
    inbox: Vec<Envelope>,
}

impl Peer {
    async fn send(&mut self, e: Envelope) {
        self.ws.send(Ws::Text(serde_json::to_string(&e).unwrap().into())).await.unwrap();
    }

    async fn all(&mut self, out: Vec<Envelope>) {
        for e in out {
            self.send(e).await;
        }
    }

    async fn poll(&mut self) {
        let now = now_ms() as u64;
        if let Some(sub) = &mut self.sub {
            let out = sub.tick(now);
            self.all(out).await;
        }
        let Ok(Some(Ok(frame))) = tokio::time::timeout(Duration::from_millis(5), self.ws.next()).await else { return };
        let Ws::Text(text) = frame else { return };
        let e: Envelope = serde_json::from_str(&text).unwrap();
        if let Some(sub) = &mut self.sub {
            let out = sub.on_wire(&e, now);
            self.all(out).await;
        }
        self.inbox.push(e);
    }

    fn cell(&self, outlet: &str) -> Option<String> {
        self.inbox.iter().rev().find_map(|e| match &e.message {
            Message::Cell(c) if c.outlet == outlet => Some(match c.cell.layers.as_deref().and_then(|l| l.first()).map(|l| &l.src) {
                Some(protocol::LayerSrc::Piece(p)) => p.piece.clone(),
                _ => "black".to_string(),
            }),
            _ => None,
        })
    }

    fn sub(&self) -> &Subtree {
        self.sub.as_ref().unwrap()
    }

    fn sub_mut(&mut self) -> &mut Subtree {
        self.sub.as_mut().unwrap()
    }

    fn logged(&self, f: impl Fn(&Logged) -> bool) -> bool {
        self.sub().log().any(|e| f(&e.what))
    }
}

async fn wait(peers: &mut [&mut Peer], until: impl Fn(&[&mut Peer]) -> bool) {
    let start = Instant::now();
    while !until(peers) {
        assert!(start.elapsed() < Duration::from_secs(8), "timed out");
        for p in peers.iter_mut() {
            p.poll().await;
        }
    }
}

/// a little more time for what must NOT arrive
async fn linger(peers: &mut [&mut Peer]) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(250) {
        for p in peers.iter_mut() {
            p.poll().await;
        }
    }
}

async fn join(port: u16, code: &str, role: Role, group: Option<&str>, token: Option<&str>, sub: Option<Subtree>) -> Peer {
    let url = format!("ws://127.0.0.1:{port}/live/{code}");
    let (mut ws, _) = connect_async(&url).await.unwrap();
    let hello = Hello { code: code.into(), role, group: group.map(str::to_owned), nick: None, token: token.map(str::to_owned), resume: None };
    ws.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Hello(hello))).unwrap().into())).await.unwrap();
    loop {
        let frame = tokio::time::timeout(Duration::from_secs(3), ws.next()).await.expect("a welcome").unwrap().unwrap();
        let Ws::Text(text) = frame else { continue };
        let e: Envelope = serde_json::from_str(&text).unwrap();
        if let Message::Welcome(w) = &e.message {
            // the welcome stays in the inbox: its snapshot is what a client finds when it joins
            let id = w.client.clone();
            return Peer { ws, id, sub, inbox: vec![e] };
        }
    }
}

async fn relay_on(port: u16) -> (u16, Runtime) {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let listener = rt.spawn(async move { TcpListener::bind(("127.0.0.1", port)).await.unwrap() }).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    rt.spawn(relay::serve(listener, Arc::new(Relay::new())));
    (port, rt)
}

async fn child_relay() -> (u16, Arc<Relay>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let relay = Arc::new(Relay::new());
    tokio::spawn(relay::serve(listener, relay.clone()));
    (port, relay)
}

fn show(json: &str, clock: ClockChoice) -> Option<Subtree> {
    Some(Subtree::new(parse(json).unwrap(), clock))
}

fn story(id: &str) -> Envelope {
    Envelope::new("", Message::Story(json!({ "type": "showMessage", "id": id }).as_object().unwrap().clone()))
}

fn link(port: u16) -> Parent {
    Parent { url: format!("ws://127.0.0.1:{port}"), code: "TOP".into(), seat: "foyer".into(), token: "t".into(), session: "FOY".into() }
}

struct Two {
    parent_rt: Runtime,
    parent_port: u16,
    child_port: u16,
    top: Peer,
    left: Peer,
    foyer: Peer,
    wall: Peer,
}

/// Two relays linked, with a conductor and a deck on each.
async fn two(clock: ClockChoice) -> Two {
    let (parent_port, parent_rt) = relay_on(0).await;
    let (child_port, child) = child_relay().await;
    let top = join(parent_port, "TOP", Role::Conductor, None, Some("t"), show(TOP, ClockChoice::Parent)).await;
    let left = join(parent_port, "TOP", Role::Output, Some("left"), None, None).await;
    let foyer = join(child_port, "FOY", Role::Conductor, None, Some("t"), show(FOYER, clock)).await;
    let wall = join(child_port, "FOY", Role::Output, Some("wall"), None, None).await;
    tokio::spawn(keep_linked(child, link(parent_port)));
    let mut t = Two { parent_rt, parent_port, child_port, top, left, foyer, wall };
    let (top, foyer) = (&mut t.top, &mut t.foyer);
    wait(&mut [top, foyer], |p| p[0].inbox.iter().any(|e| matches!(&e.message, Message::Presence(x) if x.seats.as_ref().is_some_and(|s| s.contains_key("foyer"))))).await;
    t
}

fn now() -> u64 {
    now_ms() as u64
}

macro_rules! fire {
    ($p:expr, $row:expr) => {{
        let out = $p.sub_mut().fire($row, now());
        $p.all(out).await;
    }};
}

macro_rules! go {
    ($p:expr) => {{
        let out = $p.sub_mut().go(now());
        $p.all(out).await;
    }};
}

// ---------------------------------------------------------------- item 1 and 2: the reader, the tree

#[tokio::test(flavor = "multi_thread")]
async fn a_phone_reader_fires_the_parents_row_and_the_child_follows_it_over_two_relays() {
    let mut t = two(ClockChoice::Parent).await;
    let mut phone = join(t.parent_port, "TOP", Role::Reader, None, None, None).await;
    phone.send(story("start")).await;

    let (top, left, foyer, wall) = (&mut t.top, &mut t.left, &mut t.foyer, &mut t.wall);
    wait(&mut [top, left, foyer, wall], |p| p[3].cell("wall").is_some() && p[1].cell("left").is_some()).await;
    assert_eq!(t.left.cell("left").as_deref(), Some("scene:nave"));
    assert_eq!(t.wall.cell("wall").as_deref(), Some("scene:fireflies"));
    assert_eq!(t.foyer.sub().conductor().current(), Some("idle"));
    assert_eq!(t.top.sub().conductor().current(), Some("open"));
    t.parent_rt.shutdown_background();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_reader_is_not_allowed_more_than_its_story_and_nobody_else_hears_it() {
    let mut t = two(ClockChoice::Parent).await;
    let mut phone = join(t.parent_port, "TOP", Role::Reader, None, None, None).await;
    phone.send(Envelope::new("", Message::Panic(protocol::Panic { black: Some(true), stop: None }))).await;
    phone.send(story("nobody.listens")).await;
    let (top, left, ph) = (&mut t.top, &mut t.left, &mut phone);
    wait(&mut [top, left, ph], |p| p[2].inbox.iter().any(|e| matches!(&e.message, Message::Error(x) if x.code == "authority"))).await;
    let (top, left, ph) = (&mut t.top, &mut t.left, &mut phone);
    wait(&mut [top, left, ph], |p| p[0].logged(|l| matches!(l, Logged::NoRow { .. }))).await;
    assert!(!t.left.inbox.iter().any(|e| matches!(e.message, Message::Story(_))), "a reader's event is the conductor's");
    t.parent_rt.shutdown_background();
}

#[tokio::test(flavor = "multi_thread")]
async fn values_readiness_and_events_come_up_with_a_path() {
    let mut t = two(ClockChoice::Parent).await;
    let graph = DeckGraph { nodes: vec![], links: vec![], character: None, size: None, moods: None, expose: None };
    t.wall.send(Envelope::new("", Message::DeckGraph(graph))).await;
    let values = DeckValues { values: [("a.b".to_string(), vec![1.0])].into_iter().collect(), mood: None };
    t.wall.send(Envelope::new("", Message::DeckValues(values))).await;
    t.wall.send(Envelope::new("", Message::Ready(protocol::Empty {}))).await;
    let wall_id = t.wall.id.clone();
    let (top, foyer) = (&mut t.top, &mut t.foyer);
    wait(&mut [top, foyer], |p| p[0].inbox.iter().any(|e| matches!(e.message, Message::DeckValues(_)))).await;
    let v = t.top.inbox.iter().find(|e| matches!(e.message, Message::DeckValues(_))).unwrap();
    assert_eq!(v.from, format!("foyer/{wall_id}"));
    // the child's readiness, in the parent's presence
    let (top, foyer) = (&mut t.top, &mut t.foyer);
    wait(&mut [top, foyer], |p| p[0].inbox.iter().any(|e| matches!(&e.message, Message::Presence(x) if x.seats.as_ref().is_some_and(|s| s["foyer"].ready == Some(1) && s["foyer"].online == Some(true))))).await;

    // the child's reader: its own conductor's row (on hand: ignored) and, the event being on the face, the parent's row
    let mut phone = join(t.child_port, "FOY", Role::Reader, None, None, None).await;
    phone.send(story("door")).await;
    let (top, left, foyer, ph) = (&mut t.top, &mut t.left, &mut t.foyer, &mut phone);
    wait(&mut [top, left, foyer, ph], |p| p[1].cell("left").is_some()).await;
    assert_eq!(t.left.cell("left").as_deref(), Some("black"), "the parent's row `doorbell`");
    assert_eq!(t.foyer.sub().conductor().current(), None, "on the child that event is on hand");
    t.parent_rt.shutdown_background();
}

// ---------------------------------------------------------------- the clock

fn tempo(bpm: f64) -> Tempo {
    Tempo { bpm, per_bar: 4, beat: 0.0, running: true, at: None, domain: None }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_clock_passes_down_unless_a_subtree_has_its_own() {
    let mut t = two(ClockChoice::Parent).await;
    let out = t.top.sub_mut().lead_clock(tempo(120.0), now());
    t.top.all(out).await;
    let (top, foyer, wall) = (&mut t.top, &mut t.foyer, &mut t.wall);
    wait(&mut [top, foyer, wall], |p| p[2].inbox.iter().any(|e| matches!(&e.message, Message::Tempo(x) if x.bpm == 120.0))).await;
    assert!(t.foyer.sub().clock_at(now()).is_some(), "the child's moments fall on the parent's clock");
    t.parent_rt.shutdown_background();

    // its own time: the parent's anchor is logged and goes no further, and its own stays below it
    let mut t = two(ClockChoice::Own("foyer".into())).await;
    let out = t.foyer.sub_mut().lead_clock(tempo(90.0), now());
    t.foyer.all(out).await;
    let (top, foyer, wall) = (&mut t.top, &mut t.foyer, &mut t.wall);
    wait(&mut [top, foyer, wall], |p| p[2].inbox.iter().any(|e| matches!(&e.message, Message::Tempo(x) if x.domain.as_deref() == Some("foyer")))).await;
    let out = t.top.sub_mut().lead_clock(tempo(140.0), now());
    t.top.all(out).await;
    let (top, foyer, wall) = (&mut t.top, &mut t.foyer, &mut t.wall);
    wait(&mut [top, foyer, wall], |p| p[1].logged(|l| *l == Logged::ClockKept)).await;
    let (top, foyer, wall) = (&mut t.top, &mut t.foyer, &mut t.wall);
    linger(&mut [top, foyer, wall]).await;
    assert!(!t.wall.inbox.iter().any(|e| matches!(&e.message, Message::Tempo(x) if x.bpm == 140.0)), "the child's decks stay on the child's time");
    assert!(!t.top.inbox.iter().any(|e| matches!(&e.message, Message::Tempo(x) if x.bpm == 90.0)), "a domain does not go up");
    assert_eq!(t.foyer.sub().clock_at(now()).map(|c| c.bpm), Some(90.0));
    t.parent_rt.shutdown_background();
}

// ---------------------------------------------------------------- the parent goes away

#[tokio::test(flavor = "multi_thread")]
async fn if_the_parent_goes_away_the_child_keeps_playing_and_goes_on_by_hand() {
    let mut t = two(ClockChoice::Parent).await;
    fire!(t.top, "open");
    let (top, foyer, wall) = (&mut t.top, &mut t.foyer, &mut t.wall);
    wait(&mut [top, foyer, wall], |p| p[2].cell("wall").is_some()).await;

    let parent_port = t.parent_port;
    let (rt, top) = (t.parent_rt, t.top);
    drop(top);
    rt.shutdown_background();

    // the link is gone, the child is told, what plays plays on: no new cell arrives
    let n = t.wall.inbox.len();
    let (foyer, wall) = (&mut t.foyer, &mut t.wall);
    wait(&mut [foyer, wall], |p| p[0].logged(|l| *l == Logged::ParentLost)).await;
    assert_eq!(t.foyer.sub().conductor().current(), Some("idle"));
    assert_eq!(t.wall.inbox[n..].iter().filter(|e| matches!(e.message, Message::Cell(_))).count(), 0, "nothing changed on the stage");

    // the operator goes on: the local GO always works
    go!(t.foyer);
    let (foyer, wall) = (&mut t.foyer, &mut t.wall);
    wait(&mut [foyer, wall], |p| p[1].cell("wall").as_deref() == Some("scene:storm")).await;

    // the parent comes back on its port: the link finds it, and a word reaches the child again
    let (port, rt) = relay_on(parent_port).await;
    assert_eq!(port, parent_port);
    let mut top = join(port, "TOP", Role::Conductor, None, Some("t"), show(TOP, ClockChoice::Parent)).await;
    let (tp, foyer) = (&mut top, &mut t.foyer);
    wait(&mut [tp, foyer], |p| p[1].logged(|l| *l == Logged::ParentBack)).await;
    fire!(top, "open");
    let (tp, foyer, wall) = (&mut top, &mut t.foyer, &mut t.wall);
    wait(&mut [tp, foyer, wall], |p| p[2].cell("wall").as_deref() == Some("scene:fireflies")).await;
    rt.shutdown_background();
}

// ---------------------------------------------------------------- item 3: sub conductors

#[tokio::test(flavor = "multi_thread")]
async fn the_word_follow_suggest_free_and_the_local_go_always_works() {
    let mut t = two(ClockChoice::Parent).await;
    // suggest: the child's own word (the nearest wins): its operator decides
    t.foyer.sub_mut().set_word(Some(Word::Suggest), now()).ok();
    fire!(t.top, "rain");
    let (top, foyer) = (&mut t.top, &mut t.foyer);
    wait(&mut [top, foyer], |p| p[1].sub().conductor().suggestion().is_some()).await;
    assert_eq!(t.foyer.sub().conductor().current(), None, "a suggestion fires nothing");
    go!(t.foyer);
    let (foyer, wall) = (&mut t.foyer, &mut t.wall);
    wait(&mut [foyer, wall], |p| p[1].cell("wall").as_deref() == Some("scene:storm")).await;

    // free: ignored, the local GO still works
    t.foyer.sub_mut().set_word(Some(Word::Free), now()).ok();
    fire!(t.top, "open");
    let (top, foyer) = (&mut t.top, &mut t.foyer);
    wait(&mut [top, foyer], |p| p[1].logged(|l| matches!(l, Logged::ParentIgnored { row: Some(r) } if r == "idle"))).await;
    assert_eq!(t.foyer.sub().conductor().current(), Some("storm"));
    fire!(t.foyer, "backstage");
    let (foyer, wall) = (&mut t.foyer, &mut t.wall);
    wait(&mut [foyer, wall], |p| p[1].cell("wall").as_deref() == Some("black")).await;

    // back to the parent's word: follow
    t.foyer.sub_mut().set_word(None, now()).ok();
    fire!(t.top, "rain");
    let (top, foyer, wall) = (&mut t.top, &mut t.foyer, &mut t.wall);
    wait(&mut [top, foyer, wall], |p| p[2].cell("wall").as_deref() == Some("scene:storm")).await;
    t.parent_rt.shutdown_background();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_lock_from_above_wins_over_the_childs_word_and_goes_away_again() {
    let mut t = two(ClockChoice::Parent).await;
    t.foyer.sub_mut().set_word(Some(Word::Free), now()).ok();
    let out = t.top.sub_mut().lock_seat("foyer", true, now());
    t.top.all(out).await;
    let (top, foyer) = (&mut t.top, &mut t.foyer);
    wait(&mut [top, foyer], |p| p[1].sub().locked()).await;
    assert!(t.foyer.sub_mut().set_word(Some(Word::Free), now()).is_err(), "not under a lock");
    assert!(t.foyer.logged(|l| *l == Logged::WordRefused));

    // the word in force is the parent's, though the child had said free
    fire!(t.top, "rain");
    let (top, foyer, wall) = (&mut t.top, &mut t.foyer, &mut t.wall);
    wait(&mut [top, foyer, wall], |p| p[2].cell("wall").as_deref() == Some("scene:storm")).await;
    // GO is always allowed, also under a lock
    fire!(t.foyer, "backstage");
    let (foyer, wall) = (&mut t.foyer, &mut t.wall);
    wait(&mut [foyer, wall], |p| p[1].cell("wall").as_deref() == Some("black")).await;

    // unlocked: the child's own word is the one in force again (the nearest wins)
    let out = t.top.sub_mut().lock_seat("foyer", false, now());
    t.top.all(out).await;
    let (top, foyer) = (&mut t.top, &mut t.foyer);
    wait(&mut [top, foyer], |p| !p[1].sub().locked()).await;
    assert_eq!(t.foyer.sub().word(), Some(Word::Free));
    t.parent_rt.shutdown_background();
}

// ---------------------------------------------------------------- item 4: the flash

fn flash_lock() -> Lock {
    Lock { shows: vec![Pinned { id: "top".into(), version: 1 }, Pinned { id: "foyer-loop".into(), version: 3 }] }
}

/// What a plain deck does with a flash: the one rule, and its report.
async fn deck_takes(deck: &mut Peer, standing: &mut Option<Assignment>, cannot: Option<&str>) {
    let f = deck.inbox.iter().rev().find_map(|e| match &e.message {
        Message::Flash(f) => Some(f.clone()),
        _ => None,
    });
    let f: Flash = f.expect("a flash");
    let report = flash::take(standing, &f, &Pinned { id: "deck".into(), version: 0 }, cannot);
    deck.send(Envelope::new("", Message::FlashReport(report))).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_flash_goes_down_the_tree_and_every_deck_reports_what_it_runs() {
    let mut t = two(ClockChoice::Parent).await;
    let out = t.top.sub_mut().flash(&flash_lock(), false, now());
    t.top.all(out).await;
    let (top, foyer, wall) = (&mut t.top, &mut t.foyer, &mut t.wall);
    wait(&mut [top, foyer, wall], |p| p[2].inbox.iter().any(|e| matches!(e.message, Message::Flash(_)))).await;
    // the child took it at one hop from the operator, its deck at two
    assert_eq!(t.foyer.sub().assignment().map(|a| a.hops), Some(1));
    assert_eq!(t.foyer.sub().wanted(), Some(&Pinned { id: "foyer-loop".into(), version: 3 }), "a version it has yet to fetch");
    let (mut standing, wall) = (None, &mut t.wall);
    deck_takes(wall, &mut standing, Some("no screen")).await;
    let (top, foyer, wall) = (&mut t.top, &mut t.foyer, &mut t.wall);
    wait(&mut [top, foyer, wall], |p| p[0].logged(|l| matches!(l, Logged::Reported { refused: Some(why), .. } if why == "no screen"))).await;

    // the child's own report came up too, with its path, and after the install the new version runs
    assert!(t.top.logged(|l| matches!(l, Logged::Reported { from, .. } if from.starts_with("foyer/"))));
    let mut doc = parse(FOYER).unwrap();
    doc.version = 3;
    let out = t.foyer.sub_mut().install(doc, now());
    t.foyer.all(out).await;
    let (top, foyer) = (&mut t.top, &mut t.foyer);
    wait(&mut [top, foyer], |p| p[0].logged(|l| matches!(l, Logged::Reported { running, .. } if running == "foyer-loop@3"))).await;
    t.parent_rt.shutdown_background();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_nearer_hand_keeps_its_flash_and_a_lock_overrides_it() {
    let mut t = two(ClockChoice::Parent).await;
    let mine = Lock { shows: vec![Pinned { id: "foyer-loop".into(), version: 2 }] };
    let out = t.foyer.sub_mut().flash(&mine, false, now());
    t.foyer.all(out).await;
    let out = t.top.sub_mut().flash(&flash_lock(), false, now());
    t.top.all(out).await;
    let (top, foyer) = (&mut t.top, &mut t.foyer);
    wait(&mut [top, foyer], |p| p[1].logged(|l| matches!(l, Logged::FlashRefused { why } if why == "assigned by a nearer hand"))).await;
    assert_eq!(t.foyer.sub().assignment().map(|a| a.hops), Some(0));

    let out = t.top.sub_mut().flash(&flash_lock(), true, now());
    t.top.all(out).await;
    let (top, foyer) = (&mut t.top, &mut t.foyer);
    wait(&mut [top, foyer], |p| p[1].sub().assignment().is_some_and(|a| a.hops == 1 && a.locked)).await;
    t.parent_rt.shutdown_background();
}

fn last_flash(p: &Peer) -> Flash {
    p.inbox.iter().rev().find_map(|e| match &e.message {
        Message::Flash(f) => Some(f.clone()),
        _ => None,
    })
    .expect("a flash")
}

/// #471: the operator unticked the top's deck; the foyer's deck takes it, and a deck that joins the foyer later finds it.
#[tokio::test(flavor = "multi_thread")]
async fn a_chosen_flash_leaves_out_who_was_unticked_and_a_deck_that_joins_later_finds_it() {
    use deck_show::ready::Deck;
    let mut t = two(ClockChoice::Parent).await;
    let pick = deck_show::choose::Pick { skip: vec![t.left.id.clone()], by: Some("ana".into()), ..Default::default() };
    let out = t.top.sub_mut().flash_to(&flash_lock(), &pick, now());
    t.top.all(out).await;
    let (top, foyer, left, wall) = (&mut t.top, &mut t.foyer, &mut t.left, &mut t.wall);
    let got = |p: &Peer| p.inbox.iter().any(|e| matches!(e.message, Message::Flash(_)));
    wait(&mut [top, foyer, left, wall], |p| got(p[2]) && got(p[3])).await;
    let mut left = Deck::new("left");
    left.set_id(&t.left.id);
    let refused = left.take(&last_flash(&t.left)).unwrap_err();
    assert_eq!(refused.why.as_deref(), Some("left out by the conductor"));
    t.left.send(Envelope::new("", Message::FlashReport(*refused))).await;
    let (top, left_peer) = (&mut t.top, &mut t.left);
    wait(&mut [top, left_peer], |p| p[0].inbox.iter().any(|e| matches!(e.message, Message::FlashReport(_)))).await;
    // a conductor that joins later finds what each deck last said
    let second = join(t.parent_port, "TOP", Role::Conductor, None, Some("t"), None).await;
    let reports = second.inbox.iter().find_map(|e| match &e.message {
        Message::Welcome(w) => w.snapshot.reports.clone(),
        _ => None,
    });
    assert!(reports.is_some_and(|r| r.contains_key(&t.left.id)));
    let mut wall = Deck::new("wall");
    wall.set_id(&t.wall.id);
    assert_eq!(wall.take(&last_flash(&t.wall)).unwrap().to_string(), "foyer-loop@3");
    // not here when it went out: the flash waits in the session's snapshot
    let late = join(t.child_port, "FOY", Role::Output, Some("wall"), None, None).await;
    let kept = late.inbox.iter().find_map(|e| match &e.message {
        Message::Welcome(w) => w.snapshot.flash.clone(),
        _ => None,
    });
    let mut deck = Deck::new("wall");
    deck.set_id(&late.id);
    assert_eq!(deck.take(&kept.expect("the flash in a late deck's snapshot")).unwrap().to_string(), "foyer-loop@3");
    t.parent_rt.shutdown_background();
}

// ---------------------------------------------------------------- item 5: addressing

async fn answer(deck: &mut Peer, plays: &[String], group: &str) {
    let a = deck.inbox.iter().rev().find_map(|e| match &e.message {
        Message::Address(a) => Some(a.clone()),
        _ => None,
    });
    let a = a.expect("an address");
    let r = Receiver { group: Some(group), device: Some("native"), plays, handles: &|act| act == "dance:samba" };
    if let Some(ans) = receive(&a, r) {
        deck.send(Envelope::new("", Message::AddressAnswer(ans))).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_address_goes_down_the_tree_and_the_log_counts_what_it_found() {
    let mut t = two(ClockChoice::Parent).await;
    let out = t.top.sub_mut().address("character:yellowmouse", "dance:samba", None, now());
    t.top.all(out).await;
    let (top, foyer, left, wall) = (&mut t.top, &mut t.foyer, &mut t.left, &mut t.wall);
    wait(&mut [top, foyer, left, wall], |p| p.iter().skip(2).all(|d| d.inbox.iter().any(|e| matches!(e.message, Message::Address(_))))).await;
    answer(&mut t.wall, &["character:yellowmouse.ghost".to_string()], "wall").await;
    answer(&mut t.left, &["scene:nave".to_string()], "left").await; // says nothing
    let (top, foyer, left, wall) = (&mut t.top, &mut t.foyer, &mut t.left, &mut t.wall);
    wait(&mut [top, foyer, left, wall], |p| p[0].logged(|l| matches!(l, Logged::Reached { reached: 1, passed: 1, answered: 1, .. }))).await;
    t.parent_rt.shutdown_background();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_address_that_finds_nobody_stays_silent_and_is_logged() {
    let mut t = two(ClockChoice::Parent).await;
    let filter = protocol::AddressFilter { group: Some("nowhere".into()), device: None };
    let out = t.top.sub_mut().address("location:*", "close_doors", Some(filter), now());
    t.top.all(out).await;
    let (top, foyer) = (&mut t.top, &mut t.foyer);
    wait(&mut [top, foyer], |p| p[0].logged(|l| matches!(l, Logged::Reached { reached: 0, passed: 0, answered: 0, .. }))).await;
    t.parent_rt.shutdown_background();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cell_goes_down_to_an_outlet_the_seat_uses_while_the_child_follows() {
    let mut t = two(ClockChoice::Parent).await;
    let cell: protocol::Cell = serde_json::from_value(json!({ "mode": "black" })).unwrap();
    fire!(t.top, "open"); // the seat's word reaches the child with the row
    let (top, foyer, wall) = (&mut t.top, &mut t.foyer, &mut t.wall);
    wait(&mut [top, foyer, wall], |p| p[2].cell("wall").is_some()).await;
    let out = t.top.sub_mut().cell_down("foyer", "wall", cell, now());
    t.top.all(out).await;
    let (top, foyer, wall) = (&mut t.top, &mut t.foyer, &mut t.wall);
    wait(&mut [top, foyer, wall], |p| p[2].cell("wall").as_deref() == Some("black")).await;
    t.parent_rt.shutdown_background();
}
