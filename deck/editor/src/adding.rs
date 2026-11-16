//! Adding and taking away in Build: a node in a track (its kind from the core's list; a far world node with its holder
//! from the address book), a track (an instrument of the toon's catalog or the rig's own). The kinds, the ids and whether
//! the result may stand are the core's (rig/edit.rs, validate); this offers them and says what happened.
use crate::inspector::Sel;
use crate::wires::{self, Port};
use deck_kit::editor::typeahead::{Item, Typeahead};
use deck_space::piece::Found;
use deck_space::rig::address::Place;
use deck_space::rig::edit::node_kinds;
use deck_space::rig::{Bundle, Instrument};

/// a row of the Add typeahead
#[derive(Debug, Clone, PartialEq)]
pub enum Choice {
    Node { track: String, kind: &'static str },
    /// its holder is picked from the address book next
    Far { track: String },
    Track(Box<Instrument>),
}

/// what may be added: nodes into `track` when there is one, tracks of `instruments` at the rig
pub fn choices(track: Option<&str>, at_rig: bool, instruments: &[Instrument]) -> (Vec<Item>, Vec<Choice>) {
    let mut rows: Vec<(String, String, Choice)> = Vec::new();
    if let Some(t) = track {
        rows.push(("world · far".into(), format!("node in {t} · its holder from the address book"), Choice::Far { track: t.to_string() }));
        rows.extend(node_kinds().into_iter().map(|k| (k.to_string(), format!("node in {t}"), Choice::Node { track: t.to_string(), kind: k })));
    }
    if at_rig {
        rows.extend(instruments.iter().map(|i| (i.id.clone(), format!("track · {}", format!("{:?}", i.kind).to_lowercase()), Choice::Track(Box::new(i.clone())))));
    }
    rows.into_iter().enumerate().map(|(id, (label, detail, c))| (Item { id, label, detail }, c)).unzip()
}

/// the catalog's instruments and the rig's own, one each, by id
pub fn instruments(bundle: &Bundle, catalog: Vec<Instrument>) -> Vec<Instrument> {
    let mut all = catalog;
    for i in &bundle.instruments {
        if !all.iter().any(|x| x.id == i.id) {
            all.push(i.clone());
        }
    }
    all.sort_by(|a, b| a.id.cmp(&b.id));
    all
}

/// a choice made (not `Far`: that waits for its holder): the bundle as the core checks it, and the words
pub fn apply(bundle: &Bundle, c: &Choice) -> Result<(Bundle, String), String> {
    let (next, words) = match c {
        Choice::Node { track, kind } => bundle.node_added(track, kind).map(|(b, id)| (b, format!("{track}:{id} added")))?,
        Choice::Track(i) => bundle.track_added(i).map(|(b, id)| (b, format!("track {id} added, playing {}", i.id)))?,
        Choice::Far { track } => return Err(format!("a far node in {track} needs its holder first")),
    };
    Ok((wires::checked(next)?, words))
}

/// a far world node into `track`, listening to `holder` (its rig `held`)
pub fn far(bundle: &Bundle, track: &str, holder: &Place, held: &Bundle) -> Result<(Bundle, String), String> {
    let (next, id) = bundle.far_added(track, holder, held)?;
    Ok((wires::checked(next)?, format!("{track}:{id} listens to {holder}")))
}

/// the track and the node id of graph node `id` (`<track>_<node>`), from its ports
pub fn of_node(ports: &[Port], id: &str) -> Option<(String, String)> {
    let p = ports.iter().find(|p| p.node == id && !p.track.is_empty())?;
    Some((p.track.clone(), id.strip_prefix(&format!("{}_", p.track))?.to_string()))
}

/// a node taken away
pub fn remove_node(bundle: &Bundle, track: &str, node: &str) -> Result<(Bundle, String), String> {
    Ok((wires::checked(bundle.node_removed(track, node)?)?, format!("{track}:{node} removed")))
}

/// a track taken away
pub fn remove_track(bundle: &Bundle, track: &str) -> Result<(Bundle, String), String> {
    Ok((wires::checked(bundle.track_removed(track)?)?, format!("track {track} removed")))
}

/// Build's Add and Remove on the editor (main.rs): the typeahead, the pick, the selection taken away, a check run's
impl crate::App {
    /// the Add typeahead: inside a track (or with one selected) its nodes, at the rig a new track
    pub(crate) fn open_add(&mut self, x: f32, y: f32) {
        let track = match &self.sel {
            _ if !self.stage.path.is_empty() => self.stage.path.last().cloned(),
            Sel::Track(t) => Some(t.clone()),
            Sel::Node(n) => self.stage.node_ids.get(*n).and_then(|id| of_node(&self.stage.ports, id)).map(|(t, _)| t),
            Sel::Rig => None,
        };
        let all = instruments(&self.stage.bundle, self.rigs.instruments());
        let (items, add) = choices(track.as_deref(), self.stage.path.is_empty(), &all);
        self.menu = Some(crate::Menu { ta: Typeahead::new(items, x, y), from: None, far: None, rigs: false, add, far_new: None });
    }

