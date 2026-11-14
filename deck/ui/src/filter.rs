//! One condition, one look (#546, research/SHOW.md §15 2b; owner 2026-11-14: row and cell conditions "very similar and
//! perhaps share a UI"): the facts a filter may name, which of them are on, and the words – for a role's `for` (any one
//! fact admits, `Admits`), a row address's `where` and a show cell's column filter (every fact holds, `Filter`). Moved here
//! from deck-show roles.rs (`filter_words`, its `Fact` and toggles) so the role filter, the cell filter (#551 draws it) and
//! later the row addresses (#553) have one model and one set of words; drawn with deck-kit's chips. Plain data and
//! words; the rule that tests a filter is protocol `filter::passes`, the role admission the relay's.
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

/// The chips of a show cell's column filter: the account facts, an approved deck, each device kind (and one it names). No
/// group: the column is already the group (SHOW.md §15 2b).
pub fn cell_toggles(f: Option<&Filter>) -> Vec<Toggle> {
    let f = f.cloned().unwrap_or_default();
    let mut facts: Vec<Fact> = ACCOUNTS.iter().map(|x| Fact::Account(*x)).collect();
    facts.push(Fact::Approved);
    facts.extend(DEVICES.iter().map(|d| Fact::Device(d.to_string())));
    facts.extend(f.device.iter().filter(|d| !DEVICES.contains(&d.as_str())).cloned().map(Fact::Device));
    facts.into_iter().map(|fact| Toggle { word: fact.word(), on: fact.in_filter(&f), fact }).collect()
}

/// a filter that asks nothing (it goes from the document)
pub fn is_empty(f: &Filter) -> bool {
    f == &Filter::default()
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
        let chips = cell_toggles(Some(&f(r#"{ "device": "phone", "account": ["member"] }"#)));
        let on: Vec<&str> = chips.iter().filter(|t| t.on).map(|t| t.word.as_str()).collect();
        assert_eq!(on, ["member", "device phone"]);
        assert!(chips.iter().all(|t| !matches!(t.fact, Fact::Group(_))), "the column is the group");
        assert_eq!(chips.len(), ACCOUNTS.len() + 1 + DEVICES.len());
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
