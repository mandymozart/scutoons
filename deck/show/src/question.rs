//! A vote as cells of the launcher (#551; SHOW.md §15 points 5, 6; owner 2026-11-14 [live]: "created by selecting multiple
//! cells in a row and then adding with a right click a new questionary or choose from a possible question from a story";
//! "add to question" for a column that is not adjacent; nothing opens a vote outside the show). Here once: what the right
//! click on the selected cells of a row offers ([`offers`] – a new question, the story's questions, add to a question of
//! the row, leave it – found as typed, each made by `rows::edit` or `story::pick` and checked by validate), what those
//! edits do to the document (the helpers `rows::edit` calls), and the question in the cell inspector ([`inspect`]: #533's
//! form without groups for a typed question, the story question's snapshot with the warning of `story::changed`; a
//! change is [`Set`], an edit too). A new question starts with `device: phone` on (§15 question 13: today's phone poll,
//! one chip widens it). The editor lays out and hands the presses in (RULES A1, A2).
use crate::document::{Cell, Loc, Question, Row, Show};
use crate::rows::{Edit, Edited};
use crate::story::{self, StoryChoice};
use crate::vote::DEFAULT_LOCALE;
use deck_ui::vote::{Form, Rule};
use protocol::{Filter, VoteRule};

/// the right click's lines
pub const NEW: &str = "New question …";
pub const LEAVE: &str = "Leave the question";
/// the line that stands for the story's questions where there is no list (why not is its detail)
pub const STORY: &str = "the story's questions";

/// `Add to question q1`
pub fn add_to(question: &str) -> String {
    format!("Add to question {question}")
}

/// the device a new question's filter names: today's phone poll (SHOW.md §15 question 13)
pub const NEW_DEVICE: &str = "phone";

/// the filter a new question starts with: `device: phone`
pub fn new_filter() -> Filter {
    Filter { device: Some(NEW_DEVICE.to_string()), ..Filter::default() }
}

/// `columns` of row `r` ask `question`: `ask` cells with the one filter of the vote – the filter the row's other columns
/// that ask it carry (validate's rule), else `fresh` (a new question's; none: no filter)
pub(crate) fn join(r: &mut Row, columns: &[String], question: &str, fresh: Option<Filter>) {
    let sibling = r.cells.iter().find(|(k, c)| !columns.contains(k) && matches!(c, Cell::Ask { question: q } if q == question)).map(|(k, _)| k.clone());
    let filter = match sibling {
        Some(k) => r.r#where.get(&k).cloned(),
        None => fresh,
    };
    for c in columns {
        r.cells.insert(c.clone(), Cell::Ask { question: question.to_string() });
        match &filter {
            Some(f) => r.r#where.insert(c.clone(), f.clone()),
            None => r.r#where.remove(c),
        };
    }
}

/// the first free question id: `q1`, `q2` …
pub(crate) fn free_id(show: &Show) -> String {
    (1..).map(|n| format!("q{n}")).find(|id| show.question(id).is_none()).unwrap_or_default()
}

/// "New question …": a typed question with the vote form's defaults (deck-ui `Form::default`) asked by `columns` of row
/// `i`, for phones; its id
pub(crate) fn new(next: &mut Show, i: usize, columns: &[String]) -> Result<String, String> {
    let id = free_id(next);
    next.questions.push(crate::vote::question(&Form::default(), &id)?);
    join(&mut next.rows[i], columns, &id, Some(new_filter()));
    Ok(id)
}

/// `column` of row `r` asks nothing any more: its filter goes with it (the cell itself is the caller's)
pub(crate) fn left(r: &mut Row, column: &str) {
    r.r#where.remove(column);
}

/// "Leave the question": the `columns` of row `i` that ask keep again; which went
pub(crate) fn leave(next: &mut Show, i: usize, columns: &[String]) -> Vec<String> {
    let r = &mut next.rows[i];
    let asking: Vec<String> = columns.iter().filter(|c| matches!(r.cells.get(*c), Some(Cell::Ask { .. }))).cloned().collect();
    for c in &asking {
        r.cells.remove(c);
        left(r, c);
    }
    asking
}

