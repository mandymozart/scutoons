//! The time types the tempo clock runs on (moved from deck/native's clock.rs, #308): the seqlock that hands words from
//! one writer to readers without a lock, the time base's line (frame ↔ seconds) and one video frame's `Moment`.
//! Platform-free: nothing here reads a clock – the caller passes `now_ns`.
use core::sync::atomic::{fence, AtomicU64, Ordering};

/// N words handed from ONE writer to any readers without a lock and without allocating (a seqlock): the writer never
/// waits, a reader retries while a write is in flight. The realtime side may be either end.
pub struct SeqLock<const N: usize> {
    seq: AtomicU64,
    words: [AtomicU64; N],
}

impl<const N: usize> Default for SeqLock<N> {
    fn default() -> Self {
        SeqLock { seq: AtomicU64::new(0), words: core::array::from_fn(|_| AtomicU64::new(0)) }
    }
}

impl<const N: usize> SeqLock<N> {
    /// one writer only (each SeqLock has a single owner thread that writes)
    pub fn write(&self, w: [u64; N]) {
        let s = self.seq.load(Ordering::Relaxed);
        self.seq.store(s.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release);
        for (a, v) in self.words.iter().zip(w) {
            a.store(v, Ordering::Relaxed);
        }
        self.seq.store(s.wrapping_add(2), Ordering::Release);
    }

    /// (the version, the words) – None before the first write (or when a writer keeps it busy, which it never does)
    pub fn read(&self) -> Option<(u64, [u64; N])> {
        for _ in 0..64 {
            let s1 = self.seq.load(Ordering::Acquire);
            if s1 & 1 == 1 {
                core::hint::spin_loop();
                continue;
            }
            let w = core::array::from_fn(|i| self.words[i].load(Ordering::Relaxed));
            fence(Ordering::Acquire);
            if self.seq.load(Ordering::Relaxed) == s1 {
                return (s1 != 0).then_some((s1, w));
            }
        }
        None
    }

    pub fn write_f64(&self, w: [f64; N]) {
        self.write(w.map(f64::to_bits));
    }

    pub fn read_f64(&self) -> Option<(u64, [f64; N])> {
        self.read().map(|(v, w)| (v, w.map(f64::from_bits)))
    }
}


/// The DLL's line: frame `f` is heard at `t` (s), one frame lasts `spf` (s)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub t: f64,
    pub f: f64,
    pub spf: f64,
}

impl Line {
    /// the frame position heard at `t` (s)
    pub fn frames_at(&self, t: f64) -> f64 {
        self.f + (t - self.t) / self.spf
    }
    /// when frame `f` is heard (s)
    pub fn time_of(&self, f: f64) -> f64 {
        self.t + (f - self.f) * self.spf
    }
}

/// One video frame's view of the time base: what is heard now, what the audio thread has rendered, and the line that
/// maps any moment (an event's timestamp) to a frame position. Without audio a virtual 48 kHz line from the start.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moment {
    pub now_ns: u64,
    pub rate: f64,
    /// the frame position heard now
    pub heard: f64,
    /// frames the audio thread has rendered (≥ heard by the output latency + a block)
    pub rendered: f64,
    pub line: Line,
}

impl Moment {
    /// no audio: frames on the wall clock since `origin_ns` at `rate` (heard = rendered)
    pub fn wall(now_ns: u64, origin_ns: u64, rate: f64) -> Moment {
        let line = Line { t: origin_ns as f64 * 1e-9, f: 0.0, spf: 1.0 / rate };
        let heard = line.frames_at(now_ns as f64 * 1e-9).max(0.0);
        Moment { now_ns, rate, heard, rendered: heard, line }
    }

    /// the frame position heard at `ns` (an event's moment)
    pub fn frames_at(&self, ns: u64) -> f64 {
        self.line.frames_at(ns as f64 * 1e-9)
    }

    pub fn seconds(&self) -> f64 {
        self.heard / self.rate
    }

    /// samples rendered past the heard moment
    pub fn lead(&self) -> usize {
        (self.rendered - self.heard).max(0.0) as usize
    }
}

/// the loop bandwidth (Hz): low enough to smooth a bursty driver, high enough to settle within a second or two
const BANDWIDTH: f64 = 1.0;
/// an error past this re-anchors (a stalled device, a restart) instead of being smoothed
const JUMP_S: f64 = 0.05;
/// past the last block the position runs on for this long, then holds (the device stopped – the picture waits)
const STALL_S: f64 = 0.25;

