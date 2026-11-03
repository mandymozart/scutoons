//! The show clock's side of the relay link (#321, plan §3.7): `Source::Relay` made real. The link thread (space/link.rs)
//! stamps what the relay says about time the moment it reads it – a pong, a `tempo` anchor, a welcome – and hands it
//! over; the frame thread moves it into deck-core's controller, which decides everything: when to ping (the link sends
//! one, stamped with its real `t0` in µs of `now_ns`), whether this deck leads and what anchor goes out.
use super::Tempo;
use crate::clock::now_ns;
use deck_core::tempo::{Anchor, Event, What};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Shared by the link thread and the frame thread (an `Arc`): a few events a second, a flag, one anchor.
#[derive(Default)]
pub struct ShowLink {
    inbox: Mutex<Vec<Event>>,
    ping: AtomicBool,
    out: Mutex<Option<Anchor>>,
}

/// the wire's anchor as deck-core's (None without a moment – the relay always sets one)
pub fn anchor_of(t: &protocol::Tempo) -> Option<Anchor> {
    Some(Anchor { bpm: t.bpm, per_bar: t.per_bar.clamp(1, 32) as u32, beat: t.beat, at: t.at?, running: t.running })
}

pub fn tempo_of(a: &Anchor) -> protocol::Tempo {
    protocol::Tempo { bpm: a.bpm, per_bar: u64::from(a.per_bar), beat: a.beat, running: a.running, at: Some(a.at), domain: None }
}

impl ShowLink {
    fn push(&self, what: What) {
        let e = Event { at_ns: now_ns(), what };
        if let Ok(mut q) = self.inbox.lock() {
            q.push(e);
        }
    }

    /// the link thread: a pong just read (a `t0` of 0 is not a deck's – ignored)
    pub fn pong(&self, p: &protocol::Pong) {
        if p.t0 > 0 {
            self.push(What::Pong { t0_ns: p.t0 as u64 * 1000, t1_ms: p.t1 });
        }
    }

    /// the link thread: the relay's `tempo` (or the welcome snapshot's)
    pub fn tempo(&self, t: &protocol::Tempo) {
        if let Some(a) = anchor_of(t) {
            self.push(What::Tempo(a));
        }
    }

    /// the link thread: joined (again) – the clock's filter starts over, a leader sends again
    pub fn joined(&self) {
        self.push(What::Joined);
    }

    /// the link thread: the ping the clock asked for, stamped now
    pub fn take_ping(&self) -> Option<protocol::Ping> {
        self.ping.swap(false, Ordering::Relaxed).then(|| protocol::Ping { t0: (now_ns() / 1000) as i64 })
    }

    /// the link thread: the anchor this deck leads with, to send
    pub fn take_tempo(&self) -> Option<protocol::Tempo> {
        self.out.lock().ok()?.take().as_ref().map(tempo_of)
    }

    /// the frame thread, before the clock's frame: what the link stamped
    pub fn feed(&self, tempo: &mut Tempo) {
        let events = self.inbox.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
        for e in events {
            tempo.push(e);
        }
    }

    /// the frame thread, after the clock's frame: a ping if one is due, the anchor if this deck leads and it moved
    pub fn after(&self, tempo: &mut Tempo) {
        if tempo.ping_due(now_ns()) {
            self.ping.store(true, Ordering::Relaxed);
        }
        if let Some(a) = tempo.take_anchor() {
            if let Ok(mut o) = self.out.lock() {
                *o = Some(a);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wire_and_the_core_agree_on_an_anchor() {
        let t = protocol::Tempo { bpm: 128.0, per_bar: 3, beat: 7.5, running: true, at: Some(1_761_700_003_000), domain: None };
        let a = anchor_of(&t).unwrap();
        assert_eq!(tempo_of(&a), t);
        // the clock domain is reserved: the one show clock's anchor carries none on the wire (an older deck reads it)
        assert!(!serde_json::to_string(&tempo_of(&a)).unwrap().contains("domain"));
        assert!(anchor_of(&protocol::Tempo { at: None, ..t.clone() }).is_none());
        let s = ShowLink::default();
        s.pong(&protocol::Pong { t0: 0, t1: 5 });
        s.pong(&protocol::Pong { t0: 42, t1: 5 });
        assert_eq!(s.inbox.lock().unwrap().len(), 1, "a keepalive's t0 0 is not a sample");
        assert!(s.take_ping().is_none());
        s.ping.store(true, Ordering::Relaxed);
        assert!(s.take_ping().unwrap().t0 > 0);
    }
}
