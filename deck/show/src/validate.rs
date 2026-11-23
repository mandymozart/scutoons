//! Validity is the core's (RULES A2). [`validate`] reads one document; [`resolve`] follows the seats from the top, the only
//! way a tree exists (the child does not name its parent, SHOW.md §4b), and writes down the exact versions a flash carries.
use crate::document::{Addressed, Cell, Face, OutletKind, Pinned, Question, Row, Seat, Show, READER};
use protocol::{Filter, PARENT};
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

/// what a show without its toon is told
pub const NO_TOON: &str = "it names no toon – pick the show's toon";

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
        Cell::Play { piece, send, shot, .. } => {
            match piece {
                Some(p) => {
                    let p = place(at, p, out);
                    if let Some(s) = shot {
                        if p.is_some_and(|p| p.kind != "scene") {
                            out.push(refused(at, "a shot belongs to a scene's camera – the piece is no scene"));
                        }
                        if let Some(Err(why)) = deck_space::rig::address::CameraWord::parse(&format!("{}:{s}", deck_space::rig::address::SHOT)) {
                            out.push(refused(at, why));
                        }
                    }
                }
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
        Cell::Ask { question } if show.question(question).is_none() => out.push(Problem::Unknown { what: "question", id: question.clone(), at: at.clone() }),
        Cell::Ask { .. } => {}
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
    if let Some(Err(why)) = deck_space::rig::address::PhoneCamera::parse(&a.address, &a.action) {
        out.push(refused(at, why));
    }
    if let Some(f) = &a.filter {
        filter(at, &a.address, f, out);
    }
}

/// #546: a filter (an address's `where`, a column's in a row): no empty group or device, an account list that names someone,
/// each account once
fn filter(at: &Spot, of: &str, f: &Filter, out: &mut Vec<Problem>) {
    if f.group.as_deref() == Some("") || f.device.as_deref() == Some("") {
        out.push(refused(at, format!("{of}: an empty filter")));
    }
    if let Some(list) = &f.account {
        if list.is_empty() {
            out.push(refused(at, format!("{of}: the filter's account list names nobody (leave it out: every account)")));
        }
        let mut seen = BTreeSet::new();
        for a in list.iter().filter(|a| !seen.insert(a.as_str())) {
            out.push(refused(at, format!("{of}: 'account:{}' twice", a.as_str())));
        }
    }
}

/// #546: a question of the show – an id, typed or the story's (one of them), its options' labels, the form's checks (deck-ui
/// `vote::problem`, the one place)
fn question(q: &Question, out: &mut Vec<Problem>) {
    let at = Spot::Show;
    let id = &q.id;
    match (&q.typed, &q.story) {
        (Some(_), Some(_)) => out.push(refused(&at, format!("question '{id}': typed or the story's, not both"))),
        (None, None) => out.push(refused(&at, format!("question '{id}': neither typed nor the story's"))),
        (None, Some(s)) if s.toon.is_empty() || s.event.is_empty() => out.push(refused(&at, format!("question '{id}': the story's choice names no toon or event"))),
        _ => {}
    }
    let options = q.options();
    for (i, o) in options.iter().enumerate() {
        if protocol::content_text::pick_text(o, "en", None).is_none() {
            out.push(refused(&at, format!("question '{id}': option {} has no label", i + 1)));
        }
    }
    let rule = crate::vote::rule(q.rule);
    if let Some(why) = deck_ui::vote::problem(options.len(), rule, q.n.unwrap_or(0), q.seconds, q.fallback) {
        out.push(refused(&at, format!("question '{id}': {why}")));
    }
}

/// #546: a row's questions – one per row (the relay runs one vote at a time), the same filter on every column that asks it
/// (the relay tests one per vote); a column's filter on a stage outlet of the show
fn asks(show: &Show, row: &Row, out: &mut Vec<Problem>) {
    let at = Spot::Row(row.id.clone());
    let asked: BTreeSet<&str> = row.cells.values().filter_map(|c| if let Cell::Ask { question } = c { Some(question.as_str()) } else { None }).collect();
    if asked.len() > 1 {
        let names: Vec<String> = asked.iter().map(|q| format!("'{q}'")).collect();
        out.push(refused(&at, format!("one question per row, it asks {}", names.join(" and "))));
    }
    for q in asked {
        let columns: Vec<&str> = row.cells.iter().filter(|(_, c)| matches!(c, Cell::Ask { question } if question == q)).map(|(k, _)| k.as_str()).collect();
        let first = row.r#where.get(columns[0]);
        if columns.iter().any(|c| row.r#where.get(*c) != first) {
            out.push(refused(&at, format!("the columns of question '{q}' ({}) carry different filters", columns.join(", "))));
        }
    }
    for (column, f) in &row.r#where {
        let at = Spot::Cell { row: row.id.clone(), column: column.clone() };
        match show.outlet(column) {
            Some(o) if o.kind == OutletKind::Stage => {}
            Some(_) => out.push(refused(&at, "a filter is for a stage outlet's column, a device outlet has no audience")),
            None if show.seat(column).is_some() => out.push(refused(&at, "a filter is for a stage outlet's column, not a seat's")),
            None => out.push(Problem::Unknown { what: "outlet", id: column.clone(), at: at.clone() }),
        }
        filter(&at, &format!("the filter of '{column}'"), f, out);
    }
}

