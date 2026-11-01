//! The relay adapter (EVENTS.md §12): the relay's **moments** – `cue`, `fx`, `say`, `line`, `colour`, `story`, `react`, `pickup`,
//! `panic` – become `relay.<kind>` events; the state kinds (`deck.graph`, `snapshot`, `welcome`, `presence` …) stay
//! messages (`Ok(None)`). `at` (relay ms, clock-synced) → `Time::Seconds` on the deck's timeline through `RelayClock`
//! (no `at` = a trigger); `p` → props / data; a cue's `id` → the event's id and its `actions` → children
//! (`relay.<verb>` for `{ verb: {…} }`, `relay.action` for a flat one) that start with it.
use super::{fill, AdapterError};
use crate::event::Event;
use crate::names::Names;
use crate::Time;
use protocol::Envelope;
use serde_json::Value;

/// the kinds that happen (the rest is state)
pub const MOMENTS: &[&str] = &["cue", "fx", "say", "line", "colour", "story", "react", "pickup", "panic"];

/// relay time → the deck's timeline: `zero_ms` is the relay time at the timeline's second 0
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelayClock {
    pub zero_ms: i64,
}

impl RelayClock {
    pub fn seconds(&self, at_ms: i64) -> f64 {
        (at_ms - self.zero_ms) as f64 / 1000.0
    }
}

/// a typed envelope as an event (it goes through its wire JSON, so both forms read the same)
pub fn from_envelope(names: &mut Names, env: &Envelope, clock: RelayClock) -> Result<Option<Event>, AdapterError> {
    let json = serde_json::to_value(env).map_err(|e| AdapterError::Shape(e.to_string()))?;
    from_json(names, &json, clock)
}

/// an envelope's wire JSON (`{ v, seq, at?, from, kind, p }`) as an event
pub fn from_json(names: &mut Names, env: &Value, clock: RelayClock) -> Result<Option<Event>, AdapterError> {
    let kind = env.get("kind").and_then(Value::as_str).ok_or_else(|| AdapterError::Shape("an envelope needs a string `kind`".into()))?;
    if !MOMENTS.contains(&kind) {
        return Ok(None);
    }
    let start = env.get("at").and_then(Value::as_i64).map_or(Time::Now, |at| Time::Seconds(clock.seconds(at)));
    let mut ev = Event::new(names.event(&format!("relay.{kind}"))?, start);
    let p = match env.get("p") {
        Some(Value::Object(p)) => p,
        Some(_) => return Err(AdapterError::Shape(format!("{kind}: `p` is not an object"))),
        None => {
            ev.source = source(env);
            return Ok(Some(ev));
        }
    };
    if kind == "cue" {
        ev.id = p.get("id").and_then(Value::as_str).map(str::to_string);
        fill(&mut ev, names, p, &["id", "actions"])?;
        for a in p.get("actions").and_then(Value::as_array).into_iter().flatten() {
            ev.children.push(action(names, a)?);
        }
    } else {
        fill(&mut ev, names, p, &[])?;
    }
    ev.source = source(env);
    Ok(Some(ev))
}

fn source(env: &Value) -> Option<String> {
    env.get("from").and_then(Value::as_str).map(|f| format!("relay:{f}"))
}

/// one cue action: `{ <verb>: { outlet, … } }` (B.5.2) or flat `{ outlet, … }`
fn action(names: &mut Names, a: &Value) -> Result<Event, AdapterError> {
    let o = a.as_object().ok_or_else(|| AdapterError::Shape("a cue action is not an object".into()))?;
    if let [(verb, Value::Object(body))] = o.iter().collect::<Vec<_>>()[..] {
        if let Ok(name) = names.event(&format!("relay.{verb}")) {
            let mut e = Event::now(name);
            fill(&mut e, names, body, &[])?;
            return Ok(e);
        }
    }
    let mut e = Event::now(names.event("relay.action")?);
    fill(&mut e, names, o, &[])?;
    Ok(e)
}
