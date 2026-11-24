//! A show's rows, columns and a cell's content in the Show workspace, edited in place in the rows band: a row, a column or
//! a group renamed in its head (ctrl+r, Enter, a second press on the selected head, its right click), columns grouped
//! (ctrl+g or their head's right click), a row added under the last or after one and removed from its head's right click,
//! a cell's content picked in the cell (its typeahead: Enter, a press on the selected cell, the cell
//! inspector's piece field) or dropped from the scene list, delete empties a cell (a stop slot). The cell inspector keeps
//! the mode chips, the knobs, the question and the filter. Every edit goes through deck-show's `rows::edit` and
//! `roles::edit` (checked there, refused in their words); this file hands the presses and keys in (RULES A1, A2).
use super::roles::{Ask, Typing};
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_show::rows::{self, Edit};
use deck_show::{view, Show};
use winit::keyboard::{Key, NamedKey};

/// the language of the book's labels (the editor's own words are English, main.rs)
pub(super) const LOCALE: &str = "en";

impl Host {
    /// An edit of the shown document's rows or cells: deck-show makes and checks it; the row it names is selected.
    pub(super) fn edit_rows(&mut self, e: &Edit) -> bool {
        self.edit_rows_with(e, None)
    }

    /// several edits as one change (one undo step), or none of them
    pub(super) fn edit_rows_all(&mut self, edits: &[Edit]) -> bool {
        let done = rows::edit_all(self.shown(), edits);
        self.rows_edited(done)
    }

    /// the same, with the knobs of the cell's piece a value edit is checked against
    pub(super) fn edit_rows_with(&mut self, e: &Edit, knobs: Option<&[deck_show::values::Knob]>) -> bool {
        let done = rows::edit_with(self.shown(), e, knobs);
        self.rows_edited(done)
    }

    fn rows_edited(&mut self, done: Result<rows::Edited, String>) -> bool {
        match done {
            Ok(done) => {
                self.status = done.said;
                if let Some(id) = &done.row {
                    self.row = done.show.rows.iter().position(|r| r.id == *id).unwrap_or(0);
                }
                self.role = None;
                self.changed(done.show);
                true
            }
            Err(why) => {
                self.refuse(why);
                false
            }
        }
    }

    /// a column, group or name edit: deck-show's `roles::edit`
    fn edit_columns(&mut self, e: &deck_show::roles::Edit) -> bool {
        match deck_show::roles::edit(self.shown(), e) {
            Ok(done) => {
                self.status = done.said;
                self.changed(done.show);
                true
            }
            Err(why) => {
                self.refuse(why);
                false
            }
        }
    }

    /// the row inspected and the column whose cell is selected (ids)
    pub(super) fn selected(&self) -> (Option<String>, Option<String>) {
        let show = self.shown();
        let row = show.rows.get(self.row).map(|r| r.id.clone());
        let col = self.cell().cloned().filter(|c| show.outlet(c).is_some() || show.seat(c).is_some());
        (row, col)
    }

    pub(super) fn rows_press(&mut self, hit: Hit) {
        let (row, col) = self.selected();
        match hit {
            Hit::RowAdd => {
                self.edit_rows(&Edit::Add { after: row });
            }
            Hit::RowName => {
                if let Some(id) = row {
                    self.typing = Some((Typing::RowName(id.clone()), Field::with(&id)));
                }
            }
            Hit::RowRemove => self.asking = row.map(Ask::RemoveRow),
            // a press on the one selected cell picks its content in place; with shift its column joins the selection
            Hit::Cell(r, c) => {
                let again = r == self.row && self.role.is_none() && self.cells.len() == 1 && view::plan(self.shown()).columns.get(c).is_some_and(|x| self.cells[0] == x.id);
                self.select_cell(r, c, false);
                if again && self.asked_question().is_none() {
                    self.open_pieces();
                }
            }
            Hit::CellAlso(r, c) => self.select_cell(r, c, true),
            // the one selected column's head, or the selected group's, pressed again: renamed in place
            Hit::Column(c) => {
                let again = self.group.is_none() && self.columns.len() == 1 && view::plan(self.shown()).columns.get(c).is_some_and(|x| self.columns[0] == x.id);
                self.select_column(c, false);
                if again {
                    self.rename_selected();
                }
            }
            Hit::ColumnAlso(c) => self.select_column(c, true),
            Hit::Group(g) => {
                let m = view::plan(self.shown());
                if let Some(group) = m.groups.get(g) {
                    if self.group.as_deref() == Some(group.name.as_str()) {
                        self.rename_selected();
                        return;
                    }
                    self.columns = m.columns[group.first..group.first + group.span].iter().map(|c| c.id.clone()).collect();
                    self.group = Some(group.name.clone());
                    self.cells.clear();
                    self.role = None;
                }
            }
            Hit::CellFact(i) => {
                let (Some(row), Some(column)) = (row, col) else { return };
                if let Some(fact) = rows::cell_condition(self.shown(), &row, &column).and_then(|c| c.fact(i)) {
                    self.edit_rows(&Edit::CellFact { row, column, fact });
                }
            }
            Hit::AddressPick(k) => self.address = (self.address != Some(k)).then_some(k),
            Hit::AddressFact(i) => {
                let (Some(row), Some(address)) = (row, self.address) else { return };
                if let Some(fact) = rows::address_condition(self.shown(), &row, address).and_then(|c| c.fact(i)) {
                    self.edit_rows(&Edit::AddressFact { row, address, fact });
                }
            }
            Hit::CellMode(i) => {
                let (Some(row), Some(column)) = (row, col) else { return };
                let Some(m) = rows::modes(self.shown(), &row, &column).into_iter().nth(i) else { return };
                if m.picks {
                    self.open_pieces();
                } else {
                    self.edit_rows(&Edit::Mode { row, column, mode: m.mode });
                }
            }
            Hit::CellPiece => self.open_pieces(),
            Hit::CellShot => self.open_shots(),
            _ => {}
        }
    }

