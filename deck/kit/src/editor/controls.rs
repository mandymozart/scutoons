//! The controls, on the kit's tokens. A control draws itself into the list and answers with where it landed; what a press
//! means stays with the caller, the state with its one owner. The look: a pressed segment in the pressed colours, small
//! uppercase spaced labels, one primary action per view, an arrow out on what leaves for elsewhere.
use crate::base::draw::{DrawList, Font};
pub use crate::base::draw::Rect;
use crate::base::tokens::Tokens;

/// `text` in at most `cols` characters, an ellipsis where it is cut
pub fn fit(text: &str, cols: usize) -> String {
    if text.chars().count() <= cols {
        return text.to_string();
    }
    let mut s: String = text.chars().take(cols.saturating_sub(1)).collect();
    if cols > 0 {
        s.push('…');
    }
    s
}

/// `text` in the sans face within `room` pixels at size `px`, an ellipsis where it is cut
pub fn fit_sans(d: &DrawList, px: f32, text: &str, room: f32) -> String {
    if d.sans_width(px, text) <= room {
        return text.to_string();
    }
    let mut s = String::new();
    for c in text.chars() {
        s.push(c);
        if d.sans_width(px, &s) + d.sans_width(px, "…") > room {
            s.pop();
            break;
        }
    }
    if room > 0.0 {
        s.push('…');
    }
    s
}

/// a sentence on a filled strip, kept inside `max_x`
pub fn chip(d: &mut DrawList, t: &Tokens, at: (f32, f32), max_x: f32, text: &str, bad: bool) {
    let px = t.size.font;
    let w = d.sans_width(px, text) + 16.0;
    let x = at.0.min(max_x - w - 8.0).max(8.0);
    let h = px * 1.25 + 10.0;
    d.round_rect(x, at.1, w, h, t.size.radius, if bad { t.color.danger } else { t.color.surface_head });
    d.sans(x + 8.0, at.1 + 5.0, px, text, if bad { t.color.bright } else { t.color.text });
}

/// a small uppercase label, letter-spaced; the width it took
pub fn label(d: &mut DrawList, t: &Tokens, x: f32, y: f32, text: &str, color: [f32; 4]) -> f32 {
    d.set(Font::Sans, x, y, t.size.label, &text.to_uppercase(), color, t.size.spacing)
}

/// how wide `label` draws `text`
pub fn label_width(d: &DrawList, t: &Tokens, text: &str) -> f32 {
    d.measure(Font::Sans, t.size.label, &text.to_uppercase(), t.size.spacing)
}

/// the arrow of an action that leads elsewhere, in a square of `size`
pub fn arrow_out(d: &mut DrawList, (x, y): (f32, f32), size: f32, color: [f32; 4]) {
    let (a, b) = ((x + size * 0.2, y + size * 0.8), (x + size * 0.8, y + size * 0.2));
    let thick = (size / 9.0).max(1.0);
    d.segment(a, b, thick, color);
    d.segment((b.0 - size * 0.45, b.1), b, thick, color);
    d.segment((b.0, b.1 + size * 0.45), b, thick, color);
}

/// the view's one primary action: lime, its edge, ink text, an arrow out when it leads elsewhere; its place
pub fn primary(d: &mut DrawList, t: &Tokens, r: Rect, text: &str, outward: bool) -> Rect {
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.primary);
    d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.0), t.color.primary_line);
    let px = t.size.font;
    let w = d.sans_width(px, text);
    let tx = if outward { r.x + t.size.pad * 1.5 } else { r.x + (r.w - w) * 0.5 };
    d.sans(tx, r.y + (r.h - px * 1.25) * 0.5, px, text, t.color.primary_ink);
    if outward {
        let s = px * 0.8;
        arrow_out(d, (r.x + r.w - t.size.pad * 1.5 - s, r.y + (r.h - s) * 0.5), s, t.color.primary_ink);
    }
    r
}

/// pills side by side, one of them on; their places
pub fn pills(d: &mut DrawList, t: &Tokens, x: f32, y: f32, h: f32, names: &[&str], on: usize) -> Vec<Rect> {
    pills_rounded(d, t, (x, y, h), names, on, t.size.radius)
}

