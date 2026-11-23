//! What the deck editor's Show and Perform workspaces draw (#91, SHOW.md §3: "editor, conductor and web only draw it"): the
//! matrix of rows × columns with each cell's words and state, the log's lines, a row's inspector, the conductors rail. The
//! words and every decision about a cell's state live here once (RULES A1, A6); the editor lays them out and hands the
//! operator's presses back to the [`Subtree`].
use crate::conductor::{Entry, Logged, OnStage, Origin, Playing, Suggestion};
use crate::document::{Addressed, Cell, Fire, How, Moment, OutletKind, Row, Show, Values, Word};
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
    /// #478: its name for people (a role's label, else the id)
    pub name: String,
    /// #478: a role's filter in words (`guest and up`, `live · approved deck`); none: no role (a device outlet, a seat)
    pub filter: Option<String>,
    /// the group whose head stands over it
    pub group: Option<String>,
    /// its stop has something to stop (a seat's always: it leaves the child free)
    pub stops: bool,
}

/// A group's head over adjacent columns: its name, its first column, how many it spans.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupView {
    pub name: String,
    pub first: usize,
    pub span: usize,
}

/// What a cell or a row is doing at the moment the matrix is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Idle,
    /// launched, waiting for its row's moment (the next bar, the next phrase)
    Queued,
    Playing,
}

/// One cell as the operator sees it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CellView {
    pub state: State,
    /// where the bar stands while it plays on a running clock (0–1)
    pub progress: Option<f32>,
    /// `scene:nave · light 0.2`, `values · rage 0.8`, `↑` (keep), `black`, `go to idle`
    pub text: String,
    /// `keep`: nothing happens on this column (drawn quiet)
    pub keep: bool,
    /// the empty cell, a stop slot: launched (or its row fired) the column stops
    pub empty: bool,
    /// this row's cell is what plays there now
    pub on: bool,
    /// it plays because the operator launched it off its row (marked in its corner, SHOW.md §7)
    pub off_row: bool,
    /// why it would not play as written (`wrong piece`: a `values` cell meets another piece)
    pub problem: Option<String>,
    /// #546: a cell of a question (`ask`): its question, where it stands among the row's columns that ask it
    pub ask: Option<AskView>,
}

/// #546 (SHOW.md §15 5): an `ask` cell as the matrix draws it – adjacent cells of one question as one wide cell, one apart
/// with the same mark (`q1 · 2 of 3`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AskView {
    /// the question's id
    pub question: String,
    /// what a person reads for it: the typed question, `story: <event>`, else its id
    pub label: String,
    /// this column's place among the row's columns that ask it (1-based, in the matrix's order) and how many ask it
    pub nth: usize,
    pub of: usize,
    /// it starts a run of adjacent columns that ask it: the wide cell is drawn from here over `run` columns
    pub first: bool,
    pub run: usize,
    /// who in the column gets it, in words (`everyone`, `device phone · member`: deck-ui's filter words)
    pub filter: String,
}

impl AskView {
    /// `q1 · 2 of 3`
    pub fn mark(&self) -> String {
        format!("{} · {} of {}", self.question, self.nth, self.of)
    }

    /// What the cell drawn from here says (#551; none: a column inside a wide cell, drawn by the one that starts it): one
    /// wide cell that holds every column of the question `q1 · Which way?`; a cell apart its mark first, `q1 · 2 of 3 ·
    /// Which way?`; a run among others apart its places, `q1 · 1–2 of 3 · Which way?`.
    pub fn words(&self) -> Option<String> {
        if !self.first {
            return None;
        }
        let label = Some(&self.label).filter(|l| **l != self.question).map(|l| format!(" · {l}")).unwrap_or_default();
        Some(match (self.run == self.of, self.run) {
            (true, _) => format!("{}{label}", self.question),
            (false, 0 | 1) => format!("{}{label}", self.mark()),
            (false, run) => format!("{} · {}–{} of {}{label}", self.question, self.nth, self.nth + run - 1, self.of),
        })
    }
}

