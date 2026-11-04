//! The show clock over the real relay on 127.0.0.1 (#321, plan P8 + the owner's MIDI-in addition, 2026-11-03): a deck on
//! MIDI clock in – the table-top clock's bytes arrive on its clock's channel stamped on arrival, as midir's callback
//! sends them (tempo/midi.rs `connect_in`) – leads; a second deck on `Source::Relay` follows. Both through their own
//! relay link for the clock alone (no being), each frame thread at 60 Hz. The follower's beat against the leader's at
//! the same moment stays within the P8 bound (≤ 5 ms p95 on a LAN – here the loopback); the table-top clock goes away:
//! the last tempo holds on both, the leader's HUD says so. Ports 7100–7199 only; everything stops with the test.
//!
//! No wall-clock assertion in deck:ci (#382): the first test waits for each STATE (locked, leading, holding) as long as a
//! loaded machine needs; the show clock's accuracy on a virtual clock is deck-core's tests/show_clock.rs and the headless
//! scenarios 04–06. The real-time p95 over the loopback is the second test: `#[ignore]`, for a quiet machine or the
//! hardware rig (`cargo test -p deck-native --test show_clock_relay -- --ignored`).
#![cfg(all(feature = "live", feature = "space"))]

use deck_native::clock::{now_ns, Moment};
use deck_native::space::link::Link;
use deck_native::space::Join;
use deck_native::tempo::relay::ShowLink;
use deck_native::tempo::{Config, Event, Shared, Source, Tempo, What};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

/// How long the test waits for a state before it gives up: a deadline, not a measurement (paid only on failure).
const PATIENCE: Duration = Duration::from_secs(60);

/// polls `ok` every 20 ms until it holds; the panic names `what` and the state `show` reads
fn until(what: &str, show: impl Fn() -> String, ok: impl Fn() -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !ok() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}: {}", show());
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn status(d: &Deck) -> String {
    d.tempo.lock().unwrap().status()
}

fn bpm_of(d: &Deck) -> f64 {
    d.beat(now_ns()).1
}

/// the venue's table-top clock: both decks on one relay room, the leader on MIDI clock in, 126 BPM, 24 ticks a beat
/// through the leader's clock channel as midir's callback sends them
struct Rig {
    _rt: tokio::runtime::Runtime,
    leader: Deck,
    follower: Deck,
    ticking: Arc<AtomicBool>,
    clock: Option<std::thread::JoinHandle<()>>,
}

impl Rig {
    fn start() -> Rig {
        let (rt, port) = relay();
        let url = format!("ws://127.0.0.1:{port}/live/CLOCK");
        let leader = Deck::start(&url, Config { source: Source::Midi, ..Config::default() }, "table top");
        let follower = Deck::start(&url, Config { source: Source::Relay, ..Config::default() }, "follower");
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
        Rig { _rt: rt, leader, follower, ticking, clock: Some(clock) }
    }

    /// the table top stops sending
    fn stop_ticks(&mut self) {
        self.ticking.store(false, Ordering::Relaxed);
        if let Some(c) = self.clock.take() {
            c.join().unwrap();
        }
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.stop_ticks();
    }
}

/// the states, not the milliseconds: the follower locks on the leader's anchors and says SHOW at the table top's tempo, the
/// leader says LEADS; the table top goes away and the last tempo holds on both, the leader's HUD says so
#[test]
fn a_deck_on_midi_clock_in_leads_the_show_through_the_relay() {
    let mut rig = Rig::start();
    {
        let (leader, follower) = (&rig.leader, &rig.follower);
        // the tempo estimate of a loaded machine wanders (ticks stamped on arrival): 3 BPM, not the 0.5 of a quiet one
        until(
            "the follower on the table top's tempo",
            || format!("{} {:.1} / {}", status(follower), bpm_of(follower), status(leader)),
            || status(follower).starts_with("SHOW") && (bpm_of(follower) - 126.0).abs() < 3.0 && status(leader).ends_with("LEADS"),
        );
    }
    rig.stop_ticks();
    let leader = &rig.leader;
    until("the leader to say the table top is gone", || status(leader), || status(leader).starts_with("NO CLOCK - HOLDS 12"));
    let (lbpm, fbpm) = (bpm_of(&rig.leader), bpm_of(&rig.follower));
    assert!((lbpm - 126.0).abs() < 3.0 && (fbpm - 126.0).abs() < 3.0, "the last tempo holds on both: {lbpm} {fbpm}");
    assert!(status(&rig.leader).ends_with("LEADS"), "{}", status(&rig.leader));
}

/// The follower's beat against the leader's at the same moment over the real relay on the loopback: p95 within the P8 bound.
/// Real time on purpose - a measurement, not a state: it needs a quiet machine (a build running beside it fails it), so it
/// does not run in deck:ci; the hardware rig / the owner runs it with `--ignored`. The same bound on a virtual clock, with
/// jitter and drift, is deck-core's tests/show_clock.rs.
#[test]
#[ignore = "hardware rig / quiet machine: run with --ignored"]
fn the_follower_stays_within_5_ms_of_the_leader_on_the_loopback() {
    let mut rig = Rig::start();
    // the follower locks (its filter, the leader's first anchors), then the two are compared for four seconds
    std::thread::sleep(Duration::from_secs(4));
    let mut errors = Vec::new();
    for _ in 0..200 {
        let ns = now_ns();
        let ((l, bpm), (f, _)) = (rig.leader.beat(ns), rig.follower.beat(ns));
        errors.push(((f - l) / (bpm / 60.0) * 1e3).abs());
        std::thread::sleep(Duration::from_millis(20));
    }
    errors.sort_by(f64::total_cmp);
    let p95 = errors[errors.len() * 95 / 100];
    eprintln!("show clock over the loopback relay: p95 {p95:.3} ms, max {:.3} ms", errors[errors.len() - 1]);
    assert!(p95 <= 5.0, "the follower is {p95:.3} ms off the leader (p95)");
    // and after the table top goes away, still together
    rig.stop_ticks();
    std::thread::sleep(Duration::from_millis(1500));
    let ns = now_ns();
    let ((l, _), (f, _)) = (rig.leader.beat(ns), rig.follower.beat(ns));
    assert!(((f - l) / 2.1 * 1e3).abs() < 5.0, "still together: {:.3} ms", (f - l) / 2.1 * 1e3);
}
