//! The show clock end to end (#321, plan §3.7 / P8): a leader on MIDI clock in (the venue's table-top clock, owner
//! 2026-11-03) and a follower on `Source::Relay`, each with its own crystal (offset and drift against the relay's), the
//! relay's whole milliseconds, a network with queues – the follower's beat against the leader's at the same true moment.
//! The bound: ≤ 5 ms p95 on a LAN, ≤ 20 ms hosted (plan §3.7).
#![cfg(feature = "tempo")]

use deck_core::tempo::{Anchor, Config, Controller, Event, Line, Moment, NoHost, Shared, Source, What};
use std::sync::Arc;

const RATE: f64 = 48_000.0;

fn jitter(i: u64) -> f64 {
    let x = i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((x >> 33) as f64 / (1u64 << 31) as f64) - 1.0
}

/// 0..1
fn unit(i: u64) -> f64 {
    0.5 * (jitter(i) + 1.0)
}

/// A network path: `base` s each way, a queue of up to `queue` s (mostly short), every `burst`-th message stuck up to
/// 80 ms more (Wi-Fi power save).
#[derive(Clone, Copy)]
struct Net {
    base: f64,
    queue: f64,
    burst: u64,
}

impl Net {
    fn delay(&self, k: u64) -> f64 {
        self.base + self.queue * unit(k).powi(3) + if k % self.burst == 7 { 0.08 * unit(k ^ 0xabcd) } else { 0.0 }
    }
}

/// A deck: its clock is the true time × (1 + drift) + off; its audio heard from local `origin` on at `RATE`, `lead`
/// frames rendered ahead.
struct Deck {
    ctl: Controller,
    off: f64,
    drift: f64,
    origin: f64,
    net: Net,
    seed: u64,
}

impl Deck {
    fn new(source: Source, off: f64, drift: f64, net: Net, seed: u64) -> Deck {
        let mut ctl = Controller::new(&Config { bpm: 100.0, ..Config::default() }, Arc::new(Shared::default()), RATE);
        ctl.set_source(source, &mut NoHost);
        Deck { ctl, off, drift, origin: 0.0, net, seed }
    }

    fn local(&self, t: f64) -> f64 {
        t * (1.0 + self.drift) + self.off
    }

    fn ns(&self, t: f64) -> u64 {
        (self.local(t) * 1e9) as u64
    }

    fn line(&self) -> Line {
        Line { t: self.origin, f: 0.0, spf: 1.0 / RATE }
    }

    fn moment(&self, t: f64) -> Moment {
        let line = self.line();
        let heard = line.frames_at(self.local(t));
        Moment { now_ns: self.ns(t), rate: RATE, heard, rendered: heard + 1024.0, line }
    }

    /// the beat this deck plays at true time `t` (its published map at the heard frame)
    fn beat(&self, t: f64) -> f64 {
        let (map, _) = self.ctl.shared().map().unwrap();
        map.beat_at(self.line().frames_at(self.local(t)))
    }
}

/// The scenario: the leader follows a MIDI clock at `bpm` (± 1 ms per tick, the USB / ALSA path), which changes to
/// `bpm2` half way; both decks ping the relay on the core's schedule; the leader's anchors reach the follower through
/// the relay. Returns the follower's error against the leader (ms) at every frame of the last `measure` s.
fn run(net: Net, bpm: f64, bpm2: f64, secs: f64, measure: f64) -> Vec<f64> {
    let mut leader = Deck::new(Source::Midi, 1234.0, 30e-6, net, 1);
    let mut follower = Deck::new(Source::Relay, 50.3, -45e-6, net, 2);
    follower.origin = follower.local(0.0) + 0.0123;
    leader.origin = leader.local(0.0) - 0.0456;
    // messages on their way: (true arrival, to the follower?, what)
    let mut wire: Vec<(f64, bool, What, u64)> = Vec::new();
    let relay_ms = |t: f64| (t * 1e3).floor() as i64;
    let dt = 1.0 / 60.0;
    let (mut tick_t, mut tick) = (1.0, 0u64);
    let mut k = 0u64;
    let mut out = Vec::new();
    leader.ctl.push(Event { at_ns: leader.ns(0.99), what: What::Midi([0xFA, 0, 0]) });
    let mut t = 0.0;
    while t < secs {
        t += dt;
        // the table-top clock's ticks up to now, stamped on the leader's clock as they arrive
        loop {
            let spt = 60.0 / (if tick_t < secs / 2.0 { bpm } else { bpm2 }) / 24.0;
            if tick_t > t {
                break;
            }
            let at = leader.ns(tick_t + 0.001 * jitter(tick + 99));
            leader.ctl.push(Event { at_ns: at, what: What::Midi([0xF8, 0, 0]) });
            tick += 1;
            tick_t += spt;
        }
        // what reached a deck by now
        wire.sort_by(|a, b| a.0.total_cmp(&b.0));
        while wire.first().is_some_and(|w| w.0 <= t) {
            let (arrive, to_follower, what, _) = wire.remove(0);
            let d = if to_follower { &mut follower } else { &mut leader };
            d.ctl.push(Event { at_ns: d.ns(arrive), what });
        }
        for (i, d) in [&mut leader, &mut follower].into_iter().enumerate() {
            if d.ctl.ping_due(d.ns(t)) {
                k += 1;
                let up = d.net.delay(d.seed * 1000 + 2 * k);
                let down = d.net.delay(d.seed * 1000 + 2 * k + 1);
                let what = What::Pong { t0_ns: d.ns(t), t1_ms: relay_ms(t + up) };
                wire.push((t + up + down, i == 1, what, k));
            }
            d.ctl.frame(&d.moment(t), &mut NoHost);
        }
        if let Some(a) = leader.ctl.take_anchor() {
            // leader → relay → follower
            k += 1;
            let arrive = t + net.delay(5000 + 2 * k) + net.delay(5001 + 2 * k);
            wire.push((arrive, true, What::Tempo(a), k));
        }
        if t > secs - measure {
            let bps = follower.ctl.now().bpm / 60.0;
            out.push((follower.beat(t) - leader.beat(t)) / bps * 1e3);
        }
    }
    out
}

