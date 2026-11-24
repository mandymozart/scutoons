//! A Show tab running its show (decisions/deck.md 2026-11-18 [workspaces]: no Perform workspace, its panels are optional
//! panels of Show). Go live joins the relay as the show's conductor (the decks, the readers, the vote); End leaves it. Only
//! one show is live in the editor: going live with a second asks to end the first (live_tabs.rs). The lock refuses every
//! edit of the document while it is on – the live night's, not the deck's editor lock (deck.toml `editor`).
//!
//! The optional panels, closed until the show goes live: **Next** (the suggestion with GO / Drop, else GO; the seats),
//! **Stage now** (what plays on each outlet), **Session** (this conductor, its seats, the linked readers, the vote, the
//! outlets). The rows band is the cue matrix (band.rs). All words and states are deck-show's view.rs; this file lays them
//! out and names the places (`Hit`).
use super::{nodes::NodeHit, values::KnobHit, Hit, Host, Sheet};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_show::choose::{Prompt, Tone};
use deck_show::view::{self, Matrix};

/// what an edit is told while the lock is on
pub const LOCKED: &str = "locked for the live night: Unlock in the toolbar to edit";

/// A press that changes the document or puts another in its place: refused while the lock is on. Selecting, launching,
/// GO, the sheets, the session and the vote stay.
pub fn edits(hit: Hit) -> bool {
    match hit {
        Hit::Nodes(n) => !matches!(n, NodeHit::Canvas | NodeHit::Node(_) | NodeHit::NodeAlso(_)),
        Hit::Knob(k) => !matches!(k, KnobHit::Build | KnobHit::Prelisten),
        Hit::Sheet(s) => matches!(s, Sheet::Scene | Sheet::NewShow | Sheet::Demos | Sheet::Event),
        _ => matches!(
            hit,
            Hit::RoleAdd
                | Hit::RoleName
                | Hit::RoleFact(_)
                | Hit::RoleRemove
                | Hit::RowAdd
                | Hit::RowName
                | Hit::RowRemove
                | Hit::CellMode(_)
                | Hit::CellPiece
                | Hit::CellShot
                | Hit::CellFact(_)
                | Hit::AddressFact(_)
                | Hit::Slot(_)
                | Hit::EventScope(_)
                | Hit::EventPick(_)
                | Hit::EventType
                | Hit::Background
                | Hit::BackgroundColour
                | Hit::Port(..)
                | Hit::Ask(_)
                | Hit::Compose(_)
                | Hit::Demo(_)
                | Hit::Load
                | Hit::NewName
                | Hit::ToonFind
                | Hit::ToonPick(_)
        ),
    }
}

/// the prompt before going live while `other` is live
pub fn end_prompt(other: &str, this: &str) -> Prompt {
    let lines = vec![
        (format!("Only one show is live: {other} ends – its decks and readers leave the relay – and {this} goes live."), Tone::Warn),
        ("Its tab stays open; Go live there brings it back.".to_string(), Tone::Muted),
    ];
    Prompt { title: format!("End {other} and go live with {this}?"), lines, warn: true, confirm: "Go live".into(), can: true }
}

impl Host {
    /// whether the show is live (joined to a relay)
    pub fn live(&self) -> bool {
        self.link.is_some()
    }

    /// Go live: join `relay` as the show's conductor; the running show's panels open.
    pub fn go_live(&mut self, relay: &str) {
        if !self.live() {
            match super::link::Link::start(relay, std::env::var("SCUTOONS_LIVE_TOKEN").ok()) {
                Ok(l) => {
                    self.link = Some(l);
                    self.status = format!("{} is live on {relay}", self.sub.conductor().show().id);
                    self.bad = false;
                }
                Err(e) => return self.refuse(format!("relay: {e}")),
            }
        }
        for id in crate::panels::OPTIONAL {
            self.show_panels.floats.reopen(id);
        }
    }

