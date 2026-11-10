//! Addressing across the tree (#90, SHOW.md §8, §9), for the workshop kind and id, filters group and device. Who picks up goes
//! down, the receiver tests itself and says what it did. An address that finds nobody stays silent, and that is logged: it
//! never goes up to a wider one (owner 2026-11-09).
use protocol::{Address, AddressAnswer, AddressFilter};

/// One receiver: what it plays and the facts about itself.
#[derive(Clone, Copy)]
pub struct Receiver<'a> {
    pub group: Option<&'a str>,
    pub device: Option<&'a str>,
    /// what is on stage there, as places (`character:yellowmouse.ghost`, `scene:nave`)
    pub plays: &'a [String],
    /// does it have a handler for this action?
    pub handles: &'a dyn Fn(&str) -> bool,
}

/// `character:*` is every character; `character:yellowmouse` every persona and look of it (SHOW.md §8).
pub fn picks_up(address: &str, playing: &str) -> bool {
    let Some((kind, id)) = address.split_once(':') else { return false };
    let Some((there_kind, there)) = playing.split_once(':') else { return false };
    kind == there_kind && (id == "*" || there == id || there.strip_prefix(id).is_some_and(|rest| rest.starts_with('.')))
}

/// A missing fact fails the filter (SHOW.md §9).
pub fn passes(filter: Option<&AddressFilter>, group: Option<&str>, device: Option<&str>) -> bool {
    let Some(filter) = filter else { return true };
    filter.group.as_deref().is_none_or(|g| group == Some(g)) && filter.device.as_deref().is_none_or(|d| device == Some(d))
}

/// What one receiver answers; `None` = the address does not name it, it says nothing.
pub fn receive(a: &Address, r: Receiver) -> Option<AddressAnswer> {
    if !r.plays.iter().any(|p| picks_up(&a.address, p)) {
        return None;
    }
    let passed = passes(a.r#where.as_ref(), r.group, r.device);
    let answered = passed && (r.handles)(&a.action);
    Some(AddressAnswer { id: a.id.clone(), reached: 1, passed: passed as u64, answered: answered as u64 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(a: &str, filter: Option<AddressFilter>) -> Address {
        Address { id: "a1".into(), address: a.into(), action: "dance:samba".into(), r#where: filter }
    }

    fn receiver<'a>(group: Option<&'a str>, device: Option<&'a str>, plays: &'a [String]) -> Receiver<'a> {
        Receiver { group, device, plays, handles: &|a| a == "dance:samba" }
    }

    #[test]
    fn who_picks_up_goes_down_the_ladder() {
        assert!(picks_up("character:yellowmouse", "character:yellowmouse"));
        assert!(picks_up("character:yellowmouse", "character:yellowmouse.ghost"));
        assert!(!picks_up("character:yellowmouse", "character:yellowmouseling"));
        assert!(picks_up("character:*", "character:catpope"));
        assert!(!picks_up("location:*", "character:catpope"));
    }

    #[test]
    fn a_receiver_that_is_not_named_says_nothing() {
        let plays = ["scene:nave".to_string()];
        assert_eq!(receive(&address("character:yellowmouse", None), receiver(None, None, &plays)), None);
    }

    #[test]
    fn filters_test_the_receiver_and_a_missing_fact_fails() {
        let plays = ["character:yellowmouse".to_string()];
        let f = || Some(AddressFilter { group: Some("left".into()), device: Some("browser".into()) });
        let a = receive(&address("character:yellowmouse", f()), receiver(Some("left"), Some("browser"), &plays)).unwrap();
        assert_eq!((a.reached, a.passed, a.answered), (1, 1, 1));
        let a = receive(&address("character:yellowmouse", f()), receiver(Some("left"), None, &plays)).unwrap();
        assert_eq!((a.reached, a.passed, a.answered), (1, 0, 0));
    }

    #[test]
    fn who_cannot_handle_the_action_picked_up_but_did_not_answer() {
        let plays = ["character:yellowmouse".to_string()];
        let r = Receiver { group: None, device: None, plays: &plays, handles: &|_| false };
        let a = receive(&address("character:yellowmouse", None), r).unwrap();
        assert_eq!((a.reached, a.passed, a.answered), (1, 1, 0));
    }
}
