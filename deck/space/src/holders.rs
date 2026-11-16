//! Who a piece is to a far world node (#568, research/SHOW.md §6, MODULE-WIRING.md §3): the places a piece plays – what
//! a far address's holder (`character:` / `item:`, world.rs `FAR_HOLDERS`) can name – and the far world nodes it listens
//! with. Read from the piece as a cell loads it (piece.rs `load`): a rig holds its subject, a scene the members of its cast
//! (their `look` nodes' `who`, or the being of a single member at the scene's own spot). What is wired to what across a
//! show's row is deck-show's (far.rs); the editor draws it.
use crate::piece::{load_played, Found};
use crate::rig::address::{Address, Place};
use crate::rig::{Bundle, Played};
use crate::world::{Source, World};

/// One far world node of a piece: its compiled node id (`far_in`, `<holder>_far_in` in a scene of several) and the
/// address it listens to.
#[derive(Debug, Clone, PartialEq)]
pub struct FarNode {
    pub node: String,
    pub address: Address,
}

impl FarNode {
    /// the holder its address names (`character:bells`); none: it names a deck or scene alone
    pub fn holder(&self) -> Option<&Place> {
        self.address.places.iter().find(|p| p.kind == "character" || p.kind == "item")
    }
}

/// What a piece is to far addresses: the places it plays and the far nodes it listens with.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reach {
    pub holds: Vec<Place>,
    pub far: Vec<FarNode>,
}

impl Reach {
    /// whether `place` plays in this piece
    pub fn holds(&self, place: &Place) -> bool {
        self.holds.contains(place)
    }
}

/// The reach of `played`, the piece at `place` (`character:keys`, `scene:nave`).
pub fn reach_of(place: &Place, played: &Played) -> Reach {
    let mut holds = Vec::new();
    let mut add = |p: Place| {
        if !holds.contains(&p) {
            holds.push(p);
        }
    };
    if matches!(place.kind.as_str(), "character" | "item") {
        add(place.clone());
    }
    if !played.character.is_empty() {
        add(Place::new("character", &played.character));
    }
    for n in played.nodes.iter().filter(|n| n.kind == "look") {
        if let Some(p) = n.params.get("who").and_then(|v| v.as_str()).and_then(|w| Place::parse(w).ok()) {
            add(p);
        }
    }
    let far = played
        .nodes
        .iter()
        .filter(|n| n.kind == crate::graph::Kind::World.name())
        .filter_map(|n| match World::read(&n.id, &n.params) {
            Ok(World { source: Source::Far(address), .. }) => Some(FarNode { node: n.id.clone(), address }),
            _ => None,
        })
        .collect();
    Reach { holds, far }
}

/// What a piece's cell inspector and the row's wires need of it, loaded once: its knobs and its reach.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Read {
    pub knobs: Vec<crate::rig::bundle::Knob>,
    pub reach: Reach,
}

/// `piece` loaded as a cell would load it (a core rig, else what `find` has), no mood: its knobs and its reach. Err = why not.
pub fn read(piece: &str, find: impl FnOnce(&Place) -> Option<Found>) -> Result<Read, String> {
    let place = Place::parse(piece).map_err(|e| format!("{piece}: {e}"))?;
    let played = load_played(&place, find)?;
    Ok(Read { reach: reach_of(&place, &played), knobs: played.knobs })
}

/// A rig a cell's piece opens in Build: who plays it (`character:keys`), its bundle, and the prefix its slots carry inside
/// the piece (`""`: the piece's slots are the rig's own – a rig, or a scene's single member at its own spot).
#[derive(Debug, Clone)]
pub struct Opened {
    pub place: String,
    pub bundle: Bundle,
    pub prefix: String,
}

/// The rig `piece` opens in Build when it is a rig, a character or an item (a core rig, else `find`'s rig or bundle). A
/// scene's members come from its cast (published.rs `Cast::opened`). Err = why not.
pub fn opened(piece: &str, find: impl FnOnce(&Place) -> Option<Found>) -> Result<Opened, String> {
    let place = Place::parse(piece).map_err(|e| format!("{piece}: {e}"))?;
    let core = matches!(place.kind.as_str(), "character" | "item").then(|| Bundle::core(&format!("{}.{}", place.kind, place.id))).flatten();
    let bundle = match (core, place.kind.as_str()) {
        (Some(b), _) => b.map_err(|e| format!("{piece}: {e:#}"))?,
        (None, "character" | "item" | "rig") => match find(&place) {
            Some(Found::Rig(text)) => toml::from_str::<Bundle>(&text).map_err(|e| format!("{piece}: {e}"))?,
            Some(Found::Bundle(b)) => *b,
            Some(Found::Missing(why)) => return Err(why),
            Some(Found::Scene(_)) | None => return Err(format!("{piece} is not on this deck")),
        },
        (None, "scene") => return Err(format!("{piece}: a scene opens its cast's rigs")),
        (None, kind) => return Err(format!("{piece}: a piece of the kind '{kind}' has no rig to open")),
    };
    let place = Place::of_subject(&bundle.rig.subject).map_or_else(|| place.to_string(), |p| p.to_string());
    Ok(Opened { place, bundle, prefix: String::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// two rigs that listen to each other (the far-nodes demo's, deck/demos/rigs)
    pub const KEYS: &str = include_str!("../../demos/rigs/keys.toml");
    pub const BELLS: &str = include_str!("../../demos/rigs/bells.toml");

    fn find(place: &Place) -> Option<Found> {
        match place.id.as_str() {
            "keys" => Some(Found::Rig(KEYS.into())),
            "bells" => Some(Found::Rig(BELLS.into())),
            _ => None,
        }
    }

    #[test]
    fn a_rig_holds_its_subject_and_lists_its_far_nodes() {
        let r = read("character:keys", find).unwrap();
        assert_eq!(r.reach.holds, [Place::new("character", "keys")]);
        assert_eq!(r.reach.far.len(), 1, "{:?}", r.reach.far);
        assert_eq!(r.reach.far[0].node, "far_in");
        assert_eq!(r.reach.far[0].holder(), Some(&Place::new("character", "bells")));
        assert!(!r.knobs.is_empty());
        // a core rig without far nodes holds its subject too
        let c = read("item:incense_burner", |_| None).unwrap();
        assert_eq!((c.reach.holds, c.reach.far), (vec![Place::new("item", "incense_burner")], vec![]));
        assert!(read("character:nobody", find).unwrap_err().contains("not on this deck"));
    }

    /// a scene holds its cast: the look nodes' `who`, and the being of a single member
    #[test]
    fn a_scene_holds_its_cast() {
        let text = "[being]\ncharacter = \"catpope\"\n\n[[node]]\nid = \"a_look\"\nkind = \"look\"\nholder = \"a\"\nwho = \"item:incense_burner\"\n\n[[node]]\nid = \"out\"\nkind = \"audio_out\"\n";
        let played = Played::scene(text).unwrap();
        let r = reach_of(&Place::new("scene", "nave"), &played);
        assert_eq!(r.holds, [Place::new("character", "catpope"), Place::new("item", "incense_burner")]);
    }

    #[test]
    fn a_rig_piece_opens_its_bundle_and_a_scene_does_not() {
        let o = opened("character:bells", find).unwrap();
        assert_eq!((o.place.as_str(), o.prefix.as_str(), o.bundle.rig.subject.as_str()), ("character:bells", "", "character.bells"));
        assert_eq!(opened("character:catpope", |_| None).unwrap().bundle.rig.subject, "character.catpope");
        assert!(opened("scene:nave", find).unwrap_err().contains("cast"));
    }
}
