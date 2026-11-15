//! A vote as cells of the launcher, in the Show workspace (#551; SHOW.md §15 points 5, 6; owner 2026-11-14 [live]): a press
//! selects a cell, shift and a press add a column of the same row (a set within one row); the right click on the selection
//! opens deck-kit's typeahead – the Build workspace's context menu, the one element (owner: "the context typeahead we used
//! for the address") – with "New question …", the story's questions, "Add to question <q>" and "Leave the question", found
//! as typed. The question of the selected cell shows in the cell inspector: #533's form without groups for a typed one, the
//! story question's snapshot read-only with its warning. What is offered, what a pick and a field do and every word are
//! deck-show's (`question.rs`: `offers`, `inspect`, the `rows::Edit`s checked by validate); this file opens the typeahead,
//! lays the form out and hands the presses and the typed text in (RULES A1, A2).
use super::roles::{line, Typing};
use super::rows::field;
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::typeahead::{Item, Typeahead};
use deck_show::question::{self, Offers, QuestionView, Set};
use deck_show::rows::Edit;
use deck_show::view;
use deck_ui::vote::{self as words, RULES};
use std::sync::Arc;
use winit::keyboard::{Key, NamedKey};

/// the language of the labels (the editor's own words are English, main.rs)
const LOCALE: &str = "en";

/// The right click's typeahead on the selected cells, and what its lines do.
pub struct Context {
    ta: Typeahead,
    offers: Arc<Offers>,
}

/// A typed field of the question's form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskField {
    Text,
    Options,
    N,
    Seconds,
}

/// A press on the question's form in the cell inspector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AskHit {
    Field(AskField),
    Rule(usize),
    Fallback(usize),
    Counts,
}

impl Host {
    /// the column whose cell the cell inspector shows: the one selected last
    pub fn cell(&self) -> Option<&String> {
        self.cells.last()
    }

    /// the row inspected and its selected columns that the document shown has, in the matrix's order
    pub(super) fn selection(&self) -> (Option<String>, Vec<String>) {
        let show = self.shown();
        let row = show.rows.get(self.row).map(|r| r.id.clone());
        let columns = view::plan(show).columns.into_iter().map(|c| c.id).filter(|c| self.cells.contains(c)).collect();
        (row, columns)
    }

    /// A press on a cell: it alone is selected; with shift (`also`) a column of the same row joins the selection, or
    /// leaves it – a cell of another row starts over there.
    pub(super) fn select_cell(&mut self, r: usize, c: usize, also: bool) {
        let Some(column) = view::plan(self.shown()).columns.get(c).map(|c| c.id.clone()) else { return };
        if r != self.row {
            self.address = None;
        }
        let same = r == self.row && self.role.is_none();
        self.row = r;
        self.role = None;
        match (also && same, self.cells.iter().position(|x| *x == column)) {
            (true, Some(at)) if self.cells.len() > 1 => {
                self.cells.remove(at);
            }
            (true, Some(_)) => {}
            (true, None) => self.cells.push(column),
            (false, _) => self.cells = vec![column],
        }
        // a story question's warning needs the toon's list: asked once, when such a cell is first looked at
        if self.asked_question().is_some_and(|q| q.story.is_some()) {
            self.story_load();
        }
    }

    /// the question the inspected cell asks, as the cell inspector shows it (deck-show `question::inspect`)
    pub(super) fn asked_question(&self) -> Option<QuestionView> {
        let (row, column) = (self.shown().rows.get(self.row)?, self.cell()?);
        let toon = self.story_toon();
        let listed = match (&toon, &self.story) {
            (Some(toon), Some(Ok(s))) => Some((toon.as_str(), s.choices.as_slice())),
            _ => None,
        };
        question::inspect(self.shown(), &row.id, column, listed, LOCALE)
    }

