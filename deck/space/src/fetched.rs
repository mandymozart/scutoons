//! Pieces a deck does not carry, fetched (#479, agents/research/SHOW.md §5): a show's cell may name a character or an
//! item whose rig is no core rig. Its published rig is in scene-content (#271, `rig/content.rs`), so a deck asks there –
//! once per piece – and plays it when the answer is here. Sans IO: `find` answers from what is here and notes what to
//! ask, the platform fetches `asks` (the API's path) and hands the bytes or the reason back (`give`). The piece side
//! (`piece.rs`) and a flash's check (deck-show `ready.rs`) read the same store.
//!
//! A published scene (`scene:<id>[@v]`, #246) is loaded through the same store by the core's loader (published.rs): its
//! document, then scene-content for what it needs, each an ask (`scene`, `answered` – `reply` takes either kind of
//! answer); the browser deck's `?scene=` starts one, and a flash's check one for each piece `scene:<id>` the deck does
//! not carry (#513, deck-show flashed.rs). A deck with a cache of its own (the native deck) drives the same loader with
//! its copy and hands the outcome back (`loaded`). A bare rig (`rig:<id>`) stays "not on this deck". #520: what the
//! loader would have a host keep of a scene it fetched (`Loaded::store`) stays here (`kept`) – the browser deck, which
//! keeps no cache file, plays its cast from it (published.rs `CastLink`).
use crate::piece::Found;
use crate::published::{Load, SceneRef, Step};
use crate::rig::address::Place;
use crate::rig::content::{safe_id, SceneRigs};
use std::collections::HashMap;

/// the store keeps this many pieces; a show names a handful (no growth without bound, RULES A10)
pub const CAP: usize = 64;

#[derive(Debug, Clone)]
enum Got {
    Asked,
    /// a published scene on its way (its loader asks again until it plays or fails)
    Loading(Box<Load>),
    Have(Found),
    Failed(String),
}

/// What a deck fetched of the pieces it does not carry.
#[derive(Debug, Clone, Default)]
pub struct Fetched {
    got: HashMap<Place, Got>,
    /// what the platform fetches next: the piece and its path
    asks: Vec<(Place, String)>,
    /// nothing is asked for: the deck plays what it carries
    carried_only: bool,
    /// #520: each published scene here, as the loader would have a host keep it (`Loaded::store`)
    kept: HashMap<Place, String>,
}

/// Where `place` is asked for: the path after the API's base. None = no place serves its kind.
pub fn path(place: &Place) -> Option<String> {
    let (characters, items) = match place.kind.as_str() {
        "character" => (place.id.as_str(), ""),
        "item" => ("", place.id.as_str()),
        _ => return None,
    };
    safe_id(&place.id).then(|| format!("/library/scene-content?sets=&characters={characters}&items={items}"))
}

/// scene-content's answer → the published rig of `place`. Err = why it does not play here, in the HUD's words.
pub fn read(answer: &str, place: &Place) -> Result<Found, String> {
    let rigs = SceneRigs::parse_json(answer).map_err(|e| format!("{place}: {e:#}"))?;
    let entry = match place.kind.as_str() {
        "character" => rigs.character(&place.id),
        "item" => rigs.item(&place.id),
        _ => None,
    };
    let entry = entry.ok_or_else(|| format!("{place} has no published rig"))?;
    rigs.bundle(entry).map(|b| Found::Bundle(Box::new(b))).map_err(|e| format!("{place}: {e:#}"))
}

impl Fetched {
    /// The store of a deck that fetches no pieces (the native deck: its cache is what it carries): nothing is asked
    /// for, so a piece that is not there says so at once.
    pub fn carried_only() -> Fetched {
        Fetched { carried_only: true, ..Fetched::default() }
    }

    /// What is here of `place`, for a cell. A piece that can be asked for and was not yet is noted (`asks`) and None
    /// until its answer (`waits`). One that failed says why once and is asked again the next time a cell names it.
    pub fn find(&mut self, place: &Place) -> Option<Found> {
        match self.got.get(place) {
            Some(Got::Have(f)) => Some(f.clone()),
            Some(Got::Asked | Got::Loading(_)) => None,
            Some(Got::Failed(_)) => match self.got.remove(place) {
                Some(Got::Failed(why)) => Some(Found::Missing(why)),
                _ => None,
            },
            None => {
                if let Some(at) = path(place).filter(|_| !self.carried_only && self.got.len() < CAP) {
                    self.got.insert(place.clone(), Got::Asked);
                    self.asks.push((place.clone(), at));
                }
                None
            }
        }
    }

    /// What is here of `place`, without asking and without forgetting (a flash's check, once nothing waits).
    pub fn have(&self, place: &Place) -> Option<Found> {
        match self.got.get(place)? {
            Got::Have(f) => Some(f.clone()),
            Got::Failed(why) => Some(Found::Missing(why.clone())),
            Got::Asked | Got::Loading(_) => None,
        }
    }

