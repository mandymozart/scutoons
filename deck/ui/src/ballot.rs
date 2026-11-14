//! The ballot (#556, research/SHOW.md §15 point 1; owner 2026-11-14 [live]: "Deck draws the questions", decks vote): the
//! voter's side of a vote, drawn by the deck itself over the scene that plays – beside the conductor's `Panel` (vote.rs).
//! Plain data and words, no window: [`State`] is what a deck has of the vote it may cast in (the relay sends `vote.open`
//! only to who may vote, #547; deck-space's relay client feeds it and casts, relay.rs `Client::ballot`, `vote_cast`),
//! [`View`] its words by the deck's locale at a moment – the question and the options, the time left, mine, the shares
//! once this deck has voted and the vote shows them, then the result and how –, [`Page`] the picture (deck-kit live's
//! panel parts, the layer `VOTE` of the deck's screen) and what a key or a press on it means: [`Do::Cast`].
//! The rule stays the relay's: nothing here counts a vote or decides one.
use crate::live::Do;
use crate::vote::{decided, percent, How, Rule};
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::input::Input;
use deck_kit::base::screen::Layer;
use deck_kit::live::panel::{self, Part};
use protocol::content_text::pick_text;
use protocol::{Message, VoteOpen, VoteResult, VoteTally};

/// how long the result shows before the ballot goes (ms)
pub const RESULT_MS: f64 = 8000.0;

/// how solid the ballot's ground is over the scene (the panel's own is 1)
const GROUND: f32 = 0.72;

/// What a deck has of the vote it may cast in. Times are ms on the clock it is fed with (the relay client's).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    on: Option<On>,
}

#[derive(Debug, Clone, PartialEq)]
struct On {
    open: VoteOpen,
    /// when the countdown ends
    ends: f64,
    /// the conductor holds it: the ms left stand still
    held: Option<f64>,
    mine: Option<usize>,
    /// the last tally's counts (the relay sends a voter the tally only after its vote, and only when the vote shows it)
    counts: Option<Vec<i64>>,
    /// the option that won, how, and when this deck heard it
    result: Option<(usize, How, f64)>,
}

impl State {
    /// One message of the relay at `now`: `vote.open` (also the welcome's snapshot's), `vote.tally`, `vote.result`;
    /// anything else is not the ballot's.
    pub fn hear(&mut self, m: &Message, now: f64) {
        match m {
            Message::VoteOpen(v) => self.open(v, now),
            Message::VoteTally(t) => self.tally(t, now),
            Message::VoteResult(r) => self.result(r, now),
            _ => {}
        }
    }

    /// A vote opens for this deck. The vote that is open said again (a snapshot after a lost connection) keeps the
    /// countdown and what this deck cast.
    pub fn open(&mut self, v: &VoteOpen, now: f64) {
        if self.on.as_ref().is_some_and(|on| on.result.is_none() && on.open.vote == v.vote) {
            return;
        }
        self.on = Some(On { open: v.clone(), ends: now + v.seconds as f64 * 1000.0, held: None, mine: None, counts: None, result: None });
    }

    /// The open vote's tally: its counts, the time left as the relay has it, held or not. Another vote's is not heard.
    pub fn tally(&mut self, t: &VoteTally, now: f64) {
        let Some(on) = self.on.as_mut().filter(|on| on.result.is_none() && on.open.vote == t.vote) else { return };
        on.counts = Some(t.counts.clone());
        on.held = (t.paused == Some(true)).then_some(t.left as f64);
        on.ends = now + t.left as f64;
    }

    /// The vote closed. `vote.result` goes to everyone: the result of a vote this deck never heard is ignored.
    pub fn result(&mut self, r: &VoteResult, now: f64) {
        let Some(on) = self.on.as_mut().filter(|on| on.result.is_none() && on.open.vote == r.vote) else { return };
        let Some(option) = usize::try_from(r.option).ok().filter(|o| *o < on.open.options.len()) else {
            // no such option: nothing to show
            self.on = None;
            return;
        };
        on.result = Some((option, r.how.into(), now));
    }

    /// This deck cast `option` (it may change it until the vote closes); false: no vote is open, or it has no such option.
    pub fn cast(&mut self, option: usize) -> bool {
        match self.on.as_mut().filter(|on| on.result.is_none() && option < on.open.options.len()) {
            Some(on) => {
                on.mine = Some(option);
                true
            }
            None => false,
        }
    }

