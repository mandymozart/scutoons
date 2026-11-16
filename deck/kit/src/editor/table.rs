//! A table of equal cells under sticky heads (a pure layout model): row heads on the left, column heads on top, both
//! staying put while the body scrolls both ways under them. The panel around it grows to the whole table and, once it
//! cannot, the body scrolls; a strip at the foot then counts what is out of view (`+ 9 rows ↓`, `+ 2 columns →`).
use crate::base::draw::Rect;

/// the table's measures, in pixels of this frame
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Table {
    /// the row heads' width, the column heads' height
    pub head: (f32, f32),
    pub cell: (f32, f32),
    pub rows: usize,
    pub cols: usize,
    /// the counters' strip, taken from the body only while something is out of view
    pub foot: f32,
}

/// how far the body is scrolled
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Scroll {
    pub x: f32,
    pub y: f32,
}

/// a table laid into its visible place
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Laid {
    pub table: Table,
    pub area: Rect,
    /// where the cells show
    pub body: Rect,
    /// the strip of the column heads right of the corner, the column of the row heads under it
    pub heads_top: Rect,
    pub heads_left: Rect,
    pub foot: Option<Rect>,
    /// clamped
    pub scroll: Scroll,
    /// how far the body can scroll
    pub over: (f32, f32),
}

