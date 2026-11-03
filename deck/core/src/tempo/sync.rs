//! The show clock over the relay (plan §3.7, #321): the relay's time (ms) seen from a deck, and the anchor that carries
//! the show's tempo line in it. Platform-free – the adapters only send the pings, hand the pongs' and the `tempo`
//! messages' numbers in, and send what `Lead` hands out.
//!
//! - **The offset filter** (`Filter`): every ping / pong is a sample – sent at `t0`, the relay's `t1`, back at `t2`
//!   (local) – of the relay's clock minus the deck's, good to half its round trip. A queue on the way (Wi-Fi power
//!   save, a busy router) only ever makes a round trip longer, so the samples with the shortest round trips of the
//!   window are the true ones: a line through them by least squares (deck-core's `fit`) gives the offset and, once the
//!   window spans `DRIFT_SPAN` s, the drift of the two crystals.
//! - **The anchor** (`Anchor`): at relay ms `at` the show was at `beat`, running at `bpm` (or stopped) – the leader's
//!   tempo map as a line in relay time, sent only when it changed (`Lead`). A follower maps it to its own seconds
//!   through its filter, the time base maps those to frames (`WallLine::to_map`), and the controller follows the line
//!   with a gentle tempo nudge, never a jump (controller.rs).
//! - **The ping schedule** (`Pinger`): quick while the filter has few samples (after a join), then every `SLOW_S`.
use super::fit;
use super::midi_in::WallLine;
use super::time::Line;
use super::{clamp_bpm, TempoMap};

/// samples kept (the newest); at `SLOW_S` that is a little over two minutes
const KEEP: usize = 64;
/// a sample this much longer than the window's shortest round trip still counts (and at least `NEAR_S`)
const NEAR: f64 = 0.5;
const NEAR_S: f64 = 0.0005;
/// the fewest samples the estimate rests on (the shortest round trips, when fewer are near the shortest)
const LEAST: usize = 3;
/// the window must span this long before its slope (the crystals' drift) is believed
const DRIFT_SPAN: f64 = 30.0;
/// the drift is at most this (crystals: tens of ppm; anything more is a fit through noise)
const MAX_DRIFT: f64 = 2e-4;
/// the filter says nothing below this many samples
pub const LOCK: usize = 3;

/// One ping's round trip: when it left and came back (local s), the relay's time in between (s).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Sample {
    /// the middle of the round trip (local s)
    t: f64,
    /// the relay's clock minus the deck's (s)
    off: f64,
    rtt: f64,
}

/// The relay's clock seen from the deck: offset + drift from the pings' samples with the shortest round trips.
#[derive(Debug, Clone)]
pub struct Filter {
    samples: [Sample; KEEP],
    len: usize,
    /// the next slot to write
    head: usize,
    /// the estimate: offset(t) = a + b (t − t_ref)
    a: f64,
    b: f64,
    t_ref: f64,
    /// the shortest round trip in the window (s)
    rtt: f64,
}

impl Default for Filter {
    fn default() -> Self {
        Filter { samples: [Sample::default(); KEEP], len: 0, head: 0, a: 0.0, b: 0.0, t_ref: 0.0, rtt: 0.0 }
    }
}

impl Filter {
    /// forget every sample (another relay: its clock is another)
    pub fn reset(&mut self) {
        *self = Filter::default();
    }

    /// samples in the window
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// enough samples to map times
    pub fn locked(&self) -> bool {
        self.len >= LOCK
    }

    /// One ping's round trip: sent at `t0`, back at `t2` (local s), the relay's clock read `t1` (s) in between. A pong
    /// that came back before it left (a clock step) is dropped.
    pub fn pong(&mut self, t0: f64, t1: f64, t2: f64) {
        let rtt = t2 - t0;
        if !((0.0..10.0).contains(&rtt) && t1.is_finite()) {
            return;
        }
        let mid = 0.5 * (t0 + t2);
        self.samples[self.head] = Sample { t: mid, off: t1 - mid, rtt };
        self.head = (self.head + 1) % KEEP;
        self.len = (self.len + 1).min(KEEP);
        self.estimate();
    }

    /// the window's samples, oldest first
    fn window(&self) -> impl Iterator<Item = &Sample> + Clone {
        let start = (self.head + KEEP - self.len) % KEEP;
        (0..self.len).map(move |i| &self.samples[(start + i) % KEEP])
    }

