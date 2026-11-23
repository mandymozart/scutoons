//! Taking the stage (the creator's `@scutoons/ui/editor` stage.ts, the same rules): a view – a panel – can lie on the
//! stage, the whole window under the floating panels, in one of four layouts: one view · a half and two quarters · two
//! quarters and a half · four quarters; or go full screen. A panel dragged to a screen edge or corner lands there
//! (`zone_at`, `place_on`); the floating panels themselves never dock. A free cell shows what lies under the stage (the
//! open item's own view). Every view is in one of four states: hidden, a panel, on the stage, full screen.
use deck_kit::base::draw::Rect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Layout {
    One,
    HalfLeft,
    HalfRight,
    Quarters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cell {
    Whole,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

pub const LAYOUTS: [Layout; 4] = [Layout::One, Layout::HalfLeft, Layout::HalfRight, Layout::Quarters];
pub const ZONE_CELLS: [Cell; 7] = [Cell::Whole, Cell::Left, Cell::Right, Cell::TopLeft, Cell::TopRight, Cell::BottomLeft, Cell::BottomRight];

impl Layout {
    pub fn cells(self) -> &'static [Cell] {
        match self {
            Layout::One => &[Cell::Whole],
            Layout::HalfLeft => &[Cell::Left, Cell::TopRight, Cell::BottomRight],
            Layout::HalfRight => &[Cell::TopLeft, Cell::BottomLeft, Cell::Right],
            Layout::Quarters => &[Cell::TopLeft, Cell::TopRight, Cell::BottomLeft, Cell::BottomRight],
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Layout::One => "One view",
            Layout::HalfLeft => "Half and two quarters",
            Layout::HalfRight => "Two quarters and a half",
            Layout::Quarters => "Four quarters",
        }
    }
}

impl Cell {
    /// its share of the stage: x, y, w, h in 0..1
    fn unit(self) -> (f32, f32, f32, f32) {
        match self {
            Cell::Whole => (0.0, 0.0, 1.0, 1.0),
            Cell::Left => (0.0, 0.0, 0.5, 1.0),
            Cell::Right => (0.5, 0.0, 0.5, 1.0),
            Cell::TopLeft => (0.0, 0.0, 0.5, 0.5),
            Cell::TopRight => (0.5, 0.0, 0.5, 0.5),
            Cell::BottomLeft => (0.0, 0.5, 0.5, 0.5),
            Cell::BottomRight => (0.5, 0.5, 0.5, 0.5),
        }
    }

    fn area(self) -> f32 {
        let (_, _, w, h) = self.unit();
        w * h
    }

    fn right(self) -> bool {
        matches!(self, Cell::Right | Cell::TopRight | Cell::BottomRight)
    }

    /// whether `self` covers all of `other`
    fn contains(self, other: Cell) -> bool {
        let (a, b) = (self.unit(), other.unit());
        a.0 <= b.0 && a.1 <= b.1 && a.0 + a.2 >= b.0 + b.2 && a.1 + a.3 >= b.1 + b.3
    }

    /// its rectangle in `area`
    pub fn rect(self, area: Rect) -> Rect {
        let (x, y, w, h) = self.unit();
        Rect { x: area.x + x * area.w, y: area.y + y * area.h, w: w * area.w, h: h * area.h }
    }
}

/// The four states of a view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewState {
    Hidden,
    Panel,
    Stage,
    Full,
}

pub const VIEW_STATES: [ViewState; 4] = [ViewState::Hidden, ViewState::Panel, ViewState::Stage, ViewState::Full];

impl ViewState {
    pub fn word(self) -> &'static str {
        match self {
            ViewState::Hidden => "Hidden",
            ViewState::Panel => "Panel",
            ViewState::Stage => "Stage",
            ViewState::Full => "Full screen",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            ViewState::Hidden => "close",
            ViewState::Panel => "layers",
            ViewState::Stage => "grid",
            ViewState::Full => "eye",
        }
    }
}

