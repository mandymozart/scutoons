//! The on-screen HUD (CYBERDECKS.md §8 "a small on-screen HUD"): a few text lines and a frame-time graph rastered on
//! the CPU into a small RGBA image (font8x8, no font files), drawn over the screen by render.rs at an integer scale.
//! The tempo line carries a light per beat of the bar (#181): the current one lit, the bar's first in amber.
//!
//! #209: F1 cycles the full HUD → the bar → nothing; the bar is rastered at the screen's width in HUD pixels
//! (`bar_width`), so it spans the screen at the HUD's integer scale. #374: what the lines and the bar's items say, the
//! mode and the sizes are deck-ui's (the platform-free UI model); this file only rasters them.
pub use deck_ui::{wrap, Item, Level, Mode, COLS, LINES};
use font8x8::UnicodeFonts;
use std::collections::VecDeque;

pub const W: u32 = 320;
pub const H: u32 = 4 + LINES as u32 * 10 + GRAPH_H + 4;
const GRAPH_H: u32 = 20;
/// the graph's full height in ms (two 60 Hz frames)
const GRAPH_MS: f32 = 33.3;
const TEXT: [u8; 4] = [235, 235, 235, 255];
/// the audio glitch warning's red (#208)
const ALERT: [u8; 4] = [230, 40, 40, 255];
/// #364: a number out of the green but not bad yet (the browser bar's amber)
const WARN: [u8; 4] = [240, 170, 40, 255];

/// the bar's height (one text line and a pixel above / below, in HUD pixels) and its widest raster
pub const BAR_H: u32 = 10;
pub const BAR_MAX: u32 = 2048;

/// the HUD's integer scale for a screen `sh` pixels high – rem-like: 2 at 1080p
pub fn scale(sh: f32) -> f32 {
    (sh / 540.0).round().max(1.0)
}

/// where the full HUD sits on a `sw` × `sh` screen: `[x, y, w, h]` in screen pixels – top left, a 1rem margin at its scale
pub fn rect(sw: f32, sh: f32) -> [f32; 4] {
    let s = scale(sh);
    let (hw, hh) = ((W as f32 * s).min(sw), (H as f32 * s).min(sh));
    let m = (16.0 * s).min(sw - hw).max(0.0);
    [m, m.min(sh - hh).max(0.0), hw, hh]
}

/// #478: the full HUD's text line under the screen point (x, y) – a tap on a page's line (the Live page)
pub fn line_at(x: f32, y: f32, sw: f32, sh: f32) -> Option<usize> {
    line_in(rect(sw, sh), scale(sh), x, y)
}

/// #478: the text line under the screen point (x, y) of the full HUD drawn at `at` (`[x, y, w, h]`) and scale `s`
pub fn line_in(at: [f32; 4], s: f32, x: f32, y: f32) -> Option<usize> {
    let [rx, ry, rw, _] = at;
    let (hx, hy) = ((x - rx) / s, (y - ry) / s);
    let row = ((hy - 3.0) / 10.0).floor();
    (hx >= 0.0 && hx < rw / s && row >= 0.0 && (row as usize) < LINES).then_some(row as usize)
}

/// #478: where a page on the full HUD (the Live page) sits when the host lays its own things over the screen's top edge
/// and corners (the browser deck: its stats line across the top, the reader's logo and menu in the corners): centred in
/// what is below `top` screen pixels. Its scale is the one the HUD has on a 1080p screen, per CSS pixel (`density` screen
/// pixels each) – the browser page as large as the native deck's page at 1080p, its 8 px font at 16 CSS px – or the
/// largest integer below it that fits with the HUD's 1rem margin (never below 1). `([x, y, w, h], scale)`.
pub fn page_rect(sw: f32, sh: f32, density: f32, top: f32) -> ([f32; 4], f32) {
    let want = (scale(1080.0) * density.max(0.5)).round();
    let below = (sh - top).max(0.0);
    let fits = (sw / (W as f32 + 32.0)).min(below / (H as f32 + 32.0)).floor();
    let s = want.min(fits).max(1.0);
    let (w, h) = ((W as f32 * s).min(sw), (H as f32 * s).min(sh));
    let y = (top + (below - h) / 2.0).min(sh - h).max(0.0);
    ([((sw - w) / 2.0).max(0.0), y, w, h], s)
}

/// the build label's integer scale: the HUD's, by the width on a portrait screen (a phone)
pub fn label_scale(sw: f32, sh: f32) -> f32 {
    if sh > sw { (sw / 375.0).round().max(1.0) } else { scale(sh) }
}

