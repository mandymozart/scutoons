//! `live-core` – the universal core for the browser deck (#184, DECISIONS 2026-10-30; P6 #320: deck-engine, the native
//! deck's engine): one wasm module a Web Worker hosts (shared/live-core). The boundary carries no JSON on the hot path:
//!
//! - **In:** the worker writes wire records (wire.rs) straight into the **inbox** – a region of wasm memory it holds a
//!   `Uint8Array` view on – and calls `lc_apply(len)`. No argument copying, no glue.
//! - **Out:** `lc_render` / `lc_analyse` return pointers; the worker reads the audio (interleaved device channels) and
//!   the analysis frame as `Float32Array` views and copies them once into its SharedArrayBuffer rings.
//! - **Edges:** the scene goes in as TOML text (`lc_load`, once), the load table comes out as JSON in the **outbox**
//!   (once); errors too (a negative length).
//!
//! Exports (all `lc_*`, no imports): `lc_init(rate, channels)`, `lc_inbox(len) -> ptr`, `lc_outbox() -> ptr`,
//! `lc_load(len) -> ±len`, `lc_apply(len) -> count | -len`, `lc_render(frames) -> ptr`, `lc_analyse(dt, lead) -> ptr`,
//! `lc_signals() -> len`, `lc_signals_at() -> ptr`; the tempo clock (#321, clock.rs): `lc_clock_frame(now_ms) -> ptr`
//! (the state block, `clock::STATE` f64), `lc_clock_stamp`, `lc_clock_pong`, `lc_clock_tempo`, `lc_clock_midi`,
//! `lc_clock_joined`, `lc_clock_mode` (#370), `lc_clock_key`, `lc_clock_line(now_ms) -> len` (the line, in the outbox), the
//! MIDI clock out `lc_clock_out_on(on)`, `lc_clock_out() -> count`, `lc_clock_out_at() -> ptr`; the adaptive lead (#365,
//! deck-core `lead`) `lc_lead_class(class) -> frames`, `lc_lead(now_ms, dropouts) -> frames`; the audio side of the
//! monitor (#364, deck-core `monitor::AudioWindow`) `lc_note_block(us)`, `lc_note_analysis(us)`, `lc_note_gap(ms)`,
//! `lc_note_warm(ms)`, `lc_monitor_take() -> ptr` (`lc_monitor_words()` f64 for the page's monitor). The control half steps
//! inside the render (every 128-frame quantum) – there is no step export. A grown memory detaches the views – the host
//! re-makes them (shared/live-core).
pub mod clock;
pub mod core;
pub mod layout;
pub mod signals;
pub mod wire;

use std::cell::RefCell;

pub use crate::clock::Clock;
pub use crate::core::Core;
use deck_core::lead::{AdaptiveLead, DeviceClass};
use deck_core::monitor::AudioWindow;
use deck_core::tempo::Anchor;

/// The inbox's first size; `lc_inbox(len)` grows it for a bigger scene.
pub const INBOX: usize = 64 * 1024;

/// The state behind the exports: the inbox / outbox and the loaded core.
pub struct Host {
    rate: f32,
    /// the device channels the page's audio destination has (the engine renders them all)
    channels: usize,
    inbox: Vec<u8>,
    outbox: Vec<u8>,
    core: Option<Core>,
    /// the tempo clock: it outlives a scene (the show goes on)
    pub clock: Clock,
    /// #365: the adaptive lead (the page's device class), #364: the audio side's window and its words for the page;
    /// the last render's frames (a block's budget)
    pub lead: AdaptiveLead,
    pub audio: AudioWindow,
    words: Vec<f64>,
    rendered: usize,
}

impl Host {
    pub fn new(rate: f32, channels: usize) -> Host {
        let lead = AdaptiveLead::of(DeviceClass::Desktop);
        let audio = AudioWindow::with_lead(lead.current(), rate);
        Host { rate, channels, inbox: vec![0; INBOX], outbox: Vec::new(), core: None, clock: Clock::new(rate), lead, audio, words: vec![0.0; AudioWindow::WORDS], rendered: 0 }
    }

