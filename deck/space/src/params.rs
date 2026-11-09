//! The control inputs as parameters (#188 phase 3): each kind's ports with a range, a curve and the default the stage
//! uses when neither a link nor a param sets them – one table for the renderer's passes, the being's 3D buttons and
//! (later) MIDI / OSC learn and #189's float-vector mapping. Audio kinds' defaults are the engine's (`default_for`).
use crate::graph::Kind;

/// an LFO's shapes by name (its `shape` param), in the order its shape index counts them (deck-native's control.rs,
/// the being's waveform button)
pub const LFO_SHAPES: [&str; 3] = ["sine", "saw", "square"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Curve {
    Lin,
    /// equal ratios per equal travel (frequencies, times, rates); min must be > 0
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spec {
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub curve: Curve,
    /// #429: only whole values (a switch, a choice, a count): the knob clicks from one to the next
    pub whole: bool,
}

impl Spec {
    /// 0..1, linear: a path without a range of its own
    pub const UNIT: Spec = Spec { min: 0.0, max: 1.0, default: 0.0, curve: Curve::Lin, whole: false };

    /// a knob's range as a deck's scene names it (#369: the page's sliders, a rig's knobs – `log` for pitches, cutoffs, rates)
    pub fn knob(min: f32, max: f32, default: f32, log: bool) -> Spec {
        Spec { min, max, default, curve: if log { Curve::Log } else { Curve::Lin }, whole: false }
    }

    /// value → 0..1 (clamped)
    pub fn norm(&self, v: f32) -> f32 {
        let n = match self.curve {
            Curve::Lin => (v - self.min) / (self.max - self.min),
            Curve::Log => (v.max(self.min).ln() - self.min.ln()) / (self.max.ln() - self.min.ln()),
        };
        if n.is_finite() {
            n.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// 0..1 → value
    pub fn value(&self, n: f32) -> f32 {
        let n = n.clamp(0.0, 1.0);
        let v = match self.curve {
            Curve::Lin => self.min + n * (self.max - self.min),
            Curve::Log => (self.min.ln() + n * (self.max.ln() - self.min.ln())).exp(),
        };
        if self.whole {
            v.round().clamp(self.min, self.max)
        } else {
            v
        }
    }
}

/// the space node's control inputs that are post knobs (#204, post.rs): no button unless `expose` names them
pub const POST_PORTS: [&str; 7] = ["bloom", "threshold", "lift", "gamma", "gain", "pixel", "levels"];

/// One entry of the being's `expose` list (#424): `"node"` (all its control inputs), `"node.port"`, or a counted group
/// `"node.step1..16 <= seq.steps"` – the ports `step1` to `step16` of `node`, of which only as many as the VALUE of the
/// parameter on the right (rounded, clamped to the group's size) are shown, the first ones, live (a hidden knob keeps
/// its value). The plain strings keep their meaning; the list's order is the knobs' order on a node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Expose<'a> {
    Node(&'a str),
    Port { node: &'a str, port: &'a str },
    Group { node: &'a str, stem: &'a str, from: u32, to: u32, count: (&'a str, &'a str) },
}

/// A group member's limit: knob number `index` (1 = the first of the group) of `size`, shown while the parameter
/// `count` (node id, port name) holds at least `index`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limit<'a> {
    pub count: (&'a str, &'a str),
    pub index: u32,
    pub size: u32,
}

impl<'a> Expose<'a> {
    pub fn parse(e: &'a str) -> Expose<'a> {
        let group = || -> Option<Expose<'a>> {
            let (lhs, rhs) = e.split_once("<=")?;
            let (node, range) = lhs.trim().split_once('.')?;
            let (head, to) = range.split_once("..")?;
            let stem = head.trim_end_matches(|c: char| c.is_ascii_digit());
            let (from, to) = (head[stem.len()..].parse().ok()?, to.trim().parse().ok()?);
            let (cnode, cport) = rhs.trim().split_once('.')?;
            (from <= to).then_some(Expose::Group { node, stem, from, to, count: (cnode, cport) })
        };
        group().unwrap_or_else(|| match e.split_once('.') {
            Some((node, port)) => Expose::Port { node, port },
            None => Expose::Node(e),
        })
    }

    /// whether this entry names the control input `node.port` (a node entry: any port of it)
    pub fn names(&self, node: &str, port: &str) -> bool {
        match *self {
            Expose::Node(n) => n == node,
            Expose::Port { node: n, port: p } => (n, p) == (node, port),
            Expose::Group { .. } => self.limit(node, port).is_some(),
        }
    }

    /// the limit of `node.port` when this entry is a group naming it
    pub fn limit(&self, node: &str, port: &str) -> Option<Limit<'a>> {
        let Expose::Group { node: n, stem, from, to, count } = *self else { return None };
        let k: u32 = port.strip_prefix(stem)?.parse().ok()?;
        (n == node && (from..=to).contains(&k)).then(|| Limit { count, index: k - from + 1, size: to - from + 1 })
    }
}

/// #429: whether `node.port` is the count parameter of a counted group in `expose` (a count is a whole number)
pub fn is_count(node: &str, port: &str, expose: &[String]) -> bool {
    expose.iter().any(|e| matches!(Expose::parse(e), Expose::Group { count, .. } if count == (node, port)))
}

/// how many of a group of `size` knobs the count parameter's `value` shows: rounded, clamped 0..=size (not a number: all)
pub fn shown_count(value: f32, size: u32) -> u32 {
    if value.is_finite() {
        value.round().clamp(0.0, size as f32) as u32
    } else {
        size
    }
}

/// the first limit `expose` puts on `node.port` (None = always shown)
pub fn limit<'a>(node: &str, port: &str, expose: &'a [String]) -> Option<Limit<'a>> {
    expose.iter().find_map(|e| Expose::parse(e).limit(node, port))
}