    fn estimate(&mut self) {
        let min = self.window().fold(f64::INFINITY, |m, s| m.min(s.rtt));
        // the near ones; too few → the LEAST shortest (their round trips, sorted, without allocating)
        let mut cut = min + (NEAR * min).max(NEAR_S);
        if self.window().filter(|s| s.rtt <= cut).count() < LEAST.min(self.len) {
            let mut rtts = [0.0f64; KEEP];
            for (i, s) in self.window().enumerate() {
                rtts[i] = s.rtt;
            }
            let r = &mut rtts[..self.len];
            r.sort_unstable_by(f64::total_cmp);
            cut = r[LEAST.min(self.len) - 1];
        }
        let (mean, b, t_ref) = {
            let near = self.window().filter(move |s| s.rtt <= cut);
            let (n, st, so) = near.clone().fold((0.0, 0.0, 0.0), |(n, st, so), s| (n + 1.0, st + s.t, so + s.off));
            let (t_ref, mean) = (st / n, so / n);
            let (first, last) = near.clone().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), s| (a.min(s.t), b.max(s.t)));
            let b = if last - first >= DRIFT_SPAN {
                fit(near.map(|s| (s.t - t_ref, s.off - mean))).map_or(0.0, |(_, b)| b.clamp(-MAX_DRIFT, MAX_DRIFT))
            } else {
                0.0
            };
            (mean, b, t_ref)
        };
        (self.a, self.b, self.t_ref, self.rtt) = (mean, b, t_ref, min);
    }

    /// the relay's clock minus the deck's at local `t` (s) – None until locked
    pub fn offset(&self, t: f64) -> Option<f64> {
        self.locked().then_some(self.a + self.b * (t - self.t_ref))
    }

    /// the drift: relay seconds per local second − 1
    pub fn drift(&self) -> f64 {
        self.b
    }

    /// the shortest round trip in the window (s)
    pub fn rtt(&self) -> f64 {
        self.rtt
    }

    /// local s → the relay's s
    pub fn to_relay(&self, t: f64) -> Option<f64> {
        Some(t + self.offset(t)?)
    }

    /// the relay's s → local s (the line inverted exactly)
    pub fn to_local(&self, r: f64) -> Option<f64> {
        self.offset(0.0)?;
        Some((r - self.a + self.b * self.t_ref) / (1.0 + self.b))
    }
}

/// The show clock's anchor (the `tempo` message): at relay ms `at` the show was at `beat`, running at `bpm` (or
/// stopped – the beat stands), `per_bar` beats a bar. Relay time on the wire; a deck maps it with its filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub bpm: f64,
    pub per_bar: u32,
    pub beat: f64,
    /// relay ms
    pub at: i64,
    pub running: bool,
}

impl Anchor {
    /// the anchor as a line in this deck's seconds (None until the filter is locked)
    pub fn local(&self, f: &Filter) -> Option<WallLine> {
        let t = f.to_local(self.at as f64 * 1e-3)?;
        // a relay second lasts 1 / (1 + drift) local seconds: the beats per local second
        let bps = clamp_bpm(self.bpm) / 60.0 * (1.0 + f.drift());
        Some(WallLine { t, beat: self.beat, bps, running: self.running })
    }

    /// The beat at relay ms `at` on this anchor's line.
    pub fn beat_at(&self, at: f64) -> f64 {
        if self.running {
            self.beat + (at - self.at as f64) * 1e-3 * clamp_bpm(self.bpm) / 60.0
        } else {
            self.beat
        }
    }

    /// A deck's tempo map as an anchor at relay ms `at`: the map (frames) → its heard seconds (`line`) → relay time.
    pub fn of_map(map: &TempoMap, line: &Line, f: &Filter, at: i64) -> Option<Anchor> {
        let local = f.to_local(at as f64 * 1e-3)?;
        let beat = map.beat_at(line.frames_at(local));
        // beats per local second, then per relay second
        let bps = map.bpm / 60.0 / map.rate / line.spf / (1.0 + f.drift());
        Some(Anchor { bpm: bps * 60.0, per_bar: map.per_bar, beat, at, running: map.running })
    }
}

/// a leader sends an anchor that drifted this far from the last one it sent (s at the anchor's moment)
const RESEND_S: f64 = 0.0005;
/// … this far: at once; less: at most every `CALM_S`
const URGENT_S: f64 = 0.005;
const CALM_S: f64 = 0.25;

/// The leader's side: when its map (whatever its source – the internal clock, MIDI clock in, Link) has moved off the
/// anchor it sent last, a new one goes out.
#[derive(Debug, Clone, Default)]
pub struct Lead {
    sent: Option<Anchor>,
    /// when it went out (local s)
    when: f64,
}

