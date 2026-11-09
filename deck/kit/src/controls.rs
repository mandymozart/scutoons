//! The controls (#456; owner 2026-11-09: the editor's component library). A control draws itself into the list and
//! answers with where it landed; what a press means stays with the caller, the state with its one owner.
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

/// a sentence on a filled strip, kept inside `max_x`
pub fn chip(d: &mut DrawList, t: &Tokens, at: (f32, f32), max_x: f32, text: &str, bad: bool) {
    let px = t.size.font * 0.85;
    let w = d.advance(px) * text.chars().count() as f32 + 16.0;
    let x = at.0.min(max_x - w - 8.0).max(8.0);
    let h = px * 1.25 + 8.0;
    d.rect(x, at.1, w, h, if bad { t.color.refused } else { t.color.panel });
    d.text(x + 8.0, at.1 + 4.0, px, text, t.color.text);
}

/// a row of joined cells, one of them on; the cells' places
pub fn switch(d: &mut DrawList, t: &Tokens, x: f32, y: f32, h: f32, names: &[&str], on: usize) -> Vec<Rect> {
    let px = t.size.font * 0.8;
    let mut cx = x;
    let mut out = Vec::with_capacity(names.len());
    for (i, name) in names.iter().enumerate() {
        let w = d.advance(px) * name.chars().count() as f32 + t.size.pad;
        d.rect(cx, y, w, h, if i == on { t.color.cable } else { t.color.tab });
        d.text(cx + t.size.pad * 0.5, y + (h - px * 1.25) * 0.5, px, name, if i == on { t.color.panel } else { t.color.muted });
        out.push(Rect { x: cx, y, w, h });
        cx += w;
    }
    out
}

/// tabs, one of them active; their places
pub fn tabs(d: &mut DrawList, t: &Tokens, x: f32, y: f32, names: &[&str], active: usize) -> Vec<Rect> {
    let (font, pad, row) = (t.size.font, t.size.pad, t.size.row);
    let mut tx = x;
    let mut out = Vec::with_capacity(names.len());
    for (i, name) in names.iter().enumerate() {
        let w = d.advance(font) * name.chars().count() as f32 + pad;
        let on = i == active;
        d.rect(tx, y, w, row - 4.0, if on { t.color.tab_active } else { t.color.tab });
        if on {
            d.rect(tx, y + row - 6.0, w, 2.0, t.color.accent);
        }
        d.text(tx + pad / 2.0, y + 4.0, font, name, if on { t.color.text } else { t.color.muted });
        out.push(Rect { x: tx, y, w, h: row });
        tx += w + 4.0;
    }
    out
}

/// a number you drag: its label left, a bar filled to `fraction` (0..1) on the right half; the bar's place
pub fn scrub(d: &mut DrawList, t: &Tokens, x: f32, y: f32, w: f32, label: &str, value: &str, fraction: f32, hot: bool) -> Rect {
    let (font, row) = (t.size.font * 0.9, t.size.row);
    let bar = Rect { x: x + w * 0.5, y, w: w * 0.5, h: row };
    d.text(x, y + 5.0, font, label, t.color.text);
    d.rect(bar.x, y + 2.0, bar.w, row - 6.0, t.color.field);
    d.rect(bar.x, y + 2.0, bar.w * fraction, row - 6.0, [t.color.accent[0], t.color.accent[1], t.color.accent[2], 0.35]);
    d.text(bar.x + 6.0, y + 5.0, font, value, if hot { t.color.accent } else { t.color.text });
    bar
}

/// the way down a tree: every step a cell, the last one on; the cells' places, whatever the depth
pub fn breadcrumb(d: &mut DrawList, t: &Tokens, x: f32, y: f32, path: &[String]) -> Vec<Rect> {
    let (font, row, pad) = (t.size.font * 0.9, t.size.row, t.size.pad);
    let mut cx = x;
    let mut out = Vec::with_capacity(path.len());
    for (i, step) in path.iter().enumerate() {
        if i > 0 {
            cx += d.text(cx + 3.0, y + 6.0, font, ">", t.color.muted) + 6.0;
        }
        let w = d.advance(font) * step.chars().count() as f32 + pad;
        let last = i + 1 == path.len();
        d.rect(cx, y, w, row - 4.0, if last { t.color.text } else { t.color.tab });
        d.text(cx + pad * 0.5, y + 5.0, font, step, if last { t.color.panel } else { t.color.text });
        out.push(Rect { x: cx, y, w, h: row });
        cx += w;
    }
    out
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
        let mut d = list();
        let path: Vec<String> = (0..9).map(|i| format!("level{i}")).collect();
        let cells = breadcrumb(&mut d, &t, 0.0, 0.0, &path);
        assert_eq!(cells.len(), 9);
        assert!(cells.windows(2).all(|c| c[1].x >= c[0].x + c[0].w), "they do not overlap");
    }

    #[test]
    fn the_cells_of_a_switch_join() {
        let t = Tokens::default();
        let cells = switch(&mut list(), &t, 10.0, 0.0, 20.0, &["PLAY", "WIRE"], 1);
        assert_eq!(cells[1].x, cells[0].x + cells[0].w);
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
