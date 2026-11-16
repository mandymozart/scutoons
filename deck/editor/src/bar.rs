//! The toolbar over the stage: pills in a row, nothing under them but the stage. Left the logo with the main menu's
//! hamburger, the workspaces, the path of what is open (its steps go back up) with a word of status; the tempo pill
//! (the play button, the clock's BPM); right the sign-in, the load meters, day / night and the version, which keeps its
//! place at any width (every build shows it, always).
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};

/// the pills' distance from the window's top
fn margin(t: &Tokens) -> f32 {
    t.size.rem(0.57)
}

/// where the toolbar ends: the panels start a gap under it
pub fn bottom(t: &Tokens) -> f32 {
    margin(t) + t.size.top
}

/// a main menu's line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    New,
    Publish,
    DayNight,
    Settings,
    Audio,
    Account,
    Quit,
}

/// the main menu, top down: what each line says, whether it leaves for the browser (no save: everything saves itself)
pub const MENU: [(Action, &str, bool); 8] = [
    (Action::Open, "Open…", false),
    (Action::New, "New", false),
    (Action::Publish, "Publish in the creator", true),
    (Action::DayNight, "Day / night", false),
    (Action::Settings, "Settings…", false),
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
    /// the dark look is on
    pub night: bool,
    /// a panic holds: the pill resumes
    pub panicked: bool,
}

/// what the panic pill says: what a press does
pub fn panic_label(panicked: bool) -> &'static str {
    if panicked {
        "RESUME"
    } else {
        "PANIC"
    }
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
    /// the panic pill, in every workspace
    pub panic: Rect,
}

