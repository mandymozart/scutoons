//! A show's rows and a cell's piece in the deck editor's Show workspace (#537; SHOW.md §5, §7; SCENES.md 2·6; #267 draft
//! 4): a row is added, renamed and removed here, a cell's mode set and its piece picked – a published scene from the
//! toon's address book (`GET /api/addresses/<toon>`, the editor's book; its scene entries are the published scenes, each
//! with the version listed) or, where the editor has no book, typed as `scene:<id>` and read by the one parser. Every edit
//! changes the document and is checked by `validate` ([`edit`]: a refused edit says why, in its words); #539: the event a
//! row fires on, picked from the toon's story events (story.rs), is an edit here too – how and hold stay as they are. #553: a
//! row address's filter and a cell's column filter are edits here too, shown as deck-ui's one `Condition`. The words, the modes a column offers, the prompt before a
//! remove and the picker's lines are here once; the editor lays them out (RULES A1, A2), like roles.rs (#478).
use crate::choose::{Prompt, Tone};
use crate::document::{Cell, OutletKind, Row, Show, Values, PIECE_KINDS};
use crate::values::{Knob, KnobView};
use crate::view::{address_text, cell_text};
use deck_space::rig::book::Book;
use deck_ui::filter::{Condition, Fact};

/// the tree's line that adds a row
pub const ADD: &str = "new row";
/// the row inspector's name field and its remove
pub const NAME: &str = "name";
pub const REMOVE: &str = "Remove row …";
/// the cell inspector's piece field and what it says while the cell names none
pub const PIECE: &str = "piece";
pub const NO_PIECE: &str = "none – pick a published scene";
/// what the picker says where the editor has no address book (and why not comes from the book's loader)
pub const TYPE_IT: &str = "Type scene:<id> and Enter – the core reads it; a book (--toon) lists the published scenes.";
/// what the picker says when the book lists no published scene that matches
pub const NONE_FOUND: &str = "No published scene matches.";
/// the cell inspector's field of a scene cell's camera shot
pub const SHOT: &str = "shot";
/// a scene cell that picks no shot: the scene opens on its own
pub const NO_SHOT: &str = "the scene's opening";
/// what the shot picker says where the editor does not know the scene's camera
pub const TYPE_SHOT: &str = "Type shot:<id> and Enter – the deck's camera reads it; Build's open scene lists its shots.";
/// a shot asked for where the cell plays no scene
pub const NO_SCENE_SHOT: &str = "A shot belongs to the scene a cell plays – pick the scene first.";

/// A cell's mode as the inspector offers it (SHOW.md §5). Device outlets' sends and looks are not authored here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Play,
    Values,
    Mirror,
    Black,
    Keep,
    Go,
    Free,
    /// #546: a question of the show to this column's audience
    Ask,
}

impl Mode {
    pub fn word(self) -> &'static str {
        match self {
            Mode::Play => "play",
            Mode::Values => "values",
            Mode::Mirror => "mirror",
            Mode::Black => "stop",
            Mode::Keep => "keep",
            Mode::Go => "go",
            Mode::Free => "free",
            Mode::Ask => "ask",
        }
    }

    fn of(cell: &Cell) -> Option<Mode> {
        Some(match cell {
            Cell::Play { .. } => Mode::Play,
            Cell::Values { .. } => Mode::Values,
            Cell::Mirror { .. } => Mode::Mirror,
            Cell::Black => Mode::Black,
            Cell::Keep => Mode::Keep,
            Cell::Go { .. } => Mode::Go,
            Cell::Free => Mode::Free,
            Cell::Ask { .. } => Mode::Ask,
            Cell::Look { .. } => return None,
        })
    }
}

/// One mode chip of the cell inspector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeView {
    pub mode: Mode,
    pub word: &'static str,
    /// the cell's mode now
    pub on: bool,
    /// choosing it opens the piece picker first: `play` on a cell that names no piece yet
    pub picks: bool,
}

/// A change to a show's rows or a cell, made in the Show workspace.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// a new row after `after` (none: after the last), by hand, every column keeps
    Add { after: Option<String> },
    /// its name (the row's id: the face's list follows)
    Rename { id: String, to: String },
    /// the row and its cells go
    Remove { id: String },
    /// the cell's mode; what it held carries over where the new mode holds it (the piece, the values)
    Mode { row: String, column: String, mode: Mode },
    /// the cell plays this piece (a `values` cell expects it instead); values meant for another piece go
    Piece { row: String, column: String, piece: String },
    /// one of the cell's scene's camera shots (its id; none: the scene's own opening), sent as `<scene> > shot:<id>`
    Shot { row: String, column: String, shot: Option<String> },
    /// an event that fires the row, on if it was off and off if it was on (none or empty: none, by hand only); how, hold
    /// and moment stay
    Event { row: String, event: Option<String> },
    /// #553: one fact of the filter of the row's address `address` (its index in the row) on or off; a filter that asks
    /// nothing goes
    AddressFact { row: String, address: usize, fact: Fact },
    /// #553: one fact of `column`'s filter in the row on or off; an `ask` column's filter is its question's – every column
    /// of the row that asks it takes the same (validate's rule); a filter that asks nothing goes
    CellFact { row: String, column: String, fact: Fact },
    /// #551 (SHOW.md §15 5): the selected cells of a row ask a new typed question (the vote form's defaults), for phones
    NewQuestion { row: String, columns: Vec<String> },
    /// #551: the selected cells join a question the row asks ("add to question": also a column that is not adjacent), with
    /// the filter its other columns carry
    AddTo { row: String, columns: Vec<String>, question: String },
    /// #551: the selected cells that ask keep again; a question no cell asks any more goes
    Leave { row: String, columns: Vec<String> },
    /// #551: one field of a question's form in the cell inspector (question.rs `Set`)
    Question { question: String, set: crate::question::Set },
    /// knob `path` of the cell's piece set to `to` (none: the piece's own again), checked against the piece's knobs
    Value { row: String, column: String, path: String, to: Option<f32> },
    /// "take current" – these knob positions (values.rs `current`) written into the cell
    Take { row: String, column: String, positions: Vec<(String, f32)> },
    /// the show's stage background, over what its pieces set (none: theirs)
    Background { to: Option<deck_space::Background> },
    /// the show's master tempo, kept from the play pill (none: the clock as it is)
    Tempo { to: Option<crate::document::Tempo> },
    /// the show's home toon: first in its toons (moved there when it is named already)
    Toon { to: String },
    /// #708: a toon the show uses beside its home, added at the end – or taken out when it is named (the last one stays)
    ToggleToon { toon: String },
    /// a wire from one cell's outlet into another's inlet in the row (wiring.rs); `replaces` = the wire it was lifted from
    Wire { row: String, wire: crate::wiring::CellWire, replaces: Option<crate::wiring::CellWire> },
    /// the wire goes
    Unwire { row: String, wire: crate::wiring::CellWire },
}

/// An edit made: the changed document, what it did in words, the row to select after it.
#[derive(Debug, Clone, PartialEq)]
pub struct Edited {
    pub show: Show,
    pub said: String,
    pub row: Option<String>,
}

fn row_at(show: &Show, id: &str) -> Result<usize, String> {
    show.rows.iter().position(|r| r.id == id).ok_or_else(|| format!("no row '{id}'"))
}

/// the camera shot a `play` cell picks
fn shot_of(cell: Option<&Cell>) -> Option<&str> {
    match cell? {
        Cell::Play { shot, .. } => shot.as_deref(),
        _ => None,
    }
}