/// The layout and the view in each of its cells (none: free), in the order of `layout.cells()`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageState {
    pub layout: Layout,
    pub slots: Vec<Option<String>>,
}

impl Default for StageState {
    fn default() -> StageState {
        StageState { layout: Layout::One, slots: vec![None] }
    }
}

impl StageState {
    /// the views on the stage, in cell order
    pub fn views(&self) -> Vec<&str> {
        self.slots.iter().flatten().map(String::as_str).collect()
    }

    pub fn has(&self, view: &str) -> bool {
        self.slots.iter().any(|s| s.as_deref() == Some(view))
    }

    pub fn cell_of(&self, view: &str) -> Option<Cell> {
        let i = self.slots.iter().position(|s| s.as_deref() == Some(view))?;
        self.layout.cells().get(i).copied()
    }

    /// every cell with its view and its rectangle in `area`
    pub fn slot_rects(&self, area: Rect) -> Vec<(Cell, Option<&str>, Rect)> {
        self.layout.cells().iter().enumerate().map(|(i, c)| (*c, self.slots.get(i).and_then(|s| s.as_deref()), c.rect(area))).collect()
    }

    /// a view off the stage: its cell is free; with no view left the stage is empty
    pub fn without(&self, view: &str) -> StageState {
        if !self.has(view) {
            return self.clone();
        }
        let slots: Vec<Option<String>> = self.slots.iter().map(|s| s.clone().filter(|v| v != view)).collect();
        if slots.iter().any(Option::is_some) {
            StageState { layout: self.layout, slots }
        } else {
            StageState::default()
        }
    }

    /// the layout a view dropped on `target` makes of the layout there is
    fn layout_for(&self, target: Cell) -> Layout {
        match target {
            Cell::Whole => Layout::One,
            Cell::Left => Layout::HalfLeft,
            Cell::Right => Layout::HalfRight,
            _ => match self.layout {
                Layout::Quarters => Layout::Quarters,
                Layout::HalfLeft if target.right() => Layout::HalfLeft,
                Layout::HalfRight if !target.right() => Layout::HalfRight,
                Layout::HalfLeft | Layout::HalfRight => Layout::Quarters,
                // one view or none: the half on the other side keeps it
                Layout::One if target.right() => Layout::HalfLeft,
                Layout::One => Layout::HalfRight,
            },
        }
    }

    /// A view dropped on a cell: the layout follows, the view takes that cell and the others keep theirs where they
    /// still exist – else the cell their old one covers, else a free one; a view without a cell left goes back to its
    /// panel.
    pub fn place(&self, view: &str, target: Cell) -> StageState {
        let layout = self.layout_for(target);
        let cells = layout.cells();
        let mut slots: Vec<Option<String>> = cells.iter().map(|c| (*c == target).then(|| view.to_string())).collect();
        let rest: Vec<(String, Cell)> = self.layout.cells().iter().zip(&self.slots).filter_map(|(c, s)| s.clone().filter(|v| v != view).map(|v| (v, *c))).collect();
        let mut left: Vec<bool> = vec![true; rest.len()];
        let mut order: Vec<usize> = (0..cells.len()).collect();
        // the larger cells first: a view that had the whole stage keeps a half rather than a quarter
        order.sort_by(|a, b| cells[*b].area().total_cmp(&cells[*a].area()));
        let mut fill = |fits: &dyn Fn(Cell, Cell) -> bool| {
            for &i in &order {
                if slots[i].is_some() {
                    continue;
                }
                if let Some(k) = (0..rest.len()).find(|&k| left[k] && fits(rest[k].1, cells[i])) {
                    slots[i] = Some(rest[k].0.clone());
                    left[k] = false;
                }
            }
        };
        fill(&|from, cell| from == cell);
        fill(&|from, cell| from.contains(cell));
        if layout != Layout::One {
            fill(&|_, _| true);
        }
        StageState { layout, slots }
    }

