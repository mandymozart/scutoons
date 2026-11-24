//! The Library the server answers (`GET /api/library`, server/docs/API.md "The extended Library"; the types are generated:
//! protocol::library): its entries as the browser's, the places a member asks and the query of each, a thing stepped
//! into. The server decides what an entry is, its name, its picture and what a place holds; this only reads.
use super::{Entry, Kind, Preview};
use protocol::library::{FileKind, LibraryEntry, LibraryLevel, LibraryLink, LibraryList, LibraryStep, ThingType};

impl Kind {
    pub fn of_thing(t: ThingType) -> Kind {
        match t {
            ThingType::Toon => Kind::Toon,
            ThingType::Character => Kind::Character,
            ThingType::Scene => Kind::Scene,
            ThingType::Show => Kind::Show,
            ThingType::Place => Kind::Place,
            ThingType::Item => Kind::Item,
            ThingType::Module => Kind::Module,
            ThingType::Instrument => Kind::Instrument,
            ThingType::Rig => Kind::Rig,
            ThingType::Generator => Kind::Generator,
            ThingType::Kanon => Kind::Kanon,
            ThingType::Person => Kind::Person,
        }
    }

    pub fn of_file(k: FileKind) -> Kind {
        match k {
            FileKind::Image => Kind::Image,
            FileKind::Audio => Kind::Audio,
            FileKind::Video => Kind::Video,
            FileKind::Model => Kind::Model,
            FileKind::Animation => Kind::Animation,
            FileKind::File => Kind::File,
        }
    }
}

/// One entry of the answer as the browser's: its picture as the server gave it, else the stage's still of the thing
/// (taken when the deck plays it), else its kind; the line under it names an item's subtype and how often it is used.
pub fn entry(e: &LibraryEntry) -> Entry {
    let kind = match (e.r#type, e.kind) {
        (Some(t), _) => Kind::of_thing(t),
        (None, Some(k)) => Kind::of_file(k),
        (None, None) => Kind::File,
    };
    let preview = match &e.picture {
        Some(url) => Preview::Picture { url: url.clone() },
        None if !kind.is_file() => Preview::Still { key: e.address.clone() },
        None => Preview::Kind,
    };
    let used = (e.used_in > 0).then(|| format!("used in {}", e.used_in));
    let detail = [e.subtype.map(|s| s.as_str().replace('-', " ")), used].into_iter().flatten().collect::<Vec<_>>().join(" · ");
    Entry { address: e.address.clone(), label: e.name.clone(), detail, kind, preview, made_for: Vec::new() }
}

/// the entries of a list's pages, in the server's order
pub fn entries(pages: &[LibraryList]) -> Vec<Entry> {
    pages.iter().flat_map(|p| p.entries.iter().map(entry)).collect()
}

/// a page of `GET /api/library`
pub fn list(text: &str) -> Result<LibraryList, String> {
    serde_json::from_str(text).map_err(|e| format!("the Library's answer: {e}"))
}

/// `GET /api/library?at=`
pub fn step(text: &str) -> Result<LibraryStep, String> {
    serde_json::from_str(text).map_err(|e| format!("the Library's answer: {e}"))
}

/// a place of the Library (LIBRARY.md "places"): what the open item uses, the toon, everything, the unused files, what
/// was added lately
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Place {
    /// what the open item uses (`scene:nave`, `character:catpope`)
    Item(String),
    Toon,
    All,
    Unused,
    Recent,
}

