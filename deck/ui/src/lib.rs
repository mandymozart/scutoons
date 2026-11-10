//! deck-ui (#374) – the deck's own UI model, platform-free (agents/research/DECK-UI-MODEL.md). The first slice: the HUD
//! (CYBERDECKS.md §8 "a small on-screen HUD") – its lines, the collapsed stats bar (#209) and the mode F1 cycles. An
//! adapter reads what it has into a [`Status`] and draws what [`hud_lines`] / [`bar_items`] answer; the words, their
//! order and the cut are here, the numbers' judgement is deck-core's monitor (#364), the tempo line its controller's (#370).
//!
//! #209 (owner 2026-10-31: "the collapsed panel shows very basic statistics … using the entire width, like OSCT's
//! collapsed bar"): F1 cycles the full HUD → **the bar** (one dense line across the screen's top edge: FPS, CPU / frame,
//! the audio's glitches and load, the quality, the relay, the backend – a number out of the green in amber / red) → nothing.
pub use deck_core::monitor::{Level, Report};

/// #478: the Live page (F6) – the roles this deck may take, no role, its name
pub mod live;

/// text lines the HUD has room for (above the frame-time graph; #208: two audio lines + the glitch warning)
pub const LINES: usize = 13;
/// characters a line has room for (8 px each, a 4 px margin)
pub const COLS: usize = 39;
/// #208: the audio glitch warning – its HUD line's start (the line flashes red)
pub const ALERT: &str = "AUDIO!";

/// #209: what the HUD shows (F1 cycles them)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Full,
    /// the collapsed stats bar across the top
    Bar,
    Off,
}

impl Mode {
    /// F1: the full HUD → the bar → nothing → the full HUD
    pub fn next(self) -> Mode {
        match self {
            Mode::Full => Mode::Bar,
            Mode::Bar => Mode::Off,
            Mode::Off => Mode::Full,
        }
    }
}

/// one item of the bar: its text, and amber / red when it is out of the green (deck-core's monitor judges, #364)
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub text: String,
    pub level: Level,
}

/// the GPU as the adapter names it: the adapter's name and its backend (`Vulkan`, `Dx12` …)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Gpu {
    pub name: String,
    pub backend: String,
}

/// #220: the render quality now – deck-space's governor's level and ceiling (their `name()`), Auto, its scale, and the
/// render scale the renderer draws at
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Quality {
    pub level: String,
    pub ceiling: String,
    pub auto: bool,
    pub scale: f32,
    pub render: f32,
}

/// #208: the audio device – its backend's name, its rate (Hz), its largest callback (frames), how far ahead of the ear
/// the audio is rendered (frames, #188), and the clock's rate the frames are turned into ms with
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Audio {
    pub name: String,
    pub rate: u32,
    pub buffer: u32,
    pub lead: usize,
    pub clock_rate: f32,
}

/// the tempo clock's master (deck-core `Source::name`), its state (the controller's `status`, or the source's own I/O –
/// Link peers, the OSC port, an error), and whether the MIDI clock out runs
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Source {
    pub name: String,
    pub status: String,
    pub midi_out: bool,
}

/// a scene's space (#188): the being's mood line, the space's own line, its session lines (#366's relay status first),
/// and whether it joined a session (a deck, the conductor – not alone)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Space {
    pub mood: String,
    pub line: String,
    pub session: Vec<String>,
    pub joined: bool,
}

