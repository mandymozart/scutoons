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
            FlashReport { running: wire(running), accepted: true, why: None, outlet: None, ready: None, lacks: None }
        }
        Err(why) => FlashReport { running: wire(running), accepted: false, why: Some(why), outlet: None, ready: None, lacks: None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flash(hops: u64, lock: bool) -> Flash {
        Flash { root: protocol::Pinned { id: "top".into(), version: 3 }, shows: vec![protocol::Pinned { id: "top".into(), version: 3 }], lock: lock.then_some(true), hops, here: None }
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
