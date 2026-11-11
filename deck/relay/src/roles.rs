//! The conductor's roles in its session (#478, owner 2026-11-10 [role]): the conductor says which roles its show has (`roles`:
//! a stage outlet each, its name, who may take it); the relay keeps them for the session and gives every client the roles it
//! may take, with how many are in each – when it joins and whenever that changes. A role is the group a client sits in: the
//! conductor's `assign` moves a client (with `lock`, a lock from above: it cannot leave), a deck's own `assign` (itself, a role
//! it may take or "" for none) is a deck taking a role; its `rename` (itself) is its name. Presence tells the conductor who
//! sits where under which name, so it can say `role: name`.
//!
//! The facts: `account: guest` (anyone with the code), the `device` a client says in its `hello`, the `group` it joined with
//! (proposal: its QR's, a deck's configured outlet – the group it sits in is its role, so that cannot be the fact); the
//! account facts (member, creator, admin, live) and "an approved deck" only by the API's signed proof in its `hello.proof`
//! (#478 slice 4, token.rs `proof`, kept as the client's `facts` when it joins). No proof, a bad or an expired one: a guest.
use crate::session::{Client, Session};
use protocol::{Account, Admits, Present, Role, ShowRole};
use std::collections::HashMap;

/// the clients that take a role: the decks (an outlet, a performer's or musician's) and the phones
pub fn takes_roles(role: Role) -> bool {
    matches!(role, Role::Output | Role::Performer | Role::Musician | Role::Audience)
}

/// Does the proven account fact `has` count as the fact `asked` a role names? Exact, as slice 3 built it: a role that says
/// member admits a member. THE LADDER'S ONE PLACE – open owner question (#478): do admin and creator count as member? To
/// make it a ladder, change only this function (deck-show's filter words read the facts exactly too).
pub fn counts_as(has: Account, asked: Account) -> bool {
    has == asked
}

/// May `c` take a role with this filter? None: a guest may, anyone with the code. Any one fact admits.
pub fn admits(a: Option<&Admits>, c: &Client) -> bool {
    let Some(a) = a else { return true };
    let has = |list: &Option<Vec<String>>, fact: Option<&str>| fact.is_some_and(|f| list.iter().flatten().any(|x| x == f));
    let account = |asked: &Account| *asked == Account::Guest || c.facts.account.iter().any(|f| counts_as(*f, *asked));
    a.account.iter().flatten().any(account)
        || (a.approved == Some(true) && c.facts.approved)
        || has(&a.device, c.device.as_deref())
        || has(&a.group, c.came.as_deref())
}

/// The strongest account fact `c` has proven, as presence says it (guest without one). Proposal (no owner word on an
/// order): admin, creator, live, member.
pub fn strongest(c: &Client) -> Account {
    const ORDER: [Account; 4] = [Account::Admin, Account::Creator, Account::Live, Account::Member];
    ORDER.into_iter().find(|a| c.facts.account.contains(a)).unwrap_or(Account::Guest)
}

/// how many clients sit in the role `id`
pub fn here(s: &Session, id: &str) -> u64 {
    s.clients.values().filter(|c| takes_roles(c.role) && c.group.as_deref() == Some(id)).count() as u64
}

/// The roles a client sees: the conductor (and the uplink) every one with its filter; a deck or a phone those it may take,
/// without the filter; anyone else none. Each with how many are in it.
pub fn offer(s: &Session, c: &Client) -> Option<Vec<ShowRole>> {
    let all = matches!(c.role, Role::Conductor | Role::Uplink);
    if !all && !takes_roles(c.role) {
        return None;
    }
    let shown = s.roles().iter().filter(|r| all || admits(r.r#for.as_ref(), c));
    Some(shown.map(|r| ShowRole { here: Some(here(s, &r.id)), r#for: if all { r.r#for.clone() } else { None }, ..r.clone() }).collect())
}

/// Why `c` (client `id`) may not take `group` ("" = no role) by its own `assign`, if it may not.
pub fn refusal(s: &Session, id: &str, c: &Client, client: &str, group: &str, lock: Option<bool>) -> Option<&'static str> {
    if client != id {
        Some("a client moves only itself")
    } else if lock.is_some() {
        Some("only the conductor locks")
    } else if c.locked {
        Some("locked by the conductor")
    } else if !takes_roles(c.role) {
        Some("this client takes no role")
    } else if !group.is_empty() && !s.roles().iter().any(|r| r.id == group && admits(r.r#for.as_ref(), c)) {
        Some("not a role this client may take")
    } else {
        None
    }
}

/// Who sits where (presence's `who`): every deck, and a phone that sits in a role (proposal: a phone in no role is left out –
/// hundreds of them would be sent on every join).
pub fn who(s: &Session) -> HashMap<String, Present> {
    let role_ids: Vec<&str> = s.roles().iter().map(|r| r.id.as_str()).collect();
    s.clients
        .iter()
        .filter(|(_, c)| takes_roles(c.role) && (c.role != Role::Audience || c.group.as_deref().is_some_and(|g| role_ids.contains(&g))))
        .map(|(id, c)| (id.clone(), Present { group: c.group.clone(), nick: c.nick.clone(), device: c.device.clone(), account: Some(strongest(c)), locked: c.locked.then_some(true) }))
        .collect()
}
