//! The scene list beside the rows band in Show: the book's published scenes, then the toon's drafts never published
//! (marked: new_scene.rs), filtered as typed; its "new scene" opens the scene sheet. A scene dragged onto a cell is deck-show's `Edit::Piece`, the same edit as a pick in the picker; a refusal shows the core's sentence where the
//! scene was dropped. Nothing here decides which cell takes a piece (RULES A1, A2).
use super::roles::Typing;
use super::rows::{field, LOCALE};
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_show::rows::{self, Edit, PieceView};
use deck_show::view;

/// a scene on its way to a cell: what it plays, what people read, where the pointer is
#[derive(Debug, Clone, PartialEq)]
pub struct Dragging {
    pub piece: String,
    pub label: String,
    pub at: Option<(f32, f32)>,
}

/// what the list says without a book
pub const NO_BOOK: &str = "the address book is still on its way";

impl Host {
    pub(super) fn scenes_press(&mut self, hit: Hit) {
        match hit {
            Hit::Scene(i) => {
                if let Some(p) = self.scene_list().get(i) {
                    self.dragging = Some(Dragging { piece: p.address.clone(), label: p.label.clone(), at: None });
                }
            }
            Hit::SceneFind => self.typing = Some((Typing::Scenes, Field::with(&self.scene_query))),
            _ => {}
        }
    }

    /// the filter field's text: what is being typed, else the query kept
    pub(super) fn scene_query_now(&self) -> String {
        match &self.typing {
            Some((Typing::Scenes, f)) => f.text.clone(),
            _ => self.scene_query.clone(),
        }
    }

    /// the published scenes of the book that match the filter, then the drafts (none without a book)
    pub(super) fn scene_list(&self) -> Vec<PieceView> {
        let Some(b) = self.book.as_ref() else { return Vec::new() };
        let query = self.scene_query_now();
        let mut list = rows::pieces(b, &query, LOCALE);
        list.extend(rows::draft_pieces(&self.drafts_unlisted(), &query));
        list
    }

    /// the pointer moves: a scene being dragged follows it
    pub fn drag_to(&mut self, x: f32, y: f32) {
        if let Some(d) = self.dragging.as_mut() {
            d.at = Some((x, y));
        }
    }

    /// the cell under the pointer (row, column as the matrix counts) – where a scene would land
    pub(super) fn drop_target(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        self.hits.iter().rev().find_map(|(r, h)| match h {
            Hit::Cell(row, col) if r.has(x, y) => Some((*row, *col)),
            _ => None,
        })
    }

    /// The button let go. A scene dropped on a cell selects it and becomes its piece, or the core says why not, there;
    /// dropped elsewhere nothing happens. True = a drag was under way.
    pub fn drop_at(&mut self, x: f32, y: f32) -> bool {
        let Some(drag) = self.dragging.take() else { return false };
        let Some((r, c)) = self.drop_target(x, y) else { return true };
        let plan = view::plan(self.shown());
        let (Some(row), Some(column)) = (plan.rows.get(r).map(|x| x.id.clone()), plan.columns.get(c).map(|x| x.id.clone())) else { return true };
        self.select_cell(r, c, false);
        if !self.edit_rows(&Edit::Piece { row, column, piece: drag.piece }) {
            self.refused = Some((x, y, self.status.clone()));
        }
        true
    }
}

/// The scene list in `a`: its heading, the filter field, the scenes found (a press starts a drag), or why there is none.
pub fn list(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, pointer: (f32, f32)) {
    let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.tree_row);
    let w = a.w - 2.0 * pad;
    let fit = |d: &DrawList, s: &str, px: f32, w: f32| controls::fit(s, (w / d.advance(px)).max(0.0) as usize);
    d.rect(a.x, a.y, a.w, a.h, t.color.surface);
    d.rect(a.x, a.y, 1.0, a.h, t.color.line);
    let mut y = a.y + pad;
    d.text(a.x + pad, y, small, "SCENES", t.color.muted);
    let nw = d.advance(small) * super::new_scene::NEW.chars().count() as f32 + 2.0 * pad;
    let new = Rect { x: a.x + a.w - nw - pad, y: y - 3.0, w: nw, h: small * 1.6 };
    d.round_rect(new.x, new.y, new.w, new.h, t.size.radius, if new.has(pointer.0, pointer.1) { t.color.hover } else { t.color.control });
    d.text(new.x + pad, y, small, super::new_scene::NEW, t.color.text);
    host.hits.push((new, Hit::Compose(super::new_scene::ComposeHit::Open)));
    y += small * 1.8;
    if host.book.is_none() {
        let why = host.book_why.clone().unwrap_or_else(|| NO_BOOK.into());
        d.text(a.x + pad, y, small, &fit(d, &why, small, w), t.color.warn);
        return;
    }
    let typing = host.typing.as_ref().filter(|(what, _)| *what == Typing::Scenes).map(|(_, f)| f.text.clone());
    let query = host.scene_query_now();
    let r = field(d, t, (a.x + pad, y, w), "find", &query, typing.as_deref());
    host.hits.push((r, Hit::SceneFind));
    y += row + pad * 0.5;
    let scenes = host.scene_list();
    if scenes.is_empty() {
        d.text(a.x + pad, y + 4.0, small, &fit(d, rows::NONE_FOUND, small, w), t.color.muted);
    }
    let dragged = host.dragging.as_ref().map(|g| g.piece.clone());
    for (i, s) in scenes.iter().enumerate() {
        if y + row > a.y + a.h {
            d.text(a.x + pad, y + 2.0, small, &format!("{} more …", scenes.len() - i), t.color.muted);
            break;
        }
        let line = Rect { x: a.x + 1.0, y, w: a.w - 1.0, h: row };
        if dragged.as_deref() == Some(s.address.as_str()) {
            d.rect(line.x, line.y, line.w, line.h, t.color.selected);
        } else if line.has(pointer.0, pointer.1) {
            d.rect(line.x, line.y, line.w, line.h, t.color.hover);
        }
        let ty = y + (row - font * 1.25) * 0.5;
        d.text(a.x + pad, ty, font, &fit(d, &s.label, font, w * 0.45), t.color.text);
        d.text(a.x + pad + w * 0.48, ty + 1.0, small, &fit(d, &s.detail, small, w * 0.52), t.color.muted);
        host.hits.push((line, Hit::Scene(i)));
        y += row;
    }
}

