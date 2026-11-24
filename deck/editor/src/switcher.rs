//! The switcher (RULES H15, the creator's switcher – picture D1): over the workspace on the overlay level, no bar, no
//! search field, no foot. What was last opened, then one list of workspaces – the deck's cards Scene · Rigging · Show,
//! the front one framed, then the creator's with a quiet chip "Creator". Home is the same page on the stage while nothing
//! is open.
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls;
use deck_ui::workspaces::Workspace;

/// a press on the switcher
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// a line of Last opened
    Recent(usize),
    Workspace(Workspace),
    /// a creator's card, by its place in the list
    Creator(usize),
}

/// a line of Last opened: its address (its kind's icon), its name, its workspace
pub struct Recent {
    pub key: String,
    pub name: String,
    pub workspace: Option<Workspace>,
}

/// a deck card: its workspace, whether it is in front, the item it shows first
pub struct Card {
    pub workspace: Workspace,
    pub front: bool,
    pub item: Option<String>,
}

/// a creator's card: its workspace's word and the item it shows (creator_cards.rs)
pub struct CreatorCard {
    pub name: String,
    pub item: Option<String>,
}

/// what the switcher shows
pub struct Switch<'a> {
    pub recent: &'a [Recent],
    pub cards: &'a [Card],
    pub creator: &'a [CreatorCard],
}

/// the widest the content gets (1160 px at the kit's base)
const MAX_W: f32 = 82.86;
/// cards per row
const PER_ROW: usize = 6;
/// lines of Last opened at most
const RECENT: usize = 5;

/// the switcher over a window `(w, h)`, its content from `top` down; what each place does
pub fn draw(d: &mut DrawList, t: &Tokens, (w, h): (f32, f32), top: f32, s: &Switch, pointer: (f32, f32)) -> Vec<(Rect, Press)> {
    let (pad, gap, row) = (t.size.pad, t.size.gap, t.size.row);
    let cw = (w - 2.0 * t.size.rem(2.0)).min(t.size.rem(MAX_W)).max(t.size.rem(20.0));
    let x0 = (w - cw) * 0.5;
    let mut y = top + t.size.rem(2.5);
    let mut out = Vec::new();
    d.halo(t.color.ring());
    // what was last opened, in a box half the content wide
    if !s.recent.is_empty() {
        let n = s.recent.len().min(RECENT);
        let bw = (cw * 0.5 - gap * 0.5).max(t.size.rem(22.0)).min(cw);
        let inner = t.size.rem(0.8);
        let bh = inner * 2.0 + t.size.label * 1.8 + n as f32 * row;
        let b = Rect { x: x0, y, w: bw, h: bh };
        let s_box = t.kit("panel", "rest");
        d.round_rect(b.x, b.y, b.w, b.h, t.size.radius * 2.0, s_box.ground.unwrap_or(t.color.panel()));
        d.outline(b.x, b.y, b.w, b.h, (t.size.radius * 2.0, t.size.edge), s_box.border.unwrap_or(t.color.line));
        controls::label(d, t, b.x + inner, b.y + inner, "Last opened", t.color.muted);
        let mut ly = b.y + inner + t.size.label * 1.8;
        for (i, r) in s.recent.iter().take(n).enumerate() {
            let line = Rect { x: b.x + inner * 0.5, y: ly, w: b.w - inner, h: row };
            if line.has(pointer.0, pointer.1) {
                d.round_rect(line.x, line.y, line.w, line.h, row * 0.5, t.kit("menu_item", "hover").ground.unwrap_or(t.color.hover));
            }
            let icon = t.size.rem(1.0);
            let kind = crate::tabs::Kind::of(&r.key);
            d.icon(line.x + pad, ly + (row - icon) * 0.5, icon, kind.map_or("entry", crate::tabs::Kind::icon), t.color.muted);
            let px = t.size.font;
            let what = r.workspace.map_or("", Workspace::word);
            let ww = d.sans_width(t.size.small, what);
            let tx = line.x + pad * 2.0 + icon;
            let name = controls::fit_sans(d, px, &r.name, (line.x + line.w - pad * 2.0 - ww - tx).max(px));
            d.sans(tx, ly + (row - px * 1.25) * 0.5, px, &name, t.color.text);
            d.sans(line.x + line.w - pad - ww, ly + (row - t.size.small * 1.25) * 0.5, t.size.small, what, t.color.muted);
            out.push((line, Press::Recent(i)));
            ly += row;
        }
        y += bh + t.size.rem(1.8);
    }
    // the workspaces: the deck's, then the creator's, one list
    controls::label(d, t, x0, y, "Workspaces", t.color.muted);
    y += t.size.label * 2.2;
    let card_w = (cw - (PER_ROW - 1) as f32 * gap) / PER_ROW as f32;
    let card_h = card_w * 0.84;
    let all = s.cards.len() + s.creator.len();
    for i in 0..all {
        let r = Rect { x: x0 + (i % PER_ROW) as f32 * (card_w + gap), y: y + (i / PER_ROW) as f32 * (card_h + gap), w: card_w, h: card_h };
        if r.y + r.h > h {
            break;
        }
        let (title, item, icon, front, creator) = match s.cards.get(i) {
            Some(c) => (c.workspace.word(), c.item.as_deref(), c.workspace.icon(), c.front, false),
            None => {
                let c = &s.creator[i - s.cards.len()];
                (c.name.as_str(), c.item.as_deref(), "external", false, true)
            }
        };
        card(d, t, r, (title, item, icon), (front, creator), pointer);
        out.push((r, if creator { Press::Creator(i - s.cards.len()) } else { Press::Workspace(s.cards[i].workspace) }));
    }
    d.halo(None);
    out
}

