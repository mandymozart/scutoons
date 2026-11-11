//! The conductor's roles in the deck editor's Show workspace (#478 slice 3, owner 2026-11-10 [role], draft 4): every
//! conductor defines its roles – a role is a stage outlet of its show with its permission filter (`for`) – and sees who is
//! here. The tree lists the roles with how many are in each (the relay's count, the `roles` it sends the conductor), the
//! matrix's column header says the role's name, its filter in words and the count. A role is added, renamed, removed and its
//! filter set here ([`edit`]: the document changed, checked by `validate`, a refused edit says why); the inspector's words
//! and the prompt before a remove too; who is here (`role: name`, the device, why it sits there) and what move to, rename
//! and kick send. The words are here once; the editor lays them out (RULES A1, A2).
use crate::choose::{Prompt, Tone};
use crate::document::{Cell, Outlet, OutletKind, Show};
use crate::subtree::Subtree;
use crate::view::{cell_text, Section};
use protocol::{Account, Admits};

/// One role as the Show workspace shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleView {
    /// the stage outlet's id
    pub id: String,
    /// its name for people (the label, else the id)
    pub name: String,
    /// who may take it, in words: `guest and up`, `member`, `live · approved deck`
    pub filter: String,
    /// how many sit in it, as the relay counts (none: no session, or the relay has not counted it yet)
    pub here: Option<u64>,
}

/// Who may take a role, in words. No filter, or one that lets a guest in, is `guest and up`: anyone with the code. Any other
/// fact is said as it is, joined by ` · ` (any one admits). Proposal (no owner word yet, the open question of the slice 1/2
/// report): the account facts are read exactly as the relay reads them, not as a ladder, so `member` says `member`.
pub fn filter_words(a: Option<&Admits>) -> String {
    let Some(a) = a else { return GUEST_AND_UP.into() };
    let accounts = a.account.iter().flatten();
    if accounts.clone().any(|x| *x == Account::Guest) {
        return GUEST_AND_UP.into();
    }
    let mut words: Vec<String> = accounts.map(|x| x.as_str().to_string()).collect();
    if a.approved == Some(true) {
        words.push(APPROVED.into());
    }
    words.extend(a.device.iter().flatten().map(|d| format!("device {d}")));
    words.extend(a.group.iter().flatten().map(|g| format!("group {g}")));
    if words.is_empty() {
        return "nobody".into();
    }
    words.join(" · ")
}

/// the filter that lets anyone with the code in
pub const GUEST_AND_UP: &str = "guest and up";
/// an approved (paired) deck, as the filter says it
pub const APPROVED: &str = "approved deck";
/// the tree's line that adds a role
pub const ADD: &str = "new role";

/// The roles of `show`, in its order: every stage outlet. `sub`, when it runs this show in a session: how many are in each.
pub fn roles(show: &Show, sub: Option<&Subtree>) -> Vec<RoleView> {
    show.outlets
        .iter()
        .filter(|o| o.kind == OutletKind::Stage)
        .map(|o| RoleView { id: o.id.clone(), name: o.name().to_string(), filter: filter_words(o.admits.as_ref()), here: sub.and_then(|s| s.here_count(&o.id)) })
        .collect()
}

/// how many are here, as a column header and the tree say it: the count, or nothing outside a session
pub fn here_text(here: Option<u64>) -> String {
    here.map_or_else(String::new, |n| n.to_string())
}

/// One fact a role's filter may ask (owner 2026-11-10 [role]: guest, member, creator, admin, live, an approved deck, a
/// device, a group); any one admits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fact {
    Account(Account),
    Approved,
    Device(String),
    Group(String),
}

impl Fact {
    /// as a toggle says it
    pub fn word(&self) -> String {
        match self {
            Fact::Account(a) => a.as_str().to_string(),
            Fact::Approved => APPROVED.to_string(),
            Fact::Device(d) => format!("device {d}"),
            Fact::Group(g) => format!("group {g}"),
        }
    }

