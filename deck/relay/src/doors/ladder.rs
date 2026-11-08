//! #438: the fallback ladder – which transport a client uses, decided without any I/O (RULES A1: this file moves into the
//! sans-IO core client, deck/space/src/relay.rs, on adoption; it sits here so the prototype stays off the default path).
//! Native decks: RUDP → QUIC → WebSocket. Browsers: QUIC → WebSocket. A platform only dials what `poll` says and reports
//! back; retransmission, congestion and crypto stay the libraries'.
//!
//! Dial before hello: only the winner of the climb says `hello`, so a race leaves no ghost client in the session.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rung {
    Rudp,
    Quic,
    WebSocket,
}

/// How the rungs are tried: one after the other, or the next one gets its chance while the one above still tries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Climb {
    InTurn,
    Race,
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub rungs: Vec<Rung>,
    pub climb: Climb,
    /// `InTurn`: the time a rung gets. `Race`: the head start of the rung above.
    pub step: Duration,
    /// nothing arrived for this long (the 1 Hz ping's pong included): the rung is gone
    pub stall: Duration,
    /// how long the relay keeps a silent connection: a `resume` before that finds the old one still there and is refused
    pub relay_idle: Duration,
    /// on a lower rung this long: try the one above in the background
    pub step_up_after: Duration,
}

