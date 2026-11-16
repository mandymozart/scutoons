//! The toolbar over the stage: pills in a row, nothing under them but the stage. Left the logo with the main menu's
//! hamburger, the workspaces, the path of what is open (its steps go back up) with a word of status; the tempo pill
//! (the play button, the clock's BPM); right the sign-in, the load meters, day / night and the version, which keeps its
//! place at any width (every build shows it, always).
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};

/// the logo's side
const LOGO: f32 = 22.0;
/// the pills' distance from the window's top
const MARGIN: f32 = 8.0;

/// where the toolbar ends: the panels start a gap under it
pub fn bottom(t: &Tokens) -> f32 {
    MARGIN + t.size.top
}

/// a main menu's line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    New,
    Publish,
    DayNight,
    Audio,
    Account,
    Quit,
}

/// the main menu, top down: what each line says, whether it leaves for the browser (no save: everything saves itself)
pub const MENU: [(Action, &str, bool); 7] = [
    (Action::Open, "Open…", false),
    (Action::New, "New", false),
    (Action::Publish, "Publish in the creator", true),
    (Action::DayNight, "Day / night", false),
    (Action::Audio, "Audio & inputs", false),
    (Action::Account, "Account", true),
    (Action::Quit, "Quit to stage  F9", false),
];

/// what the toolbar shows
pub struct Bar<'a> {
    pub workspaces: &'a [&'a str],
    pub workspace: usize,
    /// the path of what is open: each step a pill that goes back to it
    pub path: &'a [String],
    pub status: &'a str,
    /// the play button (Build): true = it starts playing
    pub play: Option<bool>,
    /// the tempo clock's BPM and whether it runs
    pub beat: Option<(f64, bool)>,
    pub sign_in: &'a str,
    pub logo_frame: usize,
    pub pointer: (f32, f32),
    /// how much of a frame's time the CPU and the GPU work (0..1); the GPU where it can be measured
    pub cpu: f32,
    pub gpu: Option<f32>,
    /// the room a workspace keeps in the row for its own pill (Show's and Perform's view switch)
    pub slot: f32,
}

/// where the toolbar's parts landed
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Placed {
    pub menu: Rect,
    pub workspaces: Vec<Rect>,
    pub path: Vec<Rect>,
    pub play: Option<Rect>,
    /// the place kept for the workspace's own pill
    pub slot: Rect,
    pub day: Rect,
}

/// a pill's ground
fn pill(d: &mut DrawList, t: &Tokens, r: Rect) {
    d.round_rect(r.x, r.y, r.w, r.h, r.h * 0.5, t.color.surface);
    d.outline(r.x, r.y, r.w, r.h, (r.h * 0.5, 1.0), t.color.line);
}

/// the widths the row's pills take at `width`: the path pill gets what the others leave
#[derive(Debug, PartialEq)]
struct Row {
    left: f32,
    path_x: f32,
    path_w: f32,
    tempo_x: f32,
    slot_x: f32,
    right_x: f32,
}

fn row(width: f32, gap: f32, left: f32, tempo: f32, slot: f32, right: f32) -> Row {
    let right_x = (width - gap - right).max(0.0);
    let slot_x = right_x - gap - slot;
    let tempo_x = slot_x - if slot > 0.0 { gap } else { 0.0 } - tempo;
    let path_x = left + gap;
    Row { left, path_x, path_w: (tempo_x - gap - path_x).max(0.0), tempo_x, slot_x, right_x }
}

