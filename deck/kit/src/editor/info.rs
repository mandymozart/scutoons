//! The Info view (#456; owner 2026-11-09, from the OSCT editor): the name and description of what is under the pointer,
//! collapsible to its head, so the screen carries no hint lines. Panels show values, states and errors; what a control does
//! is its help here.
use crate::base::draw::DrawList;
use crate::base::tokens::Tokens;

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

/// the view's content from (x, y), `width` wide (the info is a floating panel of its own): the name and what it does,
/// always as tall as its most lines, so it keeps its size while the pointer moves; the height it took
pub fn body(d: &mut DrawList, t: &Tokens, (x, y, width): (f32, f32, f32), help: Option<&Help>) -> f32 {
    let font = t.size.small;
    let line_h = font * 1.4;
    let cols = (width / d.advance(font)).max(1.0) as usize;
    match help {
        Some(h) => {
            d.text(x, y, font, &h.name, t.color.text);
            for (i, line) in wrap(&h.text, cols).iter().take(LINES).enumerate() {
                d.text(x, y + line_h * (1.0 + i as f32), font, line, t.color.muted);
            }
        }
        None => {
            d.text(x, y, font, "Point at something to see what it does.", t.color.muted);
        }
    }
    line_h * (1.0 + LINES as f32)
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
    fn it_keeps_its_height_whatever_it_says() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let empty = body(&mut d, &t, (10.0, 500.0, 300.0), None);
        let long = body(&mut d, &t, (10.0, 500.0, 300.0), Some(&Help::new("a", "word ".repeat(400))));
        assert_eq!(empty, long);
        assert!(d.extent(0).unwrap().h <= long + 1.0);
    }
}
