//! The vote's conductor side in the deck editor's Perform workspace (#533, research/SHOW.md §14; LIVE.md B.6, the vote
//! #231): what the show runner types to open a vote ([`Form`]) and the panel it watches ([`Panel`]) – the state, the
//! question, the rule and its help, the countdown, a bar per option, per group its split and its pick, how many have voted,
//! the result and how it was decided. Plain data and words, no window: deck-show `vote.rs` fills the panel from what the
//! relay said (the rule is the relay's; a group's pick its `winner`, protocol `vote.rs`) and turns a form into `vote.open`.

/// The rule a vote is decided by (the relay applies it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rule {
    #[default]
    Majority,
    Groups,
    First,
}

/// the rules in the order the form offers them
pub const RULES: [Rule; 3] = [Rule::Majority, Rule::Groups, Rule::First];

impl Rule {
    /// the rule's name on a chip and in the panel
    pub fn name(self) -> &'static str {
        match self {
            Rule::Majority => "majority",
            Rule::Groups => "per group",
            Rule::First => "first to n",
        }
    }

    /// what the rule does, in a sentence
    pub fn help(self) -> &'static str {
        match self {
            Rule::Majority => "The option with the most votes wins.",
            Rule::Groups => "Each group's own majority counts as one vote; the option with the most groups wins.",
            Rule::First => "The first option to reach n votes wins at once.",
        }
    }
}

/// How the result was decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    Rule,
    Fallback,
    Override,
}

impl From<protocol::VoteRule> for Rule {
    fn from(r: protocol::VoteRule) -> Rule {
        match r {
            protocol::VoteRule::Majority => Rule::Majority,
            protocol::VoteRule::Groups => Rule::Groups,
            protocol::VoteRule::First => Rule::First,
        }
    }
}

impl From<protocol::VoteHow> for How {
    fn from(h: protocol::VoteHow) -> How {
        match h {
            protocol::VoteHow::Rule => How::Rule,
            protocol::VoteHow::Fallback => How::Fallback,
            protocol::VoteHow::Override => How::Override,
        }
    }
}

/// How a vote was decided, as a voter reads it on the ballot (#556, ballot.rs) – in German for a `de…` locale, else in
/// English. The conductor's panel says the same in its own line (`Panel::result_line`).
pub fn decided(rule: Rule, how: How, locale: &str) -> &'static str {
    let de = locale.starts_with("de");
    match (how, rule, de) {
        (How::Fallback, _, false) => "Nobody voted – it stays as it was.",
        (How::Fallback, _, true) => "Niemand hat abgestimmt – es bleibt dabei.",
        (How::Override, _, false) => "The conductor decided.",
        (How::Override, _, true) => "Die Regie hat entschieden.",
        (How::Rule, Rule::Majority, false) => "Most votes.",
        (How::Rule, Rule::Majority, true) => "Die meisten Stimmen.",
        (How::Rule, Rule::Groups, false) => "Most groups.",
        (How::Rule, Rule::Groups, true) => "Die meisten Gruppen.",
        (How::Rule, Rule::First, false) => "First to enough votes.",
        (How::Rule, Rule::First, true) => "Zuerst genug Stimmen.",
    }
}

/// Open, held (the countdown stands, votes still count) or closed (the result is in).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Open,
    Held,
    Closed,
}

impl State {
    pub fn word(self) -> &'static str {
        match self {
            State::Open => "Open",
            State::Held => "Held",
            State::Closed => "Closed",
        }
    }
}

/// One option's bar: its label, its votes, its share in whole percent, whether it leads, how full the bar is (0..1).
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub label: String,
    pub count: i64,
    pub percent: i64,
    pub lead: bool,
    pub fill: f32,
}

/// One group under the rule "per group": its name, per option its share in % and its votes, the option it picks.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub name: String,
    pub cells: Vec<(i64, i64)>,
    pub pick: usize,
}

/// The panel while a vote is open and after it closed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Panel {
    /// the vote's id (`v1`)
    pub vote: String,
    pub state: State,
    /// the question in English, else the first given (empty: none)
    pub question: String,
    pub rule: Rule,
    /// rule first: the votes an option needs
    pub n: i64,
    /// the fallback's label (the option when nobody votes)
    pub fallback: String,
    /// who votes: the groups by name, or `all audience`
    pub voting: String,
    /// the options, in order (Override offers them)
    pub options: Vec<String>,
    /// whole seconds left, and the countdown's share of the vote's time (1 = all of it left)
    pub seconds: i64,
    pub countdown: f32,
    pub bars: Vec<Bar>,
    /// rule per group: each group's split and pick, then how many groups picked each option
    pub groups: Vec<Group>,
    pub group_votes: Vec<i64>,
    pub voters: i64,
    /// the connected voting devices the relay counts (none: not said)
    pub expected: Option<i64>,
    /// the result: the winning option's label and how it was decided
    pub result: Option<(String, How)>,
}