/// the piece a cell names, if it names one
fn piece_of(cell: Option<&Cell>) -> Option<&str> {
    match cell? {
        Cell::Play { piece: Some(p), .. } | Cell::Values { piece: p, .. } => Some(p),
        _ => None,
    }
}

/// The piece that plays on `column` when row `at` fires after the rows above it in order: the nearest row above whose
/// cell there names one; a black, a mirror or nothing above: none.
fn playing_before(show: &Show, at: usize, column: &str) -> Option<String> {
    for r in show.rows[..at].iter().rev() {
        match r.cells.get(column) {
            None | Some(Cell::Keep) => continue,
            other => return piece_of(other).map(str::to_string),
        }
    }
    None
}

/// The modes column `column` offers: a stage outlet `play`, `values`, `mirror`, `black`, `keep` (and `ask` when the show
/// has a question, #546); a seat `go`, `free`, `keep`; a device outlet `keep` (its sends and looks are not authored here).
/// The cell's mode now is on.
pub fn modes(show: &Show, row: &str, column: &str) -> Vec<ModeView> {
    let cell = show.row(row).and_then(|r| r.cells.get(column));
    let now = cell.and_then(Mode::of).unwrap_or(Mode::Keep);
    let offered: &[Mode] = match show.outlet(column).map(|o| o.kind) {
        Some(OutletKind::Stage) if !show.questions.is_empty() => &[Mode::Play, Mode::Values, Mode::Mirror, Mode::Black, Mode::Ask, Mode::Keep],
        Some(OutletKind::Stage) => &[Mode::Play, Mode::Values, Mode::Mirror, Mode::Black, Mode::Keep],
        Some(OutletKind::Device) => &[Mode::Keep],
        None if show.seat(column).is_some() => &[Mode::Go, Mode::Free, Mode::Keep],
        None => &[],
    };
    let has_piece = piece_of(cell).is_some();
    offered.iter().map(|&m| ModeView { mode: m, word: m.word(), on: m == now, picks: m == Mode::Play && !has_piece }).collect()
}

/// the cell `mode` makes of `was` on `column` in row `at`
fn cell_for(show: &Show, at: usize, column: &str, mode: Mode, was: Option<&Cell>) -> Result<Cell, String> {
    let values = match was {
        Some(Cell::Play { values, .. } | Cell::Values { values, .. }) => values.clone(),
        _ => Values::new(),
    };
    Ok(match mode {
        Mode::Keep => Cell::Keep,
        Mode::Black => Cell::Black,
        Mode::Free => Cell::Free,
        Mode::Play => match piece_of(was) {
            Some(p) => Cell::Play { piece: Some(p.to_string()), values, send: vec![], shot: shot_of(was).map(str::to_string) },
            None => return Err(format!("{column}: pick a piece – `play` names one")),
        },
        Mode::Values => match piece_of(was).map(str::to_string).or_else(|| playing_before(show, at, column)) {
            Some(piece) => Cell::Values { piece, values },
            None => return Err(format!("{column}: no piece plays there before this row – `values` changes a piece that plays")),
        },
        Mode::Mirror => {
            let row = &show.rows[at];
            let other = show.outlets.iter().filter(|o| o.kind == OutletKind::Stage && o.id != column);
            match other.map(|o| &o.id).find(|o| !matches!(row.cells.get(*o), Some(Cell::Mirror { .. }))) {
                Some(of) => Cell::Mirror { of: of.clone() },
                None => return Err(format!("{column}: no other stage outlet to mirror")),
            }
        }
        Mode::Go => match show.seat(column).and_then(|s| s.uses.rows.first()) {
            Some(r) => Cell::Go { row: r.clone() },
            None => return Err(format!("{column}: the seat uses none of its child's rows")),
        },
        // the question the row asks already (one per row), else the show's first
        Mode::Ask => match asked(&show.rows[at]).or_else(|| show.questions.first().map(|q| q.id.as_str())) {
            Some(q) => Cell::Ask { question: q.to_string() },
            None => return Err(format!("{column}: the show has no question yet")),
        },
    })
}

/// the question row `r` asks, if it asks one
pub(crate) fn asked(r: &Row) -> Option<&str> {
    r.cells.values().find_map(|c| if let Cell::Ask { question } = c { Some(question.as_str()) } else { None })
}

/// the columns whose filter changes with `column`'s: an `ask` column's and every other column of the row that asks the same
/// question (one filter per vote, validate's rule), in the row's order; else `column` alone
fn filtered_with(r: &Row, column: &str) -> Vec<String> {
    match r.cells.get(column) {
        Some(Cell::Ask { question }) => r.cells.iter().filter(|(_, c)| matches!(c, Cell::Ask { question: q } if q == question)).map(|(k, _)| k.clone()).collect(),
        _ => vec![column.to_string()],
    }
}

/// #553: the filter of row `row`'s address `address` as the one condition control shows it (deck-ui `Condition`, every
/// fact holds; the groups offered are the show's roles, deck-show `roles::groups`)
pub fn address_condition(show: &Show, row: &str, address: usize) -> Option<Condition> {
    let a = show.row(row)?.addresses.get(address)?;
    Some(Condition::filter(a.filter.as_ref(), Some(&crate::roles::groups(show))))
}

/// #553: `column`'s filter in row `row` as the cell inspector shows it (every fact holds, no group chip: the column is the
/// group). Offered where a filter does something today – an `ask` cell (the relay tests it, #547) – and where the column
/// already carries one, so it can be cleared; validate allows it on a stage outlet only.
pub fn cell_condition(show: &Show, row: &str, column: &str) -> Option<Condition> {
    let r = show.row(row)?;
    show.outlet(column).filter(|o| o.kind == OutletKind::Stage)?;
    let f = r.r#where.get(column);
    (matches!(r.cells.get(column), Some(Cell::Ask { .. })) || f.is_some()).then(|| Condition::filter(f, None))
}

/// a cell into a row: `keep` is no cell (you write only what changes)
fn put(row: &mut Row, column: &str, cell: Cell) {
    if cell == Cell::Keep {
        row.cells.remove(column);
    } else {
        row.cells.insert(column.to_string(), cell);
    }
}

/// the values of `column`'s cell in row `at`, where the cell names a piece
fn values_mut<'a>(row: &'a mut Row, column: &str) -> Result<&'a mut Values, String> {
    match row.cells.get_mut(column) {
        Some(Cell::Play { piece: Some(_), values, .. } | Cell::Values { values, .. }) => Ok(values),
        _ => Err(format!("{column}: the cell names no piece – a value belongs to a piece")),
    }
}

/// Make `e` on `show`. The document is checked after it (`validate`): an edit that adds a problem is refused, with the
/// problem's words; problems the document had before stay its own (the problems list says them).
pub fn edit(show: &Show, e: &Edit) -> Result<Edited, String> {
    edit_with(show, e, None)
}

/// `edits` made one after another as one change (one undo step): all of them or, at the first refused, none.
pub fn edit_all(show: &Show, edits: &[Edit]) -> Result<Edited, String> {
    let mut done = Edited { show: show.clone(), said: String::new(), row: None };
    for e in edits {
        let next = edit(&done.show, e)?;
        let said = if done.said.is_empty() { next.said } else { format!("{}; {}", done.said, next.said) };
        done = Edited { show: next.show, said, row: next.row.or(done.row) };
    }
    Ok(done)
}

