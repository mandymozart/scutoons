//! The toolbar over the stage, and the window's title bar where the editor draws its own frame: pills in a row, nothing
//! under them but the stage. Left the main menu's hamburger, the workspaces, the path of what is open (its steps go back up); the tempo pill
//! (the play button, the clock's BPM); right the sign-in, the load meters and the version, which keeps its
//! place at any width (every build shows it, always); on Windows the window buttons bare at the row's right end.
use deck_core::load::{Reading, CPU_MEANS, GPU_MEANS};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::window::{self, Button, Side};
pub use crate::sign_in::Signed;

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
    Settings,
    Audio,
    Account,
    Quit,
}

/// the main menu, top down: what each line says, whether it leaves for the browser (no save: everything saves itself)
pub const MENU: [(Action, &str, bool); 7] = [
    (Action::Open, "Open…", false),
    (Action::New, "New show…", false),
    (Action::Publish, "Publish in the creator", true),
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
    /// the play button (Build): true = it starts playing
    pub play: Option<bool>,
    /// the tempo clock's BPM and whether it runs
    pub beat: Option<(f64, bool)>,
    pub clock: Clock,
    pub account: Signed,
    /// 0..1, the signing-in icon's breath (1 = still, reduce motion)
    pub pulse: f32,
    pub pointer: (f32, f32),
    /// the load meters (deck-core's load), each where it can be measured
    pub cpu: Option<Reading>,
    pub gpu: Option<Reading>,
    /// the audio's sample rate and the frames a second, beside their meters
    pub khz: Option<f32>,
    pub fps: f32,
    /// the room a workspace keeps in the row for its own pill (Show's and Perform's view switch)
    pub slot: f32,
    /// a panic holds: the pill resumes
    pub panicked: bool,
    /// the editor's own frame: where the window buttons go, and whether the window is maximised
    pub window: Option<(Side, bool)>,
}

/// where the tempo comes from: the deck's own clock, or one it follows (locked, else still waiting for it)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clock {
    Int,
    Ext { locked: bool },
}

impl Clock {
    pub fn word(self) -> &'static str {
        match self {
            Clock::Int => "INT",
            Clock::Ext { .. } => "EXT",
        }
    }
}

/// the toolbar's numbers in words: the meters' full readings, for the info
pub fn meters_words(cpu: Option<Reading>, gpu: Option<Reading>, rate: Option<f32>, fps: f32) -> String {
    let audio = rate.map_or_else(|| "no audio".to_string(), |k| format!("audio at {}", khz(k)));
    let cpu = cpu.map_or_else(|| "CPU not measured here".to_string(), |c| format!("{} ({CPU_MEANS})", c.words("CPU")));
    let gpu = gpu.map_or_else(|| "GPU not measured here".to_string(), |g| format!("{} ({GPU_MEANS})", g.words("GPU")));
    format!("{cpu}, {audio}. {gpu}, {fps:.0} frames a second.")
}

