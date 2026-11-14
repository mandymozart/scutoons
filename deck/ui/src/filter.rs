//! One condition, one look (#546, research/SHOW.md §15 2b; owner 2026-11-14: row and cell conditions "very similar and
//! perhaps share a UI"): the facts a filter may name, which of them are on, and the words – for a role's `for` (any one
//! fact admits, `Admits`), a row address's `where` and a show cell's column filter (every fact holds, `Filter`). Moved here
//! from deck-show roles.rs (`filter_words`, its `Fact` and toggles) so the role filter, the row addresses and the cell
//! filter have one model and one set of words. #553: the one control's model is [`Condition`] (its [`Mode`]: any one fact
//! for a role, every fact for an address or a cell), drawn by the editor's `show::condition::draw` on deck-kit's chips.
//! Plain data and words; the rule that tests a filter is protocol `filter::passes`, the role admission the relay's.
use protocol::{Account, Admits, Filter};

/// the filter that lets anyone with the code in
pub const GUEST_AND_UP: &str = "guest and up";
/// an approved (paired) deck, as a filter says it
pub const APPROVED: &str = "approved deck";
/// a cell's or an address's filter that asks nothing
pub const EVERYONE: &str = "everyone";
/// a filter that lets nobody through
pub const NOBODY: &str = "nobody";

/// the account facts in the order a filter offers them
pub const ACCOUNTS: [Account; 5] = [Account::Guest, Account::Member, Account::Creator, Account::Admin, Account::Live];

/// The device kinds a filter offers (proposal, no owner word on the list: the words a deck's `hello.device` and its flash
/// report use, `phone` from #549; a kind a filter already names is offered too).
pub const DEVICES: [&str; 3] = ["native", "browser", "phone"];

/// One fact a filter may name (owner 2026-11-10 [role]: guest, member, creator, admin, live, an approved deck, a device, a
/// group).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fact {
    Account(Account),
    Approved,
    Device(String),
    Group(String),
}

impl Fact {
    /// as a chip and the filter's words say it
    pub fn word(&self) -> String {
        match self {
            Fact::Account(a) => a.as_str().to_string(),
            Fact::Approved => APPROVED.to_string(),
            Fact::Device(d) => format!("device {d}"),
            Fact::Group(g) => format!("group {g}"),
        }
    }

    /// does a role's filter name it?
    pub fn in_admits(&self, a: &Admits) -> bool {
        match self {
            Fact::Account(x) => a.account.iter().flatten().any(|y| y == x),
            Fact::Approved => a.approved == Some(true),
            Fact::Device(d) => a.device.iter().flatten().any(|y| y == d),
            Fact::Group(g) => a.group.iter().flatten().any(|y| y == g),
        }
    }

    /// on becomes off, off on, in a role's filter; an emptied list goes
    pub fn flip_admits(&self, a: &mut Admits) {
        match self {
            Fact::Account(x) => flip_in(&mut a.account, x),
            Fact::Approved => a.approved = (a.approved != Some(true)).then_some(true),
            Fact::Device(d) => flip_in(&mut a.device, d),
            Fact::Group(g) => flip_in(&mut a.group, g),
        }
    }

    /// does a cell's or an address's filter name it?
    pub fn in_filter(&self, f: &Filter) -> bool {
        match self {
            Fact::Account(x) => f.account.iter().flatten().any(|y| y == x),
            Fact::Approved => f.approved == Some(true),
            Fact::Device(d) => f.device.as_ref() == Some(d),
            Fact::Group(g) => f.group.as_ref() == Some(g),
        }
    }

    /// on becomes off, off on, in a cell's or an address's filter: a device or a group is one value (another replaces it),
    /// an emptied account list goes
    pub fn flip_filter(&self, f: &mut Filter) {
        let one = |slot: &mut Option<String>, v: &String| *slot = (slot.as_ref() != Some(v)).then(|| v.clone());
        match self {
            Fact::Account(x) => flip_in(&mut f.account, x),
            Fact::Approved => f.approved = (f.approved != Some(true)).then_some(true),
            Fact::Device(d) => one(&mut f.device, d),
            Fact::Group(g) => one(&mut f.group, g),
        }
    }
}

