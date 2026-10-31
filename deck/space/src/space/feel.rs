//! Feelings (#188, owner 2026-10-30: "add a few mappings for certain emotions … quite arbitrary for now; the main thing
//! is that we get a change. It has to become more exploratory again" – and then: the complete mood set, every mood
//! reachable). Two lists in the scene's `[being]` table, so a being preset (#191) carries them:
//!
//! - `[[being.feel]]` – feedback → emotion: a rule reads a source (`node.port`, a node named by its id or else by its
//!   kind – `fft.bass`, `fm.index`, `lfo.shape` –, or `bpm` · `touch` · `energy` · `phrase`) and tests it (`above` /
//!   `below` – both = between –, `rising` / `falling` = the value jumps that far over its last half second, `steady` =
//!   it moved less than `by` for that many seconds); true for `for` seconds, it fires (with `chance`) and its mood shows
//!   at once, for at least `hold` s. The heaviest `weight` among the firing rules wins (once the shown one has had
//!   MIN_SHOW); a rule shown for MAX_SHOW s, or one that stops, rests for `cooldown` s – so the moods keep turning. No
//!   rule firing = the energy picks (being.rs).
//! - `[[being.react]]` – emotion → sound: when the head turns to `mood`, `set` (an unlinked control input) is kicked
//!   `by` an amount or `to` a value over `attack` s and eases back over `release` s – a hand on the input meanwhile
//!   becomes the new resting value. Several kicks on one input add up; the input's range bounds them.
//!
//! The conductor's mirrored beings (decks.rs) run the conductor's `feel` rules on what their decks stream (their
//! nudges stay with their decks); a rule whose source a graph lacks stays quiet. `step` and `push` never allocate.
use super::values::{self, Knobs, Path, Values};
use crate::graph::{Graph, PortType};
use crate::params::{self, Spec};
use crate::tempo;
use serde::Deserialize;

/// a rule's mood shows at most this long in one go before the rule rests (s)
pub const MAX_SHOW: f32 = 6.0;
/// a rule's mood shows at least this long before a heavier one may take over (s) – a sprite you can see
pub const MIN_SHOW: f32 = 0.6;
/// the `rising` / `falling` reference: the source smoothed over this long (s)
const SLOW: f32 = 0.5;
/// `touch` falls from 1 to 0 over this long after the hand lets go of a button (s)
const TOUCH_FADE: f32 = 0.8;

/// One `[[being.feel]]` rule as the scene writes it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Rule {
    pub mood: String,
    /// the source: `node.port` (the node by id, else the first of that kind), `bpm`, `touch`, `energy`, `phrase`
    pub when: String,
    pub above: Option<f32>,
    pub below: Option<f32>,
    pub rising: Option<f32>,
    pub falling: Option<f32>,
    /// seconds the source stays within `by`
    pub steady: Option<f32>,
    pub by: f32,
    /// seconds the test must hold before the rule fires
    #[serde(rename = "for")]
    pub after: f32,
    pub hold: f32,
    pub weight: f32,
    /// 0..1: the odds that it fires when its test comes true
    pub chance: f32,
    pub cooldown: f32,
}

impl Default for Rule {
    fn default() -> Self {
        Rule { mood: String::new(), when: String::new(), above: None, below: None, rising: None, falling: None, steady: None, by: 0.03, after: 0.0, hold: 1.5, weight: 1.0, chance: 1.0, cooldown: 4.0 }
    }
}

/// One `[[being.react]]` nudge as the scene writes it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Nudge {
    pub mood: String,
    /// an unlinked control input, `node.port` (by id, else by kind)
    pub set: String,
    pub by: Option<f32>,
    pub to: Option<f32>,
    pub attack: f32,
    pub release: f32,
}

impl Default for Nudge {
    fn default() -> Self {
        Nudge { mood: String::new(), set: String::new(), by: None, to: None, attack: 0.1, release: 2.0 }
    }
}

