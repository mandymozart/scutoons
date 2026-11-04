//! The one monitor (#364, BRIDGE-AUDIT.md §2): FPS, frame time, CPU per frame, the audio's load per block, its glitches
//! per window, the lead, the limiter – computed once, from raw samples the adapters feed, judged by ONE threshold set,
//! worded once. The native HUD and the browser's stats bar / sound report only draw a `Report`.
//!
//! Two halves, as the threads are: the **audio side** (`AudioWindow`, the render thread / the browser worker's core)
//! collects each block's load, the analysis time, the pump gap and the limiter's gain reduction without allocating; the
//! **frame side** (`Monitor`, the native frame thread / the browser page beside the quality governor) takes a frame
//! sample per animation frame (drawn or skipped by the fps cap), a window of the audio side once a second and the
//! glitch counts whenever they change, and answers `report(now)`. A window crosses threads as numbers: `to_words` /
//! `from_words` (the browser posts them from the worker), or the native callback's atomics fill `Histogram` bins
//! (`load_bin`).
//!
//! **The one threshold set** (the owner decides the final numbers in #364; the alternatives are listed there):
//! - FPS against the frame target (the governor's fps cap; 60 without one): warn below 5/6 of it (50 of 60), bad below
//!   half (30 of 60); not judged in the first two seconds. The browser's old rule (warn < 50, bad < 30) at 60, but a 30
//!   fps cap is no longer "bad".
//! - CPU: the worst frame's work against the frame's budget (1000 / target ms): warn past 60 % (10 ms at 60), bad past
//!   the whole budget (the next frame is late by construction).
//! - Audio: bad while a glitch happened in the last minute, warn while one is in the 10-minute window or a block's p99
//!   passes half its budget or the longest block passes its whole budget (the lead absorbs a rare one, not a habit).
//! - Limiter: warn past 6 dB of gain reduction (a few dB shave peaks; past 6 it holds the whole sound down).
//! - Glitches are counted over 10 minutes (a set's length: did anything happen in this set?), the warning line shows
//!   for 5 s after one.
use serde::Serialize;

/// a histogram's bins: 16 octaves, eight to an octave (≈ 9 % wide)
pub const BINS: usize = 16 * PER_OCTAVE as usize;
const PER_OCTAVE: f32 = 8.0;
/// a block's load (its render time ÷ its budget) starts its first bin here: a thousandth of the budget (2.7 µs of a
/// 128-frame block at 48 kHz); the last bin starts at 65× the budget
pub const LOAD_FLOOR: f32 = 0.001;
/// a frame interval (ms) starts its first bin here: up to 4 s
pub const FRAME_FLOOR: f32 = 0.0625;

/// FPS warns below this share of the target, is bad below `FPS_BAD`
pub const FPS_WARN: f32 = 5.0 / 6.0;
pub const FPS_BAD: f32 = 0.5;
/// the frame target without a cap (fps)
pub const TARGET_FPS: f32 = 60.0;
/// the worst frame's work warns past this share of the frame's budget, is bad past `CPU_BAD`
pub const CPU_WARN: f32 = 0.6;
pub const CPU_BAD: f32 = 1.0;
/// a block's p99 load warns past this, the longest past `LOAD_MAX_WARN` (1 = the block's whole budget)
pub const LOAD_P99_WARN: f32 = 0.5;
pub const LOAD_MAX_WARN: f32 = 1.0;
/// the limiter warns past this gain reduction (dB)
pub const LIMIT_WARN_DB: f32 = 6.0;
/// a gain reduction past this (dB) counts as limiting – below it the limiter only shaves peaks unheard
pub const LIMITING_DB: f32 = 1.0;
/// glitches are counted over this window (s); one in the last `GLITCH_RECENT_S` is bad
pub const GLITCH_WINDOW_S: usize = 600;
pub const GLITCH_RECENT_S: usize = 60;
/// the warning line shows this long after a glitch (ms)
pub const ALERT_MS: f64 = 5000.0;
/// FPS is not judged before this long after the first frame (ms): the first frames build pipelines
pub const JUDGE_AFTER_MS: f64 = 2000.0;
/// the seconds of the audio kept from the start (the warm-up as a time series, #356)
pub const SERIES: usize = 120;

/// What made a callback a glitch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Cause {
    /// the OS ran the audio thread late
    Late = 0,
    /// a render took longer than its budget
    Long = 1,
    /// the device ran dry (the ring was short)
    Underrun = 2,
}

