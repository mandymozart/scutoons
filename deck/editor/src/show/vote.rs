//! The Vote sheet in Perform (#533, research/SHOW.md §14; the vote #231): the show runner watches the tally of the vote
//! open per group, holds it, closes it now or overrides it, and reads the result. What it says is deck-ui `vote` (the
//! panel), filled by deck-show `vote::panel` from what the relay said (the conductor's `Board`); the moves are the
//! conductor's (`Subtree::vote_hold / vote_close`), sent through the relay link. The rule is the relay's; this file lays
//! out and takes the presses (RULES A1, A2). #551 (SHOW.md §15 6; owner 2026-11-14 [live]: "Spontaneous votes dont
//! exist"): a vote opens only from a row's question – its row fired, or the ▶ of its wide cell in the matrix –, so the
//! sheet has no form and no Open; a question that is not the story's is typed in Show's cell inspector.
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_ui::vote::{Panel, State, FROM_A_ROW, HOLD_HELP, NONE};

/// What the sheet keeps: Override's option row open.
#[derive(Debug, Default)]
pub struct Voting {
    pub overriding: bool,
}

/// A press on the Vote sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VoteHit {
    Hold,
    Override,
    With(usize),
    Cancel,
    Close,
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

    pub(super) fn vote_press(&mut self, hit: VoteHit) {
        match hit {
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
        }
    }
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// a button sized to its word at `x`; its place
fn button(d: &mut DrawList, t: &Tokens, (x, y): (f32, f32), word: &str, primary: bool) -> Rect {
    let r = Rect { x, y, w: d.advance(t.size.font * 1.3) * word.chars().count() as f32 + 2.0 * t.size.pad, h: t.size.tree_row };
    let (ground, ink) = if primary { (t.color.pressed, t.color.pressed_text) } else { (t.color.control, t.color.text) };
    super::live::button(d, t, r, word, ground, ink);
    r
}

/// "Vote" in Perform's stage place: the panel of the vote open (or last closed); without one, where a vote comes from.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, _pointer: (f32, f32)) {
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    match host.vote_panel() {
        Some(p) => draw_panel(d, t, host, a, &p),
        None => {
            d.text(a.x + pad, a.y + pad, font, NONE, t.color.text);
            d.text(a.x + pad, a.y + pad + font * 1.6, small, &fit(d, FROM_A_ROW, small, a.w - 2.0 * pad), t.color.muted);
        }
    }
    if let Some(why) = host.sub.vote().problem.clone() {
        d.text(a.x + pad, a.y + a.h - small * 2.0, small, &fit(d, &format!("the relay: {why}"), small, a.w - 2.0 * pad), t.color.warn);
    }
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
        d.rect(x + lw, y, bw * b.fill.clamp(0.0, 1.0), font * 1.2, if b.lead { t.color.text } else { t.color.control });
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
    // the moves while it is open
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

impl Host {
    /// A check run's vote (`--launch ROW,COLUMN --vote-tally …`): the Vote sheet, the vote the cell's question opens (its ▶,
    /// the conductor's hand launch); without a relay, the relay's answers as it would say them – the vote open, then `tally`
    /// (`left=10/0/0,right=0/1/0` per group, or `10/2/0`) – handed to the conductor.
    pub(super) fn script_vote(&mut self, row: &str, column: &str, tally: &str) {
        self.press(Hit::Sheet(super::Sheet::Vote));
        let ms = super::now();
        let out = self.sub.launch_cell(row, column, ms);
        if !out.iter().any(|e| matches!(e.message, protocol::Message::VoteOpen(_))) {
            return self.status = format!("--vote-tally: row {row}, {column} asks no question");
        }
        if self.link.is_some() {
            return self.send(out);
        }
        for mut e in out {
            e.from = "relay".into();
            self.sub.on_wire(&e, ms);
        }
        let (t, Some(open)) = (tally, self.sub.vote().open.clone()) else { return };
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
