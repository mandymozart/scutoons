//! A stop signal ends the show the normal way (#263): SIGTERM / SIGHUP / SIGINT (Ctrl+C on Windows) wake the event
//! loop, which leaves like Ctrl+Q – the outputs go dark (a USB-DMX widget repeats its last frame for ever), the audio
//! fades out, exit 0. The kiosk's stop (`deck-kiosk.service`, #256) gives 5 s; a watchdog ends the process after
//! `GRACE` if the loop does not get there (a wedged driver must not hold the restart).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};

/// How long the show gets to end after the signal, before the process leaves without it (the kiosk kills at 5 s).
pub const GRACE: Duration = Duration::from_millis(3500);

/// What a signal does: the first one wakes the loop and starts the watchdog; a repeat (the group's SIGTERM, then the
/// compositor's own) changes nothing. Returns whether this was the first.
pub fn on_signal(stopping: &AtomicBool, wake: impl FnOnce(), watchdog: impl FnOnce()) -> bool {
    if stopping.swap(true, Ordering::SeqCst) {
        return false;
    }
    wake();
    watchdog();
    true
}

/// Installs the handler (once per process): `wake` runs on the handler's own thread at the first signal – it must get
/// the event loop to end the show (an `EventLoopProxy::send_event`).
pub fn install(wake: impl Fn() + Send + 'static) -> Result<()> {
    let stopping = Arc::new(AtomicBool::new(false));
    ctrlc::set_handler(move || {
        tracing::info!("stop signal received");
        on_signal(&stopping, &wake, || {
            std::thread::spawn(|| {
                std::thread::sleep(GRACE);
                tracing::error!("stop: the show did not end within {:?} – leaving", GRACE);
                std::process::exit(0);
            });
        });
    })
    .context("the stop-signal handler")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn the_first_signal_wakes_the_loop_and_arms_the_watchdog_a_repeat_does_nothing() {
        let stopping = AtomicBool::new(false);
        let (woke, armed) = (Cell::new(0), Cell::new(0));
        assert!(on_signal(&stopping, || woke.set(woke.get() + 1), || armed.set(armed.get() + 1)));
        assert!(!on_signal(&stopping, || woke.set(woke.get() + 1), || armed.set(armed.get() + 1)));
        assert_eq!((woke.get(), armed.get()), (1, 1));
    }

    #[test]
    fn the_grace_fits_the_kiosks_stop() {
        // deck-kiosk.service: TimeoutStopSec=5 – the audio's fade (≤ 0.55 s) and the outputs' blackout fit well inside
        assert!(GRACE < Duration::from_secs(5) && GRACE > Duration::from_secs(1));
    }
}
