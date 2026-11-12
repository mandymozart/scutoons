//! The native deck plays a rig (#322, plan P9a) the way every deck does: deck-space's `Deck` (deck.rs, #368 – the browser's
//! space-web `DeckState`, the headless host) plays it – `Bundle::play` flattens it (`Rig::flatten`) at the mood now, or
//! plays no sound when the deck carries no such rig or it fails (#425: the core's silent scene – the being, no audio
//! graph) – and replays it when the mood turns.
//! This file only moves what the core says: the played scene is loaded like any scene (its sound through deck-engine – the
//! browser's live-core plays the same `engine` part of it, bit for bit: deck-conformance tests/rig.rs), and a replay's
//! values go into the control domain. Asked for as `--scene rig:<character | subject>[@<mood>]` (deck.toml's `scene`, the
//! F5 switcher's value): `rig:catpope`, `rig:item.incense_burner@happy`.
//!
//! The rigs are the core rigs the image carries (`rig::bundle::CORE`); scene-content's catalog rigs and their cache are #271.
//! A code rig (#410, T3 of #314) plays the same way: its policy is a module node of the scene, and each frame its
//! `@preset` outputs, read from the heard block, go to the core (`Deck::choose`), which replays the rig with the presets
//! it chose – the values that moved go into the control domain like a mood's.
use crate::control::Controls;
use crate::graph::Graph;
use crate::scene::{LinkDef, NodeDef, Scene};
use anyhow::{anyhow, Result};
use deck_space::deck::{Deck, Step, Turn};
use deck_space::rig::{Bundle, Played};
use std::path::Path;

/// the scene argument's prefix that names a rig
pub const PREFIX: &str = "rig:";

/// What a scene argument names as a rig (`rig:catpope@laughing` → `catpope`, `laughing`); None: no rig.
pub fn named(arg: &Path) -> Option<(&str, Option<&str>)> {
    let rest = arg.to_str()?.strip_prefix(PREFIX)?;
    Some(match rest.split_once('@') {
        Some((subject, mood)) => (subject, Some(mood).filter(|m| !m.is_empty())),
        None => (rest, None),
    })
}

/// The scene `arg` names (`named`): what the core plays for it – the rig at its mood, else the silent scene (#425).
pub fn scene(arg: &Path) -> Result<Scene> {
    Player::start(arg).ok_or_else(|| anyhow!("'{}' names no rig ({PREFIX}<character>)", arg.display()))?.scene()
}

/// The rigs that need modules (#417): name → the bundle's TOML (deck/native/rigs/). Their nodes are `module:<name>` kinds that
/// only a deck built with the `modules` feature (wasmtime) and its wasm in `DECK_MODULES` has, so they are not in the core's
/// `rig::bundle::CORE` (the browser, the headless host and the server's catalog carry no module): `--scene rig:catpope-analog`.
pub const MODULE_RIGS: [(&str, &str); 2] = [("catpope-analog", include_str!("../rigs/catpope_analog.toml")), ("far-listener", include_str!("../rigs/far_listener.toml"))];

/// A module rig played (`MODULE_RIGS`): the modules of `DECK_MODULES` register their kinds first; a build without them (or
/// without the wasm) says so in the log and plays the silent scene (#425), like a rig that fails.
fn module_rig(deck: &mut Deck, name: &str, text: &str) -> Turn {
    #[cfg(feature = "modules")]
    crate::module_host::autoload();
    match toml::from_str::<Bundle>(text).map_err(anyhow::Error::from).and_then(|b| deck.play(Some(b), "")) {
        Ok(turn) => turn,
        Err(e) => {
            tracing::warn!("rig: {name}: {e:#} – it needs the modules (build with --features modules, their wasm in DECK_MODULES); the deck is silent");
            deck.play(None, "").expect("the silent scene plays")
        }
    }
}

/// A rig played on this deck: the core's deck (its ladder pinned to the being's mood) and the played scene's name.
pub struct Player {
    deck: Deck,
    /// the played scene's stage name (the rig's subject): whether a loaded scene is this one
    name: String,
}

