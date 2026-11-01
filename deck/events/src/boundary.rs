//! The references guard (#236, research/EVENTS.md §16 "References, not payloads", owner 2026-11-01): where an event
//! enters from outside – a relay envelope `{ kind, at, p }`, a story event, an OSC / MIDI adapter – its name is checked
//! against the dictionary and its loose payload (the serialized `p` / `data`, as the adapter received it) against a
//! limit. Big data never rides on an event: a database entry, a sequence, an asset travel as a reference (an id or an
//! immutable path) the receiver loads and caches itself. Refused here, before routing – the event flow never carries it.
use crate::names::{NameError, Names, Sym};
use std::fmt;

/// a loose payload's limit in bytes (its serialized form, as received)
pub const PAYLOAD_LIMIT: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundaryError {
    /// the name is not `<domain>.<name>` (names.rs)
    Name(NameError),
    /// the payload is over the limit: send a reference
    TooBig { name: String, bytes: usize, limit: usize },
}

impl fmt::Display for BoundaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoundaryError::Name(e) => e.fmt(f),
            BoundaryError::TooBig { name, bytes, limit } => write!(
                f,
                "event '{name}': its payload is {bytes} bytes, over the {limit}-byte limit – send a reference \
                 (an id like pattern:m:01H… or an immutable path), not the data"
            ),
        }
    }
}

impl std::error::Error for BoundaryError {}

/// Admit an inbound event at the default limit: its interned name, or why it is refused.
pub fn admit(names: &mut Names, name: &str, payload: &str) -> Result<Sym, BoundaryError> {
    admit_within(names, name, payload, PAYLOAD_LIMIT)
}

/// `admit` with another limit (an adapter with a tighter one, a test)
pub fn admit_within(names: &mut Names, name: &str, payload: &str, limit: usize) -> Result<Sym, BoundaryError> {
    // the size first: a refused event interns nothing
    if payload.len() > limit {
        return Err(BoundaryError::TooBig { name: name.to_string(), bytes: payload.len(), limit });
    }
    names.event(name).map_err(BoundaryError::Name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_passes() {
        let mut names = Names::new();
        let sym = admit(&mut names, "relay.cue", r#"{"pattern":"pattern:m:01HXYZ","set":"catpope/map/sets/cathedral.low.v2.glb"}"#);
        assert_eq!(sym, names.event("relay.cue").map_err(BoundaryError::Name));
        assert!(admit(&mut names, "story.fx", "").is_ok(), "no payload at all");
    }

    #[test]
    fn exactly_the_limit_passes_one_byte_more_is_refused() {
        let mut names = Names::new();
        assert!(admit(&mut names, "relay.cue", &"x".repeat(PAYLOAD_LIMIT)).is_ok());
        let err = admit(&mut names, "relay.cue", &"x".repeat(PAYLOAD_LIMIT + 1)).unwrap_err();
        assert_eq!(err, BoundaryError::TooBig { name: "relay.cue".into(), bytes: PAYLOAD_LIMIT + 1, limit: PAYLOAD_LIMIT });
    }

    #[test]
    fn the_error_says_send_a_reference() {
        let mut names = Names::new();
        let big = format!(r#"{{"notes":[{}]}}"#, vec!["60"; 600].join(","));
        let msg = admit(&mut names, "music.pattern", &big).unwrap_err().to_string();
        assert!(msg.contains("music.pattern") && msg.contains("send a reference") && msg.contains("1024"), "{msg}");
    }

    #[test]
    fn a_tighter_limit_and_a_bad_name() {
        let mut names = Names::new();
        assert!(matches!(admit_within(&mut names, "osc.value", "12345", 4), Err(BoundaryError::TooBig { limit: 4, .. })));
        assert!(matches!(admit(&mut names, "cue", "{}"), Err(BoundaryError::Name(NameError::NoDomain(_)))));
    }

    #[test]
    fn a_refused_event_interns_nothing() {
        let mut names = Names::new();
        let _ = admit(&mut names, "media.blob", &"x".repeat(PAYLOAD_LIMIT * 2));
        assert_eq!(names.get("media.blob"), None);
    }
}