impl Cause {
    pub const ALL: [Cause; 3] = [Cause::Late, Cause::Long, Cause::Underrun];

    pub fn name(self) -> &'static str {
        match self {
            Cause::Late => "late",
            Cause::Long => "long render",
            Cause::Underrun => "underrun",
        }
    }
}

/// a callback's jitter allowance on top of half the period: WASAPI's and the kernel's wake-up jitter (ns)
pub const SLACK_NS: u64 = 2_000_000;

/// One device callback's numbers (ns) – what a native audio adapter measures.
#[derive(Debug, Clone, Copy, Default)]
pub struct Timing {
    /// since the previous callback started (None: the first)
    pub interval: Option<u64>,
    /// the period the previous callback's frames covered
    pub covered: u64,
    /// the time spent in this callback
    pub render: u64,
    /// this callback's own period (its frames at the rate)
    pub budget: u64,
    /// the device ran dry before or during it
    pub underrun: bool,
}

/// The cause of a glitch, if the callback was one (the most direct cause wins: underrun, long render, late).
pub fn classify(t: &Timing) -> Option<Cause> {
    if t.underrun {
        return Some(Cause::Underrun);
    }
    if t.budget > 0 && t.render > t.budget {
        return Some(Cause::Long);
    }
    match t.interval {
        Some(i) if t.covered > 0 && i > t.covered + t.covered / 2 + SLACK_NS => Some(Cause::Late),
        _ => None,
    }
}

/// Values in logarithmic bins from `floor` – p50, p99, the mean and the longest of a window without keeping the
/// samples and without allocating. A quantile is its bin's upper edge (never past the longest).
#[derive(Debug, Clone, PartialEq)]
pub struct Histogram {
    floor: f32,
    bins: [u32; BINS],
    count: u32,
    max: f32,
    total: f64,
}

/// the bin of `v` in a histogram from `floor`
pub fn bin_of(floor: f32, v: f32) -> usize {
    if v > floor {
        ((libm::log2f(v / floor) * PER_OCTAVE) as usize).min(BINS - 1)
    } else {
        0
    }
}

/// the bin of a block's load (the native callback's atomic bins)
pub fn load_bin(load: f32) -> usize {
    bin_of(LOAD_FLOOR, load)
}

impl Histogram {
    pub const fn new(floor: f32) -> Histogram {
        Histogram { floor, bins: [0; BINS], count: 0, max: 0.0, total: 0.0 }
    }

    /// a window another thread counted: its bins, the longest and the sum
    pub fn from_parts(floor: f32, bins: [u32; BINS], max: f32, total: f64) -> Histogram {
        Histogram { floor, bins, count: bins.iter().sum(), max, total }
    }

    pub fn push(&mut self, v: f32) {
        self.bins[bin_of(self.floor, v)] += 1;
        self.count += 1;
        self.total += f64::from(v);
        if v > self.max {
            self.max = v;
        }
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn max(&self) -> f32 {
        self.max
    }

    pub fn mean(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            (self.total / f64::from(self.count)) as f32
        }
    }

    /// what `q` (0..1) of the values were at most; 0 when empty
    pub fn quantile(&self, q: f32) -> f32 {
        if self.count == 0 {
            return 0.0;
        }
        let want = libm::ceilf(q * self.count as f32).max(1.0) as u32;
        let mut seen = 0;
        for (i, &n) in self.bins.iter().enumerate() {
            seen += n;
            if seen >= want {
                // the last bin has no upper edge: its values are as long as the longest at most
                return if i == BINS - 1 { self.max } else { self.max.min(self.floor * libm::exp2f((i + 1) as f32 / PER_OCTAVE)) };
            }
        }
        self.max
    }

    pub fn reset(&mut self) {
        *self = Histogram::new(self.floor);
    }
}

/// The audio side's second: every block's load, the longest analysis frame, the longest pause between two pumps, the
/// limiter's gain reduction per analysis frame; and the facts that last (the lead, the rate, the warm-up). Fed by the
/// render thread / the worker's core, taken once a second.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioWindow {
    pub loads: Histogram,
    /// the longest analysis frame (µs) – between blocks, not in them
    pub analysis_us: f32,
    /// the longest pause between two pumps (ms) – a stalled render thread / worker shows here
    pub gap_ms: f32,
    /// the limiter: the deepest gain reduction (dB), the sum, the frames, the frames past `LIMITING_DB`
    pub limit_db: f32,
    limit_sum: f32,
    limit_frames: u32,
    limiting: u32,
    /// the lead now (frames) and the rate it counts in; the warm-up before the first sound (ms, 0 = none)
    pub lead: u32,
    pub rate: f32,
    pub warm_ms: f32,
}

