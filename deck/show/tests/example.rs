//! The hand-written example show: it loads and validates, and every kind of mistake the core must refuse is refused.
use deck_show::{parse, resolve, ParseError, Pinned, Problem, Show};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const TOP_TOY: &str = include_str!("../fixtures/top-toy.show.json");
const FOYER: &str = include_str!("../fixtures/foyer-loop.show.json");

fn library(shows: &[&Show]) -> BTreeMap<Pinned, Show> {
    shows.iter().map(|s| (Pinned { id: s.id.clone(), version: s.version }, (*s).clone())).collect()
}

fn example() -> (Value, Value) {
    (serde_json::from_str(TOP_TOY).unwrap(), serde_json::from_str(FOYER).unwrap())
}

/// the document's problems, for a changed example
fn problems(doc: Value) -> Vec<Problem> {
    match parse(&doc.to_string()) {
        Err(ParseError::Invalid(p)) => p,
        other => panic!("expected problems, got {other:?}"),
    }
}

/// the tree's problems, for changed documents
fn tree_problems(root: Value, child: Value) -> Vec<Problem> {
    let (root, child) = (parse(&root.to_string()).unwrap(), parse(&child.to_string()).unwrap());
    resolve(&root, &library(&[&root, &child])).unwrap_err()
}

#[test]
fn the_example_loads_validates_and_locks_to_exact_versions() {
    let (root, child) = (parse(TOP_TOY).unwrap(), parse(FOYER).unwrap());
    let lock = resolve(&root, &library(&[&root, &child])).unwrap();
    assert_eq!(lock.shows, vec![Pinned { id: "top-toy".into(), version: 1 }, Pinned { id: "foyer-loop".into(), version: 2 }]);
    let seat = root.seat("foyer").unwrap();
    assert!(seat.lockable);
    assert_eq!(seat.sits.as_deref(), Some("deck:foyer"));
}

#[test]
fn a_document_survives_a_round_trip() {
    let root = parse(TOP_TOY).unwrap();
    let again = parse(&serde_json::to_string(&root).unwrap()).unwrap();
    assert_eq!(root, again);
}

#[test]
fn unknown_fields_are_refused() {
    let (mut doc, _) = example();
    doc["rows"][0]["cells"]["left"]["pice"] = json!("scene:typo");
    assert!(matches!(parse(&doc.to_string()), Err(ParseError::Shape(_))));
}

#[test]
fn unknown_references_are_refused() {
    let (mut doc, _) = example();
    doc["rows"][0]["cells"]["nowhere"] = json!({ "mode": "black" });
    assert!(problems(doc.clone()).iter().any(|p| matches!(p, Problem::Unknown { what: "outlet or seat", id, .. } if id == "nowhere")));

    let (mut doc, _) = example();
    doc["rows"][0]["cells"]["right"] = json!({ "mode": "mirror", "of": "gone" });
    assert!(problems(doc).iter().any(|p| matches!(p, Problem::Unknown { what: "outlet", id, .. } if id == "gone")));

    let (mut doc, _) = example();
    doc["rows"][0]["cells"]["foyer"] = json!({ "mode": "go", "row": "not-used" });
    assert!(problems(doc).iter().any(|p| matches!(p, Problem::Unknown { id, .. } if id == "not-used")));

    let (mut doc, child) = example();
    doc["seats"][0]["show"]["version"] = json!(99);
    let problems = tree_problems(doc, child);
    assert!(problems.iter().any(|p| matches!(p, Problem::Unknown { what: "show", id, .. } if id == "foyer-loop@99")));
}

#[test]
fn cells_refused_on_the_wrong_column() {
    let (mut doc, _) = example();
    doc["rows"][0]["cells"]["left"] = json!({ "mode": "go", "row": "idle" });
    assert!(problems(doc).iter().any(|p| matches!(p, Problem::Refused { why, .. } if why.contains("seat's column"))));

    let (mut doc, _) = example();
    doc["rows"][0]["cells"]["foyer"] = json!({ "mode": "black" });
    assert!(problems(doc).iter().any(|p| matches!(p, Problem::Refused { why, .. } if why.contains("a seat takes"))));

    let (mut doc, _) = example();
    doc["rows"][0]["cells"]["lamps"] = json!({ "mode": "black" });
    assert!(problems(doc).iter().any(|p| matches!(p, Problem::Refused { why, .. } if why.contains("device outlet"))));

    let (mut doc, _) = example();
    doc["rows"][0]["cells"]["left"] = json!({ "mode": "play", "piece": "no-kind" });
    assert!(problems(doc).iter().any(|p| matches!(p, Problem::Refused { why, .. } if why.contains("not a place"))));
}

