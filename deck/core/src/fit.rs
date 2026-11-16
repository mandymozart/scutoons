//! A picture fitted into a frame, its aspect kept: `cover` fills the frame and cuts the overflow off centred, `contain`
//! shows it whole. Sizes are (width, height) in any one unit.

/// the scale that makes `picture` fill `frame`: too wide, it fits the height; too tall, the width
pub fn cover_scale(picture: (f32, f32), frame: (f32, f32)) -> f32 {
    let (pw, ph) = (picture.0.max(f32::MIN_POSITIVE), picture.1.max(f32::MIN_POSITIVE));
    (frame.0 / pw).max(frame.1 / ph)
}

/// the scale that shows `picture` whole inside `frame`
pub fn contain_scale(picture: (f32, f32), frame: (f32, f32)) -> f32 {
    let (pw, ph) = (picture.0.max(f32::MIN_POSITIVE), picture.1.max(f32::MIN_POSITIVE));
    (frame.0 / pw).min(frame.1 / ph)
}

/// The part of `picture` a cover fit shows, centred, as texture coordinates: `[u, v, width, height]` (0–1).
pub fn cover(picture: (f32, f32), frame: (f32, f32)) -> [f32; 4] {
    let k = cover_scale(picture, frame);
    let w = (frame.0 / (picture.0 * k)).clamp(0.0, 1.0);
    let h = (frame.1 / (picture.1 * k)).clamp(0.0, 1.0);
    [(1.0 - w) / 2.0, (1.0 - h) / 2.0, w, h]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: [f32; 4], b: [f32; 4]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6)
    }

    #[test]
    fn too_wide_fits_the_height() {
        // 2:1 into 16:9: the height fills, the sides are cut
        let r = cover((2000.0, 1000.0), (1600.0, 900.0));
        assert!(near(r, [(1.0 - 0.888_888_9) / 2.0, 0.0, 0.888_888_9, 1.0]), "{r:?}");
        assert_eq!(cover_scale((2000.0, 1000.0), (1600.0, 900.0)), 0.9);
    }

    #[test]
    fn too_tall_fits_the_width() {
        let r = cover((1000.0, 2000.0), (1000.0, 1000.0));
        assert!(near(r, [0.0, 0.25, 1.0, 0.5]), "{r:?}");
        assert_eq!(cover_scale((1000.0, 2000.0), (1000.0, 1000.0)), 1.0);
    }

    #[test]
    fn exact_shows_it_all() {
        assert!(near(cover((1920.0, 1080.0), (1280.0, 720.0)), [0.0, 0.0, 1.0, 1.0]));
        assert!(near(cover((640.0, 640.0), (640.0, 640.0)), [0.0, 0.0, 1.0, 1.0]));
    }

    #[test]
    fn never_distorted() {
        for (p, f) in [((300.0, 100.0), (100.0, 300.0)), ((1.0, 5.0), (7.0, 2.0)), ((1920.0, 1080.0), (1080.0, 1920.0))] {
            let [_, _, w, h] = cover(p, f);
            // the shown part has the frame's aspect
            assert!(((w * p.0) / (h * p.1) - f.0 / f.1).abs() < 1e-4, "{p:?} into {f:?}");
        }
    }

    #[test]
    fn contain_fits_the_longer_side() {
        assert_eq!(contain_scale((2000.0, 1000.0), (1000.0, 1000.0)), 0.5);
        assert_eq!(contain_scale((1000.0, 2000.0), (1000.0, 1000.0)), 0.5);
    }
}
