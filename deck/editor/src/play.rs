//! Edit and Play on a Scene or Asset tab (decisions/deck.md [workspaces], #629). In Edit the item is worked on: the
//! ports show, its clock is the editor's own, and what is addressed to it from outside only lights the ports it would
//! reach. Play runs it as a deck would: its clock follows its own source, what is addressed to it goes in (far events to
//! its far world nodes, a value to the input it names, a MIDI message to its MIDI world nodes) and its events fire. Play
//! starts from what Edit left, and Stop goes back to it: changes made while playing are dropped, as in Unreal, and none
//! of them is saved. Whether an address reaches the item is the core's (deck-space holders.rs `Reach::reached`).
use crate::scene::SceneView;
use deck_space::holders::{Reach, Reached};
use deck_space::rig::address::{Address, Place};
use deck_space::rig::Bundle;
use deck_space::scene_document::SceneDocument;
use std::time::{Duration, Instant};

/// how long a port stays lit after an input reached it
pub const LIT: Duration = Duration::from_millis(2500);
/// the inputs remembered for the ports
const KEEP: usize = 32;

/// one input from outside
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    /// an event under an address: a far world node's, or one sent to the item
    Event(protocol::DeckEvent),
    /// a value for the control input the address names
    Value { address: String, value: f32 },
    /// a MIDI message from a device: it reaches the item in front, whatever its address
    Midi([u8; 3]),
}

impl Input {
    /// A check run's input: `ADDRESS=VALUE`, `ADDRESS>NAME[:KEY=V;KEY=V]` or `midi:90,3c,64` (hex bytes).
    pub fn parse(s: &str) -> Result<Input, String> {
        let s = s.trim();
        if let Some(bytes) = s.strip_prefix("midi:") {
            let b: Vec<u8> = bytes.split(',').map(|b| u8::from_str_radix(b.trim(), 16)).collect::<Result<_, _>>().map_err(|e| format!("{s}: {e}"))?;
            let [a, b1, c] = b[..] else { return Err(format!("{s}: a MIDI message is three bytes")) };
            return Ok(Input::Midi([a, b1, c]));
        }
        if let Some((address, event)) = s.split_once('>') {
            Address::parse(address).map_err(|e| format!("{address}: {e}"))?;
            let (name, props) = event.split_once(':').unwrap_or((event, ""));
            let mut map = std::collections::HashMap::new();
            for p in props.split(';').filter(|p| !p.is_empty()) {
                let (k, v) = p.split_once('=').ok_or_else(|| format!("{p}: KEY=VALUE"))?;
                map.insert(k.to_string(), v.parse::<f64>().map_err(|e| format!("{p}: {e}"))?);
            }
            return Ok(Input::Event(protocol::DeckEvent { address: address.to_string(), name: name.to_string(), props: map, dur_s: None }));
        }
        let (address, value) = s.split_once('=').ok_or_else(|| format!("{s}: ADDRESS=VALUE, ADDRESS>NAME or midi:B,B,B"))?;
        Address::parse(address).map_err(|e| format!("{address}: {e}"))?;
        Ok(Input::Value { address: address.to_string(), value: value.parse().map_err(|e| format!("{value}: {e}"))? })
    }

    pub fn address(&self) -> Option<&str> {
        match self {
            Input::Event(e) => Some(&e.address),
            Input::Value { address, .. } => Some(address),
            Input::Midi(_) => None,
        }
    }

    /// what it says, short: `music.note pitch=60`, `= 0.4`, `midi 90 3c 64`
    pub fn words(&self) -> String {
        match self {
            Input::Event(e) => {
                let mut props: Vec<String> = e.props.iter().map(|(k, v)| format!("{k}={v}")).collect();
                props.sort();
                std::iter::once(e.name.clone()).chain(props).collect::<Vec<_>>().join(" ")
            }
            Input::Value { value, .. } => format!("= {value}"),
            Input::Midi(b) => format!("midi {:02x} {:02x} {:02x}", b[0], b[1], b[2]),
        }
    }
}

/// what an input does to the item in front
#[derive(Debug, Clone, PartialEq)]
pub enum Go {
    /// into the far world nodes listening to its address
    Far,
    /// to the control input of this compiled slot
    Slot(String),
    Midi,
}

/// An input routed: the ports it lights (graph node ids, or the item's port addresses `track:node.port`), and in Play
/// what it does; none in Edit, and none for an input that is another's (it lights nothing either).
#[derive(Debug, Clone, PartialEq)]
pub struct Routed {
    pub lit: Vec<String>,
    pub go: Option<Go>,
}