/// What the HUD reads, as plain values – the adapter fills it from its platform objects (deck-native: hud_status.rs).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Status {
    /// why the scene isn't playing (the test pattern instead)
    pub error: Option<String>,
    /// the Live outlet's own lines (#186: its link, media, cell and every reason it can't show a layer) and the note why
    /// it is off
    pub live: Vec<String>,
    pub live_note: Option<String>,
    pub scene: String,
    pub gpu: Option<Gpu>,
    pub quality: Option<Quality>,
    pub audio: Option<Audio>,
    /// deck-core's monitor (#364): the bar's items, the lines, the glitch warning
    pub report: Report,
    /// #236: the hosts' events – delivered by the dispatcher, dropped on the way
    pub events: Option<[u64; 2]>,
    /// the tempo line (deck-core `Controller::tempo_line`, #370) and the source
    pub tempo: String,
    pub source: Source,
    /// #211: what the outputs drive, or why they are off
    pub outputs: Option<String>,
    pub outputs_note: Option<String>,
    /// the deck's sync line (#195) and the conductor's push as this deck answers it (#371)
    pub sync: Option<String>,
    pub push: Option<String>,
    pub space: Option<Space>,
    /// the analysis bands: level, bass, mid, high
    pub fft: Option<[f32; 4]>,
    pub music: Vec<String>,
    pub slices: Vec<String>,
}

/// #423: characters the build label has room for (its raster is 8 px a character)
pub const LABEL_COLS: usize = 40;

/// #423: the build label's text for the build string (`deck_native::BUILD`, e.g. `0.1.0.r3093.g8eccc753.dirty`): the string
/// itself – what the update bundle carries – in ASCII (font8x8 has no more), `dev` when empty; over [`LABEL_COLS`] the
/// leading workspace version goes first (`r3093.g8eccc753.dirty`), then the cut.
pub fn build_label(build: &str) -> String {
    let ascii: String = build.trim().chars().map(|c| if c.is_ascii_graphic() { c } else { '?' }).collect();
    let mut s = ascii.as_str();
    if s.chars().count() > LABEL_COLS {
        if let Some(i) = s.find(".r") {
            s = &s[i + 1..];
        }
    }
    let s: String = s.chars().take(LABEL_COLS).collect();
    if s.is_empty() {
        "dev".into()
    } else {
        s
    }
}

/// #423: where the build label sits: `[x, y, w, h]` in screen pixels for a `sw` × `sh` screen when its raster is `w` × `h`
/// pixels drawn at the integer `scale` (the HUD's, 2 at 1080p): the TOP RIGHT corner, a 1rem (16 px at that scale) margin
/// from the top and the right edge. The full HUD and the F5 switcher are top left (the same margin, 320 px wide at that
/// scale), the pairing card lower right: the label clears them whenever the screen is wider than the HUD plus the label.
pub fn label_rect(sw: f32, sh: f32, scale: f32, w: u32, h: u32) -> [f32; 4] {
    let (lw, lh) = ((w as f32 * scale).min(sw), (h as f32 * scale).min(sh));
    let m = 16.0 * scale;
    [(sw - lw - m).max(0.0), m.min((sh - lh).max(0.0)), lw, lh]
}

