//! The one filter's one rule (#546, research/SHOW.md §9, §15 2): who a row's address reaches (the receiver tests itself,
//! deck-show `address::passes`) and who a show cell's column gets – for a vote the relay tests it (#547). One shape
//! ([`Filter`]), one evaluator ([`passes`]), read by every side; the account ladder is here once ([`counts_as`]), the relay's
//! role admission reads it too.
use crate::{Account, Filter};

/// What a client is, as whoever tests a filter knows it: the group it joined with, the device it says, the account facts
/// and "an approved deck" only from the server's proof. A fact not known is missing (none, empty, false).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Facts<'a> {
    pub group: Option<&'a str>,
    pub device: Option<&'a str>,
    pub account: &'a [Account],
    pub approved: bool,
}

/// Does the proven account fact `has` count as the fact `asked` a filter names? Exact, as #478 slice 3 built it: a filter
/// that says member admits a member. THE LADDER'S ONE PLACE – open owner question (#478): do admin and creator count as
/// member? To make it a ladder, change only this function (deck-show's filter words read the facts exactly too).
pub fn counts_as(has: Account, asked: Account) -> bool {
    has == asked
}

/// Does a client with the proven account facts `has` hold the account fact `asked`? `guest` is anyone with the code.
pub fn has_account(asked: Account, has: &[Account]) -> bool {
    asked == Account::Guest || has.iter().any(|f| counts_as(*f, asked))
}

/// Does `facts` pass `filter`? None passes everyone. Every fact the filter names must hold; `account` admits any one of
/// its values (an empty list admits nobody); a missing fact fails (SHOW.md §9).
pub fn passes(filter: Option<&Filter>, facts: &Facts) -> bool {
    let Some(f) = filter else { return true };
    f.group.as_deref().is_none_or(|g| facts.group == Some(g))
        && f.device.as_deref().is_none_or(|d| facts.device == Some(d))
        && f.account.as_ref().is_none_or(|list| list.iter().any(|a| has_account(*a, facts.account)))
        && (f.approved != Some(true) || facts.approved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(json: &str) -> Filter {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn every_named_fact_must_hold_and_a_missing_one_fails() {
        let phone = Facts { group: Some("left"), device: Some("phone"), ..Facts::default() };
        assert!(passes(None, &Facts::default()), "no filter: everyone");
        assert!(passes(Some(&filter("{}")), &Facts::default()), "a filter naming nothing asks nothing");
        assert!(passes(Some(&filter(r#"{ "group": "left", "device": "phone" }"#)), &phone));
        assert!(!passes(Some(&filter(r#"{ "group": "left", "device": "browser" }"#)), &phone), "all facts together");
        assert!(!passes(Some(&filter(r#"{ "device": "phone" }"#)), &Facts { group: Some("left"), ..Facts::default() }), "a missing fact fails");
    }

    #[test]
    fn an_account_list_admits_any_of_its_values_and_a_guest_is_anyone() {
        let member = Facts { account: &[Account::Member], ..Facts::default() };
        let guest = Facts::default();
        let f = filter(r#"{ "account": ["live", "member"] }"#);
        assert!(passes(Some(&f), &member) && !passes(Some(&f), &guest));
        assert!(passes(Some(&filter(r#"{ "account": ["guest"] }"#)), &guest), "a guest: anyone with the code");
        assert!(!passes(Some(&filter(r#"{ "account": [] }"#)), &member), "an empty list admits nobody");
        assert!(!passes(Some(&filter(r#"{ "account": ["member"], "device": "phone" }"#)), &member), "the device is missing");
        assert!(!counts_as(Account::Admin, Account::Member), "exact, no ladder yet");
    }

    #[test]
    fn an_approved_deck_only_with_the_proof() {
        let f = filter(r#"{ "approved": true, "device": "native" }"#);
        assert!(passes(Some(&f), &Facts { device: Some("native"), approved: true, ..Facts::default() }));
        assert!(!passes(Some(&f), &Facts { device: Some("native"), ..Facts::default() }));
        assert!(passes(Some(&filter(r#"{ "approved": false }"#)), &Facts::default()), "false asks nothing");
    }
}