    /// a row of Add chosen: a far node asks for its holder next, the rest lands at once
    pub(crate) fn added(&mut self, c: Choice, x: f32, y: f32) {
        if let Choice::Far { track } = c {
            let ta = self.holders(x, y);
            self.menu = Some(crate::Menu { ta, from: None, far: None, rigs: false, add: Vec::new(), far_new: Some(track) });
            return;
        }
        match apply(&self.stage.bundle, &c) {
            Ok((next, words)) => self.change(Ok(next), words, x, y),
            Err(why) => self.say(why, x, y, true),
        }
    }

    /// the holder picked from the book for a new far node in `track`: its rig is the core's, the check run's folder's or
    /// the catalog's
    pub(crate) fn far_added(&mut self, track: &str, entry: usize, x: f32, y: f32) {
        let Some(address) = self.book.book.as_ref().and_then(|b| b.entry(entry)).and_then(|e| e.address.clone()) else { return };
        let dir = self.show.knobs.rigs.clone();
        let find = |p: &Place| crate::show::values::rig_file(dir.as_deref(), p).or_else(|| self.rigs.bundle_of(p).map(|b| Found::Bundle(Box::new(b))));
        let done = deck_space::holders::opened(&address, find).and_then(|o| {
            let holder = Place::parse(&address).map_err(|e| format!("{address}: {e}"))?;
            far(&self.stage.bundle, track, &holder, &o.bundle)
        });
        match done {
            Ok((next, words)) => self.change(Ok(next), words, x, y),
            Err(why) => self.say(why, x, y, true),
        }
    }

    /// the selected node or track taken away
    pub(crate) fn remove_selected(&mut self) {
        let (x, y) = self.cursor;
        let done = match self.sel.clone() {
            Sel::Track(t) => {
                if self.stage.path.first() == Some(&t) {
                    self.up_to(0);
                }
                remove_track(&self.stage.bundle, &t)
            }
            Sel::Node(n) => match self.stage.node_ids.get(n).and_then(|id| of_node(&self.stage.ports, id)) {
                Some((t, node)) => remove_node(&self.stage.bundle, &t, &node),
                None => Err("this node is the rig's own, not a track's: nothing to remove".into()),
            },
            Sel::Rig => return,
        };
        match done {
            Ok((next, words)) => {
                self.sel = Sel::Rig;
                self.change(Ok(next), words, x, y);
            }
            Err(why) => self.say(why, x, y, true),
        }
    }

