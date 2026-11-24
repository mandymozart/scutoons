//! The library's entries from what deck-space holds: the address book's characters, items, places and scenes, and the
//! instruments with their wiring (deck-catalog `library` has the entries' rules).
use crate::rig::book::Book;
use crate::rig::Instrument;
use deck_catalog::library::{graph, Entry, Kind, Preview};
use deck_catalog::Domain;
use protocol::address_book::AddressKind;

pub mod served;

/// the book's characters (their top level), items, places and scenes; a character's and a scene's preview is the
/// stage's still of it
pub fn from_book(book: &Book, locale: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    for (i, e) in book.entries().iter().enumerate() {
        let kind = match e.kind {
            AddressKind::Character if e.parent.is_none() => Kind::Character,
            AddressKind::Item => Kind::Item,
            AddressKind::Location => Kind::Place,
            AddressKind::Scene => Kind::Scene,
            _ => continue,
        };
        let address = e.address.clone().unwrap_or_else(|| e.r#ref.clone());
        let preview = match kind {
            Kind::Character | Kind::Scene => Preview::Still { key: e.r#ref.clone() },
            _ => Preview::Kind,
        };
        let mut detail = match kind {
            Kind::Scene => e.set.clone().unwrap_or_default(),
            _ => e.r#ref.clone(),
        };
        if let Some(toon) = book.toon_of(i) {
            detail = if detail.is_empty() { toon.to_string() } else { format!("{detail} · {toon}") };
        }
        out.push(Entry { address, label: book.label(i, locale).to_string(), detail, kind, preview, made_for: Vec::new() });
    }
    out
}

/// instruments by their wiring: a node per graph node, a tube per link
pub fn from_instruments(instruments: &[Instrument]) -> Vec<Entry> {
    instruments
        .iter()
        .map(|i| {
            let nodes: Vec<(String, bool)> = i.nodes.iter().map(|n| (n.id.clone(), deck_catalog::builtin(&n.kind).is_some_and(|d| d.domain == Domain::Audio))).collect();
            let links: Vec<(String, String)> = i.links.iter().map(|l| (l.from.clone(), l.to.clone())).collect();
            let detail = format!("{} · {} nodes", format!("{:?}", i.kind).to_lowercase(), i.nodes.len());
            Entry { address: format!("instrument:{}", i.id), label: i.id.clone(), detail, kind: Kind::Instrument, preview: graph(&nodes, &links), made_for: Vec::new() }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::Bundle;

    const BOOK: &str = r#"{ "address_book": 2, "toon": "catpope", "version": "v1", "source": "en", "entries": [
        { "ref": "character:catpope", "address": "character:catpope", "kind": "character", "name": "catpope", "labels": { "en": "Cat Pope" }, "origin": { "key": "character:catpope", "state": "published" } },
        { "ref": "character:catpope.human", "address": "character:catpope.human", "kind": "persona", "name": "human", "parent": "character:catpope", "labels": { "en": "Human" }, "origin": { "key": "character:catpope", "state": "published" } },
        { "ref": "location:dom", "kind": "location", "name": "dom", "labels": { "en": "Dom" }, "origin": { "key": "location:dom", "state": "published" } },
        { "ref": "scene:nave", "address": "scene:nave", "kind": "scene", "name": "nave", "set": "cathedral", "labels": { "en": "Nave" }, "origin": { "key": "scene:nave", "state": "published" }, "version": 2 }
    ] }"#;

    #[test]
    fn the_books_characters_items_places_and_scenes_are_entries() {
        let book = Book::parse(BOOK).unwrap();
        let list = from_book(&book, "en");
        let got: Vec<(&str, Kind, &str)> = list.iter().map(|e| (e.address.as_str(), e.kind, e.label.as_str())).collect();
        assert_eq!(got, [("character:catpope", Kind::Character, "Cat Pope"), ("location:dom", Kind::Place, "Dom"), ("scene:nave", Kind::Scene, "Nave")], "a persona is not one more character");
        assert_eq!(list[0].preview, Preview::Still { key: "character:catpope".into() });
        assert_eq!(list[2].detail, "cathedral");
    }

    #[test]
    fn an_instrument_is_previewed_by_its_wiring() {
        let b: Bundle = toml::from_str(include_str!("../../demos/rigs/keys.toml")).unwrap();
        let list = from_instruments(&b.instruments);
        assert!(!list.is_empty());
        for (e, i) in list.iter().zip(&b.instruments) {
            let Preview::Graph { dots, tubes } = &e.preview else { panic!("{e:?}") };
            assert_eq!(dots.len(), i.nodes.len());
            assert!(tubes.len() <= i.links.len());
        }
    }
}