/// Where `input` goes for the item at `own` with this reach, in Play or in Edit.
pub fn route(input: &Input, own: Option<&Place>, reach: &Reach, playing: bool) -> Routed {
    let (lit, go) = match input {
        Input::Midi(_) => (vec!["midi".to_string()], Go::Midi),
        _ => {
            let Some(a) = input.address().and_then(|a| Address::parse(a).ok()) else { return Routed { lit: vec![], go: None } };
            let Some(own) = own else { return Routed { lit: vec![], go: None } };
            match (reach.reached(own, &a), input) {
                (Reached::Far(nodes), _) => (nodes, Go::Far),
                (Reached::Own(leaf), Input::Value { .. }) => match leaf.slot() {
                    Some(slot) => (vec![leaf.to_string()], Go::Slot(slot)),
                    None => (vec![leaf.to_string()], Go::Far),
                },
                (Reached::Own(leaf), _) => (vec![leaf.to_string()], Go::Far),
                (Reached::Not, _) => return Routed { lit: vec![], go: None },
            }
        }
    };
    Routed { lit, go: playing.then_some(go) }
}

/// one input that reached the item: what it lit, its words, when
#[derive(Debug, Clone)]
pub struct Arrived {
    pub lit: Vec<String>,
    pub words: String,
    pub at: Instant,
}

/// what Edit left: back to it on Stop
#[derive(Debug, Clone)]
pub struct Left {
    pub bundle: Bundle,
    pub undo: usize,
    pub knobs: Vec<(String, f32)>,
    /// the scene the tab holds, when it is a Scene tab
    pub scene: Option<SceneDocument>,
}

/// the front tab's Edit or Play, and what reached it
#[derive(Default)]
pub struct Play {
    /// Play: what Edit left; none = Edit
    pub left: Option<Left>,
    pub arrived: Vec<Arrived>,
    /// an input went into the engine during Play: Stop plays the item again from what Edit left
    pub touched: bool,
}

impl Play {
    pub fn playing(&self) -> bool {
        self.left.is_some()
    }

    pub fn arrive(&mut self, lit: Vec<String>, words: String) {
        self.arrived.push(Arrived { lit, words, at: Instant::now() });
        if self.arrived.len() > KEEP {
            self.arrived.remove(0);
        }
    }

    /// the newest input that lit `key` (a node id or a port's address) within `LIT`
    pub fn lit(&self, key: &str) -> Option<&Arrived> {
        self.arrived.iter().rev().find(|a| a.at.elapsed() < LIT && a.lit.iter().any(|l| l == key))
    }
}

/// a scene back to the document Edit left (the members resolved again); false when it was that already
pub fn restore_scene(view: &mut SceneView, doc: SceneDocument) -> Result<bool, String> {
    if view.edit.doc == doc {
        return Ok(false);
    }
    view.edit.doc = doc;
    view.refresh().map(|()| true)
}

impl crate::App {
    /// whether the front tab plays
    pub(crate) fn playing(&self) -> bool {
        match self.workspace {
            crate::show::Workspace::Media => self.media.open.as_ref().is_some_and(|o| o.playing),
            crate::show::Workspace::Module => self.module.heard,
            _ => self.play.playing(),
        }
    }

    /// The Edit / Play switch of a Scene or Asset tab. Play keeps what Edit left (what Edit edited is sent first); Stop
    /// goes back to it.
    pub(crate) fn set_play(&mut self, on: bool) {
        if self.workspace == crate::show::Workspace::Media {
            return self.media_play(on);
        }
        // a module plays alone on the stage: Play builds it (heard once built), Stop takes it off
        if self.workspace == crate::show::Workspace::Module {
            match (on, self.module.heard) {
                (true, false) => self.module.build(),
                (false, true) => {
                    self.module.leave(&mut self.stage);
                    self.note("Stop: the module is off the stage".into());
                }
                _ => {}
            }
            return;
        }
        if on == self.playing() || self.workspace != crate::show::Workspace::Stage {
            self.wire = !self.playing();
            return;
        }
        if on {
            self.leave_view();
            let scene = self.scene.as_ref().map(|v| v.edit.doc.clone());
            self.play.left = Some(Left { bundle: self.stage.bundle.clone(), undo: self.undo.len(), knobs: self.stage.knob_values(), scene });
            self.wire = false;
            self.stage.follow(true);
            let (now, own) = self.stage.clock();
            let clock = if own == "internal" { String::new() } else { format!(", its clock from {now}") };
            self.note(format!("Play: it runs as a deck would{clock}; what you change now is dropped on Stop"));
        } else {
            self.stop_play();
        }
    }