impl Default for AudioWindow {
    fn default() -> Self {
        AudioWindow { loads: Histogram::new(LOAD_FLOOR), analysis_us: 0.0, gap_ms: 0.0, limit_db: 0.0, limit_sum: 0.0, limit_frames: 0, limiting: 0, lead: 0, rate: 0.0, warm_ms: 0.0 }
    }
}

impl AudioWindow {
    /// the numbers `to_words` writes: `HEAD` facts, then the bins
    const HEAD: usize = 11;
    pub const WORDS: usize = Self::HEAD + BINS;

    /// an empty window for a lead of `lead` frames at `rate`
    pub fn with_lead(lead: u32, rate: f32) -> AudioWindow {
        AudioWindow { lead, rate, ..AudioWindow::default() }
    }

    /// a block's render time against its budget (any unit, the same for both)
    pub fn block(&mut self, took: f32, budget: f32) {
        if budget > 0.0 {
            self.loads.push(took / budget);
        }
    }

    pub fn analysis(&mut self, us: f32) {
        self.analysis_us = self.analysis_us.max(us);
    }

    pub fn gap(&mut self, ms: f32) {
        self.gap_ms = self.gap_ms.max(ms);
    }

    /// an analysis frame's gain reduction (dB)
    pub fn limiter(&mut self, db: f32) {
        self.limit_db = self.limit_db.max(db);
        self.limit_sum += db;
        self.limit_frames += 1;
        if db > LIMITING_DB {
            self.limiting += 1;
        }
    }

    /// the mean gain reduction (dB) and the share of frames limiting (%)
    pub fn limit_mean(&self) -> (f32, f32) {
        if self.limit_frames == 0 {
            return (0.0, 0.0);
        }
        let n = self.limit_frames as f32;
        (self.limit_sum / n, 100.0 * self.limiting as f32 / n)
    }

    /// the second so far; the window starts again (the lead, the rate and the warm-up stay)
    pub fn take(&mut self) -> AudioWindow {
        let fresh = AudioWindow { lead: self.lead, rate: self.rate, warm_ms: self.warm_ms, ..AudioWindow::default() };
        core::mem::replace(self, fresh)
    }

    /// as numbers for another thread (`WORDS` of them)
    pub fn to_words(&self, out: &mut [f64]) {
        let l = &self.loads;
        let head: [f64; Self::HEAD] = [
            f64::from(l.max),
            l.total,
            f64::from(self.analysis_us),
            f64::from(self.gap_ms),
            f64::from(self.limit_db),
            f64::from(self.limit_sum),
            f64::from(self.limit_frames),
            f64::from(self.limiting),
            f64::from(self.lead),
            f64::from(self.rate),
            f64::from(self.warm_ms),
        ];
        out[..Self::HEAD].copy_from_slice(&head);
        for (o, &b) in out[Self::HEAD..Self::WORDS].iter_mut().zip(&l.bins) {
            *o = f64::from(b);
        }
    }

    /// `to_words`'s numbers back (None: too few)
    pub fn from_words(w: &[f64]) -> Option<AudioWindow> {
        if w.len() < Self::WORDS {
            return None;
        }
        let mut bins = [0u32; BINS];
        for (b, &v) in bins.iter_mut().zip(&w[Self::HEAD..Self::WORDS]) {
            *b = v as u32;
        }
        Some(AudioWindow {
            loads: Histogram::from_parts(LOAD_FLOOR, bins, w[0] as f32, w[1]),
            analysis_us: w[2] as f32,
            gap_ms: w[3] as f32,
            limit_db: w[4] as f32,
            limit_sum: w[5] as f32,
            limit_frames: w[6] as u32,
            limiting: w[7] as u32,
            lead: w[8] as u32,
            rate: w[9] as f32,
            warm_ms: w[10] as f32,
        })
    }
}

/// ok, warn, bad
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    #[default]
    Ok,
    Warn,
    Bad,
}

/// one item of the collapsed bar: its label (FPS, CPU, AUDIO, LIM), its value and how it is judged
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    pub label: &'static str,
    pub value: String,
    pub level: Level,
}

