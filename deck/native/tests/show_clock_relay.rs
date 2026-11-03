//! The show clock over the real relay on 127.0.0.1 (#321, plan P8 + the owner's MIDI-in addition, 2026-11-03): a deck on
//! MIDI clock in – the table-top clock's bytes arrive on its clock's channel stamped on arrival, as midir's callback
//! sends them (tempo/midi.rs `connect_in`) – leads; a second deck on `Source::Relay` follows. Both through their own
//! relay link for the clock alone (no being), each frame thread at 60 Hz. The follower's beat against the leader's at
//! the same moment stays within the P8 bound (≤ 5 ms p95 on a LAN – here the loopback); the table-top clock goes away:
//! the last tempo holds on both, the leader's HUD says so. Ports 7100–7199 only; everything stops with the test.
#![cfg(all(feature = "live", feature = "space"))]

use deck_native::clock::{now_ns, Moment};
use deck_native::space::link::Link;
use deck_native::space::Join;
use deck_native::tempo::relay::ShowLink;
use deck_native::tempo::{Config, Event, Shared, Source, Tempo, What};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn relay() -> (tokio::runtime::Runtime, u16) {
    let start = 7150 + (std::process::id() % 40) as u16;
    let std_listener = (start..7200).chain(7100..start).find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok()).expect("a free port in 7100–7199");
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

/// a deck's clock and its link, stepped by its own frame thread at 60 Hz (no audio: the wall clock's frames)
struct Deck {
    tempo: Arc<Mutex<Tempo>>,
    _link: Link,
    stop: Arc<AtomicBool>,
}

impl Deck {
    fn start(relay: &str, cfg: Config, name: &str) -> Deck {
        let show = Arc::new(ShowLink::default());
        let link = Link::start(relay, Join::Deck, None, Some(name.into()), None, None, Some(show.clone())).unwrap();
        let tempo = Arc::new(Mutex::new(Tempo::new(cfg.clone(), Arc::new(Shared::default()), 48_000.0)));
        tempo.lock().unwrap().set_source(cfg.source);
        let stop = Arc::new(AtomicBool::new(false));
        let (t, s) = (tempo.clone(), stop.clone());
        let origin = now_ns();
        std::thread::spawn(move || {
            while !s.load(Ordering::Relaxed) {
                {
                    let mut t = t.lock().unwrap();
                    show.feed(&mut t);
                    t.frame(&Moment::wall(now_ns(), origin, 48_000.0));
                    show.after(&mut t);
                }
                std::thread::sleep(Duration::from_millis(16));
            }
        });
        Deck { tempo, _link: link, stop }
    }

    /// the beat this deck plays at `ns` (its published map, the wall clock's frames)
    fn beat(&self, ns: u64) -> (f64, f64) {
        let t = self.tempo.lock().unwrap();
        let (map, _) = t.shared().map().unwrap();
        let line = t.shared().line().unwrap();
        (map.beat_at(line.frames_at(ns as f64 * 1e-9)), map.bpm)
    }
}

impl Drop for Deck {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[test]
fn a_deck_on_midi_clock_in_leads_the_show_through_the_relay() {
    let (_rt, port) = relay();
    let url = format!("ws://127.0.0.1:{port}/live/CLOCK");
    let leader = Deck::start(&url, Config { source: Source::Midi, ..Config::default() }, "table top");
    let follower = Deck::start(&url, Config { source: Source::Relay, ..Config::default() }, "follower");
    // the table-top clock: 126 BPM, 24 ticks a beat, through the leader's clock channel as midir's callback sends them
    let tx = leader.tempo.lock().unwrap().sender();
    let ticking = Arc::new(AtomicBool::new(true));
    let on = ticking.clone();
    let clock = std::thread::spawn(move || {
        let spt = 60.0 / 126.0 / 24.0;
        let start = now_ns();
        let _ = tx.send(Event { at_ns: start, what: What::Midi([0xFA, 0, 0]) });
        let mut k = 0u64;
        while on.load(Ordering::Relaxed) {
            let due = start + ((k as f64 + 1.0) * spt * 1e9) as u64;
            while now_ns() < due {
                std::thread::sleep(Duration::from_micros(300));
            }
            let _ = tx.send(Event { at_ns: now_ns(), what: What::Midi([0xF8, 0, 0]) });
            k += 1;
        }
    });
    // the follower locks (its filter, the leader's first anchors), then the two are compared for four seconds
    std::thread::sleep(Duration::from_secs(4));
    let mut errors = Vec::new();
    for _ in 0..200 {
        let ns = now_ns();
        let ((l, bpm), (f, _)) = (leader.beat(ns), follower.beat(ns));
        errors.push(((f - l) / (bpm / 60.0) * 1e3).abs());
        std::thread::sleep(Duration::from_millis(20));
    }
    errors.sort_by(f64::total_cmp);
    let p95 = errors[errors.len() * 95 / 100];
    eprintln!("show clock over the loopback relay: p95 {p95:.3} ms, max {:.3} ms", errors[errors.len() - 1]);
    assert!(p95 <= 5.0, "the follower is {p95:.3} ms off the leader (p95)");
    let (_, bpm) = follower.beat(now_ns());
    assert!((bpm - 126.0).abs() < 0.5, "the follower's tempo: {bpm}");
    assert!(follower.tempo.lock().unwrap().status().starts_with("SHOW"), "{}", follower.tempo.lock().unwrap().status());
    assert!(leader.tempo.lock().unwrap().status().ends_with("LEADS"));
    // the table top stops sending: the last tempo holds – on the leader (its HUD says so) and on the follower
    ticking.store(false, Ordering::Relaxed);
    clock.join().unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    let status = leader.tempo.lock().unwrap().status();
    assert!(status.starts_with("NO CLOCK - HOLDS 126"), "{status}");
    let ns = now_ns();
    let ((l, lbpm), (f, fbpm)) = (leader.beat(ns), follower.beat(ns));
    assert!((lbpm - 126.0).abs() < 0.5 && (fbpm - 126.0).abs() < 0.5, "{lbpm} {fbpm}");
    assert!(((f - l) / 2.1 * 1e3).abs() < 5.0, "still together: {:.3} ms", (f - l) / 2.1 * 1e3);
}