impl Place {
    /// how a chip names it
    pub fn word(&self) -> &'static str {
        match self {
            Place::Item(_) => "this item",
            Place::Toon => "this toon",
            Place::All => "everything",
            Place::Unused => "unused",
            Place::Recent => "recently added",
        }
    }

    /// the places that can be asked with this open item and toon, in the chips' order
    pub fn offered(open: Option<&str>, toon: Option<&str>) -> Vec<Place> {
        let mut out: Vec<Place> = open.filter(|o| !o.starts_with("toon:")).map(|o| Place::Item(o.to_string())).into_iter().collect();
        if toon.is_some() {
            out.push(Place::Toon);
        }
        out.extend([Place::All, Place::Unused, Place::Recent]);
        out
    }

    /// where a member starts: what the open item uses, else the toon, else everything
    pub fn start(open: Option<&str>, toon: Option<&str>) -> Place {
        Place::offered(open, toon).swap_remove(0)
    }

    /// the query of its first page (`?` not included): the toon it is asked in, the place, the locale, one large page
    pub fn query(&self, toon: Option<&str>, locale: &str) -> String {
        let place = match self {
            Place::Item(open) => format!("item:{open}"),
            Place::Toon => "toon".into(),
            Place::All => "all".into(),
            Place::Unused => "unused".into(),
            Place::Recent => "recent".into(),
        };
        let toon = toon.filter(|_| *self != Place::All).map(|t| format!("toon={}&", encode(t))).unwrap_or_default();
        format!("{toon}place={}&locale={}&limit={PAGE}", encode(&place), encode(locale))
    }
}

/// entries asked per page (the server's most)
pub const PAGE: u32 = 5000;

/// the query of the page after `list` (none: it was the last)
pub fn next_query(first: &str, list: &LibraryList) -> Option<String> {
    list.next.as_deref().map(|c| format!("{first}&cursor={}", encode(c)))
}

/// the query of a thing stepped into
pub fn step_query(address: &str, locale: &str) -> String {
    format!("at={}&locale={}", encode(address), encode(locale))
}

