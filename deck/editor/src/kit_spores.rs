//! The kit page's spores (kit_page.rs): the Spores view's ground of dots, a group, a spore's card, a plug, a cable – each
//! drawn by deck-spores' own painters in its states.
use crate::kit_page::{head, Grid, STATES};
use deck_kit::base::canvas::View;
use deck_kit::base::draw::DrawList;
use deck_kit::base::graph::{Activity, Direction, Link, Node, Pin, Placed};
use deck_kit::base::signal::Signal;
use deck_kit::base::tokens::{alpha, Tokens};
use deck_kit::base::wire::Wire;
use deck_kit::editor::controls::Rect;
use deck_spores::paint::{self, Cam, Frame, Reveal};
use deck_spores::Seen;

/// what the painters read: one spore selected, hovered or flashing
#[derive(Default)]
struct Still {
    selected: bool,
    hovered: bool,
    flash: Option<f32>,
}

impl Seen for Still {
    fn selected(&self, _: &str) -> bool {
        self.selected
    }
    fn hovered(&self, _: &str) -> bool {
        self.hovered
    }
    fn flashing(&self, _: &str) -> Option<f32> {
        self.flash
    }
    fn previews(&self) -> bool {
        false
    }
    fn picture(&self, _: &str) -> Option<u64> {
        None
    }
    fn secs(&self) -> f32 {
        0.0
    }
    fn blob(&self) -> Option<(f32, f32)> {
        None
    }
}

fn spore(far: bool, collapsed: bool) -> Node {
    Node {
        id: "kit".into(),
        title: "image".into(),
        kind: "scene",
        detail: String::new(),
        ins: vec![Pin::new("prompt", Signal::Asset)],
        outs: vec![Pin::new("picture", Signal::Picture)],
        far,
        collapsed,
        picture: None,
        group: None,
    }
}