    /// A column head pressed: it alone is selected (a role's inspector shows a role); with shift or ctrl it joins the
    /// selection or leaves it.
    fn select_column(&mut self, c: usize, also: bool) {
        let Some(id) = view::plan(self.shown()).columns.get(c).map(|x| x.id.clone()) else { return };
        self.group = None;
        self.cells.clear();
        match (also, self.columns.iter().position(|x| *x == id)) {
            (true, Some(at)) => {
                self.columns.remove(at);
            }
            (true, None) => self.columns.push(id.clone()),
            (false, _) => self.columns = vec![id.clone()],
        }
        let role = self.shown().outlet(&id).is_some_and(|o| o.kind == deck_show::OutletKind::Stage);
        self.role = (role && self.columns.last() == Some(&id)).then_some(id);
        self.typing = None;
    }

    /// the selected cell plays `piece`: whether the core took it
    pub(super) fn pick_piece(&mut self, piece: &str) -> bool {
        let (Some(row), Some(column)) = self.selected() else { return false };
        self.edit_rows(&Edit::Piece { row, column, piece: piece.to_string() })
    }

    /// the selected cell's camera shot (none: the scene's opening), checked by deck-show
    pub(super) fn pick_shot(&mut self, shot: Option<&str>) -> bool {
        let (Some(row), Some(column)) = self.selected() else { return false };
        self.edit_rows(&Edit::Shot { row, column, shot: shot.map(str::to_string) })
    }

    /// ctrl+r: what is selected is renamed in place – the group, else the column, else the row
    pub(super) fn rename_selected(&mut self) {
        if let Some(g) = self.group.clone() {
            self.typing = Some((Typing::GroupName(g.clone()), Field::with(&g)));
        } else if let Some(c) = self.columns.last().cloned().filter(|c| self.shown().outlet(c).is_some()) {
            let name = self.shown().outlet(&c).map(|o| o.name().to_string()).unwrap_or_default();
            self.typing = Some((Typing::ColumnName(c), Field::with(&name)));
        } else {
            self.rows_press(Hit::RowName);
        }
    }

    /// ctrl+g: the selected columns one group, named next in place (the core refuses columns that stand apart)
    pub(super) fn group_selected(&mut self) {
        let ids = self.selected_columns();
        if self.edit_columns(&deck_show::roles::Edit::Group { ids: ids.clone(), name: String::new() }) {
            self.group = ids.first().and_then(|c| self.shown().outlet(c)).and_then(|o| o.group.clone());
            self.rename_selected();
        }
    }

    pub(super) fn ungroup_selected(&mut self) {
        let ids = self.selected_columns();
        if self.edit_columns(&deck_show::roles::Edit::Ungroup { ids }) {
            self.group = None;
        }
    }

    /// the selected columns in the matrix's order
    fn selected_columns(&self) -> Vec<String> {
        view::plan(self.shown()).columns.into_iter().map(|c| c.id).filter(|c| self.columns.contains(c)).collect()
    }

