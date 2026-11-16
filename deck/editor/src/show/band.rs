//! The rows band, Show's and Perform's: the session grid of rows × columns under sticky heads (deck-kit `table`). A row's
//! head holds its ▶ (the row fires), its number, its name and what fires it; a column's head its group, its name, who
//! takes it and its stop; a cell its ▶ (launched by hand) and its words. Kept cells are a bare ↑. What plays is lime with
//! the bar's progress, what waits for its moment is ringed. Every word and state is deck-show's view.rs (`matrix_at`);
//! this file lays them out and names the places.
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::table::{Laid, Scroll, Table};
use deck_show::view::{Matrix, State};

/// what the band's presses mean where it is drawn
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mode {
    /// Perform: a press on a row's head fires it, on a cell launches it; Show: they select, the ▶ launches
    pub perform: bool,
    /// the conductor runs the document shown: the ▶ and the stops work (a sub conductor's document only shows)
    pub runs: bool,
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// the band's measures for `m`
pub fn table(t: &Tokens, m: &Matrix) -> Table {
    let s = &t.size;
    let groups = if m.groups.is_empty() { 0.0 } else { s.rem(1.6) };
    Table { head: (s.rem(13.0), groups + s.rem(3.2)), cell: (s.rem(11.0), s.rem(3.6)), rows: m.rows.len(), cols: m.columns.len(), foot: s.rem(1.6) }
}

/// a ▶ in a circle of radius `r` around `c`: filled lime while it plays, ringed while it waits, quiet else
fn launcher(d: &mut DrawList, t: &Tokens, c: (f32, f32), r: f32, state: State, hot: bool) {
    match state {
        State::Playing => d.disc(c.0, c.1, r, t.color.primary),
        State::Queued => {
            d.disc(c.0, c.1, r, t.color.control);
            d.ring(c.0, c.1, r, t.color.primary_line);
            d.ring(c.0, c.1, r - 1.5, t.color.primary_line);
        }
        State::Idle => {
            d.disc(c.0, c.1, r, if hot { t.color.hover } else { t.color.control });
            d.ring(c.0, c.1, r, t.color.line);
        }
    }
    let s = r * 0.9;
    let ink = if state == State::Playing { t.color.primary_ink } else { t.color.text };
    d.icon(c.0 - s * 0.42, c.1 - s * 0.5, s, "play-fill", ink);
}

fn push(host: &mut Host, r: Rect, region: &Rect, hit: Hit) {
    if let Some(r) = Laid::shown(&r, region) {
        host.hits.push((r, hit));
    }
}

/// The band in `area` (a panel's visible content), scrolled by `scroll`; its laid table (for the wheel).
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, area: Rect, (m, mode): (&Matrix, Mode), scroll: Scroll, pointer: (f32, f32)) -> Laid {
    let l = table(t, m).lay(area, scroll);
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let (cw, ch) = l.table.cell;
    let show = host.shown().clone();
    let roles = deck_show::roles::roles(&show, host.path.is_empty().then_some(&host.sub));
    let sel = (!mode.perform).then_some(host.row).filter(|_| host.role.is_none());

    // the cells, under both heads
    d.clip(Some(l.body));
    for (i, r) in m.rows.iter().enumerate().take(l.visible_rows().end).skip(l.visible_rows().start) {
        for (c, cell) in r.cells.iter().enumerate() {
            let box_ = l.cell(i, c);
            // #551: adjacent cells of one question are one wide cell, drawn from the column that starts it; a column inside
            // it draws nothing but answers its press as the cell
            let (span, words) = match &cell.ask {
                Some(a) => (a.run.max(1), a.words()),
                None => (1, Some(cell.text.clone())),
            };
            let whole = Rect { x: box_.x + 3.0, y: box_.y + 3.0, w: l.span(i, c, span).w - 6.0, h: ch - 6.0 };
            let selected = sel == Some(i) && m.columns.get(c).is_some_and(|col| host.cells.contains(&col.id));
            let at = Rect { x: box_.x + 3.0, y: box_.y + 3.0, w: cw - 6.0, h: ch - 6.0 };
            push(host, at, &l.body, if mode.perform { Hit::Launch(i, c) } else { Hit::Cell(i, c) });
            if let Some(words) = words.filter(|_| !cell.keep) {
                let over = whole.has(pointer.0, pointer.1);
                d.round_rect(whole.x, whole.y, whole.w, whole.h, t.size.radius, if over { t.color.hover } else { t.color.control });
                match cell.state {
                    State::Playing => d.outline(whole.x, whole.y, whole.w, whole.h, (t.size.radius, 2.0), if cell.hand { t.color.warn } else { t.color.primary_line }),
                    State::Queued => d.outline(whole.x, whole.y, whole.w, whole.h, (t.size.radius, 1.0), t.color.primary_line),
                    State::Idle => d.outline(whole.x, whole.y, whole.w, whole.h, (t.size.radius, 1.0), t.color.line),
                }
                if cell.ask.is_some() {
                    d.rect(whole.x, whole.y + t.size.radius, 2.0, whole.h - 2.0 * t.size.radius, t.color.accent);
                }
                let rad = (ch * 0.19).min(t.size.rem(0.8));
                let c0 = (whole.x + pad + rad, whole.y + pad * 0.6 + rad);
                launcher(d, t, c0, rad, cell.state, Rect { x: c0.0 - rad, y: c0.1 - rad, w: 2.0 * rad, h: 2.0 * rad }.has(pointer.0, pointer.1));
                let tx = c0.0 + rad + pad * 0.75;
                d.text(tx, c0.1 - font * 0.62, font, &fit(d, &words, font, whole.x + whole.w - tx - pad), t.color.text);
                // under it: what waits for which moment, a hand launch, why it would not play, who the question reaches
                let note = match (&cell.state, &cell.problem, &cell.ask) {
                    (State::Queued, ..) => Some((r.moment.clone(), t.color.text)),
                    (_, Some(p), _) => Some((p.clone(), t.color.warn)),
                    _ if cell.hand => Some(("hand".to_string(), t.color.warn)),
                    (_, _, Some(a)) => Some((a.filter.clone(), t.color.muted)),
                    _ => None,
                };
                if let Some((words, color)) = note {
                    controls::label(d, t, tx, whole.y + whole.h - pad * 0.6 - t.size.label * 1.4, &fit(d, &words, t.size.label * 1.2, whole.x + whole.w - tx - pad), color);
                }
                if let (State::Playing, Some(p)) = (cell.state, cell.progress) {
                    let (bx, bw) = (whole.x + pad, whole.w - 2.0 * pad);
                    let by = whole.y + whole.h - 4.0;
                    d.rect(bx, by, bw, 2.0, t.color.line);
                    d.rect(bx, by, bw * p.clamp(0.0, 1.0), 2.0, t.color.primary_line);
                }
                if mode.runs {
                    push(host, Rect { x: c0.0 - rad - 2.0, y: c0.1 - rad - 2.0, w: 2.0 * rad + 4.0, h: 2.0 * rad + 4.0 }, &l.body, Hit::Launch(i, c));
                }
            } else if cell.keep {
                // a kept cell: a bare arrow, lit while what an earlier row started plays on through this row
                let color = if cell.state == State::Playing { t.color.primary_line } else { t.color.muted };
                d.text(box_.x + pad + 2.0, box_.y + (ch - font * 1.25) * 0.5, font, &cell.text, color);
            }
            if selected {
                d.outline(whole.x, whole.y, whole.w, whole.h, (t.size.radius, 1.5), t.color.accent);
            }
            // a scene being dragged: the cell under the pointer is where it lands
            if !mode.perform && host.dragging.is_some() && at.has(pointer.0, pointer.1) {
                d.outline(at.x, at.y, at.w, at.h, (t.size.radius, 2.0), t.color.go);
            }
        }
        // #568: the cells of this row whose far nodes listen to each other, joined along the row's foot
        if !mode.perform {
            for link in host.row_links(&r.id) {
                let deck_show::far::Holder::Outlet(to) = &link.holder else { continue };
                let mid = |id: &str| m.columns.iter().position(|c| c.id == id).map(|c| l.cell(i, c).x + cw * 0.5);
                if let (Some(a), Some(b)) = (mid(&link.from), mid(to)) {
                    let fy = l.cell(i, 0).y + ch - 4.0;
                    d.line((a, fy), (b, fy), 1.0, t.color.accent);
                    d.disc(a, fy, 3.0, t.color.accent);
                    d.disc(b, fy, 3.0, t.color.accent);
                }
            }
        }
    }

    // the column heads: the groups over their columns, then each column's name, who takes it and its stop
    d.clip(Some(l.heads_top));
    let gh = if m.groups.is_empty() { 0.0 } else { t.size.rem(1.6) };
    for g in &m.groups {
        let r = l.col_head(g.first, g.span);
        controls::label(d, t, r.x + pad, r.y + 4.0, &fit(d, &g.name, t.size.label * 1.2, r.w - 2.0 * pad), t.color.muted);
        d.rect(r.x + pad, r.y + gh - 6.0, r.w - 2.0 * pad, 1.0, t.color.line);
    }
    for (c, col) in m.columns.iter().enumerate() {
        let r = l.col_head(c, 1);
        let y = r.y + gh;
        let role = roles.iter().position(|x| x.id == col.id);
        if role.is_some() && host.role.as_ref() == Some(&col.id) {
            d.round_rect(r.x + 2.0, y, r.w - 4.0, r.h - gh - 2.0, t.size.radius, t.color.selected);
        }
        let stop = t.size.rem(1.3);
        let here = role.map(|k| deck_show::roles::here_text(roles[k].here)).unwrap_or_default();
        let hw = d.advance(small) * here.chars().count() as f32;
        d.text(r.x + pad, y + 4.0, font, &fit(d, &col.name, font, r.w - 3.0 * pad - stop - hw), t.color.text);
        d.text(r.x + r.w - 2.0 * pad - stop - hw, y + 4.0 + (font - small) * 0.6, small, &here, t.color.muted);
        let kind = col.filter.as_ref().map_or_else(|| col.detail.clone(), |f| format!("{} · {f}", col.detail));
        controls::label(d, t, r.x + pad, y + 6.0 + font * 1.3, &fit(d, &kind, t.size.label * 1.2, r.w - 2.0 * pad), t.color.muted);
        if let Some(k) = role.filter(|_| !mode.perform) {
            push(host, Rect { x: r.x, y, w: r.w, h: r.h - gh }, &l.heads_top, Hit::Role(k));
        }
        // the column's stop: a square, red while something plays there
        let s = Rect { x: r.x + r.w - pad - stop, y: y + 4.0, w: stop, h: stop };
        let live = mode.runs && col.stops;
        d.round_rect(s.x, s.y, s.w, s.h, t.size.radius * 0.6, if live && s.has(pointer.0, pointer.1) { t.color.hover } else { t.color.control });
        d.outline(s.x, s.y, s.w, s.h, (t.size.radius * 0.6, 1.0), t.color.line);
        let i = s.w * 0.5;
        d.icon(s.x + (s.w - i) * 0.5, s.y + (s.h - i) * 0.5, i, "stop-fill", if live { t.color.stop } else { t.color.muted });
        if mode.runs {
            push(host, s, &l.heads_top, Hit::Stop(c));
        }
    }

    // the row heads: ▶, number, name, what fires it
    d.clip(Some(l.heads_left));
    for (i, r) in m.rows.iter().enumerate().take(l.visible_rows().end).skip(l.visible_rows().start) {
        let h = l.row_head(i);
        let whole = Rect { x: h.x, y: h.y + 3.0, w: h.w - pad, h: h.h - 6.0 };
        if sel == Some(i) {
            d.round_rect(whole.x, whole.y, whole.w, whole.h, t.size.radius, t.color.selected);
        } else if whole.has(pointer.0, pointer.1) {
            d.round_rect(whole.x, whole.y, whole.w, whole.h, t.size.radius, t.color.hover);
        }
        if r.suggested {
            d.outline(whole.x, whole.y, whole.w, whole.h, (t.size.radius, 2.0), t.color.warn);
        }
        push(host, whole, &l.heads_left, if mode.perform { Hit::Fire(i) } else { Hit::Row(i) });
        let rr = (h.h * 0.26).min(t.size.rem(1.0));
        let c0 = (h.x + pad + rr, h.y + h.h * 0.5);
        launcher(d, t, c0, rr, r.state, Rect { x: c0.0 - rr, y: c0.1 - rr, w: 2.0 * rr, h: 2.0 * rr }.has(pointer.0, pointer.1));
        if mode.runs {
            push(host, Rect { x: c0.0 - rr - 2.0, y: c0.1 - rr - 2.0, w: 2.0 * rr + 4.0, h: 2.0 * rr + 4.0 }, &l.heads_left, Hit::Fire(i));
        }
        let nx = c0.0 + rr + pad;
        let number = format!("{:02}", r.number);
        let nw = d.text(nx, h.y + h.h * 0.5 - font * 1.15, small, &number, t.color.muted);
        let tx = nx + nw + pad * 0.75;
        d.text(tx, h.y + h.h * 0.5 - font * 1.2, font, &fit(d, &r.id, font, whole.x + whole.w - tx - pad), t.color.text);
        controls::label(d, t, tx, h.y + h.h * 0.5 + 3.0, &fit(d, &r.fires, t.size.label * 1.2, whole.x + whole.w - tx - pad), t.color.muted);
    }

    // the foot: what is out of view
    d.clip(Some(area));
    if let Some(f) = l.foot {
        let (rows, cols) = l.counters();
        if let Some(rows) = rows {
            d.text(f.x + pad, f.y + (f.h - small * 1.25) * 0.5, small, &rows, t.color.muted);
        }
        if let Some(cols) = cols {
            let w = d.advance(small) * cols.chars().count() as f32;
            d.text(f.x + f.w - w - pad, f.y + (f.h - small * 1.25) * 0.5, small, &cols, t.color.muted);
        }
    }
    l
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{host_with, press, H, W};
    use super::super::{author, perform, Hit, Host};
    use deck_kit::base::draw::DrawList;
    use deck_kit::base::tokens::Tokens;
    use deck_show::view::{self, State};

    const DOC: &str = r#"{ "id": "band", "version": 1,
      "outlets": [{ "id": "left", "group": "stages" }, { "id": "right", "group": "stages" }, { "id": "lights", "kind": "device", "group": "light" }],
      "rows": [
        { "id": "opening", "cells": { "left": { "mode": "play", "piece": "scene:nave" }, "right": { "mode": "play", "piece": "scene:vigil" } } },
        { "id": "together", "fire": { "event": "meet", "how": "suggest" }, "cells": { "right": { "mode": "play", "piece": "scene:choir" } } }
      ] }"#;

    fn frame(host: &mut Host, perform_: bool) -> Vec<String> {
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        host.hits.clear();
        if perform_ {
            perform::draw(&mut d, &t, host, W, H, (0.0, 0.0));
        } else {
            author::draw(&mut d, &t, host, W, H, (0.0, 0.0));
        }
        d.texts().into_iter().map(str::to_string).collect()
    }

    fn place(host: &Host, want: Hit) -> Option<deck_kit::editor::controls::Rect> {
        host.hits.iter().rev().find(|(_, h)| *h == want).map(|(r, _)| *r)
    }

    fn state(host: &Host, row: usize, col: usize) -> State {
        view::matrix(&host.sub).rows[row].cells[col].state
    }

    #[test]
    fn in_show_a_cell_selects_its_launcher_launches_the_row_head_fires_and_a_column_stops() {
        let mut host = host_with(DOC);
        press(&mut host, false, |h| *h == Hit::Cell(0, 0));
        assert_eq!(host.cell().map(String::as_str), Some("left"), "a press on the cell selects it");
        assert_eq!(state(&host, 0, 0), State::Idle, "and launches nothing");
        press(&mut host, false, |h| *h == Hit::Launch(0, 0));
        assert_eq!(state(&host, 0, 0), State::Playing, "its ▶ launches it");
        assert_eq!(view::log(host.sub.log(), 1)[0].1, "hand: left > opening, off the row");
        // the row's ▶ sits in its head; the rest of the head selects the row
        press(&mut host, false, |h| *h == Hit::Row(1));
        assert_eq!((host.row, view::matrix(&host.sub).rows[1].state), (1, State::Idle));
        press(&mut host, false, |h| *h == Hit::Fire(1));
        let m = view::matrix(&host.sub);
        assert_eq!((m.rows[1].state, m.rows[1].cells[1].state, m.rows[1].cells[0].state), (State::Playing, State::Playing, State::Playing), "left keeps what row 1 started");
        press(&mut host, false, |h| *h == Hit::Stop(1));
        assert_eq!(state(&host, 1, 1), State::Idle, "right is stopped");
        assert_eq!(view::log(host.sub.log(), 1)[0].1, "hand: right stopped");
        let m = view::matrix(&host.sub);
        assert!(m.columns[0].stops && !m.columns[1].stops);
    }

    #[test]
    fn in_perform_the_head_fires_and_the_cell_launches() {
        let mut host = host_with(DOC);
        press(&mut host, true, |h| *h == Hit::Launch(1, 1));
        assert_eq!(state(&host, 1, 1), State::Playing);
        press(&mut host, true, |h| *h == Hit::Fire(0));
        assert_eq!((state(&host, 0, 0), state(&host, 0, 1)), (State::Playing, State::Playing));
        press(&mut host, true, |h| *h == Hit::Stop(0));
        assert_eq!(state(&host, 0, 0), State::Idle);
    }

    #[test]
    fn a_kept_cell_is_a_bare_arrow_and_the_groups_head_their_columns() {
        let mut host = host_with(DOC);
        host.two_d = false;
        let texts = frame(&mut host, false);
        assert_eq!(texts.iter().filter(|t| *t == "↑").count(), 3 + 1, "row 1's lights, row 2's left and lights; the inspector's lights: {texts:?}");
        assert!(place(&host, Hit::Cell(1, 0)).is_some(), "a kept cell is still a place to select");
        assert!(place(&host, Hit::Launch(1, 0)).is_none(), "but has no ▶ of its own");
        assert!(texts.iter().any(|t| t == "STAGES") && texts.iter().any(|t| t == "LIGHT"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "01") && texts.iter().any(|t| t == "opening") && texts.iter().any(|t| t == "MEET · SUGGEST"), "{texts:?}");
        let stop = |host: &Host, c| place(host, Hit::Stop(c)).unwrap();
        assert_eq!(stop(&host, 0).y, stop(&host, 2).y);
        // a sub conductor's document only shows: no ▶, no stop
        host.path.push("x".into());
        frame(&mut host, false);
        assert!(!host.hits.iter().any(|(_, h)| matches!(h, Hit::Launch(..) | Hit::Fire(_) | Hit::Stop(_))));
    }

    #[test]
    fn a_band_too_large_scrolls_both_ways_under_its_heads_and_counts_what_is_out_of_view() {
        let outlets: Vec<String> = (0..12).map(|i| format!(r#"{{ "id": "o{i}" }}"#)).collect();
        let rows: Vec<String> = (0..30).map(|i| format!(r#"{{ "id": "r{i}", "cells": {{ "o0": {{ "mode": "play", "piece": "scene:nave" }} }} }}"#)).collect();
        let doc = format!(r#"{{ "id": "big", "version": 1, "outlets": [{}], "rows": [{}] }}"#, outlets.join(","), rows.join(","));
        let mut host = host_with(&doc);
        let texts = frame(&mut host, true);
        let counter = |texts: &[String], end: &str| texts.iter().find(|t| t.starts_with('+') && t.ends_with(end)).cloned();
        let (rows0, cols0) = (counter(&texts, "↓").expect("rows out of view"), counter(&texts, "→").expect("columns out of view"));
        let row_head = place(&host, Hit::Fire(0)).unwrap();
        let stop = place(&host, Hit::Stop(0)).unwrap();
        assert!(host.band_wheel(row_head.x + 4.0, row_head.y + 4.0, (-2.0, -3.0)));
        assert!(host.perform_scroll.x > 0.0 && host.perform_scroll.y > 0.0);
        let texts = frame(&mut host, true);
        assert!(place(&host, Hit::Fire(0)).is_none(), "row 1 scrolled away under the column heads");
        assert_eq!(place(&host, Hit::Fire(3)).unwrap().x, row_head.x, "a row head stays at the left");
        assert_eq!(place(&host, Hit::Stop(2)).unwrap().y, stop.y, "a column head stays on top");
        assert!(place(&host, Hit::Stop(0)).is_none(), "column 1 scrolled away under the row heads");
        assert_ne!(counter(&texts, "↓").unwrap(), rows0);
        assert_ne!(counter(&texts, "→").unwrap(), cols0);
        assert!(!host.band_wheel(1.0, 1.0, (0.0, -1.0)), "the wheel elsewhere leaves it");
    }
}
