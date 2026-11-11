//! Pieces a deck does not carry, fetched (#479, agents/research/SHOW.md §5): a show's cell may name a character or an
//! item whose rig is no core rig. Its published rig is in scene-content (#271, `rig/content.rs`), so a deck asks there –
//! once per piece – and plays it when the answer is here. Sans IO: `find` answers from what is here and notes what to
//! ask, the platform fetches `asks` (the API's path) and hands the bytes or the reason back (`give`). The piece side
//! (`piece.rs`) and a flash's check (deck-show `ready.rs`) read the same store.
//!
//! No place serves a scene that is not built in, nor a bare rig (`rig:<id>`), yet: those stay "not on this deck".
use crate::piece::Found;
use crate::rig::address::Place;
use crate::rig::content::{safe_id, SceneRigs};
use std::collections::HashMap;

/// the store keeps this many pieces; a show names a handful (no growth without bound, RULES A10)
pub const CAP: usize = 64;

#[derive(Debug, Clone)]
enum Got {
    Asked,
    Have(Found),
    Failed(String),
}

/// What a deck fetched of the pieces it does not carry.
#[derive(Debug, Clone, Default)]
pub struct Fetched {
    got: HashMap<Place, Got>,
    asks: Vec<Place>,
    /// nothing is asked for: the deck plays what it carries
    carried_only: bool,
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
            Some(Got::Asked) => None,
            Some(Got::Failed(_)) => match self.got.remove(place) {
                Some(Got::Failed(why)) => Some(Found::Missing(why)),
                _ => None,
            },
            None => {
                if !self.carried_only && path(place).is_some() && self.got.len() < CAP {
                    self.got.insert(place.clone(), Got::Asked);
                    self.asks.push(place.clone());
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
            Got::Asked => None,
        }
    }

    /// `place` was asked for and its answer is not here yet
    pub fn waits(&self, place: &Place) -> bool {
        matches!(self.got.get(place), Some(Got::Asked))
    }

    /// some answer is not here yet
    pub fn waiting(&self) -> bool {
        self.got.values().any(|g| matches!(g, Got::Asked))
    }

    /// What failed is asked for again the next time (a flash: its check starts fresh).
    pub fn again(&mut self) {
        self.got.retain(|_, g| !matches!(g, Got::Failed(_)));
    }

    /// what the platform fetches now: the piece and its path (`path`)
    pub fn asks(&mut self) -> Vec<(Place, String)> {
        std::mem::take(&mut self.asks).into_iter().filter_map(|p| path(&p).map(|at| (p, at))).collect()
    }

    /// the platform's answer for a piece it was asked to fetch: the bytes, or why not (offline, HTTP 404)
    pub fn give(&mut self, place: &Place, answer: Result<&str, String>) {
        if !self.waits(place) {
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