/// the same, the one on rounded by `radius` (inside a pill: the pill's radius less the inset, so their corners share a centre)
pub fn pills_rounded(d: &mut DrawList, t: &Tokens, (x, y, h): (f32, f32, f32), names: &[&str], on: usize, radius: f32) -> Vec<Rect> {
    let px = t.size.small;
    let mut cx = x;
    let mut out = Vec::with_capacity(names.len());
    for (i, name) in names.iter().enumerate() {
        let w = d.sans_width(px, name) + 2.0 * t.size.pad;
        if i == on {
            d.round_rect(cx, y, w, h, radius, t.color.pressed);
        }
        d.sans(cx + t.size.pad, y + (h - px * 1.25) * 0.5, px, name, if i == on { t.color.pressed_text } else { t.color.text });
        out.push(Rect { x: cx, y, w, h });
        cx += w + 2.0;
    }
    out
}

/// #478: chips that are each on or off (a filter's facts), side by side and on to the next line at `w`, a tree row high
/// at most; their places and the height they took
pub fn chips(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), names: &[&str], on: &[bool]) -> (Vec<Rect>, f32) {
    let (px, h, gap) = (t.size.small, t.size.tree_row - 4.0, 4.0);
    let (mut cx, mut cy) = (x, y);
    let mut out = Vec::with_capacity(names.len());
    for (i, name) in names.iter().enumerate() {
        let cw = d.sans_width(px, name) + t.size.pad;
        if cx > x && cx + cw > x + w {
            cx = x;
            cy += h + gap;
        }
        let lit = on.get(i).copied().unwrap_or(false);
        if lit {
            d.round_rect(cx, cy, cw, h, t.size.radius, t.color.accent);
        } else {
            d.outline(cx, cy, cw, h, (t.size.radius, 1.0), t.color.line);
        }
        d.sans(cx + t.size.pad * 0.5, cy + (h - px * 1.25) * 0.5, px, name, if lit { t.color.surface } else { t.color.muted });
        out.push(Rect { x: cx, y: cy, w: cw, h });
        cx += cw + gap;
    }
    let used = if names.is_empty() { 0.0 } else { cy + h - y };
    (out, used)
}

/// #478: a small menu under `at`: a tree row per line, a tick before a ticked one; its lines' places
pub fn menu(d: &mut DrawList, t: &Tokens, at: (f32, f32), w: f32, lines: &[&str], ticked: &[bool], pointer: (f32, f32)) -> Vec<Rect> {
    let (h, px) = (t.size.tree_row, t.size.font);
    d.round_rect(at.0, at.1, w, h * lines.len() as f32 + 4.0, t.size.radius, t.color.surface_head);
    d.outline(at.0, at.1, w, h * lines.len() as f32 + 4.0, (t.size.radius, 1.0), t.color.line);
    let mut out = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        let r = Rect { x: at.0 + 2.0, y: at.1 + 2.0 + i as f32 * h, w: w - 4.0, h };
        if r.has(pointer.0, pointer.1) {
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.hover);
        }
        if ticked.get(i).copied().unwrap_or(false) {
            d.disc(r.x + 8.0, r.y + h * 0.5, 3.0, t.color.accent);
        }
        let line = fit_sans(d, px, line, w - 24.0);
        d.sans(r.x + 16.0, r.y + (h - px * 1.25) * 0.5, px, &line, t.color.text);
        out.push(r);
    }
    out
}

/// a line of a menu of actions: its words, whether it can be done now, whether it leaves for the browser
#[derive(Debug, Clone, PartialEq)]
pub struct Action<'a> {
    pub label: &'a str,
    pub enabled: bool,
    pub external: bool,
}

/// a menu of actions under `at`: one that cannot be done now is muted and takes no hover; its lines' places
pub fn actions(d: &mut DrawList, t: &Tokens, at: (f32, f32), w: f32, lines: &[Action], pointer: (f32, f32)) -> Vec<Rect> {
    let (h, px, pad) = (t.size.row, t.size.font, t.size.pad);
    let whole = h * lines.len() as f32 + pad;
    d.round_rect(at.0, at.1, w, whole, t.size.radius * 2.0, t.color.surface);
    d.outline(at.0, at.1, w, whole, (t.size.radius * 2.0, 1.0), t.color.line);
    let mut out = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        let r = Rect { x: at.0 + pad * 0.5, y: at.1 + pad * 0.5 + i as f32 * h, w: w - pad, h };
        if line.enabled && r.has(pointer.0, pointer.1) {
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.hover);
        }
        let ink = if line.enabled { t.color.text } else { t.color.muted };
        let label = fit_sans(d, px, line.label, r.w - 3.0 * pad - px);
        d.sans(r.x + pad, r.y + (h - px * 1.25) * 0.5, px, &label, ink);
        if line.external {
            let s = px * 0.8;
            arrow_out(d, (r.x + r.w - pad - s, r.y + (h - s) * 0.5), s, ink);
        }
        out.push(r);
    }
    out
}

