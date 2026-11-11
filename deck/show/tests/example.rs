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

/// #478: a stage outlet is a role with its filter – who may take it; every fact is a place the one parser reads, a filter
/// names somebody, a device outlet is no role. No filter: a guest, anyone with the code.
#[test]
fn a_roles_filter_is_checked() {
    let (mut doc, _) = example();
    doc["outlets"][0]["label"] = json!("Tenor");
    doc["outlets"][0]["for"] = json!({ "account": ["guest"], "device": ["phone", "browser"], "group": ["mezzo-soprano"], "approved": true });
    doc["outlets"][1]["for"] = json!({ "account": ["live"] });
    let show = parse(&doc.to_string()).unwrap();
    assert_eq!((show.outlets[0].name(), show.outlets[2].name()), ("Tenor", "lamps"));
    assert!(show.outlets[2].admits.is_none(), "no filter: anyone with the code");
    assert_eq!(parse(&serde_json::to_string(&show).unwrap()).unwrap(), show, "`for` survives a round trip");

    doc["outlets"][0]["for"] = json!({ "device": ["phone", "phone", "big screen"], "group": ["a:b"] });
    doc["outlets"][1]["for"] = json!({});
    doc["outlets"][2]["for"] = json!({ "device": ["native"] });
    let found: Vec<String> = problems(doc.clone()).iter().map(ToString::to_string).collect();
    assert_eq!(found.len(), 5, "{found:?}");
    assert!(found[0].contains("'device:phone' twice"), "{found:?}");
    assert!(found[1].contains("' ' in"), "{found:?}");
    assert!(found[2].contains("':' in"), "{found:?}");
    assert!(found[3].contains("'right': `for` names nobody"), "{found:?}");
    assert!(found[4].contains("'lamps': a device outlet is no role"), "{found:?}");
    doc["outlets"][0]["for"] = json!({ "account": ["owner"] });
    assert!(matches!(parse(&doc.to_string()), Err(ParseError::Shape(_))), "an account fact is one of the server's roles");
}

/// #478: the conductor tells its session the roles it gives out (when it joins and with a new show) and says who is here as
/// `role: name` from presence
#[test]
fn the_conductor_says_its_roles_and_who_is_here() {
    use deck_show::{ClockChoice, Subtree};
    use protocol::{Envelope, Message, Present, Presence, Welcome};
    let (mut doc, _) = example();
    doc["outlets"][0]["label"] = json!("tenor");
    doc["outlets"][1]["for"] = json!({ "account": ["live"] });
    let show = parse(&doc.to_string()).unwrap();
    let mut sub = Subtree::new(show.clone(), ClockChoice::Parent);
    let welcome = Message::Welcome(Welcome { client: "c1".into(), group: None, snapshot: Default::default(), now: 0, key: None });
    let out = sub.on_wire(&Envelope::new("relay", welcome), 0);
    let Some(Message::Roles(r)) = out.first().map(|e| &e.message) else { panic!("{out:?}") };
    let ids: Vec<(&str, Option<&str>)> = r.roles.iter().map(|x| (x.id.as_str(), x.label.as_deref())).collect();
    assert_eq!(ids, [("left", Some("tenor")), ("right", Some("right"))], "the stage outlets, not the lamps");
    assert!(r.roles[1].r#for.is_some());
    assert!(sub.install(show, 1).iter().any(|e| matches!(e.message, Message::Roles(_))), "a new show says them again");

    let present = |group: Option<&str>, nick: Option<&str>, locked: bool| Present { group: group.map(Into::into), nick: nick.map(Into::into), device: None, account: None, locked: locked.then_some(true) };
    let who = [("c3", present(Some("left"), Some("Anna"), false)), ("c4", present(Some("right"), None, true)), ("c5", present(None, Some("phone-7"), false))];
    let presence = Presence { groups: Default::default(), ready: vec![], decks: None, routes: None, links: None, quality: None, seats: None, parent: None, who: Some(who.into_iter().map(|(k, v)| (k.to_string(), v)).collect()) };
    sub.on_wire(&Envelope::new("relay", Message::Presence(presence)), 2);
    assert_eq!(sub.here(), ["tenor: Anna", "right: c4 (locked)", "no role: phone-7"]);
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
