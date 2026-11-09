//! Events between linked decks over the real relay on 127.0.0.1 (#442, deck-space far.rs): a deck named `left` plays
//! catpope and has an event output; another deck's far world node names `/deck:left/scene:nave/character:catpope/analog:seq.notes`.
//! The second deck asks (`deck.listen`), the first answers with what leaves that output (`deck.events`), and the note
//! lands in the far node's ring with the second engine's names. Each link runs its socket thread as on a deck; the
//! engines are stood in by a queue and a list. Ports 7100–7199 only; everything stops with the test.
#![cfg(all(feature = "live", feature = "space"))]

use deck_events::{Event, Names, Time};
use deck_native::space::link::Link;
use deck_native::space::Join;
use deck_space::far::{FarInlet, FarOutlet, FarTable};
use deck_space::space::wire::Wire;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const ADDRESS: &str = "/deck:left/scene:nave/character:catpope/analog:seq.notes";

fn relay() -> (tokio::runtime::Runtime, u16) {
    let start = 7110 + (std::process::id() % 30) as u16;
    let std_listener = (start..7150).chain(7100..start).find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok()).expect("a free port in 7100–7149");
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

fn names() -> Names {
    let mut n = Names::new();
    n.event("music.note").unwrap();
    n.key("pitch").unwrap();
    n
}

#[test]
fn a_far_world_node_hears_the_named_decks_output_over_the_relay() {
    let (_rt, port) = relay();
    let url = format!("ws://127.0.0.1:{port}/live/FAR");
    // the deck `left`: one event output, nothing far
    let queue: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
    let q = queue.clone();
    let left = Link::start(&url, Join::Deck, None, Some("character.catpope".into()), None, None, None).unwrap();
    left.far(FarTable {
        name: Some("left".into()),
        subject: "character.catpope".into(),
        names: names(),
        inlets: Vec::new(),
        outlets: vec![FarOutlet { slot: "analog_seq.notes".into(), pop: Box::new(move || q.lock().unwrap().pop()) }],
        tempo: None,
    });
    // the asking deck: one far world node cabled once
    let got: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
    let g = got.clone();
    let asking = Link::start(&url, Join::Deck, None, Some("character.boy".into()), None, None, None).unwrap();
    let their = names();
    let (note, pitch) = (their.get("music.note").unwrap(), their.get("pitch").unwrap());
    asking.far(FarTable {
        name: None,
        subject: "character.boy".into(),
        names: their,
        inlets: vec![FarInlet {
            address: ADDRESS.into(),
            listen: vec!["music.note".into()],
            push: Box::new(move |e| {
                g.lock().unwrap().push(e);
                true
            }),
        }],
        outlets: Vec::new(),
        tempo: None,
    });
    // `left` drains its output every poll and drops what nobody asked for, so a fresh note goes in until one arrives
    let mine = names();
    let (my_note, my_pitch) = (mine.get("music.note").unwrap(), mine.get("pitch").unwrap());
    let t0 = Instant::now();
    loop {
        queue.lock().unwrap().push(Event::beats(my_note, 1.0).lasting(Time::Beats(0.5)).with(my_pitch, 440.0));
        if !got.lock().unwrap().is_empty() {
            break;
        }
        assert!(t0.elapsed() < Duration::from_secs(20), "no far note within 20 s – left: {} · asking: {}", left.status(), asking.status());
        std::thread::sleep(Duration::from_millis(50));
    }
    let e = got.lock().unwrap()[0].clone();
    assert_eq!((e.name, e.start), (note, Time::Now), "the asking engine's name, played on arrival");
    assert_eq!(e.dur, Some(Time::Seconds(0.25)), "half a beat at 120 bpm (no tempo: the default)");
    assert_eq!(e.props.get(pitch), Some(440.0));
    assert_eq!(e.source.as_deref(), Some(&format!("far:{ADDRESS}")[..]));
    drop(asking);
    drop(left);
}
