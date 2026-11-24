//! The ☰ menu (the creator's main menu, picture A2's deck side): a popover under the bar's ☰ at the right. Views (with
//! the four states of every view of the workspace in front, only here – RULES H16) and Save workspace · Search; Open ·
//! New … · Settings · Audio & inputs · Account ↗ · Quit to stage · Docs ↗; at its foot the load and the build.
use crate::bar::Action;
use crate::home;
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls;
use deck_ui::stage::{ViewState, VIEW_STATES};

/// a line of the menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    Views,
    SaveWorkspace,
    Search,
    Do(Action),
    Docs,
}

/// the menu top down; `None` = a divider
pub const LINES: [Option<(Line, &str, &str)>; 11] = [
    Some((Line::Views, "Views", "sidebar")),
    Some((Line::SaveWorkspace, "Save workspace", "layers")),
    Some((Line::Search, "Search", "search")),
    None,
    Some((Line::Do(Action::Open), "Open …", "folder")),
    Some((Line::Do(Action::New), "New …", "plus")),
    Some((Line::Do(Action::Settings), "Settings", "settings")),
    Some((Line::Do(Action::Audio), "Audio & inputs", "activity")),
    Some((Line::Do(Action::Account), "Account", "account")),
    Some((Line::Do(Action::Quit), "Quit to stage", "monitor")),
    Some((Line::Docs, "Docs", "book")),
];

/// what a line says at its right: a key, or that it leaves for the browser
fn trail(l: Line) -> &'static str {
    match l {
        Line::Do(Action::Quit) => "F9",
        _ => "",
    }
}

/// what New … makes; TODO(#870): a scene and a show as dialogs on the overlay level, not today's sheets
pub const NEW: [(home::New, &str); 3] = [(home::New::Scene, "Scene"), (home::New::Show, "Show"), (home::New::Module, "Module")];

/// a press on the menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Line(Line),
    New(home::New),
    View(&'static str, ViewState),
    /// the menu's ground: nothing, the menu stays
    Ground,
}

/// the part open beside the menu
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Side {
    #[default]
    None,
    Views,
    New,
}

/// what the menu shows
pub struct Menu<'a> {
    /// whether each line can be done now (a line missing: it can)
    pub disabled: &'a [Line],
    pub side: Side,
    /// the views of the workspace in front: id, name, icon, state
    pub views: &'a [(&'static str, &'static str, &'static str, ViewState)],
    /// the foot: the load, the build
    pub load: &'a str,
    pub build: &'a str,
}

