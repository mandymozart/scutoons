//! The adapters (EVENTS.md §12): the story's events (the same cases run in TypeScript – shared/live-deck/test/fixtures/
//! events.json is written from here and a stale file fails), the relay's moments, raw MIDI, the big-payload guard.
use deck_events::adapters::midi::{hz, Midi};
use deck_events::adapters::relay::{self, RelayClock};
use deck_events::adapters::story::from_story;
use deck_events::adapters::{to_json, AdapterError};
use deck_events::{Names, Time, MAX_DATA};
use serde_json::{json, Value};

/// the real chapter's slice event: a `showMessage` of catpope, happy, on an intersect trigger
pub fn chapter_event() -> Value {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/content/test/fixtures/neutral/chapter.json")).unwrap();
    fn find(v: &Value) -> Option<Value> {
        match v {
            Value::Object(o) if o.get("type") == Some(&json!("showMessage")) && o.contains_key("trigger") => Some(v.clone()),
            Value::Object(o) => o.values().find_map(find),
            Value::Array(a) => a.iter().find_map(find),
            _ => None,
        }
    }
    find(&serde_json::from_str(&text).unwrap()).expect("the chapter has a slice showMessage")
}

fn cases() -> Vec<(&'static str, Value, Option<&'static str>)> {
    vec![
        ("the chapter's slice event: a trigger, text and mood in data", chapter_event(), Some("story:neutral/1")),
        ("a plain message with a locale map; wait is the runner's", json!({"type": "showMessage", "params": {"text": {"de": "Hallo", "en": "Hello"}, "character": "catpope", "wait": true}, "id": "m1"}), None),
        ("numbers are props, the duration is the event's length", json!({"type": "fx", "params": {"effect": "flash", "duration": 1500, "intensity": 0.5, "delay": 200}}), None),
        ("a type without params", json!({"type": "hideMessage"}), None),
        ("the runner keeps choices", json!({"type": "showChoices"}), None),
        ("the runner keeps control flow", json!({"type": "jumpTo", "params": {"target": "x"}}), None),
        ("more numbers than slots: the rest goes to data", json!({"type": "fx", "params": {"a": 1, "b": 2, "c": 3, "d": 4, "e": 5, "f": 6, "g": 7, "effect": "x"}}), None),
    ]
}

#[test]
fn story_events_become_triggers_in_the_story_domain() {
    let mut n = Names::new();
    let e = from_story(&mut n, &chapter_event(), Some("story:neutral/1")).unwrap().unwrap();
    assert_eq!((n.name(e.name), e.start, e.dur), ("story.showMessage", Time::Now, None));
    assert!(n.in_domain(e.name, "story") && e.children.is_empty() && e.props.is_empty());
    let data = e.data.as_ref().unwrap();
    assert_eq!((data["character"].as_str(), data["mood"].as_str()), (Some("catpope"), Some("happy")));
    assert_eq!(e.source.as_deref(), Some("story:neutral/1"));

    let fx = json!({"type": "fx", "params": {"effect": "flash", "duration": 1500, "intensity": 0.5}, "id": "f1"});
    let e = from_story(&mut n, &fx, None).unwrap().unwrap();
    assert_eq!((e.dur, e.id.as_deref()), (Some(Time::Seconds(1.5)), Some("f1")));
    assert_eq!(e.props.get(n.get("intensity").unwrap()), Some(0.5));
    assert!(e.props.get(n.get("duration").unwrap_or_default()).is_none());
}

#[test]
fn the_runner_keeps_choices_and_the_rest_is_refused_loudly() {
    let mut n = Names::new();
    for t in ["showChoices", "addChoice", "label", "divert", "jumpTo", "setVariable", "askText"] {
        assert_eq!(from_story(&mut n, &json!({"type": t, "params": {}}), None), Ok(None), "{t}");
    }
    assert!(matches!(from_story(&mut n, &json!({"params": {}}), None), Err(AdapterError::Shape(_))));
    assert!(matches!(from_story(&mut n, &json!({"type": "Bad Name"}), None), Err(AdapterError::Name(_))));
    assert!(matches!(from_story(&mut n, &json!({"type": "fx", "params": 3}), None), Err(AdapterError::Shape(_))));
}

