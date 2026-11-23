//! The toon's story events a row fires on (#539; SHOW.md §7): `GET /api/creator/story-events` (#521; server
//! FlowIndex::storyEvents, TS `StoryEventsIndex` in shared/content/src/flow.ts) read for one toon, and the event picker's
//! lines. A row fires on an event by its id (`fire.event`); an event the chapter editor gave no id is listed but cannot be
//! picked – the conductor drops it (conductor.rs, logged). The endpoint needs a token with the role creator, admin or
//! `live` (#555); without one the words say so and the id is typed instead. The editor does the HTTP; the address,
//! the reading of the answer, the words and the lines are here (RULES A1, A2), as rows.rs is for the piece picker.
//! The same answer lists per chapter its questions (#550, SHOW.md §15 point 4): a `showChoices` of the chapter's published
//! version with its `addChoice` options, whole labels by locale, and that version – read here once with the events
//! ([`read`]), offered by the question picker ([`choice_lines`]); a question without an id is listed and cannot be picked.
//! A pick writes a show question's `story` – the labels at that version, a snapshot ([`pick`]); a published choice that
//! changed since is a warning ([`changed`]).
use crate::document::{Cell, Question, Show, StoryChoice as Snapshot};
use crate::rows::Edited;
use crate::view::{Points, ProblemView};
use serde_json::Value;
use std::collections::BTreeMap;

/// the inspector's line while the row fires on no event, the picker's line that clears it
pub const BY_HAND: &str = "manual";
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
    /// #708: whose it is, in the list of a show of several toons (none: the home's)
    pub toon: Option<String>,
}

/// `GET <api>/creator/story-events` – every toon's story events (the editor's token; role creator, admin or live)
pub fn url(api: &str) -> String {
    format!("{}/creator/story-events", api.trim().trim_end_matches('/'))
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

/// One option of a story question: the `addChoice`'s id (none: it has none) and its whole label by locale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption {
    pub id: Option<String>,
    pub label: BTreeMap<String, String>,
}

/// One question of a chapter's published version, as the API lists it (#550): a `showChoices` and the options before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryChoice {
    /// the showChoices' id; none: the chapter editor gave it none – no vote can name it
    pub event: Option<String>,
    /// the group's label (none: unlabelled)
    pub group: Option<String>,
    pub chapter: u64,
    /// every option, also one with an "Only if" (§15 question 4)
    pub options: Vec<ChoiceOption>,
    /// the chapter version (entity_versions id) the API read it from
    pub version: u64,
}

/// A toon's part of the API's answer: its story events and its chapters' questions, each in chapter and document order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Story {
    pub events: Vec<StoryEvent>,
    pub choices: Vec<StoryChoice>,
}

/// `toon`'s story events and questions from the API's answer (a toon the answer lacks has none; a field it lacks is
/// left out, as an answer before #550 had no `choices`)
pub fn read(json: &str, toon: &str) -> Result<Story, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("the story events: {e}"))?;
    let toons = v.get("toons").and_then(Value::as_array).ok_or("the story events: no toons")?;
    let Some(t) = toons.iter().find(|t| t.get("id").and_then(Value::as_str) == Some(toon)) else { return Ok(Story::default()) };
    let mut out = Story::default();
    for c in t.get("chapters").and_then(Value::as_array).into_iter().flatten() {
        let chapter = c.get("number").and_then(Value::as_u64).unwrap_or_default();
        for e in c.get("events").and_then(Value::as_array).into_iter().flatten() {
            let characters = e.get("characters").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
            let id = text(e, "id").filter(|id| !id.is_empty());
            out.events.push(StoryEvent { id, kind: text(e, "type").unwrap_or_default(), characters, excerpt: text(e, "excerpt"), group: text(e, "group"), chapter, toon: None });
        }
        for q in c.get("choices").and_then(Value::as_array).into_iter().flatten() {
            let options = q.get("options").and_then(Value::as_array).into_iter().flatten().map(option).collect();
            let event = text(q, "event").filter(|id| !id.is_empty());
            let version = q.get("version").and_then(Value::as_u64).unwrap_or_default();
            out.choices.push(StoryChoice { event, group: text(q, "group"), chapter, options, version });
        }
    }
    Ok(out)
}