fn flip_in<T: PartialEq + Clone>(slot: &mut Option<Vec<T>>, value: &T) {
    let list = slot.get_or_insert_with(Vec::new);
    if list.contains(value) {
        list.retain(|y| y != value);
    } else {
        list.push(value.clone());
    }
    if list.is_empty() {
        *slot = None;
    }
}

/// One chip of a filter: the fact, its word, whether it is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toggle {
    pub fact: Fact,
    pub word: String,
    pub on: bool,
}

/// facts in words, ` · ` between (the one join: a role's any, a filter's all)
pub fn words(facts: &[Fact]) -> String {
    facts.iter().map(Fact::word).collect::<Vec<_>>().join(" · ")
}

/// Who may take a role, in words. No filter, or one that lets a guest in, is `guest and up`: anyone with the code. Any other
/// fact is said as it is (any one admits). Proposal (no owner word yet, the open question of #478 slice 1/2): the account
/// facts are read exactly as the relay reads them, not as a ladder, so `member` says `member`.
pub fn admits_words(a: Option<&Admits>) -> String {
    let Some(a) = a else { return GUEST_AND_UP.into() };
    if a.account.iter().flatten().any(|x| *x == Account::Guest) {
        return GUEST_AND_UP.into();
    }
    let mut facts: Vec<Fact> = a.account.iter().flatten().map(|x| Fact::Account(*x)).collect();
    if a.approved == Some(true) {
        facts.push(Fact::Approved);
    }
    facts.extend(a.device.iter().flatten().cloned().map(Fact::Device));
    facts.extend(a.group.iter().flatten().cloned().map(Fact::Group));
    if facts.is_empty() {
        return NOBODY.into();
    }
    words(&facts)
}

/// The facts a cell's or an address's filter names, in the order its words say them: the group, the device, the accounts
/// (none when one is `guest`: it asks nothing), an approved deck.
pub fn filter_facts(f: &Filter) -> Vec<Fact> {
    let mut facts: Vec<Fact> = f.group.iter().cloned().map(Fact::Group).collect();
    facts.extend(f.device.iter().cloned().map(Fact::Device));
    if !f.account.iter().flatten().any(|x| *x == Account::Guest) {
        facts.extend(f.account.iter().flatten().map(|x| Fact::Account(*x)));
    }
    if f.approved == Some(true) {
        facts.push(Fact::Approved);
    }
    facts
}

/// Who a cell's or an address's filter lets through, in words (every fact holds): none, or one that asks nothing, is
/// `everyone`; an empty account list `nobody`.
pub fn filter_words(f: Option<&Filter>) -> String {
    let Some(f) = f else { return EVERYONE.into() };
    if f.account.as_ref().is_some_and(Vec::is_empty) {
        return NOBODY.into();
    }
    let facts = filter_facts(f);
    if facts.is_empty() {
        return EVERYONE.into();
    }
    words(&facts)
}

/// a filter that asks nothing (it goes from the document)
pub fn is_empty(f: &Filter) -> bool {
    f == &Filter::default()
}

/// A role without a filter as its chips show it: a guest may take it, so anyone with the code.
pub fn open_admits() -> Admits {
    Admits { account: Some(vec![Account::Guest]), ..Admits::default() }
}

/// How a condition's facts combine – the one control's mode, not a second control (#553).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// any one fact admits: a role's `for` (`Admits`, tested by the relay)
    Any,
    /// every fact holds: a row address's and a show cell's `where` (`Filter`, protocol `filter::passes`)
    All,
}

/// The one condition control (#553, SHOW.md §15 2b): what the editor draws for a role's admission, a row address's filter
/// and a show cell's column filter – the mode, the filter in words, its chips in the order drawn. Built by
/// [`Condition::admits`] or [`Condition::filter`]; a chip pressed is [`Condition::fact`], flipped in the document by
/// `Fact::flip_admits` / `Fact::flip_filter` (deck-show's edits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    pub mode: Mode,
    /// `guest and up`, `everyone`, `device phone · member`
    pub words: String,
    pub toggles: Vec<Toggle>,
}

