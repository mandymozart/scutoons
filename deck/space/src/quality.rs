//! Quality scaling (#220, owner 2026-10-31: "so the engine can turn things down at any time, and when the power is
//! there we turn it all the way up"): one ladder of rungs from the cheapest picture to the richest – each rung a set of
//! knobs (render scale, bloom, FXAA, how many beings light the room) – grouped into the levels **low · medium · high ·
//! ultra**. The deck's level is the ceiling (by hand, or its default from the GPU); the governor steps below it under
//! load and back up with headroom. The same code steers the native stage and the browser deck (space-web).
//!
//! The governor is the adaptive audio lead's rule (deck/stage/src/audio/lead.rs, #208) and the relay's link rate
//! (deck/relay/src/link.rs, #223): take the margin at once, give it back slowly.
//! - It looks at the frame interval in windows of `WINDOW_S`. A window whose mean is over `BUDGET_MS` (under 50 fps),
//!   or with a new audio glitch, is over budget. `OVER_WINDOWS` in a row step one rung down (one window when it is
//!   far over, `SEVERE_MS`); after a step the next waits `STEP_GAP_S` – the new size settles first.
//! - With vsync a frame never shows its headroom (16.7 ms at every rung), so the way up is a probe: `calm` (each window
//!   at most `CALM_MS`) for the rung's calm time steps one rung up. A probe that runs over budget within `PROBE_S`
//!   steps back and doubles that rung's calm time (up to `MAX_CALM_S`) – a rung the deck can't hold is tried less
//!   and less often, so it never flickers.
//! - A hitch longer than `HITCH_MS` (a hidden tab, a set loading, a scene change) is no sample; `hold` ignores the next
//!   `HOLD_S` (the start, a known stall).
//!
//! Pure and clock-free (the caller passes `now` in seconds), no allocation per frame (deck-stage tests/no_alloc.rs).

/// the frame interval the governor aims at (60 fps)
pub const TARGET_MS: f32 = 1000.0 / 60.0;
/// a window whose mean interval is over this is over budget (< 50 fps)
pub const BUDGET_MS: f32 = 20.0;
/// one window this far over (< 25 fps) steps down at once
pub const SEVERE_MS: f32 = 40.0;
/// a window at most this is calm (a 60 fps display holds ~16.7, a little jitter allowed)
pub const CALM_MS: f32 = 18.0;
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
/// a report goes out at most this often unless the rung changed (s), and only when the frame time moved `REPORT_MS`
pub const REPORT_S: f64 = 2.0;
pub const REPORT_MS: f32 = 1.0;

/// the levels, cheapest first – the hardware tiers S / M / L / XL (deck/HARDWARE.md)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    /// the level's richest rung (the governor's ceiling)
    pub fn top(self) -> usize {
        LADDER.iter().rposition(|r| r.level == self).unwrap_or(LADDER.len() - 1)
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

/// One rung: the knobs at this step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rung {
    pub level: Level,
    /// the 3D's render size × the screen's (the HUD and the page's UI stay native); > 1 supersamples
    pub scale: f32,
    /// the post passes this rung allows (a pass the scene doesn't list stays off anyway)
    pub bloom: bool,
    pub fxaa: bool,
    /// how many beings light the room at most (#204; space MAX_LIGHTS = 8)
    pub lights: usize,
}

const fn rung(level: Level, scale: f32, bloom: bool, fxaa: bool, lights: usize) -> Rung {
    Rung { level, scale, bloom, fxaa, lights }
}

/// The ladder, cheapest first: the render scale goes first (the cheapest visual loss), then FXAA, bloom and the lights.
pub const LADDER: [Rung; 7] = [
    rung(Level::Low, 0.5, false, false, 2),
    rung(Level::Low, 0.6, false, true, 2),
    rung(Level::Medium, 0.7, true, true, 4),
    rung(Level::Medium, 0.8, true, true, 4),
    rung(Level::High, 0.9, true, true, 8),
    rung(Level::High, 1.0, true, true, 8),
    rung(Level::Ultra, 1.25, true, true, 8),
];

