//! The engine (#211): a `Plan`, the deck's values (one per `Plan::paths`, in order) and the clock → the messages due
//! now. The host calls `step` at the time it returned (the next 40 Hz DMX frame, a flash's send time, a flash's end)
//! and plays what comes out at once – the timing is in when `step` runs, so a host that wakes on time sends on time.
//!
//! - Continuous targets advance on a fixed 40 Hz grid (the universe's frame rate, #187): each frame smooths every
//!   target by 25 ms and sends only what changed at the wire's resolution (a CC's 7 bit, a DMX channel's 8 / 16).
//! - Strobes: the flash goes out its lead before its beat is heard and ends after the flash length (≤ half the period).
//!   A level of 0 or a stopped clock keeps the fixture dark.
//!
//! `step` never allocates: `out` is the host's reused buffer.

use crate::config::{Input, MidiKind, MidiPort, Plan, StrobePlan, Wire};
use crate::shape::{self, Smooth};
use crate::strobe::{Clock, Strobe};

/// The continuous targets' frame (40 Hz – the DMX universe's rate).
pub const FRAME_S: f64 = 0.025;

/// One message to play now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Out {
    Midi { port: MidiPort, bytes: [u8; 3] },
    /// `now`: a flash – into its own frame at once (the widget's urgent path), not the next 40 Hz frame
    Dmx { ch: u16, value: u8, now: bool },
}

/// A message and the moment it was meant for (the host's clock, s) – the host plays it at once; `at` is for the log
/// and the tests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timed {
    pub at: f64,
    pub out: Out,
}

struct Flasher {
    strobe: Strobe,
    /// the flash's end, while one is lit
    off_at: Option<f64>,
}

pub struct Engine {
    plan: Plan,
    smooth: Vec<Smooth>,
    /// the last value sent per target (wire units), None = nothing yet
    sent: Vec<Option<u16>>,
    next_frame: Option<f64>,
    flashers: Vec<Flasher>,
    rested: bool,
}

fn read(input: &Option<Input>, values: &[f32], default: f32) -> f32 {
    input.as_ref().map_or(default, |i| values.get(i.source).map_or(default, |&v| i.shape.map(v)))
}

impl Engine {
    pub fn new(plan: Plan) -> Engine {
        let smooth = plan.targets.iter().map(|t| Smooth::new(t.smooth)).collect();
        let sent = vec![None; plan.targets.len()];
        let flashers = plan.strobes.iter().map(|s| Flasher { strobe: Strobe::new(s.rate), off_at: None }).collect();
        Engine { plan, smooth, sent, next_frame: None, flashers, rested: false }
    }

    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// The messages due at `now` into `out` (appended); returns when to call again. `values[i]` is `plan.paths[i]`'s
    /// value (a missing one counts as 0); `clock` None = no tempo (strobes dark).
    pub fn step(&mut self, now: f64, values: &[f32], clock: Option<&Clock>, out: &mut Vec<Timed>) -> f64 {
        if !self.rested {
            // the strobes' other channels to rest, the flash channels dark
            self.rested = true;
            for s in &self.plan.strobes {
                for &(ch, value) in &s.rest {
                    out.push(Timed { at: now, out: Out::Dmx { ch, value, now: false } });
                }
                out.push(Timed { at: now, out: Out::Dmx { ch: s.flash, value: 0, now: false } });
            }
        }
        let frame = *self.next_frame.get_or_insert(now);
        if now >= frame - 1e-9 {
            self.frame(frame, values, out);
            // on the grid; a host that fell behind (a stall) starts the grid again from now, never a catch-up burst
            let next = frame + FRAME_S;
            self.next_frame = Some(if next < now { now + FRAME_S } else { next });
        }
        let mut wake = self.next_frame.unwrap_or(now + FRAME_S);
        if let Some(clock) = clock {
            for (s, f) in self.plan.strobes.iter().zip(self.flashers.iter_mut()) {
                wake = wake.min(Self::strobe(s, f, now, values, clock, self.plan.lead, out));
            }
        } else {
            for (s, f) in self.plan.strobes.iter().zip(self.flashers.iter_mut()) {
                if let Some(off) = f.off_at {
                    if now >= off - 1e-9 {
                        out.push(Timed { at: off, out: Out::Dmx { ch: s.flash, value: 0, now: true } });
                        f.off_at = None;
                    } else {
                        wake = wake.min(off);
                    }
                }
            }
        }
        wake
    }

