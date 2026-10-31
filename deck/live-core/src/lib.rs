//! `live-core` – deck-core for the browser Live mode (#184, DECISIONS 2026-10-30): one wasm module a Web Worker hosts
//! (shared/live-core). The boundary carries no JSON on the hot path:
//!
//! - **In:** the worker writes wire records (wire.rs) straight into the **inbox** – a region of wasm memory it holds a
//!   `Uint8Array` view on – and calls `lc_apply(len)`. No argument copying, no glue.
//! - **Out:** `lc_render` / `lc_analyse` return pointers; the worker reads the audio block and the analysis frame as
//!   `Float32Array` views and copies them once into its SharedArrayBuffer rings.
//! - **Edges:** the scene goes in as TOML text (`lc_load`, once), the load table comes out as JSON in the **outbox**
//!   (once); errors too (a negative length).
//!
//! Exports (all `lc_*`, no imports): `lc_init(rate)`, `lc_inbox(len) -> ptr`, `lc_outbox() -> ptr`,
//! `lc_load(len) -> ±len`, `lc_apply(len) -> count | -len`, `lc_step(dt)`, `lc_render(frames) -> ptr`,
//! `lc_analyse(dt, lead) -> ptr`. A grown memory detaches the views – the host re-makes them (shared/live-core).
pub mod core;
pub mod wire;

use std::cell::RefCell;

pub use crate::core::Core;

/// The inbox's first size; `lc_inbox(len)` grows it for a bigger scene.
pub const INBOX: usize = 64 * 1024;

/// The state behind the exports: the inbox / outbox and the loaded core.
pub struct Host {
    rate: f32,
    inbox: Vec<u8>,
    outbox: Vec<u8>,
    core: Option<Core>,
}

impl Host {
    pub fn new(rate: f32) -> Host {
        Host { rate, inbox: vec![0; INBOX], outbox: Vec::new(), core: None }
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
            .and_then(|text| Core::load(text, self.rate));
        let r = r.map(|core| {
            let table = core.table_json();
            self.core = Some(core);
            table
        });
        self.answer(r)
    }

    /// The records in the inbox's first `len` bytes → how many were applied, or an error in the outbox (< 0).
    pub fn apply(&mut self, len: usize) -> i32 {
        let Some(core) = self.core.as_mut() else { return self.answer(Err("no scene loaded".into())) };
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
    static HOST: RefCell<Host> = RefCell::new(Host::new(48_000.0));
}

fn with<R>(f: impl FnOnce(&mut Host) -> R) -> R {
    HOST.with(|h| f(&mut h.borrow_mut()))
}

/// A fresh host at the audio context's sample rate (drops a loaded scene).
#[no_mangle]
pub extern "C" fn lc_init(rate: f32) {
    with(|h| *h = Host::new(rate));
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

#[no_mangle]
pub extern "C" fn lc_apply(len: usize) -> i32 {
    with(|h| h.apply(len))
}

#[no_mangle]
pub extern "C" fn lc_step(dt: f32) {
    with(|h| {
        if let Some(c) = h.core() {
            c.step(dt)
        }
    })
}

/// `frames` samples rendered; their address (null before a scene is loaded).
#[no_mangle]
pub extern "C" fn lc_render(frames: usize) -> *const f32 {
    with(|h| h.core().map_or(std::ptr::null(), |c| c.render(frames).as_ptr()))
}

/// The analysis frame's address (`wire::FRAME` f32; null before a scene is loaded).
#[no_mangle]
pub extern "C" fn lc_analyse(dt: f32, lead: usize) -> *const f32 {
    with(|h| h.core().map_or(std::ptr::null(), |c| c.analyse(dt, lead).as_ptr()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{encode_send, Mode};

    #[test]
    fn the_host_loads_applies_and_answers() {
        let mut h = Host::new(48_000.0);
        assert!(h.apply(0) < 0);
        assert_eq!(std::str::from_utf8(h.outbox()).unwrap(), "no scene loaded");
        let scene = b"[[node]]\nid = \"o\"\nkind = \"osc\"\nfreq = 440.0\ngain = 0.0\n";
        h.inbox(scene.len())[..scene.len()].copy_from_slice(scene);
        let n = h.load(scene.len());
        assert_eq!(&h.outbox()[..n as usize], br#"{"nodes":[{"id":"o","kind":"osc","ports":["freq","gain"]}]}"#);
        let mut b = vec![];
        encode_send(&mut b, Mode::Force, 0, 1, 1.0, &[0.5]);
        encode_send(&mut b, Mode::Blend, 0, 0, 0.5, &[880.0]);
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

    #[test]
    fn the_inbox_grows_for_a_big_scene() {
        let mut h = Host::new(48_000.0);
        assert_eq!(h.inbox(10).len(), INBOX);
        assert_eq!(h.inbox(INBOX + 1).len(), 2 * INBOX);
    }
}
