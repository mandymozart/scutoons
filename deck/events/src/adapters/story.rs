//! The story adapter (EVENTS.md §12, owner 2026-11-01): `{ type, params, id?, if?, trigger? }` – the story's `VNEvent`
//! as JSON (shared/content visual-novel.ts) – becomes `story.<type>`, a **trigger** (`Time::Now`, no steps). The story
//! keeps its own runner: `if` is evaluated by the caller before it asks, `wait`, choices, labels and jumps stay the
//! runner's (`Ok(None)`), a deck gets a copy of what happens. `duration` (ms) → `dur`; numeric params → props; the rest
//! (text, character, mood, …) → `data`. There is no TypeScript copy (#377): the Rust adapter is the one place of the rules;
//! test/fixtures/events.json is its written-out answers (UPDATE_FIXTURES=1).
use super::{fill, AdapterError};
use crate::event::Event;
use crate::names::Names;
use crate::Time;
use serde_json::Value;

/// what only the story's runner does: choices, control flow, the variable store
pub const STAYS_IN_THE_STORY: &[&str] = &["showChoices", "addChoice", "label", "divert", "jumpTo", "setVariable", "askText"];

/// `e` as an event; `Ok(None)` for a type the runner keeps. `source`: where it came from (`story:catpope/2`).
pub fn from_story(names: &mut Names, e: &Value, source: Option<&str>) -> Result<Option<Event>, AdapterError> {
    let kind = e.get("type").and_then(Value::as_str).ok_or_else(|| AdapterError::Shape("a story event needs a string `type`".into()))?;
    if STAYS_IN_THE_STORY.contains(&kind) {
        return Ok(None);
    }
    let mut ev = Event::new(names.event(&format!("story.{kind}"))?, Time::Now);
    if let Some(params) = e.get("params") {
        let params = params.as_object().ok_or_else(|| AdapterError::Shape(format!("{kind}: `params` is not an object")))?;
        // `wait` is the runner's; `duration` (ms) becomes the event's length
        let ms = params.get("duration").and_then(Value::as_f64).filter(|_| params["duration"].is_number());
        let skip: &[&str] = if ms.is_some() { &["wait", "duration"] } else { &["wait"] };
        fill(&mut ev, names, params, skip)?;
        ev.dur = ms.map(|ms| Time::Seconds(ms / 1000.0));
    }
    ev.id = e.get("id").and_then(Value::as_str).map(str::to_string);
    ev.source = source.map(str::to_string);
    Ok(Some(ev))
}