#[test]
fn a_big_payload_is_refused_with_a_hint() {
    let mut n = Names::new();
    let big = json!({"type": "showMessage", "params": {"text": "x".repeat(MAX_DATA)}});
    let err = from_story(&mut n, &big, None).unwrap_err();
    assert!(matches!(err, AdapterError::TooLarge { bytes, .. } if bytes > MAX_DATA));
    assert!(err.to_string().contains("send a reference"), "{err}");
    let ok = json!({"type": "showMessage", "params": {"text": "x".repeat(MAX_DATA - 100)}});
    assert!(from_story(&mut n, &ok, None).is_ok());
}

/// shared/live-deck's TypeScript `fromStory` must give these very events; `UPDATE_FIXTURES=1 cargo test -p deck-events
/// --test adapters story_fixture` writes the file, a plain run fails when it is stale
#[test]
fn story_fixture_for_the_typescript_adapter_is_current() {
    let mut n = Names::new();
    let rows: Vec<Value> = cases()
        .into_iter()
        .map(|(label, story, source)| {
            let event = from_story(&mut n, &story, source).unwrap().map_or(Value::Null, |e| to_json(&n, &e));
            json!({ "label": label, "story": story, "source": source, "event": event })
        })
        .collect();
    let text = serde_json::to_string_pretty(&rows).unwrap() + "\n";
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/live-deck/test/fixtures/events.json");
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(path, &text).unwrap();
    }
    let on_disk = std::fs::read_to_string(path).unwrap_or_default();
    assert!(on_disk == text, "{path} is stale: UPDATE_FIXTURES=1 cargo test -p deck-events --test adapters story_fixture");
}

const CLOCK: RelayClock = RelayClock { zero_ms: 10_000 };

#[test]
fn relay_moments_become_events_and_state_stays_a_message() {
    let mut n = Names::new();
    let fx = json!({"v": 1, "seq": 4, "at": 12_500, "from": "c7", "kind": "fx", "p": {"effect": "flash", "ms": 300}});
    let e = relay::from_json(&mut n, &fx, CLOCK).unwrap().unwrap();
    assert_eq!((n.name(e.name), e.start), ("relay.fx", Time::Seconds(2.5)));
    assert_eq!((e.props.get(n.get("ms").unwrap()), e.source.as_deref()), (Some(300.0), Some("relay:c7")));
    assert_eq!(e.data.as_ref().unwrap()["effect"], "flash");

    let no_time = json!({"v": 1, "seq": 5, "from": "c7", "kind": "react", "p": {"kind": "clap"}});
    assert_eq!(relay::from_json(&mut n, &no_time, CLOCK).unwrap().unwrap().start, Time::Now, "no `at` = a trigger");

    for state in ["deck.graph", "snapshot", "welcome", "presence", "ping", "deck.values"] {
        let env = json!({"v": 1, "seq": 1, "from": "relay", "kind": state, "p": {}});
        assert_eq!(relay::from_json(&mut n, &env, CLOCK), Ok(None), "{state}");
    }
}

#[test]
fn a_cue_carries_its_actions_as_children() {
    let mut n = Names::new();
    let cue = json!({"v": 1, "seq": 9, "at": 11_000, "from": "conductor", "kind": "cue", "p": {"id": "c1", "actions": [
        {"light": {"outlet": "left", "level": 0.5}},
        {"outlet": "right", "kind": "x", "n": 2},
    ]}});
    let e = relay::from_json(&mut n, &cue, CLOCK).unwrap().unwrap();
    assert_eq!((n.name(e.name), e.id.as_deref(), e.start), ("relay.cue", Some("c1"), Time::Seconds(1.0)));
    let names: Vec<&str> = e.children.iter().map(|c| n.name(c.name)).collect();
    assert_eq!(names, ["relay.light", "relay.action"]);
    assert_eq!(e.children[0].props.get(n.get("level").unwrap()), Some(0.5));
    assert_eq!(e.children[0].data.as_ref().unwrap()["outlet"], "left");
    // the typed envelope reads the same as its wire JSON
    let env: protocol::Envelope = serde_json::from_value(cue.clone()).unwrap();
    assert_eq!(relay::from_envelope(&mut n, &env, CLOCK).unwrap().unwrap(), e);
}

