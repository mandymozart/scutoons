//! Quality scaling (#220, owner 2026-10-31: "so the engine can turn things down at any time, and when the power is
//! there we turn it all the way up"; 2026-11-01: "a detailed settings page for each quality preset … where I can set
//! the screen resolution … know the max possible on the hardware and then scales of the rendered resolution so I can
//! save power"). Four **presets – low · medium · high · ultra** – each a set of knobs the deck's owner edits (`Presets`,
//! one TOML text: the stage's quality file, the browser's settings sheet): on / off, two render sizes (the step under
//! load and the preset's own – a scale of the display's pixels or a height in pixels), FXAA, bloom, how many beings
//! light the room, the frame rate cap. The enabled presets make the **ladder** of rungs, cheapest first. The deck's
//! level is the ceiling (chosen, or its default from the GPU); the governor steps below it under load and back up with
//! headroom. The same code steers the native stage and the browser deck (space-web).
//!
//! The governor is the adaptive audio lead's rule (deck-core lead.rs, #208 / #365) and the relay's link rate
//! (deck/relay/src/link.rs, #223): take the margin at once, give it back slowly.
//! - It looks at the drawn frames' interval in windows of `WINDOW_S`. A window whose mean is over the rung's budget
//!   (its frame rate cap × `BUDGET`, 20 ms at 60 fps), or with a new audio glitch, is over budget. `OVER_WINDOWS` in a
//!   row step one rung down (one window when it is twice over); after a step the next waits `STEP_GAP_S`.
//! - With vsync a frame never shows its headroom, so the way up is a probe: calm (each window at most the cap's
//!   interval × `CALM`) for the rung's calm time steps one rung up. A probe that runs over budget within `PROBE_S`
//!   steps back and doubles that rung's calm time (up to `MAX_CALM_S`) – a rung the deck can't hold is tried less and
//!   less often, so it never flickers.
//! - A frame interval over `HITCH_MS` (a set loading) is a skipped sample; `hold` ignores the next `HOLD_S` (the start,
//!   a known stall). A pause over `PAUSE_MS` (a hidden tab, a phone back from another app) holds `PAUSE_HOLD_S`: the
//!   device catches up after it (the S22: ~6 s of slow blocks and frames) – that is no load to step down on (#321).
//! - A rung's frame rate cap saves power: `due` tells the caller to skip a frame that comes early.
//!
//! Pure and clock-free (the caller passes `now` in seconds); no allocation per frame (deck-native tests/no_alloc.rs) –
//! only a new preset table builds a new ladder.
use serde::{Deserialize, Serialize};

/// a window whose mean interval is over the cap's interval × this is over budget (20 ms at 60 fps); twice that steps
/// at once
pub const BUDGET: f32 = 1.2;
/// a window at most the cap's interval × this is calm (a 60 fps display holds ~16.7, a little jitter allowed)
pub const CALM: f32 = 1.08;
/// the window the intervals are averaged over (s)
pub const WINDOW_S: f64 = 0.5;
/// over-budget windows in a row that step down (1 s sustained)
pub const OVER_WINDOWS: u32 = 2;
/// after a step down, at least this long until the next (s)
pub const STEP_GAP_S: f64 = 1.0;
/// a rung's calm time before the probe up (s), doubled by every failed probe up to `MAX_CALM_S`
pub const CALM_S: f64 = 8.0;
pub const MAX_CALM_S: f64 = 120.0;
/// a step up that runs over budget within this long failed (s)
pub const PROBE_S: f64 = 6.0;
/// a frame interval over this is a hitch, not load (ms)
pub const HITCH_MS: f32 = 250.0;
/// what `hold` and the start ignore (s)
pub const HOLD_S: f64 = 2.0;
/// a frame interval over this is a pause (the page was hidden), not a hitch (ms)
pub const PAUSE_MS: f32 = 1000.0;
/// what the governor ignores after a pause (s)
pub const PAUSE_HOLD_S: f64 = 6.0;
/// a report goes out at most this often unless the rung changed (s), and only when the frame time moved `REPORT_MS`
pub const REPORT_S: f64 = 2.0;
pub const REPORT_MS: f32 = 1.0;
/// the largest render size side (a supersampled 4K screen would pass WebGL2's texture limit)
pub const MAX_SIDE: u32 = 4096;
/// the beings that may light the room at most (space MAX_LIGHTS)
pub const MAX_LIGHTS: usize = 8;
/// a frame this early is skipped by the cap (ms – a display's refresh jitters)
const PACE_SLACK_MS: f32 = 3.0;

/// the levels, cheapest first – the hardware tiers S / M / L / XL (deck/HARDWARE.md)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Low,
    Medium,
    High,
    Ultra,
}

impl Level {
    pub const ALL: [Level; 4] = [Level::Low, Level::Medium, Level::High, Level::Ultra];

