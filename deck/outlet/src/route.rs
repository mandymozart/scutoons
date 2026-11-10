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
    /// #91: to the deck's piece side (deck-space piece.rs): every cell of this outlet – a piece plays, glides or fades,
    /// a cell without one (black, panic) fades what played
    Piece(Cell),
    /// #91: to the deck's flash side (deck-show `ready.rs`): the tree resolved to exact versions – keep what the outlet
    /// plays, check its pieces, report
    Flash(protocol::Flash),
    /// the relay refused something (its code and message – the HUD's line)
    Refused { code: String, message: String },
    /// #478 (proposal: a role without a cell yet keeps what plays; only no role goes to the idle scene)
    /// #478: this outlet moved – a role taken (the deck's own or the conductor's assign), or none: from now the cells of
    /// that role are its own (the snapshot that follows brings its cell); no role plays the idle scene (a black cell)
    Moved(Option<String>),
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
        Message::Cell(c) if c.outlet == outlet => vec![Routed::Piece(c.cell.clone()), Routed::Compose(Input::Cell(c.cell))],
        Message::MediaPreload(m) if m.outlet == outlet => vec![Routed::Preload(m)],
        Message::State(s) => vec![Routed::Compose(Input::Slice(s.slice.max(0) as usize))],
        Message::Panic(p) if p.black == Some(true) => vec![Routed::Piece(black()), Routed::Compose(Input::Black)],
        Message::Error(e) => vec![Routed::Refused { code: e.code, message: e.message }],
        Message::Flash(f) => vec![Routed::Flash(f)],
        // #478: only this client hears its assign
        Message::Assign(a) if a.group.is_empty() => vec![Routed::Moved(None), Routed::Piece(black()), Routed::Compose(Input::Black)],
        Message::Assign(a) => vec![Routed::Moved(Some(a.group))],
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
        out.push(Routed::Piece(c.clone()));
        out.push(Routed::Compose(Input::Cell(c)));
    }
    if let Some(f) = s.flash {
        out.push(Routed::Flash(f));
    }
    out
}

/// the cell a panic's black is to the piece side
fn black() -> Cell {
    Cell { mode: protocol::CellMode::Black, layers: None, of: None, send: None }
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
        assert!(matches!(&mine[..], [Routed::Piece(_), Routed::Compose(Input::Cell(_))]), "{mine:?}");
        assert!(route(msg(&format!(r#"{{ "kind": "cell", "p": {{ "outlet": "right", "cell": {CELL} }} }}"#)), "left").is_empty());
        let pre = r#"{ "kind": "media.preload", "p": { "outlet": "left", "items": [] } }"#;
        assert!(matches!(&route(msg(pre), "left")[..], [Routed::Preload(_)]));
        assert!(route(msg(pre), "right").is_empty());
        let flash = r#"{ "kind": "flash", "p": { "root": { "id": "top", "version": 3 }, "shows": [], "hops": 1 } }"#;
        assert!(matches!(&route(msg(flash), "left")[..], [Routed::Flash(f)] if f.hops == 1), "every outlet takes a flash");
    }

    #[test]
    fn state_panic_and_refusals() {
        assert_eq!(route(msg(r#"{ "kind": "state", "p": { "row": 0, "slice": 7, "vars": {} } }"#), "left"), [Routed::Compose(Input::Slice(7))]);
        assert_eq!(route(msg(r#"{ "kind": "state", "p": { "row": 0, "slice": -2, "vars": {} } }"#), "left"), [Routed::Compose(Input::Slice(0))]);
        assert_eq!(route(msg(r#"{ "kind": "panic", "p": { "black": true } }"#), "left"), [Routed::Piece(black()), Routed::Compose(Input::Black)]);
        assert!(route(msg(r#"{ "kind": "panic", "p": {} }"#), "left").is_empty());
        assert_eq!(
            route(msg(r#"{ "kind": "error", "p": { "code": "rate", "message": "slow down" } }"#), "left"),
            [Routed::Refused { code: "rate".into(), message: "slow down".into() }]
        );
    }

    /// #478: a role taken at runtime: the outlet is that role's from now (its snapshot follows); no role fades to the idle scene
    #[test]
    fn an_assign_moves_the_outlet() {
        let to = route(msg(r#"{ "kind": "assign", "p": { "client": "c3", "group": "tenor" } }"#), "left");
        assert_eq!(to, [Routed::Moved(Some("tenor".into()))]);
        let none = route(msg(r#"{ "kind": "assign", "p": { "client": "c3", "group": "", "lock": true } }"#), "left");
        assert_eq!(none, [Routed::Moved(None), Routed::Piece(black()), Routed::Compose(Input::Black)]);
    }

    #[test]
    fn a_snapshot_preloads_then_the_slice_then_the_cell() {
        let s = format!(
            r#"{{ "kind": "snapshot", "p": {{ "state": {{ "row": 0, "slice": 3, "vars": {{}} }}, "cells": {{ "left": {CELL}, "right": {CELL} }},
                "media": {{ "left": {{ "outlet": "left", "items": [] }} }} }} }}"#
        );
        let r = route(msg(&s), "left");
        assert!(matches!(&r[..], [Routed::Preload(_), Routed::Compose(Input::Slice(3)), Routed::Piece(_), Routed::Compose(Input::Cell(_))]), "{r:?}");
        let other = route(msg(&s), "right");
        assert!(matches!(&other[..], [Routed::Compose(Input::Slice(3)), Routed::Piece(_), Routed::Compose(Input::Cell(_))]), "{other:?}");
    }
}
