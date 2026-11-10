//! The address book (#477, decisions/deck.md [contract] [typeahead]): one toon's addressable catalog things as the server
//! serves them (`GET /api/addresses/<toon>`), typed by `protocol::address_book` (generated from
//! server/schema/api/address-book.schema.json, the one schema the creator and PHP read too). The deck editor's typeahead
//! holds and narrows it here; the platform only fetches and keeps the file. Whether an address is valid stays the parser's
//! (address.rs) and the validation's.
//!
//! **The match** is plain substring matching: every typed word, whatever the case, somewhere in the entry's label (the asked
//! locale, the one generated locale order, `protocol::content_text`), its name or its address; the book's order (parents
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
    /// address → entry
    index: HashMap<String, usize>,
}

/// One row of a typeahead.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub address: String,
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
        let index = book.entries.iter().enumerate().map(|(i, e)| (e.address.clone(), i)).collect();
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

    /// The entries an asker may reach (of `kinds`) that the typed `query` finds (the module's rule), in the book's order.
    pub fn find(&self, query: &str, kinds: &[AddressKind], locale: &str) -> Vec<usize> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        (0..self.book.entries.len())
            .filter(|&i| {
                let e = &self.book.entries[i];
                if !kinds.contains(&e.kind) {
                    return false;
                }
                let hay = format!("{} {} {}", self.label(i, locale), e.name, e.address).to_lowercase();
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

    /// a book as the server writes it (server/tests: AddressBook), cut down
    const BOOK: &str = r#"{
        "address_book": 1, "toon": "catpope", "version": "v1", "source": "de",
        "entries": [
            { "address": "character:catpope", "kind": "character", "name": "catpope", "labels": { "de": "Katzenpapst", "en": "Cat Pope" }, "origin": { "key": "character:catpope", "state": "published" } },
            { "address": "character:catpope.human", "kind": "persona", "name": "human", "parent": "character:catpope", "labels": { "de": "Mensch", "en": "Human" }, "origin": { "key": "character:catpope", "state": "published" } },
            { "address": "character:catpope.human.default", "kind": "look", "name": "default", "parent": "character:catpope.human", "labels": {}, "origin": { "key": "character:catpope", "state": "published" }, "moods": ["happy"] },
            { "address": "character:mountaintiger", "kind": "character", "name": "mountaintiger", "labels": { "fr": "Tigre", "de": " " }, "origin": { "key": "character:mountaintiger", "state": "draft" } },
            { "address": "location:dom", "kind": "location", "name": "dom", "labels": { "de": "Der Dom" }, "origin": { "key": "place:dom", "state": "published" } },
            { "address": "item:incense", "kind": "item", "name": "incense", "labels": { "en": "Incense" }, "origin": { "key": "item:incense", "state": "published" } },
            { "address": "chapter:3", "kind": "chapter", "name": "3", "labels": { "en": "Anger" }, "origin": { "key": "chapter:catpope:3", "state": "draft" } }
        ]
    }"#;

    fn book() -> Book {
        Book::parse(BOOK).unwrap()
    }

    fn found(b: &Book, query: &str, kinds: &[AddressKind], locale: &str) -> Vec<String> {
        b.find(query, kinds, locale).into_iter().map(|i| b.entries()[i].address.clone()).collect()
    }

    const ALL: [AddressKind; 8] = [Character, Persona, Look, Location, Item, Chapter, Group, Variable];

    #[test]
    fn every_word_in_the_label_the_name_or_the_address_whatever_the_case() {
        let b = book();
        assert_eq!(found(&b, "", &ALL, "en").len(), 7, "nothing typed: everything the asker may reach");
        assert_eq!(found(&b, "CAT pope", &[Character], "en"), ["character:catpope"], "both words, any case");
        assert_eq!(found(&b, "pope cat", &ALL, "en").len(), 3, "in any order; the persona and the look by their address");
        assert_eq!(found(&b, "katzen", &ALL, "de"), ["character:catpope"], "the label of the asked locale");
        assert!(found(&b, "katzen", &ALL, "en").is_empty(), "not the label of another locale");
        assert_eq!(found(&b, "human", &ALL, "de"), ["character:catpope.human", "character:catpope.human.default"], "the name, the address");
        assert_eq!(found(&b, "dom location:", &ALL, "en"), ["location:dom"], "words from label and address together");
        assert_eq!(found(&b, "  incense   item ", &ALL, "en"), ["item:incense"], "any spaces between the words");
        assert!(found(&b, "incensex", &ALL, "en").is_empty());
    }

    #[test]
    fn only_the_kinds_the_asker_may_reach_in_the_books_order() {
        let b = book();
        assert_eq!(found(&b, "", &[Character, Location, Item], "en"), ["character:catpope", "character:mountaintiger", "location:dom", "item:incense"]);
        assert_eq!(found(&b, "cat", &[Persona], "en"), ["character:catpope.human"]);
        assert!(found(&b, "", &[], "en").is_empty());
    }

    #[test]
    fn a_row_is_the_label_chain_the_kind_and_draft_or_published() {
        let b = book();
        let at = |a: &str| b.entries().iter().position(|e| e.address == a).unwrap();
        // the look has no label: its name; the chain in the asked locale
        assert_eq!(b.row(at("character:catpope.human.default"), "de").unwrap().chain, "Katzenpapst › Mensch › default");
        assert_eq!(b.row(at("character:catpope.human.default"), "en").unwrap().chain, "Cat Pope › Human › default");
        // the locale order: asked › source (de, blank here) › en › any
        assert_eq!(b.label(at("character:mountaintiger"), "en"), "Tigre");
        assert_eq!(b.label(at("location:dom"), "en"), "Der Dom", "the source language before en");
        let tiger = b.row(at("character:mountaintiger"), "en").unwrap();
        assert_eq!((tiger.kind, tiger.draft), (Character, true));
        assert!(!b.row(at("item:incense"), "en").unwrap().draft);
        assert_eq!((b.toon(), b.version()), ("catpope", "v1"));
    }

    #[test]
    fn a_kept_file_reads_back_and_a_broken_one_says_so() {
        let b = book();
        let kept = serde_json::to_string(&b.book).unwrap();
        let again = Book::parse(&kept).unwrap();
        assert_eq!((again.version(), again.entries().len()), ("v1", 7));
        assert!(Book::parse("{}").unwrap_err().starts_with("not an address book"));
        assert!(Book::parse(r#"{"address_book":1,"toon":"x","version":"v","source":"en","entries":[{"address":"a:b","kind":"scene","name":"b","labels":{},"origin":{"key":"k","state":"published"}}]}"#).is_err(), "a kind the schema does not list");
    }
}