pub fn draw(d: &mut DrawList, t: &Tokens, width: f32, b: &Bar) -> Placed {
    let (h, pad, px, small) = (t.size.top, t.size.pad, t.size.font, t.size.small);
    let gap = 16.0;
    let y = MARGIN;
    let mid = y + h * 0.5;
    let adv = d.advance(px);
    let text_y = mid - px * 0.62;
    let mut out = Placed::default();
    // the logo and the hamburger
    let first = Rect { x: gap, y, w: pad + LOGO + pad + h, h };
    pill(d, t, first);
    d.logo(first.x + pad + 2.0, mid - LOGO * 0.5, LOGO, b.logo_frame);
    out.menu = controls::icon_button(d, t, Rect { x: first.x + first.w - h, y, w: h, h }, "menu", t.color.text, b.pointer);
    // the workspaces
    let ws_w: f32 = b.workspaces.iter().map(|w| d.advance(small) * w.chars().count() as f32 + 2.0 * pad + 2.0).sum::<f32>() + 2.0 * 4.0;
    let ws = Rect { x: first.x + first.w + gap * 0.5, y, w: ws_w, h };
    pill(d, t, ws);
    out.workspaces = controls::pills(d, t, ws.x + 4.0, y + 4.0, h - 8.0, b.workspaces, b.workspace);
    // the right pill: sign-in, meters, day / night, the version
    let version_w = adv * deck_native::BUILD.chars().count() as f32;
    let sign_w = (adv * b.sign_in.chars().count() as f32).min(160.0);
    let meters = 200.0;
    let lead = if b.play.is_some() { h + pad } else { 2.0 * pad };
    let tempo_w = lead + adv * 21.0 + pad;
    let right_of = |wide: bool| 2.0 * pad + if b.sign_in.is_empty() { 0.0 } else { sign_w + pad } + if wide { meters + pad } else { 0.0 } + h + pad + version_w;
    // the load meters only where the path keeps room of its own beside them
    let wide = row(width, gap * 0.5, ws.x + ws.w, tempo_w, b.slot, right_of(false)).path_w > meters + 240.0;
    let right_w = right_of(wide);
    let r = row(width, gap * 0.5, ws.x + ws.w, tempo_w, b.slot, right_w);
    // the path, then the status in what is left
    let path = Rect { x: r.path_x, y, w: r.path_w, h };
    if path.w > 2.0 * h {
        pill(d, t, path);
        // each step at most its share of the pill (the status gets what they leave)
        let share = ((path.w - 2.0 * pad) / b.path.len().max(1) as f32 - 2.0 * pad - 14.0) / d.advance(px);
        let steps: Vec<String> = b.path.iter().map(|s| controls::fit(s, share.max(3.0) as usize)).collect();
        let crumbs = controls::breadcrumb(d, t, path.x + pad, y + 4.0, &steps);
        let end = crumbs.last().map_or(path.x + pad, |c| c.x + c.w);
        out.path = crumbs.into_iter().filter(|c| c.x + c.w <= path.x + path.w).collect();
        let room = ((path.x + path.w - pad - end - pad) / d.advance(small)).max(0.0) as usize;
        d.text(end + pad, mid - small * 0.62, small, &controls::fit(b.status, room), t.color.muted);
    }
    // the tempo: the play button, the clock (its content is still to come)
    let tempo = Rect { x: r.tempo_x, y, w: tempo_w, h };
    pill(d, t, tempo);
    if let Some(wire) = b.play {
        let at = Rect { x: tempo.x + 2.0, y, w: h, h };
        out.play = Some(if wire { controls::icon_button(d, t, at, "play-fill", t.color.go, b.pointer) } else { controls::icon_button(d, t, at, "stop-fill", t.color.stop, b.pointer) });
    }
    let clock = match b.beat {
        Some((bpm, running)) => format!("{bpm:.1} BPM · {}", if running { "INTERNAL" } else { "STOPPED" }),
        None => "– BPM".to_string(),
    };
    d.text(tempo.x + lead, text_y, px, &controls::fit(&clock, 21), t.color.text);
    out.slot = Rect { x: r.slot_x, y, w: b.slot, h };
    // the right pill
    let right = Rect { x: r.right_x, y, w: right_w, h };
    pill(d, t, right);
    let mut x = right.x + pad;
    if !b.sign_in.is_empty() {
        x += d.text(x, mid - small * 0.62, small, &controls::fit(b.sign_in, (sign_w / d.advance(small)) as usize), t.color.muted) + pad;
        x = x.max(right.x + pad + sign_w + pad);
    }
    if wide {
        let mut mx = x;
        mx += controls::meter(d, t, mx, mid - small * 0.62, "CPU", b.cpu) + 2.0 * pad;
        if let Some(g) = b.gpu {
            controls::meter(d, t, mx, mid - small * 0.62, "GPU", g);
        }
        x += meters + pad;
    }
    out.day = Rect { x, y, w: h, h };
    d.icon(x + (h - 16.0) * 0.5, mid - 8.0, 16.0, "light", t.color.muted);
    d.text(right.x + right.w - pad - version_w, text_y, px, deck_native::BUILD, t.color.muted);
    out
}

/// the main menu under its hamburger; its lines' places
pub fn menu(d: &mut DrawList, t: &Tokens, at: Rect, enabled: &dyn Fn(Action) -> bool, pointer: (f32, f32)) -> Vec<(Rect, Action)> {
    let lines: Vec<controls::Action> = MENU.iter().map(|&(a, label, external)| controls::Action { label, enabled: enabled(a), external }).collect();
    let rects = controls::actions(d, t, (at.x, at.y + at.h + 6.0), 240.0, &lines, pointer);
    rects.into_iter().zip(MENU).map(|(r, (a, ..))| (r, a)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_pill_ends_at_the_right_edge_however_wide_the_window() {
        for width in [400.0, 1280.0, 3840.0] {
            let r = row(width, 8.0, 300.0, 200.0, 120.0, 260.0);
            assert_eq!(r.right_x + 260.0 + 8.0, width);
        }
    }

    #[test]
    fn the_path_takes_what_the_others_leave_and_never_reaches_them() {
        for width in [600.0, 1280.0, 1920.0] {
            let r = row(width, 8.0, 300.0, 200.0, 120.0, 260.0);
            assert!(r.path_w == 0.0 || r.path_x + r.path_w + 8.0 <= r.tempo_x, "{width}: {r:?}");
            assert!(r.tempo_x + 200.0 + 8.0 <= r.slot_x + 0.01, "the tempo before the slot");
            assert!(r.slot_x + 120.0 + 8.0 <= r.right_x + 0.01, "the slot before the right pill");
        }
        let without = row(1280.0, 8.0, 300.0, 200.0, 0.0, 260.0);
        let with = row(1280.0, 8.0, 300.0, 200.0, 120.0, 260.0);
        assert!(without.path_w > with.path_w, "no slot: the path is wider");
    }

    #[test]
    fn every_menu_line_lands_and_a_disabled_one_is_still_placed() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let lines = menu(&mut d, &t, Rect { x: 16.0, y: 8.0, w: 36.0, h: 36.0 }, &|a| a != Action::New, (0.0, 0.0));
        assert_eq!(lines.len(), MENU.len());
        assert!(lines.windows(2).all(|w| w[1].0.y > w[0].0.y));
        assert!(lines[0].0.y > 44.0, "under the hamburger");
    }
}
