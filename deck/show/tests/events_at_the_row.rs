//! Lap 2 P7 (#871): a row fires on several events, several rows hear one, a row's fire is an event, a loop of rows stops
//! after one pass – and a show that uses any of it is one an older deck refuses instead of misreading.
use deck_show::{parse, Conductor, Logged, Now, Origin, Show};
use protocol::Message;
use serde_json::json;

fn show(rows: serde_json::Value) -> Show {
    let mut doc = json!({ "id": "loop", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }], "rows": rows });
    let mut s: Show = serde_json::from_value(doc.take()).unwrap();
    s.stamp();
    parse(&serde_json::to_string(&s).unwrap()).unwrap()
}

fn story(id: &str) -> Message {
    Message::Story(json!({ "type": "showMessage", "id": id }).as_object().unwrap().clone())
}

fn fired(c: &Conductor) -> Vec<(String, Origin)> {
    c.log().filter_map(|e| match &e.what {
        Logged::Fired { row, origin } => Some((row.clone(), *origin)),
        _ => None,
    })
    .collect()
}

fn names(f: &[(String, Origin)]) -> Vec<&str> {
    f.iter().map(|(r, _)| r.as_str()).collect()
}

#[test]
fn a_row_fires_on_any_of_its_events_and_one_event_fires_every_row_that_hears_it() {
    let s = show(json!([
        { "id": "a", "fire": { "event": ["ch3.door", "ch3.bell"], "how": "auto" }, "cells": { "left": { "mode": "play", "piece": "scene:nave" } } },
        { "id": "b", "fire": { "event": "ch3.bell", "how": "auto" } },
        { "id": "c", "fire": { "event": "ch3.bell", "how": "manual" } }
    ]));
    let mut c = Conductor::new(s);
    c.on_message(&story("ch3.door"), Now::at(10));
    assert_eq!(names(&fired(&c)), ["a"]);
    c.on_message(&story("ch3.bell"), Now::at(20));
    assert_eq!(names(&fired(&c)), ["a", "a", "b"], "both rows on auto, in the show's order");
    assert!(c.log().any(|e| e.what == Logged::Ignored { row: "c".into(), event: "ch3.bell".into() }), "the manual one ignores it");
}

#[test]
fn a_row_fired_by_a_row_fires_in_the_same_pass() {
    let s = show(json!([
        { "id": "a", "fire": { "event": "ch3.door", "how": "auto" } },
        { "id": "b", "fire": { "event": "row:a", "how": "auto" } }
    ]));
    let mut c = Conductor::new(s);
    c.fire("a", Now::at(5));
    assert_eq!(fired(&c), [("a".to_string(), Origin::Hand), ("b".to_string(), Origin::Auto)]);
    assert_eq!(c.current(), Some("b"));
}

/// A fires B fires A: one fire per row per pass – the loop stops at A and says so; the next pass runs once more, no more
#[test]
fn a_loop_of_rows_stops_after_one_pass() {
    let s = show(json!([
        { "id": "a", "fire": { "event": ["ch3.door", "row:b"], "how": "auto" } },
        { "id": "b", "fire": { "event": "row:a", "how": "auto" } }
    ]));
    let mut c = Conductor::new(s);
    c.on_message(&story("ch3.door"), Now::at(10));
    assert_eq!(names(&fired(&c)), ["a", "b"]);
    assert!(c.log().any(|e| e.what == Logged::Looped { row: "a".into() }));
    c.fire("b", Now::at(20));
    assert_eq!(names(&fired(&c)), ["a", "b", "b", "a"], "by hand: a new pass, again once each");
}

/// the pass carries over a hold: a row fired by a row after its hold time still counts the rows its pass fired
#[test]
fn a_loop_with_a_hold_stops_too() {
    let s = show(json!([
        { "id": "a", "fire": { "event": "row:b", "how": "auto" } },
        { "id": "b", "fire": { "event": "row:a", "how": "auto", "hold_ms": 500 } }
    ]));
    let mut c = Conductor::new(s);
    c.fire("a", Now::at(0));
    c.tick(Now::at(600));
    c.tick(Now::at(1_200));
    assert_eq!(names(&fired(&c)), ["a", "b"]);
    assert!(c.log().any(|e| e.what == Logged::Looped { row: "a".into() }));
}