    fn is_in(&self, a: &Admits) -> bool {
        match self {
            Fact::Account(x) => a.account.iter().flatten().any(|y| y == x),
            Fact::Approved => a.approved == Some(true),
            Fact::Device(d) => a.device.iter().flatten().any(|y| y == d),
            Fact::Group(g) => a.group.iter().flatten().any(|y| y == g),
        }
    }

    /// on becomes off, off on; an emptied list goes
    fn flip(&self, a: &mut Admits) {
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
        match self {
            Fact::Account(x) => flip_in(&mut a.account, x),
            Fact::Approved => a.approved = (a.approved != Some(true)).then_some(true),
            Fact::Device(d) => flip_in(&mut a.device, d),
            Fact::Group(g) => flip_in(&mut a.group, g),
        }
    }
}

/// the account facts in the order the inspector offers them
pub const ACCOUNTS: [Account; 5] = [Account::Guest, Account::Member, Account::Creator, Account::Admin, Account::Live];

/// The device kinds a filter offers (proposal, no owner word on the list: the words a deck's `hello.device` and its flash
/// report use; a kind a filter already names is offered too).
pub const DEVICES: [&str; 3] = ["native", "browser", "phone"];

/// One toggle of a role's filter in the inspector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toggle {
    pub fact: Fact,
    pub word: String,
    pub on: bool,
}

/// a role without a filter as the toggles show it: a guest may take it, so anyone with the code
fn as_set(a: Option<&Admits>) -> Admits {
    a.cloned().unwrap_or_else(|| Admits { account: Some(vec![Account::Guest]), ..Admits::default() })
}

/// The toggles of role `id`'s filter: the account facts, an approved deck, each device kind, each group (proposal: the
/// groups offered are the show's stage outlets – a deck joins with its configured outlet as its group – and any group the
/// filter already names).
pub fn toggles(show: &Show, id: &str) -> Vec<Toggle> {
    let Some(o) = show.outlet(id) else { return vec![] };
    let a = as_set(o.admits.as_ref());
    let mut facts: Vec<Fact> = ACCOUNTS.iter().map(|x| Fact::Account(*x)).collect();
    facts.push(Fact::Approved);
    let mut devices: Vec<String> = DEVICES.iter().map(|d| d.to_string()).collect();
    devices.extend(a.device.iter().flatten().filter(|d| !DEVICES.contains(&d.as_str())).cloned());
    facts.extend(devices.into_iter().map(Fact::Device));
    let mut groups: Vec<String> = show.outlets.iter().filter(|o| o.kind == OutletKind::Stage).map(|o| o.id.clone()).collect();
    let named: Vec<String> = a.group.iter().flatten().filter(|g| !groups.contains(g)).cloned().collect();
    groups.extend(named);
    facts.extend(groups.into_iter().map(Fact::Group));
    facts.into_iter().map(|f| Toggle { word: f.word(), on: f.is_in(&a), fact: f }).collect()
}

/// A change to the conductor's roles, made in the Show workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// a new role after the last one, open to anyone with the code
    Add,
    /// its name for people (empty: the id again)
    Label { id: String, label: String },
    /// the role and its cells go
    Remove { id: String },
    /// one fact of its filter on or off
    Fact { id: String, fact: Fact },
}

/// An edit made: the changed document, what it did in words, the role to select after it.
#[derive(Debug, Clone, PartialEq)]
pub struct Edited {
    pub show: Show,
    pub said: String,
    pub role: Option<String>,
}

/// the index of role `id` (a stage outlet) in `show`
fn role_at(show: &Show, id: &str) -> Result<usize, String> {
    show.outlets.iter().position(|o| o.id == id && o.kind == OutletKind::Stage).ok_or_else(|| format!("no role '{id}'"))
}