/// the bar's width in HUD pixels for a `sw` × `sh` screen: the whole width at the HUD's scale
pub fn bar_width(sw: f32, sh: f32) -> u32 {
    ((sw / scale(sh)) as u32).clamp(8, BAR_MAX)
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
            glyphs(&mut put, 4 + col as u32 * 8, 1, &text, match it.level {
                Level::Bad => ALERT,
                Level::Warn => WARN,
                Level::Ok => TEXT,
            });
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
pub(crate) fn glyphs(put: &mut dyn FnMut(u32, u32, [u8; 4]), x: u32, y: u32, s: &str, color: [u8; 4]) {
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

    /// #478: a tap finds the HUD's line: top left, 1rem in, 10 HUD pixels a line at the HUD's scale (2 at 1080p)
    #[test]
    fn a_tap_finds_the_line() {
        assert_eq!(rect(1920.0, 1080.0), [32.0, 32.0, 640.0, H as f32 * 2.0]);
        // line 0's text starts 4 HUD pixels down: y = 32 + 8; line 2 at 32 + 2 * (4 + 20)
        assert_eq!(line_at(40.0, 40.0, 1920.0, 1080.0), Some(0));
        assert_eq!(line_at(100.0, 32.0 + 2.0 * 24.0 + 4.0, 1920.0, 1080.0), Some(2));
        assert_eq!(line_at(10.0, 40.0, 1920.0, 1080.0), None, "left of the HUD");
        assert_eq!(line_at(700.0, 40.0, 1920.0, 1080.0), None, "right of it");
        assert_eq!(line_at(40.0, 1000.0, 1920.0, 1080.0), None, "below its lines");
    }

    /// #478: the browser's Live page – centred below the host's top line, at 16 CSS px a glyph where it fits, the tap on it
    #[test]
    fn the_browser_page_sits_free_and_readable() {
        let free = |[x, y, w, h]: [f32; 4], sw: f32, sh: f32, top: f32| x >= 0.0 && y >= top && x + w <= sw && y + h <= sh;
        // the measured pane: 1024 x 768 CSS px at density 1, the stats line 24 px high – the HUD's own scale there was 1
        assert_eq!(scale(768.0), 1.0, "why it was small: 8 px glyphs on 10 px lines");
        let (at, s) = page_rect(1024.0, 768.0, 1.0, 24.0);
        assert_eq!((at, s), ([192.0, 24.0 + (744.0 - 2.0 * H as f32) / 2.0, 640.0, 2.0 * H as f32], 2.0));
        assert!(free(at, 1024.0, 768.0, 24.0) && at[0] > 3.0 * 16.0 && at[1] > 3.5 * 16.0, "clear of the stats line and the logo's corner");
        // a phone, 375 x 812 CSS px at density 2: as large as the width allows
        let (at, s) = page_rect(750.0, 1624.0, 2.0, 48.0);
        assert_eq!((s, at[2]), (2.0, 640.0));
        assert!(free(at, 750.0, 1624.0, 48.0));
        // a 1080p screen in full screen at density 1, a retina laptop at 2: the wanted scale
        assert_eq!((page_rect(1920.0, 1080.0, 1.0, 24.0).1, page_rect(2880.0, 1800.0, 2.0, 48.0).1), (2.0, 4.0));
        // too small for anything: scale 1, inside the screen
        let (at, s) = page_rect(200.0, 100.0, 1.0, 24.0);
        assert!(s == 1.0 && at[0] >= 0.0 && at[1] >= 0.0 && at[2] <= 200.0 && at[3] <= 100.0);
        // a tap: line 0 is 4 HUD pixels into the page, line 2 two lines further
        let (at, s) = page_rect(1024.0, 768.0, 1.0, 24.0);
        assert_eq!(line_in(at, s, at[0] + 8.0, at[1] + 8.0), Some(0));
        assert_eq!(line_in(at, s, at[0] + 100.0, at[1] + 2.0 * 20.0 + 8.0), Some(2));
        assert_eq!(line_in(at, s, at[0] - 1.0, at[1] + 8.0), None, "left of the page");
        assert_eq!(line_at(40.0, 40.0, 1920.0, 1080.0), line_in(rect(1920.0, 1080.0), 2.0, 40.0, 40.0), "the native page's tap as before");
    }

    /// #209: the bar spans the screen at the HUD's scale, an item out of the green in red, the items cut at its edge
    #[test]
    fn the_bar_spans_the_width() {
        assert_eq!((bar_width(1920.0, 1080.0), bar_width(1280.0, 720.0), bar_width(9000.0, 540.0)), (960, 1280, BAR_MAX));
        let mut h = Hud::default();
        h.set_items(vec![Item { text: "FPS 60".into(), level: Level::Ok }, Item { text: "GLITCH 2/10M".into(), level: Level::Bad }]);
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
}