/// A question `before` asked that no row of `next` asks any more goes with its last cell (validate: every question is asked
/// somewhere); which went. One that was asked nowhere before stays the document's own problem.
pub(crate) fn drop_unasked(before: &Show, next: &mut Show) -> Vec<String> {
    let asked = |s: &Show, id: &str| s.rows.iter().any(|r| r.cells.values().any(|c| matches!(c, Cell::Ask { question } if question == id)));
    let gone: Vec<String> = next.questions.iter().filter(|q| asked(before, &q.id) && !asked(next, &q.id)).map(|q| q.id.clone()).collect();
    next.questions.retain(|q| !gone.contains(&q.id));
    gone
}

/// ` – question q1 went with its last cell`, or nothing
pub(crate) fn gone_words(gone: &[String]) -> String {
    match gone {
        [] => String::new(),
        ids => format!(" – question {} went with its last cell", ids.join(", ")),
    }
}

/// One change to a question, made in the cell inspector's form. What is typed arrives as typed: read here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Set {
    /// the question's words (English; empty: none – the options alone)
    Text(String),
    /// the options, separated by commas (English; an option kept by name keeps its other languages)
    Options(String),
    Rule(Rule),
    /// rule first: the votes an option needs
    N(String),
    Seconds(String),
    /// the option when nobody voted (its place)
    Fallback(usize),
    /// a voter sees the counts once they have voted: on becomes off, off on
    Counts,
}

fn en(text: &str) -> Loc {
    [(DEFAULT_LOCALE.to_string(), text.to_string())].into_iter().filter(|(_, t)| !t.is_empty()).collect()
}

fn whole(what: &str, text: &str) -> Result<i64, String> {
    text.trim().parse::<i64>().map_err(|_| format!("{what}: a whole number"))
}

/// `set` made on `q`; a story question's words are the chapter's and stay (refused, with what to do instead)
pub(crate) fn set(q: &mut Question, set: &Set) -> Result<(), String> {
    let id = q.id.clone();
    fn typed(q: &mut Question) -> Result<&mut crate::document::Typed, String> {
        let id = q.id.clone();
        q.typed.as_mut().ok_or_else(|| format!("question '{id}' is the story's: its words are the chapter's – leave it and ask a new one"))
    }
    match set {
        Set::Text(text) => typed(q)?.question = en(text.trim()),
        Set::Options(text) => {
            let t = typed(q)?;
            let list: Vec<String> = text.split(',').map(str::trim).filter(|o| !o.is_empty()).map(str::to_string).collect();
            let was = std::mem::take(&mut t.options);
            t.options = list.iter().map(|o| was.iter().find(|l| l.get(DEFAULT_LOCALE) == Some(o)).cloned().unwrap_or_else(|| en(o))).collect();
            q.fallback = q.fallback.min(list.len().saturating_sub(1));
        }
        Set::Rule(r) => {
            q.rule = crate::vote::wire_rule(*r);
            q.n = (q.rule == VoteRule::First).then(|| q.n.unwrap_or(Form::default().n));
        }
        Set::N(text) if q.rule == VoteRule::First => q.n = Some(whole(deck_ui::vote::N, text)?),
        Set::N(_) => return Err(format!("question '{id}': n is for the rule {}", Rule::First.name())),
        Set::Seconds(text) => q.seconds = whole(deck_ui::vote::SECONDS, text)?,
        Set::Fallback(i) => q.fallback = *i,
        Set::Counts => q.show = !q.show,
    }
    Ok(())
}

/// A question as #533's form shows it, without groups (the columns that ask it are the groups): its words and options in
/// `locale`'s order, how its vote runs.
pub fn form(q: &Question, locale: &str) -> Form {
    let text = |l: &Loc| protocol::content_text::pick_text(l, locale, None).unwrap_or_default().to_string();
    Form {
        question: q.typed.as_ref().map(|t| text(&t.question)).unwrap_or_default(),
        options: q.options().iter().map(text).collect::<Vec<_>>().join(", "),
        rule: crate::vote::rule(q.rule),
        n: q.n.unwrap_or(Form::default().n),
        seconds: q.seconds,
        fallback: q.fallback,
        groups: Vec::new(),
        show: q.show,
    }
}

