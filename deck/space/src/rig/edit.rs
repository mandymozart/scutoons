//! Editing a rig, an instrument or a preset by graph gestures (#434, research/MODULE-WIRING.md §2 and §5 `wire_edit`):
//! one op rewrites the one home of a cable or node, in the document's own words – a rig's `route`, an instrument's
//! `link` and `node`, a preset's `link` / `unlink`. Nothing here checks the result as a whole: that is the validation the
//! core has (validate.rs `Request`), which the editor runs after each op – the Compile for data (§2). Platform-free, for
//! the native editor (#444; owner 2026-11-09: hand-wiring UI is native only); no GUI here.
//!
//! - **Connect** `from → to`: in a rig the ends are `<track>:<node>.<port>` (and `being:<input>`, `policy:<port>`),
//!   relative only (W4); both ends in one track is the instrument's business and refused. In an instrument the ends are
//!   `node.port`. A cable into a control input replaces the one there (the library tells the port's type; an unknown
//!   kind adds, and the check says "already has a link"). In a preset it is a re-route (`link`).
//! - **Disconnect** `to` (from `from`, or every cable into it): a rig's route, an instrument's link; in a preset its own
//!   link goes, an instrument's is listed in `unlink`.
//! - **Add / remove a node** (an instrument): a node with its params; removing takes its cables and its `expose` along
//!   and clears an `out` that named it – the instrument is silent then, and the check says whether that holds.
//! - **Add / remove a track** (a rig): a track of a known instrument; removing takes its routes, its persona and mood
//!   entries and its sticks along.
//! - **"used by N"** (`used_by`): which tracks of which rigs play an instrument – the editor says it before an edit of a
//!   shared instrument (EDITOR.md §4). The rigs are the catalog's list, handed in.
//! - **The text view** (`Doc::to_toml` / `from_toml`; owner 2026-11-08: TOML if it makes sense long-term, identifiably
//!   related to the nodes): the same serde form the core sets ship in (deck/native/rigs) and the deck image reads; the
//!   ids in the text are the view's ids. Stored as JSON as today; comments are not kept (§6 Q5).
use super::address::{Address, Leaf};
use super::bundle::Bundle;
use super::wiring::Home;
use super::instrument::canonical;
use super::{Instrument, Library, Preset, Rig, Route, Track};
use crate::graph::{Kind, PortType};
use crate::scene::{LinkDef, NodeDef};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A document the editor changes, by its catalog kind (validate.rs `Request::kind`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "document", rename_all = "lowercase")]
pub enum Doc {
    Rigs(Rig),
    Instruments(Instrument),
    Presets(Preset),
}

/// One gesture of the graph as a change of a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    Connect {
        from: String,
        to: String,
        #[serde(default = "one")]
        scale: f32,
        #[serde(default)]
        offset: f32,
    },
    /// a track's fields, those given (#456: the mixer and the assignment move into the editor)
    SetTrack {
        id: String,
        #[serde(default)]
        instrument: Option<String>,
        #[serde(default)]
        preset: Option<String>,
        #[serde(default)]
        gain: Option<f32>,
        #[serde(default)]
        pan: Option<f32>,
        #[serde(default)]
        mute: Option<bool>,
    },
    Disconnect {
        to: String,
        #[serde(default)]
        from: Option<String>,
    },
    AddNode {
        id: String,
        kind: String,
        #[serde(default)]
        params: toml::Table,
    },
    RemoveNode {
        id: String,
    },
    AddTrack {
        id: String,
        instrument: String,
        #[serde(default)]
        preset: Option<String>,
    },
    RemoveTrack {
        id: String,
    },
}

fn one() -> f32 {
    1.0
}

/// a name of the grammar (rig/address.rs): no `/ : . @ #`, no space, not empty
fn word(s: &str, what: &str) -> Result<(), String> {
    match Address::parse(s) {
        Ok(a) if a.is_relative() && matches!(a.leaf, Leaf::Port { track: None, ref port, .. } if port == "out") && !s.contains('.') => Ok(()),
        _ => Err(format!("'{s}' is not {what} (letters, digits, `_` and `-`)")),
    }
}

/// the port type of `node.port` in `nodes`, when the kind is known
fn input_type(nodes: &[NodeDef], node: &str, port: &str) -> Option<PortType> {
    let n = nodes.iter().find(|n| n.id == node)?;
    let kind = Kind::parse(&n.kind)?;
    kind.input(port).map(|i| kind.inputs()[i].ty)
}

