//! A scene a deck plays, and the words that change its cast while it plays (#516, research/SCENE-EVENTS.md §3.1 E1; owner
//! 2026-11-13 [scenes]: "a join without an action is just appear … keep it very simple"). Platform-free and sans IO: the
//! scene document and its fillers go in, a word arrives as an address action (SHOW.md §8, `scene:stage > join:character:catpope`,
//! a show row's `addresses` or the relay's `address`), and the adapter gets whether the cast changed – then it loads
//! `resolved()`'s scene in place of the one before with the running values carried over (live-core `recast`).
//!
//! - **Join** (`join:<who>`): who appears at the first free spot whose placeholder admits it – `Casting::join`'s rule, a
//!   spawn zone up to its `max` – with its look and rig as a resolve gives them (`scene_resolve::resolve`, the one resolve).
//!   No animation. Already on stage: nothing. No free spot that admits it: nothing, said in the log. A rig the deck does
//!   not have: the member stands silent, said (resolve's note).
//! - **Leave** (`leave:<who>`): who disappears, its spot is free again (`Casting::leave`). Who is not on stage – or is
//!   written into the scene itself – stays as it is, said.
//! - **The rest keeps playing:** the other members keep their holders, slots and spots, the space its room and set (the
//!   same resolve of the same document); the adapter carries the running values across (live-core `recast`).
//! - An address that does not pick up this scene, or an action that is no cast word, is not this scene's: `NotMine`.
//! - **A story event** (#517, SCENE-EVENTS.md §3.2 E2a; owner 2026-11-13 [scenes]: "a scene listens"): the VNEvent as the
//!   story adapter reads it (deck-events adapters/story.rs: `story.<type>`, its id, its params as data) against the scene's
//!   listeners (scene_document.rs `Listener`): each listener of its type – and its id, when the listener names one – makes
//!   each character the event names (`characters`, else `character`) that it admits say its cast word, through `word`: the
//!   same words an address brings. No listener of it: nothing, and nothing said. The one place this matching lives; the
//!   show's conductor matches its rows by event id (deck-show conductor.rs `on_event`) on its own.
use crate::piece::LOG_CAP;
use crate::rig::address::{CastWord, Target};
use crate::scene_document::{FillerRef, Fillers, Listener, SceneDocument};
use crate::scene_resolve::{resolve, Casting, Resolved, Rigs, Sets};
use deck_events::adapters::story::from_story;
use deck_events::Names;
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};

/// the crossfade (ms) a changed cast loads under: a declick, not an entrance – the rest of the scene sounds on through it
pub const RECAST_MS: u32 = 30;

/// What a word did to the scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recast {
    /// not this scene's word (another address, another action)
    NotMine,
    /// the scene plays on as it is; the log says why
    Nothing,
    /// the cast changed at `holder`: the adapter loads `Playing::resolved` in place of the scene before (values carried)
    /// over `ms`
    Changed { holder: String, ms: u32 },
}

/// A scene document playing on a deck: its fillers so far, what they resolve to, the log of its words.
#[derive(Debug, Clone)]
pub struct Playing {
    doc: SceneDocument,
    with: BTreeMap<String, Fillers>,
    now: Resolved,
    log: VecDeque<String>,
}

impl Playing {
    /// `doc` with its fillers (a use's, by context), resolved with the caller's rigs and sets – the scene that starts
    pub fn start(doc: SceneDocument, with: BTreeMap<String, Fillers>, rigs: Rigs, sets: Sets) -> Result<Playing, Vec<String>> {
        let now = resolve(&doc, &with, false, rigs, sets)?;
        Ok(Playing { doc, with, now, log: VecDeque::new() })
    }

    /// what plays now
    pub fn resolved(&self) -> &Resolved {
        &self.now
    }

    /// the scene's place, as an address picks it up (`scene:stage`)
    pub fn place(&self) -> String {
        format!("scene:{}", self.doc.id)
    }

    /// the log, oldest first
    pub fn log(&self) -> impl Iterator<Item = &str> {
        self.log.iter().map(String::as_str)
    }

    fn note(&mut self, line: String) {
        if self.log.len() == LOG_CAP {
            self.log.pop_front();
        }
        self.log.push_back(line);
    }

