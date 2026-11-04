//! A deck's state without a stage, a window or a sound device (#367, #368): what a platform's deck holds around the
//! engine and the 3D – its mood (mood.rs `Ladder`) and what it plays (its own scene, or a character's rig replayed on
//! every mood: rig/bundle.rs `Played`, the lifecycle here once). The browser deck has it as space-web's `DeckState` (no
//! GPU: it works when the 3D can't), the headless host drives it directly. The page passes its inputs each frame,
//! applies the turns (a new scene to load, values that moved) and draws the answers.
use crate::mood::Ladder;
use crate::rig::{Bundle, Played};
use anyhow::Result;
use serde::Serialize;

/// the being's size on the wire (its radius, m)
const SIZE: f64 = 0.5;

/// an f32 as the wire's f64 – its shortest decimal (0.6, not 0.6000000238), as the scene document wrote it
fn wire(x: f32) -> f64 {
    x.to_string().parse().unwrap_or(f64::from(x))
}

/// Why the deck's scene changed: a scene played (the deck's own or a rig) or the rig replayed for a mood.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Why {
    Play,
    Mood,
}

/// What changed in what the deck plays – the platform applies it: on `reload` the scene anew (its knobs, sticks, streamed
/// paths, the being on the wire, the engine's scene), else only `values` (path → value; a knob's moves its slider).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Turn {
    pub why: Why,
    /// a rig plays (false = the deck's own scene)
    pub rig: bool,
    pub reload: bool,
    pub values: Vec<(String, f32)>,
    pub played: Played,
    /// what happened, for the page's log: which rig, or why the deck's own scene plays instead
    pub note: String,
    /// the rig failed and the deck's own scene plays instead
    pub failed: bool,
}

/// A frame's answer when the mood turned: the mood, and the rig replayed for it (or why it could not be).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Step {
    pub mood: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn: Option<Turn>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error: String,
}

/// A knob as a slider shows it: its range, value, curve and label in one locale.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Slider {
    pub path: String,
    pub min: f32,
    pub max: f32,
    pub value: f32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub log: bool,
    pub label: String,
}

/// The deck: its mood ladder and the mood it last showed, its own scene and the rig it plays.
#[derive(Debug, Clone)]
pub struct Deck {
    pub ladder: Ladder,
    /// the mood last handed out by `step` ("" before the first frame)
    shown: String,
    /// the deck's own scene (the X1's cathedral) – it plays whenever no rig does
    own: Played,
    /// the rig, its persona, and how it plays for the mood now
    rig: Option<(Bundle, String, Played)>,
}

impl Default for Deck {
    fn default() -> Self {
        Deck::new()
    }
}

impl Deck {
    pub fn new() -> Deck {
        let own = Played::deck_scene().expect("the deck's own scene is embedded and checked by the tests");
        Deck { ladder: Ladder::default(), shown: String::new(), own, rig: None }
    }

    /// the scene that plays: the rig's for the mood now, else the deck's own
    pub fn played(&self) -> &Played {
        self.rig.as_ref().map_or(&self.own, |(_, _, p)| p)
    }

    /// what plays now, as a turn to load whole
    pub fn now(&self) -> Turn {
        self.turn(Why::Play, true, Vec::new(), String::new(), false)
    }

    fn turn(&self, why: Why, reload: bool, values: Vec<(String, f32)>, note: String, failed: bool) -> Turn {
        Turn { why, rig: self.rig.is_some(), reload, values, played: self.played().clone(), note, failed }
    }

    fn mood_arg(&self) -> Option<String> {
        Some(self.ladder.mood().to_string()).filter(|m| !m.is_empty())
    }