impl Doc {
    pub fn kind(&self) -> &'static str {
        match self {
            Doc::Rigs(_) => "rigs",
            Doc::Instruments(_) => "instruments",
            Doc::Presets(_) => "presets",
        }
    }

    /// the catalog id: a rig's subject, an instrument's or preset's id
    pub fn id(&self) -> &str {
        match self {
            Doc::Rigs(r) => &r.subject,
            Doc::Instruments(i) => &i.id,
            Doc::Presets(p) => &p.id,
        }
    }

    /// the document as TOML – the form the core sets ship in
    pub fn to_toml(&self) -> Result<String> {
        let text = match self {
            Doc::Rigs(r) => toml::to_string(r),
            Doc::Instruments(i) => toml::to_string(i),
            Doc::Presets(p) => toml::to_string(p),
        };
        text.with_context(|| format!("{} {} as TOML", self.kind(), self.id()))
    }

    /// a document from its TOML (`kind` as validate.rs names them)
    pub fn from_toml(kind: &str, text: &str) -> Result<Doc, String> {
        let say = |e: toml::de::Error| format!("{kind}: {e}");
        match kind {
            "rigs" => toml::from_str(text).map(Doc::Rigs).map_err(say),
            "instruments" => toml::from_str(text).map(Doc::Instruments).map_err(say),
            "presets" => toml::from_str(text).map(Doc::Presets).map_err(say),
            other => Err(format!("unknown kind '{other}' (instruments, presets, rigs)")),
        }
    }

    /// `op` applied to this document (see the module's doc); `Err` in the document's words, the document unchanged
    pub fn apply(&mut self, op: &Op, lib: &Library) -> Result<(), String> {
        match self {
            Doc::Rigs(r) => rig(r, op, lib),
            Doc::Instruments(i) => instrument(i, op),
            Doc::Presets(p) => preset(p, op, lib),
        }
    }
}

fn rig(r: &mut Rig, op: &Op, lib: &Library) -> Result<(), String> {
    let who = format!("rig {}", r.subject);
    match op {
        Op::Connect { from, to, scale, offset } => {
            r.route_end(from, false)?;
            r.route_end(to, true)?;
            let (a, b) = (Address::leaf(from)?, Address::leaf(to)?);
            if let (Leaf::Port { track: Some(x), .. }, Leaf::Port { track: Some(y), .. }) = (&a, &b) {
                if x == y {
                    return Err(format!("{who}: both ends are in track '{x}' – inside a track the instrument's `link` is the cable's home: edit the instrument"));
                }
            }
            // a control input takes one cable: the new one replaces it (the being's inputs are control)
            let control = match &b {
                Leaf::Port { track: Some(t), node, port } => {
                    let inst = r.tracks.iter().find(|x| &x.id == t).and_then(|x| lib.instruments.get(&x.instrument));
                    inst.and_then(|i| input_type(&i.nodes, node, port)) == Some(PortType::Control)
                }
                Leaf::Being(_) => true,
                _ => false,
            };
            if control {
                r.routes.retain(|x| Address::leaf(&x.to).ok().as_ref() != Some(&b));
            }
            r.routes.push(Route { from: from.clone(), to: to.clone(), scale: *scale, offset: *offset });
            Ok(())
        }
        Op::Disconnect { to, from } => {
            let b = Address::leaf(to).map_err(|e| format!("{who}: '{to}': {e}"))?;
            let a = from.as_deref().map(Address::leaf).transpose().map_err(|e| format!("{who}: {e}"))?;
            let before = r.routes.len();
            r.routes.retain(|x| !(Address::leaf(&x.to).ok().as_ref() == Some(&b) && a.as_ref().is_none_or(|a| Address::leaf(&x.from).ok().as_ref() == Some(a))));
            if r.routes.len() == before {
                return Err(format!("{who}: no route into '{to}'{}", from.as_deref().map(|f| format!(" from '{f}'")).unwrap_or_default()));
            }
            Ok(())
        }
        Op::SetTrack { id, instrument, preset, gain, pan, mute } => {
            if let Some(i) = instrument {
                if !lib.instruments.contains_key(i) {
                    return Err(format!("{who}: unknown instrument \"{i}\""));
                }
            }
            let t = r.tracks.iter_mut().find(|t| &t.id == id).ok_or_else(|| format!("{who}: no track '{id}'"))?;
            if let Some(i) = instrument {
                t.instrument = i.clone();
            }
            if let Some(p) = preset {
                t.preset = Some(p.clone());
            }
            if let Some(g) = gain {
                t.gain = *g;
            }
            if let Some(p) = pan {
                t.pan = *p;
            }
            if let Some(m) = mute {
                t.mute = *m;
            }
            Ok(())
        }
        Op::AddNode { .. } | Op::RemoveNode { .. } => Err(format!("{who}: a rig has tracks, not nodes – add a track, or edit the track's instrument")),
        Op::AddTrack { id, instrument, preset } => {
            word(id, "a track id")?;
            if r.tracks.iter().any(|t| &t.id == id) {
                return Err(format!("{who}: track '{id}' is there already"));
            }
            if id == super::policy::NODE || id == super::address::BEING {
                return Err(format!("{who}: the track id '{id}' is reserved"));
            }
            if !lib.instruments.contains_key(instrument) {
                return Err(format!("{who}: unknown instrument \"{instrument}\""));
            }
            r.tracks.push(Track { id: id.clone(), instrument: instrument.clone(), preset: preset.clone(), gain: 1.0, pan: 0.0, mute: false });
            Ok(())
        }
        Op::RemoveTrack { id } => {
            let before = r.tracks.len();
            r.tracks.retain(|t| &t.id != id);
            if r.tracks.len() == before {
                return Err(format!("{who}: no track '{id}'"));
            }
            let in_track = |s: &str| matches!(Address::leaf(s), Ok(Leaf::Port { track: Some(t), .. }) if &t == id);
            r.routes.retain(|x| !in_track(&x.from) && !in_track(&x.to));
            for v in r.personas.values_mut().chain(r.moods.values_mut()) {
                v.presets.remove(id);
                v.values.retain(|k, _| !in_track(k));
            }
            if let Some(sticks) = r.sticks.as_mut() {
                for s in sticks.iter_mut().flatten() {
                    if in_track(s) {
                        s.clear();
                    }
                }
            }
            Ok(())
        }
    }
}