    /// Stop: back to what Edit left; nothing of Play was saved
    pub(crate) fn stop_play(&mut self) {
        if self.workspace == crate::show::Workspace::Media {
            return self.media_play(false);
        }
        let Some(left) = self.play.left.take() else { return };
        let touched = std::mem::take(&mut self.play.touched);
        self.wire = true;
        self.stage.follow(false);
        let (x, y) = self.cursor;
        let mut dropped = touched;
        let inside = self.scene.as_ref().is_some_and(|v| v.inside.is_some());
        if let (Some(doc), Some(view)) = (left.scene, self.scene.as_mut()) {
            match restore_scene(view, doc) {
                Ok(changed) => dropped |= changed,
                Err(why) => self.refuse(why, x, y),
            }
        }
        if self.stage.bundle != left.bundle || (touched && self.scene.is_none()) || (touched && inside) {
            dropped = true;
            if let Err(e) = self.stage.replay(left.bundle) {
                self.refuse(format!("{e:#}"), x, y);
            }
        } else if dropped && !inside {
            self.replay_scene();
        }
        self.undo.truncate(left.undo);
        let knobs = self.stage.knob_values();
        if knobs != left.knobs {
            dropped = true;
            self.stage.set_knob_values(&left.knobs);
        }
        self.note(if dropped { "Stop: back to Edit, what Play changed is dropped".to_string() } else { "Stop: back to Edit".to_string() });
    }

    /// the front item's place and reach: a rig's, or the scene's with its cast
    fn reach_now(&self) -> (Option<Place>, Reach) {
        match self.scene.as_ref().filter(|v| v.inside.is_none()) {
            Some(v) => {
                let holds = v.edit.members.iter().filter_map(|m| Place::parse(&m.filler).ok()).collect();
                (Place::parse(&v.edit.key()).ok(), Reach { holds, far: Vec::new() })
            }
            None => (Place::of_subject(&self.stage.bundle.rig.subject), self.stage.reach.clone()),
        }
    }

    /// An input from outside: to the front item when it plays, else only onto its ports. Only a Scene or Asset tab on
    /// the stage takes one.
    pub(crate) fn input(&mut self, input: Input) {
        if self.workspace != crate::show::Workspace::Stage {
            return;
        }
        let (own, reach) = self.reach_now();
        let r = route(&input, own.as_ref(), &reach, self.playing());
        if r.lit.is_empty() {
            return;
        }
        let went = match (&r.go, &input) {
            (Some(Go::Far), Input::Event(e)) => self.stage.take_far(std::slice::from_ref(e)) > 0,
            (Some(Go::Slot(slot)), Input::Value { value, address }) => {
                // in a scene a member's slots carry its prefix
                let prefix = self.scene.as_ref().filter(|v| v.inside.is_none()).and_then(|v| {
                    let a = Address::parse(address).ok()?;
                    let place = a.places.iter().rev().find(|p| p.kind != "deck")?.to_string();
                    v.edit.members.iter().find(|m| m.filler == place).and_then(|m| m.prefix.clone())
                });
                self.stage.set_slot(&format!("{}{slot}", prefix.unwrap_or_default()), *value)
            }
            (Some(Go::Midi), Input::Midi(b)) => self.stage.take_midi(b),
            _ => false,
        };
        let words = input.words();
        let to = input.address().unwrap_or("this deck").to_string();
        let said = match (self.playing(), went) {
            (true, true) => format!("Play: {to} {words} went in"),
            (true, false) => format!("Play: {to} {words} reached it, nothing listens to it"),
            (false, _) => format!("Edit: {to} {words} lit its port, Play takes it"),
        };
        crate::log::post(deck_ui::messages::Level::Info, "input", said);
        self.play.touched |= went;
        self.play.arrive(r.lit, words);
    }

