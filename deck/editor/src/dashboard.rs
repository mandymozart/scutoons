//! The dashboard (the creator's dashboard, the same model – research/redesign/lap/crafts-v2.html `deckfirst`): what the
//! menu button far right opens, and what shows while nothing is open – the very first open says "Nothing open yet". On
//! one floating panel over the stage: the workspaces as cards (flat, the front one first, then most used), open
//! anything (New, Recent, the library), the menu's lines, the views of the workspace in front with their four states
//! (hidden · panel · stage · full screen), and the bottom line: the account, the help, the build and the load.
use crate::bar::{self, Action};
use crate::home::{self, Line};
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls;
use deck_ui::stage::{ViewState, VIEW_STATES};
use deck_ui::workspaces::Workspace;

/// a press on the dashboard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Workspace(Workspace),
    /// New, a line of Recent or the library, the help
    Home(home::Press),
    Menu(Action),
    View(&'static str, ViewState),
    /// the load numbers
    Load,
    /// the card's ground: nothing, but it keeps the press from the stage
    Card,
}

/// a workspace's card: what is open in it
pub struct Card {
    pub workspace: Workspace,
    pub front: bool,
    /// the names of the items open in it, the one it shows first
    pub open: Vec<String>,
}

/// what the dashboard shows this frame
pub struct Dash<'a> {
    /// nothing open yet, nothing ever done
    pub welcome: bool,
    pub cards: &'a [Card],
    pub recent: &'a [Line],
    pub library: &'a [Line],
    pub library_why: Option<&'a str>,
    /// the menu's lines and whether each can be done now
    pub menu: &'a [(Action, bool)],
    /// the workspace in front and its views: id, name, icon, state
    pub workspace: Option<Workspace>,
    pub views: &'a [(&'static str, &'static str, &'static str, ViewState)],
    /// the bottom line: the account in words, the load in words
    pub account: &'a str,
    pub load: &'a str,
}

const NEW: [(&str, home::New); 3] = [("New scene", home::New::Scene), ("New module", home::New::Module), ("New show", home::New::Show)];

/// a line you press: its icon, its words, hover
fn line(d: &mut DrawList, t: &Tokens, r: Rect, icon: &'static str, words: &str, enabled: bool, pointer: (f32, f32)) {
    if enabled && r.has(pointer.0, pointer.1) {
        d.round_rect(r.x, r.y, r.w, r.h, r.h * 0.5, t.color.hover);
    }
    let (px, pad) = (t.size.font, t.size.pad);
    let icon_px = t.size.rem(1.0);
    let ink = if enabled { t.color.text } else { t.color.muted };
    d.icon(r.x + pad * 0.5, r.y + (r.h - icon_px) * 0.5, icon_px, icon, if enabled { t.color.muted } else { ink });
    let tx = r.x + pad + icon_px + pad * 0.4;
    d.sans(tx, r.y + (r.h - px * 1.25) * 0.5, px, &controls::fit_sans(d, px, words, (r.x + r.w - tx - pad * 0.5).max(px)), ink);
}

