//! The on-screen HUD (CYBERDECKS.md §8 "a small on-screen HUD"): a few text lines and a frame-time graph rastered on
//! the CPU into a small RGBA image (font8x8, no font files), drawn over the screen by render.rs at an integer scale.
use font8x8::UnicodeFonts;
use std::collections::VecDeque;

pub const W: u32 = 320;
pub const H: u32 = 84;
const GRAPH_H: u32 = 20;
/// the graph's full height in ms (two 60 Hz frames)
const GRAPH_MS: f32 = 33.3;

pub struct Hud {
    pub pixels: Vec<u8>,
    pub visible: bool,
    lines: Vec<String>,
    times: VecDeque<f32>,
}

impl Default for Hud {
    fn default() -> Self {
        Hud { pixels: vec![0; (W * H * 4) as usize], visible: true, lines: Vec::new(), times: VecDeque::new() }
    }
}

impl Hud {
    pub fn set_lines(&mut self, lines: Vec<String>) {
        self.lines = lines;
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
}
