//! The 512-channel universe the adapter owns: channel ranges set at once or faded, frames rate-limited to 40 Hz and
//! sent only when something changed, plus a keep-alive frame every second (it also finds a pulled USB cable: the
//! write fails). The clock is passed in, so the tests run without sleeping.

use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use protocol::DmxMessage;

use crate::widget::UNIVERSE;

/// The shortest gap between two frames (40 Hz; the widget itself keeps refreshing the DMX line in between).
pub const FRAME_INTERVAL: Duration = Duration::from_millis(25);
/// A frame at least this often even when nothing changed.
pub const KEEPALIVE: Duration = Duration::from_secs(1);

/// A message's range checked: `(first channel index, values)`.
pub fn check(msg: &DmxMessage) -> Result<(usize, &[u8])> {
    let ch = msg.ch as usize;
    if !(1..=UNIVERSE).contains(&ch) {
        bail!("dmx channel {ch} is outside 1…512");
    }
    if ch - 1 + msg.values.len() > UNIVERSE {
        bail!("dmx channels {ch}…{} run past 512", ch - 1 + msg.values.len());
    }
    Ok((ch - 1, &msg.values))
}

struct Fade {
    index: usize,
    from: u8,
    to: u8,
    start: Instant,
    duration: Duration,
}

impl Fade {
    fn value(&self, now: Instant) -> u8 {
        let t = (now.saturating_duration_since(self.start).as_secs_f64() / self.duration.as_secs_f64()).min(1.0);
        (f64::from(self.from) + (f64::from(self.to) - f64::from(self.from)) * t).round() as u8
    }
}

pub struct Universe {
    values: [u8; UNIVERSE],
    fades: Vec<Fade>,
    dirty: bool,
    last_sent: Option<Instant>,
}

impl Default for Universe {
    fn default() -> Self {
        Universe { values: [0; UNIVERSE], fades: Vec::new(), dirty: false, last_sent: None }
    }
}

impl Universe {
    /// Set a range (at once, or faded from the current values over `fade` ms). A set channel stops its running fade.
    pub fn apply(&mut self, msg: &DmxMessage, now: Instant) -> Result<()> {
        let (first, values) = check(msg)?;
        self.advance(now);
        let fade = Duration::from_millis(msg.fade.unwrap_or(0));
        for (i, &to) in values.iter().enumerate() {
            let index = first + i;
            self.fades.retain(|f| f.index != index);
            let from = self.values[index];
            if fade.is_zero() || from == to {
                self.dirty |= from != to;
                self.values[index] = to;
            } else {
                self.fades.push(Fade { index, from, to, start: now, duration: fade });
                self.dirty = true;
            }
        }
        Ok(())
    }

    /// The blackout: every channel 0, every fade dropped.
    pub fn blackout(&mut self) {
        self.values = [0; UNIVERSE];
        self.fades.clear();
        self.dirty = true;
    }

    fn advance(&mut self, now: Instant) {
        for f in &self.fades {
            self.values[f.index] = f.value(now);
        }
        self.fades.retain(|f| now.saturating_duration_since(f.start) < f.duration);
    }

    /// When the next frame is due: 25 ms after the last one while something changes, else the keep-alive.
    pub fn next_due(&self, now: Instant) -> Instant {
        let Some(last) = self.last_sent else { return now };
        last + if self.dirty || !self.fades.is_empty() { FRAME_INTERVAL } else { KEEPALIVE }
    }

    /// The frame to send now, if one is due.
    pub fn frame(&mut self, now: Instant) -> Option<[u8; UNIVERSE]> {
        (now >= self.next_due(now)).then(|| self.force(now))
    }

    /// The frame to send now regardless of the rate (the blackout, a reconnected widget).
    pub fn force(&mut self, now: Instant) -> [u8; UNIVERSE] {
        self.advance(now);
        self.dirty = false;
        self.last_sent = Some(now);
        self.values
    }

    pub fn values(&self) -> &[u8; UNIVERSE] {
        &self.values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ch: u64, values: &[u8], fade: Option<u64>) -> DmxMessage {
        DmxMessage { ch, values: values.to_vec(), fade }
    }

    #[test]
    fn ranges_are_checked() {
        assert!(check(&set(0, &[1], None)).is_err());
        assert!(check(&set(513, &[1], None)).is_err());
        assert!(check(&set(511, &[1, 2, 3], None)).is_err());
        assert_eq!(check(&set(510, &[1, 2, 3], None)).unwrap().0, 509);
        assert!(check(&set(1, &[0; 512], None)).is_ok());
    }

    #[test]
    fn sends_only_on_change_at_most_40_hz_with_a_keepalive() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut u = Universe::default();
        assert!(u.frame(t0).is_some(), "the first frame goes at once");
        assert!(u.frame(ms(30)).is_none(), "nothing changed");

        u.apply(&set(3, &[200, 100], None), ms(40)).unwrap();
        let f = u.frame(ms(40)).expect("changed, last frame 40 ms ago");
        assert_eq!(&f[..5], &[0, 0, 200, 100, 0]);

        u.apply(&set(1, &[9], None), ms(50)).unwrap();
        assert!(u.frame(ms(50)).is_none(), "10 ms after the last frame: rate-limited");
        assert!(u.frame(ms(64)).is_none());
        assert_eq!(u.frame(ms(65)).expect("25 ms after")[0], 9);

        // an unchanged value is no change
        u.apply(&set(1, &[9], None), ms(100)).unwrap();
        assert!(u.frame(ms(100)).is_none());
        assert_eq!(u.next_due(ms(100)), ms(1_065), "keep-alive a second after the last frame");
        assert!(u.frame(ms(1_064)).is_none());
        assert!(u.frame(ms(1_065)).is_some());
    }

    #[test]
    fn fades_from_the_current_value() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut u = Universe::default();
        u.apply(&set(1, &[100], None), t0).unwrap();
        u.force(t0);
        u.apply(&set(1, &[200, 50], Some(1_000)), ms(0)).unwrap();
        assert_eq!(u.frame(ms(25)).unwrap()[..2], [103, 1]);
        assert_eq!(u.frame(ms(500)).unwrap()[..2], [150, 25]);
        // a new set on channel 2 stops its fade there; channel 1 fades on
        u.apply(&set(2, &[0], None), ms(600)).unwrap();
        assert_eq!(u.frame(ms(600)).unwrap()[..2], [160, 0]);
        assert_eq!(u.frame(ms(1_000)).unwrap()[..2], [200, 0]);
        assert!(u.frame(ms(1_030)).is_none(), "the fade is done: back to the keep-alive");
    }

    #[test]
    fn blackout_zeroes_and_stops_fades() {
        let t0 = Instant::now();
        let mut u = Universe::default();
        u.apply(&set(10, &[255; 20], Some(5_000)), t0).unwrap();
        u.apply(&set(1, &[255], None), t0).unwrap();
        u.blackout();
        assert_eq!(u.force(t0 + Duration::from_millis(1)), [0; UNIVERSE]);
        assert_eq!(u.force(t0 + Duration::from_secs(3)), [0; UNIVERSE], "no fade comes back");
    }
}
