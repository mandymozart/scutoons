//! The preloader's decisions (#391; was the preloader thread's loop in deck-native's live/mod.rs): a `media.preload` is
//! fetched at once, what is missing is tried again on a schedule (2, 5, 10, 20, then every 30 s), and the HUD says how far
//! it is. A sans-IO state machine on the adapter's clock (`now`: the time since its start); the adapter does the fetching
//! (`Media::preload`) and the answering (`media.ready`).
use crate::recompose::POLL;
use protocol::{MediaPreload, MediaReady};
use std::time::Duration;

/// The retries of a preload with something missing (s): quick at first, then every 30 s.
pub const RETRY_S: [u64; 5] = [2, 5, 10, 20, 30];

/// The retry wait after `attempt` tries (s).
pub fn retry_s(attempt: usize) -> u64 {
    RETRY_S[attempt.min(RETRY_S.len() - 1)]
}

/// The newest preload, how often it was tried and when it is tried next.
#[derive(Debug, Clone, Default)]
pub struct Preload {
    job: Option<MediaPreload>,
    attempt: usize,
    due: Option<Duration>,
}

impl Preload {
    /// how long to wait for the next job at `now`: until the retry is due, at most `POLL`
    pub fn wait(&self, now: Duration) -> Duration {
        self.due.map_or(POLL, |d| d.saturating_sub(now).min(POLL))
    }

    /// a new preload replaces the one on and is tried at once
    pub fn job(&mut self, job: MediaPreload, now: Duration) {
        self.job = Some(job);
        self.attempt = 0;
        self.due = Some(now);
    }

    /// the preload to fetch now (its first try or a retry that is due)
    pub fn due(&self, now: Duration) -> Option<MediaPreload> {
        match (&self.job, self.due) {
            (Some(p), Some(at)) if now >= at => Some(p.clone()),
            _ => None,
        }
    }

    /// the HUD while it fetches: `LOADING n ITEMS`
    pub fn loading_line(&self) -> String {
        format!("LOADING {} ITEMS", self.job.as_ref().map_or(0, |p| p.items.len()))
    }

    /// The fetch answered `ready`: the HUD's line (`n / m READY`, or the first missing one, its reason and when it is
    /// tried again), and the next try is scheduled when something is missing.
    pub fn done(&mut self, ready: &MediaReady, now: Duration) -> String {
        let total = self.job.as_ref().map_or(0, |p| p.items.len());
        let wait = retry_s(self.attempt);
        let line = match ready.missing.first() {
            None => format!("{} / {total} READY", ready.ready.len()),
            Some(id) => {
                let why = ready.why.as_ref().and_then(|w| w.get(id)).cloned().unwrap_or_default();
                format!("{} / {total} READY - {id}: {why} - AGAIN IN {wait} S", ready.ready.len())
            }
        };
        self.due = (!ready.missing.is_empty()).then(|| now + Duration::from_secs(wait));
        self.attempt += 1;
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const T0: Duration = Duration::from_secs(50);

    fn preload(n: usize) -> MediaPreload {
        let items: Vec<_> = (0..n).map(|i| serde_json::json!({ "id": format!("m{i}"), "url": format!("https://cdn.test/m{i}.webp") })).collect();
        serde_json::from_value(serde_json::json!({ "outlet": "left", "items": items })).unwrap()
    }

    fn ready(ok: &[&str], missing: &[(&str, &str)]) -> MediaReady {
        MediaReady {
            outlet: "left".into(),
            ready: ok.iter().map(|s| s.to_string()).collect(),
            missing: missing.iter().map(|m| m.0.to_string()).collect(),
            why: (!missing.is_empty()).then(|| missing.iter().map(|m| (m.0.to_string(), m.1.to_string())).collect::<HashMap<_, _>>()),
        }
    }

    #[test]
    fn a_job_is_tried_at_once_and_a_complete_answer_ends_it() {
        let mut p = Preload::default();
        assert_eq!((p.due(T0), p.wait(T0)), (None, POLL));
        p.job(preload(2), T0);
        assert_eq!(p.due(T0).map(|j| j.items.len()), Some(2));
        assert_eq!(p.loading_line(), "LOADING 2 ITEMS");
        assert_eq!(p.done(&ready(&["m0", "m1"], &[]), T0), "2 / 2 READY");
        assert_eq!((p.due(T0 + Duration::from_secs(99)), p.wait(T0)), (None, POLL), "nothing is missing: no retry");
    }

    #[test]
    fn what_is_missing_is_tried_again_on_the_schedule_and_a_new_job_starts_over() {
        let mut p = Preload::default();
        p.job(preload(3), T0);
        let miss = ready(&["m0"], &[("m1", "HTTP 404"), ("m2", "x")]);
        assert_eq!(p.done(&miss, T0), "1 / 3 READY - m1: HTTP 404 - AGAIN IN 2 S");
        assert_eq!(p.due(T0 + Duration::from_millis(1999)), None);
        assert_eq!(p.wait(T0 + Duration::from_millis(1950)), Duration::from_millis(50));
        let mut at = T0 + Duration::from_secs(2);
        assert!(p.due(at).is_some());
        // 5, 10, 20, 30, then every 30 s
        for want in [5u64, 10, 20, 30, 30, 30] {
            assert!(p.done(&miss, at).ends_with(&format!("AGAIN IN {want} S")));
            assert_eq!(p.due(at + Duration::from_secs(want) - Duration::from_millis(1)), None);
            at += Duration::from_secs(want);
            assert!(p.due(at).is_some());
        }
        // a new preload starts at the first wait, and a reason that is not known is empty
        p.job(preload(1), at);
        assert_eq!(p.done(&ready(&[], &[("m0", "")]), at), "0 / 1 READY - m0:  - AGAIN IN 2 S");
        assert_eq!(retry_s(99), 30);
    }
}