    /// The right click at `(x, y)`: on a cell of the rows band (one outside the selection is selected alone first) the
    /// typeahead opens there with what deck-show offers for the selected cells; anywhere else an open one closes.
    pub fn context(&mut self, x: f32, y: f32) -> bool {
        self.context = None;
        let Some(Hit::Cell(r, c)) = self.hit(x, y) else { return false };
        let column = view::plan(self.shown()).columns.get(c).map(|c| c.id.clone());
        if r != self.row || self.role.is_some() || !column.is_some_and(|c| self.cells.contains(&c)) {
            self.select_cell(r, c, false);
        }
        let (Some(row), columns) = self.selection() else { return false };
        self.story_load();
        let toon = self.story_toon().unwrap_or_default();
        let listed = match &self.story {
            Some(Ok(s)) => Ok((toon.as_str(), s.choices.as_slice())),
            Some(Err(why)) => Err(why.as_str()),
            None => Err(""),
        };
        let offers = Arc::new(question::offers(self.shown(), &row, &columns, listed, LOCALE));
        let items = offers.list.iter().enumerate().map(|(id, o)| Item { id, label: o.label.clone(), detail: o.detail.clone() }).collect();
        let found = offers.clone();
        self.typing = None;
        self.context = Some(Context { ta: Typeahead::found_by(items, x, y, move |q| found.find(q)), offers });
        true
    }

    /// the pointer over the open typeahead highlights its line
    pub fn pointer(&mut self, x: f32, y: f32) {
        self.cursor = (x, y);
        self.knob_moved(x);
        if let Some(c) = self.context.as_mut() {
            c.ta.hover(x, y);
        }
    }

    /// A left press while the typeahead is open: its line is picked, a press beside it closes it. True = taken.
    pub fn context_press(&mut self, x: f32, y: f32) -> bool {
        let Some(c) = self.context.as_ref() else { return false };
        match c.ta.id_at(x, y) {
            Some(id) => self.context_pick(id),
            None => self.context = None,
        }
        true
    }

    /// A key while the typeahead is open: true = taken. Typed text narrows it, the arrows move, Enter picks, Escape closes.
    pub(super) fn context_key(&mut self, key: &Key) -> bool {
        let Some(c) = self.context.as_mut() else { return false };
        match key {
            Key::Named(NamedKey::Escape) => self.context = None,
            Key::Named(NamedKey::Enter) => {
                if let Some(id) = c.ta.chosen() {
                    self.context_pick(id);
                }
            }
            Key::Named(NamedKey::ArrowUp) => c.ta.step(-1),
            Key::Named(NamedKey::ArrowDown) => c.ta.step(1),
            Key::Named(NamedKey::ArrowLeft) => c.ta.left(),
            Key::Named(NamedKey::ArrowRight) => c.ta.right(),
            Key::Named(NamedKey::Backspace) => c.ta.backspace(),
            Key::Named(NamedKey::Delete) => c.ta.delete(),
            Key::Named(NamedKey::Space) => c.ta.insert(" "),
            Key::Character(t) => c.ta.insert(t),
            _ => {}
        }
        true
    }

    /// the lines the open typeahead shows now (their labels), for a check
    #[cfg(test)]
    pub fn context_lines(&self) -> Vec<String> {
        let Some(c) = self.context.as_ref() else { return vec![] };
        c.offers.list.iter().enumerate().filter(|(i, _)| c.ta.shows(*i)).map(|(_, o)| o.label.clone()).collect()
    }

    /// A line picked: deck-show makes the edit (checked there; a line that cannot be picked says why). A new question's
    /// words are typed next: its field opens.
    fn context_pick(&mut self, id: usize) {
        let Some(c) = self.context.take() else { return };
        let new = c.offers.list.get(id).is_some_and(|o| o.act == Some(question::Act::New));
        match c.offers.pick(self.shown(), id) {
            Ok(done) => {
                self.status = done.said;
                self.changed(done.show);
                if let Some(q) = self.asked_question().filter(|_| new) {
                    self.typing = Some((Typing::Ask(q.id, AskField::Text), Field::with("")));
                }
            }
            Err(why) => self.status = why,
        }
    }

