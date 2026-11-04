//! A rig's mood horizon (#236, research/EVENTS.md §16 "Event horizons"): the moods a being can reach next, walked with
//! deck-events' horizon and played ahead of time, so a mood switch swaps in a pre-built `Played` instead of flattening
//! and lowering the rig on the trigger.
//!
//! The scope: the energy ladder (calm → wild, the being's `look.moods`, default `space::being::MOODS`) – a mood `n`
//! rungs from the current one is `n` rows away; a mood the rig names off the ladder (`sad`, from the feelings) can come
//! at any time, one row away behind a branch (depth 1). A mood the rig does not name plays the defaults, so it shares
//! the defaults' entry (the key `""`): one build per distinct result. Building costs no download (bytes 0) – the
//! portraits and sets are the cache's (#195); bound the walk by rows, depth or items.
use super::{Bundle, Played};
use anyhow::Result;
use deck_events::horizon::{self, Bound, Dist, Reach};
use std::collections::BTreeMap;

/// the defaults' key (no mood, or one the rig does not name)
pub const DEFAULTS: &str = "";

/// The moods of a horizon, played ahead – take one on the trigger.
#[derive(Debug, Clone, PartialEq)]
pub struct Prebuilt {
    pub persona: Option<String>,
    /// key (a mood the rig names, or `DEFAULTS`) → the rig played for it, nearest first in `order`
    built: BTreeMap<String, Played>,
    order: Vec<String>,
}

impl Prebuilt {
    /// the keys, nearest first
    pub fn keys(&self) -> &[String] {
        &self.order
    }

    /// the pre-built result for `mood` (a mood the rig doesn't name = the defaults'), if it was in the horizon
    pub fn get(&self, bundle: &Bundle, mood: Option<&str>) -> Option<&Played> {
        self.built.get(bundle.mood_key(mood))
    }

    /// `get`, else a fresh `play` (a mood past the horizon still plays – it only costs the build on the trigger)
    pub fn played(&self, bundle: &Bundle, mood: Option<&str>) -> Result<Played> {
        match self.get(bundle, mood) {
            Some(p) => Ok(p.clone()),
            None => bundle.play(self.persona.as_deref(), mood),
        }
    }
}