/// a workspace's card: the kit's `workspace_card` (the front one framed in the text colour, "Front" over it), its icon
/// where a still will be, its name and what it shows, a creator's chip
fn card(d: &mut DrawList, t: &Tokens, r: Rect, (title, item, icon): (&str, Option<&str>, &'static str), (front, creator): (bool, bool), pointer: (f32, f32)) {
    let radius = t.size.radius * 2.0;
    let hover = r.has(pointer.0, pointer.1);
    let rest = t.kit("workspace_card", "rest");
    let s = t.kit("workspace_card", if front { "on" } else if hover { "hover" } else { "rest" });
    if let Some(g) = s.ground.or_else(|| hover.then(|| t.kit("workspace_card", "hover").ground).flatten()) {
        d.round_rect(r.x, r.y, r.w, r.h, radius, g);
    }
    let thick = if front { 2.0 * t.size.edge } else { t.size.edge };
    d.outline(r.x, r.y, r.w, r.h, (radius, thick), s.border.or(rest.border).unwrap_or(t.color.line));
    let pad = t.size.pad * 1.4;
    if front {
        controls::label(d, t, r.x + pad, r.y + pad, "Front", t.color.text);
    }
    let size = t.size.rem(1.6);
    d.icon(r.x + (r.w - size) * 0.5, r.y + r.h * 0.36 - size * 0.5, size, icon, t.color.muted);
    let (big, small) = (t.size.title * 1.15, t.size.small);
    let sub_y = r.y + r.h - pad - small * 1.25;
    let title_y = sub_y - big * 1.35;
    d.sans(r.x + pad, title_y, big, &controls::fit_sans(d, big, title, r.w - 2.0 * pad), t.color.text);
    let chip_w = if creator { d.sans_width(t.size.label, "Creator") + t.size.pad * 2.5 } else { 0.0 };
    if creator {
        let ch = controls::badge_height(t);
        controls::chip_in(d, t, (r.x + r.w - pad - chip_w, sub_y + small * 0.62 - ch * 0.5), "Creator", "rest");
    }
    let (words, ink) = match item {
        Some(i) => (i, t.color.text),
        None => ("Nothing open yet", t.color.muted),
    };
    let room = (r.w - 2.0 * pad - chip_w - if creator { t.size.pad } else { 0.0 }).max(small);
    d.sans(r.x + pad, sub_y, small, &controls::fit_sans(d, small, words, room), ink);
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_kit::base::draw::Prim;
    use deck_ui::workspaces::ALL;

    fn cards(front: Option<Workspace>) -> Vec<Card> {
        ALL.iter().map(|w| Card { workspace: *w, front: front == Some(*w), item: (front == Some(*w)).then(|| "scene:avatar-home".to_string()) }).collect()
    }

    #[test]
    fn the_switcher_lists_last_opened_then_the_decks_cards_then_the_creators_with_a_chip() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let recent = [Recent { key: "character:catpope".into(), name: "catpope".into(), workspace: Some(Workspace::Rigging) }];
        let c = cards(Some(Workspace::Scene));
        let creator = [CreatorCard { name: "Character".into(), item: Some("character:giantpopstar".into()) }, CreatorCard { name: "Toon".into(), item: None }];
        let places = draw(&mut d, &t, (1600.0, 900.0), 8.0, &Switch { recent: &recent, cards: &c, creator: &creator }, (0.0, 0.0));
        let texts = d.texts();
        for want in ["LAST OPENED", "catpope", "Rigging", "WORKSPACES", "Scene", "Show", "FRONT", "scene:avatar-home", "Character", "Toon", "Nothing open yet"] {
            assert!(texts.iter().any(|s| *s == want), "{want}: {texts:?}");
        }
        assert_eq!(texts.iter().filter(|s| **s == "Creator").count(), 2, "a chip on each creator's card");
        let kinds: Vec<Press> = places.iter().map(|(_, p)| *p).collect();
        assert_eq!(kinds, [Press::Recent(0), Press::Workspace(Workspace::Scene), Press::Workspace(Workspace::Rigging), Press::Workspace(Workspace::Show), Press::Creator(0), Press::Creator(1)]);
        assert!(!texts.iter().any(|s| s.contains("Needs review") || s.contains("Find")), "no Needs review box, no search field");
        let right = places.iter().map(|(r, _)| r.x + r.w).fold(0.0, f32::max);
        let left = places.iter().map(|(r, _)| r.x).fold(f32::MAX, f32::min);
        assert!(right - left <= t.size.rem(MAX_W) + 0.5, "at most 1160 px: {}", right - left);
        assert!(!d.prims().iter().any(|p| matches!(*p, Prim::Rect { color, .. } if color == t.color.pressed)), "no pressed fill");
    }

    #[test]
    fn nothing_opened_yet_shows_no_last_opened_box() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let c = cards(None);
        let places = draw(&mut d, &t, (1280.0, 800.0), 60.0, &Switch { recent: &[], cards: &c, creator: &[] }, (0.0, 0.0));
        assert!(!d.texts().contains(&"LAST OPENED"));
        assert_eq!(places.len(), 3);
        assert!(places.iter().all(|(r, _)| r.y > 60.0), "under the bar");
    }
}
