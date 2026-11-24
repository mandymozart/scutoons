//! The kit page (`--kit`): every component of deck-kit drawn by its own code in each state of `kit/tokens.toml` – the
//! deck's side of the kit's reference (agents/research/redesign/lap2/kit-d.html, the creator's `/kit`). One row per
//! component, one column per state: rest · hover · on · focus · disabled · dragged · more. The spores' parts below
//! (kit_spores.rs). Nothing here is pressed.
use deck_kit::base::draw::DrawList;
use deck_kit::base::signal::Signal;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect, TreeRow};
use deck_kit::editor::floats::{self, Place, Spec, View};
use deck_kit::editor::fold;

/// the states' columns
pub const STATES: [&str; 7] = ["rest", "hover", "on", "focus", "disabled", "dragged", "more"];

/// where no pointer is
const AWAY: (f32, f32) = (-1e4, -1e4);

/// the page's grid: the label's column, the states' columns, a row's top
pub struct Grid {
    pub x0: f32,
    pub col: f32,
    pub y: f32,
}

impl Grid {
    pub fn at(&self, i: usize) -> f32 {
        self.x0 + i as f32 * self.col
    }
}

/// what was drawn since `from` faded as the kit's disabled state of `name` says
fn disabled(d: &mut DrawList, t: &Tokens, from: usize, name: &str) {
    d.fade_from(from, t.kit(name, "disabled").opacity);
}

/// a row's head: the component's name, and a word on it
pub fn head(d: &mut DrawList, t: &Tokens, x: f32, y: f32, h: f32, name: &str, note: &str) {
    d.sans(x, y + h * 0.5 - t.size.font * 0.9, t.size.font, name, t.color.text);
    d.sans(x, y + h * 0.5 + t.size.font * 0.15, t.size.small, note, t.color.muted);
}