impl Item {
    fn new(label: &'static str, value: String, level: Level) -> Item {
        Item { label, value, level }
    }

    /// "LABEL value"
    pub fn text(&self) -> String {
        format!("{} {}", self.label, self.value)
    }
}

/// the judgement per number
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct Levels {
    pub fps: Level,
    pub cpu: Level,
    pub audio: Level,
    pub limit: Level,
}

/// What the monitor says now – every number, its judgement and the words for the bar and the lines.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Report {
    /// drawn frames per second (a frame the fps cap skips is not one), animation frames per second
    pub fps: f32,
    pub ticks: f32,
    /// the interval between drawn frames: the mean, p50, p99 (ms)
    pub frame_ms: f32,
    pub frame_p50_ms: f32,
    pub frame_p99_ms: f32,
    /// the work per animation frame: the mean and the worst (ms)
    pub cpu_ms: f32,
    pub cpu_peak_ms: f32,
    /// the frame target (fps) the numbers are judged against
    pub target_fps: f32,
    /// the audio side reported (a sound runs)
    pub sound: bool,
    /// a block's load, % of its budget: p50, p99, the longest
    pub load_p50: f32,
    pub load_p99: f32,
    pub load_max: f32,
    pub analysis_us: f32,
    pub gap_ms: f32,
    pub lead_ms: f32,
    pub warm_ms: f32,
    /// glitches over the 10-minute window, over the last minute, per cause and in all since the start
    pub glitches: u32,
    pub recent: u32,
    pub causes: [u64; 3],
    pub total: u64,
    /// the newest glitch's cause while its warning shows, and the warning's line ("" = none)
    pub alert: String,
    pub alert_line: String,
    pub limit_db: f32,
    pub limit_mean_db: f32,
    pub limiting_pct: f32,
    pub levels: Levels,
    /// the collapsed bar: FPS, CPU, AUDIO, LIM
    pub bar: Vec<Item>,
    /// the full lines (the native HUD, the sound report)
    pub lines: Vec<String>,
}

/// a second of the audio as the series keeps it
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Second {
    t: f32,
    p50: f32,
    p99: f32,
    max: f32,
    analysis_us: f32,
    gap_ms: f32,
    lead_ms: f32,
    glitches: u64,
    limit_db: f32,
    limit_mean_db: f32,
    limiting_pct: f32,
}

/// The frame side: samples in, `report` out. Never allocates while fed.
pub struct Monitor {
    target: f32,
    /// the first sample's moment (ms, NaN = none yet): the glitch seconds and the series count from it
    epoch: f64,
    first_frame: f64,
    win_start: f64,
    ticks: u32,
    drawn: u32,
    work_sum: f32,
    work_max: f32,
    last_drawn: f64,
    intervals: Histogram,
    /// the last whole frame window
    fps: f32,
    ticks_per_s: f32,
    frame: (f32, f32, f32),
    cpu: (f32, f32),
    /// the last audio window (None: no sound reported)
    audio: Option<AudioWindow>,
    /// the glitch counts seen per cause, glitches per second over the window (a ring), its newest second
    seen: [u64; 3],
    per_s: [u32; GLITCH_WINDOW_S],
    head: i64,
    newest: Option<(Cause, f64)>,
    series: [Second; SERIES],
    series_len: usize,
}

impl Default for Monitor {
    fn default() -> Self {
        Monitor::new()
    }
}

/// a share as a percentage, rounded
fn pct(v: f32) -> f32 {
    libm::roundf(100.0 * v)
}

impl Monitor {
    pub fn new() -> Monitor {
        Monitor {
            target: TARGET_FPS,
            epoch: f64::NAN,
            first_frame: f64::NAN,
            win_start: 0.0,
            ticks: 0,
            drawn: 0,
            work_sum: 0.0,
            work_max: 0.0,
            last_drawn: f64::NAN,
            intervals: Histogram::new(FRAME_FLOOR),
            fps: 0.0,
            ticks_per_s: 0.0,
            frame: (0.0, 0.0, 0.0),
            cpu: (0.0, 0.0),
            audio: None,
            seen: [0; 3],
            per_s: [0; GLITCH_WINDOW_S],
            head: 0,
            newest: None,
            series: [Second::default(); SERIES],
            series_len: 0,
        }
    }