#[test]
fn midi_note_on_and_off_become_one_note_with_a_duration() {
    let mut n = Names::new();
    let mut m = Midi::new(&mut n, Some("midi:keys"));
    let mut out = Vec::new();
    assert!(m.feed(&[0x91, 69, 127], 1.0, &mut out));
    assert!(out.is_empty(), "a note is born when it ends");
    assert!(m.feed(&[0x81, 69, 0], 1.75, &mut out));
    let e = &out[0];
    assert_eq!((n.name(e.name), e.start, e.dur), ("midi.note", Time::Seconds(1.0), Some(Time::Seconds(0.75))));
    let p = |k: &str| e.props.get(n.get(k).unwrap()).unwrap();
    assert_eq!((p("pitch"), p("vel"), p("note"), p("ch")), (440.0, 1.0, 69.0, 1.0));
    assert_eq!(e.source.as_deref(), Some("midi:keys"));
    assert!((hz(60) - 261.6256).abs() < 1e-3);

    // note on with velocity 0 is an off; an off without an on is ignored (consumed, no event)
    out.clear();
    m.feed(&[0x90, 60, 64], 2.0, &mut out);
    m.feed(&[0x90, 60, 0], 2.5, &mut out);
    assert!(m.feed(&[0x80, 60, 0], 3.0, &mut out));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].props.get(n.get("vel").unwrap()), Some(64.0 / 127.0));
}

#[test]
fn midi_retrigger_release_cc_and_the_rest() {
    let mut n = Names::new();
    let mut m = Midi::new(&mut n, None);
    let mut out = Vec::new();
    m.feed(&[0x90, 50, 100], 0.0, &mut out);
    m.feed(&[0x90, 50, 90], 1.0, &mut out);
    assert_eq!((out.len(), out[0].dur), (1, Some(Time::Seconds(1.0))), "struck again: the first ends there");
    m.feed(&[0x90, 52, 80], 1.5, &mut out);
    m.release_all(2.0, &mut out);
    assert_eq!(out.len(), 3, "the two held notes end");
    m.release_all(3.0, &mut out);
    assert_eq!(out.len(), 3, "nothing is held any more");

    out.clear();
    assert!(m.feed(&[0xB2, 7, 127], 4.0, &mut out));
    let cc = &out[0];
    let p = |k: &str| cc.props.get(n.get(k).unwrap()).unwrap();
    assert_eq!((n.name(cc.name), cc.start, p("cc"), p("value"), p("ch")), ("midi.cc", Time::Seconds(4.0), 7.0, 1.0, 2.0));
    for ignored in [&[0xE0u8, 0, 64][..], &[0xC0, 5], &[0xF8], &[0x90, 200, 1], &[]] {
        assert!(!m.feed(ignored, 5.0, &mut out), "{ignored:?}");
    }
    assert_eq!(out.len(), 1);
}

#[test]
fn a_story_event_reaches_another_deck_through_the_relay_and_is_adapted_there() {
    let mut n = Names::new();
    // the sending side puts the story event on the wire as a `story` message (protocol: Loose payload)
    let story = chapter_event();
    let env = json!({"v": 1, "seq": 7, "at": 12_000, "from": "c1", "kind": "story", "p": story});
    let e = relay::from_json(&mut n, &env, CLOCK).unwrap().unwrap();
    assert_eq!((n.name(e.name), e.start, e.source.as_deref()), ("relay.story", Time::Seconds(2.0), Some("relay:c1")));
    // the receiving deck adapts the carried story event itself – the same event the sender's own deck made
    let carried = from_story(&mut n, &story, Some("relay:c1")).unwrap().unwrap();
    assert_eq!(n.name(carried.name), "story.showMessage");
    assert_eq!(carried.data.as_ref().unwrap()["mood"], "happy");
    // and the relay's message typed: it is a Story(Loose)
    let typed: protocol::Envelope = serde_json::from_value(env).unwrap();
    assert!(matches!(typed.message, protocol::Message::Story(_)));
}
