//! The Vote sheet in Perform (#533, research/SHOW.md §14; the vote #231): the show runner opens a vote, watches its tally
//! per group, holds it, closes it now or overrides it, and reads the result. What it says is deck-ui `vote` (the panel, the
//! form's words and checks), filled by deck-show `vote::panel` from what the relay said (the conductor's `Board`); the moves
//! are the conductor's (`Subtree::vote_open / vote_hold / vote_close`), sent through the relay link. The rule is the relay's;
//! this file lays out, takes the typed fields and the presses (RULES A1, A2).
use super::roles::Typing;
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_ui::vote::{Form, Panel, State, HOLD_HELP, NONE, RULES};

/// What the sheet keeps: the form being typed, whether it shows over a closed vote, Override's option row open.
#[derive(Debug, Default)]
pub struct Voting {
    pub form: Form,
    /// the form instead of the last vote's result (New vote)
    pub new: bool,
    pub overriding: bool,
}

/// A press on the Vote sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VoteHit {
    Question,
    Options,
    Rule(usize),
    N,
    Seconds,
    Fallback(usize),
    Group(usize),
    Show,
    Open,
    Hold,
    Override,
    With(usize),
    Cancel,
    Close,
    New,
}

impl Host {
    /// a group's name: the show's role of that id, else the id
    fn group_name(&self, id: &str) -> String {
        deck_show::roles::roles(self.sub.conductor().show(), None).into_iter().find(|r| r.id == id).map_or_else(|| id.to_string(), |r| r.name)
    }

    /// the panel as the core words it; none without a vote
    pub fn vote_panel(&self) -> Option<Panel> {
        deck_show::vote::panel(self.sub.vote(), &|g| self.group_name(g))
    }

    /// the form shows: no vote yet, or New vote over a closed one
    fn vote_form_shows(&self) -> bool {
        self.voting.new || self.sub.vote().open.is_none()
    }

    pub(super) fn vote_press(&mut self, hit: VoteHit) {
        let f = &mut self.voting.form;
        match hit {
            VoteHit::Question => self.typing = Some((Typing::VoteQuestion, Field::with(&f.question))),
            VoteHit::Options => self.typing = Some((Typing::VoteOptions, Field::with(&f.options))),
            VoteHit::N => self.typing = Some((Typing::VoteN, Field::with(&f.n.to_string()))),
            VoteHit::Seconds => self.typing = Some((Typing::VoteSeconds, Field::with(&f.seconds.to_string()))),
            VoteHit::Rule(i) => f.rule = RULES.get(i).copied().unwrap_or_default(),
            VoteHit::Fallback(i) => f.fallback = i,
            VoteHit::Group(i) => {
                if let Some(id) = deck_show::roles::roles(self.sub.conductor().show(), None).get(i).map(|r| r.id.clone()) {
                    let f = &mut self.voting.form;
                    match f.groups.iter().position(|g| *g == id) {
                        Some(at) => {
                            f.groups.remove(at);
                        }
                        None => f.groups.push(id),
                    }
                }
            }
            VoteHit::Show => f.show = !f.show,
            VoteHit::Open => match self.sub.vote_open(&self.voting.form, super::now()) {
                Ok(out) => {
                    self.voting.new = false;
                    self.voting.overriding = false;
                    self.status = format!("vote opened: {}", self.voting.form.summary());
                    self.send(out);
                }
                Err(why) => self.status = why,
            },
            VoteHit::Hold => {
                let held = self.vote_panel().is_some_and(|p| p.state == State::Held);
                let out = self.sub.vote_hold(!held);
                self.send(out);
            }
            VoteHit::Override => self.voting.overriding = true,
            VoteHit::Cancel => self.voting.overriding = false,
            VoteHit::With(i) => {
                self.voting.overriding = false;
                let out = self.sub.vote_close(Some(i as i64));
                self.send(out);
            }
            VoteHit::Close => {
                let out = self.sub.vote_close(None);
                self.send(out);
            }
            VoteHit::New => self.voting.new = true,
        }
    }