    /// The ballot at `now` in `locale` (`de…`: German, else English; the question and the labels as the vote brings
    /// them): none = nothing shows – no vote, or its result has shown.
    pub fn view(&self, now: f64, locale: &str) -> Option<View> {
        let on = self.on.as_ref()?;
        let w = words(locale);
        let labels = labels(&on.open, locale);
        let question = on.open.question.as_ref().and_then(|q| pick_text(q, lang(locale), None)).unwrap_or_default().to_string();
        let label = |i: usize| labels.get(i).cloned().unwrap_or_default();
        if let Some((option, how, at)) = on.result {
            // a clock that went back shows it on
            if now - at >= RESULT_MS {
                return None;
            }
            let yours = on.mine.map(|m| format!("{}: {}", w.yours, label(m)));
            let result = Outcome { winner: label(option), how: decided(Rule::from(on.open.rule), how, locale).to_string(), yours };
            return Some(View { vote: on.open.vote.clone(), caption: (w.result.to_string(), String::new()), question, choices: Vec::new(), hint: String::new(), result: Some(result) });
        }
        let left = match on.held {
            Some(_) => w.held.to_string(),
            None => (w.left)(((on.ends - now) / 1000.0).ceil().max(0.0) as i64),
        };
        // the counts only after this deck voted, and only when the vote shows them
        let counts = on.counts.as_ref().filter(|_| on.mine.is_some() && on.open.show == Some(true));
        let choices = labels
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let percent = counts.map(|c| percent(c.get(i).copied().unwrap_or(0), c));
                Choice { label: l.clone(), mine: on.mine == Some(i), percent, share: percent.map(|p| p as f32 / 100.0) }
            })
            .collect();
        let hint = if on.mine.is_some() { w.change } else { w.pick };
        Some(View { vote: on.open.vote.clone(), caption: (w.vote.to_string(), left), question, choices, hint: hint.to_string(), result: None })
    }
}

/// the language of a locale (`de-AT` → `de`)
fn lang(locale: &str) -> &str {
    locale.split(['-', '_']).next().unwrap_or(locale)
}

/// The option labels for the deck: the vote's `labels` in its language, else English, else `options`.
fn labels(open: &VoteOpen, locale: &str) -> Vec<String> {
    let of = |l: &str| open.labels.as_ref().and_then(|m| m.get(l)).filter(|l| l.len() == open.options.len());
    of(lang(locale)).or_else(|| of("en")).unwrap_or(&open.options).clone()
}

/// The ballot's own words (the question and the labels come with the vote; how it was decided: vote.rs `decided`).
struct Words {
    vote: &'static str,
    left: fn(i64) -> String,
    held: &'static str,
    pick: &'static str,
    change: &'static str,
    result: &'static str,
    yours: &'static str,
}

fn words(locale: &str) -> Words {
    if lang(locale) == "de" {
        Words { vote: "Abstimmung", left: |s| format!("noch {s} s"), held: "angehalten", pick: "Triff deine Wahl.", change: "Du kannst bis zum Schluss wechseln.", result: "Ergebnis", yours: "Deine Stimme" }
    } else {
        Words { vote: "Vote", left: |s| format!("{s} s left"), held: "on hold", pick: "Make your choice.", change: "You can change it until the end.", result: "Result", yours: "Your vote" }
    }
}

/// One option on the ballot: its label, whether it is this deck's, its share in whole percent and as 0..1 (none: the
/// counts do not show – not voted yet, or the vote keeps them).
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub label: String,
    pub mine: bool,
    pub percent: Option<i64>,
    pub share: Option<f32>,
}

/// The result: the winning option's label, how it was decided, `Your vote: Yes` when this deck voted.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub winner: String,
    pub how: String,
    pub yours: Option<String>,
}

/// The ballot's words at a moment.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    /// the vote's id
    pub vote: String,
    /// what this is (`Vote`, `Result`) and the time left (`12 s left`, `on hold`; none with the result)
    pub caption: (String, String),
    /// the question (empty: the vote brought none)
    pub question: String,
    /// the options while the vote is open
    pub choices: Vec<Choice>,
    /// what to do (`Make your choice.`, then `You can change it until the end.`)
    pub hint: String,
    pub result: Option<Outcome>,
}

