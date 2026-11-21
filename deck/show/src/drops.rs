//! A Library piece let go on the row's node view: the cell it becomes the piece of. Over a cell's node it is that cell;
//! over the canvas between the nodes it is the row's first free column (a cell that keeps what plays) that takes it.
//! The change is `rows::Edit::Piece`, the rows band's; a node that is no cell, a cell that takes no piece and a row
//! without a free column are refused in words. A view only says which node of the graph it shows the pointer is on.
use crate::document::{Cell, Row, Show};
use crate::rows::{self, Edit};
use deck_kit::base::graph::Graph;

/// The edit that makes `piece` the piece of the cell under the pointer (`over`: a node of the graph shown, none: the
/// canvas), checked as `rows::edit` checks it; or why it does not land there.
pub fn piece(show: &Show, row: &Row, g: &Graph, over: Option<usize>, piece: &str) -> Result<Edit, String> {
    let on = |column: &str| {
        let e = Edit::Piece { row: row.id.clone(), column: column.into(), piece: piece.into() };
        rows::edit(show, &e).map(|_| e)
    };
    match over {
        Some(node) => {
            let n = g.nodes.get(node).ok_or("no such node")?;
            if !show.outlets.iter().any(|o| o.id == n.id) {
                return Err(format!("{} takes no piece: drop it on a cell of the row", n.title));
            }
            on(&n.id)
        }
        None => {
            let free = show.outlets.iter().filter(|o| matches!(row.cells.get(&o.id), None | Some(Cell::Keep)));
            free.filter(|o| g.node(&o.id).is_some())
                .find_map(|o| on(&o.id).ok())
                .ok_or_else(|| format!("no free column in row '{}' takes {piece}: drop it on a cell to change its piece", row.id))
        }
    }
}

/// the node of the graph shown that lights while `piece` is carried at `over`: the cell it would land on
pub fn target(show: &Show, row: &Row, g: &Graph, over: Option<usize>, piece: &str) -> Option<usize> {
    match self::piece(show, row, g, over, piece) {
        Ok(Edit::Piece { column, .. }) => g.node(&column),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse;
    use crate::nodes::{folded, row_graph, CLOCK, ROW};

    const SHOW: &str = r#"{ "id": "d", "version": 1, "toons": ["catpope"],
        "outlets": [{ "id": "wall" }, { "id": "floor" }, { "id": "sky" }, { "id": "lamps", "kind": "device" }],
        "rows": [
            { "id": "one", "cells": { "wall": { "mode": "play", "piece": "scene:keys", "values": { "speed": 0.5 } }, "sky": { "mode": "black" } } },
            { "id": "full", "cells": { "wall": { "mode": "play", "piece": "scene:keys" }, "floor": { "mode": "black" }, "sky": { "mode": "black" } } }] }"#;

    fn show() -> Show {
        parse(SHOW).unwrap()
    }

    #[test]
    fn a_piece_over_a_cells_node_is_the_rows_bands_edit_on_that_cell() {
        let s = show();
        let g = row_graph(&s, 0, &[]);
        for (column, said) in [("wall", "row 1 one, wall: scene:bells – the values for the other piece went"), ("floor", "row 1 one, floor: scene:bells"), ("sky", "row 1 one, sky: scene:bells")] {
            let e = piece(&s, &s.rows[0], &g, g.node(column), "scene:bells").unwrap();
            assert_eq!(e, Edit::Piece { row: "one".into(), column: column.into(), piece: "scene:bells".into() });
            assert_eq!(rows::edit(&s, &e).unwrap().said, said);
            assert_eq!(target(&s, &s.rows[0], &g, g.node(column), "scene:bells"), g.node(column), "the cell lights");
        }
        for kind in ["character:catpope", "item:lamp", "place:nave"] {
            assert!(piece(&s, &s.rows[0], &g, g.node("floor"), kind).is_ok(), "{kind}");
        }
    }

    #[test]
    fn a_node_that_is_no_cell_and_a_cell_that_takes_no_piece_say_why_and_stay_dark() {
        let s = show();
        let g = row_graph(&s, 0, &[]);
        for node in [ROW, CLOCK] {
            let why = piece(&s, &s.rows[0], &g, g.node(node), "scene:bells").unwrap_err();
            assert!(why.ends_with("takes no piece: drop it on a cell of the row"), "{why}");
            assert_eq!(target(&s, &s.rows[0], &g, g.node(node), "scene:bells"), None);
        }
        let device = piece(&s, &s.rows[0], &g, g.node("lamps"), "scene:bells").unwrap_err();
        assert_eq!(Some(device), rows::edit(&s, &Edit::Piece { row: "one".into(), column: "lamps".into(), piece: "scene:bells".into() }).err(), "the edit's own words");
        assert_eq!(target(&s, &s.rows[0], &g, g.node("lamps"), "scene:bells"), None);
        // a cell that asks a question
        let vote = parse(include_str!("../fixtures/vote-cell.show.json")).unwrap();
        let (i, row) = vote.rows.iter().enumerate().find(|(_, r)| r.id == "crossroads").unwrap();
        let g = row_graph(&vote, i, &[]);
        assert_eq!(piece(&vote, row, &g, g.node("left"), "scene:nave").unwrap_err(), "not changed: row 'crossroads', 'left' asks q1 – it takes a piece once it leaves the question");
    }

    #[test]
    fn a_piece_over_the_canvas_is_a_new_cell_in_the_first_free_column_or_refused_when_none_is() {
        let s = show();
        let g = row_graph(&s, 0, &[]);
        let e = piece(&s, &s.rows[0], &g, None, "scene:bells").unwrap();
        assert_eq!(e, Edit::Piece { row: "one".into(), column: "floor".into(), piece: "scene:bells".into() }, "floor keeps: free; the device outlet is not");
        assert_eq!(target(&s, &s.rows[0], &g, None, "scene:bells"), g.node("floor"));
        let g = row_graph(&s, 1, &[]);
        assert_eq!(piece(&s, &s.rows[1], &g, None, "scene:bells").unwrap_err(), "no free column in row 'full' takes scene:bells: drop it on a cell to change its piece");
        assert_eq!(target(&s, &s.rows[1], &g, None, "scene:bells"), None);
    }

    #[test]
    fn a_folded_group_takes_no_piece_and_hides_its_free_column_from_the_canvas() {
        let mut s = show();
        s.rows[0].nodes.groups.push(crate::canvas::Group { id: "g1".into(), label: "pair".into(), colour: "#808080".into(), nodes: vec!["floor".into(), "sky".into()], folded: true });
        let (g, _) = folded(row_graph(&s, 0, &[]), &s, 0);
        let pill = g.node("group:g1").expect("the pill");
        assert!(piece(&s, &s.rows[0], &g, Some(pill), "scene:bells").unwrap_err().ends_with("takes no piece: drop it on a cell of the row"));
        assert!(piece(&s, &s.rows[0], &g, None, "scene:bells").unwrap_err().starts_with("no free column"), "floor is inside the pill");
    }
}