    /// a typed field of the form applied (Enter)
    pub(super) fn vote_typed(&mut self, what: &Typing, text: &str) {
        let f = &mut self.voting.form;
        let number = |s: &str| s.trim().parse::<i64>().ok();
        match what {
            Typing::VoteQuestion => f.question = text.trim().to_string(),
            Typing::VoteOptions => {
                f.options = text.trim().to_string();
                f.fallback = f.fallback.min(f.option_list().len().saturating_sub(1));
            }
            Typing::VoteN => match number(text) {
                Some(n) => f.n = n,
                None => self.status = "n: a whole number".into(),
            },
            Typing::VoteSeconds => match number(text) {
                Some(s) => f.seconds = s,
                None => self.status = "Seconds: a whole number".into(),
            },
            _ => {}
        }
    }
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// a labelled row of chips (`on` lit); their places and the height it took
fn chips(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), label: &str, names: &[String], on: &[bool]) -> (Vec<Rect>, f32) {
    let (small, row, lw) = (t.size.small, t.size.tree_row, d.advance(t.size.small) * 9.0);
    d.text(x, y + (row - small * 1.25) * 0.5, small, label, t.color.muted);
    let words: Vec<&str> = names.iter().map(String::as_str).collect();
    let (r, used) = controls::chips(d, t, (x + lw, y + 2.0, w - lw), &words, on);
    (r, used.max(row - 2.0) + 4.0)
}

/// a labelled typed field: its place
fn field(d: &mut DrawList, t: &Tokens, host: &Host, (x, y, w): (f32, f32, f32), label: &str, what: Typing, value: &str, empty: &str) -> Rect {
    let (font, small, row, lw) = (t.size.font, t.size.small, t.size.tree_row, d.advance(t.size.small) * 9.0);
    d.text(x, y + (row - small * 1.25) * 0.5, small, label, t.color.muted);
    let r = Rect { x: x + lw, y: y + 2.0, w: (w - lw).min(420.0), h: row - 4.0 };
    let typing = host.typing.as_ref().filter(|(k, _)| *k == what).map(|(_, f)| format!("{}|", f.text));
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.control);
    if typing.is_some() {
        d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.0), t.color.accent);
    }
    let (text, ink) = match (&typing, value.is_empty()) {
        (Some(s), _) => (s.clone(), t.color.text),
        (None, true) => (empty.to_string(), t.color.muted),
        (None, false) => (value.to_string(), t.color.text),
    };
    d.text(r.x + 6.0, r.y + (r.h - font * 1.25) * 0.5, font, &fit(d, &text, font, r.w - 12.0), ink);
    r
}

/// a button sized to its word at `x`; its place
fn button(d: &mut DrawList, t: &Tokens, (x, y): (f32, f32), word: &str, primary: bool) -> Rect {
    let r = Rect { x, y, w: d.advance(t.size.font * 1.3) * word.chars().count() as f32 + 2.0 * t.size.pad, h: t.size.tree_row };
    let (ground, ink) = if primary { (t.color.go, t.color.surface) } else { (t.color.control, t.color.text) };
    super::perform::button(d, t, r, word, ground, ink);
    r
}

/// "Vote" in Perform's stage place: the form, or the panel of the vote open (or last closed).
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, _pointer: (f32, f32)) {
    let (pad, small) = (t.size.pad, t.size.small);
    match (host.vote_form_shows(), host.vote_panel()) {
        (false, Some(p)) => draw_panel(d, t, host, a, &p),
        _ => draw_form(d, t, host, a),
    }
    if let Some(why) = host.sub.vote().problem.clone() {
        d.text(a.x + pad, a.y + a.h - small * 2.0, small, &fit(d, &format!("the relay: {why}"), small, a.w - 2.0 * pad), t.color.warn);
    }
}

