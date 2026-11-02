//! The sequencer's thread (#236, research/EVENTS.md §16 "Threads"): deck-engine's `Sequencer` (its windows, the rings
//! it fills) runs here on a thread of its own, ahead of the render – never on the audio thread. This file is only the
//! scheduling: how far ahead (`horizon`), the nap between fills, the priority.
//!
//! How far ahead: the device's buffer + the render lead + a block – the render thread renders in bursts of about a
//! device buffer (ring.rs) and the lead grows after a glitch, so the window always covers the next burst. A live event
//! (dispatch.rs) reaches the sound after this horizon + the render's own lead.
use super::glitch::Glitches;
use deck_engine::sequencer::{Sequencer, BLOCK};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// frames to stay ahead of the render: the device's buffer + the render lead + a block
pub fn horizon(glitches: &Glitches) -> u64 {
    u64::from(glitches.device_frames().max(BLOCK as u32) + glitches.lead() + BLOCK as u32)
}

/// The sequencer on a thread of its own ("events-seq", high priority – below the render thread's): it fills to
/// the horizon, then naps a millisecond (a quarter of a block). Dropping the handle stops it.
pub fn spawn(mut seq: Sequencer, glitches: Arc<Glitches>) -> std::io::Result<Thread> {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let handle = std::thread::Builder::new().name("events-seq".into()).spawn(move || {
        match super::priority::raise_high() {
            Ok(how) => tracing::info!("events: the sequencer runs at {how}"),
            Err(e) => tracing::warn!("events: the sequencer at normal priority ({e})"),
        }
        while !flag.load(Ordering::Relaxed) {
            let ahead = horizon(&glitches);
            seq.fill(ahead);
            std::thread::park_timeout(Duration::from_millis(1));
        }
    })?;
    Ok(Thread { stop, handle: Some(handle) })
}

/// the running sequencer: dropping it stops the thread
pub struct Thread {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Thread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            h.thread().unpark();
            let _ = h.join();
        }
    }
}
