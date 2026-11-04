//! The mood's energy ladder (#367) – one walk for every deck: the native being's head (space/being.rs `step_head`), the
//! browser deck (space-web `DeckState`, no GPU) and the headless host. The energy follows the sound's level, the clock's
//! tempo and a hand on the deck – quick up, slow down; a new rung once it has held `HOLD`. A platform passes the inputs
//! each frame and shows the mood; it decides nothing.
//!
//! - `Energy` – the walk itself over `n` rungs (the being's `look.moods`), with an override (a feeling, another deck's
//!   stream) that shows at once.
//! - `Ladder` – a deck's moods from its character's portraits (the ladder's moods it has, else all it has), the walk over
//!   them, a pinned mood (the creator's mood chip), the picker's list and which portrait a mood shows.

/// the moods from calm to wild when a scene or a character names none (the sprites' `<char>_portrait-<mood>`)
pub const MOODS: [&str; 5] = ["neutral", "thinking", "happy", "laughing", "surprised"];
/// how long a new rung must hold before it shows (s)
pub const HOLD: f32 = 1.2;
/// a frame's step at most (s): a stalled page or a hitch is not a long climb
pub const MAX_DT: f32 = 0.1;

/// The energy's walk: 0..1 smoothed from the inputs, the rung it shows, the rung it asks for and since when.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Energy {
    /// 0..1: the level, the tempo and the hand, smoothed
    pub energy: f32,
    /// the rung shown
    pub rung: usize,
    candidate: usize,
    held: f32,
}

impl Energy {
    /// What the energy heads for: the level (0..1, × 1.3, the most), a running clock's tempo (60 → 160 bpm adds up to
    /// 0.2) and a hand on the deck (0..1 adds up to 0.25).
    pub fn target(level: f32, bpm: f32, running: bool, touch: f32) -> f32 {
        let level = if level.is_finite() { level.clamp(0.0, 1.0) } else { 0.0 };
        let tempo = if running && bpm.is_finite() { ((bpm - 60.0) / 100.0).clamp(0.0, 1.0) } else { 0.0 };
        let touch = if touch.is_finite() { touch.clamp(0.0, 1.0) } else { 0.0 };
        (0.8 * (level * 1.3).min(1.0) + 0.2 * tempo + 0.25 * touch).clamp(0.0, 1.0)
    }

    /// One frame of `dt` s over `n` rungs: the energy eases toward `target` (`Energy::target`; up in ~0.3 s, down in
    /// ~1.5 s); the rung it asks for – `over` (a feeling, a fixed mood) else the energy's – shows once it has held
    /// `HOLD`, an `over` at once. Returns the new rung when it turned.
    pub fn step(&mut self, target: f32, dt: f32, n: usize, over: Option<usize>) -> Option<usize> {
        let dt = if dt.is_finite() { dt.clamp(0.0, MAX_DT) } else { 0.0 };
        // quick up, slow down: a crescendo shows at once, the calm comes back gently
        let k = if target > self.energy { dt / 0.3 } else { dt / 1.5 };
        self.energy += (target - self.energy) * k.min(1.0);
        let n = n.max(1);
        let want = over.unwrap_or(((self.energy * (n - 1) as f32).round() as usize).min(n - 1));
        if want != self.candidate {
            (self.candidate, self.held) = (want, 0.0);
        }
        self.held += dt;
        if self.candidate != self.rung && (self.held >= HOLD || over.is_some()) {
            self.rung = self.candidate;
            return Some(self.rung);
        }
        None
    }
}

/// A deck's mood: the ladder its character's portraits give, the energy's walk over it, a pinned mood.
#[derive(Debug, Clone, PartialEq)]
pub struct Ladder {
    /// the rungs, calm → wild
    rungs: Vec<String>,
    /// every mood the character has a portrait for
    portraits: Vec<String>,
    walk: Energy,
    pinned: Option<String>,
}