/// #708: the story events of a show's toons from the API's answer – the home's (the first) with its questions, then each
/// other toon's events, each marked with its toon. The questions stay the home's: a vote snapshots one toon's choice.
pub fn read_toons(json: &str, toons: &[String]) -> Result<Story, String> {
    let Some((home, others)) = toons.split_first() else { return read(json, "") };
    let mut out = read(json, home)?;
    for toon in others {
        out.events.extend(read(json, toon)?.events.into_iter().map(|e| StoryEvent { toon: Some(toon.clone()), ..e }));
    }
    Ok(out)
}

/// The same, and then every other toon's events in the answer, each marked with its toon – what Find lists under Universe.
pub fn read_all(json: &str, toons: &[String]) -> Result<Story, String> {
    let mut out = read_toons(json, toons)?;
    let v: Value = serde_json::from_str(json).map_err(|e| format!("the story events: {e}"))?;
    let all = v.get("toons").and_then(Value::as_array).into_iter().flatten().filter_map(|t| t.get("id").and_then(Value::as_str));
    for toon in all.filter(|t| !toons.iter().any(|x| x == t)).map(str::to_string).collect::<Vec<_>>() {
        out.events.extend(read(json, &toon)?.events.into_iter().map(|e| StoryEvent { toon: Some(toon.clone()), ..e }));
    }
    Ok(out)
}

fn option(o: &Value) -> ChoiceOption {
    let label = o.get("label").and_then(Value::as_object).into_iter().flatten().filter_map(|(l, t)| Some((l.clone(), t.as_str()?.to_string()))).collect();
    ChoiceOption { id: text(o, "id").filter(|id| !id.is_empty()), label }
}

/// `toon`'s story events from the API's answer, in chapter and document order (a toon the answer lacks has none)
pub fn parse(json: &str, toon: &str) -> Result<Vec<StoryEvent>, String> {
    read(json, toon).map(|s| s.events)
}

/// Why there is no list: the HTTP status (none: no answer at all) and what the transport said.
pub fn refused(status: Option<u16>, detail: &str) -> String {
    match status {
        Some(401) => "the story events: not signed in – the editor's token is SCUTOONS_TOKEN".into(),
        Some(403) => "the story events need a token with the role creator, admin or live (GET /api/creator/story-events)".into(),
        Some(code) => format!("the story events: HTTP {code}"),
        None => format!("the story events: {detail}"),
    }
}

/// what the picker says where the editor knows no toon
pub fn no_toon() -> String {
    "no toon – the editor lists a toon's story events with --toon <toon>".into()
}

/// what the question picker says when no story question matches
pub const NO_CHOICE_FOUND: &str = "No story question matches.";

/// a label in `locale`, else in English, else its first locale's (none: empty)
pub fn label_in<'a>(label: &'a BTreeMap<String, String>, locale: &str) -> &'a str {
    label.get(locale).or_else(|| label.get("en")).or_else(|| label.values().next()).map_or("", String::as_str)
}

/// One line of the question picker (#550).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceView {
    /// the question's place in the list the lines were made from
    pub choice: usize,
    /// the showChoices' id; `no id` for a question without one
    pub label: String,
    /// `ch 3 · nave · Stay / Go`; why it cannot be picked first
    pub detail: String,
    /// it can be picked: it has an id and an option
    pub can: bool,
}

/// why a story question cannot be a vote's (none: it can)
pub fn unpickable(c: &StoryChoice) -> Option<&'static str> {
    if c.event.is_none() {
        Some("no id – the chapter editor gives the question one")
    } else if c.options.is_empty() {
        Some("no options – the chapter adds choices before it")
    } else {
        None
    }
}

