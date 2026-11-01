//! The scene's `[outputs]` (#211): saved with the deck's scene / preset TOML, parsed and checked here into a `Plan`
//! the engine runs. Every error names where it is (`outputs.map[1] (space.glow) to[0]`) and what would be right;
//! all errors come at once.
//!
//! ```toml
//! [outputs]
//! lead = 20                # ms: messages leave this much before their beat is heard (the widget + the fixture)
//! dmx = "auto"             # the USB-DMX widget (Enttec DMX USB Pro …): "auto" = the first found, or its port
//! midi = "UM-ONE"          # the MIDI out port (a part of its name)
//!
//! [[outputs.fixture]]
//! id = "strobe"
//! profile = "strobe"       # built in, or a profile file next to the scene ("fixtures/my-strobe.toml")
//! address = 1
//! # max_hz = 3             # optional cap (off by default)
//!
//! [[outputs.map]]
//! from = "space.glow"
//! smooth = 0.05            # s, for its targets (each may set its own)
//! to = [{ dmx = "wash.dimmer", curve = "exp" }, { cc = 74, channel = 1 }, { cc = 20, port = "conductor" }]
//!
//! [[outputs.strobe]]
//! fixture = "strobe"
//! rate = "sway.rate"       # optional: steps the divisions (0.5 = neutral)
//! neutral = "1/4"
//! level = "space.glow"     # optional: the flash's intensity
//! duration = 30            # ms
//! ```

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::fixture::{self, Profile};
use crate::shape::{Curve, Shape};
use crate::strobe::{self, Mode, Rate};

/// Where a MIDI target goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiPort {
    /// the deck's MIDI out port (`outputs.midi`)
    Out,
    /// the conductor, over the relay (`deck.midi`)
    Conductor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiKind {
    Cc(u8),
    /// a gate: on at ≥ 0.5 of the shaped value, off below
    Note { note: u8, velocity: u8 },
    /// 14 bit
    Bend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    Midi { port: MidiPort, channel: u8, kind: MidiKind },
    /// absolute channels of the universe (1-based); `fine` for 16 bit
    Dmx { ch: u16, fine: Option<u16> },
}

/// One target of a parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    /// index into `Plan::paths`
    pub source: usize,
    pub shape: Shape,
    pub smooth: f32,
    pub wire: Wire,
}

/// A parameter path read as 0..1 through a shape (the strobe's rate and level).
#[derive(Debug, Clone, PartialEq)]
pub struct Input {
    pub source: usize,
    pub shape: Shape,
}

/// A fixture's clock-synced strobe.
#[derive(Debug, Clone, PartialEq)]
pub struct StrobePlan {
    pub fixture: String,
    /// the absolute flash channel
    pub flash: u16,
    pub level: u8,
    /// the other channels' resting values (absolute channel, value)
    pub rest: Vec<(u16, u8)>,
    pub rate: Rate,
    pub rate_in: Option<Input>,
    pub level_in: Option<Input>,
    /// the flash length (s), at most half the period
    pub duration: f64,
}

/// What the engine runs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Plan {
    /// the parameter paths the plan reads (the host fills a value per path, in this order)
    pub paths: Vec<String>,
    pub targets: Vec<Target>,
    pub strobes: Vec<StrobePlan>,
    /// s
    pub lead: f64,
    /// the USB-DMX widget: None = no DMX out, Some("") = the first widget found
    pub dmx: Option<String>,
    /// the MIDI out port (a part of its name)
    pub midi: Option<String>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty() && self.strobes.is_empty()
    }

    pub fn uses_conductor(&self) -> bool {
        self.targets.iter().any(|t| matches!(t.wire, Wire::Midi { port: MidiPort::Conductor, .. }))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    #[serde(default = "default_lead")]
    lead: f64,
    dmx: Option<String>,
    midi: Option<String>,
    #[serde(default)]
    fixture: Vec<RawFixture>,
    #[serde(default)]
    map: Vec<RawMap>,
    #[serde(default)]
    strobe: Vec<RawStrobe>,
}

