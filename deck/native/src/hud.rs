//! The on-screen HUD (CYBERDECKS.md §8 "a small on-screen HUD"): a few text lines and a frame-time graph rastered on
//! the CPU into a small RGBA image (font8x8, no font files), drawn over the screen by render.rs at an integer scale.
//! The tempo line carries a light per beat of the bar (#181): the current one lit, the bar's first in amber.
//!
//! #209 (owner 2026-10-31: "the collapsed panel shows very basic statistics … using the entire width, like OSCT's
//! collapsed bar"): F1 cycles the full HUD → **the bar** (one dense line across the screen's whole top edge: FPS, CPU /
//! frame, the audio's glitches and load, the relay, the backend – a number out of the green in red) → nothing. The bar
//! is rastered at the screen's width in HUD pixels (`bar_width`), so it spans the screen at the HUD's integer scale.
use font8x8::UnicodeFonts;
use std::collections::VecDeque;

pub const W: u32 = 320;
/// text lines the HUD has room for (above the frame-time graph; #208: two audio lines + the glitch warning)
pub const LINES: usize = 13;
/// characters a line has room for (8 px each, a 4 px margin)
pub const COLS: usize = 39;
pub const H: u32 = 4 + LINES as u32 * 10 + GRAPH_H + 4;
const GRAPH_H: u32 = 20;
/// the graph's full height in ms (two 60 Hz frames)
const GRAPH_MS: f32 = 33.3;
const TEXT: [u8; 4] = [235, 235, 235, 255];
/// the audio glitch warning's red (#208)
const ALERT: [u8; 4] = [230, 40, 40, 255];

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
    pub fn next(self) -> Mode {
        match self {
            Mode::Full => Mode::Bar,
            Mode::Bar => Mode::Off,
            Mode::Off => Mode::Full,
        }
    }
}

/// the bar's height (one text line and a pixel above / below, in HUD pixels) and its widest raster
pub const BAR_H: u32 = 10;
pub const BAR_MAX: u32 = 2048;

/// the HUD's integer scale for a screen `sh` pixels high – rem-like: 2 at 1080p
pub fn scale(sh: f32) -> f32 {
    (sh / 540.0).round().max(1.0)
}

/// the bar's width in HUD pixels for a `sw` × `sh` screen: the whole width at the HUD's scale
pub fn bar_width(sw: f32, sh: f32) -> u32 {
    ((sw / scale(sh)) as u32).clamp(8, BAR_MAX)
}

/// one item of the bar: its text, and red when it is out of the green
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub text: String,
    pub bad: bool,
}

pub struct Hud {
    pub pixels: Vec<u8>,
    pub mode: Mode,
    /// #209: the bar's pixels (bar_w × BAR_H RGBA) and its items
    pub bar: Vec<u8>,
    pub bar_w: u32,
    items: Vec<Item>,
    lines: Vec<String>,
    times: VecDeque<f32>,
    beat: Option<Beat>,
    /// #208: the glitch warning's row, drawn in red; `lit` = the flash's bright half (a red bar behind the text)
    alert: Option<Alert>,
}

/// the audio glitch warning (#208): text line `row` in red, on a red bar while `lit`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Alert {
    pub row: usize,
    pub lit: bool,
}

/// the beat lights: on text line `row`, `per_bar` lights (at most 8 drawn), `beat` (0-based) lit
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beat {
    pub row: usize,
    pub beat: u32,
    pub per_bar: u32,
    pub running: bool,
}

impl Default for Hud {
    fn default() -> Self {
        Hud { pixels: vec![0; (W * H * 4) as usize], mode: Mode::Full, bar: Vec::new(), bar_w: 0, items: Vec::new(), lines: Vec::new(), times: VecDeque::new(), beat: None, alert: None }
    }
}

/// A running count's increase over a window (a minute by default), from samples taken now and then (#188: the
/// device's underruns per minute; #208: the audio glitches per 10 minutes).
pub struct PerMinute {
    samples: VecDeque<(f32, u64)>,
    window: f32,
}

impl Default for PerMinute {
    fn default() -> Self {
        PerMinute::over(60.0)
    }
}

impl PerMinute {
    /// over the last `window` seconds
    pub fn over(window: f32) -> PerMinute {
        PerMinute { samples: VecDeque::new(), window }
    }

    /// the count `n` at `t` s → its increase over the window (counted from 0 at the first sample's moment)
    pub fn push(&mut self, t: f32, n: u64) -> u64 {
        if self.samples.is_empty() {
            self.samples.push_back((t, 0));
        }
        self.samples.push_back((t, n));
        while self.samples.len() > 1 && t - self.samples[0].0 > self.window {
            self.samples.pop_front();
        }
        n.saturating_sub(self.samples[0].1)
    }
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

impl Hud {
    /// a HUD showing `mode` first
    pub fn with_mode(mode: Mode) -> Hud {
        Hud { mode, ..Hud::default() }
    }

    /// anything on screen (the full HUD or the bar)
    pub fn visible(&self) -> bool {
        self.mode != Mode::Off
    }