fn p95(v: &[f64]) -> f64 {
    let mut a: Vec<f64> = v.iter().map(|x| x.abs()).collect();
    a.sort_by(f64::total_cmp);
    a[(a.len() as f64 * 0.95) as usize]
}

#[test]
fn a_follower_on_the_lan_stays_within_5_ms_of_a_midi_leader() {
    let lan = Net { base: 0.0008, queue: 0.004, burst: 50 };
    let e = run(lan, 128.0, 128.0, 60.0, 30.0);
    let p = p95(&e);
    eprintln!("LAN, 128 BPM: p95 {p:.3} ms, max {:.3} ms ({} points)", e.iter().fold(0.0f64, |m, x| m.max(x.abs())), e.len());
    assert!(p <= 5.0, "p95 {p:.3} ms");
}

#[test]
fn a_tempo_change_on_the_table_top_reaches_the_follower() {
    let lan = Net { base: 0.0008, queue: 0.004, burst: 50 };
    // 120 → 132 BPM at 40 s; measured from 50 s on (a MIDI clock needs a beat or three to show a new tempo)
    let e = run(lan, 120.0, 132.0, 80.0, 30.0);
    let p = p95(&e);
    eprintln!("LAN, 120 → 132 BPM: p95 {p:.3} ms");
    assert!(p <= 5.0, "p95 {p:.3} ms");
}

#[test]
fn a_follower_on_a_hosted_relay_stays_within_20_ms() {
    let hosted = Net { base: 0.02, queue: 0.04, burst: 10 };
    let e = run(hosted, 128.0, 128.0, 90.0, 40.0);
    let p = p95(&e);
    eprintln!("hosted: p95 {p:.3} ms");
    assert!(p <= 20.0, "p95 {p:.3} ms");
}

#[test]
fn the_same_anchor_gives_the_same_beats_on_any_deck() {
    // two followers with different crystals and audio origins, exact filters: one anchor, the same beat at every moment
    let a = Anchor { bpm: 97.5, per_bar: 7, beat: 33.25, at: 5_000_000, running: true };
    let mut beats = Vec::new();
    for (off, drift, origin) in [(12.0, 0.0, 3.0), (777.0, 0.0, -41.5)] {
        let mut d = Deck::new(Source::Relay, off, drift, Net { base: 0.0, queue: 0.0, burst: u64::MAX }, 3);
        d.origin = origin;
        for i in 0..5 {
            let t = 4990.0 + i as f64;
            d.ctl.push_pong(d.ns(t), (t * 1e3) as i64, d.ns(t));
        }
        d.ctl.push_anchor(a, d.ns(4999.0));
        d.ctl.frame(&d.moment(5000.0), &mut NoHost);
        beats.push([5000.0, 5001.5, 5010.25].map(|t| d.beat(t)));
    }
    for (x, y) in beats[0].iter().zip(&beats[1]) {
        assert!((x - y).abs() < 1e-6, "{x} vs {y}");
    }
    assert!((beats[0][1] - (33.25 + 1.5 * 97.5 / 60.0)).abs() < 1e-6);
}