/// The page over the whole window `(w, h)`: the stage's ground, the title, the states' heads and every row.
pub fn draw(d: &mut DrawList, t: &Tokens, (w, h): (f32, f32)) {
    d.rect(0.0, 0.0, w, h, t.color.surface);
    let (pad, ch) = (t.size.rem(1.4), t.size.row);
    d.sans(pad, pad, t.size.title, "Kit · palette D", t.color.text);
    d.sans(pad + t.size.rem(9.5), pad + t.size.rem(0.2), t.size.small, "kit/tokens.toml – the deck draws every row with its own components", t.color.muted);
    let x0 = pad + t.size.rem(10.0);
    let mut g = Grid { x0, col: (w - x0 - pad) / STATES.len() as f32, y: pad + t.size.rem(2.6) };
    for (i, s) in STATES.iter().enumerate() {
        controls::label(d, t, g.at(i), g.y, s, t.color.muted);
    }
    g.y += t.size.rem(1.4);
    let rh = t.size.rem(2.75);
    let row = |d: &mut DrawList, g: &mut Grid, name: &str, note: &str, height: f32, cell: &mut dyn FnMut(&mut DrawList, usize, f32, f32)| {
        d.rect(pad, g.y, w - 2.0 * pad, t.size.edge, t.color.line);
        head(d, t, pad, g.y, height, name, note);
        for i in 0..STATES.len() {
            cell(d, i, g.at(i), g.y + (height - ch) * 0.5);
        }
        g.y += height;
    };
    let tab = |i: usize| ["rest", "hover", "on", "on", "disabled", "dragged", ""][i];
    row(d, &mut g, "Pill", "a group, a toolbar", rh, &mut |d, i, x, y| {
        if i == 0 {
            pill(d, t, x, y);
        }
    });
    row(d, &mut g, "PillTab", "item tabs, panel tabs", rh, &mut |d, i, x, y| {
        if i < 6 {
            let from = d.prims().len();
            let r = controls::pill_tab(d, t, (x, y, ch), Some("image"), "nave", tab(i));
            focus(d, t, i, r, ch * 0.5);
            if i == 4 {
                disabled(d, t, from, "tab");
            }
        }
    });
    row(d, &mut g, "Workspace row", "the active brighter", rh, &mut |d, i, x, y| {
        if i >= 6 {
            return;
        }
        let states: [&str; 3] = match i {
            0 | 3 | 4 => ["on", "rest", "rest"],
            1 => ["on", "hover", "rest"],
            2 => ["rest", "on", "rest"],
            _ => ["on", "dragged", "rest"],
        };
        let from = d.prims().len();
        let inset = t.size.rem(0.29);
        let words = ["Build", "Show", "Map"];
        let width: f32 = words.iter().map(|w| d.sans_width(t.size.font, w) + 2.0 * t.size.pad).sum::<f32>() + 2.0 * inset;
        let outer = Rect { x, y: y - inset, w: width, h: ch + 2.0 * inset };
        ground(d, t, "pill", outer, outer.h * 0.5);
        let mut cx = x + inset;
        for (k, (word, st)) in words.iter().zip(states).enumerate() {
            let r = controls::pill_tab(d, t, (cx, y, ch), None, word, st);
            if i == 3 && k == 1 {
                controls::focus_ring(d, t, r, ch * 0.5);
            }
            cx += r.w;
        }
        if i == 4 {
            disabled(d, t, from, "tab");
        }
    });
    row(d, &mut g, "IconButton", "round · alone · a dot", rh, &mut |d, i, x, y| {
        let r = Rect { x, y, w: ch, h: ch };
        match i {
            0 | 1 | 2 | 4 => {
                controls::icon_button_in(d, t, r, "home", ["rest", "hover", "on", "", "disabled"][i]);
            }
            3 => {
                controls::icon_button_in(d, t, r, "home", "rest");
                controls::focus_ring(d, t, r, ch * 0.5);
            }
            6 => {
                controls::icon_button_in(d, t, r, "menu", "alone");
                controls::icon_button_in(d, t, Rect { x: x + ch * 1.5, ..r }, "image", "dot");
            }
            _ => {}
        }
    });
    let states5 = |i: usize| ["rest", "hover", "on", "rest", "disabled"].get(i).copied();
    row(d, &mut g, "Button primary", "the pressed fill", rh, &mut |d, i, x, y| {
        if let Some(s) = states5(i) {
            let r = controls::primary_in(d, t, Rect { x, y, w: d.sans_width(t.size.font, "Publish") + 3.0 * t.size.pad, h: ch }, "Publish", false, s);
            focus(d, t, i, r, t.size.radius);
        }
    });
    for (kind, name, note, word) in [("button_plain", "Button plain", "framed", "Cancel"), ("button_quiet", "Button quiet", "words only", "Back")] {
        row(d, &mut g, name, note, rh, &mut |d, i, x, y| {
            if let Some(s) = states5(i) {
                let r = controls::button(d, t, (x, y, ch), word, kind, s);
                focus(d, t, i, r, t.size.radius);
            } else if i == 6 && kind == "button_quiet" {
                controls::button(d, t, (x, y, ch), "Delete", kind, "destroy");
            }
        });
    }
    row(d, &mut g, "Segments", "on = the pressed fill", rh, &mut |d, i, x, y| {
        if i < 5 {
            let from = d.prims().len();
            let names = ["2 legs", "wings"];
            let r = Rect { x, y, w: controls::segments_width(d, t, &names), h: ch };
            // hover: the pointer on wings
            controls::segments(d, t, (x, y, ch), &names, if i == 2 { 1 } else { 0 }, if i == 1 { (x + r.w - 4.0, y + ch * 0.5) } else { AWAY });
            focus(d, t, i, r, t.size.radius);
            if i == 4 {
                disabled(d, t, from, "segments");
            }
        }
    });
    row(d, &mut g, "Chip", "the badge, the status", rh, &mut |d, i, x, y| {
        let y = y + (ch - t.size.tree_row) * 0.5 + 4.0;
        if let Some(s) = states5(i) {
            let r = controls::chip_in(d, t, (x, y), "Deck", s);
            focus(d, t, i, r, r.h * 0.5);
        } else if i == 6 {
            let mut cx = x;
            for s in ["ok", "warning", "error"] {
                cx += controls::chip_in(d, t, (cx, y), s, s).w + t.size.pad * 0.5;
            }
        }
    });
    row(d, &mut g, "Toggle", "the one switch", rh, &mut |d, i, x, y| {
        let r = Rect { x, y: y + (ch - t.size.rem(1.125)) * 0.5, w: t.size.rem(2.25), h: t.size.rem(1.125) };
        match i {
            0..=3 => {
                controls::switch_in(d, t, r, i == 2, ["off", "hover", "on", "off"][i]);
                focus(d, t, i, r, r.h * 0.5);
            }
            4 => {
                controls::switch_in(d, t, r, false, "disabled");
                controls::switch_in(d, t, Rect { x: x + r.w * 1.5, ..r }, true, "disabled");
            }
            _ => {}
        }
    });
    let fw = t.size.rem(10.0);
    for (kind, name, note, word, chooses) in [("field", "Field", "a value in a form", "crow-fox", false), ("field", "Select", "boxed: a value", "By name", true), ("select_quiet", "Select quiet", "a filter or a sort", "By use", true)] {
        row(d, &mut g, name, note, rh, &mut |d, i, x, y| {
            let r = Rect { x, y, w: fw, h: ch };
            if let Some(s) = states5(i) {
                controls::field(d, t, r, word, (kind, s), chooses);
                focus(d, t, i, r, if kind == "select_quiet" { ch * 0.5 } else { t.size.radius });
            } else if i == 6 && kind == "field" && !chooses {
                controls::field(d, t, r, word, (kind, "error"), false);
            }
        });
    }
    row(d, &mut g, "MenuItem", "the ink ring · a tick", rh, &mut |d, i, x, y| {
        let mw = t.size.rem(9.0);
        let y = y + (ch - t.size.tree_row - 4.0) * 0.5;
        if i < 5 {
            let from = d.prims().len();
            let lines = controls::menu(d, t, (x, y), mw, &["Night"], &[i == 2], if i == 1 { (x + 10.0, y + 6.0) } else { AWAY });
            focus(d, t, i, lines[0], t.size.radius);
            if i == 4 {
                disabled(d, t, from, "tab");
            }
        }
    });
    row(d, &mut g, "ListRow", "selected = the pressed fill", rh, &mut |d, i, x, y| {
        if i >= 6 {
            return;
        }
        let from = d.prims().len();
        let lw = t.size.rem(10.0);
        let y = y + (ch - t.size.tree_row) * 0.5;
        if i == 5 {
            let s = t.kit("list_row", "dragged");
            if let Some(g) = s.ground {
                d.round_rect(x, y, lw, t.size.tree_row, t.size.radius, g);
            }
            if let Some(b) = s.border {
                d.outline(x, y, lw, t.size.tree_row, (t.size.radius, t.size.edge), b);
            }
        }
        let rows = [TreeRow { depth: 0, label: "crow-lizard".into(), detail: String::new(), icon: "image", open: None, selected: i == 2 }];
        let r = controls::tree(d, t, x, y, lw, &rows, if i == 1 { (x + 4.0, y + 4.0) } else { AWAY })[0];
        focus(d, t, i, r, t.size.radius);
        if i == 4 {
            disabled(d, t, from, "list_row");
        }
    });
    row(d, &mut g, "Tile", "picked: the text colour", rh * 1.5, &mut |d, i, x, y| {
        if i >= 5 {
            return;
        }
        let r = Rect { x, y: y - rh * 0.25 + 2.0, w: t.size.rem(5.5), h: rh * 1.5 - 8.0 };
        let from = d.prims().len();
        if i == 1 {
            ground(d, t, "tile", r, t.size.radius * 1.4);
            if let Some(g) = t.kit("tile", "hover").ground {
                d.round_rect(r.x, r.y, r.w, r.h, t.size.radius * 1.4, g);
            }
        }
        let (room, ink) = fold::tile(d, t, r, "fly through", i == 2);
        fold::picture(d, room, room.h.min(room.w) * 0.6, "image", ink);
        focus(d, t, i, r, t.size.radius * 1.4);
        if i == 4 {
            disabled(d, t, from, "tile");
        }
    });
    row(d, &mut g, "Panel head", "no border at rest", rh * 1.7, &mut |d, i, x, y| {
        if i >= 6 || i == 2 {
            return;
        }
        let from = d.prims().len();
        let y = y - rh * 0.35 + 2.0;
        let head_h = t.size.rem(2.0);
        let whole = Rect { x, y, w: t.size.rem(11.0), h: rh * 1.7 - 8.0 };
        let fold = Rect { x: whole.x + whole.w - head_h, y, w: head_h, h: head_h };
        let none = Rect { w: 0.0, h: 0.0, ..whole };
        let v = View { whole, handle: Rect { w: whole.w - head_h, h: head_h, ..whole }, fold, close: none, content: Rect { y: y + head_h, h: whole.h - head_h, ..whole }, scroll: 0.0, over: 0.0, collapsed: false, track: none, thumb: none, sizes: false, staged: false };
        let spec = Spec { id: "kit", name: "Rig", icon: "sliders", about: "", place: Place::default(), essential: true, inline: false };
        floats::begin(d, t, &spec, &v, if i == 1 || i == 5 { (x + t.size.pad, y + head_h * 0.5) } else { AWAY });
        controls::row(d, t, Rect { x: v.content.x + t.size.pad, w: v.content.w - 2.0 * t.size.pad, ..v.content }, &mut (v.content.y + 2.0), "voice", "crow");
        floats::end(d, &v);
        if i == 3 {
            controls::focus_ring(d, t, Rect { x: x + t.size.pad * 0.5, y: y + 2.0, w: t.size.rem(2.6), h: head_h - 4.0 }, (head_h - 4.0) * 0.5);
        }
        if i == 4 {
            disabled(d, t, from, "tab");
        }
    });
    row(d, &mut g, "Section fold", "open: the head's ground", rh * 1.1, &mut |d, i, x, y| {
        if i >= 5 {
            return;
        }
        let r = Rect { x, y: y + (ch - fold::head_height(t)) * 0.5, w: t.size.rem(13.0), h: fold::head_height(t) };
        let from = d.prims().len();
        if i == 1 {
            if let Some(g) = t.kit("section", "hover").ground {
                d.rect(r.x, r.y, r.w, r.h, g);
            }
        }
        fold::head(d, t, r, "sliders", "Lens", "70 mm", i == 2);
        focus(d, t, i, r, 0.0);
        if i == 4 {
            disabled(d, t, from, "section");
        }
    });
    // what keeps its colour (RULES H11): the signals, the outlet types, a group's six, ok · warning · error
    let wide = rh * 0.9;
    row(d, &mut g, "Signals", "ports, pins, cables", wide, &mut |d, i, x, y| {
        if i == 0 {
            swatches(d, t, x, y, Signal::ALL.iter().map(|s| (s.word(), Swatch::Pin(*s))).collect());
        }
    });
    let o = t.color.outlets;
    row(d, &mut g, "Outlet types", "where a thing plays", wide, &mut |d, i, x, y| {
        if i == 0 {
            swatches(d, t, x, y, vec![("page", Swatch::Dot(o.page)), ("stage", Swatch::Dot(o.stage)), ("target", Swatch::Dot(o.target)), ("device", Swatch::Dot(o.device))]);
        }
    });
    row(d, &mut g, "Groups · status", "six · ok · warning · error", wide, &mut |d, i, x, y| {
        if i == 0 {
            let mut list: Vec<(&str, Swatch)> = t.color.groups.iter().map(|c| ("", Swatch::Dot(*c))).collect();
            list.extend([("ok", Swatch::Dot(t.color.go)), ("warning", Swatch::Dot(t.color.warn)), ("error", Swatch::Dot(t.color.stop))]);
            swatches(d, t, x, y, list);
        }
    });
    crate::kit_spores::rows(d, t, &mut g, (pad, w));
}