/// `text` broken into HUD lines at spaces (a word longer than a line is cut)
pub fn wrap(text: &str, cols: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let n = line.chars().count();
        if n > 0 && n + 1 + word.chars().count() > cols {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.extend(word.chars().take(cols));
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

/// the clock's master and its state, the MIDI clock out
pub fn source_line(s: &Source) -> String {
    let out = if s.midi_out { "  > MIDI" } else { "" };
    format!("SRC {} {}{out}", s.name.to_uppercase(), s.status)
}

/// The full HUD's lines: the error, the title, the glitch warning, the outlet, the keys, the GPU, the quality, the audio,
/// the monitor's lines, the events, the clock, the outputs, the sync, the space, the bands, the music and the slices –
/// cut at [`LINES`] (font8x8 has no dashes but the ASCII one).
pub fn hud_lines(s: &Status) -> Vec<String> {
    let gpu = s.gpu.as_ref().map(|g| format!("{} {}", g.name, g.backend)).unwrap_or_default();
    // #208: the device's buffer (its largest callback) and what it costs, how far ahead of the ear the audio is rendered
    let device = s.audio.as_ref().map(|a| {
        let ms = |frames: f32| frames * 1000.0 / a.clock_rate;
        format!("AUDIO {} {}K BUF {} {:.0}MS OUT {:.0}MS", a.name.to_uppercase(), a.rate / 1000, a.buffer, ms(a.buffer as f32), ms(a.lead as f32))
    });
    let report = &s.report;
    let mut lines = Vec::new();
    if let Some(e) = &s.error {
        lines.push("SCENE ERROR - THE TEST PATTERN INSTEAD".into());
        lines.extend(wrap(&e.replace(['–', '—'], "-"), COLS).into_iter().take(LINES - 4));
    }
    // the Live outlet right under the title – it comes first (the key help and the rest follow as far as they fit)
    let mut live = s.live.clone();
    if let Some(n) = &s.live_note {
        live.push(format!("LIVE: {}", n.replace(['–', '—'], "-").to_uppercase()));
    }
    lines.push(format!("SCUTOONSDECK STAGE  {}", s.scene));
    // #208: a glitch flashes the warning right under the title for a few seconds (the adapter draws its row red)
    if !report.alert_line.is_empty() {
        lines.push(format!("{ALERT} {}", report.alert_line));
    }
    lines.extend(live.iter().flat_map(|l| wrap(l, COLS)));
    lines.extend(["F12 WEB CTRL+ALT+ F2 SHELL / DEL OFF".into(), "T TAP +/- BPM SPACE RUN F3 SRC K CLICK".into(), format!("GPU {gpu}")]);
    // #220: the quality now, the level chosen, the governor; the keys
    if let Some(q) = &s.quality {
        lines.push(format!("Q {} {:.2}X OF {} {} F4 SET", q.level, q.render, q.ceiling, if q.auto { "AUTO" } else { "FIXED" }).to_uppercase());
    }
    lines.extend(device);
    lines.extend(report.lines.iter().cloned());
    if let Some([routed, dropped]) = s.events {
        lines.push(format!("EVENTS IN {routed} DROP {dropped}"));
    }
    // the tempo line is the core's (the browser panel shows it word for word; the beat lights sit at its end, #370)
    lines.extend([s.tempo.clone(), source_line(&s.source)]);
    if let Some(o) = &s.outputs {
        lines.push(format!("OUTPUTS {}", o.to_uppercase()));
    }
    if let Some(n) = &s.outputs_note {
        lines.extend(wrap(&format!("OUTPUTS OFF: {}", n.replace(['–', '—', '…'], "-").to_uppercase()), COLS).into_iter().take(3));
    }
    // the sync (#195) while it fetches, is offline or failed
    if let Some(line) = s.sync.as_ref().filter(|l| !l.starts_with("SYNC OK")) {
        lines.push(line.clone());
    }
    lines.extend(s.push.iter().cloned());
    if let Some(sp) = &s.space {
        // the mood and why (#188's feelings) first
        lines.extend([sp.mood.clone(), sp.line.clone()]);
        lines.extend(sp.session.iter().cloned());
        lines.push("DRAG TURN/KNOB WHEEL ZOOM R HOME".into());
    }
    if let Some([l, b, m, h]) = s.fft {
        lines.push(format!("FFT L {l:.2} B {b:.2} M {m:.2} H {h:.2}"));
    }
    lines.extend(s.music.iter().map(|m| format!("MUSIC {m}")));
    lines.extend(s.slices.iter().map(|x| format!("SLICE {x}")));
    lines.truncate(LINES);
    lines
}

/// #209: the collapsed bar's items – the monitor's (FPS, CPU, AUD, LIM; #364: deck-core's words and judgement), the
/// quality only while the governor holds it below the level chosen (amber), the relay (the space's session if it joined
/// one, else the Live outlet's) if there is one, F1. The rest is on the full HUD.
pub fn bar_items(s: &Status) -> Vec<Item> {
    let item = |text: String, warn: bool| Item { text, level: if warn { Level::Warn } else { Level::Ok } };
    let mut out: Vec<Item> = s.report.bar.iter().map(|i| Item { text: i.text(), level: i.level }).collect();
    if let Some(q) = s.quality.as_ref().filter(|q| q.level != q.ceiling) {
        let at = out.len().min(1);
        out.insert(at, item(format!("Q {} {:.2}{}", q.level.to_uppercase(), q.scale, if q.auto { "" } else { " FIX" }), true));
    }
    let mut relay = s.space.as_ref().filter(|sp| sp.joined).and_then(|sp| sp.session.first().cloned()).unwrap_or_default();
    if relay.is_empty() {
        relay = s.live.first().cloned().unwrap_or_default();
    }
    if !relay.is_empty() {
        out.push(item(relay.replace(['–', '—'], "-").to_uppercase(), false));
    }
    out.push(item("F1".into(), false));
    out
}

/// #208: the glitch warning's row in `lines` (the adapter flashes it red)
pub fn alert_row(lines: &[String]) -> Option<usize> {
    lines.iter().position(|l| l.starts_with(ALERT))
}

/// #181: the tempo line's row in `lines` (the beat lights sit at its end)
pub fn beat_row(lines: &[String]) -> Option<usize> {
    lines.iter().position(|l| l.starts_with("BPM"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #423: the label is the build string itself; a long one loses the workspace version, then is cut
    #[test]
    fn the_build_label_is_the_build_string() {
        assert_eq!(build_label("0.1.0.r3093.g8eccc753"), "0.1.0.r3093.g8eccc753");
        assert_eq!(build_label(" 0.1.0.r3093.g8eccc753.dirty\n"), "0.1.0.r3093.g8eccc753.dirty");
        assert_eq!(build_label("dev"), "dev");
        assert_eq!(build_label(""), "dev");
        assert_eq!(build_label("0.1.0.r3093.g8eccc753.é"), "0.1.0.r3093.g8eccc753.?");
        let long = format!("0.1.0.r3093.g8eccc753{}", ".x".repeat(10));
        assert_eq!(build_label(&long), format!("r3093.g8eccc753{}", ".x".repeat(10)));
        assert_eq!(build_label(&"a".repeat(60)).len(), LABEL_COLS);
    }

    /// #423: top right, a 1rem margin, never off screen
    #[test]
    fn the_build_label_sits_top_right() {
        // 1080p at scale 2: a 224 x 12 raster = 448 x 24 px, 32 px from the top and right edges
        assert_eq!(label_rect(1920.0, 1080.0, 2.0, 224, 12), [1920.0 - 448.0 - 32.0, 32.0, 448.0, 24.0]);
        assert_eq!(label_rect(1280.0, 720.0, 1.0, 224, 12), [1280.0 - 224.0 - 16.0, 16.0, 224.0, 12.0]);
        // clear of the full HUD (x 16..336 at scale 1) on a 1280 px screen
        assert!(label_rect(1280.0, 720.0, 1.0, 224, 12)[0] > 16.0 + 320.0);
        // a tiny screen: clamped, never negative
        let r = label_rect(100.0, 50.0, 1.0, 224, 12);
        assert!(r[0] >= 0.0 && r[1] >= 0.0 && r[2] <= 100.0);
    }

    #[test]
    fn f1_cycles_full_bar_off() {
        assert_eq!([Mode::Full.next(), Mode::Bar.next(), Mode::Off.next()], [Mode::Bar, Mode::Off, Mode::Full]);
        assert_eq!(Mode::default(), Mode::Full);
    }

    #[test]
    fn long_text_wraps_at_spaces() {
        let w = wrap("this build of the stage does not carry the module 'space' (node space)", 20);
        assert_eq!(w, ["this build of the", "stage does not carry", "the module 'space'", "(node space)"]);
        assert_eq!(wrap("abcdefghij", 4), ["abcd"]);
        assert!(wrap("  ", 10).is_empty());
    }

    #[test]
    fn an_empty_report_still_gets_its_bar() {
        let s = Status { quality: Some(Quality { level: "high".into(), ceiling: "high".into(), auto: true, scale: 1.0, render: 1.0 }), ..Status::default() };
        let texts: Vec<String> = bar_items(&s).into_iter().map(|i| i.text).collect();
        assert_eq!(texts, ["F1"]);
    }
}