/// Over everything: the scene being dragged at the pointer, or the core's refusal where a scene was dropped.
pub fn over(d: &mut DrawList, t: &Tokens, host: &Host, (w, _h): (f32, f32)) {
    if let Some((Some((x, y)), label)) = host.dragging.as_ref().map(|g| (g.at, &g.label)) {
        controls::chip(d, t, (x + 12.0, y + 8.0), w, label, false);
    }
    if let Some((x, y, why)) = &host.refused {
        controls::chip(d, t, (*x, *y + 8.0), w, why, true);
    }
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{host_with, press, typed, BOOK};
    use super::super::{Hit, Host};
    use std::sync::Arc;
    use winit::keyboard::{Key, NamedKey};

    const VOTE: &str = include_str!("../../../show/fixtures/vote-cell.show.json");

    fn with_book(doc: &str) -> Host {
        let mut host = host_with(doc);
        host.book = Some(Arc::new(deck_space::rig::book::Book::parse(BOOK).unwrap()));
        host
    }

    fn listed(host: &Host) -> Vec<String> {
        host.scene_list().into_iter().map(|p| p.address).collect()
    }

    /// the middle of the cell on `column` in `row`, as the last frame drew it
    fn cell_at(host: &Host, row: &str, column: &str) -> (f32, f32) {
        let plan = deck_show::view::plan(host.shown());
        let (r, c) = (plan.rows.iter().position(|x| x.id == row).unwrap(), plan.columns.iter().position(|x| x.id == column).unwrap());
        let (rect, _) = host.hits.iter().find(|(_, h)| *h == Hit::Cell(r, c)).unwrap();
        (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5)
    }

    /// a scene of the list pressed, carried to the cell on `column` in `row` and let go there
    fn drag(host: &mut Host, scene: usize, row: &str, column: &str) -> (f32, f32) {
        press(host, false, |h| *h == Hit::Scene(scene));
        assert!(host.dragging.is_some());
        let at = cell_at(host, row, column);
        host.drag_to(at.0, at.1);
        assert!(host.drop_target(at.0, at.1).is_some(), "the cell under the pointer takes the drop");
        assert!(host.drop_at(at.0, at.1));
        assert_eq!(host.dragging, None);
        at
    }

    #[test]
    fn the_list_shows_the_books_scenes_filtered_or_why_there_are_none() {
        let mut host = host_with(VOTE);
        host.book_why = Some("no address book – the editor reads one with --toon <toon> (or --book FILE)".into());
        press(&mut host, false, |h| *h == Hit::RowAdd);
        assert!(listed(&host).is_empty());
        assert!(!host.hits.iter().any(|(_, h)| matches!(h, Hit::Scene(_) | Hit::SceneFind)), "without a book: the note, no field, no line");

        let mut host = with_book(VOTE);
        press(&mut host, false, |h| *h == Hit::SceneFind);
        assert_eq!(listed(&host), ["scene:nave", "scene:vigil"]);
        typed(&mut host, "vig");
        assert_eq!(host.scene_query, "vig");
        assert_eq!(listed(&host), ["scene:vigil"]);
        press(&mut host, false, |h| *h == Hit::SceneFind);
        for _ in 0..3 {
            host.type_key(&Key::Named(NamedKey::Backspace));
        }
        typed(&mut host, "zzz");
        assert!(listed(&host).is_empty(), "nothing found: the list says so");
    }

    #[test]
    fn a_scene_dropped_on_a_cell_is_its_piece() {
        let mut host = with_book(VOTE);
        drag(&mut host, 1, "opening", "middle");
        assert_eq!(host.status, "row 1 opening, middle: scene:vigil");
        assert_eq!((host.row, host.cell().map(String::as_str)), (0, Some("middle")), "the cell it landed on is selected");
        assert_eq!(host.refused, None);
        // let go over nothing: nothing changes
        let before = host.shown().clone();
        press(&mut host, false, |h| *h == Hit::Scene(0));
        assert!(host.drop_at(1.0, 1.0));
        assert_eq!(host.shown(), &before);
    }

    #[test]
    fn a_cell_that_takes_no_piece_refuses_in_the_cores_words_where_it_was_dropped() {
        let mut host = with_book(VOTE);
        let before = host.shown().clone();
        let at = drag(&mut host, 0, "crossroads", "lamps");
        assert!(host.status.starts_with("not changed: "), "{}", host.status);
        assert_eq!(host.refused, Some((at.0, at.1, host.status.clone())), "the sentence where it was dropped");
        assert_eq!(host.shown(), &before);
        drag(&mut host, 0, "crossroads", "left");
        assert!(host.status.starts_with("not changed: "), "an ask cell takes no piece: {}", host.status);
        assert_eq!(host.shown(), &before);
        press(&mut host, false, |h| *h == Hit::RowAdd);
        assert_eq!(host.refused, None, "the next press clears it");
    }
}
