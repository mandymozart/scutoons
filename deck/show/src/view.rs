//! What the deck editor's Show and Perform workspaces draw (#91, SHOW.md §3: "editor, conductor and web only draw it"): the
//! matrix of rows × columns with each cell's words and state, the log's lines, a row's inspector, the conductors rail. The
//! words and every decision about a cell's state live here once (RULES A1, A6); the editor lays them out and hands the
//! operator's presses back to the [`Subtree`].
use crate::conductor::{Entry, Logged, OnStage, Origin, Playing, Suggestion};
use crate::document::{Addressed, Cell, Fire, How, Moment, OutletKind, Show, Values, Word};
use crate::subtree::Subtree;
use protocol::DeviceMsg;

/// A column of the matrix: an outlet, or a seat (a sub conductor).
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub id: String,
    /// a seat: its cells say `go to <row>` / `free` to the child
    pub seat: bool,
    /// under the id: `stage`, `device`, `sub · follow`
    pub detail: String,
}

/// One cell as the operator sees it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CellView {
    /// `scene:nave · light 0.2`, `values · rage 0.8`, `↑` (keep), `black`, `go to idle`
    pub text: String,
    /// `keep`: nothing happens on this column (drawn quiet)
    pub keep: bool,
    /// this row's cell is what plays there now
    pub on: bool,
    /// it plays because the operator launched it off its row (marked, SHOW.md §7)
    pub hand: bool,
    /// why it would not play as written (`wrong piece`: a `values` cell meets another piece)
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RowView {
    pub id: String,
    /// 1-based, as the operator counts
    pub number: usize,
    /// `roar · suggest`, `start · auto`, `hand`
    pub fires: String,
    /// the row that fired last
    pub current: bool,
    /// held as a suggestion
    pub suggested: bool,
    pub cells: Vec<CellView>,
}

/// The suggestion card: the row, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Suggested {
    pub row: String,
    pub title: String,
    pub why: String,
}

/// One sub conductor (a seat) on the rail.
#[derive(Debug, Clone, PartialEq)]
pub struct SeatView {
    pub id: String,
    /// `sub · follow`, `sub · suggest`
    pub detail: String,
    /// who sits there, or `nobody yet`
    pub sits: String,
    /// the child's show at its version (`foyer-loop@2`)
    pub show: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub columns: Vec<Column>,
    pub rows: Vec<RowView>,
    pub suggestion: Option<Suggested>,
    /// what GO does now, in words (`GO: row 3 ROAR`, `GO: next row 2 sermon`, `GO: no next row`)
    pub go: String,
}