/// the menu under `at` (the ☰'s place), its right edge on the ☰'s; what each place does
pub fn draw(d: &mut DrawList, t: &Tokens, at: Rect, m: &Menu, pointer: (f32, f32)) -> Vec<(Rect, Press)> {
    let (row, pad, px) = (t.size.row, t.size.pad, t.size.font);
    let w = t.size.rem(17.5);
    let lines = &LINES;
    let dividers = lines.iter().filter(|l| l.is_none()).count() as f32 + 1.0;
    let foot = t.size.small * 1.5 * 2.0 + pad;
    let h = pad + lines.iter().filter(|l| l.is_some()).count() as f32 * row + dividers * pad * 1.4 + foot + pad;
    let r = Rect { x: (at.x + at.w - w).max(0.0), y: at.y + at.h + t.size.rem(0.43), w, h };
    let mut out = Vec::new();
    let halo = controls::popover(d, t, r, t.size.radius * 2.0);
    let mut y = r.y + pad * 0.5;
    for l in lines {
        let Some((line, word, icon)) = *l else {
            d.line((r.x, y + pad * 0.7), (r.x + r.w, y + pad * 0.7), 0.5, t.color.line);
            y += pad * 1.4;
            continue;
        };
        let place = Rect { x: r.x + pad * 0.5, y, w: r.w - pad, h: row };
        let enabled = !m.disabled.contains(&line);
        let open = matches!((line, m.side), (Line::Views, Side::Views) | (Line::Do(Action::New), Side::New));
        let state = if !enabled { "disabled" } else if open || place.has(pointer.0, pointer.1) { "hover" } else { "rest" };
        let s = t.kit("menu_item", state);
        if let Some(g) = s.ground.filter(|_| state == "hover") {
            d.round_rect(place.x, place.y, place.w, place.h, row * 0.5, g);
        }
        let ink = s.text.unwrap_or(if enabled { t.color.text } else { t.color.muted });
        let isz = t.size.rem(1.0);
        d.icon(place.x + pad, y + (row - isz) * 0.5, isz, icon, if enabled { t.color.muted } else { ink });
        let tx = place.x + pad * 2.0 + isz;
        let used = d.sans(tx, y + (row - px * 1.25) * 0.5, px, word, ink);
        let external = matches!(line, Line::Do(Action::Account) | Line::Docs);
        if external {
            let s = px * 0.75;
            controls::arrow_out(d, (tx + used + pad * 0.5, y + (row - s) * 0.5 - s * 0.2), s, ink);
        }
        let right = place.x + place.w - pad;
        let k = trail(line);
        if !k.is_empty() {
            let kw = d.sans_width(t.size.small, k);
            d.sans(right - kw, y + (row - t.size.small * 1.25) * 0.5, t.size.small, k, t.color.muted);
        }
        if matches!(line, Line::Views | Line::Do(Action::New)) {
            let s = t.size.rem(0.8);
            d.icon(right - s, y + (row - s) * 0.5, s, if open { "previous" } else { "next" }, t.color.muted);
        }
        out.push((place, Press::Line(line)));
        y += row;
    }
    // the foot: the load and the build, as the bar had them
    d.line((r.x, y + pad * 0.5), (r.x + r.w, y + pad * 0.5), 0.5, t.color.line);
    y += pad;
    let small = t.size.small;
    d.text(r.x + pad * 1.5, y + small * 0.25, small, &controls::fit(m.load, ((r.w - 3.0 * pad) / d.advance(small)) as usize), t.color.muted);
    d.text(r.x + pad * 1.5, y + small * 1.75, small, m.build, t.color.muted);
    d.restore_halo(halo);
    out.push((r, Press::Ground));
    // what is open beside it takes the press first
    let beside = match m.side {
        Side::Views => views(d, t, r, m.views, pointer),
        Side::New => new(d, t, r, pointer),
        Side::None => Vec::new(),
    };
    beside.into_iter().chain(out).collect()
}