/// The ballot as a picture and what a key or a press on it means.
#[derive(Debug, Clone, Default)]
pub struct Page {
    /// what is painted now: its words and the screen (none: nothing, or to paint again)
    drawn: Option<String>,
    /// where the options are, in logical pixels
    places: Vec<Rect>,
    /// device pixels per logical one, as painted
    scale: f32,
    /// where the ballot begins, in device pixels from the screen's top
    top: f32,
    /// the person uses keys: the ballot shows which option they are on
    keys: bool,
    cursor: usize,
    /// the vote the cursor is in
    vote: String,
}

impl Page {
    /// What to show now on a `w × h` screen (device pixels), in the deck's mono face (`per_px`: its advance per pixel of
    /// font size): None = as before; Some(None) = nothing any more; Some(layer) = this, along the screen's bottom.
    pub fn paint(&mut self, view: Option<&View>, (w, h): (f32, f32), per_px: f32) -> Option<Option<Layer>> {
        self.follow(view);
        let drawn = view.map(|v| format!("{v:?} {w}x{h} {} {}", self.keys, self.cursor));
        if drawn == self.drawn {
            return None;
        }
        self.drawn = drawn;
        self.places.clear();
        let Some(view) = view else { return Some(None) };
        let scale = panel::scale(w, h);
        let screen = (w / scale, h / scale);
        // the panel's look, its ground a little see-through: the scene plays on behind the ballot
        let mut tokens = deck_kit::live::tokens::panel();
        tokens.color.surface[3] = GROUND;
        let mut d = DrawList::new(per_px);
        let notes: Vec<String> = view.choices.iter().map(|c| c.percent.map(|p| format!("{p}%")).unwrap_or_default()).collect();
        let mut parts = vec![Part::Caption(&view.caption.0, &view.caption.1)];
        match &view.result {
            Some(r) => {
                parts.push(Part::Title(&r.winner));
                parts.push(Part::Line(&r.how, true));
                parts.extend(r.yours.iter().map(|y| Part::Line(y, false)));
            }
            None => {
                if !view.question.is_empty() {
                    parts.push(Part::Title(&view.question));
                }
                parts.extend(view.choices.iter().zip(&notes).enumerate().map(|(i, (c, note))| Part::Choice(&c.label, note, c.mine, self.keys && self.cursor == i, c.share)));
                parts.push(Part::Line(&view.hint, true));
            }
        }
        self.places = panel::draw(&mut d, &tokens, screen, &parts, !self.keys);
        // the glow above the edge is part of what shows
        let top = (screen.1 - panel::height(&d, &tokens, screen.0, &parts) - 16.0).max(0.0);
        (self.scale, self.top) = (scale, top * scale);
        Some(Some(panel::layer(d, scale)))
    }

    /// Where the ballot begins, in device pixels from the screen's top; none = nothing shows.
    pub fn top(&self) -> Option<f32> {
        self.drawn.as_ref().map(|_| self.top)
    }

    /// The middle of option `i` as painted, in device pixels (a scripted press: the headless host); none = not there.
    pub fn at(&self, i: usize) -> Option<(f32, f32)> {
        self.places.get(i).map(|p| ((p.x + p.w * 0.5) * self.scale, (p.y + p.h * 0.5) * self.scale))
    }

    /// The stage was made anew: the ballot is painted again with the next `paint`.
    pub fn again(&mut self) {
        self.drawn = None;
    }

    /// another vote: the keys start anew
    fn follow(&mut self, view: Option<&View>) {
        let vote = view.map_or("", |v| v.vote.as_str());
        if self.vote != vote {
            self.vote = vote.to_string();
            (self.keys, self.cursor) = (false, 0);
        }
    }

