//! The server's Library answer (`GET /api/library`; the types are generated from server/schema/api/library.schema.json:
//! protocol::library) as the browser's entries and a thing stepped into (deck-catalog `library::served`). The server
//! decides what an entry is, its name and its picture; this only reads.
use deck_catalog::library::served::{depths, Inside, Level, Link};
use deck_catalog::library::{Entry, Kind, Preview};
use protocol::library::{FileKind, LibraryEntry, LibraryList, LibraryStep, ThingType};

pub fn kind_of_thing(t: ThingType) -> Kind {
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

pub fn kind_of_file(k: FileKind) -> Kind {
    match k {
        FileKind::Image => Kind::Image,
        FileKind::Audio => Kind::Audio,
        FileKind::Video => Kind::Video,
        FileKind::Model => Kind::Model,
        FileKind::Animation => Kind::Animation,
        FileKind::File => Kind::File,
    }
}

/// One entry of the answer as the browser's: its picture as the server gave it, else the stage's still of the thing
/// (taken when the deck plays it), else its kind; the line under it names an item's subtype and how often it is used.
pub fn entry(e: &LibraryEntry) -> Entry {
    let kind = match (e.r#type, e.kind) {
        (Some(t), _) => kind_of_thing(t),
        (None, Some(k)) => kind_of_file(k),
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

/// a page of `GET /api/library`
pub fn list(text: &str) -> Result<LibraryList, String> {
    serde_json::from_str(text).map_err(|e| format!("the Library's answer: {e}"))
}

/// the entries of a list's pages, in the server's order
pub fn entries(pages: &[LibraryList]) -> Vec<Entry> {
    pages.iter().flat_map(|p| p.entries.iter().map(entry)).collect()
}

/// `GET /api/library?at=` as the browser draws it: its levels hung under their parents, the things then the files it
/// uses, what uses it
pub fn inside(text: &str) -> Result<Inside, String> {
    let s: LibraryStep = serde_json::from_str(text).map_err(|e| format!("the Library's answer: {e}"))?;
    let shape: Vec<(String, String, bool)> = s.levels.iter().map(|l| (l.address.clone(), l.parent.clone(), l.mood.is_some())).collect();
    let levels = s
        .levels
        .iter()
        .zip(depths(&shape))
        .map(|(l, depth)| Level { depth, address: l.address.clone(), level: l.level.as_str().to_string(), name: l.name.clone(), parent: l.parent.clone(), mood: l.mood.clone() })
        .collect();
    let used_in = s.used_in.iter().map(|l| Link { address: l.address.clone(), thing: l.r#type.is_some(), name: l.name.clone(), role: l.r#as.as_str().to_string() }).collect();
    Ok(Inside { at: entry(&s.at), levels, uses: s.uses.iter().chain(&s.files).map(entry).collect(), used_in })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_answer_becomes_entries_with_the_servers_kind_name_and_picture() {
        let page = list(
            &json!({ "library": 1, "version": "v", "toon": "catpope", "place": "toon", "locale": "en", "total": 5, "next": null, "counts": {}, "entries": [
                { "address": "character:catpope", "type": "character", "name": "Cat Pope", "picture": "https://cdn/x.webp", "toons": ["catpope"], "usedIn": 26, "updated": null },
                { "address": "home-scene:tea", "type": "item", "subtype": "home-scene", "name": "Tea", "picture": null, "toons": [], "usedIn": 0, "updated": null },
                { "address": "show:duet", "type": "show", "name": "Duet", "picture": null, "toons": [], "usedIn": 0, "updated": null },
                { "address": "video:cat/a.mp4", "kind": "video", "name": "a", "picture": "https://cdn/a.thumb.webp", "toons": [], "usedIn": 1, "updated": null, "origin": "imported" },
                { "address": "audio:cat/b.ogg", "kind": "audio", "name": "b", "picture": null, "toons": [], "usedIn": 0, "updated": null, "origin": "uploaded" }
            ] })
            .to_string(),
        )
        .unwrap();
        let e = entries(&[page]);
        assert_eq!(e.iter().map(|x| x.kind).collect::<Vec<_>>(), [Kind::Character, Kind::Item, Kind::Show, Kind::Video, Kind::Audio]);
        assert_eq!((e[0].label.as_str(), e[0].detail.as_str(), &e[0].preview), ("Cat Pope", "used in 26", &Preview::Picture { url: "https://cdn/x.webp".into() }));
        assert_eq!(e[1].detail, "home scene");
        assert_eq!(e[2].preview, Preview::Still { key: "show:duet".into() }, "a thing without a picture: the stage's still");
        assert_eq!(e[3].preview, Preview::Picture { url: "https://cdn/a.thumb.webp".into() }, "a video's frame is the server's still");
        assert_eq!((e[4].preview.clone(), e[4].path()), (Preview::Kind, "cat/b.ogg"));
    }

    #[test]
    fn every_type_and_kind_of_the_server_is_one_of_ours() {
        for t in ThingType::ALL {
            assert_eq!(kind_of_thing(t).id(), t.as_str());
        }
        for k in FileKind::ALL {
            assert_eq!(kind_of_file(k).id(), k.as_str());
        }
        assert_eq!(Kind::ALL.len(), ThingType::ALL.len() + FileKind::ALL.len());
    }

    #[test]
    fn a_thing_stepped_into_hangs_its_levels_under_their_parents() {
        let i = inside(
            &json!({ "library": 1, "version": "v", "locale": "en",
                "at": { "address": "character:catpope", "type": "character", "name": "Cat Pope", "picture": null, "toons": [], "usedIn": 1, "updated": null },
                "levels": [
                    { "address": "character:catpope.human", "level": "persona", "name": "human", "picture": null, "parent": "character:catpope" },
                    { "address": "character:catpope.human.helmet", "level": "look", "name": "helmet", "picture": null, "parent": "character:catpope.human" },
                    { "address": "character:catpope.human.helmet", "level": "mood", "name": "happy", "picture": null, "parent": "character:catpope.human.helmet", "mood": "happy" }
                ],
                "files": [{ "address": "image:c/a.webp", "kind": "image", "name": "a", "picture": null, "toons": [], "usedIn": 1, "updated": null }],
                "uses": [{ "address": "module:walk", "type": "module", "name": "walk", "picture": null, "toons": [], "usedIn": 1, "updated": null }],
                "usedIn": [{ "address": "season:catpope:1", "name": "Season 1", "as": "cast" }] })
            .to_string(),
        )
        .unwrap();
        assert_eq!(i.levels.iter().map(|l| l.depth).collect::<Vec<_>>(), [0, 1, 2]);
        assert_eq!(i.uses.iter().map(|e| e.address.as_str()).collect::<Vec<_>>(), ["module:walk", "image:c/a.webp"], "things, then files");
        assert_eq!((i.used_in[0].role.as_str(), i.used_in[0].thing), ("cast", false));
    }
}