/// The question picker's lines: `toon`'s story questions that match `query` (the id, group, chapter or an option's
/// label in any language), in the chapters' order, the options' labels in `locale`; `· asked by q1` when a question of
/// the show asks it already.
pub fn choice_lines(show: &Show, toon: &str, choices: &[StoryChoice], query: &str, locale: &str) -> Vec<ChoiceView> {
    let q = query.trim().to_lowercase();
    let mut out = vec![];
    for (i, c) in choices.iter().enumerate() {
        let mut words = vec![format!("ch {}", c.chapter)];
        words.extend(c.group.clone());
        if !c.options.is_empty() {
            words.push(c.options.iter().map(|o| label_in(&o.label, locale)).collect::<Vec<_>>().join(" / "));
        }
        let every = c.options.iter().flat_map(|o| o.label.values().map(String::as_str)).collect::<Vec<_>>().join(" ");
        let hay = format!("{} {} {every}", c.event.as_deref().unwrap_or_default(), words.join(" ")).to_lowercase();
        if !q.is_empty() && !hay.contains(&q) {
            continue;
        }
        let why = unpickable(c);
        let mut detail = match why {
            Some(why) => format!("{why} · {}", words.join(" · ")),
            None => words.join(" · "),
        };
        if let Some(q) = asked_by(show, toon, c) {
            detail.push_str(&format!(" · asked by {q}"));
        }
        out.push(ChoiceView { choice: i, label: c.event.clone().unwrap_or_else(|| "no id".into()), detail, can: why.is_none() });
    }
    out
}

/// The `story` a pick writes into a question (#550): the toon's question at the version listed, its options' labels as
/// they are there – a snapshot, as a seat pins a version. Refused with why for a question that cannot be picked.
pub fn snapshot(choice: &StoryChoice, toon: &str) -> Result<Snapshot, String> {
    if let Some(why) = unpickable(choice) {
        return Err(format!("not a question for a vote: {why}"));
    }
    if toon.trim().is_empty() {
        return Err(no_toon());
    }
    let chapter = u32::try_from(choice.chapter).map_err(|_| format!("chapter {} is out of range", choice.chapter))?;
    let version = u32::try_from(choice.version).map_err(|_| format!("version {} is out of range", choice.version))?;
    let options = choice.options.iter().map(|o| o.label.clone()).collect();
    Ok(Snapshot { toon: toon.trim().to_string(), chapter, version, event: choice.event.clone().unwrap_or_default(), options, ids: ids(choice) })
}

/// #552: the options' `addChoice` ids as a snapshot keeps them ('' = none)
fn ids(c: &StoryChoice) -> Vec<String> {
    c.options.iter().map(|o| o.id.clone().unwrap_or_default()).collect()
}

/// A pick (#550; the editor's "one of the story's" on selected cells, #551): row `row` asks the toon's story question.
/// The question the row asks already takes it – keeping how its vote runs (rule, n, seconds, show; its fallback while
/// still an option) and dropping its typed words; else a new question (`q1`, `q2` … the first free) with the vote form's
/// defaults (deck-ui `Form::default`). The `columns` named ask it (`ask` cells). Checked by validate as every edit is
/// (rows.rs `edit`).
pub fn pick(show: &Show, row: &str, columns: &[String], choice: &StoryChoice, toon: &str) -> Result<Edited, String> {
    let story = snapshot(choice, toon)?;
    let i = show.rows.iter().position(|r| r.id == row).ok_or_else(|| format!("no row '{row}'"))?;
    let before = crate::validate::validate(show);
    let mut next = show.clone();
    let id = match crate::rows::asked(&show.rows[i]) {
        Some(id) => {
            let q = next.questions.iter_mut().find(|q| q.id == id).ok_or_else(|| format!("no question '{id}'"))?;
            if q.fallback >= story.options.len() {
                q.fallback = 0;
            }
            q.typed = None;
            q.story = Some(story.clone());
            id.to_string()
        }
        None if columns.is_empty() => return Err(format!("row {} {row}: pick the cells that ask it", i + 1)),
        None => {
            let id = crate::question::free_id(show);
            let f = crate::vote::Form::default();
            let rule = crate::vote::wire_rule(f.rule);
            let n = (rule == protocol::VoteRule::First).then_some(f.n);
            next.questions.push(Question { id: id.clone(), typed: None, story: Some(story.clone()), rule, n, seconds: f.seconds, fallback: f.fallback, show: f.show });
            id
        }
    };
    // #551: the cells take the vote's one filter – the one the row's question carries already, a new question's phones
    crate::question::join(&mut next.rows[i], columns, &id, Some(crate::question::new_filter()));
    let said = format!("row {} {row}: {id} asks {} ch {} {} (version {})", i + 1, story.toon, story.chapter, story.event, story.version);
    let added: Vec<String> = crate::validate::validate(&next).into_iter().filter(|p| !before.contains(p)).map(|p| p.to_string()).collect();
    match added.first() {
        Some(p) => Err(format!("not changed: {p}")),
        None => Ok(Edited { show: next, said, row: Some(row.to_string()) }),
    }
}

