//! Validity is the core's (RULES A2). [`validate`] reads one document; [`resolve`] follows the seats from the top, the only
//! way a tree exists (the child does not name its parent, SHOW.md §4b), and writes down the exact versions a flash carries.
use crate::document::{Cell, Face, OutletKind, Pinned, Row, Seat, Show};
use deck_space::rig::address::Place;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// What is wrong with a show. All of them are reported at once: the editor lists them (#91).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    Duplicate { what: &'static str, id: String },
    /// a reference that finds nothing
    Unknown { what: &'static str, id: String, at: String },
    /// a cell or a field its place cannot hold
    Refused { at: String, why: String },
    /// a seat uses what the child's face does not show
    FaceLacks { seat: String, what: &'static str, id: String },
    /// `a › b › a`, by show id
    Cycle { path: Vec<String> },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::Duplicate { what, id } => write!(f, "two {what}s named '{id}'"),
            Problem::Unknown { what, id, at } => write!(f, "{at}: no {what} '{id}'"),
            Problem::Refused { at, why } => write!(f, "{at}: {why}"),
            Problem::FaceLacks { seat, what, id } => write!(f, "seat '{seat}': the child's face has no {what} '{id}'"),
            Problem::Cycle { path } => write!(f, "a cycle of shows: {}", path.join(" › ")),
        }
    }
}

fn refused(at: &str, why: impl Into<String>) -> Problem {
    Problem::Refused { at: at.to_string(), why: why.into() }
}

/// ids that must be unique among `seen`
fn unique<'a>(what: &'static str, ids: impl Iterator<Item = &'a String>, seen: &mut BTreeSet<&'a str>, out: &mut Vec<Problem>) {
    for id in ids {
        if id.is_empty() {
            out.push(refused(what, "an empty id"));
        } else if !seen.insert(id) {
            out.push(Problem::Duplicate { what, id: id.clone() });
        }
    }
}

fn place(at: &str, text: &str, out: &mut Vec<Problem>) -> Option<Place> {
    Place::parse(text).map_err(|why| out.push(refused(at, why))).ok()
}

/// a cell on a stage outlet
fn stage_cell(show: &Show, at: &str, cell: &Cell, out: &mut Vec<Problem>) {
    match cell {
        Cell::Play { piece, send, .. } => {
            match piece {
                Some(p) => drop(place(at, p, out)),
                None => out.push(refused(at, "`play` on a stage outlet names a piece")),
            }
            if !send.is_empty() {
                out.push(refused(at, "`send` belongs to a device outlet"));
            }
        }
        Cell::Values { piece, .. } => drop(place(at, piece, out)),
        Cell::Mirror { of } => match show.outlet(of) {
            None => out.push(Problem::Unknown { what: "outlet", id: of.clone(), at: at.to_string() }),
            Some(o) if o.kind == OutletKind::Device => out.push(refused(at, "a device outlet cannot be mirrored")),
            Some(_) => {}
        },
        Cell::Keep | Cell::Black => {}
        Cell::Go { .. } | Cell::Free => out.push(refused(at, "`go` and `free` are for a seat's column")),
    }
}

/// a cell on a device outlet: `play` with messages, or `keep`
fn device_cell(at: &str, cell: &Cell, out: &mut Vec<Problem>) {
    match cell {
        Cell::Play { piece: None, send, .. } if !send.is_empty() => {}
        Cell::Keep => {}
        _ => out.push(refused(at, "a device outlet takes `play` with `send`, or `keep`")),
    }
}

/// a cell on a seat's column: a row the seat uses, `free` or `keep`
fn seat_cell(seat: &Seat, at: &str, cell: &Cell, out: &mut Vec<Problem>) {
    match cell {
        Cell::Go { row } if !seat.uses.rows.contains(row) => out.push(Problem::Unknown { what: "row in the seat's `uses`", id: row.clone(), at: at.to_string() }),
        Cell::Go { .. } | Cell::Free | Cell::Keep => {}
        _ => out.push(refused(at, "a seat takes `go`, `free` or `keep`")),
    }
}

/// `a.of = b, b.of = a` inside one row; reported once, from the smallest member of the circle
fn mirror_circles(row: &Row, out: &mut Vec<Problem>) {
    for (start, cell) in &row.cells {
        let Cell::Mirror { of } = cell else { continue };
        let mut path = vec![start.as_str()];
        let mut next = of.as_str();
        while !path.contains(&next) {
            path.push(next);
            match row.cells.get(next) {
                Some(Cell::Mirror { of }) => next = of,
                _ => break,
            }
        }
        if next == start.as_str() && path.len() > 1 && path.iter().min() == Some(&start.as_str()) {
            out.push(refused(&format!("row '{}'", row.id), format!("mirrors go in a circle: {}", path.join(" › "))));
        }
    }
}