    /// The inbox, at least `len` bytes.
    pub fn inbox(&mut self, len: usize) -> &mut [u8] {
        if self.inbox.len() < len {
            self.inbox.resize(len.next_power_of_two(), 0);
        }
        &mut self.inbox
    }

    pub fn outbox(&self) -> &[u8] {
        &self.outbox
    }

    fn answer(&mut self, r: Result<String, String>) -> i32 {
        let (ok, text) = match r {
            Ok(t) => (true, t),
            Err(e) => (false, e),
        };
        self.outbox = text.into_bytes();
        let n = self.outbox.len() as i32;
        if ok {
            n
        } else {
            -n
        }
    }

    /// The scene in the inbox's first `len` bytes (UTF-8 TOML) → the load table in the outbox (> 0) or an error (< 0).
    pub fn load(&mut self, len: usize) -> i32 {
        let r = std::str::from_utf8(&self.inbox[..len.min(self.inbox.len())])
            .map_err(|e| format!("the scene is not UTF-8: {e}"))
            .and_then(|text| Core::load(text, self.rate, self.channels));
        let r = r.map(|core| {
            let table = core.table_json();
            self.clock.loaded(core.tempo(), core.tempo_node());
            self.core = Some(core);
            table
        });
        self.answer(r)
    }

    /// The warm-up (#356, `Core::warm`): the scene in the inbox's first `len` bytes rendered by a throwaway twin for
    /// `quanta` quanta; the loaded core is not touched. 0, or an error in the outbox (< 0).
    pub fn warm(&mut self, len: usize, quanta: usize) -> i32 {
        let r = std::str::from_utf8(&self.inbox[..len.min(self.inbox.len())])
            .map_err(|e| format!("the scene is not UTF-8: {e}"))
            .and_then(|text| Core::warm(text, self.rate, self.channels, quanta));
        match r {
            Ok(()) => 0,
            Err(e) => self.answer(Err(e)),
        }
    }

    /// The records in the inbox's first `len` bytes → how many were applied, or an error in the outbox (< 0).
    pub fn apply(&mut self, len: usize) -> i32 {
        let Some(core) = self.core.as_mut() else {
            return self.answer(Err("no scene loaded".into()));
        };
        match core.apply(&self.inbox[..len.min(self.inbox.len())]) {
            Ok(n) => n as i32,
            Err(e) => self.answer(Err(e)),
        }
    }

    pub fn core(&mut self) -> Option<&mut Core> {
        self.core.as_mut()
    }

    /// `frames` frames of the loaded scene (counted by the clock); None before a scene is loaded
    pub fn render(&mut self, frames: usize) -> Option<&[f32]> {
        let core = self.core.as_mut()?;
        self.clock.rendered(frames);
        self.rendered = frames;
        Some(core.render(frames))
    }

    /// the analysis frame of what is heard now, the clock's position at the heard moment in it
    pub fn analyse(&mut self, dt: f32, lead: usize) -> Option<&[f32]> {
        let now = self.clock.controller().now();
        let frame = self.core.as_mut()?.analyse(dt, lead, &now);
        self.audio.limiter(frame[wire::REDUCTION]);
        Some(frame)
    }

    /// #365: the lead for the page's device class (deck-core's table) – its first value (frames)
    pub fn lead_class(&mut self, class: DeviceClass) -> u32 {
        self.lead = AdaptiveLead::of(class);
        self.audio.lead = self.lead.current();
        self.audio.lead
    }

    /// #365: the worklet's dropouts so far at `now_ms` → the lead (frames)
    pub fn lead(&mut self, now_ms: f64, dropouts: u64) -> u32 {
        self.audio.lead = self.lead.update(now_ms / 1000.0, dropouts);
        self.audio.lead
    }

    /// #364: the last render took `us` (the worker's stopwatch) – against its frames' budget
    pub fn note_block(&mut self, us: f32) {
        self.audio.block(us, self.rendered as f32 * 1e6 / self.rate);
    }

    /// #364: the window so far as words (the page's monitor reads them); it starts again
    pub fn monitor_take(&mut self) -> &[f64] {
        self.audio.take().to_words(&mut self.words);
        &self.words
    }
}

thread_local! {
    static HOST: RefCell<Host> = RefCell::new(Host::new(48_000.0, 2));
}