fn number(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Number(n) => {
            let f = n.as_f64().unwrap_or_default();
            let s = format!("{f:.2}");
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// `rage 0.8 · mood wild` (the values in the document's order)
pub fn values_text(values: &Values) -> String {
    values.iter().map(|(k, v)| format!("{k} {}", number(v))).collect::<Vec<_>>().join(" · ")
}

fn device_text(m: &DeviceMsg) -> String {
    m.kind().to_string()
}

/// `look warm · strobe @1 · panel @10`: a look and the fixtures it lights on `outlet`, by their first channel
pub fn look_text(show: &Show, outlet: &str, look: &str) -> String {
    let lit = show.look(look).map(|l| {
        let mut f: Vec<_> = show.fixtures.iter().filter(|f| f.outlet == outlet && l.values.contains_key(&f.id)).collect();
        f.sort_by_key(|f| f.address);
        f.iter().map(|f| format!("{} @{}", f.id, f.address)).collect::<Vec<_>>()
    });
    match lit {
        Some(lit) if !lit.is_empty() => format!("look {look} · {}", lit.join(" · ")),
        Some(_) => format!("look {look} · lights nothing here"),
        None => format!("look {look} · unknown"),
    }
}

/// `character:yellowmouse > dance:samba`, `person:* > haptic:pulse where group left · device browser` (SHOW.md §8, §9)
pub fn address_text(a: &Addressed) -> String {
    let mut s = format!("{} > {}", a.address, a.action);
    if let Some(f) = &a.filter {
        let parts: Vec<String> = [("group", &f.group), ("device", &f.device)].iter().filter_map(|(k, v)| v.as_ref().map(|v| format!("{k} {v}"))).collect();
        if !parts.is_empty() {
            s = format!("{s} where {}", parts.join(" · "));
        }
    }
    s
}

/// A cell's words (on its column; `device` = a device outlet).
pub fn cell_text(cell: &Cell) -> String {
    match cell {
        Cell::Play { piece: Some(p), values, .. } if values.is_empty() => p.clone(),
        Cell::Play { piece: Some(p), values, .. } => format!("{p} · {}", values_text(values)),
        Cell::Play { piece: None, send, .. } if !send.is_empty() => format!("send {}", send.iter().map(device_text).collect::<Vec<_>>().join(" · ")),
        Cell::Play { piece: None, .. } => "nothing".into(),
        Cell::Values { values, .. } => format!("values · {}", values_text(values)),
        Cell::Keep => "↑".into(),
        Cell::Mirror { of } => format!("= {of}"),
        Cell::Black => "black".into(),
        Cell::Go { row } => format!("go to {row}"),
        Cell::Free => "free".into(),
        Cell::Look { look, fade: None } => format!("look {look}"),
        Cell::Look { look, fade: Some(ms) } => format!("look {look} · fade {:.1} s", *ms as f32 / 1000.0),
    }
}

/// What an outlet plays, in words (the stage now).
pub fn playing_text(p: &Playing) -> String {
    match p {
        Playing::Piece { piece, values } if values.is_empty() => piece.clone(),
        Playing::Piece { piece, values } => format!("{piece} · {}", values_text(values)),
        Playing::Mirror { of } => format!("= {of}"),
        Playing::Black => "black".into(),
        Playing::Sent => "sent to the device".into(),
    }
}

pub fn how_text(how: How) -> &'static str {
    match how {
        How::Auto => "auto",
        How::Suggest => "suggest",
        How::Hand => "hand",
    }
}

pub fn moment_text(m: Moment) -> &'static str {
    match m {
        Moment::Now => "now",
        Moment::Bar => "next bar",
        Moment::Phrase => "next phrase",
    }
}

pub fn word_text(w: Word) -> &'static str {
    match w {
        Word::Follow => "follow",
        Word::Suggest => "suggest",
        Word::Free => "free",
    }
}

/// `roar · suggest`, or `hand` for a row only the operator fires
pub fn fires(fire: &Fire) -> String {
    match &fire.event {
        Some(e) => format!("{e} · {}", how_text(fire.how)),
        None => "hand".into(),
    }
}

fn origin_text(o: Origin) -> &'static str {
    match o {
        Origin::Auto => "auto",
        Origin::Hand => "hand",
        Origin::Go => "GO",
        Origin::Parent => "the parent",
    }
}

