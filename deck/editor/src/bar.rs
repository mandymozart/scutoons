//! The toolbar over the stage, and the window's title bar where the editor draws its own frame (the creator's bar, the
//! same model – picture A1): left the tabs of the items open, a strip (tabs.rs: the front one carries the path inside it,
//! its steps go back up; an overflow list with Close others / Close all; + opens the switcher); in the middle the
//! workspace row Scene · Rigging · Show, most used first, with its ⌄ (the switcher) and nothing else (RULES H16); right,
//! icons only: the play pill only where something plays (Edit / Play, play, BPM, the clock's source; Panic beside it),
//! undo · redo, then Publish (only when something needs it) · account · the ☰ menu far right. The build and the load
//! meters are the ☰ menu's foot. Groups of icons and toolbars keep their bordered pill; on Windows the window buttons
//! bare at the row's right end.
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::window::{self, Button, Side};
use deck_ui::workspaces::Workspace;
pub use crate::sign_in::Signed;

/// the pills' distance from the window's top
fn margin(t: &Tokens) -> f32 {
    t.size.rem(0.57)
}

/// where the toolbar ends: the panels start a gap under it
pub fn bottom(t: &Tokens) -> f32 {
    margin(t) + t.size.top
}

/// what a line of the ☰ menu (main_menu.rs) or Publish does
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


/// a tab's mark beside its name: its item plays (Play, #629), or it is the show live
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    None,
    Playing,
    Live,
}

/// one tab as the toolbar shows it: its kind's icon, its item's name, unsaved changes, a mark
#[derive(Debug, Clone, Copy)]
pub struct TabPill<'a> {
    pub icon: &'static str,
    pub name: &'a str,
    pub dirty: bool,
    pub mark: Mark,
}

/// the front tab's switch, first in the play pill
#[derive(Debug, Clone, Copy)]
pub enum Modes<'a> {
    /// a Scene, Asset, Module or medium's Edit / Play: one switch, Keep beside it while `keep`; its places the switch, then Keep
    Switch { playing: bool, keep: bool },
    /// a Show tab's Go live, or End and Lock: one pill each, `on` the one lit
    Pills(&'a [&'a str], usize),
}

pub const EDIT: &str = "Edit";
pub const PLAY: &str = "Play";
pub const KEEP: &str = "Keep";

/// what the toolbar shows
pub struct Bar<'a> {
    /// the items open, and the one in front (none: nothing in front)
    pub tabs: &'a [TabPill<'a>],
    pub front: Option<usize>,
    /// the path inside the front tab, after its name: each step a pill that goes back to it
    pub path: &'a [String],
    /// the workspaces in the row's order, and the one in front
    pub workspaces: &'a [Workspace],
    pub workspace: Option<Workspace>,
    /// the front tab's own switch
    pub modes: Option<Modes<'a>>,
    /// the play button (Build): true = it starts playing
    pub play: Option<bool>,
    /// the tempo clock's BPM and whether it runs
    pub beat: Option<(f64, bool)>,
    /// the beat dot and the bar meter (the core's `Beat`), the metronome on or off – none: no tempo clock plays
    pub meter: Option<(deck_core::tempo::Beat, bool)>,
    pub clock: Clock,
    /// the play pill shows: something plays here
    pub plays: bool,
    /// the panic pill shows (in Show, or while something plays or a panic holds)
    pub panic: bool,
    /// whether undo and redo can be done now
    pub undo: (bool, bool),
    /// Publish shows: something needs it
    pub publish: bool,
    pub account: Signed,
    /// 0..1, the signing-in icon's breath (1 = still, reduce motion)
    pub pulse: f32,
    pub pointer: (f32, f32),
    /// a panic holds: the pill resumes
    pub panicked: bool,
    /// the ☰ menu is open: its button is lit
    pub menu: bool,
    /// the switcher is open: the row's ⌄ is lit
    pub switcher: bool,
    /// the editor's own frame: where the window buttons go, and whether the window is maximised
    pub window: Option<(Side, bool)>,
}

/// where the tempo comes from: the deck's own clock, or one it follows (locked, else still waiting for it)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clock {
    Int,
    Ext { locked: bool },
    /// a show is live (#630): the clock is its, locked
    Live,
}

impl Clock {
    pub fn word(self) -> &'static str {
        match self {
            Clock::Int => "INT",
            Clock::Ext { .. } => "EXT",
            Clock::Live => "LIVE",
        }
    }
}