/// a titled list of things that are on or off, in the look of `actions`: a dot before each one on; its lines' places
pub fn checks(d: &mut DrawList, t: &Tokens, at: (f32, f32), w: f32, title: &str, lines: &[(&str, bool)], pointer: (f32, f32)) -> Vec<Rect> {
    let (h, px, pad) = (t.size.row, t.size.font, t.size.pad);
    let whole = h * (lines.len() + 1) as f32 + pad;
    d.round_rect(at.0, at.1, w, whole, t.size.radius * 2.0, t.color.surface);
    d.outline(at.0, at.1, w, whole, (t.size.radius * 2.0, 1.0), t.color.line);
    label(d, t, at.0 + pad * 1.5, at.1 + pad * 0.5 + (h - t.size.label * 1.25) * 0.5, title, t.color.muted);
    let mut out = Vec::with_capacity(lines.len());
    for (i, &(line, on)) in lines.iter().enumerate() {
        let r = Rect { x: at.0 + pad * 0.5, y: at.1 + pad * 0.5 + (i + 1) as f32 * h, w: w - pad, h };
        if r.has(pointer.0, pointer.1) {
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.hover);
        }
        if on {
            d.disc(r.x + pad * 1.5, r.y + h * 0.5, t.size.rem(0.21), t.color.accent);
        }
        let line = fit_sans(d, px, line, r.w - 4.0 * pad);
        d.sans(r.x + pad * 3.0, r.y + (h - px * 1.25) * 0.5, px, &line, t.color.text);
        out.push(r);
    }
    out
}

/// a lone icon you press, its ground showing under the pointer; its place
pub fn icon_button(d: &mut DrawList, t: &Tokens, at: Rect, name: &'static str, color: [f32; 4], pointer: (f32, f32)) -> Rect {
    if at.has(pointer.0, pointer.1) {
        d.round_rect(at.x, at.y, at.w, at.h, t.size.radius, t.color.hover);
    }
    let size = at.h * 0.6;
    d.icon(at.x + (at.w - size) * 0.5, at.y + (at.h - size) * 0.5, size, name, color);
    at
}

/// a small labelled meter of how much of something is in use (0..1); the width it took
pub fn meter(d: &mut DrawList, t: &Tokens, x: f32, y: f32, name: &str, value: f32) -> f32 {
    let (bar, thick) = (t.size.rem(2.9), t.size.rem(0.43));
    let lw = label(d, t, x, y + (t.size.small - t.size.label) * 0.6, name, t.color.muted) + t.size.rem(0.36);
    let v = value.clamp(0.0, 1.0);
    let by = y + t.size.small * 0.35;
    d.round_rect(x + lw, by, bar, thick, thick * 0.5, t.color.within);
    if v > 0.01 {
        d.round_rect(x + lw, by, (bar * v).max(thick), thick, thick * 0.5, if v > 0.85 { t.color.danger } else { t.color.muted });
    }
    lw + bar
}

/// a row that shows a value: label left, value in its place; moves `y` on
pub fn row(d: &mut DrawList, t: &Tokens, area: Rect, y: &mut f32, label: &str, value: &str) {
    let font = t.size.font;
    let at = *y + (t.size.tree_row - font * 1.25) * 0.5;
    d.sans(area.x, at, font, label, t.color.muted);
    d.text(area.x + (area.w * 0.42).min(96.0), at, font, value, t.color.text);
    *y += t.size.tree_row;
}

