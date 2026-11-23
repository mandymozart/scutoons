//! What can fire a row: the lines of Find under its ⚡ slot (lap 2 P7, research/redesign/lap2/SPORES-VIEW.md). The scope
//! pills widen it – This toon (the home toon's story events), Show (+ the show's other toons, another row's fire, a vote's
//! result), Universe (+ every other toon's story events, and the sources that get their own plan later, greyed). A row
//! fires on several events and several rows hear one; a press turns a line's event on or off for the row (`rows::Edit::Event`),
//! "manual" clears them. The editor lays the lines out and hands the presses in (RULES A1, A2).
use crate::document::{row_fired, Show};
use crate::story::{StoryEvent, BY_HAND};

/// How wide Find looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    #[default]
    Toon,
    Show,
    Universe,
}

/// the scope pills, in their order
pub const SCOPES: [(Scope, &str); 3] = [(Scope::Toon, "This toon"), (Scope::Show, "Show"), (Scope::Universe, "Universe")];

/// Where an event comes from: the group its line stands in once Find is wider than the toon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Manual,
    Story,
    Row,
    Vote,
    /// not a source in this lap: listed greyed, a plan of its own follows
    Place,
    Device,
    Jobs,
    Assets,
}

impl Source {
    /// the group's head
    pub fn word(self) -> &'static str {
        match self {
            Source::Manual => "",
            Source::Story => "story",
            Source::Row => "rows",
            Source::Vote => "votes",
            Source::Place => "place",
            Source::Device => "device",
            Source::Jobs => "jobs",
            Source::Assets => "assets",
        }
    }
}

/// why the greyed sources cannot be picked yet
pub const LATER: &str = "later – a plan of its own";

/// One line of Find.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// what a press turns on or off; none: "manual", which clears the row's events
    pub event: Option<String>,
    pub label: String,
    /// where it comes from, who else listens (`· fires dialog, finale`); why it cannot be picked first
    pub detail: String,
    pub source: Source,
    /// the row fires on it now (its tick)
    pub on: bool,
    pub can: bool,
}

/// `· fires a, b` for the other rows that listen to `event`
fn listeners(show: &Show, row: &str, event: &str) -> String {
    let rows: Vec<&str> = show.rows.iter().filter(|r| r.id != row && r.fire.event.contains(event)).map(|r| r.id.as_str()).collect();
    if rows.is_empty() {
        String::new()
    } else {
        format!(" · fires {}", rows.join(", "))
    }
}

fn story_line(show: &Show, row: &str, e: &StoryEvent, on: &dyn Fn(&str) -> bool) -> Line {
    let mut words: Vec<String> = e.toon.iter().cloned().collect();
    words.push(format!("ch {}", e.chapter));
    words.extend(e.group.clone());
    words.push(e.kind.clone());
    if !e.characters.is_empty() {
        words.push(e.characters.join(", "));
    }
    words.extend(e.excerpt.clone());
    let detail = words.join(" · ");
    match &e.id {
        None => Line { event: None, label: "no id".into(), detail: format!("no id – the chapter editor gives it one · {detail}"), source: Source::Story, on: false, can: false },
        Some(id) => Line { event: Some(id.clone()), label: id.clone(), detail: format!("{detail}{}", listeners(show, row, id)), source: Source::Story, on: on(id), can: true },
    }
}

/// Find's lines for `row` in `scope` that match `query` (its id or its words): "manual" first while nothing is typed, then
/// by source. `events` are the story events listed (the home's without a toon, the others marked with theirs).
pub fn lines(show: &Show, row: &str, events: &[StoryEvent], scope: Scope, query: &str) -> Vec<Line> {
    let Some(r) = show.row(row) else { return vec![] };
    let on = |e: &str| r.fire.event.contains(e);
    let mut out = vec![];
    let home = |e: &&StoryEvent| e.toon.is_none();
    let of_show = |e: &&StoryEvent| e.toon.as_ref().is_some_and(|t| show.toons.contains(t));
    out.extend(events.iter().filter(home).map(|e| story_line(show, row, e, &on)));
    if scope != Scope::Toon {
        out.extend(events.iter().filter(of_show).map(|e| story_line(show, row, e, &on)));
        for other in show.rows.iter().filter(|o| o.id != row) {
            let event = row_fired(&other.id);
            let n = show.rows.iter().position(|x| x.id == other.id).unwrap_or_default() + 1;
            let detail = format!("when row {n:02} {} fires{}", other.id, listeners(show, row, &event));
            out.push(Line { on: on(&event), label: event.clone(), event: Some(event), detail, source: Source::Row, can: true });
        }
        for q in &show.questions {
            for (k, option) in q.options().iter().enumerate() {
                let event = crate::vote::event(&q.id, k);
                let detail = format!("{} · {}{}", q.label("en"), crate::story::label_in(option, "en"), listeners(show, row, &event));
                out.push(Line { on: on(&event), label: event.clone(), event: Some(event), detail, source: Source::Vote, can: true });
            }
        }
    }
    if scope == Scope::Universe {
        out.extend(events.iter().filter(|e| !home(e) && !of_show(e)).map(|e| story_line(show, row, e, &on)));
        for (source, what) in [(Source::Place, "arriving at a place"), (Source::Device, "a MIDI note, a controller, DMX in"), (Source::Jobs, "a night run, a publish, a draft saved"), (Source::Assets, "a picture arrived, a text approved")] {
            out.push(Line { event: None, label: source.word().into(), detail: format!("{what} · {LATER}"), source, on: false, can: false });
        }
    }
    let q = query.trim().to_lowercase();
    out.retain(|l| q.is_empty() || format!("{} {}", l.label, l.detail).to_lowercase().contains(&q));
    if q.is_empty() {
        out.insert(0, Line { event: None, label: BY_HAND.into(), detail: "no event – its ▶ and GO fire it".into(), source: Source::Manual, on: r.fire.event.is_empty(), can: true });
    }
    out
}