/// One log entry in words (SHOW.md §7, §8: reached, passed, answered; nobody).
pub fn line(what: &Logged) -> String {
    match what {
        Logged::Fired { row, origin } => format!("fired {row} ({})", origin_text(*origin)),
        Logged::Suggested { row } => format!("suggests {row}"),
        Logged::Withdrawn { row } => format!("{row}: suggestion gone"),
        Logged::Ignored { row, event } => format!("{event}: {row} is by hand, ignored"),
        Logged::NoRow { event } => format!("{event}: no row listens"),
        Logged::CellLaunched { row, column, off_row: true } => format!("hand: {column} > {row}, off the row"),
        Logged::CellLaunched { row, column, off_row: false } => format!("hand: {column} > {row}"),
        Logged::ValuesSkipped { row, outlet, expected, found } => {
            format!("{row}: {outlet} plays {}, not {expected} – values skipped", found.as_deref().unwrap_or("nothing"))
        }
        Logged::NoNextRow => "GO: no next row".into(),
        Logged::ParentIgnored { row: Some(r) } => format!("the parent's {r}: ignored"),
        Logged::ParentIgnored { row: None } => "the parent: free, ignored".into(),
        Logged::Addressed { address, action, reached: 0 } => format!("{address} > {action}: nobody"),
        Logged::Addressed { address, action, reached } => format!("{address} > {action}: reached {reached}"),
        Logged::Reached { address, action, reached: 0, .. } => format!("{address} > {action}: nobody"),
        Logged::Reached { address, action, reached, passed, answered } => format!("{address} > {action}: reached {reached} · passed {passed} · answered {answered}"),
        Logged::Locked => "locked from above".into(),
        Logged::Unlocked => "the lock from above went".into(),
        Logged::WordRefused => "the word stays: locked from above".into(),
        Logged::LockRefused { seat } => format!("{seat} cannot be locked"),
        Logged::NotUsed { seat, what } => format!("{seat}: {what} is not used of its face"),
        Logged::ClockKept => "own clock: the parent's ignored".into(),
        Logged::ClockRefused => "on the parent's clock: does not lead".into(),
        Logged::ParentLost => "the parent went away – playing on".into(),
        Logged::ParentBack => "the parent is back".into(),
        Logged::Flashed { root, hops } => format!("flashed {root} ({hops} hops)"),
        Logged::FlashRefused { why } => format!("flash refused: {why}"),
        Logged::Installed => "the flashed show runs".into(),
        Logged::Reported { from, running, refused: None } => format!("{from} runs {running}"),
        Logged::Reported { from, running, refused: Some(why) } => format!("{from} runs {running}, refused: {why}"),
    }
}

/// `hh:mm:ss` of a log entry (relay ms, UTC)
pub fn time(ms: u64) -> String {
    let s = (ms / 1000) % 86_400;
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

/// The log, newest first, at most `n` lines: (time, words).
pub fn log<'a>(entries: impl Iterator<Item = &'a Entry>, n: usize) -> Vec<(String, String)> {
    let all: Vec<&Entry> = entries.collect();
    all.into_iter().rev().take(n).map(|e| (time(e.ms), line(&e.what))).collect()
}

fn columns(show: &Show) -> Vec<Column> {
    let outlets = show.outlets.iter().map(|o| Column {
        id: o.id.clone(),
        seat: false,
        detail: match o.kind {
            OutletKind::Stage => "stage".into(),
            OutletKind::Device => "device".into(),
        },
    });
    let seats = show.seats.iter().map(|s| Column { id: s.id.clone(), seat: true, detail: format!("sub · {}", word_text(s.word)) });
    outlets.chain(seats).collect()
}

/// a cell's state against what plays on its column now
fn cell_view(row: &str, cell: Option<&Cell>, stage: Option<&OnStage>) -> CellView {
    let Some(cell) = cell.filter(|c| !matches!(c, Cell::Keep)) else {
        return CellView { text: cell_text(&Cell::Keep), keep: true, ..CellView::default() };
    };
    let mine = stage.filter(|s| s.row == row);
    let problem = match (cell, stage.map(|s| &s.playing)) {
        (Cell::Values { piece, .. }, Some(Playing::Piece { piece: there, .. })) if there != piece => Some("wrong piece".to_string()),
        (Cell::Values { .. }, None | Some(Playing::Black)) => Some("nothing plays".to_string()),
        _ => None,
    };
    CellView { text: cell_text(cell), keep: false, on: mine.is_some(), hand: mine.is_some_and(|s| s.off_row), problem }
}

fn suggested(show: &Show, s: &Suggestion) -> Suggested {
    let n = show.rows.iter().position(|r| r.id == s.row).map_or(0, |i| i + 1);
    let why = match &s.event {
        Some(e) => format!("event: {e}, from the reader"),
        None => "the parent's row".to_string(),
    };
    Suggested { row: s.row.clone(), title: format!("row {n} {}", s.row), why }
}