    /// An address action (`address`, `action`: a show row's, the relay's) on this scene, with the caller's rigs and sets.
    pub fn word(&mut self, address: &str, action: &str, rigs: Rigs, sets: Sets) -> Recast {
        let place = self.place();
        if !Target::parse(address).is_ok_and(|t| t.picks_up(&place)) {
            return Recast::NotMine;
        }
        let word = match CastWord::parse(action) {
            None => return Recast::NotMine,
            Some(Err(e)) => {
                self.note(e);
                return Recast::Nothing;
            }
            Some(Ok(w)) => w,
        };
        let who = word.who().to_string();
        let mut casting = Casting::new(&self.doc, &self.with);
        let cast = match &word {
            CastWord::Join(_) => match FillerRef::parse(&who) {
                Err(e) => Err(format!("join {who}: {e}")),
                Ok(f) if self.now.cast.iter().any(|m| m.place == f.place.to_string()) => Err(format!("join {who}: stands on {place} already")),
                Ok(_) => casting.join(&who).map_err(|e| format!("join {who}: {e}")),
            },
            CastWord::Leave(_) => casting.leave(&who).ok_or_else(|| format!("leave {who}: no placeholder of {place} carries it")),
        };
        let with = casting.with();
        let holder = match cast {
            Ok(h) => h,
            Err(e) => {
                self.note(e);
                return Recast::Nothing;
            }
        };
        match resolve(&self.doc, &with, false, rigs, sets) {
            Err(problems) => {
                for p in problems {
                    self.note(format!("{action}: {p}"));
                }
                Recast::Nothing
            }
            Ok(r) => {
                let said: Vec<String> = r.notes.iter().filter(|n| !self.now.notes.contains(n)).cloned().collect();
                let verb = if matches!(word, CastWord::Join(_)) { "joined" } else { "left" };
                self.note(format!("{who} {verb} {place} at {holder}"));
                for n in said {
                    self.note(n);
                }
                (self.with, self.now) = (with, r);
                Recast::Changed { holder, ms: RECAST_MS }
            }
        }
    }

    /// #517: whether the scene listens to story events – a deck asks its session for them only then
    pub fn listens(&self) -> bool {
        !self.doc.listeners.is_empty()
    }

    /// #517: a story event (the VNEvent's JSON: `{ type, params, id? }`) against the scene's listeners, with the caller's
    /// rigs and sets: what each cast word it made did, in order. Empty: no listener heard it.
    pub fn story(&mut self, event: &Value, rigs: Rigs, sets: Sets) -> Vec<Recast> {
        let mut names = Names::new();
        let e = match from_story(&mut names, event, None) {
            Ok(Some(e)) => e,
            Ok(None) => return Vec::new(),
            Err(err) => {
                self.note(format!("a story event: {err}"));
                return Vec::new();
            }
        };
        let kind = names.name(e.name).strip_prefix("story.").unwrap_or_default().to_string();
        let heard: Vec<Listener> = self.doc.listeners.iter().filter(|l| l.on == kind && l.id.as_ref().is_none_or(|id| e.id.as_ref() == Some(id))).cloned().collect();
        if heard.is_empty() {
            return Vec::new();
        }
        let named = characters(e.data.as_deref());
        if named.is_empty() {
            self.note(format!("{kind}: the event names no character"));
            return Vec::new();
        }
        let place = self.place();
        let mut did = Vec::new();
        for l in &heard {
            for who in named.iter().filter(|w| l.admits(w)) {
                did.push(self.word(&place, &format!("{}:{who}", l.act.word()), rigs, sets));
            }
        }
        did
    }
}