    /// a name typed in place and entered: a column's, a group's
    pub(super) fn name_typed(&mut self, what: &Typing, text: &str) {
        match what {
            Typing::ColumnName(id) => {
                self.edit_columns(&deck_show::roles::Edit::Label { id: id.clone(), label: text.to_string() });
            }
            Typing::GroupName(from) if self.edit_columns(&deck_show::roles::Edit::GroupName { from: from.clone(), to: text.to_string() }) => {
                self.group = Some(text.trim().to_string());
            }
            _ => {}
        }
    }

    /// The Show workspace's keys while nothing is typed: ctrl+r renames in place, ctrl+g groups the selected columns,
    /// Enter picks the selected cell's content in place (no cell selected: renames the selected head), Delete empties
    /// the selected cells (stop slots). True = taken.
    pub(super) fn show_key(&mut self, key: &Key, ctrl: bool) -> bool {
        match key {
            Key::Character(c) if ctrl && c.eq_ignore_ascii_case("r") => self.rename_selected(),
            Key::Character(c) if ctrl && c.eq_ignore_ascii_case("g") => self.group_selected(),
            Key::Named(NamedKey::Enter) if self.selected().1.is_some() => self.open_pieces(),
            Key::Named(NamedKey::Enter) if !self.columns.is_empty() || self.selected().0.is_some() => self.rename_selected(),
            Key::Named(NamedKey::Delete) if self.selected().1.is_some() => {
                let (Some(row), columns) = self.selection() else { return false };
                for column in columns {
                    self.edit_rows(&Edit::Mode { row: row.clone(), column, mode: rows::Mode::Black });
                }
            }
            _ => return false,
        }
        true
    }

    /// A check run: the row authored as the operator would – added and named in place when missing, its cell on `column`
    /// selected, `piece` typed into the cell's typeahead and entered; no piece: `play` pressed, which opens it.
    pub(super) fn script_author(&mut self, row: &str, column: &str, piece: Option<&str>) {
        if self.shown().row(row).is_none() {
            self.press(Hit::RowAdd);
            self.press(Hit::RowName);
            if let Some((_, f)) = self.typing.as_mut() {
                *f = Field::with(row);
            }
            self.type_field(&Key::Named(NamedKey::Enter));
        }
        let plan = view::plan(self.shown());
        let (Some(r), Some(c)) = (plan.rows.iter().position(|x| x.id == row), plan.columns.iter().position(|x| x.id == column)) else { return };
        self.press(Hit::Cell(r, c));
        match piece {
            Some(p) => {
                self.open_pieces();
                for ch in p.chars() {
                    self.context_key(&Key::Character(ch.to_string().into()));
                }
                self.context_key(&Key::Named(NamedKey::Enter));
            }
            None => {
                if let Some(i) = rows::modes(self.shown(), row, column).iter().position(|m| m.mode == rows::Mode::Play) {
                    self.press(Hit::CellMode(i));
                }
            }
        }
    }
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// a field you press to type into or to open something: its label on the left, the value (or what is typed) on the right
pub(super) fn field(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), label: &str, value: &str, typing: Option<&str>) -> Rect {
    let (font, row) = (t.size.font, t.size.tree_row);
    d.text(x, y + (row - font * 1.25) * 0.5, font, label, t.color.muted);
    let r = Rect { x: x + w * 0.38, y: y + 2.0, w: w * 0.62, h: row - 4.0 };
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.control);
    if typing.is_some() {
        d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.0), t.color.text);
    }
    let text = typing.map_or_else(|| value.to_string(), |s| format!("{s}|"));
    d.text(r.x + 6.0, r.y + (r.h - font * 1.25) * 0.5, font, &fit(d, &text, font, r.w - 12.0), t.color.text);
    r
}

/// A name typed in place over `r` (a row's, a column's or a group's head): the field and its caret.
pub(super) fn in_place(d: &mut DrawList, t: &Tokens, r: Rect, text: &str, px: f32) {
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.control);
    d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.0), t.color.text);
    d.text(r.x + 4.0, r.y + (r.h - px * 1.25) * 0.5, px, &fit(d, &format!("{text}|"), px, r.w - 8.0), t.color.text);
}