/// Make `e` on `show`. The document is checked after it (`validate`): an edit that adds a problem is refused, with the
/// problem's words; problems the document had before stay its own (the problems list says them).
pub fn edit(show: &Show, e: &Edit) -> Result<Edited, String> {
    let before = crate::validate::validate(show);
    let mut next = show.clone();
    let (said, role) = match e {
        Edit::Add => {
            let free = |id: &String| show.outlet(id).is_none() && show.seat(id).is_none();
            let id = (1..).map(|n| if n == 1 { "role".to_string() } else { format!("role-{n}") }).find(free).unwrap_or_default();
            let at = next.outlets.iter().rposition(|o| o.kind == OutletKind::Stage).map_or(0, |i| i + 1);
            next.outlets.insert(at, Outlet { id: id.clone(), kind: OutletKind::Stage, label: None, admits: None });
            (format!("role {id} added: {GUEST_AND_UP} may take it"), Some(id))
        }
        Edit::Label { id, label } => {
            let i = role_at(&next, id)?;
            let label = label.trim();
            next.outlets[i].label = (!label.is_empty() && label != id).then(|| label.to_string());
            (format!("role {id} is called {}", next.outlets[i].name()), Some(id.clone()))
        }
        Edit::Remove { id } => {
            let i = role_at(&next, id)?;
            let name = next.outlets.remove(i).name().to_string();
            let mut cells = 0;
            for r in &mut next.rows {
                cells += usize::from(r.cells.remove(id).is_some_and(|c| !matches!(c, Cell::Keep)));
            }
            (format!("role {name} removed with its {cells} cells"), None)
        }
        Edit::Fact { id, fact } => {
            let i = role_at(&next, id)?;
            let mut a = as_set(next.outlets[i].admits.as_ref());
            fact.flip(&mut a);
            // only the guest left: no filter, anyone with the code
            next.outlets[i].admits = (a != as_set(None)).then_some(a);
            (format!("{}: {}", next.outlets[i].name(), filter_words(next.outlets[i].admits.as_ref())), Some(id.clone()))
        }
    };
    let added: Vec<String> = crate::validate::validate(&next).into_iter().filter(|p| !before.contains(p)).map(|p| p.to_string()).collect();
    match added.first() {
        Some(p) => Err(format!("not changed: {p}")),
        None => Ok(Edited { show: next, said, role }),
    }
}

/// the cells of role `id`, a line each: (`2 kyrie`, `scene:kyrie`)
fn scenes(show: &Show, id: &str) -> Vec<(String, String)> {
    let rows = show.rows.iter().enumerate();
    rows.filter_map(|(i, r)| r.cells.get(id).filter(|c| !matches!(c, Cell::Keep)).map(|c| (format!("{} {}", i + 1, r.id), cell_text(c)))).collect()
}

/// The prompt before a role is removed: what goes with it (its cells), who sits in it.
pub fn remove_prompt(show: &Show, id: &str, here: Option<u64>) -> Prompt {
    let name = show.outlet(id).map_or(id, |o| o.name());
    let cells = scenes(show, id);
    let mut lines: Vec<(String, Tone)> = cells.iter().map(|(row, cell)| (format!("row {row}: {cell} goes"), Tone::Warn)).collect();
    if cells.is_empty() {
        lines.push(("it has no cells".into(), Tone::Muted));
    }
    if let Some(n) = here.filter(|n| *n > 0) {
        lines.push((format!("{n} sit in it now: they stay under its name until they take another role or are moved"), Tone::Warn));
    }
    lines.push(("The document changes here; Save keeps it.".into(), Tone::Muted));
    Prompt { title: format!("Remove the role {name}?"), warn: !cells.is_empty(), lines, confirm: "Remove".into(), can: true }
}

/// The inspector of a selected role: its name (the field), who may enter (the toggles), then its sections.
#[derive(Debug, Clone, PartialEq)]
pub struct Inspector {
    pub title: String,
    /// the name's label and value
    pub name: (String, String),
    /// `who may enter` and the filter in words
    pub enter: (String, String),
    pub toggles: Vec<Toggle>,
    /// here now, its scenes, defined by
    pub sections: Vec<Section>,
    /// the remove button's word
    pub remove: String,
}