fn draw_form(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect) {
    let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.tree_row);
    let (x, w) = (a.x + pad, a.w - 2.0 * pad);
    let mut y = a.y + pad;
    d.text(x, y, small, "OPEN A VOTE", t.color.muted);
    y += small * 1.8;
    let f = host.voting.form.clone();
    let mut hits: Vec<(Rect, VoteHit)> = Vec::new();
    hits.push((field(d, t, host, (x, y, w), "Question", Typing::VoteQuestion, &f.question, "none – the options alone"), VoteHit::Question));
    y += row;
    hits.push((field(d, t, host, (x, y, w), "Options", Typing::VoteOptions, &f.options, "Yes, No – separated by commas"), VoteHit::Options));
    y += row;
    let rules: Vec<String> = RULES.iter().map(|r| r.name().to_string()).collect();
    let lit: Vec<bool> = RULES.iter().map(|r| *r == f.rule).collect();
    let (rects, used) = chips(d, t, (x, y, w), "Rule", &rules, &lit);
    hits.extend(rects.into_iter().enumerate().map(|(i, r)| (r, VoteHit::Rule(i))));
    y += used;
    d.text(x + d.advance(small) * 9.0, y, small, &fit(d, f.rule.help(), small, w), t.color.muted);
    y += small * 1.8;
    if f.rule == deck_ui::vote::Rule::First {
        hits.push((field(d, t, host, (x, y, w.min(260.0)), "n", Typing::VoteN, &f.n.to_string(), ""), VoteHit::N));
        y += row;
    }
    hits.push((field(d, t, host, (x, y, w.min(260.0)), "Seconds", Typing::VoteSeconds, &f.seconds.to_string(), ""), VoteHit::Seconds));
    y += row;
    let options = f.option_list();
    let lit: Vec<bool> = (0..options.len()).map(|i| i == f.fallback).collect();
    let (rects, used) = chips(d, t, (x, y, w), "Fallback", &options, &lit);
    hits.extend(rects.into_iter().enumerate().map(|(i, r)| (r, VoteHit::Fallback(i))));
    y += used;
    let roles = deck_show::roles::roles(host.sub.conductor().show(), None);
    let names: Vec<String> = roles.iter().map(|r| r.name.clone()).collect();
    let lit: Vec<bool> = roles.iter().map(|r| f.groups.contains(&r.id)).collect();
    let (rects, used) = chips(d, t, (x, y, w), "Voting", &names, &lit);
    hits.extend(rects.into_iter().enumerate().map(|(i, r)| (r, VoteHit::Group(i))));
    y += used;
    let all = if f.groups.is_empty() { "none picked: all audience" } else { "only the groups picked" };
    d.text(x + d.advance(small) * 9.0, y, small, all, t.color.muted);
    y += small * 1.8;
    let show = Rect { x: x + d.advance(small) * 9.0, y, w: d.advance(small) * 30.0, h: row - 4.0 };
    controls::toggle(d, t, show, if f.show { "voters see the counts: yes" } else { "voters see the counts: no" }, false);
    d.text(x, y + (row - 4.0 - small * 1.25) * 0.5, small, "Counts", t.color.muted);
    hits.push((show, VoteHit::Show));
    y += row + pad * 0.5;
    match f.problem() {
        Some(p) => d.text(x, y, font, &fit(d, p, font, w), t.color.warn),
        None => d.text(x, y, font, &fit(d, &f.summary(), font, w), t.color.text),
    };
    y += font * 1.8;
    hits.push((button(d, t, (x, y), "Open vote", f.problem().is_none()), VoteHit::Open));
    host.hits.extend(hits.into_iter().map(|(r, h)| (r, Hit::Vote(h))));
}