/// a component's resting ground and line as the kit writes them
pub fn ground(d: &mut DrawList, t: &Tokens, name: &str, r: Rect, radius: f32) {
    let s = t.kit(name, "rest");
    if let Some(g) = s.ground {
        d.round_rect(r.x, r.y, r.w, r.h, radius, g);
    }
    if let Some(b) = s.border {
        d.outline(r.x, r.y, r.w, r.h, (radius, t.size.edge), b);
    }
}

/// the focus column's ring round what was drawn
fn focus(d: &mut DrawList, t: &Tokens, i: usize, r: Rect, radius: f32) {
    if i == 3 {
        controls::focus_ring(d, t, r, radius);
    }
}

/// the bar's pill with two icon buttons
fn pill(d: &mut DrawList, t: &Tokens, x: f32, y: f32) {
    let (h, inset) = (t.size.top, t.size.rem(0.29));
    let ih = h - 2.0 * inset;
    let r = Rect { x, y: y - (h - t.size.row) * 0.5, w: 2.0 * ih + 2.0 * inset + 2.0, h };
    ground(d, t, "pill", r, h * 0.5);
    d.halo(t.color.ring());
    controls::icon_button_in(d, t, Rect { x: r.x + inset, y: r.y + inset, w: ih, h: ih }, "menu", "rest");
    controls::icon_button_in(d, t, Rect { x: r.x + inset + ih + 2.0, y: r.y + inset, w: ih, h: ih }, "home", "hover");
    d.halo(None);
}