#[test]
fn duplicates_a_shared_event_and_mirror_circles_are_refused() {
    let (mut doc, _) = example();
    doc["outlets"][1]["id"] = json!("left");
    assert!(problems(doc).iter().any(|p| matches!(p, Problem::Duplicate { what: "outlet", .. })));

    let (mut doc, _) = example();
    doc["rows"][1]["fire"]["event"] = json!("ch3.end");
    assert!(problems(doc).iter().any(|p| matches!(p, Problem::Refused { why, .. } if why.contains("already fires"))));

    let (mut doc, _) = example();
    doc["rows"][0]["cells"]["left"] = json!({ "mode": "mirror", "of": "right" });
    let found = problems(doc);
    assert_eq!(found.iter().filter(|p| matches!(p, Problem::Refused { why, .. } if why.contains("circle"))).count(), 1, "{found:?}");
}

#[test]
fn a_used_row_the_face_lacks_is_refused() {
    let (mut doc, child) = example();
    doc["seats"][0]["uses"]["rows"] = json!(["idle", "storm", "backstage"]);
    let problems = tree_problems(doc, child);
    assert_eq!(problems, vec![Problem::FaceLacks { seat: "foyer".into(), what: "row", id: "backstage".into() }]);
}

#[test]
fn a_seat_cycle_is_refused() {
    let (root, mut child) = example();
    // the child seats the root: top-toy › foyer-loop › top-toy
    child["seats"] = json!([{ "id": "back", "show": { "id": "top-toy", "version": 1 }, "word": "free" }]);
    let problems = tree_problems(root, child);
    assert!(problems.iter().any(|p| matches!(p, Problem::Cycle { path } if path == &["top-toy", "foyer-loop", "top-toy"])), "{problems:?}");
}

#[test]
fn a_rows_addresses_are_read_by_the_one_parser() {
    let (mut doc, _) = example();
    doc["rows"][1]["addresses"] = json!([
        { "address": "yellowmouse", "action": "dance" },
        { "address": "location:nave.west", "action": "close_doors" },
        { "address": "character:catpope", "action": "dance samba" },
        { "address": "character:*", "action": "wave", "where": { "group": "" } }
    ]);
    let found = problems(doc);
    let at_dialog: Vec<String> = found.iter().filter(|p| p.spot() == deck_show::validate::Spot::Row("dialog".into())).map(ToString::to_string).collect();
    assert_eq!(at_dialog.len(), 4, "{found:?}");
    assert!(at_dialog[0].contains("not an address"), "{at_dialog:?}");
    assert!(at_dialog[1].contains("characters only"));
    assert!(at_dialog[2].starts_with("row 'dialog': character:catpope: "));
    assert!(at_dialog[3].contains("an empty filter"));
}

#[test]
fn a_problem_says_where_it_is() {
    let (mut doc, _) = example();
    doc["rows"][0]["cells"]["foyer"] = json!({ "mode": "go", "row": "nowhere" });
    let found = problems(doc);
    assert_eq!(found[0].spot(), deck_show::validate::Spot::Cell { row: "opening".into(), column: "foyer".into() });
    assert_eq!(found[0].to_string(), "row 'opening', 'foyer': no row in the seat's `uses` 'nowhere'");
}

#[test]
fn fixtures_and_looks_are_checked() {
    let (mut doc, _) = example();
    doc["fixtures"][1]["address"] = json!(3);
    doc["fixtures"].as_array_mut().unwrap().push(json!({ "id": "far", "outlet": "left", "address": 510, "profile": "rgbw" }));
    doc["looks"][0]["values"]["panel"]["w"] = json!(9);
    doc["looks"][0]["values"]["ghost"] = json!({ "dim": 1 });
    doc["rows"][0]["cells"]["left"] = json!({ "mode": "look", "look": "warm" });
    doc["rows"][3]["cells"]["lamps"] = json!({ "mode": "look", "look": "cold" });
    let found: Vec<String> = problems(doc).iter().map(ToString::to_string).collect();
    let has = |s: &str| found.iter().any(|p| p.contains(s));
    assert!(has("a look is for a device outlet's lamps"), "{found:?}");
    assert!(has("no look 'cold'"), "{found:?}");
    assert!(has("fixture 'panel': channel 3 is already 'strobe's"), "{found:?}");
    assert!(has("fixture 'far': 'left' is no device outlet"), "{found:?}");
    assert!(has("fixture 'far': channels 510–513 are outside 1–512"), "{found:?}");
    assert!(has("fixture 'panel' has no channel 'w'"), "{found:?}");
    assert!(has("no fixture 'ghost'"), "{found:?}");
}