    /// `place` was asked for and its answer is not here yet
    pub fn waits(&self, place: &Place) -> bool {
        matches!(self.got.get(place), Some(Got::Asked | Got::Loading(_)))
    }

    /// some answer is not here yet
    pub fn waiting(&self) -> bool {
        self.got.values().any(|g| matches!(g, Got::Asked | Got::Loading(_)))
    }

    /// What failed is asked for again the next time (a flash: its check starts fresh), and a published scene is loaded
    /// again (#513: a new version reaches a deck at its next flash – SCENES.md §4; an unchanged one costs its document).
    pub fn again(&mut self) {
        self.got.retain(|p, g| !matches!(g, Got::Failed(_)) && !(p.kind == "scene" && matches!(g, Got::Have(_))));
        let got = &self.got;
        self.kept.retain(|p, _| got.contains_key(p));
    }

    /// what the platform fetches now: the piece and its path (`path`)
    pub fn asks(&mut self) -> Vec<(Place, String)> {
        std::mem::take(&mut self.asks)
    }

    /// #246: what is here of the published scene `r` (its TOML as `Found::Scene`, or why it does not play); None = asked
    /// for (`asks`) or on its way. Loaded once per scene; one that failed says why once and is asked again next time.
    /// This store keeps no copy: a deck with a cache of its own drives `published::Load` with it (the native deck).
    pub fn scene(&mut self, r: &SceneRef) -> Option<Found> {
        let place = Place::new("scene", &r.id);
        if self.got.contains_key(&place) {
            return self.find(&place);
        }
        if self.got.len() >= CAP {
            return Some(Found::Missing(format!("{r}: too many pieces asked for")));
        }
        let (load, step) = Load::start(r.clone(), None);
        self.step(place.clone(), Box::new(load), step);
        self.find(&place)
    }

    /// #520: the published scene `r` as the loader would have a host keep it (`Loaded::store`: the document and
    /// scene-content's answer as they came) – None: not here, or it played without its sources (not kept)
    pub fn kept(&self, r: &SceneRef) -> Option<&str> {
        self.kept.get(&Place::new("scene", &r.id)).map(String::as_str)
    }

    /// #246: the platform's answer to a scene's ask: the HTTP status (0 = the server can't be reached) and the body
    pub fn answered(&mut self, place: &Place, status: u16, body: &str) {
        let Some(Got::Loading(mut load)) = self.got.remove(place) else { return };
        let step = load.answer(status, body);
        self.step(place.clone(), load, step);
    }

    /// The platform's answer to any ask of `asks`: the API's status and body, or why the fetch failed – a scene's loader
    /// takes it (`answered`, a failure as status 0), a piece its bytes (`give`).
    pub fn reply(&mut self, place: &Place, answer: Result<(u16, &str), String>) {
        match (self.got.get(place), answer) {
            (Some(Got::Loading(_)), Ok((status, body))) => self.answered(place, status, body),
            (Some(Got::Loading(_)), Err(_)) => self.answered(place, 0, ""),
            (_, answer) => self.give(place, answer.map(|(_, body)| body)),
        }
    }

    /// #513: a published scene the platform loaded itself – the core's loader with its own copy (the native deck's
    /// `<cache>/scenes/`): its TOML (`Loaded::text`), or why it does not play. Only a scene this store asked for.
    pub fn loaded(&mut self, place: &Place, got: Result<String, String>) {
        if !matches!(self.got.get(place), Some(Got::Loading(_))) {
            return;
        }
        let got = match got {
            Ok(text) => Got::Have(Found::Scene(text)),
            Err(why) => Got::Failed(why),
        };
        self.got.insert(place.clone(), got);
    }

    /// the loader's step: asked again, played (its TOML) or failed
    fn step(&mut self, place: Place, load: Box<Load>, step: Step) {
        if !matches!(step, Step::Ask(_)) {
            self.kept.remove(&place);
        }
        let got = match step {
            Step::Ask(at) => {
                self.asks.push((place.clone(), at));
                Got::Loading(load)
            }
            Step::Play(l) => match l.text() {
                Ok(text) => {
                    if let Some(kept) = l.store {
                        self.kept.insert(place.clone(), kept);
                    }
                    Got::Have(Found::Scene(text))
                }
                Err(why) => Got::Failed(why),
            },
            Step::Fail(why) => Got::Failed(why),
        };
        self.got.insert(place, got);
    }

