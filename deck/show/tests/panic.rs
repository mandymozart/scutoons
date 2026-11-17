//! The top conductor's own panic: black and silence down the tree at once, held while rows go on in the core, and the
//! decks back to the cells the core holds on resume.
use deck_show::{parse, ClockChoice, Logged, Subtree};
use protocol::{CellMode, Envelope, LayerSrc, Message};

const DOC: &str = r#"{ "id": "panic", "version": 1, "toon": "catpope", "outlets": [{ "id": "left" }, { "id": "right" }], "rows": [
  { "id": "one", "cells": { "left": { "mode": "play", "piece": "scene:nave" }, "right": { "mode": "play", "piece": "scene:choir" } } },
  { "id": "two", "cells": { "left": { "mode": "play", "piece": "scene:crypt" } } }
] }"#;

/// each cell sent: its outlet and its piece (none: black)
fn cells(out: &[Envelope]) -> Vec<(String, Option<String>)> {
    out.iter()
        .filter_map(|e| match &e.message {
            Message::Cell(c) => {
                let piece = c.cell.layers.as_ref().and_then(|l| l.first()).and_then(|l| if let LayerSrc::Piece(p) = &l.src { Some(p.piece.clone()) } else { None });
                assert!(piece.is_some() || c.cell.mode == CellMode::Black);
                Some((c.outlet.clone(), piece))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_top_panics_holds_and_resumes_to_the_current_cells() {
    let mut sub = Subtree::new(parse(DOC).unwrap(), ClockChoice::Parent);
    assert_eq!(cells(&sub.fire("one", 1)).len(), 2);

    let out = sub.panic(2);
    assert!(sub.panicked());
    let [e] = out.as_slice() else { panic!("one envelope: {out:?}") };
    let Message::Panic(p) = &e.message else { panic!("{e:?}") };
    assert_eq!((p.black, p.stop), (Some(true), Some(true)), "black and silent");
    assert!(e.to.is_none(), "to everyone in the session: every deck");

    // the operator goes on: the core moves, the stage stays black
    assert!(cells(&sub.fire("two", 3)).is_empty(), "nothing reaches the stage while the panic holds");
    assert!(cells(&sub.go(4)).is_empty());
    assert_eq!(sub.conductor().current(), Some("two"));

    let back = cells(&sub.resume(5));
    assert!(!sub.panicked());
    assert_eq!(back, [("left".to_string(), Some("scene:crypt".to_string())), ("right".to_string(), Some("scene:choir".to_string()))], "the cells the core holds now");
    assert!(sub.resume(6).is_empty(), "a second resume says nothing");
    let log: Vec<&Logged> = sub.conductor().log().map(|e| &e.what).collect();
    assert!(log.contains(&&Logged::Panicked) && log.contains(&&Logged::Resumed));

    assert_eq!(cells(&sub.fire("one", 7)).len(), 2, "after resume the rows reach the stage again");
}