/// The inspector of role `id` in `show`; `sub` when it runs this show in a session (how many are here).
pub fn inspector(show: &Show, sub: Option<&Subtree>, id: &str) -> Option<Inspector> {
    let o = show.outlet(id).filter(|o| o.kind == OutletKind::Stage)?;
    let here = sub.and_then(|s| s.here_count(id)).map_or("no session".to_string(), |n| n.to_string());
    let mut scenes = scenes(show, id);
    if scenes.is_empty() {
        scenes.push((String::new(), "no cells yet".into()));
    }
    Some(Inspector {
        title: format!("role · {}", o.id),
        name: ("name".into(), o.name().to_string()),
        enter: ("who may enter".into(), filter_words(o.admits.as_ref())),
        toggles: toggles(show, id),
        sections: vec![
            ("here".into(), vec![("here now".into(), here)]),
            ("its scenes".into(), scenes),
            ("defined by".into(), vec![("conductor".into(), show.id.clone())]),
        ],
        remove: "Remove role …".into(),
    })
}

/// One client in "who is here" (owner 2026-11-10 [role]: the conductor sees `role: name`; draft 4: with the device, what it
/// says of itself and why it sits there).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Here {
    pub client: String,
    /// the role it sits in (an outlet id); none = no role
    pub role: Option<String>,
    /// `tenor: Anna`, `no role: phone-7`
    pub line: String,
    /// what it says of itself: its device, its account fact (`browser · guest`)
    pub stated: String,
    /// why it sits there: `chose itself`, `moved here by this conductor`, `set by a conductor above · locked`
    pub why: String,
    /// held there by a lock
    pub locked: bool,
    /// the lock is from above, not this conductor's: no move is offered
    pub locked_above: bool,
}

/// the move action's word
pub const MOVE_TO: &str = "move to …";
pub const RENAME: &str = "rename";
pub const KICK: &str = "kick";
/// where a deck locked from above offers no move
pub const LOCKED: &str = "locked";

/// Who is here, as the conductor that runs `sub` sees it from presence: every deck and every phone in a role, by role
/// (the show's order, then no role) and name. The account fact is the strongest one the relay found proven (presence's
/// `account`, #478 slice 4); none is `guest` (proposal: said so rather than left out).
pub fn here(sub: &Subtree) -> Vec<Here> {
    let show = sub.conductor().show();
    let mut out: Vec<(usize, Here)> = sub
        .who()
        .iter()
        .map(|(id, p)| {
            let role = p.group.clone().filter(|g| !g.is_empty());
            let at = role.as_deref().and_then(|g| show.outlets.iter().position(|o| o.id == g));
            let name = at.map_or_else(|| role.clone(), |i| Some(show.outlets[i].name().to_string()));
            let line = deck_ui::live::role_name(name.as_deref(), p.nick.as_deref().unwrap_or(id));
            let locked = p.locked == Some(true);
            let mine = sub.moved(id).filter(|(g, _)| *g == role);
            let (why, above) = match (locked, mine, &role) {
                (true, Some((_, true)), _) => ("moved here by this conductor · locked", false),
                // proposal: a lock this conductor did not set is from above (the uplink's assign; the cascade of [venue])
                (true, _, _) => ("set by a conductor above · locked", true),
                (false, Some(_), _) => ("moved here by this conductor", false),
                (false, None, None) => ("no role · the idle scene", false),
                (false, None, Some(_)) => ("chose itself", false),
            };
            let stated = format!("{} · {}", p.device.as_deref().unwrap_or("deck"), p.account.unwrap_or(Account::Guest).as_str());
            (at.unwrap_or(usize::MAX), Here { client: id.clone(), role, line, stated, why: why.into(), locked, locked_above: above })
        })
        .collect();
    out.sort_by(|a, b| (a.0, &a.1.line).cmp(&(b.0, &b.1.line)));
    out.into_iter().map(|(_, h)| h).collect()
}