    /// the frame target (the governor's fps cap; 0 = none → `TARGET_FPS`)
    pub fn target(&mut self, fps: f32) {
        self.target = if fps > 0.0 { fps } else { TARGET_FPS };
    }

    fn start(&mut self, now: f64) {
        if self.epoch.is_nan() {
            self.epoch = now;
        }
    }

    /// An animation frame at `now` (ms) that took `work_ms` of work; `drawn` = it drew (false: the fps cap skipped it).
    pub fn frame(&mut self, now: f64, work_ms: f32, drawn: bool) {
        self.start(now);
        if self.first_frame.is_nan() {
            self.first_frame = now;
            self.win_start = now;
            if drawn {
                self.last_drawn = now;
            }
            return;
        }
        self.ticks += 1;
        self.work_sum += work_ms;
        self.work_max = self.work_max.max(work_ms);
        if drawn {
            if !self.last_drawn.is_nan() {
                self.intervals.push((now - self.last_drawn) as f32);
            }
            self.last_drawn = now;
            self.drawn += 1;
        }
        let span = now - self.win_start;
        if span >= 1000.0 {
            self.fps = (f64::from(self.drawn) * 1000.0 / span) as f32;
            self.ticks_per_s = (f64::from(self.ticks) * 1000.0 / span) as f32;
            let i = &self.intervals;
            self.frame = (i.mean(), i.quantile(0.5), i.quantile(0.99));
            self.cpu = (self.work_sum / self.ticks as f32, self.work_max);
            self.win_start = now;
            self.ticks = 0;
            self.drawn = 0;
            self.work_sum = 0.0;
            self.work_max = 0.0;
            self.intervals.reset();
        }
    }

    /// the second (from the epoch) of `now`, the ring advanced to it
    fn second(&mut self, now: f64) -> i64 {
        self.start(now);
        let s = ((now - self.epoch) / 1000.0).floor() as i64;
        if s > self.head {
            for k in (self.head + 1)..=s.min(self.head + GLITCH_WINDOW_S as i64) {
                self.per_s[k.rem_euclid(GLITCH_WINDOW_S as i64) as usize] = 0;
            }
            self.head = s;
        }
        s
    }

    /// The glitch counts per cause (running totals: late, long render, underrun) at `now` (ms). A total that went down
    /// (a restarted sound) starts over without counting.
    pub fn glitches(&mut self, now: f64, totals: [u64; 3]) {
        let s = self.second(now);
        for c in Cause::ALL {
            let i = c as usize;
            if totals[i] > self.seen[i] {
                let d = (totals[i] - self.seen[i]).min(u64::from(u32::MAX)) as u32;
                let b = &mut self.per_s[s.rem_euclid(GLITCH_WINDOW_S as i64) as usize];
                *b = b.saturating_add(d);
                self.newest = Some((c, now));
            }
        }
        self.seen = totals;
    }

    /// The audio side's last second at `now` (ms); kept in the series while it has room.
    pub fn audio(&mut self, now: f64, w: AudioWindow) {
        self.start(now);
        if self.series_len < SERIES {
            let (mean, limiting) = w.limit_mean();
            self.series[self.series_len] = Second {
                t: ((now - self.epoch) / 1000.0) as f32,
                p50: pct(w.loads.quantile(0.5)),
                p99: pct(w.loads.quantile(0.99)),
                max: pct(w.loads.max()),
                analysis_us: w.analysis_us,
                gap_ms: w.gap_ms,
                lead_ms: lead_ms(&w),
                glitches: self.seen.iter().sum(),
                limit_db: w.limit_db,
                limit_mean_db: mean,
                limiting_pct: limiting,
            };
            self.series_len += 1;
        }
        self.audio = Some(w);
    }

    /// the sound stopped: the audio side reports nothing until the next `audio`
    pub fn silent(&mut self) {
        self.audio = None;
    }

    fn window_sum(&self, s: i64, secs: usize) -> u32 {
        (0..secs as i64).map(|k| self.per_s[(s - k).rem_euclid(GLITCH_WINDOW_S as i64) as usize]).fold(0u32, u32::saturating_add)
    }