/// the views of the workspace in front beside the menu, at its left: each with its four states, the one it is in lit
fn views(d: &mut DrawList, t: &Tokens, menu: Rect, views: &[(&'static str, &'static str, &'static str, ViewState)], pointer: (f32, f32)) -> Vec<(Rect, Press)> {
    let (row, pad, px) = (t.size.row, t.size.pad, t.size.font);
    let w = t.size.rem(21.0);
    let head = t.size.label * 2.4;
    let h = pad + head + views.len().max(1) as f32 * row + pad;
    let r = Rect { x: (menu.x - w - t.size.rem(0.29)).max(0.0), y: menu.y, w, h };
    let halo = controls::popover(d, t, r, t.size.radius * 2.0);
    controls::label(d, t, r.x + pad * 1.5, r.y + pad + t.size.label * 0.3, "Every view", t.color.muted);
    let mut out = Vec::new();
    let b = row - t.size.rem(0.4);
    let states_w = VIEW_STATES.len() as f32 * (b + 2.0);
    let mut y = r.y + pad + head;
    for &(id, name, icon, state) in views {
        let isz = t.size.rem(1.0);
        d.icon(r.x + pad * 1.5, y + (row - isz) * 0.5, isz, icon, t.color.muted);
        let tx = r.x + pad * 2.5 + isz;
        let room = r.x + r.w - pad - states_w - tx - pad;
        d.sans(tx, y + (row - px * 1.25) * 0.5, px, &controls::fit_sans(d, px, name, room.max(px)), t.color.text);
        let mut bx = r.x + r.w - pad - states_w;
        for s in VIEW_STATES {
            let place = Rect { x: bx, y: y + (row - b) * 0.5, w: b, h: b };
            let kit = if s == state { "on" } else if place.has(pointer.0, pointer.1) { "hover" } else { "rest" };
            controls::icon_button_in(d, t, place, s.icon(), kit);
            out.push((place, Press::View(id, s)));
            bx += b + 2.0;
        }
        y += row;
    }
    d.restore_halo(halo);
    out.push((r, Press::Ground));
    out
}

/// New …'s three beside the menu, level with its line
fn new(d: &mut DrawList, t: &Tokens, menu: Rect, pointer: (f32, f32)) -> Vec<(Rect, Press)> {
    let (row, pad) = (t.size.row, t.size.pad);
    let w = t.size.rem(11.0);
    let at_line = LINES.iter().position(|l| matches!(l, Some((Line::Do(Action::New), ..)))).unwrap_or(0);
    let dividers = LINES[..at_line].iter().filter(|l| l.is_none()).count() as f32;
    let y = menu.y + pad * 0.5 + (at_line as f32 - dividers) * row + dividers * pad * 1.4 - pad * 0.5;
    let names: Vec<controls::Action> = NEW.iter().map(|(_, n)| controls::Action { label: n, enabled: true, external: false }).collect();
    let places = controls::actions(d, t, ((menu.x - w - t.size.rem(0.29)).max(0.0), y), w, &names, pointer);
    places.into_iter().zip(NEW).map(|(r, (what, _))| (r, Press::New(what))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu<'a>(side: Side, views: &'a [(&'static str, &'static str, &'static str, ViewState)]) -> Menu<'a> {
        Menu { disabled: &[Line::Do(Action::Quit)], side, views, load: "CPU 7 % · 48 kHz · GPU 3 % · 60 fps", build: "0.1.0.r4781" }
    }

    const ME: Rect = Rect { x: 1550.0, y: 8.0, w: 36.0, h: 36.0 };

    #[test]
    fn the_menu_has_views_and_its_lines_then_the_load_and_the_build() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let places = draw(&mut d, &t, ME, &menu(Side::None, &[]), (0.0, 0.0));
        let texts = d.texts();
        for want in ["Views", "Save workspace", "Search", "Open …", "New …", "Settings", "Audio & inputs", "Account", "Quit to stage", "F9", "Docs", "0.1.0.r4781"] {
            assert!(texts.contains(&want), "{want}: {texts:?}");
        }
        assert!(texts.iter().any(|s| s.contains("48 kHz")));
        assert!(!texts.iter().any(|s| s.contains("Publish")), "Publish is the bar's");
        let lines: Vec<Line> = places.iter().filter_map(|(_, p)| if let Press::Line(l) = p { Some(*l) } else { None }).collect();
        assert_eq!(lines.len(), 10);
        let ground = places.iter().find(|(_, p)| *p == Press::Ground).unwrap().0;
        assert!((ground.x + ground.w - (ME.x + ME.w)).abs() < 0.01 && ground.y > ME.y + ME.h, "under the ☰, its right edge on it");
    }

    #[test]
    fn views_open_beside_it_with_four_states_each_the_one_on_lit() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let views = [("rows", "Rows", "list", ViewState::Panel), ("cues", "Cues", "list", ViewState::Hidden)];
        let places = draw(&mut d, &t, ME, &menu(Side::Views, &views), (0.0, 0.0));
        let states: Vec<Press> = places.iter().map(|(_, p)| *p).filter(|p| matches!(p, Press::View(..))).collect();
        assert_eq!(states.len(), 8);
        assert!(states.contains(&Press::View("cues", ViewState::Full)));
        assert!(d.texts().contains(&"EVERY VIEW") && d.texts().contains(&"Cues"));
        let first_view = places.iter().find(|(_, p)| matches!(p, Press::View(..))).unwrap().0;
        assert!(first_view.x < ME.x - t.size.rem(17.5), "at the menu's left");
    }

    #[test]
    fn new_offers_a_scene_a_show_and_a_module() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let places = draw(&mut d, &t, ME, &menu(Side::New, &[]), (0.0, 0.0));
        let made: Vec<Press> = places.iter().map(|(_, p)| *p).filter(|p| matches!(p, Press::New(_))).collect();
        assert_eq!(made, [Press::New(home::New::Scene), Press::New(home::New::Show), Press::New(home::New::Module)]);
    }
}
