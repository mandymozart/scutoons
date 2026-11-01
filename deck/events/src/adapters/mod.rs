//! Adapters (research/EVENTS.md §12): today's three formats become `Event`s – the story's `VNEvent` (story.rs), the
//! relay's moments (relay.rs), raw MIDI (midi.rs). They live at the edge (control thread, never the audio thread).
//!
//! The rules every adapter shares (owner 2026-11-01): a name is `<domain>.<name>`; numeric fields are fast props (the
//! first `MAX_PROPS`, in key order – the same on every deck and in TypeScript), everything else a loose `data` table; an
//! event carries **references, not big data** – more than `MAX_DATA` bytes of `data` is refused ("send a reference").
pub mod midi;
pub mod relay;
pub mod story;

use crate::event::{Event, MAX_DATA};
use crate::names::{NameError, Names};
use crate::Time;
use serde_json::{Map, Value};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum AdapterError {
    /// not the JSON shape the format promises
    Shape(String),
    Name(NameError),
    /// `data` is over `MAX_DATA` bytes – put the payload in the database / an asset and send its id
    TooLarge { name: String, bytes: usize },
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdapterError::Shape(s) => write!(f, "{s}"),
            AdapterError::Name(e) => write!(f, "{e}"),
            AdapterError::TooLarge { name, bytes } => {
                write!(f, "{name}: {bytes} bytes of data (max {MAX_DATA}) – send a reference, not the payload")
            }
        }
    }
}

impl std::error::Error for AdapterError {}

impl From<NameError> for AdapterError {
    fn from(e: NameError) -> Self {
        AdapterError::Name(e)
    }
}

/// `params` into `e`: numbers → props (keys sorted; past `MAX_PROPS` they go to data), the rest → data. `skip` keys are
/// left out. Refuses a big payload.
pub(crate) fn fill(e: &mut Event, names: &mut Names, params: &Map<String, Value>, skip: &[&str]) -> Result<(), AdapterError> {
    let mut keys: Vec<&String> = params.keys().filter(|k| !skip.contains(&k.as_str())).collect();
    keys.sort();
    let mut data = Map::new();
    for k in keys {
        let v = &params[k];
        match v.as_f64().filter(|_| v.is_number()) {
            Some(x) => {
                // a key that is not a plain word cannot be a prop key – it stays loose
                let set = names.key(k).ok().is_some_and(|sym| e.props.get(sym).is_none() && e.props.set(sym, x as f32));
                if !set {
                    data.insert(k.clone(), v.clone());
                }
            }
            None => {
                data.insert(k.clone(), v.clone());
            }
        }
    }
    if !data.is_empty() {
        let data = Value::Object(data);
        let bytes = data.to_string().len();
        if bytes > MAX_DATA {
            return Err(AdapterError::TooLarge { name: names.name(e.name).to_string(), bytes });
        }
        e.data = Some(Box::new(data));
    }
    Ok(())
}

/// the event's JSON – the shared form (EVENTS.md §14) the TypeScript side and the fixtures use
pub fn to_json(names: &Names, e: &Event) -> Value {
    let mut o = Map::new();
    o.insert("start".into(), time_json(e.start));
    if let Some(d) = e.dur {
        o.insert("dur".into(), time_json(d));
    }
    o.insert("name".into(), names.name(e.name).into());
    if !e.props.is_empty() {
        let props: Map<String, Value> = e.props.iter().map(|(k, v)| (names.name(k).to_string(), f32_json(v))).collect();
        o.insert("props".into(), props.into());
    }
    if let Some(d) = &e.data {
        o.insert("data".into(), (**d).clone());
    }
    if !e.children.is_empty() {
        o.insert("children".into(), e.children.iter().map(|c| to_json(names, c)).collect::<Vec<_>>().into());
    }
    if let Some(id) = &e.id {
        o.insert("id".into(), id.clone().into());
    }
    if let Some(s) = &e.source {
        o.insert("source".into(), s.clone().into());
    }
    o.into()
}

/// an f32 as the f64 JSON (and JS) shows it: 0.3f32 is `0.3`, not `0.30000001192…`
fn f32_json(v: f32) -> Value {
    v.to_string().parse::<f64>().ok().and_then(|x| serde_json::Number::from_f64(x)).map_or(Value::Null, Value::Number)
}

fn time_json(t: Time) -> Value {
    match t {
        Time::Now => "now".into(),
        Time::Beats(b) => serde_json::json!({ "beats": b }),
        Time::Seconds(s) => serde_json::json!({ "seconds": s }),
    }
}