fn instrument(i: &mut Instrument, op: &Op) -> Result<(), String> {
    let who = format!("instrument {}", i.id);
    let end = |s: &str| -> Result<(String, String), String> {
        match Address::leaf(s).map_err(|e| format!("{who}: '{s}': {e}"))? {
            Leaf::Port { track: None, node, port } => {
                if !i.nodes.iter().any(|n| n.id == node) {
                    return Err(format!("{who}: no node '{node}' ({s})"));
                }
                Ok((node, port))
            }
            _ => Err(format!("{who}: '{s}' – inside an instrument a cable end is node.port")),
        }
    };
    match op {
        Op::Connect { from, to, scale, offset } => {
            end(from)?;
            let (node, port) = end(to)?;
            if input_type(&i.nodes, &node, &port) == Some(PortType::Control) {
                i.links.retain(|l| canonical(&l.to) != canonical(to));
            }
            i.links.push(LinkDef { from: from.clone(), to: to.clone(), scale: *scale, offset: *offset, map: Vec::new() });
            Ok(())
        }
        Op::Disconnect { to, from } => {
            let before = i.links.len();
            i.links.retain(|l| !(canonical(&l.to) == canonical(to) && from.as_deref().is_none_or(|f| canonical(&l.from) == canonical(f))));
            if i.links.len() == before {
                return Err(format!("{who}: no link into '{to}'{}", from.as_deref().map(|f| format!(" from '{f}'")).unwrap_or_default()));
            }
            Ok(())
        }
        Op::AddNode { id, kind, params } => {
            word(id, "a node id")?;
            if i.nodes.iter().any(|n| &n.id == id) {
                return Err(format!("{who}: node '{id}' is there already"));
            }
            if Kind::parse(kind).is_none() && !kind.starts_with(crate::modreg::PREFIX) {
                return Err(format!("{who}: unknown kind '{kind}'"));
            }
            i.nodes.push(NodeDef { id: id.clone(), kind: kind.clone(), params: params.clone() });
            Ok(())
        }
        Op::RemoveNode { id } => {
            let before = i.nodes.len();
            i.nodes.retain(|n| &n.id != id);
            if i.nodes.len() == before {
                return Err(format!("{who}: no node '{id}'"));
            }
            let of_node = |s: &str| super::address::node_port(s).0 == id;
            i.links.retain(|l| !of_node(&l.from) && !of_node(&l.to));
            i.expose.retain(|e| !of_node(&e.path));
            if i.out.as_deref().is_some_and(of_node) {
                i.out = None;
            }
            Ok(())
        }
        Op::SetTrack { .. } => Err(format!("{who}: a track is the rig's")),
        Op::AddTrack { .. } | Op::RemoveTrack { .. } => Err(format!("{who}: an instrument has nodes, not tracks – edit the rig")),
    }
}

