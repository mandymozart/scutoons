//! Rendering decoupled from the device (#208, owner 2026-10-31: "the dropouts keep existing – research methods to
//! compensate"): a render thread runs the graph in blocks into a lock-free SPSC ring (rtrb, interleaved stereo) ahead of
//! the device, and the device callback only copies out of it – a spike in a render (a page fault, the GPU driver, a busy
//! CPU) is absorbed by the ring's lead instead of reaching the speaker. The lead adapts (lead.rs): an underrun grows it
//! a block up to a cap, a calm minute shrinks it a block. Only an underrun – a late or long callback is the device's
//! own thread stalling, which no lead can help, and at ASIO 64 (1.3 ms periods) such stalls pushed the lead to its cap
//! for nothing (#230). live-core's worker does the same in the browser.
//!
//! The time base stays exact: the callback publishes (frames it has taken from the ring, the moment its first frame is
//! heard) through a seqlock, and the render thread stamps each block it renders with that moment plus the frames between
//! – so `Engine::mark` still says when a block is heard, the ring's lead included. Neither side allocates or locks: the
//! callback copies, publishes two words and unparks the render thread (tests/no_alloc.rs).
use super::glitch::{Glitches, Monitor};
use super::lead::AdaptiveLead;
use super::Engine;
use crate::clock::{now_ns, SeqLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// frames per render-thread block (and one lead step)
pub const BLOCK: usize = 256;
// the render thread hands the engine whole quanta
const _: () = assert!(BLOCK % deck_engine::QUANTUM == 0);
/// the lead's range (frames): one block (5.3 ms at 48 kHz) up to 16 (85 ms)
pub const LEAD_MIN: u32 = BLOCK as u32;
pub const LEAD_CAP: u32 = 16 * BLOCK as u32;
/// the ring's size in frames (stereo samples × 2): the cap + the largest device buffer we expect, with room
pub const CAPACITY: usize = 16_384;
/// the device's buffer before its first callback says it (WASAPI shared runs ~1056 frames)
const FIRST_GUESS: u32 = 1024;

/// What the two halves share.
struct Shared {
    /// the callback's stamp: [frames taken from the ring before its callback, the moment its first frame is heard]
    stamp: SeqLock<2>,
    stop: AtomicBool,
    glitches: Arc<Glitches>,
}

/// The device callback's half: copies out of the ring. Dropping it stops the render thread.
pub struct Feed {
    ring: rtrb::Consumer<f32>,
    shared: Arc<Shared>,
    pub(crate) mon: Monitor,
    /// frames taken from the ring so far
    taken: u64,
    /// the ring had audio once (an empty ring before the first block is no underrun)
    primed: bool,
    render: Option<std::thread::Thread>,
}

/// The render thread's half: renders blocks into the ring until it holds the device's buffer + the lead.
pub struct Renderer {
    engine: Engine,
    ring: rtrb::Producer<f32>,
    shared: Arc<Shared>,
    lead: AdaptiveLead,
    buf: Vec<f32>,
    /// frames pushed into the ring so far
    pushed: u64,
    rate: f64,
    first_guess: u32,
}

/// The two halves around one ring, no thread yet (the tests drive both; `spawn` runs the renderer on its thread).
pub fn pair(mut engine: Engine, first_guess: Option<u32>) -> (Feed, Renderer) {
    let mon = engine.take_monitor();
    let rate = engine.rate() as f64;
    let shared = Arc::new(Shared { stamp: SeqLock::default(), stop: AtomicBool::new(false), glitches: mon.shared.clone() });
    let (tx, rx) = rtrb::RingBuffer::new(CAPACITY * 2);
    let feed = Feed { ring: rx, shared: shared.clone(), mon, taken: 0, primed: false, render: None };
    let renderer = Renderer {
        engine,
        ring: tx,
        shared,
        lead: AdaptiveLead::new(LEAD_MIN, LEAD_CAP, BLOCK as u32),
        buf: vec![0.0; BLOCK * 2],
        pushed: 0,
        rate,
        first_guess: first_guess.unwrap_or(FIRST_GUESS).max(FIRST_GUESS),
    };
    (feed, renderer)
}

/// The feed for a device callback, its renderer running on a thread of its own at audio priority.
pub fn spawn(engine: Engine, first_guess: Option<u32>) -> std::io::Result<Feed> {
    let (mut feed, mut renderer) = pair(engine, first_guess);
    let handle = std::thread::Builder::new().name("audio-render".into()).spawn(move || {
        match super::priority::raise() {
            Ok(how) => tracing::info!("audio: the render thread runs at {how}"),
            Err(e) => tracing::warn!("audio: the render thread at normal priority ({e})"),
        }
        while !renderer.shared.stop.load(Ordering::Relaxed) {
            renderer.fill(now_ns());
            // the callback unparks it after each copy; the timeout keeps it going if a wake-up is lost
            std::thread::park_timeout(Duration::from_millis(1));
        }
    })?;
    feed.render = Some(handle.thread().clone());
    Ok(feed)
}

impl Renderer {
    fn ns(&self, frames: u64) -> u64 {
        (frames as f64 * 1e9 / self.rate) as u64
    }

    /// frames waiting in the ring
    pub fn queued(&self) -> usize {
        (CAPACITY * 2 - self.ring.slots()) / 2
    }

    /// The lead's step for the underruns so far (at `now_ns`), then blocks into the ring until it holds the device's
    /// buffer + the lead (or is full). Returns the blocks rendered. Never allocates.
    pub fn fill(&mut self, now: u64) -> usize {
        let g = &self.shared.glitches;
        let lead = self.lead.update(now as f64 * 1e-9, g.underruns());
        g.set_lead(lead);
        let want = (g.device_frames().max(self.first_guess) + lead) as usize;
        let mut blocks = 0;
        while self.queued() < want && self.ring.slots() >= BLOCK * 2 {
            // heard: the callback's newest stamp + the frames between its first and this block's
            let heard = match self.shared.stamp.read() {
                Some((_, [taken, at])) => at + self.ns(self.pushed.saturating_sub(taken)),
                None => now_ns() + self.ns(self.queued() as u64),
            };
            self.engine.mark(heard);
            let t0 = now_ns();
            self.engine.render(&mut self.buf, 2, |x| x);
            g.note_load(now_ns() - t0, self.ns(BLOCK as u64));
            if let Ok(mut chunk) = self.ring.write_chunk(BLOCK * 2) {
                let (a, b) = chunk.as_mut_slices();
                a.copy_from_slice(&self.buf[..a.len()]);
                b.copy_from_slice(&self.buf[a.len()..]);
                chunk.commit_all();
            }
            self.pushed += BLOCK as u64;
            blocks += 1;
        }
        blocks
    }
}

impl Feed {
    pub fn glitches(&self) -> &Glitches {
        &self.mon.shared
    }

    /// One device callback: `out` (interleaved, `channels` per frame) whose first frame is heard at `heard_ns` – copied
    /// from the ring (L, R, the rest silent; mono = their mean), silence where it runs short (an underrun). Never
    /// allocates.
    pub fn play<T: Copy>(&mut self, out: &mut [T], channels: usize, heard_ns: u64, conv: impl Fn(f32) -> T) {
        let start = self.mon.begin();
        let channels = channels.max(1);
        let frames = out.len() / channels;
        self.shared.stamp.write([self.taken, heard_ns]);
        let n = frames.min(self.ring.slots() / 2);
        if n > 0 {
            if let Ok(chunk) = self.ring.read_chunk(n * 2) {
                let (a, b) = chunk.as_slices();
                // stereo pairs never straddle the wrap: the ring and every read and write are even
                for (frame, lr) in out.chunks_exact_mut(channels).zip(a.chunks_exact(2).chain(b.chunks_exact(2))) {
                    if channels == 1 {
                        frame[0] = conv(0.5 * (lr[0] + lr[1]));
                        continue;
                    }
                    frame[0] = conv(lr[0]);
                    frame[1] = conv(lr[1]);
                    for x in &mut frame[2..] {
                        *x = conv(0.0);
                    }
                }
                chunk.commit_all();
            }
            self.primed = true;
        }
        for x in &mut out[n * channels..] {
            *x = conv(0.0);
        }
        self.taken += n as u64;
        if let Some(t) = &self.render {
            t.unpark();
        }
        self.mon.end(start, frames, self.primed && n < frames);
    }
}

impl Drop for Feed {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(t) = &self.render {
            t.unpark();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::scene::Scene;

    const OSC: &str = "[[node]]\nid=\"o\"\nkind=\"osc\"\nfreq=440\ngain=0.5\n\
                       [[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n[[link]]\nfrom=\"o\"\nto=\"out.in\"\n";

    fn halves() -> (Feed, Renderer) {
        let g = Graph::build(&Scene::parse(OSC).unwrap()).unwrap();
        let (e, _) = Engine::build(&g, 48_000.0, 1.0).unwrap();
        let (mut feed, r) = pair(e, None);
        feed.mon.settle = 0;
        (feed, r)
    }

    #[test]
    fn the_renderer_fills_to_the_buffer_plus_the_lead_and_stops_when_full() {
        let (_feed, mut r) = halves();
        assert_eq!(r.fill(0), (FIRST_GUESS + LEAD_MIN) as usize / BLOCK, "the first guess + one block");
        assert_eq!(r.queued(), (FIRST_GUESS + LEAD_MIN) as usize);
        assert_eq!(r.fill(1), 0, "full enough: nothing more");
        // a huge device buffer: it fills up to the ring's capacity and stops there (full)
        r.shared.glitches.note_device_frames(CAPACITY as u32 * 2);
        r.fill(2);
        assert_eq!(r.queued(), CAPACITY, "the ring is full");
        assert_eq!(r.fill(3), 0);
    }

    #[test]
    fn the_callback_copies_across_the_wrap_and_counts_frames() {
        let (mut feed, mut r) = halves();
        let mut out = vec![0.0f32; 480 * 2];
        let mut first = None;
        // 200 callbacks of 480 frames = 96 000 frames through a 16 384-frame ring: it wraps several times
        for k in 0..200u64 {
            r.fill(k * 10_000_000);
            feed.play(&mut out, 2, 1_000_000_000 + k * 10_000_000, |x| x);
            first.get_or_insert(out[2 * deck_core::limiter::LOOK + 40]);
            assert!(out.iter().any(|x| *x != 0.0), "callback {k}: sound");
        }
        assert_eq!(feed.taken, 96_000);
        assert_eq!(feed.glitches().count(), 0, "no underrun while the renderer keeps up");
        assert!(first.unwrap().abs() > 0.0);
    }

    #[test]
    fn an_empty_ring_is_silence_and_an_underrun_after_it_had_audio() {
        let (mut feed, mut r) = halves();
        let mut out = vec![1.0f32; 480 * 4];
        feed.play(&mut out, 4, 1_000, |x| x);
        assert!(out.iter().all(|x| *x == 0.0), "before the first block: silence");
        assert_eq!(feed.glitches().count(), 0, "not primed: no underrun");
        r.fill(0);
        let at = 2_000_000_000; // past the monitor's settling
        feed.play(&mut out, 4, at, |x| x);
        assert!(out.chunks(4).all(|f| f[2] == 0.0 && f[3] == 0.0), "the extra channels are silent");
        // the renderer stalls: the callbacks drain the ring and then run short
        for k in 1..6 {
            feed.play(&mut out, 4, at + k * 10_000_000, |x| x);
        }
        assert!(out.iter().all(|x| *x == 0.0), "drained: silence");
        assert!(feed.glitches().counts()[2] >= 1, "an underrun: {:?}", feed.glitches().counts());
    }

    /// the blocks are stamped with the callback's moment + the frames between (the ring's lead in the time base)
    #[test]
    fn blocks_are_stamped_through_the_ring() {
        let g = Graph::build(&Scene::parse(OSC).unwrap()).unwrap();
        let (e, h) = Engine::build(&g, 48_000.0, 1.0).unwrap();
        let (mut feed, mut r) = pair(e, None);
        r.fill(0);
        let mut out = vec![0.0f32; 480 * 2];
        feed.play(&mut out, 2, 5_000_000_000, |x| x);
        let pushed = r.pushed;
        r.fill(1);
        // the newest block starts at frame `pushed`; 480 were taken at 5 s: heard at 5 s + (pushed − 0) frames … the
        // stamp says frame f is heard at t (the limiter's look-ahead added)
        let (_, [f, t]) = h.clock.stamp.read().unwrap();
        let expect = 5_000_000_000 + ((f as f64 - 0.0) * 1e9 / 48_000.0) as u64 + (deck_core::limiter::LOOK as f64 * 1e9 / 48_000.0) as u64;
        assert!(f >= pushed && t.abs_diff(expect) < 1_000, "frame {f} at {t}, expected {expect}");
    }

    /// an underrun grows the lead a block: the renderer keeps more in the ring; a late callback doesn't – the ring was
    /// full, the device's thread stalled (#230)
    #[test]
    fn an_underrun_grows_the_lead_a_late_callback_does_not() {
        let (mut feed, mut r) = halves();
        r.fill(0);
        let before = r.queued();
        let mut out = vec![0.0f32; 480 * 2];
        // the real clock: play measures its callback with now_ns (on Linux the uptime, so a made-up moment like 2 s
        // would lie before it and never be late – CI run on c03cc929)
        let at = crate::clock::now_ns();
        feed.play(&mut out, 2, at, |x| x);
        // a callback after a 40 ms stall: late. `play` timed itself on the real clock (now_ns: CLOCK_MONOTONIC since
        // boot on Linux, process-relative elsewhere), so the stall is measured from it – a synthetic time would come
        // before it on a machine up longer than that and count as no interval at all (#230)
        let t = crate::clock::now_ns();
        assert_eq!(feed.mon.measure(t + 50_000_000, t + 50_100_000, 480, false), Some(super::super::glitch::Cause::Late));
        assert_eq!(feed.glitches().count(), 1);
        r.fill(at + 1_000_000_000);
        assert_eq!(feed.glitches().lead(), LEAD_MIN, "a late callback: the lead stays");
        // the ring ran dry under a callback: an underrun
        assert_eq!(feed.mon.measure(t + 60_000_000, t + 60_100_000, 480, true), Some(super::super::glitch::Cause::Underrun));
        assert_eq!(feed.glitches().counts(), [1, 0, 1]);
        r.fill(at + 2_000_000_000);
        assert_eq!(feed.glitches().lead(), LEAD_MIN + BLOCK as u32);
        assert!(r.queued() >= before - 480 * 2 + BLOCK, "more ahead: {} (was {before})", r.queued());
    }
}