    pub fn name(self) -> &'static str {
        match self {
            Level::Low => "low",
            Level::Medium => "medium",
            Level::High => "high",
            Level::Ultra => "ultra",
        }
    }

    pub fn parse(s: &str) -> Option<Level> {
        Level::ALL.into_iter().find(|l| l.name().eq_ignore_ascii_case(s.trim()))
    }

    /// the next level up (ultra wraps to low – the stage's key cycles)
    pub fn next(self) -> Level {
        Level::ALL[(self as usize + 1) % Level::ALL.len()]
    }

    /// The deck's default by its GPU: a software renderer low, a browser's WebGL2 (a phone's Chrome – the S22) medium,
    /// WebGPU and an integrated GPU (the X1's UHD 620) high, a discrete GPU ultra. The governor steps below it.
    pub fn default_for(gpu: Gpu) -> Level {
        match gpu {
            Gpu::Software => Level::Low,
            Gpu::WebGl2 | Gpu::Other => Level::Medium,
            Gpu::WebGpu | Gpu::Integrated => Level::High,
            Gpu::Discrete => Level::Ultra,
        }
    }

    /// the protocol's level (`deck.quality`)
    #[cfg(feature = "being")]
    pub fn wire(self) -> protocol::QualityLevel {
        match self {
            Level::Low => protocol::QualityLevel::Low,
            Level::Medium => protocol::QualityLevel::Medium,
            Level::High => protocol::QualityLevel::High,
            Level::Ultra => protocol::QualityLevel::Ultra,
        }
    }
}

/// what the deck renders on (the native stage's wgpu adapter type, the browser's backend)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gpu {
    Software,
    Integrated,
    Discrete,
    WebGl2,
    WebGpu,
    Other,
}

/// The deck's choice on the command line (deck-native `--quality` / `DECK_QUALITY`): a level or `auto` (= the preset
/// file's, else the GPU's default), and `,fixed` for no governor – `high`, `low,fixed`, `auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    pub level: Option<Level>,
    /// false = `,fixed` was given
    pub auto: bool,
}

impl Default for Choice {
    fn default() -> Self {
        Choice { level: None, auto: true }
    }
}

impl Choice {
    pub fn parse(s: &str) -> Option<Choice> {
        let mut c = Choice::default();
        for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            match part.to_ascii_lowercase().as_str() {
                "auto" => {}
                "fixed" => c.auto = false,
                other => c.level = Some(Level::parse(other)?),
            }
        }
        Some(c)
    }

    /// the level for a deck on `gpu`
    pub fn level_on(&self, gpu: Gpu) -> Level {
        self.level.unwrap_or(Level::default_for(gpu))
    }

    /// the command line over the preset file: its level when given, `,fixed` turns the governor off
    pub fn over(&self, p: &mut Presets) {
        if self.level.is_some() {
            p.level = self.level;
        }
        if !self.auto {
            p.auto = false;
        }
    }
}

/// A render size: × the display's pixels (a number ≤ 4 – 0.5, 1.25) or a height in pixels (≥ 100 – 720, 1080; the
/// width follows the display's aspect). In the file and the sheet it is that one number.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(into = "f64", try_from = "f64")]
pub enum Size {
    Scale(f32),
    Height(u32),
}

impl From<Size> for f64 {
    fn from(s: Size) -> f64 {
        match s {
            Size::Scale(x) => (x as f64 * 1000.0).round() / 1000.0,
            Size::Height(h) => h as f64,
        }
    }
}

impl TryFrom<f64> for Size {
    type Error = String;
    fn try_from(v: f64) -> Result<Size, String> {
        if (0.1..=4.0).contains(&v) {
            Ok(Size::Scale(v as f32))
        } else if (100.0..=MAX_SIDE as f64).contains(&v) {
            Ok(Size::Height(v.round() as u32))
        } else {
            Err(format!("{v}: a scale 0.1–4 or a height 100–{MAX_SIDE} px"))
        }
    }
}

impl Size {
    /// × the display's pixels on a display `display_h` high
    pub fn scale_for(self, display_h: u32) -> f32 {
        match self {
            Size::Scale(s) => s,
            Size::Height(h) => h as f32 / display_h.max(1) as f32,
        }
    }
}

/// the 3D's size for a display of `w` × `h` pixels at `size` (each side 1 …  `MAX_SIDE`)
pub fn render_size((w, h): (u32, u32), size: Size) -> (u32, u32) {
    let s = size.scale_for(h);
    let side = |v: u32| ((v as f32 * s).round() as u32).clamp(1, MAX_SIDE);
    (side(w), side(h))
}

/// One preset's knobs (the settings sheet's column).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    /// off = the governor skips it and it can't be chosen (a trial without it)
    pub on: bool,
    /// the render size under load (the preset's lower rung) and its own
    pub min: Size,
    pub max: Size,
    pub fxaa: bool,
    pub bloom: bool,
    /// how many beings light the room at most (0 … 8)
    pub lights: usize,
    /// the frame rate cap (power: 30 draws half the frames)
    pub fps: u32,
}

const fn preset(min: f32, max: f32, aa: bool, lights: usize) -> Preset {
    Preset { on: true, min: Size::Scale(min), max: Size::Scale(max), fxaa: aa, bloom: aa, lights, fps: 60 }
}