fn default_lead() -> f64 {
    20.0
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFixture {
    id: String,
    profile: String,
    address: i64,
    max_hz: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMap {
    from: String,
    #[serde(default)]
    smooth: Option<f32>,
    to: Vec<RawTarget>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawShape {
    #[serde(rename = "in")]
    input: Option<[f32; 2]>,
    curve: Option<String>,
    #[serde(default)]
    invert: bool,
    min: Option<f32>,
    max: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTarget {
    dmx: Option<toml::Value>,
    bits: Option<u8>,
    cc: Option<i64>,
    note: Option<i64>,
    #[serde(default)]
    bend: bool,
    channel: Option<i64>,
    velocity: Option<i64>,
    port: Option<String>,
    smooth: Option<f32>,
    #[serde(flatten)]
    shape: RawShape,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrobe {
    fixture: String,
    rate: Option<String>,
    rate_in: Option<[f32; 2]>,
    level: Option<String>,
    level_in: Option<[f32; 2]>,
    neutral: Option<String>,
    mode: Option<String>,
    #[serde(default = "default_duration")]
    duration: f64,
    max_hz: Option<f64>,
}

fn default_duration() -> f64 {
    30.0
}

struct Placed {
    address: u16,
    profile: Profile,
    max_hz: Option<f64>,
}

/// Reads a profile file the scene names (relative to the scene); the host's file system.
pub type Loader<'a> = &'a dyn Fn(&str) -> Result<String, String>;

/// Parse and check the scene's `[outputs]` table. `known`: the deck's parameter paths (None = not checked); `load`:
/// reads a profile file that is not built in. Every problem is one line of the error list.
pub fn parse(table: &toml::Table, known: Option<&[String]>, load: Loader<'_>) -> Result<Plan, Vec<String>> {
    let raw: Raw = toml::Value::Table(table.clone()).try_into().map_err(|e: toml::de::Error| vec![format!("outputs: {}", e.message())])?;
    let mut errors = Vec::new();
    let mut plan = Plan { dmx: raw.dmx.map(|d| if d == "auto" { String::new() } else { d }), midi: raw.midi, ..Plan::default() };
    if !(0.0..=1000.0).contains(&raw.lead) {
        errors.push(format!("outputs.lead {} ms: 0…1000", raw.lead));
    }
    plan.lead = raw.lead.clamp(0.0, 1000.0) / 1000.0;

    // fixtures: profiles, addresses, no overlaps
    let mut fixtures: BTreeMap<String, Placed> = BTreeMap::new();
    let mut taken: BTreeMap<u16, String> = BTreeMap::new();
    for (i, f) in raw.fixture.iter().enumerate() {
        let at = format!("outputs.fixture[{i}] ({})", f.id);
        let text = match fixture::builtin(&f.profile) {
            Some(t) => Ok(t.to_owned()),
            None => load(&f.profile).map_err(|e| format!("{at}: profile '{}': {e} (built in: strobe)", f.profile)),
        };
        let profile = match text.and_then(|t| Profile::parse(&t).map_err(|e| format!("{at}: {e}"))) {
            Ok(p) => p,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        let Some(address) = u16::try_from(f.address).ok().filter(|a| (1..=512).contains(a)) else {
            errors.push(format!("{at}: address {} is outside 1…512", f.address));
            continue;
        };
        let end = address as u32 + profile.footprint() as u32 - 1;
        if end > 512 {
            errors.push(format!("{at}: {} channels from {address} run past 512", profile.footprint()));
            continue;
        }
        if let Some(hz) = f.max_hz {
            if !(hz.is_finite() && hz > 0.0) {
                errors.push(format!("{at}: max_hz {hz} must be above 0 (leave it out for no cap)"));
            }
        }
        let mut clash = None;
        for c in profile.channels.values() {
            for n in [Some(c.coarse), c.fine].into_iter().flatten() {
                let abs = address + n - 1;
                if let Some(other) = taken.insert(abs, f.id.clone()) {
                    clash.get_or_insert(format!("{at}: DMX channel {abs} is also fixture '{other}'"));
                }
            }
        }
        if let Some(c) = clash {
            errors.push(c);
        }
        if fixtures.contains_key(&f.id) {
            errors.push(format!("{at}: a second fixture '{}'", f.id));
        }
        let max_hz = f.max_hz.or(profile.strobe.as_ref().and_then(|s| s.max_hz));
        fixtures.insert(f.id.clone(), Placed { address, profile, max_hz });
    }
    if !fixtures.is_empty() && plan.dmx.is_none() {
        errors.push("outputs: fixtures need a DMX out – set outputs.dmx (\"auto\" = the first USB-DMX widget found)".into());
    }

    let mut paths: Vec<String> = Vec::new();
    let mut source = |path: &str, at: &str, errors: &mut Vec<String>| -> usize {
        if !path.contains('.') || path.starts_with('.') || path.ends_with('.') {
            errors.push(format!("{at}: '{path}' is not a parameter path (node.port, e.g. space.glow)"));
        } else if let Some(k) = known {
            if !k.iter().any(|p| p == path) {
                errors.push(format!("{at}: the deck has no parameter '{path}'"));
            }
        }
        match paths.iter().position(|p| p == path) {
            Some(i) => i,
            None => {
                paths.push(path.to_owned());
                paths.len() - 1
            }
        }
    };

    // strobes first: their flash channels are theirs alone
    let mut flash_channels: BTreeMap<u16, String> = BTreeMap::new();
    for (i, s) in raw.strobe.iter().enumerate() {
        let at = format!("outputs.strobe[{i}] ({})", s.fixture);
        let Some(fx) = fixtures.get(&s.fixture) else {
            errors.push(format!("{at}: no fixture '{}' (outputs.fixture ids: {})", s.fixture, fixtures.keys().cloned().collect::<Vec<_>>().join(", ")));
            continue;
        };
        let Some(spec) = &fx.profile.strobe else {
            errors.push(format!("{at}: profile '{}' has no [strobe] section (which channel flashes)", fx.profile.name));
            continue;
        };
        let abs = |name: &str| fx.address + fx.profile.channels[name].coarse - 1;
        let neutral = match s.neutral.as_deref() {
            None => strobe::division(strobe::NEUTRAL).expect("1/4"),
            Some(n) => match strobe::division(n) {
                Some(d) => d,
                None => {
                    errors.push(format!("{at}: neutral '{n}' is not a division ({})", strobe::division_names()));
                    continue;
                }
            },
        };
        let mode = match s.mode.as_deref() {
            None | Some("steps") => Mode::Steps,
            Some("free") => Mode::Free,
            Some(m) => {
                errors.push(format!("{at}: mode '{m}' (steps, free)"));
                continue;
            }
        };
        if !(s.duration.is_finite() && s.duration > 0.0) {
            errors.push(format!("{at}: duration {} ms must be above 0", s.duration));
        }
        let max_hz = s.max_hz.or(fx.max_hz);
        if let Some(hz) = s.max_hz {
            if !(hz.is_finite() && hz > 0.0) {
                errors.push(format!("{at}: max_hz {hz} must be above 0 (leave it out for no cap)"));
            }
        }
        let input = |path: &Option<String>, range: Option<[f32; 2]>, name: &str, errors: &mut Vec<String>, src: &mut dyn FnMut(&str, &str, &mut Vec<String>) -> usize| {
            path.as_ref().map(|p| {
                let input = range.map_or((0.0, 1.0), |[a, b]| (a, b));
                if input.0 == input.1 {
                    errors.push(format!("{at}: {name}_in [{}, {}] is empty", input.0, input.1));
                }
                Input { source: src(p, &format!("{at} {name}"), errors), shape: Shape { input, ..Shape::default() } }
            })
        };
        let rate_in = input(&s.rate, s.rate_in, "rate", &mut errors, &mut source);
        let level_in = input(&s.level, s.level_in, "level", &mut errors, &mut source);
        let flash = abs(&spec.flash);
        if let Some(other) = flash_channels.insert(flash, s.fixture.clone()) {
            errors.push(format!("{at}: DMX channel {flash} already flashes for '{other}'"));
        }
        plan.strobes.push(StrobePlan {
            fixture: s.fixture.clone(),
            flash,
            level: spec.level,
            rest: spec.rest.iter().map(|(c, v)| (abs(c), *v)).collect(),
            rate: Rate { neutral, mode, max_hz },
            rate_in,
            level_in,
            duration: s.duration.max(0.0) / 1000.0,
        });
    }

    for (i, m) in raw.map.iter().enumerate() {
        let at = format!("outputs.map[{i}] ({})", m.from);
        let src = source(&m.from, &at, &mut errors);
        if m.to.is_empty() {
            errors.push(format!("{at}: no targets (to = [...])"));
        }
        for (j, t) in m.to.iter().enumerate() {
            let at = format!("{at} to[{j}]");
            match target(t, &at, &fixtures, &flash_channels, &plan) {
                Ok((wire, shape, smooth)) => {
                    let smooth = smooth.or(m.smooth).unwrap_or(0.0);
                    if !(smooth.is_finite() && smooth >= 0.0) {
                        errors.push(format!("{at}: smooth {smooth} s must be 0 or more"));
                    }
                    plan.targets.push(Target { source: src, shape, smooth: smooth.max(0.0), wire });
                }
                Err(e) => errors.push(e),
            }
        }
    }
    plan.paths = paths;
    // two targets writing one DMX channel fight every frame
    let mut written: BTreeMap<u16, usize> = BTreeMap::new();
    for (k, t) in plan.targets.iter().enumerate() {
        if let Wire::Dmx { ch, fine } = t.wire {
            for c in [Some(ch), fine].into_iter().flatten() {
                if let Some(first) = written.insert(c, k) {
                    errors.push(format!("outputs.map: DMX channel {c} is written by '{}' and '{}'", plan.paths[plan.targets[first].source], plan.paths[t.source]));
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(plan)
    } else {
        Err(errors)
    }
}

fn target(t: &RawTarget, at: &str, fixtures: &BTreeMap<String, Placed>, flash: &BTreeMap<u16, String>, plan: &Plan) -> Result<(Wire, Shape, Option<f32>), String> {
    let kinds = [t.dmx.is_some(), t.cc.is_some(), t.note.is_some(), t.bend].iter().filter(|&&k| k).count();
    if kinds != 1 {
        return Err(format!("{at}: one of dmx, cc, note or bend (it has {kinds})"));
    }
    let shape = shape(&t.shape, at)?;
    let wire = if let Some(d) = &t.dmx {
        if t.channel.is_some() || t.velocity.is_some() || t.port.is_some() {
            return Err(format!("{at}: channel / velocity / port are MIDI's – a dmx target takes dmx and bits"));
        }
        if plan.dmx.is_none() {
            return Err(format!("{at}: a DMX target needs outputs.dmx (\"auto\" = the first USB-DMX widget found)"));
        }
        let (ch, fine) = match d {
            toml::Value::Integer(n) => {
                let ch = u16::try_from(*n).ok().filter(|c| (1..=512).contains(c)).ok_or_else(|| format!("{at}: dmx {n} is outside 1…512"))?;
                match t.bits {
                    None | Some(8) => (ch, None),
                    Some(16) if ch < 512 => (ch, Some(ch + 1)),
                    Some(16) => return Err(format!("{at}: a 16 bit channel at 512 has no fine channel")),
                    Some(b) => return Err(format!("{at}: bits {b} (8 or 16)")),
                }
            }
            toml::Value::String(s) => {
                if t.bits.is_some() {
                    return Err(format!("{at}: bits comes from the fixture profile for a named channel"));
                }
                let Some((fid, cname)) = s.split_once('.') else {
                    return Err(format!("{at}: dmx '{s}' – a fixture channel (fixture.channel, e.g. strobe.dimmer) or a number"));
                };
                let Some(fx) = fixtures.get(fid) else {
                    return Err(format!("{at}: no fixture '{fid}' (outputs.fixture ids: {})", fixtures.keys().cloned().collect::<Vec<_>>().join(", ")));
                };
                let Some(c) = fx.profile.channels.get(cname) else {
                    return Err(format!("{at}: fixture '{fid}' (profile {}) has no channel '{cname}' – it has {}", fx.profile.name, fx.profile.channel_names()));
                };
                (fx.address + c.coarse - 1, c.fine.map(|f| fx.address + f - 1))
            }
            other => return Err(format!("{at}: dmx = {other} – a fixture channel name or a channel number")),
        };
        if let Some(f) = flash.get(&ch) {
            return Err(format!("{at}: DMX channel {ch} is the strobe's flash channel ('{f}' – outputs.strobe drives it)"));
        }
        Wire::Dmx { ch, fine }
    } else {
        if t.bits.is_some() {
            return Err(format!("{at}: bits is for dmx targets"));
        }
        let channel = match t.channel {
            None => 0,
            Some(c @ 1..=16) => c as u8 - 1,
            Some(c) => return Err(format!("{at}: channel {c} (MIDI channels are 1…16)")),
        };
        let port = match t.port.as_deref() {
            None | Some("out") => MidiPort::Out,
            Some("conductor") => MidiPort::Conductor,
            Some(p) => return Err(format!("{at}: port '{p}' (out = the deck's MIDI out, conductor = over the relay)")),
        };
        if port == MidiPort::Out && plan.midi.is_none() {
            return Err(format!("{at}: a MIDI target needs outputs.midi (the out port's name) – or port = \"conductor\""));
        }
        let data = |n: i64, what: &str| u8::try_from(n).ok().filter(|v| *v <= 127).ok_or_else(|| format!("{at}: {what} {n} is outside 0…127"));
        let kind = if let Some(cc) = t.cc {
            MidiKind::Cc(data(cc, "cc")?)
        } else if let Some(n) = t.note {
            let velocity = match t.velocity {
                None => 100,
                Some(v @ 1..=127) => v as u8,
                Some(v) => return Err(format!("{at}: velocity {v} (1…127)")),
            };
            MidiKind::Note { note: data(n, "note")?, velocity }
        } else {
            MidiKind::Bend
        };
        if t.velocity.is_some() && !matches!(kind, MidiKind::Note { .. }) {
            return Err(format!("{at}: velocity is for note targets"));
        }
        Wire::Midi { port, channel, kind }
    };
    Ok((wire, shape, t.smooth))
}

fn shape(s: &RawShape, at: &str) -> Result<Shape, String> {
    let curve = match s.curve.as_deref() {
        None => Curve::Linear,
        Some(c) => Curve::parse(c).ok_or_else(|| format!("{at}: curve '{c}' ({})", Curve::NAMES))?,
    };
    let input = s.input.map_or((0.0, 1.0), |[a, b]| (a, b));
    if input.0 == input.1 || !input.0.is_finite() || !input.1.is_finite() {
        return Err(format!("{at}: in [{}, {}] is empty", input.0, input.1));
    }
    let (min, max) = (s.min.unwrap_or(0.0), s.max.unwrap_or(1.0));
    for (n, v) in [("min", min), ("max", max)] {
        if !(0.0..=1.0).contains(&v) {
            return Err(format!("{at}: {n} {v} – the output range is 0…1 of the target's scale"));
        }
    }
    Ok(Shape { input, curve, invert: s.invert, output: (min, max) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> toml::Table {
        let doc: toml::Table = toml::from_str(text).unwrap();
        doc["outputs"].as_table().unwrap().clone()
    }

    fn no_files(name: &str) -> Result<String, String> {
        Err(format!("no file {name}"))
    }

    const SCENE: &str = r#"
[outputs]
lead = 25
dmx = "auto"
midi = "UM-ONE"

[[outputs.fixture]]
id = "strobe"
profile = "strobe"
address = 10

[[outputs.fixture]]
id = "head"
profile = "head.toml"
address = 20

[[outputs.map]]
from = "space.glow"
smooth = 0.05
to = [
  { dmx = "head.dimmer", curve = "exp" },
  { dmx = "head.pan" },
  { cc = 74, channel = 2, invert = true },
  { cc = 20, port = "conductor", smooth = 0 },
]

[[outputs.map]]
from = "drone.freq"
to = [{ bend = true, in = [40, 120] }, { note = 36, velocity = 90, channel = 10 }, { dmx = 100, bits = 16 }]

[[outputs.strobe]]
fixture = "strobe"
rate = "sway.rate"
rate_in = [0, 2]
level = "space.glow"
neutral = "1/8"
duration = 40
"#;

    fn head(name: &str) -> Result<String, String> {
        match name {
            "head.toml" => Ok("name = \"head\"\n[channels]\ndimmer = 1\npan = { coarse = 2, fine = 3 }\n".into()),
            _ => no_files(name),
        }
    }

    #[test]
    fn a_full_scene_resolves() {
        let known: Vec<String> = ["space.glow", "drone.freq", "sway.rate"].map(String::from).to_vec();
        let plan = parse(&table(SCENE), Some(&known), &head).unwrap();
        assert_eq!(plan.paths, ["sway.rate", "space.glow", "drone.freq"]);
        assert!((plan.lead - 0.025).abs() < 1e-12);
        assert_eq!((plan.dmx.as_deref(), plan.midi.as_deref()), (Some(""), Some("UM-ONE")));
        assert_eq!(plan.targets.len(), 7);
        // head.dimmer at 20, head.pan 16 bit at 21 / 22 (from the profile)
        assert_eq!(plan.targets[0].wire, Wire::Dmx { ch: 20, fine: None });
        assert_eq!(plan.targets[0].shape.curve, Curve::Exp);
        assert_eq!(plan.targets[0].smooth, 0.05, "the map's smoothing");
        assert_eq!(plan.targets[1].wire, Wire::Dmx { ch: 21, fine: Some(22) });
        assert_eq!(plan.targets[2].wire, Wire::Midi { port: MidiPort::Out, channel: 1, kind: MidiKind::Cc(74) });
        assert!(plan.targets[2].shape.invert);
        assert_eq!(plan.targets[3].wire, Wire::Midi { port: MidiPort::Conductor, channel: 0, kind: MidiKind::Cc(20) });
        assert_eq!(plan.targets[3].smooth, 0.0, "a target's own smoothing wins");
        assert_eq!(plan.targets[4].wire, Wire::Midi { port: MidiPort::Out, channel: 0, kind: MidiKind::Bend });
        assert_eq!(plan.targets[4].shape.input, (40.0, 120.0));
        assert_eq!(plan.targets[5].wire, Wire::Midi { port: MidiPort::Out, channel: 9, kind: MidiKind::Note { note: 36, velocity: 90 } });
        assert_eq!(plan.targets[6].wire, Wire::Dmx { ch: 100, fine: Some(101) });
        assert!(plan.uses_conductor());
        let s = &plan.strobes[0];
        assert_eq!((s.flash, s.level, s.rest.clone()), (10, 255, vec![(12, 0), (11, 0)]));
        assert_eq!(strobe::DIVISIONS[s.rate.neutral].name, "1/8");
        assert_eq!(s.rate.max_hz, None, "no cap unless asked");
        assert_eq!(s.rate_in.as_ref().unwrap().shape.input, (0.0, 2.0));
        assert_eq!(s.level_in.as_ref().unwrap().source, 1);
        assert!((s.duration - 0.04).abs() < 1e-12);
    }

    #[test]
    fn errors_say_where_and_what() {
        let errs = |text: &str| parse(&table(text), Some(&["space.glow".to_owned()]), &no_files).unwrap_err().join("\n");
        let e = errs("[outputs]\ndmx = \"auto\"\n[[outputs.fixture]]\nid = \"s\"\nprofile = \"strobe\"\naddress = 1\n[[outputs.map]]\nfrom = \"space.glow\"\nto = [{ dmx = \"s.dimer\" }]\n");
        assert!(e.contains("outputs.map[0] (space.glow) to[0]") && e.contains("no channel 'dimer'") && e.contains("dimmer, duration, rate"), "{e}");
        let e = errs("[outputs]\ndmx = \"auto\"\n[[outputs.fixture]]\nid = \"s\"\nprofile = \"strobe\"\naddress = 1\n[[outputs.strobe]]\nfixture = \"s\"\n[[outputs.map]]\nfrom = \"space.glow\"\nto = [{ dmx = \"s.dimmer\" }]\n");
        assert!(e.contains("flash channel"), "{e}");
        let e = errs("[outputs]\n[[outputs.map]]\nfrom = \"space.glo\"\nto = [{ cc = 1 }]\n");
        assert!(e.contains("the deck has no parameter 'space.glo'") && e.contains("needs outputs.midi"), "{e}");
        let e = errs("[outputs]\nmidi = \"x\"\n[[outputs.map]]\nfrom = \"space.glow\"\nto = [{ cc = 200, channel = 17 }]\n");
        assert!(e.contains("cc 200") || e.contains("channel 17"), "{e}");
        let e = errs("[outputs]\nmidi = \"x\"\n[[outputs.map]]\nfrom = \"space.glow\"\nto = [{ cc = 1, note = 2 }, { curve = \"cubic\", cc = 3 }, { cc = 1, port = \"usb\" }]\n");
        assert!(e.contains("one of dmx, cc, note or bend (it has 2)") && e.contains("curve 'cubic'") && e.contains("port 'usb'"), "{e}");
        let e = errs("[outputs]\ndmx = \"auto\"\n[[outputs.fixture]]\nid = \"a\"\nprofile = \"strobe\"\naddress = 1\n[[outputs.fixture]]\nid = \"b\"\nprofile = \"strobe\"\naddress = 3\n[[outputs.fixture]]\nid = \"c\"\nprofile = \"strobe\"\naddress = 511\n[[outputs.fixture]]\nid = \"d\"\nprofile = \"gone.toml\"\naddress = 30\n");
        assert!(e.contains("DMX channel 3 is also fixture 'a'") && e.contains("run past 512") && e.contains("no file gone.toml"), "{e}");
        let e = errs("[outputs]\n[[outputs.fixture]]\nid = \"a\"\nprofile = \"strobe\"\naddress = 1\n");
        assert!(e.contains("fixtures need a DMX out"), "{e}");
        let e = errs("[outputs]\ndmx = \"auto\"\n[[outputs.fixture]]\nid = \"a\"\nprofile = \"strobe\"\naddress = 1\n[[outputs.strobe]]\nfixture = \"a\"\nneutral = \"1/3\"\n[[outputs.strobe]]\nfixture = \"z\"\n");
        assert!(e.contains("neutral '1/3'") && e.contains("1/16T") && e.contains("no fixture 'z'"), "{e}");
        let e = errs("[outputs]\ndmx = \"auto\"\n[[outputs.map]]\nfrom = \"space.glow\"\nto = [{ dmx = 5 }, { dmx = 4, bits = 16 }]\n");
        assert!(e.contains("DMX channel 5 is written by"), "{e}");
        let e = errs("[outputs]\nlead = 20\nbogus = 1\n");
        assert!(e.contains("outputs: unknown field `bogus`"), "{e}");
        let e = errs("[outputs]\nmidi = \"x\"\n[[outputs.map]]\nfrom = \"glow\"\nto = [{ cc = 1, in = [1, 1], min = 2 }]\n");
        assert!(e.contains("not a parameter path") && e.contains("in [1, 1] is empty"), "{e}");
    }

    #[test]
    fn max_hz_comes_from_the_fixture_or_the_strobe() {
        let text = "[outputs]\ndmx = \"auto\"\n[[outputs.fixture]]\nid = \"a\"\nprofile = \"strobe\"\naddress = 1\nmax_hz = 3\n[[outputs.strobe]]\nfixture = \"a\"\n";
        let plan = parse(&table(text), None, &no_files).unwrap();
        assert_eq!(plan.strobes[0].rate.max_hz, Some(3.0));
        let text = text.replace("[[outputs.strobe]]\nfixture = \"a\"\n", "[[outputs.strobe]]\nfixture = \"a\"\nmax_hz = 8\nmode = \"free\"\n");
        let plan = parse(&table(&text), None, &no_files).unwrap();
        assert_eq!((plan.strobes[0].rate.max_hz, plan.strobes[0].rate.mode), (Some(8.0), Mode::Free));
        assert!(plan.targets.is_empty() && !plan.is_empty());
    }
}