/// The question of a cell in the cell inspector.
#[derive(Debug, Clone, PartialEq)]
pub struct QuestionView {
    pub id: String,
    /// `question q1 · left · right`: its id and the columns of the row that ask it
    pub title: String,
    /// its words, options and how its vote runs (deck-ui's form; a story question's words are read-only)
    pub form: Form,
    /// the story's: `catpope ch 3 · ch3.door · version 812` – the snapshot it was picked at; none: typed here
    pub story: Option<String>,
    /// how it runs, or what is wrong with it (the form's check, in its words)
    pub line: String,
    pub wrong: bool,
    /// the published choice changed since it was picked, or is gone (`story::changed`'s words)
    pub warning: Option<String>,
}

/// The question `column`'s cell in `row` asks, for the cell inspector; `listed`: the toon whose story questions the editor
/// has and its list (none: no list – no warning can be said).
pub fn inspect(show: &Show, row: &str, column: &str, listed: Option<(&str, &[StoryChoice])>, locale: &str) -> Option<QuestionView> {
    let r = show.row(row)?;
    let Some(Cell::Ask { question }) = r.cells.get(column) else { return None };
    let q = show.question(question)?;
    let columns: Vec<&str> = show.outlets.iter().filter(|o| matches!(r.cells.get(&o.id), Some(Cell::Ask { question: x }) if x == question)).map(|o| o.id.as_str()).collect();
    let form = form(q, locale);
    let (line, wrong) = match form.problem() {
        Some(p) => (p.to_string(), true),
        None => (form.run_line(), false),
    };
    Some(QuestionView {
        id: q.id.clone(),
        title: format!("question {} · {}", q.id, columns.join(" · ")),
        story: q.story.as_ref().map(|s| format!("{} ch {} · {} · version {}", s.toon, s.chapter, s.event, s.version)),
        warning: listed.and_then(|(toon, choices)| story::changed_text(q, toon, choices)),
        form,
        line,
        wrong,
    })
}

/// What a line of the right click does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Act {
    /// a new typed question on the selected cells
    New,
    /// the toon's story question (its place in the list) on the selected cells
    Story(usize),
    /// the selected cells join this question of the row
    AddTo(String),
    /// the selected cells that ask keep again
    Leave,
}

/// One line of the right click on the selected cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub label: String,
    /// `ch 3 · nave · Stay / Go`; for a line that cannot be picked, why not first
    pub detail: String,
    /// none: listed with its reason, a pick says it
    pub act: Option<Act>,
}

/// The right click's lines for the selected cells of one row, and what a pick does.
#[derive(Debug, Clone, PartialEq)]
pub struct Offers {
    pub row: String,
    pub columns: Vec<String>,
    pub list: Vec<Offer>,
    /// per line: the story question it stands for (its place in the toon's list; also one that cannot be picked)
    choice: Vec<Option<usize>>,
    /// for the story's lines: the document they were made for, the toon, its questions, the labels' language
    show: Show,
    toon: String,
    choices: Vec<StoryChoice>,
    locale: String,
}