impl Default for Ladder {
    fn default() -> Self {
        Ladder::of_portraits(&[])
    }
}

impl Ladder {
    /// A ladder over these rungs as they are (a scene's `[being] moods`); none = the first of MOODS.
    pub fn new(rungs: Vec<String>) -> Ladder {
        let rungs = if rungs.is_empty() { vec![MOODS[0].to_string()] } else { rungs };
        Ladder { portraits: rungs.clone(), rungs, walk: Energy::default(), pinned: None }
    }

    /// The ladder a character's portraits give: the moods of MOODS it has (in that order, two at least), else every one it
    /// has, else the first of MOODS.
    pub fn of_portraits(portraits: &[String]) -> Ladder {
        let mut l = Ladder::new(Vec::new());
        l.set_portraits(portraits);
        l
    }

    /// Another character's portraits: the ladder anew (the walk and a pinned mood stay).
    pub fn set_portraits(&mut self, portraits: &[String]) {
        let mut have: Vec<String> = Vec::new();
        for p in portraits {
            if !have.contains(p) {
                have.push(p.clone());
            }
        }
        let ladder: Vec<String> = MOODS.iter().filter(|m| have.iter().any(|p| p == *m)).map(|m| m.to_string()).collect();
        self.rungs = if ladder.len() >= 2 {
            ladder
        } else if !have.is_empty() {
            have.clone()
        } else {
            vec![MOODS[0].to_string()]
        };
        self.portraits = have;
        self.walk.rung = self.walk.rung.min(self.rungs.len() - 1);
    }

    /// Hold the mood on `mood` (shown at once, whatever the energy), or None: the energy walks it again.
    pub fn pin(&mut self, mood: Option<&str>) {
        self.pinned = mood.filter(|m| !m.is_empty()).map(String::from);
    }

    pub fn pinned(&self) -> Option<&str> {
        self.pinned.as_deref()
    }

    /// One frame: the level (0..1), the clock's tempo, a hand on the deck (0..1), `dt` s → the mood now.
    pub fn step(&mut self, level: f32, bpm: f32, running: bool, touch: f32, dt: f32) -> &str {
        self.walk.step(Energy::target(level, bpm, running, touch), dt, self.rungs.len(), None);
        self.mood()
    }

    /// the mood shown: the pinned one, else the walk's rung
    pub fn mood(&self) -> &str {
        match &self.pinned {
            Some(m) => m,
            None => &self.rungs[self.walk.rung.min(self.rungs.len() - 1)],
        }
    }

    /// the energy now (0..1)
    pub fn energy(&self) -> f32 {
        self.walk.energy
    }

    pub fn rungs(&self) -> &[String] {
        &self.rungs
    }

    /// The moods on the wire (the being's palette): the ladder first, then every other portrait, then a pinned mood
    /// the two lack.
    pub fn wire(&self) -> Vec<String> {
        let mut out = self.rungs.clone();
        for m in self.portraits.iter().chain(self.pinned.iter()) {
            if !out.contains(m) {
                out.push(m.clone());
            }
        }
        out
    }

    /// The moods a picker offers: the calm one first, then the portraits' in name order.
    pub fn choices(&self) -> Vec<String> {
        let mut rest: Vec<String> = self.portraits.iter().filter(|m| *m != MOODS[0]).cloned().collect();
        rest.sort();
        rest.dedup();
        std::iter::once(MOODS[0].to_string()).chain(rest).collect()
    }

