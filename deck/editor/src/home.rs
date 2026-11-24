//! Home (decisions/deck.md [workspaces]): what the editor shows while no tab is open – a cold start, or every tab
//! closed. The deck's idle scene plays behind it (fireflies on black). Over it: New (a scene, a module, a show), the
//! recent items, the library (what can be opened as a tab), the deck's state, and the help's link.
use crate::tabs::Kind;
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls;

/// where the help is (creator.scutoons.com/docs, #624): a press opens it in the browser, or copies it
pub const DOCS: &str = "https://creator.scutoons.com/docs";

/// `url` in the system's browser (a desktop's; a kiosk deck has none, so the caller copies the link instead)
pub fn open_in_browser(url: &str) -> std::io::Result<()> {
    let mut c = if cfg!(windows) {
        let mut c = std::process::Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    } else {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    c.spawn().map(drop)
}

/// what New makes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum New {
    Scene,
    Module,
    Show,
}

/// a press on Home
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    New(New),
    /// a line of Recent, of the library: its index in what was drawn
    Recent(usize),
    Library(usize),
    Docs,
}

/// one line: an address, its name
pub struct Line {
    pub key: String,
    pub name: String,
}

/// what Home shows this frame
pub struct Home<'a> {
    pub recent: &'a [Line],
    pub library: &'a [Line],
    /// the deck's state, a line each: the account, the audio, the relay
    pub deck: &'a [String],
    /// why the library is empty, when it is
    pub library_why: Option<&'a str>,
}

const NEW: [(&str, New); 3] = [("New scene", New::Scene), ("New module", New::Module), ("New show", New::Show)];

/// a list of lines with their kind's icon, at most as many as fit `bottom`; the places pressed
fn lines(d: &mut DrawList, t: &Tokens, (x, y, w, bottom): (f32, f32, f32, f32), lines: &[Line], pointer: (f32, f32)) -> Vec<Rect> {
    let (row, px, pad) = (t.size.row, t.size.font, t.size.pad);
    let icon = t.size.rem(1.0);
    let mut out = Vec::new();
    let mut cy = y;
    for l in lines {
        if cy + row > bottom {
            break;
        }
        let r = Rect { x, y: cy, w, h: row };
        if r.has(pointer.0, pointer.1) {
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.hover);
        }
        let kind = Kind::of(&l.key);
        d.icon(x + pad * 0.5, cy + (row - icon) * 0.5, icon, kind.map_or("entry", Kind::icon), t.color.muted);
        let tx = x + pad * 0.5 + icon + pad * 0.6;
        let room = x + w - tx - pad * 0.5;
        let used = d.sans(tx, cy + (row - px * 1.25) * 0.5, px, &controls::fit_sans(d, px, &l.name, room * 0.6), t.color.text);
        if let Some(k) = kind {
            let small = t.size.small;
            d.sans(tx + used + pad * 0.6, cy + (row - small * 1.25) * 0.5, small, &controls::fit_sans(d, small, k.word(), (room - used - pad).max(0.0)), t.color.muted);
        }
        out.push(r);
        cy += row;
    }
    out
}

