//! Validity is the core's (RULES A2). [`validate`] reads one document; [`resolve`] follows the seats from the top, the only
//! way a tree exists (the child does not name its parent, SHOW.md §4b), and writes down the exact versions a flash carries.
use crate::document::{Addressed, Cell, Face, OutletKind, Pinned, Row, Seat, Show};
use protocol::PARENT;
use deck_space::rig::address::{Place, Target};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// What is wrong with a show. All of them are reported at once: the editor lists them (#91).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    Duplicate { what: &'static str, id: String },
    /// a reference that finds nothing
    Unknown { what: &'static str, id: String, at: Spot },
    /// a cell or a field its place cannot hold
    Refused { at: Spot, why: String },
    /// a seat uses what the child's face does not show
    FaceLacks { seat: String, what: &'static str, id: String },
    /// `a › b › a`, by show id
    Cycle { path: Vec<String> },
}

/// Where in a document a problem is (#91: the problems list points at it, a click selects it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Spot {
    /// the document as a whole (its id, its columns)
    Show,
    Face,
    Seat(String),
    Row(String),
    Cell { row: String, column: String },
}

impl fmt::Display for Spot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Spot::Show => write!(f, "show"),
            Spot::Face => write!(f, "the face"),
            Spot::Seat(s) => write!(f, "seat '{s}'"),
            Spot::Row(r) => write!(f, "row '{r}'"),
            Spot::Cell { row, column } => write!(f, "row '{row}', '{column}'"),
        }
    }
}

impl Problem {
    /// where it is
    pub fn spot(&self) -> Spot {
        match self {
            Problem::Unknown { at, .. } | Problem::Refused { at, .. } => at.clone(),
            Problem::Duplicate { what: "row", id } => Spot::Row(id.clone()),
            Problem::Duplicate { what: "seat", id } | Problem::FaceLacks { seat: id, .. } => Spot::Seat(id.clone()),
            Problem::Duplicate { .. } | Problem::Cycle { .. } => Spot::Show,
        }
    }
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

fn refused(at: &Spot, why: impl Into<String>) -> Problem {
    Problem::Refused { at: at.clone(), why: why.into() }
}

/// ids that must be unique among `seen`
fn unique<'a>(what: &'static str, ids: impl Iterator<Item = &'a String>, seen: &mut BTreeSet<&'a str>, out: &mut Vec<Problem>) {
    for id in ids {
        if id.is_empty() {
            out.push(refused(&Spot::Show, format!("an empty {what} id")));
        } else if !seen.insert(id) {
            out.push(Problem::Duplicate { what, id: id.clone() });
        }
    }
}

fn place(at: &Spot, text: &str, out: &mut Vec<Problem>) -> Option<Place> {
    Place::parse(text).map_err(|why| out.push(refused(at, why))).ok()
}

/// a cell on a stage outlet
fn stage_cell(show: &Show, at: &Spot, cell: &Cell, out: &mut Vec<Problem>) {
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
            None => out.push(Problem::Unknown { what: "outlet", id: of.clone(), at: at.clone() }),
            Some(o) if o.kind == OutletKind::Device => out.push(refused(at, "a device outlet cannot be mirrored")),
            Some(_) => {}
        },
        Cell::Keep | Cell::Black => {}
        Cell::Go { .. } | Cell::Free => out.push(refused(at, "`go` and `free` are for a seat's column")),
        Cell::Look { .. } => out.push(refused(at, "a look is for a device outlet's lamps")),
    }
}

/// a cell on a device outlet: `play` with messages, a look that lights a fixture there, or `keep`
fn device_cell(show: &Show, outlet: &str, at: &Spot, cell: &Cell, out: &mut Vec<Problem>) {
    match cell {
        Cell::Play { piece: None, send, .. } if !send.is_empty() => {}
        Cell::Keep => {}
        Cell::Look { look, .. } => match show.look(look) {
            None => out.push(Problem::Unknown { what: "look", id: look.clone(), at: at.clone() }),
            Some(l) if l.dmx(show, outlet, None).is_empty() => out.push(refused(at, format!("the look '{look}' lights no fixture on '{outlet}'"))),
            Some(_) => {}
        },
        _ => out.push(refused(at, "a device outlet takes `play` with `send`, a `look`, or `keep`")),
    }
}

