//! Folding sections and icon tiles: a section is one line (its line icon, its name, what it is set to now) that opens
//! onto its body; a tile is a picture over its word, the picked one in the text colour (the kit's `tile`). Line art is
//! 1.5 px. The caller says what a press means.
use crate::base::draw::{DrawList, Font};
use crate::base::tokens::Tokens;
use crate::editor::controls::{self, Rect};

/// a line icon's side: the sections' grid
pub const HEAD_ICON: f32 = 18.0;
/// the line art's stroke
pub const LINE: f32 = 1.5;

/// the faint ground of what is picked: the text colour see-through (the kit's `tile.on`)
pub fn picked_ground(t: &Tokens) -> [f32; 4] {
    t.kit("tile", "on").ground.unwrap_or(t.color.selected)
}

/// a section head's height
pub fn head_height(t: &Tokens) -> f32 {
    t.size.rem(2.6)
}

/// A section's head across `r`: its icon, its name, what it is set to now, and whether it is open; its place.
pub fn head(d: &mut DrawList, t: &Tokens, r: Rect, icon: &'static str, name: &str, summary: &str, open: bool) -> Rect {
    if open {
        d.rect(r.x, r.y, r.w, r.h, t.color.surface_head);
    }
    d.rect(r.x, r.y + r.h - t.size.edge, r.w, t.size.edge, t.color.line);
    let pad = t.size.pad;
    let mid = r.y + r.h * 0.5;
    d.icon(r.x + pad, mid - HEAD_ICON * 0.5, HEAD_ICON, icon, t.color.text);
    let name_x = r.x + pad * 2.0 + HEAD_ICON;
    controls::label(d, t, name_x, mid - t.size.label * 0.62, name, t.color.text);
    let sum_x = name_x + t.size.rem(5.4);
    let chev = t.size.small;
    let room = r.x + r.w - pad * 2.0 - chev - sum_x;
    let px = t.size.small;
    d.sans(sum_x, mid - px * 0.62, px, &controls::fit_sans(d, px, summary, room), t.color.muted);
    d.icon(r.x + r.w - pad - chev, mid - chev * 0.5, chev, if open { "down" } else { "next" }, if open { t.color.text } else { t.color.muted });
    r
}

/// a small uppercase label over a body's group: the height it took
pub fn caption(d: &mut DrawList, t: &Tokens, x: f32, y: f32, text: &str) -> f32 {
    controls::label(d, t, x, y + t.size.pad * 0.8, text, t.color.muted);
    t.size.label + t.size.pad * 1.6
}

/// `n` cells in rows of `cols` across `w` from (`x`, `y`), each `h` high; the cells and the height they took
pub fn grid(x: f32, y: f32, w: f32, n: usize, cols: usize, h: f32, gap: f32) -> (Vec<Rect>, f32) {
    let cols = cols.max(1);
    let cw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
    let cells: Vec<Rect> = (0..n).map(|i| Rect { x: x + (i % cols) as f32 * (cw + gap), y: y + (i / cols) as f32 * (h + gap), w: cw, h }).collect();
    let rows = n.div_ceil(cols);
    (cells, if rows == 0 { 0.0 } else { rows as f32 * (h + gap) - gap })
}

/// A tile: its frame, its word at the foot (two lines at most); the room left for its picture and the picture's colour.
pub fn tile(d: &mut DrawList, t: &Tokens, r: Rect, word: &str, on: bool) -> (Rect, [f32; 4]) {
    if on {
        d.round_rect(r.x, r.y, r.w, r.h, t.size.radius * 1.4, picked_ground(t));
    }
    let s = t.kit("tile", if on { "on" } else { "rest" });
    d.outline(r.x, r.y, r.w, r.h, (t.size.radius * 1.4, t.size.edge), s.border.unwrap_or(t.color.line));
    let ink = if on { s.mark.unwrap_or(t.color.text) } else { t.color.muted };
    let px = t.size.label;
    let lines = wrap(d, px, word, r.w - 2.0);
    let lh = px * 1.25;
    let mut y = r.y + r.h - t.size.pad * 0.6 - lh * lines.len() as f32;
    let words_top = y;
    for l in &lines {
        d.sans(r.x + (r.w - d.sans_width(px, l)) * 0.5, y, px, l, ink);
        y += lh;
    }
    let top = r.y + t.size.pad * 0.6;
    (Rect { x: r.x, y: top, w: r.w, h: (words_top - top - 2.0).max(0.0) }, ink)
}

/// a word over two lines at most when it does not fit `room`
fn wrap(d: &DrawList, px: f32, word: &str, room: f32) -> Vec<String> {
    if d.sans_width(px, word) <= room {
        return vec![word.to_string()];
    }
    match word.split_once(' ') {
        Some((a, b)) => vec![a.to_string(), controls::fit_sans(d, px, b, room)],
        None => vec![controls::fit_sans(d, px, word, room)],
    }
}

