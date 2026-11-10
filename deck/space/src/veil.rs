//! A scene change through black (#250, #473): the picture fades out, the deck loads the new show behind the black, it
//! fades in once the show has what it waits for. Platform-free: the time goes in, the black's opacity and "load now" come
//! out – the native deck's switcher and a show's piece (piece.rs `Side`) on every deck go through it (`Change`: one
//! change runs, what is asked for meanwhile waits).

/// seconds the picture takes to fade to black and back (the native deck's audio fades out as long)
pub const FADE_OUT: f32 = 0.25;
pub const FADE_IN: f32 = 0.4;

/// What the transition asks of the deck on a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    None,
    /// black now: build the new show (once)
    Load,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Out,
    Hold,
    In,
}

/// One change: `to` is what loads (the deck's own word for it – a name, a file, a scene's text), `veil` the black's opacity.
#[derive(Debug, Clone)]
pub struct Transition {
    pub to: String,
    pub veil: f32,
    phase: Phase,
}

impl Transition {
    pub fn new(to: &str) -> Transition {
        Transition { to: to.to_owned(), veil: 0.0, phase: Phase::Out }
    }

    /// One frame of `dt` seconds; `ready` = the loaded show waits for nothing. Returns true when the veil is gone.
    pub fn step(&mut self, dt: f32, ready: bool) -> (Step, bool) {
        match self.phase {
            Phase::Out => {
                self.veil = (self.veil + dt / FADE_OUT).min(1.0);
                if self.veil >= 1.0 {
                    self.phase = Phase::Hold;
                    return (Step::Load, false);
                }
            }
            Phase::Hold => {
                if ready {
                    self.phase = Phase::In;
                }
            }
            Phase::In => {
                self.veil = (self.veil - dt / FADE_IN).max(0.0);
                return (Step::None, self.veil <= 0.0);
            }
        }
        (Step::None, false)
    }
}

/// A deck's scene changes: one runs at a time; what is asked for while it runs waits for it, and the newest wins (a
/// show's next cell while the last one still fades).
#[derive(Debug, Clone, Default)]
pub struct Change {
    now: Option<Transition>,
    next: Option<String>,
}

impl Change {
    /// a change to `to` is asked for (the deck's own word for it, `Transition::to`)
    pub fn ask(&mut self, to: &str) {
        self.next = Some(to.to_owned());
    }

    pub fn running(&self) -> bool {
        self.now.is_some()
    }

    /// what was asked for, once no change runs: the deck checks it and `start`s it, or says why not
    pub fn due(&mut self) -> Option<String> {
        self.next.take_if(|_| self.now.is_none())
    }

    pub fn start(&mut self, to: &str) {
        self.now = Some(Transition::new(to));
    }

    /// One frame of `dt` seconds (`ready` as `Transition::step`): what the deck loads now (once, at full black), and
    /// the black's opacity while a change runs.
    pub fn step(&mut self, dt: f32, ready: bool) -> (Option<String>, Option<f32>) {
        let Some(t) = self.now.as_mut() else { return (None, None) };
        let (step, done) = t.step(dt, ready);
        let out = ((step == Step::Load).then(|| t.to.clone()), Some(t.veil));
        if done {
            self.now = None;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_asked_for_while_one_runs_waits_and_the_newest_wins() {
        let mut c = Change::default();
        assert_eq!((c.due(), c.step(0.1, true)), (None, (None, None)));
        c.ask("cathedral");
        let to = c.due().unwrap();
        c.start(&to);
        // two more are asked for while it fades: they wait, and the first of them is never loaded
        c.ask("madness");
        c.ask("outlet");
        assert_eq!((c.due(), c.running()), (None, true));
        let mut loaded = vec![];
        for _ in 0..600 {
            if let Some(to) = c.due() {
                c.start(&to);
            }
            loaded.extend(c.step(1.0 / 60.0, true).0);
        }
        assert_eq!(loaded, ["cathedral", "outlet"]);
        assert!(!c.running());
    }

    #[test]
    fn a_transition_goes_black_loads_once_then_clears() {
        let mut t = Transition::new("madness");
        let mut loads = 0;
        let mut frames = 0;
        let mut peak = 0.0f32;
        loop {
            // not ready for 10 frames after the load
            let (step, done) = t.step(1.0 / 60.0, loads > 0 && frames > 40);
            loads += (step == Step::Load) as u32;
            peak = peak.max(t.veil);
            frames += 1;
            assert!(frames < 600, "never ends");
            if done {
                break;
            }
        }
        assert_eq!((loads, peak), (1, 1.0));
        assert_eq!(t.veil, 0.0);
    }
}
