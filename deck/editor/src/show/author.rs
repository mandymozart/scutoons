//! The Show workspace (#91, the owner's draft m1-show), floating panels over the stage: the outline (this deck's
//! conductor, its seats – a seat enters its sub conductor, the toolbar's path leads back –, the roles, the rows of the
//! document shown); the 2D node overlay (#446: what comes from far, the cells of the row and what the row sends) or the
//! sheet open; the rows band; the scene list; the inspector of the row with save and load; the info. The sheets' pills
//! sit in the toolbar, the 2D / 3D switch in the tools pill. The words are deck-show's view.rs; this file lays them out and names the
//! places (`Hit`).
use super::perform::button;
use super::{Hit, Host, Sheet, Workspace};
use deck_kit::editor::controls::{self, Rect};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_show::{view, Show};

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// the sheets the Show workspace offers over its stage (the flash list opens from Flash, not from a pill; the show runner's
/// sheets are Perform's)
pub const SHEETS: [(&str, Sheet); 4] = [("Cues", Sheet::Cues), ("Problems", Sheet::Problems), ("Who", Sheet::Who), ("Demos", Sheet::Demos)];

fn view_names(sheets: &[(&'static str, Sheet)]) -> Vec<&'static str> {
    sheets.iter().map(|(n, _)| *n).collect()
}

/// the toolbar's room for a workspace's view switch
pub fn view_width(d: &DrawList, t: &Tokens, _host: &Host, ws: Workspace) -> f32 {
    let sheets: &[(&str, Sheet)] = if ws == Workspace::Perform { &super::perform::SHEETS } else { &SHEETS };
    view_names(sheets).iter().map(|n| d.advance(t.size.small) * n.chars().count().max(2) as f32 + 2.0 * t.size.pad + 2.0).sum::<f32>() + 8.0
}

/// a pill per sheet the workspace offers (`sheets`), in the toolbar's slot (a test frame without a toolbar: left of the
/// top right corner); 3D / 2D are the tools pill's
pub fn view_switch(d: &mut DrawList, t: &Tokens, host: &mut Host, w: f32, sheets: &[(&'static str, Sheet)]) {
    let names = view_names(sheets);
    let width: f32 = names.iter().map(|n| d.advance(t.size.small) * n.chars().count().max(2) as f32 + 2.0 * t.size.pad + 2.0).sum::<f32>() + 8.0;
    let slot = if host.view_slot.w > 0.0 { host.view_slot } else { Rect { x: w - width - 360.0, y: 8.0, w: width, h: t.size.top } };
    d.round_rect(slot.x, slot.y, slot.w, slot.h, slot.h * 0.5, t.color.surface);
    d.outline(slot.x, slot.y, slot.w, slot.h, (slot.h * 0.5, 1.0), t.color.line);
    let (x, y, h) = (slot.x + 4.0, slot.y + 4.0, slot.h - 8.0);
    let on = host.sheet.and_then(|s| sheets.iter().position(|(_, x)| *x == s)).unwrap_or(usize::MAX);
    let r = controls::pills(d, t, x, y, h, &names, on);
    let hits = sheets.iter().map(|(_, s)| Hit::Sheet(*s));
    for (r, hit) in r.into_iter().zip(hits) {
        host.hits.push((r, hit));
    }
}

/// one node of the overlay: a head (name, detail) and its lines (a dot before an input, after an output)
fn node(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), head: (&str, &str), lines: &[(String, bool)], far: bool) -> f32 {
    let (font, small, pad) = (t.size.font, t.size.small, t.size.pad);
    let lh = font * 1.5;
    let h = lh + lines.len() as f32 * lh + pad;
    d.round_rect(x, y, w, h, t.size.radius, t.color.surface_head);
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
    let (prims, hits) = (d.prims().len(), host.hits.len());
    draw_once(d, t, host, (w, h), pointer);
    // a content that grew (a row added) is laid out again before the frame shows
    if host.show_panels.take_grew() {
        d.truncate(prims);
        host.hits.truncate(hits);
        draw_once(d, t, host, (w, h), pointer);
        host.show_panels.take_grew();
    }
}

fn draw_once(d: &mut DrawList, t: &Tokens, host: &mut Host, (w, h): (f32, f32), pointer: (f32, f32)) {
    let (pad, small) = (t.size.pad, t.size.small);
    let show = host.shown().clone();
    let plan = view::plan(&show);
    // the rows band: what this conductor plays when it runs the document shown, else the document alone
    let runs = host.path.is_empty();
    let band = if runs { view::matrix_at(&host.sub, super::now()) } else { plan.clone() };
    host.row = host.row.min(show.rows.len().saturating_sub(1));
    let sel = show.rows.get(host.row).map(|r| r.id.clone()).unwrap_or_default();
    let seats = view::seats(&show);
    // #478: the roles this conductor gives out, with how many are in each (the running top only: a sub conductor's
    // document shown here runs elsewhere)
    let roles = deck_show::roles::roles(&show, host.path.is_empty().then_some(&host.sub));
    view_switch(d, t, host, w, &SHEETS);
    let sheet = host.sheet.filter(|s| !s.in_perform());

    // the panels shown and what each asks for: the rows band as its whole table, scrolled once it cannot grow
    let band_size = super::band::table(t, &band, true).size();
    let mut shown: Vec<(&'static str, f32, Option<f32>)> = vec![("outline", t.size.tree * 1.3, None), ("inspector", 340.0, None)];
    if sheet.is_some() {
        shown.push(("sheet", 640.0, Some(440.0)));
    } else if host.two_d {
        shown.push(("nodes", 760.0, None));
    }
    // the library: drawn by the editor over these panels (scenes.rs `panel`)
    shown.extend([(crate::library::PANEL, crate::library::WIDTH, super::scenes::note_height(t, host)), ("rows", band_size.0, Some(band_size.1)), ("info", 260.0, Some(small * 1.4 * 6.0))]);
    host.show_panels.layout(d, t, ((w, h), crate::bar::bottom(t)), &[], &shown);

    // the outline: the conductors (the top, then the seats of the document shown), the roles, the rows
    if let Some(o) = host.show_panels.open(d, t, "outline", pointer, host.hits.len()) {
        let a = o.area;
        let mut y = a.y;
        d.text(a.x, y + 4.0, small, "CONDUCTORS", t.color.muted);
        y += t.size.tree_row;
        let depth = host.path.len();
        let mut tree = vec![controls::TreeRow { depth: 0, label: show.id.clone(), detail: if depth == 0 { "this deck".into() } else { format!("sub · {}", host.path.last().cloned().unwrap_or_default()) }, icon: "nodes", open: Some(true), selected: true }];
        for s in &seats {
            let picked = host.seat == seats.iter().position(|x| x.id == s.id);
            tree.push(controls::TreeRow { depth: 1, label: s.id.clone(), detail: s.detail.clone(), icon: "folder", open: Some(false), selected: picked });
        }
        for (i, r) in controls::tree(d, t, a.x, y, a.w, &tree, pointer).into_iter().enumerate() {
            host.hits.push((r, if i == 0 { Hit::Crumb(depth) } else { Hit::Enter(i - 1) }));
        }
        y += tree.len() as f32 * t.size.tree_row + pad;
        d.text(a.x, y + 4.0, small, "ROLES", t.color.muted);
        y += t.size.tree_row;
        let mut role_rows: Vec<controls::TreeRow> = roles
            .iter()
            .map(|r| controls::TreeRow { depth: 0, label: r.name.clone(), detail: deck_show::roles::here_text(r.here), icon: "node", open: None, selected: host.role.as_ref() == Some(&r.id) })
            .collect();
        role_rows.push(controls::TreeRow { depth: 0, label: deck_show::roles::ADD.into(), detail: String::new(), icon: "plus", open: None, selected: false });
        for (i, r) in controls::tree(d, t, a.x, y, a.w, &role_rows, pointer).into_iter().enumerate() {
            host.hits.push((r, if i + 1 == role_rows.len() { Hit::RoleAdd } else { Hit::Role(i) }));
        }
        host.show_panels.close(d, t, o, &mut host.hits);
    }

    // the sheet open, else the 2D overlay
    if let Some(o) = host.show_panels.open(d, t, "sheet", pointer, host.hits.len()) {
        super::sheets::draw(d, t, host, o.area, pointer);
        host.show_panels.close(d, t, o, &mut host.hits);
    }
    if let Some(o) = host.show_panels.open(d, t, "nodes", pointer, host.hits.len()) {
        nodes(d, t, host, (&show, &sel), o.area);
        host.show_panels.close(d, t, o, &mut host.hits);
    }

    if let Some(o) = host.show_panels.open(d, t, "rows", pointer, host.hits.len()) {
        let laid = super::band::draw(d, t, host, o.area, (&band, super::band::Mode { perform: false, runs }), host.show_scroll, pointer);
        host.show_scroll = laid.scroll;
        host.drew_band(false, laid);
        host.show_panels.close(d, t, o, &mut host.hits);
    }

    // the inspector: Load, Flash and what the last one came to, then the row's (or #478 the selected role's)
    if let Some(o) = host.show_panels.open(d, t, "inspector", pointer, host.hits.len()) {
        let a = o.area;
        let bh = 30.0;
        let bw = (a.w - pad) / 2.0;
        let load = Rect { x: a.x, y: a.y, w: bw, h: bh };
        let flash = Rect { x: a.x + bw + pad, y: a.y, w: bw, h: bh };
        button(d, t, load, "Load", t.color.control, t.color.text);
        button(d, t, flash, "Flash", t.color.control, t.color.text);
        host.hits.push((load, Hit::Load));
        host.hits.push((flash, Hit::Flash));
        let line = if host.status.is_empty() { format!("{}@{} · /api/shows", show.id, show.version) } else { host.status.clone() };
        d.text(a.x, a.y + bh + 6.0, small, &fit(d, &line, small, a.w), t.color.muted);
        let mut y = a.y + bh + 6.0 + small * 2.0;
        let typing = host.typing.as_ref().filter(|(what, _)| matches!(what, super::roles::Typing::Background)).map(|(_, f)| f.text.clone());
        match typing {
            Some(text) => {
                host.hits.push((super::rows::field(d, t, (a.x, y, a.w), "background", "", Some(&text)), Hit::Background));
                y += t.size.tree_row;
            }
            None => {
                let (swatch, at) = crate::inspector::background(d, t, a, &mut y, &crate::inspector::background_row(show.background.as_ref(), "its scenes', else black"));
                // the last rect that has the pointer takes the press: the row, then its swatch
                host.hits.push((at, Hit::Background));
                host.hits.push((swatch, Hit::BackgroundColour));
            }
        }
        let role = host.role.as_ref().and_then(|id| deck_show::roles::inspector(&show, host.path.is_empty().then_some(&host.sub), id));
        match role {
            // the role's inspector keeps its remove at its foot: a foot of its own size
            Some(insp) => super::roles::inspector(d, t, host, &insp, Rect { x: a.x, y, w: a.w, h: 360.0 }, pointer),
            None => {
                row_side(d, t, host, (&show, &sel, &seats), (a.x, y, a.w), pointer);
            }
        }
        host.show_panels.close(d, t, o, &mut host.hits);
    }

    if let Some(o) = host.show_panels.open(d, t, "info", pointer, host.hits.len()) {
        deck_kit::editor::info::body(d, t, (o.area.x, o.area.y, o.area.w), host.help.as_ref());
        host.show_panels.close(d, t, o, &mut host.hits);
    }
    // #478: a prompt asked (a role's remove) over everything: only its presses count
    let room = crate::panels::Panels::room((w, h), crate::bar::bottom(t), t.size.gap);
    super::roles::ask(d, t, host, room);
    // #551: the right click's typeahead on the selected cells, over everything
    super::ask::draw_context(d, t, host, (w, h));
    super::scenes::over(d, t, host, (w, h));
}

/// the 2D overlay: what comes from far (the reader's events, the clock), the cells of the row, what the row sends
fn nodes(d: &mut DrawList, t: &Tokens, host: &mut Host, (show, sel): (&Show, &str), a: Rect) {
    let (pad, small) = (t.size.pad, t.size.small);
    let top = a.y + 4.0;
    let colw = ((a.w - 2.0 * pad) / 3.6).max(120.0);
    let (fx, cx) = (a.x, a.x + pad + colw);
    let sx = cx + pad + colw * 1.6 + pad;
    d.text(fx, top, small, "FROM FAR", t.color.muted);
    d.text(cx, top, small, "THE CELLS OF THIS ROW", t.color.muted);
    d.text(sx, top, small, "THIS ROW SENDS", t.color.muted);
    let mut fy = top + small * 1.8;
    let events: Vec<(String, bool)> = show.rows.iter().filter_map(|r| r.fire.event.as_ref()).map(|e| (format!("event: {e}"), true)).collect();
    fy += node(d, t, (fx, fy, colw), ("reader", "phone"), &events, true) + pad;
    fy += node(d, t, (fx, fy, colw), ("clock", "show"), &[("beat".into(), true), ("bar".into(), true)], true);
    let insp = view::inspector(show, sel);
    let mut cy = top + small * 1.8;
    let row_cells = show.rows.get(host.row).map(|r| r.cells.clone()).unwrap_or_default();
    let mut boxes: Vec<(String, f32, f32)> = Vec::new();
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
        let head = if matches!(cell, deck_show::Cell::Keep) { format!("{} ↑", o.name()) } else { o.name().to_string() };
        let h = node(d, t, (cx, cy, colw * 1.6), (&head, &detail), &lines, false);
        boxes.push((o.id.clone(), cy, h));
        cy += h + pad;
    }
    // #568: a far node whose holder plays on another outlet of this row: a tube between the two cells (deck-show far.rs)
    for l in host.row_links(sel) {
        let deck_show::far::Holder::Outlet(to) = &l.holder else { continue };
        let at = |id: &str| boxes.iter().find(|(o, ..)| o == id).map(|(_, y, h)| (cx + colw * 1.6, y + h * 0.5));
        if let (Some(a), Some(b)) = (at(&l.from), at(to)) {
            d.cable(a, b, t.color.accent);
            d.disc(a.0, a.1, 4.0, t.color.accent);
            d.disc(b.0, b.1, 4.0, t.color.accent);
        }
    }
    let sends: Vec<(String, bool)> = insp.iter().find(|(h, _)| h == "this row sends").map(|(_, ls)| ls.iter().map(|(a, b)| (if a.is_empty() { b.clone() } else { format!("{a} · {b}") }, false)).collect()).unwrap_or_default();
    // a line without a target is the view's "nothing else": no send to count
    let count = insp.iter().find(|(h, _)| h == "this row sends").map_or(0, |(_, ls)| ls.iter().filter(|(a, _)| !a.is_empty()).count());
    let sy = top + small * 1.8 + node(d, t, (sx, top + small * 1.8, (a.x + a.w - sx).max(80.0)), ("on fire", &format!("{count} sends")), &sends, false);
    let note = "Grey boxes are far away: only events reach another subtree.";
    d.text(a.x, fy.max(cy).max(sy) + pad, small, &fit(d, note, small, a.w), t.color.muted);
}

/// the right side for a row: its inspector (the selected cell's inspector in place of the cells), the seats, the flash
/// reports; where it ends (it scrolls above the buttons' strip)
fn row_side(d: &mut DrawList, t: &Tokens, host: &mut Host, (show, sel, seats): (&Show, &str, &[view::SeatView]), (rx, mut y, rw): (f32, f32, f32), pointer: (f32, f32)) -> f32 {
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    for (i, (head, lines)) in view::inspector(show, sel).iter().enumerate() {
        // the row's cells are edited in place in the rows band: only the selected one's inspector shows here
        if head == "cells of this row" {
            y += super::rows::cell_side(d, t, host, (rx, y, rw), show, sel);
            continue;
        }
        y += controls::section(d, t, rx, y, rw, &head.to_uppercase(), i == 0);
        let sends = head == "this row sends";
        let mut address = 0;
        for (label, value) in lines {
            if !label.is_empty() {
                d.text(rx, y + 4.0, font, &fit(d, label, font, rw * 0.36), t.color.muted);
            }
            // #539: "fires on" opens the event picker; #553: an address selects it, its filter's chips under it
            let fires = i == 0 && label == view::FIRES_ON;
            let addr = (sends && label == view::ADDRESS).then(|| {
                address += 1;
                address - 1
            });
            let vx = if label.is_empty() { rx } else { rx + rw * 0.38 };
            let r = Rect { x: vx, y, w: rx + rw - vx, h: t.size.row - 4.0 };
            let presses = fires || addr.is_some();
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, if presses && r.has(pointer.0, pointer.1) { t.color.hover } else { t.color.control });
            let open = addr.is_some() && addr == host.address;
            if (fires && host.sheet == Some(Sheet::Event)) || open {
                d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.0), t.color.accent);
            }
            if fires {
                host.hits.push((r, Hit::FiresOn));
            }
            d.text(vx + 6.0, y + 4.0, font, &fit(d, value, font, rx + rw - vx - 12.0), t.color.text);
            y += t.size.row;
            if let Some(k) = addr {
                host.hits.push((r, Hit::AddressPick(k)));
                if let Some(c) = open.then(|| deck_show::rows::address_condition(show, sel, k)).flatten() {
                    let (chips, used) = super::condition::draw(d, t, (rx, y, rw), deck_show::rows::ADDRESS_FILTER, &c);
                    for (j, r) in chips.into_iter().enumerate() {
                        host.hits.push((r, Hit::AddressFact(j)));
                    }
                    y += used;
                }
            }
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
        y += t.size.row;
    }
    for r in &reports {
        d.disc(rx + 4.0, y + 4.0 + font * 0.62, 3.5, if r.ok { t.color.go } else { t.color.warn });
        d.text(rx + 14.0, y + 4.0, font, &fit(d, &format!("{} · {}", r.what, r.from), font, rw - 14.0), t.color.text);
        y += font * 1.5;
        for l in std::iter::once(&r.line).chain(&r.lacks) {
            d.text(rx + 14.0, y + 2.0, small, &fit(d, l, small, rw - 14.0), t.color.muted);
            y += small * 1.6;
        }
        y += pad * 0.5;
    }
    y
}