    /// End: the show leaves the relay (its decks and readers with it); the lock comes off.
    pub fn end_live(&mut self) {
        if self.link.take().is_some() {
            self.status = format!("{} ended", self.sub.conductor().show().id);
        }
        self.locked = false;
    }

    /// the lock on or off (only a live show is locked)
    pub fn flip_lock(&mut self) {
        self.locked = self.live() && !self.locked;
        self.typing = None;
        self.status = if self.locked { "locked: launching and GO only, no edits".into() } else { "unlocked".into() };
    }

    /// a show `other` is live elsewhere: ask before this one goes live
    pub fn ask_end(&mut self, other: String) {
        self.asking = Some(super::roles::Ask::EndLive(other));
    }

    /// the confirmed ask, taken once
    pub fn take_live_asked(&mut self) -> bool {
        std::mem::take(&mut self.live_asked)
    }
}

/// `text` cut to what fits in `w` at `px`
fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// what the optional panels ask for this frame (their place in the Show tab's layout)
pub fn shown(t: &Tokens, host: &Host) -> Vec<(&'static str, f32, Option<f32>)> {
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let outlets = host.sub.conductor().show().outlets.len().max(1) as f32;
    vec![("session", t.size.tree, None), ("now", outlets * (220.0 + pad), Some(2.6 * font * 1.25 + small * 2.0)), ("next", 320.0, None)]
}

/// The optional panels that are open: Stage now, Session, Next.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, m: &Matrix, pointer: (f32, f32)) {
    now(d, t, host, pointer);
    session(d, t, host, m, pointer);
    next(d, t, host, m, pointer);
}

/// the stage now: what plays on each outlet
fn now(d: &mut DrawList, t: &Tokens, host: &mut Host, pointer: (f32, f32)) {
    let Some(o) = host.show_panels.open(d, t, "now", pointer, host.hits.len()) else { return };
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let a = o.area;
    let c = host.sub.conductor();
    let outlets: Vec<&deck_show::Outlet> = c.show().outlets.iter().collect();
    let bw = ((a.w + pad) / outlets.len().max(1) as f32 - pad).min(260.0);
    for (i, o) in outlets.iter().enumerate() {
        let (bx, by) = (a.x + i as f32 * (bw + pad), a.y);
        let playing = c.on_stage(&o.id).map(|s| view::playing_text(&s.playing));
        d.round_rect(bx, by, bw, 2.6 * font * 1.25, t.size.radius, t.color.surface_head);
        d.outline(bx, by, bw, 2.6 * font * 1.25, (t.size.radius, 1.0), t.color.line);
        d.text(bx + pad, by + 4.0, font, &o.id, t.color.text);
        d.text(bx + pad, by + 4.0 + font * 1.4, small, &fit(d, playing.as_deref().unwrap_or("nothing yet"), small, bw - 2.0 * pad), t.color.text);
    }
    host.show_panels.close(d, o, &mut host.hits);
}