/// a vertical level, filled from the bottom to `position` (0..1) on a gradient: green to `ok`, through amber to `hot`, red above
pub fn level(d: &mut DrawList, t: &Tokens, r: Rect, position: f32, ok: f32, hot: f32) {
    const SLICES: usize = 32;
    let mix = |a: [f32; 4], b: [f32; 4], k: f32| [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k, 1.0];
    d.round_rect(r.x, r.y, r.w, r.h, 2.0, t.color.surface_head);
    let top = position.clamp(0.0, 1.0);
    let slice = r.h / SLICES as f32;
    for i in 0..SLICES {
        let at = i as f32 / SLICES as f32;
        if at >= top {
            break;
        }
        let color = if at < ok {
            t.color.go
        } else if at < hot {
            mix(t.color.go, t.color.warn, (at - ok) / (hot - ok).max(1e-3))
        } else {
            mix(t.color.warn, t.color.stop, ((at - hot) / (1.0 - hot).max(1e-3)).min(1.0))
        };
        let h = slice.min((top - at) * r.h);
        // a little over the next slice: their edges would show between them
        d.rect(r.x, r.y + r.h - (at * r.h) - h, r.w, h + 0.75, color);
    }
}

/// a fader as a triangle beside its meter, at `fraction` (0..1)
pub fn fader(d: &mut DrawList, t: &Tokens, r: Rect, fraction: f32, hot: bool) {
    let size = t.size.rem(0.93);
    let y = r.y + r.h * (1.0 - fraction.clamp(0.0, 1.0));
    d.icon(r.x - 1.0, y - size * 0.5, size, "left-fill", if hot { t.color.text } else { t.color.muted });
}

/// a small button that is on or off; its place
pub fn toggle(d: &mut DrawList, t: &Tokens, r: Rect, label: &str, on: bool) -> Rect {
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, if on { t.color.danger } else { t.color.control });
    let w = d.sans_width(t.size.small, label);
    d.sans(r.x + (r.w - w) * 0.5, r.y + (r.h - t.size.small * 1.25) * 0.5, t.size.small, label, if on { t.color.bright } else { t.color.muted });
    r
}

/// a switch: a round track, its knob right and the track green when on; its place
pub fn switch(d: &mut DrawList, t: &Tokens, r: Rect, on: bool) -> Rect {
    d.round_rect(r.x, r.y, r.w, r.h, r.h * 0.5, if on { t.color.go } else { t.color.within });
    d.outline(r.x, r.y, r.w, r.h, (r.h * 0.5, t.size.edge), t.color.line);
    let k = r.h * 0.38;
    let cx = if on { r.x + r.w - r.h * 0.5 } else { r.x + r.h * 0.5 };
    d.disc(cx, r.y + r.h * 0.5, k, if on { t.color.bright } else { t.color.muted });
    r
}

/// a section's head: its label; the height it took
pub fn section(d: &mut DrawList, t: &Tokens, x: f32, y: f32, w: f32, label: &str, first: bool) -> f32 {
    if !first {
        d.rect(x, y, w, t.size.edge, t.color.line);
    }
    self::label(d, t, x, y + t.size.pad, label, t.color.muted);
    t.size.row
}

/// a labelled row whose field is a number you drag: filled to `fraction` (0..1); the field's place
pub fn scrub(d: &mut DrawList, t: &Tokens, at: Rect, label: &str, value: &str, fraction: f32, hot: bool) -> Rect {
    let Rect { x, y, w, .. } = at;
    let (font, row) = (t.size.font, t.size.row);
    let label_w = (w * 0.42).min(96.0);
    let field = Rect { x: x + label_w, y: y + 1.0, w: w - label_w, h: row - 3.0 };
    d.sans(x, y + (row - font * 1.25) * 0.5, font, label, t.color.muted);
    d.round_rect(field.x, field.y, field.w, field.h, t.size.radius, t.color.control);
    d.outline(field.x, field.y, field.w, field.h, (t.size.radius, t.size.edge), t.color.line);
    let filled = field.w * fraction.clamp(0.0, 1.0);
    if filled >= 1.0 {
        d.round_rect(field.x, field.y, filled.max(2.0 * t.size.radius), field.h, t.size.radius, if hot { t.color.selected } else { t.color.within });
    }
    d.text(field.x + 8.0, y + (row - font * 1.25) * 0.5, font, value, t.color.text);
    field
}

