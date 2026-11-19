//! Show's side of the library (library.rs): what it lists – the book's published scenes, then the toon's drafts never
//! published (marked: new_scene.rs) –, the panel's content while the show names no toon or has no book, and a card
//! dropped on a cell: deck-show's `Edit::Piece`, the same edit as a pick in the picker; a refusal shows the core's
//! sentence where it was dropped. Nothing here decides which cell takes a piece (RULES A1, A2).
use super::rows::LOCALE;
use super::Host;
use crate::library::{Library, PANEL};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls;
use deck_show::rows::{self, Edit, PieceView};
use deck_show::view;

/// what the panel says without a book
pub const NO_BOOK: &str = "the address book is still on its way";

impl Host {
    /// the book's published scenes, then the drafts (none without a book)
    pub(super) fn scene_list(&self) -> Vec<PieceView> {
        let Some(b) = self.book.as_ref() else { return Vec::new() };
        let mut list = rows::pieces(b, "", LOCALE);
        list.extend(rows::draft_pieces(&self.drafts_unlisted(), ""));
        list
    }

    /// the library's entries for Show
    pub fn library_entries(&self) -> Vec<deck_catalog::library::Entry> {
        crate::library::of_show(self.scene_list(), self.book.as_deref())
    }

    /// the cell under the pointer (row, column as the matrix counts) – where a piece would land
    pub(super) fn drop_target(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        self.hits.iter().rev().find_map(|(r, h)| match h {
            super::Hit::Cell(row, col) if r.has(x, y) => Some((*row, *col)),
            _ => None,
        })
    }

    /// A piece let go at (x, y): on a cell it selects the cell and becomes its piece, or the core says why not, there.
    /// False = no cell there.
    pub fn drop_piece(&mut self, piece: &str, x: f32, y: f32) -> bool {
        let Some((r, c)) = self.drop_target(x, y) else { return false };
        let plan = view::plan(self.shown());
        let (Some(row), Some(column)) = (plan.rows.get(r).map(|x| x.id.clone()), plan.columns.get(c).map(|x| x.id.clone())) else { return true };
        self.select_cell(r, c, false);
        if !self.edit_rows(&Edit::Piece { row, column, piece: piece.to_string() }) {
            self.refused = Some((x, y));
        }
        true
    }

    /// a scene opened from the library: Build edits it
    pub fn open_scene(&mut self, piece: &str) {
        self.ask_scene_build(piece);
    }
}

/// Show's library panel: the toon picker in place while the show names none, why there is no book yet, else the library
/// with "new scene".
pub fn panel(d: &mut DrawList, t: &Tokens, host: &mut Host, lib: &mut Library, pointer: (f32, f32)) {
    let (pad, small) = (t.size.pad, t.size.small);
    if host.show_toon().is_some() && host.book.is_some() {
        if let Some(o) = host.show_panels.open(d, t, PANEL, pointer, 0) {
            lib.draw(d, t, o.area, pointer, Some(super::new_scene::NEW));
            host.show_panels.close(d, o, &mut lib.browser.hits);
        }
        return;
    }
    lib.browser.hits.clear();
    let Some(o) = host.show_panels.open(d, t, PANEL, pointer, host.hits.len()) else { return };
    let a = o.area;
    if host.show_toon().is_none() {
        super::toons::in_place(d, t, host, (a.x + pad, a.y + pad, a.w - 2.0 * pad));
    } else {
        let why = host.book_why.clone().unwrap_or_else(|| NO_BOOK.into());
        d.text(a.x + pad, a.y + pad, small, &controls::fit(&why, ((a.w - 2.0 * pad) / d.advance(small)).max(0.0) as usize), t.color.warn);
    }
    host.show_panels.close(d, o, &mut host.hits);
}

/// the panel's height while it only says why there is no book (else it measures itself)
pub fn note_height(t: &Tokens, host: &Host) -> Option<f32> {
    (host.show_toon().is_some() && host.book.is_none()).then_some(2.0 * t.size.pad + t.size.small * 1.6)
}

#[cfg(test)]
pub(in crate::show) mod tests {
    use super::super::rows::tests::{host_with, press, BOOK};
    use super::super::{Hit, Host};
    use crate::library::Library;
    use deck_catalog::library::Purpose;
    use deck_kit::editor::library::Did;
    use std::sync::Arc;

    const VOTE: &str = include_str!("../../../show/fixtures/vote-cell.show.json");

    fn with_book(doc: &str) -> Host {
        let mut host = host_with(doc);
        host.book = Some(Arc::new(deck_space::rig::book::Book::parse(BOOK).unwrap()));
        host
    }