/// The deck's quality settings: its level (missing = by the GPU), the governor, the display mode (native: a fullscreen
/// video mode "WxH"; missing = the desktop's), the browser's device pixels per CSS pixel at most, and the presets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Presets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<Level>,
    pub auto: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_ratio: Option<f32>,
    pub low: Preset,
    pub medium: Preset,
    pub high: Preset,
    pub ultra: Preset,
}

impl Default for Presets {
    /// the defaults: the render scale goes first (the cheapest visual loss), then FXAA, bloom and the lights
    fn default() -> Self {
        Presets {
            level: None,
            auto: true,
            display: None,
            pixel_ratio: None,
            low: preset(0.5, 0.6, false, 2),
            medium: preset(0.7, 0.8, true, 4),
            high: preset(0.9, 1.0, true, 8),
            ultra: preset(1.25, 1.25, true, 8),
        }
    }
}

/// the head of the file the stage writes
const HEADER: &str = "# SCUTOONS deck quality (#220, deck/native/README.md \"Quality\"): the level (low · medium · high · ultra; missing =\n# by the GPU), the governor (auto), the display mode (\"WxH\", native), the browser's pixel ratio cap, and per preset:\n# on, min / max = the render size under load / its own (a scale ≤ 4, or a height in px ≥ 100), fxaa, bloom, lights\n# (0–8), fps (the cap). The stage reloads this file when it changes.\n";

impl Presets {
    pub fn get(&self, l: Level) -> &Preset {
        match l {
            Level::Low => &self.low,
            Level::Medium => &self.medium,
            Level::High => &self.high,
            Level::Ultra => &self.ultra,
        }
    }

    pub fn get_mut(&mut self, l: Level) -> &mut Preset {
        match l {
            Level::Low => &mut self.low,
            Level::Medium => &mut self.medium,
            Level::High => &mut self.high,
            Level::Ultra => &mut self.ultra,
        }
    }

    /// The text (TOML): what is missing keeps its default, so a file may name only what it changes. Values out of
    /// range are clamped (lights 0–8, fps 10–240); a preset's `max` below its `min` becomes `min`.
    pub fn parse(text: &str) -> Result<Presets, String> {
        let given: toml::Table = text.parse().map_err(|e: toml::de::Error| e.message().to_string())?;
        let mut all = toml::Table::try_from(Presets::default()).map_err(|e| e.to_string())?;
        merge(&mut all, given);
        let p: Presets = all.try_into().map_err(|e: toml::de::Error| e.message().to_string())?;
        Ok(p.clamped())
    }

    /// the text the stage writes (all keys, a header)
    pub fn to_text(&self) -> String {
        format!("{HEADER}{}", toml::to_string(self).unwrap_or_default())
    }

    fn clamped(mut self) -> Presets {
        for l in Level::ALL {
            let p = self.get_mut(l);
            p.lights = p.lights.min(MAX_LIGHTS);
            p.fps = p.fps.clamp(10, 240);
        }
        self.pixel_ratio = self.pixel_ratio.map(|r| r.clamp(0.5, 4.0));
        self.display = self.display.filter(|d| display_mode(d).is_some());
        self
    }

    /// The ladder: each enabled preset's lower rung, then its own (once when they're the same), cheapest first; with
    /// every preset off, the default high's.
    pub fn ladder(&self) -> Vec<Rung> {
        let mut out = Vec::with_capacity(8);
        for l in Level::ALL {
            let p = self.get(l);
            if !p.on {
                continue;
            }
            let rung = |size| Rung { level: l, size, bloom: p.bloom, fxaa: p.fxaa, lights: p.lights, fps: p.fps };
            if p.min != p.max {
                out.push(rung(p.min));
            }
            out.push(rung(p.max));
        }
        if out.is_empty() {
            let h = Presets::default().high;
            out.push(Rung { level: Level::High, size: h.max, bloom: h.bloom, fxaa: h.fxaa, lights: h.lights, fps: h.fps });
        }
        out
    }

    /// the display mode (native), parsed
    pub fn display_mode(&self) -> Option<(u32, u32)> {
        self.display.as_deref().and_then(display_mode)
    }
}

/// "1920x1080" (or ×) → (1920, 1080)
pub fn display_mode(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.trim().split_once(['x', 'X', '×'])?;
    let (w, h) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    ((1..=16384).contains(&w) && (1..=16384).contains(&h)).then_some((w, h))
}

/// `given`'s keys over `base` (tables merge, values replace)
fn merge(base: &mut toml::Table, given: toml::Table) {
    for (k, v) in given {
        match (base.get_mut(&k), v) {
            (Some(toml::Value::Table(b)), toml::Value::Table(g)) => merge(b, g),
            (_, v) => {
                base.insert(k, v);
            }
        }
    }
}

/// One rung of the ladder: the knobs at this step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rung {
    pub level: Level,
    /// the 3D's render size (the HUD and the page's UI stay native)
    pub size: Size,
    /// the post passes this rung allows (a pass the scene doesn't list stays off anyway)
    pub bloom: bool,
    pub fxaa: bool,
    pub lights: usize,
    /// the frame rate cap
    pub fps: u32,
}

