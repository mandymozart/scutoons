//! The address book (#477, decisions/deck.md [contract] [typeahead]): one toon's addressable catalog things as the server
//! serves them (`GET /api/addresses/<toon>`), typed by `protocol::address_book` (generated from
//! server/schema/api/address-book.schema.json, the one schema the creator and PHP read too). The deck editor's typeahead
//! holds and narrows it here; the platform only fetches and keeps the file. Whether an address is valid stays the parser's
//! (address.rs) and the validation's.
//!
//! **Reference and address** (owner 2026-11-10 [address]): every entry has a reference (`ref`, which thing); only an entry a
//! receiver exists for has an address (who hears a word). The deck sends words, so `find` offers only entries with an address,
//! and a pick writes that address. A location, chapter, group or variable is a reference only and never offered here.
//!
//! **The match** is plain substring matching: every typed word, whatever the case, somewhere in the entry's label (the asked
//! locale, the one generated locale order, `protocol::content_text`), its name or its reference; the book's order (parents
//! before their children). It is the one rule of the book's typeahead and exists once per language that draws one: here for
//! the deck, `matchEntries` in creator/src/lib/editor/address-targets.ts for the creator (#477: the server serves the data,
//! each client filters it; no wasm route, [contract]). The tests below are the cases both must give.
use protocol::address_book::{AddressBook, AddressEntry, AddressKind, OriginState};
use protocol::content_text::pick_text;
use std::collections::{BTreeMap, HashMap};

/// between the labels of a chain: "Cat Pope › Human › Default"
pub const CHAIN: &str = " › ";

/// A parsed book.
#[derive(Debug, Clone)]
pub struct Book {
    book: AddressBook,
    /// each entry's labels in the map's own (sorted) order: the locale order's last step, `any`, takes the first
    labels: Vec<BTreeMap<String, String>>,
    /// reference → entry
    index: HashMap<String, usize>,
}

/// One row of a typeahead.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// what a pick writes; none for an entry nothing can receive a word at (`find` never offers those)
    pub address: Option<String>,
    /// the entry's label with the ones above it
    pub chain: String,
    pub kind: AddressKind,
    /// a newer draft exists, or nothing is published yet
    pub draft: bool,
}

impl Book {
    /// the document as the server sends it, or as a deck kept it
    pub fn parse(text: &str) -> Result<Book, String> {
        serde_json::from_str::<AddressBook>(text).map(Book::new).map_err(|e| format!("not an address book: {e}"))
    }