/// where a rule's value comes from
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Source {
    /// a control input: node, its name, the default
    Input(usize, &'static str, f32),
    Output(usize, usize),
    /// an audio output's block peak
    Meter(usize, usize),
    Shape(usize),
    /// the tempo clock's BPM (0 while it stands)
    Bpm,
    /// 1 while a hand turns a button, falling to 0 after
    Touch,
    /// the head's energy (being.rs)
    Energy,
    /// where in an 8-bar phrase the clock is: 0..8 (0 while it stands)
    Phrase,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Test {
    Range(f32, f32),
    Rising(f32),
    Falling(f32),
    /// seconds, tolerance
    Steady(f32, f32),
}

/// `when` → its source in `graph`
pub fn source(graph: &Graph, when: &str) -> Result<Source, String> {
    Ok(match when {
        "bpm" | "tempo.bpm" => Source::Bpm,
        "touch" => Source::Touch,
        "energy" => Source::Energy,
        "phrase" => Source::Phrase,
        _ => match values::resolve_loose(graph, when).ok_or_else(|| format!("no '{when}' in this graph"))? {
            Path::Input(n, k) => {
                let kind = graph.nodes[n].kind;
                let name = kind.inputs()[k].name;
                Source::Input(n, name, params::default(kind, name))
            }
            Path::Output(n, k) if graph.nodes[n].kind.outputs()[k].ty == PortType::Audio => Source::Meter(n, k),
            Path::Output(n, k) => Source::Output(n, k),
            Path::Shape(n) => Source::Shape(n),
        },
    })
}

/// a rule's test: exactly one of above / below (both = between), rising, falling, steady
pub fn test(r: &Rule) -> Result<Test, String> {
    let range = (r.above.is_some() || r.below.is_some()).then(|| Test::Range(r.above.unwrap_or(f32::NEG_INFINITY), r.below.unwrap_or(f32::INFINITY)));
    let tests = [range, r.rising.map(Test::Rising), r.falling.map(Test::Falling), r.steady.map(|s| Test::Steady(s, r.by.abs()))];
    let mut set = tests.into_iter().flatten();
    match (set.next(), set.next()) {
        (Some(t), None) => Ok(t),
        (None, _) => Err(format!("{}: no test (above, below, rising, falling or steady)", r.mood)),
        _ => Err(format!("{}: one test per rule", r.mood)),
    }
}

struct Live {
    mood: usize,
    when: String,
    src: Source,
    test: Test,
    after: f32,
    hold: f32,
    weight: f32,
    chance: f32,
    cooldown: f32,
    slow: f32,
    reference: f32,
    still: f32,
    on: f32,
    armed: bool,
    lucky: bool,
    /// its test is true (past `for`, and lucky) this frame
    firing: bool,
    until: f32,
    rest: f32,
}

struct Kick {
    mood: usize,
    target: usize,
    by: Option<f32>,
    to: Option<f32>,
    attack: f32,
    release: f32,
    /// when it was kicked (the feeling's clock); -inf = never
    at: f32,
}

impl Kick {
    /// 0 → 1 over the attack, then eased back to 0 over the release
    fn env(&self, t: f32) -> f32 {
        let e = t - self.at;
        if !e.is_finite() || e < 0.0 {
            return 0.0;
        }
        if e < self.attack {
            return e / self.attack.max(1e-3);
        }
        let r = 1.0 - (e - self.attack) / self.release.max(1e-3);
        if r <= 0.0 {
            0.0
        } else {
            r * r * (3.0 - 2.0 * r)
        }
    }
}

struct Target {
    node: usize,
    port: usize,
    name: &'static str,
    spec: Spec,
    /// the value it rests at (the scene's, or where a hand left it)
    base: f32,
    /// what the nudges wrote last; NaN = they leave it alone
    written: f32,
}

/// What a frame's rules hear besides the graph's values.
pub struct Senses<'a> {
    pub energy: f32,
    pub touching: bool,
    pub beat: &'a tempo::Now,
}

/// A being's compiled rules and nudges with their state.
pub struct Feel {
    rules: Vec<Live>,
    kicks: Vec<Kick>,
    targets: Vec<Target>,
    /// the rule whose mood the head shows, and since when
    pub active: Option<usize>,
    since: f32,
    t: f32,
    touch: f32,
    rng: u32,
    /// rules and nudges left out, and why (the HUD / the log)
    pub problems: Vec<String>,
}

/// the palette index of `mood`, added when it is new
fn mood_index(moods: &mut Vec<String>, mood: &str) -> usize {
    match moods.iter().position(|m| m == mood) {
        Some(i) => i,
        None => {
            moods.push(mood.to_string());
            moods.len() - 1
        }
    }
}

impl Feel {
    /// `rules` and `nudges` compiled against `graph`; every mood they name joins `moods` (the head's palette – its
    /// sprites). What doesn't resolve is left out with its reason in `problems`.
    pub fn new(graph: &Graph, rules: &[Rule], nudges: &[Nudge], moods: &mut Vec<String>, seed: u32) -> Feel {
        let mut problems = Vec::new();
        let mut live = Vec::new();
        for r in rules {
            if r.mood.is_empty() {
                problems.push(format!("a rule without a mood ({})", r.when));
                continue;
            }
            match (source(graph, &r.when), test(r)) {
                (Ok(src), Ok(test)) => live.push(Live {
                    mood: mood_index(moods, &r.mood),
                    when: r.when.clone(),
                    src,
                    test,
                    after: r.after.max(0.0),
                    hold: r.hold.max(0.0),
                    weight: r.weight,
                    chance: r.chance,
                    cooldown: r.cooldown.max(0.0),
                    slow: f32::NAN,
                    reference: 0.0,
                    still: 0.0,
                    on: 0.0,
                    armed: true,
                    lucky: false,
                    firing: false,
                    until: f32::NEG_INFINITY,
                    rest: f32::NEG_INFINITY,
                }),
                (Err(e), _) => problems.push(format!("{}: {e}", r.mood)),
                (_, Err(e)) => problems.push(e),
            }
        }
        let mut targets: Vec<Target> = Vec::new();
        let mut kicks = Vec::new();
        for n in nudges {
            let at = match values::resolve_loose(graph, &n.set) {
                Some(Path::Input(node, port)) if graph.link_into(node, port).is_none() => (node, port),
                Some(Path::Input(..)) => {
                    problems.push(format!("{}: {} is linked – a nudge needs an unlinked input", n.mood, n.set));
                    continue;
                }
                _ => {
                    problems.push(format!("{}: no control input '{}'", n.mood, n.set));
                    continue;
                }
            };
            if n.by.is_some() == n.to.is_some() {
                problems.push(format!("{}: a nudge has `by` or `to`", n.mood));
                continue;
            }
            let target = match targets.iter().position(|t| (t.node, t.port) == at) {
                Some(i) => i,
                None => {
                    let kind = graph.nodes[at.0].kind;
                    let name = kind.inputs()[at.1].name;
                    targets.push(Target { node: at.0, port: at.1, name, spec: params::spec(kind, name), base: 0.0, written: f32::NAN });
                    targets.len() - 1
                }
            };
            kicks.push(Kick { mood: mood_index(moods, &n.mood), target, by: n.by, to: n.to, attack: n.attack.max(0.0), release: n.release.max(0.0), at: f32::NEG_INFINITY });
        }
        Feel { rules: live, kicks, targets, active: None, since: 0.0, t: 0.0, touch: 0.0, rng: seed | 1, problems }
    }

    /// how many rules / nudges are in play
    pub fn counts(&self) -> (usize, usize) {
        (self.rules.len(), self.kicks.len())
    }

    /// the active rule's source (the HUD's `MOOD furious < fm.index`)
    pub fn because(&self) -> Option<&str> {
        self.active.map(|a| self.rules[a].when.as_str())
    }

    /// how many nudges move the graph right now
    pub fn nudging(&self) -> usize {
        self.kicks.iter().filter(|k| k.env(self.t) > 0.0).count()
    }

    fn roll(&mut self) -> f32 {
        // xorshift32: deterministic per being, no allocation
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    /// One frame: every rule's test on `vals` and `senses`; returns the mood (palette index) of the rule that wins, if
    /// any.
    pub fn step(&mut self, dt: f32, vals: &dyn Values, senses: &Senses) -> Option<usize> {
        self.t += dt;
        let t = self.t;
        self.touch = if senses.touching { 1.0 } else { (self.touch - dt / TOUCH_FADE).max(0.0) };
        for i in 0..self.rules.len() {
            let v = read(self.rules[i].src, vals, senses, self.touch);
            let r = &mut self.rules[i];
            if r.slow.is_nan() {
                (r.slow, r.reference) = (v, v);
            }
            let hit = match r.test {
                Test::Range(lo, hi) => v > lo && v < hi,
                Test::Rising(x) => v - r.slow > x,
                Test::Falling(x) => r.slow - v > x,
                Test::Steady(secs, by) => {
                    if (v - r.reference).abs() > by {
                        (r.reference, r.still) = (v, 0.0);
                    } else {
                        r.still += dt;
                    }
                    r.still >= secs
                }
            };
            r.slow += (v - r.slow) * (dt / SLOW).min(1.0);
            r.firing = false;
            if !hit {
                (r.on, r.armed) = (0.0, true);
                continue;
            }
            r.on += dt;
            if r.on < r.after {
                continue;
            }
            if r.armed {
                r.armed = false;
                let chance = r.chance;
                let lucky = chance >= 1.0 || self.roll() < chance;
                self.rules[i].lucky = lucky;
            }
            let r = &mut self.rules[i];
            if r.lucky {
                (r.firing, r.until) = (true, t + r.hold);
            }
        }
        let eligible = |r: &Live| (r.firing || t < r.until) && t >= r.rest;
        if let Some(a) = self.active {
            let r = &mut self.rules[a];
            if !eligible(r) || t - self.since > MAX_SHOW {
                (r.rest, r.until) = (t + r.cooldown, f32::NEG_INFINITY);
                self.active = None;
            }
        }
        let mut best: Option<(usize, f32)> = None;
        for (i, r) in self.rules.iter().enumerate() {
            if eligible(r) && best.map_or(true, |(_, w)| r.weight > w) {
                best = Some((i, r.weight));
            }
        }
        match (self.active, best) {
            // a heavier rule takes over; the one it pushes aside rests
            (Some(a), Some((b, w))) if b != a && w > self.rules[a].weight && t - self.since >= MIN_SHOW => {
                let r = &mut self.rules[a];
                (r.rest, r.until) = (t + r.cooldown, f32::NEG_INFINITY);
                (self.active, self.since) = (Some(b), t);
            }
            (None, Some((b, _))) => (self.active, self.since) = (Some(b), t),
            _ => {}
        }
        self.active.map(|a| self.rules[a].mood)
    }

    /// The head turned to `mood`: its nudges kick.
    pub fn turned(&mut self, mood: usize) {
        let t = self.t;
        for k in self.kicks.iter_mut().filter(|k| k.mood == mood) {
            k.at = t;
        }
    }

    /// The nudges into the graph: every kicked input at its resting value + the kicks, bounded by its range; an input
    /// whose kicks are over gets its resting value back once and is left alone.
    pub fn push(&mut self, ctl: &mut dyn Knobs) {
        let t = self.t;
        for (i, tg) in self.targets.iter_mut().enumerate() {
            let now = ctl.input(tg.node, tg.name, tg.spec.default);
            // nothing written yet, or a hand moved it since: that is where it rests
            if tg.written.is_nan() || (now - tg.written).abs() > 1e-6 * (1.0 + now.abs()) {
                tg.base = now;
            }
            let (mut add, mut on) = (0.0, false);
            for k in self.kicks.iter().filter(|k| k.target == i) {
                let e = k.env(t);
                if e > 0.0 {
                    on = true;
                    add += match (k.by, k.to) {
                        (Some(by), _) => by * e,
                        (None, Some(to)) => (to - tg.base) * e,
                        _ => 0.0,
                    };
                }
            }
            if !on {
                if !tg.written.is_nan() {
                    ctl.set(tg.node, tg.port, tg.base);
                    tg.written = f32::NAN;
                }
                continue;
            }
            let (lo, hi) = (tg.spec.min.min(tg.spec.max), tg.spec.min.max(tg.spec.max));
            let v = (tg.base + add).clamp(lo, hi);
            ctl.set(tg.node, tg.port, v);
            tg.written = v;
        }
    }
}

/// a source's value now
fn read(src: Source, vals: &dyn Values, s: &Senses, touch: f32) -> f32 {
    let beat = s.beat;
    match src {
        Source::Input(n, name, d) => vals.input(n, name, d),
        Source::Output(n, p) => vals.output(n, p),
        Source::Meter(n, p) => vals.meter(n, p),
        Source::Shape(n) => vals.lfo_shape(n).unwrap_or(0) as f32,
        Source::Bpm => {
            if beat.running {
                beat.bpm as f32
            } else {
                0.0
            }
        }
        Source::Touch => touch,
        Source::Energy => s.energy,
        Source::Phrase => {
            if beat.running {
                beat.pos.bar.rem_euclid(8) as f32 + beat.pos.bar_phase as f32
            } else {
                0.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;
    use crate::space::values::{Local, Mirror, Table};

    const SCENE: &str = "[[node]]\nid=\"space\"\nkind=\"space\"\n[[node]]\nid=\"voice\"\nkind=\"fm\"\nindex=2\n\
        [[node]]\nid=\"sway\"\nkind=\"lfo\"\nrate=0.5\n[[node]]\nid=\"fft\"\nkind=\"fft\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\n\
        [[link]]\nfrom=\"voice\"\nto=\"fft.in\"\n[[link]]\nfrom=\"voice\"\nto=\"out.in\"\n[[link]]\nfrom=\"fft.level\"\nto=\"space.pulse\"\n";

    fn graph() -> Graph {
        Graph::build(&Scene::parse(SCENE).unwrap()).unwrap()
    }

    fn rule(text: &str) -> Rule {
        toml::from_str(text).unwrap()
    }

    fn still() -> tempo::Now {
        tempo::Now::default()
    }

    #[test]
    fn rules_parse_with_their_defaults_and_resolve_by_id_or_kind() {
        let r = rule("mood = \"furious\"\nwhen = \"fm.index\"\nabove = 4\nweight = 3\n");
        assert_eq!((r.hold, r.cooldown, r.chance, r.after, r.weight), (1.5, 4.0, 1.0, 0.0, 3.0));
        assert_eq!(test(&r), Ok(Test::Range(4.0, f32::INFINITY)));
        let g = graph();
        let voice = g.nodes.iter().position(|n| n.id == "voice").unwrap();
        assert_eq!(source(&g, "fm.index"), Ok(Source::Input(voice, "index", params::default(crate::graph::Kind::Fm, "index"))), "by kind");
        assert_eq!(source(&g, "voice.index"), source(&g, "fm.index"), "by id");
        assert!(matches!(source(&g, "fft.bass"), Ok(Source::Output(_, 1))));
        assert!(matches!(source(&g, "voice.out"), Ok(Source::Meter(_, 0))), "an audio output = its peak");
        assert!(matches!(source(&g, "lfo.shape"), Ok(Source::Shape(_))));
        assert_eq!(source(&g, "tempo.bpm"), Ok(Source::Bpm), "the clock, also on a mirror without a tempo node");
        assert!(source(&g, "organ.gain").is_err());
        let between = rule("mood = \"x\"\nwhen = \"phrase\"\nabove = 3\nbelow = 3.5\n");
        assert_eq!(test(&between), Ok(Test::Range(3.0, 3.5)));
        assert!(test(&rule("mood = \"x\"\nwhen = \"bpm\"\n")).is_err(), "no test");
        assert!(test(&rule("mood = \"x\"\nwhen = \"bpm\"\nabove = 1\nrising = 2\n")).is_err(), "two tests");
        assert!(toml::from_str::<Rule>("mood = \"x\"\nwhen = \"bpm\"\nabove = 1\nwhenever = 2\n").is_err(), "unknown keys are refused");
        // what doesn't resolve is left out with its reason; the moods join the palette
        let mut moods = vec!["neutral".to_string()];
        let rules = [r, rule("mood = \"sad\"\nwhen = \"nobody.gain\"\nbelow = 1\n")];
        let nudges: Vec<Nudge> = ["mood = \"furious\"\nset = \"space.hue\"\nto = 0.0\n", "mood = \"happy\"\nset = \"space.pulse\"\nby = 1\n", "mood = \"dizzy\"\nset = \"sway.rate\"\nby = 2\n"].iter().map(|t| toml::from_str(t).unwrap()).collect();
        let f = Feel::new(&g, &rules, &nudges, &mut moods, 1);
        assert_eq!(f.counts(), (1, 2));
        assert_eq!(f.problems.len(), 2, "{:?}", f.problems);
        assert!(f.problems.iter().any(|p| p.contains("linked")));
        assert_eq!(moods, ["neutral", "furious", "dizzy"]);
    }

    /// a step, a ramp, silence: rising fires on the jump, above while high, steady once nothing moves; the heaviest
    /// wins, a long-shown rule rests and lets another through
    #[test]
    fn rules_fire_on_their_tests_and_the_heaviest_wins() {
        let g = graph();
        let rules: Vec<Rule> = [
            "mood = \"surprised\"\nwhen = \"fft.bass\"\nrising = 0.2\nweight = 5\nhold = 1\n",
            "mood = \"furious\"\nwhen = \"fft.level\"\nabove = 0.7\nweight = 2\n",
            "mood = \"thinking\"\nwhen = \"fft.level\"\nsteady = 2\nby = 0.02\n",
        ]
        .iter()
        .map(|t| rule(t))
        .collect();
        let mut moods = Vec::new();
        let mut f = Feel::new(&g, &rules, &[], &mut moods, 7);
        assert_eq!(moods, ["surprised", "furious", "thinking"]);
        let mut m = Mirror::new(&g);
        let beat = still();
        let s = Senses { energy: 0.0, touching: false, beat: &beat };
        let run = |f: &mut Feel, m: &Mirror, secs: f32| {
            let mut last = None;
            for _ in 0..(secs * 60.0) as usize {
                last = f.step(1.0 / 60.0, m, &s);
            }
            last
        };
        assert_eq!(run(&mut f, &m, 1.0), None, "a second of silence is nothing yet");
        assert_eq!(run(&mut f, &m, 2.0), Some(2), "two seconds of it: thinking");
        assert_eq!(f.because(), Some("fft.level"));
        m.apply(&g, "fft.bass", &[0.6]);
        m.apply(&g, "fft.level", &[0.8]);
        assert_eq!(run(&mut f, &m, 0.1), Some(0), "the bass jumps: surprised, the heaviest");
        assert_eq!(run(&mut f, &m, 1.5), Some(1), "the jump is over, it stays loud: furious");
        assert_eq!(run(&mut f, &m, MAX_SHOW), Some(2), "shown long enough, it rests – the level stands still: thinking");
        assert_eq!(run(&mut f, &m, 4.0), Some(1), "after its cooldown the heavier one is back");
    }

    #[test]
    fn a_chance_below_one_fires_sometimes() {
        let g = graph();
        let r = rule("mood = \"laughing\"\nwhen = \"fft.level\"\nabove = 0.5\nchance = 0.5\nhold = 0\ncooldown = 0\n");
        let mut moods = Vec::new();
        let mut f = Feel::new(&g, &[r], &[], &mut moods, 99);
        let (mut on, mut off) = (Mirror::new(&g), Mirror::new(&g));
        on.apply(&g, "fft.level", &[1.0]);
        let beat = still();
        let s = Senses { energy: 0.0, touching: false, beat: &beat };
        let mut fired = 0;
        for _ in 0..200 {
            if f.step(1.0 / 60.0, &on, &s).is_some() {
                fired += 1;
            }
            f.step(1.0 / 60.0, &off, &s);
            let _ = &mut off;
        }
        assert!((40..160).contains(&fired), "about half the onsets: {fired}");
    }

    /// a nudge kicks its input over the attack, eases back to where it rests, then leaves it alone; a hand on the input
    /// meanwhile becomes the new resting value; the range bounds it
    #[test]
    fn nudges_kick_and_ease_back() {
        let g = graph();
        let voice = g.nodes.iter().position(|n| n.id == "voice").unwrap();
        let space = g.nodes.iter().position(|n| n.id == "space").unwrap();
        let index = g.nodes[voice].kind.input("index").unwrap();
        let nudges: Vec<Nudge> = ["mood = \"furious\"\nset = \"fm.index\"\nby = 3\nattack = 0.1\nrelease = 1\n", "mood = \"furious\"\nset = \"space.hue\"\nto = 0.0\nattack = 0.1\nrelease = 1\n", "mood = \"happy\"\nset = \"space.glow\"\nby = 9\nrelease = 1\n"]
            .iter()
            .map(|t| toml::from_str(t).unwrap())
            .collect();
        let mut moods = vec!["neutral".to_string(), "happy".to_string()];
        let mut f = Feel::new(&g, &[], &nudges, &mut moods, 3);
        assert_eq!(moods, ["neutral", "happy", "furious"]);
        let mut ctl = Table::new(&g);
        let beat = still();
        let s = Senses { energy: 0.0, touching: false, beat: &beat };
        let frames = |f: &mut Feel, ctl: &mut Table, n: usize| {
            for _ in 0..n {
                let v = Local { ctl, meter: &|_, _| 0.0 };
                f.step(1.0 / 60.0, &v, &s);
                f.push(ctl);
            }
        };
        frames(&mut f, &mut ctl, 10);
        assert_eq!(ctl.input(voice, "index", -1.0), 2.0, "untouched until a mood turns");
        f.turned(2);
        frames(&mut f, &mut ctl, 7);
        assert!((ctl.input(voice, "index", -1.0) - 5.0).abs() < 0.1, "kicked by 3: {}", ctl.input(voice, "index", -1.0));
        assert!(ctl.input(space, "hue", -1.0) < 0.05, "to red");
        assert_eq!(f.nudging(), 2);
        frames(&mut f, &mut ctl, 30);
        let mid = ctl.input(voice, "index", -1.0);
        assert!(mid > 2.0 && mid < 5.0, "easing back: {mid}");
        frames(&mut f, &mut ctl, 60);
        assert_eq!(ctl.input(voice, "index", -1.0), 2.0, "back where it rests");
        assert_eq!((ctl.input(space, "hue", -1.0), f.nudging()), (0.52, 0));
        // a hand during a kick: its value is the new rest
        f.turned(2);
        frames(&mut f, &mut ctl, 10);
        ctl.set(voice, index, 7.0);
        frames(&mut f, &mut ctl, 120);
        assert_eq!(ctl.input(voice, "index", -1.0), 7.0);
        // bounded by the range: glow 0..2
        f.turned(1);
        frames(&mut f, &mut ctl, 8);
        assert_eq!(ctl.input(space, "glow", -1.0), 2.0);
    }

    #[test]
    fn touch_and_the_clock_are_sources() {
        let g = graph();
        let rules: Vec<Rule> = ["mood = \"smug\"\nwhen = \"touch\"\nabove = 0.5\nweight = 3\nhold = 0\n", "mood = \"laughing\"\nwhen = \"bpm\"\nabove = 120\n", "mood = \"denial\"\nwhen = \"phrase\"\nabove = 7\nhold = 0\n"].iter().map(|t| rule(t)).collect();
        let mut moods = Vec::new();
        let mut f = Feel::new(&g, &rules, &[], &mut moods, 5);
        let m = Mirror::new(&g);
        let fast = tempo::Now { bpm: 140.0, pos: tempo::position(1.0, 4), per_bar: 4, running: true };
        let last_bar = tempo::Now { bpm: 90.0, pos: tempo::position(7.5 * 4.0, 4), per_bar: 4, running: true };
        assert_eq!(f.step(0.016, &m, &Senses { energy: 0.0, touching: true, beat: &fast }), Some(0), "a hand on a button");
        // the hand lets go: the touch fades, the clock's rule comes through
        let mut last = None;
        for _ in 0..90 {
            last = f.step(0.016, &m, &Senses { energy: 0.0, touching: false, beat: &fast });
        }
        assert_eq!(last, Some(1));
        let mut f = Feel::new(&g, &rules, &[], &mut moods, 5);
        assert_eq!(f.step(0.016, &m, &Senses { energy: 0.0, touching: false, beat: &last_bar }), Some(2), "the phrase's last bar");
    }
}
