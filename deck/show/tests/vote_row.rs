//! #548 (research/SHOW.md §15 3): a row's question opens the vote – firing a row sends one `vote.open` for its `ask` columns
//! with their filter, a hand launch of one `ask` cell opens the whole question, a vote still open is closed by its rule
//! first; the result is the show's event `vote:<question>/<option>`, a row fires on it auto / suggest / hand, and it is
//! logged. The relay is simulated: what it would say back is handed to the conductor as `on_wire`.
use deck_show::{document, parse, view, ClockChoice, Logged, Origin, Subtree};
use protocol::{Envelope, Message, VoteHow, VoteOpen, VoteResult};
use serde_json::{json, Value};

const VOTE: &str = include_str!("../fixtures/vote-cell.show.json");

fn subtree(doc: &str) -> Subtree {
    Subtree::new(parse(doc).unwrap(), ClockChoice::Parent)
}

/// the fixture with a second question `q2`, asked by row `second` on `middle` (for everyone)
fn two_questions() -> String {
    let mut d: Value = serde_json::from_str(VOTE).unwrap();
    d["questions"].as_array_mut().unwrap().push(json!({ "id": "q2", "typed": { "options": [{ "en": "Up" }, { "en": "Down" }] }, "rule": "first", "n": 3, "seconds": 10 }));
    d["rows"].as_array_mut().unwrap().push(json!({ "id": "second", "cells": { "middle": { "mode": "ask", "question": "q2" } } }));
    d.to_string()
}

/// each envelope as `kind` and its payload
fn sent(out: &[Envelope]) -> Vec<(String, Value)> {
    out.iter().map(|e| serde_json::to_value(e).unwrap()).map(|v| (v["kind"].as_str().unwrap_or_default().to_string(), v["p"].clone())).collect()
}

fn opens(out: &[Envelope]) -> Vec<VoteOpen> {
    out.iter().filter_map(|e| if let Message::VoteOpen(v) = &e.message { Some(v.clone()) } else { None }).collect()
}

/// the relay says back what the conductor sent (it opens what it was asked to)
fn relay_says(sub: &mut Subtree, out: &[Envelope], ms: u64) -> Vec<Envelope> {
    out.iter().filter(|e| matches!(e.message, Message::VoteOpen(_))).flat_map(|e| sub.on_wire(&Envelope::new("relay", e.message.clone()), ms)).collect()
}

fn result(vote: &str, option: i64) -> Envelope {
    Envelope::new("relay", Message::VoteResult(VoteResult { vote: vote.into(), option, counts: vec![4, 1, 0], how: VoteHow::Rule, groups: None }))
}

fn lines(sub: &Subtree) -> Vec<String> {
    sub.log().map(|e| view::line(&e.what)).collect()
}

#[test]
fn firing_a_row_opens_the_vote_for_its_ask_columns_with_their_filter() {
    let mut sub = subtree(VOTE);
    sub.fire("opening", 0);
    let out = sub.fire("crossroads", 1_000);
    let kinds: Vec<(String, Option<String>)> = sent(&out).into_iter().map(|(k, p)| (k, p["outlet"].as_str().map(String::from))).collect();
    // the ask columns keep their piece (the scene plays on, no cell for them); middle's values; one vote.open
    assert_eq!(kinds, [("cell".to_string(), Some("middle".to_string())), ("vote.open".to_string(), None)]);
    let p = &sent(&out)[1].1;
    assert_eq!(
        *p,
        json!({
            "vote": "v1", "options": ["Left", "Right", "Stay"], "labels": { "de": ["Left", "Rechts", "Stay"] },
            "question": { "en": "Which way?", "de": "Welcher Weg?" }, "rule": "majority", "seconds": 30, "fallback": 2, "show": true,
            "groups": ["left", "right"], "where": { "device": "phone", "account": ["member", "live"] }
        })
    );
    assert_eq!(sub.conductor().on_stage("left").map(|s| s.row.as_str()), Some("crossroads"), "the ask cell kept the piece and counts as its row");
}

