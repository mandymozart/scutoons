//! The conductor pushes a scene sync (#195), all in this process: the real relay on 127.0.0.1 (7000–7099), the stub of
//! scutoons.com (tests/common), the conductor's stage and decks with their own caches. A push reaches every deck, each
//! fetches through its sync (the same scene-content request and cache) and answers `deck.synced`; the conductor sees
//! every deck ready; a deck joining later finds the push in its welcome and syncs too; a push again skips the cache;
//! missing items come back as missing, a deck without the API as offline; a deck's own push is refused.
#![cfg(all(feature = "live", feature = "space", feature = "sync"))]
mod common;

use common::*;
use deck_native::control::Controls;
use deck_native::graph::{Graph, Kind};
use deck_native::scene::{Scene, CATHEDRAL_SCENE, CONDUCTOR_SCENE};
use deck_native::space::portrait::Portraits;
use deck_native::space::sync::View;
use deck_native::space::{link::Syncs, Connect, Space};
use deck_native::sync::{Config, DeckSync};
use deck_native::tempo;
use protocol::{Envelope, Hello, Message, Role, SceneSync};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

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

/// one stage: its graph, its control domain, its space – a deck with its own sync and cache
struct Stage {
    ctl: Controls,
    space: Space,
    sync: Option<DeckSync>,
    cache: Option<PathBuf>,
}

impl Stage {
    fn new(text: &str, relay: &str, sync: Option<(DeckSync, PathBuf)>) -> Stage {
        let scene = deck_native::modules::carried_part(&Scene::parse(text).unwrap());
        let graph = Graph::build(&scene).unwrap();
        let node = graph.first(Kind::Space).unwrap();
        let nowhere = Path::new("/nowhere");
        let mut space = Space::new(&graph, node, &scene.being, nowhere, Portraits::local(nowhere));
        let (sync, cache) = sync.map_or((None, None), |(s, c)| (Some(s), Some(c)));
        space.connect(Some(relay), Some("dev".into()), None, sync.clone().map(|s| Box::new(s) as Box<dyn Syncs>));
        Stage { ctl: Controls::new(&graph, None), space, sync, cache }
    }

    fn deck(relay: &str, stub_cfg: Config, cache: PathBuf) -> Stage {
        Stage::new(CATHEDRAL_SCENE, relay, Some((DeckSync::start(stub_cfg), cache)))
    }

    fn step(&mut self, t: f32) {
        let beat = tempo::Now { bpm: 84.0, pos: tempo::position(t as f64 * 1.4, 4), per_bar: 4, running: true };
        self.ctl.update(1.0 / 60.0, 0, &beat);
        self.space.update(1.0 / 60.0, t, &mut self.ctl, &beat, &|_, _| 0.3);
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        if let Some(c) = &self.cache {
            let _ = std::fs::remove_dir_all(c);
        }
    }
}