/// the load numbers in words: the meters' full readings, for the info
pub fn meters_words(cpu: Option<deck_core::load::Reading>, gpu: Option<deck_core::load::Reading>, rate: Option<f32>, fps: f32) -> String {
    use deck_core::load::{CPU_MEANS, GPU_MEANS};
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
    /// the tabs shown: which, its place, its close button
    pub tabs: Vec<(usize, Rect, Rect)>,
    /// the tab list (every tab, Close others, Close all) and how many tabs did not fit beside it
    pub list: Option<Rect>,
    pub hidden: usize,
    /// the strip's +: the dashboard, to open anything
    pub plus: Option<Rect>,
    pub path: Vec<Rect>,
    /// the workspaces' places
    pub workspaces: Vec<(Workspace, Rect)>,
    pub modes: Vec<Rect>,
    pub play: Option<Rect>,
    /// the workspace row's ⌄: the switcher
    pub chooser: Option<Rect>,
    /// the panic pill (zero wide: not shown)
    pub panic: Rect,
    /// the play pill (zero wide: not shown)
    pub tempo: Rect,
    /// inside it: the BPM (typed on) and the source's word (cycles the pick)
    pub bpm: Rect,
    pub source: Rect,
    /// the metronome's button (none drawn: zero wide)
    pub click: Rect,
    pub undo: Rect,
    pub redo: Rect,
    pub publish: Option<Rect>,
    pub account: Rect,
    /// the menu: the dashboard
    pub menu: Rect,
    /// the window buttons, when the editor draws its own frame
    pub window: Vec<(Rect, Button)>,
    /// every pill and control in the row: what is not the empty run the window drags by
    pub solid: Vec<Rect>,
}