fn intersect(a: &Rect, b: &Rect) -> Option<Rect> {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    let (x1, y1) = ((a.x + a.w).min(b.x + b.w), (a.y + a.h).min(b.y + b.h));
    (x1 > x0 && y1 > y0).then_some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

impl Table {
    /// the whole table at once: what its panel asks for
    pub fn size(&self) -> (f32, f32) {
        (self.head.0 + self.cols as f32 * self.cell.0, self.head.1 + self.rows as f32 * self.cell.1)
    }

    /// laid into `area`, scrolled by `scroll` (clamped)
    pub fn lay(&self, area: Rect, scroll: Scroll) -> Laid {
        let whole = (self.cols as f32 * self.cell.0, self.rows as f32 * self.cell.1);
        let (bw, mut bh) = ((area.w - self.head.0).max(0.0), (area.h - self.head.1).max(0.0));
        let out = whole.0 > bw + 0.5 || whole.1 > bh + 0.5;
        if out {
            bh = (bh - self.foot).max(0.0);
        }
        let over = ((whole.0 - bw).max(0.0), (whole.1 - bh).max(0.0));
        let scroll = Scroll { x: scroll.x.clamp(0.0, over.0), y: scroll.y.clamp(0.0, over.1) };
        let body = Rect { x: area.x + self.head.0, y: area.y + self.head.1, w: bw, h: bh };
        let foot = out.then_some(Rect { x: area.x, y: body.y + bh, w: area.w, h: self.foot });
        Laid { table: *self, area, body, heads_top: Rect { x: body.x, y: area.y, w: bw, h: self.head.1 }, heads_left: Rect { x: area.x, y: body.y, w: self.head.0, h: bh }, foot, scroll, over }
    }
}

impl Laid {
    fn col_x(&self, c: usize) -> f32 {
        self.body.x + c as f32 * self.table.cell.0 - self.scroll.x
    }

    fn row_y(&self, r: usize) -> f32 {
        self.body.y + r as f32 * self.table.cell.1 - self.scroll.y
    }

    /// a cell where it stands (it may lie outside the body: clip with `shown`)
    pub fn cell(&self, r: usize, c: usize) -> Rect {
        Rect { x: self.col_x(c), y: self.row_y(r), w: self.table.cell.0, h: self.table.cell.1 }
    }

    /// `n` cells of row `r` from column `c` on, as one
    pub fn span(&self, r: usize, c: usize, n: usize) -> Rect {
        Rect { w: self.table.cell.0 * n.max(1) as f32, ..self.cell(r, c) }
    }

    /// a row's head: always at the left edge, with its row
    pub fn row_head(&self, r: usize) -> Rect {
        Rect { x: self.area.x, y: self.row_y(r), w: self.table.head.0, h: self.table.cell.1 }
    }

    /// a column's head (`n` columns from `c`, for a group's): always at the top, with its column
    pub fn col_head(&self, c: usize, n: usize) -> Rect {
        Rect { x: self.col_x(c), y: self.area.y, w: self.table.cell.0 * n.max(1) as f32, h: self.table.head.1 }
    }

    /// what shows of `r` inside `region` (a place to press is cut to it)
    pub fn shown(r: &Rect, region: &Rect) -> Option<Rect> {
        intersect(r, region)
    }

    /// the rows any part of which shows
    pub fn visible_rows(&self) -> std::ops::Range<usize> {
        let h = self.table.cell.1.max(1.0);
        let first = (self.scroll.y / h).floor() as usize;
        let last = ((self.scroll.y + self.body.h) / h).ceil() as usize;
        first.min(self.table.rows)..last.min(self.table.rows)
    }

    pub fn visible_cols(&self) -> std::ops::Range<usize> {
        let w = self.table.cell.0.max(1.0);
        let first = (self.scroll.x / w).floor() as usize;
        let last = ((self.scroll.x + self.body.w) / w).ceil() as usize;
        first.min(self.table.cols)..last.min(self.table.cols)
    }

    /// how many rows lie wholly below the body, how many columns wholly right of it
    pub fn out_of_view(&self) -> (usize, usize) {
        (self.table.rows - self.visible_rows().end, self.table.cols - self.visible_cols().end)
    }

    /// the foot's counters: `+ 9 rows ↓`, `+ 2 columns →` (none: all of it shows)
    pub fn counters(&self) -> (Option<String>, Option<String>) {
        let (rows, cols) = self.out_of_view();
        let plural = |n: usize, one: &str, many: &str| if n == 1 { format!("+ 1 {one}") } else { format!("+ {n} {many}") };
        ((rows > 0).then(|| format!("{} ↓", plural(rows, "row", "rows"))), (cols > 0).then(|| format!("{} →", plural(cols, "column", "columns"))))
    }

    /// the wheel over the table (`steps` across, down): the scroll it comes to, none when it did not move
    pub fn wheel(&self, (across, down): (f32, f32)) -> Option<Scroll> {
        let s = Scroll { x: (self.scroll.x - across * self.table.cell.0 * 0.5).clamp(0.0, self.over.0), y: (self.scroll.y - down * self.table.cell.1).clamp(0.0, self.over.1) };
        (s != self.scroll).then_some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: usize, cols: usize) -> Table {
        Table { head: (200.0, 60.0), cell: (160.0, 80.0), rows, cols, foot: 20.0 }
    }

    #[test]
    fn a_table_that_fits_asks_for_its_whole_size_and_counts_nothing() {
        let t = table(3, 2);
        assert_eq!(t.size(), (520.0, 300.0));
        let l = t.lay(Rect { x: 10.0, y: 20.0, w: 520.0, h: 300.0 }, Scroll { x: 50.0, y: 50.0 });
        assert_eq!((l.over, l.scroll, l.foot), ((0.0, 0.0), Scroll::default(), None), "nothing to scroll");
        assert_eq!(l.counters(), (None, None));
        assert_eq!(l.cell(2, 1), Rect { x: 370.0, y: 240.0, w: 160.0, h: 80.0 });
        assert_eq!(l.wheel((0.0, -1.0)), None);
    }

    #[test]
    fn heads_stay_put_while_the_body_scrolls_both_ways() {
        let t = table(16, 8);
        let area = Rect { x: 0.0, y: 0.0, w: 200.0 + 4.0 * 160.0, h: 60.0 + 4.0 * 80.0 + 20.0 };
        let l = t.lay(area, Scroll::default());
        assert_eq!(l.body.h, 320.0, "the foot is taken from the body");
        assert_eq!(l.counters(), (Some("+ 12 rows ↓".into()), Some("+ 4 columns →".into())));
        let s = l.wheel((-2.0, -3.0)).unwrap();
        assert_eq!(s, Scroll { x: 160.0, y: 240.0 });
        let l = t.lay(area, s);
        // the row head moves with its row, never sideways; the column head with its column, never down
        assert_eq!((l.row_head(3).x, l.row_head(3).y), (0.0, 60.0));
        assert_eq!((l.col_head(1, 1).x, l.col_head(1, 1).y), (200.0, 0.0));
        assert_eq!(l.cell(3, 1), Rect { x: 200.0, y: 60.0, w: 160.0, h: 80.0 }, "the cell under both heads");
        assert_eq!((l.visible_rows(), l.visible_cols()), (3..7, 1..5));
        assert_eq!(l.counters(), (Some("+ 9 rows ↓".into()), Some("+ 3 columns →".into())));
        // a head scrolled away is cut to its strip: no press where the corner is
        assert_eq!(Laid::shown(&l.row_head(2), &l.heads_left), None);
        assert_eq!(Laid::shown(&l.col_head(0, 1), &l.heads_top), None);
        // to the end and no further
        let end = t.lay(area, Scroll { x: 1e6, y: 1e6 });
        assert_eq!(end.scroll, Scroll { x: l.over.0, y: l.over.1 });
        assert_eq!(end.counters(), (None, None));
        assert_eq!(end.wheel((-1.0, -1.0)), None);
    }

    #[test]
    fn a_group_head_spans_its_columns_and_scrolls_with_them() {
        let l = table(2, 6).lay(Rect { x: 0.0, y: 0.0, w: 520.0, h: 400.0 }, Scroll { x: 80.0, y: 0.0 });
        assert_eq!(l.col_head(2, 3), Rect { x: 200.0 + 320.0 - 80.0, y: 0.0, w: 480.0, h: 60.0 });
        assert_eq!(l.span(1, 0, 2).w, 320.0);
        assert_eq!(l.counters(), (None, Some("+ 3 columns →".into())));
    }
}