    /// a check run's inputs (`--input`) at the frame `--input-at` names, its Play (`--play`) taken properly at the
    /// start and its Stop (`--stop-at`); its press on Home's New (`--home-new`)
    pub(crate) fn script_play(&mut self) {
        if self.frames == 8 && !self.wire && !self.playing() {
            self.wire = true;
            self.set_play(true);
        }
        if Some(self.frames) == self.args.input_at {
            for s in self.args.inputs.clone() {
                match Input::parse(&s) {
                    Ok(i) => self.input(i),
                    Err(e) => tracing::error!("--input {e}"),
                }
            }
        }
        if Some(self.frames) == self.args.stop_at {
            self.set_play(false);
        }
        if self.frames == 3 {
            // `scene=NAME` types the name into the prompt (it stays open: a check run's screenshot shows it)
            let arg = self.args.home_new.clone();
            let new = match arg.as_deref().map(|a| a.split_once('=').map_or((a, ""), |(w, n)| (w, n))) {
                Some(("scene", name)) => {
                    self.ask_new_scene();
                    for c in name.chars() {
                        let t = c.to_string();
                        if let Some(p) = self.scene_prompt.as_mut() {
                            p.key(winit::keyboard::Key::Character(t.as_str()), Some(&t), false, false);
                        }
                    }
                    None
                }
                Some(("show", _)) => Some(crate::home::New::Show),
                Some(("module", _)) => Some(crate::home::New::Module),
                _ => None,
            };
            if let Some(n) = new {
                self.home_press(crate::home::Press::New(n));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catpope() -> (Place, Reach) {
        let b = Bundle::core("character.catpope").unwrap().unwrap();
        let played = deck_space::deck::Deck::new().play(Some(b), "").unwrap().played;
        let own = Place::new("character", "catpope");
        let mut reach = deck_space::holders::reach_of(&own, &played);
        reach.far.push(deck_space::holders::FarNode { node: "drone_far".into(), address: Address::parse("/deck:pc/scene:demo/character:bells/analog:seq.notes").unwrap() });
        (own, reach)
    }

    #[test]
    fn an_input_addressed_to_the_item_reaches_play_but_only_lights_its_port_in_edit() {
        let (own, reach) = catpope();
        let value = Input::parse("/scene:nave/character:catpope/drone:osc.freq=220").unwrap();
        let play = route(&value, Some(&own), &reach, true);
        assert_eq!(play.go, Some(Go::Slot("drone_osc.freq".into())), "Play: it goes in");
        let edit = route(&value, Some(&own), &reach, false);
        assert_eq!((edit.go, edit.lit), (None, vec!["drone:osc.freq".to_string()]), "Edit: only its port lights");
        let far = Input::parse("/deck:pc/scene:demo/character:bells/analog:seq.notes>music.note:pitch=60;vel=0.8").unwrap();
        assert_eq!(route(&far, Some(&own), &reach, true), Routed { lit: vec!["drone_far".into()], go: Some(Go::Far) }, "a far node's address");
        assert_eq!(route(&far, Some(&own), &reach, false).go, None);
        let other = Input::parse("/scene:nave/character:bells/drone:osc.freq=220").unwrap();
        assert_eq!(route(&other, Some(&own), &reach, true), Routed { lit: vec![], go: None }, "another's: nothing");
        assert_eq!(route(&Input::parse("midi:90,3c,64").unwrap(), Some(&own), &reach, true).go, Some(Go::Midi));
        assert_eq!(route(&Input::parse("midi:90,3c,64").unwrap(), Some(&own), &reach, false).go, None);
        assert!(Input::parse("midi:90,3c").is_err() && Input::parse("nothing").is_err());
        assert_eq!(far.words(), "music.note pitch=60 vel=0.8");
    }

    #[test]
    fn a_port_stays_lit_a_while_and_the_newest_input_shows() {
        let mut p = Play::default();
        p.arrive(vec!["drone:osc.freq".into()], "= 220".into());
        p.arrive(vec!["drone:osc.freq".into()], "= 330".into());
        assert_eq!(p.lit("drone:osc.freq").map(|a| a.words.as_str()), Some("= 330"));
        assert!(p.lit("voice:osc.freq").is_none());
        for i in 0..40 {
            p.arrive(vec![format!("n{i}")], String::new());
        }
        assert_eq!(p.arrived.len(), KEEP);
    }

    /// Stop drops what Play changed: a rig's change and a scene's go back to what Edit left
    #[test]
    fn stop_drops_what_play_changed() {
        let base = Bundle::core("character.catpope").unwrap().unwrap();
        let left = Left { bundle: base.clone(), undo: 0, knobs: vec![], scene: None };
        let track = base.rig.tracks[0].id.clone();
        let changed = crate::wires::set_pan(&base, &track, 0.7).unwrap();
        assert_ne!(changed, left.bundle, "Play changed the rig");
        let mut p = Play { left: Some(left), arrived: vec![], touched: false };
        assert!(p.playing());
        let back = p.left.take().unwrap();
        assert_eq!(back.bundle, base, "Stop: the rig Edit left");

        let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        doc.recast(&["character:catpope".into(), "item:incense_burner".into()]);
        let open = crate::show::build::SceneOpen { doc, base: Some(3), kept: None, note: None, save: None };
        let (mut view, _, _) = SceneView::open(open, None).expect("it plays");
        let edit_left = view.edit.doc.clone();
        view.edit.set_background("#ff0000").unwrap();
        assert_ne!(view.edit.doc, edit_left, "Play changed the scene");
        assert!(restore_scene(&mut view, edit_left.clone()).unwrap());
        assert_eq!(view.edit.doc, edit_left, "Stop: the scene Edit left");
        assert!(!restore_scene(&mut view, edit_left).unwrap(), "nothing to drop");
    }
}
