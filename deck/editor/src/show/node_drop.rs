//! Library cards over the node view (deck-show drops.rs decides, this file draws and forwards): a card carried over the
//! canvas lights the node of the cell it would become the piece of – the node under the pointer, over the canvas
//! between the nodes the row's first free column –, and where it would not land the core's sentence stands at the
//! pointer. Let go, it lands through the rows band's drop (scenes.rs `drop_pieces`).
use super::nodes::NodeView;
use super::Host;
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_show::rows::Edit;
use deck_show::{drops, view};

/// the node view's panel
const PANEL: &str = "nodes";

impl NodeView {
    /// the node under `p`, the one drawn last first
    pub(super) fn node_at(&self, p: (f32, f32)) -> Option<usize> {
        self.drawn.iter().rposition(|(_, r)| self.on_screen(*r).has(p.0, p.1))
    }

    /// where node `i` is on screen
    pub(super) fn node_place(&self, i: usize) -> Option<Rect> {
        self.drawn.get(i).map(|(_, r)| self.on_screen(*r))
    }
}

impl Host {
    /// the node view's canvas is what lies under `at`: shown, and no other panel over it there
    fn over_nodes(&self, at: (f32, f32)) -> bool {
        self.two_d && self.sheet.is_none() && self.nodes.shown && self.nodes.canvas.has(at.0, at.1) && !self.show_panels.floats.covered(PANEL, at.0, at.1)
    }

    /// what `piece` let go at `at` would change, as the core says; none: the node view is not there
    fn node_drop_edit(&self, piece: &str, at: (f32, f32)) -> Option<Result<Edit, String>> {
        let row = self.shown().rows.get(self.row).filter(|_| self.over_nodes(at))?;
        Some(drops::piece(self.shown(), row, &self.nodes.graph, self.nodes.node_at(at), piece))
    }

    /// The cell `piece` let go at `at` on the node view lands on (row, column as the matrix counts), or the core's
    /// sentence why not; none: the node view is not there.
    pub(super) fn node_drop(&self, piece: &str, at: (f32, f32)) -> Option<Result<(usize, usize), String>> {
        Some(self.node_drop_edit(piece, at)?.and_then(|e| match e {
            Edit::Piece { column, .. } => view::plan(self.shown()).columns.iter().position(|c| c.id == column).map(|c| (self.row, c)).ok_or(format!("{column}: no column of the rows band")),
            _ => Err("not a piece".into()),
        }))
    }

    /// where a check run lets a card go in the node view: `node:<id>` its node's middle, `canvas` a free spot of the
    /// canvas beside every node
    pub fn node_drop_spot(&self, target: &str) -> Option<(f32, f32)> {
        let n = &self.nodes;
        if let Some(id) = target.strip_prefix("node:") {
            let r = n.node_place(n.graph.node(id)?)?;
            return Some((r.x + r.w * 0.5, r.y + r.h * 0.5));
        }
        let c = n.canvas;
        let spots = (1..8).flat_map(|i| (1..8).map(move |k| (c.x + c.w * k as f32 / 8.0, c.y + c.h * (8 - i) as f32 / 8.0)));
        spots.into_iter().find(|p| n.node_at(*p).is_none() && self.over_nodes(*p))
    }
}

/// A card carried over the canvas: the node it would land on lit; where it would not land, the core's sentence at the
/// pointer.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &Host, canvas: Rect) {
    let cursor = host.cursor;
    let Some(carried) = host.carried.as_ref().filter(|_| host.over_nodes(cursor)) else { return };
    let landing = carried.clone().and_then(|piece| host.node_drop_edit(&piece, cursor).unwrap_or(Err(String::new())));
    match landing {
        Ok(Edit::Piece { column, .. }) => {
            let Some(r) = host.nodes.graph.node(&column).and_then(|i| host.nodes.node_place(i)) else { return };
            let radius = t.size.radius * 1.5 * host.nodes.view.zoom;
            let go = t.color.go;
            d.round_rect(r.x, r.y, r.w, r.h, radius, [go[0], go[1], go[2], go[3] * 0.12]);
            d.outline(r.x, r.y, r.w, r.h, (radius, 2.0), go);
        }
        Ok(_) => {}
        Err(why) if why.is_empty() => {}
        // under the card's own label (deck-kit carry.rs)
        Err(why) => controls::chip(d, t, (cursor.0 + 12.0, cursor.1 + 22.0 + t.size.font * 1.25), canvas.x + canvas.w, &why, true),
    }
}

#[cfg(test)]
mod tests {
    use super::super::author;
    use super::super::rows::tests::{host_with, H, W};
    use super::*;
    use deck_kit::base::draw::Prim;
    use deck_kit::base::graph::Direction;
    use deck_kit::base::tokens::Theme;
    use deck_show::Cell;

