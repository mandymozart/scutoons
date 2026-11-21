//! Spots (research/outlets-on-models-and-node-view.md §3.1): named points on a model – where a light, an emitter, a
//! sound source, a target or a module's port sits. A model's spots live on its asset version: the file's own (the
//! GLB's empties `spot_<name>[.<kind>]` and `port_<name>`, read by deck-space) and the hand edits over them. The
//! document the API stores per version (`spots`) is `Spots`: what the core read from the file and the hand edits, as
//! the core wrote them. `Spots::resolved` is the list a deck uses, `Spots::problems` the one check (`/validate` kind
//! `spots`).
use crate::Descriptor;
use serde::{Deserialize, Serialize};

/// What a spot is for – one word each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Light,
    Emitter,
    Sound,
    Target,
    Camera,
    /// where a cable of a module's port ends on the module's body
    Port,
    /// a named point without a use of its own
    #[default]
    Mark,
}

impl Kind {
    pub const ALL: [Kind; 7] = [Kind::Light, Kind::Emitter, Kind::Sound, Kind::Target, Kind::Camera, Kind::Port, Kind::Mark];

    pub fn word(self) -> &'static str {
        match self {
            Kind::Light => "light",
            Kind::Emitter => "emitter",
            Kind::Sound => "sound",
            Kind::Target => "target",
            Kind::Camera => "camera",
            Kind::Port => "port",
            Kind::Mark => "mark",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.word() == s)
    }
}

/// What a spot moves with: the model itself, one of its nodes or one of its bones. Written `root`, `node:<name>`,
/// `bone:<name>`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Attach {
    #[default]
    Root,
    Node(String),
    Bone(String),
}

impl Attach {
    pub fn is_root(&self) -> bool {
        *self == Attach::Root
    }
}

impl TryFrom<String> for Attach {
    type Error = String;

    fn try_from(s: String) -> Result<Attach, String> {
        match s.split_once(':') {
            None if s == "root" => Ok(Attach::Root),
            Some(("node", name)) if !name.is_empty() => Ok(Attach::Node(name.to_string())),
            Some(("bone", name)) if !name.is_empty() => Ok(Attach::Bone(name.to_string())),
            _ => Err(format!("'{s}' is no attachment (root, node:<name>, bone:<name>)")),
        }
    }
}

impl From<Attach> for String {
    fn from(a: Attach) -> String {
        a.to_string()
    }
}

impl std::fmt::Display for Attach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Attach::Root => f.write_str("root"),
            Attach::Node(n) => write!(f, "node:{n}"),
            Attach::Bone(b) => write!(f, "bone:{b}"),
        }
    }
}

/// Where a spot comes from: the model's file, or a hand edit (a spot added by hand, or a file spot changed by hand).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    #[default]
    File,
    Hand,
}

impl Source {
    fn is_file(&self) -> bool {
        *self == Source::File
    }
}

/// no turn: the spot looks along what it is attached to
pub const NO_TURN: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn no_turn() -> [f32; 4] {
    NO_TURN
}

fn is_no_turn(t: &[f32; 4]) -> bool {
    *t == NO_TURN
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// A named point on a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spot {
    pub name: String,
    #[serde(default)]
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Attach::is_root")]
    pub attach: Attach,
    /// metres, in the space of what it is attached to
    #[serde(default)]
    pub at: [f32; 3],
    /// its direction as a rotation (x, y, z, w) in that space: the spot points along its own −z
    #[serde(default = "no_turn", skip_serializing_if = "is_no_turn")]
    pub turn: [f32; 4],
    /// a port spot's port on the module's descriptor, when it is not the spot's name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(default, skip_serializing_if = "Source::is_file")]
    pub source: Source,
}

impl Spot {
    pub fn new(name: &str) -> Spot {
        Spot { name: name.to_string(), kind: Kind::Mark, attach: Attach::Root, at: [0.0; 3], turn: NO_TURN, port: None, source: Source::File }
    }

    /// the module port a port spot carries
    pub fn port_name(&self) -> Option<&str> {
        (self.kind == Kind::Port).then(|| self.port.as_deref().unwrap_or(&self.name))
    }
}

/// the prefix of an empty that is a spot
pub const SPOT_PREFIX: &str = "spot_";
/// the prefix of an empty that is a module body's port
pub const PORT_PREFIX: &str = "port_";