fn with<R>(f: impl FnOnce(&mut Host) -> R) -> R {
    HOST.with(|h| f(&mut h.borrow_mut()))
}

/// A fresh host at the audio context's sample rate for a device of `channels` channels (drops a loaded scene).
#[no_mangle]
pub extern "C" fn lc_init(rate: f32, channels: usize) {
    with(|h| *h = Host::new(rate, channels));
}

/// The inbox's address, grown to at least `len` bytes (a grown wasm memory → the host re-makes its views).
#[no_mangle]
pub extern "C" fn lc_inbox(len: usize) -> *mut u8 {
    with(|h| h.inbox(len).as_mut_ptr())
}

/// The outbox's address (its length is what `lc_load` / `lc_apply` returned).
#[no_mangle]
pub extern "C" fn lc_outbox() -> *const u8 {
    with(|h| h.outbox().as_ptr())
}

#[no_mangle]
pub extern "C" fn lc_load(len: usize) -> i32 {
    with(|h| h.load(len))
}

/// The warm-up (#356): the scene in the inbox (`len` bytes of TOML) played silently by a throwaway twin for `quanta`
/// quanta (0 = `core::WARM_QUANTA`) – the code a scene runs is compiled and optimised before it is heard. 0 or −len.
#[no_mangle]
pub extern "C" fn lc_warm(len: usize, quanta: usize) -> i32 {
    with(|h| h.warm(len, if quanta == 0 { core::WARM_QUANTA } else { quanta }))
}

#[no_mangle]
pub extern "C" fn lc_apply(len: usize) -> i32 {
    with(|h| h.apply(len))
}

/// `frames` frames rendered (× the device channels, interleaved); their address (null before a scene is loaded).
#[no_mangle]
pub extern "C" fn lc_render(frames: usize) -> *const f32 {
    with(|h| h.render(frames).map_or(std::ptr::null(), <[f32]>::as_ptr))
}

/// The analysis frame's address (`wire::FRAME` f32 + the values at `wire::VALUE_COUNT`; null before a scene is loaded).
#[no_mangle]
pub extern "C" fn lc_analyse(dt: f32, lead: usize) -> *const f32 {
    with(|h| h.analyse(dt, lead).map_or(std::ptr::null(), <[f32]>::as_ptr))
}

/// The device messages the scene's `[outputs]` sent since the last call (#282): their length in bytes; the records
/// (signals.rs) are at `lc_signals_at()` until the next call. 0 before a scene is loaded.
#[no_mangle]
pub extern "C" fn lc_signals() -> usize {
    with(|h| h.core().map_or(0, |c| c.take_signals().len()))
}

/// The address of the records `lc_signals()` just handed out.
#[no_mangle]
pub extern "C" fn lc_signals_at() -> *const u8 {
    with(|h| h.core().map_or(std::ptr::null(), |c| c.signals_ptr()))
}

/// #365: the adaptive lead for the page's device class (0 phone, 1 desktop – deck-core `DeviceClass`) → its first value
/// (frames)
#[no_mangle]
pub extern "C" fn lc_lead_class(class: u32) -> u32 {
    with(|h| h.lead_class(DeviceClass::from_code(class)))
}

/// #365: the worklet's dropouts so far (a running total) at the page's `now_ms` → the lead (frames)
#[no_mangle]
pub extern "C" fn lc_lead(now_ms: f64, dropouts: f64) -> u32 {
    with(|h| h.lead(now_ms, dropouts as u64))
}

/// #364: the last `lc_render` took `us` (the worker's stopwatch)
#[no_mangle]
pub extern "C" fn lc_note_block(us: f32) {
    with(|h| h.note_block(us))
}

/// #364: an analysis frame (+ its signals) took `us` – between blocks
#[no_mangle]
pub extern "C" fn lc_note_analysis(us: f32) {
    with(|h| h.audio.analysis(us))
}

/// #364: the worker's pump paused `ms` since its last run
#[no_mangle]
pub extern "C" fn lc_note_gap(ms: f32) {
    with(|h| h.audio.gap(ms))
}

/// #356: the warm-up before the first sound took `ms`
#[no_mangle]
pub extern "C" fn lc_note_warm(ms: f32) {
    with(|h| h.audio.warm_ms = ms)
}