    const SHOW: &str = r#"{ "id": "dropped", "version": 1, "toons": ["catpope"],
        "outlets": [{ "id": "wall" }, { "id": "floor" }, { "id": "sky" }],
        "rows": [
            { "id": "one", "cells": { "wall": { "mode": "play", "piece": "scene:keys" }, "sky": { "mode": "black" } } },
            { "id": "two", "cells": { "wall": { "mode": "play", "piece": "scene:keys" }, "floor": { "mode": "black" }, "sky": { "mode": "black" } } }] }"#;

    fn frame(host: &mut Host) -> DrawList {
        let (mut d, t) = (DrawList::new(0.6), Tokens::new(Theme::Night, 16.0));
        host.hits.clear();
        author::draw(&mut d, &t, host, W, H, (-10.0, -10.0));
        d
    }

    fn host(dir: Direction) -> Host {
        let mut host = host_with(SHOW);
        host.nodes.direction = dir;
        frame(&mut host);
        host
    }

    fn piece(host: &Host, row: usize, column: &str) -> Option<String> {
        match host.shown().rows[row].cells.get(column) {
            Some(Cell::Play { piece, .. }) => piece.clone(),
            _ => None,
        }
    }

    /// how many outlines of the go colour the frame drew: the lit targets
    fn lit(d: &DrawList) -> usize {
        let go = Tokens::new(Theme::Night, 16.0).color.go;
        d.prims().iter().filter(|p| matches!(p, Prim::Outline { color, thick, .. } if *color == go && *thick == 2.0)).count()
    }

    fn carry(host: &mut Host, card: Result<&str, &str>, to: &str) -> (f32, f32) {
        let at = host.node_drop_spot(to).unwrap_or_else(|| panic!("{to} on the canvas"));
        host.carried = Some(card.map(String::from).map_err(String::from));
        host.cursor = at;
        at
    }

    #[test]
    fn a_card_carried_over_a_cells_node_lights_it_and_let_go_is_its_piece_left_to_right_and_top_to_bottom() {
        for dir in [Direction::Lr, Direction::Tb] {
            let mut host = host(dir);
            assert_eq!(lit(&frame(&mut host)), 0, "nothing carried: nothing lit");
            for (card, column) in [("scene:bells", "wall"), ("character:catpope", "floor"), ("item:lamp", "sky"), ("place:nave", "floor")] {
                let at = carry(&mut host, Ok(card), &format!("node:{column}"));
                assert_eq!(lit(&frame(&mut host)), 1, "{dir:?} {card} over {column}: its node is lit");
                assert!(host.drop_pieces(&[card.to_string()], at.0, at.1));
                assert_eq!(piece(&host, 0, column).as_deref(), Some(card), "{dir:?}: {}", host.status);
                assert_eq!((host.refused, host.cell().map(String::as_str)), (None, Some(column)), "landed and selected");
                host.carried = None;
                frame(&mut host);
            }
            assert_eq!(host.status, "row 1 one, floor: place:nave", "the rows band's edit");
        }
    }

    #[test]
    fn a_node_that_is_no_cell_stays_dark_and_says_why_at_the_pointer() {
        let mut host = host(Direction::Lr);
        let before = host.shown().clone();
        let at = carry(&mut host, Ok("scene:bells"), "node:row");
        let d = frame(&mut host);
        assert_eq!(lit(&d), 0);
        assert!(d.texts().contains(&"one takes no piece: drop it on a cell of the row"), "{:?}", d.texts());
        assert!(host.drop_pieces(&["scene:bells".into()], at.0, at.1), "the node view took the drop");
        assert_eq!(host.shown(), &before);
        assert_eq!(host.refused, Some(at), "refused where it was let go");
        assert!(host.bad && host.status == "one takes no piece: drop it on a cell of the row", "{}", host.status);
    }

    #[test]
    fn a_card_show_does_not_take_says_the_cores_sentence_at_the_pointer_and_lights_nothing() {
        let mut host = host(Direction::Lr);
        let why = deck_catalog::drop::lands(deck_catalog::drop::View::ShowRow, deck_catalog::library::Kind::Image).unwrap_err();
        assert_eq!(why, "images in a Show row become a reference node: not built yet");
        carry(&mut host, Err(&why), "node:wall");
        let d = frame(&mut host);
        assert_eq!(lit(&d), 0);
        assert!(d.texts().contains(&why.as_str()), "{:?}", d.texts());
    }

    #[test]
    fn over_the_canvas_the_first_free_column_lights_and_takes_it_and_a_full_row_refuses_in_words() {
        let mut host = host(Direction::Lr);
        let at = carry(&mut host, Ok("scene:bells"), "canvas");
        assert_eq!(host.nodes.node_at(at), None, "beside every node");
        assert_eq!(lit(&frame(&mut host)), 1, "floor keeps: the free column");
        assert!(host.drop_pieces(&["scene:bells".into()], at.0, at.1));
        assert_eq!(piece(&host, 0, "floor").as_deref(), Some("scene:bells"), "{}", host.status);
        // no column free any more
        frame(&mut host);
        let at = carry(&mut host, Ok("scene:vigil"), "canvas");
        let d = frame(&mut host);
        let why = "no free column in row 'one' takes scene:vigil: drop it on a cell to change its piece";
        assert_eq!(lit(&d), 0);
        assert!(d.texts().contains(&why), "{:?}", d.texts());
        assert!(host.drop_pieces(&["scene:vigil".into()], at.0, at.1));
        assert!(host.bad && host.status == why && host.refused == Some(at), "{}", host.status);
    }

    #[test]
    fn several_cards_fill_the_cell_and_the_ones_below_and_a_locked_show_takes_none() {
        let mut host = host(Direction::Tb);
        let at = carry(&mut host, Ok("scene:bells"), "node:sky");
        assert!(host.drop_pieces(&["scene:bells".into(), "scene:vigil".into()], at.0, at.1));
        assert_eq!((piece(&host, 0, "sky").as_deref(), piece(&host, 1, "sky").as_deref()), (Some("scene:bells"), Some("scene:vigil")));
        frame(&mut host);
        host.locked = true;
        let before = host.shown().clone();
        let at = carry(&mut host, Ok("scene:keys"), "node:sky");
        assert!(host.drop_pieces(&["scene:keys".into()], at.0, at.1));
        assert_eq!(host.shown(), &before);
        assert_eq!(host.status, super::super::live::LOCKED);
        // beside the node view nothing is taken
        assert!(!host.drop_pieces(&["scene:keys".into()], -5.0, -5.0));
    }
}