/// the way down a tree: every step a pill, the last one on; their places, whatever the depth
pub fn breadcrumb(d: &mut DrawList, t: &Tokens, x: f32, y: f32, path: &[String]) -> Vec<Rect> {
    breadcrumb_rounded(d, t, (x, y, t.size.row - 4.0), path, t.size.radius)
}

/// the same `h` high, the last step rounded by `radius` (see `pills_rounded`)
pub fn breadcrumb_rounded(d: &mut DrawList, t: &Tokens, (x, y, h): (f32, f32, f32), path: &[String], radius: f32) -> Vec<Rect> {
    let px = t.size.font;
    let mut cx = x;
    let mut out = Vec::with_capacity(path.len());
    for (i, step) in path.iter().enumerate() {
        if i > 0 {
            d.icon(cx + 1.0, y + (h - 12.0) * 0.5, 12.0, "next", t.color.muted);
            cx += 14.0;
        }
        let w = d.sans_width(px, step) + 2.0 * t.size.pad;
        let last = i + 1 == path.len();
        if last {
            d.round_rect(cx, y, w, h, radius, t.color.pressed);
        }
        d.sans(cx + t.size.pad, y + (h - px * 1.25) * 0.5, px, step, if last { t.color.pressed_text } else { t.color.muted });
        out.push(Rect { x: cx, y, w, h });
        cx += w;
    }
    out
}

/// one row of a tree
#[derive(Debug, Clone, PartialEq)]
pub struct TreeRow {
    pub depth: usize,
    pub label: String,
    pub detail: String,
    pub icon: &'static str,
    /// has children: are they shown
    pub open: Option<bool>,
    pub selected: bool,
}

/// an outliner; the rows' places
pub fn tree(d: &mut DrawList, t: &Tokens, x: f32, y: f32, w: f32, rows: &[TreeRow], pointer: (f32, f32)) -> Vec<Rect> {
    let (h, px) = (t.size.tree_row, t.size.font);
    let mut out = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let r = Rect { x, y: y + i as f32 * h, w, h };
        if row.selected {
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.pressed);
        } else if r.has(pointer.0, pointer.1) {
            d.rect(r.x, r.y, r.w, r.h, t.color.hover);
        } else if i % 2 == 1 {
            d.rect(r.x, r.y, r.w, r.h, t.color.surface_head);
        }
        let (ink, quiet) = if row.selected { (t.color.pressed_text, t.color.pressed_text) } else { (t.color.text, t.color.muted) };
        let cx = x + 4.0 + row.depth as f32 * 14.0;
        if let Some(open) = row.open {
            d.icon(cx, r.y + (h - 12.0) * 0.5, 12.0, if open { "down" } else { "next" }, quiet);
        }
        d.icon(cx + 14.0, r.y + (h - 13.0) * 0.5, 13.0, row.icon, quiet);
        let text_y = r.y + (h - px * 1.25) * 0.5;
        let dw = label_width(d, t, &row.detail);
        let room = x + w - dw - 16.0 - (cx + 32.0);
        let shown = fit_sans(d, px, &row.label, room);
        d.sans(cx + 32.0, text_y, px, &shown, ink);
        label(d, t, x + w - dw - 8.0, r.y + (h - t.size.label * 1.25) * 0.5, &row.detail, quiet);
        out.push(r);
    }
    out
}

