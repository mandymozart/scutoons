//! A flash (#90, SHOW.md §4b, owner 2026-11-02 [venue]): one assignment, the tree resolved to exact versions. It trickles down
//! the tree: "a lock from above wins, otherwise the nearest wins, a deck may reject for capability, every deck reports what
//! it runs". One rule, here; a conductor and a plain deck both call [`take`].
use crate::document::Pinned;
use protocol::{Flash, FlashReport};

/// What a conductor or a deck was assigned, and by how near a hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub root: Pinned,
    pub shows: Vec<Pinned>,
    /// 0 = its own operator, 1 = its parent, …
    pub hops: u32,
    pub locked: bool,
}

impl Assignment {
    pub fn of(flash: &Flash) -> Assignment {
        Assignment { root: pinned(&flash.root), shows: flash.shows.iter().map(pinned).collect(), hops: flash.hops as u32, locked: flash.lock == Some(true) }
    }

    /// The version this assignment wants of a show, if it names it.
    pub fn version_of(&self, show: &str) -> Option<u32> {
        self.shows.iter().find(|p| p.id == show).map(|p| p.version)
    }
}

pub(crate) fn pinned(p: &protocol::Pinned) -> Pinned {
    Pinned { id: p.id.clone(), version: p.version as u32 }
}

pub(crate) fn wire(p: &Pinned) -> protocol::Pinned {
    protocol::Pinned { id: p.id.clone(), version: p.version as u64 }
}

/// A report that says only what runs (and, refused, why); the facts a deck says about itself are added by `ready::Deck`.
pub fn report(running: &Pinned, accepted: bool, why: Option<String>) -> FlashReport {
    FlashReport { running: wire(running), accepted, why, outlet: None, ready: None, lacks: None, keep: None, unsaved: None, mappings: None, device: None, kept: None, own: None }
}

fn named(list: &Option<Vec<String>>, me: &str) -> bool {
    !me.is_empty() && list.as_ref().is_some_and(|l| l.iter().any(|p| p == me))
}

/// #471: the operator left `me` out (a client id in the session the flash is in now, or a sub conductor's show).
pub fn left_out(flash: &Flash, me: &str) -> bool {
    named(&flash.skip, me)
}

/// #471: the prompt warned of `me` and the operator accepted: flashed although it asked not to be, its configuration kept.
pub fn forced(flash: &Flash, me: &str) -> bool {
    named(&flash.force, me)
}

/// #471: the flash as it goes on into this conductor's own session. Its lists are relative to the session a flash is in:
/// a client id, or `<show>/…` for what sits in a seat (the show the seat sits: a child knows its own show, not the seat's
/// name in its parent). `me` = this conductor's show when the flash came from above (its part is taken out), none for its
/// own operator's (the lists name this document's seats); `show_of(seat)` = the show a seat of this document sits.
pub fn down(flash: &Flash, me: Option<&str>, show_of: impl Fn(&str) -> Option<String>) -> Flash {
    let relative = |list: &Option<Vec<String>>| -> Option<Vec<String>> {
        let list = list.as_ref()?;
        let mine: Vec<String> = match me {
            Some(me) => list.iter().filter_map(|p| p.strip_prefix(&format!("{me}/")).map(str::to_string)).collect(),
            None => list.clone(),
        };
        let out: Vec<String> = mine
            .into_iter()
            .map(|p| match p.split_once('/') {
                Some((seat, rest)) => format!("{}/{rest}", show_of(seat).unwrap_or_else(|| seat.to_string())),
                None => show_of(&p).unwrap_or(p),
            })
            .collect();
        (!out.is_empty()).then_some(out)
    };
    Flash { skip: relative(&flash.skip), force: relative(&flash.force), ..flash.clone() }
}

/// Whether `flash` is taken over what stands, given what the receiver can do. `Err` = why it is refused.
pub fn decide(standing: Option<&Assignment>, flash: &Flash, cannot: Option<&str>) -> Result<(), String> {
    let hops = flash.hops as u32;
    let locked = flash.lock == Some(true);
    if locked && hops > 0 {
        return Ok(());
    }
    if let Some(why) = cannot {
        return Err(why.to_string());
    }
    match standing {
        Some(s) if s.locked && s.hops > 0 && hops < s.hops => Err("locked from above".to_string()),
        Some(s) if s.hops < hops => Err("assigned by a nearer hand".to_string()),
        _ => Ok(()),
    }
}