/// one event: written as before, a string; several or a row's fire: a list or `row:` and the reader they need
#[test]
fn one_event_is_written_as_before_and_what_an_older_deck_misreads_is_stamped() {
    let one = show(json!([{ "id": "a", "fire": { "event": "ch3.door", "how": "auto" } }]));
    let text = serde_json::to_value(&one).unwrap();
    assert_eq!(text["rows"][0]["fire"]["event"], json!("ch3.door"));
    assert!(text.get("reads").is_none(), "nothing new: every deck reads it");
    let two = show(json!([{ "id": "a", "fire": { "event": ["ch3.door", "ch3.bell"], "how": "auto" } }]));
    let text = serde_json::to_value(&two).unwrap();
    assert_eq!((text["rows"][0]["fire"]["event"].clone(), text["reads"].clone()), (json!(["ch3.door", "ch3.bell"]), json!(2)));
    let by_row = show(json!([{ "id": "a" }, { "id": "b", "fire": { "event": "row:a" } }]));
    assert_eq!(by_row.reads, Some(2));
}

/// The old-deck guard. An older deck reads `fire.event` as one string, its documents refuse unknown fields: the stamped
/// show fails its read there – refused, never misread. This deck refuses a show unstamped, or stamped for a newer reader.
#[test]
fn an_older_deck_refuses_what_it_would_misread_and_this_deck_refuses_a_newer_show() {
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct OldShow {
        id: String,
        version: u32,
        toons: Vec<String>,
        outlets: Vec<serde_json::Value>,
        rows: Vec<OldRow>,
        // the rest of the fields an older deck knew, as they come
        #[serde(default)]
        face: serde_json::Value,
        #[serde(default)]
        seats: serde_json::Value,
        #[serde(default)]
        fixtures: serde_json::Value,
        #[serde(default)]
        looks: serde_json::Value,
        #[serde(default)]
        questions: serde_json::Value,
        #[serde(default)]
        background: serde_json::Value,
        #[serde(default)]
        tempo: serde_json::Value,
    }
    #[derive(Debug, serde::Deserialize)]
    #[allow(dead_code)]
    struct OldRow {
        id: String,
        #[serde(default)]
        fire: OldFire,
    }
    #[derive(Debug, serde::Deserialize, Default)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct OldFire {
        #[serde(default)]
        event: Option<String>,
        #[serde(default)]
        how: Option<String>,
        #[serde(default)]
        hold_ms: u32,
        #[serde(default)]
        moment: Option<String>,
    }
    let one = serde_json::to_string(&show(json!([{ "id": "a", "fire": { "event": "ch3.door" } }]))).unwrap();
    assert!(serde_json::from_str::<OldShow>(&one).is_ok(), "one event per row: the old deck reads it as before");
    let by_row = serde_json::to_string(&show(json!([{ "id": "a" }, { "id": "b", "fire": { "event": "row:a" } }]))).unwrap();
    assert!(serde_json::from_str::<OldShow>(&by_row).unwrap_err().to_string().contains("reads"), "a row's fire: refused by the stamp");
    let two = serde_json::to_string(&show(json!([{ "id": "a", "fire": { "event": ["x", "y"] } }]))).unwrap();
    assert!(serde_json::from_str::<OldShow>(&two).is_err());

    let unstamped = r#"{ "id": "s", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }], "rows": [{ "id": "a", "fire": { "event": ["x", "y"] } }] }"#;
    assert!(parse(unstamped).unwrap_err().to_string().contains("\"reads\": 2"));
    let newer = r#"{ "id": "s", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }], "rows": [], "reads": 9 }"#;
    assert!(parse(newer).unwrap_err().to_string().contains("it needs a newer deck"));
}

#[test]
fn a_row_cannot_fire_on_its_own_fire_or_a_row_that_is_not_there() {
    let own = r#"{ "id": "s", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }], "reads": 2, "rows": [{ "id": "a", "fire": { "event": "row:a" } }] }"#;
    assert!(parse(own).unwrap_err().to_string().contains("its own fire"));
    let gone = r#"{ "id": "s", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }], "reads": 2, "rows": [{ "id": "a", "fire": { "event": "row:nowhere" } }] }"#;
    assert!(parse(gone).unwrap_err().to_string().contains("nowhere"));
    let twice = r#"{ "id": "s", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }], "reads": 2, "rows": [{ "id": "a", "fire": { "event": ["x", "x"] } }] }"#;
    assert!(parse(twice).unwrap_err().to_string().contains("named twice"));
}

/// renaming a row renames the fires that name it; removing it takes them away
#[test]
fn a_rows_fire_follows_its_rename_and_goes_with_it() {
    use deck_show::rows::{edit, Edit};
    let s = show(json!([{ "id": "a" }, { "id": "b", "fire": { "event": ["row:a", "ch3.door"] } }]));
    let renamed = edit(&s, &Edit::Rename { id: "a".into(), to: "intro".into() }).unwrap().show;
    assert_eq!(renamed.rows[1].fire.event.as_slice(), ["row:intro", "ch3.door"]);
    let gone = edit(&renamed, &Edit::Remove { id: "intro".into() }).unwrap().show;
    assert_eq!((gone.rows[0].fire.event.as_slice(), gone.reads), (&["ch3.door".to_string()][..], None));
}