fn draw_panel(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, p: &Panel) {
    let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.tree_row);
    let big = font * 1.35;
    let (x, w) = (a.x + pad, a.w - 2.0 * pad);
    let mut y = a.y + pad;
    let state_ink = match p.state {
        State::Open => t.color.go,
        State::Held => t.color.warn,
        State::Closed => t.color.muted,
    };
    d.text(x, y, small, &p.head(), state_ink);
    y += small * 1.8;
    if !p.question.is_empty() {
        d.text(x, y, big, &fit(d, &p.question, big, w), t.color.text);
        y += big * 1.5;
    }
    d.text(x, y, small, &fit(d, &p.meta_line(), small, w), t.color.muted);
    y += small * 1.8;
    // the countdown, or the result
    match p.result_line() {
        None => {
            let sw = d.text(x, y, big, &p.seconds_line(), t.color.text);
            let (tx, tw) = (x + sw + pad, (w * 0.5 - sw - pad).max(40.0));
            d.outline(tx, y + big * 0.35, tw, 8.0, (2.0, 1.0), t.color.line);
            d.rect(tx, y + big * 0.35, tw * p.countdown, 8.0, state_ink);
        }
        Some(r) => {
            d.rect(x, y - 2.0, 4.0, big * 1.3, t.color.go);
            d.text(x + pad, y, big, &fit(d, &r, big, w - pad), t.color.text);
        }
    }
    y += big * 1.6;
    // a bar per option
    d.text(x, y, small, &p.voters_line().to_uppercase(), t.color.muted);
    y += small * 1.8;
    let (lw, nw) = ((w * 0.18).clamp(80.0, 180.0), d.advance(font) * 10.0);
    let bw = (w * 0.6 - lw - nw).max(60.0);
    for b in &p.bars {
        let ink = if b.lead { t.color.text } else { t.color.muted };
        d.text(x, y, font, &fit(d, &b.label, font, lw - pad), ink);
        d.outline(x + lw, y, bw, font * 1.2, (2.0, 1.0), t.color.line);
        d.rect(x + lw, y, bw * b.fill.clamp(0.0, 1.0), font * 1.2, if b.lead { t.color.accent } else { t.color.control });
        d.text(x + lw + bw + pad, y, font, &format!("{} · {}%", b.count, b.percent), ink);
        y += font * 1.7;
    }
    // per group: its split and its pick (the rule per group)
    if !p.groups.is_empty() {
        y += pad * 0.5;
        d.text(x, y, small, &p.groups_line().to_uppercase(), t.color.muted);
        y += small * 1.8;
        let cw = ((w * 0.6 - lw) / p.options.len().max(1) as f32).max(60.0);
        let rows: Vec<(String, Vec<String>, Option<usize>)> = p
            .groups
            .iter()
            .map(|g| (g.name.clone(), g.cells.iter().zip(&p.options).map(|((pc, n), o)| format!("{o} {pc}% ({n})")).collect(), Some(g.pick)))
            .chain(std::iter::once(("Group votes".to_string(), p.group_votes.iter().zip(&p.options).map(|(n, o)| format!("{o} {n}")).collect(), None)))
            .collect();
        for (name, cells, pick) in rows {
            if y + row > a.y + a.h - row * 2.5 {
                break;
            }
            d.text(x, y + 2.0, small, &fit(d, &name, small, lw - pad), t.color.text);
            for (i, c) in cells.iter().enumerate() {
                let cx = x + lw + i as f32 * cw;
                if pick == Some(i) {
                    d.round_rect(cx, y, cw - 4.0, small * 1.6, t.size.radius, t.color.selected);
                }
                d.text(cx + 4.0, y + 2.0, small, &fit(d, c, small, cw - 8.0), if pick == Some(i) || pick.is_none() { t.color.text } else { t.color.muted });
            }
            y += small * 1.9;
        }
    }
    // the moves while it is open; New vote after
    y = y.max(a.y + a.h - row * 2.6);
    let mut hits: Vec<(Rect, VoteHit)> = Vec::new();
    if p.can_act() {
        let mut bx = x;
        if host.voting.overriding {
            let tw = d.text(bx, y + (row - small * 1.25) * 0.5, small, "OVERRIDE WITH", t.color.muted);
            bx += tw + pad;
            for (i, o) in p.options.iter().enumerate() {
                let r = button(d, t, (bx, y), o, false);
                bx += r.w + pad;
                hits.push((r, VoteHit::With(i)));
            }
            hits.push((button(d, t, (bx, y), "Cancel", false), VoteHit::Cancel));
        } else {
            for (word, hit, primary) in [(p.hold_word(), VoteHit::Hold, false), ("Override …", VoteHit::Override, false), ("Close now", VoteHit::Close, true)] {
                let r = button(d, t, (bx, y), word, primary);
                bx += r.w + pad;
                hits.push((r, hit));
            }
        }
        d.text(x, y + row + 4.0, small, &fit(d, HOLD_HELP, small, w), t.color.muted);
    } else {
        hits.push((button(d, t, (x, y), "New vote", true), VoteHit::New));
    }
    host.hits.extend(hits.into_iter().map(|(r, h)| (r, Hit::Vote(h))));
}

