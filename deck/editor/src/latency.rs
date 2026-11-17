//! How far a held panel trails the pointer (`DECK_LATENCY=1`, printed at exit): from the newest pointer event a frame
//! draws to its present, and to the acquire that shows the frame left the display queue (with a queue of `n` frames, the
//! acquire after a present frees the frame `n - 1` presents back – it is on screen).
use std::collections::VecDeque;
use std::time::Instant;

#[derive(Default)]
pub struct Probe {
    on: bool,
    newest: Option<Instant>,
    used: Option<Instant>,
    queued: VecDeque<Option<Instant>>,
    to_present: Vec<f32>,
    to_screen: Vec<f32>,
    waits: Vec<f32>,
}

fn since(t: Instant) -> f32 {
    t.elapsed().as_secs_f32() * 1000.0
}

impl Probe {
    pub fn from_env() -> Probe {
        Probe { on: std::env::var_os("DECK_LATENCY").is_some(), ..Probe::default() }
    }

    /// a pointer event while a panel is held
    pub fn event(&mut self, held: bool) {
        if self.on && held {
            self.newest = Some(Instant::now());
        }
    }

    /// a frame starts: the newest event not drawn yet is its
    pub fn frame(&mut self) {
        self.used = self.newest.take();
    }

    pub fn presented(&mut self) {
        if !self.on {
            return;
        }
        if let Some(t) = self.used {
            self.to_present.push(since(t));
        }
        self.queued.push_back(self.used.take());
    }

    /// an acquire begun at `started` returned, the surface queueing `latency` frames
    pub fn acquired(&mut self, started: Instant, latency: usize) {
        if !self.on {
            return;
        }
        self.waits.push(since(started));
        while self.queued.len() >= latency.max(1) {
            if let Some(Some(t)) = self.queued.pop_front() {
                self.to_screen.push(since(t));
            }
        }
    }

    pub fn summary(&self) -> Option<String> {
        if !self.on || self.to_present.is_empty() {
            return None;
        }
        let stat = |v: &[f32]| {
            let mut s = v.to_vec();
            s.sort_by(f32::total_cmp);
            (s.iter().sum::<f32>() / s.len().max(1) as f32, s.get(s.len() / 2).copied().unwrap_or(0.0), s.get(s.len() * 95 / 100).copied().unwrap_or(0.0))
        };
        let (pa, pm, p95) = stat(&self.to_present);
        let (sa, sm, s95) = stat(&self.to_screen);
        let (wa, ..) = stat(&self.waits);
        Some(format!(
            "deck-editor latency: drag frames={} event->present avg={pa:.1} med={pm:.1} p95={p95:.1} ms; event->on screen avg={sa:.1} med={sm:.1} p95={s95:.1} ms; acquire wait avg={wa:.1} ms",
            self.to_present.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_counts_on_screen_once_the_queue_has_moved_past_it() {
        let mut p = Probe { on: true, ..Probe::default() };
        p.event(true);
        p.frame();
        p.presented();
        p.acquired(Instant::now(), 2);
        assert!(p.to_screen.is_empty(), "with two queued it is still waiting");
        p.frame();
        p.presented();
        p.acquired(Instant::now(), 2);
        assert_eq!(p.to_screen.len(), 1);
        p.event(false);
        p.frame();
        assert!(p.used.is_none(), "an event with no panel held is not counted");
    }
}
