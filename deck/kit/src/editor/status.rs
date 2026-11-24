//! The status bar along the window's bottom edge, one pill in the panels' look: left the newest message (its level's dot,
//! its words, its age), in the middle the app's state as quiet chips, right the error and warning counts and the Log's
//! icon. A press on the newest message or the counts is the caller's (it opens the Log panel); the bar holds no state.
use crate::base::draw::{DrawList, Rect};
use crate::base::tokens::Tokens;
use crate::editor::controls::fit_sans;

/// a state word: its text, and a dot's colour when it wants to be noticed
pub struct Chip<'a> {
    pub text: &'a str,
    pub dot: Option<[f32; 4]>,
}

/// the newest message: its dot, words and age
pub struct Newest<'a> {
    pub dot: [f32; 4],
    pub text: &'a str,
    pub age: &'a str,
}

pub struct Status<'a> {
    pub newest: Option<Newest<'a>>,
    pub chips: &'a [Chip<'a>],
    pub errors: usize,
    pub warnings: usize,
    /// the Log panel is open (its icon lit)
    pub log_open: bool,
}

/// where the bar landed: all of it, and the part that opens the Log (the newest message, the counts, the icon)
pub struct Placed {
    pub bar: Rect,
    pub opens_log: Vec<Rect>,
}

/// the bar's height
pub fn height(t: &Tokens) -> f32 {
    t.size.small * 1.25 + t.size.rem(0.86)
}

/// the room the bar takes from the window's bottom, its margin included: the panels end above it
pub fn reserve(t: &Tokens) -> f32 {
    height(t) + t.size.rem(0.57)
}

pub fn draw(d: &mut DrawList, t: &Tokens, (w, h): (f32, f32), s: &Status, pointer: (f32, f32)) -> Placed {
    let margin = t.size.rem(0.57);
    let bh = height(t);
    let bar = Rect { x: margin, y: h - margin - bh, w: (w - 2.0 * margin).max(1.0), h: bh };
    d.round_rect(bar.x, bar.y, bar.w, bar.h, bh * 0.5, t.color.panel());
    d.outline(bar.x, bar.y, bar.w, bar.h, (bh * 0.5, 1.0), t.color.line);
    d.halo(t.color.ring());
    let (px, pad) = (t.size.small, t.size.pad);
    let ty = bar.y + (bh - px * 1.25) * 0.5;
    let mid = bar.y + bh * 0.5;
    let dot = t.size.rem(0.22);
    let mut opens_log = Vec::new();
    // right: the counts and the Log's icon, from the end
    let icon = t.size.rem(1.0);
    let mut right = bar.x + bar.w - bh * 0.5 - icon;
    let log_icon = Rect { x: right - 4.0, y: bar.y, w: icon + bh * 0.5 + 4.0, h: bh };
    if log_icon.has(pointer.0, pointer.1) {
        d.round_rect(log_icon.x, log_icon.y, log_icon.w, log_icon.h, bh * 0.5, t.color.hover);
    }
    d.icon(right, mid - icon * 0.5, icon, "entry", if s.log_open { t.color.text } else { t.color.muted });
    opens_log.push(log_icon);
    right -= pad;
    let counts_end = right;
    for (n, color) in [(s.warnings, t.color.warn), (s.errors, t.color.danger)] {
        let text = n.to_string();
        let tw = d.sans_width(px, &text);
        right -= tw;
        d.sans(right, ty, px, &text, if n > 0 { t.color.text } else { t.color.muted });
        right -= dot * 2.0 + 4.0;
        d.disc(right + dot, mid, dot, if n > 0 { color } else { t.color.line });
        right -= pad;
    }
    opens_log.push(Rect { x: right, y: bar.y, w: counts_end - right, h: bh });
    // the middle: the state chips, right to left, as many as fit
    let left = bar.x + bh * 0.5;
    let room_left = (bar.w * 0.4).max(0.0);
    for c in s.chips.iter().rev() {
        let tw = d.sans_width(px, c.text);
        let cw = tw + if c.dot.is_some() { dot * 2.0 + 4.0 } else { 0.0 };
        if right - cw - pad < left + room_left {
            break;
        }
        right -= cw + pad;
        d.rect(right + cw + pad * 0.5, bar.y + bh * 0.25, 1.0, bh * 0.5, t.color.line);
        let mut x = right;
        if let Some(color) = c.dot {
            d.disc(x + dot, mid, dot, color);
            x += dot * 2.0 + 4.0;
        }
        d.sans(x, ty, px, c.text, t.color.muted);
    }
    // left: the newest message in what the chips leave
    if let Some(n) = &s.newest {
        let aw = d.sans_width(px, n.age);
        let room = right - pad - left - dot * 2.0 - 8.0 - aw - pad;
        d.disc(left + dot, mid, dot, n.dot);
        let text = fit_sans(d, px, n.text, room.max(0.0));
        let tw = d.sans(left + dot * 2.0 + 8.0, ty, px, &text, t.color.text);
        let after = left + dot * 2.0 + 8.0 + tw + pad;
        d.sans(after, ty, px, n.age, t.color.muted);
        opens_log.push(Rect { x: bar.x, y: bar.y, w: after + aw - bar.x, h: bh });
    }
    d.halo(None);
    Placed { bar, opens_log }
}

/// a press refused where it happened: a short ring there, no words (they go to the bar); `k` runs 0..1 over its life
pub fn refused_ring(d: &mut DrawList, t: &Tokens, (x, y): (f32, f32), k: f32) {
    let k = k.clamp(0.0, 1.0);
    let c = t.color.danger;
    d.ring(x, y, t.size.rem(0.6) + t.size.rem(0.9) * k, [c[0], c[1], c[2], c[3] * (1.0 - k)]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_sits_on_the_bottom_edge_and_opens_the_log_from_its_counts_and_newest() {
        let t = Tokens::default();
        let mut d = DrawList::new(1.0);
        let chips = [Chip { text: "saved · v3", dot: None }, Chip { text: "2 glitches", dot: Some(t.color.warn) }];
        let s = Status { newest: Some(Newest { dot: t.color.warn, text: "pick the shot it ends on", age: "now" }), chips: &chips, errors: 1, warnings: 2, log_open: false };
        let p = draw(&mut d, &t, (1600.0, 900.0), &s, (0.0, 0.0));
        assert!((p.bar.y + p.bar.h - (900.0 - t.size.rem(0.57))).abs() < 0.01);
        assert!(900.0 - p.bar.y <= reserve(&t) + 0.01);
        assert_eq!(p.opens_log.len(), 3, "the icon, the counts, the newest message");
        assert!(p.opens_log.iter().all(|r| r.x >= p.bar.x && r.x + r.w <= p.bar.x + p.bar.w + 0.01));
    }
}
