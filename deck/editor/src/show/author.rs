//! The Show workspace (#91, the owner's draft m1-show): the conductors tree on the left (this deck's conductor, its seats –
//! a seat enters its sub conductor, the breadcrumb leads back) and the rows of the document shown; in the middle the 2D
//! node overlay over the dimmed stage (#446): what comes from far (the reader's events, the clock), the cells of the row
//! and what the row sends, with a 2D / 3D switch; the rows band under it; on the right the row's inspector, the seats,
//! save and load. The words are deck-show's view.rs; this file lays them out and names the places (`Hit`).
use super::perform::button;
use super::{Hit, Host, Sheet};
use deck_kit::controls::{self, Rect};
use deck_kit::draw::DrawList;
use deck_kit::tokens::Tokens;
use deck_show::{view, Show};

pub struct Layout {
    pub left: Rect,
    pub stage: Rect,
    pub band: Rect,
    pub right: Rect,
}

pub fn layout(w: f32, h: f32, t: &Tokens) -> Layout {
    let top = t.size.top;
    let left_w = (t.size.tree * 1.3).min(w * 0.22);
    let right_w = (w * 0.22).clamp(240.0, 420.0);
    let cw = (w - left_w - right_w).max(1.0);
    let body = (h - top).max(1.0);
    let band_h = (body * 0.27).clamp(120.0, 300.0);
    Layout {
        left: Rect { x: 0.0, y: top, w: left_w, h: body },
        stage: Rect { x: left_w, y: top, w: cw, h: (body - band_h).max(1.0) },
        band: Rect { x: left_w, y: h - band_h, w: cw, h: band_h },
        right: Rect { x: w - right_w, y: top, w: right_w, h: body },
    }
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// the 2D / 3D switch at the top of a stage rect; with `sheets` (Show) also the cue sheet and the problems list
pub fn view_switch(d: &mut DrawList, t: &Tokens, host: &mut Host, stage: Rect, sheets: bool) {
    let names: &[&str] = if sheets { &["3D", "2D", "Cues", "Problems"] } else { &["3D", "2D"] };
    let h = t.size.row - 4.0;
    let w: f32 = names.iter().map(|n| d.advance(t.size.small) * n.chars().count().max(2) as f32 + 2.0 * t.size.pad + 2.0).sum();
    let x = stage.x + stage.w - w - t.size.pad;
    let y = stage.y + t.size.pad;
    d.round_rect(x - 3.0, y - 3.0, w + 4.0, h + 6.0, t.size.radius, t.color.surface_head);
    let on = match host.sheet.filter(|_| sheets) {
        Some(Sheet::Cues) => 2,
        Some(Sheet::Problems) => 3,
        Some(Sheet::Flash) => usize::MAX,
        None => usize::from(host.two_d),
    };
    let r = controls::pills(d, t, x, y, h, names, on);
    let hits = [Hit::View(false), Hit::View(true), Hit::Sheet(Sheet::Cues), Hit::Sheet(Sheet::Problems)];
    for (r, hit) in r.into_iter().zip(hits) {
        host.hits.push((r, hit));
    }
}

/// one node of the overlay: a head (name, detail) and its lines (a dot before an input, after an output)
fn node(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), head: (&str, &str), lines: &[(String, bool)], far: bool) -> f32 {
    let (font, small, pad) = (t.size.font, t.size.small, t.size.pad);
    let lh = font * 1.5;
    let h = lh + lines.len() as f32 * lh + pad;
    d.round_rect(x, y, w, h, t.size.radius, [t.color.surface[0], t.color.surface[1], t.color.surface[2], 0.92]);
    d.outline(x, y, w, h, (t.size.radius, 1.0), if far { t.color.muted } else { t.color.line });
    d.text(x + pad, y + 4.0, font, head.0, t.color.text);
    let dw = d.advance(small) * head.1.chars().count() as f32;
    d.text(x + w - dw - pad, y + 5.0, small, head.1, t.color.muted);
    d.rect(x, y + lh, w, 1.0, t.color.line);
    for (i, (text, out)) in lines.iter().enumerate() {
        let ly = y + lh + 4.0 + i as f32 * lh;
        if *out {
            let tw = d.advance(font) * text.chars().count() as f32;
            d.text(x + w - tw - 2.0 * pad - 4.0, ly, font, &fit(d, text, font, w - 3.0 * pad), t.color.text);
            d.disc(x + w - pad - 2.0, ly + font * 0.62, 3.5, t.color.accent);
        } else {
            d.disc(x + pad + 2.0, ly + font * 0.62, 3.5, t.color.text);
            d.text(x + 2.0 * pad + 4.0, ly, font, &fit(d, text, font, w - 3.0 * pad - 4.0), t.color.text);
        }
    }
    h
}

pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, w: f32, h: f32, pointer: (f32, f32)) {
    let l = layout(w, h, t);
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let show = host.shown().clone();
    let plan = view::plan(&show);
    host.row = host.row.min(show.rows.len().saturating_sub(1));
    let sel = show.rows.get(host.row).map(|r| r.id.clone()).unwrap_or_default();
    d.rect(l.left.x, l.left.y, l.left.w, l.left.h, t.color.surface);
    d.rect(l.left.x + l.left.w - 1.0, l.left.y, 1.0, l.left.h, t.color.line);
    d.rect(l.right.x, l.right.y, l.right.w, l.right.h, t.color.surface);
    d.rect(l.right.x, l.right.y, 1.0, l.right.h, t.color.line);
    d.rect(l.band.x, l.band.y, l.band.w, l.band.h, t.color.surface_head);
    d.rect(l.band.x, l.band.y, l.band.w, 1.0, t.color.line);

    // left: the conductors (the top, then the seats of the document shown), then its rows
    let mut y = l.left.y + pad;
    d.text(pad, y + 4.0, small, "CONDUCTORS", t.color.muted);
    y += t.size.tree_row;
    let depth = host.path.len();
    let mut tree = vec![controls::TreeRow { depth: 0, label: show.id.clone(), detail: if depth == 0 { "this deck".into() } else { format!("sub · {}", host.path.last().cloned().unwrap_or_default()) }, icon: "nodes", open: Some(true), selected: true }];
    let seats = view::seats(&show);
    for s in &seats {
        let picked = host.seat == seats.iter().position(|x| x.id == s.id);
        tree.push(controls::TreeRow { depth: 1, label: s.id.clone(), detail: s.detail.clone(), icon: "folder", open: Some(false), selected: picked });
    }
    for (i, r) in controls::tree(d, t, 0.0, y, l.left.w - 1.0, &tree, pointer).into_iter().enumerate() {
        host.hits.push((r, if i == 0 { Hit::Crumb(depth) } else { Hit::Enter(i - 1) }));
    }
    y += tree.len() as f32 * t.size.tree_row + pad;
    // #478: the roles this conductor gives out, with how many are in each (the running top only: a sub conductor's
    // document shown here runs elsewhere)
    let roles = deck_show::roles::roles(&show, host.path.is_empty().then_some(&host.sub));
    d.text(pad, y + 4.0, small, "ROLES", t.color.muted);
    y += t.size.tree_row;
    let role_rows: Vec<controls::TreeRow> = roles
        .iter()
        .map(|r| controls::TreeRow { depth: 0, label: r.name.clone(), detail: deck_show::roles::here_text(r.here), icon: "node", open: None, selected: host.role.as_ref() == Some(&r.id) })
        .collect();
    let mut role_rows = role_rows;
    role_rows.push(controls::TreeRow { depth: 0, label: deck_show::roles::ADD.into(), detail: String::new(), icon: "plus", open: None, selected: false });
    for (i, r) in controls::tree(d, t, 0.0, y, l.left.w - 1.0, &role_rows, pointer).into_iter().enumerate() {
        host.hits.push((r, if i + 1 == role_rows.len() { Hit::RoleAdd } else { Hit::Role(i) }));
    }
    y += role_rows.len() as f32 * t.size.tree_row + pad;
    d.text(pad, y + 4.0, small, "ROWS", t.color.muted);
    y += t.size.tree_row;
    let rows: Vec<controls::TreeRow> = plan
        .rows
        .iter()
        .map(|r| controls::TreeRow { depth: 0, label: format!("{} {}", r.number, r.id), detail: r.fires.clone(), icon: "list", open: None, selected: r.id == sel })
        .collect();
    for (i, r) in controls::tree(d, t, 0.0, y, l.left.w - 1.0, &rows, pointer).into_iter().enumerate() {
        host.hits.push((r, Hit::Row(i)));
    }

    // middle: the breadcrumb, then the overlay over the dimmed stage
    let crumbs: Vec<String> = host.crumbs().into_iter().chain(std::iter::once(format!("{} {}", host.row + 1, sel))).collect();
    let n = crumbs.len();
    for (i, r) in controls::breadcrumb(d, t, l.stage.x + pad, l.stage.y + pad, &crumbs).into_iter().enumerate() {
        if i + 1 < n {
            host.hits.push((r, Hit::Crumb(i)));
        }
    }
    view_switch(d, t, host, l.stage, true);
    if host.sheet.is_some() {
        let top = l.stage.y + t.size.row + pad;
        super::sheets::draw(d, t, host, Rect { x: l.stage.x, y: top, w: l.stage.w, h: l.stage.h - (top - l.stage.y) }, pointer);
    } else if host.two_d {
        d.rect(l.stage.x, l.stage.y + t.size.row + pad, l.stage.w, l.stage.h - t.size.row - pad, [t.color.stage[0], t.color.stage[1], t.color.stage[2], 0.6]);
        let top = l.stage.y + t.size.row + 2.0 * pad;
        let colw = ((l.stage.w - 4.0 * pad) / 3.6).max(120.0);
        let (fx, cx) = (l.stage.x + pad, l.stage.x + 2.0 * pad + colw);
        let sx = cx + pad + colw * 1.6 + pad;
        d.text(fx, top, small, "FROM FAR", t.color.muted);
        d.text(cx, top, small, "THE CELLS OF THIS ROW", t.color.muted);
        d.text(sx, top, small, "THIS ROW SENDS", t.color.muted);
        let mut fy = top + small * 1.8;
        let events: Vec<(String, bool)> = show.rows.iter().filter_map(|r| r.fire.event.as_ref()).map(|e| (format!("event: {e}"), true)).collect();
        fy += node(d, t, (fx, fy, colw), ("reader", "phone"), &events, true) + pad;
        node(d, t, (fx, fy, colw), ("clock", "show"), &[("beat".into(), true), ("bar".into(), true)], true);
        let insp = view::inspector(&show, &sel);
        let mut cy = top + small * 1.8;
        let row_cells = show.rows.get(host.row).map(|r| r.cells.clone()).unwrap_or_default();
        for o in &show.outlets {
            let cell = row_cells.get(&o.id).cloned().unwrap_or(deck_show::Cell::Keep);
            let mut lines = vec![];
            let detail = match &cell {
                deck_show::Cell::Play { piece: Some(p), values, .. } | deck_show::Cell::Values { piece: p, values } => {
                    lines.extend(view::values_text(values).split(" · ").filter(|s| !s.is_empty()).map(|s| (s.to_string(), false)));
                    p.clone()
                }
                other => view::cell_text(other),
            };
            let head = if matches!(cell, deck_show::Cell::Keep) { format!("{} ↑", o.id) } else { o.id.clone() };
            cy += node(d, t, (cx, cy, colw * 1.6), (&head, &detail), &lines, false) + pad;
        }
        let sends: Vec<(String, bool)> = insp.iter().find(|(h, _)| h == "this row sends").map(|(_, ls)| ls.iter().map(|(a, b)| (if a.is_empty() { b.clone() } else { format!("{a} · {b}") }, false)).collect()).unwrap_or_default();
        // a line without a target is the view's "nothing else": no send to count
        let count = insp.iter().find(|(h, _)| h == "this row sends").map_or(0, |(_, ls)| ls.iter().filter(|(a, _)| !a.is_empty()).count());
        node(d, t, (sx, top + small * 1.8, (l.stage.x + l.stage.w - sx - pad).max(80.0)), ("on fire", &format!("{count} sends")), &sends, false);
        let note = "Grey boxes are far away: only events reach another subtree.";
        d.text(cx, l.stage.y + l.stage.h - small * 2.0, small, &fit(d, note, small, l.stage.w - 2.0 * pad), t.color.muted);
    }

    // the rows band: fires on, then each column's cell; #478: a role's header says its name, how many are here and who
    // may take it (deck-show's words), and selects the role
    let (bx, mut by) = (l.band.x + pad, l.band.y + pad);
    let first_w = (l.band.w * 0.24).clamp(140.0, 300.0);
    let cols = plan.columns.len().max(1) as f32;
    let col_w = ((l.band.w - first_w - 2.0 * pad) / cols).max(40.0);
    let head_h = small * 2.9;
    d.text(bx + first_w * 0.4, by, small, "fires on", t.color.muted);
    for (c, col) in plan.columns.iter().enumerate() {
        let x = bx + first_w + c as f32 * col_w;
        let role = roles.iter().position(|r| r.id == col.id);
        let here = role.map(|i| deck_show::roles::here_text(roles[i].here)).unwrap_or_default();
        if role.is_some() && host.role.as_ref() == Some(&col.id) {
            d.round_rect(x - 2.0, by - 3.0, col_w - 2.0, head_h, t.size.radius, t.color.selected);
        }
        let hw = d.advance(small) * here.chars().count() as f32;
        d.text(x + 4.0, by, small, &fit(d, &col.name, small, col_w - hw - 14.0), t.color.text);
        d.text(x + col_w - hw - 10.0, by, small, &here, t.color.muted);
        d.text(x + 4.0, by + small * 1.3, small, &fit(d, col.filter.as_ref().unwrap_or(&col.detail), small, col_w - 8.0), t.color.muted);
        if let Some(i) = role {
            host.hits.push((Rect { x, y: by - 3.0, w: col_w - 2.0, h: head_h }, Hit::Role(i)));
        }
    }
    by += head_h + 2.0;
    let rh = ((l.band.h - head_h - 2.0 - 2.0 * pad) / plan.rows.len().max(1) as f32).clamp(18.0, t.size.row);
    for (i, r) in plan.rows.iter().enumerate() {
        let y = by + i as f32 * rh;
        if y + rh > l.band.y + l.band.h {
            break;
        }
        let whole = Rect { x: l.band.x, y, w: l.band.w, h: rh };
        if r.id == sel {
            d.rect(whole.x, whole.y, whole.w, whole.h, t.color.selected);
        } else if whole.has(pointer.0, pointer.1) {
            d.rect(whole.x, whole.y, whole.w, whole.h, t.color.hover);
        }
        let ty = y + (rh - font * 1.25) * 0.5;
        d.text(bx, ty, font, &fit(d, &format!("{} {}", r.number, r.id), font, first_w * 0.4 - 4.0), t.color.text);
        d.text(bx + first_w * 0.4, ty, font, &fit(d, &r.fires, font, first_w * 0.6 - 4.0), t.color.muted);
        for (c, cell) in r.cells.iter().enumerate() {
            let x = bx + first_w + c as f32 * col_w;
            if !cell.keep {
                d.round_rect(x, y + 2.0, col_w - 6.0, rh - 4.0, t.size.radius, t.color.control);
            }
            d.text(x + 6.0, ty, font, &fit(d, &cell.text, font, col_w - 14.0), if cell.keep { t.color.muted } else { t.color.text });
        }
        host.hits.push((whole, Hit::Row(i)));
    }

    // right: the inspector of the row (or #478 the selected role's), the seats, save / load
    let (rx, rw) = (l.right.x + pad, l.right.w - 2.0 * pad);
    let y = l.right.y + pad;
    let bh = 34.0;
    let foot = l.right.y + l.right.h - bh - pad - small * 2.0;
    let role = host.role.as_ref().and_then(|id| deck_show::roles::inspector(&show, host.path.is_empty().then_some(&host.sub), id));
    match role {
        Some(insp) => super::roles::inspector(d, t, host, &insp, Rect { x: rx, y, w: rw, h: foot - y }, pointer),
        None => row_side(d, t, host, (&show, &sel, &seats), (rx, y, rw), foot),
    }
    let bw = (rw - 2.0 * pad) / 3.0;
    let load = Rect { x: rx, y: foot, w: bw, h: bh };
    let flash = Rect { x: rx + bw + pad, y: foot, w: bw, h: bh };
    let save = Rect { x: rx + 2.0 * (bw + pad), y: foot, w: bw, h: bh };
    button(d, t, load, "Load", t.color.control, t.color.text);
    button(d, t, flash, "Flash", t.color.control, t.color.text);
    button(d, t, save, "Save", t.color.accent, t.color.surface);
    host.hits.push((save, Hit::Save));
    host.hits.push((load, Hit::Load));
    host.hits.push((flash, Hit::Flash));
    let line = if host.status.is_empty() { format!("{}@{} · /api/shows", show.id, show.version) } else { host.status.clone() };
    d.text(rx, foot + bh + 6.0, small, &fit(d, &line, small, rw), t.color.muted);
    // #478: a prompt asked (a role's remove) over the stage: only its presses count
    super::roles::ask(d, t, host, l.stage);
}