impl Player {
    fn start(arg: &Path) -> Option<Player> {
        let (subject, mood) = named(arg)?;
        let mut deck = Deck::new();
        deck.ladder.pin(mood);
        let turn = match MODULE_RIGS.iter().find(|(n, _)| *n == subject) {
            Some((n, text)) => module_rig(&mut deck, n, text),
            None => deck.play_core(subject, ""),
        };
        log(&turn);
        let name = Scene::parse(&turn.played.scene).map(|s| s.stage.name).unwrap_or_default();
        Some(Player { deck, name })
    }

    /// A rig bundle played on this deck at `mood` (a catalog rig, a code rig with its policy's descriptor) – an error when
    /// it does not play (no fallback: the caller asked for this bundle).
    pub fn play(bundle: Bundle, mood: Option<&str>) -> Result<Player> {
        let mut deck = Deck::new();
        deck.ladder.pin(mood);
        let turn = deck.play(Some(bundle), "")?;
        log(&turn);
        let name = Scene::parse(&turn.played.scene)?.stage.name;
        Ok(Player { deck, name })
    }

    /// what the core plays now (the presets each track plays, the scene)
    pub fn played(&self) -> &Played {
        self.deck.played()
    }

    /// The player of the rig `arg` names, when `scene` is what it plays (not the test pattern after a failed load, not the
    /// silent scene after a failed rig: nothing to replay then).
    pub fn of(arg: Option<&Path>, scene: &Scene) -> Option<Player> {
        Player::start(arg?).filter(|p| p.deck.now().rig && p.name == scene.stage.name)
    }

    /// the played scene, its bare space node staged in the deck's own room (`staged`)
    pub fn scene(&self) -> Result<Scene> {
        Ok(staged(Scene::parse(&self.deck.played().scene)?))
    }

    /// One frame: the being's mood (`mood` – the head's, the same energy walk as every deck's) to the core; when it turned,
    /// the values the core's replay moved go into the control domain as their inputs' bases (a knob's, a preset's param).
    /// Then a code rig's `@preset` outputs as the heard block left them (#410): when the policy chose another preset, the
    /// core's replay with it, applied the same way. A replay that changes the rig's shape needs a new scene: the core
    /// says so, the deck keeps the shape it plays until the next load (F5, `--dev`) – no core rig changes its shape with
    /// a mood.
    pub fn frame(&mut self, mood: &str, graph: &Graph, ctl: &mut Controls, dt: f32) {
        self.deck.ladder.pin(Some(mood));
        let heard: Vec<f32> = self.deck.preset_paths().map(|p| output(graph, ctl, p)).collect();
        if let Some(step) = self.deck.frame(0.0, 0.0, false, 0.0, dt, &heard) {
            apply(step, graph, ctl);
        }
    }
}

/// a replay's values into the control domain as their inputs' bases (in order: a later one on a path wins)
fn apply(step: Step, graph: &Graph, ctl: &mut Controls) {
    let Some(turn) = step.turn else {
        if !step.error.is_empty() {
            tracing::warn!("rig: mood {}: {} – it plays on as it was", step.mood, step.error);
        }
        return;
    };
    if turn.reload {
        tracing::warn!("rig: {} – a new shape, loaded with the next scene load (F5)", turn.note);
        return;
    }
    for (path, value) in &turn.values {
        set(graph, ctl, path, *value);
    }
    tracing::info!("rig: {}", turn.note);
}

/// one value (`node.port`) into the control domain as its input's base (a replay's)
pub fn set(graph: &Graph, ctl: &mut Controls, path: &str, value: f32) -> bool {
    ctl.set_path(graph, path, value)
}

/// a control output's value in the heard block (`node.output`; NaN: no such node or output)
fn output(graph: &Graph, ctl: &Controls, path: &str) -> f32 {
    let at = path.rsplit_once('.').and_then(|(node, out)| {
        let i = graph.nodes.iter().position(|n| n.id == node)?;
        Some((i, graph.nodes[i].kind.output(out)?))
    });
    at.and_then(|(i, o)| ctl.outputs(i).get(o).copied()).unwrap_or(f32::NAN)
}