/// a pill's ground (the kit's `pill`: line and the panel ground)
fn pill(d: &mut DrawList, t: &Tokens, r: Rect) {
    let s = t.kit("pill", "rest");
    d.round_rect(r.x, r.y, r.w, r.h, r.h * 0.5, s.ground.unwrap_or(t.color.panel()));
    d.outline(r.x, r.y, r.w, r.h, (r.h * 0.5, 1.0), s.border.unwrap_or(t.color.line));
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

/// A group of icons in one bordered pill ending at `right`: each a round button inset by `inset`; the pill and the
/// buttons' places
fn group(d: &mut DrawList, t: &Tokens, (right, y, inset): (f32, f32, f32), icons: &[(&'static str, [f32; 4])], pointer: (f32, f32)) -> (Rect, Vec<Rect>) {
    let h = t.size.top;
    let ih = h - 2.0 * inset;
    let n = icons.len() as f32;
    let w = n * ih + (n - 1.0).max(0.0) * 2.0 + 2.0 * inset;
    let r = Rect { x: right - w, y, w, h };
    pill(d, t, r);
    let places = icons.iter().enumerate().map(|(i, (name, ink))| inner_button(d, t, Rect { x: r.x + inset + i as f32 * (ih + 2.0), y: y + inset, w: ih, h: ih }, name, *ink, pointer)).collect();
    (r, places)
}

/// `c` at `a` of its strength
fn faded(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

/// the beat dot: lit on the first part of each beat (the bar's first in the text colour, the others fainter), dark between
/// and while standing
fn beat_dot(d: &mut DrawList, t: &Tokens, (x, y): (f32, f32), b: deck_core::tempo::Beat) {
    let ink = match (b.lit, b.accent) {
        (true, true) => t.color.text,
        (true, false) => faded(t.color.text, 0.6),
        _ => faded(t.color.muted, 0.35),
    };
    d.disc(x, y, t.size.rem(0.24), ink);
}

/// the bar meter: a cell per beat of the bar, the beat now filled (the first in the text colour), the others faint
fn bar_meter(d: &mut DrawList, t: &Tokens, r: Rect, b: deck_core::tempo::Beat) {
    let n = b.per_bar.max(1);
    let gap = (r.w * 0.12 / n as f32).min(t.size.rem(0.18));
    let w = (r.w - gap * (n - 1) as f32) / n as f32;
    for i in 0..n {
        let ink = match (i == b.beat, i == 0) {
            (true, true) => t.color.text,
            (true, false) if b.running => faded(t.color.text, 0.6),
            (true, false) => t.color.muted,
            _ => faded(t.color.muted, 0.3),
        };
        d.round_rect(r.x + i as f32 * (w + gap), r.y, w, r.h, (r.h * 0.2).min(w * 0.5), ink);
    }
}

/// the modes' width inside the play pill
fn modes_width(d: &DrawList, t: &Tokens, modes: Option<Modes>, h: f32) -> f32 {
    let pills_w = |names: &[&str]| names.iter().map(|n| d.sans_width(t.size.small, n) + 2.0 * t.size.pad + 2.0).sum::<f32>();
    match modes {
        None => 0.0,
        Some(Modes::Pills(names, _)) => pills_w(names) + t.size.pad * 0.5,
        Some(Modes::Switch { keep, .. }) => switch_width(d, t, h) + 2.0 + if keep { pills_w(&[KEEP]) } else { 0.0 } + t.size.pad * 0.5,
    }
}

pub fn draw(d: &mut DrawList, t: &Tokens, width: f32, b: &Bar) -> Placed {
    let (h, pad, px) = (t.size.top, t.size.pad, t.size.font);
    let gap = t.size.gap * 0.5;
    // what sits inside a pill is inset by `inset`, `ih` high, rounded by the pill's radius less the inset; text keeps
    // `end` from a pill's round ends
    let inset = t.size.rem(0.29);
    let (ih, ir) = (h - 2.0 * inset, h * 0.5 - inset);
    let end = pad + inset;
    let y = margin(t);
    let mid = y + h * 0.5;
    let text_y = mid - px * 0.62;
    let mut out = Placed::default();
    d.halo(t.color.ring());
    // macOS's traffic lights (the system's) keep the row's start; Windows' buttons its end
    let frame = |side: Side| if b.window.is_some_and(|(s, _)| s == side) { window::width(t) + gap } else { 0.0 };
    let (lead_room, end_room) = (frame(Side::Left), frame(Side::Right));
    let room = width - end_room;
    // right, from the end: Publish · account · the menu
    let mut x = room - t.size.gap;
    let account_ink = match b.account {
        Signed::In => t.color.text,
        Signed::Out => t.color.danger,
        Signed::Signing => faded(t.color.muted, 0.35 + 0.65 * b.pulse),
    };
    let mut icons: Vec<(&'static str, [f32; 4])> = Vec::new();
    if b.publish {
        icons.push(("upload-cloud", t.color.text));
    }
    icons.extend([("account", account_ink), ("menu", t.color.text)]);
    let (end_pill, places) = group(d, t, (x, y, inset), &icons, b.pointer);
    if b.menu {
        controls::icon_button_in(d, t, places[places.len() - 1], "menu", "on");
    }
    out.publish = b.publish.then(|| places[0]);
    out.account = places[places.len() - 2];
    out.menu = places[places.len() - 1];
    x = end_pill.x - gap;
    // undo · redo
    let ink = |on: bool| if on { t.color.text } else { faded(t.color.muted, 0.5) };
    let (undo_pill, places) = group(d, t, (x, y, inset), &[("undo", ink(b.undo.0)), ("redo", ink(b.undo.1))], b.pointer);
    (out.undo, out.redo) = (places[0], places[1]);
    x = undo_pill.x - gap;
    // the play pill: the modes, the play button, the beat, the BPM, the source, the metronome – only where something plays
    let modes_w = modes_width(d, t, b.modes, ih);
    let words = [Clock::Int, Clock::Ext { locked: true }, Clock::Live].iter().map(|c| controls::label_width(d, t, c.word())).fold(0.0, f32::max);
    let mark = t.size.rem(0.5);
    let (dot_w, meter_w) = (t.size.rem(0.85), t.size.rem(3.0));
    let lead = if b.play.is_some() { ih + pad } else { pad };
    let clock_w = inset + modes_w + lead + dot_w + d.advance(px) * 5.0 + pad + meter_w + pad + words + mark + ih + end;
    let panic_w = controls::label_width(d, t, panic_label(true)).max(controls::label_width(d, t, panic_label(false))) + 4.0 * pad;
    if b.panic {
        let panic = Rect { x: x - panic_w, y, w: panic_w, h };
        let hot = panic.has(b.pointer.0, b.pointer.1);
        d.round_rect(panic.x, panic.y, panic.w, panic.h, h * 0.5, if hot != b.panicked { t.color.stop } else { t.color.danger });
        if b.panicked {
            d.outline(panic.x, panic.y, panic.w, panic.h, (h * 0.5, 2.0), t.color.bright);
        }
        let label = panic_label(b.panicked);
        controls::label(d, t, panic.x + (panic.w - controls::label_width(d, t, label)) * 0.5, mid - t.size.label * 0.62, label, t.color.bright);
        out.panic = panic;
        x = panic.x - gap;
    }
    if b.plays {
        let tempo = Rect { x: x - clock_w, y, w: clock_w, h };
        pill(d, t, tempo);
        let mut cx = tempo.x + inset;
        match b.modes {
            Some(Modes::Pills(names, on)) => out.modes = controls::pills_rounded(d, t, (cx, y + inset, ih), names, on, ir),
            Some(Modes::Switch { playing, keep }) => out.modes = edit_play(d, t, (cx, y + inset, ih), (playing, keep), b.pointer),
            None => {}
        }
        cx += modes_w;
        if let Some(wire) = b.play {
            let at = Rect { x: cx, y: y + inset, w: ih, h: ih };
            out.play = Some(if wire { inner_button(d, t, at, "play-fill", t.color.go, b.pointer) } else { inner_button(d, t, at, "stop-fill", t.color.stop, b.pointer) });
        }
        // the BPM, muted while the clock stands, and where it comes from; a source not locked yet carries a small mark
        let (value, vink) = match b.beat {
            Some((bpm, running)) => (format!("{bpm:.1}"), if running { t.color.text } else { t.color.muted }),
            None => ("–".to_string(), t.color.muted),
        };
        let bx = cx + lead + dot_w;
        let vw = d.text(bx, text_y, px, &value, vink);
        let meter_x = bx + d.advance(px) * 5.0 + pad;
        if let Some((beat, _)) = b.meter {
            beat_dot(d, t, (cx + lead + dot_w * 0.35, mid), beat);
            bar_meter(d, t, Rect { x: meter_x, y: mid - t.size.rem(0.19), w: meter_w, h: t.size.rem(0.38) }, beat);
        }
        let wx = meter_x + meter_w + pad;
        let ww = controls::label(d, t, wx, mid - t.size.label * 0.62, b.clock.word(), t.color.muted);
        if b.clock == (Clock::Ext { locked: false }) {
            d.disc(wx + ww + mark * 0.6, mid - t.size.label * 0.3, mark * 0.25, t.color.warn);
        }
        if let Some((_, on)) = b.meter {
            let at = Rect { x: tempo.x + clock_w - end - ih, y: y + inset, w: ih, h: ih };
            if on {
                d.round_rect(at.x, at.y, at.w, at.h, ir, t.color.pressed);
            }
            out.click = inner_button(d, t, at, if on { "metronome" } else { "metronome-off" }, if on { t.color.pressed_text } else { t.color.muted }, b.pointer);
        }
        out.tempo = tempo;
        out.bpm = Rect { x: bx - pad * 0.5, y, w: vw + pad, h };
        out.source = Rect { x: wx - pad * 0.5, y, w: ww + pad, h };
        x = tempo.x - gap;
    }
    let right_start = x;
    // the workspace row and its ⌄: in the window's middle when they fit there, else just before the right end
    let names: Vec<&str> = b.workspaces.iter().map(|w| w.word()).collect();
    let ws_w = names.iter().map(|n| d.sans_width(px, n) + 2.0 * pad + 2.0).sum::<f32>() + ih + 2.0 * inset;
    let left = t.size.gap + lead_room;
    let min_strip = 3.0 * h;
    let ws_x = ((width - ws_w) * 0.5).min(right_start - ws_w).max(left + min_strip + gap);
    let ws_fits = !names.is_empty() && ws_x + ws_w <= right_start + 0.01;
    let mut strip_end = right_start;
    if ws_fits {
        let r = Rect { x: ws_x, y, w: ws_w, h };
        pill(d, t, r);
        let mut cx = r.x + inset;
        // a workspace carries no border at rest: the one in front in the text colour, a rounded border under the pointer
        for (ws, name) in b.workspaces.iter().zip(&names) {
            let place = Rect { x: cx, y: y + inset, w: d.sans_width(px, name) + 2.0 * pad, h: ih };
            let front = Some(*ws) == b.workspace;
            let state = if place.has(b.pointer.0, b.pointer.1) { "hover" } else if front { "on" } else { "rest" };
            let ink = controls::tab_frame(d, t, place, ir, state);
            let ink = if front { t.kit("tab", "on").text.unwrap_or(t.color.text) } else { ink };
            d.sans(place.x + pad, mid - px * 0.62, px, name, ink);
            out.workspaces.push((*ws, place));
            cx += place.w + 2.0;
        }
        let chooser = Rect { x: r.x + r.w - inset - ih, y: y + inset, w: ih, h: ih };
        out.chooser = Some(if b.switcher { controls::icon_button_in(d, t, chooser, "down", "on") } else { inner_button(d, t, chooser, "down", t.color.muted, b.pointer) });
        strip_end = r.x - gap;
    }
    // the tabs: a strip from the left, the front one with its path; the list and the + at its end
    let strip = Rect { x: left, y, w: (strip_end - left).max(0.0), h };
    if strip.w >= 2.0 * h {
        pill(d, t, strip);
        let plus = Rect { x: strip.x + strip.w - inset - ih, y: y + inset, w: ih, h: ih };
        out.plus = Some(inner_button(d, t, plus, "plus", t.color.text, b.pointer));
        if !b.tabs.is_empty() {
            tabs(d, t, Rect { x: strip.x + inset, y: y + inset, w: plus.x - 2.0 - strip.x - inset, h: ih }, b, &mut out);
        }
    }
    if let Some((Side::Right, maximised)) = b.window {
        out.window = window::draw(d, t, room, y, maximised, b.pointer);
    }
    d.halo(None);
    out.solid = [end_pill, undo_pill, out.tempo, out.panic].into_iter().chain(out.chooser).chain(out.workspaces.iter().map(|w| w.1)).chain(out.tabs.iter().map(|t| t.1)).chain(out.list).chain(out.plus).chain(out.modes.iter().copied()).filter(|r| r.w > 0.0).collect();
    out
}

/// a tab's width: its icon, its name, a dot or a mark, its close button; the front one's path too
fn tab_width(d: &DrawList, t: &Tokens, tab: &TabPill, path: &[String], h: f32) -> f32 {
    let (px, pad) = (t.size.font, t.size.pad);
    let icon = t.size.rem(1.0);
    let steps: f32 = path.iter().map(|s| 14.0 + d.sans_width(px, s) + 2.0 * pad).sum();
    let marks = if tab.dirty { t.size.rem(0.6) } else { 0.0 } + if tab.mark != Mark::None { t.size.rem(1.0) } else { 0.0 };
    pad + icon + pad * 0.6 + d.sans_width(px, tab.name) + steps + marks + pad * 0.3 + h
}

/// the Edit / Play switch's width: its track and the longer word
fn switch_width(d: &DrawList, t: &Tokens, h: f32) -> f32 {
    let word = d.sans_width(t.size.small, EDIT).max(d.sans_width(t.size.small, PLAY));
    h * 1.5 + t.size.pad * 2.5 + word
}

/// Edit / Play as one switch: a track whose knob goes right and green in Play, the mode's word beside it; Keep a pill
/// after it while `keep`. Its places: the switch, then Keep.
fn edit_play(d: &mut DrawList, t: &Tokens, (x, y, h): (f32, f32, f32), (playing, keep): (bool, bool), pointer: (f32, f32)) -> Vec<Rect> {
    let w = switch_width(d, t, h);
    let pill = Rect { x, y, w, h };
    if pill.has(pointer.0, pointer.1) {
        d.round_rect(x, y, w, h, h * 0.5, t.color.hover);
    }
    let th = h * 0.62;
    let track = Rect { x: x + t.size.pad, y: y + (h - th) * 0.5, w: h * 1.5 - t.size.pad * 0.5, h: th };
    controls::switch(d, t, track, playing);
    let px = t.size.small;
    let word = if playing { PLAY } else { EDIT };
    d.sans(track.x + track.w + t.size.pad, y + (h - px * 1.25) * 0.5, px, word, t.color.text);
    let mut out = vec![pill];
    if keep {
        out.extend(controls::pills_rounded(d, t, (x + w + 2.0, y, h), &[KEEP], usize::MAX, h * 0.5));
    }
    out
}

/// The tabs in `a` (the strip's inside): the front one always, the others in their order while they fit; the list's
/// button at the end, counting the ones left out. The front one is the active tab: a thin outline; the others muted.
fn tabs(d: &mut DrawList, t: &Tokens, a: Rect, b: &Bar, out: &mut Placed) {
    let (px, pad, h) = (t.size.font, t.size.pad, a.h);
    let radius = h * 0.5;
    let list_w = h + d.sans_width(t.size.small, "+99");
    let room = a.w - list_w - 4.0;
    let widths: Vec<f32> = b.tabs.iter().enumerate().map(|(i, tab)| tab_width(d, t, tab, if Some(i) == b.front { b.path } else { &[] }, h) + 2.0).collect();
    let front_w = b.front.and_then(|f| widths.get(f)).copied().unwrap_or(0.0);
    // the front tab's path shortens to fit before anything else gives way
    let squeeze = (front_w - room).max(0.0);
    let mut left = room - (front_w - squeeze);
    let shown: Vec<bool> = widths
        .iter()
        .enumerate()
        .map(|(i, w)| {
            if Some(i) == b.front {
                return true;
            }
            let fits = *w <= left;
            if fits {
                left -= w;
            }
            fits
        })
        .collect();
    out.hidden = shown.iter().filter(|s| !**s).count();
    let mut x = a.x;
    let mid = a.y + h * 0.5;
    let text_y = mid - px * 0.62;
    for (i, tab) in b.tabs.iter().enumerate().filter(|(i, _)| shown[*i]) {
        let front = Some(i) == b.front;
        let w = if front { widths[i] - squeeze } else { widths[i] } - 2.0;
        let r = Rect { x, y: a.y, w: w.max(h), h };
        // a tab carries no border at rest: the front one in the text colour, the others muted; a rounded border under
        // the pointer (the kit's `tab`)
        let hover = controls::tab_frame(d, t, r, radius, if r.has(b.pointer.0, b.pointer.1) { "hover" } else if front { "on" } else { "rest" });
        let ink = if front { t.kit("tab", "on").text.unwrap_or(t.color.text) } else { hover };
        let icon = t.size.rem(1.0);
        let mut cx = r.x + pad;
        d.icon(cx, mid - icon * 0.5, icon, tab.icon, ink);
        cx += icon + pad * 0.6;
        let end = r.x + r.w - h;
        cx += d.sans(cx, text_y, px, &controls::fit_sans(d, px, tab.name, (end - cx).max(px)), ink);
        if tab.dirty {
            d.disc(cx + t.size.rem(0.3), mid, t.size.rem(0.17), t.color.warn);
            cx += t.size.rem(0.6);
        }
        match tab.mark {
            Mark::Playing => d.icon(cx + t.size.rem(0.15), mid - t.size.rem(0.35), t.size.rem(0.7), "play-fill", ink),
            Mark::Live => d.disc(cx + t.size.rem(0.45), mid, t.size.rem(0.25), t.color.danger),
            Mark::None => {}
        }
        if tab.mark != Mark::None {
            cx += t.size.rem(1.0);
        }
        if front {
            // the path inside the tab, each step back to it
            for step in b.path {
                if cx + 14.0 + 2.0 * px > end {
                    break;
                }
                d.icon(cx + 1.0, mid - 6.0, 12.0, "next", t.color.muted);
                cx += 14.0;
                let fitted = controls::fit_sans(d, px, step, (end - cx - 2.0 * pad).max(px));
                let sw = d.sans_width(px, &fitted) + 2.0 * pad;
                if (Rect { x: cx, y: a.y, w: sw, h }).has(b.pointer.0, b.pointer.1) {
                    d.round_rect(cx, a.y, sw, h, radius, t.color.hover);
                }
                d.sans(cx + pad, text_y, px, &fitted, ink);
                out.path.push(Rect { x: cx, y: a.y, w: sw, h });
                cx += sw;
            }
        }
        let close = Rect { x: end, y: a.y, w: h, h };
        if close.has(b.pointer.0, b.pointer.1) {
            d.round_rect(close.x + 3.0, close.y + 3.0, close.w - 6.0, close.h - 6.0, radius, t.color.hover);
        }
        let s = t.size.rem(0.8);
        d.icon(close.x + (h - s) * 0.5, mid - s * 0.5, s, "close", if front { ink } else { t.color.muted });
        out.tabs.push((i, r, close));
        x += w + 2.0;
    }
    // the list: every tab, Close others, Close all; the count of the ones left out
    let list = Rect { x: a.x + a.w - list_w, y: a.y, w: list_w, h };
    if list.has(b.pointer.0, b.pointer.1) {
        d.round_rect(list.x, list.y, list.w, list.h, radius, t.color.hover);
    }
    let s = t.size.rem(1.0);
    d.icon(list.x + (h - s) * 0.5, mid - s * 0.5, s, "down", t.color.text);
    if out.hidden > 0 {
        d.sans(list.x + h - pad * 0.3, mid - t.size.small * 0.62, t.size.small, &format!("+{}", out.hidden), t.color.text);
    }
    out.list = Some(list);
}

/// a line of the tab list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListLine {
    Tab(usize),
    CloseOthers,
    CloseAll,
}

/// the tab list under its button: every tab (the front one marked), then Close others and Close all; its lines' places
pub fn tab_list(d: &mut DrawList, t: &Tokens, at: Rect, tabs: &[TabPill], front: Option<usize>, pointer: (f32, f32)) -> Vec<(Rect, ListLine)> {
    let names: Vec<String> = tabs.iter().enumerate().map(|(i, tab)| format!("{}{}{}", if Some(i) == front { "› " } else { "  " }, tab.name, if tab.dirty { " •" } else { "" })).collect();
    let mut lines: Vec<controls::Action> = names.iter().map(|n| controls::Action { label: n, enabled: true, external: false }).collect();
    lines.push(controls::Action { label: "Close others", enabled: front.is_some() && tabs.len() > 1, external: false });
    lines.push(controls::Action { label: "Close all", enabled: !tabs.is_empty(), external: false });
    let w = t.size.rem(17.0);
    let x = (at.x + at.w - w).max(0.0);
    let rects = controls::actions(d, t, (x, at.y + at.h + t.size.rem(0.43)), w, &lines, pointer);
    let what = (0..tabs.len()).map(ListLine::Tab).chain([ListLine::CloseOthers, ListLine::CloseAll]);
    rects.into_iter().zip(what).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_kit::base::draw::Prim;
    use deck_ui::workspaces::ALL;

    const TABS: [TabPill<'static>; 2] = [TabPill { icon: "user", name: "catpope", dirty: false, mark: Mark::None }, TabPill { icon: "list", name: "duet", dirty: true, mark: Mark::Live }];

    fn bar<'a>(tabs: &'a [TabPill<'a>], front: Option<usize>, path: &'a [String]) -> Bar<'a> {
        Bar {
            tabs,
            front,
            path,
            workspaces: &ALL,
            workspace: Some(Workspace::Rigging),
            modes: None,
            play: None,
            beat: Some((120.0, true)),
            meter: None,
            clock: Clock::Int,
            plays: true,
            panic: false,
            undo: (true, false),
            publish: false,
            account: Signed::In,
            pulse: 1.0,
            pointer: (0.0, 0.0),
            panicked: false,
            menu: false,
            switcher: false,
            window: None,
        }
    }

    fn drawn_with(b: &Bar, width: f32) -> (DrawList, Placed) {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let p = draw(&mut d, &t, width, b);
        (d, p)
    }

    #[test]
    fn the_menu_is_far_right_then_account_then_publish_undo_and_redo_before_them() {
        let path = vec!["main".to_string()];
        let mut b = bar(&TABS, Some(0), &path);
        b.publish = true;
        let (_, p) = drawn_with(&b, 1600.0);
        let publish = p.publish.unwrap();
        assert!(p.menu.x > p.account.x && p.account.x > publish.x && publish.x > p.redo.x && p.redo.x > p.undo.x, "{p:?}");
        let t = Tokens::default();
        assert!((p.menu.x + p.menu.w + t.size.rem(0.29) + t.size.gap - 1600.0).abs() < 0.01, "the menu ends the row");
        b.publish = false;
        let (_, q) = drawn_with(&b, 1600.0);
        assert!(q.publish.is_none() && (q.menu.x - p.menu.x).abs() < 0.01, "no Publish when nothing needs it; the menu keeps its place");
    }

    #[test]
    fn the_bar_is_icons_only_no_build_no_meters_no_home() {
        let path = vec!["main".to_string()];
        let (d, _) = drawn_with(&bar(&TABS, Some(0), &path), 1600.0);
        let texts = d.texts();
        assert!(!texts.contains(&deck_native::BUILD), "the build is the dashboard's: {texts:?}");
        assert!(!texts.iter().any(|s| s.contains("fps") || s.contains("kHz")));
        assert!(!d.prims().iter().any(|p| matches!(p, Prim::Icon { name: "home", .. })), "no Home");
        for icon in ["undo", "redo", "account", "menu", "plus"] {
            assert!(d.prims().iter().any(|p| matches!(p, Prim::Icon { name, .. } if *name == icon)), "{icon}");
        }
    }

    #[test]
    fn the_workspace_row_sits_in_the_middle_with_its_chooser_and_nothing_else() {
        let path = vec!["main".to_string()];
        let (d, p) = drawn_with(&bar(&TABS, Some(0), &path), 1920.0);
        assert_eq!(p.workspaces.iter().map(|w| w.0).collect::<Vec<_>>(), ALL);
        let first = p.workspaces[0].1;
        let chooser = p.chooser.expect("the row's ⌄");
        let centre = (first.x + chooser.x + chooser.w) * 0.5;
        assert!((centre - 960.0).abs() < 20.0, "in the middle: {centre}");
        assert!(chooser.x > p.workspaces[2].1.x, "the ⌄ ends the row");
        let t = Tokens::default();
        let rigging = p.workspaces[1].1;
        assert!(!d.prims().iter().any(|q| matches!(*q, Prim::Outline { x, w, .. } if (x - rigging.x).abs() < 0.01 && (w - rigging.w).abs() < 0.01)), "the front one has no border at rest");
        assert!(d.prims().iter().any(|q| matches!(*q, Prim::Text { color, x, .. } if color == t.color.text && (x - rigging.x - t.size.pad).abs() < 0.01)), "the front one in the text colour");
        assert!(!d.prims().iter().any(|q| matches!(*q, Prim::Rect { color, .. } if color == t.color.pressed)), "no pressed fill anywhere");
        for word in ["Cues", "Problems", "Who", "Session", "Vote", "Demos"] {
            assert!(!d.texts().contains(&word), "{word} is a view in the ☰, never a pill of the bar");
        }
    }

    #[test]
    fn the_play_pill_and_panic_only_where_something_plays() {
        let path = vec!["main".to_string()];
        let mut b = bar(&TABS, Some(0), &path);
        b.plays = false;
        let (d, p) = drawn_with(&b, 1600.0);
        assert!(p.tempo.w == 0.0 && p.panic.w == 0.0 && !d.texts().contains(&"INT"));
        b.plays = true;
        b.panic = true;
        b.modes = Some(Modes::Switch { playing: true, keep: true });
        let (d, p) = drawn_with(&b, 1600.0);
        assert!(p.tempo.w > 0.0 && p.panic.w > 0.0 && d.texts().contains(&"PANIC"));
        assert_eq!(p.modes.len(), 2, "the switch and Keep, inside the play pill");
        assert!(p.modes.iter().all(|m| m.x >= p.tempo.x && m.x + m.w <= p.tempo.x + p.tempo.w + 0.01));
        assert!(p.panic.x > p.tempo.x, "Panic after the play pill");
        assert!(p.panic.x + p.panic.w <= p.undo.x, "before undo");
    }

    #[test]
    fn the_panic_pill_latches_until_resume_and_keeps_its_place() {
        let mut host = crate::show::Host::new(None, None, None, "http://127.0.0.1:9".into());
        let path = vec!["main".to_string()];
        let pill = |host: &crate::show::Host| {
            let mut b = bar(&TABS, Some(1), &path);
            b.panic = true;
            b.panicked = host.sub.panicked();
            let (d, p) = drawn_with(&b, 1600.0);
            (p.panic, d.texts().iter().map(|s| s.to_string()).collect::<Vec<_>>())
        };
        let (at, texts) = pill(&host);
        assert!(texts.iter().any(|s| s == "PANIC"));
        host.panic_pill();
        let (latched, texts) = pill(&host);
        assert!(texts.iter().any(|s| s == "RESUME") && latched == at);
        host.panic_pill();
        assert!(!host.sub.panicked());
    }

    #[test]
    fn groups_keep_their_bordered_pill() {
        let path = vec!["main".to_string()];
        let (d, p) = drawn_with(&bar(&TABS, Some(0), &path), 1600.0);
        let t = Tokens::default();
        let bordered = |r: Rect| d.prims().iter().any(|q| matches!(*q, Prim::Outline { x, y, w, h, color, .. } if color == t.color.line && x <= r.x && y <= r.y && x + w >= r.x + r.w && y + h >= r.y + r.h));
        for (r, what) in [(p.undo, "undo"), (p.redo, "redo"), (p.account, "account"), (p.menu, "menu")] {
            assert!(bordered(r), "{what} sits in a bordered group");
        }
    }

    #[test]
    fn the_bar_is_the_title_bar_buttons_at_the_end_the_run_drags_the_controls_do_not() {
        use deck_kit::editor::window::{Frame, Zone};
        let t = Tokens::default();
        let path = vec!["main".to_string()];
        let mut b = bar(&TABS, Some(0), &path);
        b.play = Some(true);
        b.window = Some((Side::Right, false));
        let (width, height) = (1600.0, 900.0);
        let (_, p) = drawn_with(&b, width);
        let [min, max, close] = [p.window[0], p.window[1], p.window[2]];
        assert_eq!((min.1, max.1, close.1), (Button::Minimise, Button::Maximise, Button::Close));
        let right = p.solid.iter().map(|r| r.x + r.w).fold(0.0, f32::max);
        assert!(right <= min.0.x, "the menu makes room for the buttons");
        let f = Frame { size: (width, height), maximised: false, band: bottom(&t), buttons: p.window.clone(), solid: p.solid.clone() };
        let ws = p.workspaces[0].1;
        let mid = margin(&t) + t.size.top * 0.5;
        assert_eq!(f.hit(ws.x - t.size.gap * 0.6, mid), Zone::Drag, "the gap before the workspaces");
        for (r, what) in [(p.menu, "menu"), (p.tabs[1].1, "a tab"), (p.tabs[0].2, "a close"), (p.list.unwrap(), "the tab list"), (p.path[0], "a path step"), (p.play.unwrap(), "play"), (p.tempo, "tempo"), (p.account, "account"), (ws, "a workspace"), (p.undo, "undo")] {
            assert_eq!(f.hit(r.x + r.w * 0.5, r.y + r.h * 0.5), Zone::App, "{what}");
        }
        let (_, q) = drawn_with(&Bar { window: None, ..b }, width);
        assert!(q.window.is_empty(), "fullscreen and the decks: no buttons");
    }

    #[test]
    fn nothing_open_shows_the_strip_with_its_plus_and_no_tabs() {
        let (_, p) = drawn_with(&bar(&[], None, &[]), 1600.0);
        assert!(p.tabs.is_empty() && p.list.is_none() && p.plus.is_some());
    }

    #[test]
    fn the_front_tab_always_shows_the_rest_overflow_into_the_list() {
        let names = ["nave", "catpope", "analog", "duet", "fireflies", "cathedral", "balkon", "yellowmouse", "walk", "drone"];
        let tabs: Vec<TabPill> = names.iter().map(|n| TabPill { icon: "map", name: n, dirty: false, mark: Mark::None }).collect();
        let path = vec!["analog".to_string()];
        let (_, wide) = drawn_with(&bar(&tabs, Some(9), &path), 5000.0);
        assert_eq!((wide.tabs.len(), wide.hidden), (10, 0));
        let (d, narrow) = drawn_with(&bar(&tabs, Some(9), &path), 1280.0);
        assert!(narrow.hidden > 0 && narrow.tabs.len() + narrow.hidden == 10);
        assert!(narrow.tabs.iter().any(|(i, ..)| *i == 9), "the front one stays");
        assert!(d.texts().iter().any(|s| *s == format!("+{}", narrow.hidden)));
        let list = narrow.list.unwrap();
        assert!(narrow.tabs.iter().all(|(_, r, c)| r.x + r.w <= list.x && c.x + c.w <= list.x));
        assert!(narrow.tabs.iter().all(|(_, r, _)| r.x + r.w <= narrow.workspaces[0].1.x), "the strip ends before the workspaces");
    }

    #[test]
    fn the_tab_list_has_every_tab_then_close_others_and_close_all() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let lines = tab_list(&mut d, &t, Rect { x: 900.0, y: 8.0, w: 36.0, h: 36.0 }, &TABS, Some(1), (0.0, 0.0));
        assert_eq!(lines.iter().map(|l| l.1).collect::<Vec<_>>(), [ListLine::Tab(0), ListLine::Tab(1), ListLine::CloseOthers, ListLine::CloseAll]);
    }

    const BEAT: deck_core::tempo::Beat = deck_core::tempo::Beat { beat: 1, per_bar: 4, lit: true, accent: false, running: true };

    #[test]
    fn the_play_pill_says_the_bpm_and_its_source_and_keeps_its_width() {
        let path = vec!["main".to_string()];
        let mut b = bar(&TABS[..1], Some(0), &path);
        b.meter = Some((BEAT, true));
        let (d, p) = drawn_with(&b, 1800.0);
        assert!(d.texts().contains(&"120.0") && d.texts().contains(&"INT"));
        let inside = |r: Rect| r.x >= p.tempo.x && r.x + r.w <= p.tempo.x + p.tempo.w + 0.01 && r.w > 0.0;
        assert!(inside(p.bpm) && inside(p.source) && inside(p.click));
        b.clock = Clock::Ext { locked: false };
        let (_, q) = drawn_with(&b, 1800.0);
        assert_eq!((q.tempo, q.bpm, q.source), (p.tempo, p.bpm, p.source), "whatever the source");
        assert!(meters_words(None, None, Some(48.0), 60.0).contains("audio at 48 kHz"));
    }

    #[test]
    fn the_account_icon_is_ink_red_or_breathing() {
        let t = Tokens::default();
        let path = vec!["main".to_string()];
        let ink = |s: Signed, pulse: f32| {
            let mut b = bar(&TABS, Some(0), &path);
            b.account = s;
            b.pulse = pulse;
            let (d, _) = drawn_with(&b, 1600.0);
            d.prims().iter().find_map(|p| if let Prim::Icon { name: "account", color, .. } = *p { Some(color) } else { None }).unwrap()
        };
        assert_eq!(ink(Signed::In, 1.0), t.color.text);
        assert_eq!(ink(Signed::Out, 1.0), t.color.danger);
        assert!(ink(Signed::Signing, 0.0)[3] < ink(Signed::Signing, 1.0)[3]);
    }
}
