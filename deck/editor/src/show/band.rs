//! The rows band, which is the cue matrix too: the session grid of rows × columns under sticky heads (deck-kit `table`),
//! after Ableton's session view – authored and launched in one place, a cell edited while it plays. No grid lines: each
//! cell is a card (deck-kit `cards`) – its ▶, its words and a status line with a dot; a kept cell says it carries on, an
//! empty one is a stop slot. A row's head holds its ▶, its number, its name and – auto or suggest – its fire tag, which
//! says only what the name does not; a column's head its group, its name, who takes it and its small stop. The edges
//! between columns and rows show only under the pointer, as a handle to drag; a double click fits (sizing.rs). What plays
//! is the text colour with the bar's progress, what waits for its moment is ringed, a cell started off its row has a corner mark.
//! The selection shows: the row as a soft band (no bar on its side, RULES H10), a column as a band from its head to its last row,
//! a group's head, a cell outlined; names are typed in place in their heads. Every word and state is deck-show's view.rs
//! (`matrix_at`) and every size limit its sizes.rs; this file lays them out and names the places.
use super::roles::Typing;
use super::sizing::{Fits, GRIP};
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::signal::Signal;
use deck_kit::base::tokens::Tokens;
use deck_kit::base::wire::Wire;
use deck_kit::editor::cards::{self, fade as soft, fit, Tag};
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::table::{Edge, Laid, Scroll, Table};
use deck_show::sizes::{Sizes, CELL};
use deck_show::view::{CellView, FireChip, Matrix, RowView, State};
use deck_show::Moment;

/// the band's measures for `m` at the viewer's `sizes`; in Show (`adds`) one more row under the last: the new row's place
pub fn table(t: &Tokens, m: &Matrix, adds: bool, sizes: &Sizes) -> Table {
    let s = &t.size;
    let groups = if m.groups.is_empty() { 0.0 } else { s.rem(1.6) };
    let widths = m.columns.iter().map(|c| s.rem(sizes.column(&c.id))).collect();
    let mut heights: Vec<f32> = m.rows.iter().map(|r| s.rem(sizes.row(&r.id))).collect();
    if adds {
        heights.push(s.rem(CELL.1));
    }
    Table { head: (s.rem(sizes.heads()), groups + s.rem(3.2)), widths, heights, foot: s.rem(1.6) }
}

/// a ▶ in a circle of radius `r` around `c`: filled in the text colour while it plays, ringed while it waits, quiet else
fn launcher(d: &mut DrawList, t: &Tokens, c: (f32, f32), r: f32, state: State, hot: bool) {
    match state {
        State::Playing => d.disc(c.0, c.1, r, t.color.text),
        State::Queued => {
            d.disc(c.0, c.1, r, t.color.control);
            d.ring(c.0, c.1, r, t.color.text);
            d.ring(c.0, c.1, r - 1.5, t.color.text);
        }
        State::Idle => {
            d.disc(c.0, c.1, r, if hot { t.color.hover } else { t.color.control });
            d.ring(c.0, c.1, r, t.color.line);
        }
    }
    let s = r * 0.9;
    let ink = if state == State::Playing { t.color.pressed_text } else { t.color.text };
    d.icon(c.0 - s * 0.42, c.1 - s * 0.5, s, "play-fill", ink);
}

/// the stop slot of an empty cell: a small square around `c`, ringed while it waits for its moment
fn stop_slot(d: &mut DrawList, t: &Tokens, c: (f32, f32), r: f32, state: State, hot: bool) {
    let s = r * 1.3;
    let (x, y) = (c.0 - s * 0.5, c.1 - s * 0.5);
    if hot {
        d.round_rect(x, y, s, s, 2.0, t.color.hover);
    }
    let color = match state {
        State::Queued => t.color.text,
        _ => t.color.muted,
    };
    d.outline(x, y, s, s, (2.0, 1.0), color);
    if state == State::Queued {
        d.outline(x - 3.0, y - 3.0, s + 6.0, s + 6.0, (4.0, 1.0), color);
    }
}

/// the moment's ticks' width: now one, the next bar four, the next phrase four
pub fn ticks_width(moment: Moment) -> f32 {
    if moment == Moment::Now {
        3.0
    } else {
        12.0
    }
}

/// the moment as ticks: now one, the next bar a tall and three short, the next phrase four tall
fn ticks(d: &mut DrawList, (x, y, h): (f32, f32, f32), moment: Moment, color: [f32; 4]) {
    let tall: &[bool] = match moment {
        Moment::Now => &[true],
        Moment::Bar => &[true, false, false, false],
        Moment::Phrase => &[true, true, true, true],
    };
    for (i, high) in tall.iter().enumerate() {
        let th = if *high { h } else { h * 0.5 };
        d.rect(x + i as f32 * 3.0, y + h - th, 1.5, th, color);
    }
}

/// the tag of a row's fire chip: a filled bolt for auto, an outlined one and a dashed border for suggest
pub fn fire_tag_of<'a>(t: &Tokens, chip: &'a FireChip) -> Tag<'a> {
    Tag { icon: if chip.auto { "bolt-fill" } else { "bolt" }, tint: t.color.warn, text: &chip.tag, dashed: !chip.auto, trail: ticks_width(chip.moment) }
}

/// A row's fire tag at `(x, y)`, no wider than `w`: the bolt, what the row's name does not say, the moment's ticks; its width.
pub fn fire_tag(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), chip: &FireChip) -> f32 {
    let (r, trail) = cards::tag(d, t, (x, y, w), &fire_tag_of(t, chip));
    let h = cards::tag_height(t);
    ticks(d, (trail, y + h * 0.28, h * 0.44), chip.moment, t.color.muted);
    r.w
}

/// a far node's or a question's link: its two ports and the signal it carries
type Hung = ((f32, f32), (f32, f32), Signal);

