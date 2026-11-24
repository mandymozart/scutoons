//! Cards on a panel without grid lines: a cell is a card, its status a line with a dot, what it says of itself a quiet
//! tag (transparent, a thin border, a coloured kind icon, the text muted and cut with an ellipsis), the edge between
//! cards a handle that shows only while the pointer is on it or drags it, with the size it comes to. The caller says
//! what each piece means.
use crate::base::draw::DrawList;
use crate::base::tokens::Tokens;
use crate::editor::controls::{self, Rect};

/// `color` at `share` of its opacity
pub fn fade(color: [f32; 4], share: f32) -> [f32; 4] {
    [color[0], color[1], color[2], color[3] * share]
}

/// the faint colour: what only says that nothing new happens here
pub fn faint(t: &Tokens) -> [f32; 4] {
    fade(t.color.muted, 0.7)
}

/// `text` cut with an ellipsis to `w` at `px`
pub fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// A dashed outline around `r`, 1 px: what is offered and waits.
pub fn dashed(d: &mut DrawList, r: Rect, color: [f32; 4]) {
    let dash = 4.0;
    let mut at = r.x;
    while at < r.x + r.w {
        let end = (at + dash).min(r.x + r.w);
        d.rect(at, r.y, end - at, 1.0, color);
        d.rect(at, r.y + r.h - 1.0, end - at, 1.0, color);
        at += dash * 2.0;
    }
    let mut at = r.y;
    while at < r.y + r.h {
        let end = (at + dash).min(r.y + r.h);
        d.rect(r.x, at, 1.0, end - at, color);
        d.rect(r.x + r.w - 1.0, at, 1.0, end - at, color);
        at += dash * 2.0;
    }
}

/// a tag's height
pub fn tag_height(t: &Tokens) -> f32 {
    t.size.label * 1.2 * 1.6
}

/// One quiet tag: its kind's icon in its colour, its words, and room for something the caller draws after them.
#[derive(Debug, Clone, Copy)]
pub struct Tag<'a> {
    pub icon: &'static str,
    pub tint: [f32; 4],
    pub text: &'a str,
    /// offered, waiting for a yes: a dashed border
    pub dashed: bool,
    /// the width kept free after the words (the caller draws there)
    pub trail: f32,
}

/// how wide `tag` draws when nothing cuts it
pub fn tag_width(d: &DrawList, t: &Tokens, tag: &Tag) -> f32 {
    let h = tag_height(t);
    let gap = if tag.text.is_empty() { 0.0 } else { h * 0.25 };
    let trail = if tag.trail > 0.0 { tag.trail + h * 0.3 } else { 0.0 };
    h * 0.3 + h * 0.62 + gap + trail + h * 0.3 + d.advance(t.size.label * 1.2) * tag.text.chars().count() as f32
}

/// A tag at `(x, y)`, no wider than `w`: its place, and where its trail starts.
pub fn tag(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), tag: &Tag) -> (Rect, f32) {
    let px = t.size.label * 1.2;
    let h = tag_height(t);
    let icon = h * 0.62;
    let inset = h * 0.3;
    let gap = if tag.text.is_empty() { 0.0 } else { h * 0.25 };
    let trail = if tag.trail > 0.0 { tag.trail + h * 0.3 } else { 0.0 };
    let frame = inset + icon + gap + trail + inset;
    let words = fit(d, tag.text, px, (w - frame).max(0.0));
    let tw = d.advance(px) * words.chars().count() as f32;
    let r = Rect { x, y, w: (frame + tw).min(w.max(frame)), h };
    let border = fade(t.color.muted, 0.55);
    if tag.dashed {
        dashed(d, r, border);
    } else {
        d.outline(r.x, r.y, r.w, r.h, (t.size.radius * 0.6, 1.0), border);
    }
    d.icon(x + inset, y + (h - icon) * 0.5, icon, tag.icon, tag.tint);
    let tx = x + inset + icon + gap;
    if !words.is_empty() {
        d.text(tx, y + (h - px * 1.25) * 0.5, px, &words, t.color.muted);
    }
    (r, tx + tw + if trail > 0.0 { h * 0.3 } else { 0.0 })
}

/// A card's status line at `(x, y)`, no wider than `w`: a dot in `color`, the words in it, not shouted; its height.
pub fn status(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), text: &str, color: [f32; 4]) -> f32 {
    let px = t.size.label * 1.2;
    let lh = px * 1.25;
    let r = px * 0.22;
    d.disc(x + r, y + lh * 0.5, r, color);
    let tx = x + r * 2.0 + px * 0.45;
    d.text(tx, y, px, &fit(d, text, px, (x + w - tx).max(0.0)), color);
    lh
}

/// The handle of an edge between cards along `a`–`b`: a line in the text colour, shown while the pointer is on it or drags it.
pub fn handle(d: &mut DrawList, t: &Tokens, (a, b): ((f32, f32), (f32, f32)), dragging: bool) {
    let thick = if dragging { 2.0 } else { 1.5 };
    if a.0 == b.0 {
        d.rect(a.0 - thick * 0.5, a.1, thick, b.1 - a.1, t.color.text);
    } else {
        d.rect(a.0, a.1 - thick * 0.5, b.0 - a.0, thick, t.color.text);
    }
}

/// The size an edge being dragged comes to, as a badge centred on `x` with its top at `y`: `right · 230 px`.
pub fn badge(d: &mut DrawList, t: &Tokens, (x, y): (f32, f32), text: &str) -> Rect {
    let px = t.size.label * 1.2;
    let h = px * 1.7;
    let w = d.advance(px) * text.chars().count() as f32 + px * 1.2;
    let r = Rect { x: x - w * 0.5, y, w, h };
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius * 0.6, t.color.pressed);
    d.text(r.x + px * 0.6, r.y + (h - px * 1.25) * 0.5, px, text, t.color.pressed_text);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_cuts_its_words_to_its_width_and_keeps_its_trail() {
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        let long = Tag { icon: "bolt", tint: t.color.muted, text: "ch3 · yellowmouse-enters-the-nave", dashed: false, trail: 12.0 };
        let (r, trail) = tag(&mut d, &t, (0.0, 0.0, 120.0), &long);
        assert!(r.w <= 120.0 + 0.01, "{r:?}");
        assert!(d.texts().iter().any(|s| s.ends_with('…')), "{:?}", d.texts());
        assert!(trail + 12.0 <= r.x + r.w + 0.01, "the trail stays inside: {trail} {r:?}");
        let mut d = DrawList::new(0.6);
        let (short, _) = tag(&mut d, &t, (0.0, 0.0, 400.0), &Tag { text: "", ..long });
        assert!(short.w < 60.0 && d.texts().is_empty(), "the bolt and the ticks alone: {short:?}");
    }

    #[test]
    fn a_dashed_border_is_pieces_and_a_status_line_is_not_shouted() {
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        dashed(&mut d, Rect { x: 0.0, y: 0.0, w: 40.0, h: 16.0 }, t.color.muted);
        assert!(d.prims().len() > 8);
        let mut d = DrawList::new(0.6);
        status(&mut d, &t, (0.0, 0.0, 200.0), "nothing plays", t.color.warn);
        assert_eq!(d.texts(), ["nothing plays"]);
    }
}