    /// Play a rig for `persona` and the mood now (None = the deck's own scene again). An error leaves what played.
    pub fn play(&mut self, bundle: Option<Bundle>, persona: &str) -> Result<Turn> {
        self.rig = match bundle {
            None => None,
            Some(b) => {
                let persona_arg = Some(persona).filter(|p| !p.is_empty());
                let played = b.play(persona_arg, self.mood_arg().as_deref())?;
                Some((b, persona.to_string(), played))
            }
        };
        let note = match self.rig.as_ref() {
            Some((b, _, p)) => format!("played {}: {} knobs, {} nodes", b.rig.subject, p.knobs.len(), p.nodes.len()),
            None => "the deck's own scene plays".into(),
        };
        Ok(self.turn(Why::Play, true, Vec::new(), note, false))
    }

    /// `play` with a bundle as JSON ("" = the deck's own scene)
    pub fn play_json(&mut self, bundle: &str, persona: &str) -> Result<Turn> {
        let b = if bundle.is_empty() { None } else { Some(Bundle::parse_json(bundle)?) };
        self.play(b, persona)
    }

    /// The core rig of a character (`catpope`) or a subject (`character.catpope`, `item.<id>`) – the deck's own scene
    /// when the deck carries none or it fails (the note says which, `failed` that it failed).
    pub fn play_core(&mut self, character: &str, persona: &str) -> Turn {
        let subject = if character.contains('.') { character.to_string() } else { format!("character.{character}") };
        let tried = match Bundle::core(&subject) {
            None => Ok(None),
            Some(b) => b.and_then(|b| self.play(Some(b), persona).map(Some)),
        };
        match tried {
            Ok(Some(t)) => t,
            Ok(None) => {
                self.rig = None;
                self.turn(Why::Play, true, Vec::new(), format!("no core rig for {subject} – the deck's own scene plays"), false)
            }
            Err(e) => {
                self.rig = None;
                self.turn(Why::Play, true, Vec::new(), format!("{subject}: {e:#} – the deck's own scene plays"), true)
            }
        }
    }

    /// The rig replayed for the mood now: what moved, or a new scene to load; None without a rig.
    fn replay(&mut self) -> Result<Option<Turn>> {
        let mood = self.mood_arg();
        let Some((bundle, persona, played)) = self.rig.as_mut() else { return Ok(None) };
        let next = bundle.play(Some(persona.as_str()).filter(|p| !p.is_empty()), mood.as_deref())?;
        let d = played.diff(&next);
        *played = next;
        let note = format!("mood {}: {} values{}", mood.unwrap_or_default(), d.values.len(), if d.reload { ", a new scene" } else { "" });
        Ok(Some(self.turn(Why::Mood, d.reload, d.values, note, false)))
    }

    /// One frame: the sound's level (0..1), the clock's tempo and whether it runs, a hand on the deck (0..1), `dt` s →
    /// when the mood is another than the last frame's (the ladder's turn, a pinned mood, another character's ladder):
    /// the mood and the rig replayed for it.
    pub fn step(&mut self, level: f32, bpm: f32, running: bool, touch: f32, dt: f32) -> Option<Step> {
        let mood = self.ladder.step(level, bpm, running, touch, dt).to_string();
        if mood == self.shown {
            return None;
        }
        self.shown = mood.clone();
        Some(match self.replay() {
            Ok(turn) => Step { mood, turn, error: String::new() },
            Err(e) => Step { mood, turn: None, error: format!("{e:#}") },
        })
    }

    /// the mood now (the ladder's or the pinned one)
    pub fn mood(&self) -> &str {
        self.ladder.mood()
    }