/// A link hanging from port `a` to port `b` (deck-kit wire.rs), the ports drawn; `flow` = the seconds the beads have
/// run, none: no beads (Signal flow on wires off, or Reduce motion).
pub fn wire(d: &mut DrawList, t: &Tokens, (a, b): ((f32, f32), (f32, f32)), (unit, flow): (f32, Option<f32>), color: [f32; 4]) {
    let w = Wire::hang(a, b, unit);
    let beads = flow.map(|s| w.flowing(unit, s)).unwrap_or_default();
    d.wire(&w, 1.5, color, &beads);
    for p in [a, b] {
        d.port(p, unit * 0.3, color, t.color.surface);
    }
}

fn push(host: &mut Host, r: Rect, region: &Rect, hit: Hit) {
    if let Some(r) = Laid::shown(&r, region) {
        host.hits.push((r, hit));
    }
}

/// what is typed in place now, when it names `what`
fn typed(host: &Host, what: impl Fn(&Typing) -> bool) -> Option<String> {
    host.typing.as_ref().filter(|(w, _)| what(w)).map(|(_, f)| f.text.clone())
}

/// A cell's status line: what waits for which moment, why it would not play, who the question reaches, a published scene
/// playing while a newer draft of it exists.
pub fn note(t: &Tokens, host: &Host, (r, c): (&RowView, usize), m: &Matrix, cell: &CellView) -> Option<(String, [f32; 4])> {
    let show = host.shown();
    let newer = cell.state == State::Playing && host.book.as_ref().zip(m.columns.get(c)).is_some_and(|(b, col)| deck_show::rows::newer_draft(show, &r.id, &col.id, b));
    match (&cell.state, &cell.problem, &cell.ask) {
        (State::Queued, ..) => Some((r.moment.clone(), t.color.text)),
        (_, Some(p), _) => Some((p.clone(), t.color.warn)),
        (_, _, Some(a)) => Some((a.filter.clone(), t.color.muted)),
        _ if newer => Some((deck_show::rows::NEWER_DRAFT.to_string(), t.color.warn)),
        _ => None,
    }
}