/// The show's story questions of `toon` held against the toon's list (#550): a question whose published choice changed
/// since it was picked (other options or labels at a newer version) or that the published chapter no longer has. Warnings
/// – GO is never blocked; each points at the first row that asks it. The editor adds them to the problems list.
pub fn changed(show: &Show, toon: &str, choices: &[StoryChoice]) -> Vec<ProblemView> {
    let mut out = vec![];
    for q in &show.questions {
        let Some(text) = changed_text(q, toon, choices) else { continue };
        let points = show.rows.iter().position(|r| r.cells.values().any(|c| matches!(c, Cell::Ask { question } if *question == q.id))).map_or(Points::Show, Points::Row);
        out.push(ProblemView { text, error: false, points });
    }
    out
}

/// One question held against `toon`'s list: the warning's words (none: not the toon's story question, or unchanged). The
/// problems list ([`changed`]) and the question in the cell inspector (#551, question.rs) say the same.
pub fn changed_text(q: &Question, toon: &str, choices: &[StoryChoice]) -> Option<String> {
    let s = q.story.as_ref().filter(|s| s.toon == toon)?;
    match choices.iter().find(|c| c.chapter == u64::from(s.chapter) && c.event.as_deref() == Some(s.event.as_str())) {
        None => Some(format!("question '{}': ch {} of {toon} has no published question {} any more", q.id, s.chapter, s.event)),
        // #552: the options' ids too – the linked reader finds the voted option by its id (a snapshot from before has none)
        Some(c) if c.options.iter().map(|o| &o.label).ne(s.options.iter()) || ids(c) != s.ids => Some(format!("question '{}': {} changed since it was picked (version {} → {}) – pick it again", q.id, s.event, s.version, c.version)),
        Some(_) => None,
    }
}