/// the characters a story event's data names, as references: `characters` (joinScene / leaveScene: `{ character, persona? }`,
/// the old form's `name`), else `character` (a message's speaker: an id, or the same object)
pub(crate) fn characters(data: Option<&Value>) -> Vec<String> {
    let one = |c: &Value| -> Option<String> {
        let id = c.as_str().or_else(|| c.get("character").or_else(|| c.get("name")).and_then(Value::as_str)).filter(|id| !id.is_empty())?;
        Some(match c.get("persona").and_then(Value::as_str).filter(|p| !p.is_empty()) {
            Some(p) => format!("character:{id}.{p}"),
            None => format!("character:{id}"),
        })
    };
    match data.map(|d| (d.get("characters"), d.get("character"))) {
        Some((Some(Value::Array(cs)), _)) => cs.iter().filter_map(one).collect(),
        Some((_, Some(c))) => one(c).into_iter().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_resolve::{core_rigs, no_rigs, no_sets};

    fn stage() -> SceneDocument {
        let text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/templates/stage.json"));
        SceneDocument::parse_json(text).unwrap()
    }

    fn ids(p: &Playing) -> Vec<String> {
        p.resolved().scene.nodes.iter().map(|n| n.id.clone()).collect()
    }

    /// join → she stands at the first free spot with her look and rig; leave → the spot is empty and her nodes gone; the
    /// others keep their holders and node ids
    #[test]
    fn join_appears_at_the_first_free_spot_and_leave_frees_it() {
        let mut p = Playing::start(stage(), BTreeMap::new(), &core_rigs, &no_sets).unwrap();
        assert!(p.resolved().cast.is_empty());
        let join = |p: &mut Playing, who: &str| p.word("scene:stage", &format!("join:{who}"), &core_rigs, &no_sets);
        assert_eq!(join(&mut p, "character:catpope"), Recast::Changed { holder: "left".into(), ms: RECAST_MS });
        let m = &p.resolved().cast[0];
        assert_eq!((m.holder.as_str(), m.spot.as_deref(), m.slots.as_deref()), ("left", Some("spot_left"), Some("left_")));
        assert!(ids(&p).contains(&"left_look".to_string()) && ids(&p).contains(&"left_voice_voice".to_string()), "{:?}", ids(&p));
        let before = ids(&p);
        assert_eq!(join(&mut p, "character:toptoy"), Recast::Changed { holder: "centre".into(), ms: RECAST_MS });
        assert!(before.iter().all(|id| ids(&p).contains(id)), "what stood there stays as it was");
        assert!(p.log().any(|l| l.contains("character.toptoy") && l.contains("silent")), "a rig the deck lacks: silent, said");
        // already on stage – any persona of her: nothing
        assert_eq!(join(&mut p, "character:catpope.pope"), Recast::Nothing);
        assert!(p.log().last().unwrap().contains("stands on scene:stage already"));
        assert_eq!(p.word("scene:stage", "leave:character:catpope", &core_rigs, &no_sets), Recast::Changed { holder: "left".into(), ms: RECAST_MS });
        assert_eq!(p.resolved().cast.iter().map(|m| m.holder.as_str()).collect::<Vec<_>>(), ["centre"]);
        assert!(!ids(&p).iter().any(|id| id.starts_with("left_")), "her look and rig are gone");
        assert_eq!(p.word("scene:stage", "leave:character:catpope", &core_rigs, &no_sets), Recast::Nothing, "gone already");
    }

    /// the stage with listeners (`listener`, as a document writes them)
    fn listening(listeners: &str) -> SceneDocument {
        let mut v: Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/templates/stage.json"))).unwrap();
        v["listener"] = serde_json::from_str(listeners).unwrap();
        SceneDocument::from_value(v).unwrap()
    }

    fn story(kind: &str, characters: &[&str], id: Option<&str>) -> Value {
        let cs: Vec<Value> = characters.iter().map(|c| serde_json::json!({ "character": c })).collect();
        serde_json::json!({ "type": kind, "params": { "characters": cs }, "id": id })
    }

    /// #517: a joinScene the scene listens to joins each character it names at the first free spot; leaveScene leaves
    #[test]
    fn a_listened_story_event_joins_and_leaves() {
        let doc = listening(r#"[{"on":"joinScene","do":"join"},{"on":"leaveScene","do":"leave"}]"#);
        let mut p = Playing::start(doc, BTreeMap::new(), &core_rigs, &no_sets).unwrap();
        assert!(p.listens());
        let did = p.story(&story("joinScene", &["catpope", "toptoy"], Some("e1")), &core_rigs, &no_sets);
        assert_eq!(did, [Recast::Changed { holder: "left".into(), ms: RECAST_MS }, Recast::Changed { holder: "centre".into(), ms: RECAST_MS }]);
        assert_eq!(p.resolved().cast.iter().map(|m| (m.holder.as_str(), m.place.as_str())).collect::<Vec<_>>(), [("left", "character:catpope"), ("centre", "character:toptoy")]);
        assert!(p.log().any(|l| l == "character:catpope joined scene:stage at left"), "the same words an address brings");
        let did = p.story(&story("leaveScene", &["catpope"], None), &core_rigs, &no_sets);
        assert_eq!(did, [Recast::Changed { holder: "left".into(), ms: RECAST_MS }]);
        assert_eq!(p.resolved().cast.iter().map(|m| m.holder.as_str()).collect::<Vec<_>>(), ["centre"]);
        // a message's speaker is the character of a single-character event
        let doc = listening(r#"[{"on":"showMessage","do":"join"}]"#);
        let mut p = Playing::start(doc, BTreeMap::new(), &core_rigs, &no_sets).unwrap();
        let said = serde_json::json!({ "type": "showMessage", "params": { "text": "Hallo", "character": "catpope", "mood": "happy" } });
        assert_eq!(p.story(&said, &core_rigs, &no_sets), [Recast::Changed { holder: "left".into(), ms: RECAST_MS }]);
    }

    /// #517: only what a listener names – its type, its id, the characters it admits; no listener, or an event the story
    /// keeps: nothing, and nothing said
    #[test]
    fn a_story_event_no_listener_takes_does_nothing() {
        let mut p = Playing::start(stage(), BTreeMap::new(), &core_rigs, &no_sets).unwrap();
        assert!(!p.listens());
        assert_eq!(p.story(&story("joinScene", &["catpope"], None), &core_rigs, &no_sets), []);
        assert!(p.resolved().cast.is_empty() && p.log().next().is_none());
        let doc = listening(r#"[{"on":"joinScene","id":"e9","character":"character:catpope","do":"join"}]"#);
        let mut p = Playing::start(doc, BTreeMap::new(), &core_rigs, &no_sets).unwrap();
        assert_eq!(p.story(&story("joinScene", &["catpope"], Some("e1")), &core_rigs, &no_sets), [], "another event id");
        assert_eq!(p.story(&story("leaveScene", &["catpope"], Some("e9")), &core_rigs, &no_sets), [], "another type");
        assert_eq!(p.story(&story("joinScene", &["toptoy"], Some("e9")), &core_rigs, &no_sets), [], "a character it does not admit");
        assert_eq!(p.story(&serde_json::json!({ "type": "jumpTo", "params": { "label": "x" } }), &core_rigs, &no_sets), [], "the story's runner keeps it");
        assert_eq!(p.story(&story("joinScene", &[], Some("e9")), &core_rigs, &no_sets), []);
        assert_eq!(p.log().last(), Some("joinScene: the event names no character"));
        assert_eq!(p.story(&story("joinScene", &["toptoy", "catpope"], Some("e9")), &core_rigs, &no_sets), [Recast::Changed { holder: "left".into(), ms: RECAST_MS }]);
        assert_eq!(p.resolved().cast.len(), 1, "Top Toy is not admitted");
        assert_eq!(p.story(&serde_json::json!({ "params": {} }), &core_rigs, &no_sets), []);
        assert!(p.log().last().unwrap().starts_with("a story event: "), "a broken event is said");
    }

    /// no free spot that admits who: nothing, said; another scene's word, or another action, is not this one's
    #[test]
    fn no_free_admitting_spot_is_nothing_and_said() {
        let with = ["left", "centre", "right", "back"].iter().enumerate().map(|(i, h)| (h.to_string(), Fillers::One(format!("character:c{i}")))).collect();
        let mut p = Playing::start(stage(), with, &no_rigs, &no_sets).unwrap();
        assert_eq!(p.word("scene:stage", "join:character:catpope", &no_rigs, &no_sets), Recast::Nothing);
        assert_eq!(p.log().last(), Some("join character:catpope: no placeholder of scene:stage with room admits 'character:catpope'"));
        assert_eq!(p.word("scene:stage", "join:item:incense", &no_rigs, &no_sets), Recast::Nothing, "an item the placeholders do not take");
        assert_eq!(p.resolved().cast.len(), 4);
        assert_eq!(p.word("scene:nave", "join:character:catpope", &no_rigs, &no_sets), Recast::NotMine);
        assert_eq!(p.word("scene:stage", "dance:samba", &no_rigs, &no_sets), Recast::NotMine);
        assert_eq!(p.word("scene:*", "join:catpope", &no_rigs, &no_sets), Recast::Nothing, "a cast word without a kind: said");
        assert!(p.log().last().unwrap().contains("<kind>:<id>"));
    }
}