impl Bundle {
    /// the key a mood plays under: itself when the rig names it, else the defaults'
    pub fn mood_key<'m>(&self, mood: Option<&'m str>) -> &'m str {
        match mood {
            Some(m) if self.rig.moods.contains_key(m) => m,
            _ => DEFAULTS,
        }
    }

    /// The scope: every mood reachable from `current` with its distance (keys, not yet deduplicated).
    pub fn mood_reach(&self, ladder: &[&str], current: Option<&str>) -> Vec<Reach<String>> {
        let at = current.and_then(|c| ladder.iter().position(|m| *m == c));
        let mut out = vec![Reach::new(self.mood_key(current).to_string(), Dist::default(), 0)];
        for (i, m) in ladder.iter().enumerate() {
            // off the ladder now: the whole ladder is one branch away, by rungs from its calm end
            let dist = match at {
                Some(a) => Dist::rows(a.abs_diff(i) as u32),
                None => Dist { rows: i as u32 + 1, secs: 0.0, depth: 1 },
            };
            out.push(Reach::new(self.mood_key(Some(m)).to_string(), dist, 0));
        }
        for m in self.rig.moods.keys().filter(|m| !ladder.contains(&m.as_str())) {
            out.push(Reach::new(m.clone(), Dist { rows: 1, secs: 0.0, depth: 1 }, 0));
        }
        out
    }

    /// Walk the mood horizon from `current` within `bound` and play each reachable mood ahead.
    pub fn prebuild(&self, persona: Option<&str>, ladder: &[&str], current: Option<&str>, bound: &Bound) -> Result<Prebuilt> {
        let h = horizon::walk(self.mood_reach(ladder, current), bound);
        let mut built = BTreeMap::new();
        let mut order = Vec::new();
        for key in h.refs() {
            let mood = (key != DEFAULTS).then_some(key.as_str());
            built.insert(key.clone(), self.play(persona, mood)?);
            order.push(key.clone());
        }
        Ok(Prebuilt { persona: persona.map(str::to_string), built, order })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the energy ladder's moods, calm → wild
    const LADDER: [&str; 5] = crate::mood::MOODS;

    fn catpope() -> Bundle {
        Bundle::core("character.catpope").unwrap().unwrap()
    }

    fn near(rows: u32) -> Bound {
        Bound { rows: Some(rows), depth: Some(0), ..Bound::all() }
    }

    #[test]
    fn the_whole_horizon_nearest_first_one_build_per_result() {
        let b = catpope();
        let p = b.prebuild(None, &LADDER, Some("neutral"), &Bound::all()).unwrap();
        // neutral is not in catpope's rig: it plays the defaults; sad is off the ladder, one branch away
        assert_eq!(p.keys(), ["", "thinking", "sad", "happy", "laughing", "surprised"]);
    }

    #[test]
    fn a_prebuilt_mood_equals_a_fresh_play() {
        let b = catpope();
        let p = b.prebuild(None, &LADDER, Some("neutral"), &Bound::all()).unwrap();
        for mood in [None, Some("neutral"), Some("thinking"), Some("sad"), Some("happy"), Some("laughing"), Some("surprised"), Some("dizzy")] {
            let fresh = b.play(None, mood).unwrap();
            assert_eq!(p.get(&b, mood), Some(&fresh), "{mood:?}");
        }
        // the swap really differs between moods (it is not one scene for all)
        assert_ne!(p.get(&b, Some("happy")), p.get(&b, Some("sad")));
    }

    #[test]
    fn the_walk_is_bounded() {
        let b = catpope();
        assert_eq!(b.prebuild(None, &LADDER, Some("neutral"), &near(1)).unwrap().keys(), ["", "thinking"]);
        // from the middle of the ladder: both neighbours, ties in ladder order; sad (a branch) is past depth 0
        let p = b.prebuild(None, &LADDER, Some("happy"), &near(1)).unwrap();
        assert_eq!(p.keys(), ["happy", "thinking", "laughing"]);
        assert_eq!(p.get(&b, Some("surprised")), None, "two rungs away: outside the horizon");
        assert_eq!(p.played(&b, Some("surprised")).unwrap(), b.play(None, Some("surprised")).unwrap(), "still plays, built on the trigger");
        let p = b.prebuild(None, &LADDER, Some("happy"), &Bound { items: Some(2), ..Bound::all() }).unwrap();
        assert_eq!(p.keys(), ["happy", "thinking"]);
    }

    #[test]
    fn off_the_ladder_the_ladder_is_one_branch_away() {
        let b = catpope();
        let p = b.prebuild(None, &LADDER, Some("sad"), &Bound { depth: Some(0), ..Bound::all() }).unwrap();
        assert_eq!(p.keys(), ["sad"], "only where it is");
        let rows = |n| b.prebuild(None, &LADDER, Some("sad"), &Bound { rows: Some(n), ..Bound::all() }).unwrap();
        assert_eq!(rows(1).keys(), ["sad", ""], "the ladder's calm end (neutral = the defaults) first");
        assert_eq!(rows(2).keys(), ["sad", "", "thinking"], "then up the ladder");
    }

    #[test]
    fn a_persona_is_built_in() {
        let mut b = catpope();
        let mut v = super::super::Variant::default();
        v.presets.insert("voice".into(), "cathedral_voice.low".into());
        b.rig.personas.insert("biker".into(), v);
        let p = b.prebuild(Some("biker"), &LADDER, Some("neutral"), &near(0)).unwrap();
        assert_eq!(p.get(&b, None), Some(&b.play(Some("biker"), None).unwrap()));
        assert_ne!(p.get(&b, None), Some(&b.play(None, None).unwrap()));
    }
}