    /// a check run's Add: open it at the Add pill, type, pick; a far node's holder then from `--far`
    pub(crate) fn script_add(&mut self) {
        const START: u64 = 60;
        let Some((query, pick)) = self.args.add.clone() else { return };
        let at = self.panel.hits.iter().find(|(_, h)| *h == crate::Hit::Add).map_or(self.cursor, |(r, _)| (r.x, r.y + r.h));
        let type_in = |app: &mut crate::App, q: &str| {
            if let Some(m) = app.menu.as_mut() {
                m.ta.insert(q);
            }
        };
        let choose = |app: &mut crate::App| {
            if let Some(id) = app.menu.as_ref().and_then(|m| m.ta.chosen()) {
                app.pick(id);
            }
        };
        match self.frames {
            f if f == START => {
                // the rig may have changed since frame 0 (a cell opened in Build): `--select` names a track of it
                if let Some(t) = self.args.select.clone().filter(|t| self.stage.bundle.rig.tracks.iter().any(|x| x.id == *t)) {
                    self.sel = Sel::Track(t);
                }
                self.open_add(at.0, at.1);
            }
            f if f == START + 10 => type_in(self, &query),
            f if f == START + 20 && pick => choose(self),
            f if f == START + 30 => {
                if let Some((q, _)) = self.args.far.clone() {
                    type_in(self, &q);
                }
            }
            f if f == START + 40 && self.args.far.as_ref().is_some_and(|(_, p)| *p) => choose(self),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(text: &str) -> Bundle {
        toml::from_str(text).expect("a fixture rig")
    }

    fn keys() -> Bundle {
        fixture(include_str!("../../demos/rigs/keys.toml"))
    }

    fn bells() -> Bundle {
        fixture(include_str!("../../demos/rigs/bells.toml"))
    }

    #[test]
    fn inside_a_track_nodes_at_the_rig_tracks() {
        let all = instruments(&keys(), bells().instruments);
        let (items, rows) = choices(Some("tone"), false, &all);
        assert_eq!(items.len(), rows.len());
        assert_eq!(rows[0], Choice::Far { track: "tone".into() });
        assert!(items.iter().any(|i| i.label == "osc") && !items.iter().any(|i| i.label == "screen"));
        let (_, at_rig) = choices(None, true, &all);
        let ids: Vec<&str> = at_rig.iter().filter_map(|c| if let Choice::Track(i) = c { Some(i.id.as_str()) } else { None }).collect();
        assert_eq!(ids, ["bells_air", "from_bells", "from_keys", "keys_tone"]);
    }

    #[test]
    fn a_node_and_a_track_come_and_go_through_the_core() {
        let b = keys();
        let (next, words) = apply(&b, &Choice::Node { track: "tone".into(), kind: "lfo" }).expect("a node");
        assert_eq!(words, "tone:lfo added");
        let ports = wires::ports(&next.play(None, None).expect("plays")).expect("wires");
        assert_eq!(of_node(&ports, "tone_lfo"), Some(("tone".into(), "lfo".into())));
        assert_eq!(remove_node(&next, "tone", "lfo").expect("removed").0.instruments, b.instruments);
        let air = bells().instruments.into_iter().find(|i| i.id == "bells_air").expect("bells_air");
        let (with, _) = apply(&b, &Choice::Track(Box::new(air))).expect("a track");
        assert_eq!(with.rig.tracks.len(), 3);
        assert_eq!(remove_track(&with, "bells_air").expect("removed").0.rig, b.rig);
        assert!(apply(&b, &Choice::Far { track: "tone".into() }).is_err(), "a far node waits for its holder");
    }

    #[test]
    fn a_new_track_and_its_instrument_come_back_from_the_draft() {
        let b = keys();
        let air = bells().instruments.into_iter().find(|i| i.id == "bells_air").expect("bells_air");
        let (with, _) = apply(&b, &Choice::Track(Box::new(air))).expect("a track");
        let file = std::env::temp_dir().join(format!("deck-editor-add-{}", std::process::id())).join("rig.toml");
        wires::save_draft(&file, &b, &with, None).expect("written");
        let read = wires::load_draft(&file, &b);
        assert_eq!((&read.rig, &read.instruments), (&with.rig, &with.instruments));
        std::fs::remove_file(&file).ok();
    }

    /// the done-when: in the far-outlets fixture a far node added to the keys listens to the bells
    #[test]
    fn a_far_node_added_to_the_keys_listens_to_the_bells() {
        let (next, words) = far(&keys(), "tone", &Place::new("character", "bells"), &bells()).expect("added");
        assert_eq!(words, "tone:far listens to character:bells");
        let reach = deck_space::holders::reach_of(&Place::new("character", "keys"), &next.play(None, None).expect("plays"));
        assert!(reach.far.iter().any(|f| f.node == "tone_far" && f.holder() == Some(&Place::new("character", "bells"))), "{:?}", reach.far);
    }
}