/// a sample rate as people say it: 48 kHz, 44.1 kHz
pub fn khz(k: f32) -> String {
    if (k - k.round()).abs() < 0.05 {
        format!("{k:.0} kHz")
    } else {
        format!("{k:.1} kHz")
    }
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
    /// the panic pill, in every workspace
    pub panic: Rect,
    pub tempo: Rect,
    pub account: Rect,
    /// the load meters, where there was room for them
    pub meters: Option<Rect>,
    /// the window buttons, when the editor draws its own frame
    pub window: Vec<(Rect, Button)>,
    /// every pill and control in the row: what is not the empty run the window drags by
    pub solid: Vec<Rect>,
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
    let y = margin(t);
    let mid = y + h * 0.5;
    let text_y = mid - px * 0.62;
    let small_y = mid - small * 0.62;
    let mut out = Placed::default();
    d.halo(t.color.ring());
    // macOS's traffic lights (the system's) keep the row's start; Windows' buttons its end
    let frame = |side: Side| if b.window.is_some_and(|(s, _)| s == side) { window::width(t) + gap } else { 0.0 };
    let (lead_room, end_room) = (frame(Side::Left), frame(Side::Right));
    // the hamburger
    let first = Rect { x: t.size.gap + lead_room, y, w: ih + 2.0 * inset, h };
    pill(d, t, first);
    out.menu = inner_button(d, t, Rect { x: first.x + inset, y: y + inset, w: ih, h: ih }, "menu", t.color.text, b.pointer);
    // the workspaces
    let ws_w: f32 = b.workspaces.iter().map(|w| d.sans_width(small, w) + 2.0 * pad).sum::<f32>() + 2.0 * b.workspaces.len().saturating_sub(1) as f32 + 2.0 * inset;
    let ws = Rect { x: first.x + first.w + gap, y, w: ws_w, h };
    pill(d, t, ws);
    out.workspaces = controls::pills_rounded(d, t, (ws.x + inset, y + inset, ih), b.workspaces, b.workspace, ir);
    // the right pill: the account, the meters, the version
    let version_w = d.advance(px) * deck_native::BUILD.chars().count() as f32;
    // a meter's bar and its number: the widest the number gets
    let reading = |name: &str, most: &str| controls::label_width(d, t, name) + t.size.rem(0.36) + t.size.rem(2.9) + pad * 0.5 + d.sans_width(small, most);
    let meters = reading("CPU", "48 kHz") + 2.0 * pad + reading("GPU", "240 fps");
    let lead = if b.play.is_some() { inset + ih + pad } else { end };
    // the clock as wide as its widest words: the row does not move when the source changes
    let words = [Clock::Int, Clock::Ext { locked: true }].iter().map(|c| controls::label_width(d, t, c.word())).fold(0.0, f32::max);
    let mark = t.size.rem(0.5);
    let clock_w = lead + d.advance(px) * 5.0 + pad + words + mark + end;
    // as wide in both states: the row does not move when it latches
    let panic_w = controls::label_width(d, t, panic_label(true)).max(controls::label_width(d, t, panic_label(false))) + 4.0 * pad;
    let tempo_w = clock_w + gap + panic_w;
    let right_of = |wide: bool| inset + ih + pad + if wide { meters + pad } else { 0.0 } + version_w + end;
    // the load meters only where the path keeps room of its own beside them
    let room = width - end_room;
    let wide = row(room, gap, ws.x + ws.w, tempo_w, b.slot, right_of(false)).path_w > meters + t.size.rem(17.0);
    let right_w = right_of(wide);
    let r = row(room, gap, ws.x + ws.w, tempo_w, b.slot, right_w);
    // the path, only the path (the state words are the status bar's)
    let path = Rect { x: r.path_x, y, w: r.path_w, h };
    if path.w > 2.0 * h {
        pill(d, t, path);
        // each step at most its share of the pill
        let share = (path.w - 2.0 * inset) / b.path.len().max(1) as f32 - 2.0 * pad - 14.0;
        let steps: Vec<String> = b.path.iter().map(|s| controls::fit_sans(d, px, s, share.max(3.0 * px))).collect();
        let crumbs = controls::breadcrumb_rounded(d, t, (path.x + inset, y + inset, ih), &steps, ir);
        out.path = crumbs.into_iter().filter(|c| c.x + c.w <= path.x + path.w).collect();
    }
    // the tempo: the play button, the clock's value and what it is
    let tempo = Rect { x: r.tempo_x, y, w: clock_w, h };
    pill(d, t, tempo);
    if let Some(wire) = b.play {
        let at = Rect { x: tempo.x + inset, y: y + inset, w: ih, h: ih };
        out.play = Some(if wire { inner_button(d, t, at, "play-fill", t.color.go, b.pointer) } else { inner_button(d, t, at, "stop-fill", t.color.stop, b.pointer) });
    }
    // the BPM, muted while the clock stands, and where it comes from; a source not locked yet carries a small mark
    let (value, ink) = match b.beat {
        Some((bpm, running)) => (format!("{bpm:.1}"), if running { t.color.text } else { t.color.muted }),
        None => ("–".to_string(), t.color.muted),
    };
    let vw = d.text(tempo.x + lead, text_y, px, &value, ink);
    let wx = tempo.x + lead + vw + pad;
    let ww = controls::label(d, t, wx, mid - t.size.label * 0.62, b.clock.word(), t.color.muted);
    if b.clock == (Clock::Ext { locked: false }) {
        d.disc(wx + ww + mark * 0.6, mid - t.size.label * 0.3, mark * 0.25, t.color.warn);
    }
    out.tempo = tempo;
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
    // the account: ink when signed in, red when not, breathing while it signs in
    let at = Rect { x: right.x + inset, y: y + inset, w: ih, h: ih };
    let ink = match b.account {
        Signed::In => t.color.text,
        Signed::Out => t.color.danger,
        Signed::Signing => {
            let [r, g, bl, a] = t.color.muted;
            [r, g, bl, a * (0.35 + 0.65 * b.pulse)]
        }
    };
    out.account = inner_button(d, t, at, "account", ink, b.pointer);
    if wide {
        let start = at.x + ih + pad;
        let mut mx = start;
        let number = |d: &mut DrawList, x: f32, text: &str| x + pad * 0.5 + d.sans(x + pad * 0.5, small_y, small, text, t.color.muted);
        let (cpu, gpu) = (b.cpu.unwrap_or_default(), b.gpu.unwrap_or_default());
        mx += controls::meter(d, t, mx, small_y, "CPU", cpu.value, cpu.peak);
        mx = number(d, mx, &b.khz.map_or_else(|| "–".to_string(), khz)) + 2.0 * pad;
        mx += controls::meter(d, t, mx, small_y, "GPU", gpu.value, gpu.peak);
        number(d, mx, &format!("{:.0} fps", b.fps));
        out.meters = Some(Rect { x: start, y, w: meters, h });
    }
    d.text(right.x + right.w - end - version_w, text_y, px, deck_native::BUILD, t.color.muted);
    if let Some((Side::Right, maximised)) = b.window {
        out.window = window::draw(d, t, room, y, maximised, b.pointer);
    }
    d.halo(None);
    // the path pill's ground is the run too: only its steps are controls
    out.solid = [first, ws, tempo, panic, out.slot, right].into_iter().chain(out.path.iter().copied()).filter(|r| r.w > 0.0).collect();
    out
}

