//! A deck's state without a stage, a window or a sound device (#367, #368): what a platform's deck holds around the
//! engine and the 3D – its mood (mood.rs `Ladder`) and what it plays (its own scene, or a character's rig replayed on
//! every mood: rig/bundle.rs `Played`, the lifecycle here once). The browser deck has it as space-web's `DeckState` (no
//! GPU: it works when the 3D can't), the headless host drives it directly. The page passes its inputs each frame,
//! applies the turns (a new scene to load, values that moved) and draws the answers.
//! A code rig (#410, T3 of #314): its policy chooses a track's preset through an `@preset` output; the platform hands the
//! deck those outputs' values each frame (`choose`), and the deck replays the rig with the choices – the mood's replay,
//! the same diff (without the inputs the policy owns), at most one frame after the policy wrote.
use crate::context::Context;
use crate::mood::Ladder;
use crate::rig::{Bundle, Played};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// the being's size on the wire (its radius, m)
const SIZE: f64 = 0.5;

/// an f32 as the wire's f64 – its shortest decimal (0.6, not 0.6000000238), as the scene document wrote it
fn wire(x: f32) -> f64 {
    x.to_string().parse().unwrap_or(f64::from(x))
}

/// Why the deck's scene changed: a scene played (the deck's own or a rig), the rig replayed for a mood, or for the
/// presets its policy chose (#410).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Why {
    Play,
    Mood,
    Preset,
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

/// A frame's answer when the mood turned (or a code rig's policy chose another preset, `choose`): the mood, and the rig
/// replayed for it (or why it could not be).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Step {
    pub mood: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn: Option<Turn>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error: String,
}

/// A knob as a slider shows it: its range, value, curve and label in one locale (the page hands it on to the controls, #369).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slider {
    pub path: String,
    pub min: f32,
    pub max: f32,
    pub value: f32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub log: bool,
    #[serde(default)]
    pub label: String,
}

impl Slider {
    /// its range as a parameter (params.rs: value ↔ 0..1 on its curve)
    pub fn spec(&self) -> crate::params::Spec {
        crate::params::Spec::knob(self.min, self.max, self.value, self.log)
    }
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
    /// #410: the presets a code rig's policy chose (track → preset), played over the tables on every replay
    chosen: BTreeMap<String, String>,
    /// the last frame's hand on the deck (0..1, `step`)
    touch: f32,
    /// how hard the output's limiter works (0..1, the being's red flush) – the platform's, where it has one (#409)
    pub strain: f32,
}

impl Default for Deck {
    fn default() -> Self {
        Deck::new()
    }
}

