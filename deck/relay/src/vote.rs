//! The vote tally (LIVE.md B.6, decision B.11 (3)): a rule per vote – majority, per group, first to n – plus a time and a
//! fallback; the relay applies it by itself, the conductor can override. Pure: the session feeds the casts in.
use std::collections::HashMap;

use protocol::{VoteHow, VoteOpen, VoteResult, VoteRule};

/// An open vote: what the conductor opened, when it closes, one cast per client (changeable until the close).
#[derive(Debug, Clone)]
pub struct OpenVote {
    pub open: VoteOpen,
    /// relay time (ms) when the vote closes by itself
    pub deadline: i64,
    /// client → (option, the client's group when it cast)
    pub casts: HashMap<String, (usize, Option<String>)>,
}

impl OpenVote {
    pub fn new(open: VoteOpen, now: i64) -> Self {
        let deadline = now + open.seconds.max(1) * 1000;
        OpenVote { open, deadline, casts: HashMap::new() }
    }

    /// May a client in `group` vote here? `groups` missing = every audience client (the session checks the role).
    pub fn admits(&self, group: Option<&str>) -> bool {
        match &self.open.groups {
            None => true,
            Some(gs) => group.is_some_and(|g| gs.iter().any(|x| x == g)),
        }
    }

    pub fn counts(&self) -> Vec<i64> {
        count(self.open.options.len(), self.casts.values().map(|(o, _)| *o))
    }

    /// Rule `first`: the option that has reached `n` (checked after every cast).
    pub fn reached(&self) -> Option<usize> {
        if self.open.rule != VoteRule::First {
            return None;
        }
        let n = self.open.n.unwrap_or(1).max(1);
        self.counts().iter().position(|&c| c >= n)
    }

    /// The result: the conductor's override, else the rule, else (nobody voted) the fallback.
    pub fn decide(&self, override_option: Option<usize>) -> VoteResult {
        let counts = self.counts();
        let fallback = self.fallback();
        let (option, how) = if let Some(o) = override_option.filter(|&o| o < self.open.options.len()) {
            (o, VoteHow::Override)
        } else if self.casts.is_empty() {
            (fallback, VoteHow::Fallback)
        } else {
            let option = match self.open.rule {
                VoteRule::Majority => winner(&counts, fallback),
                VoteRule::First => self.reached().unwrap_or_else(|| winner(&counts, fallback)),
                VoteRule::Groups => {
                    // each group's majority is one vote; clients without a group vote as one group of their own
                    let mut by_group: HashMap<Option<&str>, Vec<usize>> = HashMap::new();
                    for (o, g) in self.casts.values() {
                        by_group.entry(g.as_deref()).or_default().push(*o);
                    }
                    let n = self.open.options.len();
                    let group_votes = by_group.values().map(|os| winner(&count(n, os.iter().copied()), fallback));
                    winner(&count(n, group_votes), fallback)
                }
            };
            (option, VoteHow::Rule)
        };
        VoteResult { vote: self.open.vote.clone(), option: option as i64, counts, how }
    }

    fn fallback(&self) -> usize {
        let f = self.open.fallback.unwrap_or(0).max(0) as usize;
        if f < self.open.options.len() { f } else { 0 }
    }
}

fn count(n: usize, options: impl Iterator<Item = usize>) -> Vec<i64> {
    let mut counts = vec![0; n];
    for o in options {
        if let Some(c) = counts.get_mut(o) {
            *c += 1;
        }
    }
    counts
}

/// The option with the most votes; a tie goes to the fallback if it is among the tied, else to the first tied option.
fn winner(counts: &[i64], fallback: usize) -> usize {
    let max = counts.iter().copied().max().unwrap_or(0);
    if counts.get(fallback) == Some(&max) {
        return fallback;
    }
    counts.iter().position(|&c| c == max).unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(rule: VoteRule, n: Option<i64>) -> OpenVote {
        let v = VoteOpen {
            vote: "v1".into(),
            options: vec!["left".into(), "right".into(), "stay".into()],
            rule,
            n,
            seconds: 20,
            fallback: Some(2),
            groups: None,
            show: None,
        };
        OpenVote::new(v, 1_000)
    }

    fn cast(v: &mut OpenVote, client: &str, option: usize, group: &str) {
        v.casts.insert(client.into(), (option, Some(group.into())));
    }

    #[test]
    fn deadline_from_seconds() {
        assert_eq!(open(VoteRule::Majority, None).deadline, 21_000);
    }

    #[test]
    fn nobody_voted_is_the_fallback() {
        let r = open(VoteRule::Majority, None).decide(None);
        assert_eq!((r.option, r.how), (2, VoteHow::Fallback));
        assert_eq!(r.counts, [0, 0, 0]);
    }

    #[test]
    fn majority_and_ties() {
        let mut v = open(VoteRule::Majority, None);
        cast(&mut v, "a", 0, "g");
        cast(&mut v, "b", 1, "g");
        cast(&mut v, "c", 1, "g");
        assert_eq!(v.decide(None).option, 1);
        // a changed vote replaces the old one: 0 and 1 tie, the fallback (2) is not tied → the first tied
        cast(&mut v, "c", 0, "g");
        assert_eq!(v.decide(None).option, 0);
        // a tie that includes the fallback goes to the fallback
        cast(&mut v, "a", 2, "g");
        cast(&mut v, "b", 2, "g");
        assert_eq!(v.decide(None).counts, [1, 0, 2]);
        cast(&mut v, "d", 0, "g");
        assert_eq!(v.decide(None).option, 2);
    }

    #[test]
    fn groups_count_once_each() {
        let mut v = open(VoteRule::Groups, None);
        // the big left block wants 0, two small blocks want 1 → 1 wins 2 : 1
        for i in 0..10 {
            cast(&mut v, &format!("l{i}"), 0, "left");
        }
        cast(&mut v, "r1", 1, "right");
        cast(&mut v, "b1", 1, "balcony");
        let r = v.decide(None);
        assert_eq!((r.option, r.how), (1, VoteHow::Rule));
        assert_eq!(r.counts, [10, 2, 0]);
    }

    #[test]
    fn first_to_n() {
        let mut v = open(VoteRule::First, Some(2));
        cast(&mut v, "a", 1, "g");
        assert_eq!(v.reached(), None);
        cast(&mut v, "b", 1, "g");
        assert_eq!(v.reached(), Some(1));
        assert_eq!(v.decide(None).option, 1);
    }

    #[test]
    fn override_wins_unless_out_of_range() {
        let mut v = open(VoteRule::Majority, None);
        cast(&mut v, "a", 0, "g");
        let r = v.decide(Some(1));
        assert_eq!((r.option, r.how), (1, VoteHow::Override));
        assert_eq!(v.decide(Some(9)).how, VoteHow::Rule);
    }

    #[test]
    fn admits_by_group() {
        let mut v = open(VoteRule::Majority, None);
        assert!(v.admits(None));
        v.open.groups = Some(vec!["left".into()]);
        assert!(v.admits(Some("left")));
        assert!(!v.admits(Some("right")));
        assert!(!v.admits(None));
    }
}
