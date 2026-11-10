//! The conductor's roles in the deck editor's Show workspace (#478 slice 3, owner 2026-11-10 [role], draft 4): every
//! conductor defines its roles – a role is a stage outlet of its show with its permission filter (`for`) – and sees who is
//! here. The tree lists the roles with how many are in each (the relay's count, the `roles` it sends the conductor), the
//! matrix's column header says the role's name, its filter in words and the count. The words are here once; the editor
//! lays them out (RULES A1, A2).
use crate::document::{OutletKind, Show};
use crate::subtree::Subtree;
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
}