fn preset(p: &mut Preset, op: &Op, lib: &Library) -> Result<(), String> {
    let who = format!("preset {}", p.id);
    match op {
        Op::Connect { from, to, scale, offset } => {
            for s in [from, to] {
                if !matches!(Address::leaf(s), Ok(Leaf::Port { track: None, .. })) {
                    return Err(format!("{who}: '{s}' – a preset's re-route joins node.port ends of its instrument"));
                }
            }
            p.links.retain(|l| canonical(&l.to) != canonical(to));
            p.unlink.retain(|u| canonical(u) != canonical(to));
            p.links.push(LinkDef { from: from.clone(), to: to.clone(), scale: *scale, offset: *offset, map: Vec::new() });
            Ok(())
        }
        Op::Disconnect { to, .. } => {
            let before = p.links.len();
            p.links.retain(|l| canonical(&l.to) != canonical(to));
            if p.links.len() == before {
                // the instrument's own link: the preset removes it
                let has = lib.instruments.get(&p.instrument).is_some_and(|i| i.links.iter().any(|l| canonical(&l.to) == canonical(to)));
                if !has {
                    return Err(format!("{who}: no link into '{to}', neither the preset's nor the instrument's"));
                }
                if !p.unlink.iter().any(|u| canonical(u) == canonical(to)) {
                    p.unlink.push(to.clone());
                }
            }
            Ok(())
        }
        _ => Err(format!("{who}: a preset holds values and re-routes – nodes and tracks are the instrument's and the rig's")),
    }
}

/// both ends in one track: that track and the ends as its instrument names them (`node.port`)
fn inside_track(from: &str, to: &str) -> Result<Option<(String, String, String)>, String> {
    Ok(match (Address::leaf(from)?, Address::leaf(to)?) {
        (Leaf::Port { track: Some(x), node: n1, port: p1 }, Leaf::Port { track: Some(y), node: n2, port: p2 }) if x == y => Some((x, format!("{n1}.{p1}"), format!("{n2}.{p2}"))),
        _ => None,
    })
}

impl Bundle {
    fn on_instrument(&self, track: &str, op: &Op) -> Result<Bundle, String> {
        let id = &self.rig.tracks.iter().find(|t| t.id == track).ok_or_else(|| format!("rig {}: no track '{track}'", self.rig.subject))?.instrument;
        let at = self.instruments.iter().position(|i| &i.id == id).ok_or_else(|| format!("rig {}: unknown instrument \"{id}\"", self.rig.subject))?;
        let mut doc = Doc::Instruments(self.instruments[at].clone());
        doc.apply(op, &self.library())?;
        let Doc::Instruments(changed) = doc else { unreachable!("an instrument stays an instrument") };
        let mut next = self.clone();
        next.instruments[at] = changed;
        Ok(next)
    }

    fn on_rig(&self, op: &Op) -> Result<Bundle, String> {
        let mut doc = Doc::Rigs(self.rig.clone());
        doc.apply(op, &self.library())?;
        let Doc::Rigs(rig) = doc else { unreachable!("a rig stays a rig") };
        Ok(Bundle { rig, ..self.clone() })
    }

    /// A track's fields set on the rig (the mixer, the inspector).
    pub fn track_set(&self, op: &Op) -> Result<Bundle, String> {
        self.on_rig(op)
    }

    /// A cable drawn between two `track:node.port` ends, on the document that holds it (#431): inside one track its
    /// instrument's `link`, between tracks the rig's `route`.
    pub fn connected(&self, from: &str, to: &str) -> Result<Bundle, String> {
        match inside_track(from, to)? {
            Some((track, f, t)) => self.on_instrument(&track, &Op::Connect { from: f, to: t, scale: 1.0, offset: 0.0 }),
            None => self.on_rig(&Op::Connect { from: from.to_string(), to: to.to_string(), scale: 1.0, offset: 0.0 }),
        }
    }