/// One frame of the being (#409, T2 of #314): its live context to every module node of the engine – the mood its head
/// shows, the persona (the native deck plays none yet), its energy, a hand on it, its strain – then, when a rig plays,
/// the rig's replay on that mood (`Player::frame`). The face sends an unchanged context only once.
#[cfg(feature = "space")]
pub fn frame(rig: Option<&mut Player>, being: &crate::space::being::Being, graph: &Graph, ctl: &mut Controls, dt: f32) {
    let mood = being.moods.get(being.head.mood).map_or("", String::as_str);
    let touch = if being.dragging { 1.0 } else { 0.0 };
    ctl.set_context(crate::control::Context::new(mood, "", being.head.walk.energy, touch, being.strain));
    if let Some(r) = rig.filter(|_| !mood.is_empty()) {
        r.frame(mood, graph, ctl, dt);
    }
}

fn log(turn: &Turn) {
    if turn.failed {
        tracing::warn!("rig: {}", turn.note);
    } else {
        tracing::info!("rig: {}", turn.note);
    }
}

/// A rig's space node comes bare ("the caller sets its set", `Played::scene`): the deck plays it in the staging (#427:
/// its character's home set, none yet – the neutral staging, never the cathedral) – every param of the staging's space
/// node (set, camera, light, join, post) the rig's does not set itself – and puts its
/// picture on the window: the flattened rig has no `screen` node (the browser deck draws the space itself), so without
/// `space → screen` a native deck showed only the HUD on black (#418).
fn staged(mut scene: Scene) -> Scene {
    let own = crate::scene::staging(None).nodes.into_iter().find(|n| n.kind == "space");
    if let (Some(own), Some(space)) = (own, scene.nodes.iter_mut().find(|n| n.kind == "space")) {
        for (k, v) in own.params {
            space.params.entry(k).or_insert(v);
        }
    }
    if scene.nodes.iter().any(|n| n.kind == "space") && !scene.nodes.iter().any(|n| n.kind == "screen") {
        scene.nodes.push(NodeDef { id: "screen".into(), kind: "screen".into(), params: toml::Table::new() });
        scene.links.push(LinkDef { from: "space".into(), to: "screen.in".into(), scale: 1.0, offset: 0.0, map: Vec::new() });
    }
    scene
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scene_argument_names_a_rig_and_its_mood() {
        assert_eq!(named(Path::new("rig:catpope")), Some(("catpope", None)));
        assert_eq!(named(Path::new("rig:item.incense_burner@happy")), Some(("item.incense_burner", Some("happy"))));
        assert_eq!(named(Path::new("rig:catpope@")), Some(("catpope", None)));
        assert_eq!(named(Path::new("cathedral")), None);
        assert_eq!(named(Path::new("scenes/rig.toml")), None);
    }

    /// the core's played scene, as it is, in the deck's own room: the same nodes and links, the space node staged
    #[test]
    fn catpope_plays_the_core_s_scene_in_the_deck_s_room() {
        let scene = scene(Path::new("rig:catpope")).unwrap();
        let mut deck = Deck::new();
        let played = deck.play_core("catpope", "").played;
        assert_eq!(scene.stage.name, "character.catpope");
        let ids = |s: &[crate::scene::NodeDef]| s.iter().map(|n| (n.id.clone(), n.kind.clone())).collect::<Vec<_>>();
        // the played scene's nodes and links as they are, plus the window's `screen` fed by the space (#418)
        let mut nodes = ids(&played.nodes);
        nodes.push(("screen".into(), "screen".into()));
        assert_eq!(ids(&scene.nodes), nodes);
        assert_eq!(scene.links[..played.links.len()], played.links[..]);
        let last = scene.links.last().unwrap();
        assert_eq!((scene.links.len(), last.from.as_str(), last.to.as_str()), (played.links.len() + 1, "space", "screen.in"));
        let space = scene.nodes.iter().find(|n| n.kind == "space").unwrap();
        assert_eq!(space.params.get("set"), None, "the staging: no home set yet, no cathedral (#427)");
        assert_eq!(space.params.get("join").and_then(|v| v.as_str()), Some("deck"));
        crate::modules::check(&scene).unwrap();
        assert!(Player::of(Some(Path::new("rig:catpope")), &scene).is_some());
    }

    /// no such rig (#425): the core plays the silent scene (said in the log) – the being in the deck's room, no audio
    /// graph, nothing to replay. The cathedral is a test scene: no rig player, and no fallback either.
    #[test]
    fn a_rig_the_deck_lacks_plays_the_silent_scene() {
        let scene = scene(Path::new("rig:nobody")).unwrap();
        let cathedral = Scene::parse(crate::scene::CATHEDRAL_SCENE).unwrap();
        assert_eq!(scene.stage.name, "silent");
        assert!(scene.nodes.iter().all(|n| matches!(n.kind.as_str(), "space" | "screen")), "no audio node: {:?}", scene.nodes.iter().map(|n| &n.kind).collect::<Vec<_>>());
        let space = scene.nodes.iter().find(|n| n.kind == "space").unwrap();
        assert_eq!(space.params.get("set"), None, "the picture stays: the staging, no cathedral (#427)");
        assert!(Player::of(Some(Path::new("rig:nobody")), &scene).is_none());
        assert!(Player::of(Some(Path::new("cathedral")), &cathedral).is_none());
    }

    /// #417: `catpope-analog` is catpope plus the analog track and nothing else – its rig, instruments and presets equal
    /// catpope.toml's once the track, its instrument and its mood values are taken out
    #[test]
    fn catpope_analog_is_catpope_and_one_more_track() {
        let toml_of = |text: &str| toml::Value::try_from(toml::from_str::<Bundle>(text).unwrap()).unwrap();
        let catpope = toml_of(deck_space::rig::bundle::CORE.iter().find(|(s, _)| *s == "character.catpope").unwrap().1);
        let mut with = toml_of(MODULE_RIGS[0].1);
        let rig = with["rig"].as_table_mut().unwrap();
        let tracks = rig["track"].as_array_mut().unwrap();
        assert_eq!(tracks.pop().unwrap()["id"].as_str(), Some("analog"));
        for mood in rig["moods"].as_table_mut().unwrap().iter_mut().map(|(_, v)| v) {
            if let Some(values) = mood.get_mut("values").and_then(|v| v.as_table_mut()) {
                values.retain(|k, _| !k.starts_with("analog:"));
            }
        }
        with["instrument"].as_array_mut().unwrap().retain(|i| i["id"].as_str() != Some("analog_seq"));
        assert_eq!(with, catpope);
        // and nothing in the core carries it: the browser, the headless host and the catalog play without modules
        assert!(deck_space::rig::bundle::CORE.iter().all(|(_, text)| !text.contains("module:")));
    }

    /// a build (or a deck) without the modules says so and plays the silent scene (#425), like a rig that fails
    #[cfg(not(feature = "modules"))]
    #[test]
    fn a_module_rig_without_the_modules_plays_the_silent_scene() {
        let scene = scene(Path::new("rig:catpope-analog")).unwrap();
        assert_eq!(scene.stage.name, "silent");
        assert!(Player::of(Some(Path::new("rig:catpope-analog")), &scene).is_none());
    }

    /// a mood moves the core's values into the control domain – laughing: the voice's FM index 4, its ratio 3 (deck.rs)
    #[test]
    fn a_mood_moves_the_rig_s_values_into_the_controls() {
        let arg = Path::new("rig:catpope");
        let scene = scene(arg).unwrap();
        let graph = Graph::build(&scene).unwrap();
        let mut ctl = Controls::new(&graph, None);
        let mut p = Player::of(Some(arg), &scene).unwrap();
        let voice = graph.nodes.iter().position(|n| n.id == "voice_voice").unwrap();
        let index = |c: &Controls| c.input(voice, "index", f32::NAN);
        p.frame("neutral", &graph, &mut ctl, 1.0 / 60.0);
        assert_eq!(index(&ctl), 2.0, "neutral plays the defaults");
        p.frame("laughing", &graph, &mut ctl, 1.0 / 60.0);
        assert_eq!((index(&ctl), ctl.input(voice, "ratio", f32::NAN)), (4.0, 3.0));
        p.frame("neutral", &graph, &mut ctl, 1.0 / 60.0);
        assert_eq!(index(&ctl), 2.0, "and back");
    }
}