/// One receiver takes a flash: the rule, what stands afterwards, and its report. `running` = what it plays now.
pub fn take(standing: &mut Option<Assignment>, flash: &Flash, running: &Pinned, cannot: Option<&str>) -> FlashReport {
    match decide(standing.as_ref(), flash, cannot) {
        Ok(()) => {
            *standing = Some(Assignment::of(flash));
            report(running, true, None)
        }
        Err(why) => report(running, false, Some(why)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flash(hops: u64, lock: bool) -> Flash {
        Flash { root: protocol::Pinned { id: "top".into(), version: 3 }, shows: vec![protocol::Pinned { id: "top".into(), version: 3 }], lock: lock.then_some(true), hops, here: None, id: None, by: None, skip: None, force: None, needs: None }
    }

    fn standing(hops: u32, locked: bool) -> Assignment {
        Assignment { root: Pinned { id: "x".into(), version: 1 }, shows: vec![], hops, locked }
    }

    #[test]
    fn a_lock_from_above_wins_over_a_refusal_and_over_what_stands() {
        assert_eq!(decide(Some(&standing(0, false)), &flash(1, true), Some("no screen")), Ok(()));
    }

    #[test]
    fn a_deck_may_refuse_what_is_not_locked() {
        assert_eq!(decide(None, &flash(1, false), Some("no screen")), Err("no screen".to_string()));
    }

    #[test]
    fn otherwise_the_nearest_wins() {
        assert_eq!(decide(Some(&standing(0, false)), &flash(2, false), None), Err("assigned by a nearer hand".to_string()));
        assert_eq!(decide(Some(&standing(2, false)), &flash(1, false), None), Ok(()));
        assert_eq!(decide(Some(&standing(1, false)), &flash(1, false), None), Ok(()));
    }

    #[test]
    fn a_locked_assignment_is_not_replaced_by_a_nearer_hand() {
        assert_eq!(decide(Some(&standing(2, true)), &flash(0, false), None), Err("locked from above".to_string()));
        assert_eq!(decide(Some(&standing(2, true)), &flash(3, false), None), Err("assigned by a nearer hand".to_string()));
    }

    #[test]
    fn the_lists_go_down_relative_to_each_session_by_the_show_a_seat_sits() {
        let mut f = flash(0, false);
        f.skip = Some(vec!["c7".into(), "foyer/c3".into(), "bar".into(), "foyer/inner/c9".into()]);
        f.force = Some(vec!["c8".into()]);
        let show_of = |seat: &str| match seat {
            "foyer" => Some("foyer-loop".to_string()),
            "bar" => Some("bar-show".to_string()),
            "inner" => Some("inner-show".to_string()),
            _ => None,
        };
        // the top's operator names its own seats; on the way down they become the shows they sit
        let top = down(&f, None, show_of);
        assert_eq!(top.skip.as_deref(), Some(&["c7".to_string(), "foyer-loop/c3".into(), "bar-show".into(), "foyer-loop/inner/c9".into()][..]));
        assert!(left_out(&top, "c7") && !left_out(&top, "c8") && forced(&top, "c8"));
        assert!(left_out(&top, "bar-show"), "a sub conductor left out is named by its show");
        // the child foyer-loop takes its part out, names its own seats by their shows
        let child = down(&top, Some("foyer-loop"), show_of);
        assert_eq!(child.skip.as_deref(), Some(&["c3".to_string(), "inner-show/c9".into()][..]));
        assert_eq!(child.force, None);
        assert!(!left_out(&child, ""), "a deck that does not know its id is never named");
    }

    #[test]
    fn a_report_says_what_runs_either_way() {
        let run = Pinned { id: "mine".into(), version: 7 };
        let mut s = None;
        let r = take(&mut s, &flash(1, false), &run, Some("no screen"));
        assert!(!r.accepted);
        assert_eq!(r.running.version, 7);
        assert!(s.is_none());
        assert!(take(&mut s, &flash(1, false), &run, None).accepted);
        assert!(s.is_some());
    }
}
