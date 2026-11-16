//! Rows after Ableton's play / stop: a clip cell plays, the empty cell (`black`) is a stop slot – launched, or its row
//! fired, the column goes black; a cell left out keeps (↑ continues) what an earlier row started. A row's firing is auto,
//! suggest or manual; manual (once `hand`) shows no chip.
use deck_show::conductor::Playing;
use deck_show::document::How;
use deck_show::roles::{edit, Edit};
use deck_show::{parse, view, ClockChoice, Subtree};

const DOC: &str = r#"{ "id": "slots", "version": 1, "outlets": [{ "id": "left" }, { "id": "right" }, { "id": "lights", "kind": "device" }],
  "rows": [
    { "id": "opening", "cells": { "left": { "mode": "play", "piece": "scene:nave" }, "right": { "mode": "play", "piece": "scene:vigil" } } },
    { "id": "dark-left", "cells": { "left": { "mode": "black" } } },
    { "id": "old", "fire": { "event": "door", "how": "manual" } },
    { "id": "meet", "fire": { "event": "meet", "how": "suggest", "moment": "bar" } },
    { "id": "start", "fire": { "event": "start", "how": "auto" } }
  ] }"#;

fn playing(sub: &Subtree, column: &str) -> Option<Playing> {
    sub.conductor().on_stage(column).map(|s| s.playing.clone())
}

#[test]
fn an_empty_cell_is_a_stop_slot_and_a_left_out_cell_keeps() {
    let mut sub = Subtree::new(parse(DOC).unwrap(), ClockChoice::Parent);
    sub.fire("opening", 1);
    let m = view::matrix(&sub);
    assert!(m.rows[1].cells[0].empty && m.rows[1].cells[0].text == "stop", "{:?}", m.rows[1].cells[0]);
    assert!(m.rows[1].cells[1].keep && !m.rows[1].cells[1].empty, "left out: keep, not empty");
    // its row fired: the stop slot stops left, right continues
    sub.fire("dark-left", 2);
    assert_eq!(playing(&sub, "left"), Some(Playing::Black));
    assert!(matches!(playing(&sub, "right"), Some(Playing::Piece { piece, .. }) if piece == "scene:vigil"));
    // launched by hand: the same stop
    sub.fire("opening", 3);
    sub.launch_cell("dark-left", "left", 4);
    assert_eq!(playing(&sub, "left"), Some(Playing::Black));
    assert!(!view::matrix(&sub).columns[0].stops, "nothing left to stop on left");
}

#[test]
fn hand_is_read_as_manual_and_saved_as_manual() {
    let show = parse(DOC).unwrap();
    assert_eq!(show.rows[2].fire.how, How::Manual);
    assert_eq!(show.rows[0].fire.how, How::Manual, "no fire written: manual");
    let saved = serde_json::to_string(&show).unwrap();
    assert!(saved.contains(r#""how":"manual""#) && !saved.contains(r#""hand""#), "{saved}");
    assert_eq!(parse(&saved).unwrap(), show);
}

#[test]
fn a_rows_chip_says_auto_or_suggest_with_its_moment_and_manual_has_none() {
    let mut sub = Subtree::new(parse(DOC).unwrap(), ClockChoice::Parent);
    let m = view::plan(sub.conductor().show());
    assert_eq!((m.rows[0].chip.clone(), m.rows[2].chip.clone()), (None, None), "manual shows nothing, with an event or without");
    let suggest = m.rows[3].chip.clone().unwrap();
    assert!(!suggest.auto && suggest.event == "meet" && suggest.moment == deck_show::document::Moment::Bar && !suggest.waiting);
    let auto = m.rows[4].chip.clone().unwrap();
    assert!(auto.auto && auto.event == "start" && !auto.waiting);
    // the reader's event: the suggestion stands, its chip waits for GO
    let story = serde_json::json!({ "id": "meet" }).as_object().cloned().unwrap();
    sub.on_wire(&protocol::Envelope::new("reader", protocol::Message::Story(story)), 5);
    assert!(view::matrix(&sub).rows[3].chip.as_ref().unwrap().waiting);
}

#[test]
fn columns_are_grouped_when_adjacent_named_and_renamed_through_the_core() {
    let show = parse(DOC).unwrap();
    let ids = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let g = edit(&show, &Edit::Group { ids: ids(&["left", "right"]), name: String::new() }).unwrap();
    assert_eq!(g.said, "left · right grouped as group");
    assert_eq!(view::plan(&g.show).groups.iter().map(|g| (g.name.as_str(), g.first, g.span)).collect::<Vec<_>>(), [("group", 0, 2)]);
    let named = edit(&g.show, &Edit::GroupName { from: "group".into(), to: "stages".into() }).unwrap();
    assert_eq!(view::plan(&named.show).columns[1].group.as_deref(), Some("stages"));
    // left and lights stand apart: the core refuses the split group in its words
    let split = edit(&show, &Edit::Group { ids: ids(&["left", "lights"]), name: "far".into() }).unwrap_err();
    assert_eq!(split, "not changed: show: outlet 'lights': the group 'far' is split, its columns stand apart");
    let out = edit(&named.show, &Edit::Ungroup { ids: ids(&["right"]) }).unwrap();
    assert_eq!(view::plan(&out.show).groups.iter().map(|g| g.span).collect::<Vec<_>>(), [1]);
    // a device column is named in place too
    let lamp = edit(&show, &Edit::Label { id: "lights".into(), label: "lamps".into() }).unwrap();
    assert_eq!((lamp.said.as_str(), view::plan(&lamp.show).columns[2].name.as_str()), ("column lights is called lamps", "lamps"));
}