    /// Everything now (`now` in ms): the numbers, their judgement, the bar's items and the lines.
    pub fn report(&mut self, now: f64) -> Report {
        let s = self.second(now);
        let mut r = Report {
            fps: self.fps,
            ticks: self.ticks_per_s,
            frame_ms: self.frame.0,
            frame_p50_ms: self.frame.1,
            frame_p99_ms: self.frame.2,
            cpu_ms: self.cpu.0,
            cpu_peak_ms: self.cpu.1,
            target_fps: self.target,
            glitches: self.window_sum(s, GLITCH_WINDOW_S),
            recent: self.window_sum(s, GLITCH_RECENT_S),
            causes: self.seen,
            total: self.seen.iter().sum(),
            alert: match self.newest {
                Some((c, at)) if now - at < ALERT_MS => c.name().to_string(),
                _ => String::new(),
            },
            ..Report::default()
        };
        if let Some(w) = &self.audio {
            let (mean, limiting) = w.limit_mean();
            r.sound = true;
            r.load_p50 = pct(w.loads.quantile(0.5));
            r.load_p99 = pct(w.loads.quantile(0.99));
            r.load_max = pct(w.loads.max());
            r.analysis_us = w.analysis_us;
            r.gap_ms = w.gap_ms;
            r.lead_ms = lead_ms(w);
            r.warm_ms = w.warm_ms;
            r.limit_db = w.limit_db;
            r.limit_mean_db = mean;
            r.limiting_pct = limiting;
        }
        let judged = !self.first_frame.is_nan() && now - self.first_frame >= JUDGE_AFTER_MS;
        r.levels = judge(&r, judged);
        if !r.alert.is_empty() {
            r.alert_line = format!("{} - {} IN 10 MIN", r.alert.to_uppercase(), r.glitches.max(1));
        }
        r.bar = bar(&r);
        r.lines = lines(&r);
        r
    }

    /// The first seconds of the audio as a table (#356's sound report): t · p50 · p99 · max (% of the budget) ·
    /// analysis (µs) · pump gap (ms) · lead (ms) · glitches so far · the limiter's deepest and mean gain reduction (dB)
    /// and its share of the second (%).
    pub fn series_table(&self) -> Vec<String> {
        let mut rows = vec!["t_s  p50%  p99%  max%  ana_us  gap_ms  lead_ms  drops  lim_db  avg_db  lim%".to_string()];
        for s in &self.series[..self.series_len] {
            rows.push(format!(
                "{:>4.1} {:>5} {:>5} {:>5} {:>7} {:>7.1} {:>8} {:>6} {:>7.1} {:>7.1} {:>5}",
                s.t, s.p50, s.p99, s.max, libm::roundf(s.analysis_us), s.gap_ms, libm::roundf(s.lead_ms), s.glitches, s.limit_db, s.limit_mean_db, libm::roundf(s.limiting_pct)
            ));
        }
        rows
    }
}

fn lead_ms(w: &AudioWindow) -> f32 {
    if w.rate > 0.0 {
        libm::roundf(1000.0 * w.lead as f32 / w.rate)
    } else {
        0.0
    }
}

/// the one threshold set (the module's header says why)
fn judge(r: &Report, judged: bool) -> Levels {
    let fps = if !judged || r.fps <= 0.0 {
        Level::Ok
    } else if r.fps < FPS_BAD * r.target_fps {
        Level::Bad
    } else if r.fps < FPS_WARN * r.target_fps {
        Level::Warn
    } else {
        Level::Ok
    };
    let budget = 1000.0 / r.target_fps;
    let cpu = if r.cpu_peak_ms > CPU_BAD * budget {
        Level::Bad
    } else if r.cpu_peak_ms > CPU_WARN * budget {
        Level::Warn
    } else {
        Level::Ok
    };
    let audio = if !r.sound {
        Level::Ok
    } else if r.recent > 0 {
        Level::Bad
    } else if r.glitches > 0 || r.load_p99 > pct(LOAD_P99_WARN) || r.load_max > pct(LOAD_MAX_WARN) {
        Level::Warn
    } else {
        Level::Ok
    };
    let limit = if r.sound && r.limit_db > LIMIT_WARN_DB { Level::Warn } else { Level::Ok };
    Levels { fps, cpu, audio, limit }
}