/// a line icon of side `size` in the middle of `area`
pub fn picture(d: &mut DrawList, area: Rect, size: f32, icon: &'static str, color: [f32; 4]) {
    let s = size.min(area.w).min(area.h);
    d.icon(area.x + (area.w - s) * 0.5, area.y + (area.h - s) * 0.5, s, icon, color);
}

/// A focal length as the cone of what it sees: from its eye on the left, spread by `half` (radians) each side, in `area`.
pub fn cone(d: &mut DrawList, area: Rect, half: f32, color: [f32; 4]) {
    let len = (area.w * 0.78).min(area.h * 1.4);
    let eye = (area.x + (area.w - len) * 0.5, area.y + area.h * 0.5);
    let spread = (len * half.tan()).min(area.h * 0.48);
    let end = eye.0 + len;
    d.segment(eye, (end, eye.1 - spread), LINE, color);
    d.segment(eye, (end, eye.1 + spread), LINE, color);
    d.outline(eye.0 - 2.5, eye.1 - 2.5, 5.0, 5.0, (2.5, LINE), color);
}

/// A field that says one exact value (mono, on the left) and a word about it (muted, on the right); its place.
pub fn value(d: &mut DrawList, t: &Tokens, r: Rect, value: &str, note: &str, hot: bool) -> Rect {
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius * 1.4, t.color.control);
    d.outline(r.x, r.y, r.w, r.h, (t.size.radius * 1.4, t.size.edge), if hot { t.color.text } else { t.color.line });
    let px = t.size.small;
    let y = r.y + (r.h - px * 1.25) * 0.5;
    let used = d.text(r.x + t.size.pad * 1.2, y, px, value, t.color.text);
    let room = r.w - used - t.size.pad * 3.6;
    let note = controls::fit(note, (room / d.advance(px)).max(0.0) as usize);
    let w = d.measure(Font::Mono, px, &note, 0.0);
    d.text(r.x + r.w - t.size.pad * 1.2 - w, y, px, &note, t.color.muted);
    r
}

/// A shot's small picture: a frame of its view, the horizon where its pitch puts it, the aim in the middle; an
/// orthographic view as a grid.
pub fn view_thumb(d: &mut DrawList, t: &Tokens, r: Rect, pitch_deg: f32, ortho: bool, color: [f32; 4]) {
    let (sky, ground) = (t.color.control, t.color.surface_head);
    d.gradient(r.x, r.y, r.w, r.h, [sky, sky, ground, ground]);
    d.outline(r.x, r.y, r.w, r.h, (t.size.radius * 0.8, t.size.edge), t.color.line);
    let dim = [color[0], color[1], color[2], 0.55];
    if ortho {
        for k in 1..3 {
            let f = k as f32 / 3.0;
            d.segment((r.x + r.w * f, r.y + 3.0), (r.x + r.w * f, r.y + r.h - 3.0), 1.0, dim);
            d.segment((r.x + 3.0, r.y + r.h * f), (r.x + r.w - 3.0, r.y + r.h * f), 1.0, dim);
        }
    } else {
        let y = r.y + r.h * (0.5 + (pitch_deg / 90.0).clamp(-0.45, 0.45));
        d.segment((r.x + 3.0, y), (r.x + r.w - 3.0, y), 1.0, dim);
    }
    d.disc(r.x + r.w * 0.5, r.y + r.h * 0.5, 1.6, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_wraps_its_cells_and_says_its_height() {
        let (cells, h) = grid(0.0, 0.0, 100.0, 7, 6, 20.0, 4.0);
        assert_eq!(cells.len(), 7);
        assert_eq!((cells[6].x, cells[6].y), (0.0, 24.0));
        assert_eq!(h, 44.0);
        assert_eq!(grid(0.0, 0.0, 100.0, 0, 6, 20.0, 4.0).1, 0.0);
    }

    #[test]
    fn a_tile_leaves_room_over_its_word_and_a_head_says_its_summary() {
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        let (room, ink) = tile(&mut d, &t, Rect { x: 0.0, y: 0.0, w: 50.0, h: 60.0 }, "fly through", true);
        assert!(room.h > 10.0 && room.y >= 0.0);
        assert_eq!(ink, t.color.text, "the picked one in the text colour");
        head(&mut d, &t, Rect { x: 0.0, y: 100.0, w: 340.0, h: head_height(&t) }, "dop-lens", "LENS", "70 mm · perspective", false);
        assert!(d.texts().iter().any(|s| s.contains("70 mm")));
    }
}
