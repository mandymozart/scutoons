//! The on-screen HUD (CYBERDECKS.md §8 "a small on-screen HUD"): a few text lines and a frame-time graph rastered on
//! the CPU into a small RGBA image (font8x8, no font files), drawn over the screen by render.rs at an integer scale.
//! The tempo line carries a light per beat of the bar (#181): the current one lit, the bar's first in amber.
use font8x8::UnicodeFonts;
use std::collections::VecDeque;

pub const W: u32 = 320;
/// text lines the HUD has room for (above the frame-time graph)
pub const LINES: usize = 11;
/// characters a line has room for (8 px each, a 4 px margin)
pub const COLS: usize = 39;
pub const H: u32 = 4 + LINES as u32 * 10 + GRAPH_H + 4;
const GRAPH_H: u32 = 20;
/// the graph's full height in ms (two 60 Hz frames)
const GRAPH_MS: f32 = 33.3;

pub struct Hud {
    pub pixels: Vec<u8>,
    pub visible: bool,
    lines: Vec<String>,
    times: VecDeque<f32>,
    beat: Option<Beat>,
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
        Hud { pixels: vec![0; (W * H * 4) as usize], visible: true, lines: Vec::new(), times: VecDeque::new(), beat: None }
    }
}

/// A running count's increase over the last minute, from samples taken now and then (#188: the device's underruns per
/// minute on the AUDIO line).
#[derive(Default)]
pub struct PerMinute {
    samples: VecDeque<(f32, u64)>,
}

impl PerMinute {
    /// the count `n` at `t` s → its increase over the last 60 s (counted from 0 at the first sample's moment)
    pub fn push(&mut self, t: f32, n: u64) -> u64 {
        if self.samples.is_empty() {
            self.samples.push_back((t, 0));
        }
        self.samples.push_back((t, n));
        while self.samples.len() > 1 && t - self.samples[0].0 > 60.0 {
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
    pub fn set_lines(&mut self, lines: Vec<String>) {
        self.lines = lines;
    }

    pub fn set_beat(&mut self, beat: Option<Beat>) {
        self.beat = beat;
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
            self.text(4, 4 + row as u32 * 10, line);
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

    fn text(&mut self, x: u32, y: u32, s: &str) {
        for (i, ch) in s.chars().take((W / 8) as usize).enumerate() {
            let glyph = font8x8::BASIC_FONTS.get(ch).or_else(|| font8x8::BASIC_FONTS.get('?')).unwrap_or([0; 8]);
            for (gy, bits) in glyph.iter().enumerate() {
                for gx in 0..8 {
                    if bits & (1 << gx) != 0 {
                        self.put(x + i as u32 * 8 + gx, y + gy as u32, [235, 235, 235, 255]);
                    }
                }
            }
        }
    }

    fn put(&mut self, x: u32, y: u32, c: [u8; 4]) {
        if x < W && y < H {
            let i = ((y * W + x) * 4) as usize;
            self.pixels[i..i + 4].copy_from_slice(&c);
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

    #[test]
    fn long_text_wraps_at_spaces() {
        let w = wrap("this build of the stage does not carry the module 'space' (node space)", 20);
        assert_eq!(w, ["this build of the", "stage does not carry", "the module 'space'", "(node space)"]);
        assert_eq!(wrap("abcdefghij", 4), ["abcd"]);
        assert!(wrap("  ", 10).is_empty());
    }
}