/// a pill's ground
fn pill(d: &mut DrawList, t: &Tokens, r: Rect) {
    let s = t.color.surface;
    d.round_rect(r.x, r.y, r.w, r.h, r.h * 0.5, [s[0], s[1], s[2], s[3] * t.color.panel_opacity]);
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

/// an icon you press inside a pill, inset from its edge: round, its corners sharing the pill's centre
fn inner_button(d: &mut DrawList, t: &Tokens, r: Rect, name: &'static str, color: [f32; 4], pointer: (f32, f32)) -> Rect {
    if r.has(pointer.0, pointer.1) {
        d.round_rect(r.x, r.y, r.w, r.h, r.h * 0.5, t.color.hover);
    }
    let size = t.size.rem(1.15);
    d.icon(r.x + (r.w - size) * 0.5, r.y + (r.h - size) * 0.5, size, name, color);
    r
}

pub fn draw(d: &mut DrawList, t: &Tokens, width: f32, b: &Bar) -> Placed {
    let (h, pad, px, small) = (t.size.top, t.size.pad, t.size.font, t.size.small);
    let gap = t.size.gap * 0.5;
    // what sits inside a pill is inset by `inset`, `ih` high, rounded by the pill's radius less the inset; text keeps
    // `end` from a pill's round ends
    let inset = t.size.rem(0.29);
    let (ih, ir) = (h - 2.0 * inset, h * 0.5 - inset);
    let end = pad + inset;
    let logo = t.size.rem(1.57);
    let y = margin(t);
    let mid = y + h * 0.5;
    let text_y = mid - px * 0.62;
    let small_y = mid - small * 0.62;
    let mut out = Placed::default();
    // the logo and the hamburger
    let first = Rect { x: t.size.gap, y, w: end + logo + pad + ih + inset, h };
    pill(d, t, first);
    d.logo(first.x + end, mid - logo * 0.5, logo, b.logo_frame);
    out.menu = inner_button(d, t, Rect { x: first.x + first.w - inset - ih, y: y + inset, w: ih, h: ih }, "menu", t.color.text, b.pointer);
    // the workspaces
    let ws_w: f32 = b.workspaces.iter().map(|w| d.sans_width(small, w) + 2.0 * pad).sum::<f32>() + 2.0 * b.workspaces.len().saturating_sub(1) as f32 + 2.0 * inset;
    let ws = Rect { x: first.x + first.w + gap, y, w: ws_w, h };
    pill(d, t, ws);
    out.workspaces = controls::pills_rounded(d, t, (ws.x + inset, y + inset, ih), b.workspaces, b.workspace, ir);
    // the right pill: sign-in, meters, day / night, the version
    let version_w = d.advance(px) * deck_native::BUILD.chars().count() as f32;
    let sign_w = d.sans_width(small, b.sign_in).min(t.size.rem(11.4));
    let meters = t.size.rem(14.3);
    let lead = if b.play.is_some() { inset + ih + pad } else { end };
    // the clock as wide as its widest words
    let words = ["BPM · internal", "BPM · stopped"].iter().map(|w| controls::label_width(d, t, w)).fold(0.0, f32::max);
    let clock_w = lead + d.advance(px) * 5.0 + pad + words + end;
    // as wide in both states: the row does not move when it latches
    let panic_w = controls::label_width(d, t, panic_label(true)).max(controls::label_width(d, t, panic_label(false))) + 4.0 * pad;
    let tempo_w = clock_w + gap + panic_w;
    let right_of = |wide: bool| end + if b.sign_in.is_empty() { 0.0 } else { sign_w + pad } + if wide { meters + pad } else { 0.0 } + ih + pad + version_w + end;
    // the load meters only where the path keeps room of its own beside them
    let wide = row(width, gap, ws.x + ws.w, tempo_w, b.slot, right_of(false)).path_w > meters + t.size.rem(17.0);
    let right_w = right_of(wide);
    let r = row(width, gap, ws.x + ws.w, tempo_w, b.slot, right_w);
    // the path, then the status in what is left
    let path = Rect { x: r.path_x, y, w: r.path_w, h };
    if path.w > 2.0 * h {
        pill(d, t, path);
        // each step at most its share of the pill (the status gets what they leave)
        let share = (path.w - 2.0 * inset) / b.path.len().max(1) as f32 - 2.0 * pad - 14.0;
        let steps: Vec<String> = b.path.iter().map(|s| controls::fit_sans(d, px, s, share.max(3.0 * px))).collect();
        let crumbs = controls::breadcrumb_rounded(d, t, (path.x + inset, y + inset, ih), &steps, ir);
        let last = crumbs.last().map_or(path.x + inset, |c| c.x + c.w);
        out.path = crumbs.into_iter().filter(|c| c.x + c.w <= path.x + path.w).collect();
        let status = controls::fit_sans(d, small, b.status, path.x + path.w - end - last - pad);
        d.sans(last + pad, small_y, small, &status, t.color.muted);
    }
    // the tempo: the play button, the clock's value and what it is
    let tempo = Rect { x: r.tempo_x, y, w: clock_w, h };
    pill(d, t, tempo);
    if let Some(wire) = b.play {
        let at = Rect { x: tempo.x + inset, y: y + inset, w: ih, h: ih };
        out.play = Some(if wire { inner_button(d, t, at, "play-fill", t.color.go, b.pointer) } else { inner_button(d, t, at, "stop-fill", t.color.stop, b.pointer) });
    }
    let (value, what) = match b.beat {
        Some((bpm, running)) => (format!("{bpm:.1}"), format!("BPM · {}", if running { "internal" } else { "stopped" })),
        None => ("–".to_string(), "BPM".to_string()),
    };
    let vw = d.text(tempo.x + lead, text_y, px, &value, t.color.text);
    controls::label(d, t, tempo.x + lead + vw + pad, mid - t.size.label * 0.62, &what, t.color.muted);
    // the panic: red, always there, beside the tempo; latched it is ringed and says RESUME
    let panic = Rect { x: tempo.x + clock_w + gap, y, w: panic_w, h };
    let hot = panic.has(b.pointer.0, b.pointer.1);
    d.round_rect(panic.x, panic.y, panic.w, panic.h, h * 0.5, if hot != b.panicked { t.color.stop } else { t.color.danger });
    if b.panicked {
        d.outline(panic.x, panic.y, panic.w, panic.h, (h * 0.5, 2.0), t.color.bright);
    }
    let label = panic_label(b.panicked);
    let lx = panic.x + (panic.w - controls::label_width(d, t, label)) * 0.5;
    controls::label(d, t, lx, mid - t.size.label * 0.62, label, t.color.bright);
    out.panic = panic;
    out.slot = Rect { x: r.slot_x, y, w: b.slot, h };
    // the right pill
    let right = Rect { x: r.right_x, y, w: right_w, h };
    pill(d, t, right);
    let mut x = right.x + end;
    if !b.sign_in.is_empty() {
        d.sans(x, small_y, small, &controls::fit_sans(d, small, b.sign_in, sign_w), t.color.muted);
        x += sign_w + pad;
    }
    if wide {
        let mut mx = x;
        mx += controls::meter(d, t, mx, small_y, "CPU", b.cpu) + 2.0 * pad;
        if let Some(g) = b.gpu {
            controls::meter(d, t, mx, small_y, "GPU", g);
        }
        x += meters + pad;
    }
    let hot = Rect { x, y: y + inset, w: ih, h: ih }.has(b.pointer.0, b.pointer.1);
    out.day = inner_button(d, t, Rect { x, y: y + inset, w: ih, h: ih }, if b.night { "dark" } else { "light" }, if hot { t.color.text } else { t.color.muted }, b.pointer);
    d.text(right.x + right.w - end - version_w, text_y, px, deck_native::BUILD, t.color.muted);
    out
}

/// what a main menu line says now: the look to switch to, the UI size
fn line_label(a: Action, label: &str, look: &crate::tokens::Look) -> String {
    match a {
        Action::DayNight => format!("{label}: {}", if look.theme == crate::tokens::Theme::Day { "day" } else { "night" }),
        _ => label.to_string(),
    }
}

/// the main menu under its hamburger; its lines' places
pub fn menu(d: &mut DrawList, t: &Tokens, at: Rect, enabled: &dyn Fn(Action) -> bool, look: &crate::tokens::Look, pointer: (f32, f32)) -> Vec<(Rect, Action)> {
    let labels: Vec<String> = MENU.iter().map(|&(a, label, _)| line_label(a, label, look)).collect();
    let lines: Vec<controls::Action> = MENU.iter().zip(&labels).map(|(&(a, _, external), label)| controls::Action { label, enabled: enabled(a), external }).collect();
    let rects = controls::actions(d, t, (at.x, at.y + at.h + t.size.rem(0.43)), t.size.rem(17.0), &lines, pointer);
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

    /// the pill as the toolbar draws it for the host's core: where it is and what the bar says
    fn panic_pill(host: &crate::show::Host) -> (Rect, Vec<String>) {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let path = vec!["main".to_string()];
        let b = Bar { workspaces: &["Build", "Show"], workspace: 1, path: &path, status: "", play: None, beat: None, sign_in: "", logo_frame: 0, pointer: (0.0, 0.0), cpu: 0.0, gpu: None, slot: 0.0, night: false, panicked: host.sub.panicked() };
        let placed = draw(&mut d, &t, 1600.0, &b);
        (placed.panic, d.texts().iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn the_panic_pill_latches_until_resume_and_keeps_its_place() {
        let mut host = crate::show::Host::new(None, None, None, "http://127.0.0.1:9".into());
        let (at, texts) = panic_pill(&host);
        assert!(texts.iter().any(|s| s == "PANIC") && !texts.iter().any(|s| s == "RESUME"), "{texts:?}");
        host.panic_pill();
        assert!(host.sub.panicked());
        let (latched, texts) = panic_pill(&host);
        assert!(texts.iter().any(|s| s == "RESUME") && !texts.iter().any(|s| s == "PANIC"), "{texts:?}");
        assert_eq!(latched, at, "the same place latched");
        host.panic_pill();
        assert!(!host.sub.panicked());
        assert!(panic_pill(&host).1.iter().any(|s| s == "PANIC"));
    }

    #[test]
    fn what_sits_in_a_pill_is_whole_inset_and_concentric_with_it() {
        use deck_kit::base::draw::Prim;
        let t = Tokens::default();
        let faces = deck_kit::base::fonts::Faces::shipped(deck_outlet::text::mono_ttf());
        let mut d = DrawList::with_widths(std::sync::Arc::new(faces.widths()));
        let path = vec!["1 Opening".to_string()];
        let b = Bar { workspaces: &["Build", "Show"], workspace: 0, path: &path, status: "", play: Some(true), beat: Some((120.0, true)), sign_in: "signing in …", logo_frame: 0, pointer: (0.0, 0.0), cpu: 0.0, gpu: None, slot: 0.0, night: false, panicked: false };
        draw(&mut d, &t, 1600.0, &b);
        let texts = d.texts();
        for whole in ["signing in …", deck_native::BUILD, "1 Opening", "BPM · INTERNAL"] {
            assert!(texts.contains(&whole), "{whole}: {texts:?}");
        }
        let inset = t.size.rem(0.29);
        let inner = t.size.top * 0.5 - inset;
        let pressed: Vec<(f32, f32)> = d.prims().iter().filter_map(|p| if let Prim::Rect { r, h, color, .. } = *p { (color == t.color.pressed).then_some((r, h)) } else { None }).collect();
        assert_eq!(pressed.len(), 2, "the workspace on and the path's last step");
        for (r, h) in pressed {
            assert!((r - inner).abs() < 0.01 && (h - (t.size.top - 2.0 * inset)).abs() < 0.01, "{r} {h}");
        }
    }

    #[test]
    fn every_menu_line_lands_and_a_disabled_one_is_still_placed() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let lines = menu(&mut d, &t, Rect { x: 16.0, y: 8.0, w: 36.0, h: 36.0 }, &|a| a != Action::New, &crate::tokens::Look::default(), (0.0, 0.0));
        assert_eq!(lines.len(), MENU.len());
        assert!(lines.windows(2).all(|w| w[1].0.y > w[0].0.y));
        assert!(lines[0].0.y > 44.0, "under the hamburger");
        assert!(d.texts().iter().any(|l| l.ends_with(": day")), "the look line says the look");
    }
}