/// the heading of "who is here"
pub fn here_heading(n: usize) -> String {
    format!("WHO IS HERE – {n} · MOVE TO IS THE RELAY'S ASSIGN; A LOCK FROM ABOVE HOLDS A DECK")
}

/// what "who is here" says while nobody is
pub const NOBODY: &str = "Nobody is here yet: decks, and phones in a role, show up as they join.";

/// One line of the move menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    /// into this role, none = no role
    Role(Option<String>),
    /// the lock for the move: on, the conductor holds it there
    Lock,
}

/// The move menu for `client`: every role of the show (proposal: the conductor's move is not held to the role's filter –
/// a lock from above wins, the relay does not check the conductor's assign), then no role, then the lock; ticked: where it
/// sits, and whether the move locks.
pub fn move_menu(sub: &Subtree, client: &str, lock: bool) -> Vec<(Pick, String, bool)> {
    let now = sub.who().get(client).and_then(|p| p.group.clone()).filter(|g| !g.is_empty());
    let show = sub.conductor().show();
    let mut out: Vec<(Pick, String, bool)> = show.outlets.iter().filter(|o| o.kind == OutletKind::Stage).map(|o| (Pick::Role(Some(o.id.clone())), o.name().to_string(), now.as_deref() == Some(&o.id))).collect();
    out.push((Pick::Role(None), deck_ui::live::NO_ROLE.into(), now.is_none()));
    out.push((Pick::Lock, "lock it there".into(), lock));
    out
}

fn line_of(sub: &Subtree, client: &str) -> String {
    here(sub).into_iter().find(|h| h.client == client).map_or_else(|| client.to_string(), |h| h.line)
}

/// "move to": the relay's `assign` (with `lock`, a lock from above for that deck), and what it did in words.
pub fn move_to(sub: &mut Subtree, client: &str, role: Option<&str>, lock: bool) -> (Vec<protocol::Envelope>, String) {
    let was = line_of(sub, client);
    let name = role.map_or(deck_ui::live::NO_ROLE.to_string(), |r| sub.conductor().show().outlet(r).map_or(r.to_string(), |o| o.name().to_string()));
    let out = sub.assign(client, role, lock);
    (out, format!("{was} → {name}{}", if lock { " · locked" } else { "" }))
}

/// "rename": the relay's `rename` (it cleans the name), and what it did in words.
pub fn rename(sub: &mut Subtree, client: &str, nick: &str) -> (Vec<protocol::Envelope>, String) {
    let was = line_of(sub, client);
    let nick = nick.trim();
    (sub.rename(client, nick), if nick.is_empty() { format!("{was}: name cleared") } else { format!("{was} is called {nick}") })
}

/// "kick", once the prompt is confirmed: the relay's `kick`, and what it did in words.
pub fn kick(sub: &mut Subtree, client: &str) -> (Vec<protocol::Envelope>, String) {
    let was = line_of(sub, client);
    (sub.kick(client), format!("{was} kicked"))
}