/// the rail's line for the vote: `vote v1 · Open · 12s · 7 votes`, `vote v1 · right`, or none
pub fn rail_line(host: &Host) -> String {
    match host.vote_panel() {
        None => NONE.trim_end_matches('.').to_lowercase(),
        Some(p) => match &p.result {
            Some((label, _)) => format!("vote {} · {label}", p.vote),
            None => format!("vote {} · {} · {} · {} votes", p.vote, p.state.word(), p.seconds_line(), p.voters),
        },
    }
}

/// A check run's vote (`--vote QUESTION;OPTION, OPTION…[;RULE]`, RULE `majority`, `groups` or `first:N`): the form.
pub fn form_arg(v: &str) -> Form {
    let mut p = v.splitn(3, ';');
    let mut f = Form { question: p.next().unwrap_or_default().trim().to_string(), ..Form::default() };
    if let Some(o) = p.next().filter(|o| !o.trim().is_empty()) {
        f.options = o.trim().to_string();
    }
    match p.next().map(str::trim) {
        Some("groups") => f.rule = deck_ui::vote::Rule::Groups,
        Some(r) if r.starts_with("first") => {
            f.rule = deck_ui::vote::Rule::First;
            f.n = r.split_once(':').and_then(|(_, n)| n.trim().parse().ok()).unwrap_or(f.n);
        }
        _ => {}
    }
    f
}

impl Host {
    /// A check run's vote: the Vote sheet, the vote opened from `form`; without a relay, the relay's answers as it would
    /// say them – the vote open, then `tally` (`left=10/0/0,right=0/1/0` per group, or `10/2/0`) – handed to the conductor.
    pub(super) fn script_vote(&mut self, form: &Form, tally: Option<&str>) {
        self.voting.form = form.clone();
        self.press(Hit::Sheet(super::Sheet::Vote));
        let ms = super::now();
        let out = match self.sub.vote_open(form, ms) {
            Ok(out) => out,
            Err(why) => return self.status = why,
        };
        if self.link.is_some() {
            return self.send(out);
        }
        for mut e in out {
            e.from = "relay".into();
            self.sub.on_wire(&e, ms);
        }
        let (Some(t), Some(open)) = (tally, self.sub.vote().open.clone()) else { return };
        let n = open.options.len();
        let mut counts = vec![0i64; n];
        let mut groups = serde_json::Map::new();
        for part in t.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let (group, split) = part.split_once('=').map_or((None, part), |(g, s)| (Some(g.trim()), s));
            let c: Vec<i64> = split.split('/').map(|x| x.trim().parse().unwrap_or(0)).chain(std::iter::repeat(0)).take(n).collect();
            counts.iter_mut().zip(&c).for_each(|(a, b)| *a += b);
            if let Some(g) = group {
                groups.insert(g.to_string(), serde_json::json!(c));
            }
        }
        let voters: i64 = counts.iter().sum();
        let p = serde_json::json!({ "vote": open.vote, "counts": counts, "voters": voters, "left": open.seconds * 400, "expected": voters + 3, "groups": (!groups.is_empty()).then_some(groups) });
        if let Ok(t) = serde_json::from_value::<protocol::VoteTally>(p) {
            self.sub.on_wire(&protocol::Envelope::new("relay", protocol::Message::VoteTally(t)), ms);
        }
    }
}
