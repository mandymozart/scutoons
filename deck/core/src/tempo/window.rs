//! The show clock's error window (#387; the page's `ClockWindow` of #321 before): how far off the show's line this deck
//! played over the last minute – the sound report's measurement without a microphone, the same numbers and words on every
//! deck. The controller feeds it once per frame while it follows the show's line (`Controller::frame`): it decides which
//! frames count (a follower on a running line, from its first snap onto that line on – the frames before it are the deck
//! before the line, minutes off); the window keeps them for `SPAN_S` and answers their spread. Platform-free and on the
//! caller's time (`t_s`), so a virtual clock replays it exactly.
use std::collections::VecDeque;

/// the window's span (s): the last minute
pub const SPAN_S: f64 = 60.0;

/// The error's spread over the window (ms): its size's p50 / p95 / max, its signed mean, the frames counted and the
/// seconds they span.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    pub n: usize,
    pub p50: f64,
    pub p95: f64,
    pub max: f64,
    pub mean: f64,
    pub seconds: f64,
}

/// The frames' errors of the last `span_s` (time s, error ms).
#[derive(Debug, Clone)]
pub struct SyncWindow {
    samples: VecDeque<(f64, f64)>,
    span_s: f64,
}

impl Default for SyncWindow {
    fn default() -> Self {
        SyncWindow::new(SPAN_S)
    }
}

impl SyncWindow {
    pub fn new(span_s: f64) -> SyncWindow {
        SyncWindow { samples: VecDeque::new(), span_s }
    }

    /// a counted frame at `t_s`, `error_ms` off the show's line; the ones older than the span go
    pub fn add(&mut self, t_s: f64, error_ms: f64) {
        self.samples.push_back((t_s, error_ms));
        while self.samples.front().is_some_and(|&(t, _)| t < t_s - self.span_s) {
            self.samples.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// the spread of the frames in the window; None before the first
    pub fn summary(&self) -> Option<Summary> {
        let n = self.samples.len();
        let (first, last) = (self.samples.front()?.0, self.samples.back()?.0);
        let mut size: Vec<f64> = self.samples.iter().map(|&(_, e)| e.abs()).collect();
        size.sort_by(f64::total_cmp);
        let at = |q: f64| size[((q * n as f64) as usize).min(n - 1)];
        let mean = self.samples.iter().map(|&(_, e)| e).sum::<f64>() / n as f64;
        Some(Summary { n, p50: at(0.5), p95: at(0.95), max: size[n - 1], mean, seconds: last - first })
    }

    /// the window in words – the sound report's and the HUD's
    pub fn line(&self) -> String {
        match self.summary() {
            None => "no minute of following yet".into(),
            Some(s) => format!(
                "last {:.0} s |show| p50 {:.1} · p95 {:.1} · max {:.1} ms (mean {:+.1}, {} frames)",
                s.seconds, s.p50, s.p95, s.max, s.mean, s.n
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spread_of_scripted_errors() {
        let mut w = SyncWindow::new(60.0);
        assert_eq!(w.summary(), None);
        assert_eq!(w.line(), "no minute of following yet");
        // 100 frames 25 ms apart, sizes 0.1 … 10 ms: the first 30 behind the line (-0.1 … -3.0), the rest ahead
        for k in 0..100 {
            let e = f64::from(k + 1) * 0.1;
            w.add(f64::from(k) * 0.025, if k < 30 { -e } else { e });
        }
        let s = w.summary().unwrap();
        assert_eq!(s.n, 100);
        assert!((s.p50 - 5.1).abs() < 1e-9, "{s:?}");
        assert!((s.p95 - 9.6).abs() < 1e-9, "{s:?}");
        assert!((s.max - 10.0).abs() < 1e-9);
        assert!((s.mean - 4.12).abs() < 1e-9, "(505 - 2 · 46.5) / 100: {s:?}");
        assert!((s.seconds - 2.475).abs() < 1e-9);
        assert_eq!(w.line(), "last 2 s |show| p50 5.1 · p95 9.6 · max 10.0 ms (mean +4.1, 100 frames)");
    }

    #[test]
    fn only_the_last_span_counts() {
        let mut w = SyncWindow::new(60.0);
        w.add(0.0, 40.0);
        for k in 1..=2400 {
            w.add(30.0 + f64::from(k) * 0.025, -1.0);
        }
        let s = w.summary().unwrap();
        assert_eq!(s.n, 2400, "the frame of t = 0 left the window");
        assert_eq!((s.max, s.mean), (1.0, -1.0));
        assert!((s.seconds - 59.975).abs() < 1e-9);
        assert_eq!(w.line(), "last 60 s |show| p50 1.0 · p95 1.0 · max 1.0 ms (mean -1.0, 2400 frames)");
    }
}
