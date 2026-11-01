//! deck-events (#236, research/EVENTS.md): generators that talk in events.
//!
//! - names.rs – the name dictionary with domains (`story.fx`, `music.note`, `relay.cue` …) and prop keys, interned
//! - event.rs – `Event` (start, dur, name, props, children, hops) and `Time` (now · beats · seconds)
//! - generator.rs – the `Generator` trait: `query(span)` → the events starting in the span
//! - graph.rs – the event graph: zero-delay cycles refused, delayed edges, per-edge caps, a hop limit
//! - rt.rs – events resolved to frame offsets for the audio thread (`RtEvent`)
//! - glide.rs – `Glide`: an exponential pitch glide, the same on the stage and in the browser
//! - generators/ – `random_walk`, `pulse`, `arp`; bridge.rs – `gate_cv` (events → seq's cv: pitch left, gate right)
//!
//! Encapsulated (EVENTS.md §10): nothing else in deck/ depends on it yet; the stage's host node is the next slice.
pub mod bridge;
pub mod event;
pub mod generator;
pub mod generators;
pub mod glide;
pub mod graph;
pub mod names;
pub mod rt;

pub use event::{Event, Props, Time, MAX_PROPS};
pub use generator::{Generator, Inbox, Outbox, Ports, Span};
pub use glide::Glide;
pub use graph::{EventGraph, GraphError, Stats, EDGE_CAP, MAX_HOPS};
pub use names::{NameError, Names, Sym};
pub use rt::RtEvent;