    /// The cable `from` → `to` cut, on the document its `home` says (#431); the sentence why when no gesture may change it.
    pub fn cut(&self, from: &str, to: &str, home: &Home) -> Result<Bundle, String> {
        match home {
            Home::Route { .. } => self.on_rig(&Op::Disconnect { to: to.to_string(), from: Some(from.to_string()) }),
            Home::Instrument { track } => {
                let (_, f, t) = inside_track(from, to)?.ok_or_else(|| format!("{from} → {to} is not inside track '{track}'"))?;
                self.on_instrument(track, &Op::Disconnect { to: t, from: Some(f) })
            }
            Home::Preset { preset, .. } => Err(format!("{from} → {to} is a re-route of preset {preset}: edit the preset")),
            Home::Compiled => Err(format!("{from} → {to} is made by the rig's flatten: it cannot be cut")),
            Home::Bound => Err(format!("{from} → {to} is bound by the module's Rust: it cannot be cut")),
        }
    }
}

/// Which tracks of which rigs play `instrument`: (subject, track), in the rigs' order – "used by N" before an edit.
pub fn used_by(rigs: &[Rig], instrument: &str) -> Vec<(String, String)> {
    rigs.iter().flat_map(|r| r.tracks.iter().filter(|t| t.instrument == instrument).map(|t| (r.subject.clone(), t.id.clone()))).collect()
}

/// `{ kind, document, op, instruments?: [..] }` → the changed document as JSON (the hosts' form; the check is
/// validate.rs's `rig_problems` afterwards)
pub fn edit_json(request: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct Request {
        #[serde(flatten)]
        doc: Doc,
        op: Op,
        #[serde(default)]
        instruments: Vec<Instrument>,
    }
    let mut r: Request = serde_json::from_str(request).context("not an edit request { kind, document, op, instruments? }")?;
    let lib = Library::new(r.instruments.clone(), []);
    r.doc.apply(&r.op, &lib).map_err(anyhow::Error::msg)?;
    Ok(serde_json::to_string(&r.doc)?)
}

/// `{ kind, document }` → `{ "toml": "…" }`
pub fn to_toml_json(request: &str) -> Result<String> {
    let doc: Doc = serde_json::from_str(request).context("not a document { kind, document }")?;
    Ok(serde_json::json!({ "toml": doc.to_toml()? }).to_string())
}

/// `{ kind, toml }` → the document as JSON (`{ kind, document }`)
pub fn from_toml_json(request: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct Request {
        kind: String,
        toml: String,
    }
    let r: Request = serde_json::from_str(request).context("not a text { kind, toml }")?;
    let doc = Doc::from_toml(&r.kind, &r.toml).map_err(anyhow::Error::msg)?;
    Ok(serde_json::to_string(&doc)?)
}

#[cfg(test)]
mod gesture_tests {
    use super::*;

    fn catpope() -> Bundle {
        Bundle::core("character.catpope").expect("the core rig").expect("it reads")
    }

    #[test]
    fn a_cable_inside_a_track_lands_in_the_instrument() {
        let b = catpope();
        let next = b.connected("drone:drone.out", "drone:fifth.cv").expect("inside the track");
        assert_eq!(next.rig, b.rig, "the rig is not its home");
        assert_ne!(next.instruments, b.instruments);
    }

    #[test]
    fn a_tracks_fields_are_set_on_the_rig() {
        let b = catpope();
        let next = b.track_set(&Op::SetTrack { id: "voice".into(), instrument: None, preset: None, gain: Some(0.5), pan: None, mute: Some(true) }).expect("set");
        let t = next.rig.tracks.iter().find(|t| t.id == "voice").expect("the track");
        assert_eq!((t.gain, t.mute), (0.5, true));
        assert!(b.track_set(&Op::SetTrack { id: "nope".into(), instrument: None, preset: None, gain: None, pan: None, mute: None }).is_err());
    }

    #[test]
    fn a_cable_between_tracks_lands_in_the_rig() {
        let b = catpope();
        let next = b.connected("voice:voice.out", "drone:mix.in3").expect("between tracks");
        assert_eq!(next.rig.routes.len(), b.rig.routes.len() + 1);
        assert_eq!(next.instruments, b.instruments);
    }

