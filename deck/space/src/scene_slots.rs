//! A scene's slots in an editor (research/SCENES.md §9.2–§9.3): a placeholder added, its filter (`fill`) asked as the
//! address book's question – its kinds, a hand-picked list, a parent – and filled with a preview the filter admits. The
//! question is drawn like the one condition control (deck-ui `filter::Condition`): words, then chips, every fact holds.
//! A change the core finds a problem in is refused with its words (`SceneEdit::change`).
use crate::scene_document::{Fill, FillerRef, Fillers, Holder, Take, HOLDER_KINDS, STAGING_SPOTS};
use crate::scene_edit::{Line, Part, SceneEdit};
use protocol::address_book::AddressKind;

/// One fact of a slot's filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FillFact {
    Kind(AddressKind),
    /// one of a hand-picked list
    Ref(String),
    /// only what is below this reference
    Parent(String),
}

impl FillFact {
    /// as its chip says it
    pub fn word(&self) -> String {
        match self {
            FillFact::Kind(k) => k.as_str().to_string(),
            FillFact::Ref(r) => r.clone(),
            FillFact::Parent(p) => format!("below {p}"),
        }
    }
}

/// A slot's filter as the control draws it: the words, the facts offered in the order drawn, which are on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillQuestion {
    pub words: String,
    pub facts: Vec<(FillFact, bool)>,
}

impl FillQuestion {
    /// the words on the chips, and which are on
    pub fn chips(&self) -> (Vec<String>, Vec<bool>) {
        (self.facts.iter().map(|(f, _)| f.word()).collect(), self.facts.iter().map(|(_, on)| *on).collect())
    }

    /// the chip of `word`, as a check run names it
    pub fn chip(&self, word: &str) -> Option<usize> {
        self.facts.iter().position(|(f, _)| f.word() == word)
    }
}

/// a filter in words: `character or item · one of character:catpope · below character:catpope`
pub fn fill_words(fill: &Fill) -> String {
    let mut parts = vec![fill.kind.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(" or ")];
    if !fill.refs.is_empty() {
        parts.push(format!("one of {}", fill.refs.join(", ")));
    }
    if let Some(p) = &fill.parent {
        parts.push(format!("below {p}"));
    }
    parts.join(" · ")
}

impl SceneEdit {
    /// the placeholder at holder `slot`
    fn slot(&self, slot: usize) -> Option<(&Holder, &Fill)> {
        let h = self.doc.holders.get(slot)?;
        Some((h, h.fill.as_ref()?))
    }

    /// the lines of the tree for the slots: their id, filter and preview
    pub(crate) fn slot_lines(&self) -> Vec<Line> {
        (0..self.doc.holders.len())
            .filter_map(|i| {
                let (h, _) = self.slot(i)?;
                let detail = h.preview.as_ref().map_or_else(|| "empty slot".to_string(), |p| p.list().iter().map(|r| r.split_once(':').map_or(*r, |(_, id)| id)).collect::<Vec<_>>().join(", "));
                Some(Line { depth: 1, label: h.id.clone(), detail, part: Part::Slot(i) })
            })
            .collect()
    }

    /// A slot taking one `kind`, at the first staging spot no holder takes: its holder's place. Err: the core's words.
    pub fn add_slot(&mut self, kind: AddressKind) -> Result<usize, String> {
        if !HOLDER_KINDS.contains(&kind) {
            return Err(format!("a slot on the stage takes a character, a persona, a look or an item, not a {}", kind.as_str()));
        }
        let taken = |d: &crate::scene_document::SceneDocument, id: &str| d.holders.iter().any(|h| h.id == id) || d.location_placeholder().is_some_and(|p| p.id == id);
        let id = (1..).map(|n| if n == 1 { "slot".to_string() } else { format!("slot_{n}") }).find(|id| !taken(&self.doc, id)).unwrap_or_default();
        let spot = STAGING_SPOTS.iter().find(|s| !self.doc.holders.iter().any(|h| h.spot.as_deref() == Some(**s))).map(|s| s.to_string());
        let fill = Fill { kind: vec![kind], refs: Vec::new(), parent: None };
        self.change(|d| {
            d.holders.push(Holder { id, spot, fill: Some(fill), ..Holder::default() });
            Ok(())
        })?;
        Ok(self.doc.holders.len() - 1)
    }

