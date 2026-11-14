//! The toon's story events a row fires on (#539; SHOW.md §7): `GET /api/creator/story-events` (#521; server
//! FlowIndex::storyEvents, TS `StoryEventsIndex` in shared/content/src/flow.ts) read for one toon, and the event picker's
//! lines. A row fires on an event by its id (`fire.event`); an event the chapter editor gave no id is listed but cannot be
//! picked – the conductor drops it (conductor.rs, logged). The endpoint needs a content editor's token (creator or admin;
//! the role `live` alone gets 403): the words say so and the id is typed instead. The editor does the HTTP; the address,
//! the reading of the answer, the words and the lines are here (RULES A1, A2), as rows.rs is for the piece picker.
use crate::document::Show;
use serde_json::Value;

/// the inspector's line while the row fires on no event, the picker's line that clears it
pub const BY_HAND: &str = "by hand only";
/// the picker's field where there is no list: the event's id is typed
pub const EVENT: &str = "event";
/// what the picker says where there is no list (why not comes first, from [`refused`])
pub const TYPE_IT: &str = "Type the event's id and Enter – the conductor fires the row on it.";
/// what the picker says when no story event matches
pub const NONE_FOUND: &str = "No story event matches.";

/// One story event of a chapter, as the API lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryEvent {
    /// none: the chapter editor gave it none – no row can listen to it
    pub id: Option<String>,
    /// its VNEvent type: `showMessage`, `joinScene` …
    pub kind: String,
    pub characters: Vec<String>,
    pub excerpt: Option<String>,
    /// the group's label (none: unlabelled)
    pub group: Option<String>,
    pub chapter: u64,
}

/// `GET <api>/creator/story-events` – every toon's story events (the editor's token; role creator)
pub fn url(api: &str) -> String {
    format!("{}/creator/story-events", api.trim().trim_end_matches('/'))
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

/// `toon`'s story events from the API's answer, in chapter and document order (a toon the answer lacks has none)
pub fn parse(json: &str, toon: &str) -> Result<Vec<StoryEvent>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("the story events: {e}"))?;
    let toons = v.get("toons").and_then(Value::as_array).ok_or("the story events: no toons")?;
    let Some(t) = toons.iter().find(|t| t.get("id").and_then(Value::as_str) == Some(toon)) else { return Ok(vec![]) };
    let mut out = vec![];
    for c in t.get("chapters").and_then(Value::as_array).into_iter().flatten() {
        let chapter = c.get("number").and_then(Value::as_u64).unwrap_or_default();
        for e in c.get("events").and_then(Value::as_array).into_iter().flatten() {
            let characters = e.get("characters").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
            let id = text(e, "id").filter(|id| !id.is_empty());
            out.push(StoryEvent { id, kind: text(e, "type").unwrap_or_default(), characters, excerpt: text(e, "excerpt"), group: text(e, "group"), chapter });
        }
    }
    Ok(out)
}

/// Why there is no list: the HTTP status (none: no answer at all) and what the transport said.
pub fn refused(status: Option<u16>, detail: &str) -> String {
    match status {
        Some(401) => "the story events: not signed in – the editor's token is SCUTOONS_TOKEN".into(),
        Some(403) => "the story events need a creator's token (GET /api/creator/story-events) – the role live alone may not read them".into(),
        Some(code) => format!("the story events: HTTP {code}"),
        None => format!("the story events: {detail}"),
    }
}

/// what the picker says where the editor knows no toon
pub fn no_toon() -> String {
    "no toon – the editor lists a toon's story events with --toon <toon>".into()
}

/// One line of the event picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventView {
    /// what a pick sets: the id, or none for [`BY_HAND`]
    pub event: Option<String>,
    /// the id; `no id` for an event without one
    pub label: String,
    /// `ch 3 · nave · showMessage · catpope · Miau, Freunde!`, `· fires dialog` when another row fires on it
    pub detail: String,
    /// it can be picked (an event without an id cannot)
    pub can: bool,
}

/// The picker's lines for `row`: [`BY_HAND`] first while nothing is typed, then the toon's story events that match
/// `query` (its id, type, characters, group or text), in the chapters' order.
pub fn lines(show: &Show, row: &str, events: &[StoryEvent], query: &str) -> Vec<EventView> {
    let q = query.trim().to_lowercase();
    let mut out = vec![];
    if q.is_empty() {
        out.push(EventView { event: None, label: BY_HAND.into(), detail: "no event – GO and the hand fire it".into(), can: true });
    }
    for e in events {
        let mut words = vec![format!("ch {}", e.chapter)];
        words.extend(e.group.clone());
        words.push(e.kind.clone());
        if !e.characters.is_empty() {
            words.push(e.characters.join(", "));
        }
        words.extend(e.excerpt.clone());
        let hay = format!("{} {}", e.id.as_deref().unwrap_or_default(), words.join(" ")).to_lowercase();
        if !q.is_empty() && !hay.contains(&q) {
            continue;
        }
        let mut detail = words.join(" · ");
        match &e.id {
            None => detail = format!("no id – the chapter editor gives it one · {detail}"),
            Some(id) => {
                if let Some(other) = show.rows.iter().find(|r| r.id != row && r.fire.event.as_deref() == Some(id.as_str())) {
                    detail.push_str(&format!(" · fires {}", other.id));
                }
            }
        }
        out.push(EventView { event: e.id.clone(), label: e.id.clone().unwrap_or_else(|| "no id".into()), detail, can: e.id.is_some() });
    }
    out
}