/// what a swatch shows
pub enum Swatch {
    Pin(Signal),
    Dot([f32; 4]),
}

/// a line of swatches, each with its word
fn swatches(d: &mut DrawList, t: &Tokens, x: f32, y: f32, list: Vec<(&str, Swatch)>) {
    let (mid, r, px) = (y + t.size.row * 0.5, t.size.rem(0.36), t.size.small);
    let mut cx = x + r;
    for (word, s) in list {
        match s {
            Swatch::Pin(sig) => d.pin((cx, mid), r, (sig.shape(), (1.0, 0.0)), t.color.signals.of(sig), t.color.surface),
            Swatch::Dot(c) => d.disc(cx, mid, r, c),
        }
        cx += r + t.size.pad * 0.6;
        if !word.is_empty() {
            cx += d.sans(cx, mid - px * 0.62, px, word, t.color.muted) + t.size.pad * 1.6;
        } else {
            cx += t.size.pad * 0.4;
        }
        cx += r;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_kit::base::draw::Prim;
    use deck_kit::base::tokens::{Colors, Theme};

    /// every colour the kit file gives a look
    fn palette(c: &Colors) -> Vec<[f32; 4]> {
        let (s, o) = (c.signals, c.outlets);
        let mut all = vec![c.stage, c.surface, c.surface_head, c.line, c.text, c.muted, c.control, c.hover, c.pressed, c.pressed_text, c.selected, c.within, c.bright, c.selection, c.danger, c.go, c.warn, c.stop];
        all.extend([s.event, s.control, s.clock, s.audio, s.picture, s.address, s.vote, s.scene, s.far, s.job, s.asset, o.page, o.stage, o.target, o.device]);
        all.extend(c.groups.iter().chain(&c.strips).chain(&c.axis));
        all
    }

    #[test]
    fn the_kit_page_draws_every_row_in_no_colour_outside_the_kit_file() {
        for theme in [Theme::Night, Theme::Day] {
            let t = Tokens::new(theme, 14.0);
            let mut d = DrawList::new(0.6);
            draw(&mut d, &t, (1600.0, 1300.0));
            let known = palette(&t.color);
            // by its red, green and blue: a token see-through is the token
            let near = |c: [f32; 4]| c[3] == 0.0 || known.iter().any(|k| (0..3).all(|i| (k[i] - c[i]).abs() < 2e-3));
            for p in d.prims() {
                let colours: Vec<[f32; 4]> = match *p {
                    Prim::Rect { color, .. } | Prim::Outline { color, .. } | Prim::Text { color, .. } | Prim::Icon { color, .. } | Prim::Segment { color, .. } | Prim::Poly { color, .. } => vec![color],
                    Prim::Gradient { corners, .. } => corners.to_vec(),
                    _ => vec![],
                };
                for c in colours {
                    assert!(near(c), "{theme:?}: {c:?} in {p:?} is no colour of kit/tokens.toml");
                }
            }
            let texts = d.texts();
            for name in ["PillTab", "Button primary", "Toggle", "MenuItem", "Panel head", "Signals", "Spore", "Cable"] {
                assert!(texts.contains(&name), "{theme:?}: the row {name}");
            }
        }
    }
}