/// [`edit`] with the knobs of the cell's piece (deck-space `piece::knobs`), which a value edit is checked against.
pub fn edit_with(show: &Show, e: &Edit, knobs: Option<&[Knob]>) -> Result<Edited, String> {
    let before = crate::validate::validate(show);
    let mut next = show.clone();
    let (said, row) = match e {
        Edit::Add { after } => {
            let free = |id: &String| show.row(id).is_none();
            let id = (1..).map(|n| if n == 1 { "row".to_string() } else { format!("row-{n}") }).find(free).unwrap_or_default();
            let at = match after {
                Some(a) => row_at(show, a)? + 1,
                None => next.rows.len(),
            };
            next.rows.insert(at, Row { id: id.clone(), fire: Default::default(), cells: Default::default(), addresses: vec![], r#where: Default::default(), wires: vec![], nodes: Default::default() });
            (format!("row {} {id} added: manual, every column keeps", at + 1), Some(id))
        }
        Edit::Rename { id, to } => {
            let i = row_at(&next, id)?;
            let to = to.trim().to_string();
            for r in next.face.rows.iter_mut().filter(|r| *r == id) {
                r.clone_from(&to);
            }
            next.rows[i].id.clone_from(&to);
            let (from, renamed) = (crate::document::row_fired(id), crate::document::row_fired(&to));
            for r in &mut next.rows {
                r.fire.event.rename(&from, &renamed);
            }
            (format!("row {} {id} is called {to}", i + 1), Some(to))
        }
        Edit::Remove { id } => {
            let i = row_at(&next, id)?;
            let gone = next.rows.remove(i);
            next.face.rows.retain(|r| r != id);
            let fired = crate::document::row_fired(id);
            for r in &mut next.rows {
                r.fire.event.remove(&fired);
            }
            let cells = gone.cells.values().filter(|c| !matches!(c, Cell::Keep)).count();
            let select = i.checked_sub(1).or((!next.rows.is_empty()).then_some(0)).map(|j| next.rows[j].id.clone());
            let unasked = crate::question::drop_unasked(show, &mut next);
            (format!("row {id} removed with its {cells} cells{}", crate::question::gone_words(&unasked)), select)
        }
        Edit::Mode { row, column, mode } => {
            let i = row_at(&next, row)?;
            let was_ask = matches!(next.rows[i].cells.get(column), Some(Cell::Ask { .. }));
            let cell = cell_for(&next, i, column, *mode, next.rows[i].cells.get(column))?;
            let text = cell_text(&cell);
            match &cell {
                // #546: a column joining the row's question takes the filter its other columns carry
                Cell::Ask { question } => crate::question::join(&mut next.rows[i], std::slice::from_ref(column), question, None),
                _ => {
                    // #551: a cell that asked takes its filter along; its question goes with its last cell
                    if was_ask {
                        crate::question::left(&mut next.rows[i], column);
                    }
                    put(&mut next.rows[i], column, cell);
                }
            }
            let unasked = crate::question::drop_unasked(show, &mut next);
            (format!("row {} {row}, {column}: {text}{}", i + 1, crate::question::gone_words(&unasked)), Some(row.clone()))
        }
        Edit::Piece { row, column, piece } => {
            let i = row_at(&next, row)?;
            if let Some(Cell::Ask { question }) = next.rows[i].cells.get(column) {
                return Err(format!("not changed: row '{row}', '{column}' asks {question} – it takes a piece once it leaves the question"));
            }
            let piece = piece.trim().to_string();
            let (cell, dropped) = match next.rows[i].cells.get(column) {
                Some(Cell::Values { piece: p, values }) if *p == piece => (Cell::Values { piece: piece.clone(), values: values.clone() }, false),
                Some(Cell::Values { values, .. }) => (Cell::Values { piece: piece.clone(), values: Values::new() }, !values.is_empty()),
                Some(Cell::Play { piece: Some(p), values, shot, .. }) if *p == piece => (Cell::Play { piece: Some(piece.clone()), values: values.clone(), send: vec![], shot: shot.clone() }, false),
                Some(Cell::Play { values, .. }) => (Cell::Play { piece: Some(piece.clone()), values: Values::new(), send: vec![], shot: None }, !values.is_empty()),
                _ => (Cell::Play { piece: Some(piece.clone()), values: Values::new(), send: vec![], shot: None }, false),
            };
            let text = cell_text(&cell);
            put(&mut next.rows[i], column, cell);
            (format!("row {} {row}, {column}: {text}{}", i + 1, if dropped { " – the values for the other piece went" } else { "" }), Some(row.clone()))
        }
        Edit::Shot { row, column, shot } => {
            let i = row_at(&next, row)?;
            let shot = shot.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| s.strip_prefix("shot:").unwrap_or(s).to_string());
            match next.rows[i].cells.get_mut(column) {
                Some(Cell::Play { piece: Some(_), shot: s, .. }) => s.clone_from(&shot),
                _ => return Err(format!("not changed: row '{row}', '{column}' plays no scene – a shot belongs to the scene the cell plays")),
            }
            let text = cell_text(&next.rows[i].cells[column]);
            (format!("row {} {row}, {column}: {text}", i + 1), Some(row.clone()))
        }
        Edit::Event { row, event } => {
            let i = row_at(&next, row)?;
            let fire = &mut next.rows[i].fire.event;
            match event.as_deref().map(str::trim).filter(|e| !e.is_empty()) {
                Some(e) => {
                    fire.toggle(e);
                }
                None => fire.clear(),
            }
            let said = match fire.is_empty() {
                true => format!("row {} {row}: {}", i + 1, crate::story::BY_HAND),
                false => format!("row {} {row} fires on {}", i + 1, fire.as_slice().join(", ")),
            };
            (said, Some(row.clone()))
        }
        Edit::AddressFact { row, address, fact } => {
            let i = row_at(&next, row)?;
            let a = next.rows[i].addresses.get_mut(*address).ok_or_else(|| format!("row {row} has no address {}", address + 1))?;
            let mut f = a.filter.take().unwrap_or_default();
            fact.flip_filter(&mut f);
            a.filter = (!deck_ui::filter::is_empty(&f)).then_some(f);
            (format!("row {} {row}, address {}: {}", i + 1, address + 1, address_text(a)), Some(row.clone()))
        }
        Edit::CellFact { row, column, fact } => {
            let i = row_at(&next, row)?;
            let r = &mut next.rows[i];
            let mut f = r.r#where.get(column).cloned().unwrap_or_default();
            fact.flip_filter(&mut f);
            let columns = filtered_with(r, column);
            for c in &columns {
                if deck_ui::filter::is_empty(&f) {
                    r.r#where.remove(c);
                } else {
                    r.r#where.insert(c.clone(), f.clone());
                }
            }
            (format!("row {} {row}, {}: {}", i + 1, columns.join(" · "), deck_ui::filter::filter_words(r.r#where.get(column))), Some(row.clone()))
        }
        Edit::NewQuestion { row, columns } | Edit::AddTo { row, columns, .. } | Edit::Leave { row, columns } if columns.is_empty() => {
            return Err(format!("row {} {row}: pick the cells that ask it", row_at(show, row)? + 1));
        }
        Edit::NewQuestion { row, columns } => {
            let i = row_at(&next, row)?;
            let id = crate::question::new(&mut next, i, columns)?;
            let to = deck_ui::filter::filter_words(next.rows[i].r#where.get(&columns[0]));
            (format!("row {} {row}: {} {} the new question {id} – {to}", i + 1, columns.join(" · "), if columns.len() == 1 { "asks" } else { "ask" }), Some(row.clone()))
        }
        Edit::AddTo { row, columns, question } => {
            let i = row_at(&next, row)?;
            if next.question(question).is_none() {
                return Err(format!("no question '{question}'"));
            }
            crate::question::join(&mut next.rows[i], columns, question, None);
            let unasked = crate::question::drop_unasked(show, &mut next);
            let to = deck_ui::filter::filter_words(next.rows[i].r#where.get(&columns[0]));
            let said = format!("row {} {row}: {} {} {question} – {to}{}", i + 1, columns.join(" · "), if columns.len() == 1 { "asks" } else { "ask" }, crate::question::gone_words(&unasked));
            (said, Some(row.clone()))
        }
        Edit::Leave { row, columns } => {
            let i = row_at(&next, row)?;
            let went = crate::question::leave(&mut next, i, columns);
            if went.is_empty() {
                return Err(format!("row {} {row}: none of the cells picked asks a question", i + 1));
            }
            let unasked = crate::question::drop_unasked(show, &mut next);
            (format!("row {} {row}: {} {} again{}", i + 1, went.join(" · "), if went.len() == 1 { "keeps" } else { "keep" }, crate::question::gone_words(&unasked)), Some(row.clone()))
        }
        Edit::Question { question, set } => {
            let q = next.questions.iter_mut().find(|q| q.id == *question).ok_or_else(|| format!("no question '{question}'"))?;
            crate::question::set(q, set)?;
            let label = Some(q.label(crate::vote::DEFAULT_LOCALE)).filter(|l| l != question).map(|l| format!("{l} · ")).unwrap_or_default();
            let said = format!("question {question}: {label}{}", crate::question::form(q, crate::vote::DEFAULT_LOCALE).run_line());
            let row = next.rows.iter().find(|r| r.cells.values().any(|c| matches!(c, Cell::Ask { question: x } if x == question))).map(|r| r.id.clone());
            (said, row)
        }
        Edit::Value { row, column, .. } | Edit::Take { row, column, .. } => {
            let i = row_at(&next, row)?;
            let knobs = knobs.ok_or_else(|| format!("{column}: the piece's knobs are not known here – {}", crate::values::LOADING))?;
            let values = values_mut(&mut next.rows[i], column)?;
            let did = match e {
                Edit::Value { path, to, .. } => crate::values::set(values, knobs, path, *to),
                Edit::Take { positions, .. } => crate::values::take(values, knobs, positions).map(|n| format!("took {n} knobs – {}", cell_text(&next.rows[i].cells[column]))),
                _ => unreachable!(),
            };
            (format!("row {} {row}, {column}: {}", i + 1, did.map_err(|why| format!("not changed: {why}"))?), Some(row.clone()))
        }
        Edit::Background { to } => {
            next.background.clone_from(to);
            let said = to.as_ref().map_or_else(|| "the show's background: none – its scenes', else black".to_string(), |b| format!("the show's background: {} – over every scene", b.text()));
            (said, None)
        }
        Edit::Tempo { to } => {
            next.tempo = *to;
            let said = to.map_or_else(|| "the show's tempo: none – the clock as it is".to_string(), |t| format!("the show's tempo: {:.1} BPM, {}", t.bpm, t.source.name()));
            (said, None)
        }
        Edit::Toon { to } => {
            let to = to.trim().to_string();
            if to.is_empty() {
                return Err(format!("not changed: show: {}", crate::validate::NO_TOON));
            }
            next.toons.retain(|t| *t != to);
            next.toons.insert(0, to);
            (toons_said(&next), None)
        }
        Edit::ToggleToon { toon } => {
            let toon = toon.trim().to_string();
            if next.toons.contains(&toon) {
                if next.toons.len() == 1 {
                    return Err(format!("not changed: the show keeps its one toon {toon} – pick another first"));
                }
                next.toons.retain(|t| *t != toon);
            } else {
                next.toons.push(toon);
            }
            (toons_said(&next), None)
        }
        Edit::Wire { row, wire, replaces } => {
            let i = row_at(&next, row)?;
            let wires = &mut next.rows[i].wires;
            if let Some(old) = replaces {
                wires.retain(|w| w != old);
            }
            if wires.contains(wire) {
                return Err(format!("not changed: {} is wired already", wire.words()));
            }
            wires.push(wire.clone());
            (format!("row {} {row}: wired {}", i + 1, wire.words()), Some(row.clone()))
        }
        Edit::Unwire { row, wire } => {
            let i = row_at(&next, row)?;
            let n = next.rows[i].wires.len();
            next.rows[i].wires.retain(|w| w != wire);
            if next.rows[i].wires.len() == n {
                return Err(format!("not changed: {} is no wire of row {row}", wire.words()));
            }
            (format!("row {} {row}: unwired {}", i + 1, wire.words()), Some(row.clone()))
        }
    };
    // a wire whose ends a change took away goes with it
    let pruned = if matches!(e, Edit::Wire { .. }) { Vec::new() } else { crate::wiring::prune(&mut next) };
    let said = if pruned.is_empty() { said } else { format!("{said}; unwired {}", pruned.join(", ")) };
    // the reader it needs, written with the change: an older deck refuses what it would misread
    next.stamp();
    let added: Vec<String> = crate::validate::validate(&next).into_iter().filter(|p| !before.contains(p)).map(|p| p.to_string()).collect();
    match added.first() {
        Some(p) => Err(format!("not changed: {p}")),
        None => Ok(Edited { show: next, said, row }),
    }
}

/// The prompt before a row is removed: what goes with it (its cells, the event it listens to, the face's entry).
pub fn remove_prompt(show: &Show, id: &str) -> Prompt {
    let Some((i, r)) = show.rows.iter().enumerate().find(|(_, r)| r.id == id) else {
        return Prompt { title: format!("No row '{id}'"), warn: false, lines: vec![], confirm: "Remove".into(), can: false };
    };
    let mut lines: Vec<(String, Tone)> = r.cells.iter().filter(|(_, c)| !matches!(c, Cell::Keep)).map(|(col, c)| (format!("{col}: {} goes", cell_text(c)), Tone::Warn)).collect();
    let cells = !lines.is_empty();
    if !cells {
        lines.push(("every column keeps: no cell goes".into(), Tone::Muted));
    }
    for e in r.fire.event.iter().filter(|e| !show.rows.iter().any(|o| o.id != id && o.fire.event.contains(e))) {
        lines.push((format!("the event '{e}' fires no row then"), Tone::Muted));
    }
    let fired = crate::document::row_fired(id);
    for other in show.rows.iter().filter(|o| o.fire.event.contains(&fired)) {
        lines.push((format!("row '{}' no longer fires on it", other.id), Tone::Muted));
    }
    if show.face.rows.contains(&r.id) {
        lines.push(("the face shows it: a parent that seats this show loses it".into(), Tone::Warn));
    }
    lines.push(("The document changes here; Save keeps it.".into(), Tone::Muted));
    Prompt { title: format!("Remove the row {} {id}?", i + 1), warn: cells, lines, confirm: "Remove".into(), can: true }
}

/// The cell inspector: the selected cell of the row inspected, its modes as chips, its piece.
#[derive(Debug, Clone, PartialEq)]
pub struct CellInspector {
    /// `cell · row 1 start · left`
    pub title: String,
    pub modes: Vec<ModeView>,
    /// a stage outlet's piece field: the piece, or [`NO_PIECE`]; none: the column holds no piece (a seat, a device) or
    /// its cell asks a question (the question's form shows instead: question.rs `inspect`)
    pub piece: Option<String>,
    /// a `play` cell's camera shot of its scene: `shot:<id>`, or [`NO_SHOT`]; none: the cell plays no scene
    pub shot: Option<String>,
    /// #553: who in the column gets the cell – [`FILTER`] and the one condition control ([`cell_condition`]); none: not
    /// offered here
    pub filter: Option<Condition>,
    /// the piece whose knobs the cell's values turn (a `play` or `values` cell's): the editor's loader gets them
    pub knobs_of: Option<String>,
    /// a `values` cell that meets another piece on the rows' path – its sentence (values.rs `warnings`)
    pub warning: Option<String>,
}

/// the knobs of `column`'s cell in `row` as scrub numbers – the cell's values over the piece's own (`knobs`: the
/// piece's, as [`CellInspector::knobs_of`] names it)
pub fn cell_knobs(show: &Show, row: &str, column: &str, knobs: &[Knob], locale: &str) -> Vec<KnobView> {
    match show.row(row).and_then(|r| r.cells.get(column)) {
        Some(Cell::Play { piece: Some(_), values, .. } | Cell::Values { values, .. }) => crate::values::knob_views(values, knobs, locale),
        _ => vec![],
    }
}

/// the cell inspector's label over its filter, and the row inspector's over an address's
pub const FILTER: &str = "who gets it";
pub const ADDRESS_FILTER: &str = "who picks it up";

/// The inspector of `column`'s cell in `row`.
pub fn cell_inspector(show: &Show, row: &str, column: &str) -> Option<CellInspector> {
    let (i, r) = show.rows.iter().enumerate().find(|(_, r)| r.id == row)?;
    let stage = show.outlet(column).filter(|o| o.kind == OutletKind::Stage);
    if stage.is_none() && show.outlet(column).is_none() && show.seat(column).is_none() {
        return None;
    }
    // #551 (found in #553): a cell that asks holds no piece – the outlet's piece plays on under the question
    let asks = matches!(r.cells.get(column), Some(Cell::Ask { .. }));
    let piece = stage.filter(|_| !asks).map(|_| piece_of(r.cells.get(column)).map_or_else(|| NO_PIECE.to_string(), str::to_string));
    let knobs_of = piece.as_ref().and_then(|_| piece_of(r.cells.get(column))).map(str::to_string);
    let warning = crate::values::warnings(show).into_iter().find(|w| w.row == row && w.column == column).map(|w| w.text);
    let shot = match r.cells.get(column) {
        Some(Cell::Play { piece: Some(p), shot, .. }) if stage.is_some() && p.starts_with("scene:") => Some(shot.as_deref().map_or_else(|| NO_SHOT.to_string(), |s| format!("{}:{s}", deck_space::rig::address::SHOT))),
        _ => None,
    };
    Some(CellInspector { title: format!("cell · row {} {row} · {}", i + 1, show.outlets.iter().find(|o| o.id == column).map_or(column, |o| o.name())), modes: modes(show, row, column), piece, shot, filter: cell_condition(show, row, column), knobs_of, warning })
}

/// what an edit of the toons says: the one toon, or the home first and the others
fn toons_said(show: &Show) -> String {
    match show.toons.as_slice() {
        [one] => format!("the show's toon: {one}"),
        [home, rest @ ..] => format!("the show's toons: {home} (home), {}", rest.join(", ")),
        [] => String::new(),
    }
}

/// One published scene the piece picker lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PieceView {
    /// what the piece holds: `scene:nave`
    pub address: String,
    /// the label chain for people: `Nave`
    pub label: String,
    /// `scene:nave · v2`, `· newer draft`, `· template`, `· <toon>` in a book of several toons
    pub detail: String,
}

/// The published scenes of the toon's book that match `query` (the book's own find, the kinds a piece may be: [`PIECE_KINDS`]),
/// in the book's order: each with the version the book lists. A scene the book lists without a version is not published.
pub fn pieces(book: &Book, query: &str, locale: &str) -> Vec<PieceView> {
    book.find(query, &PIECE_KINDS, locale)
        .into_iter()
        .filter_map(|i| {
            let e = book.entry(i)?;
            let (address, version) = (e.address.clone()?, e.version?);
            let row = book.row(i, locale)?;
            let mut detail = format!("{address} · v{version}");
            if row.draft {
                detail.push_str(" · newer draft");
            }
            if !e.placeholders.as_ref().is_none_or(Vec::is_empty) {
                detail.push_str(" · template");
            }
            if let Some(toon) = &row.toon {
                detail.push_str(&format!(" · {toon}"));
            }
            Some(PieceView { address, label: row.chain, detail })
        })
        .collect()
}

/// what a cell playing a published scene says while that scene has a newer draft (the draft plays only where it is edited)
pub const NEWER_DRAFT: &str = "a newer draft exists";

/// `column`'s cell in `row` holds a published scene of `book` that has a newer draft: [`NEWER_DRAFT`] is said under it.
pub fn newer_draft(show: &Show, row: &str, column: &str, book: &Book) -> bool {
    let Some(piece) = show.rows.iter().find(|r| r.id == row).and_then(|r| piece_of(r.cells.get(column))) else { return false };
    book.entries().iter().any(|e| e.address.as_deref() == Some(piece) && e.version.is_some() && e.origin.state == protocol::address_book::OriginState::Draft)
}

/// what marks a scene saved as a draft and never published: usable only in the editor where it is made, played on other
/// decks once published (decisions/deck.md 2026-11-08 [scenes])
pub const DRAFT: &str = "draft – this editor only until published";

/// The draft scenes (`scene:<id>`, its stage's name) that match `query` (in the id or the name), marked as drafts.
pub fn draft_pieces(drafts: &[(String, String)], query: &str) -> Vec<PieceView> {
    let q = query.trim().to_lowercase();
    drafts
        .iter()
        .filter(|(address, name)| q.is_empty() || address.to_lowercase().contains(&q) || name.to_lowercase().contains(&q))
        .map(|(address, name)| PieceView { address: address.clone(), label: if name.trim().is_empty() { address.clone() } else { name.clone() }, detail: format!("{address} · {DRAFT}") })
        .collect()
}

/// the picker's heading for `column`'s cell in `row`
pub fn picker_title(show: &Show, row: &str, column: &str) -> String {
    let n = show.rows.iter().position(|r| r.id == row).map_or(0, |i| i + 1);
    format!("PIECE FOR ROW {n} {} · {} – A PUBLISHED SCENE OF THE TOON'S ADDRESS BOOK", row.to_uppercase(), column.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{open, parse};
    use crate::subtree::{ClockChoice, Subtree};
    use protocol::{LayerSrc, Message};

    /// the editor's new show (deck-editor show/mod.rs `EMPTY`): one role, no row yet
    const NOTHING: &str = r#"{ "id": "new-show", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }], "rows": [] }"#;
    /// two roles and a lamp, no row yet
    const NEW: &str = r#"{ "id": "new-show", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }, { "id": "right" }, { "id": "lamps", "kind": "device" }], "rows": [] }"#;

    fn ed(show: &Show, e: Edit) -> Edited {
        edit(show, &e).unwrap_or_else(|why| panic!("{e:?}: {why}"))
    }

    fn mode(row: &str, column: &str, mode: Mode) -> Edit {
        Edit::Mode { row: row.into(), column: column.into(), mode }
    }

    #[test]
    fn a_show_without_its_toon_opens_with_the_problem_and_gets_one_picked() {
        let show = open(r#"{ "id": "old", "version": 4, "outlets": [{ "id": "left" }], "rows": [] }"#).unwrap();
        assert_eq!(crate::validate(&show).iter().map(ToString::to_string).collect::<Vec<_>>(), ["show: it names no toon – pick the show's toon"]);
        assert_eq!(edit(&show, &Edit::Toon { to: " ".into() }).unwrap_err(), "not changed: show: it names no toon – pick the show's toon");
        assert!(edit(&show, &Edit::Toon { to: "Cat Pope".into() }).unwrap_err().contains("is no toon id"));
        let set = ed(&show, Edit::Toon { to: " catpope ".into() });
        assert_eq!((set.said.as_str(), set.show.home()), ("the show's toon: catpope", "catpope"));
        assert!(crate::validate(&set.show).is_empty());
        assert_eq!(serde_json::to_value(&set.show).unwrap()["toons"], serde_json::json!(["catpope"]), "kept with the document");
    }

    /// #708: a second toon beside the home, taken out again; the home moves first; the last toon stays
    #[test]
    fn a_show_uses_several_toons_the_first_its_home() {
        let show = open(NOTHING).unwrap();
        let two = ed(&show, Edit::ToggleToon { toon: "yellowmouse".into() });
        assert_eq!((two.said.as_str(), two.show.toons.as_slice()), ("the show's toons: catpope (home), yellowmouse", &["catpope".to_string(), "yellowmouse".to_string()][..]));
        let home = ed(&two.show, Edit::Toon { to: "yellowmouse".into() });
        assert_eq!((home.show.home(), home.show.toons.len()), ("yellowmouse", 2));
        let one = ed(&home.show, Edit::ToggleToon { toon: "yellowmouse".into() });
        assert_eq!((one.said.as_str(), one.show.home()), ("the show's toon: catpope", "catpope"));
        assert!(edit(&one.show, &Edit::ToggleToon { toon: "catpope".into() }).unwrap_err().contains("keeps its one toon"));
        assert!(edit(&one.show, &Edit::ToggleToon { toon: "Yellow Mouse".into() }).unwrap_err().contains("is no toon id"));
    }

    /// #537's done-when, the core's half: from nothing to a row with a play cell naming a published scene, through the
    /// document; saved as JSON it is the document the headless scenario fires (show/fixtures/first-row.show.json)
    #[test]
    fn a_show_gets_a_row_and_a_play_cell_naming_a_scene_from_nothing() {
        let show = parse(NOTHING).unwrap();
        let added = ed(&show, Edit::Add { after: None });
        assert_eq!((added.said.as_str(), added.row.as_deref()), ("row 1 row added: manual, every column keeps", Some("row")));
        let named = ed(&added.show, Edit::Rename { id: "row".into(), to: " opening ".into() });
        assert_eq!(named.said, "row 1 row is called opening");
        let m = modes(&named.show, "opening", "left");
        let words: Vec<(&str, bool, bool)> = m.iter().map(|m| (m.word, m.on, m.picks)).collect();
        assert_eq!(words, [("play", false, true), ("values", false, false), ("mirror", false, false), ("stop", false, false), ("keep", true, false)], "play picks a piece first");
        assert_eq!(edit(&named.show, &mode("opening", "left", Mode::Play)).unwrap_err(), "left: pick a piece – `play` names one");
        let played = ed(&named.show, Edit::Piece { row: "opening".into(), column: "left".into(), piece: "scene:nave".into() });
        assert_eq!(played.said, "row 1 opening, left: scene:nave");
        assert!(crate::validate(&played.show).is_empty());
        let saved = serde_json::to_string_pretty(&played.show).unwrap();
        let fixture: Show = parse(include_str!("../fixtures/first-row.show.json")).unwrap();
        assert_eq!(parse(&saved).unwrap(), fixture, "the saved document is the scenario's");
        // fired: the conductor sends left the full cell with the scene as its piece
        let mut sub = Subtree::new(fixture, ClockChoice::Parent);
        let out = sub.fire("opening", 1000);
        let piece = out.iter().find_map(|e| match &e.message {
            Message::Cell(c) => c.cell.layers.as_ref()?.first().and_then(|l| if let LayerSrc::Piece(p) = &l.src { Some((c.outlet.clone(), p.piece.clone())) } else { None }),
            _ => None,
        });
        assert_eq!(piece, Some(("left".to_string(), "scene:nave".to_string())), "{out:?}");
    }

    #[test]
    fn rows_are_added_after_the_selected_renamed_and_refused_in_validates_words() {
        let show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        let added = ed(&show, Edit::Add { after: Some("opening".into()) });
        assert_eq!(added.show.rows[1].id, "row");
        assert_eq!(added.said, "row 2 row added: manual, every column keeps");
        assert_eq!(ed(&added.show, Edit::Add { after: None }).row.as_deref(), Some("row-2"));
        assert_eq!(edit(&added.show, &Edit::Rename { id: "row".into(), to: "dialog".into() }).unwrap_err(), "not changed: two rows named 'dialog'");
        assert_eq!(edit(&added.show, &Edit::Rename { id: "row".into(), to: " ".into() }).unwrap_err(), "not changed: show: an empty row id");
        assert_eq!(edit(&show, &Edit::Rename { id: "nowhere".into(), to: "x".into() }).unwrap_err(), "no row 'nowhere'");
        // the face's list follows the name
        let mut faced = show.clone();
        faced.face.rows = vec!["finale".into()];
        let renamed = ed(&faced, Edit::Rename { id: "finale".into(), to: "curtain".into() });
        assert_eq!(renamed.show.face.rows, ["curtain"]);
    }

    #[test]
    fn the_shows_background_is_set_checked_and_taken_off() {
        use deck_space::Background;
        let show = parse(NOTHING).unwrap();
        let set = ed(&show, Edit::Background { to: Some(Background::colour("#204060")) });
        assert_eq!((set.said.as_str(), set.row), ("the show's background: #204060 – over every scene", None));
        assert_eq!(set.show.background, Some(Background::colour("#204060")));
        let bad = Edit::Background { to: Some(Background::colour("teal")) };
        assert_eq!(edit(&show, &bad).unwrap_err(), "not changed: show: background: 'teal' is no colour: #rgb or #rrggbb");
        let off = ed(&set.show, Edit::Background { to: None });
        assert!(off.said.contains("none") && off.show.background.is_none());
    }

    #[test]
    fn the_shows_tempo_is_kept_checked_and_round_trips() {
        use crate::document::Tempo;
        use deck_core::tempo::Source;
        let show = parse(NOTHING).unwrap();
        let set = ed(&show, Edit::Tempo { to: Some(Tempo { bpm: 96.0, source: Source::Midi }) });
        assert_eq!(set.said, "the show's tempo: 96.0 BPM, midi");
        let text = serde_json::to_string(&set.show).unwrap();
        assert!(text.contains(r#""tempo":{"bpm":96.0,"source":"midi"}"#), "{text}");
        assert_eq!(parse(&text).unwrap().tempo, Some(Tempo { bpm: 96.0, source: Source::Midi }));
        assert!(!serde_json::to_string(&show).unwrap().contains("tempo"), "none: not written");
        let slow = Edit::Tempo { to: Some(Tempo { bpm: 5.0, source: Source::Internal }) };
        assert_eq!(edit(&show, &slow).unwrap_err(), "not changed: show: tempo: 5 BPM is out of 20-300");
        let relay = Edit::Tempo { to: Some(Tempo { bpm: 120.0, source: Source::Relay }) };
        assert!(edit(&show, &relay).unwrap_err().contains("the source 'relay' is none a show keeps"));
        let off = ed(&set.show, Edit::Tempo { to: None });
        assert!(off.show.tempo.is_none() && off.said.contains("none"));
        assert_eq!(Tempo::of(900.0, Source::Osc), Tempo { bpm: 300.0, source: Source::Internal }, "kept within what a show keeps");
        assert_eq!(Tempo::of(90.0, Source::Link), Tempo { bpm: 90.0, source: Source::Link });
        // a source left out is the own clock
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v["tempo"] = serde_json::json!({ "bpm": 110 });
        assert_eq!(parse(&v.to_string()).unwrap().tempo, Some(Tempo { bpm: 110.0, source: Source::Internal }));
    }

    #[test]
    fn a_remove_asks_first_and_says_what_goes() {
        let mut show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        show.face.rows = vec!["dialog".into()];
        let p = remove_prompt(&show, "dialog");
        assert_eq!(p.title, "Remove the row 2 dialog?");
        assert_eq!(p.lines[0], ("lamps: look warm · fade 0.8 s goes".to_string(), Tone::Warn));
        assert!(p.lines.iter().any(|(l, _)| l.starts_with("the event 'ch3.dialog-start' fires no row then")), "{:?}", p.lines);
        assert!(p.lines.iter().any(|(l, t)| l.starts_with("the face shows it") && *t == Tone::Warn));
        assert!(p.warn && p.can);
        let gone = ed(&show, Edit::Remove { id: "dialog".into() });
        assert_eq!(gone.said, "row dialog removed with its 2 cells");
        assert_eq!((gone.row.as_deref(), gone.show.face.rows.len()), (Some("opening"), 0), "the row before is selected; the face's entry went");
        let first = ed(&show, Edit::Remove { id: "opening".into() });
        assert_eq!(first.row.as_deref(), Some("dialog"), "the first row gone: the new first");
        let empty = parse(NEW).unwrap();
        let one = ed(&empty, Edit::Add { after: None });
        assert_eq!(ed(&one.show, Edit::Remove { id: "row".into() }).row, None);
    }

    #[test]
    fn a_cells_mode_carries_its_piece_and_values_where_it_can() {
        let show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        // dialog's left is `values` on scene:nave: play keeps the piece and its values
        let play = ed(&show, mode("dialog", "left", Mode::Play));
        assert_eq!(play.said, "row 2 dialog, left: scene:nave · dim 0.5");
        assert!(modes(&play.show, "dialog", "left").iter().all(|m| !m.picks), "it names a piece: play picks none");
        // values on a cell that names no piece: the piece that plays there before it (yellowmouse-enters' rig)
        let values = ed(&show, mode("finale", "left", Mode::Values));
        assert!(matches!(values.show.row("finale").unwrap().cells.get("left"), Some(Cell::Values { piece, .. }) if piece == "character:yellowmouse"));
        let fresh = ed(&parse(NEW).unwrap(), Edit::Add { after: None }).show;
        assert_eq!(edit(&fresh, &mode("row", "left", Mode::Values)).unwrap_err(), "left: no piece plays there before this row – `values` changes a piece that plays");
        // mirror: the other stage outlet; keep: no cell
        let mirrored = ed(&fresh, mode("row", "right", Mode::Mirror));
        assert_eq!(mirrored.show.rows[0].cells.get("right"), Some(&Cell::Mirror { of: "left".into() }));
        assert_eq!(edit(&mirrored.show, &mode("row", "left", Mode::Mirror)).unwrap_err(), "left: no other stage outlet to mirror");
        let kept = ed(&mirrored.show, mode("row", "right", Mode::Keep));
        assert!(kept.show.rows[0].cells.is_empty() && kept.said == "row 1 row, right: ↑");
        // a device outlet offers keep only; a seat go / free / keep
        assert_eq!(modes(&fresh, "row", "lamps").iter().map(|m| m.word).collect::<Vec<_>>(), ["keep"]);
        assert_eq!(modes(&show, "dialog", "foyer").iter().map(|m| m.word).collect::<Vec<_>>(), ["go", "free", "keep"]);
    }

    /// #546: `ask` is offered where the show has a question; a column joins the row's question with its filter
    #[test]
    fn a_column_joins_the_rows_question_with_its_filter() {
        let show = parse(include_str!("../fixtures/vote-cell.show.json")).unwrap();
        let words: Vec<&str> = modes(&show, "crossroads", "left").iter().filter(|m| m.on || m.mode == Mode::Ask).map(|m| m.word).collect();
        assert_eq!(words, ["ask"]);
        let joined = ed(&show, mode("crossroads", "middle", Mode::Ask));
        assert_eq!(joined.said, "row 2 crossroads, middle: ask q1");
        let row = joined.show.row("crossroads").unwrap();
        assert_eq!(row.r#where.get("middle"), row.r#where.get("left"), "the filter its other columns carry");
        assert!(crate::validate(&joined.show).is_empty());
        let top = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        assert!(modes(&top, "finale", "left").iter().all(|m| m.mode != Mode::Ask), "no question: not offered");
    }

    fn address_fact(row: &str, address: usize, fact: Fact) -> Edit {
        Edit::AddressFact { row: row.into(), address, fact }
    }

    fn cell_fact(row: &str, column: &str, fact: Fact) -> Edit {
        Edit::CellFact { row: row.into(), column: column.into(), fact }
    }

    /// #553: a row address's filter is set and cleared through the document, in deck-ui's words, checked by validate
    #[test]
    fn an_addresss_filter_is_set_and_cleared() {
        let show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        let c = address_condition(&show, "yellowmouse-enters", 0).unwrap();
        assert_eq!((c.mode, c.words.as_str()), (deck_ui::filter::Mode::All, "group left"));
        assert!(c.toggles[c.chip("group left").unwrap()].on && c.chip("group right").is_some(), "the show's roles are the groups offered");
        assert_eq!(address_condition(&show, "yellowmouse-enters", 1).unwrap().words, "everyone");
        let phone = ed(&show, address_fact("yellowmouse-enters", 1, Fact::Device("phone".into())));
        assert_eq!(phone.said, "row 3 yellowmouse-enters, address 2: location:* > close_doors where device phone");
        let member = ed(&phone.show, address_fact("yellowmouse-enters", 1, Fact::Account(protocol::Account::Member)));
        assert_eq!(member.said, "row 3 yellowmouse-enters, address 2: location:* > close_doors where device phone · member");
        assert!(crate::validate(&member.show).is_empty());
        // another group replaces the one it names; every fact off: no filter
        let right = ed(&show, address_fact("yellowmouse-enters", 0, Fact::Group("right".into())));
        assert_eq!(right.show.row("yellowmouse-enters").unwrap().addresses[0].filter.as_ref().and_then(|f| f.group.as_deref()), Some("right"));
        let cleared = ed(&show, address_fact("yellowmouse-enters", 0, Fact::Group("left".into())));
        assert_eq!(cleared.show.row("yellowmouse-enters").unwrap().addresses[0].filter, None);
        assert_eq!(cleared.said, "row 3 yellowmouse-enters, address 1: character:yellowmouse > dance:samba");
        assert_eq!(edit(&show, &address_fact("yellowmouse-enters", 5, Fact::Approved)).unwrap_err(), "row yellowmouse-enters has no address 6");
        assert!(address_condition(&show, "dialog", 0).is_none(), "a row without addresses");
    }

    /// #553: a cell's filter in the cell inspector – no group chip, an `ask` row's columns take one filter together
    #[test]
    fn a_cells_filter_is_set_on_every_column_of_its_question() {
        let show = parse(include_str!("../fixtures/vote-cell.show.json")).unwrap();
        let i = cell_inspector(&show, "crossroads", "left").unwrap();
        let c = i.filter.unwrap();
        assert_eq!(c.words, "device phone · member · live");
        assert_eq!(c.chip("group left"), None, "the column is the group");
        assert!(cell_inspector(&show, "crossroads", "middle").unwrap().filter.is_none(), "a values cell without a filter: not offered");
        assert_eq!((i.piece, cell_inspector(&show, "crossroads", "middle").unwrap().piece.as_deref()), (None, Some("scene:choir")), "#551: an ask cell shows no piece field");
        assert!(cell_inspector(&show, "opening", "left").unwrap().filter.is_none());
        // device browser on left: right, which asks the same question, takes it too
        let browser = ed(&show, cell_fact("crossroads", "left", Fact::Device("browser".into())));
        assert_eq!(browser.said, "row 2 crossroads, left · right: device browser · member · live");
        let row = browser.show.row("crossroads").unwrap();
        assert_eq!(row.r#where.get("left"), row.r#where.get("right"));
        assert!(crate::validate(&browser.show).is_empty());
        // every fact off: the filter goes from both
        let mut cleared = browser.show;
        for f in [Fact::Device("browser".into()), Fact::Account(protocol::Account::Member), Fact::Account(protocol::Account::Live)] {
            cleared = ed(&cleared, cell_fact("crossroads", "right", f)).show;
        }
        assert!(cleared.row("crossroads").unwrap().r#where.is_empty(), "{:?}", cleared.row("crossroads"));
        assert_eq!(cell_inspector(&cleared, "crossroads", "left").unwrap().filter.unwrap().words, "everyone", "an ask cell offers it still");
        // a device outlet has no audience: refused in validate's words
        let refused = edit(&show, &cell_fact("crossroads", "lamps", Fact::Approved)).unwrap_err();
        assert!(refused.starts_with("not changed: ") && refused.contains("device outlet has no audience"), "{refused}");
    }

    #[test]
    fn a_piece_is_picked_or_typed_and_the_core_reads_it() {
        let show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        // a values cell expects the new piece; its values were the other piece's
        let swapped = ed(&show, Edit::Piece { row: "dialog".into(), column: "left".into(), piece: "scene:vigil".into() });
        assert_eq!(swapped.said, "row 2 dialog, left: values · scene:vigil – the values for the other piece went");
        assert!(matches!(swapped.show.row("dialog").unwrap().cells.get("left"), Some(Cell::Values { piece, values }) if piece == "scene:vigil" && values.is_empty()));
        // a typed piece the one parser does not read is refused in its words
        let typed = edit(&show, &Edit::Piece { row: "finale".into(), column: "left".into(), piece: "nave".into() }).unwrap_err();
        assert!(typed.starts_with("not changed: row 'finale', 'left': "), "{typed}");
        // a device outlet takes no piece
        assert!(edit(&show, &Edit::Piece { row: "finale".into(), column: "lamps".into(), piece: "scene:nave".into() }).is_err());
        // nor does a cell that asks a question
        let vote = parse(include_str!("../fixtures/vote-cell.show.json")).unwrap();
        let asks = edit(&vote, &Edit::Piece { row: "crossroads".into(), column: "left".into(), piece: "scene:nave".into() }).unwrap_err();
        assert_eq!(asks, "not changed: row 'crossroads', 'left' asks q1 – it takes a piece once it leaves the question");
    }

    #[test]
    fn the_cell_inspector_and_the_picker_say_the_published_scenes() {
        let show = open(include_str!("../fixtures/top-toy.show.json")).unwrap();
        let i = cell_inspector(&show, "dialog", "left").unwrap();
        assert_eq!(i.title, "cell · row 2 dialog · left");
        assert_eq!(i.piece.as_deref(), Some("scene:nave"));
        assert_eq!(cell_inspector(&show, "finale", "left").unwrap().piece.as_deref(), Some(NO_PIECE));
        assert_eq!(cell_inspector(&show, "finale", "lamps").unwrap().piece, None, "a device holds no piece");
        assert!(cell_inspector(&show, "finale", "nowhere").is_none());
        assert_eq!(picker_title(&show, "finale", "left"), "PIECE FOR ROW 4 FINALE · LEFT – A PUBLISHED SCENE OF THE TOON'S ADDRESS BOOK");
        let book = Book::parse(BOOK).unwrap();
        let all = pieces(&book, "", "en");
        assert_eq!(all.iter().map(|p| p.address.as_str()).collect::<Vec<_>>(), ["scene:nave", "scene:stage"], "scenes only, not the chapter's group or the unpublished crypt");
        assert_eq!(all[0], PieceView { address: "scene:nave".into(), label: "Nave".into(), detail: "scene:nave · v2 · newer draft".into() });
        assert_eq!(all[1].detail, "scene:stage · v1 · template");
        assert_eq!(pieces(&book, "nav", "en").len(), 1);
        let drafts = [("scene:duet".to_string(), "Duet".to_string()), ("scene:crypt".to_string(), String::new())];
        let d = draft_pieces(&drafts, "");
        assert_eq!(d[0], PieceView { address: "scene:duet".into(), label: "Duet".into(), detail: format!("scene:duet · {DRAFT}") });
        assert_eq!(d[1].label, "scene:crypt", "no name: its address");
        assert_eq!(draft_pieces(&drafts, "DUE").len(), 1, "found by its name or id");
    }

    #[test]
    fn a_cell_of_a_published_scene_with_a_newer_draft_says_so() {
        let show = open(include_str!("../fixtures/top-toy.show.json")).unwrap();
        let book = Book::parse(BOOK).unwrap();
        assert!(newer_draft(&show, "dialog", "left", &book), "scene:nave v2 has a newer draft");
        assert!(!newer_draft(&show, "finale", "left", &book), "no piece");
        let clean = BOOK.replace(r#""key": "scene:nave", "state": "draft""#, r#""key": "scene:nave", "state": "published""#);
        assert!(!newer_draft(&show, "dialog", "left", &Book::parse(&clean).unwrap()), "published, no draft");
        let unpublished = BOOK.replace(r#""state": "draft" }, "version": 2"#, r#""state": "draft" }"#);
        assert!(!newer_draft(&show, "dialog", "left", &Book::parse(&unpublished).unwrap()), "a draft never published is a draft, not a newer one");
    }

    /// a toon's book as the API gives it: two published scenes (one a template), one not yet published, other kinds
    pub(crate) const BOOK: &str = r#"{ "address_book": 2, "toon": "catpope", "version": "v1", "source": "en", "entries": [
        { "ref": "character:catpope", "address": "character:catpope", "kind": "character", "name": "catpope", "labels": { "en": "Cat Pope" }, "origin": { "key": "character:catpope", "state": "published" } },
        { "ref": "chapter:3.nave", "kind": "group", "name": "nave", "parent": "chapter:3", "labels": {}, "origin": { "key": "chapter:catpope:3", "state": "published" } },
        { "ref": "scene:nave", "address": "scene:nave", "kind": "scene", "name": "nave", "labels": { "en": "Nave" }, "origin": { "key": "scene:nave", "state": "draft" }, "version": 2 },
        { "ref": "scene:stage", "address": "scene:stage", "kind": "scene", "name": "stage", "labels": { "en": "Stage" }, "origin": { "key": "scene:stage", "state": "published" }, "version": 1,
          "placeholders": [{ "id": "who", "fill": { "kind": ["character"] }, "label": { "en": "Who" } }] },
        { "ref": "scene:crypt", "address": "scene:crypt", "kind": "scene", "name": "crypt", "labels": { "en": "Crypt" }, "origin": { "key": "scene:crypt", "state": "draft" } }
    ] }"#;
}