    pub fn new(book: AddressBook) -> Book {
        let labels = book.entries.iter().map(|e| e.labels.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).collect();
        let index = book.entries.iter().enumerate().map(|(i, e)| (e.r#ref.clone(), i)).collect();
        Book { book, labels, index }
    }

    pub fn toon(&self) -> &str {
        &self.book.toon
    }

    /// the catalog's version it was built from (the server's ETag)
    pub fn version(&self) -> &str {
        &self.book.version
    }

    pub fn entries(&self) -> &[AddressEntry] {
        &self.book.entries
    }

    pub fn entry(&self, i: usize) -> Option<&AddressEntry> {
        self.book.entries.get(i)
    }

    /// what entry `i` is called in `locale` (the one locale order: asked › the toon's source language › en › any); its name
    /// when it has no label
    pub fn label(&self, i: usize, locale: &str) -> &str {
        let Some(e) = self.book.entries.get(i) else { return "" };
        pick_text(&self.labels[i], locale, Some(&self.book.source)).unwrap_or(&e.name)
    }

    /// the entry's label with the ones above it, the top first
    pub fn chain(&self, i: usize, locale: &str) -> String {
        let mut chain = vec![i];
        let mut up = self.book.entries.get(i).and_then(|e| e.parent.as_ref());
        while let Some(&p) = up.and_then(|a| self.index.get(a)) {
            if chain.contains(&p) {
                break;
            }
            chain.push(p);
            up = self.book.entries[p].parent.as_ref();
        }
        chain.iter().rev().map(|&e| self.label(e, locale)).collect::<Vec<_>>().join(CHAIN)
    }

    /// The entries an asker may reach – with an address (a receiver exists) and of `kinds` – that the typed `query` finds
    /// (the module's rule), in the book's order.
    pub fn find(&self, query: &str, kinds: &[AddressKind], locale: &str) -> Vec<usize> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        (0..self.book.entries.len())
            .filter(|&i| {
                let e = &self.book.entries[i];
                if e.address.is_none() || !kinds.contains(&e.kind) {
                    return false;
                }
                let hay = format!("{} {} {}", self.label(i, locale), e.name, e.r#ref).to_lowercase();
                words.iter().all(|w| hay.contains(w.as_str()))
            })
            .collect()
    }

    /// what a typeahead shows of entry `i`
    pub fn row(&self, i: usize, locale: &str) -> Option<Row> {
        let e = self.book.entries.get(i)?;
        Some(Row { address: e.address.clone(), chain: self.chain(i, locale), kind: e.kind, draft: e.origin.state == OriginState::Draft })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AddressKind::*;

    /// a book as the server writes it (server/tests: AddressBook), cut down: an address only where a receiver exists
    const BOOK: &str = r#"{
        "address_book": 2, "toon": "catpope", "version": "v1", "source": "de",
        "entries": [
            { "ref": "character:catpope", "address": "character:catpope", "kind": "character", "name": "catpope", "labels": { "de": "Katzenpapst", "en": "Cat Pope" }, "origin": { "key": "character:catpope", "state": "published" } },
            { "ref": "character:catpope.human", "address": "character:catpope.human", "kind": "persona", "name": "human", "parent": "character:catpope", "labels": { "de": "Mensch", "en": "Human" }, "origin": { "key": "character:catpope", "state": "published" } },
            { "ref": "character:catpope.human.default", "address": "character:catpope.human.default", "kind": "look", "name": "default", "parent": "character:catpope.human", "labels": {}, "origin": { "key": "character:catpope", "state": "published" }, "moods": ["happy"] },
            { "ref": "character:mountaintiger", "address": "character:mountaintiger", "kind": "character", "name": "mountaintiger", "labels": { "fr": "Tigre", "de": " " }, "origin": { "key": "character:mountaintiger", "state": "draft" } },
            { "ref": "location:dom", "kind": "location", "name": "dom", "labels": { "de": "Der Dom" }, "origin": { "key": "place:dom", "state": "published" } },
            { "ref": "item:incense", "address": "item:incense", "kind": "item", "name": "incense", "labels": { "en": "Incense" }, "origin": { "key": "item:incense", "state": "published" } },
            { "ref": "chapter:3", "kind": "chapter", "name": "3", "labels": { "en": "Anger" }, "origin": { "key": "chapter:catpope:3", "state": "draft" } },
            { "ref": "chapter:3.intro", "kind": "group", "name": "intro", "parent": "chapter:3", "labels": {}, "origin": { "key": "chapter:catpope:3", "state": "draft" } },
            { "ref": "scene:nave", "address": "scene:nave", "kind": "scene", "name": "nave", "labels": { "de": "Kirchenschiff" }, "origin": { "key": "scene:nave", "state": "published" }, "set": "cathedral", "spot": "spot_altar", "version": 2 }
        ]
    }"#;

    fn book() -> Book {
        Book::parse(BOOK).unwrap()
    }

    /// what a pick of each found entry writes: its address
    fn found(b: &Book, query: &str, kinds: &[AddressKind], locale: &str) -> Vec<String> {
        b.find(query, kinds, locale).into_iter().filter_map(|i| b.entries()[i].address.clone()).collect()
    }

    const ALL: [AddressKind; 9] = [Character, Persona, Look, Location, Item, Chapter, Group, Variable, Scene];

    #[test]
    fn every_word_in_the_label_the_name_or_the_reference_whatever_the_case() {
        let b = book();
        assert_eq!(found(&b, "", &ALL, "en").len(), 6, "nothing typed: everything the asker may reach that has an address");
        assert_eq!(found(&b, "CAT pope", &[Character], "en"), ["character:catpope"], "both words, any case");
        assert_eq!(found(&b, "pope cat", &ALL, "en").len(), 3, "in any order; the persona and the look by their reference");
        assert_eq!(found(&b, "katzen", &ALL, "de"), ["character:catpope"], "the label of the asked locale");
        assert!(found(&b, "katzen", &ALL, "en").is_empty(), "not the label of another locale");
        assert_eq!(found(&b, "human", &ALL, "de"), ["character:catpope.human", "character:catpope.human.default"], "the name, the reference");
        assert_eq!(found(&b, "incense item:", &ALL, "en"), ["item:incense"], "words from label and reference together");
        assert_eq!(found(&b, "  incense   item ", &ALL, "en"), ["item:incense"], "any spaces between the words");
        assert!(found(&b, "incensex", &ALL, "en").is_empty());
    }

    /// owner 2026-11-10 [address]: sending a word somewhere needs a receiver of that type – a reference only is never offered
    #[test]
    fn a_reference_only_is_never_offered_whatever_the_asker_asks_for() {
        let b = book();
        assert!(found(&b, "dom", &[Location], "de").is_empty(), "a location has no receiver yet");
        assert!(b.find("", &[Chapter, Group, Variable], "en").is_empty(), "chapters, groups, variables are references");
        let dom = b.entries().iter().position(|e| e.r#ref == "location:dom").unwrap();
        assert_eq!(b.row(dom, "de").unwrap().address, None);
    }

    /// #487: a published deck scene is an address (a deck plays it), found like any entry by its label, name or reference;
    /// a far source's holder is never one
    #[test]
    fn a_scene_is_found_by_its_label_name_or_reference() {
        let b = book();
        assert_eq!(found(&b, "nav", &[Scene], "en"), ["scene:nave"]);
        assert_eq!(found(&b, "kirchen", &ALL, "de"), ["scene:nave"], "the stage's name, its label");
        assert_eq!(found(&b, "scene:", &ALL, "en"), ["scene:nave"]);
        assert!(found(&b, "nave", &crate::world::FAR_HOLDERS, "en").is_empty(), "a scene holds no far source");
        let at = b.entries().iter().position(|e| e.r#ref == "scene:nave").unwrap();
        let row = b.row(at, "en").unwrap();
        assert_eq!((row.kind, row.draft, row.chain.as_str(), row.address.as_deref()), (Scene, false, "Kirchenschiff", Some("scene:nave")));
        let e = &b.entries()[at];
        assert_eq!((e.set.as_deref(), e.spot.as_deref(), e.version), (Some("cathedral"), Some("spot_altar"), Some(2)));
    }

    #[test]
    fn only_the_kinds_the_asker_may_reach_in_the_books_order() {
        let b = book();
        assert_eq!(found(&b, "", &[Character, Location, Item], "en"), ["character:catpope", "character:mountaintiger", "item:incense"]);
        assert_eq!(found(&b, "cat", &[Persona], "en"), ["character:catpope.human"]);
        assert!(found(&b, "", &[], "en").is_empty());
    }

    #[test]
    fn a_row_is_the_label_chain_the_kind_and_draft_or_published() {
        let b = book();
        let at = |r: &str| b.entries().iter().position(|e| e.r#ref == r).unwrap();
        // the look has no label: its name; the chain in the asked locale, up the references
        assert_eq!(b.row(at("character:catpope.human.default"), "de").unwrap().chain, "Katzenpapst › Mensch › default");
        assert_eq!(b.row(at("character:catpope.human.default"), "en").unwrap().chain, "Cat Pope › Human › default");
        assert_eq!(b.chain(at("chapter:3.intro"), "en"), "Anger › intro");
        // the locale order: asked › source (de, blank here) › en › any
        assert_eq!(b.label(at("character:mountaintiger"), "en"), "Tigre");
        assert_eq!(b.label(at("location:dom"), "en"), "Der Dom", "the source language before en");
        let tiger = b.row(at("character:mountaintiger"), "en").unwrap();
        assert_eq!((tiger.kind, tiger.draft, tiger.address.as_deref()), (Character, true, Some("character:mountaintiger")));
        assert!(!b.row(at("item:incense"), "en").unwrap().draft);
        assert_eq!((b.toon(), b.version()), ("catpope", "v1"));
    }

    #[test]
    fn a_kept_file_reads_back_and_a_broken_one_says_so() {
        let b = book();
        let kept = serde_json::to_string(&b.book).unwrap();
        let again = Book::parse(&kept).unwrap();
        assert_eq!((again.version(), again.entries().len()), ("v1", 9));
        assert!(Book::parse("{}").unwrap_err().starts_with("not an address book"));
        assert!(Book::parse(r#"{"address_book":2,"toon":"x","version":"v","source":"en","entries":[{"ref":"a:b","kind":"set","name":"b","labels":{},"origin":{"key":"k","state":"published"}}]}"#).is_err(), "a kind the schema does not list");
        // a book kept in the first format (no reference) is not read: the editor fetches a new one
        assert!(Book::parse(r#"{"address_book":1,"toon":"x","version":"v","source":"en","entries":[{"address":"item:a","kind":"item","name":"a","labels":{},"origin":{"key":"k","state":"published"}}]}"#).is_err());
    }
}
