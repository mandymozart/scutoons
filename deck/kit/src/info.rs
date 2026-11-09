//! The Info view (#456; owner 2026-11-09, from the OSCT editor): the name and description of what is under the pointer,
//! collapsible to its head, so the screen carries no hint lines. Panels show values, states and errors; what a control does
//! is its help here.
use crate::controls::Rect;
use crate::draw::DrawList;
use crate::tokens::Tokens;

const WIDTH: f32 = 360.0;
const LINES: usize = 5;

/// `text` in lines of at most `cols` characters, broken at spaces
pub fn wrap(text: &str, cols: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let extra = usize::from(!line.is_empty()) + word.chars().count();
        if !line.is_empty() && line.chars().count() + extra > cols {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// What the view shows
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Help {
    pub name: String,
    pub text: String,
}

impl Help {
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Help {
        Help { name: name.into(), text: text.into() }
    }
}

/// Where it landed: the head toggles it, the whole keeps the pointer from what is beneath
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Landed {
    pub head: Rect,
    pub whole: Rect,
}

/// the view in the lower left corner of the area ending at `bottom`
pub fn view(d: &mut DrawList, t: &Tokens, x: f32, bottom: f32, open: bool, help: Option<&Help>) -> Landed {
    let (font, row, pad) = (t.size.font * 0.82, t.size.row * 0.8, t.size.pad * 0.7);
    let line_h = font * 1.4;
    let adv = d.advance(font);
    let cols = ((WIDTH - 2.0 * pad) / adv) as usize;
    let lines = help.map_or_else(Vec::new, |h| wrap(&h.text, cols));
    let body = if open { line_h * (1.0 + LINES as f32) + pad } else { 0.0 };
    let top = bottom - row - body;
    let whole = Rect { x, y: top, w: WIDTH, h: row + body };
    d.rect(x, top, WIDTH, whole.h, [t.color.panel[0], t.color.panel[1], t.color.panel[2], 0.9]);
    let text_y = (row - font * 1.25) * 0.5;
    d.text(x + pad, top + text_y, font, if open { "v info" } else { "^ info" }, t.color.muted);
    if open {
        let mut y = top + row;
        match help {
            Some(h) => {
                d.text(x + pad, y, font, &h.name, t.color.text);
                y += line_h;
                for line in lines.iter().take(LINES) {
                    d.text(x + pad, y, font, line, t.color.muted);
                    y += line_h;
                }
            }
            None => {
                d.text(x + pad, y, font, "Point at something to see what it does.", t.color.muted);
            }
        }
    }
    Landed { head: Rect { x, y: top, w: WIDTH, h: row }, whole }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_text_breaks_at_spaces_and_a_long_word_stays_whole() {
        assert_eq!(wrap("drag a knob to turn it", 10), ["drag a", "knob to", "turn it"]);
        assert_eq!(wrap("voice:voice.out x", 5), ["voice:voice.out", "x"]);
        assert!(wrap("  ", 5).is_empty());
    }

    #[test]
    fn closed_it_is_only_its_head() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let l = view(&mut d, &t, 10.0, 500.0, false, None);
        assert_eq!(l.head, l.whole);
        assert_eq!(l.whole.y + l.whole.h, 500.0);
        let l = view(&mut d, &t, 10.0, 500.0, true, Some(&Help::new("a", "b")));
        assert!(l.whole.h > l.head.h);
        assert_eq!(l.whole.y + l.whole.h, 500.0, "it grows up from its corner");
    }
}