    fn frame(&mut self, at: f64, values: &[f32], out: &mut Vec<Timed>) {
        for (k, t) in self.plan.targets.iter().enumerate() {
            let raw = values.get(t.source).copied().unwrap_or(0.0);
            let y = self.smooth[k].step(t.shape.map(raw), FRAME_S as f32);
            let prev = self.sent[k];
            match t.wire {
                Wire::Dmx { ch, fine: None } => {
                    let v = shape::to_8bit(y);
                    if prev != Some(u16::from(v)) {
                        out.push(Timed { at, out: Out::Dmx { ch, value: v, now: false } });
                        self.sent[k] = Some(u16::from(v));
                    }
                }
                Wire::Dmx { ch, fine: Some(fine) } => {
                    let (c, f) = shape::split_16bit(y);
                    let v = u16::from(c) << 8 | u16::from(f);
                    if prev != Some(v) {
                        if prev.map(|p| (p >> 8) as u8) != Some(c) {
                            out.push(Timed { at, out: Out::Dmx { ch, value: c, now: false } });
                        }
                        if prev.map(|p| (p & 0xFF) as u8) != Some(f) {
                            out.push(Timed { at, out: Out::Dmx { ch: fine, value: f, now: false } });
                        }
                        self.sent[k] = Some(v);
                    }
                }
                Wire::Midi { port, channel, kind } => match kind {
                    MidiKind::Cc(cc) => {
                        let v = shape::to_7bit(y);
                        if prev != Some(u16::from(v)) {
                            out.push(Timed { at, out: Out::Midi { port, bytes: shape::cc(channel, cc, v) } });
                            self.sent[k] = Some(u16::from(v));
                        }
                    }
                    MidiKind::Bend => {
                        let v = shape::to_14bit(y);
                        if prev != Some(v) {
                            out.push(Timed { at, out: Out::Midi { port, bytes: shape::bend(channel, v) } });
                            self.sent[k] = Some(v);
                        }
                    }
                    MidiKind::Note { note, velocity } => {
                        let on = u16::from(y >= 0.5);
                        if prev != Some(on) {
                            // nothing sent yet and off: stay quiet (no stray note off at start)
                            if prev.is_some() || on == 1 {
                                let bytes = if on == 1 { shape::note_on(channel, note, velocity) } else { shape::note_off(channel, note) };
                                out.push(Timed { at, out: Out::Midi { port, bytes } });
                            }
                            self.sent[k] = Some(on);
                        }
                    }
                },
            }
        }
    }

    fn strobe(s: &StrobePlan, f: &mut Flasher, now: f64, values: &[f32], clock: &Clock, lead: f64, out: &mut Vec<Timed>) -> f64 {
        let mut wake = f64::INFINITY;
        if let Some(off) = f.off_at {
            if now >= off - 1e-9 {
                out.push(Timed { at: off, out: Out::Dmx { ch: s.flash, value: 0, now: true } });
                f.off_at = None;
            } else {
                wake = off;
            }
        }
        let p = read(&s.rate_in, values, 0.5);
        let Some(next) = f.strobe.peek(clock, p, now, lead) else { return wake };
        if next.at <= now + 1e-9 {
            f.strobe.fired(next.beat);
            let level = (read(&s.level_in, values, 1.0) * f32::from(s.level)).round().clamp(0.0, 255.0) as u8;
            if level > 0 {
                out.push(Timed { at: next.at, out: Out::Dmx { ch: s.flash, value: level, now: true } });
                let off = now + f.strobe.flash_length(s.duration, p, clock);
                f.off_at = Some(off);
                wake = wake.min(off);
            }
            // the one after
            if let Some(n) = f.strobe.peek(clock, p, now, lead) {
                wake = wake.min(n.at);
            }
        } else {
            wake = wake.min(next.at);
        }
        wake
    }

