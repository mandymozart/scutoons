//! sing.rs (#819): what starts a subject's voice – a rig's `trigger:sing` (rig/address.rs `SOURCES`), 0 shut … 1 open.
//! Flatten writes a node for each rig that reads it (`<prefix>trigger_sing`, an `add`); the stage sets it every frame it
//! changes. The default character rig wires it to its voice's gates (`voice:@open`), so a character sings only while:
//!
//! - **held** – the player holds the pad's sing button (`gamepad.rs` `SING`, west: X / square) or the sing key (`KEY`)
//!   on the picked member; nothing picked, the space's own being (a rig played alone);
//! - **a text line of its character shows** – a story's `showMessage` naming it opens its trigger for the line's time
//!   (`line`: the event's `duration`, else a reading time from its text); the next line, narration or `hideMessage`
//!   shuts it earlier.
//!
//! What the trigger opens is the rig's wiring, nothing here. Never allocates on the frame.
use super::values::Knobs;
use super::Space;
use crate::cast::Who;
use crate::graph::{Graph, Kind};
use crate::rig::address::{source_slot, TRIGGER};
use serde_json::Value;

/// the key that sings while held (the DOM's `KeyboardEvent.key`; the native deck says the same)
pub const KEY: &str = "v";
/// a text line's shortest time open (s)
pub const LINE_MIN: f32 = 1.2;
/// and the time per character of its text (s)
pub const LINE_PER_CHAR: f32 = 0.06;
/// a line's longest time open (s): a reader who waits does not keep it singing
pub const LINE_MAX: f32 = 8.0;

/// One subject's trigger: whose (a cast member by its place in the cast; none = the space's own being), its node, and
/// what holds it open.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trigger {
    pub member: Option<usize>,
    pub node: usize,
    held: bool,
    /// a text line keeps it open until this show time (s)
    until: f32,
    /// the value last set (none: not yet)
    set: Option<f32>,
}

impl Trigger {
    fn value(&self, now: f32) -> f32 {
        if self.held || now < self.until {
            1.0
        } else {
            0.0
        }
    }
}

/// the node a rig's `trigger:<name>` lands on (`trigger_sing`), under a prefix
fn node_id(prefix: &str) -> Option<String> {
    source_slot(TRIGGER, "sing").and_then(|s| s.split_once('.')).map(|(n, _)| format!("{prefix}{n}"))
}

/// The triggers of `graph`: each `look` node's (in the cast's order, its nodes under its prefix) and, with no looks, the
/// space's own being's (a rig played alone, unprefixed). Only subjects whose rig reads one.
pub fn triggers(graph: &Graph) -> Vec<Trigger> {
    let find = |prefix: &str| {
        let id = node_id(prefix)?;
        graph.nodes.iter().position(|n| n.id == id && n.kind == Kind::Add)
    };
    let of = |member: Option<usize>, prefix: &str| find(prefix).map(|node| Trigger { member, node, held: false, until: f32::NEG_INFINITY, set: None });
    let looks: Vec<&str> = graph.nodes.iter().filter(|n| n.kind == Kind::Look).map(|n| n.id.strip_suffix(crate::scene::look_id("").as_str()).unwrap_or_default()).collect();
    if looks.is_empty() {
        return of(None, "").into_iter().collect();
    }
    looks.iter().enumerate().filter_map(|(i, p)| of(Some(i), p)).collect()
}

/// What a story event does to the triggers.
#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    /// a text line of `character` shows for `seconds`
    Says { character: String, seconds: f32 },
    /// the text box is gone or speaks for no one: every line's trigger shuts
    Ends,
    /// nothing of the triggers'
    Other,
}

/// A story event (the VNEvent's JSON, as the relay brings it) as a line: `showMessage` with a `character` says it for its
/// `duration` (ms) or its reading time (`LINE_MIN` + `LINE_PER_CHAR` a character, at most `LINE_MAX`); without one
/// (narration) and `hideMessage` end it.
pub fn line(event: &Value) -> Line {
    let params = event.get("params");
    match event.get("type").and_then(Value::as_str) {
        Some("showMessage") => {
            let who = params.and_then(|p| p.get("character")).and_then(Value::as_str).map(str::trim).filter(|c| !c.is_empty());
            let Some(who) = who else { return Line::Ends };
            let character = who.strip_prefix("character:").unwrap_or(who).to_string();
            let seconds = match params.and_then(|p| p.get("duration")).and_then(Value::as_f64) {
                Some(ms) if ms > 0.0 => ms as f32 / 1000.0,
                _ => {
                    let text = params.and_then(|p| p.get("text"));
                    let text = text.and_then(Value::as_str).or_else(|| text.and_then(Value::as_object).and_then(|m| m.values().find_map(Value::as_str)));
                    (LINE_MIN + text.map_or(0, |t| t.chars().count()) as f32 * LINE_PER_CHAR).min(LINE_MAX)
                }
            };
            Line::Says { character, seconds }
        }
        Some("hideMessage") => Line::Ends,
        _ => Line::Other,
    }
}

/// whether `key` (the DOM's name) is the sing key
pub fn is_key(key: &str) -> bool {
    key.eq_ignore_ascii_case(KEY)
}