/// a main menu line: an action, or a panel of the workspace's list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    Do(Action),
    Panel(&'static str),
}

/// the main menu under its hamburger, the workspace's panels that close under it (`(id, name, open)`); its lines' places
pub fn menu(d: &mut DrawList, t: &Tokens, at: Rect, enabled: &dyn Fn(Action) -> bool, panels: &[(&'static str, &'static str, bool)], pointer: (f32, f32)) -> Vec<(Rect, Line)> {
    let lines: Vec<controls::Action> = MENU.iter().map(|&(a, label, external)| controls::Action { label, enabled: enabled(a), external }).collect();
    let w = t.size.rem(17.0);
    let rects = controls::actions(d, t, (at.x, at.y + at.h + t.size.rem(0.43)), w, &lines, pointer);
    let below = rects.last().map_or(at.y + at.h, |r| r.y + r.h + t.size.pad * 0.5) + t.size.rem(0.43);
    let mut out: Vec<(Rect, Line)> = rects.into_iter().zip(MENU).map(|(r, (a, ..))| (r, Line::Do(a))).collect();
    if !panels.is_empty() {
        let lines: Vec<(&str, bool)> = panels.iter().map(|p| (p.1, p.2)).collect();
        let rects = controls::checks(d, t, (at.x, below), w, "PANELS", &lines, pointer);
        out.extend(rects.into_iter().zip(panels).map(|(r, p)| (r, Line::Panel(p.0))));
    }
    out
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
        let b = Bar { workspaces: &["Build", "Show"], workspace: 1, path: &path, play: None, beat: None, clock: Clock::Int, account: Signed::In, pulse: 1.0, pointer: (0.0, 0.0), cpu: None, gpu: None, khz: None, fps: 60.0, slot: 0.0, panicked: host.sub.panicked(), window: None };
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
        let b = Bar { workspaces: &["Build", "Show"], workspace: 0, path: &path, play: Some(true), beat: Some((120.0, true)), clock: Clock::Int, account: Signed::Signing, pulse: 0.5, pointer: (0.0, 0.0), cpu: Some(Reading { value: 0.2, peak: 0.3 }), gpu: Some(Reading { value: 0.1, peak: 0.1 }), khz: Some(48.0), fps: 60.0, slot: 0.0, panicked: false, window: None };
        draw(&mut d, &t, 1600.0, &b);
        let texts = d.texts();
        for whole in [deck_native::BUILD, "1 Opening", "120.0", "INT", "48 kHz", "60 fps"] {
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

    fn drawn(clock: Clock, account: Signed, pulse: f32) -> (DrawList, Placed) {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let path = vec!["main".to_string()];
        let b = Bar { workspaces: &["Build"], workspace: 0, path: &path, play: None, beat: Some((128.0, true)), clock, account, pulse, pointer: (0.0, 0.0), cpu: Some(Reading { value: 0.3, peak: 0.3 }), gpu: None, khz: Some(44.1), fps: 59.6, slot: 0.0, panicked: false, window: None };
        let placed = draw(&mut d, &t, 1800.0, &b);
        (d, placed)
    }

    /// the colour the account icon is drawn in
    fn account_ink(d: &DrawList) -> [f32; 4] {
        use deck_kit::base::draw::Prim;
        d.prims().iter().find_map(|p| if let Prim::Icon { name: "account", color, .. } = *p { Some(color) } else { None }).expect("the account icon")
    }

    #[test]
    fn the_tempo_pill_says_the_bpm_and_int_or_ext_and_nothing_else() {
        let (d, placed) = drawn(Clock::Int, Signed::In, 1.0);
        let texts = d.texts();
        assert!(texts.contains(&"128.0") && texts.contains(&"INT"), "{texts:?}");
        assert!(!texts.iter().any(|s| s.contains("BPM") || s.contains("INTERNAL")), "{texts:?}");
        let (ext, waiting) = (drawn(Clock::Ext { locked: true }, Signed::In, 1.0), drawn(Clock::Ext { locked: false }, Signed::In, 1.0));
        assert!(ext.0.texts().contains(&"EXT"));
        assert_eq!(ext.1.tempo, placed.tempo, "the pill keeps its width whatever the source");
        assert_eq!(waiting.0.prims().len(), ext.0.prims().len() + 1, "a source not locked yet: a small mark");
    }

    #[test]
    fn the_account_icon_is_ink_red_or_breathing() {
        let t = Tokens::default();
        assert_eq!(account_ink(&drawn(Clock::Int, Signed::In, 1.0).0), t.color.text);
        assert_eq!(account_ink(&drawn(Clock::Int, Signed::Out, 1.0).0), t.color.danger);
        let (a, b) = (account_ink(&drawn(Clock::Int, Signed::Signing, 0.0).0), account_ink(&drawn(Clock::Int, Signed::Signing, 1.0).0));
        assert!(a[3] < b[3], "signing in: it breathes");
    }

    #[test]
    fn the_meters_carry_the_audio_rate_and_the_frames() {
        let (d, placed) = drawn(Clock::Int, Signed::In, 1.0);
        let texts = d.texts();
        assert!(texts.contains(&"44.1 kHz") && texts.contains(&"60 fps"), "{texts:?}");
        assert!(placed.meters.is_some());
        let words = meters_words(Some(Reading { value: 0.07, peak: 0.12 }), None, Some(48.0), 60.0);
        assert!(words.contains("audio at 48 kHz") && words.contains("CPU 7 %, peak 12 %") && words.contains(CPU_MEANS) && words.contains("GPU not measured"), "{words}");
    }

    #[test]
    fn the_bar_is_the_title_bar_buttons_at_the_end_the_run_drags_the_controls_do_not() {
        use deck_kit::base::draw::Prim;
        use deck_kit::editor::window::{Frame, Zone};
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let path = vec!["main".to_string()];
        let b = Bar { workspaces: &["Build", "Show"], workspace: 0, path: &path, play: Some(true), beat: Some((120.0, true)), clock: Clock::Int, account: Signed::In, pulse: 1.0, pointer: (0.0, 0.0), cpu: None, gpu: None, khz: None, fps: 60.0, slot: 0.0, panicked: false, window: Some((Side::Right, false)) };
        let (width, height) = (1600.0, 900.0);
        let p = draw(&mut d, &t, width, &b);
        assert!(!d.prims().iter().any(|p| matches!(p, Prim::Logo { .. })), "no logo: the row starts with the menu");
        assert!(p.menu.x < p.workspaces[0].x);
        let [min, max, close] = [p.window[0], p.window[1], p.window[2]];
        assert_eq!((min.1, max.1, close.1), (Button::Minimise, Button::Maximise, Button::Close));
        let right = p.solid.iter().map(|r| r.x + r.w).fold(0.0, f32::max);
        assert!(right <= min.0.x, "the version pill makes room for the buttons");
        assert!((close.0.x + close.0.w - (width - t.size.gap * 0.5)).abs() < 0.01, "close ends the row");
        let f = Frame { size: (width, height), maximised: false, band: bottom(&t), buttons: p.window.clone(), solid: p.solid.clone() };
        let mid = margin(&t) + t.size.top * 0.5;
        assert_eq!(f.hit(p.tempo.x - t.size.gap * 0.5 - 4.0, mid), Zone::Drag, "the path pill's empty end");
        let ws = p.solid[1];
        assert_eq!(f.hit(ws.x + ws.w + t.size.gap * 0.25, mid), Zone::Drag, "the gap after the workspaces");
        for (r, what) in [(p.menu, "menu"), (p.workspaces[1], "a workspace"), (p.path[0], "a path step"), (p.play.unwrap(), "play"), (p.tempo, "tempo"), (p.panic, "panic"), (p.account, "account")] {
            assert_eq!(f.hit(r.x + r.w * 0.5, r.y + r.h * 0.5), Zone::App, "{what}");
        }
        assert_eq!(f.hit(close.0.x + 4.0, mid), Zone::Button(Button::Close));
        let mut e = DrawList::new(0.6);
        assert!(draw(&mut e, &t, width, &Bar { window: None, ..b }).window.is_empty(), "fullscreen and the decks: no buttons");
    }

    #[test]
    fn every_menu_line_lands_and_a_disabled_one_is_still_placed() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let panels = [("info", "Info", true), ("mixer", "Mixer", false)];
        let lines = menu(&mut d, &t, Rect { x: 16.0, y: 8.0, w: 36.0, h: 36.0 }, &|a| a != Action::New, &panels, (0.0, 0.0));
        assert_eq!(lines.len(), MENU.len() + panels.len());
        assert!(lines.windows(2).all(|w| w[1].0.y > w[0].0.y));
        assert!(lines[0].0.y > 44.0, "under the hamburger");
        assert!(!d.texts().iter().any(|l| l.contains("night")), "the look is in the settings");
        assert_eq!(lines[MENU.len()].1, Line::Panel("info"), "the panels after the actions");
        assert!(d.texts().contains(&"PANELS") && d.texts().contains(&"Mixer"));
        let mut e = DrawList::new(0.6);
        assert_eq!(menu(&mut e, &t, Rect { x: 16.0, y: 8.0, w: 36.0, h: 36.0 }, &|_| true, &[], (0.0, 0.0)).len(), MENU.len(), "no list when nothing closes");
    }
}
