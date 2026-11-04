//! What a relay message means for an outlet (#186, LIVE.md B.6): the cells, preloads and the show's slice for this outlet
//! go to the composer and the preloader, the rest is not the outlet's. The adapter (deck-native's live/client.rs) keeps
//! the socket, the join and resume (deck-space's relay.rs), and hands each `Routed` to its thread.
use protocol::{Cell, MediaPreload, Message, Snapshot};

/// What the composer reacts to.
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Cell(Cell),
    /// the show's slice (`state.slice`) – surtitles follow it
    Slice(usize),
    /// panic black
    Black,
    /// the preloader has new files: a cell waiting for them is composed again
    Media,
}

/// Where a message goes, in the order the outlet acts on it.
#[derive(Debug, Clone, PartialEq)]
pub enum Routed {
    /// joined as output `client` (its snapshot follows)
    Joined(String),
    /// to the composer
    Compose(Input),
    /// to the preloader: this outlet's media
    Preload(MediaPreload),
    /// the relay refused something (its code and message – the HUD's line)
    Refused { code: String, message: String },
}

/// A message for `outlet` → where its parts go (nothing when it is not the outlet's).
pub fn route(message: Message, outlet: &str) -> Vec<Routed> {
    match message {
        Message::Welcome(w) => {
            let mut out = vec![Routed::Joined(w.client)];
            out.extend(snapshot(w.snapshot, outlet));
            out
        }
        Message::Snapshot(s) => snapshot(s, outlet),
        Message::Cell(c) if c.outlet == outlet => vec![Routed::Compose(Input::Cell(c.cell))],
        Message::MediaPreload(m) if m.outlet == outlet => vec![Routed::Preload(m)],
        Message::State(s) => vec![Routed::Compose(Input::Slice(s.slice.max(0) as usize))],
        Message::Panic(p) if p.black == Some(true) => vec![Routed::Compose(Input::Black)],
        Message::Error(e) => vec![Routed::Refused { code: e.code, message: e.message }],
        _ => vec![],
    }
}

/// A snapshot (join, reconnect outside the replay buffer, an assign): the outlet's preload, the slice, its cell.
pub fn snapshot(s: Snapshot, outlet: &str) -> Vec<Routed> {
    let mut out = vec![];
    if let Some(m) = s.media.and_then(|mut m| m.remove(outlet)) {
        out.push(Routed::Preload(m));
    }
    if let Some(st) = s.state {
        out.push(Routed::Compose(Input::Slice(st.slice.max(0) as usize)));
    }
    if let Some(c) = s.cells.and_then(|mut c| c.remove(outlet)) {
        out.push(Routed::Compose(Input::Cell(c)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(json: &str) -> Message {
        serde_json::from_str(json).unwrap()
    }

    const CELL: &str = r##"{ "mode": "own", "layers": [ { "id": "wash", "src": { "kind": "colour", "p": { "colour": "#000" } } } ] }"##;

    #[test]
    fn the_outlets_cells_and_preloads_others_not() {
        let mine = route(msg(&format!(r#"{{ "kind": "cell", "p": {{ "outlet": "left", "cell": {CELL} }} }}"#)), "left");
        assert!(matches!(&mine[..], [Routed::Compose(Input::Cell(_))]), "{mine:?}");
        assert!(route(msg(&format!(r#"{{ "kind": "cell", "p": {{ "outlet": "right", "cell": {CELL} }} }}"#)), "left").is_empty());
        let pre = r#"{ "kind": "media.preload", "p": { "outlet": "left", "items": [] } }"#;
        assert!(matches!(&route(msg(pre), "left")[..], [Routed::Preload(_)]));
        assert!(route(msg(pre), "right").is_empty());
    }

    #[test]
    fn state_panic_and_refusals() {
        assert_eq!(route(msg(r#"{ "kind": "state", "p": { "row": 0, "slice": 7, "vars": {} } }"#), "left"), [Routed::Compose(Input::Slice(7))]);
        assert_eq!(route(msg(r#"{ "kind": "state", "p": { "row": 0, "slice": -2, "vars": {} } }"#), "left"), [Routed::Compose(Input::Slice(0))]);
        assert_eq!(route(msg(r#"{ "kind": "panic", "p": { "black": true } }"#), "left"), [Routed::Compose(Input::Black)]);
        assert!(route(msg(r#"{ "kind": "panic", "p": {} }"#), "left").is_empty());
        assert_eq!(
            route(msg(r#"{ "kind": "error", "p": { "code": "rate", "message": "slow down" } }"#), "left"),
            [Routed::Refused { code: "rate".into(), message: "slow down".into() }]
        );
    }

    #[test]
    fn a_snapshot_preloads_then_the_slice_then_the_cell() {
        let s = format!(
            r#"{{ "kind": "snapshot", "p": {{ "state": {{ "row": 0, "slice": 3, "vars": {{}} }}, "cells": {{ "left": {CELL}, "right": {CELL} }},
                "media": {{ "left": {{ "outlet": "left", "items": [] }} }} }} }}"#
        );
        let r = route(msg(&s), "left");
        assert!(matches!(&r[..], [Routed::Preload(_), Routed::Compose(Input::Slice(3)), Routed::Compose(Input::Cell(_))]), "{r:?}");
        let other = route(msg(&s), "right");
        assert!(matches!(&other[..], [Routed::Compose(Input::Slice(3)), Routed::Compose(Input::Cell(_))]), "{other:?}");
    }
}