/// the share of `c` in `counts`, in whole percent (0 when nobody voted)
pub fn percent(c: i64, counts: &[i64]) -> i64 {
    let total: i64 = counts.iter().sum();
    if total == 0 {
        0
    } else {
        (c as f64 * 100.0 / total as f64).round() as i64
    }
}

impl Panel {
    /// the head: `VOTE v1 · Open`
    pub fn head(&self) -> String {
        format!("VOTE {} · {}", self.vote, self.state.word())
    }

    /// `majority`, `first to n · n 20`
    pub fn rule_line(&self) -> String {
        match self.rule {
            Rule::First => format!("{} · n {}", self.rule.name(), self.n),
            r => r.name().to_string(),
        }
    }

    /// `Rule majority · Fallback No · Voting all audience`
    pub fn meta_line(&self) -> String {
        format!("Rule {} · Fallback {} · Voting {}", self.rule_line(), self.fallback, self.voting)
    }

    /// `Votes · 3 of 5 devices` (`All votes` under the rule per group; the devices only while open)
    pub fn voters_line(&self) -> String {
        let head = if self.rule == Rule::Groups { "All votes" } else { "Votes" };
        match self.expected.filter(|_| self.result.is_none()) {
            Some(e) => format!("{head} · {} of {e} devices", self.voters),
            None => format!("{head} · {}", self.voters),
        }
    }

    /// `Per group · 2 groups, one vote each`
    pub fn groups_line(&self) -> String {
        let n = self.groups.len();
        format!("Per group · {n} {}, one vote each", if n == 1 { "group" } else { "groups" })
    }

    /// `12s`
    pub fn seconds_line(&self) -> String {
        format!("{}s", self.seconds)
    }

    /// `Result: Yes – by the rule (majority)`, none while open
    pub fn result_line(&self) -> Option<String> {
        let (label, how) = self.result.as_ref()?;
        let how = match how {
            How::Rule => format!("by the rule ({})", self.rule.name()),
            How::Fallback => "by the fallback – nobody voted".into(),
            How::Override => "by override – the conductor chose".into(),
        };
        Some(format!("Result: {label} – {how}"))
    }

    /// Hold, Override and Close now are offered while the vote is open or held
    pub fn can_act(&self) -> bool {
        self.result.is_none()
    }

    /// the hold button's word: Hold, or Run on while held
    pub fn hold_word(&self) -> &'static str {
        if self.state == State::Held {
            "Run on"
        } else {
            "Hold"
        }
    }
}

/// what the panel says under the actions
pub const HOLD_HELP: &str = "Hold stops the countdown; votes still count. A held vote does not close when everyone has voted.";
/// what the panel says without a vote
pub const NONE: &str = "No vote open.";

/// What the show runner types to open a vote: the question, the options (separated by `,` – one line in the editor), the
/// rule, n (first to n), the seconds, the fallback (an option's place), the groups that vote (none: the whole audience),
/// whether a voter sees the counts after voting.
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    pub question: String,
    pub options: String,
    pub rule: Rule,
    pub n: i64,
    pub seconds: i64,
    pub fallback: usize,
    pub groups: Vec<String>,
    pub show: bool,
}

impl Default for Form {
    fn default() -> Self {
        Form { question: String::new(), options: "Yes, No".into(), rule: Rule::Majority, n: 20, seconds: 30, fallback: 0, groups: Vec::new(), show: true }
    }
}

/// at most this many options
pub const MAX_OPTIONS: usize = 6;

/// the form's labels and what its fields say while empty (the editor draws them: the question in a cell's inspector, #551)
pub const QUESTION: &str = "Question";
pub const NO_QUESTION: &str = "none – the options alone";
pub const OPTIONS: &str = "Options";
pub const OPTIONS_HINT: &str = "Yes, No – separated by commas";
pub const RULE: &str = "Rule";
pub const N: &str = "n";
pub const SECONDS: &str = "Seconds";
pub const FALLBACK: &str = "Fallback";
pub const COUNTS: &str = "Counts";

/// the toggle's words: does a voter see the counts once they have voted
pub fn counts_word(show: bool) -> &'static str {
    if show {
        "voters see the counts: yes"
    } else {
        "voters see the counts: no"
    }
}

/// What is wrong with a vote's form – `options` options, the rule, n (first to n), the seconds, the fallback (an option's
/// place) – in words; none = it opens. The one place (#546): the typed form and every question of a show (deck-show
/// validate) are checked here.
pub fn problem(options: usize, rule: Rule, n: i64, seconds: i64, fallback: usize) -> Option<&'static str> {
    if options < 2 {
        Some("A vote needs at least two options (separated by commas).")
    } else if options > MAX_OPTIONS {
        Some("At most six options.")
    } else if seconds < 1 {
        Some("Seconds: at least 1.")
    } else if rule == Rule::First && n < 1 {
        Some("First to n: n is at least 1.")
    } else if fallback >= options {
        Some("The fallback is one of the options.")
    } else {
        None
    }
}

impl Form {
    /// the options typed, trimmed, the empty ones left out
    pub fn option_list(&self) -> Vec<String> {
        self.options.split(',').map(str::trim).filter(|o| !o.is_empty()).map(str::to_string).collect()
    }