/// #364: the audio window since the last call as `lc_monitor_words()` f64 (deck-core `AudioWindow::to_words`) – the
/// address; the window starts again
#[no_mangle]
pub extern "C" fn lc_monitor_take() -> *const f64 {
    with(|h| h.monitor_take().as_ptr())
}

/// #364: how many words `lc_monitor_take` hands out (`AudioWindow::WORDS`)
#[no_mangle]
pub extern "C" fn lc_monitor_words() -> usize {
    AudioWindow::WORDS
}

/// One clock frame at the page's `now_ms` (`performance.timeOrigin + performance.now()`): the state block's address
/// (`clock::STATE` f64 – the heard moment's beat, the show clock, a ping due, an anchor to send).
#[no_mangle]
pub extern "C" fn lc_clock_frame(now_ms: f64) -> *const f64 {
    with(|h| h.clock.frame(now_ms).as_ptr())
}

/// The clock's line at the page's `now_ms` (deck-core `Controller::line`: the tempo line and the source's state, the
/// native HUD's texts, #370) → its length; the text is in the outbox.
#[no_mangle]
pub extern "C" fn lc_clock_line(now_ms: f64) -> i32 {
    with(|h| {
        let line = h.clock.hud_line(now_ms);
        h.answer(Ok(line))
    })
}


/// The page's MIDI OUT: the clock out on (1) or off (0) – deck-core's `ClockOut`, the native deck's schedule.
#[no_mangle]
pub extern "C" fn lc_clock_out_on(on: u32) {
    with(|h| h.clock.set_out(on != 0));
}

/// The clock out's messages of the last clock frame → how many; at `lc_clock_out_at()`: [page ms, status, d1, d2] f64
/// each (NaN = no such data byte), to write to Web MIDI with that timestamp.
#[no_mangle]
pub extern "C" fn lc_clock_out() -> usize {
    with(|h| h.clock.out().len() / 4)
}

#[no_mangle]
pub extern "C" fn lc_clock_out_at() -> *const f64 {
    with(|h| h.clock.out().as_ptr())
}

/// `getOutputTimestamp`: context second `context` heard at the page's `heard_ms`; `skew` = the worklet's context frame
/// minus the frames it played
#[no_mangle]
pub extern "C" fn lc_clock_stamp(context: f64, heard_ms: f64, skew: f64) {
    with(|h| h.clock.stamp(context, heard_ms, skew));
}

/// a pong: `t0_us` (the page's µs it sent), the relay's `t1_ms`, back at the page's `t2_ms`
#[no_mangle]
pub extern "C" fn lc_clock_pong(t0_us: f64, t1_ms: f64, t2_ms: f64) {
    with(|h| h.clock.pong(t0_us, t1_ms, t2_ms));
}

/// the relay's `tempo`: the anchor (relay ms `at`), arrived at the page's `arrived_ms`
#[no_mangle]
pub extern "C" fn lc_clock_tempo(bpm: f64, per_bar: u32, beat: f64, at: f64, running: u32, arrived_ms: f64) {
    let a = Anchor { bpm, per_bar, beat, at: at as i64, running: running != 0 };
    with(|h| h.clock.anchor(a, arrived_ms));
}

/// Web MIDI: a clock message (status byte first), stamped with the page's ms
#[no_mangle]
pub extern "C" fn lc_clock_midi(status: u32, d1: u32, d2: u32, at_ms: f64) {
    with(|h| h.clock.midi([status as u8, d1 as u8, d2 as u8], at_ms));
}

/// joined the relay (again)
#[no_mangle]
pub extern "C" fn lc_clock_joined(now_ms: f64) {
    with(|h| h.clock.joined(now_ms));
}

/// the picker: `deck_core::tempo::ClockMode::ALL`'s index (show, own, lead, midi) – #370
#[no_mangle]
pub extern "C" fn lc_clock_mode(mode: u32) {
    with(|h| h.clock.set_mode(mode as usize));
}