/// where `expose` first names `node.port` – the knob's place among its node's knobs (a port named by nothing but its
/// node, or by an empty list, keeps its own order after the named ones)
pub fn place(node: &str, port: &str, expose: &[String]) -> usize {
    expose.iter().position(|e| Expose::parse(e).names(node, port)).unwrap_or(usize::MAX)
}

/// Whether a scene shows the control input `node.port` – the being's 3D button, the deck page's knob (#360): the being's
/// `expose` names the node ("node", all its inputs), the port ("node.port") or a counted group (`Expose`); empty = every
/// control input – except the space node's post knobs, shown only when named. (Whether a counted knob is shown NOW is the
/// being's: `limit`.)
pub fn shown(kind: Kind, node: &str, port: &str, expose: &[String]) -> bool {
    let named = expose.iter().any(|e| !matches!(Expose::parse(e), Expose::Node(_)) && Expose::parse(e).names(node, port));
    // the bounce is wired, not turned (#455)
    if kind == Kind::Space && (POST_PORTS.contains(&port) || port == "bounce") {
        return named;
    }
    named || expose.is_empty() || expose.iter().any(|e| Expose::parse(e) == Expose::Node(node))
}

/// the default of `kind`'s control input `port` (what an unlinked input without a param reads)
pub fn default(kind: Kind, port: &str) -> f32 {
    spec(kind, port).default
}

/// a control input's value when neither the node nor a link sets it: the catalog's default (deck-catalog, #315; NaN =
/// not driven until a param or a link sets it, an events node's knobs), a module's own for `module:*`, else 0
pub fn default_for(kind: Kind, port: &str) -> f32 {
    match kind {
        // a module's parameters: the defaults its descriptor declares (modreg.rs)
        Kind::Module(m) => m.default_for(port),
        _ => kind.descriptor().and_then(|d| d.param(port)).map_or(0.0, |p| p.default.unwrap_or(f32::NAN)),
    }
}