impl Deck {
    pub fn new() -> Deck {
        let own = Played::deck_scene().expect("the deck's own scene is embedded and checked by the tests");
        Deck { ladder: Ladder::default(), shown: String::new(), own, rig: None, chosen: BTreeMap::new(), touch: 0.0, strain: 0.0 }
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
        self.chosen.clear();
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
                self.chosen.clear();
                self.turn(Why::Play, true, Vec::new(), format!("no core rig for {subject} – the deck's own scene plays"), false)
            }
            Err(e) => {
                self.rig = None;
                self.chosen.clear();
                self.turn(Why::Play, true, Vec::new(), format!("{subject}: {e:#} – the deck's own scene plays"), true)
            }
        }
    }

    /// The rig replayed for the mood now and the presets its policy chose: what moved (without the inputs the policy
    /// owns, `Played::diff`), or a new scene to load; None without a rig.
    fn replay(&mut self, why: Why) -> Result<Option<Turn>> {
        let mood = self.mood_arg();
        let Some((bundle, persona, played)) = self.rig.as_mut() else { return Ok(None) };
        let next = bundle.play_with(Some(persona.as_str()).filter(|p| !p.is_empty()), mood.as_deref(), &self.chosen)?;
        let d = played.diff(&next);
        *played = next;
        let what = match why {
            Why::Preset => format!("presets {}", self.chosen.iter().map(|(t, p)| format!("{t}={p}")).collect::<Vec<_>>().join(" ")),
            _ => format!("mood {}", mood.unwrap_or_default()),
        };
        let note = format!("{what}: {} values{}", d.values.len(), if d.reload { ", a new scene" } else { "" });
        Ok(Some(self.turn(why, d.reload, d.values, note, false)))
    }

    /// #410: the code rig's preset outputs – the paths the platform reads each frame and hands to `choose`, in order
    /// (`policy.<out>`, streamed as control outputs); none without a code rig
    pub fn preset_paths(&self) -> impl Iterator<Item = &str> + '_ {
        self.rig.iter().flat_map(|(_, _, p)| p.presets.iter().map(|c| c.path.as_str()))
    }

    /// #410: one frame's values of the code rig's preset outputs (`preset_paths`' order; the index into the output's
    /// choices, rounded and clamped; NaN = not heard, nothing chosen). When a track's choice is another preset than it
    /// plays: the rig replayed with the choices – a `Step` like a mood's (why `preset`); else None. A choice holds over
    /// every later mood replay until the policy chooses again or another rig plays. An output the policy has not
    /// written yet reads 0: its first choice.
    pub fn choose(&mut self, values: &[f32]) -> Option<Step> {
        let (_, _, played) = self.rig.as_ref()?;
        let mut picks = Vec::new();
        for (c, &v) in played.presets.iter().zip(values) {
            let Some(last) = c.choices.len().checked_sub(1) else { continue };
            if !v.is_finite() {
                continue;
            }
            let pick = &c.choices[(v.round().max(0.0) as usize).min(last)];
            let plays = played.tracks.iter().find(|t| t.track == c.track).and_then(|t| t.preset.as_ref());
            if self.chosen.get(&c.track).or(plays) != Some(pick) {
                picks.push((c.track.clone(), pick.clone()));
            }
        }
        if picks.is_empty() {
            return None;
        }
        self.chosen.extend(picks);
        let mood = self.ladder.mood().to_string();
        Some(match self.replay(Why::Preset) {
            Ok(turn) => Step { mood, turn, error: String::new() },
            Err(e) => Step { mood, turn: None, error: format!("{e:#}") },
        })
    }

    /// One frame: the sound's level (0..1), the clock's tempo and whether it runs, a hand on the deck (0..1), `dt` s →
    /// when the mood is another than the last frame's (the ladder's turn, a pinned mood, another character's ladder):
    /// the mood and the rig replayed for it.
    pub fn step(&mut self, level: f32, bpm: f32, running: bool, touch: f32, dt: f32) -> Option<Step> {
        self.touch = touch;
        let mood = self.ladder.step(level, bpm, running, touch, dt).to_string();
        if mood == self.shown {
            return None;
        }
        self.shown = mood.clone();
        Some(match self.replay(Why::Mood) {
            Ok(turn) => Step { mood, turn, error: String::new() },
            Err(e) => Step { mood, turn: None, error: format!("{e:#}") },
        })
    }

    /// the mood now (the ladder's or the pinned one)
    pub fn mood(&self) -> &str {
        self.ladder.mood()
    }

    /// #409: the context the engine's module nodes play in now – the mood, the rig's persona, the energy, the last
    /// frame's touch, the strain (context.rs); the platform sends it every frame (deck-engine `Controls::set_context`)
    pub fn context(&self) -> Context {
        let persona = self.rig.as_ref().map_or("", |(_, p, _)| p.as_str());
        Context::new(self.ladder.mood(), persona, self.ladder.energy(), self.touch, self.strain)
    }

    /// where a value of `path` goes in the engine (None: a node the engine does not play)
    pub fn target<'a>(&self, path: &'a str) -> Option<&'a str> {
        self.played().target(path)
    }

    /// #369: where `value` sits on knob `path`'s range, 0..1 on its curve (a value the host lacks – NaN – reads as the
    /// knob's default; a path that is no knob: 0) – the creator's signal strip, the lab's sliders
    pub fn norm(&self, path: &str, value: f32) -> f32 {
        self.played().knobs.iter().find(|k| k.path == path).map_or(0.0, |k| crate::controls::norm_of(&k.spec(), value))
    }

    /// #369: knob `path`'s value at `t` (0..1 on its curve; None: no such knob)
    pub fn value_at(&self, path: &str, t: f32) -> Option<f32> {
        self.played().knobs.iter().find(|k| k.path == path).map(|k| k.spec().value(t))
    }

    /// the knobs as sliders, labelled in `locale` (else English, else any, else the path)
    pub fn sliders(&self, locale: &str) -> Vec<Slider> {
        let k = &self.played().knobs;
        k.iter().map(|k| Slider { path: k.path.clone(), min: k.min, max: k.max, value: k.value, log: k.log, label: k.label_in(locale).to_string() }).collect()
    }

    /// #282: the played scene's wiring – its nodes, ports, links and the paths of the cables' values (the rig designer's
    /// Nodes tab); none with the problem when it does not build
    pub fn wiring(&self) -> crate::rig::wiring::Wiring {
        self.played().wiring().unwrap_or_else(|e| crate::rig::wiring::Wiring::failed(format!("{e:#}")))
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
    use std::collections::BTreeMap;

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
        let own = serde_json::json!({ "now": j(&d.now()), "sliders": j(&d.sliders("en")), "graph": j(&d.graph("yellowmouse")), "targets": targets(&d), "wiring": j(&d.wiring()) });
        let play = d.play_core("catpope", "");
        let mut catpope = serde_json::json!({ "play": j(&play), "sliders": j(&d.sliders("en")), "sliders_de": j(&d.sliders("de")), "graph": j(&d.graph("yellowmouse")), "targets": targets(&d), "wiring": j(&d.wiring()) });
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

    /// #409: what the module nodes get – the mood the step said, the rig's persona, the walk's energy, the hand
    #[test]
    fn the_context_is_the_deck_s_mood_persona_and_senses() {
        let mut d = Deck::new();
        d.ladder.set_portraits(&MOODS.map(String::from));
        assert_eq!(d.context().mood(), "neutral");
        d.play_core("catpope", "dream");
        d.ladder.pin(Some("laughing"));
        d.step(1.0, 120.0, true, 0.5, 0.1);
        d.strain = 0.25;
        let c = d.context();
        assert_eq!((c.mood(), c.persona(), c.touch, c.strain), ("laughing", "dream", 0.5, 0.25));
        assert!(c.energy > 0.0 && c.energy == d.ladder.energy(), "{}", c.energy);
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

    /// what the engine's inputs read: every numeric param of the played nodes (the policy's aside), the inputs the policy
    /// owns set to its outputs (`owned` in `Played::owned`'s order)
    fn effective(p: &Played, owned: &[f32]) -> BTreeMap<String, f32> {
        let mut v = BTreeMap::new();
        for n in p.nodes.iter().filter(|n| n.id != crate::rig::policy::NODE) {
            for (k, x) in &n.params {
                if let Some(f) = x.as_float() {
                    v.insert(format!("{}.{k}", n.id), f as f32);
                }
            }
        }
        assert_eq!(p.owned.len(), owned.len(), "{:?}", p.owned);
        v.extend(p.owned.iter().cloned().zip(owned.iter().copied()));
        v
    }

    /// #410 (T3 of #314): catpope as a code rig on the deck, its policy a stand-in – the outputs research/RIG-MODULE-GROUPS
    /// §3's module writes per mood, as the engine streams them – against the declarative catpope: after each mood's
    /// replay, the policy's `voice_preset` output replays the rig with the preset the tables choose; what the engine's
    /// inputs read is the same, and no turn moves an input the policy owns
    #[test]
    fn a_code_rig_s_preset_outputs_replay_it_with_the_tables_choices() {
        // mood → the policy's outputs (voice_preset, voice_index, drone_gain)
        let policy = [("neutral", 0.0, 2.0, 0.1), ("thinking", 1.0, 1.0, 0.1), ("sad", 1.0, 1.0, 0.14), ("happy", 2.0, 2.0, 0.1), ("laughing", 2.0, 4.0, 0.1), ("surprised", 2.0, 6.0, 0.05)];
        let mut tables = Deck::new();
        tables.play_core("catpope", "");
        let mut code = Deck::new();
        code.play(Some(crate::rig::policy::tests::code_catpope()), "").unwrap();
        assert_eq!(code.preset_paths().collect::<Vec<_>>(), ["policy.voice_preset"]);
        assert_eq!(code.played().owned, ["voice_voice.index", "drone_drone.gain"]);
        assert_eq!(tables.preset_paths().count(), 0, "a declarative rig has none");
        let preset = |d: &Deck| d.played().tracks.iter().find(|t| t.track == "voice").and_then(|t| t.preset.clone());
        let mut chose = Vec::new();
        for (mood, choice, index, drone) in policy {
            for d in [&mut tables, &mut code] {
                d.ladder.pin(Some(mood));
                let turn = quiet(d).and_then(|s| s.turn).expect("the mood replayed");
                assert!(!turn.values.iter().any(|(p, _)| d.played().owned.contains(p)), "{mood}: {:?}", turn.values);
            }
            // the policy's output: once it is another preset than the voice plays, a replay (why preset)
            if let Some(s) = code.choose(&[choice]) {
                let turn = s.turn.expect("replayed");
                assert_eq!((s.mood.as_str(), turn.why, turn.reload, s.error.as_str()), (mood, Why::Preset, false, ""));
                assert!(!turn.values.iter().any(|(p, _)| p == "voice_voice.index"), "low's index is the policy's: {:?}", turn.values);
                chose.push(mood);
            }
            assert_eq!(code.choose(&[choice]), None, "{mood}: the same choice again replays nothing");
            assert_eq!(code.choose(&[f32::NAN]), None, "not heard: nothing chosen");
            assert_eq!(preset(&code), preset(&tables), "{mood}: the preset the tables choose");
            assert_eq!(effective(code.played(), &[index, drone]), effective(tables.played(), &[]), "{mood}");
        }
        assert_eq!(chose, ["thinking", "happy"], "a choice holds over the moods that keep it");
        // out of range clamps to the last choice; another rig forgets the choices
        code.choose(&[-3.0]);
        assert_eq!(preset(&code).as_deref(), Some("cathedral_voice.base"));
        code.choose(&[9.0]);
        assert_eq!(preset(&code).as_deref(), Some("cathedral_voice.bright"));
        code.play(Some(crate::rig::policy::tests::code_catpope()), "").unwrap();
        assert_eq!(preset(&code).as_deref(), Some("cathedral_voice.base"));
        assert_eq!(tables.choose(&[1.0]), None, "a declarative rig chooses nothing");
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