    /// #209: the bar's items (FPS, CPU …), left to right
    pub fn set_items(&mut self, items: Vec<Item>) {
        self.items = items;
    }

    /// #209: the bar rastered `width` HUD pixels wide (`bar_width`): the items two spaces apart, cut at the edge
    pub fn raster_bar(&mut self, width: u32) {
        let width = width.clamp(8, BAR_MAX);
        if width != self.bar_w {
            self.bar_w = width;
            self.bar = vec![0; (width * BAR_H * 4) as usize];
        }
        for px in self.bar.chunks_exact_mut(4) {
            px.copy_from_slice(&[0, 0, 0, 200]);
        }
        let cols = (width as usize).saturating_sub(8) / 8;
        let items = std::mem::take(&mut self.items);
        let mut col = 0usize;
        for it in &items {
            if col >= cols {
                break;
            }
            let text: String = it.text.chars().take(cols - col).collect();
            let n = text.chars().count();
            let (bw, bh) = (self.bar_w, BAR_H);
            let mut put = |x: u32, y: u32, c: [u8; 4]| {
                if x < bw && y < bh {
                    let i = ((y * bw + x) * 4) as usize;
                    self.bar[i..i + 4].copy_from_slice(&c);
                }
            };
            glyphs(&mut put, 4 + col as u32 * 8, 1, &text, if it.bad { ALERT } else { TEXT });
            col += n + 2;
        }
        self.items = items;
    }

    pub fn set_lines(&mut self, lines: Vec<String>) {
        self.lines = lines;
    }

    pub fn set_beat(&mut self, beat: Option<Beat>) {
        self.beat = beat;
    }

    pub fn set_alert(&mut self, alert: Option<Alert>) {
        self.alert = alert;
    }

    pub fn push_frame(&mut self, ms: f32) {
        if self.times.len() == (W / 2) as usize {
            self.times.pop_front();
        }
        self.times.push_back(ms);
    }

    pub fn raster(&mut self) {
        for px in self.pixels.chunks_exact_mut(4) {
            px.copy_from_slice(&[0, 0, 0, 170]);
        }
        let lines = std::mem::take(&mut self.lines);
        for (row, line) in lines.iter().enumerate() {
            let y = 4 + row as u32 * 10;
            match self.alert.filter(|a| a.row == row) {
                // #208: the glitch warning – white on a red bar, then red on black: it flashes
                Some(Alert { lit: true, .. }) => {
                    for yy in y - 1..y + 9 {
                        for x in 0..W {
                            self.put(x, yy, ALERT);
                        }
                    }
                    self.text_in(4, y, line, TEXT);
                }
                Some(_) => self.text_in(4, y, line, ALERT),
                None => self.text_in(4, y, line, TEXT),
            }
        }
        self.lines = lines;
        if let Some(b) = self.beat {
            let n = b.per_bar.clamp(1, 8);
            for i in 0..n {
                let c = match (b.running && i == b.beat % n, i == 0) {
                    (true, true) => [250, 170, 40, 255],
                    (true, false) => [235, 235, 235, 255],
                    _ => [70, 70, 70, 255],
                };
                let (x, y) = (W - 4 - (n - i) * 8, 4 + b.row as u32 * 10 + 1);
                for dy in 0..6 {
                    for dx in 0..6 {
                        self.put(x + dx, y + dy, c);
                    }
                }
            }
        }
        let base = H - 2;
        let target = base - (16.7 / GRAPH_MS * GRAPH_H as f32) as u32;
        for x in 0..W {
            self.put(x, target, [90, 90, 90, 255]);
        }
        for i in 0..self.times.len() {
            let ms = self.times[i];
            let h = ((ms / GRAPH_MS).min(1.0) * GRAPH_H as f32).max(1.0) as u32;
            let c = if ms < 17.5 { [80, 220, 120, 255] } else if ms < 34.0 { [240, 200, 60, 255] } else { [240, 70, 70, 255] };
            for y in 0..h {
                self.put(i as u32 * 2, base - y, c);
                self.put(i as u32 * 2 + 1, base - y, c);
            }
        }
    }

    fn text_in(&mut self, x: u32, y: u32, s: &str, color: [u8; 4]) {
        let s: String = s.chars().take((W / 8) as usize).collect();
        let mut put = |x: u32, y: u32, c: [u8; 4]| self.put(x, y, c);
        glyphs(&mut put, x, y, &s, color);
    }