/// the collapsed bar's monitor items (the stage HUD's abbreviations, language-neutral like a meter's)
fn bar(r: &Report) -> Vec<Item> {
    let l = &r.levels;
    let mut out = vec![
        Item::new("FPS", if r.fps > 0.0 { format!("{:.0}", r.fps) } else { "-".into() }, l.fps),
        Item::new("CPU", format!("{:.1}/{:.1}MS", r.cpu_ms, r.cpu_peak_ms), l.cpu),
    ];
    if r.sound {
        out.push(Item::new("AUDIO", format!("{}/10M {}/{}/{}% +{}MS", r.glitches, r.load_p50, r.load_p99, r.load_max, r.lead_ms), l.audio));
        out.push(Item::new("LIM", format!("{:.1}DB", r.limit_db), l.limit));
    } else {
        out.push(Item::new("AUDIO", "OFF".into(), Level::Ok));
    }
    out
}

/// the full lines (≤ 39 characters: the native HUD's width)
fn lines(r: &Report) -> Vec<String> {
    let mut out = Vec::new();
    out.push(format!("FPS {:.1} FRAME {:.1}/{:.1} CPU {:.1}/{:.1}", r.fps, r.frame_ms, r.frame_p99_ms, r.cpu_ms, r.cpu_peak_ms));
    if r.sound {
        out.push(format!("DSP {}/{}/{}%  LEAD {}MS  LIM {:.1}", r.load_p50, r.load_p99, r.load_max, r.lead_ms, r.limit_db));
        let [late, long, under] = r.causes;
        out.push(format!("GLITCH {}/10M LATE {late} LONG {long} UND {under}", r.glitches));
    } else {
        out.push("AUDIO OFF".into());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    #[test]
    fn the_classifier_names_the_most_direct_cause() {
        let ok = Timing { interval: Some(10 * MS), covered: 10 * MS, render: 3 * MS, budget: 10 * MS, underrun: false };
        assert_eq!(classify(&ok), None);
        assert_eq!(classify(&Timing { interval: None, ..ok }), None, "the first callback");
        assert_eq!(classify(&Timing { interval: Some(16 * MS), ..ok }), None, "a little late: within half a period + 2 ms");
        assert_eq!(classify(&Timing { interval: Some(17_100_000), ..ok }), Some(Cause::Late));
        assert_eq!(classify(&Timing { render: 11 * MS, ..ok }), Some(Cause::Long), "over the budget");
        assert_eq!(classify(&Timing { render: 11 * MS, interval: Some(30 * MS), ..ok }), Some(Cause::Long), "long before late");
        assert_eq!(classify(&Timing { underrun: true, render: 11 * MS, ..ok }), Some(Cause::Underrun), "underrun first");
        // WASAPI: a big first callback (the whole buffer), then the engine's period
        assert_eq!(classify(&Timing { interval: Some(10 * MS), covered: 22 * MS, ..ok }), None);
    }

    #[test]
    fn a_histogram_answers_quantiles_within_a_bin() {
        let mut h = Histogram::new(LOAD_FLOOR);
        for _ in 0..98 {
            h.push(0.02);
        }
        h.push(0.5);
        h.push(3.0);
        assert_eq!(h.count(), 100);
        assert!((h.quantile(0.5) - 0.02).abs() < 0.02 * 0.1, "p50 {}", h.quantile(0.5));
        assert!((h.quantile(0.99) - 0.5).abs() < 0.5 * 0.1, "p99 {}", h.quantile(0.99));
        assert_eq!(h.max(), 3.0, "the longest is exact");
        assert_eq!(h.quantile(1.0), 3.0, "never past the longest");
        assert!((h.mean() - (98.0 * 0.02 + 3.5) / 100.0).abs() < 1e-6);
        assert_eq!(Histogram::new(1.0).quantile(0.5), 0.0, "empty");
    }

    #[test]
    fn an_audio_window_crosses_threads_as_words() {
        let mut w = AudioWindow { lead: 2048, rate: 48_000.0, warm_ms: 70.0, ..AudioWindow::default() };
        w.block(30.0, 2667.0);
        w.block(2900.0, 2667.0);
        w.analysis(80.0);
        w.gap(12.5);
        w.limiter(0.5);
        w.limiter(3.0);
        let mut words = [0.0; AudioWindow::WORDS];
        w.to_words(&mut words);
        assert_eq!(AudioWindow::from_words(&words).as_ref(), Some(&w));
        let taken = w.take();
        assert_eq!(taken.loads.count(), 2);
        assert_eq!((w.loads.count(), w.lead, w.rate, w.warm_ms), (0, 2048, 48_000.0, 70.0), "the window starts again, the facts stay");
        assert_eq!(taken.limit_mean(), (1.75, 50.0));
    }

    /// 60 drawn frames a second, then a cap of 30: the FPS is what was drawn, not what the page was called for
    #[test]
    fn fps_counts_drawn_frames() {
        let mut m = Monitor::new();
        let mut t = 0.0;
        for _ in 0..=61 {
            m.frame(t, 2.0, true);
            t += 1000.0 / 60.0;
        }
        let r = m.report(t);
        assert!((r.fps - 60.0).abs() < 1.0, "{}", r.fps);
        assert!((r.frame_ms - 16.67).abs() < 0.05);
        assert_eq!(r.cpu_ms, 2.0);
        m.target(30.0);
        for k in 0..=120 {
            m.frame(t, 1.0, k % 2 == 0);
            t += 1000.0 / 60.0;
        }
        let r = m.report(t);
        assert!((r.fps - 30.0).abs() < 1.0, "half the animation frames drew: {}", r.fps);
        assert!((r.ticks - 60.0).abs() < 1.0);
        assert!((r.frame_ms - 33.3).abs() < 0.1);
        assert_eq!(r.levels.fps, Level::Ok, "30 of a 30 fps cap is fine");
    }

    #[test]
    fn one_threshold_set_judges_every_number() {
        let mut m = Monitor::new();
        let mut t = 0.0;
        // 25 fps with a frame of 20 ms of work: bad and bad at 60
        for _ in 0..=100 {
            m.frame(t, 20.0, true);
            t += 40.0;
        }
        let r = m.report(t);
        assert_eq!((r.levels.fps, r.levels.cpu, r.levels.audio), (Level::Bad, Level::Bad, Level::Ok));
        assert_eq!(r.bar[2].text(), "AUDIO OFF");
        // a sound with a p99 past half the budget: a warning, no glitch
        let mut w = AudioWindow { lead: 2400, rate: 48_000.0, ..AudioWindow::default() };
        for _ in 0..100 {
            w.block(0.6, 1.0);
        }
        w.limiter(7.0);
        m.audio(t, w.take());
        let r = m.report(t);
        assert_eq!((r.levels.audio, r.levels.limit), (Level::Warn, Level::Warn));
        assert_eq!(r.lead_ms, 50.0);
        assert!(r.bar[2].text().starts_with("AUDIO 0/10M 6"), "{}", r.bar[2].text());
        assert_eq!(r.bar[3].text(), "LIM 7.0DB");
        assert!(r.lines.iter().all(|l| l.chars().count() <= 39), "{:?}", r.lines);
    }

    #[test]
    fn glitches_count_over_ten_minutes_and_a_recent_one_is_bad() {
        let mut m = Monitor::new();
        m.audio(0.0, AudioWindow::default());
        m.glitches(1000.0, [0, 0, 1]);
        m.glitches(2500.0, [0, 1, 1]);
        let r = m.report(3000.0);
        assert_eq!((r.glitches, r.recent, r.total), (2, 2, 2));
        assert_eq!(r.alert, "long render", "the newest one's cause, while its warning shows");
        assert_eq!(r.levels.audio, Level::Bad);
        assert_eq!(r.alert_line, "LONG RENDER - 2 IN 10 MIN");
        assert_eq!(r.lines[2], "GLITCH 2/10M LATE 0 LONG 1 UND 1");
        let r = m.report(120_000.0);
        assert_eq!((r.glitches, r.recent, r.alert.as_str()), (2, 0, ""));
        assert_eq!(r.levels.audio, Level::Warn, "still in the 10 minutes");
        let r = m.report(700_000.0);
        assert_eq!((r.glitches, r.levels.audio), (0, Level::Ok), "out of the window");
        m.glitches(701_000.0, [0, 0, 0]);
        assert_eq!(m.report(701_000.0).glitches, 0, "a restarted sound's totals start over uncounted");
    }

    #[test]
    fn the_series_keeps_the_first_seconds() {
        let mut m = Monitor::new();
        for s in 0..(SERIES + 5) {
            let mut w = AudioWindow { lead: 4096, rate: 48_000.0, ..AudioWindow::default() };
            w.block(0.02, 1.0);
            w.block(if s == 0 { 1.5 } else { 0.02 }, 1.0);
            m.audio(s as f64 * 1000.0, w);
        }
        let t = m.series_table();
        assert_eq!(t.len(), SERIES + 1, "the header + the first {SERIES} seconds");
        assert!(t[1].starts_with(" 0.0     2   150   150       0     0.0       85"), "{}", t[1]);
    }
}