/// what Find's head says: the row and the scope
pub fn title(show: &Show, row: &str) -> String {
    let n = show.rows.iter().position(|r| r.id == row).map_or(0, |i| i + 1);
    format!("row {n:02} {row} fires on")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::{edit, Edit};

    const TOP_TOY: &str = include_str!("../fixtures/top-toy.show.json");
    const EVENTS: &str = include_str!("../../demos/story-events.json");

    fn labels(l: &[Line]) -> Vec<&str> {
        l.iter().map(|l| l.label.as_str()).collect()
    }

    /// the pills widen Find: the toon's story events; + the other rows' fires and the votes' results; + the greyed sources
    #[test]
    fn the_scope_pills_widen_find_from_the_toon_to_the_show_to_the_universe() {
        let show = crate::document::open(TOP_TOY).unwrap();
        let events = crate::story::read_toons(EVENTS, &show.toons).unwrap().events;
        let toon = lines(&show, "dialog", &events, Scope::Toon, "");
        assert_eq!(toon[0].label, BY_HAND);
        assert!(toon.iter().skip(1).all(|l| l.source == Source::Story), "{:?}", labels(&toon));
        let wide = lines(&show, "dialog", &events, Scope::Show, "");
        let rows: Vec<&str> = wide.iter().filter(|l| l.source == Source::Row).map(|l| l.label.as_str()).collect();
        assert!(!rows.contains(&"row:dialog") && rows.contains(&"row:opening"), "the other rows, not its own: {rows:?}");
        let universe = lines(&show, "dialog", &events, Scope::Universe, "");
        let greyed: Vec<&Line> = universe.iter().filter(|l| !l.can && l.source != Source::Story).collect();
        assert_eq!(greyed.iter().map(|l| l.label.as_str()).collect::<Vec<_>>(), ["place", "device", "jobs", "assets"]);
        assert!(greyed.iter().all(|l| l.detail.ends_with(LATER)));
        // found by typing, manual only while nothing is typed
        let found = lines(&show, "dialog", &events, Scope::Show, "opening");
        assert_eq!(labels(&found), ["row:opening"]);
    }

    /// a row fires on two events, both ticked; another row hears one of them too and the line says so
    #[test]
    fn a_row_takes_several_events_and_an_event_several_rows() {
        let show = crate::document::open(TOP_TOY).unwrap();
        let events = crate::story::read_toons(EVENTS, &show.toons).unwrap().events;
        assert_eq!(show.reads, None);
        let two = edit(&show, &Edit::Event { row: "dialog".into(), event: Some("ch3.vigil".into()) }).unwrap();
        assert_eq!(two.said, "row 2 dialog fires on ch3.dialog-start, ch3.vigil");
        assert_eq!(two.show.reads, Some(2), "stamped: an older deck refuses it");
        let l = lines(&two.show, "dialog", &events, Scope::Show, "");
        let ticked: Vec<&str> = l.iter().filter(|l| l.on).map(|l| l.label.as_str()).collect();
        assert_eq!(ticked, ["ch3.dialog-start", "ch3.vigil"]);
        let finale = edit(&two.show, &Edit::Event { row: "finale".into(), event: Some("ch3.vigil".into()) }).unwrap();
        let heard = lines(&finale.show, "opening", &events, Scope::Toon, "vigil");
        assert!(heard[0].detail.ends_with(" · fires dialog, finale"), "{}", heard[0].detail);
        // pressed again: off, and the stamp goes with what needed it; a row's fire needs it too; manual clears them
        let off = edit(&two.show, &Edit::Event { row: "dialog".into(), event: Some("ch3.vigil".into()) }).unwrap();
        assert_eq!((off.said.as_str(), off.show.reads), ("row 2 dialog fires on ch3.dialog-start", None));
        let by_row = edit(&off.show, &Edit::Event { row: "finale".into(), event: Some("row:opening".into()) }).unwrap();
        assert_eq!(by_row.show.reads, Some(2));
        let hand = edit(&two.show, &Edit::Event { row: "dialog".into(), event: None }).unwrap();
        assert!(hand.show.rows[1].fire.event.is_empty());
    }
}