    /// The portrait a mood shows: its own, else the calm one's, else any; None without portraits.
    pub fn face(&self, mood: &str) -> Option<&str> {
        let find = |m: &str| self.portraits.iter().find(|p| *p == m).map(String::as_str);
        find(mood).or_else(|| find(MOODS[0])).or_else(|| self.portraits.first().map(String::as_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// silence is the first rung; a loud, fast deck climbs to the last – a rung only once it has held
    #[test]
    fn the_energy_climbs_quickly_and_holds_each_rung() {
        let mut e = Energy::default();
        for _ in 0..60 {
            assert_eq!(e.step(Energy::target(0.0, 0.0, false, 0.0), 1.0 / 60.0, 5, None), None);
        }
        assert_eq!((e.rung, e.energy), (0, 0.0));
        for _ in 0..20 {
            e.step(Energy::target(1.0, 170.0, true, 0.0), 1.0 / 60.0, 5, None);
        }
        assert_eq!(e.rung, 0, "a third of a second is not yet a mood");
        for _ in 0..240 {
            e.step(Energy::target(1.0, 170.0, true, 0.0), 1.0 / 60.0, 5, None);
        }
        assert!(e.energy > 0.95, "{}", e.energy);
        assert_eq!(e.rung, 4);
        assert_eq!(e.step(Energy::target(1.0, 170.0, true, 0.0), 1.0 / 60.0, 5, Some(1)), Some(1), "an override shows at once");
    }

    #[test]
    fn the_target_weighs_level_tempo_and_hand() {
        assert_eq!(Energy::target(0.0, 160.0, true, 0.0), 0.2);
        assert_eq!(Energy::target(0.0, 160.0, false, 0.0), 0.0, "a stopped clock adds nothing");
        assert_eq!(Energy::target(0.0, 0.0, false, 1.0), 0.25);
        assert_eq!(Energy::target(1.0, 160.0, true, 1.0), 1.0);
        assert_eq!(Energy::target(f32::NAN, f32::NAN, true, f32::NAN), 0.0);
        let mut e = Energy::default();
        e.step(Energy::target(1.0, 0.0, false, 0.0), 5.0, 5, None);
        assert!((e.energy - 0.8 * MAX_DT / 0.3).abs() < 1e-6, "a long frame counts as MAX_DT: {}", e.energy);
    }

    #[test]
    fn a_ladder_from_the_portraits() {
        assert_eq!(Ladder::of_portraits(&names(&["neutral"])).rungs(), names(&["neutral"]));
        assert_eq!(Ladder::of_portraits(&names(&["sad", "happy", "neutral", "surprised"])).rungs(), names(&["neutral", "happy", "surprised"]));
        assert_eq!(Ladder::of_portraits(&names(&MOODS)).rungs(), names(&MOODS));
        assert_eq!(Ladder::of_portraits(&names(&["sad", "angry"])).rungs(), names(&["sad", "angry"]), "all it has");
        assert_eq!(Ladder::of_portraits(&[]).rungs(), names(&["neutral"]));
        let l = Ladder::of_portraits(&names(&["sad", "happy", "neutral", "surprised"]));
        assert_eq!(l.wire(), names(&["neutral", "happy", "surprised", "sad"]), "the ladder first, then the rest");
        assert_eq!(l.choices(), names(&["neutral", "happy", "sad", "surprised"]));
        assert_eq!((l.face("sad"), l.face("angry")), (Some("sad"), Some("neutral")));
        assert_eq!(Ladder::of_portraits(&names(&["sad"])).face("happy"), Some("sad"));
        assert_eq!(Ladder::of_portraits(&[]).face("happy"), None);
    }

    #[test]
    fn a_pinned_mood_shows_at_once_and_lets_go() {
        let mut l = Ladder::of_portraits(&names(&MOODS));
        for _ in 0..300 {
            l.step(1.0, 140.0, true, 0.0, 1.0 / 60.0);
        }
        assert_eq!(l.mood(), "surprised");
        l.pin(Some("thinking"));
        assert_eq!(l.step(1.0, 140.0, true, 0.0, 1.0 / 60.0), "thinking");
        l.pin(None);
        assert_eq!(l.step(1.0, 140.0, true, 0.0, 1.0 / 60.0), "surprised", "the walk went on underneath");
        l.pin(Some("grumpy"));
        assert_eq!(l.wire().last().map(String::as_str), Some("grumpy"), "a pinned mood is on the wire");
    }
}