/// a query value: what is not unreserved is percent-encoded
pub fn encode(v: &str) -> String {
    v.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b':' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// A thing stepped into, as the browser draws it: the thing, its levels (each with how deep it hangs), the things and
/// the files it uses as entries, what uses it.
#[derive(Debug, Clone, PartialEq)]
pub struct Inside {
    pub at: Entry,
    pub levels: Vec<(usize, LibraryLevel)>,
    pub uses: Vec<Entry>,
    pub used_in: Vec<LibraryLink>,
}

impl Inside {
    pub fn of(s: &LibraryStep) -> Inside {
        let depth = |l: &LibraryLevel| {
            let mut n = 0;
            let mut up = l.parent.as_str();
            while let Some(p) = s.levels.iter().find(|x| x.address == up && x.mood.is_none() && x.address != s.at.address) {
                n += 1;
                if n > s.levels.len() {
                    break;
                }
                up = p.parent.as_str();
            }
            n
        };
        Inside {
            at: entry(&s.at),
            levels: s.levels.iter().map(|l| (depth(l), l.clone())).collect(),
            uses: s.uses.iter().chain(&s.files).map(entry).collect(),
            used_in: s.used_in.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn listed(v: serde_json::Value) -> LibraryList {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn the_answer_becomes_entries_with_the_servers_kind_name_and_picture() {
        let page = listed(json!({ "library": 1, "version": "v", "toon": "catpope", "place": "toon", "locale": "en", "total": 4, "next": null, "counts": {}, "entries": [
            { "address": "character:catpope", "type": "character", "name": "Cat Pope", "picture": "https://cdn/x.webp", "toons": ["catpope"], "usedIn": 26, "updated": null },
            { "address": "recipe:tea", "type": "item", "subtype": "home-scene", "name": "Tea", "picture": null, "toons": [], "usedIn": 0, "updated": null },
            { "address": "show:duet", "type": "show", "name": "Duet", "picture": null, "toons": [], "usedIn": 0, "updated": null },
            { "address": "video:cat/a.mp4", "kind": "video", "name": "a", "picture": "https://cdn/a.thumb.webp", "toons": [], "usedIn": 1, "updated": null, "origin": "imported" },
            { "address": "audio:cat/b.ogg", "kind": "audio", "name": "b", "picture": null, "toons": [], "usedIn": 0, "updated": null, "origin": "uploaded" }
        ] }));
        let e = entries(&[page]);
        assert_eq!(e.iter().map(|x| x.kind).collect::<Vec<_>>(), [Kind::Character, Kind::Item, Kind::Show, Kind::Video, Kind::Audio]);
        assert_eq!((e[0].label.as_str(), e[0].detail.as_str(), &e[0].preview), ("Cat Pope", "used in 26", &Preview::Picture { url: "https://cdn/x.webp".into() }));
        assert_eq!(e[1].detail, "home scene");
        assert_eq!(e[2].preview, Preview::Still { key: "show:duet".into() }, "a thing without a picture: the stage's still");
        assert_eq!(e[3].preview, Preview::Picture { url: "https://cdn/a.thumb.webp".into() }, "a video's frame is the server's still");
        assert_eq!((e[4].preview.clone(), e[4].path()), (Preview::Kind, "cat/b.ogg"));
    }

    #[test]
    fn every_type_and_kind_of_the_server_is_one_of_ours_with_its_word() {
        for t in ThingType::ALL {
            assert_eq!(Kind::of_thing(t).id(), t.as_str());
        }
        for k in FileKind::ALL {
            assert_eq!(Kind::of_file(k).id(), k.as_str());
        }
        assert_eq!(Kind::ALL.len(), ThingType::ALL.len() + FileKind::ALL.len());
    }

    #[test]
    fn the_places_and_their_queries() {
        assert_eq!(Place::offered(Some("scene:nave"), Some("catpope")), [Place::Item("scene:nave".into()), Place::Toon, Place::All, Place::Unused, Place::Recent]);
        assert_eq!(Place::start(None, Some("catpope")), Place::Toon);
        assert_eq!(Place::start(Some("toon:catpope"), None), Place::All);
        assert_eq!(Place::Item("scene:nave".into()).query(Some("catpope"), "en"), "toon=catpope&place=item:scene:nave&locale=en&limit=5000");
        assert_eq!(Place::All.query(Some("catpope"), "de"), "place=all&locale=de&limit=5000", "everything is every toon's");
        assert_eq!(step_query("character:cat pope", "en"), "at=character:cat%20pope&locale=en");
    }

    #[test]
    fn a_thing_stepped_into_hangs_its_levels_under_their_parents() {
        let s: LibraryStep = serde_json::from_value(json!({ "library": 1, "version": "v", "locale": "en",
            "at": { "address": "character:catpope", "type": "character", "name": "Cat Pope", "picture": null, "toons": [], "usedIn": 1, "updated": null },
            "levels": [
                { "address": "character:catpope.human", "level": "persona", "name": "human", "picture": null, "parent": "character:catpope" },
                { "address": "character:catpope.human.helmet", "level": "look", "name": "helmet", "picture": null, "parent": "character:catpope.human" },
                { "address": "character:catpope.human.helmet", "level": "mood", "name": "happy", "picture": null, "parent": "character:catpope.human.helmet", "mood": "happy" }
            ],
            "files": [{ "address": "image:c/a.webp", "kind": "image", "name": "a", "picture": null, "toons": [], "usedIn": 1, "updated": null }],
            "uses": [{ "address": "module:walk", "type": "module", "name": "walk", "picture": null, "toons": [], "usedIn": 1, "updated": null }],
            "usedIn": [{ "address": "season:catpope:1", "name": "Season 1", "as": "cast" }] }))
        .unwrap();
        let i = Inside::of(&s);
        assert_eq!(i.levels.iter().map(|(d, _)| *d).collect::<Vec<_>>(), [0, 1, 2]);
        assert_eq!(i.uses.iter().map(|e| e.address.as_str()).collect::<Vec<_>>(), ["module:walk", "image:c/a.webp"], "things, then files");
        assert_eq!(i.used_in[0].r#as.as_str(), "cast");
    }
}