/// What the right click on `columns` of `row` offers, in this order: "New question …" (a row asks one question: not while
/// it asks one), the toon's story questions (`listed`: the toon and its list, or why there is none – a question without
/// an id or options is listed with why it cannot be picked), "Add to question <q>" for each question of the row a
/// selected cell does not ask yet, "Leave the question" when a selected cell asks.
pub fn offers(show: &Show, row: &str, columns: &[String], listed: Result<(&str, &[StoryChoice]), &str>, locale: &str) -> Offers {
    let mut list = vec![];
    let mut choice = vec![];
    let r = show.row(row);
    let asks = |c: &String| r.and_then(|r| r.cells.get(c)).and_then(|c| if let Cell::Ask { question } = c { Some(question.as_str()) } else { None });
    let mut of_row: Vec<&str> = r.iter().flat_map(|r| r.cells.values()).filter_map(|c| if let Cell::Ask { question } = c { Some(question.as_str()) } else { None }).collect();
    of_row.sort_unstable();
    of_row.dedup();
    if of_row.is_empty() {
        list.push(Offer { label: NEW.into(), detail: format!("typed here · {}", columns.join(" · ")), act: Some(Act::New) });
    }
    let (toon, choices) = match listed {
        Ok((toon, choices)) => {
            for v in story::choice_lines(show, toon, choices, "", locale) {
                choice.resize(list.len(), None);
                choice.push(Some(v.choice));
                list.push(Offer { label: v.label, detail: v.detail, act: v.can.then_some(Act::Story(v.choice)) });
            }
            (toon.to_string(), choices.to_vec())
        }
        Err(why) => {
            list.push(Offer { label: STORY.into(), detail: why.to_string(), act: None });
            (String::new(), vec![])
        }
    };
    for q in of_row.iter().filter(|q| columns.iter().any(|c| asks(c) != Some(**q))) {
        let label = show.question(q).map_or_else(String::new, |x| x.label(locale));
        list.push(Offer { label: add_to(q), detail: label, act: Some(Act::AddTo(q.to_string())) });
    }
    if columns.iter().any(|c| asks(c).is_some()) {
        list.push(Offer { label: LEAVE.into(), detail: "the cell keeps again".into(), act: Some(Act::Leave) });
    }
    choice.resize(list.len(), None);
    Offers { row: row.to_string(), columns: columns.to_vec(), list, choice, show: show.clone(), toon, choices, locale: locale.to_string() }
}

impl Offers {
    /// The lines `query` finds (their places in `list`, in its order): a story question by the question picker's rule
    /// (`story::choice_lines`: its id, group, chapter, an option's label in any language), the others by every word typed.
    pub fn find(&self, query: &str) -> Vec<usize> {
        let found: Vec<usize> = story::choice_lines(&self.show, &self.toon, &self.choices, query, &self.locale).into_iter().map(|v| v.choice).collect();
        let words: Vec<String> = query.to_lowercase().split_whitespace().map(str::to_string).collect();
        let reads = |o: &Offer| {
            let hay = format!("{} {}", o.label, o.detail).to_lowercase();
            words.iter().all(|w| hay.contains(w))
        };
        (0..self.list.len()).filter(|&i| self.choice[i].map_or_else(|| reads(&self.list[i]), |c| found.contains(&c))).collect()
    }

