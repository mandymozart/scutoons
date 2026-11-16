//! The Perform workspace (#91, the owner's draft m2-perform), floating panels over the stage: the cues – the matrix of
//! rows × outlets and seats, the big row ▶ (rows are the main way to launch), a small ▶ per cell (a hand launch, marked off
//! its row) –, the log, the session rail, the suggestion with GO / Drop and GO itself, the stage now in 2D. All words and
//! states are deck-show's view.rs; this file lays them out and names the places (`Hit`).
//!
//! #533 (owner 2026-11-14 [live]: the show runner works from Perform): the show runner's sheets open in a panel of their
//! own – **Session** (session.rs: the linked reader's link, its QR, the linked readers) and **Vote** (vote.rs: the vote's
//! tally per group, Hold / Close now / Override, the result); the rail names the linked readers and the vote.
use super::{Hit, Host, Sheet};
use deck_kit::editor::controls::{self, Rect};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_show::view;

/// the show runner's sheets over the stage
pub const SHEETS: [(&str, Sheet); 2] = [("Session", Sheet::Session), ("Vote", Sheet::Vote)];

/// the show runner's sheet open now (none: the stage)
pub fn open_sheet(host: &Host) -> Option<Sheet> {
    host.sheet.filter(|s| s.in_perform())
}

/// `text` cut to what fits in `w` at `px`
fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, w: f32, h: f32, pointer: (f32, f32)) {
    let (prims, hits) = (d.prims().len(), host.hits.len());
    draw_once(d, t, host, (w, h), pointer);
    // a content that grew is laid out again before the frame shows
    if host.perform_panels.take_grew() {
        d.truncate(prims);
        host.hits.truncate(hits);
        draw_once(d, t, host, (w, h), pointer);
        host.perform_panels.take_grew();
    }
}

fn draw_once(d: &mut DrawList, t: &Tokens, host: &mut Host, (w, h): (f32, f32), pointer: (f32, f32)) {
    let sheet = open_sheet(host);
    let m = view::matrix_at(&host.sub, super::now());
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let big = font * 1.35;
    super::author::view_switch(d, t, host, w, &SHEETS);
    // the panels shown and what each asks for: the matrix as wide as its columns, a row of cues as tall as two lines
    let matrix = super::band::table(t, &m).size();
    let outlets = host.sub.conductor().show().outlets.len().max(1) as f32;
    let mut shown: Vec<(&'static str, f32, Option<f32>)> = vec![("rail", t.size.tree, None), ("side", 320.0, None)];
    if sheet.is_some() {
        shown.push(("sheet", 600.0, Some(420.0)));
    } else if host.two_d {
        shown.push(("now", outlets * (220.0 + pad), Some(2.6 * font * 1.25 + small * 2.0)));
    }
    shown.extend([("log", 520.0, Some(small * 2.2 + 8.0 * font * 1.35)), ("matrix", matrix.0, Some(matrix.1)), ("info", 260.0, Some(small * 1.4 * 6.0))]);
    host.perform_panels.layout(d, t, ((w, h), crate::bar::bottom(t)), &[], &shown);

    // the show runner's sheet
    if let Some(o) = host.perform_panels.open(d, t, "sheet", pointer, host.hits.len()) {
        match sheet {
            Some(Sheet::Vote) => super::vote::draw(d, t, host, o.area, pointer),
            _ => super::session::draw(d, t, host, o.area, pointer),
        }
        host.perform_panels.close(d, t, o, &mut host.hits);
    }
    // the stage now: what plays on each outlet
    if let Some(o) = host.perform_panels.open(d, t, "now", pointer, host.hits.len()) {
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
            d.text(bx + pad, by + 4.0 + font * 1.4, small, &fit(d, playing.as_deref().unwrap_or("nothing yet"), small, bw - 2.0 * pad), t.color.accent);
        }
        host.perform_panels.close(d, t, o, &mut host.hits);
    }

    // the cues: the rows band, a press on a row fires it, on a cell launches it
    if let Some(o) = host.perform_panels.open(d, t, "matrix", pointer, host.hits.len()) {
        let laid = super::band::draw(d, t, host, o.area, (&m, super::band::Mode { perform: true, runs: true }), host.perform_scroll, pointer);
        host.perform_scroll = laid.scroll;
        host.drew_band(true, laid);
        host.perform_panels.close(d, t, o, &mut host.hits);
    }

    // the log, newest first
    if let Some(o) = host.perform_panels.open(d, t, "log", pointer, host.hits.len()) {
        let a = o.area;
        let lines = ((a.h - small * 0.4) / (font * 1.35)).max(0.0) as usize;
        for (i, (time, words)) in view::log(host.sub.log(), lines).iter().enumerate() {
            let y = a.y + i as f32 * font * 1.35;
            let tw = d.text(a.x, y, font, time, t.color.muted);
            let warn = words.contains("nobody") || words.contains("skipped") || words.contains("off the row");
            d.text(a.x + tw + pad, y, font, &fit(d, words, font, a.w - tw - pad), if warn { t.color.warn } else { t.color.text });
        }
        host.perform_panels.close(d, t, o, &mut host.hits);
    }

    // the rail: this conductor, its seats, the far side, the outlets
    if let Some(o) = host.perform_panels.open(d, t, "rail", pointer, host.hits.len()) {
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
        d.text(a.x, y, small, &fit(d, &format!("relay · {}", host.link_line()), small, a.w), t.color.text);
        y += t.size.tree_row;
        // #533: the linked readers at a glance (the Session sheet has the link and the list)
        let readers = host.session_panel().readers_line();
        d.text(a.x, y, small, &fit(d, &readers, small, a.w), t.color.text);
        host.hits.push((Rect { x: a.x, y: y - 4.0, w: a.w, h: t.size.tree_row }, Hit::Sheet(Sheet::Session)));
        y += t.size.tree_row;
        // #533: the vote at a glance (the Vote sheet opens, holds and closes it)
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
        host.perform_panels.close(d, t, o, &mut host.hits);
    }

    // the side: the suggestion with GO / Drop, else GO; then the seats; what the last press came to
    if let Some(o) = host.perform_panels.open(d, t, "side", pointer, host.hits.len()) {
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
        if !host.status.is_empty() {
            d.text(sx, y, small, &fit(d, &host.status, small, sw), t.color.muted);
        }
        host.perform_panels.close(d, t, o, &mut host.hits);
    }

    if let Some(o) = host.perform_panels.open(d, t, "info", pointer, host.hits.len()) {
        deck_kit::editor::info::body(d, t, (o.area.x, o.area.y, o.area.w), host.help.as_ref());
        host.perform_panels.close(d, t, o, &mut host.hits);
    }
}

/// a filled button with its word centred
pub fn button(d: &mut DrawList, t: &Tokens, r: Rect, label: &str, ground: [f32; 4], ink: [f32; 4]) {
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, ground);
    let px = t.size.font * 1.3;
    let tw = d.advance(px) * label.chars().count() as f32;
    d.text(r.x + (r.w - tw) * 0.5, r.y + (r.h - px * 1.25) * 0.5, px, label, ink);
}