/// the show's question that asks this story question already (its id)
fn asked_by<'a>(show: &'a Show, toon: &str, c: &StoryChoice) -> Option<&'a str> {
    let event = c.event.as_deref()?;
    show.questions.iter().find(|q| q.story.as_ref().is_some_and(|s| s.toon == toon && u64::from(s.chapter) == c.chapter && s.event == event)).map(|q| q.id.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{parse as show, Events};
    use crate::inlet::{lines, Line, Scope, Source};
    use crate::rows::{edit, Edit};

    const EVENTS: &str = include_str!("../../demos/story-events.json");

    fn top_toy() -> Show {
        show(include_str!("../fixtures/top-toy.show.json")).unwrap()
    }

    /// #708: a show of two toons lists both toons' events, the other's marked with its toon (and found by it); the questions
    /// stay the home's
    #[test]
    fn a_show_of_several_toons_lists_each_toons_events_marked() {
        let toons = ["catpope".to_string(), "toptoy".to_string()];
        let story = read_toons(EVENTS, &toons).unwrap();
        assert_eq!(story.events.len(), 6);
        assert_eq!((story.events[5].id.as_deref(), story.events[5].toon.as_deref()), (Some("set1"), Some("toptoy")));
        assert!(story.events[..5].iter().all(|e| e.toon.is_none()), "the home's unmarked");
        assert_eq!(story.choices, read(EVENTS, "catpope").unwrap().choices);
        let found = lines(&top_toy(), "dialog", &story.events, Scope::Universe, "toptoy");
        assert_eq!(found.iter().map(|l| l.label.as_str()).collect::<Vec<_>>(), ["set1"]);
        assert!(found[0].detail.starts_with("toptoy · ch 1"), "{}", found[0].detail);
        assert_eq!(read_toons(EVENTS, &toons[..1]).unwrap(), read(EVENTS, "catpope").unwrap(), "one toon: as before");
    }

    /// #539's done-when, the core's half: the API's answer read for the toon, its lines, a pick through `rows::edit`
    #[test]
    fn the_toons_story_events_are_read_and_a_rows_event_is_picked_from_them() {
        let events = parse(EVENTS, "catpope").unwrap();
        assert_eq!(events.len(), 5, "toptoy's are not catpope's");
        assert_eq!(events[0], StoryEvent { id: Some("ch3.dialog-start".into()), kind: "showMessage".into(), characters: vec!["catpope".into()], excerpt: Some("Miau, Freunde!".into()), group: Some("nave".into()), chapter: 3, toon: None });
        assert_eq!(events[1].id, None);
        assert!(parse(EVENTS, "nobody").unwrap().is_empty());
        assert!(parse("{}", "catpope").is_err());
        assert_eq!(url("http://127.0.0.1:9000/api/"), "http://127.0.0.1:9000/api/creator/story-events");

        let s = top_toy();
        let all = lines(&s, "opening", &events, Scope::Toon, "");
        assert_eq!(all[0], Line { event: None, label: BY_HAND.into(), detail: "no event – its ▶ and GO fire it".into(), source: Source::Manual, on: true, can: true });
        assert_eq!(all[1].detail, "ch 3 · nave · showMessage · catpope · Miau, Freunde! · fires dialog");
        assert_eq!((all[2].label.as_str(), all[2].can), ("no id", false));
        assert!(all[2].detail.starts_with("no id – the chapter editor gives it one · ch 3 · nave"), "{}", all[2].detail);
        assert_eq!(all[4].detail, "ch 3 · crypt · showMessage · catpope, yellowmouse · Die Kerzen brennen.");
        assert_eq!(all[5].detail, "ch 3 · jumpTo · fires finale");
        // the row's own event is not "another row's"
        assert!(!lines(&s, "dialog", &events, Scope::Toon, "dialog")[0].detail.contains("fires"));
        // found by type, character, text; typing hides the by-hand line
        assert_eq!(lines(&s, "opening", &events, Scope::Toon, "kerzen").iter().map(|l| l.label.as_str()).collect::<Vec<_>>(), ["ch3.vigil"]);
        assert_eq!(lines(&s, "opening", &events, Scope::Toon, "JOINSCENE").len(), 1);
        assert!(lines(&s, "opening", &events, Scope::Toon, "nothing like it").is_empty());

        // picked: the row fires on it; another row's event too – several rows hear one event; by hand clears it
        let vigil = all.iter().find(|l| l.label == "ch3.vigil").unwrap();
        let picked = edit(&s, &Edit::Event { row: "opening".into(), event: vigil.event.clone() }).unwrap();
        assert_eq!(picked.said, "row 1 opening fires on ch3.vigil");
        assert_eq!(picked.show.rows[0].fire.event, Events::one("ch3.vigil"));
        assert_eq!(picked.show.rows[0].fire.how, s.rows[0].fire.how, "how stays");
        assert_eq!(edit(&s, &Edit::Event { row: "opening".into(), event: Some("ch3.end".into()) }).unwrap().said, "row 1 opening fires on ch3.end");
        let hand = edit(&picked.show, &Edit::Event { row: "opening".into(), event: None }).unwrap();
        assert_eq!((hand.said.as_str(), hand.show.rows[0].fire.event.is_empty()), ("row 1 opening: manual", true));
        assert_eq!(edit(&s, &Edit::Event { row: "dialog".into(), event: Some("  ".into()) }).unwrap().show.rows[1].fire.event, Events::default(), "empty typed: by hand");
    }

    /// #550, the core's half: the toon's questions read with its events, the question picker's lines, an id-less one not pickable
    #[test]
    fn the_toons_story_questions_are_read_and_offered() {
        let story = read(EVENTS, "catpope").unwrap();
        let s = top_toy();
        assert_eq!(story.events, parse(EVENTS, "catpope").unwrap(), "one answer, read once");
        assert_eq!(story.choices.len(), 3);
        let door = &story.choices[0];
        assert_eq!((door.event.as_deref(), door.group.as_deref(), door.chapter, door.version), (Some("ch3.door"), Some("nave"), 3, 812));
        assert_eq!(door.options[0], ChoiceOption { id: Some("stay".into()), label: BTreeMap::from([("de".into(), "Bleiben".into()), ("en".into(), "Stay".into())]) });
        assert_eq!(door.options[1].id, None, "an option without an id is read");
        assert_eq!(story.choices[1].event, None);
        // an answer without `choices` (toptoy's, as before #550) has none; a toon the answer lacks has nothing
        let toptoy = read(EVENTS, "toptoy").unwrap();
        assert_eq!((toptoy.events.len(), toptoy.choices.len()), (1, 0));
        assert_eq!(read(EVENTS, "nobody").unwrap(), Story::default());

        let all = choice_lines(&s, "catpope", &story.choices, "", "de");
        assert_eq!(all[0], ChoiceView { choice: 0, label: "ch3.door".into(), detail: "ch 3 · nave · Bleiben / Gehen".into(), can: true });
        assert_eq!(all[1], ChoiceView { choice: 1, label: "no id".into(), detail: "no id – the chapter editor gives the question one · ch 3 · crypt · Ja / Nein".into(), can: false });
        assert_eq!((all[2].can, all[2].detail.as_str()), (false, "no options – the chapter adds choices before it · ch 3"));
        // the labels in the editor's language, else English, else the first there is
        assert_eq!(choice_lines(&s, "catpope", &story.choices, "", "fr")[0].detail, "ch 3 · nave · Stay / Go");
        assert_eq!(choice_lines(&s, "catpope", &story.choices, "", "en")[1].detail, "no id – the chapter editor gives the question one · ch 3 · crypt · Ja / Nein");
        // found by an option's label in any language, the group, the id
        assert_eq!(choice_lines(&s, "catpope", &story.choices, "go", "de").iter().map(|l| l.choice).collect::<Vec<_>>(), [0]);
        assert_eq!(choice_lines(&s, "catpope", &story.choices, "CRYPT", "de").iter().map(|l| l.choice).collect::<Vec<_>>(), [1]);
        assert_eq!(choice_lines(&s, "catpope", &story.choices, "ch3.empty", "de").len(), 1);
        assert!(choice_lines(&s, "catpope", &story.choices, "nothing like it", "de").is_empty());
        assert_eq!(unpickable(door), None);
    }

    /// #550's done-when, part 3: a picked story question becomes a show question's `story` (a snapshot at that version);
    /// a published choice that changed since is a warning
    #[test]
    fn a_picked_story_question_becomes_a_question_and_a_changed_one_a_warning() {
        let story = read(EVENTS, "catpope").unwrap();
        let door = &story.choices[0];
        let show = show(include_str!("../fixtures/vote-cell.show.json")).unwrap();

        let cols = |c: &[&str]| c.iter().map(|c| c.to_string()).collect::<Vec<_>>();

        // a row asking nothing yet: a new question (the first free id), the snapshot, the vote form's defaults; the cells ask it
        let picked = pick(&show, "opening", &cols(&["left", "right"]), door, "catpope").unwrap();
        assert_eq!((picked.said.as_str(), picked.row.as_deref()), ("row 1 opening: q2 asks catpope ch 3 ch3.door (version 812)", Some("opening")));
        assert_eq!(picked.show.rows[0].cells["left"], Cell::Ask { question: "q2".into() });
        assert_eq!(picked.show.rows[0].cells["right"], Cell::Ask { question: "q2".into() });
        let q2 = picked.show.question("q2").unwrap();
        let labels = |pairs: &[(&str, &str)]| pairs.iter().map(|(l, t)| (l.to_string(), t.to_string())).collect::<BTreeMap<_, _>>();
        assert_eq!(
            q2.story,
            Some(Snapshot { toon: "catpope".into(), chapter: 3, version: 812, event: "ch3.door".into(), options: vec![labels(&[("de", "Bleiben"), ("en", "Stay")]), labels(&[("de", "Gehen"), ("en", "Go")])], ids: vec!["stay".into(), "".into()] }),
            "#552: the options' ids with their labels ('' = none)"
        );
        assert_eq!((q2.typed.is_none(), q2.rule, q2.n, q2.seconds, q2.fallback, q2.show), (true, protocol::VoteRule::Majority, None, 30, 0, true));
        assert_eq!(q2.label("en"), "story: ch3.door");
        assert!(crate::validate::validate(&picked.show).is_empty(), "{:?}", crate::validate::validate(&picked.show));
        // the picker says which question asks it
        assert!(choice_lines(&picked.show, "catpope", &story.choices, "", "en")[0].detail.ends_with(" · asked by q2"));
        assert!(!choice_lines(&picked.show, "toptoy", &story.choices, "", "en")[0].detail.contains("asked by"), "another toon's");

        // the question the row asks already takes it: how its vote runs is kept, its typed words go; its fallback beyond the
        // options goes to the first
        let q1 = pick(&show, "crossroads", &[], door, "catpope").unwrap().show;
        let q = q1.question("q1").unwrap();
        assert_eq!((q.typed.is_none(), q.story.as_ref().map(|s| s.event.as_str()), q.seconds, q.fallback), (true, Some("ch3.door"), 30, 0));

        // refused with why: no id, no options, no toon, no cells, a cell validate refuses
        let two = cols(&["left", "middle"]);
        assert_eq!(pick(&show, "opening", &two, &story.choices[1], "catpope").unwrap_err(), "not a question for a vote: no id – the chapter editor gives the question one");
        assert_eq!(pick(&show, "opening", &two, &story.choices[2], "catpope").unwrap_err(), "not a question for a vote: no options – the chapter adds choices before it");
        assert_eq!(pick(&show, "opening", &two, door, " ").unwrap_err(), no_toon());
        assert_eq!(pick(&show, "opening", &[], door, "catpope").unwrap_err(), "row 1 opening: pick the cells that ask it");
        assert_eq!(pick(&show, "nowhere", &two, door, "catpope").unwrap_err(), "no row 'nowhere'");
        assert!(pick(&show, "opening", &cols(&["lamps"]), door, "catpope").unwrap_err().starts_with("not changed: "), "a device outlet does not ask");

        // held against the list: unchanged – nothing; a newer version with the same labels – nothing
        assert!(changed(&q1, "catpope", &story.choices).is_empty());
        let mut newer = story.choices.clone();
        newer[0].version = 815;
        assert!(changed(&q1, "catpope", &newer).is_empty(), "the same choice at a newer version");
        // #552: an option's id changed (or a snapshot from before #552 has none): pick it again
        let mut renamed = newer.clone();
        renamed[0].options[1].id = Some("go".into());
        assert_eq!(changed(&q1, "catpope", &renamed)[0].text, "question 'q1': ch3.door changed since it was picked (version 812 → 815) – pick it again");
        let mut old = q1.clone();
        old.questions.iter_mut().find(|q| q.id == "q1").and_then(|q| q.story.as_mut()).unwrap().ids.clear();
        assert_eq!(changed(&old, "catpope", &newer).len(), 1, "a snapshot without ids");
        // a label changed: a warning at the row that asks it
        newer[0].options[1].label.insert("en".into(), "Leave".into());
        let w = changed(&q1, "catpope", &newer);
        assert_eq!(w, vec![ProblemView { text: "question 'q1': ch3.door changed since it was picked (version 812 → 815) – pick it again".into(), error: false, points: Points::Row(1) }]);
        // an option fewer: changed too; asked by no row: points at the show
        newer[0].options.pop();
        assert_eq!(changed(&picked.show, "catpope", &newer)[0].points, Points::Row(0));
        let mut loose = picked.show.clone();
        loose.rows[0].cells.clear();
        assert_eq!(changed(&loose, "catpope", &newer)[0].points, Points::Show);
        // gone from the published chapter
        assert_eq!(changed(&q1, "catpope", &story.choices[1..]).iter().map(|p| p.text.as_str()).collect::<Vec<_>>(), ["question 'q1': ch 3 of catpope has no published question ch3.door any more"]);
        // another toon's list says nothing of catpope's questions
        assert!(changed(&q1, "toptoy", &[]).is_empty());
    }

    #[test]
    fn the_picker_says_why_there_is_no_list() {
        assert!(refused(Some(403), "").contains("creator"));
        assert!(refused(Some(401), "").contains("SCUTOONS_TOKEN"));
        assert_eq!(refused(Some(500), ""), "the story events: HTTP 500");
        assert_eq!(refused(None, "offline"), "the story events: offline");
        assert!(no_toon().contains("--toon"));
    }
}