/// What GO fires now: the suggestion, else the row after the current one (the conductor's `go`).
fn go_text(show: &Show, current: Option<&str>, s: Option<&Suggestion>) -> String {
    if let Some(s) = s {
        return format!("GO: {}", suggested(show, s).title);
    }
    let next = current.and_then(|c| show.rows.iter().position(|r| r.id == c)).map_or(0, |i| i + 1);
    match show.rows.get(next) {
        Some(r) => format!("GO: next row {} {}", next + 1, r.id),
        None => "GO: no next row".into(),
    }
}

/// The Perform matrix of one conductor: rows × (outlets, then seats), each cell against what plays.
pub fn matrix(sub: &Subtree) -> Matrix {
    let c = sub.conductor();
    let show = c.show();
    let columns = columns(show);
    let rows = show
        .rows
        .iter()
        .enumerate()
        .map(|(i, r)| RowView {
            id: r.id.clone(),
            number: i + 1,
            fires: fires(&r.fire),
            current: c.current() == Some(r.id.as_str()),
            suggested: c.suggestion().is_some_and(|s| s.row == r.id),
            cells: columns.iter().map(|col| cell_view(&r.id, r.cells.get(&col.id), c.on_stage(&col.id))).collect(),
        })
        .collect();
    Matrix { columns, rows, suggestion: c.suggestion().map(|s| suggested(show, s)), go: go_text(show, c.current(), c.suggestion()) }
}

/// The same matrix of a document no conductor runs (the Show workspace: authoring, nothing plays).
pub fn plan(show: &Show) -> Matrix {
    let columns = columns(show);
    let rows = show
        .rows
        .iter()
        .enumerate()
        .map(|(i, r)| RowView {
            id: r.id.clone(),
            number: i + 1,
            fires: fires(&r.fire),
            current: false,
            suggested: false,
            cells: columns.iter().map(|col| cell_view(&r.id, r.cells.get(&col.id), None)).map(|mut v| {
                v.problem = None;
                v
            }).collect(),
        })
        .collect();
    Matrix { columns, rows, suggestion: None, go: go_text(show, None, None) }
}

/// The seats of a show: its sub conductors, for the rail and the tree.
pub fn seats(show: &Show) -> Vec<SeatView> {
    show.seats
        .iter()
        .map(|s| SeatView {
            id: s.id.clone(),
            detail: format!("sub · {}", word_text(s.word)),
            sits: s.sits.clone().unwrap_or_else(|| "nobody yet".into()),
            show: s.show.to_string(),
        })
        .collect()
}

/// One deck's or sub conductor's answer to the last flash (#91), for the Show workspace.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportView {
    /// the path it came along (a client id, `foyer/<client>` from a sub conductor's session)
    pub from: String,
    /// `outlet left`, or `conductor`
    pub what: String,
    /// `FLASH top@3 ready`, `FLASH top@3: 2 missing`, `FLASH refused: …`
    pub line: String,
    /// what it lacks, one line each
    pub lacks: Vec<String>,
    /// nothing lacks and nothing was refused
    pub ok: bool,
}

/// The answers to the last flash, in the order of their paths.
pub fn reports(sub: &Subtree) -> Vec<ReportView> {
    sub.reports()
        .iter()
        .map(|(from, r)| ReportView {
            from: from.clone(),
            what: r.outlet.as_ref().map_or("conductor".to_string(), |o| format!("outlet {o}")),
            line: crate::ready::line(r),
            lacks: r.lacks.clone().unwrap_or_default(),
            ok: r.accepted && r.ready != Some(false),
        })
        .collect()
}

/// One line of the cue sheet (#91 item 6): a row in order, how it fires, what each column does when it fires.
#[derive(Debug, Clone, PartialEq)]
pub struct Cue {
    pub number: usize,
    pub row: String,
    /// `ch3.dialog-start · auto · now`, `ch3.yellowmouse-enters · suggest · hold 1.5 s · next bar`, `by hand`
    pub trigger: String,
    /// (column, words) for each column the row changes; a kept column is not listed
    pub does: Vec<(String, String)>,
    /// what else it sends: device messages, addresses
    pub sends: Vec<String>,
}

