//! controls.rs (#209): what the hands drive – the engine's half of the controls UI; the page (the viewer's /live/deck,
//! deck/space-web's `DeckControls`) and later the stage are only the bridge (DECISIONS 2026-10-31: "the Svelte /live
//! wrapper is only the bridge"). Everything here speaks **paths** (`node.port`, the page's knobs) and **0..1**.
//!
//! - **The knobs' ranges** (#369): a knob's value ↔ its 0..1 on its curve (params.rs `Spec`; `norm` / `value`), a knob
//!   dragged (`turn`: a drag's length over the host's travel, fine a tenth as fast) or stepped by a key (`step`), the
//!   sticks' labels and the panel's line (`selection_line`). The deck hands them in with its sliders (`set_knobs`).
//! - **The gamepad** (#228, #369, gamepad.rs): `pad` – a poll's sticks and A / Y; the selection cycles here.
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
//! The event path (`stick_move`, `midi` on a bound control, `pad`) never allocates; learning, loading and cycling do.
use crate::deck::Slider;
use crate::gamepad::{Pad, PadEvent, PadEvents};
use crate::params::Spec;

/// a knob dragged or stepped with fine on (Shift) moves a tenth as fast
pub const FINE: f32 = 0.1;
/// the keys that step a knob (the DOM's names – a native keyboard says the same), and how far on its 0..1
pub const KEY_STEPS: [(&str, f32); 6] = [("ArrowUp", 0.01), ("ArrowRight", 0.01), ("ArrowDown", -0.01), ("ArrowLeft", -0.01), ("PageUp", 0.1), ("PageDown", -0.1)];
/// what a stick label shows for an axis that drives nothing
pub const NONE: &str = "–";

/// `value` on `spec`'s 0..1 – a value the host lacks (NaN) reads as the default
pub fn norm_of(spec: &Spec, value: f32) -> f32 {
    spec.norm(if value.is_finite() { value } else { spec.default })
}

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
/// rate") – the cathedral's; a deck without them takes its first paths two by two. A rig names its own (`set_deck`,
/// #191: the rig's `sticks`, else its first four knobs).
pub const BEING_PAIRS: [[&str; 2]; 2] = [["voice.ratio", "voice.index"], ["incense.cutoff", "sway.rate"]];

/// The master section's slots, (x, y) per stick: each a list – the first path the deck has wins. `master.*` = the
/// master FX chain to come (filter, drive, delay, reverb – #209's plan); until the engine exposes it, the look and the
/// mix. A slot whose paths the deck has none of drives nothing (a rig has no `voice.gain`).
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
    /// each path's range and label (0..1 and the path until the deck names them: `set_knobs`, `describe`)
    specs: Vec<Spec>,
    labels: Vec<String>,
    selection: Selection,
    /// the being's pairs as indices into `paths` (BEING_PAIRS or the first paths two by two)
    being: [[Option<usize>; 2]; 2],
    /// a stick axis assigned by hand (the being's – #209: "what a stick drives is assignable … saved with the preset")
    assigned: [[Option<usize>; 2]; 2],
    sticks: [Stick; 2],
    bindings: Vec<Binding>,
    learning: Option<usize>,
    pad: Pad,
}

impl Controls {
    /// the deck's paths (its knobs)
    pub fn new<S: AsRef<str>>(paths: &[S]) -> Controls {
        let paths: Vec<String> = paths.iter().map(|p| p.as_ref().to_string()).collect();
        let being = Self::default_pairs(&paths);
        let (specs, labels) = (vec![Spec::UNIT; paths.len()], paths.clone());
        Controls { paths, specs, labels, selection: Selection::Master, being, assigned: [[None; 2]; 2], sticks: Default::default(), bindings: Vec::new(), learning: None, pad: Pad::default() }
    }

    /// #369: the deck's knobs as its sliders (deck.rs – their ranges and labels) and the being's pairs on them: `set_deck`
    /// with what the knobs' values and the panel's line need
    pub fn set_knobs(&mut self, knobs: &[Slider], pairs: Option<[[&str; 2]; 2]>) {
        let paths: Vec<&str> = knobs.iter().map(|k| k.path.as_str()).collect();
        self.set_deck(&paths, pairs);
        self.specs = knobs.iter().map(Slider::spec).collect();
        self.labels = knobs.iter().map(|k| if k.label.is_empty() { k.path.clone() } else { k.label.clone() }).collect();
    }

