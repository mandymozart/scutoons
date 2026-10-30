//! The Cathedral of decks over the real relay on 127.0.0.1 (#188 phase 4), all in this process: the conductor's stage
//! (scenes/conductor.toml, `join = "conductor"`) and three decks (scenes/cathedral.toml, `join = "deck"`, each another
//! character) join one session. The decks stream their beings; the conductor gathers them into a circle around its own,
//! mirrors their values (a button turned on a deck shows on its being at the conductor), draws a hose per route, sizes
//! its buffers anew; a deck that leaves is gone from the circle. Ports 7000–7099 only; everything stops with the test.
#![cfg(all(feature = "live", feature = "space"))]

use deck_stage::control::Controls;
use deck_stage::graph::{Graph, Kind};
use deck_stage::scene::{Scene, CATHEDRAL_SCENE, CONDUCTOR_SCENE};
use deck_stage::space::portrait::Portraits;
use deck_stage::space::Space;
use deck_stage::tempo;
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn listen() -> TcpListener {
    let start = 7050 + (std::process::id() % 40) as u16;
    (start..7100).chain(7000..start).find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok()).expect("a free port in 7000–7099")
}

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

/// one stage: its graph, its control domain, its space
struct Stage {
    graph: Graph,
    ctl: Controls,
    space: Space,
}

impl Stage {
    fn new(text: &str, character: Option<&str>, relay: &str) -> Stage {
        let mut scene = deck_stage::modules::carried_part(&Scene::parse(text).unwrap());
        if let Some(c) = character {
            scene.being.character = c.into();
        }
        let graph = Graph::build(&scene).unwrap();
        let node = graph.first(Kind::Space).unwrap();
        let nowhere = Path::new("/nowhere");
        let mut space = Space::new(&graph, node, &scene.being, nowhere, Portraits::local(nowhere));
        space.connect(Some(relay), Some("dev".into()), None);
        let ctl = Controls::new(&graph, None);
        Stage { graph, ctl, space }
    }

    fn step(&mut self, t: f32) {
        let beat = tempo::Now { bpm: 84.0, pos: tempo::position(t as f64 * 1.4, 4), per_bar: 4, running: true };
        self.ctl.update(1.0 / 60.0, 0, &beat);
        self.space.update(1.0 / 60.0, t, &mut self.ctl, &beat, &|_, _| 0.3);
    }
}

/// step every stage (the show's time `t` moves a frame per step; the network needs a little real time) until `done`
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
    panic!("timed out: {what}");
}

#[test]
fn decks_gather_around_the_conductor() {
    let (_rt, port) = relay();
    let url = format!("ws://127.0.0.1:{port}/live/CATH");
    let mut cond = Stage::new(CONDUCTOR_SCENE, None, &url);
    let mut decks: Vec<Stage> = ["catpope", "yellowmouse", "billthefrog"].iter().map(|c| Stage::new(CATHEDRAL_SCENE, Some(c), &url)).collect();
    let mut t = 0.0f32;
    let generation = cond.space.generation;
    let (_, tubes_alone) = cond.space.capacity();
    {
        let [a, b, c] = &mut decks[..] else { unreachable!() };
        run(&mut [&mut cond, a, b, c], &mut t, "three decks around the conductor with a route each", &mut |s| {
            s[0].space.others.len() == 3 && s[0].space.routes.len() == 3
        });
    }
    let sp = &cond.space;
    let mut chars: Vec<&str> = sp.others.iter().map(|r| r.being.look.character.as_str()).collect();
    chars.sort();
    assert_eq!(chars, ["billthefrog", "catpope", "yellowmouse"], "each deck's own head");
    assert!(sp.generation > generation && sp.capacity().1 > tubes_alone, "buffers sized for the congregation");
    // every route ends in the conductor's own being (None), from a deck
    assert!(sp.routes.iter().all(|r| r.from.is_some() && r.to.is_none()), "{:?}", sp.routes.iter().map(|r| (r.from, r.to)).collect::<Vec<_>>());
    // the same body as on the deck: a point per node, a hose per link
    let deck_being = &decks[0].space.being;
    assert!(sp.others.iter().all(|r| r.being.points.len() == deck_being.points.len() && r.being.hoses.len() == deck_being.hoses.len()));
    assert!(sp.being.look.size > deck_being.look.size, "the conductor's being is the bigger one");

    // a button turned on the first deck shows on its being at the conductor
    let space_node = decks[0].graph.first(Kind::Space).unwrap();
    let glow = Kind::Space.input("glow").unwrap();
    decks[0].ctl.set(space_node, glow, 1.7);
    let client = {
        // the deck that is catpope
        let r = cond.space.others.iter().find(|r| r.being.look.character == "catpope").unwrap();
        r.client.clone()
    };
    {
        let [a, b, c] = &mut decks[..] else { unreachable!() };
        run(&mut [&mut cond, a, b, c], &mut t, "deck 0's glow at the conductor", &mut |s| {
            let r = s[0].space.others.iter().find(|r| r.client == client).unwrap();
            r.being.buttons.iter().any(|b| b.name == "glow" && (b.value - 1.7).abs() < 1e-3)
        });
    }
    // the circle: around the conductor's being, at the ring's radius, the decks drifting into their places
    let center = cond.space.being.anchor;
    for r in &cond.space.others {
        let d = r.place - center;
        assert!((Vec2(d.x, d.z).len() - 3.0).abs() < 1e-3 && d.y.abs() < 1e-3, "{d}");
    }

    // one deck leaves: gone from the circle and its hose with it
    decks.pop();
    {
        let [a, b] = &mut decks[..] else { unreachable!() };
        run(&mut [&mut cond, a, b], &mut t, "two decks left", &mut |s| s[0].space.others.len() == 2 && s[0].space.routes.len() == 2);
    }
    assert!(cond.space.others.iter().all(|r| r.being.look.character != "billthefrog"));
    let lines = cond.space.session_lines();
    assert!(lines[0].starts_with("CONDUCTOR C") && lines[0].ends_with(" 2 DECKS 2 ROUTES"), "{lines:?}");
}

struct Vec2(f32, f32);
impl Vec2 {
    fn len(&self) -> f32 {
        (self.0 * self.0 + self.1 * self.1).sqrt()
    }
}