/// A node's name in a model's file as a spot: `spot_<name>` (a mark), `spot_<name>.<kind>`, `port_<name>` (the port
/// `<name>`). None: the node is no spot. The name is taken as written – `Spots::problems` says when it is none.
pub fn of_file_name(node: &str) -> Option<Spot> {
    if let Some(name) = node.strip_prefix(PORT_PREFIX) {
        return Some(Spot { kind: Kind::Port, ..Spot::new(name) });
    }
    let rest = node.strip_prefix(SPOT_PREFIX)?;
    Some(match rest.rsplit_once('.').and_then(|(name, kind)| Some((name, Kind::parse(kind)?))) {
        Some((name, kind)) => Spot { kind, ..Spot::new(name) },
        None => Spot::new(rest),
    })
}

/// A spot's name: one lowercase word – a letter first, then letters, digits and `_`, at most 64.
pub fn name(s: &str) -> Result<(), String> {
    let ok = s.len() <= 64 && s.bytes().next().is_some_and(|b| b.is_ascii_lowercase()) && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if ok {
        Ok(())
    } else {
        Err(format!("'{s}' is no spot name (one lowercase word: a letter first, then letters, digits or '_')"))
    }
}

/// One hand edit: a spot added by hand (`of` none), or a change of the file's spot `of` – whatever is given replaces
/// the file's, a different `name` renames it, `hide` drops it.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hand {
    pub name: String,
    /// the file spot it changes, by the file's name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hide: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach: Option<Attach>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<[f32; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
}

impl Hand {
    /// the edit laid over `base` (a file spot, or a new one)
    fn over(&self, base: &Spot) -> Spot {
        Spot {
            name: self.name.clone(),
            kind: self.kind.unwrap_or(base.kind),
            attach: self.attach.clone().unwrap_or_else(|| base.attach.clone()),
            at: self.at.unwrap_or(base.at),
            turn: self.turn.unwrap_or(base.turn),
            port: self.port.clone().or_else(|| base.port.clone()),
            source: Source::Hand,
        }
    }
}

/// What the core read from a model version's file: the names a spot can attach to, and the file's own spots.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    /// the named nodes
    #[serde(default)]
    pub nodes: Vec<String>,
    /// the joints of its skins
    #[serde(default)]
    pub bones: Vec<String>,
    #[serde(default)]
    pub spots: Vec<Spot>,
}

/// A model version's spots as the API stores them (`spots` of a version in `GET /api/assets/{id}`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spots {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<File>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hand: Vec<Hand>,
}

impl Spots {
    /// nothing to store
    pub fn none(&self) -> bool {
        self.file.is_none() && self.hand.is_empty()
    }

    fn file_spots(&self) -> &[Spot] {
        self.file.as_ref().map_or(&[], |f| &f.spots)
    }

    /// The model's spots: the file's with the hand edits over them, then the ones added by hand. A hand edit of a spot
    /// the file does not have changes nothing (`problems` reports it).
    pub fn resolved(&self) -> Vec<Spot> {
        let mut out = Vec::new();
        for f in self.file_spots() {
            match self.hand.iter().find(|h| h.of.as_deref() == Some(f.name.as_str())) {
                Some(h) if h.hide => {}
                Some(h) => out.push(h.over(f)),
                None => out.push(f.clone()),
            }
        }
        out.extend(self.hand.iter().filter(|h| h.of.is_none() && !h.hide).map(|h| h.over(&Spot::new(&h.name))));
        out
    }

    /// The spot of that name.
    pub fn spot(&self, name: &str) -> Option<Spot> {
        self.resolved().into_iter().find(|s| s.name == name)
    }

    /// These hand edits over another version's file (a re-export): the edits that still resolve stay, and each one
    /// that names a file spot the new file does not have is returned with its sentence.
    pub fn carried(&self, file: File) -> (Spots, Vec<String>) {
        let (kept, lost): (Vec<Hand>, Vec<Hand>) = self.hand.iter().cloned().partition(|h| h.of.as_ref().is_none_or(|of| file.spots.iter().any(|s| &s.name == of)));
        let said = lost.iter().map(|h| format!("the hand edit '{}' was of the file's spot '{}', which the new file does not have", h.name, h.of.as_deref().unwrap_or_default())).collect();
        (Spots { file: Some(file), hand: kept }, said)
    }