#[test]
fn a_hand_launch_of_one_ask_cell_opens_the_whole_question() {
    let mut sub = subtree(VOTE);
    let out = sub.launch_cell("crossroads", "right", 1_000);
    let o = opens(&out);
    assert_eq!((o.len(), o[0].groups.as_deref()), (1, Some(&["left".to_string(), "right".to_string()][..])));
    assert!(o[0].r#where.is_some());
    // #551: while its vote is open the question's cells are what is on in the matrix – every column that asks it
    let on = |sub: &Subtree| -> Vec<bool> { view::matrix(sub).rows[1].cells.iter().map(|c| c.on).collect() };
    assert_eq!(on(&sub), [true, false, true, false]);
    // another cell of the row by hand opens nothing
    assert!(opens(&sub.launch_cell("crossroads", "middle", 1_100)).is_empty());
    relay_says(&mut sub, &out, 1_200);
    sub.on_wire(&result("v1", 1), 2_000);
    assert_eq!(on(&sub), [false, false, false, false], "decided: the question is over");
}

#[test]
fn a_vote_still_open_is_closed_by_its_rule_first_then_the_new_one_opens() {
    let mut sub = subtree(&two_questions());
    let first = sub.fire("crossroads", 1_000);
    relay_says(&mut sub, &first, 1_010);
    assert!(sub.vote().is_open());
    let out = sub.fire("second", 2_000);
    let s = sent(&out);
    assert_eq!(s.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), ["vote.close", "vote.open"]);
    assert_eq!(s[0].1, json!({ "vote": "v1" }), "by its rule: no override");
    assert_eq!((s[1].1["vote"].as_str(), &s[1].1["groups"], s[1].1.get("where"), s[1].1["n"].as_i64()), (Some("v2"), &json!(["middle"]), None, Some(3)));
    // before the relay said it opened: the vote just sent is the one closed (GO twice in a row)
    let again = sent(&sub.fire("crossroads", 2_005));
    assert_eq!((again[0].0.as_str(), &again[0].1["vote"], again[1].1["vote"].as_str()), ("vote.close", &json!("v2"), Some("v3")));
}

#[test]
fn the_result_fires_the_row_set_to_its_event_and_is_logged() {
    let mut sub = subtree(VOTE);
    sub.fire("opening", 0);
    let out = sub.fire("crossroads", 1_000);
    relay_says(&mut sub, &out, 1_010);
    // the relay decides: the first option, Left → `vote:q1/1` → went-left fires on auto, left goes black (right mirrors it)
    let fired = sub.on_wire(&result("v1", 0), 31_000);
    let s = sent(&fired);
    let cells: Vec<(&str, Option<&str>, Option<&str>)> = s.iter().map(|(k, p)| (k.as_str(), p["outlet"].as_str(), p["cell"]["mode"].as_str())).collect();
    assert_eq!(cells, [("cell", Some("left"), Some("black")), ("cell", Some("right"), Some("black"))]);
    assert_eq!(sub.conductor().current(), Some("went-left"));
    let log = lines(&sub);
    let at = log.iter().position(|l| l == "vote v1: Left (by the rule) → vote:q1/1").unwrap_or_else(|| panic!("{log:?}"));
    assert!(matches!(sub.log().nth(at + 1).map(|e| &e.what), Some(Logged::Fired { row, origin: Origin::Auto }) if row == "went-left"));
    // another option: no row listens – said so
    let mut sub = subtree(VOTE);
    let out = sub.fire("crossroads", 0);
    relay_says(&mut sub, &out, 10);
    assert!(sub.on_wire(&result("v1", 2), 100).is_empty());
    assert!(lines(&sub).ends_with(&["vote v1: Stay (by the rule) → vote:q1/3".to_string(), "vote:q1/3: no row listens".to_string()]), "{:?}", lines(&sub));
}

#[test]
fn a_result_on_suggest_waits_for_the_operator_and_on_hand_does_nothing() {
    for (how, expect) in [("suggest", "suggests went-left"), ("hand", "vote:q1/1: went-left is by hand, ignored")] {
        let mut d: Value = serde_json::from_str(VOTE).unwrap();
        d["rows"][2]["fire"]["how"] = json!(how);
        let mut sub = subtree(&d.to_string());
        let out = sub.fire("crossroads", 0);
        relay_says(&mut sub, &out, 10);
        assert!(sub.on_wire(&result("v1", 0), 100).is_empty(), "{how}: nothing fires by itself");
        assert_eq!(lines(&sub).last().map(String::as_str), Some(expect), "{how}: {:?}", lines(&sub));
        assert_eq!(sub.conductor().suggestion().map(|s| s.row.as_str()), (how == "suggest").then_some("went-left"));
    }
}

#[test]
fn a_question_that_cannot_open_is_logged_and_nothing_is_sent() {
    // a draft (opened unchecked, as the editor runs it): the row names a question the show lacks
    let mut d: Value = serde_json::from_str(VOTE).unwrap();
    d["rows"][1]["cells"]["left"]["question"] = json!("q9");
    d["rows"][1]["cells"]["right"]["question"] = json!("q9");
    let mut sub = Subtree::new(document::open(&d.to_string()).unwrap(), ClockChoice::Parent);
    assert!(opens(&sub.fire("crossroads", 0)).is_empty());
    assert_eq!(lines(&sub).last().map(String::as_str), Some("row crossroads: question q9 not opened – no question 'q9'"));
}
