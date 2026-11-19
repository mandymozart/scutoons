//! The device watch's time: the real clock in a deck, a virtual one in the tests, so the watch's ticks, the timer's
//! blocks and a fake device's callbacks keep their schedule however loaded the machine is.
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

#[derive(Clone)]
pub enum Time {
    Real,
    #[cfg(test)]
    Virtual(std::sync::Arc<virtual_clock::Virtual>),
}

impl Time {
    pub fn now_ns(&self) -> u64 {
        match self {
            Time::Real => crate::clock::now_ns(),
            #[cfg(test)]
            Time::Virtual(v) => v.now_ns(),
        }
    }

    /// The handle of a thread that waits on this clock: made before the thread is spawned, dropped when it ends.
    pub fn sleeper(&self) -> Sleeper {
        match self {
            Time::Real => Sleeper::Real,
            #[cfg(test)]
            Time::Virtual(v) => Sleeper::Virtual(v.seat()),
        }
    }
}

pub enum Sleeper {
    Real,
    #[cfg(test)]
    Virtual(virtual_clock::Seat),
}

impl Sleeper {
    pub fn now_ns(&self) -> u64 {
        match self {
            Sleeper::Real => crate::clock::now_ns(),
            #[cfg(test)]
            Sleeper::Virtual(s) => s.now_ns(),
        }
    }

    /// until `ns` (on the virtual clock also until `stop`: its thread is joined)
    pub fn sleep_until(&self, ns: u64, stop: &AtomicBool) {
        match self {
            Sleeper::Real => std::thread::sleep(Duration::from_nanos(ns.saturating_sub(crate::clock::now_ns()))),
            #[cfg(test)]
            Sleeper::Virtual(s) => {
                s.wait(ns, || stop.load(std::sync::atomic::Ordering::Relaxed));
            }
        }
    }

    /// `ready` checked every millisecond: true once it holds, false once `ns` has come
    pub fn poll_until(&self, ns: u64, mut ready: impl FnMut() -> bool) -> bool {
        match self {
            Sleeper::Real => loop {
                if ready() {
                    return true;
                }
                if crate::clock::now_ns() >= ns {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(1));
            },
            #[cfg(test)]
            Sleeper::Virtual(s) => s.wait(ns, ready),
        }
    }

    /// the next message, or `Timeout` once `ns` has come
    pub fn recv_until<T>(&self, rx: &Receiver<T>, ns: u64) -> Result<T, RecvTimeoutError> {
        match self {
            Sleeper::Real => rx.recv_timeout(Duration::from_nanos(ns.saturating_sub(crate::clock::now_ns()))),
            #[cfg(test)]
            Sleeper::Virtual(s) => {
                let mut got = None;
                s.wait(ns, || match rx.try_recv() {
                    Ok(m) => {
                        got = Some(Ok(m));
                        true
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => false,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        got = Some(Err(RecvTimeoutError::Disconnected));
                        true
                    }
                });
                got.unwrap_or(Err(RecvTimeoutError::Timeout))
            }
        }
    }
}

/// Time stands still until the test moves it. Every thread that waits on it has a seat; a waiting thread checks what it
/// waits for every real millisecond and counts each check that finds nothing. `settle` waits until every seat has
/// checked twice since and is waiting for a later time; `advance` moves time on to each seat's deadline in turn.
#[cfg(test)]
pub mod virtual_clock {
    use std::collections::HashMap;
    use std::sync::{Arc, Condvar, Mutex, MutexGuard};
    use std::time::{Duration, Instant};

    /// Only a hang fails on this; the threads go idle in microseconds.
    const HANG: Duration = Duration::from_secs(60);

    #[derive(Default)]
    struct Waiting {
        /// the checks that found nothing
        empty: u64,
        /// its deadline while it waits
        until: Option<u64>,
    }

    struct State {
        now: u64,
        next: usize,
        seats: HashMap<usize, Waiting>,
    }

    pub struct Virtual {
        state: Mutex<State>,
        changed: Condvar,
    }

    pub struct Seat {
        clock: Arc<Virtual>,
        id: usize,
    }

    impl Virtual {
        pub fn new() -> Arc<Virtual> {
            // a plausible monotonic time, so nothing that also reads the real one sees the far past
            let now = crate::clock::now_ns();
            Arc::new(Virtual { state: Mutex::new(State { now, next: 0, seats: HashMap::new() }), changed: Condvar::new() })
        }

        pub fn now_ns(&self) -> u64 {
            self.state.lock().unwrap().now
        }

        pub(super) fn seat(self: &Arc<Self>) -> Seat {
            let mut s = self.state.lock().unwrap();
            let id = s.next;
            s.next += 1;
            s.seats.insert(id, Waiting::default());
            Seat { clock: self.clone(), id }
        }

        /// Waits until every thread has handled what was sent before and is waiting for a later time.
        fn settled(&self) -> MutexGuard<'_, State> {
            let mut s = self.state.lock().unwrap();
            let from: HashMap<usize, u64> = s.seats.iter().map(|(id, w)| (*id, w.empty)).collect();
            let end = Instant::now() + HANG;
            loop {
                let now = s.now;
                let idle = s.seats.iter().all(|(id, w)| w.empty >= from.get(id).copied().unwrap_or(0) + 2 && w.until.is_some_and(|u| u > now));
                if idle {
                    return s;
                }
                assert!(Instant::now() < end, "a thread on the virtual clock never went idle");
                s = self.changed.wait_timeout(s, Duration::from_millis(1)).unwrap().0;
            }
        }

        pub fn settle(&self) {
            drop(self.settled());
        }

        /// Moves time on by `d`, stopping at each deadline on the way (every thread wakes exactly when it is due).
        pub fn advance(&self, d: Duration) {
            let target = self.now_ns() + d.as_nanos() as u64;
            loop {
                let mut s = self.settled();
                let next = s.seats.values().filter_map(|w| w.until).fold(target, u64::min);
                if next <= s.now {
                    return;
                }
                s.now = next;
                self.changed.notify_all();
            }
        }
    }

    impl Seat {
        pub fn now_ns(&self) -> u64 {
            self.clock.now_ns()
        }

        /// true once `ready`, false once `until` has come
        pub(super) fn wait(&self, until: u64, mut ready: impl FnMut() -> bool) -> bool {
            let done = |r: bool| {
                if let Some(w) = self.clock.state.lock().unwrap().seats.get_mut(&self.id) {
                    w.until = None;
                }
                r
            };
            loop {
                if ready() {
                    return done(true);
                }
                let mut s = self.clock.state.lock().unwrap();
                if s.now >= until {
                    drop(s);
                    return done(false);
                }
                let w = s.seats.get_mut(&self.id).expect("a seated thread");
                w.empty += 1;
                w.until = Some(until);
                self.clock.changed.notify_all();
                // woken by `advance`; the short timeout picks up messages and flags (they have no wake-up of their own here)
                drop(self.clock.changed.wait_timeout(s, Duration::from_millis(1)).unwrap());
            }
        }
    }

    impl Drop for Seat {
        fn drop(&mut self) {
            self.clock.state.lock().unwrap().seats.remove(&self.id);
            self.clock.changed.notify_all();
        }
    }
}
