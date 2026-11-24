//! The overlay level's ground (RULES H15): a still of the frame, taken once when the overlay opens, made small and
//! blurred once, the overlay's see-through ground laid over it – drawn under the overlay instead of the workspace, so a
//! frame under the overlay costs one picture and the projector's output is never touched.

/// a still: its pixels (RGBA, 8 bits, straight alpha) and size
#[derive(Debug, Clone, PartialEq)]
pub struct Still {
    pub rgba: Vec<u8>,
    pub w: u32,
    pub h: u32,
}

/// how much smaller the still is kept than the frame: the blur hides what a larger one would keep
pub const SHRINK: u32 = 4;

/// `rgba` (`w` × `h`) made `SHRINK` times smaller, blurred by `radius` frame pixels (three box passes each way: close to
/// a Gaussian), `ground` (straight RGBA 0..1) laid over it; an empty frame gives an empty still
pub fn blurred(rgba: &[u8], (w, h): (u32, u32), radius: f32, ground: [f32; 4]) -> Still {
    let (sw, sh) = (w.div_ceil(SHRINK).max(1), h.div_ceil(SHRINK).max(1));
    if w == 0 || h == 0 || rgba.len() < (w * h * 4) as usize {
        return Still { rgba: Vec::new(), w: 0, h: 0 };
    }
    // the mean of each SHRINK × SHRINK block, in floats
    let mut px = vec![[0f32; 3]; (sw * sh) as usize];
    let mut n = vec![0f32; (sw * sh) as usize];
    for y in 0..h {
        let row = (y / SHRINK * sw) as usize;
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let o = row + (x / SHRINK) as usize;
            for c in 0..3 {
                px[o][c] += f32::from(rgba[i + c]);
            }
            n[o] += 1.0;
        }
    }
    for (p, k) in px.iter_mut().zip(&n) {
        for c in p.iter_mut() {
            *c /= k.max(1.0);
        }
    }
    let r = ((radius / SHRINK as f32) / 3.0f32.sqrt()).round().max(1.0) as i32;
    for _ in 0..3 {
        px = pass(&px, (sw as i32, sh as i32), r, true);
        px = pass(&px, (sw as i32, sh as i32), r, false);
    }
    let a = ground[3].clamp(0.0, 1.0);
    let mut out = Vec::with_capacity(px.len() * 4);
    for p in px {
        for c in 0..3 {
            out.push((p[c] * (1.0 - a) + ground[c] * 255.0 * a).round().clamp(0.0, 255.0) as u8);
        }
        out.push(255);
    }
    Still { rgba: out, w: sw, h: sh }
}

/// one box pass of radius `r` along a row (`across`) or a column, the edges held
fn pass(px: &[[f32; 3]], (w, h): (i32, i32), r: i32, across: bool) -> Vec<[f32; 3]> {
    let mut out = vec![[0f32; 3]; px.len()];
    let (lines, len) = if across { (h, w) } else { (w, h) };
    let at = |line: i32, k: i32| -> usize { if across { (line * w + k.clamp(0, w - 1)) as usize } else { (k.clamp(0, h - 1) * w + line) as usize } };
    let span = (2 * r + 1) as f32;
    for line in 0..lines {
        let mut sum = [0f32; 3];
        for k in -r..=r {
            let p = px[at(line, k)];
            for c in 0..3 {
                sum[c] += p[c];
            }
        }
        for k in 0..len {
            let o = at(line, k);
            for c in 0..3 {
                out[o][c] = sum[c] / span;
            }
            let (add, drop) = (px[at(line, k + r + 1)], px[at(line, k - r)]);
            for c in 0..3 {
                sum[c] += add[c] - drop[c];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_still_is_smaller_blurred_and_carries_the_ground() {
        // a white square in the middle of black, 64 × 32
        let (w, h) = (64u32, 32u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 12..20 {
            for x in 28..36 {
                let i = ((y * w + x) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        let s = blurred(&rgba, (w, h), 8.0, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!((s.w, s.h, s.rgba.len()), (16, 8, 16 * 8 * 4));
        let at = |x: u32, y: u32| s.rgba[((y * s.w + x) * 4) as usize];
        assert!(at(8, 4) > at(5, 4) && at(5, 4) > at(0, 4), "brightest in the middle, fading out: {} {} {}", at(8, 4), at(5, 4), at(0, 4));
        assert!(at(8, 4) < 255, "the square is spread");
        assert!(s.rgba.chunks(4).all(|p| p[3] == 255), "opaque: the stage under it is the still");
        let grounded = blurred(&rgba, (w, h), 8.0, [1.0, 1.0, 1.0, 0.5]);
        let g = |x: u32, y: u32| grounded.rgba[((y * grounded.w + x) * 4) as usize];
        assert!(g(0, 0) >= 127, "half the ground over black");
    }

    #[test]
    fn no_frame_no_still() {
        assert_eq!(blurred(&[], (0, 0), 8.0, [0.0; 4]).w, 0);
        assert_eq!(blurred(&[0; 8], (4, 4), 8.0, [0.0; 4]).w, 0, "too few pixels");
    }
}
