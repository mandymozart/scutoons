//! The animated logo in the bar (owner 2026-11-09: "can you use the same animated svg we have on the creator?"): the creator's
//! SVG, its one scroll animation evaluated here and rasterised into frames at start.
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

const SVG: &str = include_str!("../../../creator/static/SCUTOONS-logo-animated.svg");
/// the animation's length and how finely it is sampled
pub const SECONDS: f32 = 6.0;
pub const FRAMES: usize = 72;
/// a frame's side in the atlas (pixels)
pub const SIDE: u32 = 72;

const TIMES: [f32; 5] = [0.0, 0.3, 0.4, 0.7, 1.0];
const VALUES: [f32; 5] = [8.0, -292.0, -342.0, -942.0, -1234.0];
const SPLINES: [[f32; 4]; 4] = [[0.4, 0.0, 0.6, 1.0], [0.33, 0.0, 0.67, 1.0], [0.2, 0.0, 0.8, 1.0], [0.4, 0.0, 0.6, 1.0]];

/// a cubic bezier's y at x, from (0, 0) to (1, 1)
fn ease(s: [f32; 4], x: f32) -> f32 {
    let bez = |a: f32, b: f32, t: f32| 3.0 * (1.0 - t) * (1.0 - t) * t * a + 3.0 * (1.0 - t) * t * t * b + t * t * t;
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..24 {
        let t = (lo + hi) * 0.5;
        if bez(s[0], s[2], t) < x {
            lo = t;
        } else {
            hi = t;
        }
    }
    bez(s[1], s[3], (lo + hi) * 0.5)
}

/// the scroll's y at `t` (0..1 of the animation)
pub fn scroll(t: f32) -> f32 {
    let i = (1..TIMES.len()).find(|&i| t < TIMES[i]).unwrap_or(TIMES.len() - 1);
    let k = ((t - TIMES[i - 1]) / (TIMES[i] - TIMES[i - 1])).clamp(0.0, 1.0);
    VALUES[i - 1] + (VALUES[i] - VALUES[i - 1]) * ease(SPLINES[i - 1], k)
}

/// every frame, straight-alpha RGBA, SIDE × SIDE
pub fn frames() -> Vec<Vec<u8>> {
    const GROUP: &str = "<g id=\"scroll-animator\">";
    assert!(SVG.contains(GROUP), "the logo's scroll group");
    let scale = SIDE as f32 / 256.0;
    (0..FRAMES)
        .map(|f| {
            let y = scroll(f as f32 / FRAMES as f32);
            let at = format!("<g id=\"scroll-animator\" transform=\"translate(74.5 {y})\">");
            let text = SVG.replace(GROUP, &at);
            let mut map = Pixmap::new(SIDE, SIDE).expect("a frame's pixmap");
            if let Ok(tree) = Tree::from_str(&text, &Options::default()) {
                resvg::render(&tree, Transform::from_scale(scale, scale), &mut map.as_mut());
            }
            map.pixels().iter().flat_map(|p| { let c = p.demultiply(); [c.red(), c.green(), c.blue(), c.alpha()] }).collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scroll_runs_through_its_keys() {
        assert_eq!(scroll(0.0), 8.0);
        assert!((scroll(0.3) + 292.0).abs() < 0.01);
        assert!((scroll(0.9999) + 1234.0).abs() < 1.0);
    }

    #[test]
    fn a_frame_is_drawn() {
        assert!(frames()[0].chunks(4).any(|p| p[3] > 0));
    }
}