/// the picker's heading for `row`
pub fn picker_title(show: &Show, row: &str, toon: &str) -> String {
    let n = show.rows.iter().position(|r| r.id == row).map_or(0, |i| i + 1);
    let of = if toon.is_empty() { "THE TOON'S".to_string() } else { format!("{}'S", toon.to_uppercase()) };
    format!("EVENT FOR ROW {n} {} – A STORY EVENT OF {of} CHAPTERS", row.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse as show;
    use crate::rows::{edit, Edit};

    const EVENTS: &str = include_str!("../fixtures/story-events.json");

    fn top_toy() -> Show {
        show(include_str!("../fixtures/top-toy.show.json")).unwrap()
    }

    /// #539's done-when, the core's half: the API's answer read for the toon, its lines, a pick through `rows::edit`
    #[test]
    fn the_toons_story_events_are_read_and_a_rows_event_is_picked_from_them() {
        let events = parse(EVENTS, "catpope").unwrap();
        assert_eq!(events.len(), 5, "toptoy's are not catpope's");
        assert_eq!(events[0], StoryEvent { id: Some("ch3.dialog-start".into()), kind: "showMessage".into(), characters: vec!["catpope".into()], excerpt: Some("Miau, Freunde!".into()), group: Some("nave".into()), chapter: 3 });
        assert_eq!(events[1].id, None);
        assert!(parse(EVENTS, "nobody").unwrap().is_empty());
        assert!(parse("{}", "catpope").is_err());
        assert_eq!(url("http://127.0.0.1:9000/api/"), "http://127.0.0.1:9000/api/creator/story-events");

        let s = top_toy();
        let all = lines(&s, "opening", &events, "");
        assert_eq!(all[0], EventView { event: None, label: BY_HAND.into(), detail: "no event – GO and the hand fire it".into(), can: true });
        assert_eq!(all[1].detail, "ch 3 · nave · showMessage · catpope · Miau, Freunde! · fires dialog");
        assert_eq!((all[2].label.as_str(), all[2].can), ("no id", false));
        assert!(all[2].detail.starts_with("no id – the chapter editor gives it one · ch 3 · nave"), "{}", all[2].detail);
        assert_eq!(all[4].detail, "ch 3 · crypt · showMessage · catpope, yellowmouse · Die Kerzen brennen.");
        assert_eq!(all[5].detail, "ch 3 · jumpTo · fires finale");
        // the row's own event is not "another row's"
        assert!(!lines(&s, "dialog", &events, "dialog")[0].detail.contains("fires"));
        // found by type, character, text; typing hides the by-hand line
        assert_eq!(lines(&s, "opening", &events, "kerzen").iter().map(|l| l.label.as_str()).collect::<Vec<_>>(), ["ch3.vigil"]);
        assert_eq!(lines(&s, "opening", &events, "JOINSCENE").len(), 1);
        assert!(lines(&s, "opening", &events, "nothing like it").is_empty());

        // picked: the row fires on it; another row's event is refused in validate's words; by hand clears it
        let vigil = all.iter().find(|l| l.label == "ch3.vigil").unwrap();
        let picked = edit(&s, &Edit::Event { row: "opening".into(), event: vigil.event.clone() }).unwrap();
        assert_eq!(picked.said, "row 1 opening fires on ch3.vigil");
        assert_eq!(picked.show.rows[0].fire.event.as_deref(), Some("ch3.vigil"));
        assert_eq!(picked.show.rows[0].fire.how, s.rows[0].fire.how, "how stays");
        assert_eq!(edit(&s, &Edit::Event { row: "opening".into(), event: Some("ch3.end".into()) }).unwrap_err(), "not changed: row 'finale': the event 'ch3.end' already fires row 'opening'");
        let hand = edit(&picked.show, &Edit::Event { row: "opening".into(), event: None }).unwrap();
        assert_eq!((hand.said.as_str(), hand.show.rows[0].fire.event.as_ref()), ("row 1 opening: by hand only", None));
        assert_eq!(edit(&s, &Edit::Event { row: "dialog".into(), event: Some("  ".into()) }).unwrap().show.rows[1].fire.event, None, "empty typed: by hand");
    }

    #[test]
    fn the_picker_says_why_there_is_no_list() {
        assert!(refused(Some(403), "").contains("creator"));
        assert!(refused(Some(401), "").contains("SCUTOONS_TOKEN"));
        assert_eq!(refused(Some(500), ""), "the story events: HTTP 500");
        assert_eq!(refused(None, "offline"), "the story events: offline");
        assert!(no_toon().contains("--toon"));
        assert_eq!(picker_title(&top_toy(), "dialog", "catpope"), "EVENT FOR ROW 2 DIALOG – A STORY EVENT OF CATPOPE'S CHAPTERS");
    }
}
