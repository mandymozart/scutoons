//! The device messages as a monitor reads them (#373, BRIDGE-AUDIT.md §2): a message of the scene's `[outputs]` (MIDI bytes, an
//! OSC address with typed arguments, DMX channel values) in ONE short line (`Signal::describe`), and per protocol the last
//! line, a count and an activity that jumps to 1 on a message and decays (`DeviceMonitor`). The browser's pages feed what the
//! engine resolved and draw the rows; a native HUD feeds what its runner sends – the words and the lamp's timing are the same.
//!
//! The model (`Signal`, `OscArg`) is also what live-core encodes into its signal records (live-core `signals.rs`).
use serde::Serialize;
use std::fmt::Write;

/// an activity lamp loses 63 % per this many seconds after a message
pub const ACTIVITY_SECONDS: f64 = 0.4;
/// a lamp is lit while its activity is at least this
pub const LIT: f64 = 0.25;

/// The protocols, in the order a monitor lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Midi = 0,
    Osc = 1,
    Dmx = 2,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Midi, Kind::Osc, Kind::Dmx];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Midi => "midi",
            Kind::Osc => "osc",
            Kind::Dmx => "dmx",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OscArg {
    Int(i32),
    Float(f32),
    Str(String),
    Bool(bool),
}

impl OscArg {
    /// An argument the way a page names it: `kind` is `int` / `float` / `string` / `bool`; the value is `num` (a bool is
    /// non-zero) or `text`. None for another kind.
    pub fn named(kind: &str, num: f64, text: &str) -> Option<OscArg> {
        Some(match kind {
            "int" => OscArg::Int(num as i32),
            "float" => OscArg::Float(num as f32),
            "string" => OscArg::Str(text.to_owned()),
            "bool" => OscArg::Bool(num != 0.0),
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Signal<'a> {
    Midi { conductor: bool, bytes: &'a [u8] },
    Dmx { ch: u16, values: &'a [u8], flash: bool },
    Osc { address: &'a str, args: &'a [OscArg] },
}

/// a string as JSON writes it (an OSC string argument is shown quoted)
fn quoted(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl Signal<'_> {
    pub fn kind(&self) -> Kind {
        match self {
            Signal::Midi { .. } => Kind::Midi,
            Signal::Dmx { .. } => Kind::Dmx,
            Signal::Osc { .. } => Kind::Osc,
        }
    }

    /// The message in a short line: `CC 74 = 127 · ch 1`, `note on 60 · vel 127 · ch 1`, `DMX 20–21 = 255 255`, `/fx/level 0.5`.
    pub fn describe(&self) -> String {
        let mut s = String::new();
        self.describe_into(&mut s);
        s
    }

    /// `describe` into `out` (cleared first) – a monitor reuses one buffer per protocol.
    pub fn describe_into(&self, out: &mut String) {
        out.clear();
        match *self {
            Signal::Osc { address, args } => {
                out.push_str(address);
                for a in args {
                    out.push(' ');
                    match a {
                        OscArg::Str(v) => quoted(out, v),
                        OscArg::Int(v) => {
                            let _ = write!(out, "{v}");
                        }
                        OscArg::Float(v) => {
                            let _ = write!(out, "{v}");
                        }
                        OscArg::Bool(v) => {
                            let _ = write!(out, "{v}");
                        }
                    }
                }
            }
            Signal::Dmx { ch, values, flash } => {
                let _ = write!(out, "DMX ");
                if values.len() > 1 {
                    let _ = write!(out, "{ch}–{}", u32::from(ch) + values.len() as u32 - 1);
                } else {
                    let _ = write!(out, "{ch}");
                }
                out.push_str(" =");
                for v in values {
                    let _ = write!(out, " {v}");
                }
                if flash {
                    out.push_str(" · flash");
                }
            }
            Signal::Midi { conductor, bytes } => {
                let at = |i: usize| bytes.get(i).copied().unwrap_or(0);
                let (status, a, b) = (at(0), at(1), at(2));
                let ch = (status & 0x0f) + 1;
                match status & 0xf0 {
                    0x80 => {
                        let _ = write!(out, "note off {a} · ch {ch}");
                    }
                    0x90 if b == 0 => {
                        let _ = write!(out, "note off {a} · ch {ch}");
                    }
                    0x90 => {
                        let _ = write!(out, "note on {a} · vel {b} · ch {ch}");
                    }
                    0xb0 => {
                        let _ = write!(out, "CC {a} = {b} · ch {ch}");
                    }
                    0xc0 => {
                        let _ = write!(out, "program {a} · ch {ch}");
                    }
                    0xe0 => {
                        let _ = write!(out, "bend {} · ch {ch}", (u16::from(b) << 7) | u16::from(a));
                    }
                    _ => {
                        out.push_str("MIDI");
                        for x in bytes {
                            let _ = write!(out, " {x:02x}");
                        }
                    }
                }
                if conductor {
                    out.push_str(" → conductor");
                }
            }
        }
    }
}

/// One protocol's row of a monitor.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    pub kind: Kind,
    /// messages since the monitor started (or was reset)
    pub count: u64,
    /// the last message in a short line
    pub text: String,
    /// 1 on a message, falling to 0 (`ACTIVITY_SECONDS`)
    pub activity: f64,
    /// traffic just now – the lamp is lit (`LIT`)
    pub active: bool,
}

#[derive(Debug, Clone, Default)]
struct Kept {
    count: u64,
    /// the clock (ms) of the last message
    at: Option<f64>,
    text: String,
}

/// Per protocol the last message as a line, a count and an activity. The caller brings the clock (ms).
#[derive(Debug, Clone, Default)]
pub struct DeviceMonitor {
    kept: [Kept; 3],
}

impl DeviceMonitor {
    pub fn new() -> DeviceMonitor {
        DeviceMonitor::default()
    }

    /// A message that arrived at `now_ms`. No allocation once the line's buffer has grown to its size.
    pub fn push(&mut self, s: &Signal, now_ms: f64) {
        let k = &mut self.kept[s.kind() as usize];
        k.count += 1;
        k.at = Some(now_ms);
        s.describe_into(&mut k.text);
    }

    /// The activity of `kind` at `now_ms`: 1 at a message, then falling; 0 before the first.
    pub fn activity(&self, kind: Kind, now_ms: f64) -> f64 {
        match self.kept[kind as usize].at {
            None => 0.0,
            Some(at) => libm::exp(-(now_ms - at).max(0.0) / 1000.0 / ACTIVITY_SECONDS),
        }
    }

    /// The last line of `kind` ('' = none yet).
    pub fn text(&self, kind: Kind) -> &str {
        &self.kept[kind as usize].text
    }

    pub fn count(&self, kind: Kind) -> u64 {
        self.kept[kind as usize].count
    }

    /// A row per protocol that has carried a message (a protocol nothing uses shows nothing), as of `now_ms`.
    pub fn rows(&self, now_ms: f64) -> Vec<Row> {
        Kind::ALL
            .into_iter()
            .filter(|&k| self.count(k) > 0)
            .map(|kind| {
                let activity = self.activity(kind, now_ms);
                Row { kind, count: self.count(kind), text: self.text(kind).to_owned(), activity, active: activity >= LIT }
            })
            .collect()
    }

    pub fn reset(&mut self) {
        *self = DeviceMonitor::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn midi(bytes: &[u8]) -> String {
        Signal::Midi { conductor: false, bytes }.describe()
    }

    #[test]
    fn short_lines_for_what_a_monitor_shows() {
        assert_eq!(midi(&[0xb0, 74, 127]), "CC 74 = 127 · ch 1");
        assert_eq!(midi(&[0x90, 60, 127]), "note on 60 · vel 127 · ch 1");
        assert_eq!(midi(&[0x93, 36, 0]), "note off 36 · ch 4", "note on with velocity 0 is a note off");
        assert_eq!(midi(&[0x80, 36, 64]), "note off 36 · ch 1");
        assert_eq!(midi(&[0xe1, 127, 127]), "bend 16383 · ch 2");
        assert_eq!(midi(&[0xc0, 5]), "program 5 · ch 1");
        assert_eq!(midi(&[0xf8]), "MIDI f8");
        assert_eq!(midi(&[0xf0, 0x7e, 0xf7]), "MIDI f0 7e f7");
        assert_eq!(Signal::Midi { conductor: true, bytes: &[0xb0, 1, 2] }.describe(), "CC 1 = 2 · ch 1 → conductor");
        assert_eq!(Signal::Dmx { ch: 7, values: &[255], flash: true }.describe(), "DMX 7 = 255 · flash");
        assert_eq!(Signal::Dmx { ch: 1, values: &[0, 128, 255], flash: false }.describe(), "DMX 1–3 = 0 128 255");
        let args = [OscArg::Float(0.5), OscArg::Str("hi".into()), OscArg::Bool(true), OscArg::Int(-3)];
        assert_eq!(Signal::Osc { address: "/fx/level", args: &args }.describe(), "/fx/level 0.5 \"hi\" true -3");
        assert_eq!(Signal::Osc { address: "/go", args: &[] }.describe(), "/go");
        assert_eq!(Signal::Osc { address: "/s", args: &[OscArg::Str("a\"b\n".into())] }.describe(), "/s \"a\\\"b\\n\"");
    }

    #[test]
    fn a_monitor_keeps_the_last_line_a_count_and_an_activity() {
        let mut m = DeviceMonitor::new();
        assert!(m.rows(0.0).is_empty(), "nothing wired, nothing shown");
        m.push(&Signal::Midi { conductor: false, bytes: &[0xb0, 74, 1] }, 1000.0);
        m.push(&Signal::Midi { conductor: false, bytes: &[0xb0, 74, 127] }, 1500.0);
        m.push(&Signal::Dmx { ch: 1, values: &[9], flash: false }, 1500.0);
        assert_eq!(m.count(Kind::Midi), 2);
        assert_eq!(m.text(Kind::Midi), "CC 74 = 127 · ch 1");
        assert_eq!(m.activity(Kind::Osc, 1500.0), 0.0, "before the first message");
        assert_eq!(m.activity(Kind::Midi, 1500.0), 1.0);
        assert!((m.activity(Kind::Midi, 1500.0 + ACTIVITY_SECONDS * 1000.0) - 1.0 / std::f64::consts::E).abs() < 1e-9);
        assert_eq!(m.activity(Kind::Midi, 1000.0), 1.0, "a clock that went back: still 1");
        let rows = m.rows(1500.0 + 2000.0);
        assert_eq!(rows.iter().map(|r| (r.kind, r.count, r.active)).collect::<Vec<_>>(), [(Kind::Midi, 2, false), (Kind::Dmx, 1, false)]);
        assert!(m.rows(1500.0).iter().all(|r| r.active), "lit while the activity is up");
        m.reset();
        assert!(m.rows(0.0).is_empty());
    }

    #[test]
    fn an_argument_is_named_by_its_kind() {
        assert_eq!(OscArg::named("float", 0.5, ""), Some(OscArg::Float(0.5)));
        assert_eq!(OscArg::named("string", 0.0, "x"), Some(OscArg::Str("x".into())));
        assert_eq!(OscArg::named("bool", 1.0, ""), Some(OscArg::Bool(true)));
        assert_eq!(OscArg::named("blob", 0.0, ""), None);
    }
}