/// The facts offered, in the order drawn: the account facts, an approved deck, each device kind (and any other `named`),
/// then – when there are groups (`None`: the column is already the group) – each group offered and any other `named`.
fn offered(devices: &[String], groups: Option<(&[String], &[String])>) -> Vec<Fact> {
    let mut facts: Vec<Fact> = ACCOUNTS.iter().map(|x| Fact::Account(*x)).collect();
    facts.push(Fact::Approved);
    facts.extend(DEVICES.iter().map(|d| Fact::Device(d.to_string())));
    facts.extend(devices.iter().filter(|d| !DEVICES.contains(&d.as_str())).cloned().map(Fact::Device));
    if let Some((offer, named)) = groups {
        facts.extend(offer.iter().cloned().map(Fact::Group));
        facts.extend(named.iter().filter(|g| !offer.contains(g)).cloned().map(Fact::Group));
    }
    facts
}

impl Condition {
    /// A role's admission (any one fact): none is open to anyone with the code ([`open_admits`]); `groups` the groups the
    /// show offers (its stage outlets).
    pub fn admits(a: Option<&Admits>, groups: &[String]) -> Condition {
        let set = a.cloned().unwrap_or_else(open_admits);
        let devices: Vec<String> = set.device.iter().flatten().cloned().collect();
        let named: Vec<String> = set.group.iter().flatten().cloned().collect();
        let toggles = offered(&devices, Some((groups, &named))).into_iter().map(|fact| Toggle { word: fact.word(), on: fact.in_admits(&set), fact }).collect();
        Condition { mode: Mode::Any, words: admits_words(a), toggles }
    }

    /// A row address's or a show cell's filter (every fact holds). `groups`: the groups the show offers for an address; none
    /// for a cell – its column is already the group (SHOW.md §15 2b), so no group chip.
    pub fn filter(f: Option<&Filter>, groups: Option<&[String]>) -> Condition {
        let set = f.cloned().unwrap_or_default();
        let devices: Vec<String> = set.device.iter().cloned().collect();
        let named: Vec<String> = set.group.iter().cloned().collect();
        let toggles = offered(&devices, groups.map(|g| (g, named.as_slice()))).into_iter().map(|fact| Toggle { word: fact.word(), on: fact.in_filter(&set), fact }).collect();
        Condition { mode: Mode::All, words: filter_words(f), toggles }
    }

    /// the fact of chip `i`
    pub fn fact(&self, i: usize) -> Option<Fact> {
        self.toggles.get(i).map(|t| t.fact.clone())
    }

    /// the chip of `word` (`device phone`, `member`, `group left`), as a check run names it
    pub fn chip(&self, word: &str) -> Option<usize> {
        self.toggles.iter().position(|t| t.word == word)
    }

