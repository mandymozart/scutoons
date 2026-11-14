//! #546: a question as an `ask` cell (research/SHOW.md §15 1, 2): one question over two outlets that are not adjacent, with
//! the column filter, parses; every way it can be wrong says why; the old documents still parse.
use deck_show::view::{self, AskView};
use deck_show::{parse, Cell, ParseError, Problem, Show};
use protocol::filter::{passes, Facts};
use protocol::Account;
use serde_json::{json, Value};

const VOTE: &str = include_str!("../fixtures/vote-cell.show.json");

fn doc() -> Value {
    serde_json::from_str(VOTE).unwrap()
}

/// the problems' words, for a changed document
fn why(doc: Value) -> Vec<String> {
    match parse(&doc.to_string()) {
        Err(ParseError::Invalid(p)) => p.iter().map(Problem::to_string).collect(),
        other => panic!("expected problems, got {other:?}"),
    }
}

#[test]
fn one_question_over_two_outlets_apart_with_a_filter_parses() {
    let show = parse(VOTE).unwrap();
    let q = show.question("q1").unwrap();
    assert_eq!((q.label("en"), q.label("de"), q.options().len(), q.fallback), ("Which way?".to_string(), "Welcher Weg?".to_string(), 3, 2));
    let row = show.row("crossroads").unwrap();
    assert!(matches!(row.cells.get("left"), Some(Cell::Ask { question }) if question == "q1"));
    assert_eq!(row.r#where.get("left"), row.r#where.get("right"));
    // the filter's one rule: a member's phone gets it, a guest's or a browser deck does not
    let f = row.r#where.get("left");
    assert!(passes(f, &Facts { device: Some("phone"), account: &[Account::Member], ..Facts::default() }));
    assert!(!passes(f, &Facts { device: Some("phone"), ..Facts::default() }));
    assert!(!passes(f, &Facts { device: Some("browser"), account: &[Account::Live], ..Facts::default() }));
    // saved and read again: the same document
    assert_eq!(parse(&serde_json::to_string(&show).unwrap()).unwrap(), show);
}

#[test]
fn the_matrix_marks_the_cells_of_a_question_and_the_inspector_says_to_whom() {
    let show: Show = parse(VOTE).unwrap();
    let m = view::plan(&show);
    let row = &m.rows[1];
    assert_eq!(row.cells[0].text, "ask q1");
    let left = row.cells[0].ask.clone().unwrap();
    assert_eq!(left, AskView { question: "q1".into(), label: "Which way?".into(), nth: 1, of: 2, first: true, run: 1, filter: "device phone · member · live".into() });
    assert_eq!(row.cells[2].ask.as_ref().map(AskView::mark).as_deref(), Some("q1 · 2 of 2"), "apart: the same mark");
    assert!(row.cells[1].ask.is_none() && row.cells[3].ask.is_none());
    let asks = view::inspector(&show, "crossroads").into_iter().find(|(h, _)| h == "this row asks").unwrap().1;
    assert_eq!(asks[0], ("left".to_string(), "q1: Which way? · device phone · member · live".to_string()));
    let cue = &view::cue_sheet(&show)[1];
    assert!(cue.does.contains(&("right".to_string(), "asks q1, the piece plays on".to_string())), "{:?}", cue.does);
    // adjacent: one wide cell from the first
    let mut d = doc();
    d["rows"][1]["cells"]["middle"] = json!({ "mode": "ask", "question": "q1" });
    d["rows"][1]["where"]["middle"] = d["rows"][1]["where"]["left"].clone();
    let wide = view::plan(&parse(&d.to_string()).unwrap());
    let runs: Vec<(bool, usize)> = wide.rows[1].cells[..3].iter().map(|c| c.ask.as_ref().map(|a| (a.first, a.run)).unwrap()).collect();
    assert_eq!(runs, [(true, 3), (false, 0), (false, 0)]);
    // #551: what each cell drawn says – one wide cell the question; cells apart their mark; a run among others its places
    let words = |m: &view::Matrix| -> Vec<Option<String>> { m.rows[1].cells[..3].iter().map(|c| c.ask.as_ref().and_then(AskView::words)).collect() };
    assert_eq!(words(&wide), [Some("q1 · Which way?".to_string()), None, None]);
    assert_eq!(words(&m), [Some("q1 · 1 of 2 · Which way?".to_string()), None, Some("q1 · 2 of 2 · Which way?".to_string())]);
    d["outlets"] = json!([{ "id": "left" }, { "id": "middle" }, { "id": "far" }, { "id": "right" }, { "id": "lamps", "kind": "device" }]);
    let run = view::plan(&parse(&d.to_string()).unwrap());
    let said: Vec<Option<String>> = run.rows[1].cells[..4].iter().map(|c| c.ask.as_ref().and_then(AskView::words)).collect();
    assert_eq!(said, [Some("q1 · 1–2 of 3 · Which way?".to_string()), None, None, Some("q1 · 3 of 3 · Which way?".to_string())]);
}

#[test]
fn a_question_that_is_wrong_says_why() {
    let mut d = doc();
    d["rows"][1]["cells"]["left"]["question"] = json!("q9");
    assert!(why(d).iter().any(|p| p == "row 'crossroads', 'left': no question 'q9'"));

    let mut d = doc();
    d["rows"][1]["cells"]["lamps"] = json!({ "mode": "ask", "question": "q1" });
    assert!(why(d).iter().any(|p| p == "row 'crossroads', 'lamps': a device outlet takes `play` with `send`, a `look`, or `keep`"), "ask only on a stage outlet");

    let mut d = doc();
    d["questions"].as_array_mut().unwrap().push(json!({ "id": "q2", "typed": { "options": [{ "en": "a" }, { "en": "b" }] }, "rule": "majority", "seconds": 10 }));
    d["rows"][1]["cells"]["middle"] = json!({ "mode": "ask", "question": "q2" });
    assert!(why(d).contains(&"row 'crossroads': one question per row, it asks 'q1' and 'q2'".to_string()));

    let mut d = doc();
    d["rows"][1]["where"]["right"] = json!({ "device": "browser" });
    assert!(why(d).contains(&"row 'crossroads': the columns of question 'q1' (left, right) carry different filters".to_string()));

    let mut d = doc();
    d["rows"][1]["cells"]["right"] = json!({ "mode": "black" });
    d["rows"][1]["cells"]["left"] = json!({ "mode": "black" });
    assert!(why(d).contains(&"show: question 'q1' is asked in no row".to_string()));

    let mut d = doc();
    d["questions"][0]["typed"]["options"] = json!([{ "en": "only" }]);
    d["questions"][0]["fallback"] = json!(0);
    assert!(why(d).contains(&"show: question 'q1': A vote needs at least two options (separated by commas).".to_string()), "the form's checks");

    let mut d = doc();
    d["questions"][0]["rule"] = json!("first");
    assert!(why(d).contains(&"show: question 'q1': First to n: n is at least 1.".to_string()));

    let mut d = doc();
    d["questions"][0]["typed"]["options"][1] = json!({ "en": " " });
    assert!(why(d).contains(&"show: question 'q1': option 2 has no label".to_string()));

    let mut d = doc();
    d["questions"][0]["story"] = json!({ "toon": "catpope", "chapter": 3, "version": 2, "event": "c1", "options": [{ "en": "a" }, { "en": "b" }] });
    assert!(why(d).contains(&"show: question 'q1': typed or the story's, not both".to_string()));

    let mut d = doc();
    let twin = d["questions"][0].clone();
    d["questions"].as_array_mut().unwrap().push(twin);
    assert!(why(d).contains(&"two questions named 'q1'".to_string()));

    let mut d = doc();
    d["rows"][1]["where"]["lamps"] = json!({ "device": "phone" });
    d["rows"][1]["where"]["nowhere"] = json!({ "device": "phone" });
    d["rows"][1]["where"]["left"]["account"] = json!(["member", "member"]);
    d["rows"][1]["where"]["right"]["account"] = json!(["member", "member"]);
    let w = why(d);
    assert!(w.contains(&"row 'crossroads', 'lamps': a filter is for a stage outlet's column, a device outlet has no audience".to_string()), "{w:?}");
    assert!(w.contains(&"row 'crossroads', 'nowhere': no outlet 'nowhere'".to_string()), "{w:?}");
    assert!(w.contains(&"row 'crossroads', 'left': the filter of 'left': 'account:member' twice".to_string()), "{w:?}");

    // an unknown field of a question is no question
    let mut d = doc();
    d["questions"][0]["groups"] = json!(["left"]);
    assert!(matches!(parse(&d.to_string()), Err(ParseError::Shape(_))), "the columns are the groups");
}

#[test]
fn a_story_question_parses_and_says_its_event() {
    let mut d = doc();
    d["questions"][0] = json!({ "id": "q1", "story": { "toon": "catpope", "chapter": 3, "version": 2, "event": "crossroads-choice", "options": [{ "en": "Left" }, { "en": "Right" }] }, "rule": "groups", "seconds": 20 });
    let show = parse(&d.to_string()).unwrap();
    let q = show.question("q1").unwrap();
    assert_eq!((q.label("en"), q.fallback, q.show), ("story: crossroads-choice".to_string(), 0, true), "the form's defaults");
}

#[test]
fn the_old_documents_still_parse_unchanged() {
    for doc in [
        include_str!("../fixtures/top-toy.show.json"),
        include_str!("../fixtures/foyer-loop.show.json"),
        include_str!("../fixtures/first-row.show.json"),
        include_str!("../fixtures/stage-cast.show.json"),
    ] {
        let show = parse(doc).unwrap_or_else(|e| panic!("{e}"));
        assert!(show.questions.is_empty() && show.rows.iter().all(|r| r.r#where.is_empty()));
        let saved: Value = serde_json::to_value(&show).unwrap();
        assert!(saved.get("questions").is_none() && saved["rows"].as_array().unwrap().iter().all(|r| r.get("where").is_none()), "nothing new is written");
    }
}
