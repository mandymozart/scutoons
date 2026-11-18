//! The window's own frame where the toolbar is the title bar (Windows: the system frame is off): the three window
//! buttons bare at the row's right end, and where a press lands – an edge or corner resizes, a window button,
//! the toolbar's empty run drags, anything else is the app's. No window system here: the editor maps the answers
//! onto its window. macOS keeps the system's traffic lights at the row's left (`Side::Left`, not drawn by us).
use crate::base::draw::{DrawList, Rect};
use crate::base::tokens::Tokens;

/// the resize band along every edge, in logical pixels
pub const EDGE: f32 = 8.0;
/// a corner reaches this far along both of its edges
pub const CORNER: f32 = 2.0 * EDGE;
/// two presses this close in time (seconds) and place (logical pixels) are a double click
pub const DOUBLE: (f64, f32) = (0.5, 4.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Minimise,
    /// maximise, or restore when maximised
    Maximise,
    Close,
}

/// an edge or a corner of the window
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

/// where the window buttons sit in the toolbar row: Windows at its right end; macOS's traffic lights at its left
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// what a press at a point does
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Resize(Edge),
    Button(Button),
    /// the toolbar's empty run: drag the window, a double click maximises or restores
    Drag,
    /// a control of the toolbar, or anything under it
    App,
}

/// the frame as the last drawn toolbar left it
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    /// the window's size, logical pixels
    pub size: (f32, f32),
    /// maximised: no resize band
    pub maximised: bool,
    /// the toolbar's band, top to its bottom, across the window
    pub band: f32,
    pub buttons: Vec<(Rect, Button)>,
    /// every control and pill in the band that is not the empty run
    pub solid: Vec<Rect>,
}

impl Frame {
    /// what a press at (x, y) does: an edge before a button before a control before the empty run
    pub fn hit(&self, x: f32, y: f32) -> Zone {
        if let Some(e) = self.edge(x, y) {
            return Zone::Resize(e);
        }
        if let Some((_, b)) = self.buttons.iter().find(|(r, _)| r.has(x, y)) {
            return Zone::Button(*b);
        }
        if y < 0.0 || y >= self.band || self.solid.iter().any(|r| r.has(x, y)) {
            return Zone::App;
        }
        Zone::Drag
    }

    fn edge(&self, x: f32, y: f32) -> Option<Edge> {
        let (w, h) = self.size;
        if self.maximised || w <= 0.0 || h <= 0.0 {
            return None;
        }
        let near = |d: f32, reach: f32| d < reach;
        let (left, right, top, bottom) = (x, w - x, y, h - y);
        let corner = |a: f32, b: f32| (near(a, EDGE) && near(b, CORNER)) || (near(b, EDGE) && near(a, CORNER));
        Some(if corner(top, left) {
            Edge::NorthWest
        } else if corner(top, right) {
            Edge::NorthEast
        } else if corner(bottom, left) {
            Edge::SouthWest
        } else if corner(bottom, right) {
            Edge::SouthEast
        } else if near(top, EDGE) {
            Edge::North
        } else if near(bottom, EDGE) {
            Edge::South
        } else if near(left, EDGE) {
            Edge::West
        } else if near(right, EDGE) {
            Edge::East
        } else {
            return None;
        })
    }
}

/// the last press on the empty run, for the double click
#[derive(Debug, Clone, Copy, Default)]
pub struct Clicks {
    last: Option<(f64, (f32, f32))>,
}

impl Clicks {
    /// a press at `at`, `now` seconds: true when it doubles the one before (which it then consumes)
    pub fn press(&mut self, now: f64, at: (f32, f32)) -> bool {
        let double = self.last.is_some_and(|(t, p)| now - t <= DOUBLE.0 && (p.0 - at.0).abs() <= DOUBLE.1 && (p.1 - at.1).abs() <= DOUBLE.1);
        self.last = if double { None } else { Some((now, at)) };
        double
    }
}

/// a button's side: round, as high as the toolbar's pills
fn side(t: &Tokens) -> f32 {
    t.size.top
}

/// the room the three buttons take in the row
pub fn width(t: &Tokens) -> f32 {
    3.0 * side(t)
}