    /// Slot `slot`'s filter as the control asks it: the kinds a holder takes, then each reference `offered` (the book's
    /// characters and items) and any the filter names, as a hand-picked list, then each character offered as a parent.
    pub fn question(&self, slot: usize, offered: &[String]) -> Option<FillQuestion> {
        let (_, fill) = self.slot(slot)?;
        let mut facts: Vec<(FillFact, bool)> = HOLDER_KINDS.iter().map(|k| (FillFact::Kind(*k), fill.kind.contains(k))).collect();
        let mut refs: Vec<&String> = offered.iter().collect();
        refs.extend(fill.refs.iter().filter(|r| !offered.contains(r)));
        facts.extend(refs.iter().map(|r| (FillFact::Ref((*r).clone()), fill.refs.contains(r))));
        let character = |r: &String| FillerRef::parse(r).is_ok_and(|f| f.kind == AddressKind::Character);
        let mut parents: Vec<&String> = offered.iter().filter(|r| character(r)).collect();
        if let Some(p) = fill.parent.as_ref().filter(|p| !parents.contains(p)) {
            parents.push(p);
        }
        facts.extend(parents.into_iter().map(|p| (FillFact::Parent(p.clone()), fill.parent.as_ref() == Some(p))));
        Some(FillQuestion { words: fill_words(fill), facts })
    }

    /// One fact of slot `slot`'s filter on or off (a parent is one: another replaces it). Err: the core's words – a filter
    /// with no kind, a reference of a kind it does not take, a preview it would no longer admit.
    pub fn flip(&mut self, slot: usize, fact: &FillFact) -> Result<(), String> {
        self.slot(slot).ok_or("no such slot")?;
        self.change(|d| {
            let fill = d.holders[slot].fill.as_mut().expect("a slot");
            match fact {
                FillFact::Kind(k) => flip_in(&mut fill.kind, k),
                FillFact::Ref(r) => flip_in(&mut fill.refs, r),
                FillFact::Parent(p) => fill.parent = (fill.parent.as_ref() != Some(p)).then(|| p.clone()),
            }
            Ok(())
        })
    }

    /// Slot `slot` previewed by `filler` (a spawn zone adds it to its previews). Err: the filter's sentence when it does
    /// not admit it, or another problem the core finds (it stands on this stage already).
    pub fn fill(&mut self, slot: usize, filler: &str) -> Result<(), String> {
        let (h, fill) = self.slot(slot).ok_or("no such slot")?;
        fill.admits(filler).map_err(|e| format!("{}: {e}", h.id))?;
        self.change(|d| {
            let h = &mut d.holders[slot];
            h.preview = Some(match (h.take, h.preview.take()) {
                (Take::Many, Some(p)) => Fillers::Many(p.list().into_iter().map(String::from).chain([filler.to_string()]).collect()),
                (Take::Many, None) => Fillers::Many(vec![filler.to_string()]),
                (Take::One, _) => Fillers::One(filler.to_string()),
            });
            Ok(())
        })
    }

    /// slot `slot` without its preview: it plays nothing, its spot stays empty
    pub fn empty(&mut self, slot: usize) -> Result<(), String> {
        self.slot(slot).ok_or("no such slot")?;
        self.change(|d| {
            d.holders[slot].preview = None;
            Ok(())
        })
    }

    /// slot `slot` taken off the stage
    pub fn remove_slot(&mut self, slot: usize) -> Result<(), String> {
        self.slot(slot).ok_or("no such slot")?;
        self.change(|d| {
            d.holders.remove(slot);
            Ok(())
        })
    }
}