    /// B.6 `panic`: the flashes end, the strobes forget their grid (the host blacks out the devices itself).
    pub fn panic(&mut self) {
        for f in &mut self.flashers {
            f.off_at = None;
            f.strobe.reset();
        }
        for s in &mut self.sent {
            *s = None;
        }
        self.rested = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    fn plan(text: &str) -> Plan {
        let doc: toml::Table = toml::from_str(text).unwrap();
        config::parse(doc["outputs"].as_table().unwrap(), None, &|n| Err(n.to_owned())).unwrap()
    }

    /// Runs the engine as a host would – wake at each returned time – with `values(t)` and the clock; collects all.
    fn run(e: &mut Engine, from: f64, secs: f64, clock: Option<&Clock>, values: impl Fn(f64) -> Vec<f32>) -> Vec<Timed> {
        let mut out = Vec::new();
        let mut now = from;
        while now < from + secs {
            let wake = e.step(now, &values(now), clock, &mut out);
            assert!(wake > now - 1e-9, "the next wake {wake} is not before now {now}");
            now = wake.max(now + 1e-6);
        }
        out
    }

    #[test]
    fn continuous_targets_send_changes_at_40_hz() {
        let mut e = Engine::new(plan(
            "[outputs]\ndmx = \"auto\"\nmidi = \"x\"\n[[outputs.map]]\nfrom = \"a.b\"\nto = [{ dmx = 1 }, { dmx = 3, bits = 16 }, { cc = 7, channel = 2 }, { bend = true }]\n",
        ));
        // a ramp 0 → 1 over one second
        let out = run(&mut e, 10.0, 1.2, None, |t| vec![((t - 10.0) as f32).clamp(0.0, 1.0)]);
        let dmx1: Vec<&Timed> = out.iter().filter(|m| matches!(m.out, Out::Dmx { ch: 1, .. })).collect();
        // at most one per 25 ms frame, and on the 40 Hz grid
        assert!(dmx1.len() <= 41 && dmx1.len() >= 39, "{}", dmx1.len());
        for w in dmx1.windows(2) {
            let d = w[1].at - w[0].at;
            assert!((d / FRAME_S - (d / FRAME_S).round()).abs() < 1e-6 && d >= FRAME_S - 1e-9, "{d}");
        }
        assert_eq!(dmx1.last().unwrap().out, Out::Dmx { ch: 1, value: 255, now: false });
        // 16 bit: coarse on 3, fine on 4 – the last pair is 255 / 255
        let last = |ch: u16| out.iter().rev().find_map(|m| match m.out { Out::Dmx { ch: c, value, .. } if c == ch => Some(value), _ => None });
        assert_eq!((last(3), last(4)), (Some(255), Some(255)));
        // the CC: 7 bit changes only – 128 values at most, channel 2
        let ccs: Vec<[u8; 3]> = out.iter().filter_map(|m| match m.out { Out::Midi { bytes, .. } if bytes[0] == 0xB1 => Some(bytes), _ => None }).collect();
        assert!(ccs.len() <= 41 && ccs.windows(2).all(|w| w[1][2] > w[0][2]), "{ccs:?}");
        assert_eq!(*ccs.last().unwrap(), [0xB1, 7, 127]);
        // pitch bend ends at the top
        let bend = out.iter().rev().find_map(|m| match m.out { Out::Midi { bytes, .. } if bytes[0] == 0xE0 => Some(bytes), _ => None });
        assert_eq!(bend, Some([0xE0, 0x7F, 0x7F]));
        // steady: nothing more goes out
        let mut more = Vec::new();
        e.step(12.0, &[1.0], None, &mut more);
        e.step(12.025, &[1.0], None, &mut more);
        assert!(more.is_empty(), "{more:?}");
    }

    #[test]
    fn smoothing_runs_on_the_frame_grid() {
        let mut e = Engine::new(plan("[outputs]\ndmx = \"auto\"\n[[outputs.map]]\nfrom = \"a.b\"\nsmooth = 0.1\nto = [{ dmx = 1 }]\n"));
        // 0 until 1.0 s, then a step to 1: after 0.1 s (4 frames) ≈ 63 %
        let out = run(&mut e, 0.0, 1.6, None, |t| vec![if t < 1.0 { 0.0 } else { 1.0 }]);
        let at = |t: f64| out.iter().filter(|m| m.at <= t + 1e-9).filter_map(|m| match m.out { Out::Dmx { ch: 1, value, .. } => Some(value), _ => None }).next_back();
        let v = at(1.075).unwrap();
        assert!((155..=165).contains(&v), "{v}");
        // 9 frames (2.25 tau): 89 %, 5 tau: 99 %
        assert!((222..=234).contains(&at(1.2).unwrap()), "{:?}", at(1.2));
        assert!(at(1.5).unwrap() >= 252);
    }

    #[test]
    fn notes_gate_at_the_middle() {
        let mut e = Engine::new(plan("[outputs]\nmidi = \"x\"\n[[outputs.map]]\nfrom = \"a.b\"\nto = [{ note = 36, channel = 10, velocity = 90 }]\n"));
        let out = run(&mut e, 0.0, 1.0, None, |t| vec![if (0.3..0.6).contains(&t) { 0.8 } else { 0.1 }]);
        let notes: Vec<[u8; 3]> = out.iter().filter_map(|m| match m.out { Out::Midi { bytes, .. } => Some(bytes), _ => None }).collect();
        assert_eq!(notes, vec![[0x99, 36, 90], [0x89, 36, 0]], "on at 0.3, off at 0.6 – no stray note off at the start");
    }

    const STROBE: &str = "[outputs]\ndmx = \"auto\"\nlead = 20\n[[outputs.fixture]]\nid = \"s\"\nprofile = \"strobe\"\naddress = 1\n[[outputs.strobe]]\nfixture = \"s\"\nrate = \"r.x\"\nlevel = \"l.x\"\nduration = 30\n";

    fn flashes(out: &[Timed]) -> Vec<(f64, u8)> {
        out.iter().filter_map(|m| match m.out { Out::Dmx { ch: 1, value, now: true } => Some((m.at, value)), _ => None }).collect()
    }

    #[test]
    fn strobe_rests_then_flashes_on_the_grid_ahead_by_the_lead() {
        for bpm in [84.0, 120.0, 174.0] {
            let mut e = Engine::new(plan(STROBE));
            let clock = Clock::new(0.0, 0.0, bpm, 4);
            let out = run(&mut e, 0.1, 4.0, Some(&clock), |_| vec![0.5, 1.0]);
            // the rest values first: rate and duration 0, the flash channel dark
            assert_eq!(out[0].out, Out::Dmx { ch: 3, value: 0, now: false });
            assert_eq!(out[1].out, Out::Dmx { ch: 2, value: 0, now: false });
            assert_eq!(out[2].out, Out::Dmx { ch: 1, value: 0, now: false });
            let f = flashes(&out);
            let ons: Vec<f64> = f.iter().filter(|(_, v)| *v == 255).map(|(t, _)| *t).collect();
            let offs: Vec<f64> = f.iter().filter(|(_, v)| *v == 0).map(|(t, _)| *t).collect();
            assert!(ons.len() >= 5, "{bpm}: {ons:?}");
            for (k, t) in ons.iter().enumerate() {
                // a quarter (neutral): beat k + 1 is heard at (k + 1) × 60 / bpm, sent 20 ms before
                let beat = (k + 1) as f64;
                assert!((t + 0.02 - beat * 60.0 / bpm).abs() < 1e-9, "{bpm}: flash {k} at {t}");
                // and dark 30 ms later
                assert!((offs[k] - t - 0.03).abs() < 1e-9, "{bpm}: off {} after on {t}", offs[k]);
            }
        }
    }

    #[test]
    fn the_rate_parameter_steps_divisions_and_level_zero_is_dark() {
        let clock = Clock::new(0.0, 0.0, 120.0, 4);
        // 1/16 (p = 0.875): every 125 ms; the flash length clamps to ≤ 62.5 ms (30 ms stays)
        let mut e = Engine::new(plan(STROBE));
        let out = run(&mut e, 0.0, 2.0, Some(&clock), |_| vec![0.875, 0.5]);
        let ons: Vec<(f64, u8)> = flashes(&out).into_iter().filter(|(_, v)| *v > 0).collect();
        assert!(ons.iter().all(|(_, v)| *v == 128), "the level parameter scales the flash");
        for w in ons.windows(2) {
            assert!((w[1].0 - w[0].0 - 0.125).abs() < 1e-9);
        }
        // level 0: no flash at all
        let mut e = Engine::new(plan(STROBE));
        let out = run(&mut e, 0.0, 2.0, Some(&clock), |_| vec![0.875, 0.0]);
        assert!(flashes(&out).is_empty());
        // no clock: dark
        let mut e = Engine::new(plan(STROBE));
        let out = run(&mut e, 0.0, 1.0, None, |_| vec![0.5, 1.0]);
        assert!(flashes(&out).is_empty());
    }

    #[test]
    fn panic_ends_a_flash_and_rests_again() {
        let clock = Clock::new(0.0, 0.0, 120.0, 4);
        let mut e = Engine::new(plan(STROBE));
        let mut out = Vec::new();
        let mut now = 0.0;
        // up to just after the first flash
        while now < 0.49 {
            now = e.step(now, &[0.5, 1.0], Some(&clock), &mut out);
        }
        assert!(flashes(&out).iter().any(|(_, v)| *v == 255));
        e.panic();
        out.clear();
        e.step(now, &[0.5, 1.0], Some(&clock), &mut out);
        assert!(out.iter().any(|m| m.out == Out::Dmx { ch: 2, value: 0, now: false }), "rests again");
    }
}