/// `kind`'s control input `port` as a parameter: its range, curve and default from the catalog; a module's ports and
/// unknown ones read 0..1 linear
pub fn spec(kind: Kind, port: &str) -> Spec {
    // #429: a module has no catalog descriptor here, its parameters' ranges are in the registry
    if let Kind::Module(m) = kind {
        if let Some(&(_, min, max, curve, whole)) = m.spec().ranges.iter().find(|r| r.0 == port) {
            let log = curve == deck_catalog::Curve::Log && min > 0.0;
            return Spec { min, max, default: default_for(kind, port), curve: if log { Curve::Log } else { Curve::Lin }, whole };
        }
    }
    match kind.descriptor().and_then(|d| d.param(port)) {
        Some(p) => Spec {
            min: p.min,
            max: p.max,
            default: p.default.unwrap_or(f32::NAN),
            curve: match p.curve {
                deck_catalog::Curve::Lin => Curve::Lin,
                deck_catalog::Curve::Log => Curve::Log,
            },
            whole: !p.choices.is_empty(),
        },
        None => Spec { min: 0.0, max: 1.0, default: default_for(kind, port), curve: Curve::Lin, whole: false },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #424: the counted group in `expose`: the plain forms unchanged, the group's members limited by the count parameter
    #[test]
    fn a_counted_group_names_its_members_and_limits_them() {
        let ex: Vec<String> = ["seq.steps", "seq.step1..16 <= seq.steps", "synth", "seq.on1..16<=seq.steps"].map(String::from).into();
        assert_eq!(Expose::parse("seq.steps"), Expose::Port { node: "seq", port: "steps" });
        assert_eq!(Expose::parse("synth"), Expose::Node("synth"));
        assert_eq!(
            Expose::parse(&ex[1]),
            Expose::Group { node: "seq", stem: "step", from: 1, to: 16, count: ("seq", "steps") }
        );
        // every member is shown by the scene, plain entries as before
        assert!(shown(Kind::Fm, "seq", "step16", &ex) && shown(Kind::Fm, "seq", "on3", &ex) && shown(Kind::Fm, "synth", "cutoff", &ex));
        assert!(!shown(Kind::Fm, "seq", "step17", &ex) && !shown(Kind::Fm, "seq", "rate", &ex), "outside the group, not named");
        assert!(!shown(Kind::Fm, "other", "step3", &ex));
        // the limit: the knob's number in its group and the group's size; plain ports have none
        assert_eq!(limit("seq", "step3", &ex), Some(Limit { count: ("seq", "steps"), index: 3, size: 16 }));
        assert_eq!(limit("seq", "on16", &ex).map(|l| l.index), Some(16));
        assert_eq!(limit("seq", "steps", &ex), None);
        assert_eq!(limit("synth", "cutoff", &ex), None);
        // a group that doesn't start at 1 counts from its first member; a malformed one is just a name that matches nothing
        let late = Expose::parse("a.k3..6 <= a.n");
        assert_eq!(late.limit("a", "k5").map(|l| (l.index, l.size)), Some((3, 4)));
        assert_eq!(late.limit("a", "k2"), None);
        assert!(!Expose::parse("a.k..6 <= a.n").names("a", "k1"));
        // the list's order is the place: steps first
        assert!(place("seq", "steps", &ex) < place("seq", "step1", &ex) && place("seq", "step1", &ex) < place("seq", "on1", &ex));
        assert_eq!(place("seq", "rate", &ex), usize::MAX);
    }

    #[test]
    fn the_count_is_the_rounded_value_clamped_to_the_group() {
        assert_eq!([0.0, 0.4, 0.6, 6.0, 6.49, 16.0, 17.0, -3.0].map(|v| shown_count(v, 16)), [0, 0, 1, 6, 6, 16, 16, 0]);
        assert_eq!(shown_count(f32::NAN, 16), 16, "not driven: all of them");
    }

    #[test]
    fn norm_and_value_round_trip() {
        for s in [spec(Kind::Fm, "freq"), spec(Kind::Fm, "index"), spec(Kind::Lfo, "rate"), spec(Kind::Space, "hue")] {
            for n in [0.0, 0.25, 0.5, 1.0] {
                assert!((s.norm(s.value(n)) - n).abs() < 1e-4, "{s:?} at {n}");
            }
            assert_eq!(s.norm(s.max * 10.0), 1.0);
        }
        let f = spec(Kind::Osc, "freq");
        assert!((f.value(0.5) - (20.0f32 * 4000.0).sqrt()).abs() < 0.1, "log: the middle is the geometric mean");
    }

    #[test]
    fn defaults_are_the_engines_and_the_passes() {
        assert_eq!(default(Kind::Fm, "index"), 1.0);
        assert_eq!(default(Kind::Noise, "cutoff"), 4000.0);
        assert_eq!(default(Kind::AudioOut, "volume"), 0.8);
        assert_eq!(default(Kind::Shader, "a"), 0.5);
        assert_eq!(default(Kind::Shader, "d"), 0.0);
        assert_eq!(default(Kind::Lfo, "rate"), 0.25);
        assert_eq!(default(Kind::Chapter, "hold"), 6.0);
    }
}
