//! The vote's conductor side (#533, research/SHOW.md §14; LIVE.md B.6, #231): what this conductor knows of the vote its
//! session runs – the vote open, the relay's last tally, the result, the relay's refusal ([`Board`], kept in `Subtree`, fed
//! by `on_wire`) – and the panel in words for the editor's Perform workspace ([`panel`], deck-ui `vote::Panel`). The rule
//! is the relay's (relay/vote.rs tallies, holds, closes, decides); a group's pick shown here is the relay's own `winner`
//! (protocol `vote.rs`). A typed form becomes `vote.open` here ([`open`]); the moves are `Subtree::vote_open`, `vote_hold`,
//! `vote_close`.
use deck_ui::vote::{percent, Bar, Group, How};
/// the panel's model and the form are deck-ui's (named here for a caller without deck-ui)
pub use deck_ui::vote::{Form, Panel, Rule, State};
use protocol::{Message, VoteHow, VoteOpen, VoteResult, VoteRule, VoteTally};

/// What the conductor knows of its session's vote.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Board {
    /// the vote open now, or the last one (with its result)
    pub open: Option<VoteOpen>,
    pub tally: Option<VoteTally>,
    pub result: Option<VoteResult>,
    /// the relay refused a vote move (`error` with code `vote`): its words, until the next vote opens
    pub problem: Option<String>,
    /// votes opened from here, for the next id
    opened: u32,
}

impl Board {
    /// A vote is open (not closed yet).
    pub fn is_open(&self) -> bool {
        self.open.is_some() && self.result.is_none()
    }

    /// The id the next vote opened here gets: `v1`, `v2` …, past any the session already had.
    pub fn next_id(&self) -> String {
        let mut n = self.opened + 1;
        while self.open.as_ref().is_some_and(|o| o.vote == format!("v{n}")) {
            n += 1;
        }
        format!("v{n}")
    }

    pub(crate) fn opening(&mut self) {
        self.opened += 1;
    }

    /// What the relay said of the vote; true when it is news for the log (opened, decided).
    pub fn heard(&mut self, m: &Message) -> bool {
        match m {
            Message::VoteOpen(v) => {
                let fresh = self.open.as_ref().is_none_or(|o| o.vote != v.vote) || self.result.is_some();
                if fresh {
                    *self = Board { open: Some(v.clone()), opened: self.opened, ..Board::default() };
                }
                fresh
            }
            Message::VoteTally(t) if self.open.as_ref().is_some_and(|o| o.vote == t.vote) && self.result.is_none() => {
                self.tally = Some(t.clone());
                false
            }
            Message::VoteResult(r) if self.open.as_ref().is_some_and(|o| o.vote == r.vote) && self.result.is_none() => {
                self.result = Some(r.clone());
                true
            }
            Message::Error(e) if e.code == "vote" => {
                self.problem = Some(e.message.clone());
                false
            }
            _ => false,
        }
    }

    /// The result's option label (none while open).
    pub fn result_label(&self) -> Option<String> {
        let (o, r) = (self.open.as_ref()?, self.result.as_ref()?);
        Some(o.options.get(r.option.max(0) as usize).cloned().unwrap_or_else(|| r.option.to_string()))
    }
}

/// The question in English, else the first given.
fn question(o: &VoteOpen) -> String {
    o.question.as_ref().and_then(|q| q.get("en").or_else(|| q.values().next())).cloned().unwrap_or_default()
}

pub(crate) fn rule(r: VoteRule) -> Rule {
    match r {
        VoteRule::Majority => Rule::Majority,
        VoteRule::Groups => Rule::Groups,
        VoteRule::First => Rule::First,
    }
}

fn how(h: VoteHow) -> How {
    match h {
        VoteHow::Rule => How::Rule,
        VoteHow::Fallback => How::Fallback,
        VoteHow::Override => How::Override,
    }
}