    pub(super) fn ask_press(&mut self, hit: AskHit) {
        let Some(q) = self.asked_question() else { return };
        let set = match hit {
            AskHit::Field(f) => {
                let now = match f {
                    AskField::Text => q.form.question.clone(),
                    AskField::Options => q.form.options.clone(),
                    AskField::N => q.form.n.to_string(),
                    AskField::Seconds => q.form.seconds.to_string(),
                };
                self.typing = Some((Typing::Ask(q.id, f), Field::with(&now)));
                return;
            }
            AskHit::Rule(i) => Set::Rule(RULES.get(i).copied().unwrap_or_default()),
            AskHit::Fallback(i) => Set::Fallback(i),
            AskHit::Counts => Set::Counts,
        };
        self.edit_rows(&Edit::Question { question: q.id, set });
    }

    /// a typed field of the question's form applied (Enter): deck-show reads and checks it
    pub(super) fn ask_typed(&mut self, question: String, field: AskField, text: &str) {
        let set = match field {
            AskField::Text => Set::Text(text.to_string()),
            AskField::Options => Set::Options(text.to_string()),
            AskField::N => Set::N(text.to_string()),
            AskField::Seconds => Set::Seconds(text.to_string()),
        };
        self.edit_rows(&Edit::Question { question, set });
    }

    /// The wheel over the inspector scrolls it (its buttons keep their own strip). True = taken.
    pub fn wheel(&mut self, x: f32, y: f32, steps: f32) -> bool {
        if let Some(Hit::Knob(super::values::KnobHit::Turn(i))) = self.hit(x, y) {
            self.knob_wheel(i, steps);
            return true;
        }
        if !self.side.has(x, y) {
            return false;
        }
        self.side_scroll = (self.side_scroll - steps * 40.0).clamp(0.0, self.side_over);
        true
    }

    /// A check run (`--question [QUERY][,pick]`): the right click on the selected cells – the typeahead opens on the
    /// selection's first cell in the rows band, `query` is typed, the first line found is picked if asked.
    pub(super) fn script_question(&mut self, query: &str, pick: bool) {
        let plan = view::plan(self.shown());
        let Some(c) = plan.columns.iter().position(|c| self.cells.contains(&c.id)) else { return };
        let Some((r, _)) = self.hits.iter().rev().find(|(_, h)| *h == Hit::Cell(self.row, c)).copied() else { return };
        if !self.context(r.x + r.w * 0.5, r.y + r.h * 0.5) {
            return;
        }
        for ch in query.chars() {
            self.context_key(&Key::Character(ch.to_string().into()));
        }
        if pick {
            self.context_key(&Key::Named(NamedKey::Enter));
            // a check run types nothing more: the new question's field closes
            self.typing = None;
        }
    }
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// `text` on lines of at most `w` at `px`, broken at spaces
fn wrap(d: &DrawList, text: &str, px: f32, w: f32) -> Vec<String> {
    let cols = ((w / d.advance(px)) as usize).max(8);
    let mut lines: Vec<String> = vec![];
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(l) if l.chars().count() + 1 + word.chars().count() <= cols => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// small quiet text over the lines it needs; the height used
fn note(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), text: &str, color: [f32; 4]) -> f32 {
    let small = t.size.small;
    let lines = wrap(d, text, small, w);
    for (i, l) in lines.iter().enumerate() {
        d.text(x, y + i as f32 * small * 1.5, small, l, color);
    }
    lines.len() as f32 * small * 1.5 + 4.0
}

/// a small label, then chips on the next line; their places and the height used
fn chips(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), label: &str, names: &[String], on: &[bool]) -> (Vec<Rect>, f32) {
    let small = t.size.small;
    d.text(x, y + 2.0, small, label, t.color.muted);
    let words: Vec<&str> = names.iter().map(String::as_str).collect();
    let (r, used) = controls::chips(d, t, (x, y + small * 1.7, w), &words, on);
    (r, small * 1.7 + used + t.size.pad * 0.5)
}