/// the `ask` cells of `row` along `cols`, one entry per column (none: no question there)
fn ask_views(show: &Show, row: &Row, cols: &[Column]) -> Vec<Option<AskView>> {
    let asks: Vec<Option<&str>> = cols.iter().map(|c| if let Some(Cell::Ask { question }) = row.cells.get(&c.id) { Some(question.as_str()) } else { None }).collect();
    (0..cols.len())
        .map(|i| {
            let q = asks[i]?;
            let nth = asks[..=i].iter().filter(|a| **a == Some(q)).count();
            let of = asks.iter().filter(|a| **a == Some(q)).count();
            let first = i == 0 || asks[i - 1] != Some(q);
            let run = if first { asks[i..].iter().take_while(|a| **a == Some(q)).count() } else { 0 };
            let label = show.question(q).map_or_else(|| q.to_string(), |x| x.label("en"));
            Some(AskView { question: q.to_string(), label, nth, of, first, run, filter: deck_ui::filter::filter_words(row.r#where.get(&cols[i].id)) })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct RowView {
    pub id: String,
    /// 1-based, as the operator counts
    pub number: usize,
    /// `roar · suggest`, `start · auto`, `manual`
    pub fires: String,
    /// the head's chip; none for a manual row
    pub chip: Option<FireChip>,
    /// when what it launches starts: `now`, `next bar`, `next phrase`
    pub moment: String,
    pub state: State,
    /// the row that fired last
    pub current: bool,
    /// held as a suggestion
    pub suggested: bool,
    pub cells: Vec<CellView>,
}

impl RowView {
    /// The wires between the parts of one question that stand apart in this row: each part's first column to the next
    /// part's, in the row's order.
    pub fn ask_links(&self) -> Vec<(usize, usize)> {
        let starts: Vec<(usize, &str)> = self.cells.iter().enumerate().filter_map(|(c, cell)| cell.ask.as_ref().filter(|a| a.first).map(|a| (c, a.question.as_str()))).collect();
        let mut out = Vec::new();
        for (i, (c, q)) in starts.iter().enumerate() {
            if let Some((next, _)) = starts[i + 1..].iter().find(|(_, x)| x == q) {
                out.push((*c, *next));
            }
        }
        out
    }
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
    pub groups: Vec<GroupView>,
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
    let s = format!("{} > {}", a.address, a.action);
    // #546: the filter's words are deck-ui's, one set for a role's, an address's and a cell's filter
    match a.filter.as_ref().map(deck_ui::filter::filter_facts) {
        Some(facts) if !facts.is_empty() => format!("{s} where {}", deck_ui::filter::words(&facts)),
        _ => s,
    }
}

/// A cell's words (on its column; `device` = a device outlet).
pub fn cell_text(cell: &Cell) -> String {
    match cell {
        Cell::Play { piece: Some(p), values, shot, .. } => {
            let shot = shot.as_deref().map(|s| format!(" > {}:{s}", deck_space::rig::address::SHOT)).unwrap_or_default();
            if values.is_empty() {
                format!("{p}{shot}")
            } else {
                format!("{p}{shot} · {}", values_text(values))
            }
        }
        Cell::Play { piece: None, send, .. } if !send.is_empty() => format!("send {}", send.iter().map(device_text).collect::<Vec<_>>().join(" · ")),
        Cell::Play { piece: None, .. } => "nothing".into(),
        Cell::Values { piece, values } if values.is_empty() => format!("values · {piece}"),
        Cell::Values { values, .. } => format!("values · {}", values_text(values)),
        Cell::Keep => "↑".into(),
        Cell::Mirror { of } => format!("= {of}"),
        Cell::Black => "stop".into(),
        Cell::Go { row } => format!("go to {row}"),
        Cell::Free => "free".into(),
        Cell::Look { look, fade: None } => format!("look {look}"),
        Cell::Look { look, fade: Some(ms) } => format!("look {look} · fade {:.1} s", *ms as f32 / 1000.0),
        Cell::Ask { question } => format!("ask {question}"),
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
        How::Manual => "manual",
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

/// `roar · suggest`, or `manual` for a row only the operator fires
pub fn fires(fire: &Fire) -> String {
    match fire.event.is_empty() {
        false => format!("{} · {}", fire.event.words(), how_text(fire.how)),
        true => how_text(How::Manual).into(),
    }
}

/// How a row fires, drawn as a tag in its head: an event that fires it by itself (`auto`, a filled bolt) or offers it and
/// waits for GO (`suggest`, dashed). A manual row has none: it waits for its ▶ or GO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FireChip {
    pub event: String,
    /// what the tag says: only what the row's name does not (`ch3` when the row is named after `ch3.dialog`, nothing
    /// when it is named after a bare event), else the whole event (`ch3 · yellowmouse-enters`)
    pub tag: String,
    /// the row is named after its event
    pub named: bool,
    pub auto: bool,
    pub moment: Moment,
    /// a suggestion of this row stands: it waits for GO
    pub waiting: bool,
}

/// What a row's fire tag says of `event` beside the row's name `row`, and whether the row is named after it: an event's
/// part before its first `.` is where it comes from (the chapter: `ch3`), the rest its name.
pub fn fire_tag(event: &str, row: &str) -> (String, bool) {
    let (scope, name) = match event.split_once('.') {
        Some((s, n)) => (Some(s), n),
        None => (None, event),
    };
    match scope {
        _ if name == row => (scope.unwrap_or_default().to_string(), true),
        Some(s) => (format!("{s} · {name}"), false),
        None => (event.to_string(), false),
    }
}

/// the chip of `fire` on the row `row` (none: manual, or no event)
pub fn fire_chip(fire: &Fire, row: &str, waiting: bool) -> Option<FireChip> {
    let auto = match fire.how {
        How::Auto => true,
        How::Suggest => false,
        How::Manual => return None,
    };
    let first = fire.event.first()?;
    let (tag, named) = fire_tag(first, row);
    // several events: how many more first (a narrow head cuts the end), then the first
    let tag = match fire.event.len() {
        1 => tag,
        n if tag.is_empty() => format!("+{}", n - 1),
        n => format!("+{} {tag}", n - 1),
    };
    Some(FireChip { event: fire.event.words(), tag, named, auto, moment: fire.moment, waiting: waiting && !auto })
}

/// what a kept cell says: what an earlier row started there carries on
pub const CARRIES_ON: &str = "↑ carries on";

/// what a waiting suggestion says beside its chip
pub const WAITS: &str = "waits for GO";

/// the inspector's line that says what fires a row (#539: pressed, it opens the event picker)
pub const FIRES_ON: &str = "fires on";
/// the label of a row address's line under "this row sends", in the row's order (#553: pressed, its filter's chips show)
pub const ADDRESS: &str = "address";

/// `event: roar`, or `by hand only` for a row no event fires
pub fn fires_on(fire: &Fire) -> String {
    match fire.event.is_empty() {
        true => crate::story::BY_HAND.to_string(),
        false => format!("event: {}", fire.event.words()),
    }
}

fn origin_text(o: Origin) -> &'static str {
    match o {
        Origin::Auto => "auto",
        Origin::Hand => "▶",
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
        Logged::Ignored { row, event } => format!("{event}: {row} is manual, ignored"),
        Logged::NoRow { event } => format!("{event}: no row listens"),
        Logged::Looped { row } => format!("{row}: fired already in this pass – the loop stops"),
        Logged::NoId { kind } if kind.is_empty() => "a story event without an id: dropped – no row can listen to it".into(),
        Logged::NoId { kind } => format!("a {kind} without an id: dropped – no row can listen to it"),
        Logged::CellLaunched { row, column, off_row: true } => format!("off-row: {column} > {row}"),
        Logged::CellLaunched { row, column, off_row: false } => format!("{column} > {row}"),
        Logged::Stopped { column } => format!("{column} stopped"),
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
        Logged::VoteOpened { vote, question } if question.is_empty() => format!("vote {vote} opened"),
        Logged::VoteOpened { vote, question } => format!("vote {vote} opened: {question}"),
        Logged::VoteDecided { vote, option, how, event } => {
            let how = match how.as_str() {
                "override" => "by override",
                "fallback" => "by the fallback",
                _ => "by the rule",
            };
            let event = event.as_ref().map(|e| format!(" → {e}")).unwrap_or_default();
            format!("vote {vote}: {option} ({how}){event}")
        }
        Logged::VoteNotOpened { row, question, why } => format!("row {row}: question {question} not opened – {why}"),
        Logged::Panicked => "PANIC – every deck below black and silent".to_string(),
        Logged::Resumed => "resumed – the decks back to the current cells".to_string(),
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
        name: o.name().to_string(),
        filter: (o.kind == OutletKind::Stage).then(|| crate::roles::filter_words(o.admits.as_ref())),
        group: o.group.clone().filter(|g| !g.trim().is_empty()),
        stops: false,
    });
    let seats = show.seats.iter().map(|s| Column { id: s.id.clone(), seat: true, detail: format!("sub · {}", word_text(s.word)), name: s.id.clone(), filter: None, group: None, stops: true });
    outlets.chain(seats).collect()
}

/// the heads over runs of adjacent columns of one group
fn groups(columns: &[Column]) -> Vec<GroupView> {
    let mut out: Vec<GroupView> = vec![];
    for (i, c) in columns.iter().enumerate() {
        let Some(name) = &c.group else { continue };
        match out.last_mut() {
            Some(g) if g.name == *name && g.first + g.span == i => g.span += 1,
            _ => out.push(GroupView { name: name.clone(), first: i, span: 1 }),
        }
    }
    out
}

/// where the bar stands on the clock at `ms` (0–1)
fn bar_progress(sub: &Subtree, ms: u64) -> Option<f32> {
    let clock = sub.clock_at(ms)?;
    let per_bar = f64::from(clock.per_bar.max(1));
    Some((clock.beat.rem_euclid(per_bar) / per_bar) as f32)
}

/// what `row`'s cell on a column does against what is there at `ms` (none: no time, nothing waits)
fn state_of(row: &str, stage: Option<&OnStage>, ms: Option<u64>) -> State {
    let Some(s) = stage else { return State::Idle };
    let queued = ms.is_some_and(|ms| s.queued(ms));
    match (s.row == row, queued) {
        (true, true) => State::Queued,
        (true, false) => State::Playing,
        (false, true) if s.before.as_deref() == Some(row) => State::Playing,
        _ => State::Idle,
    }
}

/// a cell's state against what plays on its column now
fn cell_view(row: &str, cell: Option<&Cell>, stage: Option<&OnStage>) -> CellView {
    let Some(cell) = cell.filter(|c| !matches!(c, Cell::Keep)) else {
        // a kept cell plays on what an earlier row started there: the arrow says so
        let lit = stage.filter(|s| !matches!(s.playing, Playing::Black));
        return CellView { text: CARRIES_ON.into(), keep: true, state: state_of(row, lit, None), ..CellView::default() };
    };
    let mine = stage.filter(|s| s.row == row);
    let problem = match (cell, stage.map(|s| &s.playing)) {
        (Cell::Values { piece, .. }, Some(Playing::Piece { piece: there, .. })) if there != piece => Some("wrong piece".to_string()),
        (Cell::Values { .. }, None | Some(Playing::Black)) => Some("nothing plays".to_string()),
        _ => None,
    };
    CellView { text: cell_text(cell), keep: false, empty: matches!(cell, Cell::Black), on: mine.is_some(), off_row: mine.is_some_and(|s| s.off_row), problem, ask: None, state: state_of(row, stage, None), progress: None }
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
    matrix_of(sub, None)
}

/// The same at relay time `ms`: a cell launched for its moment is queued until then, the one it replaces plays on, and
/// what plays shows where the bar stands.
pub fn matrix_at(sub: &Subtree, ms: u64) -> Matrix {
    matrix_of(sub, Some(ms))
}

fn matrix_of(sub: &Subtree, ms: Option<u64>) -> Matrix {
    let c = sub.conductor();
    let show = c.show();
    let mut columns = columns(show);
    for col in columns.iter_mut().filter(|col| !col.seat) {
        col.stops = c.on_stage(&col.id).is_some_and(|s| !matches!(s.playing, Playing::Black) || (ms.is_some_and(|ms| s.queued(ms)) && s.before.is_some()));
    }
    let progress = ms.and_then(|ms| bar_progress(sub, ms));
    // #551: the question whose vote is open now – its cells are what is on (a row asks it once: validate)
    let open = sub.vote().open_now().and_then(|v| sub.vote().asks(v));
    let rows = show
        .rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let cells: Vec<CellView> = columns
                .iter()
                .zip(ask_views(show, r, &columns))
                .map(|(col, ask)| {
                    let stage = c.on_stage(&col.id);
                    let view = cell_view(&r.id, r.cells.get(&col.id), stage);
                    let asked = ask.as_ref().is_some_and(|a| open == Some(a.question.as_str()));
                    let state = state_of(&r.id, stage, ms);
                    CellView { on: view.on || asked, ask, state, progress: progress.filter(|_| state == State::Playing && !view.keep), ..view }
                })
                .collect();
            let current = c.current() == Some(r.id.as_str());
            let state = match (current, cells.iter().any(|x| x.state == State::Queued)) {
                (true, true) => State::Queued,
                (true, false) => State::Playing,
                _ => State::Idle,
            };
            let suggested = c.suggestion().is_some_and(|s| s.row == r.id);
            RowView { id: r.id.clone(), number: i + 1, fires: fires(&r.fire), chip: fire_chip(&r.fire, &r.id, suggested), moment: moment_text(r.fire.moment).into(), state, current, suggested, cells }
        })
        .collect();
    Matrix { groups: groups(&columns), columns, rows, suggestion: c.suggestion().map(|s| suggested(show, s)), go: go_text(show, c.current(), c.suggestion()) }
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
            chip: fire_chip(&r.fire, &r.id, false),
            moment: moment_text(r.fire.moment).into(),
            state: State::Idle,
            current: false,
            suggested: false,
            cells: columns.iter().zip(ask_views(show, r, &columns)).map(|(col, ask)| CellView { ask, problem: None, ..cell_view(&r.id, r.cells.get(&col.id), None) }).collect(),
        })
        .collect();
    Matrix { groups: groups(&columns), columns, rows, suggestion: None, go: go_text(show, None, None) }
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
    if fire.event.is_empty() {
        return match fire.moment {
            Moment::Now => "manual".into(),
            m => format!("manual · {}", moment_text(m)),
        };
    };
    let mut s = format!("{} · {}", fire.event.words(), how_text(fire.how));
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
        Cell::Ask { question } => format!("asks {question}, the piece plays on"),
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
    // a values cell that meets another piece on the rows' path: a warning, it is skipped at the show
    for w in crate::values::warnings(show) {
        let points = match (row(&w.row), cols.iter().position(|c| c.id == w.column)) {
            (Some(r), Some(c)) => Points::Cell(r, c),
            _ => Points::Show,
        };
        out.push(ProblemView { text: w.text, error: false, points });
    }
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
        (FIRES_ON.to_string(), fires_on(&r.fire)),
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
        sends.push((ADDRESS.to_string(), address_text(a)));
    }
    // #546: the question the row asks, on which columns, to whom
    let cols = columns(show);
    let asks: Vec<(String, String)> = cols
        .iter()
        .zip(ask_views(show, r, &cols))
        .filter_map(|(c, a)| a.map(|a| (c.id.clone(), format!("{}: {} · {}", a.question, a.label, a.filter))))
        .collect();
    let mut out = vec![(format!("row {} · {}", i + 1, r.id), fire), ("cells of this row".to_string(), cells)];
    if !asks.is_empty() {
        out.push(("this row asks".to_string(), asks));
    }
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

    #[test]
    fn a_rows_fire_tag_says_only_what_its_name_does_not() {
        assert_eq!(fire_tag("ch3.dialog", "dialog"), ("ch3".to_string(), true), "named after its event: the chapter");
        assert_eq!(fire_tag("meet", "meet"), (String::new(), true), "a bare event it is named after: nothing but the bolt");
        assert_eq!(fire_tag("ch3.yellowmouse-enters", "yellowmouse enters"), ("ch3 · yellowmouse-enters".to_string(), false), "another name: the whole event");
        assert_eq!(fire_tag("meet", "together"), ("meet".to_string(), false));
        let fire = Fire { event: crate::document::Events::one("ch3.dialog"), how: How::Suggest, hold_ms: 0, moment: Moment::Bar };
        let chip = fire_chip(&fire, "dialog", true).unwrap();
        assert_eq!((chip.tag.as_str(), chip.named, chip.event.as_str(), chip.waiting), ("ch3", true, "ch3.dialog", true));
    }

    const SERMON: &str = r#"{
      "id": "sermon", "version": 3,
      "toons": ["catpope"], "outlets": [{ "id": "left" }, { "id": "right" }, { "id": "lights", "kind": "device" }],
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
        let filters: Vec<Option<&str>> = m.columns.iter().map(|c| c.filter.as_deref()).collect();
        assert_eq!(filters, [Some("guest and up"), Some("guest and up"), None, None], "#478: a role's header says who may take it");
        assert_eq!(m.rows[1].fires, "roar · suggest");
        assert_eq!(m.rows[2].fires, "manual");
        assert_eq!(m.rows[1].cells[0].text, "values · light 0.8");
        assert!(m.rows[1].cells[2].keep && m.rows[1].cells[2].text == CARRIES_ON);
        assert_eq!(m.go, "GO: next row 1 arrival");
        sub.fire("arrival", 1000);
        let m = matrix(&sub);
        assert!(m.rows[0].current && m.rows[0].cells[0].on && !m.rows[0].cells[0].off_row);
        assert_eq!(m.rows[1].cells[1].problem.as_deref(), Some("nothing plays"), "right is black");
        assert_eq!(m.rows[1].cells[0].problem, None, "the same piece");
        sub.launch_cell("exit", "left", 2000);
        let m = matrix(&sub);
        assert!(m.rows[2].cells[0].on && m.rows[2].cells[0].off_row, "a launch off its row is marked");
        assert_eq!(m.go, "GO: next row 2 roar");
    }

    fn clocked() -> Subtree {
        let mut sub = Subtree::new(parse(SERMON).unwrap(), ClockChoice::Parent);
        // 120 bpm, four beats a bar from 0: a bar is 2 s
        sub.lead_clock(protocol::Tempo { bpm: 120.0, per_bar: 4, beat: 0.0, running: true, at: Some(0), domain: None }, 0);
        sub
    }

    #[test]
    fn a_cell_waits_for_its_moment_while_the_one_it_replaces_plays_on_and_the_bar_shows() {
        let mut sub = clocked();
        sub.fire("arrival", 100);
        let m = matrix_at(&sub, 500);
        assert_eq!((m.rows[0].state, m.rows[0].cells[0].state), (State::Playing, State::Playing));
        assert_eq!(m.rows[0].cells[0].progress, Some(0.25), "beat 1 of 4");
        // roar falls on the next bar (2 s)
        sub.fire("roar", 1000);
        let m = matrix_at(&sub, 1500);
        assert_eq!((m.rows[1].state, m.rows[1].moment.as_str()), (State::Queued, "next bar"));
        assert_eq!(m.rows[1].cells[0].state, State::Queued);
        assert_eq!(m.rows[0].cells[0].state, State::Playing, "arrival plays until the bar");
        assert_eq!(m.rows[1].cells[0].progress, None, "nothing to show while it waits");
        let m = matrix_at(&sub, 2500);
        assert_eq!((m.rows[1].cells[0].state, m.rows[0].cells[0].state), (State::Playing, State::Idle));
        assert_eq!(m.rows[0].state, State::Idle);
        // the device column keeps (↑) and nothing plays there: the arrow stays quiet
        assert!(m.rows[1].cells[2].keep && m.rows[1].cells[2].state == State::Idle);
    }

    #[test]
    fn a_kept_cell_is_the_arrow_lit_while_what_an_earlier_row_started_plays_on() {
        let doc = r#"{ "id": "k", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }, { "id": "right" }],
          "rows": [{ "id": "a", "cells": { "left": { "mode": "play", "piece": "scene:nave" }, "right": { "mode": "black" } } }, { "id": "b", "cells": { "right": { "mode": "play", "piece": "scene:vigil" } } }] }"#;
        let mut sub = Subtree::new(parse(doc).unwrap(), ClockChoice::Parent);
        sub.fire("a", 0);
        sub.fire("b", 10);
        let m = matrix_at(&sub, 20);
        let left = &m.rows[1].cells[0];
        assert!(left.keep && left.text == CARRIES_ON && left.state == State::Playing, "{left:?}");
        assert_eq!(m.rows[0].cells[0].state, State::Idle, "it counts as the row that kept it");
    }

    #[test]
    fn a_columns_stop_waits_for_the_moment_of_the_row_playing_there() {
        let mut sub = clocked();
        sub.fire("roar", 0);
        sub.fire("arrival", 2100);
        let m = matrix_at(&sub, 2200);
        assert!(m.columns[0].stops && m.rows[0].cells[0].state == State::Playing);
        // arrival fires now: its stop is now too
        let out = sub.stop_column("left", 3000);
        let black = out.iter().find(|e| matches!(&e.message, protocol::Message::Cell(c) if c.outlet == "left" && c.cell.mode == protocol::CellMode::Black));
        assert_eq!(black.map(|e| e.at), Some(None));
        let m = matrix_at(&sub, 3100);
        assert!(!m.columns[0].stops && m.rows[0].cells[0].state == State::Idle);
        assert_eq!(log(sub.log(), 1)[0].1, "left stopped");
        // roar's left waits for a bar (4 s): so does its stop
        sub.fire("arrival", 3200);
        sub.fire("roar", 3300);
        let out = sub.stop_column("left", 4200);
        let at = out.iter().find_map(|e| matches!(&e.message, protocol::Message::Cell(c) if c.outlet == "left").then_some(e.at)).flatten();
        assert_eq!(at, Some(6000), "the next bar after 4.2 s");
        assert_eq!(matrix_at(&sub, 5000).rows[1].cells[0].state, State::Playing, "roar plays on until then");
        assert_eq!(matrix_at(&sub, 6100).rows[1].cells[0].state, State::Idle);
        // a seat's stop leaves the child free
        let out = sub.stop_column("foyer", 6200);
        assert!(out.iter().any(|e| matches!(&e.message, protocol::Message::Seat(s) if s.seat == "foyer" && s.says == protocol::Says::Free)));
    }

    #[test]
    fn columns_of_a_group_stand_under_one_head() {
        let doc = r#"{ "id": "g", "version": 1, "rows": [], "toons": ["catpope"], "outlets": [{ "id": "left", "group": "stages" }, { "id": "right", "group": "stages" },
          { "id": "lights", "kind": "device", "group": "light & sound" }, { "id": "wall" }] }"#;
        let show = parse(doc).unwrap();
        let m = plan(&show);
        assert_eq!(m.groups, [GroupView { name: "stages".into(), first: 0, span: 2 }, GroupView { name: "light & sound".into(), first: 2, span: 1 }]);
        assert_eq!(m.columns[3].group, None);
        let split = doc.replace(r#"{ "id": "wall" }"#, r#"{ "id": "wall", "group": "stages" }"#);
        let problems: Vec<String> = crate::validate(&crate::document::open(&split).unwrap()).iter().map(ToString::to_string).collect();
        assert_eq!(problems, ["show: outlet 'wall': the group 'stages' is split, its columns stand apart"]);
        let empty = doc.replace(r#""group": "light & sound""#, r#""group": " ""#);
        assert!(crate::validate(&crate::document::open(&empty).unwrap()).iter().any(|p| p.to_string().contains("an empty group")));
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

    /// LIVE-CAMERAS.md §5a: a row's `scene:<id> > camera:<shot>` switches a phone onto that camera – the relay picks the
    /// phone by the row's filter; it never goes down to the decks. A camera word on no scene is refused.
    #[test]
    fn a_rows_camera_word_forces_a_phone_onto_that_camera() {
        let mut show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        let row = show.rows.iter().position(|r| r.id == "yellowmouse-enters").unwrap();
        let left = protocol::Filter { group: Some("left".into()), ..Default::default() };
        show.rows[row].addresses = vec![crate::document::Addressed { address: "scene:duet".into(), action: "camera:close".into(), filter: Some(left.clone()) }];
        assert!(crate::validate::validate(&show).is_empty(), "{:?}", crate::validate::validate(&show));
        let mut sub = Subtree::new(show.clone(), ClockChoice::Parent);
        let out = sub.fire("yellowmouse-enters", 1000);
        let forced: Vec<&protocol::CameraForce> = out.iter().filter_map(|e| if let protocol::Message::CameraForce(f) = &e.message { Some(f) } else { None }).collect();
        assert_eq!(forced, [&protocol::CameraForce { client: String::new(), camera: "scene:duet > shot:close".into(), r#where: Some(left) }]);
        assert!(!out.iter().any(|e| matches!(e.message, protocol::Message::Address(_))), "not a deck's word");
        show.rows[row].addresses[0].address = "character:yellowmouse".into();
        assert!(crate::validate::validate(&show).iter().any(|p| p.to_string().contains("a phone takes a scene's camera")));
        // the cameras its scenes offer go to the session once, and again only when they change
        let cams = vec![protocol::LiveCamera { id: "scene:duet".into(), label: None, r#for: None, release_ms: None, holder: None, nick: None, away: None }];
        assert_eq!(sub.offer_cameras(cams.clone()).len(), 1);
        assert!(sub.offer_cameras(cams).is_empty());
    }

    #[test]
    fn the_cue_sheet_says_each_row_in_order_and_what_changes() {
        let show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        let cues = cue_sheet(&show);
        assert_eq!(cues.len(), 4);
        assert_eq!(cues[0].trigger, "manual");
        assert_eq!(cues[0].does[0], ("left".to_string(), "plays scene:nave · light 0.2".to_string()));
        assert_eq!(cues[0].does[1], ("right".to_string(), "shows what left shows".to_string()));
        assert_eq!(cues[0].sends, ["lamps · dmx"]);
        assert_eq!(cues[1].does, [("left".to_string(), "glides scene:nave: dim 0.5".to_string()), ("lamps".to_string(), "look warm · fade 0.8 s".to_string())], "a kept column is not listed");
        let sends = &inspector(&show, "dialog").into_iter().find(|(h, _)| h == "this row sends").unwrap().1;
        assert_eq!(sends, &[("lamps".to_string(), "look warm · strobe @1 · panel @10".to_string())]);
        assert_eq!(cues[2].trigger, "ch3.yellowmouse-enters · suggest · hold 1.5 s · next bar");
        assert_eq!(cues[2].sends, ["character:yellowmouse > dance:samba where group left", "location:* > close_doors"]);
        assert_eq!(cues[3].trigger, "ch3.end · manual · now");
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
