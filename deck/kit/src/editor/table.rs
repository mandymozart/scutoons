//! A table of cells under sticky heads (a pure layout model): row heads on the left, column heads on top, both staying
//! put while the body scrolls both ways under them. Each column has its own width and each row its own height; their
//! edges are places to grab (`edge_at`, `edge_place`) – the caller resizes. The panel around it grows to the whole table
//! and, once it cannot, the body scrolls; a strip at the foot then counts what is out of view (`+ 9 rows ↓`,
//! `+ 2 columns →`).
use crate::base::draw::Rect;

/// the table's measures, in pixels of this frame
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// the row heads' width, the column heads' height
    pub head: (f32, f32),
    /// each column's width, each row's height
    pub widths: Vec<f32>,
    pub heights: Vec<f32>,
    /// the counters' strip, taken from the body only while something is out of view
    pub foot: f32,
}

/// how far the body is scrolled
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Scroll {
    pub x: f32,
    pub y: f32,
}

/// An edge of the table to resize: the row heads' right edge, a column's right edge, a row's bottom edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Heads,
    Column(usize),
    Row(usize),
}

/// a table laid into its visible place
#[derive(Debug, Clone, PartialEq)]
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
    xs: Vec<f32>,
    ys: Vec<f32>,
}

fn intersect(a: &Rect, b: &Rect) -> Option<Rect> {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    let (x1, y1) = ((a.x + a.w).min(b.x + b.w), (a.y + a.h).min(b.y + b.h));
    (x1 > x0 && y1 > y0).then_some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

/// where each of `sizes` starts, and the end of the last
fn starts(sizes: &[f32]) -> Vec<f32> {
    let mut at = 0.0;
    let mut out = Vec::with_capacity(sizes.len() + 1);
    out.push(0.0);
    for s in sizes {
        at += s;
        out.push(at);
    }
    out
}

/// the items of `starts` any part of which lies in `[from, to)`
fn visible(starts: &[f32], from: f32, to: f32) -> std::ops::Range<usize> {
    let n = starts.len() - 1;
    let first = (0..n).find(|i| starts[i + 1] > from + 0.01).unwrap_or(n);
    let last = (first..n).find(|i| starts[*i] >= to - 0.01).unwrap_or(n);
    first..last
}

impl Table {
    /// `rows` × `cols` cells all of one size
    pub fn uniform(head: (f32, f32), cell: (f32, f32), rows: usize, cols: usize, foot: f32) -> Table {
        Table { head, widths: vec![cell.0; cols], heights: vec![cell.1; rows], foot }
    }

    pub fn rows(&self) -> usize {
        self.heights.len()
    }

    pub fn cols(&self) -> usize {
        self.widths.len()
    }

    /// the cells' whole size
    fn whole(&self) -> (f32, f32) {
        (self.widths.iter().sum(), self.heights.iter().sum())
    }

    /// the whole table at once: what its panel asks for
    pub fn size(&self) -> (f32, f32) {
        let (w, h) = self.whole();
        (self.head.0 + w, self.head.1 + h)
    }

    /// laid into `area`, scrolled by `scroll` (clamped)
    pub fn lay(&self, area: Rect, scroll: Scroll) -> Laid {
        let whole = self.whole();
        let (bw, mut bh) = ((area.w - self.head.0).max(0.0), (area.h - self.head.1).max(0.0));
        let out = whole.0 > bw + 0.5 || whole.1 > bh + 0.5;
        if out {
            bh = (bh - self.foot).max(0.0);
        }
        let over = ((whole.0 - bw).max(0.0), (whole.1 - bh).max(0.0));
        let scroll = Scroll { x: scroll.x.clamp(0.0, over.0), y: scroll.y.clamp(0.0, over.1) };
        let body = Rect { x: area.x + self.head.0, y: area.y + self.head.1, w: bw, h: bh };
        let foot = out.then_some(Rect { x: area.x, y: body.y + bh, w: area.w, h: self.foot });
        let (heads_top, heads_left) = (Rect { x: body.x, y: area.y, w: bw, h: self.head.1 }, Rect { x: area.x, y: body.y, w: self.head.0, h: bh });
        Laid { table: self.clone(), area, body, heads_top, heads_left, foot, scroll, over, xs: starts(&self.widths), ys: starts(&self.heights) }
    }
}

impl Laid {
    fn col_x(&self, c: usize) -> f32 {
        self.body.x + self.xs[c.min(self.xs.len() - 1)] - self.scroll.x
    }

    fn row_y(&self, r: usize) -> f32 {
        self.body.y + self.ys[r.min(self.ys.len() - 1)] - self.scroll.y
    }

    fn width(&self, c: usize) -> f32 {
        self.table.widths.get(c).copied().unwrap_or_default()
    }

    fn height(&self, r: usize) -> f32 {
        self.table.heights.get(r).copied().unwrap_or_default()
    }

    /// a cell where it stands (it may lie outside the body: clip with `shown`)
    pub fn cell(&self, r: usize, c: usize) -> Rect {
        Rect { x: self.col_x(c), y: self.row_y(r), w: self.width(c), h: self.height(r) }
    }

    /// `n` cells of row `r` from column `c` on, as one
    pub fn span(&self, r: usize, c: usize, n: usize) -> Rect {
        let end = (c + n.max(1)).min(self.table.cols());
        Rect { w: self.col_x(end) - self.col_x(c), ..self.cell(r, c) }
    }

    /// `n` rows from row `r` on of column `c`, as one
    pub fn down(&self, r: usize, c: usize, n: usize) -> Rect {
        let end = (r + n.max(1)).min(self.table.rows());
        Rect { h: self.row_y(end) - self.row_y(r), ..self.cell(r, c) }
    }

    /// a row's head: always at the left edge, with its row
    pub fn row_head(&self, r: usize) -> Rect {
        Rect { x: self.area.x, y: self.row_y(r), w: self.table.head.0, h: self.height(r) }
    }

    /// a column's head (`n` columns from `c`, for a group's): always at the top, with its column
    pub fn col_head(&self, c: usize, n: usize) -> Rect {
        let end = (c + n.max(1)).min(self.table.cols());
        Rect { x: self.col_x(c), y: self.area.y, w: self.col_x(end) - self.col_x(c), h: self.table.head.1 }
    }

    /// what shows of `r` inside `region` (a place to press is cut to it)
    pub fn shown(r: &Rect, region: &Rect) -> Option<Rect> {
        intersect(r, region)
    }

    /// the rows any part of which shows
    pub fn visible_rows(&self) -> std::ops::Range<usize> {
        visible(&self.ys, self.scroll.y, self.scroll.y + self.body.h)
    }

    pub fn visible_cols(&self) -> std::ops::Range<usize> {
        visible(&self.xs, self.scroll.x, self.scroll.x + self.body.w)
    }

    /// how many rows lie wholly below the body, how many columns wholly right of it
    pub fn out_of_view(&self) -> (usize, usize) {
        (self.table.rows() - self.visible_rows().end, self.table.cols() - self.visible_cols().end)
    }

    /// the foot's counters: `+ 9 rows ↓`, `+ 2 columns →` (none: all of it shows)
    pub fn counters(&self) -> (Option<String>, Option<String>) {
        let (rows, cols) = self.out_of_view();
        let plural = |n: usize, one: &str, many: &str| if n == 1 { format!("+ 1 {one}") } else { format!("+ {n} {many}") };
        ((rows > 0).then(|| format!("{} ↓", plural(rows, "row", "rows"))), (cols > 0).then(|| format!("{} →", plural(cols, "column", "columns"))))
    }

    /// the wheel over the table (`steps` across, down): the scroll it comes to, none when it did not move; a step is half
    /// the first column in view across, the first row in view down
    pub fn wheel(&self, (across, down): (f32, f32)) -> Option<Scroll> {
        let w = self.width(self.visible_cols().start.min(self.table.cols().saturating_sub(1)));
        let h = self.height(self.visible_rows().start.min(self.table.rows().saturating_sub(1)));
        let s = Scroll { x: (self.scroll.x - across * w * 0.5).clamp(0.0, self.over.0), y: (self.scroll.y - down * h).clamp(0.0, self.over.1) };
        (s != self.scroll).then_some(s)
    }

    /// The line of `edge` where it stands, along the whole table: (from, to).
    pub fn edge_line(&self, edge: Edge) -> ((f32, f32), (f32, f32)) {
        let bottom = self.body.y + self.body.h;
        match edge {
            Edge::Heads => ((self.body.x, self.area.y), (self.body.x, bottom)),
            Edge::Column(c) => {
                let x = self.col_x(c + 1);
                ((x, self.area.y), (x, bottom))
            }
            Edge::Row(r) => {
                let y = self.row_y(r + 1);
                ((self.area.x, y), ((self.body.x + self.body.w).min(self.col_x(self.table.cols())), y))
            }
        }
    }

    /// A press band along `edge`, `grip` either side, cut to where it shows (none: scrolled away): the place a caller
    /// registers for a press, and where the pointer shows the edge as a handle.
    pub fn edge_place(&self, edge: Edge, grip: f32) -> Option<Rect> {
        let (a, b) = self.edge_line(edge);
        let bottom = self.body.y + self.body.h;
        let (r, within) = if a.0 == b.0 {
            // a column's edge stands only over the body, the heads' only at the body's left
            let x_ok = match edge {
                Edge::Heads => true,
                _ => a.0 > self.body.x + 0.5 && a.0 <= self.body.x + self.body.w + 0.5,
            };
            if !x_ok {
                return None;
            }
            (Rect { x: a.0 - grip, y: a.1, w: 2.0 * grip, h: b.1 - a.1 }, Rect { x: self.area.x, y: self.area.y, w: self.area.w, h: bottom - self.area.y })
        } else {
            if a.1 <= self.body.y + 0.5 || a.1 > bottom + 0.5 {
                return None;
            }
            (Rect { x: a.0, y: a.1 - grip, w: b.0 - a.0, h: 2.0 * grip }, Rect { x: self.area.x, y: self.body.y, w: self.area.w, h: self.body.h + grip })
        };
        intersect(&r, &within)
    }

    /// The edge whose place holds `(x, y)`, the nearest when two do.
    pub fn edge_at(&self, x: f32, y: f32, grip: f32) -> Option<Edge> {
        let mut all = vec![Edge::Heads];
        all.extend(self.visible_cols().map(Edge::Column));
        all.extend(self.visible_rows().map(Edge::Row));
        let dist = |e: Edge| {
            let (a, _) = self.edge_line(e);
            if matches!(e, Edge::Row(_)) {
                (y - a.1).abs()
            } else {
                (x - a.0).abs()
            }
        };
        all.into_iter().filter(|e| self.edge_place(*e, grip).is_some_and(|r| r.has(x, y))).min_by(|a, b| dist(*a).total_cmp(&dist(*b)))
    }

    /// the size `edge` stands at now
    pub fn edge_size(&self, edge: Edge) -> f32 {
        match edge {
            Edge::Heads => self.table.head.0,
            Edge::Column(c) => self.width(c),
            Edge::Row(r) => self.height(r),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: usize, cols: usize) -> Table {
        Table::uniform((200.0, 60.0), (160.0, 80.0), rows, cols, 20.0)
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

    #[test]
    fn columns_and_rows_of_their_own_size_lay_out_and_their_edges_are_places_to_grab() {
        let t = Table { head: (100.0, 40.0), widths: vec![50.0, 200.0, 80.0], heights: vec![30.0, 90.0], foot: 20.0 };
        assert_eq!(t.size(), (430.0, 160.0));
        let l = t.lay(Rect { x: 0.0, y: 0.0, w: 430.0, h: 160.0 }, Scroll::default());
        assert_eq!(l.cell(1, 2), Rect { x: 350.0, y: 70.0, w: 80.0, h: 90.0 });
        assert_eq!(l.span(0, 0, 2).w, 250.0);
        assert_eq!(l.down(0, 1, 2).h, 120.0);
        assert_eq!(l.row_head(1).h, 90.0);
        assert_eq!(l.col_head(1, 1), Rect { x: 150.0, y: 0.0, w: 200.0, h: 40.0 });
        // a column's right edge, from its head down; a row's bottom edge across its head and cells; the heads' edge
        assert_eq!(l.edge_at(351.0, 10.0, 3.0), Some(Edge::Column(1)), "on the column's head");
        assert_eq!(l.edge_at(349.0, 120.0, 3.0), Some(Edge::Column(1)), "beside its cells");
        assert_eq!(l.edge_at(20.0, 71.0, 3.0), Some(Edge::Row(0)), "on the row's head");
        assert_eq!(l.edge_at(300.0, 69.0, 3.0), Some(Edge::Row(0)), "under its cells");
        assert_eq!(l.edge_at(101.0, 20.0, 3.0), Some(Edge::Heads));
        assert_eq!(l.edge_at(250.0, 120.0, 3.0), None, "inside a cell");
        assert_eq!(l.edge_at(152.0, 69.5, 3.0), Some(Edge::Row(0)), "where two meet, the nearer");
        assert_eq!((l.edge_size(Edge::Column(1)), l.edge_size(Edge::Row(1)), l.edge_size(Edge::Heads)), (200.0, 90.0, 100.0));
        assert_eq!(l.edge_line(Edge::Column(0)), ((150.0, 0.0), (150.0, 160.0)), "the handle lights the whole line");
        // scrolled: an edge under the heads has no place
        let small = t.lay(Rect { x: 0.0, y: 0.0, w: 300.0, h: 160.0 }, Scroll { x: 60.0, y: 0.0 });
        assert_eq!(small.edge_place(Edge::Column(0), 3.0), None, "column 0's edge lies under the row heads");
        assert!(small.edge_place(Edge::Column(1), 3.0).is_some());
    }
}