/// the fixtures (on a device outlet, inside the universe, each channel once) and the looks (known fixtures and channels)
fn lights(show: &Show, out: &mut Vec<Problem>) {
    let mut ids = BTreeSet::new();
    unique("fixture", show.fixtures.iter().map(|f| &f.id), &mut ids, out);
    let mut taken: BTreeMap<(&str, u16), &str> = BTreeMap::new();
    for f in &show.fixtures {
        let at = Spot::Show;
        match show.outlet(&f.outlet) {
            None => out.push(Problem::Unknown { what: "outlet", id: f.outlet.clone(), at: at.clone() }),
            Some(o) if o.kind != OutletKind::Device => out.push(refused(&at, format!("fixture '{}': '{}' is no device outlet", f.id, f.outlet))),
            Some(_) => {}
        }
        let last = f.address as usize + f.profile.channels().len() - 1;
        if f.address == 0 || last > 512 {
            out.push(refused(&at, format!("fixture '{}': channels {}–{last} are outside 1–512", f.id, f.address)));
        }
        for ch in f.address..=(last.min(512) as u16) {
            if let Some(other) = taken.insert((f.outlet.as_str(), ch), &f.id) {
                out.push(refused(&at, format!("fixture '{}': channel {ch} is already '{other}'s", f.id)));
                break;
            }
        }
    }
    let mut looks = BTreeSet::new();
    unique("look", show.looks.iter().map(|l| &l.id), &mut looks, out);
    for l in &show.looks {
        for (fixture, set) in &l.values {
            let Some(f) = show.fixture(fixture) else {
                out.push(Problem::Unknown { what: "fixture", id: fixture.clone(), at: Spot::Show });
                continue;
            };
            for c in set.keys().filter(|c| !f.profile.channels().contains(&c.as_str())) {
                out.push(refused(&Spot::Show, format!("look '{}': fixture '{fixture}' has no channel '{c}'", l.id)));
            }
        }
    }
}

/// a cell on a seat's column: a row the seat uses, `free` or `keep`
fn seat_cell(seat: &Seat, at: &Spot, cell: &Cell, out: &mut Vec<Problem>) {
    match cell {
        Cell::Go { row } if !seat.uses.rows.contains(row) => out.push(Problem::Unknown { what: "row in the seat's `uses`", id: row.clone(), at: at.clone() }),
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
            out.push(refused(&Spot::Row(row.id.clone()), format!("mirrors go in a circle: {}", path.join(" › "))));
        }
    }
}

/// an address a row sends to: the one parser's address and action (RULES A14); a filter names something
fn addressed(at: &Spot, a: &Addressed, out: &mut Vec<Problem>) {
    if let Err(why) = Target::parse(&a.address) {
        out.push(refused(at, why));
    }
    if let Err(why) = Target::action(&a.action) {
        out.push(refused(at, format!("{}: {why}", a.address)));
    }
    if let Some(f) = &a.filter {
        if f.group.as_deref() == Some("") || f.device.as_deref() == Some("") {
            out.push(refused(at, format!("{}: an empty filter", a.address)));
        }
    }
}

/// #478: a role's filter – a stage outlet's only (a device outlet is a bridge's, no deck takes it); every fact once, each a
/// place the one parser reads (`device:<word>`, `group:<word>`, RULES A14); a filter that names nothing lets nobody in.
fn admits(outlet: &crate::document::Outlet, out: &mut Vec<Problem>) {
    let Some(a) = &outlet.admits else { return };
    let at = Spot::Show;
    let id = &outlet.id;
    if outlet.kind == OutletKind::Device {
        out.push(refused(&at, format!("outlet '{id}': a device outlet is no role, it takes no `for`")));
    }
    let accounts = a.account.iter().flatten().map(|x| ("account", x.as_str().to_string()));
    let facts: Vec<(&str, String)> = accounts.chain(a.device.iter().flatten().map(|d| ("device", d.clone()))).chain(a.group.iter().flatten().map(|g| ("group", g.clone()))).collect();
    if facts.is_empty() && a.approved != Some(true) {
        out.push(refused(&at, format!("outlet '{id}': `for` names nobody (leave it out: anyone with the code)")));
    }
    let mut seen = BTreeSet::new();
    for (kind, value) in facts {
        let fact = format!("{kind}:{value}");
        match Place::parse(&fact) {
            Err(why) => out.push(refused(&at, format!("outlet '{id}': {why}"))),
            Ok(_) if !seen.insert(fact.clone()) => out.push(refused(&at, format!("outlet '{id}': '{fact}' twice"))),
            Ok(_) => {}
        }
    }
}