    /// Everything wrong with these spots (none = valid): names, doubles, hand edits that resolve to nothing, what a
    /// spot is attached to, its numbers, and – with the descriptor of the module this model is the body of – its
    /// port spots against the module's ports.
    pub fn problems(&self, module: Option<&Descriptor>) -> Vec<String> {
        let mut out = Vec::new();
        let file = self.file_spots();
        for (i, h) in self.hand.iter().enumerate() {
            match &h.of {
                Some(of) if !file.iter().any(|s| &s.name == of) => out.push(format!("the hand edit '{}' changes the file's spot '{of}', which this version's file does not have", h.name)),
                Some(of) if self.hand[..i].iter().any(|g| g.of.as_ref() == Some(of)) => out.push(format!("the file's spot '{of}' has two hand edits")),
                None if h.hide => out.push(format!("the hand edit '{}' hides nothing (only a file spot is hidden; a spot added by hand is removed)", h.name)),
                _ => {}
            }
        }
        let spots = self.resolved();
        let none = File::default();
        let names = self.file.as_ref().unwrap_or(&none);
        for (i, s) in spots.iter().enumerate() {
            if let Err(e) = name(&s.name) {
                out.push(match s.source {
                    Source::File => format!("the file's spot {e} – rename the empty, or rename it by hand"),
                    Source::Hand => format!("spot {e}"),
                });
            }
            if spots[..i].iter().any(|o| o.name == s.name) {
                out.push(format!("spot '{}' is there twice", s.name));
            }
            match &s.attach {
                Attach::Bone(b) if !names.bones.contains(b) => out.push(format!("spot '{}': the model has no bone '{b}'", s.name)),
                Attach::Node(n) if !names.nodes.contains(n) => out.push(format!("spot '{}': the model has no node '{n}'", s.name)),
                _ => {}
            }
            if !s.at.iter().all(|x| x.is_finite()) {
                out.push(format!("spot '{}': its position is not three numbers", s.name));
            }
            let length = s.turn.iter().map(|x| x * x).sum::<f32>().sqrt();
            if !length.is_finite() || (length - 1.0).abs() > 1e-3 {
                out.push(format!("spot '{}': its turn is no rotation (x, y, z, w of length 1)", s.name));
            }
            if s.kind != Kind::Port && s.port.is_some() {
                out.push(format!("spot '{}': only a port spot names a port (it is a {})", s.name, s.kind.word()));
            }
        }
        if let Some(m) = module {
            out.extend(port_problems(&spots, m));
        }
        out
    }
}