/// The question of the selected cell in the cell inspector: #533's form without groups (a typed question's words and
/// options are fields, the story's a read-only snapshot with its warning), how its vote runs. The height used.
pub fn question_side(d: &mut DrawList, t: &Tokens, host: &mut Host, (x, y, w): (f32, f32, f32), q: &QuestionView) -> f32 {
    let (pad, row) = (t.size.pad, t.size.tree_row);
    let top = y;
    let mut y = y + controls::section(d, t, x, y, w, &q.title.to_uppercase(), false);
    let typing = |host: &Host, f: AskField| host.typing.as_ref().filter(|(what, _)| *what == Typing::Ask(q.id.clone(), f)).map(|(_, f)| f.text.clone());
    let mut hits: Vec<(Rect, AskHit)> = vec![];
    let f = &q.form;
    match &q.story {
        Some(source) => {
            // the story's: its options as they were when picked, read-only
            // the snapshot whole (wrapped): the chapter, the event and the version it was picked at
            y += note(d, t, (x, y + 2.0, w), &format!("story · {source}"), t.color.text);
            line(d, t, (x, y, w), words::OPTIONS, &f.options);
            y += row;
            if let Some(warning) = &q.warning {
                y += note(d, t, (x, y + 2.0, w), warning, t.color.warn);
            }
        }
        None => {
            let (text, empty) = (typing(host, AskField::Text), f.question.is_empty());
            let r = field(d, t, (x, y, w), words::QUESTION, if empty { words::NO_QUESTION } else { &f.question }, text.as_deref());
            hits.push((r, AskHit::Field(AskField::Text)));
            y += row;
            let text = typing(host, AskField::Options);
            let r = field(d, t, (x, y, w), words::OPTIONS, if f.options.is_empty() { words::OPTIONS_HINT } else { &f.options }, text.as_deref());
            hits.push((r, AskHit::Field(AskField::Options)));
            y += row;
        }
    }
    y += pad * 0.5;
    let rules: Vec<String> = RULES.iter().map(|r| r.name().to_string()).collect();
    let lit: Vec<bool> = RULES.iter().map(|r| *r == f.rule).collect();
    let (rects, used) = chips(d, t, (x, y, w), words::RULE, &rules, &lit);
    hits.extend(rects.into_iter().enumerate().map(|(i, r)| (r, AskHit::Rule(i))));
    y += used;
    y += note(d, t, (x, y, w), f.rule.help(), t.color.muted);
    if f.rule == words::Rule::First {
        let text = typing(host, AskField::N);
        hits.push((field(d, t, (x, y, w), words::N, &f.n.to_string(), text.as_deref()), AskHit::Field(AskField::N)));
        y += row;
    }
    let text = typing(host, AskField::Seconds);
    hits.push((field(d, t, (x, y, w), words::SECONDS, &f.seconds.to_string(), text.as_deref()), AskHit::Field(AskField::Seconds)));
    y += row + pad * 0.5;
    let options = f.option_list();
    let lit: Vec<bool> = (0..options.len()).map(|i| i == f.fallback).collect();
    let (rects, used) = chips(d, t, (x, y, w), words::FALLBACK, &options, &lit);
    hits.extend(rects.into_iter().enumerate().map(|(i, r)| (r, AskHit::Fallback(i))));
    y += used;
    let counts = Rect { x, y, w, h: row - 4.0 };
    controls::toggle(d, t, counts, words::counts_word(f.show), false);
    hits.push((counts, AskHit::Counts));
    y += row;
    // how it runs, or what is wrong with it
    d.text(x, y + 2.0, t.size.small, &fit(d, &q.line, t.size.small, w), if q.wrong { t.color.warn } else { t.color.muted });
    y += t.size.small * 1.8;
    host.hits.extend(hits.into_iter().map(|(r, h)| (r, Hit::Ask(h))));
    y - top
}