/// The time base (CYBERDECKS.md §8 "Two clocks", #181; moved here from deck/native's clock.rs with #321 – the browser
/// deck needs the same): block stamps in – (frames rendered before a block, the moment its first frame is heard) –
/// through a delay-locked loop (Fons Adriaensen's DLL, JACK's and PipeWire's own), the heard position at any moment out.
/// The loop interpolates between blocks and smooths the stamps' jitter; the answer never goes backwards. The native
/// deck stamps every audio callback; the browser deck stamps what `getOutputTimestamp` says now and then.
pub struct TimeBase {
    rate: f64,
    line: Option<Line>,
    version: u64,
    out: f64,
}

impl TimeBase {
    pub fn new(rate: f32) -> TimeBase {
        TimeBase { rate: rate as f64, line: None, version: 0, out: 0.0 }
    }

    pub fn line(&self) -> Option<Line> {
        self.line
    }

    /// read the newest stamp (a new block since the last frame → into the loop)
    pub fn poll(&mut self, stamp: &SeqLock<2>) {
        if let Some((v, [frames, heard])) = stamp.read() {
            if v != self.version {
                self.version = v;
                self.block(frames, heard);
            }
        }
    }

    /// one block: `frames` rendered before it, its first frame heard at `heard_ns`
    pub fn block(&mut self, frames: u64, heard_ns: u64) {
        let (t, f) = (heard_ns as f64 * 1e-9, frames as f64);
        let nominal = 1.0 / self.rate;
        let Some(m) = self.line.as_mut() else {
            self.line = Some(Line { t, f, spf: nominal });
            return;
        };
        let df = f - m.f;
        if df <= 0.0 {
            if df < 0.0 {
                self.line = Some(Line { t, f, spf: nominal });
            }
            return;
        }
        let predicted = m.t + df * m.spf;
        let e = t - predicted;
        if e.abs() > JUMP_S {
            *m = Line { t, f, spf: m.spf };
            return;
        }
        // the second-order loop, per block of any length: w = 2π B Δt, b = √2 w, c = w²
        let w = (core::f64::consts::TAU * BANDWIDTH * df * m.spf).min(0.5);
        let (b, c) = (core::f64::consts::SQRT_2 * w, w * w);
        m.t = predicted + b * e;
        m.f = f;
        m.spf = (m.spf + c * e / df).clamp(nominal * 0.98, nominal * 1.02);
    }

    /// the heard position (s of audio) at `now_ns` – None before the first block; never less than the last answer
    pub fn seconds(&mut self, now_ns: u64) -> Option<f64> {
        let m = self.line?;
        let t = (now_ns as f64 * 1e-9).min(m.t + STALL_S);
        let s = (m.frames_at(t) / self.rate).max(self.out);
        self.out = s;
        Some(s)
    }