impl Policy {
    fn with(rungs: Vec<Rung>, climb: Climb) -> Policy {
        let step = match climb {
            Climb::InTurn => Duration::from_secs(1),
            Climb::Race => Duration::from_millis(250),
        };
        Policy { rungs, climb, step, stall: Duration::from_secs(2), relay_idle: Duration::from_secs(3), step_up_after: Duration::from_secs(60) }
    }
    pub fn native(climb: Climb) -> Policy {
        Policy::with(vec![Rung::Rudp, Rung::Quic, Rung::WebSocket], climb)
    }
    pub fn browser(climb: Climb) -> Policy {
        Policy::with(vec![Rung::Quic, Rung::WebSocket], climb)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Dial(Rung),
    Close(Rung),
    /// send `hello` on this rung (with `resume` when the client was in the session before)
    Join { rung: Rung, resume: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// the transport's handshake is done
    Connected(Rung),
    Failed(Rung),
    /// anything arrived on the rung in use
    Inbound,
}

#[derive(Debug)]
enum Phase {
    Climbing { top: usize, next: usize, at: Instant, open: Vec<usize>, resume: bool },
    /// a rung is up; the hello waits until the relay has let the silent connection go
    Holding { rung: usize, join_at: Instant },
    Joined { rung: usize, probe_at: Option<Instant>, probing: Option<(usize, Instant)>, backoff: Duration },
}

pub struct Ladder {
    p: Policy,
    phase: Phase,
    last_inbound: Instant,
}

const MARGIN: Duration = Duration::from_millis(100);
const LONGEST_BACKOFF: Duration = Duration::from_secs(300);

impl Ladder {
    pub fn new(p: Policy, now: Instant) -> Ladder {
        Ladder { p, phase: Phase::Climbing { top: 0, next: 0, at: now, open: vec![], resume: false }, last_inbound: now }
    }

    fn index(&self, r: Rung) -> Option<usize> {
        self.p.rungs.iter().position(|x| *x == r)
    }

    fn joined(&self, rung: usize, now: Instant, backoff: Duration) -> Phase {
        let probe_at = (rung > 0).then_some(now + self.p.step_up_after);
        Phase::Joined { rung, probe_at, probing: None, backoff }
    }

    pub fn on(&mut self, event: Event, now: Instant) -> Vec<Action> {
        let mut out = vec![];
        match event {
            Event::Inbound => self.last_inbound = now,
            Event::Connected(r) => {
                let Some(i) = self.index(r) else { return out };
                match &mut self.phase {
                    Phase::Climbing { open, resume, .. } if open.contains(&i) => {
                        let resume = *resume;
                        for other in open.drain(..).filter(|o| *o != i) {
                            out.push(Action::Close(self.p.rungs[other]));
                        }
                        let free_at = self.last_inbound + self.p.relay_idle + MARGIN;
                        if resume && now < free_at {
                            self.phase = Phase::Holding { rung: i, join_at: free_at };
                        } else {
                            out.push(Action::Join { rung: r, resume });
                            self.last_inbound = now;
                            self.phase = self.joined(i, now, self.p.step_up_after);
                        }
                    }
                    Phase::Joined { rung, probing: Some((p, _)), backoff, .. } if *p == i => {
                        let (old, backoff) = (*rung, *backoff);
                        out.push(Action::Close(self.p.rungs[old]));
                        out.push(Action::Join { rung: r, resume: true });
                        self.last_inbound = now;
                        self.phase = self.joined(i, now, backoff);
                    }
                    _ => out.push(Action::Close(r)),
                }
            }
            Event::Failed(r) => {
                let Some(i) = self.index(r) else { return out };
                match &mut self.phase {
                    Phase::Climbing { open, at, .. } => {
                        open.retain(|o| *o != i);
                        if open.is_empty() {
                            *at = now;
                        }
                    }
                    Phase::Holding { rung, .. } if *rung == i => self.fall(i, now, &mut out),
                    Phase::Joined { rung, .. } if *rung == i => self.fall(i, now, &mut out),
                    Phase::Joined { probing, probe_at, backoff, .. } if probing.is_some_and(|(p, _)| p == i) => {
                        *probing = None;
                        *backoff = (*backoff * 2).min(LONGEST_BACKOFF);
                        *probe_at = Some(now + *backoff);
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// The rung in use is gone: climb on from the one below (the last rung retries itself).
    fn fall(&mut self, rung: usize, now: Instant, out: &mut Vec<Action>) {
        if let Phase::Joined { probing: Some((p, _)), .. } = self.phase {
            out.push(Action::Close(self.p.rungs[p]));
        }
        out.push(Action::Close(self.p.rungs[rung]));
        let top = (rung + 1).min(self.p.rungs.len() - 1);
        self.phase = Phase::Climbing { top, next: top, at: now, open: vec![], resume: true };
    }

    /// What to do at `now`; call it after every event and at `next_wake`.
    pub fn poll(&mut self, now: Instant) -> Vec<Action> {
        let mut out = vec![];
        let give_up = self.p.step * 4;
        let (climb, step, rungs) = (self.p.climb, self.p.step, self.p.rungs.len());
        match &mut self.phase {
            Phase::Climbing { top, next, at, open, .. } => {
                if now >= *at {
                    if *next < rungs {
                        if climb == Climb::InTurn {
                            for o in open.drain(..) {
                                out.push(Action::Close(self.p.rungs[o]));
                            }
                        }
                        open.push(*next);
                        out.push(Action::Dial(self.p.rungs[*next]));
                        *next += 1;
                        *at = now + if *next == rungs { give_up } else { step };
                    } else {
                        // nothing came up: from the top of this climb again
                        for o in open.drain(..) {
                            out.push(Action::Close(self.p.rungs[o]));
                        }
                        *next = *top;
                        *at = now;
                    }
                }
            }
            Phase::Holding { rung, join_at } => {
                if now >= *join_at {
                    let i = *rung;
                    out.push(Action::Join { rung: self.p.rungs[i], resume: true });
                    self.last_inbound = now;
                    self.phase = self.joined(i, now, self.p.step_up_after);
                }
            }
            Phase::Joined { rung, probe_at, probing, backoff } => {
                let r = *rung;
                if now.saturating_duration_since(self.last_inbound) >= self.p.stall {
                    self.fall(r, now, &mut out);
                } else if let Some((p, since)) = *probing {
                    if now.saturating_duration_since(since) >= give_up {
                        out.push(Action::Close(self.p.rungs[p]));
                        *probing = None;
                        *backoff = (*backoff * 2).min(LONGEST_BACKOFF);
                        *probe_at = Some(now + *backoff);
                    }
                } else if probe_at.is_some_and(|t| now >= t) {
                    *probe_at = None;
                    *probing = Some((r - 1, now));
                    out.push(Action::Dial(self.p.rungs[r - 1]));
                }
            }
        }
        out
    }

    /// When `poll` has something to do next.
    pub fn next_wake(&self, now: Instant) -> Instant {
        match &self.phase {
            Phase::Climbing { at, .. } => *at,
            Phase::Holding { join_at, .. } => *join_at,
            Phase::Joined { probe_at, probing, .. } => {
                let stall = self.last_inbound + self.p.stall;
                let probe = probing.map(|(_, since)| since + self.p.step * 4).or(*probe_at);
                probe.map_or(stall, |p| p.min(stall)).max(now)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Action::*;
    use Rung::*;

    fn at(t0: Instant, ms: u64) -> Instant {
        t0 + Duration::from_millis(ms)
    }

    #[test]
    fn the_race_gives_a_blocked_rung_its_head_start_and_no_more() {
        let t0 = Instant::now();
        let mut l = Ladder::new(Policy::native(Climb::Race), t0);
        assert_eq!(l.poll(t0), [Dial(Rudp)]);
        assert_eq!(l.poll(at(t0, 100)), []);
        assert_eq!(l.poll(at(t0, 250)), [Dial(Quic)]);
        assert_eq!(l.on(Event::Connected(Quic), at(t0, 270)), [Close(Rudp), Join { rung: Quic, resume: false }]);
    }

    #[test]
    fn in_turn_closes_a_rung_before_the_next() {
        let t0 = Instant::now();
        let mut l = Ladder::new(Policy::browser(Climb::InTurn), t0);
        assert_eq!(l.poll(t0), [Dial(Quic)]);
        assert_eq!(l.poll(at(t0, 1000)), [Close(Quic), Dial(WebSocket)]);
        assert_eq!(l.on(Event::Connected(WebSocket), at(t0, 1020)), [Join { rung: WebSocket, resume: false }]);
    }

    #[test]
    fn a_failed_rung_does_not_use_up_its_time() {
        let t0 = Instant::now();
        let mut l = Ladder::new(Policy::browser(Climb::InTurn), t0);
        l.poll(t0);
        l.on(Event::Failed(Quic), at(t0, 5));
        assert_eq!(l.poll(at(t0, 5)), [Dial(WebSocket)]);
    }

    #[test]
    fn nothing_up_climbs_again_from_the_top() {
        let t0 = Instant::now();
        let mut l = Ladder::new(Policy::browser(Climb::Race), t0);
        l.poll(t0);
        l.poll(at(t0, 250));
        assert_eq!(l.poll(at(t0, 1250)), [Close(Quic), Close(WebSocket)]);
        assert_eq!(l.poll(at(t0, 1250)), [Dial(Quic)]);
    }

    /// UDP dies mid-show: two silent seconds, the next rung up at once, but the hello waits until the relay has let the old
    /// connection go – a resume before that would be refused and the missed cues lost.
    #[test]
    fn a_stall_steps_down_and_the_resume_waits_for_the_relay() {
        let t0 = Instant::now();
        let mut l = Ladder::new(Policy::native(Climb::Race), t0);
        l.poll(t0);
        assert_eq!(l.on(Event::Connected(Rudp), at(t0, 20)), [Join { rung: Rudp, resume: false }]);
        l.on(Event::Inbound, at(t0, 1000));
        assert_eq!(l.poll(at(t0, 2999)), []);
        assert_eq!(l.poll(at(t0, 3000)), [Close(Rudp)]);
        assert_eq!(l.poll(at(t0, 3000)), [Dial(Quic)]);
        assert_eq!(l.poll(at(t0, 3250)), [Dial(WebSocket)]);
        assert_eq!(l.on(Event::Connected(WebSocket), at(t0, 3270)), [Close(Quic)]);
        assert_eq!(l.poll(at(t0, 4099)), []);
        assert_eq!(l.poll(at(t0, 4100)), [Join { rung: WebSocket, resume: true }]);
    }

    #[test]
    fn the_last_rung_retries_itself() {
        let t0 = Instant::now();
        let mut l = Ladder::new(Policy { rungs: vec![WebSocket], ..Policy::browser(Climb::Race) }, t0);
        l.poll(t0);
        l.on(Event::Connected(WebSocket), at(t0, 10));
        assert_eq!(l.poll(at(t0, 2010)), [Close(WebSocket)]);
        assert_eq!(l.poll(at(t0, 2010)), [Dial(WebSocket)]);
    }

    #[test]
    fn a_lower_rung_probes_the_one_above_and_backs_off_when_it_stays_shut() {
        let t0 = Instant::now();
        let mut l = Ladder::new(Policy::browser(Climb::Race), t0);
        l.poll(t0);
        l.poll(at(t0, 250));
        l.on(Event::Connected(WebSocket), at(t0, 260));
        let keep = |l: &mut Ladder, ms| {
            l.on(Event::Inbound, at(t0, ms));
            l.poll(at(t0, ms))
        };
        assert_eq!(keep(&mut l, 60_259), []);
        assert_eq!(keep(&mut l, 60_260), [Dial(Quic)]);
        assert_eq!(keep(&mut l, 61_260), [Close(Quic)]);
        // 120 s now; nothing is dialled before that
        assert_eq!(keep(&mut l, 62_000), []);
        assert_eq!(keep(&mut l, 181_260), [Dial(Quic)]);
        // it answers: over to the better rung, by resume
        assert_eq!(l.on(Event::Connected(Quic), at(t0, 181_280)), [Close(WebSocket), Join { rung: Quic, resume: true }]);
    }
}