/// The cell inspector of the selected cell (its title, the mode chips, a stage outlet's piece field – or, #551, the question
/// a cell asks –, who gets it). The height used.
pub fn cell_side(d: &mut DrawList, t: &Tokens, host: &mut Host, (x, y, w): (f32, f32, f32), show: &Show, sel: &str) -> f32 {
    let Some(col) = host.cell().cloned() else { return 0.0 };
    let Some(insp) = rows::cell_inspector(show, sel, &col) else { return 0.0 };
    let (pad, row) = (t.size.pad, t.size.tree_row);
    let top = y;
    let mut y = y + controls::section(d, t, x, y, w, &insp.title.to_uppercase(), false);
    let names: Vec<&str> = insp.modes.iter().map(|m| m.word).collect();
    let on: Vec<bool> = insp.modes.iter().map(|m| m.on).collect();
    let (chips, used) = controls::chips(d, t, (x, y, w), &names, &on);
    for (i, r) in chips.into_iter().enumerate() {
        host.hits.push((r, Hit::CellMode(i)));
    }
    y += used + pad * 0.5;
    if let Some(piece) = &insp.piece {
        let r = field(d, t, (x, y, w), rows::PIECE, &host.marked(piece), None);
        host.hits.push((r, Hit::CellPiece));
        y += row;
        if let Some(shot) = &insp.shot {
            let r = field(d, t, (x, y, w), rows::SHOT, shot, None);
            host.hits.push((r, Hit::CellShot));
            y += row;
        }
    }
    if let Some(p) = &insp.knobs_of {
        y += super::values::side(d, t, host, (x, y, w), show, (sel, &col), (p, insp.warning.as_deref()));
    }
    // #551: the question the cell asks – the form, or the story's snapshot (ask.rs)
    if let Some(q) = host.asked_question() {
        y += super::ask::question_side(d, t, host, (x, y, w), &q);
    }
    // #553: who in the column gets it – the one condition control, every fact, no group chip
    if let Some(c) = &insp.filter {
        let (chips, used) = super::condition::draw(d, t, (x, y, w), rows::FILTER, c);
        for (i, r) in chips.into_iter().enumerate() {
            host.hits.push((r, Hit::CellFact(i)));
        }
        y += used;
    }
    y + pad * 0.5 - top
}

/// One line of a picker in the stage's place: its label, its detail, whether it is what is set now, whether a press picks it.
pub(super) struct Line<'a> {
    pub label: &'a str,
    pub detail: &'a str,
    pub now: bool,
    pub can: bool,
}

/// What a picker in the stage's place shows (#537 the piece, #539 the event – one look): its heading, its typed field
/// (the label, the query, the text while typed, its place), then the lines (a press on line `i` is `pick(i)`; none match:
/// `none`) or, where there is no list, why not and what to type.
pub(super) struct Picker<'a> {
    pub title: &'a str,
    pub field: (&'a str, &'a str, Option<&'a str>, Hit),
    pub list: Result<Vec<Line<'a>>, (&'a str, &'a str)>,
    pub none: &'a str,
    pub pick: fn(usize) -> Hit,
}

pub(super) fn draw_picker(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, p: Picker, pointer: (f32, f32)) {
    let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.tree_row);
    let mut y = a.y + pad;
    d.text(a.x + pad, y, small, &fit(d, p.title, small, a.w - 2.0 * pad), t.color.muted);
    y += small * 1.8;
    let (label, query, typing, hit) = p.field;
    let r = field(d, t, (a.x + pad, y, (a.w - 2.0 * pad).min(520.0)), label, query, typing);
    host.hits.push((r, hit));
    y += row + pad * 0.5;
    d.rect(a.x, y - 4.0, a.w, 1.0, t.color.line);
    let list = match p.list {
        Ok(list) => list,
        Err((why, type_it)) => {
            d.text(a.x + pad, y + 4.0, font, &fit(d, why, font, a.w - 2.0 * pad), t.color.warn);
            d.text(a.x + pad, y + 4.0 + font * 1.7, font, &fit(d, type_it, font, a.w - 2.0 * pad), t.color.muted);
            return;
        }
    };
    if list.is_empty() {
        d.text(a.x + pad, y + 4.0, font, p.none, t.color.muted);
    }
    for (i, l) in list.iter().enumerate() {
        if y + row > a.y + a.h {
            d.text(a.x + pad, y + 2.0, small, &format!("{} more …", list.len() - i), t.color.muted);
            break;
        }
        let whole = Rect { x: a.x, y, w: a.w, h: row };
        if l.now {
            d.rect(whole.x, whole.y, whole.w, whole.h, t.color.selected);
        } else if l.can && whole.has(pointer.0, pointer.1) {
            d.rect(whole.x, whole.y, whole.w, whole.h, t.color.hover);
        }
        let ty = y + (row - font * 1.25) * 0.5;
        d.text(a.x + pad, ty, font, &fit(d, l.label, font, a.w * 0.4 - pad), if l.can { t.color.text } else { t.color.muted });
        d.text(a.x + a.w * 0.4, ty + 1.0, small, &fit(d, l.detail, small, a.w * 0.6 - pad), t.color.muted);
        d.rect(a.x, y + row - 1.0, a.w, 1.0, t.color.line);
        if l.can {
            host.hits.push((whole, (p.pick)(i)));
        }
        y += row;
    }
}


