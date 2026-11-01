//! deck-events (#236, research/EVENTS.md): generators that talk in events.
//!
//! - names.rs – the name dictionary with domains (`story.fx`, `music.note`, `relay.cue` …) and prop keys, interned
//! - event.rs – `Event` (start, dur, name, props, children, hops) and `Time` (now · beats · seconds)
//! - generator.rs – the `Generator` trait: `query(span)` → the events starting in the span
//! - graph.rs – the event graph: zero-delay cycles refused, delayed edges, per-edge caps, a hop limit
//! - rt.rs – events resolved to frame offsets for the audio thread (`RtEvent`)
//! - glide.rs – `Glide`: an exponential pitch glide, the same on the stage and in the browser
//! - patch.rs – generators rigged together as data (TOML / JSON): what a rig's `events` node holds
//! - horizon.rs – event horizons: a scope's reachable references, nearest first, bounded – what to preload (§16)
//! - boundary.rs – the references guard: an inbound event's loose payload over 1 KB is refused ("send a reference")
//! - generators/ – `random_walk`, `pulse`, `arp`; bridge.rs – `gate_cv` (events → seq's cv: pitch left, gate right)
//!
//! Encapsulated (EVENTS.md §10): the stage's `walk` / `events` nodes and live-core's walks / events host it.
pub mod boundary;
pub mod bridge;
pub mod event;
pub mod generator;
pub mod generators;
pub mod glide;
pub mod graph;
pub mod horizon;
pub mod names;
pub mod patch;
pub mod rt;

pub use event::{Event, Props, Time, MAX_PROPS};
pub use generator::{Generator, Inbox, Outbox, Ports, Span};
pub use glide::Glide;
pub use graph::{EventGraph, GraphError, Stats, EDGE_CAP, MAX_HOPS};
pub use names::{NameError, Names, Sym};
pub use boundary::{admit, BoundaryError, PAYLOAD_LIMIT};
pub use horizon::{Bound, Cut, Dist, Horizon, Reach, CHOICE_DEPTH, DECK_BYTES, SHOW_ROWS, SHOW_SECS};
pub use rt::RtEvent;
