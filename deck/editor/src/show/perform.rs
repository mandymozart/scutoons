//! The Perform workspace (#91, the owner's draft m2-perform): the stage now on top, under it the matrix of rows × outlets
//! and seats – the big row ▶ (rows are the main way to launch), a small ▶ per cell (a hand launch, marked off its row) –,
//! the log at the bottom; the conductors rail on the left; the suggestion with GO / Drop and GO itself on the right. All
//! words and states are deck-show's view.rs; this file lays them out and names the places (`Hit`).
use super::{Hit, Host};
use deck_kit::controls::{self, Rect};
use deck_kit::draw::DrawList;
use deck_kit::tokens::Tokens;
use deck_show::view;

pub struct Layout {
    pub rail: Rect,
    pub stage: Rect,
    pub matrix: Rect,
    pub log: Rect,
    pub side: Rect,
}

pub fn layout(w: f32, h: f32, t: &Tokens) -> Layout {
    let top = t.size.top;
    let rail_w = t.size.tree.min(w * 0.2);
    let side_w = (w * 0.24).clamp(240.0, 460.0);
    let cx = rail_w;
    let cw = (w - rail_w - side_w).max(1.0);
    let body = (h - top).max(1.0);
    let stage_h = (body * 0.27).max(80.0);
    let log_h = (body * 0.2).clamp(90.0, 240.0);
    Layout {
        rail: Rect { x: 0.0, y: top, w: rail_w, h: body },
        stage: Rect { x: cx, y: top, w: cw, h: stage_h },
        matrix: Rect { x: cx, y: top + stage_h, w: cw, h: (body - stage_h - log_h).max(1.0) },
        log: Rect { x: cx, y: h - log_h, w: cw, h: log_h },
        side: Rect { x: w - side_w, y: top, w: side_w, h: body },
    }
}