/// the session: this conductor, its seats, the far side, the outlets
fn session(d: &mut DrawList, t: &Tokens, host: &mut Host, m: &Matrix, pointer: (f32, f32)) {
    let Some(o) = host.show_panels.open(d, t, "session", pointer, host.hits.len()) else { return };
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let a = o.area;
    let mut y = a.y;
    let rail = |d: &mut DrawList, y: &mut f32, label: &str| {
        d.text(a.x, *y + 4.0, small, label, t.color.muted);
        *y += t.size.tree_row;
    };
    rail(d, &mut y, "CONDUCTORS");
    let c = host.sub.conductor();
    let main_detail = m.rows.iter().find(|r| r.current).map_or("–".to_string(), |r| format!("row {}", r.number));
    let mut rows = vec![controls::TreeRow { depth: 0, label: c.show().id.clone(), detail: main_detail, icon: "nodes", open: None, selected: true }];
    for s in view::seats(c.show()) {
        rows.push(controls::TreeRow { depth: 1, label: s.id, detail: s.detail.replace("sub · ", ""), icon: "folder", open: None, selected: false });
    }
    let n = rows.len();
    controls::tree(d, t, a.x, y, a.w, &rows, pointer);
    y += n as f32 * t.size.tree_row + pad;
    rail(d, &mut y, "FROM FAR");
    let readers = host.session_panel().readers_line();
    d.text(a.x, y, small, &fit(d, &readers, small, a.w), t.color.text);
    host.hits.push((Rect { x: a.x, y: y - 4.0, w: a.w, h: t.size.tree_row }, Hit::Sheet(Sheet::Session)));
    y += t.size.tree_row;
    let vote = super::vote::rail_line(host);
    d.text(a.x, y, small, &fit(d, &vote, small, a.w), if host.sub.vote().is_open() { t.color.go } else { t.color.muted });
    host.hits.push((Rect { x: a.x, y: y - 4.0, w: a.w, h: t.size.tree_row }, Hit::Sheet(Sheet::Vote)));
    y += t.size.tree_row + pad;
    rail(d, &mut y, "OUTLETS");
    let c = host.sub.conductor();
    for o in &c.show().outlets {
        let on = c.on_stage(&o.id).is_some();
        d.disc(a.x + 4.0, y + font * 0.65, 4.0, if on { t.color.go } else { t.color.muted });
        d.text(a.x + 16.0, y, font, &o.id, t.color.text);
        y += t.size.tree_row;
    }
    y += pad;
    super::cue::rail(d, t, host, a.x, y, a.w);
    host.show_panels.close(d, o, &mut host.hits);
}

/// next: the suggestion with GO / Drop, else GO; then the seats
fn next(d: &mut DrawList, t: &Tokens, host: &mut Host, m: &Matrix, pointer: (f32, f32)) {
    let Some(o) = host.show_panels.open(d, t, "next", pointer, host.hits.len()) else { return };
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let big = font * 1.35;
    let (sx, sw) = (o.area.x, o.area.w);
    let mut y = o.area.y;
    // the head, the title, why, then the buttons (40) – nothing under them
    let card_h = pad + small * 1.5 + big * 1.4 + small * 1.6 + pad + 40.0 + pad;
    match &m.suggestion {
        Some(s) => {
            d.round_rect(sx, y, sw, card_h, t.size.radius, t.color.surface_head);
            d.outline(sx, y, sw, card_h, (t.size.radius, 2.0), t.color.warn);
            d.text(sx + pad, y + pad, small, "SUGGESTED", t.color.warn);
            d.text(sx + pad, y + pad + small * 1.5, big, &fit(d, &s.title, big, sw - 2.0 * pad), t.color.text);
            d.text(sx + pad, y + pad + small * 1.5 + big * 1.4, small, &fit(d, &s.why, small, sw - 2.0 * pad), t.color.muted);
            let by = y + card_h - 48.0;
            let go = Rect { x: sx + pad, y: by, w: (sw - 3.0 * pad) * 0.68, h: 40.0 };
            let drop = Rect { x: go.x + go.w + pad, y: by, w: sw - 3.0 * pad - go.w, h: 40.0 };
            controls::primary(d, t, go, "GO", false);
            button(d, t, drop, "Drop", t.color.control, t.color.text);
            host.hits.push((go, Hit::Go));
            host.hits.push((drop, Hit::Drop));
        }
        None => {
            d.round_rect(sx, y, sw, card_h, t.size.radius, t.color.surface_head);
            d.text(sx + pad, y + pad, small, "NEXT", t.color.muted);
            d.text(sx + pad, y + pad + small * 1.5, font, &fit(d, &m.go, font, sw - 2.0 * pad), t.color.text);
            let go = Rect { x: sx + pad, y: y + card_h - 48.0, w: sw - 2.0 * pad, h: 40.0 };
            controls::primary(d, t, go, "GO", false);
            host.hits.push((go, Hit::Go));
        }
    }
    y += card_h + pad;
    for s in view::seats(host.sub.conductor().show()) {
        let ch = font * 1.4 + small * 2.8 + 2.0 * pad;
        d.round_rect(sx, y, sw, ch, t.size.radius, t.color.surface_head);
        d.text(sx + pad, y + pad, font, &format!("{} · {}", s.id, s.detail), t.color.text);
        d.text(sx + pad, y + pad + font * 1.4, small, &fit(d, &format!("{} · {}", s.show, s.sits), small, sw - 2.0 * pad), t.color.muted);
        d.text(sx + pad, y + pad + font * 1.4 + small * 1.4, small, "own operator, own rows", t.color.muted);
        y += ch + pad;
    }
    host.show_panels.close(d, o, &mut host.hits);
}