    /// the platform's answer for a piece it was asked to fetch: the bytes, or why not (offline, HTTP 404)
    pub fn give(&mut self, place: &Place, answer: Result<&str, String>) {
        if !matches!(self.got.get(place), Some(Got::Asked)) {
            return;
        }
        let got = match answer.map_err(|why| format!("{place}: {why}")).and_then(|text| read(text, place)) {
            Ok(found) => Got::Have(found),
            Err(why) => Got::Failed(why),
        };
        self.got.insert(place.clone(), got);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::available;

    /// scene-content as the server writes it (its SceneContentRigsTest): catpope and the incense burner have a rig, boy none
    const ANSWER: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/rigs/scene-content.json"));

    #[test]
    fn a_piece_is_asked_for_once_and_plays_when_its_answer_is_here() {
        let mut f = Fetched::default();
        let burner = Place::new("item", "incense_burner");
        assert_eq!(path(&burner).as_deref(), Some("/library/scene-content?sets=&characters=&items=incense_burner"));
        assert!(f.find(&burner).is_none() && f.waits(&burner) && f.waiting());
        assert_eq!(f.asks(), vec![(burner.clone(), path(&burner).unwrap())]);
        // asked once: a second cell while it is on its way asks nothing
        assert!(f.find(&burner).is_none() && f.asks().is_empty());
        f.give(&burner, Ok(ANSWER));
        assert!(!f.waiting());
        let Some(Found::Bundle(b)) = f.find(&burner) else { panic!("the burner's bundle") };
        assert_eq!(b.rig.subject, "item.incense_burner");
    }

    #[test]
    fn what_is_not_there_says_why_and_is_asked_again() {
        let mut f = Fetched::default();
        let boy = Place::new("character", "boy");
        assert!(f.find(&boy).is_none());
        f.give(&boy, Ok(ANSWER));
        // a flash's check reads without forgetting; a cell hears it once, the next cell asks again
        assert_eq!(f.have(&boy), Some(Found::Missing("character:boy has no published rig".into())));
        assert_eq!(available("character:boy", |p| f.have(p)), Err("character:boy has no published rig".to_string()));
        assert_eq!(f.find(&boy), Some(Found::Missing("character:boy has no published rig".into())));
        assert!(f.find(&boy).is_none() && f.waits(&boy));
        f.asks();
        f.give(&boy, Err("HTTP 503".into()));
        assert_eq!(f.have(&boy), Some(Found::Missing("character:boy: HTTP 503".into())));
        f.again();
        assert_eq!(f.have(&boy), None);
    }

    /// #246: the browser deck's `?scene=scene:nave` – the document, then scene-content, then its TOML
    #[test]
    fn a_published_scene_is_loaded_through_the_store() {
        let nave = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../server/tests/fixtures/scenes/nave.json"));
        let mut doc: serde_json::Value = serde_json::from_str(nave).unwrap();
        doc["version"] = 2.into();
        let answer = serde_json::json!({ "status": "success", "scene": doc }).to_string();
        let mut f = Fetched::default();
        let r = crate::published::SceneRef::parse("scene:nave").unwrap().unwrap();
        let place = Place::new("scene", "nave");
        assert!(f.scene(&r).is_none() && f.waits(&place));
        assert_eq!(f.asks(), vec![(place.clone(), "/scenes/nave".to_string())]);
        assert!(f.scene(&r).is_none() && f.asks().is_empty(), "asked once");
        f.answered(&place, 200, &answer);
        let asks = f.asks();
        assert!(asks[0].1.starts_with("/library/scene-content?") && f.waits(&place));
        f.answered(&place, 200, ANSWER);
        let Some(Found::Scene(text)) = f.scene(&r) else { panic!("nave's TOML") };
        assert_eq!(crate::scene::Scene::parse(&text).unwrap().being_character(), "catpope");
        // #520: what a host would keep of it, for its cast (the browser keeps no file)
        let kept = f.kept(&r).expect("kept");
        assert!(crate::published::Cast::from_cache(kept).is_ok());
        f.again();
        assert!(f.kept(&r).is_none(), "loaded again: the copy goes with it");
        // unknown: said once, asked again next time
        let crypt = crate::published::SceneRef::parse("scene:crypt").unwrap().unwrap();
        assert!(f.scene(&crypt).is_none());
        f.answered(&Place::new("scene", "crypt"), 404, "");
        assert_eq!(f.scene(&crypt), Some(Found::Missing("scene:crypt is not published".into())));
        assert!(f.scene(&crypt).is_none());
    }

    #[test]
    fn a_kind_no_place_serves_is_not_asked_for() {
        let mut f = Fetched::default();
        for p in [Place::new("scene", "crypt"), Place::new("rig", "x"), Place::new("character", "../x")] {
            assert!(f.find(&p).is_none() && !f.waits(&p), "{p}");
        }
        assert!(f.asks().is_empty());
        // an answer nobody asked for is not kept
        f.give(&Place::new("character", "boy"), Ok(ANSWER));
        assert_eq!(f.have(&Place::new("character", "boy")), None);
    }
}