/// the three buttons bare in the row at `x` (their left), `y` its top: a glyph each, a quiet ground on hover
pub fn draw(d: &mut DrawList, t: &Tokens, x: f32, y: f32, maximised: bool, pointer: (f32, f32)) -> Vec<(Rect, Button)> {
    let s = side(t);
    let glyph = t.size.rem(0.86);
    [(Button::Minimise, "window-min"), (Button::Maximise, if maximised { "window-restore" } else { "window-max" }), (Button::Close, "close")]
        .into_iter()
        .enumerate()
        .map(|(i, (b, icon))| {
            let r = Rect { x: x + i as f32 * s, y, w: s, h: s };
            if r.has(pointer.0, pointer.1) {
                d.round_rect(r.x, r.y, r.w, r.h, s * 0.5, t.color.hover);
            }
            d.icon(r.x + (s - glyph) * 0.5, r.y + (s - glyph) * 0.5, glyph, icon, t.color.text);
            (r, b)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a 1280 × 720 window, its toolbar 50 high: a menu control at the left, a pill, the buttons at the right
    fn frame(maximised: bool) -> Frame {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let buttons = draw(&mut d, &t, 1280.0 - 8.0 - width(&t), 8.0, maximised, (0.0, 0.0));
        Frame { size: (1280.0, 720.0), maximised, band: 50.0, buttons, solid: vec![Rect { x: 16.0, y: 8.0, w: 36.0, h: 36.0 }, Rect { x: 400.0, y: 8.0, w: 200.0, h: 36.0 }] }
    }

    #[test]
    fn the_empty_run_drags_and_a_control_never_does() {
        let f = frame(false);
        assert_eq!(f.hit(300.0, 20.0), Zone::Drag, "a gap between pills");
        assert_eq!(f.hit(20.0, 20.0), Zone::App, "the menu");
        assert_eq!(f.hit(500.0, 20.0), Zone::App, "a pill");
        assert_eq!(f.hit(300.0, 60.0), Zone::App, "under the toolbar");
    }

    #[test]
    fn the_buttons_sit_at_the_right_end_in_order_and_hit_before_the_run() {
        let f = frame(false);
        let order: Vec<Button> = f.buttons.iter().map(|(_, b)| *b).collect();
        assert_eq!(order, [Button::Minimise, Button::Maximise, Button::Close]);
        assert!(f.buttons.windows(2).all(|w| w[0].0.x + w[0].0.w <= w[1].0.x + 0.01));
        let close = f.buttons[2].0;
        assert!((close.x + close.w - (1280.0 - 8.0)).abs() < 0.01, "close ends the row");
        for (r, b) in &f.buttons {
            assert_eq!(f.hit(r.x + r.w * 0.5, r.y + r.h * 0.5), Zone::Button(*b));
        }
    }

    #[test]
    fn the_edges_and_corners_resize_before_anything_else() {
        let f = frame(false);
        assert_eq!(f.hit(300.0, 2.0), Zone::Resize(Edge::North), "the top band over the run");
        assert_eq!(f.hit(640.0, 715.0), Zone::Resize(Edge::South));
        assert_eq!(f.hit(3.0, 300.0), Zone::Resize(Edge::West));
        assert_eq!(f.hit(1277.0, 300.0), Zone::Resize(Edge::East));
        assert_eq!(f.hit(2.0, 12.0), Zone::Resize(Edge::NorthWest), "a corner reaches along its edges");
        assert_eq!(f.hit(1278.0, 3.0), Zone::Resize(Edge::NorthEast));
        assert_eq!(f.hit(12.0, 718.0), Zone::Resize(Edge::SouthWest));
        assert_eq!(f.hit(1279.0, 710.0), Zone::Resize(Edge::SouthEast));
        assert_eq!(f.hit(300.0, EDGE), Zone::Drag, "the band is 8 wide, no more");
    }

    #[test]
    fn maximised_there_is_no_band_and_the_glyph_says_restore() {
        let f = frame(true);
        assert_eq!(f.hit(300.0, 2.0), Zone::Drag);
        assert_eq!(f.hit(1.0, 300.0), Zone::App);
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        draw(&mut d, &t, 0.0, 0.0, true, (0.0, 0.0));
        let icons: Vec<&str> = d.prims().iter().filter_map(|p| if let crate::base::draw::Prim::Icon { name, .. } = *p { Some(name) } else { None }).collect();
        assert_eq!(icons, ["window-min", "window-restore", "close"]);
    }

    #[test]
    fn a_hovered_button_gets_the_quiet_ground_and_close_is_not_red() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let s = width(&t) / 3.0;
        draw(&mut d, &t, 0.0, 0.0, false, (2.5 * s, 0.5 * s));
        use crate::base::draw::Prim;
        let grounds: Vec<[f32; 4]> = d.prims().iter().filter_map(|p| if let Prim::Rect { color, .. } = *p { Some(color) } else { None }).collect();
        assert_eq!(grounds, [t.color.hover], "one ground, under the pointer");
        assert!(d.prims().iter().all(|p| !matches!(*p, Prim::Icon { color, .. } if color == t.color.danger || color == t.color.stop)));
    }

    #[test]
    fn two_presses_close_in_time_and_place_are_a_double_click() {
        let mut c = Clicks::default();
        assert!(!c.press(1.0, (100.0, 20.0)));
        assert!(c.press(1.3, (102.0, 21.0)));
        assert!(!c.press(1.4, (102.0, 21.0)), "a third press starts again");
        assert!(!c.press(2.5, (102.0, 21.0)), "too late");
        assert!(!c.press(2.6, (140.0, 21.0)), "too far");
    }
}