/// the dashboard over a window `size` under the toolbar ending at `top`; what each place does
pub fn draw(d: &mut DrawList, t: &Tokens, (w, h): (f32, f32), top: f32, dash: &Dash, pointer: (f32, f32)) -> Vec<(Rect, Press)> {
    let (pad, row, gap) = (t.size.pad, t.size.row, t.size.gap);
    let bottom_room = deck_kit::editor::status::reserve(t) + gap;
    let card_w = (w - 2.0 * t.size.rem(2.0)).min(t.size.rem(72.0)).max(t.size.rem(24.0));
    let card_h = (h - top - bottom_room - 2.0 * gap).max(row * 10.0);
    let card = Rect { x: (w - card_w) * 0.5, y: top + gap, w: card_w, h: card_h };
    // over a workspace it covers what lies there: the stage's own ground, a hairline around it
    let radius = t.size.radius * 2.0;
    d.round_rect(card.x, card.y, card.w, card.h, radius, t.color.stage);
    d.outline(card.x, card.y, card.w, card.h, (radius, 1.0), t.color.line);
    let mut out = Vec::new();
    let inner = t.size.rem(1.5);
    let (x0, mut y) = (card.x + inner, card.y + inner);
    let room_w = card.w - 2.0 * inner;
    let title = t.size.title;
    d.sans(x0, y, title, if dash.welcome { "Nothing open yet" } else { "Dashboard" }, t.color.text);
    let sub = if dash.welcome { "Open anything: start something new, or pick a workspace below. From then on the editor always resumes as you left it." } else { "The workspaces, anything to open, the menu and the views. Esc goes back." };
    d.sans(x0, y + title * 1.4, t.size.small, &controls::fit_sans(d, t.size.small, sub, room_w), t.color.muted);
    y += title * 1.4 + t.size.small * 1.4 + inner * 0.8;
    // the workspaces, flat, the front one first
    controls::label(d, t, x0, y, if dash.welcome { "Workspaces" } else { "Workspaces · most used first" }, t.color.muted);
    y += t.size.label * 1.8;
    let n = dash.cards.len().max(1) as f32;
    let tile_w = (room_w - (n - 1.0) * gap) / n;
    let tile_h = row * 2.6;
    for (i, c) in dash.cards.iter().enumerate() {
        let r = Rect { x: x0 + i as f32 * (tile_w + gap), y, w: tile_w, h: tile_h };
        let tr = t.size.radius * 2.0;
        if r.has(pointer.0, pointer.1) {
            d.round_rect(r.x, r.y, r.w, r.h, tr, t.color.hover);
        }
        if c.front {
            controls::active_tab(d, t, r, tr);
        } else {
            d.outline(r.x, r.y, r.w, r.h, (tr, 1.0), t.color.line);
        }
        let icon = t.size.rem(1.0);
        d.icon(r.x + pad, r.y + pad, icon, c.workspace.icon(), if c.front { t.color.text } else { t.color.muted });
        d.sans(r.x + pad + icon + pad * 0.6, r.y + pad, t.size.font, c.workspace.word(), t.color.text);
        let what = if c.open.is_empty() { "nothing open yet".to_string() } else { c.open.join(" · ") };
        let ink = if c.open.is_empty() { t.color.muted } else { t.color.text };
        d.sans(r.x + pad, r.y + r.h - pad - t.size.small * 1.25, t.size.small, &controls::fit_sans(d, t.size.small, &what, r.w - 2.0 * pad), ink);
        out.push((r, Press::Workspace(c.workspace)));
    }
    y += tile_h + inner;
    d.line((card.x, y), (card.x + card.w, y), 0.5, t.color.line);
    y += inner * 0.8;
    // three columns: open anything · the menu · the views
    let foot = card.y + card.h - inner - row;
    let col_gap = inner;
    let col_w = (room_w - 2.0 * col_gap) / 3.0;
    let cols = [x0, x0 + col_w + col_gap, x0 + 2.0 * (col_w + col_gap)];
    let views_head = dash.workspace.map_or_else(|| "Views".to_string(), |ws| format!("Views · {}", ws.word()));
    for (c, head) in ["Open anything", "Menu", views_head.as_str()].iter().enumerate() {
        controls::label(d, t, cols[c], y, head, t.color.muted);
    }
    let top_y = y + t.size.label * 1.8;
    let list_bottom = foot - pad;
    // open anything: New as pills, then Recent and the library
    let names: Vec<&str> = NEW.iter().map(|(n, _)| *n).collect();
    let mut nx = cols[0];
    let mut ny = top_y;
    for (name, (_, what)) in names.iter().zip(NEW) {
        let pw = d.sans_width(t.size.small, name) + 2.0 * pad;
        if nx + pw > cols[0] + col_w {
            nx = cols[0];
            ny += t.size.top + pad * 0.5;
        }
        let r = Rect { x: nx, y: ny, w: pw, h: t.size.top * 0.85 };
        if r.has(pointer.0, pointer.1) {
            d.round_rect(r.x, r.y, r.w, r.h, r.h * 0.5, t.color.hover);
        }
        d.outline(r.x, r.y, r.w, r.h, (r.h * 0.5, 1.0), t.color.line);
        d.sans(r.x + pad, r.y + (r.h - t.size.small * 1.25) * 0.5, t.size.small, name, t.color.text);
        out.push((r, Press::Home(home::Press::New(what))));
        nx += pw + pad * 0.5;
    }
    let mut ly = ny + t.size.top + pad;
    for (i, l) in dash.recent.iter().enumerate() {
        if ly + row > list_bottom {
            break;
        }
        let r = Rect { x: cols[0], y: ly, w: col_w, h: row };
        line(d, t, r, crate::tabs::Kind::of(&l.key).map_or("entry", crate::tabs::Kind::icon), &l.name, true, pointer);
        out.push((r, Press::Home(home::Press::Recent(i))));
        ly += row;
    }
    if !dash.library.is_empty() && ly + 2.0 * row <= list_bottom {
        ly += pad * 0.5;
        controls::label(d, t, cols[0], ly + pad * 0.5, "Library", t.color.muted);
        ly += t.size.label * 1.8;
    }
    for (i, l) in dash.library.iter().enumerate() {
        if ly + row > list_bottom {
            break;
        }
        let r = Rect { x: cols[0], y: ly, w: col_w, h: row };
        line(d, t, r, crate::tabs::Kind::of(&l.key).map_or("entry", crate::tabs::Kind::icon), &l.name, true, pointer);
        out.push((r, Press::Home(home::Press::Library(i))));
        ly += row;
    }
    if let (true, true, Some(why)) = (dash.library.is_empty(), dash.recent.is_empty(), dash.library_why) {
        let cols_n = (col_w / d.advance(t.size.small)).max(8.0) as usize;
        for (k, l) in crate::empty::wrap(why, cols_n).iter().enumerate() {
            d.sans(cols[0], ly + pad * 0.5 + k as f32 * t.size.small * 1.4, t.size.small, l, t.color.muted);
        }
    }
    // the menu
    let mut my = top_y;
    for (a, enabled) in dash.menu {
        let label = bar::MENU.iter().find(|(m, ..)| m == a).map_or("", |(_, l, _)| *l);
        let icon = match a {
            Action::Open => "folder",
            Action::New => "plus",
            Action::Publish => "upload-cloud",
            Action::Settings => "settings",
            Action::Audio => "activity",
            Action::Account => "account",
            Action::Quit => "monitor",
        };
        let r = Rect { x: cols[1], y: my, w: col_w, h: row };
        line(d, t, r, icon, label, *enabled, pointer);
        out.push((r, Press::Menu(*a)));
        my += row;
    }
    // the views of the workspace in front, each with its four states (the active one outlined)
    let mut vy = top_y;
    if dash.views.is_empty() {
        d.sans(cols[2], vy + pad * 0.5, t.size.small, "Open something: its workspace's views show here.", t.color.muted);
    }
    let words: Vec<&str> = VIEW_STATES.iter().map(|s| s.word()).collect();
    let states_w: f32 = words.iter().map(|w| d.sans_width(t.size.small, w) + 2.0 * pad + 2.0).sum();
    for &(id, name, icon, state) in dash.views {
        if vy + row > list_bottom {
            break;
        }
        let name_w = (col_w - states_w - pad).max(t.size.rem(4.0));
        line(d, t, Rect { x: cols[2], y: vy, w: name_w, h: row }, icon, name, false, pointer);
        let on = VIEW_STATES.iter().position(|s| *s == state).unwrap_or(usize::MAX);
        let ph = row - t.size.rem(0.4);
        let places = controls::pills_rounded(d, t, (cols[2] + col_w - states_w, vy + (row - ph) * 0.5, ph), &words, on, ph * 0.5);
        for (r, s) in places.into_iter().zip(VIEW_STATES) {
            out.push((r, Press::View(id, s)));
        }
        vy += row;
    }
    // the bottom line: the account, the help, the build, the load
    d.line((card.x, foot - pad * 0.5), (card.x + card.w, foot - pad * 0.5), 0.5, t.color.line);
    let small = t.size.small;
    let fy = foot + (row - small * 1.25) * 0.5;
    let mut fx = x0;
    fx += d.sans(fx, fy, small, dash.account, t.color.text) + pad * 1.5;
    let help = format!("Help: {}", home::DOCS.trim_start_matches("https://"));
    let hw = d.sans_width(small, &help);
    let hr = Rect { x: fx - pad * 0.5, y: foot, w: hw + pad, h: row };
    if hr.has(pointer.0, pointer.1) {
        d.round_rect(hr.x, hr.y, hr.w, hr.h, row * 0.5, t.color.hover);
    }
    d.sans(fx, fy, small, &help, t.color.text);
    out.push((hr, Press::Home(home::Press::Docs)));
    fx += hw + pad * 1.5;
    fx += d.text(fx, fy, small, deck_native::BUILD, t.color.muted) + pad * 1.5;
    let lw = d.sans_width(small, dash.load).min(card.x + card.w - inner - fx);
    let lx = card.x + card.w - inner - lw;
    d.sans(lx, fy, small, &controls::fit_sans(d, small, dash.load, lw.max(small)), t.color.muted);
    out.push((Rect { x: lx, y: foot, w: lw, h: row }, Press::Load));
    // the whole card keeps the press from the stage
    out.push((card, Press::Card));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_ui::workspaces::ALL;

    fn cards(open: bool) -> Vec<Card> {
        ALL.iter().map(|w| Card { workspace: *w, front: open && *w == Workspace::Scene, open: if open && *w == Workspace::Scene { vec!["nave".into()] } else { Vec::new() } }).collect()
    }

    fn dash<'a>(cards: &'a [Card], welcome: bool, views: &'a [(&'static str, &'static str, &'static str, ViewState)], menu: &'a [(Action, bool)]) -> Dash<'a> {
        Dash { welcome, cards, recent: &[], library: &[], library_why: None, menu, workspace: (!views.is_empty()).then_some(Workspace::Scene), views, account: "signed in", load: "CPU 7 % · 48 kHz · GPU 3 % · 60 fps" }
    }

    #[test]
    fn the_very_first_open_says_nothing_open_yet_with_the_four_workspaces_empty() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let menu: Vec<(Action, bool)> = bar::MENU.iter().map(|(a, ..)| (*a, true)).collect();
        let c = cards(false);
        let places = draw(&mut d, &t, (1600.0, 900.0), 60.0, &dash(&c, true, &[], &menu), (0.0, 0.0));
        let texts = d.texts();
        assert!(texts.contains(&"Nothing open yet"), "{texts:?}");
        assert_eq!(texts.iter().filter(|s| **s == "nothing open yet").count(), 3, "every workspace empty");
        assert!(!texts.contains(&"XR"), "XR is the creator's");
        for want in ["Scene", "Rigging", "Show", "New scene", deck_native::BUILD] {
            assert!(texts.contains(&want), "{want}");
        }
        assert_eq!(places.iter().filter(|(_, p)| matches!(p, Press::Workspace(_))).count(), 3);
        assert_eq!(places.iter().filter(|(_, p)| matches!(p, Press::Menu(_))).count(), bar::MENU.len());
        assert!(places.iter().all(|(r, _)| r.y > 60.0), "under the toolbar");
    }

    #[test]
    fn the_views_carry_their_four_states_the_front_card_outlined() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let views = [("mixer", "Mixer", "sliders", ViewState::Stage), ("info", "Info", "info", ViewState::Hidden)];
        let c = cards(true);
        let places = draw(&mut d, &t, (1920.0, 1080.0), 60.0, &dash(&c, false, &views, &[]), (0.0, 0.0));
        assert!(d.texts().contains(&"Dashboard") && d.texts().contains(&"nave"));
        let states: Vec<Press> = places.iter().map(|(_, p)| *p).filter(|p| matches!(p, Press::View(..))).collect();
        assert_eq!(states.len(), 8, "four states each");
        assert!(states.contains(&Press::View("mixer", ViewState::Full)));
        use deck_kit::base::draw::Prim;
        assert!(!d.prims().iter().any(|q| matches!(*q, Prim::Rect { color, .. } if color == t.color.pressed)), "no pressed fill");
    }
}