/// Home on the stage `size`, under the toolbar ending at `top`; what each place does
pub fn draw(d: &mut DrawList, t: &Tokens, (w, h): (f32, f32), top: f32, home: &Home, pointer: (f32, f32)) -> Vec<(Rect, Press)> {
    let (pad, row, px) = (t.size.pad, t.size.row, t.size.font);
    let bottom_room = deck_kit::editor::status::reserve(t) + t.size.rem(2.0);
    let card_w = (w - 2.0 * t.size.rem(2.0)).min(t.size.rem(64.0)).max(t.size.rem(20.0));
    let card_h = (h - top - bottom_room - t.size.rem(3.0)).min(t.size.rem(34.0)).max(row * 8.0);
    let card = Rect { x: (w - card_w) * 0.5, y: top + t.size.rem(3.0), w: card_w, h: card_h };
    let s = t.kit("panel", "rest");
    d.round_rect(card.x, card.y, card.w, card.h, t.size.radius * 2.0, s.ground.unwrap_or(t.color.panel()));
    d.outline(card.x, card.y, card.w, card.h, (t.size.radius * 2.0, 1.0), s.border.unwrap_or(t.color.line));
    d.halo(t.color.ring());
    let mut out = Vec::new();
    let inner = t.size.rem(1.5);
    let (x0, mut y) = (card.x + inner, card.y + inner);
    let title = t.size.title;
    d.sans(x0, y, title, "Home", t.color.text);
    let sub = "Nothing is open. Open something recent or from the library, or start something new.";
    d.sans(x0, y + title * 1.4, t.size.small, &controls::fit_sans(d, t.size.small, sub, card.w * 0.6), t.color.muted);
    // New, at the right of the title
    let names: Vec<&str> = NEW.iter().map(|(n, _)| *n).collect();
    let nw: f32 = names.iter().map(|n| d.sans_width(t.size.small, n) + 2.0 * pad + 2.0).sum();
    let pills = controls::pills(d, t, card.x + card.w - inner - nw, y, row, &names, usize::MAX);
    for (r, (_, what)) in pills.into_iter().zip(NEW) {
        d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.0), t.color.line);
        out.push((r, Press::New(what)));
    }
    y += title * 1.4 + t.size.small * 1.4 + inner;
    d.line((card.x, y), (card.x + card.w, y), 0.5, t.color.line);
    y += inner;
    // three columns: Recent, the library, this deck; the help's link at the foot
    let foot = card.y + card.h - inner - row;
    let col_gap = inner;
    let col_w = (card.w - 2.0 * inner - 2.0 * col_gap) / 3.0;
    let heads = ["Recent", "Library", "This deck"];
    for (c, head) in heads.iter().enumerate() {
        controls::label(d, t, x0 + c as f32 * (col_w + col_gap), y, head, t.color.muted);
    }
    let list_y = y + t.size.label * 1.8;
    let list_bottom = foot - pad;
    if home.recent.is_empty() {
        d.sans(x0, list_y + pad * 0.5, t.size.small, "Nothing yet.", t.color.muted);
    }
    for (i, r) in lines(d, t, (x0, list_y, col_w, list_bottom), home.recent, pointer).into_iter().enumerate() {
        out.push((r, Press::Recent(i)));
    }
    let lx = x0 + col_w + col_gap;
    if let (true, Some(why)) = (home.library.is_empty(), home.library_why) {
        let room = (col_w / d.advance(t.size.small)).max(8.0) as usize;
        for (k, l) in crate::empty::wrap(why, room).iter().enumerate() {
            d.sans(lx, list_y + pad * 0.5 + k as f32 * t.size.small * 1.4, t.size.small, l, t.color.muted);
        }
    }
    for (i, r) in lines(d, t, (lx, list_y, col_w, list_bottom), home.library, pointer).into_iter().enumerate() {
        out.push((r, Press::Library(i)));
    }
    let dx = x0 + 2.0 * (col_w + col_gap);
    for (k, line) in home.deck.iter().enumerate() {
        let ly = list_y + pad * 0.5 + k as f32 * row;
        if ly + row > list_bottom {
            break;
        }
        d.sans(dx, ly, px, &controls::fit_sans(d, px, line, col_w), t.color.text);
    }
    // the help
    let help = format!("Help: {}", DOCS.trim_start_matches("https://"));
    let hw = d.sans_width(px, &help) + 2.0 * pad;
    let hr = Rect { x: x0 - pad, y: foot, w: hw, h: row };
    if hr.has(pointer.0, pointer.1) {
        d.round_rect(hr.x, hr.y, hr.w, hr.h, t.size.radius, t.color.hover);
    }
    d.sans(x0, foot + (row - px * 1.25) * 0.5, px, &help, t.color.text);
    out.push((hr, Press::Docs));
    d.halo(None);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(key: &str) -> Line {
        Line { key: key.into(), name: crate::tabs::name_of(key) }
    }

    #[test]
    fn home_offers_new_recent_the_library_the_deck_and_the_docs() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let recent = [line("scene:nave"), line("module:analog")];
        let library = [line("character:catpope"), line("scene:cathedral"), line("item:lamp")];
        let deck = ["signed in".to_string(), "audio: ASIO · 48 kHz".to_string(), "relay: not joined".to_string()];
        let places = draw(&mut d, &t, (2560.0, 1440.0), 60.0, &Home { recent: &recent, library: &library, deck: &deck, library_why: None }, (0.0, 0.0));
        let count = |f: &dyn Fn(&Press) -> bool| places.iter().filter(|(_, p)| f(p)).count();
        assert_eq!(count(&|p| matches!(p, Press::New(_))), 3);
        assert_eq!(count(&|p| matches!(p, Press::Recent(_))), 2);
        assert_eq!(count(&|p| matches!(p, Press::Library(_))), 3);
        assert_eq!(count(&|p| *p == Press::Docs), 1);
        let texts = d.texts();
        for want in ["Home", "New scene", "New module", "New show", "nave", "catpope", "relay: not joined"] {
            assert!(texts.contains(&want), "{want}: {texts:?}");
        }
        assert!(texts.iter().any(|s| s.contains("creator.scutoons.com/docs")));
        assert!(places.iter().all(|(r, _)| r.y > 60.0), "all under the toolbar");
    }

    #[test]
    fn an_empty_library_says_why_and_a_small_window_still_lands_new() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let places = draw(&mut d, &t, (800.0, 500.0), 60.0, &Home { recent: &[], library: &[], deck: &[], library_why: Some("sign in to see the toon's library") }, (0.0, 0.0));
        assert!(d.texts().iter().any(|s| s.contains("sign in")));
        assert_eq!(places.iter().filter(|(_, p)| matches!(p, Press::New(_))).count(), 3);
    }
}
