//! The controls (#456; the look: the OSCT editor's, on the kit's tokens). A control draws itself into the list and answers
//! with where it landed; what a press means stays with the caller, the state with its one owner.
use crate::draw::DrawList;
use crate::tokens::Tokens;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn has(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

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

/// a sentence on a filled strip, kept inside `max_x`
pub fn chip(d: &mut DrawList, t: &Tokens, at: (f32, f32), max_x: f32, text: &str, bad: bool) {
    let px = t.size.font;
    let w = d.advance(px) * text.chars().count() as f32 + 16.0;
    let x = at.0.min(max_x - w - 8.0).max(8.0);
    let h = px * 1.25 + 10.0;
    d.round_rect(x, at.1, w, h, t.size.radius, if bad { t.color.danger } else { t.color.surface_head });
    d.text(x + 8.0, at.1 + 5.0, px, text, t.color.text);
}

/// pills side by side, one of them on; their places
pub fn pills(d: &mut DrawList, t: &Tokens, x: f32, y: f32, h: f32, names: &[&str], on: usize) -> Vec<Rect> {
    let px = t.size.small;
    let mut cx = x;
    let mut out = Vec::with_capacity(names.len());
    for (i, name) in names.iter().enumerate() {
        let w = d.advance(px) * name.chars().count() as f32 + 2.0 * t.size.pad;
        if i == on {
            d.round_rect(cx, y, w, h, t.size.radius, t.color.pressed);
        }
        d.text(cx + t.size.pad, y + (h - px * 1.25) * 0.5, px, name, if i == on { t.color.text } else { t.color.muted });
        out.push(Rect { x: cx, y, w, h });
        cx += w + 2.0;
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
pub fn meter(d: &mut DrawList, t: &Tokens, x: f32, y: f32, label: &str, value: f32) -> f32 {
    const BAR: f32 = 40.0;
    let px = t.size.small;
    let lw = d.text(x, y, px, label, t.color.muted) + 5.0;
    let v = value.clamp(0.0, 1.0);
    let by = y + px * 0.35;
    d.round_rect(x + lw, by, BAR, 6.0, 3.0, t.color.control);
    if v > 0.01 {
        d.round_rect(x + lw, by, (BAR * v).max(6.0), 6.0, 3.0, if v > 0.85 { t.color.danger } else { t.color.muted });
    }
    lw + BAR
}

/// a row that shows a value: label left, value in its place; moves `y` on
pub fn row(d: &mut DrawList, t: &Tokens, area: Rect, y: &mut f32, label: &str, value: &str) {
    let font = t.size.font;
    let at = *y + (t.size.tree_row - font * 1.25) * 0.5;
    d.text(area.x, at, font, label, t.color.muted);
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
    const SIZE: f32 = 13.0;
    let y = r.y + r.h * (1.0 - fraction.clamp(0.0, 1.0));
    d.icon(r.x - 1.0, y - SIZE * 0.5, SIZE, "left-fill", if hot { t.color.text } else { t.color.muted });
}

/// a small button that is on or off; its place
pub fn toggle(d: &mut DrawList, t: &Tokens, r: Rect, label: &str, on: bool) -> Rect {
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, if on { t.color.danger } else { t.color.control });
    let w = d.advance(t.size.small) * label.chars().count() as f32;
    d.text(r.x + (r.w - w) * 0.5, r.y + (r.h - t.size.small * 1.25) * 0.5, t.size.small, label, if on { t.color.text } else { t.color.muted });
    r
}

/// a section's head: its label; the height it took
pub fn section(d: &mut DrawList, t: &Tokens, x: f32, y: f32, w: f32, label: &str, first: bool) -> f32 {
    if !first {
        d.rect(x, y, w, 1.0, t.color.line);
    }
    d.text(x, y + 8.0, t.size.small, label, t.color.muted);
    t.size.row
}

/// a labelled row whose field is a number you drag: filled to `fraction` (0..1); the field's place
pub fn scrub(d: &mut DrawList, t: &Tokens, at: Rect, label: &str, value: &str, fraction: f32, hot: bool) -> Rect {
    let Rect { x, y, w, .. } = at;
    let (font, row) = (t.size.font, t.size.row);
    let label_w = (w * 0.42).min(96.0);
    let field = Rect { x: x + label_w, y: y + 1.0, w: w - label_w, h: row - 3.0 };
    d.text(x, y + (row - font * 1.25) * 0.5, font, label, t.color.muted);
    d.round_rect(field.x, field.y, field.w, field.h, t.size.radius, t.color.control);
    let filled = field.w * fraction.clamp(0.0, 1.0);
    if filled >= 1.0 {
        d.round_rect(field.x, field.y, filled.max(2.0 * t.size.radius), field.h, t.size.radius, if hot { t.color.selected } else { t.color.pressed });
    }
    d.text(field.x + 8.0, y + (row - font * 1.25) * 0.5, font, value, t.color.text);
    field
}

/// the way down a tree: every step a pill, the last one on; their places, whatever the depth
pub fn breadcrumb(d: &mut DrawList, t: &Tokens, x: f32, y: f32, path: &[String]) -> Vec<Rect> {
    let (px, h) = (t.size.font, t.size.row - 4.0);
    let mut cx = x;
    let mut out = Vec::with_capacity(path.len());
    for (i, step) in path.iter().enumerate() {
        if i > 0 {
            d.icon(cx + 1.0, y + (h - 12.0) * 0.5, 12.0, "next", t.color.muted);
            cx += 14.0;
        }
        let w = d.advance(px) * step.chars().count() as f32 + 2.0 * t.size.pad;
        let last = i + 1 == path.len();
        d.round_rect(cx, y, w, h, t.size.radius, if last { t.color.pressed } else { t.color.surface_head });
        d.text(cx + t.size.pad, y + (h - px * 1.25) * 0.5, px, step, if last { t.color.text } else { t.color.muted });
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
            d.rect(r.x, r.y, r.w, r.h, t.color.selected);
            d.rect(r.x, r.y, 2.0, r.h, t.color.text);
        } else if r.has(pointer.0, pointer.1) {
            d.rect(r.x, r.y, r.w, r.h, t.color.hover);
        } else if i % 2 == 1 {
            d.rect(r.x, r.y, r.w, r.h, t.color.surface_head);
        }
        let cx = x + 4.0 + row.depth as f32 * 14.0;
        if let Some(open) = row.open {
            d.icon(cx, r.y + (h - 12.0) * 0.5, 12.0, if open { "down" } else { "next" }, t.color.muted);
        }
        d.icon(cx + 14.0, r.y + (h - 13.0) * 0.5, 13.0, row.icon, t.color.muted);
        let text_y = r.y + (h - px * 1.25) * 0.5;
        d.text(cx + 32.0, text_y, px, &row.label, t.color.text);
        let dw = d.advance(t.size.small) * row.detail.chars().count() as f32;
        d.text(x + w - dw - 8.0, text_y + 1.0, t.size.small, &row.detail, t.color.muted);
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
        let crate::draw::Prim::Rect { x, w, .. } = d.prims()[0] else { panic!("a strip") };
        assert!(x + w <= 1000.0);
    }
}