/// the largest render size side (a supersampled 4K screen would pass WebGL2's texture limit)
pub const MAX_SIDE: u32 = 4096;

/// the 3D's size for a screen of `w` × `h` pixels at `scale`
pub fn render_size((w, h): (u32, u32), scale: f32) -> (u32, u32) {
    let side = |v: u32| ((v as f32 * scale).round() as u32).clamp(1, MAX_SIDE);
    (side(w), side(h))
}

/// What a deck reports (the conductor sees it next to its link – protocol `deck.quality`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Report {
    pub level: Level,
    pub scale: f32,
    pub auto: bool,
    /// the mean frame interval of the last window (ms)
    pub frame_ms: f32,
}

#[derive(Debug, Clone)]
pub struct Governor {
    auto: bool,
    /// the richest rung allowed (the level's top) and the rung now
    ceiling: usize,
    rung: usize,
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
    calm_need: [f64; LADDER.len()],
    hold_until: f64,
    /// the last window's mean (ms) and what was last reported
    frame_ms: f32,
    reported: Option<(Report, f64)>,
}

impl Governor {
    /// at `level`'s top, the governor on (`auto`) or off; it ignores the first `HOLD_S` from the first frame
    pub fn new(level: Level, auto: bool) -> Governor {
        let top = level.top();
        Governor {
            auto,
            ceiling: top,
            rung: top,
            win_start: None,
            win_frames: 0,
            win_ms: 0.0,
            win_glitch: false,
            glitches: 0,
            over: 0,
            last_down: None,
            calm_since: None,
            probe: None,
            calm_need: [CALM_S; LADDER.len()],
            hold_until: f64::NEG_INFINITY,
            frame_ms: 0.0,
            reported: None,
        }
    }

    pub fn rung(&self) -> &Rung {
        &LADDER[self.rung]
    }

    pub fn index(&self) -> usize {
        self.rung
    }

    /// the level the deck is at now (the rung's)
    pub fn level(&self) -> Level {
        LADDER[self.rung].level
    }

    /// the level chosen (the ceiling)
    pub fn ceiling(&self) -> Level {
        LADDER[self.ceiling].level
    }

    pub fn auto(&self) -> bool {
        self.auto
    }

    /// the last window's mean frame interval (ms; 0 before the first)
    pub fn frame_ms(&self) -> f32 {
        self.frame_ms
    }