fn faded(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

/// `text` cut to what fits in `w` at `px`
fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, w: f32, h: f32, pointer: (f32, f32)) {
    let l = layout(w, h, t);
    let m = view::matrix(&host.sub);
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let big = font * 1.35;
    // the regions' grounds (the stage shows through its own rect)
    d.rect(l.rail.x, l.rail.y, l.rail.w, l.rail.h, t.color.surface);
    d.rect(l.rail.x + l.rail.w - 1.0, l.rail.y, 1.0, l.rail.h, t.color.line);
    d.rect(l.matrix.x, l.matrix.y, l.matrix.w, l.matrix.h + l.log.h, t.color.surface_head);
    d.rect(l.side.x, l.side.y, l.side.w, l.side.h, t.color.surface);
    d.rect(l.side.x, l.side.y, 1.0, l.side.h, t.color.line);

    // the stage now: what plays on each outlet, over the dimmed stage (2D) or the stage alone (3D)
    super::author::view_switch(d, t, host, l.stage);
    d.text(l.stage.x + pad, l.stage.y + pad, small, "THE STAGE NOW", t.color.muted);
    if host.two_d {
        d.rect(l.stage.x, l.stage.y, l.stage.w, l.stage.h, faded(t.color.stage, 0.55));
        let c = host.sub.conductor();
        let outlets: Vec<&deck_show::Outlet> = c.show().outlets.iter().collect();
        let bw = ((l.stage.w - pad) / outlets.len().max(1) as f32 - pad).min(260.0);
        for (i, o) in outlets.iter().enumerate() {
            let (bx, by) = (l.stage.x + pad + i as f32 * (bw + pad), l.stage.y + 2.0 * pad + small * 1.6);
            let playing = c.on_stage(&o.id).map(|s| view::playing_text(&s.playing));
            d.round_rect(bx, by, bw, 2.6 * font * 1.25, t.size.radius, faded(t.color.surface, 0.9));
            d.outline(bx, by, bw, 2.6 * font * 1.25, (t.size.radius, 1.0), t.color.line);
            d.text(bx + pad, by + 4.0, font, &o.id, t.color.text);
            d.text(bx + pad, by + 4.0 + font * 1.4, small, &fit(d, playing.as_deref().unwrap_or("nothing yet"), small, bw - 2.0 * pad), t.color.accent);
        }
    }

    // the matrix: a head, then one line per row
    let head_h = t.size.row;
    let row_w = (l.matrix.w * 0.22).clamp(150.0, 240.0);
    let cols = m.columns.len().max(1) as f32;
    let col_w = ((l.matrix.w - row_w - 2.0 * pad) / cols).max(40.0);
    let rows_h = l.matrix.h - head_h - pad;
    let rh = (rows_h / m.rows.len().max(1) as f32).clamp(34.0, 92.0);
    let (mx, my) = (l.matrix.x + pad, l.matrix.y + pad);
    d.text(mx, my + 6.0, small, "rows", t.color.muted);
    for (c, col) in m.columns.iter().enumerate() {
        let x = mx + row_w + c as f32 * col_w;
        let label = if col.seat { format!("{} · {}", col.id, col.detail) } else { col.id.clone() };
        d.text(x + 4.0, my + 6.0, small, &fit(d, &label, small, col_w - 8.0), t.color.muted);
    }
    for (r, row) in m.rows.iter().enumerate() {
        let y = my + head_h + r as f32 * rh;
        if y + rh > l.matrix.y + l.matrix.h + 1.0 {
            break;
        }
        // the row ▶: green when it is the row that fired last, outlined when suggested
        let b = Rect { x: mx, y: y + 3.0, w: rh - 6.0, h: rh - 6.0 };
        let hot = Rect { x: mx, y, w: row_w - pad, h: rh }.has(pointer.0, pointer.1);
        d.round_rect(b.x, b.y, b.w, b.h, t.size.radius, if row.current { t.color.go } else if hot { t.color.hover } else { t.color.control });
        if row.suggested {
            d.outline(b.x, b.y, b.w, b.h, (t.size.radius, 2.0), t.color.warn);
        }
        let s = (b.h * 0.42).min(22.0);
        d.icon(b.x + (b.w - s) * 0.5, b.y + (b.h - s) * 0.5, s, "play-fill", if row.current { t.color.surface } else { t.color.text });
        let tx = b.x + b.w + pad;
        d.text(tx, y + rh * 0.5 - font * 1.3, font, &fit(d, &format!("{} {}", row.number, row.id), font, row_w - b.w - 2.0 * pad), t.color.text);
        d.text(tx, y + rh * 0.5 + 2.0, small, &fit(d, &row.fires, small, row_w - b.w - 2.0 * pad), t.color.muted);
        host.hits.push((Rect { x: mx, y, w: row_w - pad, h: rh }, Hit::Fire(r)));
        for (c, cell) in row.cells.iter().enumerate() {
            let cr = Rect { x: mx + row_w + c as f32 * col_w + 2.0, y: y + 3.0, w: col_w - 6.0, h: rh - 6.0 };
            if cell.keep {
                d.text(cr.x + 6.0, cr.y + 4.0, font, &cell.text, t.color.muted);
                continue;
            }
            let over = cr.has(pointer.0, pointer.1);
            d.round_rect(cr.x, cr.y, cr.w, cr.h, t.size.radius, if over { t.color.hover } else { t.color.control });
            if cell.on {
                // what plays now; a hand launch off its row is marked in the warning colour
                d.outline(cr.x, cr.y, cr.w, cr.h, (t.size.radius, 2.0), if cell.hand { t.color.warn } else { t.color.go });
            }
            d.icon(cr.x + 5.0, cr.y + 5.0, 12.0, "play-fill", t.color.muted);
            d.text(cr.x + 22.0, cr.y + 4.0, font, &fit(d, &cell.text, font, cr.w - 28.0), t.color.text);
            let mut note = cr.y + 4.0 + font * 1.45;
            if cell.hand {
                d.text(cr.x + 6.0, note, small, "hand", t.color.warn);
                note += small * 1.3;
            }
            if let Some(p) = &cell.problem {
                if note + small < cr.y + cr.h {
                    d.text(cr.x + 6.0, note, small, &fit(d, p, small, cr.w - 12.0), t.color.warn);
                }
            }
            host.hits.push((cr, Hit::Launch(r, c)));
        }
    }

    // the log, newest first
    d.rect(l.log.x, l.log.y, l.log.w, 1.0, t.color.line);
    d.text(l.log.x + pad, l.log.y + 6.0, small, "LOG", t.color.muted);
    let lines = ((l.log.h - small * 2.4) / (font * 1.35)).max(0.0) as usize;
    for (i, (time, words)) in view::log(host.sub.log(), lines).iter().enumerate() {
        let y = l.log.y + small * 2.2 + i as f32 * font * 1.35;
        let tw = d.text(l.log.x + pad, y, font, time, t.color.muted);
        let warn = words.contains("nobody") || words.contains("skipped") || words.contains("off the row");
        d.text(l.log.x + pad + tw + pad, y, font, &fit(d, words, font, l.log.w - tw - 3.0 * pad), if warn { t.color.warn } else { t.color.text });
    }

    // the rail: this conductor, its seats, the far side, the outlets
    let mut y = l.rail.y + pad;
    let rail = |d: &mut DrawList, y: &mut f32, label: &str| {
        d.text(pad, *y + 4.0, small, label, t.color.muted);
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
    controls::tree(d, t, 0.0, y, l.rail.w - 1.0, &rows, pointer);
    y += n as f32 * t.size.tree_row + pad;
    rail(d, &mut y, "FROM FAR");
    d.text(pad, y, small, &fit(d, &format!("relay · {}", host.link_line()), small, l.rail.w - 2.0 * pad), t.color.text);
    y += t.size.tree_row + pad;
    rail(d, &mut y, "OUTLETS");
    for o in &c.show().outlets {
        let on = c.on_stage(&o.id).is_some();
        d.disc(pad + 4.0, y + font * 0.65, 4.0, if on { t.color.go } else { t.color.muted });
        d.text(pad + 16.0, y, font, &o.id, t.color.text);
        y += t.size.tree_row;
    }

    // the side: the suggestion with GO / Drop, else GO; then the seats
    let (sx, sw) = (l.side.x + pad, l.side.w - 2.0 * pad);
    let mut y = l.side.y + pad;
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
            button(d, t, go, "GO", t.color.go, t.color.surface);
            button(d, t, drop, "Drop", t.color.control, t.color.text);
            host.hits.push((go, Hit::Go));
            host.hits.push((drop, Hit::Drop));
        }
        None => {
            d.round_rect(sx, y, sw, card_h, t.size.radius, t.color.surface_head);
            d.text(sx + pad, y + pad, small, "NEXT", t.color.muted);
            d.text(sx + pad, y + pad + small * 1.5, font, &fit(d, &m.go, font, sw - 2.0 * pad), t.color.text);
            let go = Rect { x: sx + pad, y: y + card_h - 48.0, w: sw - 2.0 * pad, h: 40.0 };
            button(d, t, go, "GO", t.color.go, t.color.surface);
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
        d.text(sx, l.side.y + l.side.h - small * 2.0, small, &fit(d, &host.status, small, sw), t.color.muted);
    }
}

/// a filled button with its word centred
pub fn button(d: &mut DrawList, t: &Tokens, r: Rect, label: &str, ground: [f32; 4], ink: [f32; 4]) {
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, ground);
    let px = t.size.font * 1.3;
    let tw = d.advance(px) * label.chars().count() as f32;
    d.text(r.x + (r.w - tw) * 0.5, r.y + (r.h - px * 1.25) * 0.5, px, label, ink);
}