/// The panel for what `b` knows; none without a vote. `name` gives a group its name (the show's role, else its id).
pub fn panel(b: &Board, name: &dyn Fn(&str) -> String) -> Option<Panel> {
    let o = b.open.as_ref()?;
    let (t, r) = (b.tally.as_ref(), b.result.as_ref());
    let n = o.options.len();
    let counts: Vec<i64> = r.map(|r| r.counts.clone()).or_else(|| t.map(|t| t.counts.clone())).unwrap_or_else(|| vec![0; n]);
    let voters: i64 = counts.iter().sum();
    let fallback = o.fallback.unwrap_or(0).max(0) as usize;
    let fallback = if fallback < n { fallback } else { 0 };
    let total_ms = (o.seconds.max(1) * 1000) as f32;
    let left = if r.is_some() { 0 } else { t.map_or(o.seconds * 1000, |t| t.left) };
    let state = match (r, t.and_then(|t| t.paused)) {
        (Some(_), _) => State::Closed,
        (None, Some(true)) => State::Held,
        _ => State::Open,
    };
    let top = counts.iter().copied().max().unwrap_or(0);
    let leading = r.map_or_else(|| counts.iter().position(|&c| c == top).unwrap_or(0), |r| r.option.max(0) as usize);
    // first to n: the bar is the way to n, else the share of the voters
    let scale = if o.rule == VoteRule::First { o.n.unwrap_or(1).max(top).max(1) } else { voters.max(1) } as f32;
    let bars = o
        .options
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let c = counts.get(i).copied().unwrap_or(0);
            Bar { label: label.clone(), count: c, percent: percent(c, &counts), lead: i == leading && (voters > 0 || r.is_some()), fill: c as f32 / scale }
        })
        .collect();
    let by_group = r.and_then(|r| r.groups.clone()).or_else(|| t.and_then(|t| t.groups.clone())).unwrap_or_default();
    let mut groups: Vec<Group> = if o.rule == VoteRule::Groups {
        by_group
            .iter()
            .map(|(g, c)| Group { name: name(g), cells: c.iter().map(|&x| (percent(x, c), x)).collect(), pick: protocol::vote::winner(c, fallback) })
            .collect()
    } else {
        Vec::new()
    };
    groups.sort_by(|a, b| a.name.cmp(&b.name));
    let group_votes = (0..n).map(|i| groups.iter().filter(|g| g.pick == i).count() as i64).collect();
    let voting = o.groups.as_ref().filter(|g| !g.is_empty()).map_or_else(|| "all audience".to_string(), |g| g.iter().map(|x| name(x)).collect::<Vec<_>>().join(", "));
    Some(Panel {
        vote: o.vote.clone(),
        state,
        question: question(o),
        rule: rule(o.rule),
        n: o.n.unwrap_or(1),
        fallback: o.options.get(fallback).cloned().unwrap_or_default(),
        voting,
        options: o.options.clone(),
        seconds: (left.max(0) + 999) / 1000,
        countdown: (left.max(0) as f32 / total_ms).clamp(0.0, 1.0),
        bars,
        groups,
        group_votes,
        voters,
        expected: t.and_then(|t| t.expected),
        result: r.map(|r| (b.result_label().unwrap_or_default(), how(r.how))),
    })
}