/// a module body's port spots against the module's ports: none twice, none missing, none extra
fn port_problems(spots: &[Spot], module: &Descriptor) -> Vec<String> {
    let mut out = Vec::new();
    let carried: Vec<(&Spot, &str)> = spots.iter().filter_map(|s| Some((s, s.port_name()?))).collect();
    for (i, (s, port)) in carried.iter().enumerate() {
        if let Some((first, _)) = carried[..i].iter().find(|(_, p)| p == port) {
            out.push(format!("the spots '{}' and '{}' both carry the port '{port}'", first.name, s.name));
        }
    }
    let known = |port: &str| module.ports.iter().any(|p| p.name == port);
    let extra: Vec<&str> = carried.iter().filter(|(_, port)| !known(port)).map(|(_, port)| *port).collect();
    let missing: Vec<&str> = module.ports.iter().map(|p| p.name.as_ref()).filter(|name| !carried.iter().any(|(_, port)| port == name)).collect();
    match (&extra[..], &missing[..]) {
        ([was], [is]) => out.push(format!("the port spot '{was}' names no port of the module '{}', and its port '{is}' has no spot – renamed?", module.name)),
        _ => {
            let all = || module.ports.iter().map(|p| p.name.as_ref()).collect::<Vec<_>>().join(", ");
            out.extend(extra.iter().map(|port| format!("the port spot '{port}' names no port of the module '{}' ({})", module.name, all())));
            out.extend(missing.iter().map(|port| format!("the module '{}' has the port '{port}', which has no spot on its body", module.name)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn file() -> File {
        File {
            nodes: vec!["rig".into(), "lamp".into()],
            bones: vec!["hand_l".into(), "head".into()],
            spots: vec![Spot { kind: Kind::Light, attach: Attach::Node("lamp".into()), at: [0.0, 0.5, 0.0], ..Spot::new("flame") }, Spot { attach: Attach::Bone("hand_l".into()), ..Spot::new("grip") }],
        }
    }

    #[test]
    fn a_file_name_is_a_spot_a_port_or_neither() {
        assert_eq!(of_file_name("spot_hand_l"), Some(Spot::new("hand_l")));
        assert_eq!(of_file_name("spot_flame.light"), Some(Spot { kind: Kind::Light, ..Spot::new("flame") }));
        assert_eq!(of_file_name("port_out"), Some(Spot { kind: Kind::Port, ..Spot::new("out") }));
        assert_eq!(of_file_name("spot_flame.001").unwrap().name, "flame.001", "no kind: the name as written, for the check to refuse");
        assert_eq!((of_file_name("view"), of_file_name("Spot_a"), of_file_name("spotlight")), (None, None, None));
        assert_eq!(of_file_name("port_out").unwrap().port_name(), Some("out"));
        assert_eq!(Spot { port: Some("gate".into()), ..of_file_name("port_a").unwrap() }.port_name(), Some("gate"));
        assert_eq!(Spot::new("a").port_name(), None);
    }

    #[test]
    fn a_name_is_one_lowercase_word() {
        for good in ["a", "hand_l", "nozzle2"] {
            assert!(name(good).is_ok(), "{good}");
        }
        for bad in ["", "Hand", "hand l", "2nd", "_x", "a.b", "a:b", "a/b", "ä", &"x".repeat(65)] {
            assert!(name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_stored_form_reads_back_and_leaves_out_what_is_default() {
        let spots = Spots {
            file: Some(file()),
            hand: vec![
                Hand { name: "flame".into(), of: Some("flame".into()), at: Some([0.0, 0.75, 0.0]), ..Hand::default() },
                Hand { name: "ear".into(), kind: Some(Kind::Sound), attach: Some(Attach::Bone("head".into())), turn: Some([0.0, 1.0, 0.0, 0.0]), ..Hand::default() },
            ],
        };
        let v = serde_json::to_value(&spots).unwrap();
        assert_eq!(
            v,
            json!({
                "file": { "nodes": ["rig", "lamp"], "bones": ["hand_l", "head"], "spots": [
                    { "name": "flame", "kind": "light", "attach": "node:lamp", "at": [0.0, 0.5, 0.0] },
                    { "name": "grip", "kind": "mark", "attach": "bone:hand_l", "at": [0.0, 0.0, 0.0] }
                ] },
                "hand": [
                    { "name": "flame", "of": "flame", "at": [0.0, 0.75, 0.0] },
                    { "name": "ear", "kind": "sound", "attach": "bone:head", "turn": [0.0, 1.0, 0.0, 0.0] }
                ]
            })
        );
        assert_eq!(serde_json::from_value::<Spots>(v).unwrap(), spots);
        assert_eq!(serde_json::to_value(Spots::default()).unwrap(), json!({}));
        assert!(Spots::default().none() && !spots.none());
        assert!(serde_json::from_value::<Spots>(json!({ "hand": [{ "name": "a", "attach": "limb:x" }] })).is_err());
        assert!(serde_json::from_value::<Spots>(json!({ "hand": [{ "name": "a", "size": 2 }] })).is_err(), "an unknown field is refused");
    }

    #[test]
    fn hand_edits_move_rename_hide_and_add() {
        let mut spots = Spots { file: Some(file()), hand: vec![] };
        assert_eq!(spots.resolved(), file().spots, "no edit: the file's");
        spots.hand = vec![
            Hand { name: "wick".into(), of: Some("flame".into()), at: Some([0.0, 0.7, 0.0]), ..Hand::default() },
            Hand { name: "grip".into(), of: Some("grip".into()), hide: true, ..Hand::default() },
            Hand { name: "ear".into(), kind: Some(Kind::Sound), attach: Some(Attach::Bone("head".into())), ..Hand::default() },
        ];
        let got = spots.resolved();
        assert_eq!(got.iter().map(|s| (s.name.as_str(), s.kind, s.source)).collect::<Vec<_>>(), [("wick", Kind::Light, Source::Hand), ("ear", Kind::Sound, Source::Hand)]);
        assert_eq!((got[0].at, &got[0].attach), ([0.0, 0.7, 0.0], &Attach::Node("lamp".into())), "what the edit leaves out stays the file's");
        assert_eq!(spots.problems(None), Vec::<String>::new());
        assert!(spots.spot("grip").is_none() && spots.spot("ear").is_some());
    }

    #[test]
    fn the_check_names_bad_names_doubles_and_what_is_not_there() {
        let mut f = file();
        f.spots.push(Spot::new("Hand L"));
        let spots = Spots {
            file: Some(f),
            hand: vec![
                Hand { name: "flame".into(), ..Hand::default() },
                Hand { name: "tip".into(), attach: Some(Attach::Bone("wrist".into())), ..Hand::default() },
                Hand { name: "knob".into(), attach: Some(Attach::Node("dial".into())), at: Some([f32::NAN, 0.0, 0.0]), turn: Some([0.0; 4]), port: Some("gate".into()), ..Hand::default() },
                Hand { name: "old".into(), of: Some("wick".into()), ..Hand::default() },
                Hand { name: "gone".into(), hide: true, ..Hand::default() },
            ],
        };
        assert_eq!(
            spots.problems(None),
            [
                "the hand edit 'old' changes the file's spot 'wick', which this version's file does not have",
                "the hand edit 'gone' hides nothing (only a file spot is hidden; a spot added by hand is removed)",
                "the file's spot 'Hand L' is no spot name (one lowercase word: a letter first, then letters, digits or '_') – rename the empty, or rename it by hand",
                "spot 'flame' is there twice",
                "spot 'tip': the model has no bone 'wrist'",
                "spot 'knob': the model has no node 'dial'",
                "spot 'knob': its position is not three numbers",
                "spot 'knob': its turn is no rotation (x, y, z, w of length 1)",
                "spot 'knob': only a port spot names a port (it is a mark)",
            ]
        );
        let renamed = Spots { hand: vec![Hand { name: "hand_l".into(), of: Some("Hand L".into()), ..Hand::default() }], ..spots.clone() };
        assert_eq!(renamed.problems(None), Vec::<String>::new(), "a bad file name is healed by a hand rename");
        let twice = Spots { file: Some(file()), hand: vec![Hand { name: "a".into(), of: Some("grip".into()), ..Hand::default() }, Hand { name: "b".into(), of: Some("grip".into()), ..Hand::default() }] };
        assert_eq!(twice.problems(None), ["the file's spot 'grip' has two hand edits"]);
        let unread = Spots { file: None, hand: vec![Hand { name: "a".into(), attach: Some(Attach::Bone("head".into())), ..Hand::default() }] };
        assert_eq!(unread.problems(None), ["spot 'a': the model has no bone 'head'"], "no file read: nothing to attach to but the root");
    }

    fn module(ports: &[&str]) -> Descriptor {
        let ports: Vec<_> = ports.iter().map(|p| json!({ "name": p, "dir": "out", "type": "audio" })).collect();
        serde_json::from_value(json!({ "name": "claw", "version": "1", "domain": "audio", "ports": ports })).unwrap()
    }

    #[test]
    fn a_module_body_s_port_spots_match_its_ports() {
        let port = |name: &str| Spot { kind: Kind::Port, ..Spot::new(name) };
        let body = |spots: Vec<Spot>| Spots { file: Some(File { spots, ..File::default() }), hand: vec![] };
        let m = module(&["in", "out"]);
        assert_eq!(body(vec![port("in"), port("out"), Spot::new("logo")]).problems(Some(&m)), Vec::<String>::new());
        assert_eq!(body(vec![port("in"), Spot { port: Some("out".into()), ..port("nozzle") }]).problems(Some(&m)), Vec::<String>::new(), "a port spot may carry another name");
        assert_eq!(body(vec![port("in"), port("output")]).problems(Some(&m)), ["the port spot 'output' names no port of the module 'claw', and its port 'out' has no spot – renamed?"]);
        assert_eq!(
            body(vec![port("in"), port("out"), port("aux"), Spot { port: Some("in".into()), ..port("second") }]).problems(Some(&m)),
            ["the spots 'in' and 'second' both carry the port 'in'", "the port spot 'aux' names no port of the module 'claw' (in, out)"]
        );
        assert_eq!(body(vec![]).problems(Some(&m)), ["the module 'claw' has the port 'in', which has no spot on its body", "the module 'claw' has the port 'out', which has no spot on its body"]);
        assert_eq!(body(vec![port("aux")]).problems(None), Vec::<String>::new(), "no module: the ports are not asked");
    }

    #[test]
    fn a_re_export_keeps_the_hand_edits_that_still_resolve() {
        let spots = Spots {
            file: Some(file()),
            hand: vec![Hand { name: "wick".into(), of: Some("flame".into()), ..Hand::default() }, Hand { name: "grip".into(), of: Some("grip".into()), hide: true, ..Hand::default() }, Hand { name: "ear".into(), ..Hand::default() }],
        };
        let new = File { spots: vec![Spot::new("grip"), Spot::new("toe")], ..File::default() };
        let (carried, said) = spots.carried(new);
        assert_eq!(carried.hand.iter().map(|h| h.name.as_str()).collect::<Vec<_>>(), ["grip", "ear"]);
        assert_eq!(said, ["the hand edit 'wick' was of the file's spot 'flame', which the new file does not have"]);
        assert_eq!(carried.resolved().iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["toe", "ear"]);
    }
}