    /// this frame's Moment (None before the first block): `rendered` = the engine's frame counter
    pub fn moment(&mut self, now_ns: u64, rendered: u64) -> Option<Moment> {
        let s = self.seconds(now_ns)?;
        let heard = s * self.rate;
        Some(Moment { now_ns, rate: self.rate, heard, rendered: (rendered as f64).max(heard), line: self.line? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    /// a small deterministic noise in −1..1 (no rand dependency)
    fn jitter(i: u64) -> f64 {
        let x = i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((x >> 33) as f64 / (1u64 << 31) as f64) - 1.0
    }

    /// drive the time base: blocks of `period` frames, stamped by `stamp(k)` (seconds), video frames every 1/60 s
    /// (with 0.3 ms of jitter); returns (the true heard seconds, the answer) per video frame
    fn run(period: u64, seconds: f64, stamp: impl Fn(u64) -> f64) -> Vec<(f64, f64)> {
        let mut tb = TimeBase::new(RATE as f32);
        let mut out = Vec::new();
        let mut k = 0;
        let frames = (seconds * 60.0) as u64;
        for i in 1..frames {
            let now = i as f64 / 60.0 + 0.0003 * jitter(i + 7);
            // the blocks whose callback has run by now (rendered ahead of the heard moment by the latency)
            while stamp(k) - 0.02 <= now {
                tb.block(k * period, (stamp(k) * 1e9) as u64);
                k += 1;
            }
            out.push((now, tb.seconds((now * 1e9) as u64).unwrap_or(0.0)));
        }
        out
    }

    fn check_smooth(v: &[(f64, f64)], what: &str, tolerance: f64) {
        let settled = &v[v.len() / 2..];
        for w in settled.windows(2) {
            let (step, real) = (w[1].1 - w[0].1, w[1].0 - w[0].0);
            assert!((step - real).abs() < 0.0015, "{what}: a frame moved {:.2} ms for {:.2} ms", step * 1e3, real * 1e3);
        }
        for &(t, s) in settled {
            // the first block is heard at 0.03 s
            assert!((s - (t - 0.03)).abs() < tolerance, "{what}: {s:.4} vs the true {:.4}", t - 0.03);
        }
    }

    #[test]
    fn exact_stamps_interpolate_between_blocks() {
        let mut tb = TimeBase::new(48_000.0);
        tb.block(0, 1_000_000_000);
        tb.block(1024, 1_000_000_000 + 21_333_333);
        let s = tb.seconds(1_010_000_000).unwrap();
        assert!((s - 0.010).abs() < 1e-5, "{s}");
        let s = tb.seconds(1_030_000_000).unwrap(); // past the last block: runs on
        assert!((s - 0.030).abs() < 1e-5, "{s}");
    }

    #[test]
    fn it_never_goes_backwards_and_holds_on_a_stall() {
        let mut tb = TimeBase::new(48_000.0);
        assert!(tb.seconds(5).is_none());
        tb.block(48_000, 2_000_000_000);
        let a = tb.seconds(2_100_000_000).unwrap();
        let b = tb.seconds(2_050_000_000).unwrap(); // an earlier moment: the same answer, never less
        assert_eq!(a, b);
        let held = tb.seconds(9_000_000_000).unwrap(); // the device stopped: STALL_S on, then it holds
        assert!((held - (1.0 + STALL_S)).abs() < 1e-6, "{held}");
    }

    #[test]
    fn a_jump_re_anchors() {
        let mut tb = TimeBase::new(48_000.0);
        tb.block(0, 1_000_000_000);
        tb.block(48_000, 3_000_000_000); // a second of audio, two seconds later (a stalled device)
        assert!((tb.seconds(3_000_000_000).unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn whole_periods_per_frame_or_none_give_smooth_time() {
        // the X1: 1024-frame periods (21.3 ms) against 16.7 ms video frames, the callback 1.5 ms late or early
        let p = 1024.0 / RATE;
        check_smooth(&run(1024, 6.0, |k| 0.03 + k as f64 * p + 0.0015 * jitter(k)), "1024 jittery", 0.002);
    }

    #[test]
    fn a_bursty_driver_gives_smooth_time() {
        // 4 quanta of 128 frames back to back (0.1 ms apart), then nothing for the rest of 10.7 ms: smooth, and off by
        // at most half a burst (the stamps themselves say four blocks are heard at once)
        let p = 512.0 / RATE;
        check_smooth(&run(128, 6.0, |k| 0.03 + (k / 4) as f64 * p + (k % 4) as f64 * 0.0001), "bursts", 256.0 / RATE);
    }

    #[test]
    fn a_device_running_fast_is_followed() {
        // the device's crystal: 48 048 Hz against the nominal 48 000 – the loop learns the rate
        let p = 512.0 / 48_048.0;
        let v = run(512, 12.0, |k| 0.03 + k as f64 * p);
        for &(t, s) in &v[v.len() / 2..] {
            // the heard position in nominal seconds runs 0.1 % fast
            assert!((s - (t - 0.03) * 48_048.0 / RATE).abs() < 0.002, "{s} vs {t}");
        }
    }

    #[test]
    fn sparse_stamps_of_a_browser_lock_too() {
        // the browser deck: `getOutputTimestamp` every 100 ms, each 1 ms off (the main thread's timer)
        let v = run(4800, 12.0, |k| 0.03 + k as f64 * 0.1 + 0.001 * jitter(k + 3));
        for &(t, s) in &v[v.len() / 2..] {
            assert!((s - (t - 0.03)).abs() < 0.0015, "{s} vs {t}");
        }
    }

    #[test]
    fn the_seqlock_hands_over_whole_words() {
        let s = SeqLock::<2>::default();
        assert!(s.read().is_none());
        s.write([7, 9]);
        let (v1, w) = s.read().unwrap();
        assert_eq!(w, [7, 9]);
        s.write([8, 10]);
        let (v2, w) = s.read().unwrap();
        assert!(v2 > v1);
        assert_eq!(w, [8, 10]);
        let l = SeqLock::<3>::default();
        l.write_f64([1.5, -2.0, 0.25]);
        assert_eq!(l.read_f64().unwrap().1, [1.5, -2.0, 0.25]);
    }
}