    #[test]
    fn a_cut_goes_to_the_home_of_the_cable() {
        let b = catpope();
        let home = Home::Instrument { track: "voice".into() };
        let next = b.cut("voice:melody.cv", "voice:voice.cv", &home).expect("the instrument's link");
        assert_ne!(next.instruments, b.instruments);
        let drawn = b.connected("voice:voice.out", "drone:mix.in3").expect("drawn");
        let back = drawn.cut("voice:voice.out", "drone:mix.in3", &Home::Route { index: 0 }).expect("the route");
        assert_eq!(back.rig.routes, b.rig.routes);
    }

    #[test]
    fn what_no_gesture_may_change_says_so() {
        let b = catpope();
        assert!(b.cut("a:b.c", "d:e.f", &Home::Compiled).expect_err("compiled").contains("cannot be cut"));
        assert!(b.cut("a:b.c", "d:e.f", &Home::Bound).expect_err("bound").contains("cannot be cut"));
        assert!(b.cut("a:b.c", "a:e.f", &Home::Preset { track: "a".into(), preset: "p".into() }).expect_err("preset").contains("edit the preset"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::Bundle;

    const SET: &str = r#"
        [rig]
        subject = "character.two"
        track = [{ id = "a", instrument = "sway" }, { id = "b", instrument = "tone" }]
        route = [{ from = "a:lfo.out", to = "b:osc.freq", scale = 20.0 }]
        sticks = [["b:osc.freq", "a:lfo.rate"], ["", ""]]
        [rig.moods.sad]
        presets = { b = "tone.low" }
        values = { "b:osc.gain" = 0.05, "a:lfo.rate" = 0.2 }
        [[instrument]]
        id = "sway"
        kind = "pad"
        node = [{ id = "lfo", kind = "lfo", rate = 0.5 }, { id = "lfo2", kind = "lfo" }, { id = "o", kind = "osc", gain = 0.0 }]
        out = "o"
        expose = [{ path = "lfo.rate" }]
        [[instrument]]
        id = "tone"
        kind = "pad"
        node = [{ id = "osc", kind = "osc", freq = 110.0 }]
        out = "osc"
        expose = [{ path = "osc.freq" }]
        [[preset]]
        id = "tone.low"
        instrument = "tone"
        values = { "osc.freq" = 55.0 }
    "#;

    fn set() -> (Bundle, Library) {
        let b: Bundle = toml::from_str(SET).unwrap();
        let lib = b.library();
        (b, lib)
    }

    fn connect(from: &str, to: &str) -> Op {
        Op::Connect { from: from.into(), to: to.into(), scale: 1.0, offset: 0.0 }
    }

    #[test]
    fn a_rigs_cables_are_its_routes_and_a_control_input_takes_one() {
        let (b, lib) = set();
        let mut d = Doc::Rigs(b.rig.clone());
        let routes = |d: &Doc| if let Doc::Rigs(r) = d { r.routes.iter().map(|x| format!("{} → {}", x.from, x.to)).collect::<Vec<_>>() } else { unreachable!() };
        d.apply(&connect("a:lfo.out", "being:glow"), &lib).unwrap();
        assert_eq!(routes(&d), ["a:lfo.out → b:osc.freq", "a:lfo.out → being:glow"]);
        // another cable into the same control input replaces it
        d.apply(&connect("a:lfo2.out", "b:osc.freq"), &lib).unwrap();
        assert_eq!(routes(&d), ["a:lfo.out → being:glow", "a:lfo2.out → b:osc.freq"]);
        d.apply(&Op::Disconnect { to: "being:glow".into(), from: None }, &lib).unwrap();
        assert_eq!(routes(&d), ["a:lfo2.out → b:osc.freq"]);
        assert!(d.apply(&Op::Disconnect { to: "being:glow".into(), from: None }, &lib).unwrap_err().contains("no route into"));
        for (from, to, says) in [
            ("a:lfo.out", "a:o.freq", "inside a track"),
            ("/deck:left/scene:x/character:two/a:lfo.out", "b:osc.freq", "leaves the rig"),
            ("nobody:x.y", "b:osc.freq", "no track 'nobody'"),
            ("a:lfo.out", "b:@gain", "is not a port"),
        ] {
            let e = d.apply(&connect(from, to), &lib).unwrap_err();
            assert!(e.contains(says), "{from} → {to}: {e}");
        }
        assert_eq!(routes(&d), ["a:lfo2.out → b:osc.freq"], "a refused op changes nothing");
        assert!(d.apply(&Op::AddNode { id: "x".into(), kind: "osc".into(), params: Default::default() }, &lib).unwrap_err().contains("tracks, not nodes"));
        // the result is a rig like any: checked and played
        let Doc::Rigs(r) = d else { unreachable!() };
        r.check(&lib).unwrap();
        let b2 = Bundle { rig: r, ..b };
        b2.play(None, None).unwrap();
    }

    #[test]
    fn a_track_comes_and_goes_with_its_routes_variants_and_sticks() {
        let (b, lib) = set();
        let mut d = Doc::Rigs(b.rig);
        d.apply(&Op::AddTrack { id: "c".into(), instrument: "tone".into(), preset: Some("tone.low".into()) }, &lib).unwrap();
        assert!(d.apply(&Op::AddTrack { id: "c".into(), instrument: "tone".into(), preset: None }, &lib).unwrap_err().contains("there already"));
        assert!(d.apply(&Op::AddTrack { id: "d".into(), instrument: "kazoo".into(), preset: None }, &lib).unwrap_err().contains("unknown instrument"));
        assert!(d.apply(&Op::AddTrack { id: "bad.id".into(), instrument: "tone".into(), preset: None }, &lib).unwrap_err().contains("not a track id"));
        d.apply(&connect("c:osc.out", "being:pulse"), &lib).unwrap();
        d.apply(&Op::RemoveTrack { id: "b".into() }, &lib).unwrap();
        let Doc::Rigs(r) = &d else { unreachable!() };
        assert_eq!(r.tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["a", "c"]);
        assert_eq!(r.routes.iter().map(|x| x.to.as_str()).collect::<Vec<_>>(), ["being:pulse"], "b's route went with it");
        let sad = &r.moods["sad"];
        assert!(sad.presets.is_empty() && sad.values.keys().eq(["a:lfo.rate"]), "{sad:?}");
        assert_eq!(r.sticks.as_ref().unwrap()[0], ["", "a:lfo.rate"]);
        r.check(&lib).unwrap();
        assert!(d.apply(&Op::RemoveTrack { id: "b".into() }, &lib).unwrap_err().contains("no track 'b'"));
    }

    #[test]
    fn an_instrument_gains_and_loses_nodes_and_cables() {
        let (b, lib) = set();
        let mut d = Doc::Instruments(b.instruments[1].clone());
        let links = |d: &Doc| if let Doc::Instruments(i) = d { i.links.iter().map(|l| format!("{} → {}", l.from, l.to)).collect::<Vec<_>>() } else { unreachable!() };
        d.apply(&Op::AddNode { id: "wob".into(), kind: "lfo".into(), params: toml::toml! { rate = 3.0 }.clone() }, &lib).unwrap();
        d.apply(&connect("wob", "osc.freq"), &lib).unwrap();
        d.apply(&Op::AddNode { id: "wob2".into(), kind: "lfo".into(), params: Default::default() }, &lib).unwrap();
        d.apply(&connect("wob2.out", "osc.freq"), &lib).unwrap();
        assert_eq!(links(&d), ["wob2.out → osc.freq"], "a control input takes one cable");
        d.apply(&connect("wob", "osc.gain"), &lib).unwrap();
        d.apply(&Op::RemoveNode { id: "wob2".into() }, &lib).unwrap();
        assert_eq!(links(&d), ["wob → osc.gain"], "its cable went with it");
        for (op, says) in [
            (Op::AddNode { id: "wob".into(), kind: "lfo".into(), params: Default::default() }, "there already"),
            (Op::AddNode { id: "z".into(), kind: "blender".into(), params: Default::default() }, "unknown kind"),
            (connect("nobody", "osc.gain"), "no node 'nobody'"),
            (connect("a:wob.out", "osc.gain"), "node.port"),
            (Op::RemoveNode { id: "nobody".into() }, "no node"),
            (Op::AddTrack { id: "t".into(), instrument: "tone".into(), preset: None }, "nodes, not tracks"),
        ] {
            assert!(d.apply(&op, &lib).unwrap_err().contains(says), "{op:?}");
        }
        d.apply(&Op::AddNode { id: "m".into(), kind: "module:nobody".into(), params: Default::default() }, &lib).unwrap();
        let Doc::Instruments(i) = &d else { unreachable!() };
        assert_eq!(i.nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), ["osc", "wob", "m"]);
        let mut d2 = d.clone();
        d2.apply(&Op::RemoveNode { id: "osc".into() }, &lib).unwrap();
        let Doc::Instruments(i2) = &d2 else { unreachable!() };
        assert!(i2.out.is_none() && i2.links.is_empty() && i2.expose.is_empty(), "the out, its cables and its exposed params went with it");
    }

    #[test]
    fn a_preset_reroutes_and_unlinks() {
        let (b, lib) = set();
        let mut d = Doc::Presets(b.presets[0].clone());
        assert!(d.apply(&connect("a:osc.out", "osc.gain"), &lib).unwrap_err().contains("node.port ends"));
        let (mut sway, lib2) = {
            let b2: Bundle = toml::from_str(&SET.replace("instrument = \"tone\"\n        values = { \"osc.freq\" = 55.0 }", "instrument = \"sway\"")).unwrap();
            (Doc::Presets(b2.presets[0].clone()), b2.library())
        };
        sway.apply(&connect("lfo", "o.gain"), &lib2).unwrap();
        sway.apply(&Op::Disconnect { to: "o.gain".into(), from: None }, &lib2).unwrap();
        assert!(sway.apply(&Op::Disconnect { to: "o.gain".into(), from: None }, &lib2).unwrap_err().contains("no link into"));
        assert!(d.apply(&Op::AddNode { id: "x".into(), kind: "osc".into(), params: Default::default() }, &lib).unwrap_err().contains("values and re-routes"));
        let Doc::Presets(p) = &sway else { unreachable!() };
        assert!(p.links.is_empty() && p.unlink.is_empty());
    }

    #[test]
    fn used_by_names_every_track_that_plays_an_instrument() {
        let (b, _) = set();
        let mut other = b.rig.clone();
        other.subject = "character.other".into();
        let rigs = [b.rig.clone(), other];
        assert_eq!(used_by(&rigs, "tone"), [("character.two".to_string(), "b".to_string()), ("character.other".to_string(), "b".to_string())]);
        assert!(used_by(&rigs, "kazoo").is_empty());
    }

    /// the text view: the core rigs' documents through TOML and back are the same documents
    #[test]
    fn documents_round_trip_through_toml() {
        let b = Bundle::core("character.catpope").unwrap().unwrap();
        let docs = [Doc::Rigs(b.rig.clone()), Doc::Instruments(b.instruments[0].clone()), Doc::Presets(b.presets[1].clone())];
        for d in docs {
            let text = d.to_toml().unwrap();
            assert!(text.contains(d.id()), "{text}");
            let back = Doc::from_toml(d.kind(), &text).unwrap_or_else(|e| panic!("{e}\n{text}"));
            assert_eq!(back, d, "{text}");
        }
        assert!(Doc::from_toml("rigs", "subject = 1").unwrap_err().starts_with("rigs: "));
        assert!(Doc::from_toml("items", "").unwrap_err().contains("unknown kind"));
    }

    #[test]
    fn the_hosts_forms() {
        let (b, _) = set();
        let rig = serde_json::to_value(&b.rig).unwrap();
        let req = serde_json::json!({ "kind": "rigs", "document": rig, "op": { "op": "disconnect", "to": "b:osc.freq" } }).to_string();
        let out: serde_json::Value = serde_json::from_str(&edit_json(&req).unwrap()).unwrap();
        assert_eq!(out["kind"], "rigs");
        assert!(out["document"].get("route").is_none(), "{out}");
        let text: serde_json::Value = serde_json::from_str(&to_toml_json(&serde_json::json!({ "kind": "rigs", "document": out["document"] }).to_string()).unwrap()).unwrap();
        assert!(text["toml"].as_str().unwrap().starts_with("subject = \"character.two\""), "{text}");
        let back: serde_json::Value = serde_json::from_str(&from_toml_json(&serde_json::json!({ "kind": "rigs", "toml": text["toml"] }).to_string()).unwrap()).unwrap();
        assert_eq!(back["document"]["subject"], "character.two");
        assert!(edit_json("{}").is_err());
    }
}
