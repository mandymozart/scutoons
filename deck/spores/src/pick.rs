//! Picking: a drag on the empty canvas draws a box that selects the spores it touches (shift adds to the selection), a
//! click on a thread pins its value beside it and a click again lets it go.
use deck_kit::base::draw::DrawList;
use deck_kit::base::graph::Graph;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::Rect;

/// how far the pointer goes before a press on the canvas is a drag, px
pub const DRAG: f32 = 4.0;

/// a box being drawn: where it started and where the pointer is, whether it adds, what was selected before it, the
/// thread under the press
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Boxing {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub add: bool,
    pub before: Vec<String>,
    pub wire: Option<String>,
}

impl Boxing {
    pub fn rect(&self) -> Rect {
        let (x, y) = (self.from.0.min(self.to.0), self.from.1.min(self.to.1));
        Rect { x, y, w: (self.from.0 - self.to.0).abs(), h: (self.from.1 - self.to.1).abs() }
    }

    pub fn dragged(&self) -> bool {
        (self.to.0 - self.from.0).hypot(self.to.1 - self.from.1) >= DRAG
    }
}

/// a thread's key, kept while the graph is laid out again: its ends by spore id and port; None for one without a value
pub fn wire_key(g: &Graph, i: usize) -> Option<String> {
    let l = g.links.get(i).filter(|l| l.value.is_some())?;
    let (a, b) = (g.nodes.get(l.from.0)?, g.nodes.get(l.to.0)?);
    Some(format!("{}.{}>{}.{}", a.id, l.from.1, b.id, l.to.1))
}

pub fn meets(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// the box as it is drawn: a faint fill and a thin edge in the selection's colour
pub fn draw_box(d: &mut DrawList, t: &Tokens, b: &Boxing) {
    let r = b.rect();
    if !b.dragged() {
        return;
    }
    let c = t.color.selection;
    d.rect(r.x, r.y, r.w, r.h, [c[0], c[1], c[2], c[3] * 0.18]);
    d.outline(r.x, r.y, r.w, r.h, (0.0, 1.0), [c[0], c[1], c[2], c[3] * 0.9]);
}