    /// one Show frame with its library panel; the library's entries are the host's
    pub(in crate::show) fn frame(host: &mut Host, lib: &mut Library) {
        let (mut d, t) = (deck_kit::base::draw::DrawList::new(0.6), deck_kit::base::tokens::Tokens::default());
        host.hits.clear();
        super::super::author::draw(&mut d, &t, host, 2560.0, 1440.0, (0.0, 0.0));
        lib.set(Purpose::Show, host.library_entries());
        super::panel(&mut d, &t, host, lib, (0.0, 0.0));
    }

    /// the middle of the cell on `column` in `row`, as the last frame drew it
    fn cell_at(host: &Host, row: &str, column: &str) -> (f32, f32) {
        let plan = deck_show::view::plan(host.shown());
        let (r, c) = (plan.rows.iter().position(|x| x.id == row).unwrap(), plan.columns.iter().position(|x| x.id == column).unwrap());
        let (rect, _) = host.hits.iter().find(|(_, h)| *h == Hit::Cell(r, c)).unwrap();
        (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5)
    }

    /// the library's card of `address` carried to the cell on `column` in `row` and let go there
    pub(in crate::show) fn drag(host: &mut Host, lib: &mut Library, address: &str, row: &str, column: &str) -> (f32, f32) {
        // twice: the second frame lays the library out at the height the first drew
        frame(host, lib);
        frame(host, lib);
        let i = lib.entries.iter().position(|e| e.address == address).unwrap_or_else(|| panic!("{address} in {:?}", lib.entries));
        let (r, _) = *lib.browser.hits.iter().find(|(_, h)| *h == deck_kit::editor::library::Hit::Card(i)).expect("its card");
        static CLOCK: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let now = f64::from(CLOCK.fetch_add(1, std::sync::atomic::Ordering::Relaxed)) * 10.0;
        lib.browser.press(r.x + 4.0, r.y + 4.0, now);
        let at = cell_at(host, row, column);
        lib.browser.drag_to(at.0, at.1);
        assert!(lib.carrying().is_some(), "Show takes a scene");
        let Some(Did::Drop { entry, at }) = lib.browser.release(at.0, at.1) else { panic!("a drop") };
        assert!(host.drop_piece(&lib.entries[entry].address.clone(), at.0, at.1), "a cell under the pointer");
        at
    }

    #[test]
    fn the_library_lists_the_books_scenes_and_characters_or_why_there_are_none() {
        let mut host = host_with(VOTE);
        host.book_why = Some("no address book – the editor reads one with --toon <toon> (or --book FILE)".into());
        let mut lib = Library::new(std::env::temp_dir().join("deck-editor-show-library"));
        frame(&mut host, &mut lib);
        assert!(lib.browser.hits.is_empty(), "without a book: the note, no card");
        let mut host = with_book(VOTE);
        frame(&mut host, &mut lib);
        let listed: Vec<&str> = lib.entries.iter().map(|e| e.address.as_str()).collect();
        assert_eq!(listed, ["scene:nave", "scene:vigil", "character:catpope"]);
        assert!(lib.browser.hits.iter().any(|(_, h)| *h == deck_kit::editor::library::Hit::Action), "new scene");
    }

    #[test]
    fn a_scene_dropped_on_a_cell_is_its_piece() {
        let mut host = with_book(VOTE);
        let mut lib = Library::new(std::env::temp_dir().join("deck-editor-show-library"));
        drag(&mut host, &mut lib, "scene:vigil", "opening", "middle");
        assert_eq!(host.status, "row 1 opening, middle: scene:vigil");
        assert_eq!((host.row, host.cell().map(String::as_str)), (0, Some("middle")), "the cell it landed on is selected");
        assert_eq!(host.refused, None);
        let before = host.shown().clone();
        assert!(!host.drop_piece("scene:nave", 1.0, 1.0), "let go over nothing: nothing changes");
        assert_eq!(host.shown(), &before);
    }

    #[test]
    fn a_cell_that_takes_no_piece_refuses_in_the_cores_words_where_it_was_dropped() {
        let mut host = with_book(VOTE);
        let mut lib = Library::new(std::env::temp_dir().join("deck-editor-show-library"));
        let before = host.shown().clone();
        let at = drag(&mut host, &mut lib, "scene:nave", "crossroads", "lamps");
        assert!(host.status.starts_with("not changed: "), "{}", host.status);
        assert_eq!(host.refused, Some(at), "the ring where it was dropped");
        assert_eq!(host.shown(), &before);
        drag(&mut host, &mut lib, "scene:nave", "crossroads", "left");
        assert!(host.status.starts_with("not changed: "), "an ask cell takes no piece: {}", host.status);
        assert_eq!(host.shown(), &before);
        press(&mut host, |h| *h == Hit::RowAdd);
        assert_eq!(host.refused, None, "the next press clears it");
    }
}