    /// where a value of `path` goes in the engine (None: a node the engine does not play)
    pub fn target<'a>(&self, path: &'a str) -> Option<&'a str> {
        self.played().target(path)
    }

    /// the knobs as sliders, labelled in `locale` (else English, else any, else the path)
    pub fn sliders(&self, locale: &str) -> Vec<Slider> {
        let k = &self.played().knobs;
        k.iter().map(|k| Slider { path: k.path.clone(), min: k.min, max: k.max, value: k.value, log: k.log, label: k.label_in(locale).to_string() }).collect()
    }

    /// The being on the wire: the played scene's nodes and links (no params – the values stream), its character – a
    /// rig's own, else the deck's `character` –, the mood ladder's moods, the knobs it shows.
    pub fn graph(&self, character: &str) -> protocol::DeckGraph {
        let p = self.played();
        protocol::DeckGraph {
            nodes: p.nodes.iter().map(|n| protocol::DeckNode { id: n.id.clone(), kind: n.kind.clone() }).collect(),
            links: p
                .links
                .iter()
                .map(|l| protocol::DeckLink {
                    from: l.from.clone(),
                    to: l.to.clone(),
                    scale: (l.scale != 1.0).then(|| wire(l.scale)),
                    offset: (l.offset != 0.0).then(|| wire(l.offset)),
                })
                .collect(),
            character: Some(if self.rig.is_some() { p.character.clone() } else { character.to_string() }),
            size: Some(SIZE),
            moods: Some(self.ladder.wire()),
            expose: Some(p.expose.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mood::MOODS;

    fn quiet(d: &mut Deck) -> Option<Step> {
        d.step(0.0, 0.0, false, 0.0, 1.0 / 60.0)
    }

    /// The browser deck wears the X1's cathedral as the conductor builds it (deck/lab/fixtures/deck-graph.json, read by
    /// space/wire.rs's test): the being on the wire is this graph, byte for byte.
    #[test]
    fn the_deck_s_own_being_on_the_wire_is_the_fixture() {
        let mut d = Deck::new();
        d.ladder.set_portraits(&MOODS.map(String::from));
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../../lab/fixtures/deck-graph.json")).unwrap();
        // numbers as a page reads them (3000 and 3000.0 are one number)
        fn norm(v: serde_json::Value) -> serde_json::Value {
            use serde_json::Value::*;
            match v {
                Number(n) => serde_json::json!(n.as_f64()),
                Array(a) => Array(a.into_iter().map(norm).collect()),
                Object(o) => Object(o.into_iter().map(|(k, v)| (k, norm(v))).collect()),
                v => v,
            }
        }
        assert_eq!(norm(serde_json::to_value(d.graph("yellowmouse")).unwrap()), norm(fixture["p"].clone()));
    }

    /// shared/live-deck/test/fixtures/deck.json – the core's answers the browser deck's tests apply (#368: the page
    /// computes none of it). `UPDATE_FIXTURES=1 cargo test -p deck-space deck_fixture` writes it; a plain run fails when stale.
    #[test]
    fn deck_fixture_for_the_browser_deck_is_current() {
        fn j<T: Serialize>(v: &T) -> serde_json::Value {
            serde_json::to_value(v).unwrap()
        }
        // where every knob and every node's `gain` goes, and a node nobody has
        let targets = |d: &Deck| -> serde_json::Map<String, serde_json::Value> {
            let p = d.played();
            let paths = p.knobs.iter().map(|k| k.path.clone()).chain(p.nodes.iter().map(|n| format!("{}.gain", n.id))).chain(["nobody.gain".to_string()]);
            paths.map(|path| (path.clone(), serde_json::json!(d.target(&path).unwrap_or_default()))).collect()
        };
        let mut d = Deck::new();
        d.ladder.set_portraits(&MOODS.map(String::from));
        let own = serde_json::json!({ "now": j(&d.now()), "sliders": j(&d.sliders("en")), "graph": j(&d.graph("yellowmouse")), "targets": targets(&d) });
        let play = d.play_core("catpope", "");
        let mut catpope = serde_json::json!({ "play": j(&play), "sliders": j(&d.sliders("en")), "sliders_de": j(&d.sliders("de")), "graph": j(&d.graph("yellowmouse")), "targets": targets(&d) });
        d.ladder.pin(Some("laughing"));
        catpope["laughing"] = j(&quiet(&mut d).unwrap());
        let fixture = serde_json::json!({ "own": own, "catpope": catpope });
        let text = serde_json::to_string_pretty(&fixture).unwrap() + "\n";
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/live-deck/test/fixtures/deck.json");
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(path, &text).unwrap();
        }
        let on_disk = std::fs::read_to_string(path).unwrap_or_default().replace('\r', "");
        assert!(on_disk == text, "{path} is stale: UPDATE_FIXTURES=1 cargo test -p deck-space deck_fixture");
    }

    #[test]
    fn a_step_says_the_mood_only_when_it_turned() {
        let mut d = Deck::new();
        d.ladder.set_portraits(&MOODS.map(String::from));
        assert_eq!(quiet(&mut d).map(|s| s.mood), Some("neutral".into()), "the first frame says it");
        assert_eq!(quiet(&mut d), None);
        d.ladder.pin(Some("happy"));
        assert_eq!(quiet(&mut d).map(|s| s.mood), Some("happy".into()));
        d.ladder.pin(None);
        assert_eq!(quiet(&mut d).map(|s| s.mood), Some("neutral".into()));
    }

    /// rig.test.ts's cases, against the core (#368): a mood changes values only, a new routing reloads
    #[test]
    fn a_mood_replays_the_rig_values_only_unless_its_shape_changed() {
        let mut d = Deck::new();
        let t = d.play_core("catpope", "");
        assert!(t.rig && t.reload && !t.failed, "{}", t.note);
        d.ladder.pin(Some("laughing"));
        let s = quiet(&mut d).unwrap();
        let turn = s.turn.expect("the rig replayed");
        assert_eq!((turn.why, turn.reload), (Why::Mood, false));
        let mut values = turn.values.clone();
        values.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(values, [("voice_voice.index".to_string(), 4.0), ("voice_voice.ratio".to_string(), 3.0)]);
        let mut rerouted = d.played().clone();
        rerouted.links.push(crate::scene::LinkDef { from: "fft.level".into(), to: "voice_voice.index".into(), scale: 1.0, offset: 0.0, map: Vec::new() });
        assert!(d.played().diff(&rerouted).reload);
        assert!(!d.played().diff(&d.played().clone()).reload && d.played().diff(&d.played().clone()).values.is_empty());
    }

    #[test]
    fn the_facts_of_a_played_scene() {
        let mut d = Deck::new();
        d.play_core("character.catpope", "");
        let p = d.played();
        let paths: Vec<&str> = p.streamed.iter().map(|(p, _)| p.as_str()).collect();
        for k in &p.knobs {
            assert!(paths.contains(&k.path.as_str()), "{}", k.path);
        }
        assert!(paths.contains(&"fft.level") && paths.contains(&"fft.bass") && paths.contains(&"mix1.out"), "{paths:?}");
        assert_eq!(d.target("voice_melody.spread"), Some("voice_melody.spread"), "live-core's walk row");
        assert_eq!(d.target("mix1.gain1"), Some("mix1.gain1"), "the mixers are the engine's too");
        assert_eq!(d.target("nobody.gain"), None);
        let de = d.sliders("de");
        assert_eq!(de.iter().find(|s| s.path == "drone_drone.gain").map(|s| s.label.as_str()), Some("Bordun"));
        let g = d.graph("yellowmouse");
        assert_eq!(g.character.as_deref(), Some("catpope"), "a rig wears its own character");
        assert_eq!(g.expose.as_ref().map(Vec::len), Some(p.knobs.len()));
        // the deck's own scene: no rig sum, the deck's character, a node the browser does not carry yet takes nothing
        d.play(None, "").unwrap();
        let own = d.played();
        assert!(!own.streamed.iter().any(|(_, s)| *s == crate::rig::Stream::Audio));
        assert_eq!(d.graph("yellowmouse").character.as_deref(), Some("yellowmouse"));
        assert_eq!(d.target("music.gain"), None, "chapter_music is the page's");
    }

    #[test]
    fn a_missing_or_broken_rig_falls_back_to_the_deck_s_own_scene() {
        let mut d = Deck::new();
        let t = d.play_core("nobody", "");
        assert!(!t.rig && !t.failed && t.note.contains("no core rig for character.nobody"), "{}", t.note);
        assert!(d.play_json("{\"rig\": 1}", "").is_err(), "an explicit bad bundle is an error");
        assert!(!d.now().rig, "and what played stays");
    }
}