    /// A view put on the stage without a place (the Views list): an empty stage, it takes the whole; else the first free
    /// cell; else the layout grows (one → a half and two quarters → four quarters); four full, it takes the last quarter
    pub fn add(&self, view: &str) -> StageState {
        if self.has(view) {
            return self.clone();
        }
        if self.views().is_empty() {
            return StageState { layout: Layout::One, slots: vec![Some(view.to_string())] };
        }
        if let Some(free) = self.slots.iter().position(Option::is_none) {
            let mut s = self.clone();
            s.slots[free] = Some(view.to_string());
            return s;
        }
        let next = match self.layout {
            Layout::One => Cell::TopRight,
            Layout::HalfLeft => Cell::TopLeft,
            Layout::HalfRight => Cell::TopRight,
            Layout::Quarters => Cell::BottomRight,
        };
        self.place(view, next)
    }

    /// a stored stage, checked: its number of slots, each view once (`known`: a view that still exists)
    pub fn checked(self, known: &dyn Fn(&str) -> bool) -> StageState {
        let mut seen: Vec<String> = Vec::new();
        let slots: Vec<Option<String>> = (0..self.layout.cells().len())
            .map(|i| {
                let v = self.slots.get(i).cloned().flatten().filter(|v| !v.is_empty() && known(v) && !seen.contains(v))?;
                seen.push(v.clone());
                Some(v)
            })
            .collect();
        if slots.iter().any(Option::is_some) {
            StageState { layout: self.layout, slots }
        } else {
            StageState::default()
        }
    }
}

/// how far the drop zones reach in from the window's edges: the bands along the edges, the corners along each edge
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneSize {
    pub band: f32,
    pub corner: f32,
}

/// The drop zone under the pointer, over the whole window: a corner (along either edge near it), the top edge (the
/// whole stage), the left or right edge (that half); none elsewhere, nor along the bottom
pub fn zone_at(x: f32, y: f32, (w, h): (f32, f32), z: ZoneSize) -> Option<Cell> {
    let (l, r, t, b) = (x <= z.band, x >= w - z.band, y <= z.band, y >= h - z.band);
    if (t && x <= z.corner) || (l && y <= z.corner) {
        Some(Cell::TopLeft)
    } else if (t && x >= w - z.corner) || (r && y <= z.corner) {
        Some(Cell::TopRight)
    } else if (b && x <= z.corner) || (l && y >= h - z.corner) {
        Some(Cell::BottomLeft)
    } else if (b && x >= w - z.corner) || (r && y >= h - z.corner) {
        Some(Cell::BottomRight)
    } else if t {
        Some(Cell::Whole)
    } else if l {
        Some(Cell::Left)
    } else if r {
        Some(Cell::Right)
    } else {
        None
    }
}