/// One document on its own. Empty = valid.
pub fn validate(show: &Show) -> Vec<Problem> {
    let mut out = vec![];
    if show.id.is_empty() {
        out.push(refused(&Spot::Show, "an empty id"));
    }
    // outlets and seats are the matrix's columns, one name for each
    let mut columns = BTreeSet::new();
    unique("outlet", show.outlets.iter().map(|o| &o.id), &mut columns, &mut out);
    unique("seat", show.seats.iter().map(|s| &s.id), &mut columns, &mut out);
    let mut rows = BTreeSet::new();
    unique("row", show.rows.iter().map(|r| &r.id), &mut rows, &mut out);

    // #90: `foyer/wall` is an outlet of the child in a seat, `parent` is the far end of the link
    for id in show.outlets.iter().map(|o| &o.id).chain(show.seats.iter().map(|s| &s.id)) {
        if id == PARENT || id.contains('/') {
            out.push(refused(&Spot::Show, format!("a column cannot be named '{id}'")));
        }
    }
    for outlet in &show.outlets {
        admits(outlet, &mut out);
    }

    for seat in &show.seats {
        let at = Spot::Seat(seat.id.clone());
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
            let at = Spot::Row(row.id.clone());
            if event.is_empty() {
                out.push(refused(&at, "an empty event id"));
            } else if let Some(first) = events.insert(event, &row.id) {
                out.push(refused(&at, format!("the event '{event}' already fires row '{first}'")));
            }
        }
        for (column, cell) in &row.cells {
            let at = Spot::Cell { row: row.id.clone(), column: column.clone() };
            if let Some(outlet) = show.outlet(column) {
                match outlet.kind {
                    OutletKind::Stage => stage_cell(show, &at, cell, &mut out),
                    OutletKind::Device => device_cell(show, column, &at, cell, &mut out),
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
        for a in &row.addresses {
            addressed(&Spot::Row(row.id.clone()), a, &mut out);
        }
    }

    lights(show, &mut out);
    for id in show.face.rows.iter().filter(|r| show.row(r).is_none()) {
        out.push(Problem::Unknown { what: "row", id: id.clone(), at: Spot::Face });
    }
    for id in show.face.outlets.iter().filter(|o| show.outlet(o).is_none()) {
        out.push(Problem::Unknown { what: "outlet", id: id.clone(), at: Spot::Face });
    }
    out
}

/// One document and the tree under it, checked for the API (the build service, #90): the core's own words, one per problem.
pub fn problems(document: &serde_json::Value, shows: &[serde_json::Value]) -> Vec<String> {
    let Ok(root) = serde_json::from_value::<Show>(document.clone()) else {
        return match crate::document::parse(&document.to_string()) {
            Err(e) => vec![e.to_string()],
            Ok(_) => vec![],
        };
    };
    let mut library = BTreeMap::new();
    let mut out: Vec<String> = vec![];
    for s in shows {
        match serde_json::from_value::<Show>(s.clone()) {
            Ok(show) => {
                library.insert(Pinned { id: show.id.clone(), version: show.version }, show);
            }
            Err(e) => out.push(format!("a child show: {e}")),
        }
    }
    if let Err(problems) = resolve(&root, &library) {
        out.extend(problems.iter().map(Problem::to_string));
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
            out.push(Problem::Unknown { what: "show", id: seat.show.to_string(), at: Spot::Seat(seat.id.clone()) });
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