/// a region's edge you can drag: a hairline; hovered or held, a stronger line and a grip
pub fn edge(d: &mut DrawList, t: &Tokens, x: f32, y: f32, h: f32, hot: bool) {
    d.rect(x, y, t.size.edge, h, t.color.line);
    if hot {
        d.rect(x - 0.5, y, 2.0, h, t.color.muted);
        d.round_rect(x - 1.5, y + h * 0.5 - 12.0, 4.0, 24.0, 2.0, t.color.text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> DrawList {
        DrawList::new(0.6)
    }

    #[test]
    fn a_rect_knows_what_is_inside() {
        let r = Rect { x: 1.0, y: 1.0, w: 2.0, h: 2.0 };
        assert!(r.has(1.0, 2.9));
        assert!(!r.has(3.0, 2.0));
    }

    #[test]
    fn a_breadcrumb_has_a_cell_for_every_step_at_any_depth() {
        let t = Tokens::default();
        let path: Vec<String> = (0..9).map(|i| format!("level{i}")).collect();
        let cells = breadcrumb(&mut list(), &t, 0.0, 0.0, &path);
        assert_eq!(cells.len(), 9);
        assert!(cells.windows(2).all(|c| c[1].x >= c[0].x + c[0].w), "they do not overlap");
    }

    #[test]
    fn a_tree_row_is_where_it_says() {
        let t = Tokens::default();
        let row = |depth| TreeRow { depth, label: "x".into(), detail: String::new(), icon: "nodes", open: None, selected: false };
        let cells = tree(&mut list(), &t, 0.0, 10.0, 200.0, &[row(0), row(1), row(9)], (0.0, 0.0));
        assert_eq!(cells[2].y, 10.0 + 2.0 * t.size.tree_row);
    }

    #[test]
    fn chips_wrap_at_the_width_and_stay_a_tree_row_high() {
        let t = Tokens::default();
        let names = ["guest", "member", "creator", "admin", "live", "approved deck"];
        let (cells, used) = chips(&mut list(), &t, (10.0, 0.0, 160.0), &names, &[true, false]);
        assert_eq!(cells.len(), names.len());
        assert!(cells.iter().all(|c| c.x >= 10.0 && c.x + c.w <= 170.0 && c.h <= t.size.tree_row), "{cells:?}");
        assert!(cells.windows(2).all(|c| c[1].y > c[0].y || c[1].x >= c[0].x + c[0].w), "they do not overlap");
        assert!(used > t.size.tree_row, "more than one line");
        assert_eq!(chips(&mut list(), &t, (0.0, 0.0, 100.0), &[], &[]).1, 0.0);
    }

    #[test]
    fn a_menu_line_is_a_tree_row() {
        let t = Tokens::default();
        let cells = menu(&mut list(), &t, (5.0, 100.0), 120.0, &["tenor", "no role", "lock"], &[true], (0.0, 0.0));
        assert_eq!(cells.len(), 3);
        assert!((cells[2].y - cells[1].y - t.size.tree_row).abs() < 1e-3);
        assert!(cells[0].has(20.0, 110.0));
    }

    #[test]
    fn an_action_that_cannot_be_done_takes_no_hover() {
        let t = Tokens::default();
        let mut d = list();
        let lines = [Action { label: "Open", enabled: true, external: false }, Action { label: "Account", enabled: false, external: true }];
        let cells = actions(&mut d, &t, (0.0, 0.0), 200.0, &lines, (20.0, 20.0 + t.size.row));
        assert_eq!(cells.len(), 2);
        assert!(cells[1].has(20.0, 20.0 + t.size.row));
        let hover = d.prims().iter().filter(|p| matches!(p, crate::base::draw::Prim::Rect { color, .. } if *color == t.color.hover)).count();
        assert_eq!(hover, 0, "the pointer is on the muted line");
    }

    #[test]
    fn a_check_list_puts_its_lines_under_the_title_and_dots_the_ones_on() {
        let t = Tokens::default();
        let mut d = list();
        let cells = checks(&mut d, &t, (0.0, 0.0), 200.0, "PANELS", &[("Info", false), ("Mixer", true)], (0.0, 0.0));
        assert_eq!(cells.len(), 2);
        assert!(cells[0].y >= t.size.row, "under the title");
        assert!((cells[1].y - cells[0].y - t.size.row).abs() < 1e-3);
        let dots = d.prims().iter().filter(|p| matches!(p, crate::base::draw::Prim::Rect { color, .. } if *color == t.color.accent)).count();
        assert_eq!(dots, 1, "one is on");
    }

    #[test]
    fn a_long_text_is_cut_with_an_ellipsis() {
        assert_eq!(fit("tokens.toml", 7), "tokens…");
        assert_eq!(fit("short", 7), "short");
        assert_eq!(fit("äöü", 3), "äöü");
        assert_eq!(fit("anything", 0), "");
    }

    #[test]
    fn a_chip_stays_inside() {
        let t = Tokens::default();
        let mut d = list();
        chip(&mut d, &t, (990.0, 0.0), 1000.0, "a long sentence", false);
        let crate::base::draw::Prim::Rect { x, w, .. } = d.prims()[0] else { panic!("a strip") };
        assert!(x + w <= 1000.0);
    }
}