/// The form as `vote.open` with the id `vote`, or what is wrong with it (deck-ui `Form::problem`).
pub fn open(f: &Form, vote: &str) -> Result<VoteOpen, String> {
    if let Some(p) = f.problem() {
        return Err(p.to_string());
    }
    let question = f.question.trim();
    Ok(VoteOpen {
        vote: vote.to_string(),
        options: f.option_list(),
        rule: match f.rule {
            Rule::Majority => VoteRule::Majority,
            Rule::Groups => VoteRule::Groups,
            Rule::First => VoteRule::First,
        },
        n: (f.rule == Rule::First).then_some(f.n),
        seconds: f.seconds,
        fallback: Some(f.fallback as i64),
        groups: (!f.groups.is_empty()).then(|| f.groups.clone()),
        show: Some(f.show),
        question: (!question.is_empty()).then(|| [("en".to_string(), question.to_string())].into_iter().collect()),
        labels: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse, ClockChoice, Logged, Subtree};
    use protocol::Envelope;
    use serde_json::json;

    fn wire(m: serde_json::Value) -> Envelope {
        serde_json::from_value(json!({ "v": 1, "seq": 1, "from": "relay", "kind": m["kind"], "p": m["p"] })).unwrap()
    }

    fn named(g: &str) -> String {
        if g == "left" { "Left side".into() } else { g.to_string() }
    }

    const SHOW: &str = r#"{ "id": "sermon", "version": 1, "outlets": [{ "id": "left" }], "rows": [{ "id": "start", "cells": { "left": { "mode": "black" } } }] }"#;

    #[test]
    fn the_show_runner_opens_watches_holds_and_overrides_a_vote_through_its_conductor() {
        let mut sub = Subtree::new(parse(SHOW).unwrap(), ClockChoice::Parent);
        assert!(sub.vote_close(None).is_empty() && sub.vote_hold(true).is_empty(), "nothing to close or hold");
        // the form → vote.open, out to the relay
        let form = Form { question: "Which way?".into(), options: "left, right, stay".into(), rule: Rule::Groups, fallback: 2, ..Form::default() };
        let out = sub.vote_open(&form, 1_000).unwrap();
        assert_eq!(out.len(), 1);
        let sent = serde_json::to_value(&out[0]).unwrap();
        assert_eq!((sent["kind"].as_str(), sent["p"]["vote"].as_str(), sent["p"]["rule"].as_str(), sent["p"]["fallback"].as_i64()), (Some("vote.open"), Some("v1"), Some("groups"), Some(2)));
        assert_eq!(sent["p"]["question"], json!({ "en": "Which way?" }));
        assert_eq!(sub.vote_open(&Form { options: "only".into(), ..Form::default() }, 1_000).unwrap_err(), "A vote needs at least two options (separated by commas).");
        // the relay says it is open, then tallies; the panel follows
        sub.on_wire(&wire(json!({ "kind": "vote.open", "p": sent["p"] })), 1_100);
        assert!(sub.vote().is_open());
        let p = panel(sub.vote(), &named).unwrap();
        assert_eq!((p.head().as_str(), p.question.as_str(), p.meta_line().as_str(), p.seconds), ("VOTE v1 · Open", "Which way?", "Rule per group · Fallback stay · Voting all audience", 30));
        let tally = json!({ "kind": "vote.tally", "p": { "vote": "v1", "counts": [10, 2, 0], "voters": 12, "left": 12_400, "expected": 15, "paused": true,
            "groups": { "left": [10, 0, 0], "right": [0, 1, 0], "balcony": [0, 1, 0] } } });
        sub.on_wire(&wire(tally), 1_200);
        let p = panel(sub.vote(), &named).unwrap();
        assert_eq!((p.state, p.seconds, p.voters_line().as_str(), p.hold_word()), (State::Held, 13, "All votes · 12 of 15 devices", "Run on"));
        assert_eq!(p.bars.iter().map(|b| (b.count, b.percent, b.lead)).collect::<Vec<_>>(), [(10, 83, true), (2, 17, false), (0, 0, false)]);
        assert_eq!(p.groups.iter().map(|g| (g.name.as_str(), g.pick)).collect::<Vec<_>>(), [("Left side", 0), ("balcony", 1), ("right", 1)], "each group's own pick: the relay's winner");
        assert_eq!(p.group_votes, [1, 2, 0]);
        // the moves go to the vote open now
        let hold = serde_json::to_value(&sub.vote_hold(false)[0]).unwrap();
        assert_eq!((hold["kind"].as_str(), &hold["p"]), (Some("vote.hold"), &json!({ "vote": "v1", "hold": false })));
        let over = serde_json::to_value(&sub.vote_close(Some(1))[0]).unwrap();
        assert_eq!((over["kind"].as_str(), &over["p"]), (Some("vote.close"), &json!({ "vote": "v1", "override": 1 })));
        // the result: closed, how, the log
        sub.on_wire(&wire(json!({ "kind": "vote.result", "p": { "vote": "v1", "option": 1, "counts": [10, 2, 0], "how": "override" } })), 1_300);
        let p = panel(sub.vote(), &named).unwrap();
        assert_eq!((p.state, p.can_act(), p.result_line().as_deref()), (State::Closed, false, Some("Result: right – by override – the conductor chose")));
        assert!(!sub.vote().is_open() && sub.vote_close(None).is_empty(), "a closed vote takes no moves");
        let log: Vec<String> = sub.log().filter(|e| matches!(e.what, Logged::VoteOpened { .. } | Logged::VoteDecided { .. })).map(|e| crate::view::line(&e.what)).collect();
        assert_eq!(log, ["vote v1 opened: Which way?", "vote v1: right (by override)"]);
        // a refusal is said; the next vote gets the next id
        sub.on_wire(&Envelope::new("relay", Message::Error(protocol::Error { code: "vote".into(), message: "a vote is open".into() })), 1_400);
        assert_eq!(sub.vote().problem.as_deref(), Some("a vote is open"));
        assert_eq!(sub.vote().next_id(), "v2");
    }

    #[test]
    fn first_to_n_draws_the_way_to_n_and_a_late_conductor_gets_the_open_vote_from_the_welcome() {
        let mut b = Board::default();
        let open: VoteOpen = serde_json::from_value(json!({ "vote": "v7", "options": ["Yes", "No"], "rule": "first", "n": 4, "seconds": 20 })).unwrap();
        assert!(b.heard(&Message::VoteOpen(open.clone())));
        assert!(!b.heard(&Message::VoteOpen(open)), "the same vote again is no news");
        b.heard(&Message::VoteTally(serde_json::from_value(json!({ "vote": "v7", "counts": [2, 1], "voters": 3, "left": 5_000 })).unwrap()));
        let p = panel(&b, &named).unwrap();
        assert_eq!((p.rule_line().as_str(), p.bars[0].fill, p.bars[1].fill, p.countdown), ("first to n · n 4", 0.5, 0.25, 0.25));
        assert!(!b.heard(&Message::VoteTally(serde_json::from_value(json!({ "vote": "v6", "counts": [9, 9], "voters": 18, "left": 1 })).unwrap())), "another vote's tally is not this one's");
        assert_eq!(panel(&b, &named).unwrap().voters, 3);
        assert_eq!(b.next_id(), "v1");
        let mut sub = Subtree::new(parse(SHOW).unwrap(), ClockChoice::Parent);
        let welcome = json!({ "kind": "welcome", "p": { "client": "c1", "now": 10, "snapshot": { "vote": { "vote": "v3", "options": ["a", "b"], "rule": "majority", "seconds": 10 } } } });
        sub.on_wire(&wire(welcome), 10);
        assert_eq!(panel(sub.vote(), &named).map(|p| p.vote), Some("v3".into()));
        assert_eq!(panel(&Board::default(), &named), None);
    }
}