/// The prompt before a kick.
pub fn kick_prompt(sub: &Subtree, client: &str) -> Prompt {
    let line = line_of(sub, client);
    let lines = vec![
        (format!("{client} is told it was removed and its link ends"), Tone::Warn),
        ("It can join again with the code; then it is here again.".to_string(), Tone::Muted),
    ];
    Prompt { title: format!("Kick {line}?"), lines, warn: true, confirm: "Kick".into(), can: true }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse;
    use crate::subtree::ClockChoice;
    use protocol::{Envelope, Message, Roles, ShowRole};

    pub(crate) const CHOIR: &str = r#"{
      "id": "choir", "version": 1,
      "outlets": [
        { "id": "tenor" },
        { "id": "baritone", "for": { "account": ["member"] } },
        { "id": "click", "label": "click monitor", "for": { "account": ["live"], "approved": true } },
        { "id": "front", "label": "front", "for": { "device": ["phone"], "group": ["foyer"] } },
        { "id": "lamps", "kind": "device" }
      ],
      "rows": [
        { "id": "warm-up", "cells": { "tenor": { "mode": "play", "piece": "scene:scale" }, "baritone": { "mode": "play", "piece": "scene:scale" } } },
        { "id": "click", "cells": { "click": { "mode": "play", "piece": "scene:click" } } }
      ]
    }"#;

    #[test]
    fn a_filter_in_words() {
        assert_eq!(filter_words(None), "guest and up");
        let a = |json: &str| serde_json::from_str::<Admits>(json).unwrap();
        assert_eq!(filter_words(Some(&a(r#"{ "account": ["guest", "member"] }"#))), "guest and up", "a guest lets everyone in");
        assert_eq!(filter_words(Some(&a(r#"{ "account": ["member"] }"#))), "member");
        assert_eq!(filter_words(Some(&a(r#"{ "account": ["live"], "approved": true }"#))), "live · approved deck");
        assert_eq!(filter_words(Some(&a(r#"{ "device": ["phone"], "group": ["foyer"] }"#))), "device phone · group foyer");
        assert_eq!(filter_words(Some(&a("{}"))), "nobody");
    }

    #[test]
    fn the_roles_are_the_stage_outlets_with_the_relays_count() {
        let show = parse(CHOIR).unwrap();
        let mut sub = Subtree::new(show.clone(), ClockChoice::Parent);
        let names: Vec<(String, String)> = roles(&show, None).into_iter().map(|r| (r.name, r.filter)).collect();
        assert_eq!(names[2], ("click monitor".to_string(), "live · approved deck".to_string()));
        assert_eq!(names.len(), 4, "the lamps are no role");
        assert!(roles(&show, Some(&sub)).iter().all(|r| r.here.is_none()), "no count before the relay's");
        let counted = |id: &str, n| ShowRole { id: id.into(), label: None, r#for: None, here: Some(n) };
        sub.on_wire(&Envelope::new("relay", Message::Roles(Roles { roles: vec![counted("tenor", 2), counted("click", 1)] })), 0);
        let here: Vec<String> = roles(&show, Some(&sub)).iter().map(|r| here_text(r.here)).collect();
        assert_eq!(here, ["2", "", "1", ""]);
    }

    fn fact(show: &Show, id: &str, f: Fact) -> Result<Edited, String> {
        edit(show, &Edit::Fact { id: id.into(), fact: f })
    }

    #[test]
    fn a_role_is_added_renamed_and_its_filter_set_through_the_document() {
        let show = parse(CHOIR).unwrap();
        let added = edit(&show, &Edit::Add).unwrap();
        assert_eq!(added.role.as_deref(), Some("role"));
        assert_eq!(added.show.outlets[4].id, "role", "after the last role, before the lamps");
        assert_eq!(added.said, "role role added: guest and up may take it");
        assert_eq!(edit(&added.show, &Edit::Add).unwrap().role.as_deref(), Some("role-2"));
        let mut sub = Subtree::new(show.clone(), ClockChoice::Parent);
        let out = sub.edited(added.show.clone());
        assert!(matches!(&out[0].message, Message::Roles(r) if r.roles.len() == 5), "the session hears the new role at once");
        let named = edit(&show, &Edit::Label { id: "tenor".into(), label: " tenor 1 ".into() }).unwrap();
        assert_eq!(named.show.outlets[0].label.as_deref(), Some("tenor 1"));
        assert_eq!(named.said, "role tenor is called tenor 1");
        assert_eq!(edit(&named.show, &Edit::Label { id: "tenor".into(), label: "".into() }).unwrap().show.outlets[0].label, None);
        // guest and up: the toggles show guest on; member on keeps the guest – still anyone
        let t = toggles(&show, "tenor");
        assert!(t[0].on && t[0].word == "guest" && !t[1].on);
        let words: Vec<&str> = t.iter().map(|t| t.word.as_str()).collect();
        assert_eq!(words[5..9], ["approved deck", "device native", "device browser", "device phone"]);
        assert!(words.contains(&"group click"), "the show's roles are the groups offered");
        let member = fact(&show, "tenor", Fact::Account(Account::Member)).unwrap();
        assert_eq!(member.said, "tenor: guest and up");
        let only_member = fact(&member.show, "tenor", Fact::Account(Account::Guest)).unwrap();
        assert_eq!(only_member.said, "tenor: member");
        let back = fact(&fact(&only_member.show, "tenor", Fact::Account(Account::Guest)).unwrap().show, "tenor", Fact::Account(Account::Member)).unwrap();
        assert_eq!(back.show.outlets[0].admits, None, "only the guest left: no filter");
        assert_eq!(fact(&show, "click", Fact::Approved).unwrap().said, "click monitor: live");
        assert_eq!(fact(&show, "front", Fact::Device("phone".into())).unwrap().said, "front: group foyer");
    }

    #[test]
    fn an_edit_that_adds_a_problem_is_refused_in_deck_shows_words() {
        let show = parse(CHOIR).unwrap();
        // the guest off a role open to everyone: the filter would name nobody
        let refused = fact(&show, "tenor", Fact::Account(Account::Guest)).unwrap_err();
        assert_eq!(refused, "not changed: show: outlet 'tenor': `for` names nobody (leave it out: anyone with the code)");
        assert_eq!(edit(&show, &Edit::Remove { id: "lamps".into() }).unwrap_err(), "no role 'lamps'", "a device outlet is no role");
        let mut doc: serde_json::Value = serde_json::from_str(CHOIR).unwrap();
        doc["rows"][1]["cells"]["front"] = serde_json::json!({ "mode": "mirror", "of": "click" });
        let mirrored: Show = serde_json::from_value(doc).unwrap();
        assert_eq!(edit(&mirrored, &Edit::Remove { id: "click".into() }).unwrap_err(), "not changed: row 'click', 'front': no outlet 'click'");
    }

    #[test]
    fn a_remove_asks_first_and_says_what_goes() {
        let show = parse(CHOIR).unwrap();
        let p = remove_prompt(&show, "tenor", Some(2));
        assert_eq!(p.title, "Remove the role tenor?");
        assert_eq!(p.lines[0], ("row 1 warm-up: scene:scale goes".to_string(), Tone::Warn));
        assert_eq!(p.lines[1].0, "2 sit in it now: they stay under its name until they take another role or are moved");
        assert!(p.warn && p.can && p.confirm == "Remove");
        assert_eq!(remove_prompt(&show, "front", None).lines[0], ("it has no cells".to_string(), Tone::Muted));
        let gone = edit(&show, &Edit::Remove { id: "tenor".into() }).unwrap();
        assert_eq!(gone.said, "role tenor removed with its 1 cells");
        assert!(!gone.show.rows[0].cells.contains_key("tenor") && gone.role.is_none());
    }

    #[test]
    fn the_inspector_of_a_role() {
        let show = parse(CHOIR).unwrap();
        let i = inspector(&show, None, "click").unwrap();
        assert_eq!(i.title, "role · click");
        assert_eq!(i.name, ("name".to_string(), "click monitor".to_string()));
        assert_eq!(i.enter.1, "live · approved deck");
        assert_eq!(i.sections[0].1[0].1, "no session");
        assert_eq!(i.sections[1].1, [("2 click".to_string(), "scene:click".to_string())]);
        assert_eq!(i.sections[2].1[0], ("conductor".to_string(), "choir".to_string()));
        assert!(inspector(&show, None, "lamps").is_none());
    }

    /// a client in presence: id, role, name, device, locked (c1 has proven it is a member, #478 slice 4)
    type Who<'a> = (&'a str, Option<&'a str>, Option<&'a str>, Option<&'a str>, bool);

    fn presence(who: &[Who]) -> Envelope {
        let who = who.iter().map(|(id, g, nick, device, locked)| {
            let p = protocol::Present { group: g.map(Into::into), nick: nick.map(Into::into), device: device.map(Into::into), account: (*id == "c1").then_some(Account::Member), locked: locked.then_some(true) };
            (id.to_string(), p)
        });
        let p = protocol::Presence { groups: Default::default(), ready: vec![], decks: None, routes: None, links: None, quality: None, seats: None, parent: None, who: Some(who.collect()) };
        Envelope::new("relay", Message::Presence(p))
    }

    #[test]
    fn who_is_here_says_role_name_device_and_why() {
        let mut sub = Subtree::new(parse(CHOIR).unwrap(), ClockChoice::Parent);
        let (out, said) = move_to(&mut sub, "c2", Some("baritone"), false);
        assert!(matches!(&out[0].message, Message::Assign(a) if a.client == "c2" && a.group == "baritone" && a.lock.is_none()), "{out:?}");
        assert_eq!(said, "c2 → baritone");
        let (out, _) = move_to(&mut sub, "pc", Some("click"), true);
        assert!(matches!(&out[0].message, Message::Assign(a) if a.lock == Some(true)));
        sub.on_wire(
            &presence(&[
                ("c1", Some("tenor"), Some("Anna"), Some("browser"), false),
                ("c2", Some("baritone"), Some("Jo"), Some("native"), false),
                ("pc", Some("click"), None, Some("native"), true),
                ("c9", Some("tenor"), Some("Ben"), Some("phone"), true),
                ("phone-7", None, None, Some("phone"), false),
            ]),
            0,
        );
        let h = here(&sub);
        let rows: Vec<(&str, &str, &str, bool)> = h.iter().map(|h| (h.line.as_str(), h.stated.as_str(), h.why.as_str(), h.locked_above)).collect();
        assert_eq!(
            rows,
            [
                ("tenor: Anna", "browser · member", "chose itself", false),
                ("tenor: Ben", "phone · guest", "set by a conductor above · locked", true),
                ("baritone: Jo", "native · guest", "moved here by this conductor", false),
                ("click monitor: pc", "native · guest", "moved here by this conductor · locked", false),
                ("no role: phone-7", "phone · guest", "no role · the idle scene", false),
            ]
        );
        assert_eq!(sub.here()[1], "tenor: Ben (locked)", "the conductor's lines are the same list");
        // the deck went on by itself: no longer "moved here"
        sub.on_wire(&presence(&[("c2", Some("tenor"), Some("Jo"), Some("native"), false)]), 1);
        assert_eq!(here(&sub)[0].why, "chose itself");
    }

    #[test]
    fn the_move_menu_rename_and_kick_say_what_they_send() {
        let mut sub = Subtree::new(parse(CHOIR).unwrap(), ClockChoice::Parent);
        sub.on_wire(&presence(&[("c1", Some("tenor"), Some("Anna"), Some("browser"), false)]), 0);
        let menu = move_menu(&sub, "c1", false);
        let words: Vec<(&str, bool)> = menu.iter().map(|(_, w, on)| (w.as_str(), *on)).collect();
        assert_eq!(words, [("tenor", true), ("baritone", false), ("click monitor", false), ("front", false), ("no role", false), ("lock it there", false)]);
        assert_eq!(menu[4].0, Pick::Role(None));
        let (out, said) = move_to(&mut sub, "c1", None, false);
        assert!(matches!(&out[0].message, Message::Assign(a) if a.group.is_empty()), "no role is an empty group");
        assert_eq!(said, "tenor: Anna → no role");
        let (out, said) = rename(&mut sub, "c1", " Annie ");
        assert!(matches!(&out[0].message, Message::Rename(r) if r.client == "c1" && r.nick == "Annie"));
        assert_eq!(said, "tenor: Anna is called Annie");
        let p = kick_prompt(&sub, "c1");
        assert_eq!(p.title, "Kick tenor: Anna?");
        assert!(p.warn && p.confirm == "Kick");
        let (out, said) = kick(&mut sub, "c1");
        assert!(matches!(&out[0].message, Message::Kick(k) if k.client == "c1"));
        assert_eq!(said, "tenor: Anna kicked");
    }
}
