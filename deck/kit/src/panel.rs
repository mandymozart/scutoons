//! The deck's own UI for the person at a deck (#481; owner 2026-11-10: it follows the reader's style and is built in
//! Rust): the reader's dark panel – black, white type, square corners, 2 px lines, the accent on its top edge, the
//! actions at its bottom (viewer `pull-panel.svelte`, `StickyActions`) – and its parts: a caption line, a title, lines,
//! buttons, a key line, a row with a mark, a switch row, a field, and the one-line notice. Draft 1 of #481.
//!
//! A panel is a list of parts: `height` says how tall it is, `draw` paints it over the bottom of a screen and answers
//! where each part that can be pressed landed. What a press means stays with the caller. The same parts make a prompt
//! (a flash), a notice, a choice (a role, a slot, a vote).
use crate::controls::Rect;
use crate::draw::DrawList;
use crate::tokens::{Colors, Sizes, Tokens};

const fn rgb(v: u32) -> [f32; 4] {
    [((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0, 1.0]
}

/// The reader's dark panel as the kit's tokens: ink white, paper black, the muted greys, the cyan accent; a control is
/// 36 high, the lines 2 wide, the corners square.
pub fn tokens() -> Tokens {
    let black = rgb(0x000000);
    Tokens {
        color: Colors {
            stage: black,
            surface: black,
            surface_head: black,
            line: rgb(0x555555),
            text: rgb(0xffffff),
            muted: rgb(0xa6a6a6),
            control: black,
            hover: rgb(0x222222),
            pressed: rgb(0x222222),
            selected: rgb(0x222222),
            within: rgb(0x333333),
            accent: rgb(0x00d1ff),
            ..Colors::default()
        },
        size: Sizes { edge: 2.0, pad: 16.0, font: 14.0, small: 12.0, row: 36.0, radius: 0.0, ..Sizes::default() },
    }
}

/// How many device pixels one logical pixel of the deck's UI is on a `w × h` screen: whole steps by the shorter side, so
/// a phone held upright (780 wide at two pixels per point) and a 1080p stage screen both read like the reader does – 2
/// and 3; never under 1, at most 4.
pub fn scale(w: f32, h: f32) -> f32 {
    (w.min(h) / 400.0).round().clamp(1.0, 4.0)
}

/// the title's size, and a line's height per pixel of font size (the reader's 1.35)
const TITLE: f32 = 20.0;
const LINE: f32 = 1.35;
/// between two parts, between two buttons
const GAP: f32 = 8.0;

/// One part of a panel, top down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Part<'a> {
    /// small, muted, upper case: what this is (left) and who or what it is about (right)
    Caption(&'a str, &'a str),
    Title(&'a str),
    /// a sentence, wrapped; muted or not
    Line(&'a str, bool),
    /// the actions side by side, the first one the primary; the one the keys are on; a countdown (0..1 of its time gone)
    /// after which the primary happens by itself – the reader's line along the primary's bottom (its chapter end, `next-bar`)
    Buttons(&'a [&'a str], Option<usize>, Option<f32>),
    /// what the keys do, for a deck without touch: (key, what)
    Keys(&'a [(&'a str, &'a str)]),
    /// a row to choose: its label, a note at its right, chosen or not
    Mark(&'a str, &'a str, bool),
    /// a row with a switch
    Switch(&'a str, bool),
    /// one line of text to edit; empty shows the hint, muted
    Field(&'a str, &'a str),
}

/// `text` in lines of at most `cols` characters, broken at spaces
pub fn wrap(text: &str, cols: usize) -> Vec<String> {
    let mut out: Vec<String> = vec![String::new()];
    for word in text.split_whitespace() {
        let last = out.last_mut().expect("one line");
        let room = cols.saturating_sub(last.chars().count());
        if last.is_empty() {
            last.push_str(word);
        } else if word.chars().count() < room {
            last.push(' ');
            last.push_str(word);
        } else {
            out.push(word.to_string());
        }
    }
    out
}

fn cols(d: &DrawList, px: f32, width: f32) -> usize {
    ((width / d.advance(px)) as usize).max(8)
}

fn part_height(d: &DrawList, t: &Tokens, width: f32, part: &Part) -> f32 {
    match part {
        Part::Caption(..) => t.size.small * LINE,
        Part::Title(_) => TITLE * LINE,
        Part::Line(text, _) => wrap(text, cols(d, t.size.font, width)).len() as f32 * t.size.font * LINE,
        Part::Buttons(..) => t.size.row + 6.0,
        Part::Keys(_) => t.size.small * LINE,
        Part::Mark(..) | Part::Switch(..) => t.size.row,
        Part::Field(..) => t.size.row,
    }
}

/// two rows share their line: no gap between them
fn gap(before: &Part, after: &Part) -> f32 {
    let row = |p: &Part| matches!(p, Part::Mark(..) | Part::Switch(..));
    if row(before) && row(after) {
        0.0
    } else {
        GAP
    }
}

/// How tall the panel of `parts` is on a screen `width` wide.
pub fn height(d: &DrawList, t: &Tokens, width: f32, parts: &[Part]) -> f32 {
    let inner = width - 2.0 * t.size.pad;
    let gaps: f32 = parts.windows(2).map(|w| gap(&w[0], &w[1])).sum();
    t.size.edge + t.size.pad + parts.iter().map(|p| part_height(d, t, inner, p)).sum::<f32>() + gaps + t.size.pad
}

/// The panel of `parts` over the bottom of a `screen`: its ground, the accent edge with its glow, a grip on a touch
/// screen, the parts. The places of what can be pressed, in the parts' order: each button, each row, each field.
pub fn draw(d: &mut DrawList, t: &Tokens, screen: (f32, f32), parts: &[Part], grip: bool) -> Vec<Rect> {
    let h = height(d, t, screen.0, parts);
    let top = screen.1 - h;
    // the glow above the edge, then the edge, then the ground
    for i in 0..8 {
        let a = 0.22 * (1.0 - i as f32 / 8.0).powi(2);
        d.rect(0.0, top - 2.0 * (i + 1) as f32, screen.0, 2.0, [t.color.accent[0], t.color.accent[1], t.color.accent[2], a]);
    }
    d.rect(0.0, top, screen.0, h, t.color.surface);
    d.rect(0.0, top, screen.0, t.size.edge, t.color.accent);
    if grip {
        d.rect(screen.0 * 0.5 - 20.0, top + t.size.edge + 4.0, 40.0, 4.0, t.color.line);
    }
    let area = Rect { x: t.size.pad, y: top + t.size.edge + t.size.pad, w: screen.0 - 2.0 * t.size.pad, h: h - t.size.edge - 2.0 * t.size.pad };
    let mut y = area.y;
    let mut places = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let ph = part_height(d, t, area.w, part);
        match *part {
            Part::Caption(left, right) => {
                d.text(area.x, y, t.size.small, &left.to_uppercase(), t.color.muted);
                let right = right.to_uppercase();
                let w = d.advance(t.size.small) * right.chars().count() as f32;
                d.text(area.x + area.w - w, y, t.size.small, &right, t.color.muted);
            }
            Part::Title(text) => {
                d.text(area.x, y, TITLE, text, t.color.text);
            }
            Part::Line(text, muted) => {
                for (n, line) in wrap(text, cols(d, t.size.font, area.w)).iter().enumerate() {
                    d.text(area.x, y + n as f32 * t.size.font * LINE, t.size.font, line, if muted { t.color.muted } else { t.color.text });
                }
            }
            Part::Buttons(labels, focus, gone) => {
                let n = labels.len().max(1) as f32;
                let w = (area.w - GAP * (n - 1.0)) / n;
                for (b, label) in labels.iter().enumerate() {
                    let at = Rect { x: area.x + b as f32 * (w + GAP), y: y + 6.0, w, h: t.size.row };
                    button(d, t, at, label, b == 0, focus == Some(b));
                    if let Some(gone) = gone.filter(|_| b == 0) {
                        // on the ink primary: a paper line that fills as the time goes
                        d.rect(at.x, at.y + at.h - t.size.edge, at.w * gone.clamp(0.0, 1.0), t.size.edge, t.color.surface);
                    }
                    places.push(at);
                }
            }
            Part::Keys(keys) => {
                let px = t.size.small;
                let mut x = area.x;
                for (k, (key, what)) in keys.iter().enumerate() {
                    if k > 0 {
                        x += d.text(x, y, px, " · ", t.color.muted);
                    }
                    let w = d.advance(px) * key.chars().count() as f32 + 8.0;
                    d.outline(x, y - 1.0, w, px * LINE + 2.0, (0.0, 1.0), t.color.line);
                    d.text(x + 4.0, y, px, key, t.color.text);
                    x += w + 6.0;
                    x += d.text(x, y, px, what, t.color.muted);
                }
            }
            Part::Mark(label, note, on) => {
                let at = Rect { x: area.x, y, w: area.w, h: ph };
                row(d, t, at, label, on, i == 0 || !matches!(parts[i - 1], Part::Mark(..) | Part::Switch(..)));
                let m = Rect { x: at.x + 2.0, y: y + (ph - 14.0) * 0.5, w: 14.0, h: 14.0 };
                if on {
                    d.rect(m.x, m.y, m.w, m.h, t.color.text);
                } else {
                    d.outline(m.x, m.y, m.w, m.h, (0.0, t.size.edge), t.color.line);
                }
                let w = d.advance(t.size.small) * note.chars().count() as f32;
                d.text(at.x + at.w - w - 2.0, y + (ph - t.size.small * LINE) * 0.5, t.size.small, note, t.color.muted);
                places.push(at);
            }
            Part::Switch(label, on) => {
                let at = Rect { x: area.x, y, w: area.w, h: ph };
                row(d, t, at, label, false, i == 0 || !matches!(parts[i - 1], Part::Mark(..) | Part::Switch(..)));
                let s = Rect { x: at.x + at.w - 36.0, y: y + (ph - 18.0) * 0.5, w: 34.0, h: 18.0 };
                if on {
                    d.rect(s.x, s.y, s.w, s.h, t.color.text);
                    d.rect(s.x + s.w - 14.0, s.y + 4.0, 10.0, 10.0, t.color.surface);
                } else {
                    d.outline(s.x, s.y, s.w, s.h, (0.0, t.size.edge), t.color.text);
                    d.rect(s.x + 4.0, s.y + 4.0, 10.0, 10.0, t.color.text);
                }
                places.push(at);
            }
            Part::Field(text, hint) => {
                let at = Rect { x: area.x, y, w: area.w, h: ph };
                d.outline(at.x, at.y, at.w, at.h, (0.0, t.size.edge), t.color.line);
                let (shown, color) = if text.is_empty() { (hint, t.color.muted) } else { (text, t.color.text) };
                d.text(at.x + 10.0, y + (ph - t.size.font * LINE) * 0.5, t.size.font, shown, color);
                places.push(at);
            }
        }
        y += ph + parts.get(i + 1).map_or(0.0, |next| gap(part, next));
    }
    places
}

/// a button: the primary one filled with the ink, the others framed; the one the keys are on wears the accent
fn button(d: &mut DrawList, t: &Tokens, at: Rect, label: &str, primary: bool, focus: bool) {
    if primary {
        d.rect(at.x, at.y, at.w, at.h, t.color.text);
    } else {
        d.outline(at.x, at.y, at.w, at.h, (0.0, t.size.edge), t.color.line);
    }
    if focus {
        d.outline(at.x - 4.0, at.y - 4.0, at.w + 8.0, at.h + 8.0, (0.0, 3.0), t.color.accent);
    }
    let px = t.size.font;
    let w = d.advance(px) * label.chars().count() as f32;
    d.text(at.x + (at.w - w) * 0.5, at.y + (at.h - px * LINE) * 0.5, px, label, if primary { t.color.surface } else { t.color.text });
}

/// a row's ground (the chosen one lifted), its hairlines, its label after the mark's place
fn row(d: &mut DrawList, t: &Tokens, at: Rect, label: &str, on: bool, first: bool) {
    if on {
        d.rect(at.x, at.y, at.w, at.h, t.color.hover);
    }
    if first {
        d.rect(at.x, at.y, at.w, 1.0, t.color.within);
    }
    d.rect(at.x, at.y + at.h - 1.0, at.w, 1.0, t.color.within);
    d.text(at.x + 26.0, at.y + (at.h - t.size.font * LINE) * 0.5, t.size.font, label, t.color.text);
}

/// The one-line notice at the bottom of a `screen`: no panel and no question – what happened (left), who (right). It goes
/// by itself; the caller says when.
pub fn notice(d: &mut DrawList, t: &Tokens, screen: (f32, f32), left: &str, right: &str) -> Rect {
    let px = t.size.small;
    let at = Rect { x: 12.0, y: screen.1 - 12.0 - (px * LINE + 16.0), w: screen.0 - 24.0, h: px * LINE + 16.0 };
    d.rect(at.x, at.y, at.w, at.h, t.color.surface);
    d.outline(at.x, at.y, at.w, at.h, (0.0, t.size.edge), t.color.line);
    d.text(at.x + 12.0, at.y + 8.0, px, left, t.color.text);
    let w = d.advance(px) * right.chars().count() as f32;
    d.text(at.x + at.w - 12.0 - w, at.y + 8.0, px, right, t.color.muted);
    at
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Prim;

    fn texts(d: &DrawList) -> Vec<String> {
        d.prims().iter().filter_map(|p| if let Prim::Text { start, end, .. } = *p { Some(d.text_of(start, end).to_string()) } else { None }).collect()
    }

    #[test]
    fn a_prompt_is_as_tall_as_its_parts_and_says_where_its_buttons_are() {
        let (mut d, t) = (DrawList::new(0.6), tokens());
        let parts = [
            Part::Caption("Flash · Vorschlag", "von ana"),
            Part::Title("top · Version 3"),
            Part::Line("Du spielst deine eigene Version (top · 2). Sie bleibt behalten.", true),
            Part::Buttons(&["Annehmen", "Ablehnen"], Some(0), Some(0.5)),
            Part::Keys(&[("Enter", "annehmen"), ("Esc", "ablehnen")]),
        ];
        let screen = (640.0, 400.0);
        let h = height(&d, &t, screen.0, &parts);
        let places = draw(&mut d, &t, screen, &parts, false);
        // two buttons, side by side, inside the panel and its padding
        assert_eq!(places.len(), 2);
        assert!(places[0].x == t.size.pad && places[1].x > places[0].x + places[0].w && places[1].x + places[1].w <= screen.0 - t.size.pad + 0.01);
        assert!(places.iter().all(|p| p.y > screen.1 - h && p.y + p.h < screen.1 - t.size.pad + 0.01), "{places:?} in a panel {h} tall");
        assert!(places[0].has(100.0, places[0].y + 5.0) && !places[0].has(100.0, screen.1 - h - 5.0));
        let said = texts(&d);
        assert!(said.contains(&"FLASH · VORSCHLAG".to_string()) && said.contains(&"VON ANA".to_string()), "the caption is upper case: {said:?}");
        assert!(said.contains(&"Annehmen".to_string()) && said.contains(&"Enter".to_string()));
        // the countdown: a paper line along the primary's bottom, half its width at half the time
        let (a, edge) = (places[0], t.size.edge);
        let line = Prim::Rect { x: a.x, y: a.y + a.h - edge, w: a.w * 0.5, h: edge, r: 0.0, color: t.color.surface };
        assert!(d.prims().contains(&line), "the countdown's line");
    }

    #[test]
    fn the_scale_follows_the_shorter_side_in_whole_steps() {
        assert_eq!((scale(780.0, 1688.0), scale(1920.0, 1080.0), scale(800.0, 600.0), scale(320.0, 240.0), scale(3840.0, 2160.0)), (2.0, 3.0, 2.0, 1.0, 4.0));
    }

    #[test]
    fn a_long_line_wraps_and_makes_the_panel_taller() {
        let (d, t) = (DrawList::new(0.6), tokens());
        let long = "Du spielst deine eigene Version (top · 2). Sie bleibt behalten.";
        assert_eq!(wrap(long, 29), ["Du spielst deine eigene", "Version (top · 2). Sie bleibt", "behalten."]);
        assert_eq!(wrap("", 10), [""]);
        let (wide, narrow) = (height(&d, &t, 640.0, &[Part::Line(long, true)]), height(&d, &t, 300.0, &[Part::Line(long, true)]));
        assert!(narrow > wide, "{narrow} on a phone, {wide} on a deck");
    }

    #[test]
    fn rows_a_switch_and_a_field_are_places_and_the_notice_is_one_line() {
        let (mut d, t) = (DrawList::new(0.6), tokens());
        let parts = [
            Part::Caption("Rolle", "Show: top"),
            Part::Mark("Publikum", "jetzt", true),
            Part::Mark("Licht", "nur live", false),
            Part::Switch("Bitte nicht flashen", true),
            Part::Field("", "warum"),
            Part::Buttons(&["Übernehmen"], None, None),
        ];
        let places = draw(&mut d, &t, (300.0, 560.0), &parts, true);
        assert_eq!(places.len(), 5, "two rows, the switch, the field, the button");
        // the rows share their lines: no gap between them
        assert_eq!(places[1].y, places[0].y + places[0].h);
        assert!(texts(&d).contains(&"warum".to_string()), "an empty field shows its hint");
        d.clear();
        let at = notice(&mut d, &t, (300.0, 560.0), "Geflasht: top · Version 3", "von ana");
        assert!((at.y + at.h - 548.0).abs() < 0.01 && at.x == 12.0);
        assert_eq!(texts(&d), ["Geflasht: top · Version 3", "von ana"]);
    }
}