/// The band in `area` (a panel's visible content), scrolled by `scroll`; its laid table (for the wheel and the edges) and
/// what its content asks for. `runs`: the conductor runs the document shown – the ▶ and the stops work (a sub
/// conductor's document only shows).
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, area: Rect, (m, runs): (&Matrix, bool), scroll: Scroll, pointer: (f32, f32)) -> (Laid, Fits) {
    let sizes = host.view_sizes();
    let l = table(t, m, true, &sizes).lay(area, scroll);
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    // the vertical layout of a row stands on a row's height as it comes: a taller row only has more room below
    let h0 = t.size.rem(CELL.1);
    let show = host.shown().clone();
    let roles = deck_show::roles::roles(&show, host.path.is_empty().then_some(&host.sub));
    let picked: Vec<String> = host.columns.clone();
    let columns_on = |id: &str| picked.iter().any(|c| c == id);
    let sel = (host.role.is_none() && host.columns.is_empty() && host.group.is_none()).then_some(host.row);
    let gh = if m.groups.is_empty() { 0.0 } else { t.size.rem(1.6) };
    let n = m.rows.len();
    let cols = m.columns.len();
    let mut wires: Vec<Hung> = Vec::new();
    let mut laid: Vec<super::wiring::RowLay> = Vec::new();
    let rad = (h0 * 0.19).min(t.size.rem(0.8));
    let fits = super::sizing::measure(d, t, host, m, rad);

    // the cells, under both heads; first the selection's bands
    d.clip(Some(l.body));
    if let Some(i) = sel.filter(|i| *i < n && cols > 0) {
        let a = l.span(i, 0, cols);
        d.rect(a.x, a.y + 1.0, a.w, a.h - 2.0, soft(t.color.selected, 0.45));
    }
    for (c, col) in m.columns.iter().enumerate() {
        if columns_on(&col.id) && n > 0 {
            let a = l.down(0, c, n);
            d.rect(a.x + 1.0, a.y, a.w - 2.0, a.h, soft(t.color.selected, 0.45));
        }
    }
    for (i, r) in m.rows.iter().enumerate().take(l.visible_rows().end.min(n)).skip(l.visible_rows().start) {
        let mut boxes = vec![None; r.cells.len()];
        for (c, cell) in r.cells.iter().enumerate() {
            let box_ = l.cell(i, c);
            // #551: adjacent cells of one question are one wide cell, drawn from the column that starts it; a column inside
            // it draws nothing but answers its press as the cell
            let (span, words) = match &cell.ask {
                Some(a) => (a.run.max(1), a.words()),
                None => (1, Some(cell.text.clone())),
            };
            let whole = Rect { x: box_.x + 3.0, y: box_.y + 3.0, w: l.span(i, c, span).w - 6.0, h: box_.h - 6.0 };
            let selected = host.role.is_none() && host.row == i && m.columns.get(c).is_some_and(|col| host.cells.contains(&col.id));
            let at = Rect { x: box_.x + 3.0, y: box_.y + 3.0, w: box_.w - 6.0, h: box_.h - 6.0 };
            push(host, at, &l.body, Hit::Cell(i, c));
            let c0 = (whole.x + pad + rad, whole.y + pad * 0.6 + rad);
            let on_launcher = Rect { x: c0.0 - rad, y: c0.1 - rad, w: 2.0 * rad, h: 2.0 * rad }.has(pointer.0, pointer.1);
            if cell.empty {
                // an empty cell: Ableton's stop slot
                stop_slot(d, t, c0, rad, cell.state, on_launcher);
                if runs {
                    push(host, Rect { x: c0.0 - rad - 2.0, y: c0.1 - rad - 2.0, w: 2.0 * rad + 4.0, h: 2.0 * rad + 4.0 }, &l.body, Hit::Launch(i, c));
                }
            } else if let Some(words) = words.filter(|_| !cell.keep) {
                let over = whole.has(pointer.0, pointer.1);
                boxes[c] = Some(whole);
                d.round_rect(whole.x, whole.y, whole.w, whole.h, t.size.radius, if over { t.color.hover } else { t.color.control });
                match cell.state {
                    State::Playing => d.outline(whole.x, whole.y, whole.w, whole.h, (t.size.radius, 2.0), t.color.text),
                    State::Queued => d.outline(whole.x, whole.y, whole.w, whole.h, (t.size.radius, 1.0), t.color.text),
                    State::Idle => d.outline(whole.x, whole.y, whole.w, whole.h, (t.size.radius, 1.0), t.color.line),
                }
                if cell.ask.is_some() {
                    d.rect(whole.x, whole.y + t.size.radius, 2.0, whole.h - 2.0 * t.size.radius, t.color.text);
                }
                launcher(d, t, c0, rad, cell.state, on_launcher);
                let tx = c0.0 + rad + pad * 0.75;
                d.text(tx, c0.1 - font * 0.62, font, &fit(d, &words, font, whole.x + whole.w - tx - pad), t.color.text);
                if let Some((words, color)) = note(t, host, (r, c), m, cell) {
                    let y = (c0.1 + rad + pad * 0.4).min(whole.y + whole.h - pad * 0.5 - t.size.label * 1.5);
                    cards::status(d, t, (whole.x + pad, y, whole.w - 2.0 * pad), &words, color);
                }
                if cell.off_row {
                    let (mx, my) = (whole.x + whole.w - pad * 0.9, whole.y + pad * 0.9);
                    d.ring(mx, my, 4.0, t.color.warn);
                    d.disc(mx, my, 2.0, t.color.warn);
                }
                if let (State::Playing, Some(p)) = (cell.state, cell.progress) {
                    let (bx, bw) = (whole.x + pad, whole.w - 2.0 * pad);
                    let by = whole.y + whole.h - 4.0;
                    d.rect(bx, by, bw, 2.0, t.color.line);
                    d.rect(bx, by, bw * p.clamp(0.0, 1.0), 2.0, t.color.text);
                }
                if runs {
                    push(host, Rect { x: c0.0 - rad - 2.0, y: c0.1 - rad - 2.0, w: 2.0 * rad + 4.0, h: 2.0 * rad + 4.0 }, &l.body, Hit::Launch(i, c));
                }
            } else if cell.keep {
                // a kept cell: what an earlier row started carries on, lit while it plays on through this row
                let color = if cell.state == State::Playing { t.color.text } else { cards::faint(t) };
                let tw = d.advance(small) * cell.text.chars().count() as f32;
                let text = fit(d, &cell.text, small, box_.w - 2.0 * pad);
                d.text(box_.x + ((box_.w - tw) * 0.5).max(pad), box_.y + (h0 - small * 1.25) * 0.5, small, &text, color);
            }
            if selected {
                d.outline(whole.x, whole.y, whole.w, whole.h, (t.size.radius, 1.5), t.color.text);
            }
            // a library card being dragged: the cell under the pointer is where it lands
            if host.carrying() && at.has(pointer.0, pointer.1) {
                d.outline(at.x, at.y, at.w, at.h, (t.size.radius, 2.0), t.color.go);
            }
        }
        // the cells of this row whose far nodes listen to each other, and the parts of one question, hang on wires from
        // ports at their feet
        let row = l.cell(i, 0);
        let foot = row.y + row.h - 3.0;
        let port = |c: usize| {
            let run = r.cells.get(c).and_then(|x| x.ask.as_ref()).map_or(1, |a| a.run.max(1));
            let s = l.span(i, c, run);
            (s.x + s.w * 0.5, foot)
        };
        for link in host.row_links(&r.id) {
            let deck_show::far::Holder::Outlet(to) = &link.holder else { continue };
            let col = |id: &str| m.columns.iter().position(|c| c.id == id);
            if let (Some(a), Some(b)) = (col(&link.from), col(to)) {
                // the signal runs from the holder to the cell that listens
                wires.push((port(b), port(a), Signal::Far));
            }
        }
        wires.extend(r.ask_links().into_iter().map(|(a, b)| (port(a), port(b), Signal::Vote)));
        laid.push(super::wiring::RowLay { i, id: r.id.clone(), cells: boxes });
    }
    // a row's bottom edge: before the ports, which take a press at a cell's foot first
    for i in l.visible_rows().filter(|i| *i < n) {
        if let Some(r) = l.edge_place(Edge::Row(i), GRIP) {
            host.hits.push((r, Hit::Edge(Edge::Row(i))));
        }
    }
    let unit = t.size.rem(1.0);
    for (a, b, s) in wires {
        wire(d, t, (a, b), (unit, host.flow), t.color.signals.of(s));
    }
    let columns: Vec<String> = m.columns.iter().map(|c| c.id.clone()).collect();
    super::wiring::draw(d, t, host, (&show, &columns), &laid, (&l.body, pointer));

    // the column heads: the groups over their columns, then each column's name, who takes it and its stop
    d.clip(Some(l.heads_top));
    for (gi, g) in m.groups.iter().enumerate() {
        let r = l.col_head(g.first, g.span);
        let on = host.group.as_ref() == Some(&g.name);
        if on {
            d.rect(r.x + 1.0, r.y, r.w - 2.0, gh - 4.0, soft(t.color.selected, 0.45));
        }
        match typed(host, |w| *w == Typing::GroupName(g.name.clone())) {
            Some(text) => super::rows::in_place(d, t, Rect { x: r.x + pad * 0.5, y: r.y + 1.0, w: r.w - pad, h: gh - 6.0 }, &text, t.size.label * 1.2),
            None => {
                controls::label(d, t, r.x + pad, r.y + 4.0, &fit(d, &g.name, t.size.label * 1.2, r.w - 2.0 * pad), t.color.muted);
            }
        }
        d.rect(r.x + pad, r.y + gh - 6.0, r.w - 2.0 * pad, if on { 2.0 } else { 1.0 }, if on { t.color.text } else { t.color.line });
        push(host, Rect { x: r.x, y: r.y, w: r.w, h: gh - 4.0 }, &l.heads_top, Hit::Group(gi));
    }
    for (c, col) in m.columns.iter().enumerate() {
        let r = l.col_head(c, 1);
        let y = r.y + gh;
        let role = roles.iter().position(|x| x.id == col.id);
        let on = columns_on(&col.id) || (role.is_some() && host.role.as_ref() == Some(&col.id));
        if on {
            // the column's band, the same x and width as its cells below
            d.rect(r.x + 1.0, y, r.w - 2.0, r.h - gh, soft(t.color.selected, 0.45));
            d.rect(r.x + pad, r.y + r.h - 3.0, r.w - 2.0 * pad, 2.0, t.color.text);
        }
        let stop = t.size.rem(0.85);
        let here = role.map(|k| deck_show::roles::here_text(roles[k].here)).unwrap_or_default();
        let hw = d.advance(small) * here.chars().count() as f32;
        match typed(host, |w| matches!(w, Typing::ColumnName(id) | Typing::Label(id) if *id == col.id)) {
            Some(text) => super::rows::in_place(d, t, Rect { x: r.x + pad * 0.5, y: y + 1.0, w: r.w - 3.0 * pad - stop, h: font * 1.5 }, &text, font),
            None => {
                d.text(r.x + pad, y + 4.0, font, &fit(d, &col.name, font, r.w - 3.0 * pad - stop - hw), t.color.text);
                d.text(r.x + r.w - 2.0 * pad - stop - hw, y + 4.0 + (font - small) * 0.6, small, &here, t.color.muted);
            }
        }
        let kind = col.filter.as_ref().map_or_else(|| col.detail.clone(), |f| format!("{} · {f}", col.detail));
        controls::label(d, t, r.x + pad, y + 6.0 + font * 1.3, &fit(d, &kind, t.size.label * 1.2, r.w - 2.0 * pad), t.color.muted);
        push(host, Rect { x: r.x, y, w: r.w, h: r.h - gh }, &l.heads_top, Hit::Column(c));
        // the column's stop: a small square, filled only while something plays there
        let s = Rect { x: r.x + r.w - pad - stop, y: y + 4.0 + (font * 1.25 - stop) * 0.5, w: stop, h: stop };
        let live = runs && col.stops;
        if runs && s.has(pointer.0, pointer.1) {
            d.round_rect(s.x - 2.0, s.y - 2.0, s.w + 4.0, s.h + 4.0, 3.0, t.color.hover);
        }
        if live {
            d.round_rect(s.x, s.y, s.w, s.h, 2.0, t.color.text);
        } else {
            d.outline(s.x, s.y, s.w, s.h, (2.0, 1.0), t.color.muted);
        }
        if runs {
            push(host, Rect { x: s.x - 3.0, y: s.y - 3.0, w: s.w + 6.0, h: s.h + 6.0 }, &l.heads_top, Hit::Stop(c));
        }
    }

    // the row heads: ▶, number, name, the fire tag; in Show the new row's place under the last
    d.clip(Some(l.heads_left));
    for (i, r) in m.rows.iter().enumerate().take(l.visible_rows().end.min(n)).skip(l.visible_rows().start) {
        let h = l.row_head(i);
        let whole = Rect { x: h.x, y: h.y + 3.0, w: h.w - pad, h: h.h - 6.0 };
        if sel == Some(i) {
            d.rect(whole.x, h.y + 1.0, h.w, h.h - 2.0, soft(t.color.selected, 0.45));
        } else if whole.has(pointer.0, pointer.1) {
            d.round_rect(whole.x, whole.y, whole.w, whole.h, t.size.radius, t.color.hover);
        }
        if r.suggested {
            d.outline(whole.x, whole.y, whole.w, whole.h, (t.size.radius, 2.0), t.color.warn);
        }
        push(host, whole, &l.heads_left, Hit::Row(i));
        let rr = (h0 * 0.26).min(t.size.rem(1.0));
        let c0 = (h.x + pad + 3.0 + rr, h.y + h0 * 0.5);
        launcher(d, t, c0, rr, r.state, Rect { x: c0.0 - rr, y: c0.1 - rr, w: 2.0 * rr, h: 2.0 * rr }.has(pointer.0, pointer.1));
        if runs {
            push(host, Rect { x: c0.0 - rr - 2.0, y: c0.1 - rr - 2.0, w: 2.0 * rr + 4.0, h: 2.0 * rr + 4.0 }, &l.heads_left, Hit::Fire(i));
        }
        let nx = c0.0 + rr + pad;
        let number = format!("{:02}", r.number);
        let top = if r.chip.is_some() { h.y + h0 * 0.5 - font * 1.2 } else { h.y + (h0 - font * 1.25) * 0.5 };
        let nw = d.text(nx, top + (font - small) * 0.5, small, &number, t.color.muted);
        let tx = nx + nw + pad * 0.75;
        // a suggestion that stands says so at the end of the name's line
        let waits = r.chip.as_ref().is_some_and(|c| c.waiting);
        let wpx = t.size.label * 1.2;
        let ww = if waits { d.advance(wpx) * deck_show::view::WAITS.chars().count() as f32 + pad } else { 0.0 };
        // a row without its chip: the empty ⚡ slot at the end of the name's line
        let slot_w = if r.chip.is_none() { font * 1.25 + 4.0 } else { 0.0 };
        let name = Rect { x: tx - 3.0, y: top - 2.0, w: whole.x + whole.w - tx - pad + 3.0 - ww - slot_w, h: font * 1.25 + 4.0 };
        match typed(host, |w| *w == Typing::RowName(r.id.clone())) {
            Some(text) => super::rows::in_place(d, t, name, &text, font),
            None => {
                d.text(tx, top, font, &fit(d, &r.id, font, name.w - 3.0), t.color.text);
            }
        }
        // the selected row's name, pressed again: renamed in place
        if sel == Some(i) {
            push(host, name, &l.heads_left, Hit::RowName);
        }
        // the ⚡ slot: the fire tag, or empty; pressed, Find opens under it
        let open = host.sheet == Some(super::Sheet::Event) && host.row == i;
        let slot = match &r.chip {
            Some(chip) => {
                let y = h.y + h0 * 0.5 + 3.0;
                let w = fire_tag(d, t, (tx, y, whole.x + whole.w - tx - pad), chip);
                if waits {
                    d.text(whole.x + whole.w - ww + pad * 0.5, top + (font - wpx) * 0.5, wpx, deck_show::view::WAITS, t.color.text);
                }
                Rect { x: tx, y, w, h: cards::tag_height(t) }
            }
            None => {
                let s = font * 1.25;
                let r = Rect { x: whole.x + whole.w - pad * 0.5 - s - ww, y: top - 1.0, w: s, h: s };
                let lit = open || r.has(pointer.0, pointer.1);
                d.outline(r.x, r.y, r.w, r.h, (s * 0.5, 1.0), if lit { t.color.text } else { t.color.line });
                d.icon(r.x + s * 0.2, r.y + s * 0.2, s * 0.6, "bolt", if lit { t.color.text } else { t.color.muted });
                r
            }
        };
        if open {
            d.outline(slot.x - 2.0, slot.y - 2.0, slot.w + 4.0, slot.h + 4.0, (t.size.radius * 2.0, 1.0), t.color.text);
        }
        if i == host.row {
            host.slot_at = Some(slot);
        }
        push(host, slot, &l.heads_left, Hit::Slot(i));
    }
    if l.visible_rows().end > n {
        let h = l.row_head(n);
        let whole = Rect { x: h.x, y: h.y + 3.0, w: h.w - pad, h: h.h - 6.0 };
        if whole.has(pointer.0, pointer.1) {
            d.round_rect(whole.x, whole.y, whole.w, whole.h, t.size.radius, t.color.hover);
        }
        d.icon(h.x + pad + 3.0, h.y + (h.h - font) * 0.5, font, "plus", t.color.muted);
        d.text(h.x + 2.0 * pad + 3.0 + font, h.y + (h.h - font * 1.25) * 0.5, font, deck_show::rows::ADD, t.color.muted);
        push(host, whole, &l.heads_left, Hit::RowAdd);
    }

    // the edges of the heads and the columns, over their heads; the one under the pointer or in the hand as a handle
    d.clip(Some(area));
    for e in std::iter::once(Edge::Heads).chain(l.visible_cols().map(Edge::Column)) {
        // under the groups' strip: a group's head is pressed whole
        if let Some(r) = l.edge_place(e, GRIP).and_then(|r| Laid::shown(&r, &Rect { y: area.y + gh, h: area.h - gh, ..area })) {
            host.hits.push((r, Hit::Edge(e)));
        }
    }
    let held = host.resizing.map(|r| r.edge);
    let hot = held.or_else(|| match host.hit(pointer.0, pointer.1) {
        Some(Hit::Edge(e)) => Some(e),
        _ => None,
    });
    if let Some(e) = hot.filter(|e| l.edge_place(*e, GRIP).is_some()) {
        let (a, b) = l.edge_line(e);
        cards::handle(d, t, (a, b), held.is_some());
        if let Some(words) = host.edge_badge() {
            let at = match e {
                Edge::Row(_) => (l.area.x + l.table.head.0 * 0.5, a.1 + 3.0),
                _ => (a.0, l.area.y + 1.0),
            };
            cards::badge(d, t, at, &words);
        }
    }

    // the foot: what is out of view
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
    (l, fits)
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{host_with, press, H, W};
    use super::super::{author, Hit, Host};
    use deck_kit::base::draw::DrawList;
    use deck_kit::base::tokens::Tokens;
    use deck_show::view::{self, State};

    const DOC: &str = r#"{ "id": "band", "version": 1,
      "toons": ["catpope"], "outlets": [{ "id": "left", "group": "stages" }, { "id": "right", "group": "stages" }, { "id": "lights", "kind": "device", "group": "light" }],
      "rows": [
        { "id": "opening", "cells": { "left": { "mode": "play", "piece": "scene:nave" }, "right": { "mode": "play", "piece": "scene:vigil" } } },
        { "id": "together", "fire": { "event": "meet", "how": "suggest" }, "cells": { "right": { "mode": "play", "piece": "scene:choir" } } }
      ] }"#;

    fn frame(host: &mut Host) -> Vec<String> {
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        host.hits.clear();
        author::draw(&mut d, &t, host, W, H, (0.0, 0.0));
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
        press(&mut host, |h| *h == Hit::Cell(0, 0));
        assert_eq!(host.cell().map(String::as_str), Some("left"), "a press on the cell selects it");
        assert_eq!(state(&host, 0, 0), State::Idle, "and launches nothing");
        press(&mut host, |h| *h == Hit::Launch(0, 0));
        assert_eq!(state(&host, 0, 0), State::Playing, "its ▶ launches it");
        assert_eq!(view::log(host.sub.log(), 1)[0].1, "off-row: left > opening");
        // the row's ▶ sits in its head; the rest of the head selects the row
        press(&mut host, |h| *h == Hit::Row(1));
        assert_eq!((host.row, view::matrix(&host.sub).rows[1].state), (1, State::Idle));
        press(&mut host, |h| *h == Hit::Fire(1));
        let m = view::matrix(&host.sub);
        assert_eq!((m.rows[1].state, m.rows[1].cells[1].state, m.rows[1].cells[0].state), (State::Playing, State::Playing, State::Playing), "left keeps what row 1 started");
        press(&mut host, |h| *h == Hit::Stop(1));
        assert_eq!(state(&host, 1, 1), State::Idle, "right is stopped");
        assert_eq!(view::log(host.sub.log(), 1)[0].1, "right stopped");
        let m = view::matrix(&host.sub);
        assert!(m.columns[0].stops && !m.columns[1].stops);
    }

    #[test]
    fn a_cell_playing_a_published_scene_with_a_newer_draft_says_so() {
        let mut host = host_with(DOC);
        let book = super::super::rows::tests::BOOK.replace(r#""key": "scene:nave", "state": "published""#, r#""key": "scene:nave", "state": "draft""#);
        host.book = Some(std::sync::Arc::new(deck_space::rig::book::Book::parse(&book).unwrap()));
        assert!(!frame(&mut host).iter().any(|t| t.starts_with("a newer draft")), "idle: nothing said");
        press(&mut host, |h| *h == Hit::Fire(0));
        let texts = frame(&mut host);
        assert_eq!(texts.iter().filter(|t| t.starts_with("a newer draft")).count(), 1, "nave plays with a newer draft, vigil without: {texts:?}");
    }

    #[test]
    fn a_kept_cell_is_a_bare_arrow_and_the_groups_head_their_columns() {
        let mut host = host_with(DOC);
        host.two_d = false;
        let texts = frame(&mut host);
        assert_eq!(texts.iter().filter(|t| *t == view::CARRIES_ON).count(), 3, "row 1's lights, row 2's left and lights (the inspector lists no cells): {texts:?}");
        assert!(place(&host, Hit::Cell(1, 0)).is_some(), "a kept cell is still a place to select");
        assert!(place(&host, Hit::Launch(1, 0)).is_none(), "but has no ▶ of its own");
        assert!(texts.iter().any(|t| t == "STAGES") && texts.iter().any(|t| t == "LIGHT"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "01") && texts.iter().any(|t| t == "opening") && texts.iter().any(|t| t == "meet") && texts.iter().any(|t| t == deck_show::rows::ADD), "the chip, the new row's place: {texts:?}");
        let stop = |host: &Host, c| place(host, Hit::Stop(c)).unwrap();
        assert_eq!(stop(&host, 0).y, stop(&host, 2).y);
        // a sub conductor's document only shows: no ▶, no stop
        host.path.push("x".into());
        frame(&mut host);
        assert!(!host.hits.iter().any(|(_, h)| matches!(h, Hit::Launch(..) | Hit::Fire(_) | Hit::Stop(_))));
    }

    #[test]
    fn a_band_too_large_scrolls_both_ways_under_its_heads_and_counts_what_is_out_of_view() {
        let outlets: Vec<String> = (0..12).map(|i| format!(r#"{{ "id": "o{i}" }}"#)).collect();
        let rows: Vec<String> = (0..30).map(|i| format!(r#"{{ "id": "r{i}", "cells": {{ "o0": {{ "mode": "play", "piece": "scene:nave" }} }} }}"#)).collect();
        let doc = format!(r#"{{ "id": "big", "version": 1, "toons": ["catpope"], "outlets": [{}], "rows": [{}] }}"#, outlets.join(","), rows.join(","));
        let mut host = host_with(&doc);
        let texts = frame(&mut host);
        let counter = |texts: &[String], end: &str| texts.iter().find(|t| t.starts_with('+') && t.ends_with(end)).cloned();
        let (rows0, cols0) = (counter(&texts, "↓").expect("rows out of view"), counter(&texts, "→").expect("columns out of view"));
        let row_head = place(&host, Hit::Fire(0)).unwrap();
        let stop = place(&host, Hit::Stop(0)).unwrap();
        assert!(host.band_wheel(row_head.x + 4.0, row_head.y + 4.0, (-2.0, -3.0)));
        assert!(host.show_scroll.x > 0.0 && host.show_scroll.y > 0.0);
        let texts = frame(&mut host);
        assert!(place(&host, Hit::Fire(0)).is_none(), "row 1 scrolled away under the column heads");
        assert_eq!(place(&host, Hit::Fire(3)).unwrap().x, row_head.x, "a row head stays at the left");
        assert_eq!(place(&host, Hit::Stop(2)).unwrap().y, stop.y, "a column head stays on top");
        assert!(place(&host, Hit::Stop(0)).is_none(), "column 1 scrolled away under the row heads");
        assert_ne!(counter(&texts, "↓").unwrap(), rows0);
        assert_ne!(counter(&texts, "→").unwrap(), cols0);
        assert!(!host.band_wheel(1.0, 1.0, (0.0, -1.0)), "the wheel elsewhere leaves it");
    }

    const SLOTS: &str = r#"{ "id": "slots", "version": 1,
      "toons": ["catpope"], "outlets": [{ "id": "left" }, { "id": "right" }, { "id": "lights", "kind": "device" }],
      "rows": [
        { "id": "opening", "cells": { "left": { "mode": "play", "piece": "scene:nave" }, "right": { "mode": "play", "piece": "scene:vigil" } } },
        { "id": "dark", "fire": { "event": "meet", "how": "suggest", "moment": "bar" }, "cells": { "left": { "mode": "black" } } },
        { "id": "storm", "fire": { "event": "thunder", "how": "auto" }, "cells": { "right": { "mode": "play", "piece": "scene:storm" } } },
        { "id": "old", "fire": { "event": "door", "how": "manual" } }
      ] }"#;

    fn key(host: &mut Host, k: &str, ctrl: bool) -> bool {
        host.key(&winit::keyboard::Key::Character(k.into()), ctrl)
    }

    #[test]
    fn an_empty_cell_is_a_stop_slot_its_launch_and_its_rows_stop_the_column_and_a_kept_cell_continues() {
        let mut host = host_with(SLOTS);
        let texts = frame(&mut host);
        assert!(!texts.iter().any(|t| t == "stop" || t == "black"), "a stop slot has no words: {texts:?}");
        assert!(place(&host, Hit::Launch(1, 0)).is_some(), "but its own launch");
        press(&mut host, |h| *h == Hit::Fire(0));
        assert_eq!((state(&host, 0, 0), state(&host, 0, 1)), (State::Playing, State::Playing));
        // the stop slot launched: left stops, right plays on
        press(&mut host, |h| *h == Hit::Launch(1, 0));
        let stage = |host: &Host, c: &str| host.sub.conductor().on_stage(c).map(|s| s.playing.clone());
        assert_eq!(stage(&host, "left"), Some(deck_show::conductor::Playing::Black));
        assert_eq!(state(&host, 0, 1), State::Playing, "right keeps playing");
        // its row fired: the same stop on left, right continues (↑ lit)
        press(&mut host, |h| *h == Hit::Fire(0));
        press(&mut host, |h| *h == Hit::Fire(1));
        assert_eq!(stage(&host, "left"), Some(deck_show::conductor::Playing::Black));
        let m = view::matrix(&host.sub);
        assert!(m.rows[1].cells[1].keep && m.rows[1].cells[1].state == State::Playing, "↑ continues what row 1 started");
        assert!(!m.columns[0].stops && m.columns[1].stops, "the column stop fills only where something plays");
        // Delete on a selected cell empties it
        press(&mut host, |h| *h == Hit::Cell(0, 1));
        assert!(host.key(&winit::keyboard::Key::Named(winit::keyboard::NamedKey::Delete), false));
        assert_eq!(host.shown().rows[0].cells.get("right"), Some(&deck_show::Cell::Black));
    }

    #[test]
    fn a_rows_fire_shows_as_a_chip_auto_filled_suggest_outlined_and_manual_shows_nothing() {
        let mut host = host_with(SLOTS);
        let texts = frame(&mut host);
        assert!(texts.iter().any(|t| t == "meet") && texts.iter().any(|t| t == "thunder"), "{texts:?}");
        // the band alone, without the node view (which lists the row's events)
        host.two_d = false;
        let band = frame(&mut host);
        assert!(!band.iter().any(|t| t == "door" || t.contains("hand") || t.contains("HAND")), "manual shows nothing: {band:?}");
        assert!(!texts.iter().any(|t| t == view::WAITS));
        let story = serde_json::json!({ "id": "meet" }).as_object().cloned().unwrap();
        host.sub.on_wire(&protocol::Envelope::new("reader", protocol::Message::Story(story)), 5);
        let texts = frame(&mut host);
        assert!(texts.iter().any(|t| t == view::WAITS), "a standing suggestion waits for GO: {texts:?}");
        // a cell started off its row: the corner mark, no word
        press(&mut host, |h| *h == Hit::Launch(2, 1));
        let texts = frame(&mut host);
        assert!(view::matrix(&host.sub).rows[2].cells[1].off_row && !texts.iter().any(|t| t.contains("hand")), "no word for it: {texts:?}");
    }

    #[test]
    fn selection_shows_as_bands_and_names_are_typed_in_place_with_ctrl_r() {
        let mut host = host_with(DOC);
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        let soft = super::soft(t.color.selected, 0.45);
        let bands = |d: &DrawList| d.prims().iter().filter(|p| matches!(p, deck_kit::base::draw::Prim::Rect { color, .. } if *color == soft)).count();
        author::draw(&mut d, &t, &mut host, super::super::rows::tests::W, super::super::rows::tests::H, (0.0, 0.0));
        assert_eq!(bands(&d), 2, "row 1 selected: its band over the cells and its head");
        // a column head: its band from the head down to the last row, lined up with the cells
        press(&mut host, |h| *h == Hit::Column(1));
        let head = place(&host, Hit::Column(1)).unwrap();
        let cell = place(&host, Hit::Cell(1, 1)).unwrap();
        assert!((head.x - (cell.x - 3.0)).abs() < 0.5, "the head stands over its column");
        assert_eq!(host.columns, ["right"]);
        assert!(key(&mut host, "r", true));
        assert_eq!(frame(&mut host).iter().filter(|t| *t == "right|").count(), 1, "the column's name typed in its head");
        super::super::rows::tests::typed(&mut host, "-choir");
        assert_eq!(host.status, "role right is called right-choir");
        // ctrl+r on a row: its name in its head
        press(&mut host, |h| *h == Hit::Row(1));
        assert!(host.columns.is_empty());
        assert!(key(&mut host, "r", true));
        assert!(frame(&mut host).iter().any(|t| t == "together|"));
        host.type_key(&winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape));
        assert_eq!(host.shown().rows[1].id, "together", "Escape leaves it as it was");
    }

    #[test]
    fn columns_selected_with_shift_are_grouped_by_ctrl_g_or_the_heads_menu_and_named_in_place() {
        let mut host = host_with(SLOTS);
        press(&mut host, |h| *h == Hit::Column(0));
        let right = place(&host, Hit::Column(1)).unwrap();
        host.press(host.hit(right.x + 4.0, right.y + 4.0).unwrap().with_shift());
        assert_eq!(host.columns, ["left", "right"]);
        assert!(key(&mut host, "g", true));
        assert_eq!(host.status, "left · right grouped as group");
        assert!(frame(&mut host).iter().any(|t| t == "group|"), "named next, in place");
        super::super::rows::tests::typed(&mut host, "s");
        assert_eq!(host.status, "group group is called groups");
        // the group's head selects it and its columns; ctrl+r names it again
        press(&mut host, |h| *h == Hit::Group(0));
        assert_eq!((host.group.as_deref(), host.columns.clone()), (Some("groups"), vec!["left".to_string(), "right".to_string()]));
        // columns apart: the core refuses in its words (left and lights, right between)
        press(&mut host, |h| *h == Hit::Column(0));
        let lights = place(&host, Hit::Column(2)).unwrap();
        host.press(host.hit(lights.x + 4.0, lights.y + 4.0).unwrap().with_shift());
        let at = place(&host, Hit::Column(2)).unwrap();
        assert!(host.context(at.x + 4.0, at.y + 4.0), "the head's right click opens its menu");
        assert_eq!(host.context_lines(), ["Group the selected columns", "Ungroup", "Rename"]);
        host.key(&winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter), false);
        assert_eq!(host.status, "not changed: show: outlet 'lights': the group 'group' is split, its columns stand apart");
    }

    #[test]
    fn the_parts_of_a_question_apart_hang_on_a_wire_whose_beads_run_only_with_signal_flow() {
        use deck_kit::base::draw::Prim;
        let mut host = host_with(include_str!("../../../demos/vote-and-question.show.json"));
        host.two_d = false;
        let t = Tokens::default();
        let drawn = |host: &mut Host| {
            let mut d = DrawList::new(0.6);
            host.hits.clear();
            author::draw(&mut d, &t, host, W, H, (0.0, 0.0));
            let vote = t.color.signals.vote;
            let bead = [vote[0], vote[1], vote[2], vote[3] * deck_kit::base::wire::BEAD_ALPHA];
            let wire = d.prims().iter().filter(|p| matches!(p, Prim::Segment { color, .. } if *color == vote)).count();
            let dots = d.prims().iter().filter(|p| matches!(p, Prim::Rect { color, r, .. } if *color == bead && *r > 0.0)).count();
            (wire, dots)
        };
        host.flow = None;
        let (wire, still) = drawn(&mut host);
        assert_eq!(wire, deck_kit::base::wire::SAMPLES, "one wire, left to right");
        host.flow = Some(0.0);
        let (_, running) = drawn(&mut host);
        assert!(running > still, "beads on it: {still} → {running}");
    }

    fn enter(host: &mut Host) -> bool {
        host.key(&winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter), false)
    }

    #[test]
    fn a_selected_head_pressed_again_or_entered_is_renamed_in_place_and_escape_leaves_it() {
        let mut host = host_with(DOC);
        press(&mut host, |h| *h == Hit::Column(0));
        assert!(host.typing.is_none(), "the first press selects");
        press(&mut host, |h| *h == Hit::Column(0));
        assert!(frame(&mut host).iter().any(|t| t == "left|"), "the second one names it in its head");
        super::super::rows::tests::typed(&mut host, "-nave");
        assert_eq!(host.status, "role left is called left-nave");
        // a group's head: selected, then Enter names it
        press(&mut host, |h| *h == Hit::Group(0));
        assert!(enter(&mut host));
        assert!(frame(&mut host).iter().any(|t| t == "stages|"));
        host.type_key(&winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape));
        assert_eq!(host.shown().outlets[0].group.as_deref(), Some("stages"), "Escape leaves it as it was");
        press(&mut host, |h| *h == Hit::Group(0));
        assert!(frame(&mut host).iter().any(|t| t == "stages|"), "pressed again: in place too");
        super::super::rows::tests::typed(&mut host, "-wide");
        assert_eq!(host.shown().outlets[1].group.as_deref(), Some("stages-wide"));
        // a row head selected, no cell: Enter names the row
        press(&mut host, |h| *h == Hit::Row(1));
        host.cells.clear();
        assert!(enter(&mut host));
        assert!(frame(&mut host).iter().any(|t| t == "together|"));
    }

    #[test]
    fn a_row_heads_right_click_renames_adds_after_and_removes_it() {
        let mut host = host_with(DOC);
        frame(&mut host);
        let head = place(&host, Hit::Row(0)).unwrap();
        assert!(host.context(head.x + head.w - 2.0, head.y + 2.0), "the head's right click opens its menu");
        assert_eq!(host.row, 0);
        assert_eq!(host.context_lines(), ["Rename", "New row", deck_show::rows::REMOVE]);
        assert!(enter(&mut host), "Rename");
        assert!(frame(&mut host).iter().any(|t| t == "opening|"), "the name in its head");
        super::super::rows::tests::typed(&mut host, "-act");
        assert_eq!(host.shown().rows[0].id, "opening-act");
        // another row's head: selected first; a new row after it
        let head = place(&host, Hit::Row(1)).unwrap();
        assert!(host.context(head.x + head.w - 2.0, head.y + 2.0));
        assert_eq!(host.row, 1);
        for c in "new".chars() {
            host.key(&winit::keyboard::Key::Character(c.to_string().into()), false);
        }
        assert!(enter(&mut host));
        assert_eq!(host.shown().rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["opening-act", "together", "row"]);
        // its remove asks first
        frame(&mut host);
        let head = place(&host, Hit::Row(0)).unwrap();
        assert!(host.context(head.x + head.w - 2.0, head.y + 2.0));
        for c in "remove".chars() {
            host.key(&winit::keyboard::Key::Character(c.to_string().into()), false);
        }
        assert!(enter(&mut host));
        assert_eq!(host.shown().rows.len(), 3, "not yet");
        press(&mut host, |h| *h == Hit::AskYes);
        assert_eq!(host.shown().rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["together", "row"]);
        // the inspector carries no second remove
        frame(&mut host);
        assert!(place(&host, Hit::RowRemove).is_none());
    }
}
