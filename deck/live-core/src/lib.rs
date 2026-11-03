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
//! `lc_load(len) -> ±len`, `lc_tempo(bpm) -> 0 | 1`, `lc_apply(len) -> count | -len`, `lc_render(frames) -> ptr`,
//! `lc_analyse(dt, lead) -> ptr`, `lc_signals() -> len`, `lc_signals_at() -> ptr`. The control half steps inside the
//! render (every 128-frame quantum) – there is no step export. A grown memory detaches the views – the host re-makes them
//! (shared/live-core).
pub mod core;
pub mod signals;
pub mod wire;

use std::cell::RefCell;

pub use crate::core::Core;

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
}

impl Host {
    pub fn new(rate: f32, channels: usize) -> Host {
        Host { rate, channels, inbox: vec![0; INBOX], outbox: Vec::new(), core: None }
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

/// The page's tempo for a loaded scene without a clock of its own (`Core::set_tempo`): 1 = taken, 0 = not.
#[no_mangle]
pub extern "C" fn lc_tempo(bpm: f32) -> i32 {
    with(|h| h.core().map_or(0, |c| i32::from(c.set_tempo(bpm))))
}

#[no_mangle]
pub extern "C" fn lc_apply(len: usize) -> i32 {
    with(|h| h.apply(len))
}

/// `frames` frames rendered (× the device channels, interleaved); their address (null before a scene is loaded).
#[no_mangle]
pub extern "C" fn lc_render(frames: usize) -> *const f32 {
    with(|h| h.core().map_or(std::ptr::null(), |c| c.render(frames).as_ptr()))
}

/// The analysis frame's address (`wire::FRAME` f32 + the values at `wire::VALUE_COUNT`; null before a scene is loaded).
#[no_mangle]
pub extern "C" fn lc_analyse(dt: f32, lead: usize) -> *const f32 {
    with(|h| h.core().map_or(std::ptr::null(), |c| c.analyse(dt, lead).as_ptr()))
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