/// the open typeahead, over everything else in the workspace
pub fn draw_context(d: &mut DrawList, t: &Tokens, host: &mut Host, limit: (f32, f32)) {
    if let Some(c) = host.context.as_mut() {
        c.ta.draw(d, t, limit);
    }
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{fake_api, host_with, press, typed, H, W};
    use super::super::{api, author, link, Hit, Host};
    use super::*;
    use deck_show::document::Cell;

    const VOTE: &str = include_str!("../../../show/fixtures/vote-cell.show.json");
    const EVENTS: &str = include_str!("../../../show/fixtures/story-events.json");

    /// one frame of the Show workspace: its texts
    fn frame(host: &mut Host) -> Vec<String> {
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        host.hits.clear();
        author::draw(&mut d, &t, host, W, H, (0.0, 0.0));
        d.texts().into_iter().map(str::to_string).collect()
    }

    /// the middle of the place a hit has this frame
    fn at(host: &mut Host, want: Hit) -> (f32, f32) {
        frame(host);
        let r = host.hits.iter().rev().find(|(_, h)| *h == want).map(|(r, _)| *r).unwrap_or_else(|| panic!("no place {want:?}"));
        (r.x + r.w * 0.5, r.y + r.h * 0.5)
    }

    /// a cell pressed with shift held, as main.rs hands it in
    fn shift(host: &mut Host, r: usize, c: usize) {
        let (x, y) = at(host, Hit::Cell(r, c));
        let h = host.hit(x, y).unwrap();
        host.press(h.with_shift());
    }

    /// the right click on a cell, as main.rs hands it in
    fn right_click(host: &mut Host, r: usize, c: usize) {
        let (x, y) = at(host, Hit::Cell(r, c));
        assert!(host.context(x, y), "the typeahead opens on a cell");
    }

    fn column(host: &Host, id: &str) -> usize {
        view::plan(host.shown()).columns.iter().position(|c| c.id == id).unwrap()
    }

    /// step 1: a press selects a cell alone; shift adds a column of the same row (also one not adjacent) or takes it out
    /// again; a cell of another row starts over there; each selected cell is outlined
    #[test]
    fn shift_adds_a_column_of_the_same_row_to_the_selection() {
        let mut host = host_with(VOTE);
        let (left, middle, right) = (column(&host, "left"), column(&host, "middle"), column(&host, "right"));
        press(&mut host, false, |h| *h == Hit::Cell(0, left));
        shift(&mut host, 0, right);
        assert_eq!(host.selection(), (Some("opening".into()), vec!["left".to_string(), "right".to_string()]));
        assert_eq!(host.cell().map(String::as_str), Some("right"), "the inspector shows the cell added last");
        shift(&mut host, 0, middle);
        shift(&mut host, 0, left);
        assert_eq!(host.selection().1, ["middle", "right"], "shift on a selected cell takes it out");
        // visible: an outline per selected cell (the accent at 1.5 px)
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        author::draw(&mut d, &t, &mut host, W, H, (0.0, 0.0));
        let outlined = d.prims().iter().filter(|p| matches!(p, deck_kit::base::draw::Prim::Outline { thick, .. } if *thick == 1.5)).count();
        assert!(outlined >= 2, "{outlined} outlines");
        shift(&mut host, 1, left);
        assert_eq!(host.selection(), (Some("crossroads".into()), vec!["left".to_string()]), "another row: it starts over");
        press(&mut host, false, |h| *h == Hit::Cell(1, middle));
        assert_eq!(host.selection().1, ["middle"], "a press without shift: the cell alone");
    }

    /// #551's done-when, the editor's half: two cells that are not adjacent become one question by the right click, its
    /// words typed in the cell inspector; saved and loaded through /api/shows; Perform's ▶ on it opens the vote through
    /// the relay link and the Vote sheet's panel shows it
    #[test]
    fn two_cells_apart_become_one_question_by_right_click_saved_loaded_and_opened_from_perform() {
        let cache = std::env::temp_dir().join(format!("deck-editor-ask-{}", std::process::id()));
        let base = fake_api();
        let mut host = host_with(VOTE);
        host.api = api::Api::with_cache(base, cache.join("shows"));
        let (left, right) = (column(&host, "left"), column(&host, "right"));
        press(&mut host, false, |h| *h == Hit::Cell(0, left));
        shift(&mut host, 0, right);
        right_click(&mut host, 0, right);
        // no toon: the story's questions say why in one line
        assert_eq!(host.context_lines(), [question::NEW, question::STORY]);
        assert!(frame(&mut host).iter().any(|t| t == question::NEW), "the typeahead is drawn");
        typed(&mut host, "new");
        assert_eq!(host.status, "row 1 opening: left · right ask the new question q2 – device phone");
        assert!(host.context.is_none(), "a pick closes it");
        // the new question's words are typed next, in the cell inspector's form; then its options
        typed(&mut host, "Which door?");
        press(&mut host, false, |h| *h == Hit::Ask(AskHit::Field(AskField::Options)));
        for _ in 0.."Yes, No".len() {
            host.type_key(&Key::Named(NamedKey::Backspace));
        }
        typed(&mut host, "Red, Blue");
        assert_eq!(host.status, "question q2: Which door? · Red, Blue · majority · 30 s · fallback Red");
        let r = host.shown().row("opening").unwrap();
        assert_eq!((r.cells.get("left"), r.cells.get("right")), (Some(&Cell::Ask { question: "q2".into() }), Some(&Cell::Ask { question: "q2".into() })));
        assert!(matches!(r.cells.get("middle"), Some(Cell::Play { .. })), "the column between plays on");
        assert_eq!(r.r#where.get("left"), Some(&question::new_filter()), "a new question starts with device: phone");
        // the cell inspector: the form without groups, the cell's filter with the shared control, no piece field
        let texts = frame(&mut host);
        assert!(texts.iter().any(|t| t == "QUESTION Q2 · LEFT · RIGHT"), "{texts:?}");
        assert!(host.hits.iter().any(|(_, h)| matches!(h, Hit::CellFact(_))), "the cell's filter chips");
        assert!(!host.hits.iter().any(|(_, h)| *h == Hit::CellPiece), "an ask cell has no piece field");
        let made = host.shown().clone();
        assert!(deck_show::validate::validate(&made).is_empty(), "{:?}", deck_show::validate::validate(&made));
        let fixture = deck_show::parse(include_str!("../../../show/fixtures/asked-by-right-click.show.json")).unwrap();
        assert_eq!(made, fixture, "the document headless scenario 43 launches: {}", serde_json::to_string(&made).unwrap());

        press(&mut host, false, |h| *h == Hit::Save);
        assert_eq!(host.status, "saved vote-cell as version 2");
        host.sub = deck_show::Subtree::new(deck_show::parse(r#"{ "id": "vote-cell", "version": 1, "outlets": [{ "id": "left" }], "rows": [] }"#).unwrap(), deck_show::ClockChoice::Parent);
        press(&mut host, false, |h| *h == Hit::Load);
        assert_eq!(host.shown(), &made, "loaded as saved");

        // Perform: the cell's ▶ – the conductor opens the question's vote for both columns, through the relay
        let (l, sent, relay) = link::Link::loopback();
        host.link = Some(l);
        press(&mut host, true, |h| *h == Hit::Launch(0, left));
        let open = sent.try_iter().find(|e| matches!(e.message, protocol::Message::VoteOpen(_))).expect("a vote.open went out");
        let protocol::Message::VoteOpen(v) = &open.message else { unreachable!() };
        assert_eq!((v.groups.clone(), v.options.clone(), v.r#where.clone()), (Some(vec!["left".to_string(), "right".to_string()]), vec!["Red".to_string(), "Blue".to_string()], Some(question::new_filter())));
        relay.send(protocol::Envelope { from: "relay".into(), ..open }).unwrap();
        host.frame();
        let p = host.vote_panel().expect("the panel shows the vote");
        assert_eq!((p.question.as_str(), p.state), ("Which door?", deck_ui::vote::State::Open));
        let _ = std::fs::remove_dir_all(&cache);
    }

    /// a story question picked by the same right click: the snapshot read-only in the inspector, the warning when the
    /// published choice changed; an id-less one listed with its reason
    #[test]
    fn a_story_question_is_picked_by_right_click_and_read_only_in_the_inspector() {
        let mut host = host_with(VOTE);
        host.toon = Some("catpope".into());
        host.story = Some(deck_show::story::read(EVENTS, "catpope"));
        let (left, right) = (column(&host, "left"), column(&host, "right"));
        press(&mut host, false, |h| *h == Hit::Cell(0, left));
        shift(&mut host, 0, right);
        right_click(&mut host, 0, left);
        assert_eq!(host.context_lines(), [question::NEW, "ch3.door", "no id", "ch3.empty"]);
        typed(&mut host, "bleiben");
        assert_eq!(host.status, "row 1 opening: q2 asks catpope ch 3 ch3.door (version 812)");
        let texts = frame(&mut host);
        assert!(texts.join(" ").contains("story · catpope ch 3 · ch3.door · version 812"), "{texts:?}");
        assert!(!host.hits.iter().any(|(_, h)| matches!(h, Hit::Ask(AskHit::Field(AskField::Text | AskField::Options)))), "its words are the chapter's");
        assert!(host.hits.iter().any(|(_, h)| matches!(h, Hit::Ask(AskHit::Rule(_)))), "how its vote runs is not");
        // the published choice changed: the inspector warns in story::changed's words
        if let Some(Ok(s)) = host.story.as_mut() {
            s.choices[0].version = 815;
            s.choices[0].options[1].label.insert("en".into(), "Leave".into());
        }
        assert!(frame(&mut host).join(" ").contains("question 'q2': ch3.door changed since it was picked (version 812 → 815) – pick it again"));
        // the id-less line: a pick says why
        right_click(&mut host, 0, left);
        typed(&mut host, "crypt");
        assert!(host.status.starts_with("no id – the chapter editor gives the question one"), "{}", host.status);
        // Escape closes it without a pick
        right_click(&mut host, 0, left);
        host.type_key(&Key::Named(NamedKey::Escape));
        assert!(host.context.is_none());
    }

    /// one frame of Perform: its texts
    fn perform_frame(host: &mut Host) -> Vec<String> {
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        host.hits.clear();
        super::super::perform::draw(&mut d, &t, host, W, H, (0.0, 0.0));
        d.texts().into_iter().map(str::to_string).collect()
    }

    /// step 4: cells of one question apart are each marked (`q1 · 2 of 2`); adjacent ones are one wide cell, in the rows
    /// band and in Perform's matrix; every column of it launches, and its ▶ opens the vote of the whole question
    #[test]
    fn the_question_is_one_wide_cell_in_show_and_perform_and_its_play_opens_the_vote() {
        let mut host = host_with(VOTE);
        let (left, middle, right) = (column(&host, "left"), column(&host, "middle"), column(&host, "right"));
        let marks = |texts: &[String], prefix: &str| texts.iter().filter(|t| t.starts_with(prefix)).count();
        let (show, perform) = (frame(&mut host), perform_frame(&mut host));
        for texts in [&show, &perform] {
            assert_eq!((marks(texts, "q1 · 1 of 2"), marks(texts, "q1 · 2 of 2")), (1, 1), "two cells apart: {texts:?}");
        }
        // the column between joins: one wide cell over three columns, said once
        press(&mut host, false, |h| *h == Hit::Cell(1, middle));
        right_click(&mut host, 1, middle);
        typed(&mut host, "add");
        assert_eq!(host.status, "row 2 crossroads: middle asks q1 – device phone · member · live");
        let (show, perform) = (frame(&mut host), perform_frame(&mut host));
        for texts in [&show, &perform] {
            assert_eq!((marks(texts, "q1 · Which way?"), marks(texts, "q1 · ")), (1, 1), "one wide cell: {texts:?}");
        }
        let launches: Vec<usize> = host.hits.iter().filter_map(|(_, h)| if let Hit::Launch(1, c) = h { Some(*c) } else { None }).collect();
        assert!([left, middle, right].iter().all(|c| launches.contains(c)), "{launches:?}");
        let (l, sent, _relay) = link::Link::loopback();
        host.link = Some(l);
        press(&mut host, true, |h| *h == Hit::Launch(1, right));
        let groups = sent.try_iter().find_map(|e| if let protocol::Message::VoteOpen(v) = e.message { v.groups } else { None });
        assert_eq!(groups, Some(vec!["left".to_string(), "middle".to_string(), "right".to_string()]), "the whole question votes");
    }

    /// step 5: nothing opens a vote outside a row – the Vote sheet has no form and no Open; a fired row's question opens
    /// it and the sheet keeps the live panel with Hold, Override and Close now
    #[test]
    fn the_vote_sheet_opens_nothing_a_rows_question_does_and_the_panel_stays() {
        let mut host = host_with(VOTE);
        let (l, sent, relay) = link::Link::loopback();
        host.link = Some(l);
        press(&mut host, true, |h| *h == Hit::Sheet(super::super::Sheet::Vote));
        let texts = perform_frame(&mut host);
        assert!(texts.iter().any(|t| t == deck_ui::vote::NONE) && texts.iter().any(|t| t.starts_with("A vote opens from a row's question")), "{texts:?}");
        assert!(!host.hits.iter().any(|(_, h)| matches!(h, Hit::Vote(_))), "no form, no Open: {:?}", host.hits.iter().map(|(_, h)| h).collect::<Vec<_>>());
        assert!(sent.try_iter().all(|e| !matches!(e.message, protocol::Message::VoteOpen(_))));
        // the row crossroads fires: its question's vote opens, the panel shows it
        press(&mut host, true, |h| *h == Hit::Fire(1));
        let open = sent.try_iter().find(|e| matches!(e.message, protocol::Message::VoteOpen(_))).expect("the row's question opened its vote");
        relay.send(protocol::Envelope { from: "relay".into(), ..open }).unwrap();
        host.frame();
        let texts = perform_frame(&mut host);
        assert!(texts.iter().any(|t| t == "Which way?"), "{texts:?}");
        for want in [super::super::vote::VoteHit::Hold, super::super::vote::VoteHit::Override, super::super::vote::VoteHit::Close] {
            assert!(host.hits.iter().any(|(_, h)| *h == Hit::Vote(want)), "{want:?}");
        }
    }

    /// #553's finding: the cell inspector with the question's form is taller than its panel – it scrolls above Load /
    /// Flash / Save, which keep their own strip; nothing it draws under the strip answers a press
    #[test]
    fn the_inspector_scrolls_above_its_buttons() {
        let mut host = host_with(VOTE);
        let left = column(&host, "left");
        press(&mut host, false, |h| *h == Hit::Cell(1, left));
        press(&mut host, false, |h| *h == Hit::Ask(AskHit::Rule(2)));
        assert!(host.status.contains("first to 20"), "{}", host.status);
        frame(&mut host);
        let strip = host.side.y + host.side.h;
        assert!(host.side_over > 0.0, "the inspector is taller than its place");
        let inside = |h: &Hit| !matches!(h, Hit::Save | Hit::Load | Hit::Flash);
        assert!(host.hits.iter().filter(|(r, h)| r.x >= host.side.x && inside(h)).all(|(r, _)| r.y + r.h <= strip + 0.5), "nothing under the strip");
        let counts = |host: &Host| host.hits.iter().any(|(_, h)| *h == Hit::Ask(AskHit::Counts));
        let (cx, cy) = (host.side.x + 10.0, host.side.y + 10.0);
        for _ in 0..40 {
            assert!(host.wheel(cx, cy, -1.0));
        }
        frame(&mut host);
        assert!(host.side_scroll > 0.0 && host.side_scroll <= host.side_over);
        assert!(counts(&host), "scrolled: the form's last toggle answers");
        assert!(!host.wheel(10.0, 10.0, -1.0), "the wheel elsewhere is not the inspector's");
    }
}
