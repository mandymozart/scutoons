//! A Show tab (#91, the owner's draft m1-show), floating panels over the stage: the outline (this deck's conductor, its
//! seats – a seat enters its sub conductor, the toolbar's path leads back –, the roles, the rows of the document shown);
//! the node view of the selected row (nodes.rs) or the sheet open; the rows band, which is the cue matrix too; the scene
//! list; the inspector of the row with save and load; the info; a running show's optional panels (live.rs). The sheets'
//! pills sit in the toolbar, the 2D / 3D switch in the tools pill. The words are deck-show's view.rs; this file lays them
//! out and names the places (`Hit`).
use super::live::button;
use super::{Hit, Host, Sheet};
use deck_kit::editor::controls::{self, Rect};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_show::{view, Show};

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// the Show's views of its own (RULES H16): each a sheet in the sheet panel, listed in the ☰ Views – id, name, icon (the
/// flash list opens from Flash)
pub const SHEETS: [(&str, &str, &str, Sheet); 6] = [
    ("cues", "Cues", "list", Sheet::Cues),
    ("problems", "Problems", "alert", Sheet::Problems),
    ("who", "Who", "users", Sheet::Who),
    ("session", "Session", "link", Sheet::Session),
    ("vote", "Vote", "activity", Sheet::Vote),
    ("demos", "Demos", "play", Sheet::Demos),
];

/// the panel every sheet shows in
pub const SHEET_PANEL: &str = "sheet";

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
    let sheet = host.sheet;

    // the panels shown and what each asks for: the rows band as its whole table, scrolled once it cannot grow
    let band_size = super::band::table(t, &band, true, &host.view_sizes()).size();
    let mut shown: Vec<(&'static str, f32, Option<f32>)> = vec![("outline", t.size.tree * 1.3, None), ("inspector", 340.0, None)];
    // the slot's Find hangs under its slot, not in a sheet
    if sheet.is_some_and(|s| s != Sheet::Event) {
        shown.push((SHEET_PANEL, 640.0, Some(440.0)));
    } else if host.two_d {
        shown.push(("nodes", 960.0, Some(560.0)));
    }
    // the library: drawn by the editor over these panels (scenes.rs `panel`)
    shown.extend([(crate::library::PANEL, crate::library::WIDTH, super::scenes::note_height(t, host)), ("rows", band_size.0, Some(band_size.1)), ("info", 260.0, Some(small * 1.4 * 6.0))]);
    shown.extend(super::live::shown(t, host));
    host.show_panels.layout(d, t, ((w, h), crate::bar::bottom(t)), &[], &shown);
    // a running show's panels: what plays now, the session, GO (the top's conductor runs; a sub's document only shows)
    super::live::draw(d, t, host, &view::matrix_at(&host.sub, super::now()), pointer);

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
        host.show_panels.close(d, o, &mut host.hits);
    }

    // the sheet open, else the 2D overlay
    if let Some(o) = host.show_panels.open(d, t, "sheet", pointer, host.hits.len()) {
        super::sheets::draw(d, t, host, o.area, pointer);
        host.show_panels.close(d, o, &mut host.hits);
    }
    host.nodes.shown = false;
    if let Some(o) = host.show_panels.open(d, t, "nodes", pointer, host.hits.len()) {
        let far = host.row_links(&sel);
        let mut g = deck_show::nodes::row_graph(&show, host.row, &far);
        if host.nodes.motion.activity && host.path.is_empty() {
            deck_show::nodes::activity(&mut g, &show, host.row, host.sub.conductor(), super::now());
        }
        let (g, frames) = deck_show::nodes::folded(g, &show, host.row);
        super::nodes::draw(d, t, host, (&g, &frames), o.area);
        host.show_panels.close(d, o, &mut host.hits);
    }

    if let Some(o) = host.show_panels.open(d, t, "rows", pointer, host.hits.len()) {
        let (laid, fits) = super::band::draw(d, t, host, o.area, (&band, runs), host.show_scroll, pointer);
        host.show_scroll = laid.scroll;
        host.drew_band(laid, fits);
        host.show_panels.close(d, o, &mut host.hits);
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
        let mut y = a.y + bh + 6.0 + small * 0.6;
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
        if host.show_toon().is_some() && host.path.is_empty() {
            y += super::toons::of_show(d, t, host, (a.x, y, a.w));
        }
        let role = host.role.as_ref().and_then(|id| deck_show::roles::inspector(&show, host.path.is_empty().then_some(&host.sub), id));
        match role {
            // the role's inspector keeps its remove at its foot: a foot of its own size
            Some(insp) => super::roles::inspector(d, t, host, &insp, Rect { x: a.x, y, w: a.w, h: 360.0 }, pointer),
            None => {
                row_side(d, t, host, (&show, &sel, &seats), (a.x, y, a.w), pointer);
            }
        }
        host.show_panels.close(d, o, &mut host.hits);
    }

    if let Some(o) = host.show_panels.open(d, t, "info", pointer, host.hits.len()) {
        deck_kit::editor::info::body(d, t, (o.area.x, o.area.y, o.area.w), host.help.as_ref());
        host.show_panels.close(d, o, &mut host.hits);
    }
    // the slot's Find, over the panels
    super::events::find(d, t, host, (w, h), pointer);
    // #478: a prompt asked (a role's remove) over everything: only its presses count
    let room = crate::panels::Panels::room((w, h), crate::bar::bottom(t), t.size.gap);
    super::roles::ask(d, t, host, room);
    // #551: the right click's typeahead on the selected cells, over everything
    super::ask::draw_context(d, t, host, (w, h));
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
            // #553: an address selects it, its filter's chips under it (what fires the row is set at its ⚡ slot)
            let addr = (sends && label == view::ADDRESS).then(|| {
                address += 1;
                address - 1
            });
            let vx = if label.is_empty() { rx } else { rx + rw * 0.38 };
            let r = Rect { x: vx, y, w: rx + rw - vx, h: t.size.row - 4.0 };
            let presses = addr.is_some();
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, if presses && r.has(pointer.0, pointer.1) { t.color.hover } else { t.color.control });
            let open = addr.is_some() && addr == host.address;
            if open {
                d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.0), t.color.text);
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
