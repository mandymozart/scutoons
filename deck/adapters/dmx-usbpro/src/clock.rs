//! The adapter thread's clock (#639): the real one, and in the tests a virtual one, so the frame timing is checked
//! without wall-time budgets a loaded machine misses.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Instant;

use crate::Cmd;

pub(crate) trait Clock: Send + Sync {
    fn now(&self) -> Instant;
    /// The next command, or `Timeout` once `deadline` has come.
    fn recv_until(&self, rx: &Receiver<Cmd>, deadline: Instant) -> Result<Cmd, RecvTimeoutError>;
}

pub(crate) struct Real;

impl Clock for Real {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn recv_until(&self, rx: &Receiver<Cmd>, deadline: Instant) -> Result<Cmd, RecvTimeoutError> {
        rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
    }
}

/// Time stands still until the test moves it. The adapter thread reports each time it is idle (no command waiting,
/// its deadline not reached): `settle` waits for that, `advance` moves time on deadline by deadline.
#[cfg(test)]
pub(crate) mod virtual_clock {
    use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
    use std::sync::{Condvar, Mutex};
    use std::time::{Duration, Instant};

    use super::Clock;
    use crate::Cmd;

    /// Only a hang fails on this; a correct adapter goes idle in microseconds.
    const HANG: Duration = Duration::from_secs(60);

    struct State {
        now: Instant,
        /// how often the thread found itself idle
        idle: u64,
        /// its deadline when it was last idle
        deadline: Option<Instant>,
    }

    pub(crate) struct Virtual {
        state: Mutex<State>,
        changed: Condvar,
    }

    impl Virtual {
        pub(crate) fn new() -> Self {
            Virtual { state: Mutex::new(State { now: Instant::now(), idle: 0, deadline: None }), changed: Condvar::new() }
        }

        /// Waits until the thread has handled every command sent before and every frame due now.
        pub(crate) fn settle(&self) {
            let mut s = self.state.lock().unwrap();
            // the next idle report may come from a poll before the call; the one after it polled after the call
            let want = s.idle + 2;
            let end = Instant::now() + HANG;
            while s.idle < want {
                assert!(Instant::now() < end, "the adapter thread never went idle");
                s = self.changed.wait_timeout(s, Duration::from_millis(1)).unwrap().0;
            }
        }

        /// Moves time on by `d`, stopping at each of the thread's deadlines on the way (frames go out exactly when due).
        pub(crate) fn advance(&self, d: Duration) {
            let target = self.state.lock().unwrap().now + d;
            loop {
                self.settle();
                let mut s = self.state.lock().unwrap();
                let next = s.deadline.map_or(target, |dl| dl.min(target));
                if next <= s.now {
                    return;
                }
                s.now = next;
                self.changed.notify_all();
            }
        }
    }

    impl Clock for Virtual {
        fn now(&self) -> Instant {
            self.state.lock().unwrap().now
        }

        fn recv_until(&self, rx: &Receiver<Cmd>, deadline: Instant) -> Result<Cmd, RecvTimeoutError> {
            loop {
                match rx.try_recv() {
                    Ok(cmd) => return Ok(cmd),
                    Err(TryRecvError::Disconnected) => return Err(RecvTimeoutError::Disconnected),
                    Err(TryRecvError::Empty) => {}
                }
                let mut s = self.state.lock().unwrap();
                if s.now >= deadline {
                    return Err(RecvTimeoutError::Timeout);
                }
                s.idle += 1;
                s.deadline = Some(deadline);
                self.changed.notify_all();
                // woken by `advance`; the short timeout picks up commands (the channel has no wake-up of its own here)
                drop(self.changed.wait_timeout(s, Duration::from_millis(1)).unwrap());
            }
        }
    }
}