/// `ch3.end · hand` → the trigger in words, with the hold and the moment when they say something
pub fn trigger(fire: &Fire) -> String {
    let Some(e) = &fire.event else {
        return match fire.moment {
            Moment::Now => "by hand".into(),
            m => format!("by hand · {}", moment_text(m)),
        };
    };
    let mut s = format!("{e} · {}", how_text(fire.how));
    if fire.hold_ms > 0 {
        s = format!("{s} · hold {:.1} s", fire.hold_ms as f32 / 1000.0);
    }
    format!("{s} · {}", moment_text(fire.moment))
}

/// What a cell does when its row fires, in words (`plays scene:nave · light 0.2`, `glides scene:nave: dim 0.5`, `= left`)
pub fn does(cell: &Cell) -> String {
    match cell {
        Cell::Play { piece: Some(_), .. } => format!("plays {}", cell_text(cell)),
        Cell::Values { piece, values } => format!("glides {piece}: {}", values_text(values)),
        Cell::Mirror { of } => format!("shows what {of} shows"),
        other => cell_text(other),
    }
}

/// The cue sheet: every row in order, its trigger, what each outlet and seat does.
pub fn cue_sheet(show: &Show) -> Vec<Cue> {
    let cols = columns(show);
    show.rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut sends = vec![];
            let does = cols
                .iter()
                .filter_map(|c| {
                    let cell = r.cells.get(&c.id).filter(|c| !matches!(c, Cell::Keep))?;
                    if let Cell::Play { piece, send, .. } = cell {
                        sends.extend(send.iter().map(|m| format!("{} · {}", c.id, device_text(m))));
                        if piece.is_none() {
                            // a device's cell is what it sends, said once
                            return None;
                        }
                    }
                    Some((c.id.clone(), does(cell)))
                })
                .collect();
            sends.extend(r.addresses.iter().map(address_text));
            Cue { number: i + 1, row: r.id.clone(), trigger: trigger(&r.fire), does, sends }
        })
        .collect()
}

/// What a problem points at in the document shown: a click selects it (#91 item 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Points {
    /// the document as a whole
    Show,
    /// a row (by index)
    Row(usize),
    /// a row's cell: row index, column index (outlets, then seats, as the matrix)
    Cell(usize, usize),
    /// a seat (by index)
    Seat(usize),
}

/// One line of the problems list.
#[derive(Debug, Clone, PartialEq)]
pub struct ProblemView {
    /// the core's words
    pub text: String,
    /// an error (the document is not valid) or a warning (a deck's report: it never blocks GO)
    pub error: bool,
    pub points: Points,
}

/// Everything wrong with the document shown, then what the decks lack after the last flash (when `sub` runs it). Errors
/// come from `validate`, warnings from the flash reports; each points at its row, cell or seat.
pub fn problems(show: &Show, sub: Option<&Subtree>) -> Vec<ProblemView> {
    use crate::validate::Spot;
    let cols = columns(show);
    let row = |id: &str| show.rows.iter().position(|r| r.id == id);
    let mut out: Vec<ProblemView> = crate::validate::validate(show)
        .into_iter()
        .map(|p| {
            let points = match p.spot() {
                Spot::Row(r) => row(&r).map_or(Points::Show, Points::Row),
                Spot::Cell { row: r, column } => match (row(&r), cols.iter().position(|c| c.id == column)) {
                    (Some(r), Some(c)) => Points::Cell(r, c),
                    (Some(r), None) => Points::Row(r),
                    _ => Points::Show,
                },
                Spot::Seat(s) => show.seats.iter().position(|x| x.id == s).map_or(Points::Show, Points::Seat),
                Spot::Show | Spot::Face => Points::Show,
            };
            ProblemView { text: p.to_string(), error: true, points }
        })
        .collect();
    for r in sub.map(reports).unwrap_or_default() {
        if r.ok {
            continue;
        }
        let head = format!("{} · {}", r.what, r.from);
        if r.lacks.is_empty() {
            out.push(ProblemView { text: format!("{head}: {}", r.line), error: false, points: Points::Show });
        }
        for l in &r.lacks {
            let points = crate::ready::lack_row(l, show).and_then(row).map_or(Points::Show, Points::Row);
            out.push(ProblemView { text: format!("{head}: {l}"), error: false, points });
        }
    }
    out
}