/// the spores' rows under the page's grid `g`, between `pad` and the page's width
pub fn rows(d: &mut DrawList, t: &Tokens, g: &mut Grid, (pad, w): (f32, f32)) {
    let unit = t.size.rem(1.0);
    let cam = |x: f32, y: f32| Cam { at: (x, y), view: View::default(), unit, dir: Direction::Lr };
    let cell = g.col - t.size.rem(1.0);
    let row = |d: &mut DrawList, g: &mut Grid, (name, note): (&str, &str), h: f32, f: &mut dyn FnMut(&mut DrawList, usize, Rect)| {
        d.rect(pad, g.y, w - 2.0 * pad, t.size.edge, t.color.line);
        head(d, t, pad, g.y, h, name, note);
        for i in 0..STATES.len() {
            f(d, i, Rect { x: g.at(i), y: g.y + t.size.rem(0.4), w: cell, h: h - t.size.rem(0.8) });
        }
        g.y += h;
    };
    let ground_h = t.size.rem(4.4);
    row(d, g, ("Spores view", "dots everywhere · the reveal"), ground_h, &mut |d, i, r| {
        if i > 1 {
            return;
        }
        d.clip(Some(r));
        paint::field(d, t, &cam(r.x, r.y), r);
        if i == 1 {
            let under = Rect { x: r.x + r.w * 0.3, y: r.y + r.h * 0.3, w: r.w * 0.4, h: r.h * 0.4 };
            let f = Frame { k: 0, id: "kit".into(), label: String::new(), colour: t.color.text, rect: under };
            let shown = Reveal { key: "group:kit".into(), colour: t.color.text, since: -10.0, left: None };
            paint::reveal(d, &shown, (&Still::default(), &cam(r.x, r.y), r), (&[f], &[], &Default::default(), &[]));
        }
        d.clip(None);
    });
    row(d, g, ("Group", "its six · hover · folded"), ground_h, &mut |d, i, r| {
        let c = cam(r.x, r.y);
        match i {
            0..=2 => {
                d.clip(Some(r));
                paint::field(d, t, &c, r);
                let f = Frame { k: 0, id: "kit".into(), label: "from the shot".into(), colour: t.color.group(if i == 2 { 2 } else { 0 }), rect: Rect { x: r.x + 4.0, y: r.y + 4.0, w: r.w - 8.0, h: r.h - 8.0 } };
                paint::ground(d, t, &f, &c, r, i == 1);
                paint::label(d, t, &f, &c);
                d.clip(None);
            }
            5 => {
                let n = Node { title: "from the shot".into(), kind: "group", ..spore(false, true) };
                let p = Placed { rect: Rect { x: 0.0, y: 0.0, w: r.w.min(t.size.rem(11.0)), h: t.size.rem(2.0) }, ins: vec![(0.0, unit)], outs: vec![(r.w.min(t.size.rem(11.0)), unit)] };
                let tint = deck_kit::editor::colour::rgb(&deck_kit::base::tokens::to_hex(t.color.group(0)));
                deck_spores::card::card(d, t, (&n, &p, tint), &c, &Still::default());
            }
            _ => {}
        }
    });
    let card_h = t.size.rem(5.2);
    row(d, g, ("Spore", "selected: the text colour"), card_h, &mut |d, i, r| {
        let (n, v) = match i {
            0 => (spore(false, false), Still::default()),
            1 => (spore(false, false), Still { hovered: true, ..Still::default() }),
            2 => (spore(false, false), Still { selected: true, ..Still::default() }),
            3 => (spore(false, false), Still { flash: Some(0.35), ..Still::default() }),
            4 => (spore(false, false), Still::default()),
            5 => (spore(true, false), Still::default()),
            _ => (spore(false, true), Still::default()),
        };
        let size = (r.w.min(t.size.rem(11.0)), if n.collapsed { t.size.rem(2.0) } else { r.h - unit });
        let p = Placed { rect: Rect { x: unit * 0.5, y: 0.0, w: size.0, h: size.1 }, ins: vec![(unit * 0.5, size.1 * 0.6)], outs: vec![(unit * 0.5 + size.0, size.1 * 0.6)] };
        let from = d.prims().len();
        deck_spores::card::card(d, t, (&n, &p, None), &cam(r.x, r.y), &v);
        if i == 4 {
            d.fade_from(from, t.kit("tile", "disabled").opacity);
        }
    });
    row(d, g, ("Plug", "wired · hover · off · may"), t.size.rem(2.6), &mut |d, i, r| {
        let (rad, mid) = (t.size.rem(0.32), (r.x + unit, r.y + r.h * 0.5));
        let mut x = mid.0;
        for s in [Signal::Event, Signal::Control, Signal::Audio, Signal::Address, Signal::Far] {
            let c = t.color.signals.of(s);
            match i {
                1 => d.disc(x, mid.1, rad * 2.2, alpha(c, 0.18)),
                5 => {
                    d.disc(x, mid.1, rad * 2.2, alpha(c, 0.18));
                    d.ring(x, mid.1, rad * 2.2, c);
                }
                0 | 4 => {}
                _ => continue,
            }
            d.pin((x, mid.1), rad, (s.shape(), (1.0, 0.0)), if i == 4 { alpha(c, 0.35) } else { c }, t.color.surface);
            x += unit * 1.6;
        }
    });
    row(d, g, ("Cable", "by signal · hover · lit · pulse"), t.size.rem(3.0), &mut |d, i, r| {
        let activity = match i {
            2 => Activity::Stream,
            3 => Activity::Pulse(0.5),
            0 | 1 => Activity::Idle,
            _ => return,
        };
        let link = Link { activity, ..Link::new((0, 0), (1, 0), Signal::Picture) };
        let wire = Wire::hang((r.x + unit, r.y + 4.0), (r.x + r.w - unit, r.y + 4.0), unit);
        paint::wire(d, t, (&link, &wire), (&cam(r.x, r.y), true, Some(0.3)), i == 1);
        for end in [(r.x + unit, r.y + 4.0), (r.x + r.w - unit, r.y + 4.0)] {
            d.pin(end, t.size.rem(0.32), (Signal::Picture.shape(), (1.0, 0.0)), t.color.signals.picture, t.color.surface);
        }
    });
}