/// One document on its own. Empty = valid.
pub fn validate(show: &Show) -> Vec<Problem> {
    let mut out = vec![];
    if show.id.is_empty() {
        out.push(refused("show", "an empty id"));
    }
    // outlets and seats are the matrix's columns, one name for each
    let mut columns = BTreeSet::new();
    unique("outlet", show.outlets.iter().map(|o| &o.id), &mut columns, &mut out);
    unique("seat", show.seats.iter().map(|s| &s.id), &mut columns, &mut out);
    let mut rows = BTreeSet::new();
    unique("row", show.rows.iter().map(|r| &r.id), &mut rows, &mut out);

    for seat in &show.seats {
        let at = format!("seat '{}'", seat.id);
        if seat.show.id.is_empty() {
            out.push(refused(&at, "it names no show"));
        }
        if let Some(p) = seat.sits.as_deref().and_then(|s| place(&at, s, &mut out)) {
            if p.kind != "deck" && p.kind != "member" {
                out.push(refused(&at, format!("a deck or a member sits on a seat, not a {}", p.kind)));
            }
        }
    }

    let mut events: BTreeMap<&str, &str> = BTreeMap::new();
    for row in &show.rows {
        if let Some(event) = &row.fire.event {
            let at = format!("row '{}'", row.id);
            if event.is_empty() {
                out.push(refused(&at, "an empty event id"));
            } else if let Some(first) = events.insert(event, &row.id) {
                out.push(refused(&at, format!("the event '{event}' already fires row '{first}'")));
            }
        }
        for (column, cell) in &row.cells {
            let at = format!("row '{}', '{column}'", row.id);
            if let Some(outlet) = show.outlet(column) {
                match outlet.kind {
                    OutletKind::Stage => stage_cell(show, &at, cell, &mut out),
                    OutletKind::Device => device_cell(&at, cell, &mut out),
                }
                if matches!(cell, Cell::Mirror { of } if of == column) {
                    out.push(refused(&at, "an outlet cannot mirror itself"));
                }
            } else if let Some(seat) = show.seat(column) {
                seat_cell(seat, &at, cell, &mut out);
            } else {
                out.push(Problem::Unknown { what: "outlet or seat", id: column.clone(), at });
            }
        }
        mirror_circles(row, &mut out);
    }

    for id in show.face.rows.iter().filter(|r| show.row(r).is_none()) {
        out.push(Problem::Unknown { what: "row", id: id.clone(), at: "the face".into() });
    }
    for id in show.face.outlets.iter().filter(|o| show.outlet(o).is_none()) {
        out.push(Problem::Unknown { what: "outlet", id: id.clone(), at: "the face".into() });
    }
    out
}

/// Where the shows of a tree come from (the API later, #90; a map of hand-written documents now).
pub trait Library {
    fn show(&self, pinned: &Pinned) -> Option<&Show>;
}

impl Library for BTreeMap<Pinned, Show> {
    fn show(&self, pinned: &Pinned) -> Option<&Show> {
        self.get(pinned)
    }
}

/// The tree in exact versions, the root first, each show once: what "the show at a pinned version" means for a tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    pub shows: Vec<Pinned>,
}

fn face_uses(seat: &Seat, face: &Face, out: &mut Vec<Problem>) {
    let lacks = |what: &'static str, used: &[String], has: &[String], out: &mut Vec<Problem>| {
        for id in used.iter().filter(|u| !has.contains(u)) {
            out.push(Problem::FaceLacks { seat: seat.id.clone(), what, id: id.clone() });
        }
    };
    lacks("row", &seat.uses.rows, &face.rows, out);
    lacks("event", &seat.uses.events, &face.events, out);
    lacks("outlet", &seat.uses.outlets, &face.outlets, out);
}

fn walk(show: &Show, library: &impl Library, path: &mut Vec<String>, lock: &mut Vec<Pinned>, out: &mut Vec<Problem>) {
    path.push(show.id.clone());
    let pinned = Pinned { id: show.id.clone(), version: show.version };
    if !lock.contains(&pinned) {
        lock.push(pinned);
    }
    out.extend(validate(show));
    for seat in &show.seats {
        let Some(child) = library.show(&seat.show) else {
            out.push(Problem::Unknown { what: "show", id: seat.show.to_string(), at: format!("seat '{}'", seat.id) });
            continue;
        };
        face_uses(seat, &child.face, out);
        if path.contains(&child.id) {
            let mut cycle = path.clone();
            cycle.push(child.id.clone());
            out.push(Problem::Cycle { path: cycle });
        } else {
            walk(child, library, path, lock, out);
        }
    }
    path.pop();
}

/// The whole tree from its top document: valid, with every version it runs on.
pub fn resolve(root: &Show, library: &impl Library) -> Result<Lock, Vec<Problem>> {
    let (mut path, mut shows, mut problems) = (vec![], vec![], vec![]);
    walk(root, library, &mut path, &mut shows, &mut problems);
    if problems.is_empty() {
        Ok(Lock { shows })
    } else {
        Err(problems)
    }
}
