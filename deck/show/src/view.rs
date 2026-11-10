//! What the deck editor's Show and Perform workspaces draw (#91, SHOW.md §3: "editor, conductor and web only draw it"): the
//! matrix of rows × columns with each cell's words and state, the log's lines, a row's inspector, the conductors rail. The
//! words and every decision about a cell's state live here once (RULES A1, A6); the editor lays them out and hands the
//! operator's presses back to the [`Subtree`].
use crate::conductor::{Entry, Logged, OnStage, Origin, Playing, Suggestion};
use crate::document::{Cell, Fire, How, Moment, OutletKind, Show, Values, Word};
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