    /// a knob's range and label (the native stage: its being's buttons)
    pub fn describe(&mut self, path: &str, spec: Spec, label: &str) {
        if let Some(i) = self.index(path) {
            (self.specs[i], self.labels[i]) = (spec, label.to_string());
        }
    }

    /// BEING_PAIRS when the deck has them all, else its first paths two by two
    fn default_pairs(paths: &[String]) -> [[Option<usize>; 2]; 2] {
        let at = |p: &str| paths.iter().position(|q| q == p);
        let preferred = BEING_PAIRS.map(|pair| pair.map(at));
        if preferred.iter().flatten().all(Option::is_some) {
            preferred
        } else {
            let mut it = (0..paths.len()).map(Some).chain(std::iter::repeat(None));
            [[it.next().flatten(), it.next().flatten()], [it.next().flatten(), it.next().flatten()]]
        }
    }

    /// Another deck's paths (a rig's knobs, or the fixed patch's again) and the being's pairs on them (#191: `None` =
    /// the defaults; a path the deck lacks, or "", is no axis). The selection goes back to the master section; the
    /// MIDI bindings and hand-assigned axes the new paths still have stay, the rest go; the thumbs let go.
    pub fn set_deck<S: AsRef<str>>(&mut self, paths: &[S], pairs: Option<[[&str; 2]; 2]>) {
        let new: Vec<String> = paths.iter().map(|p| p.as_ref().to_string()).collect();
        let assigned = self.assigned.map(|axes| axes.map(|a| a.map(|i| self.paths[i].clone())));
        self.bindings.retain(|b| new.contains(&b.path));
        (self.specs, self.labels) = (vec![Spec::UNIT; new.len()], new.clone());
        self.paths = new;
        self.being = match pairs {
            Some(p) => p.map(|pair| pair.map(|x| self.index(x))),
            None => Self::default_pairs(&self.paths),
        };
        self.assigned = assigned.map(|axes| axes.map(|a| a.and_then(|p| self.index(&p))));
        self.selection = Selection::Master;
        self.sticks = Default::default();
        self.learning = None;
        self.pad = Pad::default();
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

    /// The thumb's deflection (−1..1 each, y up; inside the unit circle – a corner reaches as far as a side): the two
    /// values' new 0..1 (a slot the stick doesn't drive keeps its start). Never allocates.
    pub fn stick_move(&self, side: usize, dx: f32, dy: f32) -> [f32; 2] {
        let st = &self.sticks[side.min(1)];
        let len = dx.hypot(dy);
        let (dx, dy) = if len > 1.0 { (dx / len, dy / len) } else { (dx, dy) };
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

    /// The selection button (a pad's A / Y): the next selection in the cycle – the master section, the being, each knob
    /// in the deck's order – or the previous one (`back`); an object counts as the master section.
    pub fn cycle(&mut self, back: bool) {
        let n = self.paths.len() + 2;
        let at = match &self.selection {
            Selection::Master | Selection::Object(_) => 0,
            Selection::Being => 1,
            Selection::Knob(p) => self.index(p).map_or(0, |i| i + 2),
        };
        self.selection = match if back { (at + n - 1) % n } else { (at + 1) % n } {
            0 => Selection::Master,
            1 => Selection::Being,
            i => Selection::Knob(self.paths[i - 2].clone()),
        };
    }

    /// One poll of a gamepad (gamepad.rs: the standard mapping's axes and buttons; empty = no pad): what the host does with
    /// its sticks – the same calls as a thumb's – in order; a cycle has already moved the selection. Never allocates but
    /// for a cycle.
    pub fn pad(&mut self, axes: &[f32], buttons: &[bool]) -> PadEvents {
        let events = self.pad.poll(axes, buttons);
        for e in events.iter() {
            if let PadEvent::Cycle { back } = e {
                self.cycle(back);
            }
        }
        events
    }

    // ── the knobs' ranges (#369) ──

    fn spec(&self, path: &str) -> Option<&Spec> {
        self.index(path).map(|i| &self.specs[i])
    }

    /// where `value` sits on knob `path`'s range, 0..1 on its curve (NaN = the host lacks it: the default; no such knob: 0)
    pub fn norm(&self, path: &str, value: f32) -> f32 {
        self.spec(path).map_or(0.0, |s| norm_of(s, value))
    }

    /// knob `path`'s value at `t` (0..1, clamped, on its curve); None: no such knob
    pub fn value(&self, path: &str, t: f32) -> Option<f32> {
        self.spec(path).map(|s| s.value(t))
    }

    /// A knob dragged by `dy` (up = less, as screen y) from its value `start`: `travel` turns it end to end (at least 1),
    /// `fine` a tenth as fast; it stops at its ends. The new value (None: no such knob).
    pub fn turn(&self, path: &str, start: f32, dy: f32, travel: f32, fine: bool) -> Option<f32> {
        let speed = if fine { FINE } else { 1.0 };
        self.value(path, self.norm(path, start) - dy / travel.max(1.0) * speed)
    }

    /// A key on a knob (`KEY_STEPS`; `fine` a tenth of the step): its new value – None: no such knob, or not a key that
    /// turns it.
    pub fn step(&self, path: &str, value: f32, key: &str, fine: bool) -> Option<f32> {
        let (_, d) = KEY_STEPS.iter().find(|(k, _)| *k == key)?;
        self.value(path, self.norm(path, value) + d * if fine { FINE } else { 1.0 })
    }

    /// a knob's label ("" = none: `NONE`; a path the deck lacks: the path)
    pub fn label<'a>(&'a self, path: &'a str) -> &'a str {
        match self.index(path) {
            _ if path.is_empty() => NONE,
            Some(i) => &self.labels[i],
            None => path,
        }
    }

    /// stick `side`'s (x, y) labels now (`NONE` for an axis that drives nothing)
    pub fn stick_labels(&self, side: usize) -> [&str; 2] {
        let [x, y] = self.stick_paths(side);
        [self.label(x), self.label(y)]
    }

    /// the knob that owns the sticks ("" none)
    pub fn knob(&self) -> &str {
        match &self.selection {
            Selection::Knob(p) => p,
            _ => "",
        }
    }

    /// What the sticks drive, in one line (#209): the selection – a knob's label, else the deck's word for the being or the
    /// master section (`being`, `master`: the host's language) – and each stick's x / y labels:
    /// `FM index · L FM ratio / FM index · R cutoff / LFO rate`.
    pub fn selection_line(&self, being: &str, master: &str) -> String {
        let what = match &self.selection {
            Selection::Being => being,
            Selection::Knob(p) => self.label(p),
            _ => master,
        };
        let [l, r] = [self.stick_labels(0), self.stick_labels(1)];
        format!("{what} · L {} / {} · R {} / {}", l[0], l[1], r[0], r[1])
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
        let [x, y] = c.stick_move(LEFT, 0.6, -0.8);
        let shape = |d: f32| 0.6 * (d - DEAD) / (1.0 - DEAD);
        assert!((x - (0.3 + shape(0.6))).abs() < 1e-5 && (y - (0.7 - shape(0.8))).abs() < 1e-5, "{x} {y}");
        let [x, y] = c.stick_move(LEFT, 1.0, 0.0);
        assert!((x - 0.9).abs() < 1e-5 && y == 0.7, "full deflection: {x} {y}");
        let [x, y] = c.stick_move(LEFT, 1.0, 1.0);
        let corner = 0.6 * (std::f32::consts::FRAC_1_SQRT_2 - DEAD) / (1.0 - DEAD);
        assert!((x - (0.3 + corner)).abs() < 1e-5 && y == 1.0, "a corner inside the unit circle, the value clamped: {x} {y}");
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

    /// #191: in a rig the knobs are `<track>_<node>.<port>` – the sticks follow the rig's pairs, the master section
    /// skips the slots the rig lacks, and the fixed patch comes back with its defaults
    #[test]
    fn a_rig_names_the_sticks() {
        let rig = ["voice_voice.ratio", "voice_voice.index", "voice_melody.spread", "drone_drone.freq", "voice_voice.gain"];
        let mut c = Controls::new(&PHONE);
        c.set_deck(&rig, Some([["voice_voice.ratio", "voice_voice.index"], ["voice_melody.spread", "drone_drone.freq"]]));
        c.select(Selection::Being);
        assert_eq!(c.stick_paths(LEFT), ["voice_voice.ratio", "voice_voice.index"]);
        assert_eq!(c.stick_paths(RIGHT), ["voice_melody.spread", "drone_drone.freq"]);
        // the master section: nothing of the fixed patch is there, so its slots drive nothing
        c.select(Selection::Master);
        assert_eq!((c.stick_paths(LEFT), c.stick_paths(RIGHT)), (["", ""], ["", ""]));
        // a pair naming a path the rig lacks has no axis there
        c.set_deck(&rig, Some([["voice_voice.ratio", "nowhere.x"], ["", "drone_drone.freq"]]));
        c.select(Selection::Being);
        assert_eq!((c.stick_paths(LEFT), c.stick_paths(RIGHT)), (["voice_voice.ratio", ""], ["", "drone_drone.freq"]));
        // no pairs: the defaults for these paths – the first knobs, two by two
        c.set_deck(&rig, None);
        c.select(Selection::Being);
        assert_eq!(c.stick_paths(LEFT), ["voice_voice.ratio", "voice_voice.index"]);
        assert_eq!(c.stick_paths(RIGHT), ["voice_melody.spread", "drone_drone.freq"]);
        // back to the fixed patch: its pairs and master
        c.set_deck(&PHONE, None);
        c.select(Selection::Being);
        assert_eq!(c.stick_paths(LEFT), ["voice.ratio", "voice.index"]);
        c.select(Selection::Master);
        assert_eq!(c.stick_paths(LEFT), ["space.hue", "space.glow"]);
    }

    /// the deck's own scene's knobs as the page hands them in (its sliders) and its stick pairs
    fn own_deck() -> Controls {
        let d = crate::deck::Deck::new();
        let mut c = Controls::new::<&str>(&[]);
        let s = &d.now().played.sticks;
        c.set_knobs(&d.sliders("en"), Some([[&s[0][0], &s[0][1]], [&s[1][0], &s[1][1]]]));
        c
    }

    /// #369: a knob's value ↔ 0..1 on its curve, a drag (up = more, fine a tenth, stops at the ends), a key's step
    #[test]
    fn a_knob_turns_on_its_range() {
        let c = own_deck();
        let k = &crate::deck::Deck::new().sliders("en")[0];
        let (lo, hi) = (k.min, k.max);
        assert_eq!(c.norm(&k.path, lo), 0.0);
        assert_eq!(c.norm(&k.path, hi * 10.0), 1.0, "clamped");
        assert!((c.norm(&k.path, f32::NAN) - c.norm(&k.path, k.value)).abs() < 1e-6, "a value the host lacks: the default");
        assert_eq!((c.norm("nowhere.x", 1.0), c.value("nowhere.x", 0.5)), (0.0, None));
        let mid = c.value(&k.path, 0.5).unwrap();
        assert!((c.turn(&k.path, mid, -100.0, 200.0, false).unwrap() - hi).abs() < 1e-3 * hi, "up half the travel: the top");
        let quarter = c.turn(&k.path, mid, 50.0, 200.0, false).unwrap();
        assert!((c.norm(&k.path, quarter) - 0.25).abs() < 1e-4);
        let fine = c.turn(&k.path, mid, 50.0, 200.0, true).unwrap();
        assert!((c.norm(&k.path, fine) - 0.475).abs() < 1e-4, "fine: a tenth");
        assert_eq!(c.turn(&k.path, hi, -1000.0, 0.0, false), Some(hi), "a travel under 1 counts as 1; the end stops it");
        let up = c.step(&k.path, mid, "ArrowUp", false).unwrap();
        assert!((c.norm(&k.path, up) - 0.51).abs() < 1e-4);
        let page = c.step(&k.path, mid, "PageDown", true).unwrap();
        assert!((c.norm(&k.path, page) - 0.49).abs() < 1e-4, "fine: a tenth of the page");
        assert_eq!(c.step(&k.path, mid, "Enter", false), None);
    }

    /// owner 2026-10-31 (#228): the button walks master → the being → each knob → master, back the other way; the panel's
    /// line names the selection and each stick's labels (#209)
    #[test]
    fn the_selection_cycles_and_its_line_names_the_sticks() {
        let mut c = Controls::new(&["a.x", "a.y", "b.x"]);
        c.describe("a.x", Spec::UNIT, "ax");
        let mut seen = Vec::new();
        for _ in 0..6 {
            c.cycle(false);
            seen.push(c.selection().name());
        }
        assert_eq!(seen, ["being", "knob:a.x", "knob:a.y", "knob:b.x", "", "being"]);
        c.cycle(true);
        c.cycle(true);
        assert_eq!(c.selection(), &Selection::Knob("b.x".into()), "from the master section back: the last knob");
        c.select(Selection::Object("candle".into()));
        c.cycle(false);
        assert_eq!(c.selection(), &Selection::Being, "an object counts as the master section");
        assert_eq!(c.selection_line("Being", "Master"), "Being · L ax / a.y · R b.x / –");
        c.select(Selection::Knob("a.y".into()));
        assert_eq!((c.knob(), c.selection_line("Being", "Master").as_str()), ("a.y", "a.y · L ax / a.y · R b.x / –"));
        c.select(Selection::Master);
        assert_eq!((c.knob(), c.stick_labels(LEFT)), ("", [NONE, NONE]), "a deck without the master's paths");
        assert_eq!(c.selection_line("Being", "Master"), "Master · L – / – · R – / –");
    }

    /// #228, #369: a pad's poll – its events in order, A cycles the selection here
    #[test]
    fn a_pad_drives_the_sticks_and_cycles_the_selection() {
        let mut c = own_deck();
        let ev = |c: &mut Controls, axes: &[f32], buttons: &[bool]| c.pad(axes, buttons).iter().collect::<Vec<_>>();
        assert_eq!(ev(&mut c, &[0.0, -0.9], &[true]), [PadEvent::Down(LEFT), PadEvent::Move(LEFT, 0.0, 0.9), PadEvent::Cycle { back: false }]);
        assert_eq!(c.selection(), &Selection::Being);
        assert_eq!(ev(&mut c, &[], &[]), [PadEvent::Up(LEFT)]);
    }

    /// The script both pads run (#369's acceptance: the same input on the native stage – deck-native pad.rs reads it – and
    /// the browser – shared/live-deck/test/fixtures/controls.json, the core's answers its tests apply): frames of a pad in the
    /// standard mapping.
    const PAD_SCRIPT: [([f32; 4], [bool; 4]); 13] = [
        ([0.1, 0.0, 0.0, 0.0], [false; 4]),
        ([0.6, -0.8, 0.0, 0.0], [false; 4]),
        ([0.2, 0.0, 0.0, 0.0], [false; 4]),
        ([0.1, 0.0, 0.0, 0.0], [false; 4]),
        ([0.0, 0.0, 1.0, 1.0], [false; 4]),
        ([0.0, 0.0, 0.9, 0.0], [true, false, false, false]),
        ([0.0, 0.0, 0.0, 0.0], [true, false, false, false]),
        ([0.0, 0.0, 0.0, 0.0], [false; 4]),
        ([0.0, 0.0, 0.0, 0.0], [true, false, false, false]),
        ([0.0, 0.0, 0.0, 0.0], [false, false, false, true]),
        ([0.0, 0.0, 0.0, 0.0], [false, false, false, true]),
        ([-0.25, 0.25, 0.0, 0.0], [false, false, false, false]),
        ([-0.7, 0.7, 0.0, 0.0], [false, false, false, false]),
    ];

    /// shared/live-deck/test/fixtures/controls.json – the core's answers the browser's controls tests apply (#369: the page
    /// computes none of it). `UPDATE_FIXTURES=1 cargo test -p deck-space controls_fixture` writes it; a plain run fails when
    /// stale.
    #[test]
    fn controls_fixture_for_the_browser_deck_is_current() {
        use serde_json::{json, Value};
        let mut c = own_deck();
        let knobs = crate::deck::Deck::new().sliders("en");
        let mut pad = Vec::new();
        for (axes, buttons) in PAD_SCRIPT {
            let events: Vec<[f32; 4]> = c.pad(&axes, &buttons).iter().map(|e| e.code()).collect();
            pad.push(json!({ "axes": axes, "buttons": buttons, "events": events, "selection": c.selection().name() }));
        }
        let mut answers: serde_json::Map<String, Value> = serde_json::Map::new();
        let mut put = |name: &str, args: Value, result: Value| {
            answers.entry(name).or_insert_with(|| json!([])).as_array_mut().unwrap().push(json!([args, result]));
        };
        for k in knobs.iter().chain([&crate::deck::Slider { path: "nowhere.x".into(), min: 0.0, max: 1.0, value: 0.5, log: false, label: String::new() }]) {
            put("norm", json!([k.path, k.value]), json!(c.norm(&k.path, k.value)));
            for t in [0.0, 0.3, 0.6, 1.0] {
                put("value_at", json!([k.path, t]), json!(c.value(&k.path, t)));
            }
            for (dy, fine) in [(-100.0, false), (50.0, false), (50.0, true), (-1000.0, false)] {
                put("turn", json!([k.path, k.value, dy, 200.0, fine]), json!(c.turn(&k.path, k.value, dy, 200.0, fine)));
            }
            for (key, fine) in [("ArrowUp", false), ("PageDown", true), ("Enter", false)] {
                put("step", json!([k.path, k.value, key, fine]), json!(c.step(&k.path, k.value, key, fine)));
            }
        }
        let outside = knobs.iter().find(|k| !c.being.iter().flatten().any(|i| *i == c.index(&k.path))).map_or("", |k| k.path.as_str());
        for sel in [String::new(), "being".into(), format!("knob:{}", knobs[0].path), format!("knob:{outside}")] {
            c.select(Selection::parse(&sel));
            let sel = c.selection().name();
            put("selection_line", json!([sel, "Being", "Master"]), json!(c.selection_line("Being", "Master")));
            for side in [LEFT, RIGHT] {
                put("stick_labels", json!([sel, side]), json!(c.stick_labels(side)));
                put("stick_paths", json!([sel, side]), json!(c.stick_paths(side)));
            }
            put("knob", json!([sel]), json!(c.knob()));
        }
        // a thumb on the being's left stick, landing at the knobs' values: what its deflection sets
        c.select(Selection::Being);
        let [px, py] = c.stick_paths(LEFT).map(String::from);
        let at = |p: &str| knobs.iter().find(|k| k.path == p).map_or(f32::NAN, |k| k.value);
        let start = [c.norm(&px, at(&px)), c.norm(&py, at(&py))];
        c.stick_down(LEFT, start[0], start[1]);
        put("stick_down", json!([LEFT, start[0], start[1]]), Value::Null);
        for (dx, dy) in [(0.5, 0.0), (1.0, 1.0)] {
            put("stick_move", json!([LEFT, dx, dy]), json!(c.stick_move(LEFT, dx, dy)));
        }
        for (p, t) in [(&px, c.stick_move(LEFT, 0.5, 0.0)[0]), (&py, c.stick_move(LEFT, 0.5, 0.0)[1]), (&px, c.stick_move(LEFT, 1.0, 1.0)[0]), (&py, c.stick_move(LEFT, 1.0, 1.0)[1])] {
            put("value_at", json!([p, t]), json!(c.value(p, t)));
        }
        let fixture = json!({ "knobs": knobs, "pad": pad, "answers": answers });
        let text = serde_json::to_string_pretty(&fixture).unwrap() + "\n";
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/live-deck/test/fixtures/controls.json");
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(path, &text).unwrap();
        }
        let on_disk = std::fs::read_to_string(path).unwrap_or_default().replace('\r', "");
        assert!(on_disk == text, "{path} is stale: UPDATE_FIXTURES=1 cargo test -p deck-space controls_fixture");
    }

    /// a new deck keeps the mappings its paths still have and drops the rest
    #[test]
    fn a_new_deck_keeps_the_mappings_it_still_has() {
        let mut c = Controls::new(&PHONE);
        c.learn("voice.index");
        c.midi(&[0xB0, 1, 0]);
        c.learn("sway.rate");
        c.midi(&[0xB0, 2, 0]);
        c.assign(LEFT, 0, "sway.rate");
        c.set_deck(&["voice.index", "x.y"], None);
        assert_eq!(c.bindings().len(), 1);
        assert_eq!(c.mapped("voice.index").as_deref(), Some("CC 1 · 1"));
        assert_eq!(c.to_text(), "cc 1 1 voice.index\n");
    }
}