    /// What is wrong with the form, in words; none = it opens.
    pub fn problem(&self) -> Option<&'static str> {
        problem(self.option_list().len(), self.rule, self.n, self.seconds, self.fallback)
    }

    /// how the vote runs: `Yes, No · majority · 30 s · fallback Yes`
    pub fn run_line(&self) -> String {
        let options = self.option_list();
        let fallback = options.get(self.fallback).cloned().unwrap_or_else(|| "–".into());
        let rule = if self.rule == Rule::First { format!("first to {}", self.n) } else { self.rule.name().to_string() };
        format!("{} · {rule} · {} s · fallback {fallback}", options.join(", "), self.seconds)
    }

    /// `Yes, No · majority · 30 s · fallback Yes · all audience`
    pub fn summary(&self) -> String {
        let voting = if self.groups.is_empty() { "all audience".to_string() } else { self.groups.join(", ") };
        format!("{} · {voting}", self.run_line())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Panel {
        Panel {
            vote: "v1".into(),
            rule: Rule::Majority,
            fallback: "No".into(),
            voting: "all audience".into(),
            options: vec!["Yes".into(), "No".into()],
            voters: 3,
            expected: Some(5),
            ..Panel::default()
        }
    }

    #[test]
    fn the_panel_says_the_state_the_rule_the_voters_and_how_it_was_decided() {
        let mut p = panel();
        assert_eq!(p.head(), "VOTE v1 · Open");
        assert_eq!(p.meta_line(), "Rule majority · Fallback No · Voting all audience");
        assert_eq!(p.voters_line(), "Votes · 3 of 5 devices");
        assert_eq!((p.hold_word(), p.can_act(), p.result_line()), ("Hold", true, None));
        p.state = State::Held;
        assert_eq!(p.hold_word(), "Run on");
        p.rule = Rule::First;
        p.n = 20;
        assert_eq!(p.rule_line(), "first to n · n 20");
        p.rule = Rule::Groups;
        p.groups = vec![Group { name: "left".into(), cells: vec![(50, 1), (50, 1)], pick: 1 }];
        assert_eq!((p.voters_line().as_str(), p.groups_line().as_str()), ("All votes · 3 of 5 devices", "Per group · 1 group, one vote each"));
        p.state = State::Closed;
        p.result = Some(("No".into(), How::Rule));
        assert_eq!(p.result_line().as_deref(), Some("Result: No – by the rule (per group)"));
        assert_eq!(p.voters_line(), "All votes · 3", "the devices only while open");
        assert!(!p.can_act());
        p.result = Some(("Yes".into(), How::Override));
        assert_eq!(p.result_line().as_deref(), Some("Result: Yes – by override – the conductor chose"));
        p.result = Some(("No".into(), How::Fallback));
        assert_eq!(p.result_line().as_deref(), Some("Result: No – by the fallback – nobody voted"));
    }

    #[test]
    fn a_voter_reads_how_it_was_decided_in_the_decks_language() {
        assert_eq!(decided(Rule::Groups, How::Rule, "en"), "Most groups.");
        assert_eq!(decided(Rule::First, How::Rule, "de-AT"), "Zuerst genug Stimmen.");
        assert_eq!(decided(protocol::VoteRule::Majority.into(), protocol::VoteHow::Fallback.into(), "de"), "Niemand hat abgestimmt – es bleibt dabei.");
        assert_eq!(decided(Rule::Majority, How::Override, "fr"), "The conductor decided.");
    }

    #[test]
    fn shares_are_whole_percent_of_the_votes() {
        assert_eq!((percent(1, &[1, 2]), percent(2, &[1, 2]), percent(0, &[0, 0])), (33, 67, 0));
    }

    #[test]
    fn the_form_says_what_is_wrong_before_it_opens() {
        let mut f = Form::default();
        assert_eq!((f.problem(), f.option_list()), (None, vec!["Yes".to_string(), "No".to_string()]));
        assert_eq!(f.summary(), "Yes, No · majority · 30 s · fallback Yes · all audience");
        f.options = "Yes, ,".into();
        assert_eq!(f.problem(), Some("A vote needs at least two options (separated by commas)."));
        f.options = "a,b,c,d,e,f,g".into();
        assert_eq!(f.problem(), Some("At most six options."));
        f.options = "left, right, stay".into();
        f.seconds = 0;
        assert_eq!(f.problem(), Some("Seconds: at least 1."));
        f.seconds = 20;
        f.rule = Rule::First;
        f.n = 0;
        assert_eq!(f.problem(), Some("First to n: n is at least 1."));
        f.n = 3;
        f.fallback = 3;
        assert_eq!(f.problem(), Some("The fallback is one of the options."));
        f.fallback = 2;
        f.groups = vec!["left".into(), "balcony".into()];
        assert_eq!((f.problem(), f.summary().as_str()), (None, "left, right, stay · first to 3 · 20 s · fallback stay · left, balcony"));
    }
}