/// the right side for a row: its inspector, the seats, the flash reports, down to `foot`
fn row_side(d: &mut DrawList, t: &Tokens, host: &Host, (show, sel, seats): (&Show, &str, &[view::SeatView]), (rx, mut y, rw): (f32, f32, f32), foot: f32) {
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    for (i, (head, lines)) in view::inspector(show, sel).iter().enumerate() {
        y += controls::section(d, t, rx, y, rw, &head.to_uppercase(), i == 0);
        for (label, value) in lines {
            if !label.is_empty() {
                d.text(rx, y + 4.0, font, &fit(d, label, font, rw * 0.36), t.color.muted);
            }
            let vx = if label.is_empty() { rx } else { rx + rw * 0.38 };
            d.round_rect(vx, y, rx + rw - vx, t.size.row - 4.0, t.size.radius, t.color.control);
            d.text(vx + 6.0, y + 4.0, font, &fit(d, value, font, rx + rw - vx - 12.0), t.color.text);
            y += t.size.row;
        }
        y += pad * 0.5;
    }
    y += controls::section(d, t, rx, y, rw, "SEATS", false);
    if seats.is_empty() {
        d.text(rx, y + 4.0, small, "none: this show seats no sub conductor", t.color.muted);
        y += t.size.row;
    }
    for s in seats {
        d.text(rx, y + 4.0, font, &s.id, t.color.text);
        d.text(rx + rw * 0.38, y + 4.0, small, &fit(d, &format!("{} · {}", s.show, s.detail), small, rw * 0.62), t.color.muted);
        y += font * 1.5;
        d.text(rx + rw * 0.38, y + 2.0, small, &fit(d, &s.sits, small, rw * 0.62), t.color.muted);
        y += t.size.row;
    }
    // #91 item 5: what each deck and sub conductor answered to the last flash (deck-show's words)
    y += controls::section(d, t, rx, y, rw, "FLASH REPORTS", false);
    let reports = view::reports(&host.sub);
    if reports.is_empty() {
        d.text(rx, y + 4.0, small, "no answer yet: Flash sends the tree down", t.color.muted);
    }
    for r in &reports {
        if y + font * 1.5 > foot {
            break;
        }
        d.disc(rx + 4.0, y + 4.0 + font * 0.62, 3.5, if r.ok { t.color.go } else { t.color.warn });
        d.text(rx + 14.0, y + 4.0, font, &fit(d, &format!("{} · {}", r.what, r.from), font, rw - 14.0), t.color.text);
        y += font * 1.5;
        for l in std::iter::once(&r.line).chain(&r.lacks) {
            if y + small * 1.6 > foot {
                break;
            }
            d.text(rx + 14.0, y + 2.0, small, &fit(d, l, small, rw - 14.0), t.color.muted);
            y += small * 1.6;
        }
        y += pad * 0.5;
    }
}