    /// Line `i` picked: the edit made on `show` (`rows::edit`, `story::pick`: checked by validate), or why not – a line
    /// that cannot be picked says its reason.
    pub fn pick(&self, show: &Show, i: usize) -> Result<Edited, String> {
        let o = self.list.get(i).ok_or("no such line")?;
        let (row, columns) = (self.row.clone(), self.columns.clone());
        match &o.act {
            None => Err(o.detail.clone()),
            Some(Act::New) => crate::rows::edit(show, &Edit::NewQuestion { row, columns }),
            Some(Act::AddTo(question)) => crate::rows::edit(show, &Edit::AddTo { row, columns, question: question.clone() }),
            Some(Act::Leave) => crate::rows::edit(show, &Edit::Leave { row, columns }),
            Some(Act::Story(c)) => story::pick(show, &row, &columns, self.choices.get(*c).ok_or("no such story question")?, &self.toon),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse;
    use crate::rows::edit;
    use crate::validate::validate;

    const VOTE: &str = include_str!("../fixtures/vote-cell.show.json");
    const EVENTS: &str = include_str!("../fixtures/story-events.json");

    fn cols(c: &[&str]) -> Vec<String> {
        c.iter().map(|c| c.to_string()).collect()
    }

    fn ed(show: &Show, e: Edit) -> Edited {
        edit(show, &e).unwrap_or_else(|why| panic!("{e:?}: {why}"))
    }

    fn labels(o: &Offers) -> Vec<&str> {
        o.list.iter().map(|o| o.label.as_str()).collect()
    }

    /// #551's done-when, the core's half: two cells that are not adjacent become one new question, for phones; a third joins
    /// it; one leaves; the last one leaving takes the question with it
    #[test]
    fn selected_cells_become_a_new_question_a_cell_joins_and_leaves() {
        let show = parse(VOTE).unwrap();
        let made = ed(&show, Edit::NewQuestion { row: "opening".into(), columns: cols(&["left", "right"]) });
        assert_eq!((made.said.as_str(), made.row.as_deref()), ("row 1 opening: left · right ask the new question q2 – device phone", Some("opening")));
        let r = made.show.row("opening").unwrap();
        assert_eq!((r.cells.get("left"), r.cells.get("right")), (Some(&Cell::Ask { question: "q2".into() }), Some(&Cell::Ask { question: "q2".into() })));
        assert!(matches!(r.cells.get("middle"), Some(Cell::Play { .. })), "the column between stays");
        assert_eq!((r.r#where.get("left"), r.r#where.get("right")), (Some(&new_filter()), Some(&new_filter())), "a new question starts with device: phone");
        let q2 = made.show.question("q2").unwrap();
        assert_eq!((form(q2, "en").run_line().as_str(), q2.typed.as_ref().map(|t| t.question.is_empty())), ("Yes, No · majority · 30 s · fallback Yes", Some(true)));
        assert!(validate(&made.show).is_empty(), "{:?}", validate(&made.show));
        // saved and read again: the same document
        assert_eq!(parse(&serde_json::to_string(&made.show).unwrap()).unwrap(), made.show);

        // a row asks one question: a second new one is refused in validate's words; a device outlet does not ask
        let second = edit(&made.show, &Edit::NewQuestion { row: "opening".into(), columns: cols(&["middle"]) }).unwrap_err();
        assert!(second.starts_with("not changed: row 'opening': one question per row"), "{second}");
        assert!(edit(&show, &Edit::NewQuestion { row: "opening".into(), columns: cols(&["lamps"]) }).unwrap_err().starts_with("not changed: "));
        assert_eq!(edit(&show, &Edit::NewQuestion { row: "opening".into(), columns: vec![] }).unwrap_err(), "row 1 opening: pick the cells that ask it");

        // add to question: the column between joins with the filter the others carry
        let joined = ed(&made.show, Edit::AddTo { row: "opening".into(), columns: cols(&["middle"]), question: "q2".into() });
        assert_eq!(joined.said, "row 1 opening: middle asks q2 – device phone");
        assert_eq!(joined.show.row("opening").unwrap().r#where.get("middle"), Some(&new_filter()));
        assert!(validate(&joined.show).is_empty());
        assert_eq!(edit(&made.show, &Edit::AddTo { row: "opening".into(), columns: cols(&["middle"]), question: "q9".into() }).unwrap_err(), "no question 'q9'");

        // leave: the cell keeps again and its filter goes; the last cell takes the question with it
        let one = ed(&joined.show, Edit::Leave { row: "opening".into(), columns: cols(&["left", "middle"]) });
        assert_eq!(one.said, "row 1 opening: left · middle keep again");
        let r = one.show.row("opening").unwrap();
        assert_eq!((r.cells.len(), r.r#where.len()), (1, 1));
        let none = ed(&one.show, Edit::Leave { row: "opening".into(), columns: cols(&["right"]) });
        assert_eq!(none.said, "row 1 opening: right keeps again – question q2 went with its last cell");
        assert!(none.show.question("q2").is_none() && none.show.row("opening").unwrap().r#where.is_empty());
        assert!(validate(&none.show).is_empty());
        assert_eq!(edit(&none.show, &Edit::Leave { row: "opening".into(), columns: cols(&["left"]) }).unwrap_err(), "row 1 opening: none of the cells picked asks a question");

        // a cell's mode chip off `ask`, and a removed row, take the question with its last cell too
        let mode = ed(&one.show, Edit::Mode { row: "opening".into(), column: "right".into(), mode: crate::rows::Mode::Black });
        assert!(mode.show.question("q2").is_none() && mode.said.ends_with("– question q2 went with its last cell"), "{}", mode.said);
        let removed = ed(&show, Edit::Remove { id: "crossroads".into() });
        assert!(removed.show.questions.is_empty(), "q1 was asked only there");
    }

    /// the question in the cell inspector: #533's form without groups; each field is an edit checked by validate
    #[test]
    fn the_question_in_the_inspector_is_the_form_and_its_fields_are_edits() {
        let show = parse(VOTE).unwrap();
        let v = inspect(&show, "crossroads", "left", None, "en").unwrap();
        assert_eq!((v.title.as_str(), v.story.as_deref(), v.line.as_str(), v.wrong, v.warning.as_deref()), ("question q1 · left · right", None, "Left, Right, Stay · majority · 30 s · fallback Stay", false, None));
        assert_eq!((v.form.question.as_str(), v.form.options.as_str(), v.form.groups.len()), ("Which way?", "Left, Right, Stay", 0));
        assert_eq!(inspect(&show, "crossroads", "left", None, "de").unwrap().form.options, "Left, Rechts, Stay", "the labels in the editor's language");
        assert!(inspect(&show, "crossroads", "middle", None, "en").is_none(), "no question there");

        let q = |set: Set| Edit::Question { question: "q1".into(), set };
        let text = ed(&show, q(Set::Text(" Where to? ".into())));
        assert_eq!((text.said.as_str(), text.row.as_deref()), ("question q1: Where to? · Left, Right, Stay · majority · 30 s · fallback Stay", Some("crossroads")));
        assert_eq!(text.show.question("q1").unwrap().typed.as_ref().unwrap().question.get("de").map(String::as_str), None, "typed anew: the other languages' words went");
        // the options: one kept by name keeps its other languages; the fallback stays an option
        let options = ed(&show, q(Set::Options("Right, Up".into())));
        let t = options.show.question("q1").unwrap();
        assert_eq!((t.options()[0].get("de").map(String::as_str), t.options()[1].len(), t.fallback), (Some("Rechts"), 1, 1));
        assert_eq!(edit(&show, &q(Set::Options("only".into()))).unwrap_err(), "not changed: show: question 'q1': A vote needs at least two options (separated by commas).");
        // the rule first to n brings n; n and the seconds are read here
        let first = ed(&show, q(Set::Rule(Rule::First)));
        assert_eq!(first.show.question("q1").unwrap().n, Some(20));
        assert_eq!(ed(&first.show, q(Set::N("3".into()))).said, "question q1: Which way? · Left, Right, Stay · first to 3 · 30 s · fallback Stay");
        assert_eq!(edit(&first.show, &q(Set::N("many".into()))).unwrap_err(), "n: a whole number");
        assert_eq!(edit(&show, &q(Set::N("3".into()))).unwrap_err(), "question 'q1': n is for the rule first to n");
        assert_eq!(ed(&first.show, q(Set::Rule(Rule::Majority))).show.question("q1").unwrap().n, None);
        assert_eq!(edit(&show, &q(Set::Seconds("0".into()))).unwrap_err(), "not changed: show: question 'q1': Seconds: at least 1.");
        assert_eq!(edit(&show, &q(Set::Seconds("soon".into()))).unwrap_err(), "Seconds: a whole number");
        assert_eq!(ed(&show, q(Set::Seconds("45".into()))).show.question("q1").unwrap().seconds, 45);
        assert_eq!(ed(&show, q(Set::Fallback(0))).show.question("q1").unwrap().fallback, 0);
        assert_eq!(edit(&show, &q(Set::Fallback(7))).unwrap_err(), "not changed: show: question 'q1': The fallback is one of the options.");
        assert!(!ed(&show, q(Set::Counts)).show.question("q1").unwrap().show);
        assert_eq!(edit(&show, &Edit::Question { question: "q9".into(), set: Set::Counts }).unwrap_err(), "no question 'q9'");
    }

    /// the right click's lines in the issue's order, found as typed; a story question picked the same way; its snapshot is
    /// read-only in the inspector and warns when the published choice changed
    #[test]
    fn the_right_click_offers_a_new_question_the_storys_add_to_and_leave() {
        let show = parse(VOTE).unwrap();
        let story = story::read(EVENTS, "catpope").unwrap();
        let listed = Ok(("catpope", story.choices.as_slice()));
        // a row that asks nothing: a new question, then the story's – the id-less one listed with its reason
        let o = offers(&show, "opening", &cols(&["left", "right"]), listed, "en");
        assert_eq!(labels(&o), [NEW, "ch3.door", "no id", "ch3.empty"]);
        assert_eq!(o.list[0].detail, "typed here · left · right");
        assert_eq!((o.list[2].act.clone(), o.list[2].detail.as_str()), (None, "no id – the chapter editor gives the question one · ch 3 · crypt · Ja / Nein"));
        assert_eq!(o.pick(&show, 2).unwrap_err(), "no id – the chapter editor gives the question one · ch 3 · crypt · Ja / Nein");
        // typed to search: the story's by the question picker's rule (an option's label in any language), the others by word
        assert_eq!(o.find(""), [0, 1, 2, 3]);
        assert_eq!(o.find("bleiben"), [1], "German label, English list");
        assert_eq!(o.find("crypt"), [2]);
        assert_eq!(o.find("new"), [0]);
        assert!(o.find("nothing like it").is_empty());

        // "New question …" picked
        let new = o.pick(&show, 0).unwrap();
        assert_eq!(new.show.row("opening").unwrap().cells.get("right"), Some(&Cell::Ask { question: "q2".into() }));
        // a story question picked: a new question with its snapshot, for phones
        let door = o.pick(&show, 1).unwrap();
        assert_eq!(door.said, "row 1 opening: q2 asks catpope ch 3 ch3.door (version 812)");
        assert_eq!(door.show.row("opening").unwrap().r#where.get("left"), Some(&new_filter()));
        assert!(validate(&door.show).is_empty(), "{:?}", validate(&door.show));
        let v = inspect(&door.show, "opening", "left", Some(("catpope", &story.choices)), "en").unwrap();
        assert_eq!((v.story.as_deref(), v.form.options.as_str(), v.form.question.as_str(), v.warning), (Some("catpope ch 3 · ch3.door · version 812"), "Stay, Go", "", None));
        // its words are the chapter's: read-only; how its vote runs is not
        let refused = edit(&door.show, &Edit::Question { question: "q2".into(), set: Set::Options("a, b".into()) }).unwrap_err();
        assert!(refused.starts_with("question 'q2' is the story's"), "{refused}");
        assert_eq!(ed(&door.show, Edit::Question { question: "q2".into(), set: Set::Seconds("20".into()) }).said, "question q2: story: ch3.door · Stay, Go · majority · 20 s · fallback Stay");
        // the published choice changed since: the inspector warns in story::changed's words
        let mut newer = story.choices.clone();
        newer[0].version = 815;
        newer[0].options[1].label.insert("en".into(), "Leave".into());
        let warned = inspect(&door.show, "opening", "left", Some(("catpope", &newer)), "en").unwrap();
        assert_eq!(warned.warning.as_deref(), Some("question 'q2': ch3.door changed since it was picked (version 812 → 815) – pick it again"));

        // a row that asks q1 on left and right: no new question; the cell between may join, a cell that asks may leave
        let o = offers(&show, "crossroads", &cols(&["middle"]), listed, "en");
        assert_eq!(labels(&o), ["ch3.door", "no id", "ch3.empty", "Add to question q1"]);
        assert_eq!(o.list[3].detail, "Which way?");
        let added = o.pick(&show, 3).unwrap();
        assert_eq!(added.said, "row 2 crossroads: middle asks q1 – device phone · member · live");
        assert!(validate(&added.show).is_empty());
        let o = offers(&show, "crossroads", &cols(&["left", "middle"]), listed, "en");
        assert_eq!(labels(&o)[3..], ["Add to question q1", LEAVE]);
        assert_eq!(o.find("leave"), [4]);
        let o = offers(&show, "crossroads", &cols(&["left", "right"]), listed, "en");
        assert_eq!(labels(&o)[3..], [LEAVE], "both ask it already");
        // the story's question on a row that asks: its question takes it (story::pick)
        assert_eq!(o.pick(&show, 0).unwrap().show.question("q1").unwrap().story.as_ref().map(|s| s.event.as_str()), Some("ch3.door"));

        // no list: one line says why, a pick says it too
        let o = offers(&show, "opening", &cols(&["left"]), Err("no toon – the editor lists a toon's story events with --toon <toon>"), "en");
        assert_eq!(labels(&o), [NEW, STORY]);
        assert_eq!(o.pick(&show, 1).unwrap_err(), "no toon – the editor lists a toon's story events with --toon <toon>");
        assert_eq!(o.find("story"), [1]);
    }
}