/// The triggers' values into the control domain at show time `now`: only what changed.
pub fn push(triggers: &mut [Trigger], now: f32, ctl: &mut dyn Knobs) {
    let Some(port) = Kind::Add.input("a") else { return };
    for t in triggers {
        let v = t.value(now);
        if t.set != Some(v) {
            ctl.set(t.node, port, v);
            t.set = Some(v);
        }
    }
}

impl Space {
    /// whether a subject on this stage has a trigger (its rig reads `trigger:sing`)
    pub fn sings(&self) -> bool {
        !self.triggers.is_empty()
    }

    /// the trigger nodes, in the cast's order
    pub fn sing_nodes(&self) -> impl Iterator<Item = usize> + '_ {
        self.triggers.iter().map(|t| t.node)
    }

    /// The player's sing button or key: held (`on`) opens the picked member's trigger – nothing picked, the space's own
    /// being's –, let go shuts every held one. false = no one there has a trigger.
    pub fn sing(&mut self, on: bool) -> bool {
        if !on {
            self.triggers.iter_mut().for_each(|t| t.held = false);
            return true;
        }
        let picked = self.picked();
        match self.triggers.iter_mut().find(|t| t.member == picked) {
            Some(t) => {
                t.held = true;
                true
            }
            None => false,
        }
    }

    /// A key by the DOM's name pressed (`down`) or let go: the sing key holds the trigger (`sing`). true = it was the key.
    pub fn sing_key(&mut self, key: &str, down: bool) -> bool {
        is_key(key) && {
            self.sing(down);
            true
        }
    }

    /// A story event (the VNEvent's JSON): a text line of a character opens its triggers for the line's time and shuts the
    /// line before it; narration and `hideMessage` shut it. true = a trigger opened.
    pub fn sing_line(&mut self, event: &Value) -> bool {
        let (character, seconds) = match line(event) {
            Line::Other => return false,
            Line::Ends => (None, 0.0),
            Line::Says { character, seconds } => (Some(character), seconds),
        };
        let now = self.clock;
        let is = |member: Option<usize>, c: &str| match member {
            Some(i) => self.cast.get(i).is_some_and(|m| matches!(&m.who, Who::Character(w) if w == c)),
            None => self.being.look.character == c,
        };
        let opened: Vec<bool> = self.triggers.iter().map(|t| character.as_deref().is_some_and(|c| is(t.member, c))).collect();
        for (t, open) in self.triggers.iter_mut().zip(&opened) {
            t.until = if *open { now + seconds } else { f32::NEG_INFINITY };
        }
        opened.contains(&true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;
    use crate::space::values::Table;
    use serde_json::json;

    fn graph(text: &str) -> Graph {
        Graph::build(&Scene::parse(text).unwrap()).unwrap()
    }

    #[test]
    fn a_text_line_says_for_its_duration_or_its_reading_time() {
        let says = |e: Value| match line(&e) {
            Line::Says { character, seconds } => (character, (seconds * 100.0).round() / 100.0),
            l => panic!("{l:?}"),
        };
        assert_eq!(says(json!({ "type": "showMessage", "params": { "character": "catpope", "text": "Miau!", "duration": 2500 } })), ("catpope".into(), 2.5));
        assert_eq!(says(json!({ "type": "showMessage", "params": { "character": "character:catpope", "text": { "de": "Hallo" } } })), ("catpope".into(), 1.5), "a locale map; the place's prefix goes");
        assert_eq!(says(json!({ "type": "showMessage", "params": { "character": "x", "text": "a".repeat(500) } })).1, LINE_MAX, "at most");
        assert_eq!(line(&json!({ "type": "showMessage", "params": { "text": "Es war Nacht." } })), Line::Ends, "narration speaks for no one");
        assert_eq!(line(&json!({ "type": "hideMessage" })), Line::Ends);
        assert_eq!(line(&json!({ "type": "fx", "params": { "effect": "shake" } })), Line::Other);
    }

    #[test]
    fn a_trigger_is_open_while_held_or_while_its_line_shows_and_set_only_when_it_changes() {
        let g = graph("[[node]]\nid=\"space\"\nkind=\"space\"\n[[node]]\nid=\"trigger_sing\"\nkind=\"add\"\n");
        let mut t = triggers(&g);
        assert_eq!(t.len(), 1, "a rig played alone: the space's own being");
        let mut table = Table::new(&g);
        let n = t[0].node;
        table.set(n, 0, -1.0);
        push(&mut t, 0.0, &mut table);
        assert_eq!(table.input(n, "a", -1.0), 0.0, "shut until triggered");
        table.set(n, 0, -1.0);
        push(&mut t, 0.1, &mut table);
        assert_eq!(table.input(n, "a", 0.0), -1.0, "unchanged: not set again");
        t[0].held = true;
        push(&mut t, 0.2, &mut table);
        assert_eq!(table.input(n, "a", -1.0), 1.0);
        (t[0].held, t[0].until) = (false, 3.0);
        push(&mut t, 2.9, &mut table);
        assert_eq!(table.input(n, "a", -1.0), 1.0, "the line still shows");
        push(&mut t, 3.0, &mut table);
        assert_eq!(table.input(n, "a", -1.0), 0.0, "its time is up");
        assert!(triggers(&graph("[[node]]\nid=\"space\"\nkind=\"space\"\n")).is_empty(), "no rig reads it: nothing to set");
        assert!(is_key("v") && is_key("V") && !is_key("s"));
    }
}