    /// One input of the person at the deck while `view` shows – as the menu takes it (deck-kit `Input`: a key, a pad's
    /// button, a click, a tap; places in device pixels). Up and Down go through the options (the first key only shows
    /// where the keys are: on this deck's option, else the first), Enter casts the one they are on; a press on an option
    /// casts it. Whether the input was the ballot's, and what the adapter does for it.
    pub fn input(&mut self, i: Input, view: Option<&View>) -> (bool, Option<Do>) {
        self.follow(view);
        let Some(view) = view else { return (false, None) };
        let n = view.choices.len();
        match i {
            Input::Press(x, y) => {
                let (lx, ly) = (x / self.scale.max(1.0), y / self.scale.max(1.0));
                match self.places.iter().position(|at| at.has(lx, ly)).filter(|i| *i < n) {
                    Some(i) => {
                        (self.keys, self.cursor) = (false, i);
                        (true, Some(Do::Cast(i)))
                    }
                    // a press on the ballot beside its options is not the scene's
                    None => (self.top().is_some_and(|top| y >= top), None),
                }
            }
            Input::Up | Input::Down | Input::Enter if n > 0 => {
                if !self.keys {
                    self.keys = true;
                    self.cursor = view.choices.iter().position(|c| c.mine).unwrap_or(0);
                    return (true, None);
                }
                self.cursor = self.cursor.min(n - 1);
                match i {
                    Input::Up => self.cursor = self.cursor.saturating_sub(1),
                    Input::Down => self.cursor = (self.cursor + 1).min(n - 1),
                    _ => return (true, Some(Do::Cast(self.cursor))),
                }
                (true, None)
            }
            _ => (false, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCREEN: (f32, f32) = (800.0, 600.0);
    const FONT: f32 = 0.6;

    fn open(vote: &str, show: bool) -> VoteOpen {
        serde_json::from_value(json!({
            "vote": vote, "options": ["Yes", "No", "Later"], "rule": "majority", "seconds": 20, "show": show,
            "question": { "en": "Shall the cat sing?", "de": "Soll die Katze singen?" },
            "labels": { "de": ["Ja", "Nein", "Später"] }
        }))
        .unwrap()
    }

    fn tally(vote: &str, counts: [i64; 3], left: i64, paused: bool) -> VoteTally {
        serde_json::from_value(json!({ "vote": vote, "counts": counts, "voters": counts.iter().sum::<i64>(), "left": left, "paused": paused })).unwrap()
    }

    fn result(vote: &str, option: i64, how: &str) -> VoteResult {
        serde_json::from_value(json!({ "vote": vote, "option": option, "counts": [2, 1, 0], "how": how })).unwrap()
    }

    fn texts(l: &Layer) -> Vec<String> {
        l.list.texts().into_iter().map(String::from).collect()
    }

    #[test]
    fn the_words_follow_the_decks_locale_and_the_countdown_runs() {
        let mut s = State::default();
        assert_eq!(s.view(0.0, "en"), None, "no vote: nothing shows");
        s.open(&open("v1", true), 1000.0);
        let en = s.view(1000.0, "en").unwrap();
        assert_eq!((en.caption.0.as_str(), en.caption.1.as_str(), en.question.as_str(), en.hint.as_str()), ("Vote", "20 s left", "Shall the cat sing?", "Make your choice."));
        assert_eq!(en.choices.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(), ["Yes", "No", "Later"], "no English labels: the options");
        let de = s.view(8500.0, "de-AT").unwrap();
        assert_eq!((de.caption.0.as_str(), de.caption.1.as_str(), de.question.as_str(), de.hint.as_str()), ("Abstimmung", "noch 13 s", "Soll die Katze singen?", "Triff deine Wahl."));
        assert_eq!(de.choices.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(), ["Ja", "Nein", "Später"]);
        assert_eq!(s.view(60_000.0, "fr").unwrap().caption.1, "0 s left", "past its time it waits for the relay's result; another language reads English");
        // the relay's time left wins over the deck's clock; held, it stands
        s.tally(&tally("v1", [0, 0, 0], 4200, false), 9000.0);
        assert_eq!(s.view(9000.0, "en").unwrap().caption.1, "5 s left");
        s.tally(&tally("v1", [0, 0, 0], 4000, true), 9200.0);
        assert_eq!(s.view(30_000.0, "en").unwrap().caption.1, "on hold");
        assert_eq!(s.view(30_000.0, "de").unwrap().caption.1, "angehalten");
    }

    #[test]
    fn the_tally_shows_only_after_voting_and_only_when_the_vote_shows_it() {
        let mut s = State::default();
        s.open(&open("v1", true), 0.0);
        s.tally(&tally("v1", [2, 1, 0], 9000, false), 100.0);
        assert!(s.view(100.0, "en").unwrap().choices.iter().all(|c| c.percent.is_none() && c.share.is_none() && !c.mine), "not voted: no counts");
        assert!(!s.cast(3) && s.cast(1), "an option the vote has");
        let v = s.view(200.0, "en").unwrap();
        assert_eq!(v.choices.iter().map(|c| (c.mine, c.percent)).collect::<Vec<_>>(), [(false, Some(67)), (true, Some(33)), (false, Some(0))]);
        assert_eq!((v.choices[0].share, v.hint.as_str()), (Some(0.67), "You can change it until the end."));
        // changed until the close
        assert!(s.cast(0));
        assert!(s.view(300.0, "en").unwrap().choices[0].mine);
        // a vote that keeps its counts: never, whatever arrives
        let mut hidden = State::default();
        hidden.open(&open("v2", false), 0.0);
        hidden.cast(0);
        hidden.tally(&tally("v2", [1, 0, 0], 9000, false), 100.0);
        assert!(hidden.view(100.0, "en").unwrap().choices.iter().all(|c| c.percent.is_none()));
        // another vote's tally is not this one's
        s.tally(&tally("v9", [0, 0, 9], 1, true), 400.0);
        assert_eq!(s.view(400.0, "en").unwrap().choices[2].percent, Some(0));
    }

    #[test]
    fn the_result_shows_with_how_it_was_decided_then_the_ballot_goes() {
        let mut s = State::default();
        s.open(&open("v1", true), 0.0);
        s.cast(1);
        s.result(&result("v1", 0, "rule"), 5000.0);
        let v = s.view(5000.0, "en").unwrap();
        assert_eq!((v.caption.0.as_str(), v.caption.1.as_str(), v.choices.len()), ("Result", "", 0));
        assert_eq!(v.result, Some(Outcome { winner: "Yes".into(), how: "Most votes.".into(), yours: Some("Your vote: No".into()) }));
        let de = s.view(5000.0 + RESULT_MS - 1.0, "de").unwrap();
        assert_eq!(de.result, Some(Outcome { winner: "Ja".into(), how: "Die meisten Stimmen.".into(), yours: Some("Deine Stimme: Nein".into()) }));
        assert_eq!(s.view(5000.0 + RESULT_MS, "en"), None, "the result has shown: the ballot goes");
        assert!(!s.cast(0), "a closed vote takes no cast");
        // the fallback and an override, a deck that did not vote
        for (how, words) in [("fallback", "Nobody voted – it stays as it was."), ("override", "The conductor decided.")] {
            let mut s = State::default();
            s.open(&open("v1", true), 0.0);
            s.result(&result("v1", 2, how), 10.0);
            assert_eq!(s.view(10.0, "en").unwrap().result, Some(Outcome { winner: "Later".into(), how: words.into(), yours: None }));
        }
    }

    #[test]
    fn the_result_of_a_vote_never_heard_is_ignored() {
        let mut s = State::default();
        // `vote.result` goes to everyone: a deck that did not pass the vote's filter hears only that
        s.hear(&Message::VoteResult(result("v1", 0, "rule")), 0.0);
        s.hear(&Message::VoteTally(tally("v1", [1, 0, 0], 100, false)), 0.0);
        assert_eq!((s.view(0.0, "en"), s.cast(0)), (None, false));
        // … and while its own vote is open, another vote's result does not close it
        s.hear(&Message::VoteOpen(open("v2", true)), 0.0);
        s.hear(&Message::VoteResult(result("v1", 0, "rule")), 10.0);
        assert!(s.view(10.0, "en").is_some_and(|v| v.result.is_none()));
        // the open vote said again (the snapshot after a lost connection) keeps what was cast and the countdown
        s.cast(2);
        s.hear(&Message::VoteOpen(open("v2", true)), 15_000.0);
        let v = s.view(15_000.0, "en").unwrap();
        assert!(v.choices[2].mine && v.caption.1 == "5 s left", "{v:?}");
        // the next vote starts clean, also while the last result still shows
        s.hear(&Message::VoteResult(result("v2", 1, "rule")), 16_000.0);
        s.hear(&Message::VoteOpen(open("v3", true)), 17_000.0);
        let v = s.view(17_000.0, "en").unwrap();
        assert!(v.result.is_none() && v.choices.iter().all(|c| !c.mine) && v.caption.1 == "20 s left");
    }

    #[test]
    fn keys_and_a_press_cast() {
        let mut s = State::default();
        s.open(&open("v1", true), 0.0);
        let mut p = Page::default();
        assert_eq!(p.input(Input::Enter, None), (false, None), "no ballot: the key is not its own");
        let view = s.view(0.0, "en");
        let layer = p.paint(view.as_ref(), SCREEN, FONT).flatten().expect("the ballot");
        assert!(["VOTE", "20 S LEFT", "Shall the cat sing?", "Yes", "No", "Later", "Make your choice."].iter().all(|t| texts(&layer).contains(&t.to_string())), "{:?}", texts(&layer));
        assert_eq!(p.paint(view.as_ref(), SCREEN, FONT), None, "nothing changed");
        // the first key only shows where the keys are; then Down, Down, Up, Enter casts the second
        assert_eq!(p.input(Input::Enter, view.as_ref()), (true, None));
        assert!(p.paint(view.as_ref(), SCREEN, FONT).flatten().is_some(), "the keys' place is painted");
        for i in [Input::Down, Input::Down, Input::Down, Input::Up] {
            assert_eq!(p.input(i, view.as_ref()), (true, None));
        }
        assert_eq!(p.input(Input::Enter, view.as_ref()), (true, Some(Do::Cast(1))));
        // the menu's other inputs are not the ballot's
        for i in [Input::Left, Input::Right, Input::Back, Input::Open, Input::Secondary, Input::Point(400.0, 590.0)] {
            assert_eq!(p.input(i, view.as_ref()), (false, None), "{i:?}");
        }
        // a press on the third option (800 × 600 is two device pixels per logical one) casts it; beside the options but on
        // the ballot it is swallowed, above the ballot it is the scene's
        let (x, y) = p.at(2).expect("the third option's place");
        assert_eq!(p.input(Input::Press(x, y), view.as_ref()), (true, Some(Do::Cast(2))));
        assert_eq!(p.input(Input::Press(400.0, 598.0), view.as_ref()), (true, None));
        assert_eq!(p.input(Input::Press(400.0, 10.0), view.as_ref()), (false, None));
        // with this deck's vote on it the keys start there
        s.cast(2);
        let mut q = Page::default();
        let view = s.view(0.0, "en");
        q.paint(view.as_ref(), SCREEN, FONT);
        q.input(Input::Up, view.as_ref());
        assert_eq!(q.input(Input::Up, view.as_ref()), (true, None));
        assert_eq!(q.input(Input::Enter, view.as_ref()), (true, Some(Do::Cast(1))));
    }

    #[test]
    fn the_result_takes_no_keys_and_the_ballot_goes_from_the_screen() {
        let mut s = State::default();
        s.open(&open("v1", true), 0.0);
        s.cast(0);
        s.result(&result("v1", 0, "rule"), 1000.0);
        let mut p = Page::default();
        let view = s.view(1000.0, "en");
        let layer = p.paint(view.as_ref(), SCREEN, FONT).flatten().expect("the result");
        assert!(["RESULT", "Yes", "Most votes.", "Your vote: Yes"].iter().all(|t| texts(&layer).contains(&t.to_string())), "{:?}", texts(&layer));
        assert_eq!(p.input(Input::Enter, view.as_ref()), (false, None), "nothing to choose");
        assert_eq!(p.input(Input::Press(400.0, 598.0), view.as_ref()), (true, None), "a press on it is not the scene's");
        assert!(p.top().is_some_and(|top| top > 200.0 && top < 600.0));
        let gone = s.view(1000.0 + RESULT_MS, "en");
        assert_eq!(p.paint(gone.as_ref(), SCREEN, FONT), Some(None));
        assert_eq!((p.top(), p.input(Input::Press(400.0, 598.0), gone.as_ref())), (None, (false, None)));
        // the stage made anew: painted again
        let view = s.view(1000.0, "en");
        assert!(p.paint(view.as_ref(), SCREEN, FONT).flatten().is_some());
        p.again();
        assert!(p.paint(view.as_ref(), SCREEN, FONT).flatten().is_some());
    }
}