    /// A level by hand: its top rung now, every rung's calm time back to `CALM_S`.
    pub fn set_level(&mut self, level: Level) {
        self.ceiling = level.top();
        self.rung = self.ceiling;
        self.calm_need = [CALM_S; LADDER.len()];
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

    /// One frame at `now` (s) that took `dt_ms` since the last; `glitches` = the audio's running glitch count (0
    /// without audio). Returns whether the rung changed (the caller applies `rung()` – a resize only then).
    pub fn frame(&mut self, now: f64, dt_ms: f32, glitches: u64) -> bool {
        let glitched = glitches > self.glitches;
        self.glitches = glitches;
        if self.win_start.is_none() && self.hold_until == f64::NEG_INFINITY {
            // the very first frame: the start is a stall (shaders compile, the set loads)
            self.hold_until = now + HOLD_S;
        }
        if now < self.hold_until || !(0.0..=HITCH_MS).contains(&dt_ms) {
            self.reset_window();
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
        let over = mean > BUDGET_MS || glitch;
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
            let sustained = self.over >= OVER_WINDOWS || mean > SEVERE_MS;
            let may = self.last_down.map_or(true, |t| now - t >= STEP_GAP_S);
            return sustained && may && self.step_down(now);
        }
        self.over = 0;
        if mean > CALM_MS {
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
        Report { level: self.level(), scale: self.rung().scale, auto: self.auto, frame_ms: self.frame_ms }
    }

    /// What to send at `now`, if anything: at once when the level, scale or auto changed; else at most every
    /// `REPORT_S` when the frame time moved `REPORT_MS` (the conductor's presence isn't rebuilt for jitter).
    pub fn report_due(&mut self, now: f64) -> Option<Report> {
        let r = self.report();
        let due = match &self.reported {
            None => true,
            Some((last, at)) => (last.level, last.scale, last.auto) != (r.level, r.scale, r.auto) || (now - at >= REPORT_S && (last.frame_ms - r.frame_ms).abs() >= REPORT_MS),
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

    #[test]
    fn the_ladder_runs_cheapest_first_and_each_level_has_its_rungs() {
        for w in LADDER.windows(2) {
            assert!(w[0].level <= w[1].level && w[0].scale < w[1].scale, "{w:?}");
            assert!(w[0].lights <= w[1].lights && (!w[0].bloom || w[1].bloom) && (!w[0].fxaa || w[1].fxaa));
        }
        for l in Level::ALL {
            assert_eq!(LADDER[l.top()].level, l);
            assert_eq!(Level::parse(l.name()), Some(l));
        }
        assert_eq!(LADDER[Level::High.top()].scale, 1.0, "high = native");
        assert!(LADDER[Level::Ultra.top()].scale > 1.0, "ultra supersamples");
        assert_eq!(Level::parse(" ULTRA"), Some(Level::Ultra));
        assert_eq!(Level::Ultra.next(), Level::Low);
    }

    #[test]
    fn the_defaults_per_gpu() {
        assert_eq!(Level::default_for(Gpu::Software), Level::Low);
        assert_eq!(Level::default_for(Gpu::WebGl2), Level::Medium, "the S22's Chrome");
        assert_eq!(Level::default_for(Gpu::Integrated), Level::High, "the X1's UHD 620");
        assert_eq!(Level::default_for(Gpu::WebGpu), Level::High);
        assert_eq!(Level::default_for(Gpu::Discrete), Level::Ultra);
    }

    #[test]
    fn the_render_size() {
        assert_eq!(render_size((1920, 1080), 0.5), (960, 540));
        assert_eq!(render_size((1920, 1080), 1.25), (2400, 1350));
        assert_eq!(render_size((3840, 2160), 1.25), (4096, 2700), "capped");
        assert_eq!(render_size((1, 1), 0.5), (1, 1));
    }

    #[test]
    fn sustained_overload_steps_down_fast_one_rung_per_gap() {
        let mut g = Governor::new(Level::High, true);
        let mut t = 0.0;
        assert_eq!(run(&mut g, &mut t, 3.0, 16.7), 0, "the start holds, then calm at the top");
        assert_eq!(g.index(), Level::High.top());
        // 40 fps: 2 windows (1 s) step one rung, then one per second
        assert_eq!(run(&mut g, &mut t, 0.9, 25.0), 0, "one window over: nothing yet");
        assert_eq!(run(&mut g, &mut t, 1.1, 25.0), 1, "a sustained second (two windows): a step");
        assert_eq!(g.index(), Level::High.top() - 1);
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
    }

    #[test]
    fn calm_steps_up_slowly_and_a_failed_probe_doubles_its_wait() {
        let mut g = Governor::new(Level::High, true);
        let mut t = 0.0;
        run(&mut g, &mut t, 3.0, 16.7);
        run(&mut g, &mut t, 1.1, 25.0);
        let low = g.index();
        assert_eq!(low, Level::High.top() - 1);
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
        assert_eq!(g.index(), Level::High.top());
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
        assert_eq!((g.index(), g.ceiling()), (Level::Low.top(), Level::Low));
        run(&mut g, &mut t, 60.0, 16.7);
        assert_eq!(g.index(), Level::Low.top(), "never above the chosen level");
        g.set_level(Level::High);
        run(&mut g, &mut t, 1.3, 40.0);
        g.set_auto(false);
        assert_eq!(g.index(), Level::High.top(), "off: back at the level's top");
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
    }
}