impl Lead {
    /// send the next anchor whatever it says (after a join: the relay may have lost the last)
    pub fn again(&mut self) {
        self.sent = None;
    }

    /// The anchor to send now (at local `now` s), if the map has moved off the last one: tempo, running or the bar at
    /// once, a drift of the line past `URGENT_S` at once, past `RESEND_S` at most every `CALM_S`.
    pub fn due(&mut self, map: &TempoMap, line: &Line, f: &Filter, now: f64) -> Option<Anchor> {
        let at = (f.to_relay(now)? * 1e3).round() as i64;
        let a = Anchor::of_map(map, line, f, at)?;
        let send = match self.sent {
            None => true,
            Some(s) if s.running != a.running || s.per_bar != a.per_bar => true,
            Some(s) => {
                let bps = clamp_bpm(a.bpm) / 60.0;
                let off = (s.beat_at(at as f64) - a.beat).abs() / bps;
                let tempo = (s.bpm - a.bpm).abs() > 1e-3;
                off > URGENT_S || ((off > RESEND_S || tempo) && now - self.when >= CALM_S)
            }
        };
        send.then(|| {
            self.sent = Some(a);
            self.when = now;
            a
        })
    }
}

/// a ping every this often once the filter has its samples …
pub const SLOW_S: f64 = 2.0;
/// … and this often until it has `QUICK` (after a join: locked in a second or two)
pub const FAST_S: f64 = 0.25;
const QUICK: usize = 12;

/// When to ping: the adapter asks, the answer counts as sent.
#[derive(Debug, Clone, Default)]
pub struct Pinger {
    last: Option<f64>,
}