    fn put(&mut self, x: u32, y: u32, c: [u8; 4]) {
        if x < W && y < H {
            let i = ((y * W + x) * 4) as usize;
            self.pixels[i..i + 4].copy_from_slice(&c);
        }
    }
}

/// `s` in font8x8 from (x, y), through `put`
fn glyphs(put: &mut dyn FnMut(u32, u32, [u8; 4]), x: u32, y: u32, s: &str, color: [u8; 4]) {
    for (i, ch) in s.chars().enumerate() {
        let glyph = font8x8::BASIC_FONTS.get(ch).or_else(|| font8x8::BASIC_FONTS.get('?')).unwrap_or([0; 8]);
        for (gy, bits) in glyph.iter().enumerate() {
            for gx in 0..8 {
                if bits & (1 << gx) != 0 {
                    put(x + i as u32 * 8 + gx, y + gy as u32, color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_minute_counts_the_last_minute() {
        let mut p = PerMinute::default();
        assert_eq!(p.push(0.0, 0), 0);
        assert_eq!(p.push(1.0, 2), 2, "two underruns in the first second");
        for s in 2..=61 {
            p.push(s as f32, 2);
        }
        assert_eq!(p.push(62.0, 3), 1, "the first two are older than a minute");
        assert!(p.samples.len() <= 62);
    }

    #[test]
    fn text_and_graph_land_in_the_image() {
        let mut h = Hud::default();
        h.set_lines(vec!["FPS 60".into()]);
        h.push_frame(16.0);
        h.raster();
        let lit = h.pixels.chunks_exact(4).filter(|p| p[0] == 235).count();
        assert!(lit > 20, "text pixels: {lit}");
        let green = h.pixels.chunks_exact(4).filter(|p| p[1] == 220).count();
        assert!(green > 0);
    }

    #[test]
    fn the_window_is_ten_minutes_for_glitches() {
        let mut p = PerMinute::over(600.0);
        p.push(0.0, 0);
        assert_eq!(p.push(30.0, 1), 1);
        assert_eq!(p.push(500.0, 1), 1, "still within 10 minutes");
        assert_eq!(p.push(640.0, 1), 0, "older than 10 minutes");
    }

    /// #208: the warning row flashes – a red bar with white text, then red text on the HUD's black
    #[test]
    fn the_glitch_warning_flashes_red() {
        let mut h = Hud::default();
        h.set_lines(vec!["TITLE".into(), "AUDIO GLITCH".into()]);
        let red = |h: &Hud| h.pixels.chunks_exact(4).filter(|p| p == &ALERT).count();
        h.raster();
        assert_eq!(red(&h), 0);
        h.set_alert(Some(Alert { row: 1, lit: true }));
        h.raster();
        assert!(red(&h) > W as usize * 8, "a red bar across the HUD");
        h.set_alert(Some(Alert { row: 1, lit: false }));
        h.raster();
        let text = red(&h);
        assert!(text > 20 && text < 12 * 64, "red letters only: {text}");
    }

    #[test]
    fn the_beat_lights_show_the_bar() {
        let mut h = Hud::default();
        let amber = |h: &Hud| h.pixels.chunks_exact(4).filter(|p| p == &[250, 170, 40, 255]).count();
        h.set_beat(Some(Beat { row: 2, beat: 0, per_bar: 4, running: true }));
        h.raster();
        assert_eq!(amber(&h), 36, "the bar's first beat, lit");
        h.set_beat(Some(Beat { row: 2, beat: 1, per_bar: 4, running: true }));
        h.raster();
        assert_eq!(amber(&h), 0);
    }

    /// #209: F1 cycles full → bar → off; the bar spans the screen at the HUD's scale, an item out of the green in red,
    /// the items cut at its edge
    #[test]
    fn the_bar_spans_the_width() {
        assert_eq!([Mode::Full.next(), Mode::Bar.next(), Mode::Off.next()], [Mode::Bar, Mode::Off, Mode::Full]);
        assert_eq!((bar_width(1920.0, 1080.0), bar_width(1280.0, 720.0), bar_width(9000.0, 540.0)), (960, 1280, BAR_MAX));
        let mut h = Hud::default();
        h.set_items(vec![Item { text: "FPS 60".into(), bad: false }, Item { text: "GLITCH 2/10M".into(), bad: true }]);
        h.raster_bar(960);
        assert_eq!(h.bar.len(), (960 * BAR_H * 4) as usize);
        let count = |h: &Hud, c: [u8; 4]| h.bar.chunks_exact(4).filter(|p| *p == c).count();
        assert!(count(&h, TEXT) > 20 && count(&h, ALERT) > 20, "white and red letters");
        // the second item starts after the first and two spaces: column 8 = x 68
        let red_x = h.bar.chunks_exact(4).enumerate().filter(|(_, p)| *p == ALERT).map(|(i, _)| i as u32 % 960).min().unwrap();
        assert!((68..76).contains(&red_x), "{red_x}");
        h.raster_bar(40);
        assert!(count(&h, ALERT) == 0, "cut at the edge: four columns hold only FPS");
        h.mode = Mode::Off;
        assert!(!h.visible());
    }

    #[test]
    fn long_text_wraps_at_spaces() {
        let w = wrap("this build of the stage does not carry the module 'space' (node space)", 20);
        assert_eq!(w, ["this build of the", "stage does not carry", "the module 'space'", "(node space)"]);
        assert_eq!(wrap("abcdefghij", 4), ["abcd"]);
        assert!(wrap("  ", 10).is_empty());
    }
}