/// step every stage until `done` (10 s at most)
fn run(stages: &mut [&mut Stage], t: &mut f32, what: &str, done: &mut dyn FnMut(&[&mut Stage]) -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    while Instant::now() < until {
        for s in stages.iter_mut() {
            s.step(*t);
        }
        *t += 1.0 / 60.0;
        if done(stages) {
            return;
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    let views: Vec<_> = stages[0].space.others.iter().map(|r| (r.client.clone(), stages[0].space.sync.view(&r.client))).collect();
    panic!("timed out: {what} – the conductor sees {views:?}");
}

fn push(characters: &[&str]) -> SceneSync {
    SceneSync {
        scene: "cathedral".into(),
        sets: Some(vec!["cathedral".into()]),
        characters: Some(characters.iter().map(|c| c.to_string()).collect()),
        beings: None,
        decks: None,
    }
}

fn views(s: &Stage) -> Vec<Option<View>> {
    s.space.others.iter().map(|r| s.space.sync.view(&r.client)).collect()
}

#[test]
fn a_push_reaches_every_deck_and_ready_comes_back() {
    let stub = stub_with_files();
    let (_rt, port) = relay();
    let url = format!("ws://127.0.0.1:{port}/live/SYNC");
    let mut cond = Stage::new(CONDUCTOR_SCENE, &url, None);
    let caches: Vec<PathBuf> = ["a", "b", "late"].iter().map(|n| dir(&format!("push-{n}"))).collect();
    let mut a = Stage::deck(&url, stub.config(&caches[0]), caches[0].clone());
    let mut b = Stage::deck(&url, stub.config(&caches[1]), caches[1].clone());
    let mut t = 0.0f32;
    run(&mut [&mut cond, &mut a, &mut b], &mut t, "two decks around the conductor", &mut |s| s[0].space.others.len() == 2);
    assert!(!cond.space.decks_ready(), "nothing pushed yet");
    assert!(cond.space.push_sync(push(&["catpope"])));
    assert!(!a.space.push_sync(push(&[])), "a deck doesn't push");
    run(&mut [&mut cond, &mut a, &mut b], &mut t, "both decks ready", &mut |s| s[0].space.decks_ready());
    for c in &caches[..2] {
        assert_eq!(std::fs::read(c.join("sets/cathedral/cathedral.low.glb")).unwrap(), glb("low"), "{}", c.display());
        assert!(c.join("portraits/catpope/catpope_portrait-neutral.png").is_file() && c.join("portraits/catpope/catpope_portrait-happy.png").is_file());
    }
    // each deck: one API request, its three files (the deck's quality only)
    assert_eq!((stub.hits("/api/").len(), stub.hits("/cdn/").len()), (2, 6));
    assert!(stub.hits("/api/").iter().all(|h| h.contains("sets=cathedral&characters=catpope")));
    let lines = cond.space.session_lines();
    assert!(lines.iter().any(|l| l == "SYNC 2/2 READY CATHEDRAL"), "{lines:?}");
    let answer = a.sync.as_ref().unwrap().synced(0).unwrap().1;
    assert_eq!((answer.ready.clone(), answer.done, answer.total), (vec!["set:cathedral".to_string(), "character:catpope".to_string()], Some(3), Some(3)));
    assert_eq!(a.sync.as_ref().unwrap().push_line().unwrap(), "PUSH READY CATHEDRAL");
    // the word over each deck's being
    let mut labels = Vec::new();
    cond.space.write_labels(&mut labels);
    assert_eq!(labels.iter().filter(|l| l.text.as_str() == "READY").count(), 2);

    // a deck joining later finds the push in its welcome and syncs too
    let mut late = Stage::deck(&url, stub.config(&caches[2]), caches[2].clone());
    run(&mut [&mut cond, &mut a, &mut b, &mut late], &mut t, "the late deck ready too", &mut |s| s[0].space.others.len() == 3 && s[0].space.decks_ready());
    assert!(caches[2].join("sets/cathedral/cathedral.low.glb").is_file());
    assert_eq!(stub.hits("/cdn/").len(), 9);

    // pushed again (S): asked afresh, nothing fetched – the caches have it all
    assert!(cond.space.push_sync(push(&["catpope"])));
    assert_eq!(views(&cond), [Some(View::Waiting); 3], "a push forgets the old answers");
    run(&mut [&mut cond, &mut a, &mut b, &mut late], &mut t, "ready again", &mut |s| s[0].space.decks_ready());
    assert_eq!((stub.hits("/api/").len(), stub.hits("/cdn/").len()), (6, 9), "every file skipped");
}

#[test]
fn missing_items_come_back_missing_and_no_api_is_offline() {
    let stub = stub_with_files();
    *stub.api.lock().unwrap() = r#"{"sets":{"cathedral":{"model":{"low":{"path":"catpope/map/sets/cathedral.low.v1.glb","size":11}}}},"characters":{},"missing":{"sets":[],"characters":["ghost"]}}"#.into();
    let (_rt, port) = relay();
    let url = format!("ws://127.0.0.1:{port}/live/MISS");
    let mut cond = Stage::new(CONDUCTOR_SCENE, &url, None);
    let cache = dir("missing");
    let mut deck = Stage::deck(&url, stub.config(&cache), cache.clone());
    // a deck whose API is gone, its cache empty
    let gone = listen();
    let dead = gone.local_addr().unwrap().port();
    drop(gone);
    let off_cache = dir("offline-push");
    let off_cfg = Config { api: format!("http://127.0.0.1:{dead}/api"), cdn: format!("http://127.0.0.1:{dead}/cdn"), ..stub.config(&off_cache) };
    let mut off = Stage::deck(&url, off_cfg, off_cache.clone());
    let mut t = 0.0f32;
    run(&mut [&mut cond, &mut deck, &mut off], &mut t, "two decks", &mut |s| s[0].space.others.len() == 2);
    cond.space.push_sync(push(&["ghost"]));
    run(&mut [&mut cond, &mut deck, &mut off], &mut t, "one missing, one offline", &mut |s| {
        let mut v = views(s[0]);
        v.sort_by_key(|v| format!("{v:?}"));
        v == [Some(View::Missing(1)), Some(View::Offline)]
    });
    let answer = deck.sync.as_ref().unwrap().synced(0).unwrap().1;
    assert_eq!((answer.ready, answer.missing.clone()), (vec!["set:cathedral".to_string()], vec!["character:ghost".to_string()]));
    assert!(answer.why.unwrap()["character:ghost"].contains("unknown to the API"));
    let offline = off.sync.as_ref().unwrap().synced(0).unwrap().1;
    assert_eq!(offline.missing, ["set:cathedral", "character:ghost"], "honest: nothing is in its cache");
    assert!(!cond.space.decks_ready());
    let lines = cond.space.session_lines();
    assert!(lines.iter().any(|l| l == "SYNC 0/2 READY 2 NOT CATHEDRAL"), "{lines:?}");
}

/// a deck (not the conductor) that sends `scene.sync` itself is refused, and no deck gets it
#[test]
fn only_the_conductor_pushes() {
    use tungstenite::Message as Ws;
    let (_rt, port) = relay();
    let join = |nick: &str| {
        let (mut ws, _) = tungstenite::connect(format!("ws://127.0.0.1:{port}/live/AUTH?role=musician&group=decks")).unwrap();
        let hello = Hello { code: "AUTH".into(), role: Role::Musician, group: Some("decks".into()), nick: Some(nick.into()), token: None, resume: None };
        ws.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Hello(hello))).unwrap())).unwrap();
        ws
    };
    let next = |ws: &mut tungstenite::WebSocket<_>| -> Envelope {
        loop {
            if let Ws::Text(t) = ws.read().unwrap() {
                return serde_json::from_str(&t).unwrap();
            }
        }
    };
    let mut a = join("a");
    let mut b = join("b");
    assert_eq!(next(&mut a).message.kind(), "welcome");
    assert_eq!(next(&mut b).message.kind(), "welcome");
    a.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::SceneSync(push(&["catpope"])))).unwrap())).unwrap();
    let e = next(&mut a);
    assert!(matches!(&e.message, Message::Error(x) if x.code == "authority"), "{e:?}");
    // b hears nothing but the relay's own (a ping's pong proves the line is quiet otherwise)
    b.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Ping(protocol::Ping { t0: 7 }))).unwrap())).unwrap();
    assert_eq!(next(&mut b).message.kind(), "pong");
}