impl Pinger {
    /// a ping is due at local `now` s for a filter of `samples` samples
    pub fn due(&mut self, now: f64, samples: usize) -> bool {
        let every = if samples < QUICK { FAST_S } else { SLOW_S };
        let due = self.last.map_or(true, |l| now - l >= every || now < l);
        if due {
            self.last = Some(now);
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jitter(i: u64) -> f64 {
        let x = i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((x >> 33) as f64 / (1u64 << 31) as f64) - 1.0
    }

    /// 0..1, a second stream
    fn unit(i: u64) -> f64 {
        0.5 * (jitter(i ^ 0x9e37_79b9) + 1.0)
    }

    /// A relay whose clock is the deck's + `off` + `drift` × t, pinged every `every` s for `secs` s; each way takes
    /// `base` s plus a queue of up to `queue` s (mostly short: a cubed uniform), one ping in `burst` sits in a long one
    /// (up to 60 ms more), and the relay's ms are whole (`floor`). Returns the filter.
    fn run(off: f64, drift: f64, base: f64, queue: f64, burst: u64, every: f64, secs: f64) -> Filter {
        let mut f = Filter::default();
        let n = (secs / every) as u64;
        for k in 0..n {
            let t0 = 100.0 + k as f64 * every;
            let up = base + queue * unit(2 * k).powi(3) + if k % burst == 3 { 0.06 * unit(7 * k) } else { 0.0 };
            let down = base + queue * unit(2 * k + 1).powi(3);
            let relay = |t: f64| t + off + drift * t;
            let t1 = (relay(t0 + up) * 1e3).floor() * 1e-3;
            f.pong(t0, t1, t0 + up + down);
        }
        f
    }

    /// the error of the filter's relay time at local `t` (s), less the floor's half millisecond every deck shares
    fn error(f: &Filter, off: f64, drift: f64, t: f64) -> f64 {
        f.to_relay(t).unwrap() - (t + off + drift * t) + 0.0005
    }

    #[test]
    fn a_clean_lan_locks_in_a_second() {
        let f = run(1234.5678, 0.0, 0.0008, 0.0002, 1000, FAST_S, 1.0);
        assert!(f.locked());
        let e = error(&f, 1234.5678, 0.0, 101.0);
        assert!(e.abs() < 0.0006, "{:.3} ms", e * 1e3);
        assert!((f.rtt() - 0.0016).abs() < 0.0005, "{}", f.rtt());
    }

    #[test]
    fn wifi_queues_are_filtered_out() {
        // a phone on Wi-Fi: 2 ms each way, up to 20 ms of queue either way, one ping in five stuck 60 ms more – the
        // average offset would be off by several ms, the shortest round trips are not
        let f = run(-50.0, 0.0, 0.002, 0.02, 5, SLOW_S, 120.0);
        let e = error(&f, -50.0, 0.0, 220.0);
        assert!(e.abs() < 0.002, "{:.3} ms", e * 1e3);
    }

    #[test]
    fn the_drift_of_two_crystals_is_learned() {
        // 40 ppm: after two minutes the offset has moved 4.8 ms – the line follows it and extrapolates two seconds on
        let f = run(10.0, 40e-6, 0.001, 0.003, 1000, SLOW_S, 130.0);
        assert!((f.drift() - 40e-6).abs() < 10e-6, "{}", f.drift());
        let e = error(&f, 10.0, 40e-6, 232.0);
        assert!(e.abs() < 0.001, "{:.3} ms", e * 1e3);
    }

    #[test]
    fn local_and_relay_time_invert_each_other() {
        let f = run(-3.25, -25e-6, 0.001, 0.001, 1000, SLOW_S, 80.0);
        for t in [150.0, 1234.5, 99_999.0] {
            let back = f.to_local(f.to_relay(t).unwrap()).unwrap();
            assert!((back - t).abs() < 1e-9, "{t}: {back}");
        }
        assert!(Filter::default().to_relay(1.0).is_none(), "not locked");
        let mut f = f;
        f.pong(10.0, 5.0, 9.0); // back before it left: dropped
        assert_eq!(f.len(), 40);
    }

    const RATE: f64 = 48_000.0;

    #[test]
    fn an_anchor_round_trips_between_two_decks() {
        // the leader's clock is the relay's − 5 s, the follower's the relay's + 2 s; both filters exact
        let exact = |off: f64| {
            let mut f = Filter::default();
            for k in 0..5 {
                let t = 10.0 + k as f64;
                f.pong(t, t + off, t);
            }
            f
        };
        let (lf, ff) = (exact(5.0), exact(-2.0));
        // the leader's map: 96 BPM in 3, beat 7 at frame 48 000; its frames are heard from local 10 s on, 48 kHz
        let line = Line { t: 10.0, f: 0.0, spf: 1.0 / RATE };
        let map = TempoMap { frame: 48_000.0, beat: 7.0, bpm: 96.0, rate: RATE, running: true, per_bar: 3 };
        let a = Anchor::of_map(&map, &line, &lf, 20_000).unwrap();
        // relay 20 s = leader local 15 s = frame 240 000 = beat 7 + 4 s × 1.6
        assert!((a.beat - 13.4).abs() < 1e-9 && (a.bpm - 96.0).abs() < 1e-9 && a.per_bar == 3, "{a:?}");
        // the follower: relay 20 s = its local 22 s; its frames are heard from local 0 on
        let w = a.local(&ff).unwrap();
        assert!((w.t - 22.0).abs() < 1e-9);
        let fmap = w.to_map(&Line { t: 0.0, f: 0.0, spf: 1.0 / RATE }, RATE, 3);
        // relay 21 s: the leader at frame 288 000 (local 16 s), the follower at frame 1 104 000 (local 23 s)
        assert!((map.beat_at(288_000.0) - fmap.beat_at(1_104_000.0)).abs() < 1e-9);
    }

    #[test]
    fn the_leader_sends_only_on_change() {
        let mut f = Filter::default();
        for k in 0..5 {
            f.pong(k as f64, k as f64 + 100.0, k as f64);
        }
        let line = Line { t: 0.0, f: 0.0, spf: 1.0 / RATE };
        let map = TempoMap::new(120.0, 4, RATE);
        let mut l = Lead::default();
        assert!(l.due(&map, &line, &f, 10.0).is_some(), "the first goes out");
        assert!(l.due(&map, &line, &f, 10.5).is_none(), "the same line: nothing");
        // a tempo change at frame 11 s: out, but at most every CALM_S for a small one
        let m2 = map.with_bpm(11.0 * RATE, 121.0);
        assert!(l.due(&m2, &line, &f, 11.1).is_some());
        let m3 = m2.with_bpm(11.2 * RATE, 121.01);
        assert!(l.due(&m3, &line, &f, 11.2).is_none(), "within CALM_S");
        assert!(l.due(&m3, &line, &f, 11.4).is_some());
        // a stop: at once
        let m4 = m3.with_running(11.45 * RATE, false);
        assert!(!l.due(&m4, &line, &f, 11.45).unwrap().running);
        l.again();
        assert!(l.due(&m4, &line, &f, 11.5).is_some(), "after a join: again");
    }

    #[test]
    fn pings_are_quick_until_the_filter_has_its_samples() {
        let mut p = Pinger::default();
        assert!(p.due(1.0, 0));
        assert!(!p.due(1.1, 1));
        assert!(p.due(1.25, 1));
        assert!(!p.due(2.0, QUICK));
        assert!(p.due(3.25, QUICK));
    }
}
