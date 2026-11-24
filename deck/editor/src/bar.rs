//! The toolbar over the stage, and the window's title bar where the editor draws its own frame: pills in a row, nothing
//! under them but the stage. Left the main menu's hamburger, Home, the tabs of the items open (tabs.rs: the front one carries
//! the path inside it, its steps go back up; an overflow list with Close others / Close all); the tempo pill
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
    (Action::New, "New…", false),
    (Action::Publish, "Publish in the creator", true),
    (Action::Settings, "Settings…", false),
    (Action::Audio, "Audio & inputs", false),
    (Action::Account, "Account", true),
    (Action::Quit, "Quit to stage  F9", false),
];

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

/// the front tab's switch beside the tabs
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
    /// the items open, and the one in front (none: Home)
    pub tabs: &'a [TabPill<'a>],
    pub front: Option<usize>,
    /// the path inside the front tab, after its name: each step a pill that goes back to it
    pub path: &'a [String],
    /// the front tab's own switch
    pub modes: Option<Modes<'a>>,
    /// the play button (Build): true = it starts playing
    pub play: Option<bool>,
    /// the tempo clock's BPM and whether it runs
    pub beat: Option<(f64, bool)>,
    /// the beat dot and the bar meter (the core's `Beat`), the metronome on or off – none: no tempo clock plays
    pub meter: Option<(deck_core::tempo::Beat, bool)>,
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
    pub home: Rect,
    /// the tabs shown: which, its place, its close button
    pub tabs: Vec<(usize, Rect, Rect)>,
    /// the tab list (every tab, Close others, Close all) and how many tabs did not fit beside it
    pub list: Option<Rect>,
    pub hidden: usize,
    pub path: Vec<Rect>,
    pub modes: Vec<Rect>,
    pub play: Option<Rect>,
    /// the place kept for the workspace's own pill
    pub slot: Rect,
    /// the panic pill, in every workspace
    pub panic: Rect,
    pub tempo: Rect,
    /// inside it: the BPM (typed on) and the source's word (cycles the pick)
    pub bpm: Rect,
    pub source: Rect,
    /// the metronome's button (none drawn: zero wide)
    pub click: Rect,
    pub account: Rect,
    /// the load meters, where there was room for them
    pub meters: Option<Rect>,
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
    // Home: lit while nothing is open
    let ws = Rect { x: first.x + first.w + gap, y, w: ih + 2.0 * inset, h };
    pill(d, t, ws);
    let home = Rect { x: ws.x + inset, y: y + inset, w: ih, h: ih };
    if b.front.is_none() {
        d.round_rect(home.x, home.y, home.w, home.h, ir, t.color.pressed);
    }
    out.home = inner_button(d, t, home, "home", if b.front.is_none() { t.color.pressed_text } else { t.color.text }, b.pointer);
    // the right pill: the account, the meters, the version
    let version_w = d.advance(px) * deck_native::BUILD.chars().count() as f32;
    // a meter's bar and its number: the widest the number gets
    let reading = |name: &str, most: &str| controls::label_width(d, t, name) + t.size.rem(0.36) + t.size.rem(2.9) + pad * 0.5 + d.sans_width(small, most);
    let meters = reading("CPU", "48 kHz") + 2.0 * pad + reading("GPU", "240 fps");
    let lead = if b.play.is_some() { inset + ih + pad } else { end };
    // the clock as wide as its widest words: the row does not move when the source changes
    let words = [Clock::Int, Clock::Ext { locked: true }, Clock::Live].iter().map(|c| controls::label_width(d, t, c.word())).fold(0.0, f32::max);
    let mark = t.size.rem(0.5);
    let (dot_w, meter_w) = (t.size.rem(0.85), t.size.rem(3.0));
    let clock_w = lead + dot_w + d.advance(px) * 5.0 + pad + meter_w + pad + words + mark + ih + end;
    // as wide in both states: the row does not move when it latches
    let panic_w = controls::label_width(d, t, panic_label(true)).max(controls::label_width(d, t, panic_label(false))) + 4.0 * pad;
    let tempo_w = clock_w + gap + panic_w;
    let right_of = |wide: bool| inset + ih + pad + if wide { meters + pad } else { 0.0 } + version_w + end;
    // the load meters only where the path keeps room of its own beside them
    let room = width - end_room;
    let wide = row(room, gap, ws.x + ws.w, tempo_w, b.slot, right_of(false)).path_w > meters + t.size.rem(17.0);
    let right_w = right_of(wide);
    let r = row(room, gap, ws.x + ws.w, tempo_w, b.slot, right_w);
    // the tabs, the front one with its path (the state words are the status bar's)
    let strip = Rect { x: r.path_x, y, w: r.path_w, h };
    if strip.w > 2.0 * h && !b.tabs.is_empty() {
        pill(d, t, strip);
        tabs(d, t, Rect { x: strip.x + inset, y: y + inset, w: strip.w - 2.0 * inset, h: ih }, b, &mut out);
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
    let bx = tempo.x + lead + dot_w;
    let vw = d.text(bx, text_y, px, &value, ink);
    let meter_x = bx + d.advance(px) * 5.0 + pad;
    if let Some((beat, _)) = b.meter {
        beat_dot(d, t, (tempo.x + lead + dot_w * 0.35, mid), beat);
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
    out.solid = [first, ws, tempo, panic, out.slot, right].into_iter().chain(out.tabs.iter().map(|t| t.1)).chain(out.list).chain(out.modes.iter().copied()).filter(|r| r.w > 0.0).collect();
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
/// button at the end, counting the ones left out.
fn tabs(d: &mut DrawList, t: &Tokens, a: Rect, b: &Bar, out: &mut Placed) {
    let (px, pad, h) = (t.size.font, t.size.pad, a.h);
    let radius = h * 0.5;
    let list_w = h + d.sans_width(t.size.small, "+99");
    let pills_w = |names: &[&str]| names.iter().map(|n| d.sans_width(t.size.small, n) + 2.0 * pad + 2.0).sum::<f32>();
    let modes_w = match b.modes {
        None => 0.0,
        Some(Modes::Pills(names, _)) => pills_w(names) + pad,
        Some(Modes::Switch { keep, .. }) => switch_width(d, t, h) + 2.0 + if keep { pills_w(&[KEEP]) } else { 0.0 } + pad,
    };
    let room = a.w - list_w - modes_w - 4.0;
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
        let r = Rect { x, y: a.y, w, h };
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
    match b.modes {
        Some(Modes::Pills(names, on)) => out.modes = controls::pills_rounded(d, t, (x + pad, a.y, h), names, on, radius),
        Some(Modes::Switch { playing, keep }) => out.modes = edit_play(d, t, (x + pad, a.y, h), (playing, keep), b.pointer),
        None => {}
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

    const TABS: [TabPill<'static>; 2] = [TabPill { icon: "user", name: "catpope", dirty: false, mark: Mark::None }, TabPill { icon: "list", name: "duet", dirty: true, mark: Mark::Live }];

    /// the pill as the toolbar draws it for the host's core: where it is and what the bar says
    fn panic_pill(host: &crate::show::Host) -> (Rect, Vec<String>) {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let path = vec!["main".to_string()];
        let b = Bar { tabs: &TABS, front: Some(1), path: &path, modes: None, play: None, beat: None, meter: None, clock: Clock::Int, account: Signed::In, pulse: 1.0, pointer: (0.0, 0.0), cpu: None, gpu: None, khz: None, fps: 60.0, slot: 0.0, panicked: host.sub.panicked(), window: None };
        let placed = draw(&mut d, &t, 1600.0, &b);
        (placed.panic, d.texts().iter().map(|s| s.to_string()).collect())
    }

    /// the bar with `modes`: the places beside the tabs and the words drawn
    fn modes_drawn(modes: Modes) -> (Placed, Vec<String>) {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let path = vec!["main".to_string()];
        let b = Bar { tabs: &TABS, front: Some(0), path: &path, modes: Some(modes), play: None, beat: None, meter: None, clock: Clock::Int, account: Signed::In, pulse: 1.0, pointer: (0.0, 0.0), cpu: None, gpu: None, khz: None, fps: 60.0, slot: 0.0, panicked: false, window: None };
        let placed = draw(&mut d, &t, 1600.0, &b);
        (placed, d.texts().iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn edit_and_play_are_one_switch_with_keep_beside_it_while_playing() {
        let (edit, words) = modes_drawn(Modes::Switch { playing: false, keep: false });
        assert_eq!(edit.modes.len(), 1, "one control");
        assert!(words.iter().any(|w| w == EDIT) && !words.iter().any(|w| w == PLAY || w == KEEP), "{words:?}");
        let (play, words) = modes_drawn(Modes::Switch { playing: true, keep: true });
        assert_eq!(play.modes.len(), 2, "the switch, then Keep");
        assert!(words.iter().any(|w| w == PLAY) && words.iter().any(|w| w == KEEP) && !words.iter().any(|w| w == EDIT), "{words:?}");
        assert_eq!(play.modes[0], edit.modes[0], "the switch keeps its place");
        assert!(play.modes[1].x >= play.modes[0].x + play.modes[0].w, "Keep after the switch");
        let front = edit.tabs.iter().find(|t| t.0 == 0).unwrap().1;
        assert!(edit.modes[0].x >= front.x + front.w, "after the tabs");
        assert!(edit.list.is_none_or(|l| play.modes[1].x + play.modes[1].w <= l.x), "before the list");
        let (show, _) = modes_drawn(Modes::Pills(&["Go live"], 0));
        assert_eq!(show.modes.len(), 1, "a Show tab keeps its pills");
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
        let b = Bar { tabs: &TABS, front: Some(0), path: &path, modes: None, play: Some(true), beat: Some((120.0, true)), meter: None, clock: Clock::Int, account: Signed::Signing, pulse: 0.5, pointer: (0.0, 0.0), cpu: Some(Reading { value: 0.2, peak: 0.3 }), gpu: Some(Reading { value: 0.1, peak: 0.1 }), khz: Some(48.0), fps: 60.0, slot: 0.0, panicked: false, window: None };
        draw(&mut d, &t, 1600.0, &b);
        let texts = d.texts();
        for whole in [deck_native::BUILD, "catpope", "1 Opening", "duet", "120.0", "INT", "48 kHz", "60 fps"] {
            assert!(texts.contains(&whole), "{whole}: {texts:?}");
        }
        let inset = t.size.rem(0.29);
        let inner = t.size.top * 0.5 - inset;
        assert!(!d.prims().iter().any(|p| matches!(*p, Prim::Rect { color, .. } if color == t.color.pressed)), "the front tab is the text colour, no fill");
        // under the pointer a tab takes a rounded border, concentric with the pill
        let tab = draw(&mut DrawList::with_widths(std::sync::Arc::new(faces.widths())), &t, 1600.0, &b).tabs[0].1;
        let mut d = DrawList::with_widths(std::sync::Arc::new(faces.widths()));
        draw(&mut d, &t, 1600.0, &Bar { pointer: (tab.x + tab.w * 0.5, tab.y + tab.h * 0.5), ..b });
        let borders: Vec<(f32, f32)> = d.prims().iter().filter_map(|p| if let Prim::Outline { r, h, color, .. } = *p { (color == t.color.line && h < t.size.top).then_some((r, h)) } else { None }).collect();
        assert_eq!(borders.len(), 1, "the hovered tab's border");
        for (r, h) in borders {
            assert!((r - inner).abs() < 0.01 && (h - (t.size.top - 2.0 * inset)).abs() < 0.01, "{r} {h}");
        }
    }

    fn drawn(clock: Clock, account: Signed, pulse: f32) -> (DrawList, Placed) {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let path = vec!["main".to_string()];
        let b = Bar { tabs: &TABS[..1], front: Some(0), path: &path, modes: None, play: None, beat: Some((128.0, true)), meter: Some((BEAT, false)), clock, account, pulse, pointer: (0.0, 0.0), cpu: Some(Reading { value: 0.3, peak: 0.3 }), gpu: None, khz: Some(44.1), fps: 59.6, slot: 0.0, panicked: false, window: None };
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
    fn the_bpm_and_the_source_word_are_two_places_inside_the_tempo_pill() {
        let (_, p) = drawn(Clock::Int, Signed::In, 1.0);
        let inside = |r: Rect| r.x >= p.tempo.x && r.x + r.w <= p.tempo.x + p.tempo.w + 0.01 && r.w > 0.0;
        assert!(inside(p.bpm) && inside(p.source), "{p:?}");
        assert!(p.bpm.x + p.bpm.w <= p.source.x + 0.01, "the BPM first, then the word: {p:?}");
        let (_, ext) = drawn(Clock::Ext { locked: true }, Signed::In, 1.0);
        assert_eq!((ext.bpm, ext.source), (p.bpm, p.source), "they stay where they are whatever the source");
    }

    const BEAT: deck_core::tempo::Beat = deck_core::tempo::Beat { beat: 1, per_bar: 4, lit: true, accent: false, running: true };

    fn with_meter(meter: Option<(deck_core::tempo::Beat, bool)>) -> (DrawList, Placed) {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let path = vec!["main".to_string()];
        let b = Bar { tabs: &TABS[..1], front: Some(0), path: &path, modes: None, play: None, beat: Some((128.0, true)), meter, clock: Clock::Int, account: Signed::In, pulse: 1.0, pointer: (0.0, 0.0), cpu: None, gpu: None, khz: None, fps: 60.0, slot: 0.0, panicked: false, window: None };
        let placed = draw(&mut d, &t, 1800.0, &b);
        (d, placed)
    }

    /// the rounded cells and discs drawn in `ink`
    fn inked(d: &DrawList, ink: [f32; 4]) -> usize {
        use deck_kit::base::draw::Prim;
        d.prims().iter().filter(|p| matches!(**p, Prim::Rect { color, .. } if color == ink)).count()
    }

    #[test]
    fn the_pill_has_a_beat_dot_a_bar_meter_and_the_click_button() {
        let t = Tokens::default();
        let (none, p0) = with_meter(None);
        let (on, p) = with_meter(Some((BEAT, true)));
        assert_eq!(p0.tempo, p.tempo, "the pill keeps its width with or without a clock");
        assert!(p0.click.w == 0.0 && p.click.w > 0.0, "the button only with a clock");
        let inside = p.click.x >= p.tempo.x && p.click.x + p.click.w <= p.tempo.x + p.tempo.w + 0.01;
        assert!(inside && p.source.x + p.source.w <= p.click.x + 0.01, "the button last in the pill: {p:?}");
        // beat 2 of 4, lit: the dot and the meter's cell in the text's ink at 60 %, three faint cells
        assert_eq!(inked(&on, faded(t.color.text, 0.6)), 2, "{:?}", on.prims());
        assert_eq!(inked(&on, faded(t.color.muted, 0.3)), 3);
        assert_eq!(inked(&none, faded(t.color.text, 0.6)), 0);
        let icon = |d: &DrawList| d.prims().iter().find_map(|x| if let deck_kit::base::draw::Prim::Icon { name, .. } = *x { name.starts_with("metronome").then_some(name) } else { None });
        assert_eq!(icon(&on), Some("metronome"));
        assert_eq!(icon(&with_meter(Some((BEAT, false))).0), Some("metronome-off"));
        // the bar's first beat lit: the dot and its cell in the text colour; a bar of 3 has three cells
        let first = deck_core::tempo::Beat { beat: 0, per_bar: 3, accent: true, ..BEAT };
        let (d, _) = with_meter(Some((first, false)));
        assert_eq!((inked(&d, t.color.text), inked(&d, faded(t.color.muted, 0.3))), (2, 2));
        // standing: the dot dark
        let still = deck_core::tempo::Beat { lit: false, running: false, ..BEAT };
        let (d, _) = with_meter(Some((still, false)));
        assert_eq!(inked(&d, t.color.text), 0);
        assert_eq!(inked(&d, faded(t.color.muted, 0.35)), 1, "the dark dot");
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
        let b = Bar { tabs: &TABS, front: Some(0), path: &path, modes: None, play: Some(true), beat: Some((120.0, true)), meter: None, clock: Clock::Int, account: Signed::In, pulse: 1.0, pointer: (0.0, 0.0), cpu: None, gpu: None, khz: None, fps: 60.0, slot: 0.0, panicked: false, window: Some((Side::Right, false)) };
        let (width, height) = (1600.0, 900.0);
        let p = draw(&mut d, &t, width, &b);
        assert!(!d.prims().iter().any(|p| matches!(p, Prim::Logo { .. })), "no logo: the row starts with the menu");
        assert!(p.menu.x < p.home.x);
        let [min, max, close] = [p.window[0], p.window[1], p.window[2]];
        assert_eq!((min.1, max.1, close.1), (Button::Minimise, Button::Maximise, Button::Close));
        let right = p.solid.iter().map(|r| r.x + r.w).fold(0.0, f32::max);
        assert!(right <= min.0.x, "the version pill makes room for the buttons");
        assert!((close.0.x + close.0.w - (width - t.size.gap * 0.5)).abs() < 0.01, "close ends the row");
        let f = Frame { size: (width, height), maximised: false, band: bottom(&t), buttons: p.window.clone(), solid: p.solid.clone() };
        let mid = margin(&t) + t.size.top * 0.5;
        let last = p.tabs.last().expect("the tabs").1;
        assert_eq!(f.hit(last.x + last.w + 8.0, mid), Zone::Drag, "the tab strip's empty run");
        let ws = p.solid[1];
        assert_eq!(f.hit(ws.x + ws.w + t.size.gap * 0.25, mid), Zone::Drag, "the gap after Home");
        for (r, what) in [(p.menu, "menu"), (p.home, "home"), (p.tabs[1].1, "a tab"), (p.tabs[0].2, "a close"), (p.list.unwrap(), "the tab list"), (p.path[0], "a path step"), (p.play.unwrap(), "play"), (p.tempo, "tempo"), (p.panic, "panic"), (p.account, "account")] {
            assert_eq!(f.hit(r.x + r.w * 0.5, r.y + r.h * 0.5), Zone::App, "{what}");
        }
        assert_eq!(f.hit(close.0.x + 4.0, mid), Zone::Button(Button::Close));
        let mut e = DrawList::new(0.6);
        assert!(draw(&mut e, &t, width, &Bar { window: None, ..b }).window.is_empty(), "fullscreen and the decks: no buttons");
    }

    fn with_tabs(tabs: &[TabPill], front: Option<usize>, width: f32) -> (DrawList, Placed) {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let path = vec!["analog".to_string()];
        let b = Bar { tabs, front, path: &path, modes: None, play: None, beat: None, meter: None, clock: Clock::Int, account: Signed::In, pulse: 1.0, pointer: (0.0, 0.0), cpu: None, gpu: None, khz: None, fps: 60.0, slot: 0.0, panicked: false, window: None };
        let p = draw(&mut d, &t, width, &b);
        (d, p)
    }

    #[test]
    fn nothing_open_lights_home_and_shows_no_tab_strip() {
        let t = Tokens::default();
        let (d, p) = with_tabs(&[], None, 1600.0);
        assert!(p.tabs.is_empty() && p.list.is_none());
        use deck_kit::base::draw::Prim;
        assert!(d.prims().iter().any(|q| matches!(*q, Prim::Icon { name: "home", color, .. } if color == t.color.pressed_text)), "Home lit");
    }

    #[test]
    fn the_front_tab_always_shows_the_rest_overflow_into_the_list() {
        let names = ["nave", "catpope", "analog", "duet", "fireflies", "cathedral", "balkon", "yellowmouse", "walk", "drone"];
        let tabs: Vec<TabPill> = names.iter().map(|n| TabPill { icon: "map", name: n, dirty: false, mark: Mark::None }).collect();
        let (wide_d, wide) = with_tabs(&tabs, Some(9), 3840.0);
        assert_eq!((wide.tabs.len(), wide.hidden), (10, 0), "room for all: {:?}", wide_d.texts());
        let (d, narrow) = with_tabs(&tabs, Some(9), 1280.0);
        assert!(narrow.hidden > 0 && narrow.tabs.len() + narrow.hidden == 10);
        assert!(narrow.tabs.iter().any(|(i, ..)| *i == 9), "the front one stays");
        assert!(d.texts().iter().any(|s| *s == format!("+{}", narrow.hidden)), "the list counts the ones left out");
        let list = narrow.list.unwrap();
        assert!(narrow.tabs.iter().all(|(_, r, c)| r.x + r.w <= list.x && c.x + c.w <= list.x), "no tab under the list");
    }

    #[test]
    fn the_tab_list_has_every_tab_then_close_others_and_close_all() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let lines = tab_list(&mut d, &t, Rect { x: 900.0, y: 8.0, w: 36.0, h: 36.0 }, &TABS, Some(1), (0.0, 0.0));
        assert_eq!(lines.iter().map(|l| l.1).collect::<Vec<_>>(), [ListLine::Tab(0), ListLine::Tab(1), ListLine::CloseOthers, ListLine::CloseAll]);
        assert!(d.texts().iter().any(|s| s.contains("› duet") && s.contains('•')), "the front one marked, its unsaved dot: {:?}", d.texts());
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
