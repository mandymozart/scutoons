//! controls.rs (#209): what the hands drive – the engine's half of the controls UI; the page (the viewer's /live/deck,
//! deck/space-web's `DeckControls`) and later the stage are only the bridge (DECISIONS 2026-10-31: "the Svelte /live
//! wrapper is only the bridge"). Everything here speaks **paths** (`node.port`, the page's knobs) and **0..1**: the
//! host turns a 0..1 into a value with its own ranges (the page's `toValue`, the stage's params specs).
//!
//! - **The selection** (owner 2026-10-31: "the last clicked / tapped thing controls the x / y sticks … nothing selected →
//!   a master effect section"): the master section, the being, one knob, or an object in the scene (a hook – no object
//!   is pickable yet). It picks what the two sticks drive (`pairs`).
//! - **The sticks** (owner: "mobile-game sticks … two overlays in the left and right corners"): a stick drives two paths
//!   (x, y). Its deflection (−1..1 per axis, y up) moves them from where they were when the thumb landed – relative, so
//!   a value never jumps when a stick appears under the thumb (`stick_down` / `stick_move`).
//! - **The master section**: a master FX chain isn't in the engine yet (live-core has only its output limiter, #188).
//!   The master slots name `master.*` paths first – the moment the engine's master bus exposes them they take over –
//!   and fall back to the deck's look and mix (`space.hue` / `space.glow`, the voice and the drone).
//! - **MIDI learn** (owner: "click a knob, move a controller"): `learn(path)` arms, the next control change binds; after
//!   that the control drives the path. CC (7 bit) and pitch bend (14 bit).
//! - **The preset hook** (#191: "mappings are saved with the deck / the instrument preset"): `to_text` / `load_text` – the
//!   MIDI bindings and the sticks' assignments as plain lines; the browser keeps them per deck in localStorage until a
//!   preset carries them.
//!
//! The event path (`stick_move`, `midi` on a bound control) never allocates; learning and loading do.

/// What the sticks drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// nothing selected: the master section
    Master,
    /// the being (any of its points, its head)
    Being,
    /// one knob, by path
    Knob(String),
    /// an object in the scene, by id (a hook: its `object.<id>.x` / `.y` when the deck has them, else the master)
    Object(String),
}

impl Selection {
    /// `""` master · `being` · `knob:<path>` · `object:<id>` (the names the page and deck-space's `Space` speak)
    pub fn parse(s: &str) -> Selection {
        match s {
            "" | "master" => Selection::Master,
            "being" => Selection::Being,
            _ => match (s.strip_prefix("knob:"), s.strip_prefix("object:")) {
                (Some(p), _) if !p.is_empty() => Selection::Knob(p.into()),
                (_, Some(o)) if !o.is_empty() => Selection::Object(o.into()),
                _ => Selection::Master,
            },
        }
    }

    pub fn name(&self) -> String {
        match self {
            Selection::Master => String::new(),
            Selection::Being => "being".into(),
            Selection::Knob(p) => format!("knob:{p}"),
            Selection::Object(o) => format!("object:{o}"),
        }
    }
}

pub const LEFT: usize = 0;
pub const RIGHT: usize = 1;

/// The being's main pairs, (x, y) per stick (owner 2026-10-31: "left = voice ratio / index, right = cutoff / LFO
/// rate") – the cathedral's; a deck without them takes its first paths two by two.
pub const BEING_PAIRS: [[&str; 2]; 2] = [["voice.ratio", "voice.index"], ["incense.cutoff", "sway.rate"]];

/// The master section's slots, (x, y) per stick: each a list – the first path the deck has wins. `master.*` = the
/// master FX chain to come (filter, drive, delay, reverb – #209's plan); until the engine exposes it, the look and the
/// mix.
pub const MASTER: [[&[&str]; 2]; 2] =
    [[&["master.cutoff", "space.hue"], &["master.drive", "space.glow"]], [&["master.delay", "voice.gain"], &["master.reverb", "drone.gain"]]];