/// One section of the row inspector: a heading and its (label, value) lines.
pub type Section = (String, Vec<(String, String)>);

/// The Show workspace's inspector for `row`: how it fires, its cells, what it sends (SHOW.md §7, §8).
pub fn inspector(show: &Show, row: &str) -> Vec<Section> {
    let Some((i, r)) = show.rows.iter().enumerate().find(|(_, r)| r.id == row) else { return vec![] };
    let fire = vec![
        ("fires on".to_string(), r.fire.event.as_ref().map_or("by hand only".to_string(), |e| format!("event: {e}"))),
        ("how".to_string(), how_text(r.fire.how).to_string()),
        ("hold".to_string(), format!("{:.1} s", r.fire.hold_ms as f32 / 1000.0)),
        ("moment".to_string(), moment_text(r.fire.moment).to_string()),
    ];
    let mut cells = Vec::new();
    let mut sends = Vec::new();
    let mut subs = Vec::new();
    for col in columns(show) {
        let cell = r.cells.get(&col.id).unwrap_or(&Cell::Keep);
        if col.seat {
            subs.push((col.id.clone(), cell_text(cell)));
            continue;
        }
        cells.push((col.id.clone(), cell_text(cell)));
        if let Cell::Play { send, .. } = cell {
            for m in send {
                sends.push((col.id.clone(), device_text(m)));
            }
        }
        if let Cell::Look { look, .. } = cell {
            sends.push((col.id.clone(), look_text(show, &col.id, look)));
        }
    }
    for a in &r.addresses {
        sends.push(("address".to_string(), address_text(a)));
    }
    let mut out = vec![(format!("row {} · {}", i + 1, r.id), fire), ("cells of this row".to_string(), cells)];
    out.push((
        "this row sends".to_string(),
        if sends.is_empty() { vec![("".to_string(), "only its cells – nobody else is addressed".to_string())] } else { sends },
    ));
    if !subs.is_empty() {
        out.push(("sub conductors in this row".to_string(), subs));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse;
    use crate::subtree::ClockChoice;

    const SERMON: &str = r#"{
      "id": "sermon", "version": 3,
      "outlets": [{ "id": "left" }, { "id": "right" }, { "id": "lights", "kind": "device" }],
      "seats": [{ "id": "foyer", "show": { "id": "foyer-loop", "version": 2 }, "word": "suggest", "uses": { "rows": ["storm"] } }],
      "rows": [
        { "id": "arrival", "fire": { "event": "start", "how": "auto" }, "cells": { "left": { "mode": "play", "piece": "scene:nave" }, "right": { "mode": "black" } } },
        { "id": "roar", "fire": { "event": "roar", "how": "suggest", "hold_ms": 2000, "moment": "bar" },
          "cells": { "left": { "mode": "values", "piece": "scene:nave", "values": { "light": 0.8 } }, "right": { "mode": "values", "piece": "scene:choir", "values": { "x": 1 } },
                     "foyer": { "mode": "go", "row": "storm" } } },
        { "id": "exit", "cells": { "left": { "mode": "black" } } }
      ]
    }"#;

    #[test]
    fn the_matrix_says_what_plays_where_and_what_would_not() {
        let mut sub = Subtree::new(parse(SERMON).unwrap(), ClockChoice::Parent);
        let m = matrix(&sub);
        assert_eq!(m.columns.iter().map(|c| (c.id.as_str(), c.detail.as_str())).collect::<Vec<_>>(), [("left", "stage"), ("right", "stage"), ("lights", "device"), ("foyer", "sub · suggest")]);
        assert_eq!(m.rows[1].fires, "roar · suggest");
        assert_eq!(m.rows[2].fires, "hand");
        assert_eq!(m.rows[1].cells[0].text, "values · light 0.8");
        assert!(m.rows[1].cells[2].keep && m.rows[1].cells[2].text == "↑");
        assert_eq!(m.go, "GO: next row 1 arrival");
        sub.fire("arrival", 1000);
        let m = matrix(&sub);
        assert!(m.rows[0].current && m.rows[0].cells[0].on && !m.rows[0].cells[0].hand);
        assert_eq!(m.rows[1].cells[1].problem.as_deref(), Some("nothing plays"), "right is black");
        assert_eq!(m.rows[1].cells[0].problem, None, "the same piece");
        sub.launch_cell("exit", "left", 2000);
        let m = matrix(&sub);
        assert!(m.rows[2].cells[0].on && m.rows[2].cells[0].hand, "a hand launch off its row is marked");
        assert_eq!(m.go, "GO: next row 2 roar");
    }

    #[test]
    fn a_suggestion_and_its_drop_and_the_log_in_words() {
        let mut sub = Subtree::new(parse(SERMON).unwrap(), ClockChoice::Parent);
        let story = serde_json::json!({ "id": "roar" }).as_object().cloned().unwrap();
        sub.on_wire(&protocol::Envelope::new("reader", protocol::Message::Story(story)), 0);
        sub.tick(2500);
        let m = matrix(&sub);
        assert_eq!(m.suggestion.as_ref().map(|s| (s.title.as_str(), s.why.as_str())), Some(("row 2 roar", "event: roar, from the reader")));
        assert!(m.rows[1].suggested);
        assert_eq!(m.go, "GO: row 2 roar");
        assert!(sub.drop_suggestion(3000));
        assert!(matrix(&sub).suggestion.is_none());
        assert!(!sub.drop_suggestion(3100), "nothing held");
        let lines = log(sub.log(), 10);
        assert_eq!(lines[0], ("00:00:03".to_string(), "roar: suggestion gone".to_string()));
        assert_eq!(lines[1].1, "suggests roar");
    }

    #[test]
    fn a_rows_addresses_show_in_the_inspector_and_go_down_when_it_fires() {
        let doc = include_str!("../fixtures/top-toy.show.json");
        let show = parse(doc).unwrap();
        let s = inspector(&show, "yellowmouse-enters");
        let sends = &s.iter().find(|(h, _)| h == "this row sends").unwrap().1;
        assert_eq!(sends, &[("address".to_string(), "character:yellowmouse > dance:samba where group left".to_string()), ("address".to_string(), "location:* > close_doors".to_string())]);
        let mut sub = Subtree::new(show, ClockChoice::Parent);
        let out = sub.fire("yellowmouse-enters", 1000);
        let asked: Vec<&protocol::Address> = out.iter().filter_map(|e| if let protocol::Message::Address(a) = &e.message { Some(a) } else { None }).collect();
        assert_eq!(asked.len(), 2);
        assert_eq!(asked[0].r#where.as_ref().and_then(|w| w.group.as_deref()), Some("left"));
        sub.on_wire(&protocol::Envelope::new("c4", protocol::Message::AddressAnswer(protocol::AddressAnswer { id: asked[0].id.clone(), reached: 1, passed: 1, answered: 1 })), 1200);
        sub.tick(1000 + crate::subtree::ANSWER_MS);
        let lines: Vec<String> = log(sub.log(), 5).into_iter().map(|(_, l)| l).collect();
        assert!(lines.contains(&"character:yellowmouse > dance:samba: reached 1 · passed 1 · answered 1".to_string()), "{lines:?}");
        assert!(lines.contains(&"location:* > close_doors: nobody".to_string()), "{lines:?}");
    }

    #[test]
    fn the_cue_sheet_says_each_row_in_order_and_what_changes() {
        let show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        let cues = cue_sheet(&show);
        assert_eq!(cues.len(), 4);
        assert_eq!(cues[0].trigger, "by hand");
        assert_eq!(cues[0].does[0], ("left".to_string(), "plays scene:nave · light 0.2".to_string()));
        assert_eq!(cues[0].does[1], ("right".to_string(), "shows what left shows".to_string()));
        assert_eq!(cues[0].sends, ["lamps · dmx"]);
        assert_eq!(cues[1].does, [("left".to_string(), "glides scene:nave: dim 0.5".to_string()), ("lamps".to_string(), "look warm · fade 0.8 s".to_string())], "a kept column is not listed");
        let sends = &inspector(&show, "dialog").into_iter().find(|(h, _)| h == "this row sends").unwrap().1;
        assert_eq!(sends, &[("lamps".to_string(), "look warm · strobe @1 · panel @10".to_string())]);
        assert_eq!(cues[2].trigger, "ch3.yellowmouse-enters · suggest · hold 1.5 s · next bar");
        assert_eq!(cues[2].sends, ["character:yellowmouse > dance:samba where group left", "location:* > close_doors"]);
        assert_eq!(cues[3].trigger, "ch3.end · hand · now");
    }

    #[test]
    fn the_problems_point_at_their_row_cell_or_seat_and_the_decks_reports_are_warnings() {
        let mut doc: serde_json::Value = serde_json::from_str(include_str!("../fixtures/top-toy.show.json")).unwrap();
        doc["rows"][1]["cells"]["foyer"] = serde_json::json!({ "mode": "go", "row": "nowhere" });
        doc["rows"][2]["addresses"][0]["action"] = serde_json::json!("dance samba");
        doc["seats"][0]["sits"] = serde_json::json!("phone:x");
        let show: Show = serde_json::from_value(doc).unwrap();
        let mut sub = Subtree::new(show.clone(), ClockChoice::Parent);
        let mut lack = crate::flash::report(&crate::Pinned { id: "top-toy".into(), version: 1 }, true, None);
        lack.outlet = Some("left".into());
        lack.ready = Some(false);
        lack.lacks = Some(vec!["row yellowmouse-enters: character:yellowmouse is not on this deck".into()]);
        sub.on_wire(&protocol::Envelope::new("c7", protocol::Message::FlashReport(lack)), 10);
        let p = problems(&show, Some(&sub));
        let points: Vec<(&Points, bool)> = p.iter().map(|p| (&p.points, p.error)).collect();
        assert_eq!(points, [(&Points::Seat(0), true), (&Points::Cell(1, 3), true), (&Points::Row(2), true), (&Points::Row(2), false)], "{p:?}");
        assert_eq!(p[3].text, "outlet left · c7: row yellowmouse-enters: character:yellowmouse is not on this deck");
        assert_eq!(reports(&sub)[0].line, "FLASH top-toy@1: 1 missing");
    }

    #[test]
    fn the_inspector_and_the_seats() {
        let show = parse(SERMON).unwrap();
        let s = inspector(&show, "roar");
        assert_eq!(s[0].0, "row 2 · roar");
        assert_eq!(s[0].1, [("fires on".into(), "event: roar".into()), ("how".into(), "suggest".into()), ("hold".into(), "2.0 s".into()), ("moment".into(), "next bar".into())]);
        assert_eq!(s[1].1[0], ("left".to_string(), "values · light 0.8".to_string()));
        assert_eq!(s.last().unwrap().1, [("foyer".to_string(), "go to storm".to_string())]);
        assert_eq!(seats(&show)[0], SeatView { id: "foyer".into(), detail: "sub · suggest".into(), sits: "nobody yet".into(), show: "foyer-loop@2".into() });
        assert!(plan(&show).rows.iter().all(|r| r.cells.iter().all(|c| c.problem.is_none() && !c.on)));
        assert_eq!(line(&Logged::Reached { address: "person:*".into(), action: "haptic:pulse".into(), reached: 0, passed: 0, answered: 0 }), "person:* > haptic:pulse: nobody");
    }
}