impl Rung {
    /// the cap's frame interval (ms)
    pub fn interval_ms(&self) -> f32 {
        1000.0 / self.fps.max(1) as f32
    }
}

impl Default for Rung {
    /// the default high preset's own rung (before a governor decides)
    fn default() -> Self {
        let h = Presets::default().high;
        Rung { level: Level::High, size: h.max, bloom: h.bloom, fxaa: h.fxaa, lights: h.lights, fps: h.fps }
    }
}

/// What a deck reports (the conductor sees it next to its link – protocol `deck.quality`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Report {
    pub level: Level,
    /// the level chosen (the governor's ceiling)
    pub ceiling: Level,
    /// × the display's pixels now
    pub scale: f32,
    pub auto: bool,
    /// the mean frame interval of the last window (ms)
    pub frame_ms: f32,
}

#[cfg(feature = "being")]
impl Report {
    /// `deck.quality` for the relay (the frame time to 0.1 ms)
    pub fn wire(&self) -> protocol::DeckQuality {
        protocol::DeckQuality {
            level: self.level.wire(),
            scale: (self.scale as f64 * 100.0).round() / 100.0,
            auto: self.auto,
            frame: (self.frame_ms as f64 * 10.0).round() / 10.0,
            ceiling: Some(self.ceiling.wire()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Governor {
    ladder: Vec<Rung>,
    auto: bool,
    /// the richest rung allowed (the level's top) and the rung now
    ceiling: usize,
    rung: usize,
    /// the display's size (a fixed render height's scale)
    display: (u32, u32),
    /// the window: its start, frames, summed intervals, whether a glitch fell into it
    win_start: Option<f64>,
    win_frames: u32,
    win_ms: f64,
    win_glitch: bool,
    glitches: u64,
    /// over-budget windows in a row, the last step down, since when it is calm
    over: u32,
    last_down: Option<f64>,
    calm_since: Option<f64>,
    /// the last probe: the rung it reached and when
    probe: Option<(usize, f64)>,
    /// per rung: the calm time before a probe up to it
    calm_need: Vec<f64>,
    hold_until: f64,
    /// the last window's mean (ms) and what was last reported
    frame_ms: f32,
    reported: Option<(Report, f64)>,
}

impl Governor {
    /// the default presets at `level`'s top, the governor on (`auto`) or off; it ignores the first `HOLD_S`
    pub fn new(level: Level, auto: bool) -> Governor {
        let p = Presets { level: Some(level), auto, ..Presets::default() };
        Governor::with(&p, Gpu::Other)
    }

    /// `presets` on a deck with `gpu` (its default level when the presets name none)
    pub fn with(presets: &Presets, gpu: Gpu) -> Governor {
        let mut g = Governor {
            ladder: Vec::new(),
            auto: presets.auto,
            ceiling: 0,
            rung: 0,
            display: (1920, 1080),
            win_start: None,
            win_frames: 0,
            win_ms: 0.0,
            win_glitch: false,
            glitches: 0,
            over: 0,
            last_down: None,
            calm_since: None,
            probe: None,
            calm_need: Vec::new(),
            hold_until: f64::NEG_INFINITY,
            frame_ms: 0.0,
            reported: None,
        };
        g.set_presets(presets, gpu);
        g
    }

    /// New presets (the file changed, the sheet): the new ladder, the level they name (else the GPU's default) at its
    /// top, their governor setting; the calm times start over.
    pub fn set_presets(&mut self, presets: &Presets, gpu: Gpu) {
        self.ladder = presets.ladder();
        self.calm_need = vec![CALM_S; self.ladder.len()];
        self.auto = presets.auto;
        self.set_level(presets.level.unwrap_or(Level::default_for(gpu)));
    }

    /// the display's size in pixels (a preset with a height in pixels scales by it)
    pub fn set_display(&mut self, w: u32, h: u32) {
        self.display = (w.max(1), h.max(1));
    }

    pub fn rung(&self) -> &Rung {
        &self.ladder[self.rung]
    }

    pub fn index(&self) -> usize {
        self.rung
    }

    pub fn ladder(&self) -> &[Rung] {
        &self.ladder
    }

    /// × the display's pixels now
    pub fn scale(&self) -> f32 {
        self.rung().size.scale_for(self.display.1)
    }

    /// the 3D's render size now
    pub fn render_size(&self) -> (u32, u32) {
        render_size(self.display, self.rung().size)
    }

    /// the level the deck is at now (the rung's)
    pub fn level(&self) -> Level {
        self.ladder[self.rung].level
    }

    /// the level chosen (the ceiling – an enabled one)
    pub fn ceiling(&self) -> Level {
        self.ladder[self.ceiling].level
    }

    pub fn auto(&self) -> bool {
        self.auto
    }

    /// the last window's mean frame interval (ms; 0 before the first)
    pub fn frame_ms(&self) -> f32 {
        self.frame_ms
    }

    /// A level by hand: its top rung now (a level that is off: the nearest enabled one below, else the lowest), every
    /// rung's calm time back to `CALM_S`.
    pub fn set_level(&mut self, level: Level) {
        self.ceiling = self.ladder.iter().rposition(|r| r.level <= level).unwrap_or(0);
        self.rung = self.ceiling;
        self.calm_need.iter_mut().for_each(|c| *c = CALM_S);
        self.reset_window();
        self.over = 0;
        self.probe = None;
        self.calm_since = None;
    }

    /// the governor on or off (off: back at the level's top, and it stays there)
    pub fn set_auto(&mut self, auto: bool) {
        self.auto = auto;
        if !auto {
            self.rung = self.ceiling;
        }
        self.over = 0;
        self.probe = None;
        self.calm_since = None;
    }

    /// ignore the frames of the next `HOLD_S` from `now` (a known stall: the set arrived, the scene changed)
    pub fn hold(&mut self, now: f64) {
        self.hold_until = now + HOLD_S;
        self.reset_window();
        self.calm_since = None;
    }

    fn reset_window(&mut self) {
        self.win_start = None;
        self.win_frames = 0;
        self.win_ms = 0.0;
        self.win_glitch = false;
    }

    /// The frame rate cap: draw now, `since_ms` after the last drawn frame? (false = skip it – the power the cap saves)
    pub fn due(&self, since_ms: f32) -> bool {
        since_ms + PACE_SLACK_MS >= self.rung().interval_ms()
    }

    /// One drawn frame at `now` (s), `dt_ms` after the last drawn one; `glitches` = the audio's running glitch count
    /// (0 without audio). Returns whether the rung changed (the caller applies `rung()` – a resize only then).
    pub fn frame(&mut self, now: f64, dt_ms: f32, glitches: u64) -> bool {
        let glitched = glitches > self.glitches;
        self.glitches = glitches;
        if self.win_start.is_none() && self.hold_until == f64::NEG_INFINITY {
            // the very first frame: the start is a stall (shaders compile, the set loads)
            self.hold_until = now + HOLD_S;
        }
        if now < self.hold_until {
            self.reset_window();
            return false;
        }
        // back from a pause: what follows is the device catching up
        if dt_ms > PAUSE_MS {
            self.hold(now);
            self.hold_until = now + PAUSE_HOLD_S;
            return false;
        }
        // a hitch is no sample (it doesn't end the window either: frequent hitches would starve the governor)
        if !(0.0..=HITCH_MS).contains(&dt_ms) {
            return false;
        }
        let start = *self.win_start.get_or_insert(now);
        self.win_frames += 1;
        self.win_ms += dt_ms as f64;
        self.win_glitch |= glitched;
        if now - start < WINDOW_S {
            return false;
        }
        let mean = (self.win_ms / self.win_frames as f64) as f32;
        let glitch = self.win_glitch;
        self.frame_ms = mean;
        self.reset_window();
        self.window(now, mean, glitch)
    }

    /// a closed window: its mean interval and whether the audio glitched in it
    fn window(&mut self, now: f64, mean: f32, glitch: bool) -> bool {
        if !self.auto {
            return false;
        }
        let budget = self.rung().interval_ms() * BUDGET;
        let over = mean > budget || glitch;
        if over {
            self.calm_since = None;
            self.over += 1;
            // a probe that runs over: that rung waits twice as long next time
            if let Some((r, at)) = self.probe.take() {
                if r == self.rung && now - at <= PROBE_S {
                    self.calm_need[r] = (self.calm_need[r] * 2.0).min(MAX_CALM_S);
                    return self.step_down(now);
                }
            }
            let sustained = self.over >= OVER_WINDOWS || mean > budget * 2.0;
            let may = self.last_down.map_or(true, |t| now - t >= STEP_GAP_S);
            return sustained && may && self.step_down(now);
        }
        self.over = 0;
        if mean > self.rung().interval_ms() * CALM {
            // neither over nor calm: hold the rung
            self.calm_since = None;
            return false;
        }
        if self.probe.is_some_and(|(_, at)| now - at > PROBE_S) {
            self.probe = None;
        }
        let since = *self.calm_since.get_or_insert(now);
        if self.rung < self.ceiling && now - since >= self.calm_need[self.rung + 1] {
            self.rung += 1;
            self.probe = Some((self.rung, now));
            self.calm_since = Some(now);
            return true;
        }
        false
    }

    fn step_down(&mut self, now: f64) -> bool {
        self.over = 0;
        self.calm_since = None;
        if self.rung == 0 {
            return false;
        }
        self.rung -= 1;
        self.last_down = Some(now);
        true
    }

    pub fn report(&self) -> Report {
        Report { level: self.level(), ceiling: self.ceiling(), scale: self.scale(), auto: self.auto, frame_ms: self.frame_ms }
    }

    /// What to send at `now`, if anything: at once when the level, scale or auto changed; else at most every
    /// `REPORT_S` when the frame time moved `REPORT_MS` (the conductor's presence isn't rebuilt for jitter).
    pub fn report_due(&mut self, now: f64) -> Option<Report> {
        let r = self.report();
        let due = match &self.reported {
            None => true,
            Some((last, at)) => (last.level, last.ceiling, last.scale, last.auto) != (r.level, r.ceiling, r.scale, r.auto) || (now - at >= REPORT_S && (last.frame_ms - r.frame_ms).abs() >= REPORT_MS),
        };
        if due {
            self.reported = Some((r, now));
        }
        due.then_some(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// frames of `ms` each from `*t` for `secs` – from a fresh window (the load changes on a window's edge, so the
    /// counts don't depend on the phase); returns how often the rung changed
    fn run(g: &mut Governor, t: &mut f64, secs: f64, ms: f32) -> usize {
        g.reset_window();
        let mut changes = 0;
        let end = *t + secs;
        while *t < end {
            *t += ms as f64 / 1000.0;
            changes += g.frame(*t, ms, 0) as usize;
        }
        changes
    }

    fn top(g: &Governor, l: Level) -> usize {
        g.ladder().iter().rposition(|r| r.level == l).unwrap()
    }

    #[test]
    fn the_default_ladder_runs_cheapest_first_and_each_level_has_its_rungs() {
        let ladder = Presets::default().ladder();
        assert_eq!(ladder.len(), 7);
        let scale = |r: &Rung| r.size.scale_for(1080);
        for w in ladder.windows(2) {
            assert!(w[0].level <= w[1].level && scale(&w[0]) < scale(&w[1]), "{w:?}");
            assert!(w[0].lights <= w[1].lights && (!w[0].bloom || w[1].bloom) && (!w[0].fxaa || w[1].fxaa));
        }
        let g = Governor::new(Level::High, true);
        assert_eq!(g.scale(), 1.0, "high = native");
        assert_eq!(Rung::default(), ladder[5]);
        assert!(Governor::new(Level::Ultra, true).scale() > 1.0, "ultra supersamples");
        assert_eq!(Level::parse(" ULTRA"), Some(Level::Ultra));
        assert_eq!(Level::Ultra.next(), Level::Low);
    }

    #[test]
    fn the_defaults_per_gpu_and_the_command_line() {
        assert_eq!(Level::default_for(Gpu::Software), Level::Low);
        assert_eq!(Level::default_for(Gpu::WebGl2), Level::Medium, "the S22's Chrome");
        assert_eq!(Level::default_for(Gpu::Integrated), Level::High, "the X1's UHD 620");
        assert_eq!(Level::default_for(Gpu::WebGpu), Level::High);
        assert_eq!(Level::default_for(Gpu::Discrete), Level::Ultra);
        assert_eq!(Choice::parse("auto"), Some(Choice::default()));
        assert_eq!(Choice::parse("high"), Some(Choice { level: Some(Level::High), auto: true }));
        assert_eq!(Choice::parse("Low, fixed"), Some(Choice { level: Some(Level::Low), auto: false }));
        assert_eq!(Choice::parse("fast"), None);
        assert_eq!(Choice::parse("fixed").unwrap().level_on(Gpu::Integrated), Level::High);
        let mut p = Presets { level: Some(Level::Medium), ..Presets::default() };
        Choice::parse("auto").unwrap().over(&mut p);
        assert_eq!((p.level, p.auto), (Some(Level::Medium), true), "auto keeps the file's");
        Choice::parse("ultra,fixed").unwrap().over(&mut p);
        assert_eq!((p.level, p.auto), (Some(Level::Ultra), false));
        assert_eq!(Governor::with(&Presets::default(), Gpu::Software).ceiling(), Level::Low, "no level in the file: the GPU's");
    }

    #[test]
    fn the_render_size_by_scale_or_height() {
        assert_eq!(render_size((1920, 1080), Size::Scale(0.5)), (960, 540));
        assert_eq!(render_size((1920, 1080), Size::Scale(1.25)), (2400, 1350));
        assert_eq!(render_size((3840, 2160), Size::Scale(1.25)), (4096, 2700), "capped");
        assert_eq!(render_size((1, 1), Size::Scale(0.5)), (1, 1));
        // a 4K screen rendering 1080p: a quarter of the pixels
        assert_eq!(render_size((3840, 2160), Size::Height(1080)), (1920, 1080));
        assert_eq!(render_size((1080, 2340), Size::Height(1170)), (540, 1170), "a phone upright");
        assert_eq!(Size::Height(720).scale_for(1440), 0.5);
    }

    #[test]
    fn the_preset_file_round_trips_and_names_only_what_it_changes() {
        let p = Presets::default();
        assert_eq!(Presets::parse(&p.to_text()).unwrap(), p);
        let q = Presets::parse("level = \"medium\"\ndisplay = \"3840x2160\"\n[medium]\nmax = 1080\nbloom = false\nfps = 30\n[ultra]\non = false\nlights = 99\n").unwrap();
        assert_eq!(q.level, Some(Level::Medium));
        assert_eq!(q.display_mode(), Some((3840, 2160)));
        assert_eq!((q.medium.min, q.medium.max, q.medium.bloom, q.medium.fxaa, q.medium.fps), (Size::Scale(0.7), Size::Height(1080), false, true, 30));
        assert_eq!((q.ultra.on, q.ultra.lights), (false, MAX_LIGHTS), "clamped");
        assert!(q.to_text().contains("max = 1080"));
        assert!(Presets::parse("[low]\nmin = 7\n").unwrap_err().contains("scale 0.1"));
        assert!(Presets::parse("level = \"insane\"").is_err());
        assert_eq!(Presets::parse("display = \"big\"").unwrap().display, None, "a mode that isn't WxH is dropped");
        assert_eq!(display_mode("2560 × 1440"), Some((2560, 1440)));
    }

    #[test]
    fn presets_that_are_off_leave_the_ladder() {
        let mut p = Presets::default();
        p.medium.on = false;
        p.ultra.on = false;
        p.high.min = p.high.max;
        let ladder = p.ladder();
        assert_eq!(ladder.iter().map(|r| r.level).collect::<Vec<_>>(), [Level::Low, Level::Low, Level::High]);
        let mut g = Governor::with(&Presets { level: Some(Level::Medium), ..p.clone() }, Gpu::Other);
        assert_eq!(g.ceiling(), Level::Low, "medium is off: the nearest below");
        g.set_level(Level::Ultra);
        assert_eq!(g.ceiling(), Level::High);
        for l in Level::ALL {
            p.get_mut(l).on = false;
        }
        assert_eq!(p.ladder(), [Rung::default()], "all off: the default high");
    }

    #[test]
    fn sustained_overload_steps_down_fast_one_rung_per_gap() {
        let mut g = Governor::new(Level::High, true);
        let mut t = 0.0;
        assert_eq!(run(&mut g, &mut t, 3.0, 16.7), 0, "the start holds, then calm at the top");
        assert_eq!(g.index(), top(&g, Level::High));
        // 40 fps: 2 windows (1 s) step one rung, then one per second
        assert_eq!(run(&mut g, &mut t, 0.9, 25.0), 0, "one window over: nothing yet");
        assert_eq!(run(&mut g, &mut t, 1.1, 25.0), 1, "a sustained second (two windows): a step");
        assert_eq!(g.index(), top(&g, Level::High) - 1);
        assert_eq!(run(&mut g, &mut t, 1.2, 25.0), 1, "and the next one a second later");
        assert_eq!(g.level(), Level::Medium);
        run(&mut g, &mut t, 20.0, 25.0);
        assert_eq!(g.index(), 0, "down to the floor, never below");
        assert_eq!(g.frame_ms(), 25.0);
    }

    #[test]
    fn a_severe_window_steps_at_once_and_a_hitch_is_no_load() {
        let mut g = Governor::new(Level::High, true);
        let mut t = 0.0;
        run(&mut g, &mut t, 3.0, 16.7);
        assert_eq!(run(&mut g, &mut t, 0.6, 50.0), 1, "20 fps: one window is enough");
        let r = g.index();
        // one 400 ms stall (a set arriving) between good frames changes nothing
        t += 0.4;
        assert!(!g.frame(t, 400.0, 0));
        assert_eq!(run(&mut g, &mut t, 1.0, 16.7), 0);
        assert_eq!(g.index(), r);
        // a load with a hitch every 15 frames still steps down: the hitches are skipped, the windows close
        let mut n = 0;
        for f in 0..90 {
            let ms = if f % 15 == 14 { 300.0 } else { 25.0 };
            t += ms as f64 / 1000.0;
            n += g.frame(t, ms, 0) as usize;
        }
        assert!(n >= 1, "{}", g.frame_ms());
    }

    #[test]
    fn back_from_a_hidden_page_the_catching_up_is_no_load() {
        let mut g = Governor::new(Level::High, true);
        let mut t = 0.0;
        run(&mut g, &mut t, 3.0, 16.7);
        let r = g.index();
        // 20 s hidden, then the phone catches up: 5 s at 30 fps steps nothing
        t += 20.0;
        assert!(!g.frame(t, 20_000.0, 0));
        assert_eq!(run(&mut g, &mut t, PAUSE_HOLD_S - 1.0, 33.0), 0);
        assert_eq!(g.index(), r);
        // a load that stays past the hold is load again
        assert!(run(&mut g, &mut t, 3.0, 33.0) >= 1);
    }

    #[test]
    fn calm_steps_up_slowly_and_a_failed_probe_doubles_its_wait() {
        let mut g = Governor::new(Level::High, true);
        let mut t = 0.0;
        run(&mut g, &mut t, 3.0, 16.7);
        run(&mut g, &mut t, 1.1, 25.0);
        let low = g.index();
        assert_eq!(low, top(&g, Level::High) - 1);
        assert_eq!(run(&mut g, &mut t, CALM_S - 1.0, 16.7), 0, "not calm long enough");
        assert_eq!(run(&mut g, &mut t, 2.0, 16.7), 1, "calm for CALM_S: the probe up");
        assert_eq!(g.index(), low + 1);
        // the probe runs over at once: back down, and that rung now waits twice as long
        assert_eq!(run(&mut g, &mut t, 0.55, 25.0), 1);
        assert_eq!(g.index(), low);
        assert_eq!(run(&mut g, &mut t, CALM_S * 2.0 - 1.0, 16.7), 0, "the doubled wait");
        assert_eq!(run(&mut g, &mut t, 2.0, 16.7), 1);
        // holds this time: stays up, at the ceiling – never above
        assert_eq!(run(&mut g, &mut t, 60.0, 16.7), 0);
        assert_eq!(g.index(), top(&g, Level::High));
    }

    #[test]
    fn between_calm_and_budget_it_holds() {
        let mut g = Governor::new(Level::High, true);
        let mut t = 0.0;
        run(&mut g, &mut t, 3.0, 16.7);
        run(&mut g, &mut t, 1.1, 25.0);
        let r = g.index();
        assert_eq!(run(&mut g, &mut t, 60.0, 19.0), 0, "52 fps: neither over nor calm");
        assert_eq!(g.index(), r);
    }

    #[test]
    fn an_audio_glitch_counts_as_over_budget() {
        let mut g = Governor::new(Level::Medium, true);
        let mut t = 0.0;
        run(&mut g, &mut t, 3.0, 16.7);
        let top = g.index();
        g.reset_window();
        let mut n = 0;
        for i in 0..70 {
            t += 0.0167;
            n += g.frame(t, 16.7, i / 20) as usize;
        }
        assert_eq!(n, 1, "glitching windows step down even at 60 fps");
        assert_eq!(g.index(), top - 1);
    }

    #[test]
    fn a_30_fps_cap_skips_early_frames_and_is_its_own_budget() {
        let mut p = Presets { level: Some(Level::Low), ..Presets::default() };
        p.low.fps = 30;
        let mut g = Governor::with(&p, Gpu::Other);
        // a 60 Hz display: every other frame is drawn
        assert!(!g.due(16.7) && g.due(33.3) && g.due(31.0));
        // 33 ms frames are calm at 30 fps, 45 ms is over its 40 ms budget
        let mut t = 0.0;
        assert_eq!(run(&mut g, &mut t, 3.0, 33.3), 0, "33 ms at a 30 fps cap: no load");
        let r = g.index();
        assert_eq!(run(&mut g, &mut t, 1.2, 45.0), 1, "45 ms: over the cap's 40 ms budget");
        assert_eq!(run(&mut g, &mut t, 20.0, 33.3), 1, "calm at the cap: back up");
        assert_eq!(g.index(), r);
    }

    #[test]
    fn off_or_by_hand_it_stays_at_the_level() {
        let mut g = Governor::new(Level::Ultra, false);
        let mut t = 0.0;
        assert_eq!(run(&mut g, &mut t, 10.0, 40.0), 0, "off: never steps");
        assert_eq!(g.level(), Level::Ultra);
        assert_eq!(g.frame_ms(), 40.0, "but measures");
        g.set_auto(true);
        run(&mut g, &mut t, 1.3, 40.0);
        assert_eq!(g.level(), Level::High);
        g.set_level(Level::Low);
        assert_eq!((g.index(), g.ceiling()), (top(&g, Level::Low), Level::Low));
        run(&mut g, &mut t, 60.0, 16.7);
        assert_eq!(g.index(), top(&g, Level::Low), "never above the chosen level");
        g.set_level(Level::High);
        run(&mut g, &mut t, 1.3, 40.0);
        g.set_auto(false);
        assert_eq!(g.index(), top(&g, Level::High), "off: back at the level's top");
    }

    #[test]
    fn a_fixed_height_reports_its_scale_of_the_display() {
        let mut p = Presets { level: Some(Level::High), ..Presets::default() };
        p.high.max = Size::Height(1080);
        let mut g = Governor::with(&p, Gpu::Other);
        g.set_display(3840, 2160);
        assert_eq!((g.render_size(), g.scale()), ((1920, 1080), 0.5));
        g.set_display(1280, 720);
        assert_eq!(g.render_size(), (1920, 1080), "a small screen supersamples");
    }

    #[test]
    fn reports_go_out_on_change_and_rarely_for_jitter() {
        let mut g = Governor::new(Level::High, true);
        let mut t = 0.0;
        assert!(g.report_due(0.0).is_some(), "the first");
        assert!(g.report_due(0.1).is_none());
        run(&mut g, &mut t, 3.0, 16.7);
        assert_eq!(g.report_due(t).map(|r| r.frame_ms), Some(16.7), "the frame time moved after REPORT_S");
        run(&mut g, &mut t, 3.0, 16.9);
        assert!(g.report_due(t).is_none(), "jitter under REPORT_MS");
        run(&mut g, &mut t, 1.1, 25.0);
        let r = g.report_due(t).expect("the rung changed");
        assert_eq!((r.level, r.scale, r.auto), (Level::High, 0.9, true));
        g.set_auto(false);
        assert!(g.report_due(t).is_some_and(|r| !r.auto), "auto changed: at once");
        let w = g.report().wire();
        assert_eq!((w.level, w.ceiling, w.auto, w.scale), (protocol::QualityLevel::High, Some(protocol::QualityLevel::High), false, 1.0));
    }
}