#[cfg(test)]
pub(super) mod tests {
    use super::super::{api, author, Hit, Host};
    use deck_kit::base::draw::DrawList;
    use deck_kit::base::tokens::Tokens;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use winit::keyboard::{Key, NamedKey};

    /// a toon's book with two published scenes and a character (deck-show rows.rs has the full case)
    pub(in crate::show) const BOOK: &str = r#"{ "address_book": 2, "toon": "catpope", "version": "v1", "source": "en", "entries": [
        { "ref": "character:catpope", "address": "character:catpope", "kind": "character", "name": "catpope", "labels": { "en": "Cat Pope" }, "origin": { "key": "character:catpope", "state": "published" } },
        { "ref": "scene:nave", "address": "scene:nave", "kind": "scene", "name": "nave", "labels": { "en": "Nave" }, "origin": { "key": "scene:nave", "state": "published" }, "version": 2 },
        { "ref": "scene:vigil", "address": "scene:vigil", "kind": "scene", "name": "vigil", "labels": { "en": "Vigil" }, "origin": { "key": "scene:vigil", "state": "published" }, "version": 1 }
    ] }"#;

    /// `/api/shows` as the server answers it, in memory: PUT keeps the content as the next version, GET gives it
    pub(in crate::show) fn fake_api() -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/api", l.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut kept, mut version) = (None::<serde_json::Value>, 1);
            for mut s in l.incoming().flatten() {
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut line = String::new();
                r.read_line(&mut line).unwrap();
                let mut length = 0;
                loop {
                    let mut h = String::new();
                    r.read_line(&mut h).unwrap();
                    if h.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = v.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                r.read_exact(&mut body).unwrap();
                let (code, answer) = match (line.split(' ').next(), kept.clone()) {
                    (Some("PUT"), _) => {
                        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
                        kept = v.get("content").cloned();
                        version += 1;
                        (200, serde_json::json!({ "version": { "id": version } }))
                    }
                    (Some("GET"), Some(doc)) => (200, serde_json::json!({ "content": doc, "version": { "id": version } })),
                    _ => (404, serde_json::json!({ "error": "not_found" })),
                };
                let text = answer.to_string();
                let _ = write!(s, "HTTP/1.1 {code} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
            }
        });
        base
    }

    pub(in crate::show) const W: f32 = 1600.0;
    pub(in crate::show) const H: f32 = 900.0;

    /// one frame of the Show tab, then a press on the first place that is `want`
    pub(in crate::show) fn press(host: &mut Host, want: impl Fn(&Hit) -> bool) {
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        host.hits.clear();
        author::draw(&mut d, &t, host, W, H, (0.0, 0.0));
        super::super::scenes::panel(&mut d, &t, host, &mut crate::library::Library::new(std::env::temp_dir().join("deck-editor-rows-library")), (0.0, 0.0));
        let (r, hit) = host.hits.iter().rev().find(|(_, h)| want(h)).copied().unwrap_or_else(|| panic!("no such place: {:?}", host.hits.iter().map(|(_, h)| h).collect::<Vec<_>>()));
        assert_eq!(host.hit(r.x + r.w * 0.5, r.y + r.h * 0.5), Some(hit), "the place answers its press");
        host.press(hit);
    }

    pub(in crate::show) fn typed(host: &mut Host, text: &str) {
        for c in text.chars() {
            host.type_key(&Key::Character(c.to_string().into()));
        }
        host.type_key(&Key::Named(NamedKey::Enter));
    }

    /// #537's done-when, the editor's half: from nothing to a row with a play cell naming a published scene picked from the
    /// book, by the places the operator presses; saved and loaded through /api/shows; fired by the row's ▶ in the same tab
    #[test]
    fn a_show_is_made_from_nothing_saved_loaded_and_fired_in_place() {
        let cache = std::env::temp_dir().join(format!("deck-editor-rows-{}", std::process::id()));
        let base = fake_api();
        let mut host = Host::new(None, None, None, base.clone());
        host.api = api::Api::with_cache(base, cache.join("shows"));
        host.book = Some(Arc::new(deck_space::rig::book::Book::parse(BOOK).unwrap()));
        assert!(host.shown().rows.is_empty(), "from nothing: one role, no row");
        host.toons.list = vec!["catpope".into()];
        press(&mut host, |h| *h == Hit::ToonPick(0));
        assert_eq!(host.shown().home(), "catpope", "its toon picked in the scene list");

        press(&mut host, |h| *h == Hit::RowAdd);
        assert_eq!(host.status, "row 1 row added: manual, every column keeps");
        // the selected row's name in its head, pressed again: renamed in place
        press(&mut host, |h| *h == Hit::RowName);
        assert!(frame_texts(&mut host).iter().any(|t| t == "row|"), "the field in the head");
        for _ in 0..3 {
            host.type_key(&Key::Named(NamedKey::Backspace));
        }
        typed(&mut host, "opening");
        assert_eq!(host.shown().rows[0].id, "opening");
        // the cell on `left` in the rows band, then `play`: it names no piece yet, so its typeahead opens in the cell
        press(&mut host, |h| *h == Hit::Cell(0, 0));
        assert_eq!(host.cell().map(String::as_str), Some("left"));
        press(&mut host, |h| *h == Hit::CellMode(0));
        assert_eq!(host.context_lines(), ["Nave", "Vigil"]);
        assert_eq!(host.sheet, None, "nothing in the stage's place");
        // typed to find, Enter picks the first found
        typed(&mut host, "nav");
        assert_eq!(host.status, "row 1 opening, left: scene:nave", "Enter picks the first scene found");
        assert!(host.context.is_none(), "the typeahead closes on a pick");
        let made = host.shown().clone();
        let fixture = deck_show::parse(include_str!("../../../show/fixtures/first-row.show.json")).unwrap();
        assert_eq!(made, fixture, "the document the headless scenario fires");
        // a press on the selected cell opens it again; a line chosen by the arrows; the inspector's piece field the same
        press(&mut host, |h| *h == Hit::Cell(0, 0));
        assert_eq!(host.context_lines(), ["Nave", "Vigil"]);
        host.type_key(&Key::Named(NamedKey::ArrowDown));
        host.type_key(&Key::Named(NamedKey::Enter));
        assert_eq!(host.status, "row 1 opening, left: scene:vigil");
        press(&mut host, |h| *h == Hit::CellPiece);
        host.type_key(&Key::Named(NamedKey::Enter));
        assert_eq!(host.status, "row 1 opening, left: scene:nave");

        host.save_waiting();
        assert_eq!(host.save_pill(), "saved · v2");
        host.sub = deck_show::Subtree::new(deck_show::parse(r#"{ "id": "new-show", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }], "rows": [] }"#).unwrap(), deck_show::ClockChoice::Parent);
        press(&mut host, |h| *h == Hit::Load);
        assert_eq!(host.status, "loaded new-show@1");
        assert_eq!(host.shown(), &fixture, "loaded as saved");

        // the row's ▶ in the rows band – the conductor plays the scene on left and logs the fire
        press(&mut host, |h| *h == Hit::Fire(0));
        let m = deck_show::view::matrix(&host.sub);
        assert!(m.rows[0].current && m.rows[0].cells[0].on && m.rows[0].cells[0].text == "scene:nave", "{m:?}");
        assert_eq!(deck_show::view::log(host.sub.log(), 1)[0].1, "fired opening (▶)");
        let _ = std::fs::remove_dir_all(&cache);
    }

    fn frame_texts(host: &mut Host) -> Vec<String> {
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        host.hits.clear();
        author::draw(&mut d, &t, host, W, H, (0.0, 0.0));
        d.texts().into_iter().map(str::to_string).collect()
    }

    pub(in crate::show) fn host_with(doc: &str) -> Host {
        let mut host = Host::new(None, None, None, "http://127.0.0.1:9/api".into());
        host.sub = deck_show::Subtree::new(deck_show::parse(doc).unwrap(), deck_show::ClockChoice::Parent);
        host
    }

    /// a show id that does not load opens the empty show and the status says why (no panic)
    #[test]
    fn an_unknown_show_id_opens_empty_and_says_why() {
        let host = Host::new(None, Some("no-such-show-637".into()), None, "http://127.0.0.1:9/api".into());
        assert_eq!(host.shown().id, "new-show");
        assert!(host.status.contains("no-such-show-637"), "{}", host.status);
        assert_eq!(host.unloaded.as_deref(), Some(host.status.as_str()));
        assert!(Host::new(None, None, None, "http://127.0.0.1:9/api".into()).unloaded.is_none());
    }

    /// the show's background typed in its inspector: set through the core, refused in its words, taken off
    #[test]
    fn the_shows_background_is_typed_in_the_inspector() {
        let mut host = Host::new(None, None, None, "http://127.0.0.1:9/api".into());
        press(&mut host, |h| *h == Hit::Background);
        typed(&mut host, "#204060");
        assert_eq!(host.status, "the show's background: #204060 – over every scene");
        assert_eq!(host.shown().background, Some(deck_space::Background::colour("#204060")));
        press(&mut host, |h| *h == Hit::Background);
        for _ in 0..7 {
            host.type_key(&Key::Named(NamedKey::Backspace));
        }
        typed(&mut host, "#zz");
        assert!(host.status.starts_with("not changed: '#zz' is no colour"), "{}", host.status);
        assert_eq!(host.shown().background, Some(deck_space::Background::colour("#204060")), "as it was");
        press(&mut host, |h| *h == Hit::Background);
        for _ in 0..7 {
            host.type_key(&Key::Named(NamedKey::Backspace));
        }
        typed(&mut host, "");
        assert_eq!(host.shown().background, None);
    }

    /// the play pill's tempo kept in the show: in its document (saved with it), the same one twice changes nothing
    #[test]
    fn the_pills_tempo_is_kept_in_the_show() {
        use deck_core::tempo::Source;
        let mut host = Host::new(None, None, None, "http://127.0.0.1:9/api".into());
        assert_eq!(host.tempo().1, None);
        let to = deck_show::Tempo::of(96.0, Source::Midi);
        assert!(host.keep_tempo(to));
        assert_eq!(host.status, "the show's tempo: 96.0 BPM, midi");
        assert_eq!(host.tempo(), (host.shown().id.clone(), Some(to)));
        host.status.clear();
        assert!(host.keep_tempo(to) && host.status.is_empty(), "unchanged: no edit");
        let text = serde_json::to_string(host.shown()).unwrap();
        assert_eq!(deck_show::document::open(&text).unwrap().tempo, Some(to), "it round-trips");
    }

    /// #553: the one condition control, by the chips the operator presses – a role's admission (any one fact)
    #[test]
    fn the_condition_control_sets_who_may_take_a_role() {
        let mut host = Host::new(None, None, None, "http://127.0.0.1:9/api".into());
        press(&mut host, |h| *h == Hit::Role(0));
        let c = deck_show::roles::condition(host.shown(), "left").unwrap();
        assert_eq!((c.mode, c.words.as_str()), (deck_ui::filter::Mode::Any, "guest and up"));
        let chip = |host: &Host, word: &str| deck_show::roles::condition(host.shown(), "left").unwrap().chip(word).unwrap();
        let member = chip(&host, "member");
        press(&mut host, |h| *h == Hit::RoleFact(member));
        assert_eq!(host.status, "left: guest and up", "a guest still lets anyone in");
        let guest = chip(&host, "guest");
        press(&mut host, |h| *h == Hit::RoleFact(guest));
        assert_eq!(host.status, "left: member");
        assert_eq!(host.shown().outlets[0].admits.as_ref().and_then(|a| a.account.clone()), Some(vec![protocol::Account::Member]), "a member admits");
    }

    /// #553: a row address's filter set and cleared by its chips, under the address pressed in the row inspector
    #[test]
    fn the_condition_control_sets_and_clears_an_addresss_filter() {
        let mut host = host_with(include_str!("../../../show/fixtures/top-toy.show.json"));
        host.row = 2;
        press(&mut host, |h| *h == Hit::AddressPick(1));
        assert_eq!(host.address, Some(1));
        let chip = |host: &Host, word: &str| deck_show::rows::address_condition(host.shown(), "yellowmouse-enters", 1).unwrap().chip(word).unwrap();
        let phone = chip(&host, "device phone");
        press(&mut host, |h| *h == Hit::AddressFact(phone));
        assert_eq!(host.status, "row 3 yellowmouse-enters, address 2: location:* > close_doors where device phone");
        let left = chip(&host, "group left");
        press(&mut host, |h| *h == Hit::AddressFact(left));
        assert_eq!(host.status, "row 3 yellowmouse-enters, address 2: location:* > close_doors where group left · device phone");
        for word in ["device phone", "group left"] {
            let i = chip(&host, word);
            press(&mut host, |h| *h == Hit::AddressFact(i));
        }
        assert_eq!(host.shown().rows[2].addresses[1].filter, None, "every chip off: no filter");
        press(&mut host, |h| *h == Hit::AddressPick(1));
        assert_eq!(host.address, None, "pressed again: closed");
    }

    /// #553: a cell's filter in the cell inspector – an `ask` row's two columns take it at once
    #[test]
    fn the_condition_control_sets_a_cells_filter_on_both_columns_of_its_question() {
        let mut host = host_with(include_str!("../../../show/fixtures/vote-cell.show.json"));
        let left = deck_show::view::plan(host.shown()).columns.iter().position(|c| c.id == "left").unwrap();
        press(&mut host, |h| *h == Hit::Cell(1, left));
        let c = deck_show::rows::cell_condition(host.shown(), "crossroads", "left").unwrap();
        assert_eq!(c.chip("group left"), None, "the column is the group");
        let browser = c.chip("device browser").unwrap();
        // the cell inspector with the question's form is taller than its panel: scrolled down to the filter
        author::draw(&mut DrawList::new(0.6), &Tokens::default(), &mut host, W, H, (0.0, 0.0));
        let v = host.show_panels.floats.view("inspector").unwrap();
        host.show_panels.floats.wheel(v.content.x + 4.0, v.content.y + 4.0, -100.0);
        press(&mut host, |h| *h == Hit::CellFact(browser));
        assert_eq!(host.status, "row 2 crossroads, left · right: device browser · member · live");
        let row = host.shown().row("crossroads").unwrap();
        assert_eq!(row.r#where.get("right").and_then(|f| f.device.as_deref()), Some("browser"));
        assert_eq!(row.r#where.get("left"), row.r#where.get("right"), "one filter per question");
    }

    #[test]
    fn a_row_is_removed_after_its_prompt_and_without_a_book_the_piece_is_typed() {
        let mut host = Host::new(None, None, None, "http://127.0.0.1:9/api".into());
        host.book_why = Some("no address book – the editor reads one with --toon <toon> (or --book FILE)".into());
        press(&mut host, |h| *h == Hit::RowAdd);
        press(&mut host, |h| *h == Hit::RowAdd);
        assert_eq!(host.shown().rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["row", "row-2"]);
        press(&mut host, |h| *h == Hit::Cell(1, 0));
        press(&mut host, |h| *h == Hit::CellPiece);
        assert!(host.context_lines().is_empty(), "no book: nothing listed, the typed text is the piece");
        typed(&mut host, "nave");
        assert!(host.status.starts_with("not changed: row 'row-2', 'left'"), "the core reads the typed piece: {}", host.status);
        assert!(host.context.is_some(), "refused: the typeahead stays");
        for _ in 0.."nave".len() {
            host.type_key(&Key::Named(NamedKey::Backspace));
        }
        typed(&mut host, "scene:crypt");
        assert_eq!(host.status, "row 2 row-2, left: scene:crypt");
        let (head, _) = host.hits.iter().rev().find(|(_, h)| *h == Hit::Row(1)).copied().unwrap();
        assert!(host.context(head.x + head.w - 2.0, head.y + 2.0));
        typed(&mut host, "remove");
        press(&mut host, |h| *h == Hit::AskYes);
        assert_eq!(host.status, "row row-2 removed with its 1 cells");
        assert_eq!((host.shown().rows.len(), host.row), (1, 0));
    }

    /// A scene cell's camera shot picked in place: the shots the editor knows of that scene, the scene's opening first;
    /// typed `shot:<id>` where it knows none; a cell that plays no scene says to pick the scene first
    #[test]
    fn a_scene_cells_shot_is_picked_in_place() {
        let mut host = Host::new(None, None, None, "http://127.0.0.1:9/api".into());
        press(&mut host, |h| *h == Hit::RowAdd);
        press(&mut host, |h| *h == Hit::Cell(0, 0));
        host.open_shots();
        assert_eq!(host.status, deck_show::rows::NO_SCENE_SHOT);
        press(&mut host, |h| *h == Hit::CellPiece);
        typed(&mut host, "scene:crypt");
        host.cameras.insert("scene:crypt".into(), vec![("wide".into(), "20° 8° 6 m · 30 mm".into()), ("iso".into(), "45° 35° 12 m · ortho 5 m".into())]);
        press(&mut host, |h| *h == Hit::CellShot);
        assert_eq!(host.context_lines(), ["the scene's opening", "wide", "iso"]);
        typed(&mut host, "iso");
        assert_eq!(host.status, "row 1 row, left: scene:crypt > shot:iso");
        assert!(host.context.is_none());
        host.cameras.clear();
        press(&mut host, |h| *h == Hit::CellShot);
        assert_eq!(host.context_lines(), ["the scene's opening"]);
        typed(&mut host, "shot:close");
        assert_eq!(host.status, "row 1 row, left: scene:crypt > shot:close", "typed: the deck's camera reads it");
        press(&mut host, |h| *h == Hit::CellShot);
        host.type_key(&Key::Named(NamedKey::Enter));
        assert_eq!(host.status, "row 1 row, left: scene:crypt", "the opening again");
    }
}