/// A stick's full deflection moves its values this far (0..1): from the middle it reaches both ends
pub const STICK_GAIN: f32 = 0.6;
/// deflection below this is rest (a thumb never lands exactly)
pub const DEAD: f32 = 0.04;

/// A MIDI control bound to a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// 0..15
    pub channel: u8,
    /// the controller number 0..127, or BEND (pitch bend)
    pub control: u16,
    pub path: String,
}

/// a binding's `control` for the pitch bend wheel
pub const BEND: u16 = 0x100;

/// What a MIDI message did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Midi<'a> {
    /// not a control change, or not bound
    Ignored,
    /// MIDI learn bound this control to the path
    Learned(&'a str),
    /// the bound path to this 0..1
    Set(&'a str, f32),
}

#[derive(Debug, Clone, Default)]
struct Stick {
    /// the slots it drives while the thumb is down (taken when it landed – a selection meanwhile waits for the next)
    paths: [Option<usize>; 2],
    start: [f32; 2],
    down: bool,
}

/// The controls' state: the deck's paths, the selection, the sticks, the MIDI bindings.
#[derive(Debug, Clone)]
pub struct Controls {
    /// the paths the deck has (the page's knobs, in its order)
    paths: Vec<String>,
    selection: Selection,
    /// the being's pairs as indices into `paths` (BEING_PAIRS or the first paths two by two)
    being: [[Option<usize>; 2]; 2],
    /// a stick axis assigned by hand (the being's – #209: "what a stick drives is assignable … saved with the preset")
    assigned: [[Option<usize>; 2]; 2],
    sticks: [Stick; 2],
    bindings: Vec<Binding>,
    learning: Option<usize>,
}

impl Controls {
    /// the deck's paths (its knobs)
    pub fn new<S: AsRef<str>>(paths: &[S]) -> Controls {
        let paths: Vec<String> = paths.iter().map(|p| p.as_ref().to_string()).collect();
        let at = |p: &str| paths.iter().position(|q| q == p);
        let preferred = BEING_PAIRS.map(|pair| pair.map(at));
        let being = if preferred.iter().flatten().all(Option::is_some) {
            preferred
        } else {
            let mut it = (0..paths.len()).map(Some).chain(std::iter::repeat(None));
            [[it.next().flatten(), it.next().flatten()], [it.next().flatten(), it.next().flatten()]]
        };
        Controls { paths, selection: Selection::Master, being, assigned: [[None; 2]; 2], sticks: Default::default(), bindings: Vec::new(), learning: None }
    }

    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    fn index(&self, p: &str) -> Option<usize> {
        self.paths.iter().position(|q| q == p)
    }

    fn path(&self, i: Option<usize>) -> &str {
        i.and_then(|i| self.paths.get(i)).map_or("", String::as_str)
    }

    pub fn select(&mut self, s: Selection) {
        // a knob or object the deck doesn't have: the master section
        self.selection = match s {
            Selection::Knob(p) if self.index(&p).is_none() => Selection::Master,
            s => s,
        };
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    /// the being's pairs with the hand-assigned axes over them
    fn being_pairs(&self) -> [[Option<usize>; 2]; 2] {
        let mut out = self.being;
        for (s, axes) in self.assigned.iter().enumerate() {
            for (a, p) in axes.iter().enumerate() {
                if p.is_some() {
                    out[s][a] = *p;
                }
            }
        }
        out
    }

    fn master(&self) -> [[Option<usize>; 2]; 2] {
        MASTER.map(|stick| stick.map(|slot| slot.iter().find_map(|p| self.index(p))))
    }

    /// What the sticks drive now, (x, y) per stick, as indices into `paths`.
    fn slots(&self) -> [[Option<usize>; 2]; 2] {
        let being = self.being_pairs();
        match &self.selection {
            Selection::Master => self.master(),
            Selection::Being => being,
            Selection::Knob(p) => {
                let k = self.index(p);
                // a knob of one of the being's pairs: that pair on the left stick, the other on the right
                if let Some(s) = being.iter().position(|pair| pair.contains(&k)) {
                    return [being[s], being[1 - s]];
                }
                // else the knob on the left stick's y (up / down like the knob), its node's next knob on x; the being's
                // first pair on the right
                let node = p.split_once('.').map_or(p.as_str(), |(n, _)| n);
                let sibling = (0..self.paths.len()).find(|&i| Some(i) != k && self.paths[i].split_once('.').is_some_and(|(n, _)| n == node));
                [[sibling, k], being[0]]
            }
            Selection::Object(o) => {
                let (x, y) = (self.index(&format!("object.{o}.x")), self.index(&format!("object.{o}.y")));
                if x.is_some() || y.is_some() {
                    [[x, y], self.master()[RIGHT]]
                } else {
                    self.master()
                }
            }
        }
    }

    /// stick `side`'s (x, y) paths now ("" = none)
    pub fn stick_paths(&self, side: usize) -> [&str; 2] {
        let s = self.slots()[side.min(1)];
        [self.path(s[0]), self.path(s[1])]
    }

    /// A thumb lands on stick `side`: it drives what the selection says, from the values' 0..1 now (`x`, `y` – the
    /// host reads them for `stick_paths`).
    pub fn stick_down(&mut self, side: usize, x: f32, y: f32) {
        let paths = self.slots()[side.min(1)];
        self.sticks[side.min(1)] = Stick { paths, start: [x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)], down: true };
    }

    /// The thumb's deflection (−1..1 each, y up): the two values' new 0..1 (a slot the stick doesn't drive keeps its
    /// start). Never allocates.
    pub fn stick_move(&self, side: usize, dx: f32, dy: f32) -> [f32; 2] {
        let st = &self.sticks[side.min(1)];
        let shape = |d: f32| {
            let d = d.clamp(-1.0, 1.0);
            if d.abs() < DEAD {
                0.0
            } else {
                d.signum() * (d.abs() - DEAD) / (1.0 - DEAD)
            }
        };
        [(st.start[0] + shape(dx) * STICK_GAIN).clamp(0.0, 1.0), (st.start[1] + shape(dy) * STICK_GAIN).clamp(0.0, 1.0)]
    }

    /// the paths the stick drives while the thumb is down ("" none)
    pub fn stick_driving(&self, side: usize) -> [&str; 2] {
        let st = &self.sticks[side.min(1)];
        [self.path(st.paths[0]), self.path(st.paths[1])]
    }

    pub fn stick_up(&mut self, side: usize) {
        self.sticks[side.min(1)].down = false;
    }

    pub fn stick_is_down(&self, side: usize) -> bool {
        self.sticks[side.min(1)].down
    }

    /// Assign stick `side`'s axis (0 x, 1 y) to `path` for the being ("" = back to the default pair).
    pub fn assign(&mut self, side: usize, axis: usize, path: &str) {
        self.assigned[side.min(1)][axis.min(1)] = self.index(path);
    }

    // ── MIDI learn ──

    /// Arm MIDI learn for `path` ("" or an unknown path disarms): the next control change binds to it.
    pub fn learn(&mut self, path: &str) {
        self.learning = self.index(path);
    }

    pub fn learning(&self) -> Option<&str> {
        self.learning.map(|i| self.paths[i].as_str())
    }

    /// One MIDI message (status, data…): a control change or pitch bend learned or driving its path. Never allocates
    /// unless it learns.
    pub fn midi(&mut self, msg: &[u8]) -> Midi<'_> {
        let (control, norm) = match *msg {
            [s, cc, v, ..] if s & 0xF0 == 0xB0 => (cc as u16 & 0x7F, (v & 0x7F) as f32 / 127.0),
            [s, lo, hi, ..] if s & 0xF0 == 0xE0 => (BEND, (((hi as u16 & 0x7F) << 7) | (lo as u16 & 0x7F)) as f32 / 16383.0),
            _ => return Midi::Ignored,
        };
        let channel = msg[0] & 0x0F;
        if let Some(i) = self.learning.take() {
            let path = self.paths[i].clone();
            // one control drives one path, one path is driven by one control
            self.bindings.retain(|b| !(b.channel == channel && b.control == control) && b.path != path);
            self.bindings.push(Binding { channel, control, path });
            return Midi::Learned(self.paths[i].as_str());
        }
        match self.bindings.iter().find(|b| b.channel == channel && b.control == control) {
            Some(b) => Midi::Set(b.path.as_str(), norm),
            None => Midi::Ignored,
        }
    }

    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// what drives `path`: `CC 7 · 1` (controller · channel 1..16), `BEND · 1`, or None
    pub fn mapped(&self, path: &str) -> Option<String> {
        self.bindings.iter().find(|b| b.path == path).map(|b| match b.control {
            BEND => format!("BEND · {}", b.channel + 1),
            c => format!("CC {c} · {}", b.channel + 1),
        })
    }

    pub fn unmap(&mut self, path: &str) {
        self.bindings.retain(|b| b.path != path);
    }

    // ── the preset hook (#191) ──

    /// The mappings as plain lines – what a preset saves: `cc <channel 1..16> <controller> <path>`, `bend <channel>
    /// <path>`, `stick <left|right> <x|y> <path>`.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for b in &self.bindings {
            match b.control {
                BEND => out.push_str(&format!("bend {} {}\n", b.channel + 1, b.path)),
                c => out.push_str(&format!("cc {} {c} {}\n", b.channel + 1, b.path)),
            }
        }
        for (s, axes) in self.assigned.iter().enumerate() {
            for (a, p) in axes.iter().enumerate() {
                if let Some(p) = p {
                    out.push_str(&format!("stick {} {} {}\n", ["left", "right"][s], ["x", "y"][a], self.paths[*p]));
                }
            }
        }
        out
    }

    /// Load `to_text`'s lines (replacing the mappings); returns the lines it couldn't use (a path the deck doesn't
    /// have, a malformed line) – the host says so (RULES: no silent conversion).
    pub fn load_text(&mut self, text: &str) -> Vec<String> {
        self.bindings.clear();
        self.assigned = [[None; 2]; 2];
        let mut bad = Vec::new();
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let w: Vec<&str> = line.split_whitespace().collect();
            let ok = match w.as_slice() {
                ["cc", ch, cc, path] => match (ch.parse::<u8>(), cc.parse::<u16>(), self.index(path)) {
                    (Ok(ch @ 1..=16), Ok(cc @ 0..=127), Some(_)) => {
                        self.bindings.push(Binding { channel: ch - 1, control: cc, path: path.to_string() });
                        true
                    }
                    _ => false,
                },
                ["bend", ch, path] => match (ch.parse::<u8>(), self.index(path)) {
                    (Ok(ch @ 1..=16), Some(_)) => {
                        self.bindings.push(Binding { channel: ch - 1, control: BEND, path: path.to_string() });
                        true
                    }
                    _ => false,
                },
                ["stick", side, axis, path] => {
                    let s = ["left", "right"].iter().position(|x| x == side);
                    let a = ["x", "y"].iter().position(|x| x == axis);
                    match (s, a, self.index(path)) {
                        (Some(s), Some(a), Some(p)) => {
                            self.assigned[s][a] = Some(p);
                            true
                        }
                        _ => false,
                    }
                }
                _ => false,
            };
            if !ok {
                bad.push(line.to_string());
            }
        }
        bad
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the phone's knobs (shared/live-deck's PARAMS + MELODY)
    const PHONE: [&str; 14] = [
        "voice.ratio",
        "voice.index",
        "voice.gain",
        "drone.freq",
        "drone.gain",
        "incense.cutoff",
        "incense.gain",
        "sway.rate",
        "sway.depth",
        "space.hue",
        "space.glow",
        "melody.spread",
        "melody.rate",
        "melody.glide",
    ];

    #[test]
    fn selections_have_names() {
        for s in [Selection::Master, Selection::Being, Selection::Knob("voice.index".into()), Selection::Object("candle".into())] {
            assert_eq!(Selection::parse(&s.name()), s);
        }
        assert_eq!(Selection::parse("knob:"), Selection::Master);
    }

    /// owner 2026-10-31: the being → its main pairs (left voice ratio / index, right cutoff / LFO rate); a knob → its
    /// pair (or the knob on y with its node's next on x); nothing → the master section
    #[test]
    fn the_selection_picks_what_the_sticks_drive() {
        let mut c = Controls::new(&PHONE);
        c.select(Selection::Being);
        assert_eq!(c.stick_paths(LEFT), ["voice.ratio", "voice.index"]);
        assert_eq!(c.stick_paths(RIGHT), ["incense.cutoff", "sway.rate"]);
        c.select(Selection::Knob("sway.rate".into()));
        assert_eq!(c.stick_paths(LEFT), ["incense.cutoff", "sway.rate"], "its pair on the left");
        assert_eq!(c.stick_paths(RIGHT), ["voice.ratio", "voice.index"]);
        c.select(Selection::Knob("drone.gain".into()));
        assert_eq!(c.stick_paths(LEFT), ["drone.freq", "drone.gain"], "the knob on y, its node's other knob on x");
        c.select(Selection::Knob("nowhere.x".into()));
        assert_eq!(c.selection(), &Selection::Master, "a knob the deck lacks");
        // the master section: no master bus yet – the look and the mix
        assert_eq!(c.stick_paths(LEFT), ["space.hue", "space.glow"]);
        assert_eq!(c.stick_paths(RIGHT), ["voice.gain", "drone.gain"]);
        // the hook: once the engine exposes its master FX chain, it takes over
        let mut paths = PHONE.to_vec();
        paths.extend(["master.cutoff", "master.drive", "master.delay", "master.reverb"]);
        let m = Controls::new(&paths);
        assert_eq!(m.stick_paths(LEFT), ["master.cutoff", "master.drive"]);
        assert_eq!(m.stick_paths(RIGHT), ["master.delay", "master.reverb"]);
        // an object: its own x / y when the deck has them, else the master
        c.select(Selection::Object("candle".into()));
        assert_eq!(c.stick_paths(LEFT), ["space.hue", "space.glow"]);
        // a deck without the cathedral's pairs: its first paths two by two
        let other = Controls::new(&["a.x", "a.y", "b.x"]);
        let mut other = other;
        other.select(Selection::Being);
        assert_eq!((other.stick_paths(LEFT), other.stick_paths(RIGHT)), (["a.x", "a.y"], ["b.x", ""]));
    }

    /// owner 2026-10-31: the stick shows where the thumb lands – relative, so nothing jumps; full deflection reaches
    /// both ends from the middle; a selection while the thumb is down waits for the next touch
    #[test]
    fn a_stick_moves_its_values_from_where_they_were() {
        let mut c = Controls::new(&PHONE);
        c.select(Selection::Being);
        c.stick_down(LEFT, 0.3, 0.7);
        assert_eq!(c.stick_move(LEFT, 0.0, 0.0), [0.3, 0.7], "landing changes nothing");
        assert_eq!(c.stick_move(LEFT, 0.02, -0.03), [0.3, 0.7], "the dead zone");
        let [x, y] = c.stick_move(LEFT, 0.5, -1.0);
        assert!((x - (0.3 + 0.6 * (0.5 - DEAD) / (1.0 - DEAD))).abs() < 1e-5 && (y - 0.1).abs() < 1e-5, "{x} {y}");
        let [x, y] = c.stick_move(LEFT, 1.0, 1.0);
        assert!((x - 0.9).abs() < 1e-5 && y == 1.0, "clamped: {x} {y}");
        c.select(Selection::Master);
        assert_eq!(c.stick_driving(LEFT), ["voice.ratio", "voice.index"], "the thumb keeps what it took");
        assert!(c.stick_is_down(LEFT));
        c.stick_up(LEFT);
        c.stick_down(LEFT, 0.5, 0.5);
        assert_eq!(c.stick_driving(LEFT), ["space.hue", "space.glow"]);
        // assigned by hand: the being's left x
        c.assign(LEFT, 0, "melody.spread");
        c.select(Selection::Being);
        assert_eq!(c.stick_paths(LEFT), ["melody.spread", "voice.index"]);
    }

    /// owner 2026-10-31: "click a knob, move a controller" – learn binds the next control change; a control drives one
    /// path and a path one control; pitch bend in 14 bit; other messages pass
    #[test]
    fn midi_learn_binds_the_next_control() {
        let mut c = Controls::new(&PHONE);
        assert_eq!(c.midi(&[0xB0, 7, 64]), Midi::Ignored, "unbound");
        c.learn("voice.index");
        assert_eq!(c.learning(), Some("voice.index"));
        assert_eq!(c.midi(&[0x90, 60, 100]), Midi::Ignored, "a note doesn't learn");
        assert_eq!(c.midi(&[0xB2, 21, 0]), Midi::Learned("voice.index"));
        assert_eq!(c.learning(), None);
        assert_eq!(c.midi(&[0xB2, 21, 127]), Midi::Set("voice.index", 1.0));
        assert_eq!(c.midi(&[0xB0, 21, 127]), Midi::Ignored, "another channel");
        assert_eq!(c.mapped("voice.index").as_deref(), Some("CC 21 · 3"));
        // the same control learned for another path moves over
        c.learn("drone.gain");
        c.midi(&[0xB2, 21, 5]);
        assert_eq!((c.mapped("voice.index"), c.bindings().len()), (None, 1));
        c.learn("sway.rate");
        assert_eq!(c.midi(&[0xE0, 0, 64]), Midi::Learned("sway.rate"));
        match c.midi(&[0xE0, 0x7F, 0x7F]) {
            Midi::Set("sway.rate", v) => assert!((v - 1.0).abs() < 1e-6),
            m => panic!("{m:?}"),
        }
        c.learn("");
        assert_eq!(c.learning(), None, "disarmed");
        c.unmap("sway.rate");
        assert_eq!(c.bindings().len(), 1);
    }

    /// #191's hook: the mappings round-trip as text; lines the deck can't use come back, they aren't dropped silently
    #[test]
    fn mappings_round_trip_as_text() {
        let mut c = Controls::new(&PHONE);
        c.learn("voice.index");
        c.midi(&[0xB0, 1, 0]);
        c.learn("sway.rate");
        c.midi(&[0xEF, 0, 0]);
        c.assign(RIGHT, 1, "melody.rate");
        let text = c.to_text();
        assert_eq!(text, "cc 1 1 voice.index\nbend 16 sway.rate\nstick right y melody.rate\n");
        let mut d = Controls::new(&PHONE);
        assert!(d.load_text(&text).is_empty());
        assert_eq!(d.to_text(), text);
        let bad = d.load_text("cc 1 1 voice.index\ncc 17 1 voice.gain\nstick up x voice.gain\ncc 1 2 nowhere.x\n");
        assert_eq!(bad, ["cc 17 1 voice.gain", "stick up x voice.gain", "cc 1 2 nowhere.x"]);
        assert_eq!(d.bindings().len(), 1);
    }
}