fn flip_in<T: PartialEq + Clone>(list: &mut Vec<T>, value: &T) {
    if list.contains(value) {
        list.retain(|x| x != value);
    } else {
        list.push(value.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_document::SceneDocument;
    use crate::scene_resolve::{core_rigs, no_sets};

    fn solo() -> SceneEdit {
        let mut d = SceneDocument::new("solo", "Solo", vec!["catpope".into()]);
        d.recast(&["character:catpope".into()]);
        SceneEdit::open(d, Some(1), &core_rigs, &no_sets).expect("it plays").0
    }

    #[test]
    fn a_slot_is_added_asked_and_filled_with_what_its_filter_admits() {
        let mut edit = solo();
        let s = edit.add_slot(AddressKind::Item).unwrap();
        assert_eq!(edit.doc.holders[s].id, "slot");
        assert_eq!(edit.doc.holders[s].spot.as_deref(), Some(STAGING_SPOTS[1]), "the next free spot");
        assert!(edit.changed());
        let lines = edit.tree();
        assert_eq!(lines.last().map(|l| (l.part, l.detail.as_str())), Some((Part::Slot(s), "empty slot")));
        let err = edit.fill(s, "character:toptoy").unwrap_err();
        assert!(err.contains("is a character, the placeholder takes item"), "{err}");
        edit.fill(s, "item:incense_burner").unwrap();
        assert!(edit.doc.check().is_empty());
        edit.refresh(&core_rigs, &no_sets).expect("plays");
        assert!(edit.members.iter().any(|m| m.holder == "slot" && m.filler == "item:incense_burner" && m.bundle.is_some()), "the preview plays its rig");
        assert!(!edit.tree().iter().any(|l| l.part == Part::Member(1)), "a slot's preview is no member line");
        assert!(edit.add_slot(AddressKind::Location).unwrap_err().contains("not a location"));
        let t = edit.add_slot(AddressKind::Character).unwrap();
        assert_eq!(edit.doc.holders[t].id, "slot_2");
        assert!(edit.fill(t, "character:catpope").unwrap_err().contains("stands on this stage already"), "the core's rule");
        edit.empty(s).unwrap();
        edit.remove_slot(t).unwrap();
        let back = SceneDocument::parse_json(&edit.doc.to_json().unwrap()).unwrap();
        assert_eq!(back, edit.doc, "saved with the draft");
    }

    #[test]
    fn a_slots_filter_is_the_books_question_every_fact_holds() {
        let mut edit = solo();
        let s = edit.add_slot(AddressKind::Character).unwrap();
        let offered = ["character:toptoy".to_string(), "item:incense".to_string()];
        let q = edit.question(s, &offered).unwrap();
        assert_eq!(q.words, "character");
        let (names, on) = q.chips();
        assert_eq!(names, ["character", "persona", "look", "item", "character:toptoy", "item:incense", "below character:toptoy"]);
        assert_eq!(on.iter().filter(|x| **x).count(), 1);
        edit.flip(s, &FillFact::Kind(AddressKind::Persona)).unwrap();
        edit.flip(s, &FillFact::Parent("character:toptoy".into())).unwrap();
        assert_eq!(edit.question(s, &offered).unwrap().words, "character or persona · below character:toptoy");
        assert!(edit.fill(s, "character:toptoy").unwrap_err().contains("is not below"), "the parent itself is not below it");
        edit.fill(s, "character:toptoy.puppet").unwrap();
        let err = edit.flip(s, &FillFact::Kind(AddressKind::Persona)).unwrap_err();
        assert!(err.contains("its preview"), "a filter that would refuse its preview is refused: {err}");
        assert!(edit.flip(s, &FillFact::Ref("item:incense".into())).unwrap_err().contains("a kind the filter does not take"));
        edit.empty(s).unwrap();
        edit.flip(s, &FillFact::Kind(AddressKind::Persona)).unwrap();
        assert!(edit.flip(s, &FillFact::Kind(AddressKind::Character)).unwrap_err().contains("names no kind"));
        edit.flip(s, &FillFact::Ref("character:toptoy".into())).unwrap();
        let q = edit.question(s, &[]).unwrap();
        assert_eq!(q.chip("character:toptoy").map(|i| q.facts[i].1), Some(true), "a fact named stays offered");
    }
}