    /// the words drawn on the chips, and which are on
    pub fn chips(&self) -> (Vec<&str>, Vec<bool>) {
        (self.toggles.iter().map(|t| t.word.as_str()).collect(), self.toggles.iter().map(|t| t.on).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(json: &str) -> Admits {
        serde_json::from_str(json).unwrap()
    }

    fn f(json: &str) -> Filter {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_roles_filter_in_words() {
        assert_eq!(admits_words(None), "guest and up");
        assert_eq!(admits_words(Some(&a(r#"{ "account": ["guest", "member"] }"#))), "guest and up", "a guest lets everyone in");
        assert_eq!(admits_words(Some(&a(r#"{ "account": ["member"] }"#))), "member");
        assert_eq!(admits_words(Some(&a(r#"{ "account": ["live"], "approved": true }"#))), "live · approved deck");
        assert_eq!(admits_words(Some(&a(r#"{ "device": ["phone"], "group": ["foyer"] }"#))), "device phone · group foyer");
        assert_eq!(admits_words(Some(&a("{}"))), "nobody");
    }

    #[test]
    fn a_cells_filter_in_words_and_its_chips_without_a_group() {
        assert_eq!(filter_words(None), "everyone");
        assert_eq!(filter_words(Some(&f("{}"))), "everyone");
        assert_eq!(filter_words(Some(&f(r#"{ "group": "left", "device": "browser" }"#))), "group left · device browser", "as an address's where said it");
        assert_eq!(filter_words(Some(&f(r#"{ "device": "phone", "account": ["live", "member"], "approved": true }"#))), "device phone · live · member · approved deck");
        assert_eq!(filter_words(Some(&f(r#"{ "account": ["guest"] }"#))), "everyone", "a guest asks nothing");
        assert_eq!(filter_words(Some(&f(r#"{ "account": [] }"#))), "nobody");
        let cell = Condition::filter(Some(&f(r#"{ "device": "phone", "account": ["member"] }"#)), None);
        assert_eq!((cell.mode, cell.words.as_str()), (Mode::All, "device phone · member"));
        let on: Vec<&str> = cell.toggles.iter().filter(|t| t.on).map(|t| t.word.as_str()).collect();
        assert_eq!(on, ["member", "device phone"]);
        assert!(cell.toggles.iter().all(|t| !matches!(t.fact, Fact::Group(_))), "the column is the group");
        assert_eq!(cell.toggles.len(), ACCOUNTS.len() + 1 + DEVICES.len());
    }

    /// #553: one control, the mode says how the facts combine; the same chips for a role, an address and a cell
    #[test]
    fn one_condition_for_a_role_an_address_and_a_cell() {
        let groups = ["left".to_string(), "right".to_string()];
        let role = Condition::admits(None, &groups);
        assert_eq!((role.mode, role.words.as_str()), (Mode::Any, "guest and up"));
        assert!(role.toggles[role.chip("guest").unwrap()].on, "no filter: a guest may take it");
        let named = Condition::admits(Some(&a(r#"{ "device": ["pad"], "group": ["foyer"] }"#)), &groups);
        let words: Vec<&str> = named.toggles.iter().map(|t| t.word.as_str()).collect();
        assert_eq!(words[5..], ["approved deck", "device native", "device browser", "device phone", "device pad", "group left", "group right", "group foyer"], "a fact it names is offered too");
        let address = Condition::filter(Some(&f(r#"{ "group": "left" }"#)), Some(&groups));
        assert_eq!((address.mode, address.words.as_str()), (Mode::All, "group left"));
        assert!(address.toggles[address.chip("group left").unwrap()].on && !address.toggles[address.chip("group right").unwrap()].on);
        let cell = Condition::filter(None, None);
        assert_eq!(cell.words, "everyone");
        assert_eq!(cell.chip("group left"), None);
        assert_eq!(cell.fact(cell.chip("device phone").unwrap()), Some(Fact::Device("phone".into())));
        let (names, on) = address.chips();
        assert_eq!((names.len(), on.iter().filter(|x| **x).count()), (address.toggles.len(), 1));
    }

    #[test]
    fn a_chip_flips_its_fact() {
        let mut x = Filter::default();
        Fact::Device("phone".into()).flip_filter(&mut x);
        Fact::Device("browser".into()).flip_filter(&mut x);
        assert_eq!(x.device.as_deref(), Some("browser"), "one device: another replaces it");
        Fact::Account(Account::Member).flip_filter(&mut x);
        Fact::Approved.flip_filter(&mut x);
        assert_eq!(filter_words(Some(&x)), "device browser · member · approved deck");
        for fact in [Fact::Device("browser".into()), Fact::Account(Account::Member), Fact::Approved] {
            fact.flip_filter(&mut x);
        }
        assert!(is_empty(&x), "every chip off: the filter asks nothing");
        let mut r = Admits::default();
        Fact::Group("foyer".into()).flip_admits(&mut r);
        assert!(Fact::Group("foyer".into()).in_admits(&r));
        Fact::Group("foyer".into()).flip_admits(&mut r);
        assert_eq!(r, Admits::default(), "an emptied list goes");
    }
}