/// #478: a role's filter – a stage outlet's only (proposal, no owner word yet: a device outlet is a bridge's, no deck takes
/// it, so it is no role); every fact once, each a
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

/// A column group is one head over adjacent outlets: named, and not split by another column.
fn groups(show: &Show, out: &mut Vec<Problem>) {
    let mut ended = BTreeSet::new();
    let mut last: Option<&str> = None;
    for o in &show.outlets {
        let group = o.group.as_deref();
        if group.is_some_and(|g| g.trim().is_empty()) {
            out.push(refused(&Spot::Show, format!("outlet '{}': an empty group (leave it out: no group)", o.id)));
        }
        if group != last {
            if let Some(g) = group.filter(|g| ended.contains(*g)) {
                out.push(refused(&Spot::Show, format!("outlet '{}': the group '{g}' is split, its columns stand apart", o.id)));
            }
            ended.extend(last);
        }
        last = group;
    }
}

/// The reader the show says it needs: no more than this deck's, and no less than what it uses – else an older deck would
/// take a show it misreads (the editor stamps it on every edit, `Show::stamp`).
fn reader(show: &Show, out: &mut Vec<Problem>) {
    let (says, needs) = (show.reads.unwrap_or(1), show.reader_needed());
    if says > READER {
        out.push(refused(&Spot::Show, format!("it needs a newer deck: show reader {says}, this deck reads {READER} – update the deck")));
    } else if says < needs {
        out.push(refused(&Spot::Show, format!("a row fires on several events or on a row: it needs \"reads\": {needs}, so an older deck refuses it")));
    }
}

/// One document on its own. Empty = valid.
pub fn validate(show: &Show) -> Vec<Problem> {
    let mut out = vec![];
    if show.id.is_empty() {
        out.push(refused(&Spot::Show, "an empty id"));
    }
    if show.toons.is_empty() {
        out.push(refused(&Spot::Show, NO_TOON));
    }
    for (i, toon) in show.toons.iter().enumerate() {
        if toon.is_empty() || !toon.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
            out.push(refused(&Spot::Show, format!("the toon '{toon}' is no toon id")));
        } else if show.toons[..i].contains(toon) {
            out.push(refused(&Spot::Show, format!("the toon '{toon}' is named twice")));
        }
    }
    if let Some(e) = show.background.as_ref().and_then(deck_space::Background::check) {
        out.push(refused(&Spot::Show, format!("background: {e}")));
    }
    if let Some(e) = show.tempo.as_ref().and_then(crate::document::Tempo::check) {
        out.push(refused(&Spot::Show, e));
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
    groups(show, &mut out);

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

    reader(show, &mut out);
    for row in &show.rows {
        // several rows may listen to one event, a row to several; a row's fire names a row of this show, not its own
        let at = Spot::Row(row.id.clone());
        for (i, event) in row.fire.event.iter().enumerate() {
            if event.trim().is_empty() {
                out.push(refused(&at, "an empty event id"));
            } else if row.fire.event.iter().take(i).any(|e| e == event) {
                out.push(refused(&at, format!("the event '{event}' is named twice")));
            } else if let Some(other) = event.strip_prefix(crate::document::ROW_FIRED) {
                if other == row.id {
                    out.push(refused(&at, "a row cannot fire on its own fire"));
                } else if show.row(other).is_none() {
                    out.push(Problem::Unknown { what: "row", id: other.to_string(), at: at.clone() });
                }
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
        asks(show, row, &mut out);
        for w in &row.wires {
            if let Some(why) = crate::wiring::check(show, row, w) {
                out.push(refused(&Spot::Row(row.id.clone()), why));
            }
        }
    }

    // #546: each question once, checked, and asked somewhere
    let mut questions = BTreeSet::new();
    unique("question", show.questions.iter().map(|q| &q.id), &mut questions, &mut out);
    for q in &show.questions {
        question(q, &mut out);
        if !show.rows.iter().any(|r| r.cells.values().any(|c| matches!(c, Cell::Ask { question } if *question == q.id))) {
            out.push(refused(&Spot::Show, format!("question '{}' is asked in no row", q.id)));
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