/// a filled button with its word centred
pub fn button(d: &mut DrawList, t: &Tokens, r: Rect, label: &str, ground: [f32; 4], ink: [f32; 4]) {
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, ground);
    let px = t.size.font * 1.3;
    let tw = d.advance(px) * label.chars().count() as f32;
    d.text(r.x + (r.w - tw) * 0.5, r.y + (r.h - px * 1.25) * 0.5, px, label, ink);
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{host_with, press};

    const DOC: &str = r#"{ "id": "sermon", "version": 1, "toons": ["catpope"], "outlets": [{ "id": "left" }], "rows": [] }"#;
    use super::*;

    #[test]
    fn go_live_opens_the_running_panels_and_end_takes_the_lock_off() {
        let mut host = host_with(DOC);
        assert!(!host.live());
        for id in crate::panels::OPTIONAL {
            assert!(host.show_panels.floats.closed(id), "{id}: closed until live");
        }
        host.flip_lock();
        assert!(!host.locked, "only a live show locks");
        host.go_live("ws://127.0.0.1:9/live/TEST");
        assert!(host.live());
        for id in crate::panels::OPTIONAL {
            assert!(!host.show_panels.floats.closed(id), "{id}: open once live");
        }
        host.flip_lock();
        assert!(host.locked);
        host.end_live();
        assert!(!host.live() && !host.locked);
    }

    #[test]
    fn the_lock_refuses_edits_and_keeps_launching() {
        let mut host = host_with(DOC);
        host.go_live("ws://127.0.0.1:9/live/TEST");
        host.flip_lock();
        let rows = host.shown().rows.len();
        host.press(Hit::RowAdd);
        assert_eq!(host.shown().rows.len(), rows, "no row added");
        assert!(host.bad && host.status == LOCKED);
        (host.status, host.bad) = (String::new(), false);
        press(&mut host, |h| *h == Hit::Go);
        press(&mut host, |h| *h == Hit::Sheet(Sheet::Vote));
        assert!(!host.bad && host.sheet == Some(Sheet::Vote), "GO and the Vote sheet stay: {}", host.status);
        host.flip_lock();
        host.press(Hit::RowAdd);
        assert_eq!(host.shown().rows.len(), rows + 1, "unlocked: the row is added");
    }

    #[test]
    fn what_edits_and_what_runs() {
        for h in [Hit::RowAdd, Hit::CellPiece, Hit::Port(0, 0, true, 0), Hit::Knob(KnobHit::Turn(0)), Hit::Load, Hit::Sheet(Sheet::NewShow), Hit::Nodes(NodeHit::Group)] {
            assert!(edits(h), "{h:?}");
        }
        for h in [Hit::Fire(0), Hit::Launch(0, 0), Hit::Go, Hit::Drop, Hit::Stop(0), Hit::Row(0), Hit::Cell(0, 0), Hit::Sheet(Sheet::Vote), Hit::Sheet(Sheet::Session), Hit::Knob(KnobHit::Build), Hit::Knob(KnobHit::Prelisten), Hit::CueMix, Hit::Nodes(NodeHit::Node(0))] {
            assert!(!edits(h), "{h:?}");
        }
    }

    #[test]
    fn the_prompt_names_both_shows() {
        let p = end_prompt("sermon", "feilchen");
        assert_eq!(p.title, "End sermon and go live with feilchen?");
        assert!(p.warn && p.can);
    }
}