/// what a zone covers on the screen (a corner: its two arms along the edges) – drawn while a panel is dragged
pub fn zone_rects(cell: Cell, (w, h): (f32, f32), z: ZoneSize) -> Vec<Rect> {
    let ZoneSize { band, corner } = z;
    let (mw, mh) = ((w - 2.0 * corner).max(0.0), (h - 2.0 * corner).max(0.0));
    let r = |x: f32, y: f32, w: f32, h: f32| Rect { x, y, w, h };
    match cell {
        Cell::Whole => vec![r(corner, 0.0, mw, band)],
        Cell::Left => vec![r(0.0, corner, band, mh)],
        Cell::Right => vec![r(w - band, corner, band, mh)],
        Cell::TopLeft => vec![r(0.0, 0.0, corner, band), r(0.0, band, band, corner - band)],
        Cell::TopRight => vec![r(w - corner, 0.0, corner, band), r(w - band, band, band, corner - band)],
        Cell::BottomLeft => vec![r(0.0, h - band, corner, band), r(0.0, h - corner, band, corner - band)],
        Cell::BottomRight => vec![r(w - corner, h - band, corner, band), r(w - band, h - corner, band, corner - band)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(layout: Layout, slots: &[Option<&str>]) -> StageState {
        StageState { layout, slots: slots.iter().map(|v| v.map(String::from)).collect() }
    }

    #[test]
    fn a_view_dropped_on_an_edge_takes_that_half_and_the_one_there_keeps_the_other() {
        let one = StageState::default().add("library");
        assert_eq!(one, s(Layout::One, &[Some("library")]));
        let two = one.place("mixer", Cell::Left);
        assert_eq!(two.layout, Layout::HalfLeft);
        assert_eq!(two.cell_of("mixer"), Some(Cell::Left));
        assert!(two.has("library"), "the one there keeps a cell: {two:?}");
        let corner = StageState::default().add("a").place("b", Cell::TopRight);
        assert_eq!(corner.layout, Layout::HalfLeft, "a quarter on the right: the half on the left keeps the view there");
        assert_eq!((corner.cell_of("a"), corner.cell_of("b")), (Some(Cell::Left), Some(Cell::TopRight)));
    }

    #[test]
    fn the_stage_grows_from_one_to_four_quarters_and_the_top_edge_takes_it_whole() {
        let mut st = StageState::default();
        for v in ["a", "b", "c", "d"] {
            st = st.add(v);
        }
        assert_eq!(st.layout, Layout::Quarters);
        assert_eq!(st.views().len(), 4);
        let five = st.add("e");
        assert_eq!(five.cell_of("e"), Some(Cell::BottomRight), "four full: the last quarter");
        let whole = five.place("x", Cell::Whole);
        assert_eq!(whole, s(Layout::One, &[Some("x")]), "the whole stage: the others go back to their panels");
    }

    #[test]
    fn off_the_stage_frees_the_cell_and_the_last_one_empties_it() {
        let st = StageState::default().add("a").add("b");
        let off = st.without("a");
        assert!(!off.has("a") && off.has("b") && off.layout == st.layout);
        assert_eq!(off.without("b"), StageState::default());
    }

    #[test]
    fn a_stored_stage_is_checked() {
        let json = r#"{"layout":"quarters","slots":["a","a","gone",null,"extra"]}"#;
        let st: StageState = serde_json::from_str(json).unwrap();
        let st = st.checked(&|v| v != "gone");
        assert_eq!(st, s(Layout::Quarters, &[Some("a"), None, None, None]));
        assert_eq!(s(Layout::One, &[Some("gone")]).checked(&|v| v != "gone"), StageState::default());
    }

    #[test]
    fn the_zones_follow_the_edges_and_corners() {
        let z = ZoneSize { band: 24.0, corner: 120.0 };
        let size = (1600.0, 900.0);
        assert_eq!(zone_at(800.0, 5.0, size, z), Some(Cell::Whole));
        assert_eq!(zone_at(5.0, 450.0, size, z), Some(Cell::Left));
        assert_eq!(zone_at(1595.0, 450.0, size, z), Some(Cell::Right));
        assert_eq!(zone_at(5.0, 5.0, size, z), Some(Cell::TopLeft));
        assert_eq!(zone_at(1500.0, 895.0, size, z), Some(Cell::BottomRight));
        assert_eq!(zone_at(800.0, 895.0, size, z), None, "not along the bottom");
        assert_eq!(zone_at(800.0, 450.0, size, z), None);
        for c in ZONE_CELLS {
            for r in zone_rects(c, size, z) {
                assert_eq!(zone_at(r.x + r.w * 0.5, r.y + r.h * 0.5, size, z), Some(c), "{c:?}");
            }
        }
    }

    #[test]
    fn the_cells_cover_the_stage() {
        let area = Rect { x: 10.0, y: 50.0, w: 1000.0, h: 600.0 };
        for l in LAYOUTS {
            let total: f32 = l.cells().iter().map(|c| c.rect(area)).map(|r| r.w * r.h).sum();
            assert!((total - area.w * area.h).abs() < 1.0, "{l:?}");
        }
    }
}