/// a key: 0 tap, 1 faster, 2 slower, 3 start / stop, 4 the click
#[no_mangle]
pub extern "C" fn lc_clock_key(key: u32, at_ms: f64) {
    with(|h| h.clock.key(key as usize, at_ms));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{encode_send, Mode};

    const OSC: &[u8] = b"[[node]]\nid = \"o\"\nkind = \"osc\"\nfreq = 440.0\ngain = 0.0\n[[node]]\nid = \"out\"\nkind = \"audio_out\"\n[[link]]\nfrom = \"o\"\nto = \"out.in\"\n";

    #[test]
    fn the_host_loads_applies_and_answers() {
        let mut h = Host::new(48_000.0, 2);
        assert!(h.apply(0) < 0);
        assert_eq!(std::str::from_utf8(h.outbox()).unwrap(), "no scene loaded");
        h.inbox(OSC.len())[..OSC.len()].copy_from_slice(OSC);
        let n = h.load(OSC.len());
        assert!(n > 0, "{}", std::str::from_utf8(h.outbox()).unwrap());
        let table: serde_json::Value = serde_json::from_slice(&h.outbox()[..n as usize]).unwrap();
        assert_eq!(table["channels"], 2);
        assert_eq!(table["nodes"][0]["ports"][1], "freq");
        let mut b = vec![];
        encode_send(&mut b, Mode::Force, 0, 2, 1.0, &[0.5]);
        encode_send(&mut b, Mode::Blend, 0, 1, 0.5, &[880.0]);
        h.inbox(b.len())[..b.len()].copy_from_slice(&b);
        assert_eq!(h.apply(b.len()), 2);
        for _ in 0..40 {
            h.core().unwrap().render(128);
        }
        assert!(h.core().unwrap().render(256).iter().any(|x| x.abs() > 0.3), "after the glide (#188)");
        // a broken scene keeps the running one
        h.inbox(4)[..4].copy_from_slice(b"[[no");
        assert!(h.load(4) < 0);
        assert!(h.core().is_some());
    }

    /// #356: the warm-up renders a twin – the loaded core plays on bit for bit as if nothing had happened, a broken
    /// scene is an error with its reason
    #[test]
    fn the_warm_up_never_touches_what_is_heard() {
        let scene = String::from_utf8(OSC.to_vec()).unwrap().replace("gain = 0.0", "gain = 0.5");
        let heard = |warm: bool| {
            let mut h = Host::new(48_000.0, 2);
            h.inbox(scene.len())[..scene.len()].copy_from_slice(scene.as_bytes());
            assert!(h.load(scene.len()) > 0);
            let mut out = h.core().unwrap().render(128).to_vec();
            if warm {
                assert_eq!(h.warm(scene.len(), 64), 0);
            }
            for _ in 0..20 {
                out.extend_from_slice(h.core().unwrap().render(128));
            }
            out
        };
        let (cold, warmed) = (heard(false), heard(true));
        assert!(cold.iter().zip(&warmed).all(|(a, b)| a.to_bits() == b.to_bits()), "the same samples");
        let mut h = Host::new(48_000.0, 2);
        h.inbox(4)[..4].copy_from_slice(b"[[no");
        assert!(h.warm(4, 8) < 0);
        assert!(!h.outbox().is_empty());
    }

    #[test]
    fn the_inbox_grows_for_a_big_scene() {
        let mut h = Host::new(48_000.0, 2);
        assert_eq!(h.inbox(10).len(), INBOX);
        assert_eq!(h.inbox(INBOX + 1).len(), 2 * INBOX);
    }

    /// the device's channel count reaches the engine: a stereo scene on six channels plays on the first two
    #[test]
    fn the_host_renders_the_devices_channels() {
        let mut h = Host::new(48_000.0, 6);
        let scene = String::from_utf8(OSC.to_vec()).unwrap().replace("gain = 0.0", "gain = 0.5");
        h.inbox(scene.len())[..scene.len()].copy_from_slice(scene.as_bytes());
        assert!(h.load(scene.len()) > 0);
        let out = h.core().unwrap().render(1024).to_vec();
        assert_eq!(out.len(), 1024 * 6);
        let peak = |c: usize| out[c..].iter().step_by(6).fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak(0) > 0.3 && peak(1) > 0.3);
        assert!((2..6).all(|c| peak(c) == 0.0), "no outlet on 3..6");
    }
}
